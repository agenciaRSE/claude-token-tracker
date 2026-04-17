import type { ModelUsageEntry } from "./stats";

export type TimeRange =
  | "today"
  | "yesterday"
  | "last_7_days"
  | "last_30_days"
  | "this_month"
  | "this_year"
  | "all";

export const TIME_RANGE_LABELS: Record<TimeRange, string> = {
  today: "Today",
  yesterday: "Yesterday",
  last_7_days: "Last 7 days",
  last_30_days: "Last 30 days",
  this_month: "This month",
  this_year: "This year",
  all: "All time",
};

export interface ProjectStats {
  name: string;
  dirName: string;
  totalTokens: number;
  totalCostUsd: number;
  totalMessages: number;
  totalSessions: number;
  models: ModelUsageEntry[];
}

export interface ModeStats {
  mode: string;
  totalTokens: number;
  totalCostUsd: number;
  totalMessages: number;
  totalSessions: number;
}

export interface SessionSummary {
  sessionId: string;
  project: string;
  mode: string;
  totalTokens: number;
  totalCostUsd: number;
  messages: number;
  firstActivity: string;
  lastActivity: string;
}

export interface ProjectAnalytics {
  projects: ProjectStats[];
  modes: ModeStats[];
  sessions: SessionSummary[];
}
