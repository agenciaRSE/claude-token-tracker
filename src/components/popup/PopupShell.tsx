import { getCurrentWindow } from "@tauri-apps/api/window";
import { StatusIndicator } from "./StatusIndicator";
import { QuickStats } from "./QuickStats";
import { Recommendation } from "./Recommendation";
import { PopupFooter } from "./PopupFooter";
import { ServiceStatusRow } from "./ServiceStatusRow";
import { usePeakLevel } from "../../hooks/usePeakLevel";
import { useStats } from "../../hooks/useStats";
import { PEAK_COLORS, type PeakLevel } from "../../types/peak";

// Fallback so the popup always has something valid to render even before
// the first Rust response (or if it fails).
const DEFAULT_PEAK_LEVEL: PeakLevel = {
  color: "green",
  score: 0,
  timeScore: 0,
  statusScore: 0,
  usageScore: 0,
  recommendation: "Waiting for data...",
  updatedAt: new Date().toISOString(),
};

export function PopupShell() {
  const { peakLevel, isLoading } = usePeakLevel();
  const { stats } = useStats();

  const effectiveLevel = peakLevel ?? DEFAULT_PEAK_LEVEL;
  const borderColor = PEAK_COLORS[effectiveLevel.color];

  return (
    <div
      className="h-full flex flex-col overflow-hidden"
      style={{
        background: "oklch(0.13 0 0 / 97%)",
        borderRadius: 12,
        border: `1px solid ${borderColor}33`,
      }}
    >
      {/* Title bar - draggable. Interactive children must opt out with
          data-tauri-drag-region="false" or their clicks get hijacked. */}
      <div
        className="flex items-center justify-between px-3 py-2 shrink-0"
        data-tauri-drag-region
      >
        <div className="flex items-center gap-2" data-tauri-drag-region>
          <div
            className="w-2.5 h-2.5 rounded-full"
            style={{ background: borderColor }}
            data-tauri-drag-region
          />
          <span
            className="text-xs font-medium text-foreground/70"
            data-tauri-drag-region
          >
            Claude Peak Monitor
          </span>
        </div>
        <button
          data-tauri-drag-region="false"
          onClick={() => getCurrentWindow().hide()}
          className="w-5 h-5 rounded flex items-center justify-center text-foreground/40 hover:text-foreground/80 hover:bg-white/10 transition-colors"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path
              d="M1 1L9 9M9 1L1 9"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
        </button>
      </div>

      {/* Main content */}
      <div className="flex-1 flex flex-col gap-3 px-4 pb-3 overflow-y-auto">
        {isLoading && !peakLevel ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-foreground/40 text-sm">Loading...</div>
          </div>
        ) : (
          <>
            <StatusIndicator peakLevel={effectiveLevel} />
            <QuickStats stats={stats} />
            <ServiceStatusRow />
            <Recommendation peakLevel={effectiveLevel} />
          </>
        )}
      </div>

      {/* Footer */}
      <PopupFooter />
    </div>
  );
}
