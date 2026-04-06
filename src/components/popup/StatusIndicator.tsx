import type { PeakLevel } from "../../types/peak";
import { PEAK_COLORS, PEAK_LABELS, PEAK_GLOW_COLORS } from "../../types/peak";

interface Props {
  peakLevel: PeakLevel;
}

export function StatusIndicator({ peakLevel }: Props) {
  const color = PEAK_COLORS[peakLevel.color];
  const label = PEAK_LABELS[peakLevel.color];
  const glow = PEAK_GLOW_COLORS[peakLevel.color];

  return (
    <div className="flex flex-col items-center gap-3 py-4">
      {/* Animated colored circle */}
      <div className="relative">
        <div
          className="w-20 h-20 rounded-full flex items-center justify-center transition-all duration-700"
          style={{
            background: `radial-gradient(circle at 35% 35%, ${color}dd, ${color}88)`,
            boxShadow: glow,
          }}
        >
          <span className="text-2xl font-bold text-white drop-shadow-md">
            {peakLevel.score}
          </span>
        </div>
        {/* Pulse ring animation for high levels */}
        {(peakLevel.color === "orange" || peakLevel.color === "red") && (
          <div
            className="absolute inset-0 rounded-full animate-ping opacity-20"
            style={{ background: color }}
          />
        )}
      </div>

      <div className="text-center">
        <div
          className="text-lg font-semibold transition-colors duration-500"
          style={{ color }}
        >
          {label} Usage
        </div>
        <div className="text-xs text-foreground/50 mt-0.5">
          Score: {peakLevel.score} / 100
        </div>
      </div>

      {/* Score breakdown mini-bars */}
      <div className="w-full flex gap-2 px-2">
        <ScoreBar label="Time" value={peakLevel.timeScore} color={color} />
        <ScoreBar label="Status" value={peakLevel.statusScore} color={color} />
        <ScoreBar label="Usage" value={peakLevel.usageScore} color={color} />
      </div>
    </div>
  );
}

function ScoreBar({
  label,
  value,
  color,
}: {
  label: string;
  value: number;
  color: string;
}) {
  return (
    <div className="flex-1">
      <div className="flex justify-between text-[10px] text-foreground/50 mb-0.5">
        <span>{label}</span>
        <span>{value}</span>
      </div>
      <div className="h-1.5 bg-white/5 rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-700"
          style={{
            width: `${value}%`,
            background: color,
            opacity: 0.7,
          }}
        />
      </div>
    </div>
  );
}
