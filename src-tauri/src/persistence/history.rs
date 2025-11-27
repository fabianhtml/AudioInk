use crate::models::TranscriptionEntry;
use crate::utils::AudioInkResult;
use directories::ProjectDirs;
use std::path::PathBuf;
use tokio::fs;

const MAX_HISTORY_ENTRIES: usize = 50;

/// Manager del historial de transcripciones
pub struct HistoryManager {
    history_file: PathBuf,
    transcriptions_dir: PathBuf,
}

impl HistoryManager {
    /// Crea un nuevo manager de historial
    pub fn new() -> Self {
        let (history_file, transcriptions_dir) =
            if let Some(proj_dirs) = ProjectDirs::from("com", "audioink", "AudioInk") {
                let data_dir = proj_dirs.data_dir();
                (
                    data_dir.join("history.json"),
                    data_dir.join("transcriptions"),
                )
            } else {
                (
                    PathBuf::from("./history.json"),
                    PathBuf::from("./transcriptions"),
                )
            };

        Self {
            history_file,
            transcriptions_dir,
        }
    }

    /// Inicializa los directorios necesarios
    pub async fn init(&self) -> AudioInkResult<()> {
        if let Some(parent) = self.history_file.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::create_dir_all(&self.transcriptions_dir).await?;
        Ok(())
    }

    /// Carga el historial de transcripciones
    pub async fn load_history(&self) -> AudioInkResult<Vec<TranscriptionEntry>> {
        if !self.history_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.history_file).await?;
        let history: Vec<TranscriptionEntry> = serde_json::from_str(&content)?;
        Ok(history)
    }

    /// Guarda una nueva transcripción
    pub async fn save_transcription(&self, entry: TranscriptionEntry) -> AudioInkResult<()> {
        self.init().await?;

        let mut history = self.load_history().await.unwrap_or_default();

        // Insertar al inicio
        history.insert(0, entry.clone());

        // Limitar el tamaño del historial
        history.truncate(MAX_HISTORY_ENTRIES);

        // Guardar historial JSON
        let json = serde_json::to_string_pretty(&history)?;
        fs::write(&self.history_file, json).await?;

        // Guardar archivo TXT individual
        self.save_as_txt(&entry).await?;

        Ok(())
    }

    /// Guarda la transcripción como archivo TXT
    async fn save_as_txt(&self, entry: &TranscriptionEntry) -> AudioInkResult<()> {
        let clean_name: String = entry
            .source_name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
            .take(50)
            .collect();

        let filename = format!("{}_{}.txt", entry.id, clean_name.trim().replace(' ', "_"));

        let duration_str = entry
            .audio_info
            .as_ref()
            .map(|ai| ai.duration_str.clone())
            .unwrap_or_else(|| "N/A".to_string());

        let content = format!(
            "# AudioInk Transcription
# Source: {}
# Type: {}
# Date: {}
# Duration: {}
# Words: {}
# Processing Time: {:.1}s
# ---

{}",
            entry.source_name,
            entry.source_type,
            entry.timestamp,
            duration_str,
            entry.word_count,
            entry.processing_time,
            entry.transcription
        );

        let file_path = self.transcriptions_dir.join(filename);
        fs::write(&file_path, content).await?;

        Ok(())
    }

    /// Obtiene una transcripción por ID
    pub async fn get_transcription(&self, id: &str) -> AudioInkResult<Option<TranscriptionEntry>> {
        let history = self.load_history().await?;
        Ok(history.into_iter().find(|e| e.id == id))
    }

    /// Elimina una transcripción por ID
    pub async fn delete_transcription(&self, id: &str) -> AudioInkResult<bool> {
        let mut history = self.load_history().await?;
        let initial_len = history.len();

        // Encontrar y eliminar la entrada
        if let Some(entry) = history.iter().find(|e| e.id == id) {
            // Eliminar archivo TXT asociado
            let clean_name: String = entry
                .source_name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
                .take(50)
                .collect();
            let filename = format!("{}_{}.txt", entry.id, clean_name.trim().replace(' ', "_"));
            let file_path = self.transcriptions_dir.join(filename);
            if file_path.exists() {
                let _ = fs::remove_file(&file_path).await;
            }
        }

        history.retain(|e| e.id != id);

        if history.len() < initial_len {
            let json = serde_json::to_string_pretty(&history)?;
            fs::write(&self.history_file, json).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Elimina todo el historial
    pub async fn clear_all(&self) -> AudioInkResult<()> {
        // Eliminar archivo de historial
        if self.history_file.exists() {
            fs::remove_file(&self.history_file).await?;
        }

        // Eliminar directorio de transcripciones
        if self.transcriptions_dir.exists() {
            fs::remove_dir_all(&self.transcriptions_dir).await?;
            fs::create_dir_all(&self.transcriptions_dir).await?;
        }

        Ok(())
    }

    /// Obtiene el número de entradas en el historial
    pub async fn count(&self) -> AudioInkResult<usize> {
        let history = self.load_history().await?;
        Ok(history.len())
    }

    /// Obtiene la ruta del directorio de transcripciones
    pub fn transcriptions_dir(&self) -> &PathBuf {
        &self.transcriptions_dir
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}
