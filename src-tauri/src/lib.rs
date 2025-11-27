pub mod commands;
pub mod core;
pub mod models;
pub mod persistence;
pub mod utils;

use commands::{
    // Transcription commands
    transcribe_file,
    transcribe_youtube,
    get_languages,
    get_supported_formats,
    AppState,
    // History commands
    get_history,
    get_transcription,
    delete_transcription,
    clear_history,
    get_history_count,
    // Model commands
    list_models,
    get_downloaded_models,
    check_model_downloaded,
    download_whisper_model,
    delete_whisper_model,
    get_model_path_cmd,
    // YouTube commands
    check_youtube_captions,
    get_youtube_captions,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Transcription
            transcribe_file,
            transcribe_youtube,
            get_languages,
            get_supported_formats,
            // History
            get_history,
            get_transcription,
            delete_transcription,
            clear_history,
            get_history_count,
            // Models
            list_models,
            get_downloaded_models,
            check_model_downloaded,
            download_whisper_model,
            delete_whisper_model,
            get_model_path_cmd,
            // YouTube
            check_youtube_captions,
            get_youtube_captions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
