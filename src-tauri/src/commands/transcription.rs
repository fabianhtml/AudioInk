use crate::core::{decode_audio_to_whisper_format, is_model_downloaded, WhisperEngine, download_youtube_audio, cleanup_youtube_audio, is_ytdlp_available};
use crate::models::{Language, SourceType, TranscriptionEntry, TranscriptionResult, WhisperModel};
use crate::persistence::HistoryManager;
use crate::utils::AudioInkError;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

/// Estado global de la aplicación
pub struct AppState {
    pub history_manager: HistoryManager,
    pub current_engine: Mutex<Option<(WhisperModel, WhisperEngine)>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            history_manager: HistoryManager::new(),
            current_engine: Mutex::new(None),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Obtiene o crea el motor Whisper para un modelo específico
    pub fn get_or_create_engine(&self, model: &WhisperModel) -> Result<(), AudioInkError> {
        let mut guard = self.current_engine.lock().map_err(|e| {
            AudioInkError::Internal(format!("Error de lock: {}", e))
        })?;

        // Si ya tenemos el motor correcto, no hacer nada
        if let Some((current_model, _)) = guard.as_ref() {
            if current_model == model {
                return Ok(());
            }
        }

        // Crear nuevo motor
        let engine = WhisperEngine::new(model)?;
        *guard = Some((model.clone(), engine));
        Ok(())
    }
}

/// Opciones de transcripción
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeOptions {
    pub model: String,
    pub language: String,
    #[serde(default)]
    pub include_timestamps: bool,
}

impl Default for TranscribeOptions {
    fn default() -> Self {
        Self {
            model: "base".to_string(),
            language: "auto".to_string(),
            include_timestamps: false,
        }
    }
}

/// Parsea el nombre del modelo
fn parse_model(name: &str) -> Result<WhisperModel, String> {
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

/// Parsea el idioma
fn parse_language(name: &str) -> Language {
    match name.to_lowercase().as_str() {
        "auto" => Language::Auto,
        "en" | "english" => Language::English,
        "es" | "spanish" | "español" => Language::Spanish,
        "fr" | "french" | "français" => Language::French,
        "de" | "german" | "deutsch" => Language::German,
        "it" | "italian" | "italiano" => Language::Italian,
        "pt" | "portuguese" | "português" => Language::Portuguese,
        "ja" | "japanese" | "日本語" => Language::Japanese,
        "zh" | "chinese" | "中文" => Language::Chinese,
        "ko" | "korean" | "한국어" => Language::Korean,
        "ru" | "russian" | "русский" => Language::Russian,
        _ => Language::Auto,
    }
}

/// Transcribe un archivo de audio local
#[tauri::command]
pub async fn transcribe_file(
    app: AppHandle,
    state: State<'_, AppState>,
    file_path: String,
    options: TranscribeOptions,
) -> Result<TranscriptionResult, String> {
    let path = std::path::PathBuf::from(&file_path);

    // Verificar que el archivo existe
    if !path.exists() {
        return Err(format!("Archivo no encontrado: {}", file_path));
    }

    // Parsear opciones
    let model = parse_model(&options.model)?;
    let language = parse_language(&options.language);

    // Verificar que el modelo está descargado
    if !is_model_downloaded(&model) {
        return Err(format!(
            "El modelo '{}' no está descargado. Por favor, descárgalo primero.",
            model
        ));
    }

    // Emitir evento de inicio
    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "started",
            "message": "Iniciando transcripción..."
        }),
    );

    // Decodificar audio
    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "progress",
            "progress": 0.1,
            "message": "Decodificando audio..."
        }),
    );

    let path_clone = path.clone();
    let (samples, audio_info) = tokio::task::spawn_blocking(move || {
        decode_audio_to_whisper_format(&path_clone)
    })
    .await
    .map_err(|e| format!("Error de task: {}", e))?
    .map_err(|e| e.to_string())?;

    // Crear/obtener motor Whisper
    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "progress",
            "progress": 0.2,
            "message": "Cargando modelo Whisper..."
        }),
    );

    state.get_or_create_engine(&model).map_err(|e| e.to_string())?;

    // Transcribir
    let app_clone = app.clone();
    let on_progress = Box::new(move |progress: f32, message: String| {
        let _ = app_clone.emit(
            "transcription-progress",
            serde_json::json!({
                "type": "progress",
                "progress": 0.2 + progress * 0.8,
                "message": message
            }),
        );
    });

    let include_timestamps = options.include_timestamps;
    let result = {
        let guard = state.current_engine.lock().map_err(|e| e.to_string())?;
        if let Some((_, engine)) = guard.as_ref() {
            engine.transcribe_with_timestamps(&samples, &language, Some(audio_info), Some(on_progress), include_timestamps)
                .map_err(|e| e.to_string())?
        } else {
            return Err("Motor Whisper no inicializado".to_string());
        }
    };

    // Guardar en historial
    let source_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("audio")
        .to_string();

    let entry = TranscriptionEntry::new(
        source_name,
        SourceType::Whisper,
        result.text.clone(),
        result.audio_info.clone(),
        result.processing_time,
        result.language.clone(),
    );

    state
        .history_manager
        .save_transcription(entry)
        .await
        .map_err(|e| e.to_string())?;

    // Emitir evento de completado
    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "completed",
            "message": "Transcripción completada"
        }),
    );

    Ok(result)
}

/// Check if yt-dlp is available
#[tauri::command]
pub fn check_ytdlp_available() -> bool {
    is_ytdlp_available()
}

/// Transcribe audio from a YouTube URL using Whisper
#[tauri::command]
pub async fn transcribe_youtube(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    options: TranscribeOptions,
) -> Result<TranscriptionResult, String> {
    // Check if yt-dlp is available
    if !is_ytdlp_available() {
        return Err("yt-dlp is not installed. Please install it with: brew install yt-dlp".to_string());
    }

    // Parsear opciones
    let model = parse_model(&options.model)?;
    let language = parse_language(&options.language);

    // Verificar que el modelo está descargado
    if !is_model_downloaded(&model) {
        return Err(format!(
            "El modelo '{}' no está descargado. Por favor, descárgalo primero.",
            model
        ));
    }

    // Emitir evento de inicio
    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "started",
            "message": "Downloading audio from YouTube..."
        }),
    );

    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "progress",
            "progress": 0.05,
            "message": "Downloading audio from YouTube..."
        }),
    );

    // Download audio from YouTube
    let url_clone = url.clone();
    let download_result = tokio::task::spawn_blocking(move || {
        download_youtube_audio(&url_clone)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| e.to_string())?;

    let audio_path = download_result.audio_path.clone();
    let video_title = download_result.title.clone();

    // Decode audio
    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "progress",
            "progress": 0.2,
            "message": "Decoding audio..."
        }),
    );

    let audio_path_clone = audio_path.clone();
    let (samples, audio_info) = tokio::task::spawn_blocking(move || {
        decode_audio_to_whisper_format(&audio_path_clone)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| e.to_string())?;

    // Create/get Whisper engine
    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "progress",
            "progress": 0.3,
            "message": "Loading Whisper model..."
        }),
    );

    state.get_or_create_engine(&model).map_err(|e| e.to_string())?;

    // Transcribe
    let app_clone = app.clone();
    let on_progress = Box::new(move |progress: f32, message: String| {
        let _ = app_clone.emit(
            "transcription-progress",
            serde_json::json!({
                "type": "progress",
                "progress": 0.3 + progress * 0.7,
                "message": message
            }),
        );
    });

    let include_timestamps = options.include_timestamps;
    let result = {
        let guard = state.current_engine.lock().map_err(|e| e.to_string())?;
        if let Some((_, engine)) = guard.as_ref() {
            engine.transcribe_with_timestamps(&samples, &language, Some(audio_info), Some(on_progress), include_timestamps)
                .map_err(|e| e.to_string())?
        } else {
            // Clean up before returning error
            cleanup_youtube_audio(&audio_path);
            return Err("Whisper engine not initialized".to_string());
        }
    };

    // Save to history
    let entry = TranscriptionEntry::new(
        video_title,
        SourceType::YoutubeWhisper,
        result.text.clone(),
        result.audio_info.clone(),
        result.processing_time,
        result.language.clone(),
    );

    state
        .history_manager
        .save_transcription(entry)
        .await
        .map_err(|e| e.to_string())?;

    // Clean up downloaded file
    cleanup_youtube_audio(&audio_path);

    // Emit completed event
    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "completed",
            "message": "Transcription completed"
        }),
    );

    Ok(result)
}

/// Obtiene los idiomas disponibles
#[tauri::command]
pub fn get_languages() -> Vec<serde_json::Value> {
    Language::all()
        .iter()
        .map(|lang| {
            serde_json::json!({
                "code": lang.code().unwrap_or("auto"),
                "name": lang.display_name()
            })
        })
        .collect()
}

/// Obtiene los formatos de archivo soportados
#[tauri::command]
pub fn get_supported_formats() -> serde_json::Value {
    serde_json::json!({
        "audio": crate::models::AUDIO_FORMATS,
        "video": crate::models::VIDEO_FORMATS
    })
}
