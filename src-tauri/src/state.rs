use serde::{Deserialize, Serialize};
use std::sync::{Mutex, MutexGuard};

/// Peak color levels matching the tray icon colors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeakColor {
    Green,
    Yellow,
    Orange,
    Red,
}

impl PeakColor {
    pub fn from_score(score: u8) -> Self {
        match score {
            0..=25 => PeakColor::Green,
            26..=50 => PeakColor::Yellow,
            51..=75 => PeakColor::Orange,
            _ => PeakColor::Red,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PeakColor::Green => "Low",
            PeakColor::Yellow => "Moderate",
            PeakColor::Orange => "High",
            PeakColor::Red => "Peak",
        }
    }

    pub fn recommendation(&self) -> &'static str {
        match self {
            PeakColor::Green => "Great time to use Claude! Low traffic expected.",
            PeakColor::Yellow => "Moderate usage. Good time to work, minor delays possible.",
            PeakColor::Orange => "High usage period. Consider deferring heavy tasks.",
            PeakColor::Red => "Peak hours! Expect slower responses and potential rate limits.",
        }
    }

    /// RGBA color for generating tray icons at runtime
    pub fn rgba(&self) -> [u8; 4] {
        match self {
            PeakColor::Green => [34, 197, 94, 255],     // #22c55e
            PeakColor::Yellow => [234, 179, 8, 255],    // #eab308
            PeakColor::Orange => [249, 115, 22, 255],   // #f97316
            PeakColor::Red => [239, 68, 68, 255],       // #ef4444
        }
    }
}

/// Current peak level with all scoring details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeakLevel {
    pub color: PeakColor,
    pub score: u8,
    pub time_score: u8,
    pub status_score: u8,
    pub usage_score: u8,
    pub recommendation: String,
    pub updated_at: String,
}

impl Default for PeakLevel {
    fn default() -> Self {
        Self {
            color: PeakColor::Green,
            score: 0,
            time_score: 0,
            status_score: 0,
            usage_score: 0,
            recommendation: PeakColor::Green.recommendation().to_string(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Anthropic service component status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceComponent {
    pub name: String,
    pub status: String,
}

/// Service status from Anthropic status page
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub components: Vec<ServiceComponent>,
    pub overall: String,
    pub fetched_at: String,
}

/// Stats from ~/.claude/stats-cache.json
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeStats {
    pub today_messages: u32,
    pub today_sessions: u32,
    pub today_tokens: u64,
    pub today_cost_usd: f64,
    pub total_messages: u32,
    pub total_sessions: u32,
    pub hour_counts: Vec<HourCount>,
    pub daily_tokens: Vec<DailyTokens>,
    pub model_usage: Vec<ModelUsageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourCount {
    pub hour: u8,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyTokens {
    pub date: String,
    pub tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsageEntry {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cost_usd: f64,
}

/// Time range filter for the Analytics tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    #[default]
    Today,
    Yesterday,
    Last7Days,
    Last30Days,
    ThisMonth,
    ThisYear,
    All,
}

/// Per-project token/cost breakdown.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    /// Human-readable short name (last path component).
    pub name: String,
    /// Raw directory name (full encoded path).
    pub dir_name: String,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub total_messages: u32,
    pub total_sessions: u32,
    pub models: Vec<ModelUsageEntry>,
}

/// Aggregated stats per entrypoint mode (Code / Desktop).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeStats {
    /// Display label: "Code", "Desktop", "Other".
    pub mode: String,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub total_messages: u32,
    pub total_sessions: u32,
}

/// Lightweight summary of one session (task / conversation).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: String,
    pub project: String,
    pub mode: String,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub messages: u32,
    pub first_activity: String,
    pub last_activity: String,
}

/// Top-level analytics payload sent to the frontend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAnalytics {
    pub projects: Vec<ProjectStats>,
    pub modes: Vec<ModeStats>,
    /// Top 50 sessions by cost, most recent first within equal cost.
    pub sessions: Vec<SessionSummary>,
}

/// How the user is billed for Claude. Drives whether the cost figures
/// shown across the UI are actual money owed (`Api`) or a "value extracted
/// from the subscription" estimate (`Subscription`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CostMode {
    #[default]
    Api,
    Subscription,
}

/// Claude subscription tier. Used to pick sensible default token quotas
/// for the 5-hour session and weekly limit bars. Users can override the
/// literal token thresholds via the Settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionPlan {
    #[default]
    Pro,
    Max5x,
    Max20x,
    Custom,
}

impl SubscriptionPlan {
    /// Approximate 5-hour session token budget, in **quota-relevant tokens**
    /// (input + output + cache_creation — cache_read is excluded, see
    /// `AssistantSample` for the rationale).
    ///
    /// These defaults are calibrated from empirical comparison against the
    /// Claude Desktop "Plan usage limits" panel for a real Max 5× user on
    /// 2026-04-18 (observed 262.9K ≈ 29% of session, 27.2M ≈ 25% of week),
    /// which back-solves to a session budget ≈ 900K and weekly ≈ 108M for
    /// that plan. Pro / Max 20× are scaled proportionally to Anthropic's
    /// published plan multipliers (1× / 5× / 20×).
    ///
    /// Users on any plan can override via the Custom entry or by typing a
    /// non-zero value into the session/weekly token limit fields.
    pub fn default_session_tokens(&self) -> u64 {
        match self {
            Self::Pro => 200_000,
            Self::Max5x => 1_000_000,
            Self::Max20x => 4_000_000,
            Self::Custom => 0,
        }
    }
    pub fn default_weekly_tokens(&self) -> u64 {
        match self {
            Self::Pro => 22_000_000,
            Self::Max5x => 108_000_000,
            Self::Max20x => 432_000_000,
            Self::Custom => 0,
        }
    }

    /// Approximate **dollar** budget for a 5-hour session. Claude Desktop's
    /// "Plan usage limits" session bar appears to be cost-driven rather
    /// than token-driven — the same user on the same plan shows vastly
    /// different session % depending on whether the current burst is
    /// cache_creation-heavy or not. Cost, on the other hand, stays
    /// roughly proportional to Claude's percentage.
    ///
    /// Defaults calibrated from observed Max 5× data points. Implied
    /// budgets across a week of observations:
    /// $22→11% ($203), $83→29% ($289), $113→38% ($298), $114→32% ($357),
    /// $271→60% ($452). The effective limit drifts with cache ratio, so
    /// we pick a high-end value (450) to avoid over-counting during
    /// heavy cache-read sessions; users can override via Custom.
    ///
    /// When > 0 the tracker uses this as the session quota; 0 falls back
    /// to token-based accounting.
    pub fn default_session_cost_usd(&self) -> f64 {
        match self {
            Self::Pro => 90.0,
            Self::Max5x => 450.0,
            Self::Max20x => 1_800.0,
            Self::Custom => 0.0,
        }
    }
}

/// Snapshot of the user's subscription plan usage — both the rolling 5-hour
/// session window and the weekly allowance. Computed locally from JSONL
/// timestamps, so it reflects Claude Code/Desktop usage tracked in
/// `~/.claude/projects/`. Does NOT include pure web chat on claude.ai.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionUsage {
    // ── 5-hour session ───────────────────────────────────────────────
    pub session_active: bool,
    pub session_start: Option<String>,           // RFC 3339 UTC
    pub session_end: Option<String>,             // RFC 3339 UTC
    pub session_tokens: u64,
    pub session_cost_usd: f64,
    pub session_messages: u32,
    pub session_limit_tokens: u64,
    /// 0-100+ (can exceed 100 if user has burned through allowance).
    pub session_pct: u16,
    /// Seconds until session_end. Negative/zero if session inactive.
    pub session_seconds_until_reset: i64,
    /// Estimated API-equivalent cost of tokens beyond the plan limit in
    /// the current 5h session. Zero while under the limit. Computed by
    /// applying the session's weighted average cost-per-token to the
    /// overflow amount — an approximation because the JSONL logs don't
    /// mark which specific tokens were "extra".
    pub session_extra_cost_usd: f64,

    // ── Weekly window ────────────────────────────────────────────────
    pub week_start: Option<String>,
    pub week_end: Option<String>,
    pub week_tokens: u64,
    pub week_cost_usd: f64,
    pub week_messages: u32,
    pub week_limit_tokens: u64,
    pub week_pct: u16,
    pub week_seconds_until_reset: i64,
    pub week_extra_cost_usd: f64,
}

/// User settings persisted via tauri-plugin-store
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub timezone: String,
    pub notifications_enabled: bool,
    pub notify_on_color_change: bool,
    pub daily_token_alert: Option<u64>,
    pub refresh_interval_secs: u64,
    pub autostart: bool,
    #[serde(default)]
    pub cost_mode: CostMode,

    // ── Subscription plan tracking ───────────────────────────────────
    #[serde(default)]
    pub subscription_plan: SubscriptionPlan,
    /// Override for the 5-hour session token limit. 0 means "use plan default".
    /// NOTE: session percentage is now driven by cost (see
    /// `session_cost_limit_usd`); tokens are retained for display only.
    #[serde(default)]
    pub session_token_limit: u64,
    /// Override for the weekly token limit. 0 means "use plan default".
    #[serde(default)]
    pub weekly_token_limit: u64,
    /// Override for the 5-hour session COST limit in USD. 0 means
    /// "use plan default". This is the primary quota driver for the
    /// session bar; token counting drifts wildly with cache bursts so
    /// we use cost instead. See `SubscriptionPlan::default_session_cost_usd`.
    #[serde(default)]
    pub session_cost_limit_usd: f64,
    /// Day of week for the weekly reset. 0=Sunday, 1=Monday, ..., 6=Saturday.
    #[serde(default)]
    pub weekly_reset_weekday: u8,
    /// Hour (0-23, UTC) of the weekly reset.
    #[serde(default)]
    pub weekly_reset_hour: u8,
    /// Anchor hour (0-23, UTC) for the 5-hour session slot grid. Slots
    /// boundaries are at this hour plus every multiple of 5. Default
    /// 02 → slots at 02, 07, 12, 17, 22 UTC (validated against Claude
    /// Desktop for a user in Spain on 2026-04-20). Claude's schedule
    /// is believed to be global across users; if you observe a
    /// different reset time, adjust here.
    #[serde(default = "default_session_slot_anchor_hour")]
    pub session_slot_anchor_hour: u8,
    /// Warning threshold as a percentage (0-100). Default 80.
    /// NOTE: kept for backward compatibility with v0.1 settings files, but
    /// the scheduler now reads from `usage_warning_thresholds` (multi-value).
    #[serde(default = "default_warn_pct")]
    pub subscription_warn_pct: u8,
    /// Whether to fire OS notifications when the threshold is crossed.
    #[serde(default = "default_true")]
    pub subscription_warnings_enabled: bool,

    // ── Alert triggers (NEW — which events fire notifications/sounds) ──
    /// Fire notification when a brand-new 5-hour session begins (first
    /// assistant message after a >= 5h idle period).
    #[serde(default = "default_true")]
    pub alert_session_start: bool,
    /// Fire notification when the active 5-hour session expires (idle gap).
    #[serde(default = "default_true")]
    pub alert_session_end: bool,
    /// Percentage checkpoints at which a session/weekly usage warning
    /// should fire. Each threshold fires at most once per window. Sorted
    /// ascending; values outside [1, 200] are ignored by the scheduler.
    #[serde(default = "default_usage_thresholds")]
    pub usage_warning_thresholds: Vec<u8>,

    // ── Sound system (NEW) ────────────────────────────────────────────
    /// Master on/off for sound alerts. OS notifications are independent
    /// and controlled by `notifications_enabled`.
    #[serde(default = "default_true")]
    pub sounds_enabled: bool,
    /// Sound volume 0-100. Applied to every preset.
    #[serde(default = "default_sound_volume")]
    pub sound_volume: u8,
    /// Preset id played when the peak color changes.
    #[serde(default = "default_sound_peak")]
    pub sound_peak_change: String,
    /// Preset id played when a new 5-hour session begins.
    #[serde(default = "default_sound_session_start")]
    pub sound_session_start: String,
    /// Preset id played when the 5-hour session expires.
    #[serde(default = "default_sound_session_end")]
    pub sound_session_end: String,
    /// Preset id played when a usage threshold is crossed.
    #[serde(default = "default_sound_threshold")]
    pub sound_usage_threshold: String,
}

fn default_warn_pct() -> u8 { 80 }
fn default_true() -> bool { true }
fn default_session_slot_anchor_hour() -> u8 { 2 }
fn default_usage_thresholds() -> Vec<u8> { vec![75, 90, 100] }
fn default_sound_volume() -> u8 { 70 }
fn default_sound_peak() -> String { "pulse".to_string() }
fn default_sound_session_start() -> String { "success".to_string() }
fn default_sound_session_end() -> String { "chime".to_string() }
fn default_sound_threshold() -> String { "warning".to_string() }

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            timezone: "UTC".to_string(),
            notifications_enabled: true,
            notify_on_color_change: true,
            daily_token_alert: None,
            refresh_interval_secs: 120,
            autostart: true,
            cost_mode: CostMode::Api,
            subscription_plan: SubscriptionPlan::Pro,
            session_token_limit: 0,
            weekly_token_limit: 0,
            session_cost_limit_usd: 0.0,
            weekly_reset_weekday: 1, // Monday
            weekly_reset_hour: 0,
            session_slot_anchor_hour: 2, // 02:00 UTC → slots at 02/07/12/17/22
            subscription_warn_pct: 80,
            subscription_warnings_enabled: true,
            alert_session_start: true,
            alert_session_end: true,
            usage_warning_thresholds: vec![75, 90, 100],
            sounds_enabled: true,
            sound_volume: 70,
            sound_peak_change: "pulse".to_string(),
            sound_session_start: "success".to_string(),
            sound_session_end: "chime".to_string(),
            sound_usage_threshold: "warning".to_string(),
        }
    }
}

/// Shared application state behind a Mutex
pub struct AppState {
    pub peak_level: PeakLevel,
    pub stats: ClaudeStats,
    pub analytics: ProjectAnalytics,
    pub subscription_usage: SubscriptionUsage,
    pub service_status: ServiceStatus,
    pub settings: UserSettings,
    pub previous_color: PeakColor,
    /// Date string ("YYYY-MM-DD") when the daily token alert last fired.
    /// Prevents spamming a notification every poll cycle once the threshold
    /// is crossed — the alert fires at most once per calendar day.
    pub token_alert_fired_today: Option<String>,
    /// Timestamp of the last `force_refresh` call for rate-limiting.
    pub last_force_refresh: Option<std::time::Instant>,
    /// session_start of the session whose START event we already announced.
    /// Prevents firing a "new session" alert more than once per 5h window.
    pub session_start_alerted: Option<String>,
    /// When true, the last poll observed an active subscription session.
    /// Transitioning from true → false fires a "session ended" alert.
    pub had_active_session: bool,
    /// Percentage thresholds already fired for the current 5h session.
    /// Keyed by session_start so we reset when a new session begins.
    pub fired_session_thresholds: Vec<u8>,
    pub fired_session_thresholds_key: Option<String>,
    /// Same for the weekly window.
    pub fired_week_thresholds: Vec<u8>,
    pub fired_week_thresholds_key: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            peak_level: PeakLevel::default(),
            stats: ClaudeStats::default(),
            analytics: ProjectAnalytics::default(),
            subscription_usage: SubscriptionUsage::default(),
            service_status: ServiceStatus::default(),
            settings: UserSettings::default(),
            previous_color: PeakColor::Green,
            token_alert_fired_today: None,
            last_force_refresh: None,
            session_start_alerted: None,
            had_active_session: false,
            fired_session_thresholds: Vec::new(),
            fired_session_thresholds_key: None,
            fired_week_thresholds: Vec::new(),
            fired_week_thresholds_key: None,
        }
    }
}

/// Wrapper so Tauri can manage it
pub struct AppStateWrapper(pub Mutex<AppState>);

impl AppStateWrapper {
    /// Poison-safe lock helper. If another thread panicked while holding the
    /// mutex, the guard is still returned (with a logged warning) rather than
    /// propagating the panic into a Tauri command handler and crashing the
    /// app. `AppState` only holds plain data (no invariants that a mid-update
    /// panic could violate), so recovering is safe.
    pub fn lock(&self) -> MutexGuard<'_, AppState> {
        self.0.lock().unwrap_or_else(|poisoned| {
            log::error!("AppState mutex was poisoned; recovering inner data");
            poisoned.into_inner()
        })
    }
}
