import { usePeakLevel } from "../../hooks/usePeakLevel";
import { useStats } from "../../hooks/useStats";
import { PEAK_COLORS, PEAK_LABELS, PEAK_GLOW_COLORS } from "../../types/peak";
import { ServiceStatusCards } from "./ServiceStatusCards";
import { TokenBreakdown } from "./TokenBreakdown";
import { formatTokens, formatCost, formatRelativeTime } from "../../lib/format";

export function OverviewTab() {
  const { peakLevel } = usePeakLevel();
  const { stats } = useStats();

  if (!peakLevel) return null;

  const color = PEAK_COLORS[peakLevel.color];

  return (
    <div className="flex flex-col gap-4">
      {/* Peak level hero card */}
      <div className="flex items-center gap-4 p-4 rounded-xl bg-white/3">
        <div
          className="w-16 h-16 rounded-full flex items-center justify-center shrink-0"
          style={{
            background: `radial-gradient(circle at 35% 35%, ${color}dd, ${color}66)`,
            boxShadow: PEAK_GLOW_COLORS[peakLevel.color],
          }}
        >
          <span className="text-xl font-bold text-white">{peakLevel.score}</span>
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-lg font-semibold" style={{ color }}>
              {PEAK_LABELS[peakLevel.color]} Usage
            </span>
          </div>
          <p className="text-xs text-foreground/50 mt-1">
            {peakLevel.recommendation}
          </p>
          <p className="text-[10px] text-foreground/30 mt-1">
            Updated {formatRelativeTime(peakLevel.updatedAt)}
          </p>
        </div>
      </div>

      {/* Score breakdown */}
      <div className="grid grid-cols-3 gap-3">
        <ScoreCard label="Time Pattern" value={peakLevel.timeScore} weight="40%" color={color} />
        <ScoreCard label="Service Status" value={peakLevel.statusScore} weight="35%" color={color} />
        <ScoreCard label="Your Usage" value={peakLevel.usageScore} weight="25%" color={color} />
      </div>

      {/* Today's summary */}
      {stats && (
        <div className="grid grid-cols-4 gap-2">
          <StatCard label="Messages" value={stats.todayMessages.toString()} />
          <StatCard label="Sessions" value={stats.todaySessions.toString()} />
          <StatCard label="Tokens" value={formatTokens(stats.todayTokens)} />
          <StatCard label="Cost" value={formatCost(stats.todayCostUsd)} />
        </div>
      )}

      {/* Service status cards */}
      <ServiceStatusCards />

      {/* Token breakdown by model */}
      {stats && stats.modelUsage.length > 0 && (
        <TokenBreakdown models={stats.modelUsage} />
      )}
    </div>
  );
}

function ScoreCard({
  label,
  value,
  weight,
  color,
}: {
  label: string;
  value: number;
  weight: string;
  color: string;
}) {
  return (
    <div className="p-3 rounded-lg bg-white/3">
      <div className="text-[10px] text-foreground/40 mb-2">
        {label} <span className="text-foreground/20">({weight})</span>
      </div>
      <div className="text-lg font-bold" style={{ color: value > 50 ? color : undefined }}>
        {value}
      </div>
      <div className="mt-1.5 h-1 bg-white/5 rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-700"
          style={{ width: `${value}%`, background: color, opacity: 0.6 }}
        />
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="p-2.5 rounded-lg bg-white/3 text-center">
      <div className="text-[10px] text-foreground/40 mb-1">{label}</div>
      <div className="text-sm font-semibold text-foreground/80">{value}</div>
    </div>
  );
}
