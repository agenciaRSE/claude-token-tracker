//! Reads real usage stats by walking ~/.claude/projects/**/*.jsonl session
//! files and aggregating per-line metrics. We intentionally do NOT read
//! ~/.claude/stats-cache.json because modern Claude Code no longer updates it
//! (it can be months stale) — the .jsonl session logs are the source of truth.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use serde::Deserialize;

use crate::state::{
    ClaudeStats, DailyTokens, HourCount, ModeStats, ModelUsageEntry, ProjectAnalytics,
    ProjectStats, SessionSummary, TimeRange,
};

/// Resolve a TimeRange into an inclusive (start, end) UTC date window.
/// `None` bounds mean "no restriction" on that side.
fn time_range_bounds(range: TimeRange) -> (Option<NaiveDate>, Option<NaiveDate>) {
    let today = Utc::now().date_naive();
    match range {
        TimeRange::Today => (Some(today), Some(today)),
        TimeRange::Yesterday => {
            let y = today.pred_opt().unwrap_or(today);
            (Some(y), Some(y))
        }
        TimeRange::Last7Days => (
            Some(today - chrono::Duration::days(6)),
            Some(today),
        ),
        TimeRange::Last30Days => (
            Some(today - chrono::Duration::days(29)),
            Some(today),
        ),
        TimeRange::ThisMonth => {
            let first = today.with_day(1).unwrap_or(today);
            (Some(first), Some(today))
        }
        TimeRange::ThisYear => {
            let first = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today);
            (Some(first), Some(today))
        }
        TimeRange::All => (None, None),
    }
}

fn date_in_range(date: NaiveDate, bounds: (Option<NaiveDate>, Option<NaiveDate>)) -> bool {
    let (start, end) = bounds;
    if let Some(s) = start {
        if date < s {
            return false;
        }
    }
    if let Some(e) = end {
        if date > e {
            return false;
        }
    }
    true
}

/// Don't open session files older than this — keeps the scan fast and
/// still captures enough history for the 7-day trend and today's metrics.
const RECENT_WINDOW_DAYS: u64 = 30;

/// Hard cap on per-session .jsonl size we're willing to parse. A malicious
/// symlink or a runaway log rotation bug could point us at an arbitrarily
/// large file; 64 MiB is roughly 100x the largest real session we've seen.
const MAX_SESSION_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Maximum length we accept for a session_id or model string from JSONL.
/// Anything longer is treated as malformed and the line is skipped.
const MAX_STRING_KEY_LEN: usize = 128;

/// Hard cap on the number of unique sessions we track per scan to prevent
/// a crafted file from causing unbounded HashMap growth.
const MAX_SESSIONS_PER_SCAN: usize = 50_000;

/// How far back to collect per-message samples for subscription-window
/// calculations. Must be > max(7 days, 5 hours) so both the weekly and
/// 5-hour rolling windows can be computed.
const SAMPLE_WINDOW_DAYS: i64 = 10;

/// Hard cap on the number of samples we keep to bound memory regardless
/// of how much history the user accumulates.
const MAX_SAMPLES: usize = 200_000;

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
    entrypoint: Option<String>,
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

/// One assistant-message sample used for the 5-hour + weekly subscription
/// windows. Keeping this outside the Aggregate so callers can own it.
///
/// `tokens` intentionally excludes `cache_read_input_tokens`: cache-read
/// tokens are priced at ~10% of input tokens and empirical comparison
/// against the Claude Desktop "Plan usage limits" panel shows they are
/// not counted against subscription quotas. Including them produces a
/// 5–15× overcount (heavy Claude Code sessions can read tens of millions
/// of cache tokens that Claude's own meter doesn't bill).
#[derive(Debug, Clone)]
pub struct AssistantSample {
    pub timestamp: DateTime<Utc>,
    /// Quota-relevant tokens: input + output + cache_creation.
    /// Does NOT include cache_read.
    pub tokens: u64,
    pub cost: f64,
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
    // ── Analytics aggregators ──────────────────────────────────────────
    /// Per-project directory aggregation.
    project_agg: HashMap<String, ProjectAgg>,
    /// Per-entrypoint mode aggregation.
    mode_agg: HashMap<String, ModeAgg>,
    /// Per-session (task) aggregation.
    session_agg: HashMap<String, SessionAgg>,
    /// Per-message samples from the last SAMPLE_WINDOW_DAYS — feeds the
    /// subscription tracker. Always collected regardless of UI range filter
    /// so subscription_usage stays accurate across range changes.
    samples: Vec<AssistantSample>,
    /// Precomputed cutoff below which samples are discarded.
    sample_cutoff: DateTime<Utc>,
}

#[derive(Default)]
struct ModelAgg {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_creation: u64,
    cost: f64,
}

#[derive(Default)]
struct ProjectAgg {
    tokens: u64,
    cost: f64,
    messages: u32,
    sessions: HashSet<String>,
    model_agg: HashMap<String, ModelAgg>,
}

#[derive(Default)]
struct ModeAgg {
    tokens: u64,
    cost: f64,
    messages: u32,
    sessions: HashSet<String>,
}

struct SessionAgg {
    project: String,
    mode: String,
    tokens: u64,
    cost: f64,
    messages: u32,
    first_ts: String,
    last_ts: String,
}

impl Aggregate {
    fn new(today: String, sample_cutoff: DateTime<Utc>) -> Self {
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
            project_agg: HashMap::new(),
            mode_agg: HashMap::new(),
            session_agg: HashMap::new(),
            samples: Vec::new(),
            sample_cutoff,
        }
    }
}

fn projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}

/// Walk ~/.claude/projects/*/*.jsonl and aggregate usage stats + analytics.
/// Backward-compatible default: Last30Days range (the original behavior).
pub fn read_all() -> (ClaudeStats, ProjectAnalytics) {
    read_all_with_range(TimeRange::Last30Days)
}

/// Full scan that returns stats, analytics, AND the recent assistant samples
/// needed for subscription-window tracking. Samples are always collected
/// for the last SAMPLE_WINDOW_DAYS, independent of the analytics range
/// filter, so subscription_usage stays correct across UI range changes.
pub fn read_all_with_samples() -> (ClaudeStats, ProjectAnalytics, Vec<AssistantSample>) {
    read_all_with_range_and_samples(TimeRange::Last30Days)
}

/// Same as read_all but only aggregates entries whose timestamp falls within
/// the given TimeRange. Used by the Analytics tab's range selector.
pub fn read_all_with_range(range: TimeRange) -> (ClaudeStats, ProjectAnalytics) {
    let (stats, analytics, _samples) = read_all_with_range_and_samples(range);
    (stats, analytics)
}

fn read_all_with_range_and_samples(
    range: TimeRange,
) -> (ClaudeStats, ProjectAnalytics, Vec<AssistantSample>) {
    let bounds = time_range_bounds(range);
    let root = match projects_dir() {
        Some(p) => p,
        None => {
            log::warn!("Could not resolve ~/.claude/projects/");
            return (ClaudeStats::default(), ProjectAnalytics::default(), Vec::new());
        }
    };

    if !root.exists() {
        log::warn!("~/.claude/projects/ does not exist: {:?}", root);
        return (ClaudeStats::default(), ProjectAnalytics::default(), Vec::new());
    }

    let now = Utc::now();
    let today = now.format("%Y-%m-%d").to_string();
    let sample_cutoff = now - chrono::Duration::days(SAMPLE_WINDOW_DAYS);
    // For ranges that extend past 30 days (ThisYear, All) expand the file
    // mtime cutoff so we don't silently drop files that still have in-range
    // lines. For narrow ranges, stick to 30 days for performance.
    let cutoff_days = match range {
        TimeRange::ThisYear | TimeRange::All => 400,
        _ => RECENT_WINDOW_DAYS,
    };
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(cutoff_days * 24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut agg = Aggregate::new(today, sample_cutoff);

    let project_dirs = match fs::read_dir(&root) {
        Ok(rd) => rd,
        Err(e) => {
            log::warn!("Failed to read projects dir: {}", e);
            return (ClaudeStats::default(), ProjectAnalytics::default(), Vec::new());
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

        let dir_name = project_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let files = match fs::read_dir(&project_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for file_entry in files.flatten() {
            let path = file_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            // SECURITY: Open the file first, *then* check metadata on the
            // already-open handle. This eliminates the TOCTOU race between
            // a symlink_metadata() check and a later File::open() — the
            // metadata returned by file.metadata() describes the resource
            // we actually opened, not a directory entry that could be
            // swapped for a symlink between the two calls.
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let fmeta = match file.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !fmeta.is_file() {
                continue; // Catches devices, pipes, etc.
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
            process_file_handle(file, &dir_name, bounds, &mut agg);
        }
    }

    let stats = build_claude_stats(&agg);
    // Move samples out before consuming agg for analytics.
    let samples = std::mem::take(&mut agg.samples);
    let analytics = build_project_analytics(agg);
    (stats, analytics, samples)
}


/// Map the `entrypoint` field from JSONL to a user-facing mode label.
fn mode_label(entrypoint: Option<&str>) -> &'static str {
    match entrypoint {
        Some("cli") => "Code",
        Some("claude-desktop") => "Desktop",
        Some(_) => "Other",
        None => "Unknown",
    }
}

/// Process an already-open file handle (eliminates TOCTOU race).
/// `bounds` filters entries by UTC date window — lines outside the window
/// are ignored without partial-counting.
fn process_file_handle(
    file: File,
    project_dir: &str,
    bounds: (Option<NaiveDate>, Option<NaiveDate>),
    agg: &mut Aggregate,
) {
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
        // Before applying the UI range filter, check whether this assistant
        // message should be sampled for the subscription tracker. Samples
        // must span the last SAMPLE_WINDOW_DAYS regardless of UI range.
        let ty_early = parsed.ty.as_deref().unwrap_or("");
        let within_sample_window = dt >= agg.sample_cutoff;
        if ty_early == "assistant" && within_sample_window && agg.samples.len() < MAX_SAMPLES {
            if let Some(msg) = parsed.message.as_ref() {
                if let Some(usage) = msg.usage.as_ref() {
                    // Quota-relevant sum: exclude cache_read_input_tokens.
                    // See AssistantSample doc for the rationale — matching
                    // Claude's own "Plan usage" meter requires this.
                    let quota_tokens = usage.input_tokens
                        .saturating_add(usage.output_tokens)
                        .saturating_add(usage.cache_creation_input_tokens);
                    let model = msg
                        .model
                        .as_deref()
                        .filter(|m| m.len() <= MAX_STRING_KEY_LEN)
                        .unwrap_or("unknown");
                    let cost = compute_cost(model, usage);
                    agg.samples.push(AssistantSample {
                        timestamp: dt,
                        tokens: quota_tokens,
                        cost,
                    });
                }
            }
        }

        // Apply time range filter before doing any analytics aggregation.
        if !date_in_range(dt.date_naive(), bounds) {
            continue;
        }
        let date_str = dt.format("%Y-%m-%d").to_string();
        let is_today = date_str == agg.today_date;

        let ty = parsed.ty.as_deref().unwrap_or("");
        let mode = mode_label(parsed.entrypoint.as_deref());

        // SECURITY: Cap session_id length to prevent unbounded HashMap keys.
        let sid = match parsed.session_id.as_deref() {
            Some(s) if !s.is_empty() && s.len() <= MAX_STRING_KEY_LEN => s.to_string(),
            _ => String::new(),
        };

        // Track sessions that had any activity at all.
        if !sid.is_empty() {
            agg.all_sessions.insert(sid.clone());
            if is_today {
                agg.today_sessions.insert(sid.clone());
            }
            // Ensure session entry exists, but respect the cardinality cap.
            if agg.session_agg.len() < MAX_SESSIONS_PER_SCAN {
                agg.session_agg.entry(sid.clone()).or_insert_with(|| SessionAgg {
                    project: project_dir.to_string(),
                    mode: mode.to_string(),
                    tokens: 0,
                    cost: 0.0,
                    messages: 0,
                    first_ts: ts.clone(),
                    last_ts: ts.clone(),
                });
            }
        }

        // User-initiated messages. Skip isMeta synthetic system entries.
        if ty == "user" && !parsed.is_meta {
            agg.total_messages = agg.total_messages.saturating_add(1);
            if is_today {
                agg.today_messages = agg.today_messages.saturating_add(1);
            }
            // Per-project / mode / session message counts.
            // NB: assigning to `entry().or_default().<field> = <map>[key].<field>…`
            // panics because Rust evaluates the RHS first — at that point the
            // entry is still absent and HashMap's Index impl panics.
            // Instead, take the mutable ref once and increment in place.
            {
                let pa = agg
                    .project_agg
                    .entry(project_dir.to_string())
                    .or_default();
                pa.messages = pa.messages.saturating_add(1);
            }
            {
                let ma = agg.mode_agg.entry(mode.to_string()).or_default();
                ma.messages = ma.messages.saturating_add(1);
            }
            if let Some(sa) = agg.session_agg.get_mut(&sid) {
                sa.messages = sa.messages.saturating_add(1);
                if ts.as_str() > sa.last_ts.as_str() {
                    sa.last_ts = ts.clone();
                }
                if ts.as_str() < sa.first_ts.as_str() {
                    sa.first_ts = ts.clone();
                }
            }
        }

        // Assistant messages carry the real token usage + cost info.
        if ty == "assistant" {
            if let Some(msg) = parsed.message.as_ref() {
                if let Some(usage) = msg.usage.as_ref() {
                    // SECURITY: saturating_add prevents silent u64 wrapping.
                    let total = usage.input_tokens
                        .saturating_add(usage.output_tokens)
                        .saturating_add(usage.cache_creation_input_tokens)
                        .saturating_add(usage.cache_read_input_tokens);

                    // SECURITY: Cap model string length.
                    let model = msg
                        .model
                        .as_deref()
                        .filter(|m| m.len() <= MAX_STRING_KEY_LEN)
                        .unwrap_or("unknown")
                        .to_string();
                    let cost = compute_cost(&model, usage);

                    // 7-day trend uses every day that appears, not just today.
                    let day_entry = agg.daily_tokens.entry(date_str.clone()).or_insert(0);
                    *day_entry = day_entry.saturating_add(total);

                    if is_today {
                        agg.today_tokens = agg.today_tokens.saturating_add(total);
                        agg.today_cost += cost;
                        let hour = dt.hour() as usize;
                        if hour < 24 {
                            agg.hour_counts[hour] = agg.hour_counts[hour].saturating_add(1);
                        }
                    }

                    // Global model aggregation.
                    let m = agg.model_agg.entry(model.clone()).or_default();
                    m.input = m.input.saturating_add(usage.input_tokens);
                    m.output = m.output.saturating_add(usage.output_tokens);
                    m.cache_read = m.cache_read.saturating_add(usage.cache_read_input_tokens);
                    m.cache_creation = m.cache_creation.saturating_add(usage.cache_creation_input_tokens);
                    m.cost += cost;

                    // ── Per-project aggregation ────────────────────────
                    let pa = agg.project_agg.entry(project_dir.to_string()).or_default();
                    pa.tokens = pa.tokens.saturating_add(total);
                    pa.cost += cost;
                    if !sid.is_empty() {
                        pa.sessions.insert(sid.clone());
                    }
                    {
                        let pm = pa.model_agg.entry(model).or_default();
                        pm.input = pm.input.saturating_add(usage.input_tokens);
                        pm.output = pm.output.saturating_add(usage.output_tokens);
                        pm.cache_read = pm.cache_read.saturating_add(usage.cache_read_input_tokens);
                        pm.cache_creation = pm.cache_creation.saturating_add(usage.cache_creation_input_tokens);
                        pm.cost += cost;
                    }

                    // ── Per-mode aggregation ───────────────────────────
                    let ma = agg.mode_agg.entry(mode.to_string()).or_default();
                    ma.tokens = ma.tokens.saturating_add(total);
                    ma.cost += cost;
                    if !sid.is_empty() {
                        ma.sessions.insert(sid.clone());
                    }

                    // ── Per-session aggregation ────────────────────────
                    if let Some(sa) = agg.session_agg.get_mut(&sid) {
                        sa.tokens = sa.tokens.saturating_add(total);
                        sa.cost += cost;
                        if ts.as_str() > sa.last_ts.as_str() {
                            sa.last_ts = ts.clone();
                        }
                        if ts.as_str() < sa.first_ts.as_str() {
                            sa.first_ts = ts.clone();
                        }
                    }
                }
            }
        }
    }
}

/// Extract a human-readable short name from the encoded project directory.
///
/// Claude Code encodes absolute paths using `-` for path separators and
/// `--` for drive letters (e.g. `C:\` → `C--`).
///
///   `C--Users-Usuario-kDrive-work-my-project` → `my-project`
///   `home--user--code--foo`                   → `foo`
///
/// Uses a generic heuristic: split on `--` to recover coarse path segments,
/// then take the last non-empty segment (which is the project folder name).
fn pretty_project_name(dir_name: &str) -> String {
    // Split on "--" which marks drive / major path boundaries.
    let segments: Vec<&str> = dir_name.split("--").collect();
    // The last segment contains fine-grained sub-path parts separated by "-".
    // Take it as the project display name.
    if let Some(last) = segments.iter().rev().find(|s| !s.is_empty()) {
        let trimmed = last.trim_matches('-');
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    dir_name.to_string()
}

fn build_model_usage_vec(model_agg: &HashMap<String, ModelAgg>) -> Vec<ModelUsageEntry> {
    let mut v: Vec<ModelUsageEntry> = model_agg
        .iter()
        .map(|(model, m)| ModelUsageEntry {
            model: model.clone(),
            input_tokens: m.input,
            output_tokens: m.output,
            cache_read_tokens: m.cache_read,
            cache_creation_tokens: m.cache_creation,
            cost_usd: m.cost,
        })
        .collect();
    v.sort_by(|a, b| {
        b.cost_usd
            .partial_cmp(&a.cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    v
}

fn build_claude_stats(agg: &Aggregate) -> ClaudeStats {
    let hour_counts: Vec<HourCount> = (0..24u8)
        .map(|h| HourCount {
            hour: h,
            count: agg.hour_counts[h as usize],
        })
        .collect();

    // Sort dates descending and take the most recent 7 for the trend chart.
    let mut daily_sorted: Vec<(String, u64)> = agg.daily_tokens.iter().map(|(k, v)| (k.clone(), *v)).collect();
    daily_sorted.sort_by(|a, b| b.0.cmp(&a.0));
    let daily_tokens: Vec<DailyTokens> = daily_sorted
        .into_iter()
        .take(7)
        .map(|(date, tokens)| DailyTokens { date, tokens })
        .collect();

    let model_usage = build_model_usage_vec(&agg.model_agg);

    ClaudeStats {
        today_messages: agg.today_messages,
        today_sessions: agg.today_sessions.len().min(u32::MAX as usize) as u32,
        today_tokens: agg.today_tokens,
        today_cost_usd: agg.today_cost,
        total_messages: agg.total_messages,
        total_sessions: agg.all_sessions.len().min(u32::MAX as usize) as u32,
        hour_counts,
        daily_tokens,
        model_usage,
    }
}

fn build_project_analytics(agg: Aggregate) -> ProjectAnalytics {
    // ── Projects (sorted by cost descending) ──────────────────────────
    let mut projects: Vec<ProjectStats> = agg
        .project_agg
        .iter()
        .map(|(dir, pa)| ProjectStats {
            name: pretty_project_name(dir),
            dir_name: dir.clone(),
            total_tokens: pa.tokens,
            total_cost_usd: pa.cost,
            total_messages: pa.messages,
            total_sessions: pa.sessions.len().min(u32::MAX as usize) as u32,
            models: build_model_usage_vec(&pa.model_agg),
        })
        .collect();
    projects.sort_by(|a, b| {
        b.total_cost_usd
            .partial_cmp(&a.total_cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // ── Modes (sorted by tokens descending) ───────────────────────────
    let mut modes: Vec<ModeStats> = agg
        .mode_agg
        .into_iter()
        .map(|(mode, ma)| ModeStats {
            mode,
            total_tokens: ma.tokens,
            total_cost_usd: ma.cost,
            total_messages: ma.messages,
            total_sessions: ma.sessions.len().min(u32::MAX as usize) as u32,
        })
        .collect();
    modes.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));

    // ── Sessions (top 50 by cost) ─────────────────────────────────────
    let mut sessions: Vec<SessionSummary> = agg
        .session_agg
        .into_iter()
        .filter(|(_, sa)| sa.tokens > 0)
        .map(|(sid, sa)| SessionSummary {
            session_id: sid,
            project: pretty_project_name(&sa.project),
            mode: sa.mode,
            total_tokens: sa.tokens,
            total_cost_usd: sa.cost,
            messages: sa.messages,
            first_activity: sa.first_ts,
            last_activity: sa.last_ts,
        })
        .collect();
    sessions.sort_by(|a, b| {
        b.total_cost_usd
            .partial_cmp(&a.total_cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sessions.truncate(50);

    ProjectAnalytics {
        projects,
        modes,
        sessions,
    }
}
