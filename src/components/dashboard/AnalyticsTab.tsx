import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettings } from "../../hooks/useSettings";
import { ProjectRanking } from "./ProjectRanking";
import { ModeBreakdown } from "./ModeBreakdown";
import { SessionsTable } from "./SessionsTable";
import { TimeRangeSelector } from "./TimeRangeSelector";
import { formatTokens, formatCost } from "../../lib/format";
import {
  type ProjectAnalytics,
  type TimeRange,
  TIME_RANGE_LABELS,
} from "../../types/analytics";

export function AnalyticsTab() {
  const { settings } = useSettings();
  const [range, setRange] = useState<TimeRange>("today");
  const [analytics, setAnalytics] = useState<ProjectAnalytics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchAnalytics = useCallback(async (r: TimeRange) => {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<ProjectAnalytics>("get_analytics_for_range", {
        range: r,
      });
      setAnalytics(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // Initial load + refetch when range changes.
  useEffect(() => {
    fetchAnalytics(range);
  }, [range, fetchAnalytics]);

  // Passive refresh: when the background scheduler fires analytics-updated
  // for the current "Last30Days" cache, refetch our range if it's the same.
  useEffect(() => {
    if (range !== "last_30_days") return;
    const id = setInterval(() => fetchAnalytics(range), 60_000);
    return () => clearInterval(id);
  }, [range, fetchAnalytics]);

  const costMode = settings?.costMode ?? "api";

  // ── Render states ────────────────────────────────────────────────
  if (loading && !analytics) {
    return (
      <div className="flex flex-col gap-4">
        <TimeRangeSelector value={range} onChange={setRange} loading />
        <div className="flex items-center justify-center h-40 text-foreground/30 text-sm">
          Loading analytics…
        </div>
      </div>
    );
  }

  if (error && !analytics) {
    return (
      <div className="flex flex-col gap-4">
        <TimeRangeSelector value={range} onChange={setRange} />
        <div className="p-4 rounded-lg bg-red-500/10 text-red-300 text-xs">
          Failed to load analytics: {error}
        </div>
      </div>
    );
  }

  if (!analytics) return null;

  // Totals
  const totalTokens = analytics.projects.reduce(
    (sum, p) => sum + p.totalTokens,
    0,
  );
  const totalCost = analytics.projects.reduce(
    (sum, p) => sum + p.totalCostUsd,
    0,
  );
  const totalMessages = analytics.projects.reduce(
    (sum, p) => sum + p.totalMessages,
    0,
  );

  const hasData = analytics.projects.length > 0;

  return (
    <div className="flex flex-col gap-4">
      {/* Time range selector */}
      <div className="p-3 rounded-lg bg-white/3">
        <TimeRangeSelector value={range} onChange={setRange} loading={loading} />
        <div className="text-[9px] text-foreground/25 mt-2">
          Showing data for: {TIME_RANGE_LABELS[range]}
        </div>
      </div>

      {!hasData ? (
        <div className="p-6 rounded-lg bg-white/3 text-center text-foreground/30 text-sm">
          No activity in this period.
        </div>
      ) : (
        <>
          {/* Summary cards */}
          <div className="grid grid-cols-4 gap-3">
            <SummaryCard label="Projects" value={String(analytics.projects.length)} />
            <SummaryCard label="Tokens" value={formatTokens(totalTokens)} />
            <SummaryCard label="Messages" value={String(totalMessages)} />
            <SummaryCard label="Cost" value={formatCost(totalCost)} />
          </div>

          {/* Mode breakdown donut */}
          <ModeBreakdown modes={analytics.modes} costMode={costMode} />

          {/* Project ranking */}
          <ProjectRanking projects={analytics.projects} costMode={costMode} />

          {/* Sessions table */}
          <SessionsTable sessions={analytics.sessions} costMode={costMode} />
        </>
      )}
    </div>
  );
}

function SummaryCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-3 rounded-lg bg-white/3 text-center">
      <div className="text-[10px] text-foreground/40 mb-1">{label}</div>
      <div className="text-base font-bold text-foreground/80 truncate">
        {value}
      </div>
    </div>
  );
}
