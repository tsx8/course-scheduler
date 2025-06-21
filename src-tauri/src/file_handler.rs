use super::models::AllData;
use super::AppState;
use std::path::Path;
use tauri::{command, State};
use tokio::fs as async_fs;

type CommandResult<T> = Result<T, String>;

#[command]
pub async fn load_data(state: State<'_, AppState>) -> CommandResult<AllData> {
    let path_to_load = if state.temp_path.exists() {
        &state.temp_path
    } else {
        &state.data_path
    };

    if !path_to_load.exists() {
        return Ok(AllData::default());
    }

    let content = async_fs::read_to_string(path_to_load)
        .await
        .map_err(|e| e.to_string())?;

    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[command]
pub async fn save_temp_data(content: AllData, state: State<'_, AppState>) -> CommandResult<()> {
    let json_content = serde_json::to_string_pretty(&content).map_err(|e| e.to_string())?;
    async_fs::write(&state.temp_path, json_content)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn commit_data(state: State<'_, AppState>) -> CommandResult<()> {
    if state.temp_path.exists() {
        async_fs::rename(&state.temp_path, &state.data_path)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[command]
pub async fn clear_temp_data(state: State<'_, AppState>) -> CommandResult<()> {
    if state.temp_path.exists() {
        async_fs::remove_file(&state.temp_path)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[command]
pub async fn import_data(file_path: String, state: State<'_, AppState>) -> CommandResult<AllData> {
    let content = async_fs::read_to_string(&file_path)
        .await
        .map_err(|e| e.to_string())?;
    let data: AllData = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let json_content = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    async_fs::write(&state.temp_path, json_content)
        .await
        .map_err(|e| e.to_string())?;

    Ok(data)
}

#[command]
pub async fn export_data(file_path: String, state: State<'_, AppState>) -> CommandResult<()> {
    let path_to_export = if state.temp_path.exists() {
        &state.temp_path
    } else {
        &state.data_path
    };

    if path_to_export.exists() {
        async_fs::copy(path_to_export, Path::new(&file_path))
            .await
            .map_err(|e| e.to_string())?;
    }
    else {
        let default_data = AllData::default();
        let json_content = serde_json::to_string_pretty(&default_data).map_err(|e| e.to_string())?;
        async_fs::write(Path::new(&file_path), json_content)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
