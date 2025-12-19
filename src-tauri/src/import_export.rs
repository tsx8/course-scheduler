// Import/Export functionality for JSON and SQLite database files
// Feature: 001-sqlite-migration (Issue #6 restoration)

use crate::models::*;
use crate::db_handler::{AppState, write_all_data_to_tables};
use rusqlite::Connection;
use tauri::State;
use tracing::{info, error};
use serde::Serialize;

/// Statistics returned after import operation
#[derive(Serialize, Debug, Clone)]
pub struct ImportStats {
    pub teachers: usize,
    pub courses: usize,
    pub schedules: usize,
}

type CommandResult<T> = Result<T, String>;

/// Import data from JSON file to temp tables (user reviews then commits)
#[tauri::command]
pub async fn import_json(
    file_path: String,
    state: State<'_, AppState>,
) -> CommandResult<ImportStats> {
    info!("Importing JSON from: {}", file_path);
    
    // Read and parse JSON file
    let json_content = std::fs::read_to_string(&file_path)
        .map_err(|e| {
            error!("Failed to read JSON file: {}", e);
            format!("Failed to read JSON file: {}", e)
        })?;
    
    let all_data: AllData = serde_json::from_str(&json_content)
        .map_err(|e| {
            error!("Invalid JSON format: {}", e);
            format!("Invalid JSON format: {}", e)
        })?;
    
    let teacher_count = all_data.teachers.len();
    let course_count = all_data.courses.len();
    let schedule_count = all_data.teachers.iter()
        .map(|t| t.scheduled.len())
        .sum::<usize>();
    
    info!("Parsed JSON: {} teachers, {} courses, {} schedules",
          teacher_count, course_count, schedule_count);
    
    // Write to temp tables (not main tables - user reviews then commits)
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
    
    // Write to temp tables only
    write_all_data_to_tables(&tx, &all_data, true)?;
    
    tx.commit()
        .map_err(|e| {
            error!("Failed to commit import: {}", e);
            format!("Failed to commit import: {}", e)
        })?;
    
    info!("JSON imported successfully to temp tables");
    
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
) -> CommandResult<ImportStats> {
    info!("Importing database from: {}", file_path);
    
    // Open source database connection
    let source_conn = Connection::open(&file_path)
        .map_err(|e| {
            error!("Failed to open source database: {}", e);
            format!("Failed to open source database: {}", e)
        })?;
    
    // Validate schema - check that all required tables exist
    let tables_to_check = [
        "time_slots", "days", "campuses", "venues", "courses",
        "course_venues", "teachers", "teacher_courses",
        "teacher_unavailability", "scheduled_classes", "schedule_density"
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
    let all_data = load_all_data_from_connection(&source_conn, false)?;
    
    let teacher_count = all_data.teachers.len();
    let course_count = all_data.courses.len();
    let schedule_count = all_data.teachers.iter()
        .map(|t| t.scheduled.len())
        .sum::<usize>();
    
    info!("Loaded from source DB: {} teachers, {} courses, {} schedules",
          teacher_count, course_count, schedule_count);
    
    // Write to current database temp tables
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
    
    // Write to temp tables only
    write_all_data_to_tables(&tx, &all_data, true)?;
    
    tx.commit()
        .map_err(|e| {
            error!("Failed to commit import: {}", e);
            format!("Failed to commit import: {}", e)
        })?;
    
    info!("Database imported successfully to temp tables");
    
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
) -> CommandResult<()> {
    info!("Exporting database to: {}", file_path);
    
    // Create new database file
    let target_conn = Connection::open(&file_path)
        .map_err(|e| {
            error!("Failed to create target database: {}", e);
            format!("Failed to create target database: {}", e)
        })?;
    
    // Initialize schema in target database
    let schema_sql = include_str!("../schema.sql");
    target_conn.execute_batch(schema_sql)
        .map_err(|e| {
            error!("Failed to initialize target schema: {}", e);
            format!("Failed to initialize target schema: {}", e)
        })?;
    
    info!("Target database schema initialized");
    
    // Load data from current main tables
    let db = state
        .db
        .lock()
        .map_err(|e| {
            error!("Failed to acquire database lock: {}", e);
            format!("Failed to acquire database lock: {}", e)
        })?;
    
    let all_data = crate::db_handler::load_all_data_from_connection(&*db, false)?;
    
    info!("Loaded data from main tables: {} teachers, {} courses",
          all_data.teachers.len(), all_data.courses.len());
    
    // Write to target database main tables
    let tx = target_conn
        .unchecked_transaction()
        .map_err(|e| {
            error!("Failed to begin target transaction: {}", e);
            format!("Failed to begin target transaction: {}", e)
        })?;
    
    write_all_data_to_tables(&tx, &all_data, false)?;
    
    tx.commit()
        .map_err(|e| {
            error!("Failed to commit export: {}", e);
            format!("Failed to commit export: {}", e)
        })?;
    
    info!("Database exported successfully to: {}", file_path);
    Ok(())
}

/// Export current temp tables to JSON file (working state)
#[tauri::command]
pub async fn export_json(
    file_path: String,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    info!("Exporting JSON to: {}", file_path);
    
    // Load current state from temp tables
    let db = state
        .db
        .lock()
        .map_err(|e| {
            error!("Failed to acquire database lock: {}", e);
            format!("Failed to acquire database lock: {}", e)
        })?;
    
    let all_data = crate::db_handler::load_all_data_from_connection(&*db, true)?;
    
    info!("Loaded temp data: {} teachers, {} courses",
          all_data.teachers.len(), all_data.courses.len());
    
    // Serialize to JSON with pretty formatting
    let json_content = serde_json::to_string_pretty(&all_data)
        .map_err(|e| {
            error!("Failed to serialize data: {}", e);
            format!("Failed to serialize data: {}", e)
        })?;
    
    // Write to file
    std::fs::write(&file_path, json_content)
        .map_err(|e| {
            error!("Failed to write JSON file: {}", e);
            format!("Failed to write JSON file: {}", e)
        })?;
    
    info!("JSON exported successfully to: {}", file_path);
    Ok(())
}

/// Helper function to load AllData from a connection
/// Used by import_database to read from source database
fn load_all_data_from_connection(conn: &Connection, use_temp: bool) -> CommandResult<AllData> {
    // This function needs to be made public in db_handler.rs
    // or we need to replicate the logic here
    crate::db_handler::load_all_data_from_connection(conn, use_temp)
}
