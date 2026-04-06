use tauri::State;

use crate::peak_engine::compute_peak_level;
use crate::state::{AppStateWrapper, ClaudeStats, PeakLevel, ServiceStatus, UserSettings};
use crate::stats_reader;
use crate::status_poller;

/// Get the current peak level
#[tauri::command]
pub fn get_peak_level(state: State<'_, AppStateWrapper>) -> PeakLevel {
    state.0.lock().unwrap().peak_level.clone()
}

/// Get the current Claude stats
#[tauri::command]
pub fn get_stats(state: State<'_, AppStateWrapper>) -> ClaudeStats {
    state.0.lock().unwrap().stats.clone()
}

/// Get the current service status
#[tauri::command]
pub fn get_service_status(state: State<'_, AppStateWrapper>) -> ServiceStatus {
    state.0.lock().unwrap().service_status.clone()
}

/// Get user settings
#[tauri::command]
pub fn get_settings(state: State<'_, AppStateWrapper>) -> UserSettings {
    state.0.lock().unwrap().settings.clone()
}

/// Save user settings
#[tauri::command]
pub fn save_settings(state: State<'_, AppStateWrapper>, settings: UserSettings) {
    state.0.lock().unwrap().settings = settings;
}

/// Force refresh all data sources
#[tauri::command]
pub async fn force_refresh(state: State<'_, AppStateWrapper>) -> Result<PeakLevel, String> {
    // Fetch fresh data
    let stats = stats_reader::read_stats();
    let service_status = status_poller::fetch_service_status().await;

    // Update state
    let mut state_guard = state.0.lock().unwrap();
    state_guard.stats = stats;
    state_guard.service_status = service_status;

    let peak_level = compute_peak_level(
        &state_guard.stats,
        &state_guard.service_status,
        state_guard.previous_color,
    );

    state_guard.previous_color = peak_level.color;
    state_guard.peak_level = peak_level.clone();

    Ok(peak_level)
}
