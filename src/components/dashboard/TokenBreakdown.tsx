import type { ModelUsageEntry } from "../../types/stats";
import { formatTokens, formatCost, formatModelName } from "../../lib/format";

interface Props {
  models: ModelUsageEntry[];
}

export function TokenBreakdown({ models }: Props) {
  return (
    <div className="p-3 rounded-lg bg-white/3">
      <div className="text-[10px] text-foreground/40 mb-2">
        Token Usage by Model
      </div>
      <div className="space-y-2">
        {models.map((model) => {
          const total = model.inputTokens + model.outputTokens;
          const inputPct =
            total > 0 ? (model.inputTokens / total) * 100 : 50;

          return (
            <div key={model.model} className="space-y-1">
              <div className="flex items-center justify-between">
                <span className="text-xs text-foreground/70">
                  {formatModelName(model.model)}
                </span>
                <span className="text-[10px] text-foreground/40">
                  {formatCost(model.costUsd)}
                </span>
              </div>

              {/* Stacked bar: input vs output */}
              <div className="flex h-1.5 rounded-full overflow-hidden bg-white/5">
                <div
                  className="h-full bg-blue-400/60"
                  style={{ width: `${inputPct}%` }}
                  title={`Input: ${formatTokens(model.inputTokens)}`}
                />
                <div
                  className="h-full bg-purple-400/60"
                  style={{ width: `${100 - inputPct}%` }}
                  title={`Output: ${formatTokens(model.outputTokens)}`}
                />
              </div>

              <div className="flex gap-3 text-[9px] text-foreground/30">
                <span>In: {formatTokens(model.inputTokens)}</span>
                <span>Out: {formatTokens(model.outputTokens)}</span>
                {model.cacheReadTokens > 0 && (
                  <span>Cache: {formatTokens(model.cacheReadTokens)}</span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
