use crate::models::{AudioInfo, WHISPER_SAMPLE_RATE, CHUNK_DURATION_SECS, AUDIO_FORMATS, VIDEO_FORMATS};
use crate::utils::{AudioInkError, AudioInkResult};
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Verifica si un formato de archivo es soportado
pub fn is_supported_format(extension: &str) -> bool {
    let ext = extension.to_lowercase();
    AUDIO_FORMATS.contains(&ext.as_str()) || VIDEO_FORMATS.contains(&ext.as_str())
}

/// Decodifica un archivo de audio a samples f32 mono a 16kHz (formato requerido por Whisper)
pub fn decode_audio_to_whisper_format(path: &Path) -> AudioInkResult<(Vec<f32>, AudioInfo)> {
    let file = File::open(path).map_err(|e| AudioInkError::FileError(e.to_string()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AudioInkError::UnsupportedFormat(e.to_string()))?;

    let mut format = probed.format;

    let track = format
        .default_track()
        .ok_or_else(|| AudioInkError::Audio("No se encontró pista de audio".to_string()))?;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AudioInkError::Audio(e.to_string()))?;

    let track_id = track.id;
    let original_sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u32)
        .unwrap_or(2);

    let mut all_samples: Vec<f32> = Vec::new();

    // Decodificar todos los paquetes
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {
                continue;
            }
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = decoded.spec();
                let mut sample_buf =
                    SampleBuffer::<f32>::new(decoded.capacity() as u64, *spec);
                sample_buf.copy_interleaved_ref(decoded);
                all_samples.extend(sample_buf.samples());
            }
            Err(_) => continue,
        }
    }

    // Convertir a mono si es estéreo
    let mono_samples = if channels > 1 {
        all_samples
            .chunks(channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        all_samples
    };

    // Resamplear a 16kHz si es necesario
    let resampled = if original_sample_rate != WHISPER_SAMPLE_RATE {
        resample(&mono_samples, original_sample_rate, WHISPER_SAMPLE_RATE)
    } else {
        mono_samples
    };

    let duration = resampled.len() as f64 / WHISPER_SAMPLE_RATE as f64;
    let audio_info = AudioInfo::new(duration, channels, original_sample_rate);

    Ok((resampled, audio_info))
}

/// Resamplea audio de una frecuencia a otra usando interpolación lineal
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let new_len = (samples.len() as f64 / ratio) as usize;

    (0..new_len)
        .map(|i| {
            let src_idx = i as f64 * ratio;
            let idx = src_idx as usize;
            let frac = src_idx - idx as f64;

            if idx + 1 < samples.len() {
                // Interpolación lineal entre dos muestras
                let a = samples[idx];
                let b = samples[idx + 1];
                a + (b - a) * frac as f32
            } else {
                samples.get(idx).copied().unwrap_or(0.0)
            }
        })
        .collect()
}

/// Divide el audio en chunks para procesamiento de archivos grandes
pub fn split_into_chunks(samples: &[f32]) -> Vec<Vec<f32>> {
    let chunk_size = (CHUNK_DURATION_SECS * WHISPER_SAMPLE_RATE as f32) as usize;
    samples
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

/// Calcula la duración del audio en segundos
pub fn calculate_duration(samples: &[f32]) -> f64 {
    samples.len() as f64 / WHISPER_SAMPLE_RATE as f64
}

/// Verifica si el audio es lo suficientemente largo para requerir procesamiento en chunks
pub fn needs_chunking(samples: &[f32]) -> bool {
    let duration = calculate_duration(samples);
    duration > 120.0 // > 2 minutos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_format() {
        assert!(is_supported_format("mp3"));
        assert!(is_supported_format("MP3"));
        assert!(is_supported_format("wav"));
        assert!(is_supported_format("mp4"));
        assert!(!is_supported_format("txt"));
        assert!(!is_supported_format("pdf"));
    }

    #[test]
    fn test_resample() {
        // Test simple: resamplear de 44100 a 16000
        let samples: Vec<f32> = (0..44100).map(|i| (i as f32 / 44100.0).sin()).collect();
        let resampled = resample(&samples, 44100, 16000);

        // Verificar que el tamaño es aproximadamente correcto
        let expected_len = (44100.0 / 44100.0 * 16000.0) as usize;
        assert!((resampled.len() as i32 - expected_len as i32).abs() < 10);
    }

    #[test]
    fn test_split_into_chunks() {
        // Crear 3 minutos de audio (180 segundos * 16000 samples/segundo)
        let samples: Vec<f32> = vec![0.0; 180 * 16000];
        let chunks = split_into_chunks(&samples);

        // Debería tener 3 chunks (60 segundos cada uno)
        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn test_needs_chunking() {
        // Audio corto (1 minuto)
        let short_audio: Vec<f32> = vec![0.0; 60 * 16000];
        assert!(!needs_chunking(&short_audio));

        // Audio largo (3 minutos)
        let long_audio: Vec<f32> = vec![0.0; 180 * 16000];
        assert!(needs_chunking(&long_audio));
    }
}
