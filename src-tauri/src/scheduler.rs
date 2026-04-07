use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex as TokioMutex;

use crate::peak_engine::compute_peak_level;
use crate::state::{AppStateWrapper, PeakColor};
use crate::stats_reader;
use crate::status_poller;
use crate::tray;

/// Holds the tray reference for updating from async tasks
pub struct TrayHolder {
    pub tray: TokioMutex<Option<tauri::tray::TrayIcon>>,
}

/// Spawn background tasks for polling status + watching stats
pub fn start_background_tasks(app: &AppHandle, tray_holder: Arc<TrayHolder>) {
    let app_handle = app.clone();
    let tray_holder_status = tray_holder.clone();

    // Task 1: Poll Anthropic status page every 2 minutes
    tauri::async_runtime::spawn(async move {
        loop {
            // Fetch service status
            let service_status = status_poller::fetch_service_status().await;

            // Update state and recompute peak level (scoped to drop MutexGuard before await)
            let (peak_level, _color_changed, should_notify, refresh_secs) = {
                let state = app_handle.state::<AppStateWrapper>();
                let mut guard = state.lock();
                guard.service_status = service_status.clone();

                let peak_level = compute_peak_level(
                    &guard.stats,
                    &guard.service_status,
                    guard.previous_color,
                );

                let color_changed = peak_level.color != guard.previous_color;
                guard.previous_color = peak_level.color;
                guard.peak_level = peak_level.clone();

                let should_notify = color_changed
                    && guard.settings.notifications_enabled
                    && guard.settings.notify_on_color_change;

                // Honor the user's configured refresh interval (clamped in
                // validate_settings so we don't need to defend here).
                let refresh_secs = guard.settings.refresh_interval_secs;

                (peak_level, color_changed, should_notify, refresh_secs)
            }; // MutexGuard dropped here

            // Update tray icon (async - needs MutexGuard to be dropped)
            if let Some(tray) = tray_holder_status.tray.lock().await.as_ref() {
                tray::update_tray(tray, peak_level.color, peak_level.score);
            }

            // Emit events to frontend
            let _ = app_handle.emit("peak-level-changed", &peak_level);
            let _ = app_handle.emit("service-status-updated", &service_status);

            // Notify on color change
            if should_notify {
                send_color_change_notification(&app_handle, &peak_level.color);
            }

            tokio::time::sleep(Duration::from_secs(refresh_secs)).await;
        }
    });

    let app_handle2 = app.clone();
    let tray_holder_stats = tray_holder;

    // Task 2: Poll stats-cache.json every 30 seconds
    tauri::async_runtime::spawn(async move {
        loop {
            // Read local stats (synchronous file read)
            let stats = stats_reader::read_stats();

            // Update state and recompute peak level (scoped to drop MutexGuard before await)
            let (peak_level, should_notify, should_alert_tokens, today_tokens, stats_poll_secs) = {
                let state = app_handle2.state::<AppStateWrapper>();
                let mut guard = state.lock();
                guard.stats = stats.clone();

                let peak_level = compute_peak_level(
                    &guard.stats,
                    &guard.service_status,
                    guard.previous_color,
                );

                let color_changed = peak_level.color != guard.previous_color;
                guard.previous_color = peak_level.color;
                guard.peak_level = peak_level.clone();

                let should_notify = color_changed
                    && guard.settings.notifications_enabled
                    && guard.settings.notify_on_color_change;

                let today_tokens = guard.stats.today_tokens;
                let should_alert_tokens = guard.settings.daily_token_alert
                    .map(|threshold| today_tokens >= threshold && guard.settings.notifications_enabled)
                    .unwrap_or(false);

                // Stats file scans run at 1/4 the service-poll cadence, min 15s,
                // so we respect the user's preference without hammering the disk.
                let stats_poll_secs = (guard.settings.refresh_interval_secs / 4).max(15);

                (peak_level, should_notify, should_alert_tokens, today_tokens, stats_poll_secs)
            }; // MutexGuard dropped here

            // Update tray icon (async)
            if let Some(tray) = tray_holder_stats.tray.lock().await.as_ref() {
                tray::update_tray(tray, peak_level.color, peak_level.score);
            }

            // Emit events to frontend
            let _ = app_handle2.emit("peak-level-changed", &peak_level);
            let _ = app_handle2.emit("stats-updated", &stats);

            // Notify on color change
            if should_notify {
                send_color_change_notification(&app_handle2, &peak_level.color);
            }

            // Token alert
            if should_alert_tokens {
                let _ = app_handle2.emit("token-alert", today_tokens);
            }

            tokio::time::sleep(Duration::from_secs(stats_poll_secs)).await;
        }
    });
}

fn send_color_change_notification(app: &AppHandle, color: &PeakColor) {
    let title = format!("Claude Peak Monitor - {}", color.label());
    let body = color.recommendation().to_string();
    let _ = app.emit("show-notification", serde_json::json!({
        "title": title,
        "body": body,
    }));
}
