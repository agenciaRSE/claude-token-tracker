use std::collections::HashMap;
use std::path::PathBuf;
use serde::Deserialize;
use chrono::Utc;

use crate::state::{ClaudeStats, DailyTokens, HourCount, ModelUsageEntry};

/// Raw shape of ~/.claude/stats-cache.json
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStatsCache {
    #[serde(default)]
    daily_activity: Vec<RawDailyActivity>,
    #[serde(default)]
    daily_model_tokens: Vec<RawDailyModelTokens>,
    #[serde(default)]
    model_usage: HashMap<String, RawModelUsage>,
    #[serde(default)]
    total_sessions: u32,
    #[serde(default)]
    total_messages: u32,
    #[serde(default)]
    hour_counts: HashMap<String, u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDailyActivity {
    date: String,
    #[serde(default)]
    message_count: u32,
    #[serde(default)]
    session_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDailyModelTokens {
    date: String,
    #[serde(default)]
    tokens_by_model: HashMap<String, u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawModelUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cost_usd: f64,
}

/// Resolve the path to ~/.claude/stats-cache.json cross-platform
pub fn stats_cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("stats-cache.json"))
}

/// Read and parse the stats cache file into our app's ClaudeStats struct
pub fn read_stats() -> ClaudeStats {
    let path = match stats_cache_path() {
        Some(p) => p,
        None => {
            log::warn!("Could not resolve home directory");
            return ClaudeStats::default();
        }
    };

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Could not read stats-cache.json at {:?}: {}", path, e);
            return ClaudeStats::default();
        }
    };

    let raw: RawStatsCache = match serde_json::from_str(&content) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("Could not parse stats-cache.json: {}", e);
            return ClaudeStats::default();
        }
    };

    let today = Utc::now().format("%Y-%m-%d").to_string();

    // Find today's activity
    let today_activity = raw.daily_activity.iter().find(|a| a.date == today);
    let today_messages = today_activity.map_or(0, |a| a.message_count);
    let today_sessions = today_activity.map_or(0, |a| a.session_count);

    // Compute today's tokens from daily_model_tokens
    let today_tokens: u64 = raw.daily_model_tokens.iter()
        .find(|d| d.date == today)
        .map_or(0, |d| d.tokens_by_model.values().sum());

    // Compute today's cost (proportional estimate from model usage)
    let total_all_tokens: u64 = raw.model_usage.values()
        .map(|m| m.input_tokens + m.output_tokens)
        .sum();
    let total_cost: f64 = raw.model_usage.values()
        .map(|m| m.cost_usd)
        .sum();
    let today_cost_usd = if total_all_tokens > 0 {
        (today_tokens as f64 / total_all_tokens as f64) * total_cost
    } else {
        0.0
    };

    // Parse hour counts
    let mut hour_counts: Vec<HourCount> = raw.hour_counts.iter()
        .filter_map(|(k, &v)| {
            k.parse::<u8>().ok().map(|hour| HourCount { hour, count: v })
        })
        .collect();
    hour_counts.sort_by_key(|h| h.hour);

    // Parse daily tokens for trend analysis
    let daily_tokens: Vec<DailyTokens> = raw.daily_model_tokens.iter()
        .map(|d| DailyTokens {
            date: d.date.clone(),
            tokens: d.tokens_by_model.values().sum(),
        })
        .collect();

    // Parse model usage
    let model_usage: Vec<ModelUsageEntry> = raw.model_usage.iter()
        .map(|(model, usage)| ModelUsageEntry {
            model: model.clone(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_read_tokens: usage.cache_read_input_tokens,
            cache_creation_tokens: usage.cache_creation_input_tokens,
            cost_usd: usage.cost_usd,
        })
        .collect();

    ClaudeStats {
        today_messages,
        today_sessions,
        today_tokens,
        today_cost_usd,
        total_messages: raw.total_messages,
        total_sessions: raw.total_sessions,
        hour_counts,
        daily_tokens,
        model_usage,
    }
}
