use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Información del audio procesado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInfo {
    /// Duración en segundos
    pub duration: f64,
    /// Duración formateada (M:SS)
    pub duration_str: String,
    /// Número de canales (1 = mono, 2 = stereo)
    pub channels: u32,
    /// Frecuencia de muestreo (Hz)
    pub sample_rate: u32,
}

impl AudioInfo {
    pub fn new(duration: f64, channels: u32, sample_rate: u32) -> Self {
        Self {
            duration,
            duration_str: Self::format_duration(duration),
            channels,
            sample_rate,
        }
    }

    /// Formatea la duración como M:SS
    pub fn format_duration(seconds: f64) -> String {
        let mins = (seconds / 60.0) as u32;
        let secs = (seconds % 60.0) as u32;
        format!("{}:{:02}", mins, secs)
    }
}

/// Tipo de fuente de la transcripción
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// Transcripción con Whisper desde archivo local
    Whisper,
    /// Subtítulos extraídos de YouTube
    YoutubeSubtitles,
    /// Transcripción con Whisper desde audio de YouTube (legacy)
    Youtube,
    /// Transcripción con Whisper desde audio de YouTube (usando yt-dlp)
    YoutubeWhisper,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Whisper => write!(f, "whisper"),
            SourceType::YoutubeSubtitles => write!(f, "youtube_subtitles"),
            SourceType::Youtube => write!(f, "youtube"),
            SourceType::YoutubeWhisper => write!(f, "youtube_whisper"),
        }
    }
}

/// Entrada en el historial de transcripciones
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionEntry {
    /// ID único de la transcripción
    pub id: String,
    /// Fecha y hora de creación
    pub timestamp: DateTime<Utc>,
    /// Nombre de la fuente (nombre del archivo o título del video)
    pub source_name: String,
    /// Tipo de fuente
    pub source_type: SourceType,
    /// Texto de la transcripción
    pub transcription: String,
    /// Información del audio
    pub audio_info: Option<AudioInfo>,
    /// Tiempo de procesamiento en segundos
    pub processing_time: f64,
    /// Número de palabras
    pub word_count: usize,
    /// Número de caracteres
    pub char_count: usize,
    /// Idioma detectado
    pub detected_language: Option<String>,
}

impl TranscriptionEntry {
    /// Crea una nueva entrada de transcripción
    pub fn new(
        source_name: String,
        source_type: SourceType,
        transcription: String,
        audio_info: Option<AudioInfo>,
        processing_time: f64,
        detected_language: Option<String>,
    ) -> Self {
        let word_count = transcription.split_whitespace().count();
        let char_count = transcription.chars().count();
        let id = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();

        Self {
            id,
            timestamp: Utc::now(),
            source_name,
            source_type,
            transcription,
            audio_info,
            processing_time,
            word_count,
            char_count,
            detected_language,
        }
    }
}

/// Resultado de una transcripción
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// Texto transcrito
    pub text: String,
    /// Idioma detectado (código ISO)
    pub language: Option<String>,
    /// Información del audio
    pub audio_info: Option<AudioInfo>,
    /// Tiempo de procesamiento en segundos
    pub processing_time: f64,
}

/// Información de un video de YouTube
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    /// Título del video
    pub title: String,
    /// Duración en segundos
    pub duration: u64,
    /// Nombre del canal/uploader
    pub uploader: String,
    /// URL del thumbnail
    pub thumbnail_url: Option<String>,
}

/// Evento de progreso para la UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ProgressEvent {
    /// Inicio del proceso
    Started { message: String },
    /// Progreso de transcripción (0.0 - 1.0)
    Progress { progress: f32, message: String },
    /// Procesando chunk N de M
    ChunkProgress { current: u32, total: u32 },
    /// Descarga de modelo en progreso
    ModelDownload { progress: f32, bytes_downloaded: u64, total_bytes: u64 },
    /// Proceso completado
    Completed { message: String },
    /// Error durante el proceso
    Error { message: String },
}
