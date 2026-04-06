import { useState, useEffect } from "react";
import { TitleBar } from "./TitleBar";
import { OverviewTab } from "./OverviewTab";
import { HistoryTab } from "./HistoryTab";
import { SettingsPanel } from "../settings/SettingsPanel";
import { onNavigateSettings } from "../../lib/events";

export function DashboardShell() {
  const [activeTab, setActiveTab] = useState("overview");

  // Listen for navigate-settings events from tray menu
  useEffect(() => {
    const unlisten = onNavigateSettings(() => {
      setActiveTab("settings");
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <div
      className="h-full flex flex-col overflow-hidden"
      style={{
        background: "oklch(0.13 0 0 / 97%)",
        borderRadius: 12,
        border: "1px solid oklch(1 0 0 / 8%)",
      }}
    >
      <TitleBar activeTab={activeTab} onTabChange={setActiveTab} />

      <div className="flex-1 overflow-y-auto p-4">
        {activeTab === "overview" && <OverviewTab />}
        {activeTab === "history" && <HistoryTab />}
        {activeTab === "settings" && <SettingsPanel />}
      </div>
    </div>
  );
}
