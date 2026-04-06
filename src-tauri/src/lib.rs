mod commands;
mod peak_engine;
mod scheduler;
mod state;
mod stats_reader;
mod status_poller;
mod tray;

use std::sync::Arc;
use tauri::Manager;
use state::{AppState, AppStateWrapper};
use tokio::sync::Mutex as TokioMutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .manage(AppStateWrapper(std::sync::Mutex::new(AppState::default())))
        .invoke_handler(tauri::generate_handler![
            commands::get_peak_level,
            commands::get_stats,
            commands::get_service_status,
            commands::get_settings,
            commands::save_settings,
            commands::force_refresh,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // Do initial stats read
            {
                let state = handle.state::<AppStateWrapper>();
                let mut state_guard = state.0.lock().unwrap();
                state_guard.stats = stats_reader::read_stats();

                let peak_level = peak_engine::compute_peak_level(
                    &state_guard.stats,
                    &state_guard.service_status,
                    state_guard.previous_color,
                );
                state_guard.peak_level = peak_level;
            }

            // Setup tray icon
            let tray_icon = tray::setup_tray(&handle)?;

            // Update tray with initial state
            {
                let state = handle.state::<AppStateWrapper>();
                let state_guard = state.0.lock().unwrap();
                tray::update_tray(&tray_icon, state_guard.peak_level.color, state_guard.peak_level.score);
            }

            // Wrap tray for async access
            let tray_holder = Arc::new(scheduler::TrayHolder {
                tray: TokioMutex::new(Some(tray_icon)),
            });

            // Start background polling tasks
            scheduler::start_background_tasks(&handle, tray_holder);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Claude Peak Monitor");
}
