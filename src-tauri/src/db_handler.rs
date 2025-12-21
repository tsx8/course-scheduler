// Database handler module for SQLite persistence
// Feature: 001-sqlite-migration
// Replaces file_handler.rs with database operations

use crate::models::*;
use crate::import_export::process_json_format;
use rusqlite::{params, Connection, Result as SqlResult, Transaction};
use std::path::Path;
use std::sync::Mutex;
use tauri::State;
use tracing::{info, error, warn};

// Type alias for Tauri command results
type CommandResult<T> = Result<T, String>;

// AppState structure holding database connection
pub struct AppState {
    pub db: Mutex<Connection>,
}

// ============================================================================
// DATABASE INITIALIZATION
// ============================================================================

/// Initialize database schema by executing schema.sql
/// This creates all main and temp tables with foreign key constraints
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

// ============================================================================
// JSON MIGRATION
// ============================================================================

/// Migrate existing data.json to SQLite database
/// Called on app startup - only runs if database is empty and JSON exists
/// Checks both new location (AppData/Roaming) and old location (AppData/Local)
pub fn migrate_from_json(conn: &Connection, app_data_dir: &Path) -> CommandResult<(bool, Option<MigrationStats>)> {
    info!("Checking for JSON migration");
    // Check if temp tables have data (indicates database already initialized)
    let has_data: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM time_slots UNION SELECT 1 FROM time_slots_temp)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| {
            error!("Failed to check database state: {}", e);
            format!("Failed to check database state: {}", e)
        })?;
    
    if has_data {
        info!("Database already has data, skipping migration");
        return Ok((false, None)); // Skip migration - database already has data
    }
    
    // Multi-path detection: Check new location first, then old location
    let new_json_path = app_data_dir.join("data.json");
    let old_json_path = if let Some(local_dir) = dirs::data_local_dir() {
        local_dir.join("course-scheduler").join("data.json")
    } else {
        std::path::PathBuf::new() // Empty path that won't exist
    };
    
    let (json_path, source_location) = if new_json_path.exists() {
        info!("Found data.json in new location: {}", new_json_path.display());
        (new_json_path, "AppData/Roaming")
    } else if old_json_path.exists() {
        info!("Found data.json in old location: {}", old_json_path.display());
        (old_json_path, "AppData/Local")
    } else {
        info!("No data.json found in any location, skipping migration");
        return Ok((false, None)); // Skip migration - no JSON file to migrate
    };
    
    info!("Starting migration from: {}", json_path.display());
    
    let json_content = std::fs::read_to_string(&json_path)
        .map_err(|e| {
            error!("Failed to read data.json: {}", e);
            format!("Failed to read data.json: {}", e)
        })?;
    
    let json_value: serde_json::Value = serde_json::from_str(&json_content)
        .map_err(|e| {
            error!("Invalid raw JSON format: {}", e);
            format!("Invalid raw JSON format: {}", e)
        })?;

    let normalized_json = process_json_format(json_value);

    let all_data: AllData = serde_json::from_value(normalized_json)
        .map_err(|e| {
            error!("Failed to map normalized JSON to model: {}", e);
            format!("Failed to map normalized JSON to model: {}", e)
        })?;
    
    let teacher_count = all_data.teachers.len();
    let course_count = all_data.courses.len();
    let schedule_count = all_data.scheduled_classes.len();
    
    info!("Parsed JSON: {} teachers, {} courses, {} campuses, {} schedules",
          teacher_count, course_count, all_data.campuses.len(), schedule_count);
    
    // Begin transaction for migration
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| {
            error!("Failed to begin migration transaction: {}", e);
            format!("Failed to begin migration transaction: {}", e)
        })?;
    
    // Import data into MAIN tables only (not temp tables)
    // Temp tables should remain empty after migration
    write_all_data_to_tables(&tx, &all_data, false)?; // Main tables only
    
    tx.commit()
        .map_err(|e| {
            error!("Failed to commit migration: {}", e);
            format!("Failed to commit migration: {}", e)
        })?;
    
    info!("Migration transaction committed");
    
    // Clear temp tables to ensure they're empty after migration
    clear_all_temp_tables(conn)?;
    info!("Temp tables cleared after migration");
    
    // Validate temp tables are empty (post-migration check)
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
    
    let mut total_temp_rows = 0;
    for table in &temp_tables {
        let count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM {}", table),
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        total_temp_rows += count;
    }
    
    if total_temp_rows > 0 {
        error!("Temp tables not empty after migration: {} rows", total_temp_rows);
    } else {
        info!("Post-migration validation passed: temp tables empty");
    }
    
    // Backup original JSON file with timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let backup_filename = format!("data.json.backup-{}", timestamp);
    
    // Create backup in source directory first
    let temp_backup_path = json_path.parent()
        .unwrap_or(app_data_dir)
        .join(&backup_filename);
    
    if let Err(e) = std::fs::rename(&json_path, &temp_backup_path) {
        error!("Failed to backup data.json: {}", e);
        eprintln!("Warning: Failed to backup data.json: {}", e);
        // Continue anyway - migration succeeded
    } else {
        info!("Created temporary backup: {}", temp_backup_path.display());
        
        // Move backup to new app data directory
        let final_backup_path = app_data_dir.join(&backup_filename);
        match std::fs::rename(&temp_backup_path, &final_backup_path) {
            Ok(_) => {
                info!("Backup moved from {} to {}", temp_backup_path.display(), final_backup_path.display());
            }
            Err(e) => {
                // If rename fails (cross-device), try copy + delete
                warn!("Failed to move backup ({}), trying copy+delete", e);
                match std::fs::copy(&temp_backup_path, &final_backup_path) {
                    Ok(_) => {
                        let _ = std::fs::remove_file(&temp_backup_path);
                        info!("Backup copied and moved to: {}", final_backup_path.display());
                    }
                    Err(copy_err) => {
                        warn!("Failed to move backup, keeping in source directory: {}", copy_err);
                        info!("Backup file location: {}", temp_backup_path.display());
                    }
                }
            }
        }
    }
    
    info!("Migration completed successfully");
    
    // Return migration stats
    let stats = MigrationStats {
        teachers: teacher_count,
        courses: course_count,
        schedules: schedule_count,
        source_path: json_path.to_string_lossy().to_string(),
        source_location: source_location.to_string(),
    };
    
    Ok((true, Some(stats))) // Migration completed
}

// ============================================================================
// TAURI COMMANDS (Frontend API)
// ============================================================================

/// Load current application state from database
/// Prioritizes temp tables (working state) over main tables (committed state)
#[tauri::command]
pub async fn load_data(state: State<'_, AppState>) -> CommandResult<AllData> {
    info!("Loading data from database");
    let db = state
        .db
        .lock()
        .map_err(|e| {
            error!("Failed to acquire database lock: {}", e);
            format!("Failed to acquire database lock: {}", e)
        })?;
    
    let data = load_all_data(&db)?;
    info!("Data loaded: {} teachers, {} courses, {} campuses, {} time slots, {} days",
          data.teachers.len(), data.courses.len(), data.campuses.len(), data.time.len(), data.day.len());
    Ok(data)
}

/// Save current application state to temp tables (auto-save target)
#[tauri::command]
pub async fn save_temp_data(
    content: AllData,
    _app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    info!("Saving data to temp tables: {} teachers, {} courses",
          content.teachers.len(), content.courses.len());
    let db = state
        .db
        .lock()
        .map_err(|e| {
            error!("Failed to acquire database lock: {}", e);
            format!("Failed to acquire database lock: {}", e)
        })?;
    
    let tx = db
        .unchecked_transaction()
        .map_err(|e| {
            error!("Failed to begin transaction: {}", e);
            format!("Failed to begin transaction: {}", e)
        })?;
    
    // Clear and write to temp tables
    match write_all_data_to_tables(&tx, &content, true) {
        Ok(_) => {},
        Err(e) => {
            error!("Failed to write data: {}", e);
            return Err(e);
        }
    }
    
    tx.commit()
        .map_err(|e| {
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
    _app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    info!("Committing temp data to main tables");
    let db = state
        .db
        .lock()
        .map_err(|e| {
            error!("Failed to acquire database lock: {}", e);
            format!("Failed to acquire database lock: {}", e)
        })?;
    
    let tx = db
        .unchecked_transaction()
        .map_err(|e| {
            error!("Failed to begin transaction: {}", e);
            format!("Failed to begin transaction: {}", e)
        })?;
    
    commit_temp_to_main(&tx)?;
    
    tx.commit()
        .map_err(|e| {
            error!("Failed to commit data: {}", e);
            format!("Failed to commit data: {}", e)
        })?;
    
    info!("Data committed successfully");
    
    Ok(())
}

/// Clear all temporary changes (revert operation)
#[tauri::command]
pub async fn clear_temp_data(state: State<'_, AppState>) -> CommandResult<()> {
    info!("Clearing all temp tables (revert)");
    let db = state
        .db
        .lock()
        .map_err(|e| {
            error!("Failed to acquire database lock: {}", e);
            format!("Failed to acquire database lock: {}", e)
        })?;
    
    clear_all_temp_tables(&db)?;
    info!("Temp tables cleared successfully");
    
    Ok(())
}

// ============================================================================
// DATABASE OPERATIONS - LOAD
// ============================================================================

/// Load AllData from database, prioritizing temp tables
/// Public function for use by import_export module
pub fn load_all_data_from_connection(conn: &Connection, use_temp: bool) -> CommandResult<AllData> {
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
    })
}

/// Load AllData from database, prioritizing temp tables (internal function)
fn load_all_data(conn: &Connection) -> CommandResult<AllData> {
    // Check if temp tables have data
    let has_temp_data: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM time_slots_temp)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to check temp data: {}", e))?;
    
    let use_temp = has_temp_data;
    
    load_all_data_from_connection(conn, use_temp)
}

fn load_time_slots(conn: &Connection, use_temp: bool) -> CommandResult<Vec<TimeSlot>> {
    let table = if use_temp { "time_slots_temp" } else { "time_slots" };
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
    let table = if use_temp { "campuses_temp" } else { "campuses" };
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
    let table = if use_temp { "course_venues_temp" } else { "course_venues" };
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
    let table = if use_temp { "teachers_temp" } else { "teachers" };
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
    let table = if use_temp { "teacher_courses_temp" } else { "teacher_courses" };
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
    let table = if use_temp { "scheduled_classes_temp" } else { "scheduled_classes" };
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

fn load_teacher_unavailability(conn: &Connection, use_temp: bool) -> CommandResult<Vec<TeacherUnavailability>> {
    let table = if use_temp { "teacher_unavailability_temp" } else { "teacher_unavailability" };
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
    let table = if use_temp { "schedule_density_temp" } else { "schedule_density" };
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
            params![density.campus_id, density.day_id, density.time_id, density.count],
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
    info!("Starting commit: copying temp tables to main");
    
    // Clear main tables
    clear_tables(tx, false)?;
    
    // Copy from temp to main
    let table_pairs = [
        ("time_slots", "time_slots_temp"),
        ("days", "days_temp"),
        ("campuses", "campuses_temp"),
        ("venues", "venues_temp"),
        ("courses", "courses_temp"),
        ("course_venues", "course_venues_temp"),
        ("teachers", "teachers_temp"),
        ("teacher_courses", "teacher_courses_temp"),
        ("teacher_unavailability", "teacher_unavailability_temp"),
        ("scheduled_classes", "scheduled_classes_temp"),
        ("schedule_density", "schedule_density_temp"),
    ];
    
    for (main, temp) in &table_pairs {
        tx.execute(&format!("INSERT INTO {} SELECT * FROM {}", main, temp), [])
            .map_err(|e| {
                error!("Failed to commit {} to main: {}", temp, e);
                format!("Failed to commit {} to main: {}", temp, e)
            })?;
    }
    
    info!("Copy completed, truncating temp tables");
    
    // Truncate temp tables after commit (T024-T025)
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
fn clear_all_temp_tables(conn: &Connection) -> CommandResult<()> {
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
        conn.execute(&format!("DELETE FROM {}", table), [])
            .map_err(|e| format!("Failed to clear temp table {}: {}", table, e))?;
    }
    
    Ok(())
}
