import { useSettings } from "../../hooks/useSettings";
import { isEnabled, enable, disable } from "@tauri-apps/plugin-autostart";
import { useEffect, useState } from "react";
import type { CostMode, SubscriptionPlan } from "../../types/peak";
import { WEEKDAY_LABELS } from "../../types/subscription";
import { CostLegend } from "./CostLegend";
import {
  playSound,
  SOUND_IDS,
  SOUND_LABELS,
  type SoundId,
  isValidSoundId,
} from "../../lib/sounds";

export function SettingsPanel() {
  const { settings, isLoading, updateSetting } = useSettings();
  const [autostartEnabled, setAutostartEnabled] = useState(false);

  useEffect(() => {
    isEnabled().then(setAutostartEnabled).catch(() => {});
  }, []);

  const toggleAutostart = async () => {
    try {
      if (autostartEnabled) {
        await disable();
        setAutostartEnabled(false);
      } else {
        await enable();
        setAutostartEnabled(true);
      }
      await updateSetting("autostart", !autostartEnabled);
    } catch (e) {
      console.error("Autostart toggle failed:", e);
    }
  };

  if (isLoading) {
    return <div className="text-foreground/30 text-sm">Loading settings...</div>;
  }

  return (
    <div className="flex flex-col gap-4">
      {/* General */}
      <Section title="General">
        <ToggleRow
          label="Start on system boot"
          description="Launch Claude Consume and Peak Monitor when you log in"
          checked={autostartEnabled}
          onChange={toggleAutostart}
        />
        <SelectRow
          label="Timezone"
          value={settings.timezone}
          options={TIMEZONES}
          onChange={(v) => updateSetting("timezone", v)}
        />
        <NumberRow
          label="Refresh interval (seconds)"
          value={settings.refreshIntervalSecs}
          min={30}
          max={600}
          step={30}
          onChange={(v) => updateSetting("refreshIntervalSecs", v)}
        />
      </Section>

      {/* Billing & cost */}
      <Section title="Billing & cost">
        <SegmentedRow<CostMode>
          label="Billing mode"
          description="How you pay Anthropic for Claude"
          value={settings.costMode}
          options={[
            { value: "api", label: "API" },
            { value: "subscription", label: "Subscription" },
          ]}
          onChange={(v) => updateSetting("costMode", v)}
        />
        <CostLegend mode={settings.costMode} />
      </Section>

      {/* Subscription limits — only relevant when on a subscription plan */}
      {settings.costMode === "subscription" && (
        <Section title="Subscription limits">
          <SegmentedRow<SubscriptionPlan>
            label="Plan"
            description="Used to pick default session + weekly token budgets"
            value={settings.subscriptionPlan}
            options={[
              { value: "pro", label: "Pro" },
              { value: "max5x", label: "Max 5×" },
              { value: "max20x", label: "Max 20×" },
              { value: "custom", label: "Custom" },
            ]}
            onChange={(v) => updateSetting("subscriptionPlan", v)}
          />
          <NumberRow
            label="Session cost limit USD (0 = plan default)"
            value={Math.round(settings.sessionCostLimitUsd)}
            min={0}
            max={10000}
            step={10}
            onChange={(v) => updateSetting("sessionCostLimitUsd", v)}
          />
          <NumberRow
            label="Session token limit (display only, 0 = plan default)"
            value={settings.sessionTokenLimit}
            min={0}
            max={10_000_000_000}
            step={1_000_000}
            onChange={(v) => updateSetting("sessionTokenLimit", v)}
          />
          <NumberRow
            label="Weekly token limit (0 = plan default)"
            value={settings.weeklyTokenLimit}
            min={0}
            max={10_000_000_000}
            step={10_000_000}
            onChange={(v) => updateSetting("weeklyTokenLimit", v)}
          />
          <SelectRow
            label="Weekly reset day"
            value={WEEKDAY_LABELS[settings.weeklyResetWeekday] ?? "Monday"}
            options={[...WEEKDAY_LABELS]}
            onChange={(v) =>
              updateSetting("weeklyResetWeekday", WEEKDAY_LABELS.indexOf(v as (typeof WEEKDAY_LABELS)[number]))
            }
          />
          <NumberRow
            label="Weekly reset hour (UTC)"
            value={settings.weeklyResetHour}
            min={0}
            max={23}
            step={1}
            onChange={(v) => updateSetting("weeklyResetHour", v)}
          />
          <NumberRow
            label="Warning threshold (%)"
            value={settings.subscriptionWarnPct}
            min={10}
            max={100}
            step={5}
            onChange={(v) => updateSetting("subscriptionWarnPct", v)}
          />
          <ToggleRow
            label="Enable subscription warnings"
            description="Notify once per session/week when the threshold is crossed"
            checked={settings.subscriptionWarningsEnabled}
            onChange={(v) => updateSetting("subscriptionWarningsEnabled", v)}
          />
          <div className="text-[10px] text-foreground/30 leading-relaxed">
            The <strong>session bar is driven by cost</strong> (USD) rather
            than tokens — Claude's own session meter appears cost-based, and
            token counts drift wildly with cache_creation bursts. The
            <strong> weekly bar uses tokens</strong> (excluding cache_read);
            the two metrics coincide well enough over a 7-day window.
            <br />
            <span className="text-foreground/50">
              To calibrate session:
            </span>{" "}
            note the cost shown in the popup's SESSION bar and the % Claude
            Desktop shows for the same session, then set Session cost limit =
            popup_cost ÷ (claude_% / 100).
            <br />
            Example: popup shows $114 at our 422%, Claude shows 32% →
            effective limit = $114 ÷ 0.32 ≈&nbsp;$357.
            <br />
            <span className="text-foreground/50">To calibrate weekly:</span>{" "}
            popup_tokens ÷ (claude_% / 100). Example: 27.2M ÷ 0.25 =&nbsp;108M.
          </div>
        </Section>
      )}

      {/* Notifications */}
      <Section title="Notifications">
        <ToggleRow
          label="Enable notifications"
          description="Master toggle for native OS notifications"
          checked={settings.notificationsEnabled}
          onChange={(v) => updateSetting("notificationsEnabled", v)}
        />
        <ToggleRow
          label="Peak hour changes"
          description="Alert when the peak level color changes (green → yellow → orange → red)"
          checked={settings.notifyOnColorChange}
          onChange={(v) => updateSetting("notifyOnColorChange", v)}
          disabled={!settings.notificationsEnabled}
        />
        <ToggleRow
          label="New 5-hour session started"
          description="Alert when a fresh subscription session begins after a 5h idle gap"
          checked={settings.alertSessionStart}
          onChange={(v) => updateSetting("alertSessionStart", v)}
          disabled={
            !settings.notificationsEnabled || settings.costMode !== "subscription"
          }
        />
        <ToggleRow
          label="5-hour session ended"
          description="Alert when the current subscription session expires"
          checked={settings.alertSessionEnd}
          onChange={(v) => updateSetting("alertSessionEnd", v)}
          disabled={
            !settings.notificationsEnabled || settings.costMode !== "subscription"
          }
        />
        <ToggleRow
          label="Usage threshold warnings"
          description="Fire a warning at each configured % of the plan limit"
          checked={settings.subscriptionWarningsEnabled}
          onChange={(v) => updateSetting("subscriptionWarningsEnabled", v)}
          disabled={
            !settings.notificationsEnabled || settings.costMode !== "subscription"
          }
        />
        {settings.costMode === "subscription" && (
          <ThresholdListRow
            label="Warning thresholds (%)"
            description="Each threshold fires once per window — session or weekly"
            value={settings.usageWarningThresholds}
            onChange={(v) => updateSetting("usageWarningThresholds", v)}
            disabled={
              !settings.notificationsEnabled ||
              !settings.subscriptionWarningsEnabled
            }
          />
        )}
        <NumberRow
          label="Daily token alert (0 = disabled)"
          value={settings.dailyTokenAlert ?? 0}
          min={0}
          max={10000000}
          step={10000}
          onChange={(v) => updateSetting("dailyTokenAlert", v === 0 ? null : v)}
        />
      </Section>

      {/* Sound alerts — paired with each notification category */}
      <Section title="Sound alerts">
        <ToggleRow
          label="Enable sounds"
          description="Play a short tone when alerts fire. Sounds are generated procedurally — no audio files shipped."
          checked={settings.soundsEnabled}
          onChange={(v) => updateSetting("soundsEnabled", v)}
        />
        <NumberRow
          label="Volume (0-100)"
          value={settings.soundVolume}
          min={0}
          max={100}
          step={10}
          onChange={(v) => updateSetting("soundVolume", v)}
        />
        <SoundPickerRow
          label="Peak color change"
          value={settings.soundPeakChange}
          volume={settings.soundVolume}
          onChange={(v) => updateSetting("soundPeakChange", v)}
          disabled={!settings.soundsEnabled}
        />
        <SoundPickerRow
          label="Session started"
          value={settings.soundSessionStart}
          volume={settings.soundVolume}
          onChange={(v) => updateSetting("soundSessionStart", v)}
          disabled={!settings.soundsEnabled}
        />
        <SoundPickerRow
          label="Session ended"
          value={settings.soundSessionEnd}
          volume={settings.soundVolume}
          onChange={(v) => updateSetting("soundSessionEnd", v)}
          disabled={!settings.soundsEnabled}
        />
        <SoundPickerRow
          label="Usage threshold crossed"
          value={settings.soundUsageThreshold}
          volume={settings.soundVolume}
          onChange={(v) => updateSetting("soundUsageThreshold", v)}
          disabled={!settings.soundsEnabled}
        />
        <div className="text-[10px] text-foreground/30 leading-relaxed">
          Tip: click the ▶ button next to any sound to preview. Browsers gate
          audio behind a user gesture, so the first real alert may be silent
          until you preview once.
        </div>
      </Section>

      {/* About */}
      <Section title="About">
        <div className="text-xs text-foreground/40 space-y-1">
          <p>Claude Consume and Peak Monitor v0.2.1</p>
          <p>
            Monitors Claude AI peak usage hours using time patterns,
            Anthropic service status, and your local Claude Code statistics.
          </p>
          <p className="text-foreground/25">
            Data source: ~/.claude/projects/**/*.jsonl
          </p>
        </div>
      </Section>
    </div>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-xl bg-white/3 p-4">
      <h3 className="text-xs font-semibold text-foreground/60 mb-3">{title}</h3>
      <div className="space-y-3">{children}</div>
    </div>
  );
}

function ToggleRow({
  label,
  description,
  checked,
  onChange,
  disabled,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (value: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <div
      className={`flex items-center justify-between ${disabled ? "opacity-40" : ""}`}
    >
      <div>
        <div className="text-xs text-foreground/70">{label}</div>
        {description && (
          <div className="text-[10px] text-foreground/30 mt-0.5">
            {description}
          </div>
        )}
      </div>
      <button
        onClick={() => !disabled && onChange(!checked)}
        disabled={disabled}
        className={`w-9 h-5 rounded-full transition-colors relative ${
          checked ? "bg-green-500/70" : "bg-white/10"
        }`}
      >
        <div
          className={`absolute top-0.5 w-4 h-4 rounded-full bg-white shadow-sm transition-transform ${
            checked ? "translate-x-4" : "translate-x-0.5"
          }`}
        />
      </button>
    </div>
  );
}

function SelectRow({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: string[];
  onChange: (value: string) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <div className="text-xs text-foreground/70">{label}</div>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="text-xs bg-white/5 border border-white/10 rounded-md px-2 py-1 text-foreground/70 outline-none focus:border-white/20 max-w-[180px]"
      >
        {options.map((opt) => (
          <option key={opt} value={opt}>
            {opt}
          </option>
        ))}
      </select>
    </div>
  );
}

function SegmentedRow<T extends string>({
  label,
  description,
  value,
  options,
  onChange,
}: {
  label: string;
  description?: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <div className="min-w-0">
        <div className="text-xs text-foreground/70">{label}</div>
        {description && (
          <div className="text-[10px] text-foreground/30 mt-0.5">
            {description}
          </div>
        )}
      </div>
      <div className="inline-flex rounded-md bg-white/5 border border-white/10 p-0.5 shrink-0">
        {options.map((opt) => {
          const active = opt.value === value;
          return (
            <button
              key={opt.value}
              type="button"
              onClick={() => onChange(opt.value)}
              className={`text-[11px] px-2.5 py-1 rounded transition-colors ${
                active
                  ? "bg-white/15 text-foreground/90 shadow-sm"
                  : "text-foreground/50 hover:text-foreground/75"
              }`}
            >
              {opt.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}

function NumberRow({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <div className="text-xs text-foreground/70">{label}</div>
      <input
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(e) => {
          const raw = Number(e.target.value);
          if (!Number.isFinite(raw)) return;
          onChange(Math.max(min, Math.min(max, Math.round(raw))));
        }}
        className="text-xs bg-white/5 border border-white/10 rounded-md px-2 py-1 text-foreground/70 outline-none focus:border-white/20 w-24 text-right"
      />
    </div>
  );
}

/** Sound preset picker with a preview button. Disabled when sounds are
 *  globally off; shows current value as a label inside the dropdown. */
function SoundPickerRow({
  label,
  value,
  volume,
  onChange,
  disabled,
}: {
  label: string;
  value: string;
  volume: number;
  onChange: (value: string) => void;
  disabled?: boolean;
}) {
  const currentId: SoundId = isValidSoundId(value) ? value : "chime";

  return (
    <div
      className={`flex items-center justify-between gap-2 ${
        disabled ? "opacity-40" : ""
      }`}
    >
      <div className="text-xs text-foreground/70">{label}</div>
      <div className="flex items-center gap-1.5">
        <select
          value={currentId}
          disabled={disabled}
          onChange={(e) => onChange(e.target.value)}
          className="text-xs bg-white/5 border border-white/10 rounded-md px-2 py-1 text-foreground/70 outline-none focus:border-white/20 max-w-[170px]"
        >
          {SOUND_IDS.map((id) => (
            <option key={id} value={id}>
              {SOUND_LABELS[id]}
            </option>
          ))}
        </select>
        <button
          type="button"
          disabled={disabled || currentId === "none"}
          onClick={() => playSound(currentId, volume)}
          title="Preview"
          className="w-6 h-6 rounded flex items-center justify-center text-foreground/60 hover:text-foreground/90 hover:bg-white/10 transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
            <path d="M2 1 L9 5 L2 9 Z" />
          </svg>
        </button>
      </div>
    </div>
  );
}

/** Editable comma-separated list of integer percentages. Users can type
 *  values directly; we parse, clamp to [1, 200], dedupe, and sort. */
function ThresholdListRow({
  label,
  description,
  value,
  onChange,
  disabled,
}: {
  label: string;
  description?: string;
  value: number[];
  onChange: (value: number[]) => void;
  disabled?: boolean;
}) {
  const [text, setText] = useState(() => value.join(", "));

  // Keep local text in sync if external value changes (e.g. reset to defaults).
  useEffect(() => {
    setText(value.join(", "));
  }, [value]);

  const commit = () => {
    const parsed = text
      .split(/[,\s]+/)
      .map((s) => parseInt(s.trim(), 10))
      .filter((n) => Number.isFinite(n) && n >= 1 && n <= 200);
    const unique = Array.from(new Set(parsed)).sort((a, b) => a - b);
    onChange(unique);
    setText(unique.join(", "));
  };

  return (
    <div
      className={`flex items-start justify-between gap-2 ${
        disabled ? "opacity-40" : ""
      }`}
    >
      <div className="min-w-0 flex-1">
        <div className="text-xs text-foreground/70">{label}</div>
        {description && (
          <div className="text-[10px] text-foreground/30 mt-0.5">
            {description}
          </div>
        )}
      </div>
      <input
        type="text"
        value={text}
        disabled={disabled}
        placeholder="e.g. 75, 90, 100"
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            (e.target as HTMLInputElement).blur();
          }
        }}
        className="text-xs bg-white/5 border border-white/10 rounded-md px-2 py-1 text-foreground/70 outline-none focus:border-white/20 w-32 text-right"
      />
    </div>
  );
}

const TIMEZONES = [
  "UTC",
  "America/New_York",
  "America/Chicago",
  "America/Denver",
  "America/Los_Angeles",
  "America/Bogota",
  "America/Sao_Paulo",
  "America/Argentina/Buenos_Aires",
  "America/Mexico_City",
  "Europe/London",
  "Europe/Paris",
  "Europe/Berlin",
  "Europe/Madrid",
  "Europe/Rome",
  "Europe/Moscow",
  "Asia/Dubai",
  "Asia/Kolkata",
  "Asia/Shanghai",
  "Asia/Tokyo",
  "Asia/Seoul",
  "Australia/Sydney",
  "Pacific/Auckland",
];
