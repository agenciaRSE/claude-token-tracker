use serde::{Deserialize, Serialize};
use std::sync::Mutex;

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
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            timezone: "UTC".to_string(),
            notifications_enabled: true,
            notify_on_color_change: true,
            daily_token_alert: None,
            refresh_interval_secs: 120,
            autostart: true,
        }
    }
}

/// Shared application state behind a Mutex
pub struct AppState {
    pub peak_level: PeakLevel,
    pub stats: ClaudeStats,
    pub service_status: ServiceStatus,
    pub settings: UserSettings,
    pub previous_color: PeakColor,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            peak_level: PeakLevel::default(),
            stats: ClaudeStats::default(),
            service_status: ServiceStatus::default(),
            settings: UserSettings::default(),
            previous_color: PeakColor::Green,
        }
    }
}

/// Wrapper so Tauri can manage it
pub struct AppStateWrapper(pub Mutex<AppState>);
