// Import/Export functionality for JSON and SQLite database files
// Feature: 001-normalize-3nf - Support for legacy JSON format migration

use crate::audit::create_audit_log_entry;
use crate::db_handler::{load_all_data_from_connection, write_all_data_to_tables, AppState};
use crate::models::*;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;
use tracing::{error, info, warn};

/// Statistics returned after import operation
#[derive(Serialize, Debug, Clone)]
pub struct ImportStats {
    pub teachers: usize,
    pub courses: usize,
    pub schedules: usize,
}

type CommandResult<T> = Result<T, String>;

pub fn process_json_format(json: Value) -> Value {
    let is_legacy = json.get("venues").is_none()
        || json
            .get("campuses")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("venues"))
            .is_some();

    if is_legacy {
        info!("Detected legacy JSON format during processing, applying transformation...");
        return transform_legacy_json(json);
    }
    json
}

/// Transform legacy (denormalized) JSON to normalized format
pub(crate) fn transform_legacy_json(mut json: Value) -> Value {
    info!("Transforming legacy JSON format to normalized format");

    // Ensure top-level arrays exist with defaults if missing
    if json.get("venues").is_none() {
        json.as_object_mut()
            .unwrap()
            .insert("venues".to_string(), Value::Array(Vec::new()));
    }
    if json.get("course_venues").is_none() {
        json.as_object_mut()
            .unwrap()
            .insert("course_venues".to_string(), Value::Array(Vec::new()));
    }
    if json.get("teacher_courses").is_none() {
        json.as_object_mut()
            .unwrap()
            .insert("teacher_courses".to_string(), Value::Array(Vec::new()));
    }
    if json.get("teacher_unavailability").is_none() {
        json.as_object_mut().unwrap().insert(
            "teacher_unavailability".to_string(),
            Value::Array(Vec::new()),
        );
    }
    if json.get("scheduled_classes").is_none() {
        json.as_object_mut()
            .unwrap()
            .insert("scheduled_classes".to_string(), Value::Array(Vec::new()));
    }
    if json.get("schedule_density").is_none() {
        json.as_object_mut()
            .unwrap()
            .insert("schedule_density".to_string(), Value::Array(Vec::new()));
    }

    // Extract venues from campuses
    if let Some(campuses) = json.get("campuses").and_then(|c| c.as_array()).cloned() {
        let mut all_venues = Vec::new();
        let mut all_density = Vec::new();
        let mut normalized_campuses = Vec::new();

        for campus in campuses {
            let campus_id = campus.get("id").and_then(|v| v.as_str()).unwrap_or("");

            // Extract venues with campus_id
            if let Some(venues) = campus.get("venues").and_then(|v| v.as_array()) {
                for venue in venues {
                    let mut v = venue.clone();
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert(
                            "campus_id".to_string(),
                            Value::String(campus_id.to_string()),
                        );
                        all_venues.push(Value::Object(obj.clone()));
                    }
                }
            }

            // Extract schedule_density with campus_id
            if let Some(density) = campus.get("schedule_density").and_then(|v| v.as_array()) {
                for d in density {
                    let mut sd = d.clone();
                    if let Some(obj) = sd.as_object_mut() {
                        obj.insert(
                            "campus_id".to_string(),
                            Value::String(campus_id.to_string()),
                        );
                        all_density.push(Value::Object(obj.clone()));
                    }
                }
            }

            // Keep only id and name
            let c = serde_json::json!({
                "id": campus_id,
                "name": campus.get("name").unwrap_or(&Value::String("".to_string()))
            });
            normalized_campuses.push(c);
        }

        json.as_object_mut()
            .unwrap()
            .insert("campuses".to_string(), Value::Array(normalized_campuses));

        // Merge with existing venues if any
        if !all_venues.is_empty() {
            if let Some(existing_venues) = json.get_mut("venues").and_then(|v| v.as_array_mut()) {
                existing_venues.extend(all_venues);
            } else {
                json.as_object_mut()
                    .unwrap()
                    .insert("venues".to_string(), Value::Array(all_venues));
            }
        }

        if !all_density.is_empty() {
            if let Some(existing_density) = json
                .get_mut("schedule_density")
                .and_then(|v| v.as_array_mut())
            {
                existing_density.extend(all_density);
            } else {
                json.as_object_mut()
                    .unwrap()
                    .insert("schedule_density".to_string(), Value::Array(all_density));
            }
        }
    }

    // Extract course_venues from courses
    if let Some(courses) = json.get("courses").and_then(|c| c.as_array()).cloned() {
        let mut all_course_venues = Vec::new();
        let mut normalized_courses = Vec::new();

        for course in courses {
            let course_id = course.get("id").and_then(|v| v.as_str()).unwrap_or("");

            // Extract places (legacy field name) or place
            let places = course
                .get("places")
                .or_else(|| course.get("place"))
                .and_then(|v| v.as_array());

            if let Some(places) = places {
                for place in places {
                    all_course_venues.push(serde_json::json!({
                        "course_id": course_id,
                        "venue_id": place.get("venue_id").unwrap_or(&Value::String("".to_string()))
                    }));
                }
            }

            normalized_courses.push(serde_json::json!({
                "id": course_id,
                "name": course.get("name").unwrap_or(&Value::String("".to_string()))
            }));
        }

        json.as_object_mut()
            .unwrap()
            .insert("courses".to_string(), Value::Array(normalized_courses));

        if !all_course_venues.is_empty() {
            if let Some(existing_cv) = json.get_mut("course_venues").and_then(|v| v.as_array_mut())
            {
                existing_cv.extend(all_course_venues);
            } else {
                json.as_object_mut()
                    .unwrap()
                    .insert("course_venues".to_string(), Value::Array(all_course_venues));
            }
        }
    }

    // Extract relationships from teachers
    if let Some(teachers) = json.get("teachers").and_then(|t| t.as_array()).cloned() {
        let mut all_teacher_courses = Vec::new();
        let mut all_unavailability = Vec::new();
        let mut all_scheduled = Vec::new(); // 这里将存储处理后的已排课信息
        let mut normalized_teachers = Vec::new();

        for teacher in teachers {
            let teacher_id = teacher.get("id").and_then(|v| v.as_str()).unwrap_or("");

            if let Some(teaches) = teacher.get("teaches").and_then(|v| v.as_array()) {
                for course_id in teaches {
                    all_teacher_courses.push(serde_json::json!({
                        "teacher_id": teacher_id,
                        "course_id": course_id
                    }));
                }
            }

            if let Some(unavail) = teacher.get("unavailable").and_then(|v| v.as_array()) {
                for u in unavail {
                    let mut ua = u.clone();
                    if let Some(obj) = ua.as_object_mut() {
                        obj.insert(
                            "teacher_id".to_string(),
                            Value::String(teacher_id.to_string()),
                        );
                        all_unavailability.push(Value::Object(obj.clone()));
                    }
                }
            }

            if let Some(sched) = teacher.get("scheduled").and_then(|v| v.as_array()) {
                for s in sched {
                    let mut s_item = s.clone();
                    if let Some(obj) = s_item.as_object_mut() {
                        obj.insert(
                            "teacher_id".to_string(),
                            Value::String(teacher_id.to_string()),
                        );
                        all_scheduled.push(Value::Object(obj.clone()));
                    }
                }
            }

            normalized_teachers.push(serde_json::json!({
                "id": teacher_id,
                "name": teacher.get("name").unwrap_or(&Value::String("".to_string())),
                "max_teaching_hours": teacher.get("max_teaching_hours").unwrap_or(&Value::Number(40.into())),
                "is_only_shahe": teacher.get("is_only_shahe").unwrap_or(&Value::Bool(false))
            }));
        }

        let root = json.as_object_mut().unwrap();
        root.insert("teachers".to_string(), Value::Array(normalized_teachers));
        root.insert(
            "teacher_courses".to_string(),
            Value::Array(all_teacher_courses),
        );
        root.insert(
            "teacher_unavailability".to_string(),
            Value::Array(all_unavailability),
        );
        root.insert("scheduled_classes".to_string(), Value::Array(all_scheduled));
    }

    info!("Legacy JSON transformation completed");
    json
}

/// Import data from JSON file to temp tables (user reviews then commits)
#[tauri::command]
pub async fn import_json(
    file_path: String,
    state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, Mutex<HashMap<String, User>>>,
) -> CommandResult<ImportStats> {
    info!("Importing JSON from: {}", file_path);

    let json_content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let json_value: Value =
        serde_json::from_str(&json_content).map_err(|e| format!("Invalid JSON: {}", e))?;

    let json_value = process_json_format(json_value);

    let mut all_data: AllData = serde_json::from_value(json_value)
        .map_err(|e| format!("Invalid JSON format in {}: {}", file_path, e))?;

    all_data.users = Vec::new();
    all_data.roles = Vec::new();

    let teacher_count = all_data.teachers.len();
    let course_count = all_data.courses.len();
    let schedule_count = all_data.scheduled_classes.len();

    let uid = {
        let sessions_lock = sessions.lock().map_err(|_| "Failed to lock sessions")?;
        sessions_lock.get(&session_id).map(|u| u.id.clone())
    }
    .ok_or("Invalid session or user not found")?;

    info!(
        "Parsed JSON: {} teachers, {} courses, {} schedules",
        teacher_count, course_count, schedule_count
    );

    // Write to temp tables (not main tables - user reviews then commits)
    let db = state.db.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Failed to acquire database lock: {}", e)
    })?;

    let tx = db.unchecked_transaction().map_err(|e| {
        error!("Failed to begin transaction: {}", e);
        format!("Failed to begin transaction: {}", e)
    })?;

    // Write to temp tables only
    write_all_data_to_tables(&tx, &all_data, true)?;

    tx.commit().map_err(|e| {
        error!("Failed to commit import: {}", e);
        format!("Failed to commit import: {}", e)
    })?;

    info!("JSON imported successfully to temp tables");

    let change_details = serde_json::json!({
        "file_path": file_path,
        "teachers_count": all_data.teachers.len(),
    });

    let _ = create_audit_log_entry(
        &*db,
        &uid,
        "DATA_IMPORTED",
        Some("all_tables"),
        None,
        Some(change_details),
        None,
        true,
    );

    Ok(ImportStats {
        teachers: teacher_count,
        courses: course_count,
        schedules: schedule_count,
    })
}

/// Import data from another SQLite database file to temp tables
#[tauri::command]
pub async fn import_database(
    file_path: String,
    state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, Mutex<HashMap<String, User>>>,
) -> CommandResult<ImportStats> {
    info!("Importing database from: {}", file_path);

    // Open source database connection
    let source_conn = Connection::open(&file_path).map_err(|e| {
        error!("Failed to open source database: {}", e);
        format!("Failed to open source database: {}", e)
    })?;

    // Validate schema - check that all required tables exist
    let tables_to_check = [
        "time_slots",
        "days",
        "campuses",
        "venues",
        "courses",
        "course_venues",
        "teachers",
        "teacher_courses",
        "teacher_unavailability",
        "scheduled_classes",
        "schedule_density",
    ];

    for table in &tables_to_check {
        let exists: bool = source_conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)",
                [table],
                |row| row.get(0),
            )
            .map_err(|e| {
                error!("Failed to check for table {}: {}", table, e);
                format!("Failed to check for table {}: {}", table, e)
            })?;

        if !exists {
            error!("Source database missing required table: {}", table);
            return Err(format!("Source database missing required table: {}", table));
        }
    }

    info!("Source database schema validated");

    // Read all data from source database main tables and convert to AllData
    let all_data = load_all_data_from_connection(&source_conn, false, false)?;

    let teacher_count = all_data.teachers.len();
    let course_count = all_data.courses.len();
    let schedule_count = all_data.scheduled_classes.len();

    info!(
        "Loaded from source DB: {} teachers, {} courses, {} schedules",
        teacher_count, course_count, schedule_count
    );

    // Write to current database temp tables
    let db = state.db.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Failed to acquire database lock: {}", e)
    })?;

    let tx = db.unchecked_transaction().map_err(|e| {
        error!("Failed to begin transaction: {}", e);
        format!("Failed to begin transaction: {}", e)
    })?;

    // Write to temp tables only
    write_all_data_to_tables(&tx, &all_data, true)?;

    tx.commit().map_err(|e| {
        error!("Failed to commit import: {}", e);
        format!("Failed to commit import: {}", e)
    })?;

    info!("Database imported successfully to temp tables");

    // Create audit log for database import
    if let Ok(sessions_lock) = sessions.lock() {
        if let Some(uid) = sessions_lock.get(&session_id).map(|u| u.id.clone()) {
            let change_details = serde_json::json!({
                "file_path": file_path,
                "teachers_count": teacher_count,
                "courses_count": course_count,
                "schedules_count": schedule_count
            });

            if let Err(e) = create_audit_log_entry(
                &*db,
                &uid,
                "DATA_IMPORTED",
                Some("all_tables"),
                None,
                Some(change_details),
                None,
                true,
            ) {
                warn!("Failed to create audit log for database import: {}", e);
            }
        }
    }

    Ok(ImportStats {
        teachers: teacher_count,
        courses: course_count,
        schedules: schedule_count,
    })
}

/// Export current main tables to a new SQLite database file
#[tauri::command]
pub async fn export_database(
    file_path: String,
    state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, Mutex<HashMap<String, User>>>,
) -> CommandResult<()> {
    info!("Exporting database to: {}", file_path);

    // Create new database file
    let target_conn = Connection::open(&file_path).map_err(|e| {
        error!("Failed to create target database: {}", e);
        format!("Failed to create target database: {}", e)
    })?;

    // Initialize schema in target database
    let schema_sql = include_str!("../schema.sql");
    target_conn.execute_batch(schema_sql).map_err(|e| {
        error!("Failed to initialize target schema: {}", e);
        format!("Failed to initialize target schema: {}", e)
    })?;

    info!("Target database schema initialized");

    // Load data from current main tables
    let db = state.db.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Failed to acquire database lock: {}", e)
    })?;

    let all_data = crate::db_handler::load_all_data_from_connection(&*db, false, false)?;

    info!(
        "Loaded data from main tables: {} teachers, {} courses",
        all_data.teachers.len(),
        all_data.courses.len()
    );

    // Write to target database main tables
    let tx = target_conn.unchecked_transaction().map_err(|e| {
        error!("Failed to begin target transaction: {}", e);
        format!("Failed to begin target transaction: {}", e)
    })?;

    write_all_data_to_tables(&tx, &all_data, false)?;

    tx.commit().map_err(|e| {
        error!("Failed to commit export: {}", e);
        format!("Failed to commit export: {}", e)
    })?;

    info!("Database exported successfully to: {}", file_path);

    // Create audit log for database export
    if let Ok(sessions_lock) = sessions.lock() {
        if let Some(uid) = sessions_lock.get(&session_id).map(|u| u.id.clone()) {
            let change_details = serde_json::json!({
                "file_path": file_path,
                "teachers_count": all_data.teachers.len(),
                "courses_count": all_data.courses.len(),
                "schedules_count": all_data.scheduled_classes.len()
            });

            if let Err(e) = create_audit_log_entry(
                &*db,
                &uid,
                "DATA_EXPORTED",
                Some("all_tables"),
                None,
                Some(change_details),
                None,
                false,
            ) {
                warn!("Failed to create audit log for database export: {}", e);
            }
        }
    }

    Ok(())
}

/// Export current temp tables to JSON file (working state)
#[tauri::command]
pub async fn export_json(
    file_path: String,
    state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, Mutex<HashMap<String, User>>>,
) -> CommandResult<()> {
    info!("Exporting JSON to: {}", file_path);

    // Load current state from temp tables
    let db = state.db.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Failed to acquire database lock: {}", e)
    })?;

    let all_data = crate::db_handler::load_all_data_from_connection(&*db, false, false)?;

    info!(
        "Loaded temp data: {} teachers, {} courses",
        all_data.teachers.len(),
        all_data.courses.len()
    );

    // Serialize to JSON with pretty formatting
    let json_content = serde_json::to_string_pretty(&all_data).map_err(|e| {
        error!("Failed to serialize data: {}", e);
        format!("Failed to serialize data: {}", e)
    })?;

    // Write to file
    std::fs::write(&file_path, json_content).map_err(|e| {
        error!("Failed to write JSON file: {}", e);
        format!("Failed to write JSON file: {}", e)
    })?;

    info!("JSON exported successfully to: {}", file_path);

    // Create audit log for JSON export
    if let Ok(sessions_lock) = sessions.lock() {
        if let Some(uid) = sessions_lock.get(&session_id).map(|u| u.id.clone()) {
            let change_details = serde_json::json!({
                "file_path": file_path,
                "teachers_count": all_data.teachers.len(),
                "courses_count": all_data.courses.len(),
                "schedules_count": all_data.scheduled_classes.len()
            });

            if let Err(e) = create_audit_log_entry(
                &*db,
                &uid,
                "DATA_EXPORTED",
                Some("all_tables"),
                None,
                Some(change_details),
                None,
                false,
            ) {
                warn!("Failed to create audit log for JSON export: {}", e);
            }
        }
    }

    Ok(())
}
