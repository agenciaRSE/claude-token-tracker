use chrono::{Datelike, Timelike, Utc};
use crate::state::{ClaudeStats, PeakColor, PeakLevel, ServiceStatus};

/// Time-based peak score (0-100)
/// Based on global Claude usage patterns correlated with US/EU business hours.
pub fn time_score() -> u8 {
    let now = Utc::now();
    let hour = now.hour() as usize;
    let weekday = now.weekday(); // Mon=0 .. Sun=6

    let is_weekend = matches!(weekday, chrono::Weekday::Sat | chrono::Weekday::Sun);

    // Weekday scores by UTC hour (0-23)
    // Peak: US East business hours (14-17 UTC = 9AM-12PM EST)
    let weekday_scores: [u8; 24] = [
        15, 12, 10, 10, 10, 12, // 00-05: Americas asleep, Asia winding down
        20, 30, 35, 40, 50, 55, // 06-11: EU morning ramp-up
        60, 70, 85, 90, 85, 80, // 12-17: EU+US overlap, peak at 14-15 UTC
        70, 55, 40, 30, 20, 15, // 18-23: US West, winding down
    ];

    // Weekend scores are much lower
    let weekend_scores: [u8; 24] = [
        10, 8,  5,  5,  5,  8,  // 00-05
        10, 12, 15, 18, 22, 25, // 06-11
        28, 30, 35, 35, 32, 28, // 12-17
        25, 20, 15, 12, 10, 10, // 18-23
    ];

    let mut score = if is_weekend {
        weekend_scores[hour]
    } else {
        weekday_scores[hour]
    };

    // Tuesday and Wednesday historically have highest usage
    if matches!(weekday, chrono::Weekday::Tue | chrono::Weekday::Wed) {
        score = score.saturating_add(8);
    }

    score.min(100)
}

/// Service status score (0-100) based on Anthropic status page
pub fn status_score(service_status: &ServiceStatus) -> u8 {
    if service_status.components.is_empty() {
        return 0; // No data = assume operational
    }

    let mut max_score: u8 = 0;

    for component in &service_status.components {
        let score = match component.status.as_str() {
            "operational" => 0,
            "degraded_performance" => 40,
            "partial_outage" => 70,
            "major_outage" => 100,
            _ => 0,
        };
        if score > max_score {
            max_score = score;
        }
    }

    max_score
}

/// Personal usage score (0-100) based on local stats-cache.json
pub fn usage_score(stats: &ClaudeStats) -> u8 {
    let mut score: u8 = 0;

    // 1. Compare today's tokens vs 7-day rolling average
    if stats.daily_tokens.len() >= 2 {
        let recent_days: Vec<u64> = stats.daily_tokens.iter()
            .rev()
            .skip(1) // Skip today
            .take(7)
            .map(|d| d.tokens)
            .collect();

        if !recent_days.is_empty() {
            let avg: u64 = recent_days.iter().sum::<u64>() / recent_days.len() as u64;
            if avg > 0 && stats.today_tokens > 0 {
                let ratio = (stats.today_tokens as f64) / (avg as f64);
                if ratio > 2.0 {
                    score = score.saturating_add(80);
                } else if ratio > 1.5 {
                    score = score.saturating_add(60);
                } else if ratio > 1.0 {
                    score = score.saturating_add(30);
                }
            }
        }
    }

    // 2. Check if current hour is in user's top-3 peak hours
    let current_hour = Utc::now().hour() as u8;
    let mut sorted_hours = stats.hour_counts.clone();
    sorted_hours.sort_by(|a, b| b.count.cmp(&a.count));
    let top_3_hours: Vec<u8> = sorted_hours.iter().take(3).map(|h| h.hour).collect();

    if top_3_hours.contains(&current_hour) {
        score = score.saturating_add(25);
    }

    score.min(100)
}

/// Compute composite peak level with hysteresis
pub fn compute_peak_level(
    stats: &ClaudeStats,
    service_status: &ServiceStatus,
    previous_color: PeakColor,
) -> PeakLevel {
    let ts = time_score();
    let ss = status_score(service_status);
    let us = usage_score(stats);

    // Weighted composite: time 40%, status 35%, usage 25%
    let raw_score = (ts as f64 * 0.40) + (ss as f64 * 0.35) + (us as f64 * 0.25);
    let score = (raw_score.round() as u8).min(100);

    // Apply hysteresis: require 5-point buffer to change color
    let new_color = PeakColor::from_score(score);
    let color = apply_hysteresis(score, new_color, previous_color);

    PeakLevel {
        color,
        score,
        time_score: ts,
        status_score: ss,
        usage_score: us,
        recommendation: color.recommendation().to_string(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

/// Prevent flickering by requiring 5-point buffer to change color thresholds
fn apply_hysteresis(score: u8, new_color: PeakColor, previous_color: PeakColor) -> PeakColor {
    // Thresholds: Green <= 25, Yellow <= 50, Orange <= 75, Red > 75
    // Only change if score is 5+ points past the boundary
    let threshold_crossed_firmly = match (previous_color, new_color) {
        (PeakColor::Green, PeakColor::Yellow) => score >= 30,
        (PeakColor::Yellow, PeakColor::Green) => score <= 20,
        (PeakColor::Yellow, PeakColor::Orange) => score >= 55,
        (PeakColor::Orange, PeakColor::Yellow) => score <= 45,
        (PeakColor::Orange, PeakColor::Red) => score >= 80,
        (PeakColor::Red, PeakColor::Orange) => score <= 70,
        // Skip levels (e.g., green -> orange) always go through
        _ => true,
    };

    if threshold_crossed_firmly {
        new_color
    } else {
        previous_color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peak_color_from_score() {
        assert_eq!(PeakColor::from_score(0), PeakColor::Green);
        assert_eq!(PeakColor::from_score(25), PeakColor::Green);
        assert_eq!(PeakColor::from_score(26), PeakColor::Yellow);
        assert_eq!(PeakColor::from_score(50), PeakColor::Yellow);
        assert_eq!(PeakColor::from_score(51), PeakColor::Orange);
        assert_eq!(PeakColor::from_score(75), PeakColor::Orange);
        assert_eq!(PeakColor::from_score(76), PeakColor::Red);
        assert_eq!(PeakColor::from_score(100), PeakColor::Red);
    }

    #[test]
    fn test_hysteresis_prevents_flickering() {
        // Score of 27 should NOT move from Green to Yellow (needs 30+)
        let color = apply_hysteresis(27, PeakColor::Yellow, PeakColor::Green);
        assert_eq!(color, PeakColor::Green);

        // Score of 30 SHOULD move from Green to Yellow
        let color = apply_hysteresis(30, PeakColor::Yellow, PeakColor::Green);
        assert_eq!(color, PeakColor::Yellow);
    }

    #[test]
    fn test_status_score_operational() {
        let status = ServiceStatus {
            components: vec![
                crate::state::ServiceComponent {
                    name: "Claude API".to_string(),
                    status: "operational".to_string(),
                },
            ],
            overall: "operational".to_string(),
            fetched_at: String::new(),
        };
        assert_eq!(status_score(&status), 0);
    }

    #[test]
    fn test_status_score_degraded() {
        let status = ServiceStatus {
            components: vec![
                crate::state::ServiceComponent {
                    name: "Claude API".to_string(),
                    status: "degraded_performance".to_string(),
                },
                crate::state::ServiceComponent {
                    name: "claude.ai".to_string(),
                    status: "operational".to_string(),
                },
            ],
            overall: "minor".to_string(),
            fetched_at: String::new(),
        };
        assert_eq!(status_score(&status), 40);
    }
}
