//! Computes the 5-hour session slot and the weekly allowance window
//! from a list of assistant-message samples. Used to drive the
//! subscription-mode progress bars + countdowns shown in the popup.
//!
//! The 5-hour session logic mirrors Claude's actual behavior, which is
//! NOT a rolling window from the user's first message but a fixed UTC
//! grid shared across all users:
//!  - Session slots are 5-hour windows aligned to a UTC anchor hour
//!    (default 02:00 UTC → slots at 02, 07, 12, 17, 22 UTC).
//!  - A session is always "current" — it's just whichever 5h slot now
//!    falls into. Usage within that slot counts; usage from prior slots
//!    doesn't. The slot always ends at its fixed reset time, regardless
//!    of when the first message arrived.
//!  - This matches what Claude Desktop's "Plan usage limits > Current
//!    session > Resets in Nh Mm" displays.
//!
//! The weekly window is driven by the user's configured reset weekday
//! and UTC hour (e.g. "Mondays at 00:00"), so it matches the timing
//! shown in the Claude Desktop / claude.ai settings.

use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc, Weekday};

use crate::state::{SubscriptionUsage, UserSettings};
use crate::stats_reader::AssistantSample;

const SESSION_SLOT_HOURS: i64 = 5;

/// Compute the [start, end) of the fixed 5-hour session slot that `now`
/// falls into. Slots are aligned to `anchor_hour:00 UTC` and repeat
/// every 5 hours forever. Default anchor 02:00 UTC (≡ slots at 02, 07,
/// 12, 17, 22 UTC) was confirmed to match what Claude Desktop displays
/// for a user in Spain on 2026-04-20.
fn current_session_slot(
    now: DateTime<Utc>,
    anchor_hour: u8,
) -> (DateTime<Utc>, DateTime<Utc>) {
    let anchor_hour = anchor_hour.min(23) as u32;
    // Reference moment: 2024-01-01 at anchor_hour:00 UTC. Any fixed past
    // anchor works — slot boundaries are `reference + k * 5h` for any k.
    let reference = Utc
        .with_ymd_and_hms(2024, 1, 1, anchor_hour, 0, 0)
        .single()
        .unwrap_or(now);

    let slot_seconds: i64 = SESSION_SLOT_HOURS * 3600;
    // div_euclid handles pre-reference times correctly (floor toward -∞).
    let slot_index = (now - reference).num_seconds().div_euclid(slot_seconds);
    let slot_start = reference + Duration::seconds(slot_index * slot_seconds);
    let slot_end = slot_start + Duration::hours(SESSION_SLOT_HOURS);
    (slot_start, slot_end)
}

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

    // ── 5-hour session slot ──────────────────────────────────────────
    // Claude's session is a fixed UTC slot, not a rolling window. All
    // usage within [slot_start, slot_end) counts; prior slots don't.
    let (slot_start, slot_end) =
        current_session_slot(now, settings.session_slot_anchor_hour);

    usage.session_start = Some(slot_start.to_rfc3339());
    usage.session_end = Some(slot_end.to_rfc3339());
    usage.session_seconds_until_reset = (slot_end - now).num_seconds().max(0);

    let mut tokens = 0u64;
    let mut cost = 0.0f64;
    let mut messages = 0u32;
    for sample in &sorted {
        if sample.timestamp >= slot_start && sample.timestamp < slot_end {
            tokens = tokens.saturating_add(sample.tokens);
            cost += sample.cost;
            messages = messages.saturating_add(1);
        }
    }
    // "Active" now means "at least one message in this slot". The slot
    // itself always exists, so the countdown runs even at 0% usage.
    usage.session_active = messages > 0;
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

    #[test]
    fn slot_aligned_to_anchor_grid() {
        // Session slots are fixed UTC grids at anchor_hour, +5, +10, +15, +20.
        // A call at 10:23 UTC with anchor 2 should land in the 07-12 slot.
        let now = Utc.with_ymd_and_hms(2026, 4, 20, 10, 23, 0).unwrap();
        let (start, end) = current_session_slot(now, 2);
        assert_eq!(start.hour(), 7);
        assert_eq!(end.hour(), 12);
        assert_eq!((end - start).num_hours(), 5);
    }

    #[test]
    fn slot_crosses_midnight_correctly() {
        // anchor=2, now=23:30 UTC → slot 22:00 today - 03:00 tomorrow.
        let now = Utc.with_ymd_and_hms(2026, 4, 20, 23, 30, 0).unwrap();
        let (start, end) = current_session_slot(now, 2);
        assert_eq!(start.day(), 20);
        assert_eq!(start.hour(), 22);
        assert_eq!(end.day(), 21);
        assert_eq!(end.hour(), 3);
    }

    #[test]
    fn only_current_slot_samples_are_counted() {
        // With anchor=2 and now=10:30 UTC on 2026-04-20, current slot
        // is [07:00, 12:00) UTC. Samples outside don't count.
        let mut settings = UserSettings::default();
        settings.subscription_plan = SubscriptionPlan::Pro;
        // We need fixed timestamps, not relative — reuse the default
        // compute path via real Utc::now() by crafting offsets.
        let now = Utc::now();
        let (slot_start, slot_end) = current_session_slot(now, settings.session_slot_anchor_hour);

        let in_slot_a = slot_start + Duration::minutes(5);
        let in_slot_b = slot_end - Duration::minutes(1);
        let before_slot = slot_start - Duration::hours(1);

        let samples = vec![
            AssistantSample { timestamp: in_slot_a, tokens: 100, cost: 0.5 },
            AssistantSample { timestamp: in_slot_b, tokens: 200, cost: 1.0 },
            AssistantSample { timestamp: before_slot, tokens: 999, cost: 99.0 },
        ];

        let usage = compute(&samples, &settings);
        // Only the two in-slot samples should be aggregated.
        assert_eq!(usage.session_messages, 2);
        assert_eq!(usage.session_tokens, 100 + 200);
        assert!(usage.session_active);
    }

    #[test]
    fn session_inactive_when_slot_is_empty() {
        // All samples are from a previous slot → current slot sees 0 msgs,
        // session_active = false, tokens = 0, but the slot itself still
        // has a valid start/end so countdown keeps running.
        let mut settings = UserSettings::default();
        settings.subscription_plan = SubscriptionPlan::Pro;
        let now = Utc::now();
        let (slot_start, _slot_end) = current_session_slot(now, settings.session_slot_anchor_hour);
        let before_slot = slot_start - Duration::hours(10);

        let samples = vec![AssistantSample {
            timestamp: before_slot,
            tokens: 500,
            cost: 2.0,
        }];

        let usage = compute(&samples, &settings);
        assert!(!usage.session_active);
        assert_eq!(usage.session_tokens, 0);
        assert_eq!(usage.session_messages, 0);
        // Countdown still present because the slot always exists.
        assert!(usage.session_seconds_until_reset > 0);
    }
}
