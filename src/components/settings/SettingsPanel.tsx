import { useSettings } from "../../hooks/useSettings";
import { isEnabled, enable, disable } from "@tauri-apps/plugin-autostart";
import { useEffect, useState } from "react";

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
          description="Launch Claude Peak Monitor when you log in"
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

      {/* Notifications */}
      <Section title="Notifications">
        <ToggleRow
          label="Enable notifications"
          description="Show native OS notifications"
          checked={settings.notificationsEnabled}
          onChange={(v) => updateSetting("notificationsEnabled", v)}
        />
        <ToggleRow
          label="Notify on color change"
          description="Alert when peak level changes color"
          checked={settings.notifyOnColorChange}
          onChange={(v) => updateSetting("notifyOnColorChange", v)}
          disabled={!settings.notificationsEnabled}
        />
        <NumberRow
          label="Daily token alert (0 = disabled)"
          value={settings.dailyTokenAlert ?? 0}
          min={0}
          max={10000000}
          step={10000}
          onChange={(v) => updateSetting("dailyTokenAlert", v === 0 ? null : v)}
        />
      </Section>

      {/* About */}
      <Section title="About">
        <div className="text-xs text-foreground/40 space-y-1">
          <p>Claude Peak Monitor v0.1.0</p>
          <p>
            Monitors Claude AI peak usage hours using time patterns,
            Anthropic service status, and your local Claude Code statistics.
          </p>
          <p className="text-foreground/25">
            Data source: ~/.claude/stats-cache.json
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
        onChange={(e) => onChange(Number(e.target.value))}
        className="text-xs bg-white/5 border border-white/10 rounded-md px-2 py-1 text-foreground/70 outline-none focus:border-white/20 w-24 text-right"
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
