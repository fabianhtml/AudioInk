use crate::core::{decode_audio_to_whisper_format, is_model_downloaded, WhisperEngine, download_youtube_audio, cleanup_youtube_audio, is_ytdlp_available, apply_audio_speedup, cleanup_speedup_file, is_video_format, extract_audio_from_video, cleanup_extracted_audio};
use crate::models::{Language, SourceType, TranscriptionEntry, TranscriptionResult, WhisperModel};
use crate::persistence::HistoryManager;
use crate::utils::{get_ytdlp_install_instructions, AudioInkError};
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
    /// Audio speed factor (1.0 = normal, 1.5 = 1.5x faster, max 2.0)
    #[serde(default = "default_speed")]
    pub speed: f32,
}

fn default_speed() -> f32 {
    1.0
}

impl Default for TranscribeOptions {
    fn default() -> Self {
        Self {
            model: "base".to_string(),
            language: "auto".to_string(),
            include_timestamps: false,
            speed: 1.0,
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

/// Adjust timestamps in text by multiplying them by the speed factor
/// Timestamps are in format [HH:MM:SS]
fn adjust_timestamps_in_text(text: &str, speed: f32) -> String {
    use regex::Regex;

    let re = Regex::new(r"\[(\d{2}):(\d{2}):(\d{2})\]").unwrap();

    re.replace_all(text, |caps: &regex::Captures| {
        let hours: i64 = caps[1].parse().unwrap_or(0);
        let minutes: i64 = caps[2].parse().unwrap_or(0);
        let seconds: i64 = caps[3].parse().unwrap_or(0);

        // Convert to total milliseconds
        let total_ms = (hours * 3600 + minutes * 60 + seconds) * 1000;

        // Apply speed factor
        let adjusted_ms = ((total_ms as f64) * (speed as f64)).round() as i64;

        // Convert back to HH:MM:SS
        let total_secs = adjusted_ms / 1000;
        let h = total_secs / 3600;
        let m = (total_secs % 3600) / 60;
        let s = total_secs % 60;

        format!("[{:02}:{:02}:{:02}]", h, m, s)
    }).to_string()
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
    let speed = options.speed.clamp(1.0, 2.0); // Limit to safe range

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

    // Extract audio from video if needed
    let mut extracted_audio_path: Option<std::path::PathBuf> = None;
    let base_audio_path = if is_video_format(&path) {
        let _ = app.emit(
            "transcription-progress",
            serde_json::json!({
                "type": "progress",
                "progress": 0.02,
                "message": "Extrayendo audio del video..."
            }),
        );

        let path_for_extraction = path.clone();
        let extracted_path = tokio::task::spawn_blocking(move || {
            extract_audio_from_video(&path_for_extraction)
        })
        .await
        .map_err(|e| format!("Error de task: {}", e))?
        .map_err(|e| e.to_string())?;

        extracted_audio_path = Some(extracted_path.clone());
        extracted_path
    } else {
        path.clone()
    };

    // Apply speedup if needed
    let mut speedup_path: Option<std::path::PathBuf> = None;
    let audio_path = if speed > 1.01 {
        let _ = app.emit(
            "transcription-progress",
            serde_json::json!({
                "type": "progress",
                "progress": 0.05,
                "message": format!("Acelerando audio a {}x...", speed)
            }),
        );

        let path_for_speedup = base_audio_path.clone();
        let speed_factor = speed;
        let sped_up_path = tokio::task::spawn_blocking(move || {
            apply_audio_speedup(&path_for_speedup, speed_factor)
        })
        .await
        .map_err(|e| format!("Error de task: {}", e))?
        .map_err(|e| e.to_string())?;

        speedup_path = Some(sped_up_path.clone());
        sped_up_path
    } else {
        base_audio_path.clone()
    };

    // Decodificar audio
    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "progress",
            "progress": 0.1,
            "message": "Decodificando audio..."
        }),
    );

    let (samples, mut audio_info) = tokio::task::spawn_blocking(move || {
        decode_audio_to_whisper_format(&audio_path)
    })
    .await
    .map_err(|e| format!("Error de task: {}", e))?
    .map_err(|e| e.to_string())?;

    // Adjust audio_info duration for speedup (show original duration)
    if speed > 1.01 {
        if let Some(ref mut info) = Some(&mut audio_info) {
            info.duration *= speed as f64;
            info.duration_str = crate::models::AudioInfo::format_duration(info.duration);
        }
    }

    // Clean up speedup temp file
    if let Some(ref temp_path) = speedup_path {
        cleanup_speedup_file(temp_path);
    }

    // Clean up extracted audio temp file
    if let Some(ref temp_path) = extracted_audio_path {
        cleanup_extracted_audio(temp_path);
    }

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
    let on_progress = Box::new(move |progress: f32, message: String, chunk_text: Option<String>| {
        let mut payload = serde_json::json!({
            "type": "progress",
            "progress": 0.2 + progress * 0.8,
            "message": message
        });
        if let Some(text) = chunk_text {
            payload["chunk_text"] = serde_json::json!(text);
        }
        let _ = app_clone.emit("transcription-progress", payload);
    });

    let include_timestamps = options.include_timestamps;
    let mut result = {
        let guard = state.current_engine.lock().map_err(|e| e.to_string())?;
        if let Some((_, engine)) = guard.as_ref() {
            engine.transcribe_with_timestamps(&samples, &language, Some(audio_info), Some(on_progress), include_timestamps)
                .map_err(|e| e.to_string())?
        } else {
            return Err("Motor Whisper no inicializado".to_string());
        }
    };

    // Adjust timestamps for speedup if needed
    if speed > 1.01 && include_timestamps {
        result.text = adjust_timestamps_in_text(&result.text, speed);
    }

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
        return Err(get_ytdlp_install_instructions().to_string());
    }

    // Parsear opciones
    let model = parse_model(&options.model)?;
    let language = parse_language(&options.language);
    let speed = options.speed.clamp(1.0, 2.0); // Limit to safe range

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

    // Apply speedup if needed
    let mut speedup_path: Option<std::path::PathBuf> = None;
    let decode_path = if speed > 1.01 {
        let _ = app.emit(
            "transcription-progress",
            serde_json::json!({
                "type": "progress",
                "progress": 0.15,
                "message": format!("Accelerating audio to {}x...", speed)
            }),
        );

        let path_for_speedup = audio_path.clone();
        let speed_factor = speed;
        let sped_up_path = tokio::task::spawn_blocking(move || {
            apply_audio_speedup(&path_for_speedup, speed_factor)
        })
        .await
        .map_err(|e| format!("Task error: {}", e))?
        .map_err(|e| e.to_string())?;

        speedup_path = Some(sped_up_path.clone());
        sped_up_path
    } else {
        audio_path.clone()
    };

    // Decode audio
    let _ = app.emit(
        "transcription-progress",
        serde_json::json!({
            "type": "progress",
            "progress": 0.2,
            "message": "Decoding audio..."
        }),
    );

    let (samples, mut audio_info) = tokio::task::spawn_blocking(move || {
        decode_audio_to_whisper_format(&decode_path)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
    .map_err(|e| e.to_string())?;

    // Adjust audio_info duration for speedup (show original duration)
    if speed > 1.01 {
        audio_info.duration *= speed as f64;
        audio_info.duration_str = crate::models::AudioInfo::format_duration(audio_info.duration);
    }

    // Clean up speedup temp file
    if let Some(ref temp_path) = speedup_path {
        cleanup_speedup_file(temp_path);
    }

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
    let on_progress = Box::new(move |progress: f32, message: String, chunk_text: Option<String>| {
        let mut payload = serde_json::json!({
            "type": "progress",
            "progress": 0.3 + progress * 0.7,
            "message": message
        });
        if let Some(text) = chunk_text {
            payload["chunk_text"] = serde_json::json!(text);
        }
        let _ = app_clone.emit("transcription-progress", payload);
    });

    let include_timestamps = options.include_timestamps;
    let mut result = {
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

    // Adjust timestamps for speedup if needed
    if speed > 1.01 && include_timestamps {
        result.text = adjust_timestamps_in_text(&result.text, speed);
    }

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
