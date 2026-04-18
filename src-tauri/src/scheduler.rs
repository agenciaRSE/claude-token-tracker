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
            let (
                peak_level,
                color_changed,
                should_notify,
                refresh_secs,
                sound_peak_change,
                sound_volume,
                sounds_enabled,
            ) = {
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

                let refresh_secs = guard.settings.refresh_interval_secs;
                let sound_peak_change = guard.settings.sound_peak_change.clone();
                let sound_volume = guard.settings.sound_volume;
                let sounds_enabled = guard.settings.sounds_enabled;

                (
                    peak_level,
                    color_changed,
                    should_notify,
                    refresh_secs,
                    sound_peak_change,
                    sound_volume,
                    sounds_enabled,
                )
            }; // MutexGuard dropped here

            // Update tray icon (async - needs MutexGuard to be dropped)
            if let Some(tray) = tray_holder_status.tray.lock().await.as_ref() {
                tray::update_tray(tray, peak_level.color, peak_level.score);
            }

            // Emit events to frontend
            let _ = app_handle.emit("peak-level-changed", &peak_level);
            let _ = app_handle.emit("service-status-updated", &service_status);

            // Notify + play sound on color change
            if should_notify {
                send_color_change_notification(&app_handle, &peak_level.color);
            }
            if color_changed && sounds_enabled {
                emit_play_sound(&app_handle, &sound_peak_change, sound_volume);
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
            let decisions = {
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

                let peak_notify = color_changed
                    && guard.settings.notifications_enabled
                    && guard.settings.notify_on_color_change;

                let today_tokens = guard.stats.today_tokens;
                let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();

                // SECURITY: Fire the daily token alert at most once per
                // calendar day to prevent notification spam every poll cycle.
                let alert_daily_tokens = guard.settings.daily_token_alert
                    .map(|threshold| {
                        today_tokens >= threshold
                            && guard.settings.notifications_enabled
                            && guard.token_alert_fired_today.as_deref() != Some(&today_str)
                    })
                    .unwrap_or(false);
                if alert_daily_tokens {
                    guard.token_alert_fired_today = Some(today_str);
                }

                // Subscription alerts only fire in Subscription cost mode.
                let sub_mode = guard.settings.cost_mode == CostMode::Subscription;
                let notifs_on = guard.settings.notifications_enabled;

                // ── Session START detection ───────────────────────
                // Fires when we see an active session whose session_start
                // we haven't alerted about before. One-shot per 5h window.
                let session_started = sub_mode
                    && notifs_on
                    && guard.settings.alert_session_start
                    && subscription_usage.session_active
                    && guard.session_start_alerted != subscription_usage.session_start;
                if session_started {
                    guard.session_start_alerted = subscription_usage.session_start.clone();
                }

                // ── Session END detection ─────────────────────────
                // Transition from an active session → no active session.
                let session_ended = sub_mode
                    && notifs_on
                    && guard.settings.alert_session_end
                    && guard.had_active_session
                    && !subscription_usage.session_active;
                guard.had_active_session = subscription_usage.session_active;

                // ── Multi-threshold usage warnings (session) ──────
                // Reset the fired list when we're on a new session.
                if guard.fired_session_thresholds_key != subscription_usage.session_start {
                    guard.fired_session_thresholds.clear();
                    guard.fired_session_thresholds_key = subscription_usage.session_start.clone();
                }
                let mut session_thresholds_crossed: Vec<u8> = Vec::new();
                if sub_mode
                    && notifs_on
                    && guard.settings.subscription_warnings_enabled
                    && subscription_usage.session_active
                {
                    // Clone to avoid holding a borrow of `guard` while mutating.
                    let thresholds = guard.settings.usage_warning_thresholds.clone();
                    for t in thresholds {
                        if subscription_usage.session_pct >= t as u16
                            && !guard.fired_session_thresholds.contains(&t)
                        {
                            guard.fired_session_thresholds.push(t);
                            session_thresholds_crossed.push(t);
                        }
                    }
                }

                // ── Multi-threshold usage warnings (week) ─────────
                if guard.fired_week_thresholds_key != subscription_usage.week_start {
                    guard.fired_week_thresholds.clear();
                    guard.fired_week_thresholds_key = subscription_usage.week_start.clone();
                }
                let mut week_thresholds_crossed: Vec<u8> = Vec::new();
                if sub_mode
                    && notifs_on
                    && guard.settings.subscription_warnings_enabled
                {
                    let thresholds = guard.settings.usage_warning_thresholds.clone();
                    for t in thresholds {
                        if subscription_usage.week_pct >= t as u16
                            && !guard.fired_week_thresholds.contains(&t)
                        {
                            guard.fired_week_thresholds.push(t);
                            week_thresholds_crossed.push(t);
                        }
                    }
                }

                // Stats file scans run at 1/4 the service-poll cadence, min 15s,
                // so we respect the user's preference without hammering the disk.
                let stats_poll_secs = (guard.settings.refresh_interval_secs / 4).max(15);

                AlertDecisions {
                    peak_level,
                    peak_color_changed: color_changed,
                    peak_notify,
                    alert_daily_tokens,
                    today_tokens,
                    stats_poll_secs,
                    subscription_usage,
                    session_started,
                    session_ended,
                    session_thresholds_crossed,
                    week_thresholds_crossed,
                    sounds_enabled: guard.settings.sounds_enabled,
                    sound_volume: guard.settings.sound_volume,
                    sound_peak_change: guard.settings.sound_peak_change.clone(),
                    sound_session_start: guard.settings.sound_session_start.clone(),
                    sound_session_end: guard.settings.sound_session_end.clone(),
                    sound_usage_threshold: guard.settings.sound_usage_threshold.clone(),
                }
            }; // MutexGuard dropped here

            // Update tray icon (async)
            if let Some(tray) = tray_holder_stats.tray.lock().await.as_ref() {
                tray::update_tray(tray, decisions.peak_level.color, decisions.peak_level.score);
            }

            // Emit events to frontend
            let _ = app_handle2.emit("peak-level-changed", &decisions.peak_level);
            let _ = app_handle2.emit("stats-updated", &stats);
            let _ = app_handle2.emit("analytics-updated", &analytics);
            let _ = app_handle2.emit("subscription-updated", &decisions.subscription_usage);

            // ── Notifications + sounds ────────────────────────────────
            if decisions.peak_notify {
                send_color_change_notification(&app_handle2, &decisions.peak_level.color);
            }
            if decisions.peak_color_changed && decisions.sounds_enabled {
                emit_play_sound(&app_handle2, &decisions.sound_peak_change, decisions.sound_volume);
            }

            if decisions.alert_daily_tokens {
                let _ = app_handle2.emit("token-alert", decisions.today_tokens);
                // Daily token alert shares the "threshold" sound channel.
                if decisions.sounds_enabled {
                    emit_play_sound(&app_handle2, &decisions.sound_usage_threshold, decisions.sound_volume);
                }
            }

            if decisions.session_started {
                send_session_notification(&app_handle2, true, &decisions.subscription_usage.session_end);
                if decisions.sounds_enabled {
                    emit_play_sound(&app_handle2, &decisions.sound_session_start, decisions.sound_volume);
                }
            }
            if decisions.session_ended {
                send_session_notification(&app_handle2, false, &None);
                if decisions.sounds_enabled {
                    emit_play_sound(&app_handle2, &decisions.sound_session_end, decisions.sound_volume);
                }
            }

            // Fire one notification per threshold crossed this poll. Emitting
            // multiple back-to-back is fine — in practice users only cross
            // thresholds one at a time unless large token spikes happen.
            for pct in &decisions.session_thresholds_crossed {
                let _ = app_handle2.emit(
                    "subscription-warning",
                    serde_json::json!({
                        "scope": "session",
                        "pct": *pct,
                        "secondsToReset": decisions.subscription_usage.session_seconds_until_reset,
                    }),
                );
                send_usage_threshold_notification(
                    &app_handle2,
                    "5-hour session",
                    *pct,
                    decisions.subscription_usage.session_seconds_until_reset,
                );
                if decisions.sounds_enabled {
                    emit_play_sound(&app_handle2, &decisions.sound_usage_threshold, decisions.sound_volume);
                }
            }
            for pct in &decisions.week_thresholds_crossed {
                let _ = app_handle2.emit(
                    "subscription-warning",
                    serde_json::json!({
                        "scope": "week",
                        "pct": *pct,
                        "secondsToReset": decisions.subscription_usage.week_seconds_until_reset,
                    }),
                );
                send_usage_threshold_notification(
                    &app_handle2,
                    "Weekly window",
                    *pct,
                    decisions.subscription_usage.week_seconds_until_reset,
                );
                if decisions.sounds_enabled {
                    emit_play_sound(&app_handle2, &decisions.sound_usage_threshold, decisions.sound_volume);
                }
            }

            tokio::time::sleep(Duration::from_secs(decisions.stats_poll_secs)).await;
        }
    });
}

/// Bundle of decisions made under the state lock so we can emit events
/// outside of it. Packaging into a struct avoids a 15-element tuple.
struct AlertDecisions {
    peak_level: crate::state::PeakLevel,
    peak_color_changed: bool,
    peak_notify: bool,
    alert_daily_tokens: bool,
    today_tokens: u64,
    stats_poll_secs: u64,
    subscription_usage: crate::state::SubscriptionUsage,
    session_started: bool,
    session_ended: bool,
    session_thresholds_crossed: Vec<u8>,
    week_thresholds_crossed: Vec<u8>,
    sounds_enabled: bool,
    sound_volume: u8,
    sound_peak_change: String,
    sound_session_start: String,
    sound_session_end: String,
    sound_usage_threshold: String,
}

fn emit_play_sound(app: &AppHandle, sound_id: &str, volume: u8) {
    if sound_id == "none" {
        return;
    }
    let _ = app.emit(
        "play-sound",
        serde_json::json!({ "soundId": sound_id, "volume": volume }),
    );
}

fn send_session_notification(app: &AppHandle, started: bool, session_end: &Option<String>) {
    let (title, body) = if started {
        let body = match session_end {
            Some(end) => format!("A fresh 5-hour window just started. Resets at {}.", end),
            None => "A fresh 5-hour window just started.".to_string(),
        };
        ("Claude session started".to_string(), body)
    } else {
        (
            "Claude session ended".to_string(),
            "Your 5-hour subscription window expired. The next message will start a new one.".to_string(),
        )
    };
    let _ = app.emit(
        "show-notification",
        serde_json::json!({ "title": title, "body": body }),
    );
}

fn send_usage_threshold_notification(app: &AppHandle, scope: &str, pct: u8, seconds_to_reset: i64) {
    let hrs = (seconds_to_reset.max(0) / 3600) as i64;
    let mins = ((seconds_to_reset.max(0) % 3600) / 60) as i64;
    let reset = if hrs > 0 {
        format!("{}h {}m", hrs, mins)
    } else {
        format!("{}m", mins)
    };
    let title = format!("Claude usage at {}%", pct);
    let body = format!(
        "{} is at {}% of your plan limit. Resets in {}.",
        scope, pct, reset,
    );
    let _ = app.emit(
        "show-notification",
        serde_json::json!({ "title": title, "body": body }),
    );
}

fn send_color_change_notification(app: &AppHandle, color: &PeakColor) {
    let title = format!("Claude Consume and Peak Monitor - {}", color.label());
    let body = color.recommendation().to_string();
    let _ = app.emit("show-notification", serde_json::json!({
        "title": title,
        "body": body,
    }));
}
