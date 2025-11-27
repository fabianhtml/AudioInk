use serde::{Deserialize, Serialize};

/// Modelos de Whisper disponibles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
    #[serde(rename = "large-v3-turbo")]
    LargeV3Turbo,
}

impl WhisperModel {
    /// Nombre del archivo del modelo
    pub fn filename(&self) -> &'static str {
        match self {
            WhisperModel::Tiny => "ggml-tiny.bin",
            WhisperModel::Base => "ggml-base.bin",
            WhisperModel::Small => "ggml-small.bin",
            WhisperModel::Medium => "ggml-medium.bin",
            WhisperModel::Large => "ggml-large.bin",
            WhisperModel::LargeV3Turbo => "ggml-large-v3-turbo.bin",
        }
    }

    /// URL de descarga del modelo (Hugging Face)
    pub fn download_url(&self) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.filename()
        )
    }

    /// Tamaño aproximado del modelo en bytes
    pub fn size_bytes(&self) -> u64 {
        match self {
            WhisperModel::Tiny => 75_000_000,      // ~75 MB
            WhisperModel::Base => 142_000_000,     // ~142 MB
            WhisperModel::Small => 466_000_000,    // ~466 MB
            WhisperModel::Medium => 1_500_000_000, // ~1.5 GB
            WhisperModel::Large => 2_900_000_000,  // ~2.9 GB
            WhisperModel::LargeV3Turbo => 809_000_000, // ~809 MB
        }
    }

    /// Descripción del modelo
    pub fn description(&self) -> &'static str {
        match self {
            WhisperModel::Tiny => "Ultra rápido, menor calidad",
            WhisperModel::Base => "Balance entre velocidad y calidad",
            WhisperModel::Small => "Buena calidad, velocidad moderada",
            WhisperModel::Medium => "Alta calidad, más lento",
            WhisperModel::Large => "Mejor calidad, el más lento",
            WhisperModel::LargeV3Turbo => "Excelente calidad, más rápido que large",
        }
    }

    /// Lista todos los modelos disponibles
    pub fn all() -> Vec<WhisperModel> {
        vec![
            WhisperModel::Tiny,
            WhisperModel::Base,
            WhisperModel::Small,
            WhisperModel::Medium,
            WhisperModel::Large,
            WhisperModel::LargeV3Turbo,
        ]
    }
}

impl Default for WhisperModel {
    fn default() -> Self {
        WhisperModel::Base
    }
}

impl std::fmt::Display for WhisperModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhisperModel::Tiny => write!(f, "tiny"),
            WhisperModel::Base => write!(f, "base"),
            WhisperModel::Small => write!(f, "small"),
            WhisperModel::Medium => write!(f, "medium"),
            WhisperModel::Large => write!(f, "large"),
            WhisperModel::LargeV3Turbo => write!(f, "large-v3-turbo"),
        }
    }
}

/// Idiomas soportados para transcripción
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Auto,
    English,
    Spanish,
    French,
    German,
    Italian,
    Portuguese,
    Japanese,
    Chinese,
    Korean,
    Russian,
}

impl Language {
    /// Código ISO del idioma para Whisper
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Language::Auto => None,
            Language::English => Some("en"),
            Language::Spanish => Some("es"),
            Language::French => Some("fr"),
            Language::German => Some("de"),
            Language::Italian => Some("it"),
            Language::Portuguese => Some("pt"),
            Language::Japanese => Some("ja"),
            Language::Chinese => Some("zh"),
            Language::Korean => Some("ko"),
            Language::Russian => Some("ru"),
        }
    }

    /// Nombre para mostrar
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Auto => "Auto-detect",
            Language::English => "English",
            Language::Spanish => "Español",
            Language::French => "Français",
            Language::German => "Deutsch",
            Language::Italian => "Italiano",
            Language::Portuguese => "Português",
            Language::Japanese => "日本語",
            Language::Chinese => "中文",
            Language::Korean => "한국어",
            Language::Russian => "Русский",
        }
    }

    /// Lista todos los idiomas disponibles
    pub fn all() -> Vec<Language> {
        vec![
            Language::Auto,
            Language::English,
            Language::Spanish,
            Language::French,
            Language::German,
            Language::Italian,
            Language::Portuguese,
            Language::Japanese,
            Language::Chinese,
            Language::Korean,
            Language::Russian,
        ]
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::Auto
    }
}

/// Formatos de audio soportados
pub const AUDIO_FORMATS: &[&str] = &["mp3", "wav", "m4a", "flac", "ogg"];
pub const VIDEO_FORMATS: &[&str] = &["mp4", "avi", "mov"];

/// Constantes de procesamiento de audio
pub const WHISPER_SAMPLE_RATE: u32 = 16000;
pub const CHUNK_DURATION_SECS: f32 = 60.0;
pub const LARGE_FILE_THRESHOLD_SECS: f32 = 120.0;
