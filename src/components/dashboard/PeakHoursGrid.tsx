import type { HourCount } from "../../types/stats";

interface Props {
  hourCounts: HourCount[];
}

export function PeakHoursGrid({ hourCounts }: Props) {
  const maxCount = Math.max(...hourCounts.map((h) => h.count), 1);

  // Generate 24-hour grid with intensity coloring
  const hours = Array.from({ length: 24 }, (_, i) => {
    const found = hourCounts.find((h) => h.hour === i);
    const count = found?.count ?? 0;
    const intensity = count / maxCount;
    return { hour: i, count, intensity };
  });

  const getColor = (intensity: number): string => {
    if (intensity === 0) return "rgba(255, 255, 255, 0.03)";
    if (intensity < 0.25) return "rgba(34, 197, 94, 0.2)";
    if (intensity < 0.5) return "rgba(234, 179, 8, 0.3)";
    if (intensity < 0.75) return "rgba(249, 115, 22, 0.4)";
    return "rgba(239, 68, 68, 0.5)";
  };

  const formatHourShort = (h: number): string => {
    if (h === 0) return "12a";
    if (h === 12) return "12p";
    if (h < 12) return `${h}a`;
    return `${h - 12}p`;
  };

  return (
    <div>
      <div className="grid grid-cols-12 gap-1">
        {hours.map((h) => (
          <div
            key={h.hour}
            className="aspect-square rounded-sm flex items-center justify-center cursor-default transition-colors"
            style={{ background: getColor(h.intensity) }}
            title={`${formatHourShort(h.hour)} UTC: ${h.count} messages`}
          >
            <span className="text-[7px] text-foreground/30">
              {h.count > 0 ? h.count : ""}
            </span>
          </div>
        ))}
      </div>

      {/* Hour labels */}
      <div className="grid grid-cols-12 gap-1 mt-1">
        {hours
          .filter((_, i) => i % 2 === 0)
          .map((h) => (
            <div
              key={h.hour}
              className="col-span-2 text-center text-[7px] text-foreground/20"
            >
              {formatHourShort(h.hour)}
            </div>
          ))}
      </div>

      {/* Legend */}
      <div className="flex items-center gap-2 mt-2 text-[9px] text-foreground/30">
        <span>Less</span>
        <div className="flex gap-0.5">
          {[0, 0.2, 0.4, 0.7, 1].map((intensity) => (
            <div
              key={intensity}
              className="w-3 h-3 rounded-sm"
              style={{ background: getColor(intensity) }}
            />
          ))}
        </div>
        <span>More</span>
      </div>
    </div>
  );
}
