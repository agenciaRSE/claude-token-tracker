use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex as TokioMutex;

use crate::peak_engine::compute_peak_level;
use crate::state::{AppStateWrapper, CostMode, PeakColor};
use crate::stats_reader;
use crate::status_poller;
use crate::subscription_tracker;
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

    // Task 2: Poll stats + analytics + subscription usage every 30 seconds
    tauri::async_runtime::spawn(async move {
        loop {
            // SECURITY: wrap the blocking file scan in spawn_blocking so it
            // doesn't stall the Tokio async worker thread pool.
            let (stats, analytics, samples) =
                tokio::task::spawn_blocking(stats_reader::read_all_with_samples)
                    .await
                    .unwrap_or_default();

            // Update state and recompute peak level (scoped to drop MutexGuard before await)
            let (
                peak_level,
                should_notify,
                should_alert_tokens,
                today_tokens,
                stats_poll_secs,
                subscription_usage,
                session_warning,
                week_warning,
            ) = {
                let state = app_handle2.state::<AppStateWrapper>();
                let mut guard = state.lock();
                guard.stats = stats.clone();
                guard.analytics = analytics.clone();

                // Subscription usage — always computed, UI shows it only in
                // Subscription cost mode but the backend keeps it fresh.
                let subscription_usage =
                    subscription_tracker::compute(&samples, &guard.settings);
                guard.subscription_usage = subscription_usage.clone();

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
                let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();

                // SECURITY: Fire the daily token alert at most once per
                // calendar day to prevent notification spam every poll cycle.
                let should_alert_tokens = guard.settings.daily_token_alert
                    .map(|threshold| {
                        today_tokens >= threshold
                            && guard.settings.notifications_enabled
                            && guard.token_alert_fired_today.as_deref() != Some(&today_str)
                    })
                    .unwrap_or(false);
                if should_alert_tokens {
                    guard.token_alert_fired_today = Some(today_str);
                }

                // Subscription warnings: fire once per session / per week
                // when crossing the configured threshold, and only when the
                // user is actually tracking subscription usage.
                let warn_pct = guard.settings.subscription_warn_pct as u16;
                let sub_notifs_on = guard.settings.notifications_enabled
                    && guard.settings.subscription_warnings_enabled
                    && guard.settings.cost_mode == CostMode::Subscription;

                let session_warning = if sub_notifs_on
                    && subscription_usage.session_active
                    && subscription_usage.session_pct >= warn_pct
                    && guard.subscription_session_warned != subscription_usage.session_start
                {
                    guard.subscription_session_warned = subscription_usage.session_start.clone();
                    Some((
                        subscription_usage.session_pct,
                        subscription_usage.session_seconds_until_reset,
                    ))
                } else {
                    None
                };

                let week_warning = if sub_notifs_on
                    && subscription_usage.week_pct >= warn_pct
                    && guard.subscription_week_warned != subscription_usage.week_start
                {
                    guard.subscription_week_warned = subscription_usage.week_start.clone();
                    Some((
                        subscription_usage.week_pct,
                        subscription_usage.week_seconds_until_reset,
                    ))
                } else {
                    None
                };

                // Stats file scans run at 1/4 the service-poll cadence, min 15s,
                // so we respect the user's preference without hammering the disk.
                let stats_poll_secs = (guard.settings.refresh_interval_secs / 4).max(15);

                (
                    peak_level,
                    should_notify,
                    should_alert_tokens,
                    today_tokens,
                    stats_poll_secs,
                    subscription_usage,
                    session_warning,
                    week_warning,
                )
            }; // MutexGuard dropped here

            // Update tray icon (async)
            if let Some(tray) = tray_holder_stats.tray.lock().await.as_ref() {
                tray::update_tray(tray, peak_level.color, peak_level.score);
            }

            // Emit events to frontend
            let _ = app_handle2.emit("peak-level-changed", &peak_level);
            let _ = app_handle2.emit("stats-updated", &stats);
            let _ = app_handle2.emit("analytics-updated", &analytics);
            let _ = app_handle2.emit("subscription-updated", &subscription_usage);

            // Notify on color change
            if should_notify {
                send_color_change_notification(&app_handle2, &peak_level.color);
            }

            // Token alert (fires at most once per calendar day)
            if should_alert_tokens {
                let _ = app_handle2.emit("token-alert", today_tokens);
            }

            // Subscription threshold warnings (fire once per session/week)
            if let Some((pct, secs)) = session_warning {
                let _ = app_handle2.emit(
                    "subscription-warning",
                    serde_json::json!({ "scope": "session", "pct": pct, "secondsToReset": secs }),
                );
            }
            if let Some((pct, secs)) = week_warning {
                let _ = app_handle2.emit(
                    "subscription-warning",
                    serde_json::json!({ "scope": "week", "pct": pct, "secondsToReset": secs }),
                );
            }

            tokio::time::sleep(Duration::from_secs(stats_poll_secs)).await;
        }
    });
}

fn send_color_change_notification(app: &AppHandle, color: &PeakColor) {
    let title = format!("Claude Consume and Peak Monitor - {}", color.label());
    let body = color.recommendation().to_string();
    let _ = app.emit("show-notification", serde_json::json!({
        "title": title,
        "body": body,
    }));
}
