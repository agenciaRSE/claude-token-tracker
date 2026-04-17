import type { ModeStats } from "../../types/analytics";
import type { CostMode } from "../../types/peak";
import { formatTokens, formatCost, getCostLabel } from "../../lib/format";

interface Props {
  modes: ModeStats[];
  costMode: CostMode;
}

const MODE_COLORS: Record<string, string> = {
  Code: "#22c55e",
  Desktop: "#60a5fa",
  Other: "#a78bfa",
  Unknown: "#6b7280",
};

export function ModeBreakdown({ modes, costMode }: Props) {
  const costLabel = getCostLabel(costMode);
  const totalTokens = modes.reduce((sum, m) => sum + m.totalTokens, 0);

  if (modes.length === 0 || totalTokens === 0) {
    return (
      <div className="text-xs text-foreground/30 text-center py-4">
        No mode data yet
      </div>
    );
  }

  // Build donut segments
  const radius = 36;
  const cx = 50;
  const cy = 50;
  const circumference = 2 * Math.PI * radius;
  let offset = 0;

  const segments = modes.map((m) => {
    const pct = m.totalTokens / totalTokens;
    const dashLen = pct * circumference;
    const seg = {
      mode: m.mode,
      pct,
      dashLen,
      offset,
      color: MODE_COLORS[m.mode] ?? MODE_COLORS.Other,
      stats: m,
    };
    offset += dashLen;
    return seg;
  });

  return (
    <div className="p-3 rounded-lg bg-white/3">
      <div className="text-[10px] text-foreground/40 mb-3 flex items-center justify-between">
        <span>Usage by Mode</span>
        <span className="text-foreground/30">{costLabel}</span>
      </div>

      <div className="flex items-center gap-4">
        {/* Donut chart */}
        <div className="shrink-0">
          <svg width="100" height="100" viewBox="0 0 100 100">
            {/* Background ring */}
            <circle
              cx={cx}
              cy={cy}
              r={radius}
              fill="none"
              stroke="rgba(255,255,255,0.05)"
              strokeWidth="10"
            />
            {segments.map((seg) => (
              <circle
                key={seg.mode}
                cx={cx}
                cy={cy}
                r={radius}
                fill="none"
                stroke={seg.color}
                strokeWidth="10"
                strokeDasharray={`${seg.dashLen} ${circumference - seg.dashLen}`}
                strokeDashoffset={-seg.offset}
                opacity="0.75"
                transform={`rotate(-90 ${cx} ${cy})`}
              />
            ))}
            {/* Center text */}
            <text
              x={cx}
              y={cy - 4}
              textAnchor="middle"
              fontSize="10"
              fill="rgba(255,255,255,0.7)"
            >
              {formatTokens(totalTokens)}
            </text>
            <text
              x={cx}
              y={cy + 8}
              textAnchor="middle"
              fontSize="6"
              fill="rgba(255,255,255,0.3)"
            >
              total tokens
            </text>
          </svg>
        </div>

        {/* Legend */}
        <div className="flex-1 space-y-2">
          {segments.map((seg) => (
            <div key={seg.mode} className="flex items-center gap-2">
              <span
                className="w-2.5 h-2.5 rounded-sm shrink-0"
                style={{ backgroundColor: seg.color, opacity: 0.75 }}
              />
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] text-foreground/70">
                    {seg.mode}
                  </span>
                  <span className="text-[10px] text-foreground/50">
                    {(seg.pct * 100).toFixed(1)}%
                  </span>
                </div>
                <div className="flex gap-2 text-[9px] text-foreground/30">
                  <span>{formatTokens(seg.stats.totalTokens)}</span>
                  <span>{formatCost(seg.stats.totalCostUsd)}</span>
                  <span>{seg.stats.totalSessions} sess</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
