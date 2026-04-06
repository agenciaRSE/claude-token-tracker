import { useStats } from "../../hooks/useStats";
import { UsageChart } from "./UsageChart";
import { PeakHoursGrid } from "./PeakHoursGrid";
import { formatTokens } from "../../lib/format";

export function HistoryTab() {
  const { stats } = useStats();

  if (!stats) {
    return (
      <div className="flex items-center justify-center h-40 text-foreground/30 text-sm">
        Loading history...
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      {/* 24h usage chart */}
      <div className="p-4 rounded-xl bg-white/3">
        <div className="text-xs font-medium text-foreground/60 mb-3">
          Activity by Hour (Messages)
        </div>
        <UsageChart hourCounts={stats.hourCounts} />
      </div>

      {/* Daily token trend */}
      <div className="p-4 rounded-xl bg-white/3">
        <div className="text-xs font-medium text-foreground/60 mb-3">
          Daily Token Usage
        </div>
        {stats.dailyTokens.length > 0 ? (
          <div className="space-y-1.5">
            {stats.dailyTokens.map((day) => {
              const maxTokens = Math.max(
                ...stats.dailyTokens.map((d) => d.tokens),
                1,
              );
              const pct = (day.tokens / maxTokens) * 100;

              return (
                <div key={day.date} className="flex items-center gap-2">
                  <span className="text-[10px] text-foreground/40 w-16 shrink-0">
                    {day.date.slice(5)} {/* MM-DD */}
                  </span>
                  <div className="flex-1 h-2 bg-white/5 rounded-full overflow-hidden">
                    <div
                      className="h-full rounded-full bg-blue-400/50 transition-all"
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                  <span className="text-[10px] text-foreground/50 w-12 text-right">
                    {formatTokens(day.tokens)}
                  </span>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="text-xs text-foreground/30">No data yet</div>
        )}
      </div>

      {/* Peak hours heatmap */}
      <div className="p-4 rounded-xl bg-white/3">
        <div className="text-xs font-medium text-foreground/60 mb-3">
          Your Peak Activity Hours
        </div>
        <PeakHoursGrid hourCounts={stats.hourCounts} />
      </div>

      {/* Totals */}
      <div className="grid grid-cols-2 gap-3">
        <div className="p-3 rounded-lg bg-white/3 text-center">
          <div className="text-[10px] text-foreground/40 mb-1">Total Sessions</div>
          <div className="text-lg font-bold text-foreground/80">
            {stats.totalSessions}
          </div>
        </div>
        <div className="p-3 rounded-lg bg-white/3 text-center">
          <div className="text-[10px] text-foreground/40 mb-1">Total Messages</div>
          <div className="text-lg font-bold text-foreground/80">
            {stats.totalMessages}
          </div>
        </div>
      </div>
    </div>
  );
}
