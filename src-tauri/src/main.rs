// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audit;
mod auth;
mod db_handler;
mod import_export;
mod models;
mod single_instance;

use chrono::Local;
use db_handler::AppState;
use models::{AllData, AuditLogFilters, AuditLogsResponse, SessionStore, User};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{command, AppHandle, Emitter, Listener, Manager, State, WindowEvent};
use tauri_plugin_shell::ShellExt;
use tokio::fs as async_fs;
use tracing::{error, info, warn};
use tracing_appender::non_blocking;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use uuid::Uuid;

pub struct ShutdownState {
    is_closing_unconditionally: bool,
}

/// Custom writer that reopens log file if it's deleted
struct ReopenableLogWriter {
    log_path: PathBuf,
    file: Mutex<Option<File>>,
}

impl ReopenableLogWriter {
    fn new(log_path: PathBuf) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        Ok(Self {
            log_path,
            file: Mutex::new(Some(file)),
        })
    }

    fn ensure_file_exists(&self) -> io::Result<()> {
        // Check if file still exists
        if !self.log_path.exists() {
            eprintln!("Log file deleted, recreating: {}", self.log_path.display());
            let new_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_path)?;

            let mut file_guard = self.file.lock().unwrap();
            *file_guard = Some(new_file);
            eprintln!("Log file recreated successfully");
        }
        Ok(())
    }
}

impl Write for ReopenableLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Try to ensure file exists before writing
        if let Err(e) = self.ensure_file_exists() {
            eprintln!("Failed to reopen log file: {}", e);
            return Err(e);
        }

        let mut file_guard = self.file.lock().unwrap();
        if let Some(ref mut file) = *file_guard {
            file.write(buf)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Log file not available",
            ))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut file_guard = self.file.lock().unwrap();
        if let Some(ref mut file) = *file_guard {
            file.flush()
        } else {
            Ok(())
        }
    }
}

/// Initialize file-based logging system with daily rotation
/// Logs are written to %APPDATA%/Roaming/com.tsxb.course-scheduler/logs/
fn init_logging(logs_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Create logs directory if it doesn't exist
    if !logs_dir.exists() {
        fs::create_dir_all(logs_dir)?;
    }

    // Get local date for log file naming
    let date_str = Local::now().format("%Y.%m.%d").to_string();
    let log_filename = format!("course-scheduler-{}.log", date_str);
    let log_path = logs_dir.join(&log_filename);

    eprintln!("Creating log file: {}", log_path.display());

    // Create custom reopenable writer
    let log_writer = ReopenableLogWriter::new(log_path.clone())?;

    // Wrap in non-blocking writer for async writes
    let (non_blocking_writer, _guard) = non_blocking(log_writer);

    // Store guard to prevent it from being dropped (which would stop logging)
    // We'll leak it intentionally since logging needs to persist for app lifetime
    std::mem::forget(_guard);

    // Configure log format: [YYYY-MM-DD HH:MM:SS.mmm] [LEVEL] [module] message
    let file_layer = fmt::layer()
        .with_ansi(false) // No color codes in file
        .with_writer(non_blocking_writer);

    // Use RUST_LOG env var or default to INFO level
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Build subscriber with file output
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .init();

    info!("Logging system initialized");
    info!("Logs directory: {}", logs_dir.display());
    info!("Log file: {}", log_filename);

    Ok(())
}

/// Clean up old log files, keeping minimum 3 most recent
/// Deletes files older than 30 days
fn cleanup_old_logs(logs_dir: &PathBuf) {
    info!("Starting log cleanup");

    let entries = match fs::read_dir(logs_dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Failed to read logs directory for cleanup: {}", e);
            return;
        }
    };

    let mut log_files: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "log")
                .unwrap_or(false)
        })
        .collect();

    // Sort by modification time (newest first)
    log_files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    let now = std::time::SystemTime::now();
    let thirty_days = std::time::Duration::from_secs(30 * 24 * 60 * 60);

    for (idx, entry) in log_files.iter().enumerate() {
        // Keep minimum 3 most recent files
        if idx < 3 {
            continue;
        }

        // Delete files older than 30 days
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(age) = now.duration_since(modified) {
                    if age > thirty_days {
                        match fs::remove_file(entry.path()) {
                            Ok(_) => info!("Deleted old log file: {}", entry.path().display()),
                            Err(e) => warn!(
                                "Failed to delete log file {}: {}",
                                entry.path().display(),
                                e
                            ),
                        }
                    }
                }
            }
        }
    }

    info!("Log cleanup completed");
}

#[command]
async fn record_audit_log(
    action_type: String,
    target_table: Option<String>,
    target_id: Option<String>,
    change_details: Option<serde_json::Value>,
    session_id: String,
    state: State<'_, AppState>,
    sessions: State<'_, SessionStore>,
) -> Result<(), String> {
    let user_id = {
        let sessions_guard = sessions.lock().unwrap();
        sessions_guard.get(&session_id).map(|u| u.id.clone())
    }
    .ok_or("invalid session")?;

    let db = state.db.lock().unwrap();

    crate::audit::create_audit_log_entry(
        &db,
        &user_id,
        &action_type,
        target_table.as_deref(),
        target_id.as_deref(),
        change_details,
        None,
        true,
    )
}

#[command]
async fn authenticate_user(
    username: String,
    password: String,
    state: State<'_, AppState>,
    sessions: State<'_, SessionStore>,
) -> Result<serde_json::Value, String> {
    use crate::audit::create_audit_log_entry;
    use crate::auth::verify_password;

    info!("Authentication attempt for user: {}", username);

    let db = state.db.lock().unwrap();

    // Query user from main users table
    let result: Result<(String, String, String, String, String, Option<String>), _> = db.query_row(
        "SELECT u.id, u.username, u.password_hash, r.name as role, u.role_id, u.teacher_id 
         FROM users u 
         JOIN roles r ON u.role_id = r.id 
         WHERE u.username = ?1",
        params![&username],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?, // role_id
                row.get(5)?, // teacher_id
            ))
        },
    );

    match result {
        Ok((user_id, username, password_hash, role_name, role_id, teacher_id)) => {
            // 修改 2: 接收 role_id
            match verify_password(&password, &password_hash) {
                Ok(true) => {
                    let session_id = Uuid::new_v4().to_string();
                    let user = User {
                        id: user_id.clone(),
                        username: username.clone(),
                        password_hash: None,
                        role_id: role_id.clone(),
                        teacher_id: teacher_id.clone(),
                        created_at: String::new(),
                        last_login: None,
                    };

                    {
                        let mut sessions_guard = sessions.lock().unwrap();
                        sessions_guard.insert(session_id.clone(), user.clone());
                    }

                    let now = Local::now().to_rfc3339();
                    let _ = db.execute(
                        "UPDATE users SET last_login = ?1 WHERE id = ?2",
                        params![&now, &user_id],
                    );

                    let _ = create_audit_log_entry(
                        &db,
                        &user_id,
                        "LOGIN",
                        None,
                        None,
                        Some(serde_json::json!({
                            "target_name": username,
                            "role": role_name
                        })),
                        None,
                        false,
                    );

                    info!("User {} authenticated successfully", username);

                    Ok(serde_json::json!({
                        "session_id": session_id,
                        "user": {
                            "id": user_id,
                            "username": username,
                            "role": role_name,
                            "teacher_id": teacher_id,
                            "last_login": now,
                        }
                    }))
                }
                Ok(false) => {
                    warn!("Invalid password for user: {}", username);
                    let _ = create_audit_log_entry(
                        &db,
                        "00000000-0000-0000-0000-000000000000",
                        "LOGIN_FAILED",
                        None,
                        None,
                        Some(
                            serde_json::json!({"username": username, "reason": "Password incorrect"}),
                        ),
                        None,
                        false,
                    );
                    Err("Invalid username or password".to_string())
                }
                Err(e) => {
                    error!("Password verification error: {}", e);
                    Err("Authentication error".to_string())
                }
            }
        }
        Err(_) => {
            warn!("User not found: {}", username);
            let _ = create_audit_log_entry(
                &db,
                "00000000-0000-0000-0000-000000000000",
                "LOGIN_FAILED",
                None,
                None,
                Some(serde_json::json!({"username": username, "reason": "User not found"})),
                None,
                false,
            );
            Err("Invalid username or password".to_string())
        }
    }
}

#[command]
async fn logout_user(
    session_id: String,
    state: State<'_, AppState>,
    sessions: State<'_, SessionStore>,
) -> Result<bool, String> {
    use crate::audit::create_audit_log_entry;

    info!("Logout attempt for session: {}", session_id);

    // Get user before removing session (for audit log)
    let user_info = {
        let sessions_guard = sessions.lock().unwrap();
        sessions_guard
            .get(&session_id)
            .map(|u| (u.id.clone(), u.username.clone()))
    };

    if let Some((uid, username)) = user_info {
        let db = state.db.lock().unwrap();
        let _ = create_audit_log_entry(
            &db,
            &uid,
            "LOGOUT",
            None,
            None,
            Some(serde_json::json!({
                "target_name": username
            })),
            None,
            false,
        );
    }

    let mut sessions_guard = sessions.lock().unwrap();
    sessions_guard.remove(&session_id);

    info!("Session {} logged out successfully", session_id);
    Ok(true)
}

/// Get current user from session (for session restoration)
#[command]
async fn get_current_user(
    session_id: String,
    state: State<'_, AppState>,
    sessions: State<'_, SessionStore>,
) -> Result<serde_json::Value, String> {
    // Check if session exists
    let user = {
        let sessions_guard = sessions.lock().unwrap();
        sessions_guard.get(&session_id).cloned()
    };

    match user {
        Some(user) => {
            // Fetch full user details from database
            let db = state.db.lock().unwrap();
            let result: Result<(String, String, Option<String>, Option<String>), _> = db.query_row(
                "SELECT u.username, r.name as role, u.teacher_id, u.last_login 
                 FROM users u 
                 JOIN roles r ON u.role_id = r.id 
                 WHERE u.id = ?1",
                params![&user.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            );

            match result {
                Ok((username, role, teacher_id, last_login)) => Ok(serde_json::json!({
                    "id": user.id,
                    "username": username,
                    "role": role,
                    "teacher_id": teacher_id,
                    "last_login": last_login,
                })),
                Err(_) => Err("User not found".to_string()),
            }
        }
        None => Err("Invalid session".to_string()),
    }
}

// ============================================================================
// USER MANAGEMENT COMMANDS (Feature: 001-rbac-audit-system - User Story 4)
// ============================================================================

/// List all users (Scheduler only)
#[command]
async fn list_users(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().unwrap();
    db_handler::list_users(&db)
}

/// Create new user manually (Scheduler only)
#[command]
async fn create_user(
    username: String,
    password: String,
    role_id: String,
    teacher_id: Option<String>,
    session_id: String,
    state: State<'_, AppState>,
    sessions: State<'_, SessionStore>,
) -> Result<String, String> {
    use crate::auth::hash_password;

    // Get current user ID from session
    let sessions_lock = sessions
        .lock()
        .map_err(|_| "Failed to lock sessions".to_string())?;
    let current_user = sessions_lock
        .get(&session_id)
        .ok_or("Invalid session".to_string())?;
    let creator_user_id = current_user.id.clone();
    drop(sessions_lock);

    // Hash password
    let password_hash =
        hash_password(&password).map_err(|e| format!("Failed to hash password: {}", e))?;

    let db = state.db.lock().unwrap();
    db_handler::create_user(
        &db,
        &username,
        &password_hash,
        &role_id,
        teacher_id.as_deref(),
        &creator_user_id,
    )
}

/// Update user (Scheduler only)
#[command]
async fn update_user(
    user_id: String,
    role_id: String,
    teacher_id: Option<String>,
    session_id: String,
    state: State<'_, AppState>,
    sessions: State<'_, SessionStore>,
) -> Result<(), String> {
    // Get current user ID from session
    let sessions_lock = sessions
        .lock()
        .map_err(|_| "Failed to lock sessions".to_string())?;
    let current_user = sessions_lock
        .get(&session_id)
        .ok_or("Invalid session".to_string())?;
    let current_user_id = current_user.id.clone();
    drop(sessions_lock);

    let db = state.db.lock().unwrap();
    db_handler::update_user(
        &db,
        &user_id,
        &role_id,
        teacher_id.as_deref(),
        &current_user_id,
    )
}

/// Reset user password (Scheduler only)
#[command]
async fn reset_password(
    user_id: String,
    new_password: String,
    session_id: String,
    state: State<'_, AppState>,
    sessions: State<'_, SessionStore>,
) -> Result<(), String> {
    use crate::auth::hash_password;

    // Get current user ID from session
    let sessions_lock = sessions
        .lock()
        .map_err(|_| "Failed to lock sessions".to_string())?;
    let current_user = sessions_lock
        .get(&session_id)
        .ok_or("Invalid session".to_string())?;
    let admin_user_id = current_user.id.clone();
    drop(sessions_lock);

    // Hash new password
    let new_password_hash =
        hash_password(&new_password).map_err(|e| format!("Failed to hash password: {}", e))?;

    let db = state.db.lock().unwrap();
    db_handler::reset_password(&db, &user_id, &new_password_hash, &admin_user_id)
}

/// Delete user account (Scheduler only)
#[command]
async fn delete_user(
    user_id: String,
    session_id: String,
    state: State<'_, AppState>,
    sessions: State<'_, SessionStore>,
) -> Result<(), String> {
    // Get current user ID from session
    let sessions_lock = sessions
        .lock()
        .map_err(|_| "Failed to lock sessions".to_string())?;
    let current_user = sessions_lock
        .get(&session_id)
        .ok_or("Invalid session".to_string())?;
    let current_user_id = current_user.id.clone();
    drop(sessions_lock);

    let db = state.db.lock().unwrap();
    db_handler::delete_user(&db, &user_id, &current_user_id)
}

/// Change own password (All users)
#[command]
async fn change_own_password(
    old_password: String,
    new_password: String,
    session_id: String,
    state: State<'_, AppState>,
    sessions: State<'_, SessionStore>,
) -> Result<(), String> {
    use crate::auth::hash_password;

    // Get current user ID from session
    let sessions_lock = sessions
        .lock()
        .map_err(|_| "Failed to lock sessions".to_string())?;
    let current_user = sessions_lock
        .get(&session_id)
        .ok_or("Invalid session".to_string())?;
    let user_id = current_user.id.clone();
    drop(sessions_lock);

    // Hash new password
    let new_password_hash =
        hash_password(&new_password).map_err(|e| format!("Failed to hash password: {}", e))?;

    let db = state.db.lock().unwrap();
    db_handler::change_own_password(&db, &user_id, &old_password, &new_password_hash)
}

/// List audit logs with pagination and filters (Feature: 001-rbac-audit-system Phase 9)
#[command]
async fn list_audit_logs(
    page: Option<usize>,
    per_page: Option<usize>,
    filters: Option<AuditLogFilters>,
    session_id: String,
    app_state: State<'_, AppState>,
    sessions: State<'_, SessionStore>,
) -> Result<AuditLogsResponse, String> {
    info!("Listing audit logs - session: {}", session_id);

    // Verify user is scheduler or admin
    {
        let sessions_lock = sessions
            .lock()
            .map_err(|_| "Failed to lock sessions".to_string())?;

        let user = sessions_lock
            .get(&session_id)
            .ok_or("Invalid session".to_string())?;

        let db = app_state.db.lock().unwrap();

        let role_name: String = db
            .query_row(
                "SELECT name FROM roles WHERE id = ?",
                params![&user.role_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get role: {}", e))?;

        if role_name != "Scheduler" && role_name != "Admin" {
            return Err("Unauthorized: Only Schedulers can view audit logs".to_string());
        }
    }

    let page = page.unwrap_or(1);
    let per_page = per_page.unwrap_or(50).min(100);
    let filters = filters.unwrap_or_default();

    let db = app_state.db.lock().unwrap();
    db_handler::query_audit_logs(&db, page, per_page, &filters)
}

#[command]
fn open_logs_folder(app_handle: AppHandle) -> Result<(), String> {
    let logs_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("logs");

    if !logs_dir.exists() {
        return Err("Logs directory does not exist".to_string());
    }

    let logs_path = logs_dir.to_str().ok_or("Logs path is not valid UTF-8")?;

    std::process::Command::new("explorer")
        .arg(logs_path)
        .spawn()
        .map_err(|e| format!("Failed to open logs folder: {}", e))?;

    info!("Opened logs folder via Explorer");
    Ok(())
}

#[command]
fn has_unsaved_changes(state: State<'_, AppState>) -> bool {
    let db = state.db.lock().unwrap();
    check_has_unsaved(&db)
}

#[command]
async fn run_solver(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    session_id: String,
    sessions: State<'_, SessionStore>,
) -> Result<AllData, String> {
    info!("Solver invoked");

    // Get current user ID for audit logging
    let uid = {
        let sessions_lock = sessions
            .lock()
            .map_err(|_| "Failed to lock sessions".to_string())?;
        sessions_lock
            .get(&session_id)
            .map(|user| user.id.clone())
            .ok_or("Invalid session".to_string())?
    };

    // Load current data from database (temp tables if they exist, else main tables)
    let current_data = db_handler::load_data(app_state.clone()).await?;
    info!(
        "Loaded current data for solver: {} teachers, {} courses",
        current_data.teachers.len(),
        current_data.courses.len()
    );

    // Create temporary JSON file for solver input
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|_| "Could not get app data dir")?;
    let input_path = app_data_dir.join("solver_input.tmp.json");
    let output_path = app_data_dir.join("solver_output.tmp.json");

    // Write current data to input JSON
    async_fs::write(&input_path, serde_json::to_string(&current_data).unwrap())
        .await
        .map_err(|e| format!("Failed to write solver input: {}", e))?;

    let (input_path_str, output_path_str) = (
        input_path
            .to_str()
            .ok_or("Input path is not valid UTF-8")?
            .to_string(),
        output_path
            .to_str()
            .ok_or("Output path is not valid UTF-8")?
            .to_string(),
    );

    info!("Executing solver with input: {}", input_path_str);

    let sidecar_command = app_handle
        .shell()
        .sidecar("solver")
        .unwrap()
        .args([&input_path_str, &output_path_str]);
    let output = sidecar_command.output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        error!(
            "Solver failed with exit code {:?}: {}",
            output.status.code(),
            error_message
        );
        return Err(format!(
            "Solver failed with exit code {:?}:\n{}",
            output.status.code(),
            error_message
        ));
    }

    if !output_path.exists() {
        error!("Solver did not produce output file");
        return Err("Solver ran successfully, but did not produce an output file. Probably it fails to solve the problem.".to_string());
    }

    let result_content = async_fs::read_to_string(&output_path)
        .await
        .map_err(|e| e.to_string())?;

    let solved_data: AllData = serde_json::from_str(&result_content)
        .map_err(|e| format!("Failed to parse solver output: {}", e))?;

    {
        let db = app_state.db.lock().unwrap();
        let _ = crate::audit::create_audit_log_entry(
            &db,
            &uid,
            "SOLVER_RUN",
            None,
            None,
            Some(serde_json::json!({
                "status": "success",
                "message": "自动排课完成"
            })),
            None,
            true,
        );
    }

    db_handler::save_temp_data(solved_data.clone(), app_handle, app_state).await?;

    let _ = async_fs::remove_file(&input_path).await;
    let _ = async_fs::remove_file(&output_path).await;

    Ok(solved_data)
}

/// Check if any of the 11 temp tables contain unsaved data
/// Returns true if any temp table has at least one row
fn check_has_unsaved(db: &Connection) -> bool {
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

    for table in temp_tables.iter() {
        let query = format!("SELECT COUNT(*) FROM {}", table);
        match db.query_row(&query, [], |row| row.get::<_, i64>(0)) {
            Ok(count) => {
                if count > 0 {
                    info!("Found {} unsaved rows in {}", count, table);
                    return true;
                }
            }
            Err(e) => {
                warn!("Failed to check temp table {}: {}", table, e);
                // Continue checking other tables
            }
        }
    }

    false
}

#[command]
async fn finalize_and_close(
    save: bool,
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    shutdown_state: State<'_, Arc<Mutex<ShutdownState>>>,
    session_id: String,
    sessions: State<'_, SessionStore>,
) -> Result<(), String> {
    let has_changes = {
        let db = app_state.db.lock().unwrap();
        check_has_unsaved(&db)
    };
    if save {
        info!("Finalizing with save (committing data)");
        // Commit temp data to main tables
        db_handler::commit_data(
            app_handle.clone(),
            app_state.clone(),
            session_id.clone(),
            sessions.clone(),
        )
        .await?;
        info!("Data committed successfully");
    } else if has_changes {
        info!("Finalizing without save (reverting changes)");
        db_handler::clear_temp_data(app_state.clone(), session_id.clone(), sessions.clone())
            .await?;
        info!("Changes reverted successfully");
    } else {
        info!("Finalizing without save (no changes to revert)");
    }

    let user_info = {
        let sessions_guard = sessions.lock().unwrap();
        sessions_guard
            .get(&session_id)
            .map(|u| (u.id.clone(), u.username.clone()))
    };

    if let Some((uid, username)) = user_info {
        let db = app_state.db.lock().unwrap();
        let _ = crate::audit::create_audit_log_entry(
            &db,
            &uid,
            "LOGOUT",
            None,
            None,
            Some(serde_json::json!({
                "target_name": username,
                "reason": "App Closed"
            })),
            None,
            false,
        );

        drop(db);
        let mut sessions_guard = sessions.lock().unwrap();
        sessions_guard.remove(&session_id);
        info!("Implicit logout performed for user: {}", username);
    }

    {
        let mut state = shutdown_state.lock().unwrap();
        state.is_closing_unconditionally = true;
    }

    info!("Closing application window");
    app_handle
        .get_webview_window("main")
        .unwrap()
        .close()
        .map_err(|e| {
            error!("Failed to close window: {}", e);
            e.to_string()
        })?;

    Ok(())
}

fn main() {
    let shutdown_state = Arc::new(Mutex::new(ShutdownState {
        is_closing_unconditionally: false,
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .manage(shutdown_state.clone())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory.");

            if !data_dir.exists() {
                fs::create_dir_all(&data_dir).expect("Failed to create app data directory.");
            }

            // Initialize logging system FIRST (before any other operations)
            let logs_dir = data_dir.join("logs");
            if let Err(e) = init_logging(&logs_dir) {
                eprintln!("Failed to initialize logging: {}", e);
                // Continue without logging - not critical
            }

            // Clean up old log files
            cleanup_old_logs(&logs_dir);

            info!("Application starting");
            info!("App data directory: {}", data_dir.display());

            // Check for stale lockfile first
            match single_instance::check_stale_lockfile(&data_dir) {
                Ok(was_cleaned) => {
                    if was_cleaned {
                        info!("Stale lockfile was cleaned");
                    } else {
                        info!("No stale lockfile found");
                    }
                }
                Err(msg) => {
                    // Stale lockfile check failed - another instance is actually running
                    error!("Instance already running: {}", msg);
                    eprintln!("Error: {}", msg);
                    std::process::exit(1);
                }
            };

            // Create new lockfile for this instance
            if let Err(msg) = single_instance::check_single_instance(&data_dir) {
                error!("Failed to create lockfile: {}", msg);
                eprintln!("Failed to create lockfile: {}", msg);
                std::process::exit(1);
            }

            // Open or create SQLite database
            let db_path = data_dir.join("course_scheduler.db");
            info!("Opening database at: {}", db_path.display());

            let conn = Connection::open(&db_path)
                .map_err(|e| {
                    error!("无法打开数据库: {}\n数据库文件: {}", e, db_path.display());
                    e
                })
                .expect("Failed to open database");

            // Initialize database schema
            info!("Initializing database schema");
            db_handler::init_database(&conn).expect("Failed to initialize database schema");

            // Run auth migration (seed roles and admin user)
            info!("Running auth migration");
            if let Err(e) = db_handler::run_auth_migration(&conn) {
                warn!("Auth migration failed: {}", e);
                // Non-critical - continue without admin user
            }

            app.manage(AppState {
                db: Mutex::new(conn),
            });

            // Initialize session store for authentication
            let sessions: SessionStore = Mutex::new(HashMap::new());
            app.manage(sessions);

            // Register cleanup handler
            let data_dir_clone = data_dir.clone();
            app_handle.once("tauri://close-requested", move |_| {
                info!("Close requested, cleaning up lockfile");
                single_instance::remove_lock_file(&data_dir_clone);
            });

            // Diagnostic logging for Issue #7 (window creation)
            if let Some(main_window) = app.get_webview_window("main") {
                info!("[STARTUP] Main window created with label: 'main'");
                if let Ok(monitor) = main_window.primary_monitor() {
                    info!("[STARTUP] Primary monitor: {:?}", monitor);
                }
                if let Ok(monitors) = main_window.available_monitors() {
                    let monitor_list: Vec<_> = monitors.into_iter().collect();
                    info!("[STARTUP] Available monitors: {} total", monitor_list.len());
                    for (idx, mon) in monitor_list.iter().enumerate() {
                        info!("[STARTUP] Monitor {}: {:?}", idx, mon);
                    }
                }
            }

            info!("Application setup completed");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            db_handler::load_data,
            db_handler::save_temp_data,
            db_handler::commit_data,
            db_handler::clear_temp_data,
            db_handler::list_committed_teachers,
            finalize_and_close,
            has_unsaved_changes,
            run_solver,
            open_logs_folder,
            authenticate_user,
            logout_user,
            get_current_user,
            list_users,
            create_user,
            update_user,
            reset_password,
            delete_user,
            change_own_password,
            list_audit_logs,
            record_audit_log,
            import_export::import_json,
            import_export::import_database,
            import_export::export_database,
            import_export::export_json
        ])
        .on_window_event(move |window, event| {
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let sessions = window.state::<SessionStore>();
                    let has_active_session = {
                        let sessions_guard = sessions.lock().unwrap();
                        !sessions_guard.is_empty()
                    };
                    let shutdown_state = window.state::<Arc<Mutex<ShutdownState>>>();
                    let mut is_closing = false;

                    if let Ok(state) = shutdown_state.lock() {
                        is_closing = state.is_closing_unconditionally;
                    }

                    if is_closing || !has_active_session {
                        if let Ok(app_data_dir) = window.app_handle().path().app_data_dir() {
                            let _ = single_instance::cleanup_lockfile(&app_data_dir);
                        }
                        return;
                    }

                    api.prevent_close();
                    window.emit("show-close-dialog", ()).unwrap();
                }
                WindowEvent::Destroyed => {
                    // Final cleanup when window is destroyed (T032)
                    info!("[CLOSE] WindowEvent::Destroyed fired");
                    if let Ok(app_data_dir) = window.app_handle().path().app_data_dir() {
                        if let Err(e) = single_instance::cleanup_lockfile(&app_data_dir) {
                            error!("Failed to cleanup lockfile on window destroy: {}", e);
                        }
                    }
                }
                WindowEvent::Focused(focused) => {
                    // Diagnostic logging for window focus changes
                    info!("[WINDOW] Focus changed: {}", focused);
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
