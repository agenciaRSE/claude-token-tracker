use serde::Deserialize;
use chrono::Utc;

use crate::state::{ServiceComponent, ServiceStatus};

/// Statuspage.io component response shape
#[derive(Debug, Deserialize)]
struct StatusPageResponse {
    #[serde(default)]
    components: Vec<StatusPageComponent>,
}

#[derive(Debug, Deserialize)]
struct StatusPageComponent {
    name: String,
    status: String,
    #[serde(default)]
    group: bool,
}

/// Relevant Anthropic service names to track
const TRACKED_SERVICES: &[&str] = &[
    "Claude API",
    "Claude Code",
    "claude.ai",
    "platform.claude.com",
];

/// Fetch current service status from Anthropic's status page
pub async fn fetch_service_status() -> ServiceStatus {
    let url = "https://status.anthropic.com/api/v2/components.json";

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to create HTTP client: {}", e);
            return ServiceStatus::default();
        }
    };

    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("Failed to fetch status page: {}", e);
            return ServiceStatus {
                components: vec![],
                overall: "unknown".to_string(),
                fetched_at: Utc::now().to_rfc3339(),
            };
        }
    };

    let page: StatusPageResponse = match response.json().await {
        Ok(p) => p,
        Err(e) => {
            log::warn!("Failed to parse status page response: {}", e);
            return ServiceStatus {
                components: vec![],
                overall: "unknown".to_string(),
                fetched_at: Utc::now().to_rfc3339(),
            };
        }
    };

    // Filter to only tracked Anthropic services (skip group headers)
    let components: Vec<ServiceComponent> = page.components.iter()
        .filter(|c| !c.group && TRACKED_SERVICES.iter().any(|&name| c.name.contains(name)))
        .map(|c| ServiceComponent {
            name: c.name.clone(),
            status: c.status.clone(),
        })
        .collect();

    // Overall status = worst component status
    let overall = components.iter()
        .map(|c| match c.status.as_str() {
            "major_outage" => 4,
            "partial_outage" => 3,
            "degraded_performance" => 2,
            "operational" => 1,
            _ => 0,
        })
        .max()
        .map(|level| match level {
            4 => "major_outage",
            3 => "partial_outage",
            2 => "degraded_performance",
            1 => "operational",
            _ => "unknown",
        })
        .unwrap_or("operational")
        .to_string();

    ServiceStatus {
        components,
        overall,
        fetched_at: Utc::now().to_rfc3339(),
    }
}
