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
};

export function useSettings() {
  const [settings, setSettings] = useState<UserSettings>(DEFAULT_SETTINGS);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadSettings()
      .then((saved) => {
        if (saved) {
          setSettings(saved);
          // Sync to Rust state
          invoke("save_settings", { settings: saved }).catch(() => {});
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
