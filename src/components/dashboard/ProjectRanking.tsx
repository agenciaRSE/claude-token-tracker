import type { ProjectStats } from "../../types/analytics";
import type { CostMode } from "../../types/peak";
import { formatTokens, formatCost, getCostLabel } from "../../lib/format";

interface Props {
  projects: ProjectStats[];
  costMode: CostMode;
}

export function ProjectRanking({ projects, costMode }: Props) {
  const costLabel = getCostLabel(costMode);

  if (projects.length === 0) {
    return (
      <div className="text-xs text-foreground/30 text-center py-4">
        No project data yet
      </div>
    );
  }

  const maxTokens = Math.max(...projects.map((p) => p.totalTokens), 1);

  return (
    <div className="p-3 rounded-lg bg-white/3">
      <div className="text-[10px] text-foreground/40 mb-3 flex items-center justify-between">
        <span>Token Usage by Project</span>
        <span className="text-foreground/30">{costLabel}</span>
      </div>
      <div className="space-y-2">
        {projects.slice(0, 12).map((project) => {
          const pct = (project.totalTokens / maxTokens) * 100;

          return (
            <div key={project.dirName} className="space-y-1">
              <div className="flex items-center justify-between gap-2">
                <span
                  className="text-[11px] text-foreground/70 truncate flex-1"
                  title={project.dirName}
                >
                  {project.name}
                </span>
                <div className="flex items-center gap-2 shrink-0">
                  <span className="text-[9px] text-foreground/30">
                    {formatTokens(project.totalTokens)}
                  </span>
                  <span className="text-[10px] text-foreground/50 w-14 text-right">
                    {formatCost(project.totalCostUsd)}
                  </span>
                </div>
              </div>
              <div className="flex h-1.5 rounded-full overflow-hidden bg-white/5">
                <div
                  className="h-full rounded-full transition-all"
                  style={{
                    width: `${pct}%`,
                    background: `linear-gradient(90deg, rgba(96,165,250,0.6), rgba(168,85,247,0.6))`,
                  }}
                />
              </div>
              <div className="flex gap-3 text-[9px] text-foreground/25">
                <span>{project.totalSessions} sessions</span>
                <span>{project.totalMessages} msgs</span>
                {project.models.length > 0 && (
                  <span>
                    {project.models
                      .slice(0, 2)
                      .map((m) => {
                        const short = m.model.replace("claude-", "").split("-")[0];
                        return short.charAt(0).toUpperCase() + short.slice(1);
                      })
                      .join(", ")}
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
