use crate::core::{download_model, get_model_path, is_model_downloaded, list_downloaded_models};
use crate::models::WhisperModel;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

/// Información de un modelo para el frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
    pub size_formatted: String,
    pub downloaded: bool,
}

impl From<&WhisperModel> for ModelInfo {
    fn from(model: &WhisperModel) -> Self {
        let size = model.size_bytes();
        Self {
            name: model.to_string(),
            description: model.description().to_string(),
            size_bytes: size,
            size_formatted: format_size(size),
            downloaded: is_model_downloaded(model),
        }
    }
}

fn format_size(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.0} MB", bytes as f64 / MB as f64)
    }
}

/// Lista todos los modelos disponibles
#[tauri::command]
pub fn list_models() -> Vec<ModelInfo> {
    WhisperModel::all().iter().map(ModelInfo::from).collect()
}

/// Lista solo los modelos descargados
#[tauri::command]
pub fn get_downloaded_models() -> Vec<ModelInfo> {
    list_downloaded_models()
        .iter()
        .map(ModelInfo::from)
        .collect()
}

/// Verifica si un modelo está descargado
#[tauri::command]
pub fn check_model_downloaded(model_name: String) -> Result<bool, String> {
    let model = parse_model_name(&model_name)?;
    Ok(is_model_downloaded(&model))
}

/// Descarga un modelo de Whisper
#[tauri::command]
pub async fn download_whisper_model(
    app: AppHandle,
    model_name: String,
) -> Result<String, String> {
    let model = parse_model_name(&model_name)?;

    // Callback para emitir progreso
    let app_clone = app.clone();
    let on_progress = Box::new(move |progress: f32, downloaded: u64, total: u64| {
        let _ = app_clone.emit(
            "model-download-progress",
            serde_json::json!({
                "model": model_name.clone(),
                "progress": progress,
                "downloaded": downloaded,
                "total": total,
                "downloaded_formatted": format_size(downloaded),
                "total_formatted": format_size(total),
            }),
        );
    });

    let path = download_model(&model, Some(on_progress))
        .await
        .map_err(|e| e.to_string())?;

    // Emitir evento de completado
    let _ = app.emit(
        "model-download-complete",
        serde_json::json!({
            "model": model.to_string(),
            "path": path.to_string_lossy().to_string(),
        }),
    );

    Ok(path.to_string_lossy().to_string())
}

/// Elimina un modelo descargado
#[tauri::command]
pub async fn delete_whisper_model(model_name: String) -> Result<(), String> {
    let model = parse_model_name(&model_name)?;
    crate::core::delete_model(&model)
        .await
        .map_err(|e| e.to_string())
}

/// Obtiene la ruta de un modelo
#[tauri::command]
pub fn get_model_path_cmd(model_name: String) -> Result<String, String> {
    let model = parse_model_name(&model_name)?;
    Ok(get_model_path(&model).to_string_lossy().to_string())
}

/// Parsea el nombre del modelo a enum
fn parse_model_name(name: &str) -> Result<WhisperModel, String> {
    match name.to_lowercase().as_str() {
        "tiny" => Ok(WhisperModel::Tiny),
        "base" => Ok(WhisperModel::Base),
        "small" => Ok(WhisperModel::Small),
        "medium" => Ok(WhisperModel::Medium),
        "large" => Ok(WhisperModel::Large),
        "large-v3-turbo" => Ok(WhisperModel::LargeV3Turbo),
        _ => Err(format!("Modelo desconocido: {}", name)),
    }
}
