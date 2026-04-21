import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { UserSettings } from "../types/peak";
import { loadSettings, saveSettings as persistSettings } from "../lib/storage";

const DEFAULT_SETTINGS: UserSettings = {
  timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
  notificationsEnabled: true,
  notifyOnColorChange: true,
  dailyTokenAlert: null,
  refreshIntervalSecs: 120,
  autostart: true,
  costMode: "api",
  subscriptionPlan: "pro",
  sessionTokenLimit: 0,
  weeklyTokenLimit: 0,
  sessionCostLimitUsd: 0,
  weeklyResetWeekday: 1, // Monday
  weeklyResetHour: 0,
  sessionSlotAnchorHour: 2,
  subscriptionWarnPct: 80,
  subscriptionWarningsEnabled: true,
  alertSessionStart: true,
  alertSessionEnd: true,
  usageWarningThresholds: [75, 90, 100],
  soundsEnabled: true,
  soundVolume: 70,
  soundPeakChange: "pulse",
  soundSessionStart: "success",
  soundSessionEnd: "chime",
  soundUsageThreshold: "warning",
};

export function useSettings() {
  const [settings, setSettings] = useState<UserSettings>(DEFAULT_SETTINGS);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadSettings()
      .then((saved) => {
        if (saved) {
          // Merge with defaults so upgrading users get any newly added
          // setting (e.g. costMode) populated instead of `undefined`.
          const merged = { ...DEFAULT_SETTINGS, ...saved };
          setSettings(merged);
          // Sync to Rust state
          invoke("save_settings", { settings: merged }).catch(() => {});
        }
        setIsLoading(false);
      })
      .catch(() => setIsLoading(false));
  }, []);

  const saveSettings = useCallback(
    async (newSettings: UserSettings) => {
      setSettings(newSettings);
      await persistSettings(newSettings);
      await invoke("save_settings", { settings: newSettings });
    },
    [],
  );

  const updateSetting = useCallback(
    async <K extends keyof UserSettings>(key: K, value: UserSettings[K]) => {
      const updated = { ...settings, [key]: value };
      await saveSettings(updated);
    },
    [settings, saveSettings],
  );

  return { settings, isLoading, saveSettings, updateSetting };
}
