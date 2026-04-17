import { type TimeRange, TIME_RANGE_LABELS } from "../../types/analytics";

interface Props {
  value: TimeRange;
  onChange: (value: TimeRange) => void;
  loading?: boolean;
}

const QUICK_RANGES: TimeRange[] = [
  "today",
  "yesterday",
  "last_7_days",
  "last_30_days",
  "this_month",
  "this_year",
  "all",
];

export function TimeRangeSelector({ value, onChange, loading }: Props) {
  return (
    <div className="flex items-center gap-2 flex-wrap">
      <span className="text-[10px] text-foreground/40 shrink-0">Period:</span>
      <div className="inline-flex rounded-md bg-white/5 border border-white/10 p-0.5 flex-wrap">
        {QUICK_RANGES.map((range) => {
          const active = range === value;
          return (
            <button
              key={range}
              type="button"
              onClick={() => onChange(range)}
              disabled={loading}
              className={`text-[10px] px-2 py-1 rounded transition-colors ${
                active
                  ? "bg-white/15 text-foreground/90 shadow-sm"
                  : "text-foreground/50 hover:text-foreground/75"
              } ${loading ? "opacity-50 cursor-wait" : ""}`}
            >
              {TIME_RANGE_LABELS[range]}
            </button>
          );
        })}
      </div>
      {loading && (
        <span className="text-[9px] text-foreground/30">Loading…</span>
      )}
    </div>
  );
}
