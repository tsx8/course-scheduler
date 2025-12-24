use crate::audit::create_audit_log_entry;
use crate::db_handler::{load_all_data_from_connection, write_all_data_to_tables, AppState};
use crate::models::*;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;
use tracing::{error, info, warn};

#[derive(Serialize, Debug, Clone)]
pub struct ImportStats {
    pub teachers: usize,
    pub courses: usize,
    pub schedules: usize,
}

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub async fn import_json(
    file_path: String,
    state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, Mutex<HashMap<String, User>>>,
) -> CommandResult<ImportStats> {
    let uid = {
        let sessions_lock = sessions.lock().map_err(|_| "Failed to lock sessions")?;
        let u = sessions_lock.get(&session_id).ok_or("Invalid session")?;
        u.id.clone()
    };

    info!("Importing JSON from: {}", file_path);

    let json_content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut all_data: AllData = serde_json::from_str(&json_content)
        .map_err(|e| format!("Invalid JSON format in {}: {}", file_path, e))?;

    all_data.users = Vec::new();
    all_data.roles = Vec::new();

    let teacher_count = all_data.teachers.len();
    let course_count = all_data.courses.len();
    let schedule_count = all_data.scheduled_classes.len();

    info!(
        "Parsed JSON: {} teachers, {} courses, {} schedules",
        teacher_count, course_count, schedule_count
    );

    let db = state.db.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Failed to acquire database lock: {}", e)
    })?;

    let tx = db.unchecked_transaction().map_err(|e| {
        error!("Failed to begin transaction: {}", e);
        format!("Failed to begin transaction: {}", e)
    })?;

    write_all_data_to_tables(&tx, &all_data, true)?;

    tx.commit().map_err(|e| {
        error!("Failed to commit import: {}", e);
        format!("Failed to commit import: {}", e)
    })?;

    info!("JSON imported successfully to temp tables");

    let change_details = serde_json::json!({
        "file_path": file_path,
        "teachers_count": teacher_count,
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

#[tauri::command]
pub async fn import_database(
    file_path: String,
    state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, Mutex<HashMap<String, User>>>,
) -> CommandResult<ImportStats> {
    info!("Importing database from: {}", file_path);

    let source_conn = Connection::open(&file_path).map_err(|e| {
        error!("Failed to open source database: {}", e);
        format!("Failed to open source database: {}", e)
    })?;

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

    let all_data = load_all_data_from_connection(&source_conn, false, false)?;

    let teacher_count = all_data.teachers.len();
    let course_count = all_data.courses.len();
    let schedule_count = all_data.scheduled_classes.len();

    info!(
        "Loaded from source DB: {} teachers, {} courses, {} schedules",
        teacher_count, course_count, schedule_count
    );

    let db = state.db.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Failed to acquire database lock: {}", e)
    })?;

    let tx = db.unchecked_transaction().map_err(|e| {
        error!("Failed to begin transaction: {}", e);
        format!("Failed to begin transaction: {}", e)
    })?;

    write_all_data_to_tables(&tx, &all_data, true)?;

    tx.commit().map_err(|e| {
        error!("Failed to commit import: {}", e);
        format!("Failed to commit import: {}", e)
    })?;

    info!("Database imported successfully to temp tables");

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

#[tauri::command]
pub async fn export_database(
    file_path: String,
    state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, Mutex<HashMap<String, User>>>,
) -> CommandResult<()> {
    info!("Exporting database to: {}", file_path);

    let target_conn = Connection::open(&file_path).map_err(|e| {
        error!("Failed to create target database: {}", e);
        format!("Failed to create target database: {}", e)
    })?;

    let schema_sql = include_str!("../schema.sql");
    target_conn.execute_batch(schema_sql).map_err(|e| {
        error!("Failed to initialize target schema: {}", e);
        format!("Failed to initialize target schema: {}", e)
    })?;

    info!("Target database schema initialized");

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

#[tauri::command]
pub async fn export_json(
    file_path: String,
    state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, Mutex<HashMap<String, User>>>,
) -> CommandResult<()> {
    info!("Exporting JSON to: {}", file_path);

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

    let json_content = serde_json::to_string_pretty(&all_data).map_err(|e| {
        error!("Failed to serialize data: {}", e);
        format!("Failed to serialize data: {}", e)
    })?;

    std::fs::write(&file_path, json_content).map_err(|e| {
        error!("Failed to write JSON file: {}", e);
        format!("Failed to write JSON file: {}", e)
    })?;

    info!("JSON exported successfully to: {}", file_path);

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
