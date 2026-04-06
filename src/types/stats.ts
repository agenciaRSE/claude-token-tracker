export interface HourCount {
  hour: number;
  count: number;
}

export interface DailyTokens {
  date: string;
  tokens: number;
}

export interface ModelUsageEntry {
  model: string;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
  costUsd: number;
}

export interface ClaudeStats {
  todayMessages: number;
  todaySessions: number;
  todayTokens: number;
  todayCostUsd: number;
  totalMessages: number;
  totalSessions: number;
  hourCounts: HourCount[];
  dailyTokens: DailyTokens[];
  modelUsage: ModelUsageEntry[];
}
