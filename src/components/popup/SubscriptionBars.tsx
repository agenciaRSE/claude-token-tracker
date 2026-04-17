import { useEffect, useState } from "react";
import type { SubscriptionUsage } from "../../types/subscription";
import {
  formatTokens,
  formatCost,
  formatDuration,
  usagePctColor,
} from "../../lib/format";

interface Props {
  usage: SubscriptionUsage;
}

/** Client-side countdown that ticks every second using the absolute reset
 *  timestamp as source of truth — so the displayed time stays accurate even
 *  if the popup is minimized for a while and the scheduler hasn't updated. */
function useCountdown(resetIso: string | null, fallbackSeconds: number): number {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  if (!resetIso) return Math.max(0, fallbackSeconds);
  const target = Date.parse(resetIso);
  if (Number.isNaN(target)) return Math.max(0, fallbackSeconds);
  return Math.max(0, Math.floor((target - now) / 1000));
}

interface BarProps {
  label: string;
  icon: React.ReactNode;
  pct: number;              // 0-999
  resetIso: string | null;
  fallbackSeconds: number;
  tokens: number;
  limitTokens: number;
  costUsd: number;
  subLabel?: string;        // e.g. "Session inactive"
}

function UsageBar({
  label,
  icon,
  pct,
  resetIso,
  fallbackSeconds,
  tokens,
  limitTokens,
  costUsd,
  subLabel,
}: BarProps) {
  const seconds = useCountdown(resetIso, fallbackSeconds);
  const color = usagePctColor(pct);
  const clampedPct = Math.min(pct, 100);

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between text-[10px]">
        <span className="flex items-center gap-1.5 text-foreground/60 uppercase tracking-wider">
          <span className="text-foreground/40">{icon}</span>
          {label}
        </span>
        <span className="text-foreground/40">
          {subLabel ?? (seconds > 0 ? `resets in ${formatDuration(seconds)}` : "resets soon")}
        </span>
      </div>

      <div className="relative h-2 rounded-full bg-white/5 overflow-hidden">
        <div
          className="absolute inset-y-0 left-0 rounded-full transition-all"
          style={{
            width: `${clampedPct}%`,
            backgroundColor: color,
            boxShadow: pct >= 90 ? `0 0 6px ${color}` : "none",
            opacity: 0.85,
          }}
        />
        {/* Overflow stripe when > 100% */}
        {pct > 100 && (
          <div className="absolute inset-y-0 right-0 w-full rounded-full opacity-30"
               style={{
                 background: `repeating-linear-gradient(45deg, ${color}, ${color} 3px, transparent 3px, transparent 6px)`,
               }}
          />
        )}
      </div>

      <div className="flex items-center justify-between text-[9px] text-foreground/40">
        <span>
          <span
            className="font-semibold"
            style={{ color: pct >= 70 ? color : undefined }}
          >
            {pct}%
          </span>
          {limitTokens > 0 && (
            <span className="text-foreground/30">
              {" · "}
              {formatTokens(tokens)} / {formatTokens(limitTokens)}
            </span>
          )}
          {limitTokens === 0 && tokens > 0 && (
            <span className="text-foreground/30"> · {formatTokens(tokens)}</span>
          )}
        </span>
        {costUsd > 0 && (
          <span className="text-foreground/30">{formatCost(costUsd)}</span>
        )}
      </div>
    </div>
  );
}

export function SubscriptionBars({ usage }: Props) {
  const sessionIcon = (
    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <polyline points="12 6 12 12 16 14" />
    </svg>
  );
  const weekIcon = (
    <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
      <line x1="16" y1="2" x2="16" y2="6" />
      <line x1="8" y1="2" x2="8" y2="6" />
      <line x1="3" y1="10" x2="21" y2="10" />
    </svg>
  );

  return (
    <div className="bg-white/3 rounded-lg p-3 flex flex-col gap-3">
      <UsageBar
        label="Session (5h)"
        icon={sessionIcon}
        pct={usage.sessionPct}
        resetIso={usage.sessionActive ? usage.sessionEnd : null}
        fallbackSeconds={usage.sessionSecondsUntilReset}
        tokens={usage.sessionTokens}
        limitTokens={usage.sessionLimitTokens}
        costUsd={usage.sessionCostUsd}
        subLabel={
          !usage.sessionActive
            ? "No active session"
            : undefined
        }
      />
      <UsageBar
        label="Week"
        icon={weekIcon}
        pct={usage.weekPct}
        resetIso={usage.weekEnd}
        fallbackSeconds={usage.weekSecondsUntilReset}
        tokens={usage.weekTokens}
        limitTokens={usage.weekLimitTokens}
        costUsd={usage.weekCostUsd}
      />
    </div>
  );
}
