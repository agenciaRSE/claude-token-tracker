import type { SessionSummary } from "../../types/analytics";
import type { CostMode } from "../../types/peak";
import { formatTokens, formatCost, formatRelativeTime, getCostLabel } from "../../lib/format";

interface Props {
  sessions: SessionSummary[];
  costMode: CostMode;
}

const MODE_DOT: Record<string, string> = {
  Code: "#22c55e",
  Desktop: "#60a5fa",
  Other: "#a78bfa",
  Unknown: "#6b7280",
};

export function SessionsTable({ sessions, costMode }: Props) {
  const costLabel = getCostLabel(costMode);

  if (sessions.length === 0) {
    return (
      <div className="text-xs text-foreground/30 text-center py-4">
        No session data yet
      </div>
    );
  }

  return (
    <div className="p-3 rounded-lg bg-white/3">
      <div className="text-[10px] text-foreground/40 mb-3 flex items-center justify-between">
        <span>Top Sessions by {costLabel}</span>
        <span className="text-foreground/30">Top {Math.min(sessions.length, 20)}</span>
      </div>

      <div className="space-y-1">
        {/* Header */}
        <div className="flex items-center gap-2 text-[9px] text-foreground/30 pb-1 border-b border-white/5">
          <span className="flex-1">Project</span>
          <span className="w-12 text-right">Tokens</span>
          <span className="w-12 text-right">{costLabel}</span>
          <span className="w-8 text-right">Msgs</span>
          <span className="w-14 text-right">When</span>
        </div>

        {sessions.slice(0, 20).map((session, i) => (
          <div
            key={session.sessionId}
            className="flex items-center gap-2 py-1 group hover:bg-white/3 rounded-sm px-0.5 -mx-0.5"
          >
            {/* Mode dot + project name */}
            <div className="flex items-center gap-1.5 flex-1 min-w-0">
              <span
                className="w-1.5 h-1.5 rounded-full shrink-0"
                style={{ backgroundColor: MODE_DOT[session.mode] ?? MODE_DOT.Other }}
                title={session.mode}
              />
              <span
                className="text-[10px] text-foreground/60 truncate"
                title={`${session.project} (${session.mode})`}
              >
                {i + 1}. {session.project}
              </span>
            </div>

            <span className="w-12 text-right text-[10px] text-foreground/40">
              {formatTokens(session.totalTokens)}
            </span>
            <span className="w-12 text-right text-[10px] text-foreground/50">
              {formatCost(session.totalCostUsd)}
            </span>
            <span className="w-8 text-right text-[10px] text-foreground/30">
              {session.messages}
            </span>
            <span
              className="w-14 text-right text-[9px] text-foreground/25"
              title={session.lastActivity}
            >
              {formatRelativeTime(session.lastActivity)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
