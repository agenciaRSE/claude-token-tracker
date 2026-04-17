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
  const currentHour = new Date().getUTCHours();

  return (
    <div>
      {/* Chart body — plain CSS flex so bars don't get warped by SVG scaling */}
      <div
        className="relative flex items-end gap-[2px] w-full"
        style={{ height: 96 }}
      >
        {/* Horizontal gridlines (25%, 50%, 75%) for reference */}
        <div className="absolute inset-0 flex flex-col justify-between pointer-events-none">
          <div className="h-px bg-white/5" />
          <div className="h-px bg-white/5" />
          <div className="h-px bg-white/5" />
          <div className="h-px bg-white/10" />
        </div>

        {hours.map((h) => {
          const pct = (h.count / maxCount) * 100;
          const isCurrent = h.hour === currentHour;
          const hasActivity = h.count > 0;

          return (
            <div
              key={h.hour}
              className="flex-1 flex flex-col justify-end group relative"
              style={{ height: "100%" }}
              title={`${formatHour(h.hour)} UTC — ${h.count} message${h.count === 1 ? "" : "s"}`}
            >
              {/* Bar */}
              <div
                className="w-full rounded-sm transition-all"
                style={{
                  height: `${Math.max(pct, hasActivity ? 4 : 2)}%`,
                  minHeight: hasActivity ? 3 : 2,
                  backgroundColor: isCurrent
                    ? "rgba(34, 197, 94, 0.9)"
                    : hasActivity
                      ? "rgba(96, 165, 250, 0.6)"
                      : "rgba(255, 255, 255, 0.08)",
                  boxShadow: isCurrent
                    ? "0 0 8px rgba(34,197,94,0.4)"
                    : "none",
                }}
              />
            </div>
          );
        })}
      </div>

      {/* X-axis labels — positioned in HTML instead of SVG for crisp text */}
      <div className="flex justify-between text-[9px] text-foreground/30 mt-1 px-[1px]">
        <span>12 AM</span>
        <span>6 AM</span>
        <span>12 PM</span>
        <span>6 PM</span>
        <span>11 PM</span>
      </div>

      <div className="flex items-center gap-3 mt-2 text-[9px] text-foreground/30">
        <span className="flex items-center gap-1">
          <span className="w-2 h-2 rounded-sm bg-green-500/80" /> Current hour (UTC)
        </span>
        <span className="flex items-center gap-1">
          <span className="w-2 h-2 rounded-sm bg-blue-400/60" /> Activity
        </span>
        <span className="flex items-center gap-1">
          <span className="w-2 h-2 rounded-sm bg-white/10" /> Idle
        </span>
      </div>
    </div>
  );
}
