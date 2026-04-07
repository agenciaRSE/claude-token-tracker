use tauri::State;

use crate::peak_engine::compute_peak_level;
use crate::state::{AppStateWrapper, ClaudeStats, PeakLevel, ServiceStatus, UserSettings};
use crate::stats_reader;
use crate::status_poller;

/// Get the current peak level
#[tauri::command]
pub fn get_peak_level(state: State<'_, AppStateWrapper>) -> PeakLevel {
    state.lock().peak_level.clone()
}

/// Get the current Claude stats
#[tauri::command]
pub fn get_stats(state: State<'_, AppStateWrapper>) -> ClaudeStats {
    state.lock().stats.clone()
}

/// Get the current service status
#[tauri::command]
pub fn get_service_status(state: State<'_, AppStateWrapper>) -> ServiceStatus {
    state.lock().service_status.clone()
}

/// Get user settings
#[tauri::command]
pub fn get_settings(state: State<'_, AppStateWrapper>) -> UserSettings {
    state.lock().settings.clone()
}

/// Save user settings. Values are clamped/validated so a corrupted store file
/// or a malicious renderer message can't put the backend into a bad state.
#[tauri::command]
pub fn save_settings(state: State<'_, AppStateWrapper>, settings: UserSettings) -> Result<(), String> {
    let sanitized = validate_settings(settings)?;
    state.lock().settings = sanitized;
    Ok(())
}

/// Force refresh all data sources
#[tauri::command]
pub async fn force_refresh(state: State<'_, AppStateWrapper>) -> Result<PeakLevel, String> {
    // Fetch fresh data
    let stats = stats_reader::read_stats();
    let service_status = status_poller::fetch_service_status().await;

    // Update state
    let mut state_guard = state.lock();
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

/// Clamp + sanitize a UserSettings payload. We don't trust that the value
/// arriving from the frontend (or a hand-edited store.json) is well-formed:
///  * `timezone` is length-capped and restricted to a conservative charset
///    (`A-Z`, `a-z`, `0-9`, `_`, `+`, `-`, `/`) so it can't smuggle control
///    characters or wildly long strings into our state.
///  * `refresh_interval_secs` is clamped to [10, 3600] so the user can't
///    accidentally (or maliciously) pin the CPU with a 0s loop or disable
///    polling entirely with u64::MAX.
///  * `daily_token_alert`, when present, is clamped to a sane upper bound to
///    avoid nonsense values showing up on the dashboard.
fn validate_settings(mut s: UserSettings) -> Result<UserSettings, String> {
    const MAX_TIMEZONE_LEN: usize = 64;
    const MIN_REFRESH: u64 = 10;
    const MAX_REFRESH: u64 = 3600;
    const MAX_DAILY_TOKEN_ALERT: u64 = 1_000_000_000;

    let tz = s.timezone.trim();
    if tz.is_empty() {
        return Err("timezone must not be empty".to_string());
    }
    if tz.len() > MAX_TIMEZONE_LEN {
        return Err(format!("timezone must be <= {} characters", MAX_TIMEZONE_LEN));
    }
    if !tz
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-' | '/'))
    {
        return Err("timezone contains invalid characters".to_string());
    }
    s.timezone = tz.to_string();

    s.refresh_interval_secs = s.refresh_interval_secs.clamp(MIN_REFRESH, MAX_REFRESH);

    if let Some(threshold) = s.daily_token_alert {
        s.daily_token_alert = Some(threshold.min(MAX_DAILY_TOKEN_ALERT));
    }

    Ok(s)
}
