//! Audio processing functionality using ffmpeg
//!
//! This module provides functions to:
//! - Accelerate audio files using ffmpeg's atempo filter
//! - Extract audio from video files (mp4, avi, mov)
//! Maximum recommended speed is 2.0x to maintain transcription quality.

use crate::models::VIDEO_FORMATS;
use crate::utils::{get_ffmpeg_install_instructions, AudioInkError, AudioInkResult};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Common paths where ffmpeg might be installed
const FFMPEG_PATHS: &[&str] = &[
    "ffmpeg",
    "/opt/homebrew/bin/ffmpeg",
    "/usr/local/bin/ffmpeg",
    "/usr/bin/ffmpeg",
];

/// Find the ffmpeg binary path
fn find_ffmpeg() -> Option<&'static str> {
    for path in FFMPEG_PATHS {
        if Command::new(path)
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(path);
        }
    }
    None
}

/// Check if ffmpeg is available in the system
pub fn is_ffmpeg_available() -> bool {
    find_ffmpeg().is_some()
}

/// Apply audio speedup using ffmpeg's atempo filter
///
/// # Arguments
/// * `input_path` - Path to the input audio file
/// * `speed` - Speed factor (1.0 = normal, 2.0 = 2x faster). Max recommended: 2.0
///
/// # Returns
/// * `PathBuf` - Path to the sped-up temporary audio file
///
/// # Note
/// The caller is responsible for cleaning up the temporary file after use
pub fn apply_audio_speedup(input_path: &Path, speed: f32) -> AudioInkResult<PathBuf> {
    // Validate speed range (atempo filter supports 0.5 to 2.0)
    if speed < 0.5 || speed > 2.0 {
        return Err(AudioInkError::Internal(format!(
            "Speed must be between 0.5 and 2.0, got: {}",
            speed
        )));
    }

    // If speed is 1.0, no processing needed
    if (speed - 1.0).abs() < 0.01 {
        return Ok(input_path.to_path_buf());
    }

    // Check ffmpeg availability
    let ffmpeg = find_ffmpeg().ok_or_else(|| {
        AudioInkError::Internal(get_ffmpeg_install_instructions().to_string())
    })?;

    // Create output path in temp directory
    let temp_dir = std::env::temp_dir();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let output_filename = format!("audioink_speedup_{}.wav", timestamp);
    let output_path = temp_dir.join(output_filename);

    // Build ffmpeg command
    // ffmpeg -i input.wav -filter:a "atempo=1.5" -vn output.wav
    let output = Command::new(ffmpeg)
        .arg("-i")
        .arg(input_path)
        .arg("-filter:a")
        .arg(format!("atempo={}", speed))
        .arg("-vn") // No video
        .arg("-y") // Overwrite output
        .arg(&output_path)
        .output()
        .map_err(|e| AudioInkError::Internal(format!("Failed to run ffmpeg: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AudioInkError::Internal(format!(
            "ffmpeg speedup failed: {}",
            stderr
        )));
    }

    Ok(output_path)
}

/// Clean up a temporary speedup file
pub fn cleanup_speedup_file(path: &Path) {
    // Only delete if it's in temp directory and matches our naming pattern
    if path.to_string_lossy().contains("audioink_speedup_") {
        let _ = std::fs::remove_file(path);
    }
}

/// Adjust a timestamp (in milliseconds) by the speed factor
/// When audio is sped up, timestamps need to be multiplied by the speed factor
/// to represent the original audio time
pub fn adjust_timestamp_for_speed(timestamp_ms: i64, speed: f32) -> i64 {
    ((timestamp_ms as f64) * (speed as f64)).round() as i64
}

/// Check if a file is a video format that needs audio extraction
pub fn is_video_format(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIDEO_FORMATS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Extract audio from a video file using ffmpeg
///
/// # Arguments
/// * `input_path` - Path to the input video file (mp4, avi, mov, etc.)
///
/// # Returns
/// * `PathBuf` - Path to the extracted audio file (wav format)
///
/// # Note
/// The caller is responsible for cleaning up the temporary file after use
pub fn extract_audio_from_video(input_path: &Path) -> AudioInkResult<PathBuf> {
    // Check ffmpeg availability
    let ffmpeg = find_ffmpeg().ok_or_else(|| {
        AudioInkError::Internal(get_ffmpeg_install_instructions().to_string())
    })?;

    // Create output path in temp directory
    let temp_dir = std::env::temp_dir();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let output_filename = format!("audioink_extracted_{}.wav", timestamp);
    let output_path = temp_dir.join(output_filename);

    // Build ffmpeg command to extract audio
    // ffmpeg -i input.mp4 -vn -acodec pcm_s16le -ar 16000 -ac 1 output.wav
    let output = Command::new(ffmpeg)
        .arg("-i")
        .arg(input_path)
        .arg("-vn") // No video
        .arg("-acodec")
        .arg("pcm_s16le") // PCM 16-bit little-endian
        .arg("-ar")
        .arg("16000") // 16kHz sample rate (Whisper's requirement)
        .arg("-ac")
        .arg("1") // Mono
        .arg("-y") // Overwrite output
        .arg(&output_path)
        .output()
        .map_err(|e| AudioInkError::Internal(format!("Failed to run ffmpeg: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AudioInkError::Internal(format!(
            "ffmpeg audio extraction failed: {}",
            stderr
        )));
    }

    Ok(output_path)
}

/// Clean up a temporary extracted audio file
pub fn cleanup_extracted_audio(path: &Path) {
    // Only delete if it's in temp directory and matches our naming pattern
    if path.to_string_lossy().contains("audioink_extracted_") {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_available() {
        // This will depend on the system, just ensure it doesn't panic
        let _ = is_ffmpeg_available();
    }

    #[test]
    fn test_timestamp_adjustment() {
        // 1 minute at 1.5x speed should become 1.5 minutes
        assert_eq!(adjust_timestamp_for_speed(60000, 1.5), 90000);

        // 2 minutes at 2x speed should become 4 minutes
        assert_eq!(adjust_timestamp_for_speed(120000, 2.0), 240000);

        // No change at 1x speed
        assert_eq!(adjust_timestamp_for_speed(60000, 1.0), 60000);
    }

    #[test]
    fn test_speed_validation() {
        // Speed too low
        let result = apply_audio_speedup(Path::new("/tmp/test.wav"), 0.4);
        assert!(result.is_err());

        // Speed too high
        let result = apply_audio_speedup(Path::new("/tmp/test.wav"), 2.5);
        assert!(result.is_err());
    }
}
