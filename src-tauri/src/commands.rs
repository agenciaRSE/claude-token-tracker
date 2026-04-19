use tauri::State;

use crate::peak_engine::compute_peak_level;
use crate::state::{
    AppStateWrapper, ClaudeStats, PeakLevel, ProjectAnalytics, ServiceStatus, SubscriptionUsage,
    TimeRange, UserSettings,
};
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

/// Get the current project analytics
#[tauri::command]
pub fn get_project_analytics(state: State<'_, AppStateWrapper>) -> ProjectAnalytics {
    state.lock().analytics.clone()
}

/// Get project analytics filtered by a specific time range. Performs a fresh
/// scan of the JSONL files since the cached baseline is always Last30Days.
/// Offloaded to spawn_blocking so it doesn't stall the async runtime.
#[tauri::command]
pub async fn get_analytics_for_range(range: TimeRange) -> Result<ProjectAnalytics, String> {
    let (_stats, analytics) = tokio::task::spawn_blocking(move || {
        stats_reader::read_all_with_range(range)
    })
    .await
    .map_err(|e| format!("analytics scan failed: {}", e))?;
    Ok(analytics)
}

/// Get the current subscription-plan usage snapshot (5h session + weekly).
#[tauri::command]
pub fn get_subscription_usage(state: State<'_, AppStateWrapper>) -> SubscriptionUsage {
    state.lock().subscription_usage.clone()
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
/// After saving, subscription usage is recomputed since the plan/limits/
/// reset schedule changed the percentages displayed.
#[tauri::command]
pub fn save_settings(state: State<'_, AppStateWrapper>, settings: UserSettings) -> Result<(), String> {
    let sanitized = validate_settings(settings)?;
    let mut guard = state.lock();
    guard.settings = sanitized;
    // Recompute subscription_usage from the existing state's samples is
    // impossible without rescanning, so we just update the limits in the
    // current usage snapshot using the fresh settings. The next scheduler
    // tick will produce a fully recomputed snapshot.
    let plan = guard.settings.subscription_plan;
    let session_limit = if guard.settings.session_token_limit > 0 {
        guard.settings.session_token_limit
    } else {
        plan.default_session_tokens()
    };
    let weekly_limit = if guard.settings.weekly_token_limit > 0 {
        guard.settings.weekly_token_limit
    } else {
        plan.default_weekly_tokens()
    };
    guard.subscription_usage.session_limit_tokens = session_limit;
    guard.subscription_usage.week_limit_tokens = weekly_limit;

    // Session percentage is driven by cost (token-based was wildly off
    // due to cache bursts). Compute it from the cost limit.
    let session_cost_limit = if guard.settings.session_cost_limit_usd > 0.0 {
        guard.settings.session_cost_limit_usd
    } else {
        plan.default_session_cost_usd()
    };
    if session_cost_limit > 0.0 {
        let p = (guard.subscription_usage.session_cost_usd / session_cost_limit) * 100.0;
        if p.is_finite() {
            guard.subscription_usage.session_pct = p.clamp(0.0, 999.0) as u16;
        }
    } else if session_limit > 0 {
        let p = (guard.subscription_usage.session_tokens as f64 / session_limit as f64) * 100.0;
        guard.subscription_usage.session_pct = p.clamp(0.0, 999.0) as u16;
    }
    if session_limit > 0 {
        guard.subscription_usage.session_extra_cost_usd = recompute_extra(
            guard.subscription_usage.session_tokens,
            session_limit,
            guard.subscription_usage.session_cost_usd,
        );
    }
    if weekly_limit > 0 {
        let p = (guard.subscription_usage.week_tokens as f64 / weekly_limit as f64) * 100.0;
        guard.subscription_usage.week_pct = p.clamp(0.0, 999.0) as u16;
        guard.subscription_usage.week_extra_cost_usd = recompute_extra(
            guard.subscription_usage.week_tokens,
            weekly_limit,
            guard.subscription_usage.week_cost_usd,
        );
    }
    Ok(())
}

fn recompute_extra(tokens: u64, limit: u64, total_cost: f64) -> f64 {
    if limit == 0 || tokens <= limit || tokens == 0 {
        return 0.0;
    }
    let overflow = (tokens - limit) as f64;
    (overflow * (total_cost / tokens as f64)).max(0.0)
}

/// Minimum interval between force_refresh calls (prevents DoS via rapid IPC).
const FORCE_REFRESH_COOLDOWN_SECS: u64 = 5;

/// Force refresh all data sources
#[tauri::command]
pub async fn force_refresh(state: State<'_, AppStateWrapper>) -> Result<PeakLevel, String> {
    // SECURITY: rate-limit force_refresh to prevent a misbehaving renderer
    // from triggering continuous filesystem scans + outbound HTTP requests.
    {
        let guard = state.lock();
        if let Some(last) = guard.last_force_refresh {
            if last.elapsed().as_secs() < FORCE_REFRESH_COOLDOWN_SECS {
                return Ok(guard.peak_level.clone());
            }
        }
    }

    // Fetch fresh data — wrap blocking scan in spawn_blocking.
    let (stats, analytics) = tokio::task::spawn_blocking(stats_reader::read_all)
        .await
        .unwrap_or_default();
    let service_status = status_poller::fetch_service_status().await;

    // Update state
    let mut state_guard = state.lock();
    state_guard.stats = stats;
    state_guard.analytics = analytics;
    state_guard.service_status = service_status;
    state_guard.last_force_refresh = Some(std::time::Instant::now());

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
    const MIN_REFRESH: u64 = 30;
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

    // Subscription bounds: clamp to sane ranges so a corrupted store can't
    // produce NaN percentages or pick an invalid weekday.
    const MAX_SUB_TOKEN_LIMIT: u64 = 10_000_000_000; // 10B
    const MAX_SUB_COST_LIMIT: f64 = 1_000_000.0; // $1M — absurd upper bound
    s.session_token_limit = s.session_token_limit.min(MAX_SUB_TOKEN_LIMIT);
    s.weekly_token_limit = s.weekly_token_limit.min(MAX_SUB_TOKEN_LIMIT);
    // Clamp cost: reject NaN/infinite, negative, or absurdly large values.
    if !s.session_cost_limit_usd.is_finite() || s.session_cost_limit_usd < 0.0 {
        s.session_cost_limit_usd = 0.0;
    }
    s.session_cost_limit_usd = s.session_cost_limit_usd.min(MAX_SUB_COST_LIMIT);
    s.weekly_reset_weekday = s.weekly_reset_weekday.min(6);
    s.weekly_reset_hour = s.weekly_reset_hour.min(23);
    s.subscription_warn_pct = s.subscription_warn_pct.clamp(10, 100);

    // ── Alert + sound settings (NEW) ──────────────────────────────────
    s.sound_volume = s.sound_volume.min(100);

    // Valid percentages for usage warnings are 1..=200 (allow >100 to warn
    // about overflow). De-dup, sort, and cap list length to prevent abuse.
    let mut ths: Vec<u8> = s
        .usage_warning_thresholds
        .into_iter()
        .filter(|&v| (1..=200).contains(&v))
        .collect();
    ths.sort();
    ths.dedup();
    ths.truncate(10);
    s.usage_warning_thresholds = ths;

    // Sound IDs must be one of the known presets to prevent the frontend
    // from receiving a value the sound library doesn't recognize.
    const VALID_SOUNDS: &[&str] = &[
        "none", "chime", "bell", "ping", "alert", "pulse", "success", "warning",
    ];
    fn valid_or<'a>(got: &'a str, fallback: &'a str) -> &'a str {
        if VALID_SOUNDS.contains(&got) { got } else { fallback }
    }
    s.sound_peak_change = valid_or(&s.sound_peak_change, "pulse").to_string();
    s.sound_session_start = valid_or(&s.sound_session_start, "success").to_string();
    s.sound_session_end = valid_or(&s.sound_session_end, "chime").to_string();
    s.sound_usage_threshold = valid_or(&s.sound_usage_threshold, "warning").to_string();

    Ok(s)
}
