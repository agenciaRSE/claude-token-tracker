//! Reads real usage stats by walking ~/.claude/projects/**/*.jsonl session
//! files and aggregating per-line metrics. We intentionally do NOT read
//! ~/.claude/stats-cache.json because modern Claude Code no longer updates it
//! (it can be months stale) — the .jsonl session logs are the source of truth.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Timelike, Utc};
use serde::Deserialize;

use crate::state::{ClaudeStats, DailyTokens, HourCount, ModelUsageEntry};

/// Don't open session files older than this — keeps the scan fast and
/// still captures enough history for the 7-day trend and today's metrics.
const RECENT_WINDOW_DAYS: u64 = 30;

/// Hard cap on per-session .jsonl size we're willing to parse. A malicious
/// symlink or a runaway log rotation bug could point us at an arbitrarily
/// large file; 64 MiB is roughly 100x the largest real session we've seen.
const MAX_SESSION_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Shape of one line in a session .jsonl file — we deserialize only the
/// handful of fields we actually consume.
#[derive(Debug, Deserialize)]
struct Line {
    #[serde(rename = "type")]
    ty: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    #[serde(rename = "isMeta", default)]
    is_meta: bool,
    message: Option<LineMessage>,
}

#[derive(Debug, Deserialize)]
struct LineMessage {
    model: Option<String>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize, Default)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

/// Approximate Anthropic list pricing per 1M tokens, in USD:
/// returns (input, output, cache_create, cache_read). These can drift;
/// they're a best-effort estimate for the popup/dashboard display.
fn model_pricing(model: &str) -> (f64, f64, f64, f64) {
    let m = model.to_lowercase();
    if m.contains("opus") {
        (15.0, 75.0, 18.75, 1.50)
    } else if m.contains("haiku") {
        (1.0, 5.0, 1.25, 0.10)
    } else if m.contains("sonnet") {
        (3.0, 15.0, 3.75, 0.30)
    } else {
        // Unknown model — fall back to Sonnet pricing so we don't return 0.
        (3.0, 15.0, 3.75, 0.30)
    }
}

fn compute_cost(model: &str, usage: &Usage) -> f64 {
    let (p_in, p_out, p_cc, p_cr) = model_pricing(model);
    (usage.input_tokens as f64 * p_in
        + usage.output_tokens as f64 * p_out
        + usage.cache_creation_input_tokens as f64 * p_cc
        + usage.cache_read_input_tokens as f64 * p_cr)
        / 1_000_000.0
}

/// Mutable aggregator that accumulates counters as we stream through lines.
struct Aggregate {
    today_date: String,
    today_messages: u32,
    today_tokens: u64,
    today_cost: f64,
    total_messages: u32,
    today_sessions: HashSet<String>,
    all_sessions: HashSet<String>,
    /// Assistant-message counts by UTC hour (for the popup's peak hours grid).
    hour_counts: [u32; 24],
    /// date (YYYY-MM-DD) -> total tokens that day, used for the 7-day trend.
    daily_tokens: HashMap<String, u64>,
    /// Per-model lifetime aggregation within the scan window.
    model_agg: HashMap<String, ModelAgg>,
}

#[derive(Default)]
struct ModelAgg {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_creation: u64,
    cost: f64,
}

impl Aggregate {
    fn new(today: String) -> Self {
        Self {
            today_date: today,
            today_messages: 0,
            today_tokens: 0,
            today_cost: 0.0,
            total_messages: 0,
            today_sessions: HashSet::new(),
            all_sessions: HashSet::new(),
            hour_counts: [0u32; 24],
            daily_tokens: HashMap::new(),
            model_agg: HashMap::new(),
        }
    }
}

fn projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}

/// Walk ~/.claude/projects/*/*.jsonl and aggregate usage stats.
pub fn read_stats() -> ClaudeStats {
    let root = match projects_dir() {
        Some(p) => p,
        None => {
            log::warn!("Could not resolve ~/.claude/projects/");
            return ClaudeStats::default();
        }
    };

    if !root.exists() {
        log::warn!("~/.claude/projects/ does not exist: {:?}", root);
        return ClaudeStats::default();
    }

    let now = Utc::now();
    let today = now.format("%Y-%m-%d").to_string();
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(RECENT_WINDOW_DAYS * 24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut agg = Aggregate::new(today);

    let project_dirs = match fs::read_dir(&root) {
        Ok(rd) => rd,
        Err(e) => {
            log::warn!("Failed to read projects dir: {}", e);
            return ClaudeStats::default();
        }
    };

    for project_entry in project_dirs.flatten() {
        let project_path = project_entry.path();
        // symlink_metadata() does NOT follow symlinks, so we can detect and
        // skip a project directory that's actually a link pointing elsewhere
        // on disk. Combined with the same check at the file level below, this
        // confines the scanner to the real ~/.claude/projects/ tree.
        let lmeta = match fs::symlink_metadata(&project_path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if lmeta.file_type().is_symlink() || !lmeta.is_dir() {
            continue;
        }
        let files = match fs::read_dir(&project_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for file_entry in files.flatten() {
            let path = file_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            // Refuse to follow symlinks — they could point at arbitrary files
            // on disk (e.g. /etc/passwd) and cause us to leak tokens from
            // parsing unexpected content or blow memory on huge files.
            let fmeta = match fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if fmeta.file_type().is_symlink() || !fmeta.is_file() {
                continue;
            }
            if fmeta.len() > MAX_SESSION_FILE_BYTES {
                log::warn!(
                    "Skipping oversized session file ({} bytes): {:?}",
                    fmeta.len(),
                    path
                );
                continue;
            }
            // Skip old files — keeps the scan bounded.
            if let Ok(mtime) = fmeta.modified() {
                if mtime < cutoff {
                    continue;
                }
            }
            process_file(&path, &mut agg);
        }
    }

    build_claude_stats(agg)
}

fn process_file(path: &Path, agg: &mut Aggregate) {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let reader = BufReader::new(file);

    for line in reader.lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        let parsed: Line = match serde_json::from_str(&line) {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Skip lines with no timestamp (e.g. file-history-snapshot entries).
        let ts = match parsed.timestamp.as_ref() {
            Some(t) => t,
            None => continue,
        };
        let dt = match DateTime::parse_from_rfc3339(ts) {
            Ok(d) => d.with_timezone(&Utc),
            Err(_) => continue,
        };
        let date_str = dt.format("%Y-%m-%d").to_string();
        let is_today = date_str == agg.today_date;

        let ty = parsed.ty.as_deref().unwrap_or("");

        // Track sessions that had any activity at all.
        if let Some(sid) = parsed.session_id.as_ref() {
            agg.all_sessions.insert(sid.clone());
            if is_today {
                agg.today_sessions.insert(sid.clone());
            }
        }

        // User-initiated messages. Skip isMeta synthetic system entries.
        if ty == "user" && !parsed.is_meta {
            agg.total_messages += 1;
            if is_today {
                agg.today_messages += 1;
            }
        }

        // Assistant messages carry the real token usage + cost info.
        if ty == "assistant" {
            if let Some(msg) = parsed.message.as_ref() {
                if let Some(usage) = msg.usage.as_ref() {
                    let total = usage.input_tokens
                        + usage.output_tokens
                        + usage.cache_creation_input_tokens
                        + usage.cache_read_input_tokens;

                    let model = msg
                        .model
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string());
                    let cost = compute_cost(&model, usage);

                    // 7-day trend uses every day that appears, not just today.
                    *agg.daily_tokens.entry(date_str.clone()).or_insert(0) += total;

                    if is_today {
                        agg.today_tokens += total;
                        agg.today_cost += cost;
                        let hour = dt.hour() as usize;
                        if hour < 24 {
                            agg.hour_counts[hour] += 1;
                        }
                    }

                    let m = agg.model_agg.entry(model).or_default();
                    m.input += usage.input_tokens;
                    m.output += usage.output_tokens;
                    m.cache_read += usage.cache_read_input_tokens;
                    m.cache_creation += usage.cache_creation_input_tokens;
                    m.cost += cost;
                }
            }
        }
    }
}

fn build_claude_stats(agg: Aggregate) -> ClaudeStats {
    let hour_counts: Vec<HourCount> = (0..24u8)
        .map(|h| HourCount {
            hour: h,
            count: agg.hour_counts[h as usize],
        })
        .collect();

    // Sort dates descending and take the most recent 7 for the trend chart.
    let mut daily_sorted: Vec<(String, u64)> = agg.daily_tokens.into_iter().collect();
    daily_sorted.sort_by(|a, b| b.0.cmp(&a.0));
    let daily_tokens: Vec<DailyTokens> = daily_sorted
        .into_iter()
        .take(7)
        .map(|(date, tokens)| DailyTokens { date, tokens })
        .collect();

    let mut model_usage: Vec<ModelUsageEntry> = agg
        .model_agg
        .into_iter()
        .map(|(model, m)| ModelUsageEntry {
            model,
            input_tokens: m.input,
            output_tokens: m.output,
            cache_read_tokens: m.cache_read,
            cache_creation_tokens: m.cache_creation,
            cost_usd: m.cost,
        })
        .collect();
    model_usage.sort_by(|a, b| {
        b.cost_usd
            .partial_cmp(&a.cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    ClaudeStats {
        today_messages: agg.today_messages,
        today_sessions: agg.today_sessions.len() as u32,
        today_tokens: agg.today_tokens,
        today_cost_usd: agg.today_cost,
        total_messages: agg.total_messages,
        total_sessions: agg.all_sessions.len() as u32,
        hour_counts,
        daily_tokens,
        model_usage,
    }
}
