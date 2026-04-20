mod commands;
mod peak_engine;
mod scheduler;
mod state;
mod stats_reader;
mod status_poller;
mod subscription_tracker;
mod tray;

use std::sync::Arc;
use tauri::Manager;
use state::{AppState, AppStateWrapper};
use tokio::sync::Mutex as TokioMutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single-instance plugin MUST be first — before any window or
        // state setup — so a second process detects the first one, hands
        // over its argv, and exits cleanly without ever creating a tray
        // icon. Prevents the "two yellow dots" a user can get if autostart
        // and manual launch race, or if the shell fires the autostart
        // entry twice during a flaky logon.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Second instance detected. Focus the existing popup so the
            // user sees something happen. If the popup window exists,
            // show + focus it; otherwise there's nothing to surface.
            if let Some(popup) = app.get_webview_window("popup") {
                let _ = popup.show();
                let _ = popup.set_focus();
            }
        }))
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
            commands::get_project_analytics,
            commands::get_analytics_for_range,
            commands::get_subscription_usage,
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
                let mut state_guard = state.lock();
                let (stats, analytics, samples) = stats_reader::read_all_with_samples();
                state_guard.stats = stats;
                state_guard.analytics = analytics;
                state_guard.subscription_usage =
                    subscription_tracker::compute(&samples, &state_guard.settings);

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
                let state_guard = state.lock();
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
        .expect("error while running Claude Consume and Peak Monitor");
}
