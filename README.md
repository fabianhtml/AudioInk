# AudioInk

Local audio transcription application powered by OpenAI's Whisper model. Built with Tauri 2 and Rust for native performance and privacy.

## Features

- **Local Processing**: All transcription happens on your device - your data never leaves your computer
- **Multiple Whisper Models**: Choose from Tiny, Base, Small, Medium, or Large-v3-Turbo based on your needs
- **Audio/Video Support**: Transcribe MP3, WAV, M4A, FLAC, OGG, MP4, MOV, MKV files
- **YouTube Captions**: Fetch captions directly from YouTube videos (auto-generated or manual)
- **Multi-language**: Auto-detect or specify the audio language (English, Spanish, French, German, Portuguese, Japanese, Chinese, and more)
- **Timestamps**: Optional timestamp markers in transcriptions `[HH:MM:SS]`
- **History**: Automatically saves transcriptions with metadata
- **Lightweight**: ~15MB app size with minimal resource usage

## Requirements

- macOS 10.15+ (Apple Silicon or Intel)
- ~150MB - 1.5GB disk space (depending on chosen Whisper model)

## Installation

### From Source

1. Install prerequisites:
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install Node.js (v18+)
   brew install node
   ```

2. Clone and build:
   ```bash
   git clone https://github.com/fabianhtml/audioink-rs.git
   cd audioink-rs
   npm install
   npm run tauri build
   ```

3. The app will be available in `src-tauri/target/release/bundle/`

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
