import { load, type Store } from "@tauri-apps/plugin-store";
import type { UserSettings } from "../types/peak";

let store: Store | null = null;

async function getStore(): Promise<Store> {
  if (!store) {
    store = await load("peak-monitor.json");
  }
  return store;
}

export async function loadSettings(): Promise<UserSettings | null> {
  const s = await getStore();
  return (await s.get<UserSettings>("settings")) ?? null;
}

export async function saveSettings(settings: UserSettings): Promise<void> {
  const s = await getStore();
  await s.set("settings", settings);
  await s.save();
}
