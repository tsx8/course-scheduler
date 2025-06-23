// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod file_handler;
mod models;

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{command, AppHandle, Emitter, Manager, State, WindowEvent};
use models::AllData;
use tauri_plugin_shell::ShellExt;
use tokio::fs as async_fs;

pub struct AppState {
    data_path: PathBuf,
    temp_path: PathBuf,
}

pub struct ShutdownState {
    is_closing_unconditionally: bool,
}

      
#[command]
async fn run_solver(
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
) -> Result<AllData, String> {
    let input_path = if app_state.temp_path.exists() {
        app_state.temp_path.clone()
    } else {
        app_state.data_path.clone()
    };
    if !input_path.exists() {
        let default_data = AllData::default();
        let json_content = serde_json::to_string(&default_data)
            .map_err(|e| format!("Failed to serialize default data: {}", e))?;
        async_fs::write(&input_path, json_content)
            .await
            .map_err(|e| format!("Failed to write default data to {}: {}", input_path.display(), e))?;
    }
    let app_data_dir = app_handle.path().app_data_dir().map_err(|_| "Could not get app data dir")?;
    let output_path = app_data_dir.join("solver_output.tmp.json");

    let (input_path_str, output_path_str) = (
        input_path.to_str().ok_or("Input path is not valid UTF-8")?.to_string(),
        output_path.to_str().ok_or("Output path is not valid UTF-8")?.to_string(),
    );

    let sidecar_command = app_handle
        .shell()
        .sidecar("solver")
        .map_err(|e| format!("Failed to create sidecar command: {}", e))?
        .args([&input_path_str, &output_path_str]);

    let output = sidecar_command.output().await.map_err(|e| format!("Failed to execute solver: {}", e))?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Solver failed with exit code {:?}:\n{}",
            output.status.code(),
            error_message
        ));
    }

    if !output_path.exists() {
        return Err("Solver ran successfully, but did not produce an output file. Probably it fails to solve the problem.".to_string());
    }

    let result_content = async_fs::read_to_string(&output_path)
        .await
        .map_err(|e| format!("Failed to read solver output file: {}", e))?;
    
    async_fs::remove_file(&output_path)
        .await
        .map_err(|e| format!("Failed to clean up solver output file: {}", e))?;

    let solved_data: AllData = serde_json::from_str(&result_content)
        .map_err(|e| format!("Failed to parse solver output JSON: {}", e))?;

    let solved_json = serde_json::to_string_pretty(&solved_data)
        .map_err(|e| format!("Failed to serialize solved data: {}", e))?;
    
    async_fs::write(&app_state.temp_path, solved_json)
        .await
        .map_err(|e| format!("Failed to write solved data to temp file: {}", e))?;

    Ok(solved_data)
}

#[command]
async fn finalize_and_close(
    save: bool,
    app_handle: AppHandle,
    app_state: State<'_, AppState>,
    shutdown_state: State<'_, Arc<Mutex<ShutdownState>>>,
) -> Result<(), String> {
    if save {
        if app_state.temp_path.exists() {
            fs::rename(&app_state.temp_path, &app_state.data_path).map_err(|e| e.to_string())?;
        }
    } else {
        if app_state.temp_path.exists() {
            fs::remove_file(&app_state.temp_path).map_err(|e| e.to_string())?;
        }
    }

    {
        let mut state = shutdown_state.lock().unwrap();
        state.is_closing_unconditionally = true;
    }

    app_handle
        .get_webview_window("main")
        .unwrap()
        .close()
        .map_err(|e| e.to_string())?;

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

            let data_path = data_dir.join("data.json");
            let temp_path = data_dir.join("data.tmp.json");

            app.manage(AppState {
                data_path,
                temp_path,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            file_handler::load_data,
            file_handler::save_temp_data,
            file_handler::commit_data,
            file_handler::clear_temp_data,
            file_handler::import_data,
            file_handler::export_data,
            finalize_and_close,
            run_solver
        ])
        .on_window_event(move |window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                let app_state = window.state::<AppState>();
                let shutdown_state = window.state::<Arc<Mutex<ShutdownState>>>();
                let mut is_closing = false;

                if let Ok(state) = shutdown_state.lock() {
                    is_closing = state.is_closing_unconditionally;
                }

                if is_closing {
                    return;
                }

                if app_state.temp_path.exists() {
                    api.prevent_close();
                    window.emit("show-close-dialog", ()).unwrap();
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
