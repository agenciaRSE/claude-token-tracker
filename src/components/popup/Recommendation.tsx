import type { PeakLevel } from "../../types/peak";
import { PEAK_BG_COLORS, PEAK_COLORS } from "../../types/peak";

interface Props {
  peakLevel: PeakLevel;
}

export function Recommendation({ peakLevel }: Props) {
  const bgColor = PEAK_BG_COLORS[peakLevel.color];
  const textColor = PEAK_COLORS[peakLevel.color];

  return (
    <div
      className="rounded-lg p-3 transition-colors duration-500"
      style={{ background: bgColor }}
    >
      <div
        className="text-[10px] uppercase tracking-wider font-medium mb-1"
        style={{ color: textColor }}
      >
        Recommendation
      </div>
      <p className="text-xs text-foreground/70 leading-relaxed">
        {peakLevel.recommendation}
      </p>
    </div>
  );
}
