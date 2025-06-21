// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod file_handler;
mod models;

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{command, AppHandle, Manager, State, WindowEvent, Emitter};

pub struct AppState {
    data_path: PathBuf,
    temp_path: PathBuf,
}

pub struct ShutdownState {
    is_closing_unconditionally: bool,
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

    app_handle.get_webview_window("main").unwrap().close().map_err(|e| e.to_string())?;

    Ok(())
}

fn main() {
    let shutdown_state = Arc::new(Mutex::new(ShutdownState {
        is_closing_unconditionally: false,
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .manage(shutdown_state.clone()) // 将状态交给 Tauri 管理
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
            finalize_and_close
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