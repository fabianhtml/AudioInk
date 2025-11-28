# AudioInk

Local audio transcription application powered by OpenAI's Whisper model. Built with Tauri 2 and Rust for native performance and privacy.

## Features

- **Local Processing**: All transcription happens on your device - your data never leaves your computer
- **Multiple Whisper Models**: Choose from Tiny, Base, Small, Medium, or Large-v3-Turbo based on your needs
- **Audio/Video Support**: Transcribe MP3, WAV, M4A, FLAC, OGG, MP4, MOV, MKV files
- **YouTube Integration**: Fetch captions or transcribe with Whisper using yt-dlp
- **Audio Speedup**: Optional 1.25x-2.0x acceleration for faster transcription (requires ffmpeg)
- **Progressive Display**: See transcription results in real-time as chunks complete
- **Multi-language**: Auto-detect or specify the audio language (English, Spanish, French, German, Portuguese, Japanese, Chinese, and more)
- **Timestamps**: Optional timestamp markers in transcriptions `[HH:MM:SS]`
- **History**: Automatically saves transcriptions with metadata
- **Lightweight**: ~15MB app size with minimal resource usage

## Requirements

### macOS
- macOS 10.15+ (Apple Silicon or Intel)
- ~150MB - 1.5GB disk space (depending on chosen Whisper model)

### Linux
- Ubuntu 20.04+, Fedora 36+, or equivalent
- ~150MB - 1.5GB disk space (depending on chosen Whisper model)
- Build dependencies: `build-essential`, `cmake`, `libwebkit2gtk-4.1-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

### Windows
- Windows 10/11
- ~150MB - 1.5GB disk space (depending on chosen Whisper model)

### Optional Dependencies

| Tool | macOS | Linux | Windows |
|------|-------|-------|---------|
| **yt-dlp** (YouTube transcription) | `brew install yt-dlp` | `sudo apt install yt-dlp` | `winget install yt-dlp` |
| **ffmpeg** (audio speedup) | `brew install ffmpeg` | `sudo apt install ffmpeg` | `winget install ffmpeg` |

## Installation

### From Source

1. Install prerequisites:

   **macOS:**
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install Node.js (v18+)
   brew install node
   ```

   **Linux (Ubuntu/Debian):**
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install system dependencies
   sudo apt update
   sudo apt install -y build-essential cmake libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

   # Install Node.js (v18+)
   curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
   sudo apt install -y nodejs
   ```

   **Windows:**
   ```powershell
   # Install Rust from https://rustup.rs
   # Install Node.js from https://nodejs.org
   # Install Visual Studio Build Tools with C++ workload
   ```

2. Clone and build:
   ```bash
   git clone https://github.com/fabianhtml/audioink-rs.git
   cd audioink-rs
   npm install
   npm run tauri build
   ```

3. The app will be available in `src-tauri/target/release/bundle/`
   - **macOS:** `.dmg` and `.app`
   - **Linux:** `.deb`, `.rpm`, and `.AppImage`
   - **Windows:** `.msi` and `.exe`

### Development

```bash
npm install
npm run tauri dev
```

## Usage

1. **Download a Model**: Open Settings and download your preferred Whisper model
   - **Tiny** (75 MB): Fastest, lower accuracy
   - **Base** (142 MB): Good balance for most uses
   - **Small** (466 MB): Better accuracy
   - **Medium** (1.5 GB): High accuracy
   - **Turbo** (809 MB): Best quality, optimized for speed

2. **Transcribe Audio**:
   - **File Tab**: Click to select an audio/video file
   - **YouTube Tab**: Paste a YouTube URL to fetch captions

3. **Options**:
   - Select language or use auto-detect
   - Enable timestamps in Settings for time-marked output
   - Enable audio speedup (1.25x-2.0x) for faster processing (note: not compatible with timestamps)

4. **Export**: Copy to clipboard or save as text file

## Architecture

```
audioink-rs/
├── src/                    # Frontend (Vanilla JS)
│   ├── index.html
│   ├── main.js
│   └── styles.css
└── src-tauri/              # Backend (Rust)
    └── src/
        ├── commands/       # Tauri commands (API)
        ├── core/           # Whisper engine, audio processing
        ├── models/         # Data structures
        ├── persistence/    # History management
        └── utils/          # Error handling, helpers
```

## Tech Stack

- **Frontend**: Vanilla HTML/CSS/JavaScript
- **Backend**: Rust + Tauri 2
- **Transcription**: whisper-rs (Whisper.cpp bindings)
- **Audio Decoding**: Symphonia

## License

MIT

## Acknowledgments

- [OpenAI Whisper](https://github.com/openai/whisper) - Speech recognition model
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) - C++ port of Whisper
- [Tauri](https://tauri.app/) - Desktop app framework
