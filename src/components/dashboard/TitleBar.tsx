import { getCurrentWindow } from "@tauri-apps/api/window";

interface Props {
  activeTab: string;
  onTabChange: (tab: string) => void;
}

const TABS = [
  { id: "overview", label: "Overview" },
  { id: "history", label: "History" },
  { id: "settings", label: "Settings" },
];

export function TitleBar({ activeTab, onTabChange }: Props) {
  const appWindow = getCurrentWindow();

  return (
    // data-tauri-drag-region makes this whole bar draggable, but propagates
    // to children — so every interactive child below must opt out with
    // data-tauri-drag-region="false" or its clicks get hijacked by drag.
    <div
      className="shrink-0 flex items-center justify-between px-4 py-2 border-b border-white/5"
      data-tauri-drag-region
    >
      <div
        className="flex items-center gap-4"
        data-tauri-drag-region
      >
        {/* Logo */}
        <div className="flex items-center gap-2" data-tauri-drag-region>
          <svg width="16" height="16" viewBox="0 0 32 32">
            <circle cx="16" cy="16" r="12" fill="#22c55e" opacity="0.8" />
            <circle cx="16" cy="16" r="6" fill="#22c55e" />
          </svg>
          <span
            className="text-xs font-semibold text-foreground/80"
            data-tauri-drag-region
          >
            Peak Monitor
          </span>
        </div>

        {/* Tabs */}
        <div className="flex gap-1">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              data-tauri-drag-region="false"
              onClick={() => onTabChange(tab.id)}
              className={`text-[11px] px-2.5 py-1 rounded-md transition-colors ${
                activeTab === tab.id
                  ? "bg-white/10 text-foreground/90 font-medium"
                  : "text-foreground/40 hover:text-foreground/60 hover:bg-white/5"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Window controls */}
      <div className="flex items-center gap-1">
        <button
          data-tauri-drag-region="false"
          onClick={() => appWindow.minimize()}
          className="w-6 h-6 rounded flex items-center justify-center text-foreground/30 hover:text-foreground/60 hover:bg-white/10 transition-colors"
        >
          <svg width="10" height="2" viewBox="0 0 10 2">
            <rect width="10" height="1.5" rx="0.75" fill="currentColor" />
          </svg>
        </button>
        <button
          data-tauri-drag-region="false"
          onClick={() => appWindow.hide()}
          className="w-6 h-6 rounded flex items-center justify-center text-foreground/30 hover:text-foreground/60 hover:bg-white/10 transition-colors"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path d="M1 1L9 9M9 1L1 9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </button>
      </div>
    </div>
  );
}
