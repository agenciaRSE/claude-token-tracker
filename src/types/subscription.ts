import type { SubscriptionPlan } from "./peak";

export interface SubscriptionUsage {
  // 5-hour session window
  sessionActive: boolean;
  sessionStart: string | null;
  sessionEnd: string | null;
  sessionTokens: number;
  sessionCostUsd: number;
  sessionMessages: number;
  sessionLimitTokens: number;
  sessionPct: number;
  sessionSecondsUntilReset: number;
  sessionExtraCostUsd: number;

  // Weekly window
  weekStart: string | null;
  weekEnd: string | null;
  weekTokens: number;
  weekCostUsd: number;
  weekMessages: number;
  weekLimitTokens: number;
  weekPct: number;
  weekSecondsUntilReset: number;
  weekExtraCostUsd: number;
}

export interface SubscriptionWarning {
  scope: "session" | "week";
  pct: number;
  secondsToReset: number;
}

export const SUBSCRIPTION_PLAN_LABELS: Record<SubscriptionPlan, string> = {
  pro: "Pro",
  max5x: "Max 5×",
  max20x: "Max 20×",
  custom: "Custom",
};

export const WEEKDAY_LABELS = [
  "Sunday",
  "Monday",
  "Tuesday",
  "Wednesday",
  "Thursday",
  "Friday",
  "Saturday",
] as const;
