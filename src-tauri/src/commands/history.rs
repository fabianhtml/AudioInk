use crate::commands::transcription::AppState;
use crate::models::TranscriptionEntry;
use tauri::State;

/// Obtiene todo el historial de transcripciones
#[tauri::command]
pub async fn get_history(
    state: State<'_, AppState>,
) -> Result<Vec<TranscriptionEntry>, String> {
    state
        .history_manager
        .load_history()
        .await
        .map_err(|e| e.to_string())
}

/// Obtiene una transcripción específica por ID
#[tauri::command]
pub async fn get_transcription(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<TranscriptionEntry>, String> {
    state
        .history_manager
        .get_transcription(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Elimina una transcripción del historial
#[tauri::command]
pub async fn delete_transcription(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    state
        .history_manager
        .delete_transcription(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Elimina todo el historial
#[tauri::command]
pub async fn clear_history(
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .history_manager
        .clear_all()
        .await
        .map_err(|e| e.to_string())
}

/// Obtiene el número de transcripciones en el historial
#[tauri::command]
pub async fn get_history_count(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    state
        .history_manager
        .count()
        .await
        .map_err(|e| e.to_string())
}
