// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db_handler;
mod models;
mod single_instance;
mod import_export;

use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tauri::{command, AppHandle, Emitter, Listener, Manager, State, WindowEvent};
use models::AllData;
use tauri_plugin_shell::ShellExt;
use tokio::fs as async_fs;
use rusqlite::Connection;
use db_handler::AppState;
use tracing::{info, error, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tracing_appender::non_blocking;
use chrono::Local;
use std::path::PathBuf;

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
            Err(io::Error::new(io::ErrorKind::Other, "Log file not available"))
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
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

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
            entry.path().extension()
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
                            Err(e) => warn!("Failed to delete log file {}: {}", entry.path().display(), e),
                        }
                    }
                }
            }
        }
    }

    info!("Log cleanup completed");
}

/// Tauri command to open logs folder in Windows Explorer
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

    let logs_path = logs_dir
        .to_str()
        .ok_or("Logs path is not valid UTF-8")?;

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
) -> Result<AllData, String> {
    info!("Solver invoked");
    
    // Load current data from database (temp tables if they exist, else main tables)
    let current_data = db_handler::load_data(app_state.clone()).await?;
    info!("Loaded current data for solver: {} teachers, {} courses", 
          current_data.teachers.len(), current_data.courses.len());
    
    // Create temporary JSON file for solver input
    let app_data_dir = app_handle.path().app_data_dir().map_err(|_| "Could not get app data dir")?;
    let input_path = app_data_dir.join("solver_input.tmp.json");
    let output_path = app_data_dir.join("solver_output.tmp.json");
    
    // Write current data to input JSON
    let json_content = serde_json::to_string(&current_data)
        .map_err(|e| {
            error!("Failed to serialize data for solver: {}", e);
            format!("Failed to serialize data for solver: {}", e)
        })?;
    async_fs::write(&input_path, json_content)
        .await
        .map_err(|e| {
            error!("Failed to write solver input: {}", e);
            format!("Failed to write solver input: {}", e)
        })?;

    let (input_path_str, output_path_str) = (
        input_path.to_str().ok_or("Input path is not valid UTF-8")?.to_string(),
        output_path.to_str().ok_or("Output path is not valid UTF-8")?.to_string(),
    );

    info!("Executing solver with input: {}", input_path_str);
    
    let sidecar_command = app_handle
        .shell()
        .sidecar("solver")
        .map_err(|e| {
            error!("Failed to create sidecar command: {}", e);
            format!("Failed to create sidecar command: {}", e)
        })?
        .args([&input_path_str, &output_path_str]);

    let output = sidecar_command.output().await.map_err(|e| {
        error!("Failed to execute solver: {}", e);
        format!("Failed to execute solver: {}", e)
    })?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        error!("Solver failed with exit code {:?}: {}", output.status.code(), error_message);
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
        .map_err(|e| {
            error!("Failed to read solver output file: {}", e);
            format!("Failed to read solver output file: {}", e)
        })?;
    
    // Clean up temporary files
    let _ = async_fs::remove_file(&input_path).await;
    let _ = async_fs::remove_file(&output_path).await;

    let solved_data: AllData = serde_json::from_str(&result_content)
        .map_err(|e| {
            error!("Failed to parse solver output JSON: {}", e);
            format!("Failed to parse solver output JSON: {}", e)
        })?;

    info!("Solver completed successfully, saving results to temp tables");
    
    // Save solved data to temp tables (for review/commit)
    db_handler::save_temp_data(solved_data.clone(), app_handle, app_state).await?;

    info!("Solver results saved to temp tables");
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
) -> Result<(), String> {
    if save {
        info!("Finalizing with save (committing data)");
        // Commit temp data to main tables
        db_handler::commit_data(app_handle.clone(), app_state).await?;
        info!("Data committed successfully");
    } else {
        info!("Finalizing without save (reverting changes)");
        // Clear temp data (revert)
        db_handler::clear_temp_data(app_state).await?;
        info!("Changes reverted successfully");
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
                    eprintln!("无法打开数据库: {}\n数据库文件: {}", e, db_path.display());
                    if e.to_string().contains("disk") || e.to_string().contains("space") {
                        error!("磁盘空间不足");
                        eprintln!("\n错误: 磁盘空间不足！请清理磁盘空间后重试。");
                    }
                    e
                })
                .expect("Failed to open database");

            // Initialize database schema
            info!("Initializing database schema");
            db_handler::init_database(&conn)
                .expect("Failed to initialize database schema");

            // Migrate from JSON if needed
            info!("Checking for JSON migration");
            match db_handler::migrate_from_json(&conn, &data_dir) {
                Ok((migrated, stats_opt)) => {
                    if migrated {
                        info!("Successfully migrated data from JSON to SQLite");
                        println!("Successfully migrated data from JSON to SQLite");
                        
                        if let Some(stats) = stats_opt {
                            info!("Migration stats: {} teachers, {} courses, {} schedules",
                                  stats.teachers, stats.courses, stats.schedules);
                        }
                    } else {
                        info!("No JSON migration needed");
                    }
                }
                Err(e) => {
                    error!("Migration failed: {}", e);
                    eprintln!("Warning: Migration failed, starting with empty database: {}", e);
                }
            }

            app.manage(AppState {
                db: Mutex::new(conn),
            });

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
            finalize_and_close,
            has_unsaved_changes,
            run_solver,
            open_logs_folder,
            import_export::import_json,
            import_export::import_database,
            import_export::export_database,
            import_export::export_json
        ])
        .on_window_event(move |window, event| {
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    // Diagnostic logging for Issue #7 (white flash on close)
                    info!("[CLOSE] CloseRequested event fired");
                    if let Ok(position) = window.outer_position() {
                        info!("[CLOSE] Window position: {:?}", position);
                    }
                    if let Ok(size) = window.outer_size() {
                        info!("[CLOSE] Window size: {:?}", size);
                    }
                    if let Ok(monitor) = window.current_monitor() {
                        info!("[CLOSE] Current monitor: {:?}", monitor);
                    }
                    if let Ok(monitors) = window.available_monitors() {
                        let monitor_list: Vec<_> = monitors.into_iter().collect();
                        info!("[CLOSE] Available monitors: {} total", monitor_list.len());
                    }
                    
                    let app_state = window.state::<AppState>();
                    let shutdown_state = window.state::<Arc<Mutex<ShutdownState>>>();
                    let mut is_closing = false;

                    if let Ok(state) = shutdown_state.lock() {
                        is_closing = state.is_closing_unconditionally;
                    }

                    if is_closing {
                        info!("Window close requested - unconditional close");
                        // Clean up lockfile (T032)
                        if let Ok(app_data_dir) = window.app_handle().path().app_data_dir() {
                            info!("Cleaning up lockfile on close");
                            if let Err(e) = single_instance::cleanup_lockfile(&app_data_dir) {
                                error!("Failed to cleanup lockfile on close: {}", e);
                            }
                        }
                        return;
                    }

                    // Check if temp tables have unsaved data
                    let has_unsaved = {
                        let db = app_state.db.lock();
                        match db {
                            Ok(db) => check_has_unsaved(&db),
                            Err(e) => {
                                warn!("Failed to acquire database lock for unsaved check: {}", e);
                                false // Default to no unsaved data if can't access DB
                            }
                        }
                    };
                    
                    if has_unsaved {
                        info!("Window close requested - has unsaved changes, showing dialog");
                        api.prevent_close();
                        window.emit("show-close-dialog", ()).unwrap();
                    } else {
                        info!("Window close requested - no unsaved changes");
                        // Clean up lockfile before allowing close (T032)
                        if let Ok(app_data_dir) = window.app_handle().path().app_data_dir() {
                            info!("Cleaning up lockfile on normal close");
                            if let Err(e) = single_instance::cleanup_lockfile(&app_data_dir) {
                                error!("Failed to cleanup lockfile: {}", e);
                            }
                        }
                    }
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
