use crate::auth::hash_password;
use crate::models::*;
use chrono::Local;
use rusqlite::{params, Connection, Result as SqlResult, Transaction};
use serde_json::json;
use std::sync::Mutex;
use tauri::State;
use tracing::{error, info, warn};
use uuid::Uuid;

type CommandResult<T> = Result<T, String>;

pub struct AppState {
    pub db: Mutex<Connection>,
}

pub fn init_database(conn: &Connection) -> SqlResult<()> {
    info!("Initializing database schema");
    // Enable foreign key constraints
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Execute schema SQL (embedded at compile time)
    let schema_sql = include_str!("../schema.sql");
    conn.execute_batch(schema_sql)?;

    info!("Database schema initialized successfully");
    Ok(())
}

pub fn run_auth_migration(conn: &Connection) -> CommandResult<()> {
    info!("Running auth migration - seeding roles and admin user");

    // Check if roles already exist
    let roles_exist: i64 = conn
        .query_row("SELECT COUNT(*) FROM roles", [], |row| row.get(0))
        .map_err(|e| format!("Failed to check roles: {}", e))?;

    if roles_exist > 0 {
        info!("Roles already seeded, skipping auth migration");
        return Ok(());
    }

    // Seed roles
    let scheduler_role_id = "00000000-0000-0000-0000-000000000001";
    let teacher_role_id = "00000000-0000-0000-0000-000000000002";

    conn.execute(
        "INSERT INTO roles (id, name, description) VALUES (?1, ?2, ?3)",
        params![
            scheduler_role_id,
            "Scheduler",
            "Full system access including user management and audit logs"
        ],
    )
    .map_err(|e| format!("Failed to insert Scheduler role: {}", e))?;

    conn.execute(
        "INSERT INTO roles (id, name, description) VALUES (?1, ?2, ?3)",
        params![
            teacher_role_id,
            "Teacher",
            "View own timetable and export to CSV"
        ],
    )
    .map_err(|e| format!("Failed to insert Teacher role: {}", e))?;

    info!("Roles seeded successfully");

    // Create default admin user
    let admin_id = Uuid::new_v4().to_string();
    let admin_password_hash =
        hash_password("123456").map_err(|e| format!("Failed to hash admin password: {}", e))?;
    let created_at = Local::now().to_rfc3339();

    conn.execute(
        "INSERT INTO users (id, username, password_hash, role_id, teacher_id, created_at, last_login)
         VALUES (?1, ?2, ?3, ?4, NULL, ?5, NULL)",
        params![
            admin_id,
            "admin",
            admin_password_hash,
            scheduler_role_id,
            created_at
        ],
    ).map_err(|e| format!("Failed to create admin user: {}", e))?;

    info!("Default admin user created (username: admin, password: 123456)");
    info!("Auth migration completed successfully");
    Ok(())
}

/// Get the Teacher role ID from the main roles table
pub fn get_teacher_role_id(conn: &Connection) -> Result<String, String> {
    let role_id: String = conn
        .query_row("SELECT id FROM roles WHERE name = 'Teacher'", [], |row| {
            row.get(0)
        })
        .map_err(|e| format!("Failed to get Teacher role ID: {}", e))?;

    Ok(role_id)
}

/// Automatically create a user account for a teacher
/// Returns true if user was created, false if user already exists
pub fn auto_create_user_for_teacher(
    conn: &Connection,
    teacher_id: &str,
    teacher_name: &str,
    performer_id: Option<&str>,
) -> Result<bool, String> {
    use crate::audit::create_audit_log_entry;
    use crate::auth::{generate_unique_username, hash_password};

    // Check if user already exists for this teacher
    let existing_user: Result<i64, _> = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE teacher_id = ?1",
        params![teacher_id],
        |row| row.get(0),
    );

    if let Ok(count) = existing_user {
        if count > 0 {
            info!("User already exists for teacher: {}", teacher_name);
            return Ok(false);
        }
    }

    // Generate unique username
    let username = generate_unique_username(teacher_name, conn)?;

    // Get Teacher role ID
    let role_id = get_teacher_role_id(conn)?;

    // Hash default password "123456"
    let password_hash =
        hash_password("123456").map_err(|e| format!("Failed to hash password: {}", e))?;

    let user_id = Uuid::new_v4().to_string();
    let created_at = Local::now().to_rfc3339();

    conn.execute(
        "INSERT INTO users (id, username, password_hash, role_id, teacher_id, created_at, last_login)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
        params![user_id, username, password_hash, role_id, teacher_id, created_at],
    ).map_err(|e| format!("Failed to create user: {}", e))?;

    // Create audit log entry
    // Note: Using a system user ID for automated actions
    let initiator_id = performer_id.unwrap_or("00000000-0000-0000-0000-000000000000");
    let _ = create_audit_log_entry(
        conn,
        initiator_id,
        "AUTO_USER_CREATED",
        Some("users"),
        Some(&user_id),
        Some(json!({
            "target_name": username,
            "action": "自动创建账号",
            "role_name": "Teacher",
            "teacher_name": teacher_name
        })),
        None,
        false,
    );

    info!(
        "Auto-created user for teacher: {} (username: {})",
        teacher_name, username
    );
    Ok(true)
}

#[tauri::command]
pub async fn load_data(state: State<'_, AppState>) -> CommandResult<AllData> {
    info!("Loading data from database");
    let db = state.db.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Failed to acquire database lock: {}", e)
    })?;

    let data = load_all_data(&db)?;
    info!(
        "Data loaded: {} teachers, {} courses, {} campuses, {} time slots, {} days",
        data.teachers.len(),
        data.courses.len(),
        data.campuses.len(),
        data.time.len(),
        data.day.len()
    );
    Ok(data)
}

/// Save current application state to temp tables (auto-save target)
#[tauri::command]
pub async fn save_temp_data(
    content: AllData,
    _app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    info!(
        "Saving data to temp tables: {} teachers, {} courses",
        content.teachers.len(),
        content.courses.len()
    );
    let db = state.db.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Failed to acquire database lock: {}", e)
    })?;

    let tx = db.unchecked_transaction().map_err(|e| {
        error!("Failed to begin transaction: {}", e);
        format!("Failed to begin transaction: {}", e)
    })?;

    // Clear and write to temp tables
    match write_all_data_to_tables(&tx, &content, true) {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to write data: {}", e);
            return Err(e);
        }
    }

    tx.commit().map_err(|e| {
        error!("Failed to commit temp data: {}", e);
        format!("Failed to commit temp data: {}", e)
    })?;

    info!("Data saved to temp tables successfully");
    Ok(())
}

/// Commit temporary state to permanent storage
/// Copies all temp table contents to main tables
#[tauri::command]
pub async fn commit_data(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, crate::models::SessionStore>,
) -> CommandResult<()> {
    info!("Committing temp data to main tables");

    // Get user ID from session for audit logging
    let user_id = {
        let sessions_lock = sessions
            .lock()
            .map_err(|_| "Failed to lock sessions".to_string())?;
        sessions_lock.get(&session_id).map(|u| u.id.clone())
    };

    let db = state.db.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Failed to acquire database lock: {}", e)
    })?;

    let tx = db.unchecked_transaction().map_err(|e| {
        error!("Failed to begin transaction: {}", e);
        format!("Failed to begin transaction: {}", e)
    })?;

    let mut changes_summary = serde_json::Map::new();

    let teachers_changed: Vec<String> = tx
        .prepare("SELECT name FROM teachers_temp")
        .map_err(|e| e.to_string())?
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    if !teachers_changed.is_empty() {
        changes_summary.insert("teachers".to_string(), json!(teachers_changed));
    }

    let courses_changed: Vec<String> = tx
        .prepare("SELECT name FROM courses_temp")
        .map_err(|e| e.to_string())?
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    if !courses_changed.is_empty() {
        changes_summary.insert("courses".to_string(), json!(courses_changed));
    }

    let schedule_count: i64 = tx
        .query_row("SELECT COUNT(*) FROM scheduled_classes_temp", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    if schedule_count > 0 {
        changes_summary.insert("schedules_count".to_string(), json!(schedule_count));
    }

    let mut stmt = tx
        .prepare("SELECT id, name FROM teachers_temp WHERE id NOT IN (SELECT id FROM teachers)")
        .map_err(|e| e.to_string())?;
    let new_teachers: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    commit_temp_to_main(&tx)?;

    for (t_id, t_name) in new_teachers {
        auto_create_user_for_teacher(&tx, &t_id, &t_name, user_id.as_deref())?;
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit data: {}", e))?;

    use tauri::Emitter;
    app_handle
        .emit("commit-completed", ())
        .map_err(|e| e.to_string())?;

    info!("Data committed successfully");

    // Create audit log for commit operation (Feature: 001-rbac-audit-system - User Story 6)
    if let Some(uid) = user_id {
        let _ = crate::audit::create_audit_log_entry(
            &db,
            &uid,
            "COMMIT_OPERATION",
            None,
            None,
            Some(json!({
                "target_name": "批量提交",
                "summary": "提交了临时表中的更改",
                "details": changes_summary
            })),
            None,
            false,
        );
    }

    Ok(())
}

/// Clear all temporary changes (revert operation)
#[tauri::command]
pub async fn clear_temp_data(
    state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, crate::models::SessionStore>,
) -> CommandResult<()> {
    info!("Clearing all temp tables (revert)");

    // Get user ID from session for audit logging
    let user_id = {
        let sessions_lock = sessions
            .lock()
            .map_err(|_| "Failed to lock sessions".to_string())?;
        sessions_lock.get(&session_id).map(|u| u.id.clone())
    };

    let db = state.db.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Failed to acquire database lock: {}", e)
    })?;

    clear_all_temp_tables(&db, user_id.as_deref())?;
    info!("Temp tables cleared successfully");

    Ok(())
}

// ============================================================================
// DATABASE OPERATIONS - LOAD
// ============================================================================

/// Load AllData from database, prioritizing temp tables
/// Public function for use by import_export module
pub fn load_all_data_from_connection(conn: &Connection, use_temp: bool, include_auth: bool) -> CommandResult<AllData> {
    Ok(AllData {
        time: load_time_slots(conn, use_temp)?,
        day: load_days(conn, use_temp)?,
        campuses: load_campuses(conn, use_temp)?,
        venues: load_venues(conn, use_temp)?,
        courses: load_courses(conn, use_temp)?,
        teachers: load_teachers(conn, use_temp)?,
        course_venues: load_course_venues(conn, use_temp)?,
        teacher_courses: load_teacher_courses(conn, use_temp)?,
        scheduled_classes: load_scheduled_classes(conn, use_temp)?,
        teacher_unavailability: load_teacher_unavailability(conn, use_temp)?,
        schedule_density: load_schedule_density(conn, use_temp)?,
        roles: if include_auth { load_roles(conn)? } else { Vec::new() },
        users: if include_auth { load_users(conn)? } else { Vec::new() },
    })
}

#[tauri::command]
pub async fn list_committed_teachers(state: State<'_, AppState>) -> CommandResult<Vec<Teacher>> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    load_teachers(&db, false)
}

/// Load AllData from database, prioritizing temp tables (internal function)
fn load_all_data(conn: &Connection) -> CommandResult<AllData> {
    // Check if temp tables have data
    let has_temp_data: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM time_slots_temp)", [], |row| {
            row.get(0)
        })
        .map_err(|e| format!("Failed to check temp data: {}", e))?;

    let use_temp = has_temp_data;

    load_all_data_from_connection(conn, use_temp, true)
}

fn load_time_slots(conn: &Connection, use_temp: bool) -> CommandResult<Vec<TimeSlot>> {
    let table = if use_temp {
        "time_slots_temp"
    } else {
        "time_slots"
    };
    let query = format!("SELECT id, value, corresponding_hours FROM {}", table);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare time_slots query: {}", e))?;

    let slots: SqlResult<Vec<TimeSlot>> = stmt
        .query_map([], |row| {
            Ok(TimeSlot {
                id: row.get(0)?,
                value: row.get(1)?,
                corresponding_hours: row.get(2)?,
            })
        })
        .map_err(|e| format!("Failed to query time_slots: {}", e))?
        .collect();

    slots.map_err(|e| format!("Failed to load time_slots: {}", e))
}

fn load_days(conn: &Connection, use_temp: bool) -> CommandResult<Vec<Day>> {
    let table = if use_temp { "days_temp" } else { "days" };
    let query = format!("SELECT id, value FROM {}", table);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare days query: {}", e))?;

    let days: SqlResult<Vec<Day>> = stmt
        .query_map([], |row| {
            Ok(Day {
                id: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| format!("Failed to query days: {}", e))?
        .collect();

    days.map_err(|e| format!("Failed to load days: {}", e))
}

fn load_campuses(conn: &Connection, use_temp: bool) -> CommandResult<Vec<Campus>> {
    let table = if use_temp {
        "campuses_temp"
    } else {
        "campuses"
    };
    let query = format!("SELECT id, name FROM {}", table);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare campuses query: {}", e))?;

    let campuses: SqlResult<Vec<Campus>> = stmt
        .query_map([], |row| {
            Ok(Campus {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(|e| format!("Failed to query campuses: {}", e))?
        .collect();

    campuses.map_err(|e| format!("Failed to load campuses: {}", e))
}

fn load_venues(conn: &Connection, use_temp: bool) -> CommandResult<Vec<Venue>> {
    let table = if use_temp { "venues_temp" } else { "venues" };
    let query = format!("SELECT id, campus_id, name, capacity FROM {}", table);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare venues query: {}", e))?;

    let venues: SqlResult<Vec<Venue>> = stmt
        .query_map([], |row| {
            Ok(Venue {
                id: row.get(0)?,
                campus_id: row.get(1)?,
                name: row.get(2)?,
                capacity: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to query venues: {}", e))?
        .collect();

    venues.map_err(|e| format!("Failed to load venues: {}", e))
}

fn load_courses(conn: &Connection, use_temp: bool) -> CommandResult<Vec<Course>> {
    let table = if use_temp { "courses_temp" } else { "courses" };
    let query = format!("SELECT id, name FROM {}", table);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare courses query: {}", e))?;

    let courses: SqlResult<Vec<Course>> = stmt
        .query_map([], |row| {
            Ok(Course {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(|e| format!("Failed to query courses: {}", e))?
        .collect();

    courses.map_err(|e| format!("Failed to load courses: {}", e))
}

fn load_course_venues(conn: &Connection, use_temp: bool) -> CommandResult<Vec<CourseVenue>> {
    let table = if use_temp {
        "course_venues_temp"
    } else {
        "course_venues"
    };
    let query = format!("SELECT course_id, venue_id FROM {}", table);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare course_venues query: {}", e))?;

    let course_venues: SqlResult<Vec<CourseVenue>> = stmt
        .query_map([], |row| {
            Ok(CourseVenue {
                course_id: row.get(0)?,
                venue_id: row.get(1)?,
            })
        })
        .map_err(|e| format!("Failed to query course_venues: {}", e))?
        .collect();

    course_venues.map_err(|e| format!("Failed to load course_venues: {}", e))
}

fn load_teachers(conn: &Connection, use_temp: bool) -> CommandResult<Vec<Teacher>> {
    let table = if use_temp {
        "teachers_temp"
    } else {
        "teachers"
    };
    let query = format!(
        "SELECT id, name, max_teaching_hours, is_only_shahe FROM {}",
        table
    );

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare teachers query: {}", e))?;

    let teachers: SqlResult<Vec<Teacher>> = stmt
        .query_map([], |row| {
            Ok(Teacher {
                id: row.get(0)?,
                name: row.get(1)?,
                max_teaching_hours: row.get(2)?,
                is_only_shahe: row.get::<_, i32>(3)? != 0,
            })
        })
        .map_err(|e| format!("Failed to query teachers: {}", e))?
        .collect();

    teachers.map_err(|e| format!("Failed to load teachers: {}", e))
}

fn load_teacher_courses(conn: &Connection, use_temp: bool) -> CommandResult<Vec<TeacherCourse>> {
    let table = if use_temp {
        "teacher_courses_temp"
    } else {
        "teacher_courses"
    };
    let query = format!("SELECT teacher_id, course_id FROM {}", table);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare teacher_courses query: {}", e))?;

    let teacher_courses: SqlResult<Vec<TeacherCourse>> = stmt
        .query_map([], |row| {
            Ok(TeacherCourse {
                teacher_id: row.get(0)?,
                course_id: row.get(1)?,
            })
        })
        .map_err(|e| format!("Failed to query teacher_courses: {}", e))?
        .collect();

    teacher_courses.map_err(|e| format!("Failed to load teacher_courses: {}", e))
}

fn load_scheduled_classes(conn: &Connection, use_temp: bool) -> CommandResult<Vec<ScheduledClass>> {
    let table = if use_temp {
        "scheduled_classes_temp"
    } else {
        "scheduled_classes"
    };
    let query = format!(
        "SELECT id, teacher_id, course_id, day_id, time_id, campus_id, venue_id FROM {}",
        table
    );

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare scheduled_classes query: {}", e))?;

    let scheduled_classes: SqlResult<Vec<ScheduledClass>> = stmt
        .query_map([], |row| {
            Ok(ScheduledClass {
                id: row.get(0)?,
                teacher_id: row.get(1)?,
                course_id: row.get(2)?,
                day_id: row.get(3)?,
                time_id: row.get(4)?,
                campus_id: row.get(5)?,
                venue_id: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query scheduled_classes: {}", e))?
        .collect();

    scheduled_classes.map_err(|e| format!("Failed to load scheduled_classes: {}", e))
}

fn load_teacher_unavailability(
    conn: &Connection,
    use_temp: bool,
) -> CommandResult<Vec<TeacherUnavailability>> {
    let table = if use_temp {
        "teacher_unavailability_temp"
    } else {
        "teacher_unavailability"
    };
    let query = format!("SELECT teacher_id, day_id, time_id FROM {}", table);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare teacher_unavailability query: {}", e))?;

    let teacher_unavailability: SqlResult<Vec<TeacherUnavailability>> = stmt
        .query_map([], |row| {
            Ok(TeacherUnavailability {
                teacher_id: row.get(0)?,
                day_id: row.get(1)?,
                time_id: row.get(2)?,
            })
        })
        .map_err(|e| format!("Failed to query teacher_unavailability: {}", e))?
        .collect();

    teacher_unavailability.map_err(|e| format!("Failed to load teacher_unavailability: {}", e))
}

fn load_schedule_density(conn: &Connection, use_temp: bool) -> CommandResult<Vec<ScheduleDensity>> {
    let table = if use_temp {
        "schedule_density_temp"
    } else {
        "schedule_density"
    };
    let query = format!("SELECT campus_id, day_id, time_id, count FROM {}", table);

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare schedule_density query: {}", e))?;

    let schedule_density: SqlResult<Vec<ScheduleDensity>> = stmt
        .query_map([], |row| {
            Ok(ScheduleDensity {
                campus_id: row.get(0)?,
                day_id: row.get(1)?,
                time_id: row.get(2)?,
                count: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to query schedule_density: {}", e))?
        .collect();

    schedule_density.map_err(|e| format!("Failed to load schedule_density: {}", e))
}

/// Load roles from database (always from main table - roles are static)
fn load_roles(conn: &Connection) -> CommandResult<Vec<Role>> {
    let mut stmt = conn
        .prepare("SELECT id, name, description FROM roles")
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(Role {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
            })
        })
        .map_err(|e| format!("Failed to query roles: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect roles: {}", e))
}

/// Load users from database (Feature: 001-rbac-audit-system)
fn load_users(conn: &Connection) -> CommandResult<Vec<User>> {
    let mut stmt = conn
        .prepare("SELECT id, username, password_hash, role_id, teacher_id, created_at, last_login FROM users")
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: Some(row.get(2)?),
                role_id: row.get(3)?,
                teacher_id: row.get(4)?,
                created_at: row.get(5)?,
                last_login: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query users: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect users: {}", e))
}

// ============================================================================
// DATABASE OPERATIONS - WRITE
// ============================================================================

/// Write AllData to database tables (either main or temp)
/// Write all data to database tables (main or temp)
/// Public function for use by import_export module
pub fn write_all_data_to_tables(
    tx: &Transaction,
    data: &AllData,
    is_temp: bool,
) -> CommandResult<()> {
    let suffix = if is_temp { "_temp" } else { "" };

    // Clear all tables first (in reverse order to respect foreign keys)
    clear_tables(tx, is_temp)?;

    // Insert time slots
    let time_table = format!("time_slots{}", suffix);
    for slot in &data.time {
        tx.execute(
            &format!(
                "INSERT INTO {} (id, value, corresponding_hours) VALUES (?, ?, ?)",
                time_table
            ),
            params![slot.id, slot.value, slot.corresponding_hours],
        )
        .map_err(|e| format!("Failed to insert time slot: {}", e))?;
    }

    // Insert days
    let day_table = format!("days{}", suffix);
    for day in &data.day {
        tx.execute(
            &format!("INSERT INTO {} (id, value) VALUES (?, ?)", day_table),
            params![day.id, day.value],
        )
        .map_err(|e| format!("Failed to insert day: {}", e))?;
    }

    // Insert campuses
    let campus_table = format!("campuses{}", suffix);
    for campus in &data.campuses {
        tx.execute(
            &format!("INSERT INTO {} (id, name) VALUES (?, ?)", campus_table),
            params![campus.id, campus.name],
        )
        .map_err(|e| format!("Failed to insert campus: {}", e))?;
    }

    // Insert venues
    let venue_table = format!("venues{}", suffix);
    for venue in &data.venues {
        tx.execute(
            &format!(
                "INSERT INTO {} (id, campus_id, name, capacity) VALUES (?, ?, ?, ?)",
                venue_table
            ),
            params![venue.id, venue.campus_id, venue.name, venue.capacity],
        )
        .map_err(|e| format!("Failed to insert venue: {}", e))?;
    }

    // Insert courses
    let course_table = format!("courses{}", suffix);
    for course in &data.courses {
        tx.execute(
            &format!("INSERT INTO {} (id, name) VALUES (?, ?)", course_table),
            params![course.id, course.name],
        )
        .map_err(|e| format!("Failed to insert course: {}", e))?;
    }

    // Insert course venues
    let course_venue_table = format!("course_venues{}", suffix);
    for cv in &data.course_venues {
        tx.execute(
            &format!(
                "INSERT INTO {} (course_id, venue_id) VALUES (?, ?)",
                course_venue_table
            ),
            params![cv.course_id, cv.venue_id],
        )
        .map_err(|e| format!("Failed to insert course venue: {}", e))?;
    }

    // Insert teachers
    let teacher_table = format!("teachers{}", suffix);
    for teacher in &data.teachers {
        tx.execute(
            &format!(
                "INSERT INTO {} (id, name, max_teaching_hours, is_only_shahe) VALUES (?, ?, ?, ?)",
                teacher_table
            ),
            params![
                teacher.id,
                teacher.name,
                teacher.max_teaching_hours,
                if teacher.is_only_shahe { 1 } else { 0 }
            ],
        )
        .map_err(|e| format!("Failed to insert teacher: {}", e))?;
    }

    // Insert teacher courses
    let teacher_course_table = format!("teacher_courses{}", suffix);
    for tc in &data.teacher_courses {
        tx.execute(
            &format!(
                "INSERT INTO {} (teacher_id, course_id) VALUES (?, ?)",
                teacher_course_table
            ),
            params![tc.teacher_id, tc.course_id],
        )
        .map_err(|e| format!("Failed to insert teacher course: {}", e))?;
    }

    // Insert teacher unavailability
    let unavail_table = format!("teacher_unavailability{}", suffix);
    for unavail in &data.teacher_unavailability {
        tx.execute(
            &format!(
                "INSERT INTO {} (teacher_id, day_id, time_id) VALUES (?, ?, ?)",
                unavail_table
            ),
            params![unavail.teacher_id, unavail.day_id, unavail.time_id],
        )
        .map_err(|e| format!("Failed to insert unavailability: {}", e))?;
    }

    // Insert scheduled classes
    let scheduled_table = format!("scheduled_classes{}", suffix);
    for scheduled in &data.scheduled_classes {
        tx.execute(
            &format!(
                "INSERT INTO {} (id, teacher_id, course_id, day_id, time_id, campus_id, venue_id) VALUES (?, ?, ?, ?, ?, ?, ?)",
                scheduled_table
            ),
            params![
                scheduled.id,
                scheduled.teacher_id,
                scheduled.course_id,
                scheduled.day_id,
                scheduled.time_id,
                scheduled.campus_id,
                scheduled.venue_id
            ],
        )
        .map_err(|e| format!("Failed to insert scheduled class: {}", e))?;
    }

    // Insert schedule density
    let density_table = format!("schedule_density{}", suffix);
    for density in &data.schedule_density {
        tx.execute(
            &format!(
                "INSERT INTO {} (campus_id, day_id, time_id, count) VALUES (?, ?, ?, ?)",
                density_table
            ),
            params![
                density.campus_id,
                density.day_id,
                density.time_id,
                density.count
            ],
        )
        .map_err(|e| format!("Failed to insert schedule density: {}", e))?;
    }

    Ok(())
}

/// Clear all tables (either main or temp) in reverse order to respect foreign keys
fn clear_tables(tx: &Transaction, is_temp: bool) -> CommandResult<()> {
    let suffix = if is_temp { "_temp" } else { "" };

    // Delete in reverse order to respect foreign key constraints
    let tables = [
        format!("scheduled_classes{}", suffix),
        format!("teacher_unavailability{}", suffix),
        format!("teacher_courses{}", suffix),
        format!("teachers{}", suffix),
        format!("course_venues{}", suffix),
        format!("courses{}", suffix),
        format!("schedule_density{}", suffix),
        format!("venues{}", suffix),
        format!("campuses{}", suffix),
        format!("days{}", suffix),
        format!("time_slots{}", suffix),
    ];

    for table in &tables {
        tx.execute(&format!("DELETE FROM {}", table), [])
            .map_err(|e| format!("Failed to clear table {}: {}", table, e))?;
    }

    Ok(())
}

/// Commit temp tables to main tables
fn commit_temp_to_main(tx: &Transaction) -> CommandResult<()> {
    info!("Starting safe commit sync");

    tx.execute(
        "DELETE FROM teachers WHERE id NOT IN (SELECT id FROM teachers_temp)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute("UPDATE teachers SET 
                name = (SELECT name FROM teachers_temp WHERE teachers_temp.id = teachers.id),
                max_teaching_hours = (SELECT max_teaching_hours FROM teachers_temp WHERE teachers_temp.id = teachers.id),
                is_only_shahe = (SELECT is_only_shahe FROM teachers_temp WHERE teachers_temp.id = teachers.id)
                WHERE id IN (SELECT id FROM teachers_temp)", [])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO teachers (id, name, max_teaching_hours, is_only_shahe) 
                SELECT id, name, max_teaching_hours, is_only_shahe FROM teachers_temp 
                WHERE id NOT IN (SELECT id FROM teachers)",
        [],
    )
    .map_err(|e| e.to_string())?;

    tx.execute(
        "DELETE FROM courses WHERE id NOT IN (SELECT id FROM courses_temp)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE courses SET 
                name = (SELECT name FROM courses_temp WHERE courses_temp.id = courses.id)
                WHERE id IN (SELECT id FROM courses_temp)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO courses (id, name) 
                SELECT id, name FROM courses_temp 
                WHERE id NOT IN (SELECT id FROM courses)",
        [],
    )
    .map_err(|e| e.to_string())?;

    tx.execute(
        "DELETE FROM campuses WHERE id NOT IN (SELECT id FROM campuses_temp)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE campuses SET 
                name = (SELECT name FROM campuses_temp WHERE campuses_temp.id = campuses.id)
                WHERE id IN (SELECT id FROM campuses_temp)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO campuses (id, name) 
                SELECT id, name FROM campuses_temp 
                WHERE id NOT IN (SELECT id FROM campuses)",
        [],
    )
    .map_err(|e| e.to_string())?;

    tx.execute(
        "DELETE FROM venues WHERE id NOT IN (SELECT id FROM venues_temp)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE venues SET 
                name = (SELECT name FROM venues_temp WHERE venues_temp.id = venues.id),
                campus_id = (SELECT campus_id FROM venues_temp WHERE venues_temp.id = venues.id),
                capacity = (SELECT capacity FROM venues_temp WHERE venues_temp.id = venues.id)
                WHERE id IN (SELECT id FROM venues_temp)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO venues (id, campus_id, name, capacity) 
                SELECT id, campus_id, name, capacity FROM venues_temp 
                WHERE id NOT IN (SELECT id FROM venues)",
        [],
    )
    .map_err(|e| e.to_string())?;

    let other_tables = [
        ("time_slots", "time_slots_temp"),
        ("days", "days_temp"),
        ("venues", "venues_temp"),
        ("course_venues", "course_venues_temp"),
        ("teacher_courses", "teacher_courses_temp"),
        ("teacher_unavailability", "teacher_unavailability_temp"),
        ("scheduled_classes", "scheduled_classes_temp"),
        ("schedule_density", "schedule_density_temp"),
    ];

    for (main, temp) in &other_tables {
        tx.execute(&format!("DELETE FROM {}", main), [])
            .map_err(|e| e.to_string())?;
        tx.execute(&format!("INSERT INTO {} SELECT * FROM {}", main, temp), [])
            .map_err(|e| e.to_string())?;
    }

    tx.execute("INSERT INTO audit_logs SELECT * FROM audit_logs_temp", [])
        .map_err(|e| format!("Failed to commit audit logs: {}", e))?;

    tx.execute("DELETE FROM audit_logs_temp", [])
        .map_err(|e| e.to_string())?;

    truncate_all_temp_tables(tx)?;

    info!("Commit completed successfully");
    Ok(())
}

/// Truncate all temp tables (delete all rows but keep table structure)
/// Called after commit to ensure temp tables are empty
fn truncate_all_temp_tables(tx: &Transaction) -> CommandResult<()> {
    let temp_tables = [
        "scheduled_classes_temp",
        "teacher_unavailability_temp",
        "teacher_courses_temp",
        "teachers_temp",
        "course_venues_temp",
        "courses_temp",
        "schedule_density_temp",
        "venues_temp",
        "campuses_temp",
        "days_temp",
        "time_slots_temp",
    ];

    for table in &temp_tables {
        tx.execute(&format!("DELETE FROM {}", table), [])
            .map_err(|e| {
                error!("Failed to truncate temp table {}: {}", table, e);
                format!("Failed to truncate temp table {}: {}", table, e)
            })?;
        info!("Truncated temp table: {}", table);
    }

    Ok(())
}

/// Clear all temp tables
fn clear_all_temp_tables(conn: &Connection, user_id: Option<&str>) -> CommandResult<()> {
    let temp_tables = [
        "scheduled_classes_temp",
        "teacher_unavailability_temp",
        "teacher_courses_temp",
        "teachers_temp",
        "course_venues_temp",
        "courses_temp",
        "schedule_density_temp",
        "venues_temp",
        "campuses_temp",
        "days_temp",
        "time_slots_temp",
        "audit_logs_temp",
    ];

    for table in &temp_tables {
        conn.execute(&format!("DELETE FROM {}", table), [])
            .map_err(|e| format!("Failed to clear temp table {}: {}", table, e))?;
    }

    // Create audit log for revert operation (Feature: 001-rbac-audit-system - User Story 6)
    if let Some(uid) = user_id {
        let details = json!({
            "target_name": "批量撤销",
            "action": "撤销更改",
            "description": "已清空临时表，还原至上次提交状态"
        });
        if let Err(e) = crate::audit::create_audit_log_entry(
            conn,
            uid,
            "REVERT_OPERATION",
            None,
            None,
            Some(details),
            None,
            false,
        ) {
            warn!("Failed to create audit log for revert: {}", e);
        }
    }

    Ok(())
}

/// List all users with their roles and associated teachers (Feature: 001-rbac-audit-system - User Story 4)
pub fn list_users(conn: &Connection) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT u.id, u.username, r.name as role, u.role_id, u.teacher_id, 
                    t.name as teacher_name, u.created_at, u.last_login
             FROM users u
             INNER JOIN roles r ON u.role_id = r.id
             LEFT JOIN teachers t ON u.teacher_id = t.id
             ORDER BY u.created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare list_users query: {}", e))?;

    let users: Result<Vec<serde_json::Value>, String> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "username": row.get::<_, String>(1)?,
                "role": row.get::<_, String>(2)?,
                "role_id": row.get::<_, String>(3)?,
                "teacher_id": row.get::<_, Option<String>>(4)?,
                "teacher_name": row.get::<_, Option<String>>(5)?,
                "created_at": row.get::<_, String>(6)?,
                "last_login": row.get::<_, Option<String>>(7)?
            }))
        })
        .map_err(|e| format!("Failed to execute list_users query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to fetch user rows: {}", e));

    users
}

/// Create a new user manually (Feature: 001-rbac-audit-system - User Story 4)
pub fn create_user(
    conn: &Connection,
    username: &str,
    password_hash: &str,
    role_id: &str,
    teacher_id: Option<&str>,
    creator_user_id: &str,
) -> Result<String, String> {
    use uuid::Uuid;

    // Check if username already exists in both main and temp tables
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = ?1)",
            [username],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to check username existence: {}", e))?;

    if exists {
        return Err(format!("Username '{}' already exists", username));
    }

    // Validate role exists
    let role_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM roles WHERE id = ?1)",
            [role_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to validate role: {}", e))?;

    if !role_exists {
        return Err("Role not found".to_string());
    }

    // Validate teacher exists if provided
    if let Some(tid) = teacher_id {
        let teacher_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM teachers WHERE id = ?1)",
                [tid],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to validate teacher: {}", e))?;

        if !teacher_exists {
            return Err("Teacher not found".to_string());
        }
    }

    let role_name: String = conn
        .query_row("SELECT name FROM roles WHERE id = ?1", [role_id], |row| {
            row.get(0)
        })
        .unwrap_or("Unknown".to_string());

    let teacher_name: Option<String> = if let Some(tid) = teacher_id {
        conn.query_row("SELECT name FROM teachers WHERE id = ?1", [tid], |row| {
            row.get(0)
        })
        .ok()
    } else {
        None
    };

    let user_id = Uuid::new_v4().to_string();
    let created_at = Local::now().to_rfc3339();

    conn.execute(
        "INSERT INTO users (id, username, password_hash, role_id, teacher_id, created_at, last_login)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
        rusqlite::params![
            &user_id,
            username,
            password_hash,
            role_id,
            teacher_id,
            &created_at
        ]
    )
    .map_err(|e| format!("Failed to insert user: {}", e))?;

    let mut details = json!({
        "target_name": username,
        "action": "创建用户",
        "username": username,
        "role_name": role_name
    });

    if let Some(tn) = teacher_name {
        details
            .as_object_mut()
            .unwrap()
            .insert("teacher_name".to_string(), json!(tn));
    }

    if let Err(e) = crate::audit::create_audit_log_entry(
        conn,
        creator_user_id,
        "USER_CREATED",
        Some("users"),
        Some(&user_id),
        Some(details),
        None,
        false,
    ) {
        warn!("Failed to create audit log for user creation: {}", e);
    }

    Ok(user_id)
}

pub fn update_user(
    conn: &Connection,
    user_id: &str,
    new_role_id: &str,
    new_teacher_id: Option<&str>,
    current_user_id: &str,
) -> Result<(), String> {
    if user_id == current_user_id {
        return Err("Cannot change your own role".to_string());
    }

    let new_role_name: String = conn
        .query_row(
            "SELECT name FROM roles WHERE id = ?1",
            [new_role_id],
            |row| row.get(0),
        )
        .map_err(|_| "Role not found".to_string())?;

    let new_teacher_name: String = if let Some(tid) = new_teacher_id {
        conn.query_row("SELECT name FROM teachers WHERE id = ?1", [tid], |row| {
            row.get(0)
        })
        .map_err(|_| "Teacher not found".to_string())?
    } else {
        "无".to_string()
    };

    let (target_username, old_role_name, old_teacher_name): (String, String, Option<String>) = conn
        .query_row(
            "SELECT 
                u.username, 
                r.name as role_name, 
                t.name as teacher_name
             FROM users u
             LEFT JOIN roles r ON u.role_id = r.id
             LEFT JOIN teachers t ON u.teacher_id = t.id
             WHERE u.id = ?1",
            [user_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "User not found".to_string())?;
    let old_val = json!({
        "role_name": old_role_name,
        "teacher_name": old_teacher_name.unwrap_or("无".to_string())
    });

    let new_val = json!({
        "role_name": new_role_name,
        "teacher_name": new_teacher_name
    });

    let changes = crate::audit::compute_diff(&old_val, &new_val);

    conn.execute(
        "UPDATE users SET role_id = ?2, teacher_id = ?3 WHERE id = ?1",
        rusqlite::params![user_id, new_role_id, new_teacher_id],
    )
    .map_err(|e| format!("Failed to update user: {}", e))?;

    if !changes.as_array().unwrap().is_empty() {
        let rich_details = json!({
            "target_name": target_username,
            "changes": changes
        });

        let _ = crate::audit::create_audit_log_entry(
            conn,
            current_user_id,
            "USER_UPDATED",
            Some("users"),
            Some(user_id),
            Some(rich_details),
            None,
            false,
        );
    }

    Ok(())
}

pub fn reset_password(
    conn: &Connection,
    user_id: &str,
    new_password_hash: &str,
    admin_user_id: &str,
) -> Result<(), String> {
    let target_username: String = conn
        .query_row(
            "SELECT username FROM users WHERE id = ?1",
            [user_id],
            |row| row.get(0),
        )
        .map_err(|_| "User not found".to_string())?;

    conn.execute(
        "UPDATE users SET password_hash = ?2 WHERE id = ?1",
        params![user_id, new_password_hash],
    )
    .map_err(|e| format!("Failed to reset password: {}", e))?;

    // Create audit log
    let _ = crate::audit::create_audit_log_entry(
        conn,
        admin_user_id,
        "PASSWORD_RESET",
        Some("users"),
        Some(user_id),
        Some(serde_json::json!({"target_name": target_username})),
        None,
        false,
    );

    Ok(())
}

/// Delete user account (Feature: 001-rbac-audit-system - User Story 4)
pub fn delete_user(conn: &Connection, user_id: &str, current_user_id: &str) -> Result<(), String> {
    let target_username: String = conn
        .query_row(
            "SELECT username FROM users WHERE id = ?1",
            [user_id],
            |row| row.get(0),
        )
        .map_err(|_| "User not found")?;

    if user_id == current_user_id {
        return Err("Cannot delete your own account".to_string());
    }

    let role_id: String = conn
        .query_row(
            "SELECT role_id FROM users WHERE id = ?1",
            [user_id],
            |row| row.get(0),
        )
        .map_err(|_| "User not found")?;

    let scheduler_role_id = "00000000-0000-0000-0000-000000000001";

    if role_id == scheduler_role_id {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE role_id = ?1",
                [scheduler_role_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if count <= 1 {
            return Err("Cannot delete the last remaining Scheduler account".to_string());
        }
    }

    conn.execute("DELETE FROM users WHERE id = ?1", [user_id])
        .map_err(|e| format!("Failed to delete user: {}", e))?;

    let _ = crate::audit::create_audit_log_entry(
        conn,
        current_user_id,
        "USER_DELETED",
        Some("users"),
        Some(user_id),
        Some(serde_json::json!({"target_name": target_username})),
        None,
        false,
    );

    Ok(())
}

/// Change own password (Feature: 001-rbac-audit-system - User Story 5)
pub fn change_own_password(
    conn: &Connection,
    user_id: &str,
    old_password: &str,
    new_password_hash: &str,
) -> Result<(), String> {
    use crate::auth::verify_password;

    // Get current password hash
    let current_hash: String = conn
        .query_row(
            "SELECT password_hash FROM users WHERE id = ?1",
            [user_id],
            |row| row.get(0),
        )
        .map_err(|_| format!("User not found"))?;

    // Verify old password
    match verify_password(old_password, &current_hash) {
        Ok(true) => {
            // Password correct, proceed with update
        }
        Ok(false) => {
            return Err("Current password is incorrect".to_string());
        }
        Err(e) => {
            return Err(format!("Password verification failed: {}", e));
        }
    }

    conn.execute(
        "UPDATE users SET password_hash = ?2 WHERE id = ?1",
        params![user_id, new_password_hash],
    )
    .map_err(|e| format!("Failed to update password: {}", e))?;

    // Create audit log
    if let Err(e) = crate::audit::create_audit_log_entry(
        conn,
        user_id,
        "PASSWORD_CHANGED",
        Some("users"),
        Some(user_id),
        None,
        None,
        false,
    ) {
        warn!("Failed to create audit log for password change: {}", e);
    }

    Ok(())
}

/// Query audit logs with pagination and filters (Feature: 001-rbac-audit-system Phase 9)
pub fn query_audit_logs(
    conn: &Connection,
    page: usize,
    per_page: usize,
    filters: &AuditLogFilters,
) -> CommandResult<AuditLogsResponse> {
    info!(
        "Querying audit logs - page: {}, per_page: {}",
        page, per_page
    );

    info!("Parsed Filters: {:?}", filters);

    // Build WHERE clause based on filters
    let mut where_clauses = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref username) = filters.username {
        where_clauses.push("u.username LIKE ?");
        params_vec.push(Box::new(format!("%{}%", username)));
    }

    if let Some(ref action_type) = filters.action_type {
        where_clauses.push("al.action_type = ?");
        params_vec.push(Box::new(action_type.clone()));
    }

    if let Some(ref date_from) = filters.date_from {
        where_clauses.push("DATE(al.timestamp, 'localtime') >= ?");
        params_vec.push(Box::new(date_from.clone()));
    }

    if let Some(ref date_to) = filters.date_to {
        where_clauses.push("DATE(al.timestamp, 'localtime') <= ?");
        params_vec.push(Box::new(date_to.clone()));
    }

    if let Some(ref target_table) = filters.target_table {
        where_clauses.push("al.target_table = ?");
        params_vec.push(Box::new(target_table.clone()));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    // Count total entries
    let count_sql = format!(
        "SELECT COUNT(*) FROM audit_logs al LEFT JOIN users u ON al.user_id = u.id {}",
        where_sql
    );

    let total_entries: usize = {
        let mut stmt = conn
            .prepare(&count_sql)
            .map_err(|e| format!("Failed to prepare count query: {}", e))?;

        let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        stmt.query_row(&params[..], |row| row.get(0))
            .map_err(|e| format!("Failed to count entries: {}", e))?
    };

    let total_pages = (total_entries + per_page - 1) / per_page;
    let offset = (page - 1) * per_page;

    // Query entries with pagination
    let query_sql = format!(
        "SELECT al.id, al.user_id, COALESCE(u.username, '已删除用户'), al.action_type, al.target_table, \
         al.target_id, al.timestamp, al.change_details, al.ip_address \
         FROM audit_logs al \
         LEFT JOIN users u ON al.user_id = u.id \
         {} \
         ORDER BY al.timestamp DESC \
         LIMIT ? OFFSET ?",
        where_sql
    );

    let mut stmt = conn
        .prepare(&query_sql)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    // Add LIMIT and OFFSET to params
    params_vec.push(Box::new(per_page as i64));
    params_vec.push(Box::new(offset as i64));
    let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let entries = stmt
        .query_map(&params[..], |row| {
            Ok(AuditLogEntry {
                id: row.get(0)?,
                user_id: row.get(1)?,
                username: row.get(2)?,
                action_type: row.get(3)?,
                target_table: row.get(4)?,
                target_id: row.get(5)?,
                timestamp: row.get(6)?,
                change_details: row.get(7)?,
                ip_address: row.get(8)?,
            })
        })
        .map_err(|e| format!("Failed to query entries: {}", e))?
        .collect::<SqlResult<Vec<_>>>()
        .map_err(|e| format!("Failed to collect entries: {}", e))?;

    info!(
        "Found {} audit log entries (page {} of {})",
        entries.len(),
        page,
        total_pages
    );

    Ok(AuditLogsResponse {
        entries,
        pagination: PaginationInfo {
            current_page: page,
            per_page,
            total_entries,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
    })
}
