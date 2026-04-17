/** Format large token counts: 125432 -> "125.4K" */
export function formatTokens(n: number): string {
  if (n >= 1_000_000) {
    return (n / 1_000_000).toFixed(1) + "M";
  }
  if (n >= 1_000) {
    return (n / 1_000).toFixed(1) + "K";
  }
  return n.toString();
}

/** Format cost in USD: 0.123456 -> "$0.12" */
export function formatCost(usd: number): string {
  if (usd < 0.01) {
    return usd === 0 ? "$0.00" : "<$0.01";
  }
  return "$" + usd.toFixed(2);
}

import type { CostMode } from "../types/peak";

/**
 * Short label shown above a cost figure. In API mode it's the literal
 * money owed; in subscription mode the same number is reframed as the
 * "API equivalent" the user would have paid on pay-per-token billing.
 */
export function getCostLabel(mode: CostMode): string {
  return mode === "subscription" ? "API equiv." : "Cost";
}

/** Long form used in tooltips / aria-labels. */
export function getCostDescription(mode: CostMode): string {
  return mode === "subscription"
    ? "Estimated value extracted from your Claude subscription, based on Anthropic's published API list pricing. You don't actually pay this amount — your flat monthly fee covers it."
    : "Estimated cost based on Anthropic's published API list pricing.";
}

/** Format a model name to a short display name */
export function formatModelName(model: string): string {
  // "claude-sonnet-4-5-20250929" -> "Sonnet 4.5"
  const parts = model.replace("claude-", "").split("-");
  if (parts.length >= 3) {
    const name = parts[0].charAt(0).toUpperCase() + parts[0].slice(1);
    const version = parts.slice(1).filter((p) => !p.match(/^\d{8}$/)).join(".");
    return `${name} ${version}`;
  }
  return model;
}

/** Format hour number to display: 14 -> "2 PM" */
export function formatHour(hour: number): string {
  if (hour === 0) return "12 AM";
  if (hour === 12) return "12 PM";
  if (hour < 12) return `${hour} AM`;
  return `${hour - 12} PM`;
}

/** Format a duration in seconds as a compact countdown.
 *   3661 -> "1h 01m"
 *   90   -> "1m 30s"
 *   86400*2 + 3600*3 -> "2d 3h"
 */
export function formatDuration(totalSeconds: number): string {
  if (totalSeconds <= 0) return "now";
  const days = Math.floor(totalSeconds / 86400);
  const hours = Math.floor((totalSeconds % 86400) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${String(minutes).padStart(2, "0")}m`;
  if (minutes > 0) return `${minutes}m ${String(seconds).padStart(2, "0")}s`;
  return `${seconds}s`;
}

/** Given a subscription usage percentage, return a color token. */
export function usagePctColor(pct: number): string {
  if (pct >= 100) return "#ef4444"; // red
  if (pct >= 90) return "#f97316";  // orange
  if (pct >= 70) return "#eab308";  // yellow
  return "#22c55e";                 // green
}

/** Format relative time: ISO string -> "2 min ago" */
export function formatRelativeTime(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const secs = Math.floor(diff / 1000);
  if (secs < 60) return "just now";
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

/** Get the service status display info */
export function formatServiceStatus(
  status: string,
): { label: string; color: string } {
  switch (status) {
    case "operational":
      return { label: "OK", color: "#22c55e" };
    case "degraded_performance":
      return { label: "Slow", color: "#eab308" };
    case "partial_outage":
      return { label: "Partial", color: "#f97316" };
    case "major_outage":
      return { label: "Down", color: "#ef4444" };
    default:
      return { label: "?", color: "#6b7280" };
  }
}
