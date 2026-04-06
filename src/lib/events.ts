import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PeakLevel, ServiceStatus } from "../types/peak";
import type { ClaudeStats } from "../types/stats";

export function onPeakLevelChanged(
  callback: (level: PeakLevel) => void,
): Promise<UnlistenFn> {
  return listen<PeakLevel>("peak-level-changed", (event) => {
    callback(event.payload);
  });
}

export function onStatsUpdated(
  callback: (stats: ClaudeStats) => void,
): Promise<UnlistenFn> {
  return listen<ClaudeStats>("stats-updated", (event) => {
    callback(event.payload);
  });
}

export function onServiceStatusUpdated(
  callback: (status: ServiceStatus) => void,
): Promise<UnlistenFn> {
  return listen<ServiceStatus>("service-status-updated", (event) => {
    callback(event.payload);
  });
}

export function onNavigateSettings(
  callback: () => void,
): Promise<UnlistenFn> {
  return listen("navigate-settings", () => {
    callback();
  });
}

export function onForceRefresh(
  callback: () => void,
): Promise<UnlistenFn> {
  return listen("force-refresh", () => {
    callback();
  });
}

export function onShowNotification(
  callback: (data: { title: string; body: string }) => void,
): Promise<UnlistenFn> {
  return listen<{ title: string; body: string }>("show-notification", (event) => {
    callback(event.payload);
  });
}

export function onTokenAlert(
  callback: (tokens: number) => void,
): Promise<UnlistenFn> {
  return listen<number>("token-alert", (event) => {
    callback(event.payload);
  });
}
