import type { HourCount } from "../../types/stats";
import { formatHour } from "../../lib/format";

interface Props {
  hourCounts: HourCount[];
}

export function UsageChart({ hourCounts }: Props) {
  // Build full 24h array
  const hours = Array.from({ length: 24 }, (_, i) => {
    const found = hourCounts.find((h) => h.hour === i);
    return { hour: i, count: found?.count ?? 0 };
  });

  const maxCount = Math.max(...hours.map((h) => h.count), 1);
  const barWidth = 100 / 24;
  const chartHeight = 80;

  // Current hour for highlighting
  const currentHour = new Date().getUTCHours();

  return (
    <div>
      <svg
        width="100%"
        height={chartHeight + 20}
        viewBox={`0 0 100 ${chartHeight + 20}`}
        preserveAspectRatio="none"
      >
        {hours.map((h) => {
          const barHeight = (h.count / maxCount) * chartHeight;
          const x = h.hour * barWidth;
          const isCurrent = h.hour === currentHour;

          return (
            <g key={h.hour}>
              <rect
                x={x + barWidth * 0.15}
                y={chartHeight - barHeight}
                width={barWidth * 0.7}
                height={Math.max(barHeight, 0.5)}
                rx={0.5}
                fill={
                  isCurrent
                    ? "#22c55e"
                    : h.count > 0
                      ? "rgba(96, 165, 250, 0.5)"
                      : "rgba(255, 255, 255, 0.05)"
                }
                opacity={isCurrent ? 0.9 : 0.7}
              />
              {/* Hour label - show every 6 hours */}
              {h.hour % 6 === 0 && (
                <text
                  x={x + barWidth / 2}
                  y={chartHeight + 12}
                  textAnchor="middle"
                  fontSize="3"
                  fill="rgba(255, 255, 255, 0.3)"
                >
                  {formatHour(h.hour)}
                </text>
              )}
            </g>
          );
        })}

        {/* Baseline */}
        <line
          x1="0"
          y1={chartHeight}
          x2="100"
          y2={chartHeight}
          stroke="rgba(255, 255, 255, 0.05)"
          strokeWidth="0.3"
        />
      </svg>

      <div className="flex items-center gap-3 mt-1 text-[9px] text-foreground/30">
        <span className="flex items-center gap-1">
          <span className="w-2 h-2 rounded-sm bg-green-500/80" /> Current hour
        </span>
        <span className="flex items-center gap-1">
          <span className="w-2 h-2 rounded-sm bg-blue-400/50" /> Activity
        </span>
      </div>
    </div>
  );
}
