import type { ClaudeStats } from "../../types/stats";
import { formatTokens, formatCost } from "../../lib/format";

interface Props {
  stats: ClaudeStats | null;
}

export function QuickStats({ stats }: Props) {
  if (!stats) {
    return (
      <div className="grid grid-cols-2 gap-2">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="bg-white/5 rounded-lg p-2.5 animate-pulse h-14" />
        ))}
      </div>
    );
  }

  const items = [
    {
      label: "Messages",
      value: stats.todayMessages.toString(),
      icon: (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
        </svg>
      ),
    },
    {
      label: "Sessions",
      value: stats.todaySessions.toString(),
      icon: (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
          <line x1="16" y1="2" x2="16" y2="6" /><line x1="8" y1="2" x2="8" y2="6" />
          <line x1="3" y1="10" x2="21" y2="10" />
        </svg>
      ),
    },
    {
      label: "Tokens",
      value: formatTokens(stats.todayTokens),
      icon: (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
        </svg>
      ),
    },
    {
      label: "Cost",
      value: formatCost(stats.todayCostUsd),
      icon: (
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <line x1="12" y1="1" x2="12" y2="23" />
          <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
        </svg>
      ),
    },
  ];

  return (
    <div className="grid grid-cols-2 gap-2">
      {items.map((item) => (
        <div
          key={item.label}
          className="bg-white/5 rounded-lg p-2.5 flex flex-col gap-1"
        >
          <div className="flex items-center gap-1.5 text-foreground/40">
            {item.icon}
            <span className="text-[10px] uppercase tracking-wider">
              {item.label}
            </span>
          </div>
          <div className="text-sm font-semibold text-foreground/90">
            {item.value}
          </div>
        </div>
      ))}
    </div>
  );
}
