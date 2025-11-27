use crate::utils::{AudioInkError, AudioInkResult};
use std::path::PathBuf;
use std::process::Command;

/// Result of downloading YouTube audio
pub struct YouTubeDownloadResult {
    pub audio_path: PathBuf,
    pub title: String,
}

/// Check if yt-dlp is available in the system
pub fn is_ytdlp_available() -> bool {
    Command::new("yt-dlp")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Download audio from YouTube video using yt-dlp
pub fn download_youtube_audio(url: &str) -> AudioInkResult<YouTubeDownloadResult> {
    if !is_ytdlp_available() {
        return Err(AudioInkError::Internal(
            "yt-dlp is not installed. Please install it with: brew install yt-dlp".to_string()
        ));
    }

    // Create temp directory for download
    let temp_dir = std::env::temp_dir().join("audioink_youtube");
    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        AudioInkError::Internal(format!("Failed to create temp directory: {}", e))
    })?;

    // First, get the video title
    let title_output = Command::new("yt-dlp")
        .args(["--get-title", url])
        .output()
        .map_err(|e| AudioInkError::Internal(format!("Failed to get video title: {}", e)))?;

    let title = if title_output.status.success() {
        String::from_utf8_lossy(&title_output.stdout)
            .trim()
            .to_string()
    } else {
        "YouTube Video".to_string()
    };

    // Sanitize title for filename
    let safe_title: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' })
        .collect();
    let safe_title = safe_title.trim().to_string();
    let safe_title = if safe_title.is_empty() { "audio".to_string() } else { safe_title };

    let output_template = temp_dir.join(format!("{}.%(ext)s", safe_title));

    // Download audio only in best quality, convert to wav for whisper
    let output = Command::new("yt-dlp")
        .args([
            "-x",                           // Extract audio
            "--audio-format", "wav",        // Convert to WAV (best for whisper)
            "--audio-quality", "0",         // Best quality
            "-o", output_template.to_str().unwrap(),
            "--no-playlist",                // Don't download playlist
            "--no-warnings",
            url,
        ])
        .output()
        .map_err(|e| AudioInkError::Internal(format!("Failed to run yt-dlp: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AudioInkError::Internal(format!(
            "yt-dlp failed: {}",
            stderr
        )));
    }

    // Find the downloaded file
    let audio_path = temp_dir.join(format!("{}.wav", safe_title));

    if !audio_path.exists() {
        // Try to find any audio file in the temp directory
        let entries = std::fs::read_dir(&temp_dir)
            .map_err(|e| AudioInkError::Internal(format!("Failed to read temp dir: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "wav" || ext == "m4a" || ext == "mp3" || ext == "webm" || ext == "opus" {
                    return Ok(YouTubeDownloadResult {
                        audio_path: path,
                        title,
                    });
                }
            }
        }

        return Err(AudioInkError::Internal(
            "Downloaded audio file not found".to_string()
        ));
    }

    Ok(YouTubeDownloadResult {
        audio_path,
        title,
    })
}

/// Clean up downloaded files
pub fn cleanup_youtube_audio(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
}
