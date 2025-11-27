use regex::Regex;

/// Limpia el texto de subtítulos removiendo timestamps, tags HTML y marcadores
pub fn clean_subtitle_text(text: &str) -> String {
    // Regex para timestamps VTT/SRT
    let timestamp_re = Regex::new(r"\d{2}:\d{2}:\d{2}[.,]\d{3}\s*-->\s*\d{2}:\d{2}:\d{2}[.,]\d{3}").unwrap();
    let timestamp_inline_re = Regex::new(r"<\d{2}:\d{2}:\d{2}\.\d{3}>").unwrap();

    // Regex para tags HTML
    let html_tags_re = Regex::new(r"<[^>]+>").unwrap();

    // Regex para números de secuencia SRT
    let sequence_re = Regex::new(r"^\d+$").unwrap();

    // Regex para marcadores de sonido [Music], [Applause], etc.
    let sound_markers_re = Regex::new(r"\[[^\]]*\]").unwrap();

    // Regex para encabezados WEBVTT
    let webvtt_header_re = Regex::new(r"^WEBVTT.*$").unwrap();
    let kind_re = Regex::new(r"^Kind:.*$").unwrap();
    let language_re = Regex::new(r"^Language:.*$").unwrap();

    let mut result = String::new();

    for line in text.lines() {
        let line = line.trim();

        // Saltar líneas vacías
        if line.is_empty() {
            continue;
        }

        // Saltar encabezados WEBVTT
        if webvtt_header_re.is_match(line) {
            continue;
        }
        if kind_re.is_match(line) {
            continue;
        }
        if language_re.is_match(line) {
            continue;
        }

        // Saltar timestamps
        if timestamp_re.is_match(line) {
            continue;
        }

        // Saltar números de secuencia
        if sequence_re.is_match(line) {
            continue;
        }

        // Limpiar la línea
        let mut cleaned = line.to_string();

        // Remover timestamps inline
        cleaned = timestamp_inline_re.replace_all(&cleaned, "").to_string();

        // Remover tags HTML
        cleaned = html_tags_re.replace_all(&cleaned, "").to_string();

        // Remover marcadores de sonido
        cleaned = sound_markers_re.replace_all(&cleaned, "").to_string();

        // Limpiar espacios múltiples
        let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

        if !cleaned.is_empty() {
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(&cleaned);
        }
    }

    // Normalizar puntuación
    let result = result
        .replace(" .", ".")
        .replace(" ,", ",")
        .replace(" ?", "?")
        .replace(" !", "!")
        .replace("  ", " ");

    result.trim().to_string()
}

/// Detecta el idioma basado en el nombre del archivo de subtítulos
pub fn detect_language_from_filename(filename: &str) -> Option<String> {
    let filename_lower = filename.to_lowercase();

    // Patrones comunes: video.es.vtt, video.en.srt, etc.
    let lang_patterns = [
        ("es", "Spanish"),
        ("en", "English"),
        ("fr", "French"),
        ("de", "German"),
        ("it", "Italian"),
        ("pt", "Portuguese"),
        ("ja", "Japanese"),
        ("zh", "Chinese"),
        ("ko", "Korean"),
        ("ru", "Russian"),
    ];

    for (code, name) in lang_patterns {
        if filename_lower.contains(&format!(".{}.", code))
            || filename_lower.ends_with(&format!(".{}.vtt", code))
            || filename_lower.ends_with(&format!(".{}.srt", code))
        {
            return Some(name.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_subtitle_vtt() {
        let input = r#"WEBVTT
Kind: captions
Language: en

00:00:00.000 --> 00:00:05.000
Hello, this is a test.

00:00:05.000 --> 00:00:10.000
[Music] This is <b>another</b> line."#;

        let result = clean_subtitle_text(input);
        assert_eq!(result, "Hello, this is a test. This is another line.");
    }

    #[test]
    fn test_clean_subtitle_srt() {
        let input = r#"1
00:00:00,000 --> 00:00:05,000
Hello world.

2
00:00:05,000 --> 00:00:10,000
Second line here."#;

        let result = clean_subtitle_text(input);
        assert_eq!(result, "Hello world. Second line here.");
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language_from_filename("video.es.vtt"), Some("Spanish".to_string()));
        assert_eq!(detect_language_from_filename("video.en.srt"), Some("English".to_string()));
        assert_eq!(detect_language_from_filename("video.vtt"), None);
    }
}
