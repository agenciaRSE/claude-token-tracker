import { useEffect, useState } from "react";
import type { SubscriptionUsage } from "../../types/subscription";
import type { CostMode } from "../../types/peak";
import { formatTokens, formatCost, formatDuration } from "../../lib/format";

interface Props {
  usage: SubscriptionUsage | null;
  costMode: CostMode;
}

/** Live countdown derived from the session_end absolute timestamp so it
 *  stays accurate between scheduler polls. */
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

export function QuickStats({ usage, costMode }: Props) {
  const sessionSecondsLeft = useCountdown(
    usage?.sessionActive ? (usage?.sessionEnd ?? null) : null,
    usage?.sessionSecondsUntilReset ?? 0,
  );

  if (!usage) {
    return (
      <div className="grid grid-cols-2 gap-2">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="bg-white/5 rounded-lg p-2.5 animate-pulse h-14" />
        ))}
      </div>
    );
  }

  const active = usage.sessionActive;
  const isSubscription = costMode === "subscription";

  // ── Fourth card is mode-aware ────────────────────────────────────
  //  API mode:          "Cost"          (raw API cost for this session)
  //  Subscription mode: "Extra"         (overflow cost if over plan limit,
  //                                     otherwise "Included" — no $ number
  //                                     shown because the plan already
  //                                     covers the usage.)
  const extraCard: Item = isSubscription
    ? {
        label: "Extra",
        value:
          usage.sessionExtraCostUsd > 0
            ? `+${formatCost(usage.sessionExtraCostUsd)}`
            : "Included",
        title:
          usage.sessionExtraCostUsd > 0
            ? "Estimated API-equivalent cost of tokens beyond your plan's 5h allowance."
            : "Your subscription covers this session's usage.",
        icon: (
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <path d="M8 14s1.5 2 4 2 4-2 4-2" />
            <line x1="9" y1="9" x2="9.01" y2="9" />
            <line x1="15" y1="9" x2="15.01" y2="9" />
          </svg>
        ),
        emphasize: usage.sessionExtraCostUsd > 0,
      }
    : {
        label: "Cost",
        value: formatCost(usage.sessionCostUsd),
        title: "Estimated API cost for this 5-hour session.",
        icon: (
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="12" y1="1" x2="12" y2="23" />
            <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
          </svg>
        ),
      };

  const items: Item[] = [
    {
      label: "Messages",
      value: active ? usage.sessionMessages.toString() : "—",
      title: "Assistant responses in the current 5-hour session",
      icon: (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
        </svg>
      ),
    },
    {
      label: "Tokens",
      value: active ? formatTokens(usage.sessionTokens) : "—",
      title: "Tokens used in the current 5-hour session",
      icon: (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
        </svg>
      ),
    },
    {
      label: "Time left",
      value: active && sessionSecondsLeft > 0 ? formatDuration(sessionSecondsLeft) : "—",
      title: active
        ? "Time remaining before the 5-hour session window resets"
        : "No active session — the next message starts a fresh 5h window",
      icon: (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="10" />
          <polyline points="12 6 12 12 16 14" />
        </svg>
      ),
    },
    extraCard,
  ];

  return (
    <div className="grid grid-cols-2 gap-2">
      {items.map((item) => (
        <div
          key={item.label}
          className="bg-white/5 rounded-lg p-2.5 flex flex-col gap-1"
          title={item.title}
        >
          <div className="flex items-center gap-1.5 text-foreground/40">
            {item.icon}
            <span className="text-[10px] uppercase tracking-wider">
              {item.label}
            </span>
          </div>
          <div
            className={`text-sm font-semibold ${
              item.emphasize ? "text-orange-400" : "text-foreground/90"
            }`}
          >
            {item.value}
          </div>
        </div>
      ))}
    </div>
  );
}

type Item = {
  label: string;
  value: string;
  title?: string;
  icon: React.ReactNode;
  emphasize?: boolean;
};
