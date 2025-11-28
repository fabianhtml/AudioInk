//! Audio speedup functionality using ffmpeg
//!
//! This module provides functions to accelerate audio files using ffmpeg's atempo filter.
//! Maximum recommended speed is 2.0x to maintain transcription quality.

use crate::utils::{get_ffmpeg_install_instructions, AudioInkError, AudioInkResult};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if ffmpeg is available in the system
pub fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
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
    if !is_ffmpeg_available() {
        return Err(AudioInkError::Internal(
            get_ffmpeg_install_instructions().to_string()
        ));
    }

    // Create output path in temp directory
    let temp_dir = std::env::temp_dir();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let output_filename = format!("audioink_speedup_{}.wav", timestamp);
    let output_path = temp_dir.join(output_filename);

    // Build ffmpeg command
    // ffmpeg -i input.wav -filter:a "atempo=1.5" -vn output.wav
    let output = Command::new("ffmpeg")
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
