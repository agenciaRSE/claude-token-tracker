import type { CostMode } from "../../types/peak";

interface Props {
  mode: CostMode;
}

/**
 * Explains how the cost figure on the popup and dashboard is computed,
 * and clarifies that it's an *estimate* based on Anthropic's public API
 * list pricing — not a billed amount. In subscription mode it also makes
 * clear that the user doesn't actually pay this number.
 *
 * Pricing values mirror `model_pricing()` in src-tauri/src/stats_reader.rs.
 * Keep them in sync if Anthropic publishes new rates.
 */
export function CostLegend({ mode }: Props) {
  return (
    <div className="rounded-lg bg-white/3 border border-white/5 p-3 space-y-2.5">
      <div className="flex items-center gap-1.5">
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="text-foreground/40"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="16" x2="12" y2="12" />
          <line x1="12" y1="8" x2="12.01" y2="8" />
        </svg>
        <span className="text-[10px] uppercase tracking-wider text-foreground/50 font-semibold">
          How the cost is calculated
        </span>
      </div>

      <p className="text-[11px] text-foreground/60 leading-relaxed">
        The cost shown across the popup and dashboard is an{" "}
        <span className="text-foreground/80">estimate</span> built locally
        from the token counts in your{" "}
        <code className="px-1 py-0.5 rounded bg-white/5 text-[10px]">
          ~/.claude/projects/**/*.jsonl
        </code>{" "}
        session files, multiplied by Anthropic's published API list
        pricing per model.
      </p>

      <div className="rounded-md bg-black/20 border border-white/5 p-2 space-y-1">
        <div className="grid grid-cols-[1fr_auto_auto_auto] gap-x-3 gap-y-0.5 text-[10px] text-foreground/40">
          <span className="font-semibold text-foreground/55">Model</span>
          <span className="text-right font-semibold text-foreground/55">
            Input
          </span>
          <span className="text-right font-semibold text-foreground/55">
            Output
          </span>
          <span className="text-right font-semibold text-foreground/55">
            Cache
          </span>

          <span className="text-foreground/60">Opus</span>
          <span className="text-right tabular-nums">$15</span>
          <span className="text-right tabular-nums">$75</span>
          <span className="text-right tabular-nums">$1.50</span>

          <span className="text-foreground/60">Sonnet</span>
          <span className="text-right tabular-nums">$3</span>
          <span className="text-right tabular-nums">$15</span>
          <span className="text-right tabular-nums">$0.30</span>

          <span className="text-foreground/60">Haiku</span>
          <span className="text-right tabular-nums">$1</span>
          <span className="text-right tabular-nums">$5</span>
          <span className="text-right tabular-nums">$0.10</span>
        </div>
        <div className="text-[9px] text-foreground/30 pt-1 border-t border-white/5">
          USD per 1,000,000 tokens · cache = read price
        </div>
      </div>

      {mode === "subscription" ? (
        <div className="rounded-md bg-emerald-500/8 border border-emerald-500/20 p-2 space-y-1">
          <div className="text-[10px] font-semibold text-emerald-300/90 uppercase tracking-wider">
            Subscription mode
          </div>
          <p className="text-[11px] text-foreground/65 leading-relaxed">
            You're on a flat-fee Claude plan (Pro / Max), so this number
            is <span className="text-foreground/85">not money you owe</span>{" "}
            — it's the API equivalent, i.e. how much the same prompts
            would have cost on pay-per-token billing. Useful for spotting
            which sessions are unusually expensive and how much value
            you're extracting from your subscription.
          </p>
        </div>
      ) : (
        <div className="rounded-md bg-amber-500/8 border border-amber-500/20 p-2 space-y-1">
          <div className="text-[10px] font-semibold text-amber-300/90 uppercase tracking-wider">
            API mode
          </div>
          <p className="text-[11px] text-foreground/65 leading-relaxed">
            You're billed per token by Anthropic. The cost shown is what
            you would expect to see on your invoice for the day, with the
            usual caveats: list pricing can change, volume discounts and
            credits aren't applied, and prompt-caching savings are
            estimated using the public cache-read rate.
          </p>
        </div>
      )}

      <p className="text-[10px] text-foreground/35 leading-relaxed">
        Pricing last reviewed for Sonnet 4.5 / Opus 4 / Haiku 4. If
        Anthropic publishes new rates, update{" "}
        <code className="px-1 rounded bg-white/5">model_pricing()</code>{" "}
        in <code className="px-1 rounded bg-white/5">stats_reader.rs</code>.
      </p>
    </div>
  );
}
