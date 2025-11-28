//! Platform-specific utilities
//!
//! Provides platform-aware installation instructions for external dependencies.

/// Get the installation command for yt-dlp based on the current platform
pub fn get_ytdlp_install_instructions() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "yt-dlp is not installed. Please install it with: brew install yt-dlp"
    }
    #[cfg(target_os = "linux")]
    {
        "yt-dlp is not installed. Please install it with your package manager:\n  \
         Ubuntu/Debian: sudo apt install yt-dlp\n  \
         Fedora: sudo dnf install yt-dlp\n  \
         Arch: sudo pacman -S yt-dlp"
    }
    #[cfg(target_os = "windows")]
    {
        "yt-dlp is not installed. Please install it with: winget install yt-dlp"
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "yt-dlp is not installed. Please install it from: https://github.com/yt-dlp/yt-dlp"
    }
}

/// Get the installation command for ffmpeg based on the current platform
pub fn get_ffmpeg_install_instructions() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "ffmpeg is not installed. Please install it with: brew install ffmpeg"
    }
    #[cfg(target_os = "linux")]
    {
        "ffmpeg is not installed. Please install it with your package manager:\n  \
         Ubuntu/Debian: sudo apt install ffmpeg\n  \
         Fedora: sudo dnf install ffmpeg\n  \
         Arch: sudo pacman -S ffmpeg"
    }
    #[cfg(target_os = "windows")]
    {
        "ffmpeg is not installed. Please install it with: winget install ffmpeg"
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "ffmpeg is not installed. Please install it from: https://ffmpeg.org/download.html"
    }
}
