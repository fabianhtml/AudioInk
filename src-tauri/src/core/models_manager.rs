use crate::models::WhisperModel;
use crate::utils::{AudioInkError, AudioInkResult};
use directories::ProjectDirs;
use futures_util::StreamExt;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Obtiene el directorio donde se almacenan los modelos
pub fn get_models_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "audioink", "AudioInk") {
        let models_dir = proj_dirs.data_dir().join("models");
        models_dir
    } else {
        PathBuf::from("./models")
    }
}

/// Obtiene la ruta completa de un modelo
pub fn get_model_path(model: &WhisperModel) -> PathBuf {
    get_models_dir().join(model.filename())
}

/// Verifica si un modelo está descargado
pub fn is_model_downloaded(model: &WhisperModel) -> bool {
    let path = get_model_path(model);
    path.exists()
}

/// Lista los modelos descargados
pub fn list_downloaded_models() -> Vec<WhisperModel> {
    WhisperModel::all()
        .into_iter()
        .filter(|m| is_model_downloaded(m))
        .collect()
}

/// Callback para reportar progreso de descarga
pub type DownloadProgressCallback = Box<dyn Fn(f32, u64, u64) + Send + Sync>;

/// Descarga un modelo de Whisper
pub async fn download_model(
    model: &WhisperModel,
    on_progress: Option<DownloadProgressCallback>,
) -> AudioInkResult<PathBuf> {
    let models_dir = get_models_dir();

    // Crear directorio si no existe
    fs::create_dir_all(&models_dir)
        .await
        .map_err(|e| AudioInkError::FileError(e.to_string()))?;

    let model_path = models_dir.join(model.filename());

    // Si ya existe, retornar la ruta
    if model_path.exists() {
        return Ok(model_path);
    }

    let url = model.download_url();
    let client = reqwest::Client::new();

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| AudioInkError::ModelDownload(e.to_string()))?;

    if !response.status().is_success() {
        return Err(AudioInkError::ModelDownload(format!(
            "Error al descargar modelo: HTTP {}",
            response.status()
        )));
    }

    let total_size = response.content_length().unwrap_or(model.size_bytes());

    // Archivo temporal para descarga
    let temp_path = model_path.with_extension("downloading");
    let mut file = fs::File::create(&temp_path)
        .await
        .map_err(|e| AudioInkError::FileError(e.to_string()))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AudioInkError::ModelDownload(e.to_string()))?;

        file.write_all(&chunk)
            .await
            .map_err(|e| AudioInkError::FileError(e.to_string()))?;

        downloaded += chunk.len() as u64;

        if let Some(ref callback) = on_progress {
            let progress = downloaded as f32 / total_size as f32;
            callback(progress, downloaded, total_size);
        }
    }

    file.flush()
        .await
        .map_err(|e| AudioInkError::FileError(e.to_string()))?;

    // Renombrar archivo temporal al nombre final
    fs::rename(&temp_path, &model_path)
        .await
        .map_err(|e| AudioInkError::FileError(e.to_string()))?;

    Ok(model_path)
}

/// Elimina un modelo descargado
pub async fn delete_model(model: &WhisperModel) -> AudioInkResult<()> {
    let path = get_model_path(model);
    if path.exists() {
        fs::remove_file(&path)
            .await
            .map_err(|e| AudioInkError::FileError(e.to_string()))?;
    }
    Ok(())
}

/// Obtiene información sobre el espacio usado por los modelos
pub async fn get_models_storage_info() -> AudioInkResult<ModelsStorageInfo> {
    let models_dir = get_models_dir();
    let mut total_size: u64 = 0;
    let mut model_sizes: Vec<(WhisperModel, u64)> = Vec::new();

    for model in WhisperModel::all() {
        let path = get_model_path(&model);
        if path.exists() {
            if let Ok(metadata) = fs::metadata(&path).await {
                let size = metadata.len();
                total_size += size;
                model_sizes.push((model, size));
            }
        }
    }

    Ok(ModelsStorageInfo {
        models_dir,
        total_size,
        model_sizes,
    })
}

/// Información sobre el almacenamiento de modelos
#[derive(Debug)]
pub struct ModelsStorageInfo {
    pub models_dir: PathBuf,
    pub total_size: u64,
    pub model_sizes: Vec<(WhisperModel, u64)>,
}

impl ModelsStorageInfo {
    /// Formatea el tamaño total en formato legible
    pub fn total_size_formatted(&self) -> String {
        format_bytes(self.total_size)
    }
}

/// Formatea bytes en formato legible (KB, MB, GB)
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
