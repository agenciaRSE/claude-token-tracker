use serde::Deserialize;
use chrono::Utc;

use crate::state::{ServiceComponent, ServiceStatus};

/// Hard cap on the status-page response body. The real payload is ~30 KiB;
/// 1 MiB leaves plenty of headroom while protecting against a
/// misconfigured proxy or hostile MITM trying to exhaust memory.
const MAX_STATUS_RESPONSE_BYTES: usize = 1024 * 1024;

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
        .https_only(true) // refuse to fall back to plaintext if redirected
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .user_agent("ClaudePeakMonitor/0.1 (+https://github.com/agencia-rse)")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to create HTTP client: {}", e);
            return ServiceStatus::default();
        }
    };

    let unknown_status = || ServiceStatus {
        components: vec![],
        overall: "unknown".to_string(),
        fetched_at: Utc::now().to_rfc3339(),
    };

    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("Failed to fetch status page: {}", e);
            return unknown_status();
        }
    };

    // Early-reject oversized payloads via Content-Length if the server sent
    // one — avoids even starting to buffer a multi-GB response.
    if let Some(len) = response.content_length() {
        if len as usize > MAX_STATUS_RESPONSE_BYTES {
            log::warn!("Status page response too large: {} bytes", len);
            return unknown_status();
        }
    }

    // Buffer the body. Defense in depth: the 10s client timeout above
    // bounds the total time window, and we cap the final buffer size here
    // for servers that lie about (or omit) Content-Length.
    let body = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            log::warn!("Failed to read status page body: {}", e);
            return unknown_status();
        }
    };
    if body.len() > MAX_STATUS_RESPONSE_BYTES {
        log::warn!("Status page response exceeded cap: {} bytes", body.len());
        return unknown_status();
    }

    let page: StatusPageResponse = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("Failed to parse status page response: {}", e);
            return unknown_status();
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
