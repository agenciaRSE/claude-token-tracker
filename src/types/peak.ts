export type PeakColor = "green" | "yellow" | "orange" | "red";

export interface PeakLevel {
  color: PeakColor;
  score: number;
  timeScore: number;
  statusScore: number;
  usageScore: number;
  recommendation: string;
  updatedAt: string;
}

export interface ServiceComponent {
  name: string;
  status:
    | "operational"
    | "degraded_performance"
    | "partial_outage"
    | "major_outage";
}

export interface ServiceStatus {
  components: ServiceComponent[];
  overall: string;
  fetchedAt: string;
}

/**
 * How the user is billed for Claude. Drives the cost label and the legend
 * shown across the UI.
 *  - "api"          : actual pay-per-token API billing — cost = real money.
 *  - "subscription" : flat-fee plan (Pro / Max) — cost = estimated value
 *                     extracted from the subscription, not money owed.
 */
export type CostMode = "api" | "subscription";

export type SubscriptionPlan = "pro" | "max5x" | "max20x" | "custom";

export interface UserSettings {
  timezone: string;
  notificationsEnabled: boolean;
  notifyOnColorChange: boolean;
  dailyTokenAlert: number | null;
  refreshIntervalSecs: number;
  autostart: boolean;
  costMode: CostMode;
  // Subscription tracking (only applied when costMode === "subscription")
  subscriptionPlan: SubscriptionPlan;
  sessionTokenLimit: number;       // 0 = plan default
  weeklyTokenLimit: number;        // 0 = plan default
  weeklyResetWeekday: number;      // 0=Sunday ... 6=Saturday
  weeklyResetHour: number;         // 0-23 UTC
  subscriptionWarnPct: number;     // 10-100
  subscriptionWarningsEnabled: boolean;
}

export const PEAK_COLORS: Record<PeakColor, string> = {
  green: "#22c55e",
  yellow: "#eab308",
  orange: "#f97316",
  red: "#ef4444",
};

export const PEAK_LABELS: Record<PeakColor, string> = {
  green: "Low",
  yellow: "Moderate",
  orange: "High",
  red: "Peak",
};

export const PEAK_BG_COLORS: Record<PeakColor, string> = {
  green: "rgba(34, 197, 94, 0.15)",
  yellow: "rgba(234, 179, 8, 0.15)",
  orange: "rgba(249, 115, 22, 0.15)",
  red: "rgba(239, 68, 68, 0.15)",
};

export const PEAK_GLOW_COLORS: Record<PeakColor, string> = {
  green: "0 0 20px rgba(34, 197, 94, 0.4)",
  yellow: "0 0 20px rgba(234, 179, 8, 0.4)",
  orange: "0 0 20px rgba(249, 115, 22, 0.4)",
  red: "0 0 20px rgba(239, 68, 68, 0.5)",
};
