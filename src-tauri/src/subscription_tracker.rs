//! Computes the rolling 5-hour session window and the weekly allowance
//! window from a list of assistant-message samples. Used to drive the
//! subscription-mode progress bars + countdowns shown in the popup.
//!
//! The 5-hour window logic mirrors Claude's subscription model:
//!  - A session begins with the first message after a >= 5h idle gap.
//!  - The session lasts exactly 5 hours from its start timestamp,
//!    regardless of how many messages are sent in that window.
//!  - Once the 5h elapses, the next message starts a new session.
//!
//! The weekly window is driven by the user's configured reset weekday
//! and UTC hour (e.g. "Mondays at 00:00"), so it matches the timing
//! shown in the Claude Desktop / claude.ai settings.

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday};

use crate::state::{SubscriptionUsage, UserSettings};
use crate::stats_reader::AssistantSample;

const SESSION_DURATION_HOURS: i64 = 5;

/// Compute the subscription usage snapshot given the pre-collected
/// assistant samples and the user's settings.
pub fn compute(samples: &[AssistantSample], settings: &UserSettings) -> SubscriptionUsage {
    let now = Utc::now();
    let mut usage = SubscriptionUsage::default();

    // Resolve effective limits. A 0 override means "use plan default".
    let session_limit = if settings.session_token_limit > 0 {
        settings.session_token_limit
    } else {
        settings.subscription_plan.default_session_tokens()
    };
    let weekly_limit = if settings.weekly_token_limit > 0 {
        settings.weekly_token_limit
    } else {
        settings.subscription_plan.default_weekly_tokens()
    };
    let session_cost_limit = if settings.session_cost_limit_usd > 0.0 {
        settings.session_cost_limit_usd
    } else {
        settings.subscription_plan.default_session_cost_usd()
    };
    usage.session_limit_tokens = session_limit;
    usage.week_limit_tokens = weekly_limit;

    // Sort samples ascending by timestamp.
    let mut sorted: Vec<&AssistantSample> = samples.iter().collect();
    sorted.sort_by_key(|s| s.timestamp);

    // ── 5-hour session ───────────────────────────────────────────────
    // Walk forward tracking the current session_start. A session ends
    // when the 5h window elapses; the next sample after that starts a
    // new session.
    let session_len = Duration::hours(SESSION_DURATION_HOURS);
    let mut session_start: Option<DateTime<Utc>> = None;
    for sample in &sorted {
        match session_start {
            None => session_start = Some(sample.timestamp),
            Some(start) => {
                if sample.timestamp - start >= session_len {
                    // Previous session has expired — this sample begins a new one.
                    session_start = Some(sample.timestamp);
                }
            }
        }
    }

    if let Some(start) = session_start {
        let end = start + session_len;
        let active = end > now;
        usage.session_active = active;
        usage.session_start = Some(start.to_rfc3339());
        usage.session_end = Some(end.to_rfc3339());
        usage.session_seconds_until_reset = if active {
            (end - now).num_seconds().max(0)
        } else {
            0
        };

        if active {
            // Sum only samples that belong to the current session.
            let mut tokens = 0u64;
            let mut cost = 0.0f64;
            let mut messages = 0u32;
            for sample in &sorted {
                if sample.timestamp >= start && sample.timestamp <= now {
                    tokens = tokens.saturating_add(sample.tokens);
                    cost += sample.cost;
                    messages = messages.saturating_add(1);
                }
            }
            usage.session_tokens = tokens;
            usage.session_cost_usd = cost;
            usage.session_messages = messages;
            // Session percentage is driven by COST, not tokens. Claude's
            // "Plan usage limits" session bar appears to be cost-based —
            // see SubscriptionPlan::default_session_cost_usd for the
            // empirical calibration data. When cost_limit > 0 we use cost;
            // otherwise (0 = disabled) we fall back to token-based.
            usage.session_pct = if session_cost_limit > 0.0 {
                pct_cost(cost, session_cost_limit)
            } else {
                pct(tokens, session_limit)
            };
            usage.session_extra_cost_usd = extra_cost(tokens, session_limit, cost);
        }
    }

    // ── Weekly window ────────────────────────────────────────────────
    let last_reset = last_weekly_reset(now, settings.weekly_reset_weekday, settings.weekly_reset_hour);
    let next_reset = last_reset + Duration::days(7);
    usage.week_start = Some(last_reset.to_rfc3339());
    usage.week_end = Some(next_reset.to_rfc3339());
    usage.week_seconds_until_reset = (next_reset - now).num_seconds().max(0);

    let mut tokens = 0u64;
    let mut cost = 0.0f64;
    let mut messages = 0u32;
    for sample in &sorted {
        if sample.timestamp >= last_reset && sample.timestamp <= now {
            tokens = tokens.saturating_add(sample.tokens);
            cost += sample.cost;
            messages = messages.saturating_add(1);
        }
    }
    usage.week_tokens = tokens;
    usage.week_cost_usd = cost;
    usage.week_messages = messages;
    usage.week_pct = pct(tokens, weekly_limit);
    usage.week_extra_cost_usd = extra_cost(tokens, weekly_limit, cost);

    usage
}

/// Estimated API-equivalent cost of the tokens that overflowed the plan
/// limit. Applies the weighted average cost-per-token of the window to
/// the overflow amount. Returns 0 when under the limit or when there's
/// nothing to compute from.
fn extra_cost(tokens: u64, limit: u64, total_cost: f64) -> f64 {
    if limit == 0 || tokens <= limit || tokens == 0 {
        return 0.0;
    }
    let overflow = (tokens - limit) as f64;
    let cost_per_token = total_cost / tokens as f64;
    (overflow * cost_per_token).max(0.0)
}

/// The most recent past occurrence of the configured weekday + UTC hour.
/// Falls back to "now" if the config is invalid.
fn last_weekly_reset(now: DateTime<Utc>, weekday: u8, hour: u8) -> DateTime<Utc> {
    let target_weekday = weekday_from_u8(weekday);
    let target_hour = hour.min(23) as u32;

    // Candidate: today at target_hour:00 UTC.
    let today_candidate = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), target_hour, 0, 0)
        .single()
        .unwrap_or(now);

    // Walk back up to 7 days to find the latest matching weekday+hour
    // that is <= now.
    for days_back in 0..=7 {
        let candidate = today_candidate - Duration::days(days_back);
        if candidate.weekday() == target_weekday && candidate <= now {
            return candidate;
        }
    }
    // Shouldn't happen, but degrade gracefully.
    now - Duration::days(7)
}

fn weekday_from_u8(n: u8) -> Weekday {
    match n % 7 {
        0 => Weekday::Sun,
        1 => Weekday::Mon,
        2 => Weekday::Tue,
        3 => Weekday::Wed,
        4 => Weekday::Thu,
        5 => Weekday::Fri,
        _ => Weekday::Sat,
    }
}

fn pct(used: u64, limit: u64) -> u16 {
    if limit == 0 {
        return 0;
    }
    // Cap display at 999% so the UI never gets a nonsense-sized bar.
    let p = (used as f64 / limit as f64) * 100.0;
    p.clamp(0.0, 999.0) as u16
}

fn pct_cost(used: f64, limit: f64) -> u16 {
    if limit <= 0.0 || !limit.is_finite() {
        return 0;
    }
    let p = (used / limit) * 100.0;
    if !p.is_finite() {
        return 0;
    }
    p.clamp(0.0, 999.0) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SubscriptionPlan;

    fn sample(hours_ago: i64, tokens: u64) -> AssistantSample {
        AssistantSample {
            timestamp: Utc::now() - Duration::hours(hours_ago),
            tokens,
            cost: 0.0,
        }
    }

    #[test]
    fn new_session_starts_after_previous_expires() {
        // Session 1 starts at t=-8h and expires at t=-3h (-8h + 5h).
        // The t=-6h sample is within session 1.
        // The t=-2h sample is AFTER session 1 expired (-2h > -3h), so it
        // starts session 2 which expires at t=+3h. Session 2 contains the
        // -2h and -1h samples.
        let samples = vec![
            sample(8, 100),
            sample(6, 200),
            sample(2, 300),
            sample(1, 400),
        ];
        let mut settings = UserSettings::default();
        settings.subscription_plan = SubscriptionPlan::Pro;
        let usage = compute(&samples, &settings);
        assert!(usage.session_active);
        // Current session = session 2, only the -2h and -1h samples.
        assert_eq!(usage.session_tokens, 300 + 400);
        assert_eq!(usage.session_messages, 2);
    }

    #[test]
    fn single_session_when_all_within_5h() {
        // All samples within the last 5h — one continuous session.
        let samples = vec![sample(4, 100), sample(3, 200), sample(1, 400)];
        let settings = UserSettings::default();
        let usage = compute(&samples, &settings);
        assert!(usage.session_active);
        assert_eq!(usage.session_tokens, 100 + 200 + 400);
        assert_eq!(usage.session_messages, 3);
    }

    #[test]
    fn session_inactive_when_all_samples_old() {
        let samples = vec![sample(10, 100)]; // >5h ago
        let settings = UserSettings::default();
        let usage = compute(&samples, &settings);
        assert!(!usage.session_active);
        assert_eq!(usage.session_tokens, 0);
    }
}
