use thiserror::Error;

/// Errores de la aplicación AudioInk
#[derive(Error, Debug)]
pub enum AudioInkError {
    #[error("Error de audio: {0}")]
    Audio(String),

    #[error("Error de Whisper: {0}")]
    Whisper(String),

    #[error("Error de YouTube: {0}")]
    YouTube(String),

    #[error("Modelo no encontrado: {0}")]
    ModelNotFound(String),

    #[error("Error al descargar modelo: {0}")]
    ModelDownload(String),

    #[error("Formato de archivo no soportado: {0}")]
    UnsupportedFormat(String),

    #[error("Error de archivo: {0}")]
    FileError(String),

    #[error("Error de persistencia: {0}")]
    Persistence(String),

    #[error("Error de red: {0}")]
    Network(String),

    #[error("Operación cancelada")]
    Cancelled,

    #[error("Error interno: {0}")]
    Internal(String),
}

impl From<std::io::Error> for AudioInkError {
    fn from(err: std::io::Error) -> Self {
        AudioInkError::FileError(err.to_string())
    }
}

impl From<serde_json::Error> for AudioInkError {
    fn from(err: serde_json::Error) -> Self {
        AudioInkError::Persistence(err.to_string())
    }
}

impl From<reqwest::Error> for AudioInkError {
    fn from(err: reqwest::Error) -> Self {
        AudioInkError::Network(err.to_string())
    }
}

/// Result type para AudioInk
pub type AudioInkResult<T> = Result<T, AudioInkError>;

/// Convierte AudioInkError a un formato serializable para Tauri
impl serde::Serialize for AudioInkError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
