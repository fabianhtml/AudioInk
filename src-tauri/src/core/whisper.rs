use crate::core::{get_model_path, is_model_downloaded, split_into_chunks, needs_chunking};
use crate::models::{Language, TranscriptionResult, WhisperModel, AudioInfo};
use crate::utils::{AudioInkError, AudioInkResult};
use std::sync::Arc;
use std::time::Instant;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Motor de transcripción con Whisper
pub struct WhisperEngine {
    context: WhisperContext,
    model_name: String,
}

impl WhisperEngine {
    /// Crea una nueva instancia del motor Whisper
    pub fn new(model: &WhisperModel) -> AudioInkResult<Self> {
        if !is_model_downloaded(model) {
            return Err(AudioInkError::ModelNotFound(format!(
                "Modelo '{}' no está descargado. Por favor, descárgalo primero.",
                model
            )));
        }

        let model_path = get_model_path(model);
        let params = WhisperContextParameters::default();

        let context = WhisperContext::new_with_params(
            model_path.to_str().unwrap(),
            params,
        )
        .map_err(|e| AudioInkError::Whisper(format!("Error al cargar modelo: {}", e)))?;

        Ok(Self {
            context,
            model_name: model.to_string(),
        })
    }

    /// Transcribe audio (samples f32 mono 16kHz)
    pub fn transcribe(
        &self,
        samples: &[f32],
        language: &Language,
        audio_info: Option<AudioInfo>,
        on_progress: Option<Box<dyn Fn(f32, String, Option<String>) + Send + Sync>>,
    ) -> AudioInkResult<TranscriptionResult> {
        self.transcribe_with_timestamps(samples, language, audio_info, on_progress, false)
    }

    /// Transcribe audio with optional timestamps
    /// on_progress callback receives (progress: f32, message: String, chunk_text: Option<String>)
    pub fn transcribe_with_timestamps(
        &self,
        samples: &[f32],
        language: &Language,
        audio_info: Option<AudioInfo>,
        on_progress: Option<Box<dyn Fn(f32, String, Option<String>) + Send + Sync>>,
        include_timestamps: bool,
    ) -> AudioInkResult<TranscriptionResult> {
        let start_time = Instant::now();

        // Verificar si necesita procesamiento en chunks
        if needs_chunking(samples) {
            return self.transcribe_chunked_with_timestamps(samples, language, audio_info, on_progress, include_timestamps);
        }

        // Transcripción directa para archivos cortos (no chunked, so no progressive callback needed)
        let text = self.transcribe_segment_with_options(samples, language, None, include_timestamps, 0)?;
        let detected_language = self.detect_language_from_samples(samples)?;

        // Emit the complete text for short files
        if let Some(ref callback) = on_progress {
            callback(1.0, "Transcription completed".to_string(), Some(text.clone()));
        }

        let processing_time = start_time.elapsed().as_secs_f64();

        Ok(TranscriptionResult {
            text,
            language: Some(detected_language),
            audio_info,
            processing_time,
        })
    }

    /// Transcribe audio largo en chunks with optional timestamps
    fn transcribe_chunked_with_timestamps(
        &self,
        samples: &[f32],
        language: &Language,
        audio_info: Option<AudioInfo>,
        on_progress: Option<Box<dyn Fn(f32, String, Option<String>) + Send + Sync>>,
        include_timestamps: bool,
    ) -> AudioInkResult<TranscriptionResult> {
        use crate::models::CHUNK_DURATION_SECS;

        let start_time = Instant::now();
        let chunks = split_into_chunks(samples);
        let total_chunks = chunks.len();
        let mut transcriptions: Vec<String> = Vec::new();

        // Detectar idioma en el primer chunk
        let detected_language = if !chunks.is_empty() {
            self.detect_language_from_samples(&chunks[0])?
        } else {
            "unknown".to_string()
        };

        // Calculate chunk duration in ms for offset
        let chunk_duration_ms = (CHUNK_DURATION_SECS * 1000.0) as i64;

        for (i, chunk) in chunks.iter().enumerate() {
            if let Some(ref callback) = on_progress {
                let progress = (i as f32 + 0.5) / total_chunks as f32;
                callback(
                    progress,
                    format!("Transcribing chunk {} of {}", i + 1, total_chunks),
                    None,
                );
            }

            let time_offset_ms = (i as i64) * chunk_duration_ms;
            let text = self.transcribe_segment_with_options(chunk, language, None, include_timestamps, time_offset_ms)?;
            transcriptions.push(text.clone());

            // Emit progress with the chunk text for progressive display
            if let Some(ref callback) = on_progress {
                let progress = (i + 1) as f32 / total_chunks as f32;
                callback(
                    progress,
                    format!("Chunk {} of {} completed", i + 1, total_chunks),
                    Some(text),
                );
            }
        }

        let separator = if include_timestamps { "\n" } else { " " };
        let full_text = transcriptions.join(separator);
        let processing_time = start_time.elapsed().as_secs_f64();

        Ok(TranscriptionResult {
            text: full_text,
            language: Some(detected_language),
            audio_info,
            processing_time,
        })
    }

    /// Transcribe un segmento de audio con opciones
    fn transcribe_segment_with_options(
        &self,
        samples: &[f32],
        language: &Language,
        _on_progress: Option<&Box<dyn Fn(f32, String) + Send + Sync>>,
        include_timestamps: bool,
        time_offset_ms: i64,
    ) -> AudioInkResult<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Configurar idioma
        if let Some(lang_code) = language.code() {
            params.set_language(Some(lang_code));
        } else {
            params.set_language(None); // Auto-detect
        }

        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Crear estado y ejecutar transcripción
        let mut state = self
            .context
            .create_state()
            .map_err(|e| AudioInkError::Whisper(e.to_string()))?;

        state
            .full(params, samples)
            .map_err(|e| AudioInkError::Whisper(e.to_string()))?;

        // Extraer texto de los segmentos
        let num_segments = state
            .full_n_segments()
            .map_err(|e| AudioInkError::Whisper(e.to_string()))?;

        let mut text = String::new();
        for i in 0..num_segments {
            let segment_text = state
                .full_get_segment_text(i)
                .map_err(|e| AudioInkError::Whisper(e.to_string()))?;

            if include_timestamps {
                // Get segment start time in centiseconds (whisper uses 10ms units)
                let t0 = state.full_get_segment_t0(i)
                    .map_err(|e| AudioInkError::Whisper(e.to_string()))?;
                // Convert to milliseconds and add offset
                let start_ms = (t0 * 10) as i64 + time_offset_ms;
                let timestamp = format_timestamp_ms(start_ms);
                text.push_str(&format!("[{}] {}\n", timestamp, segment_text.trim()));
            } else {
                text.push_str(&segment_text);
            }
        }

        Ok(text.trim().to_string())
    }

    /// Detecta el idioma de un audio
    pub fn detect_language(&self, samples: &[f32]) -> AudioInkResult<String> {
        self.detect_language_from_samples(samples)
    }

    /// Detecta el idioma usando los primeros 30 segundos de audio
    fn detect_language_from_samples(&self, samples: &[f32]) -> AudioInkResult<String> {
        // Usar máximo 30 segundos para detección
        let sample_size = (30.0 * 16000.0) as usize;
        let sample = if samples.len() > sample_size {
            &samples[..sample_size]
        } else {
            samples
        };

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(None); // Auto-detect

        let mut state = self
            .context
            .create_state()
            .map_err(|e| AudioInkError::Whisper(e.to_string()))?;

        state
            .full(params, sample)
            .map_err(|e| AudioInkError::Whisper(e.to_string()))?;

        let lang_id = state
            .full_lang_id_from_state()
            .map_err(|e| AudioInkError::Whisper(e.to_string()))?;

        Ok(whisper_rs::get_lang_str(lang_id).unwrap_or("unknown").to_string())
    }

    /// Retorna el nombre del modelo cargado
    pub fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Wrapper thread-safe para el motor Whisper
pub struct WhisperEngineWrapper(pub Arc<WhisperEngine>);

impl WhisperEngineWrapper {
    pub fn new(model: &WhisperModel) -> AudioInkResult<Self> {
        Ok(Self(Arc::new(WhisperEngine::new(model)?)))
    }

    pub fn engine(&self) -> &WhisperEngine {
        &self.0
    }
}

// WhisperEngine no implementa Send/Sync nativamente, pero es seguro usarlo
// en un contexto de un solo hilo por modelo
unsafe impl Send for WhisperEngine {}
unsafe impl Sync for WhisperEngine {}

/// Formatea milisegundos a formato HH:MM:SS
fn format_timestamp_ms(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Nota: Los tests reales requieren un modelo descargado
    // Estos son tests de estructura

    #[test]
    fn test_language_code() {
        assert_eq!(Language::Auto.code(), None);
        assert_eq!(Language::English.code(), Some("en"));
        assert_eq!(Language::Spanish.code(), Some("es"));
    }
}
