import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export function PopupFooter() {
  const openDashboard = async () => {
    try {
      const dashboard = await WebviewWindow.getByLabel("dashboard");
      if (dashboard) {
        await dashboard.show();
        await dashboard.setFocus();
        await dashboard.center();
      }
    } catch (e) {
      console.error("Failed to open dashboard:", e);
    }
  };

  const openSettings = async () => {
    try {
      const dashboard = await WebviewWindow.getByLabel("dashboard");
      if (dashboard) {
        await dashboard.show();
        await dashboard.setFocus();
        await dashboard.center();
        // Emit navigate to settings tab
        const { emit } = await import("@tauri-apps/api/event");
        await emit("navigate-settings");
      }
    } catch (e) {
      console.error("Failed to open settings:", e);
    }
  };

  const forceRefresh = async () => {
    try {
      await invoke("force_refresh");
    } catch (e) {
      console.error("Failed to refresh:", e);
    }
  };

  return (
    <div className="shrink-0 flex items-center gap-1.5 px-3 py-2.5 border-t border-white/5">
      <button
        onClick={openDashboard}
        className="flex-1 text-[11px] font-medium px-3 py-1.5 rounded-md bg-white/8 hover:bg-white/12 text-foreground/70 hover:text-foreground/90 transition-colors"
      >
        Dashboard
      </button>
      <button
        onClick={forceRefresh}
        className="px-2 py-1.5 rounded-md bg-white/5 hover:bg-white/10 text-foreground/40 hover:text-foreground/70 transition-colors"
        title="Refresh now"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="23 4 23 10 17 10" />
          <polyline points="1 20 1 14 7 14" />
          <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
        </svg>
      </button>
      <button
        onClick={openSettings}
        className="px-2 py-1.5 rounded-md bg-white/5 hover:bg-white/10 text-foreground/40 hover:text-foreground/70 transition-colors"
        title="Settings"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </button>
    </div>
  );
}
