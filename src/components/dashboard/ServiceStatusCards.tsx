import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ServiceStatus } from "../../types/peak";
import { onServiceStatusUpdated } from "../../lib/events";
import { formatServiceStatus, formatRelativeTime } from "../../lib/format";

export function ServiceStatusCards() {
  const [status, setStatus] = useState<ServiceStatus | null>(null);

  useEffect(() => {
    invoke<ServiceStatus>("get_service_status")
      .then(setStatus)
      .catch(() => {});

    const unlisten = onServiceStatusUpdated(setStatus);
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  if (!status || status.components.length === 0) {
    return (
      <div className="p-3 rounded-lg bg-white/3">
        <div className="text-[10px] text-foreground/40 mb-2">Service Status</div>
        <div className="text-xs text-foreground/30">Loading status...</div>
      </div>
    );
  }

  return (
    <div className="p-3 rounded-lg bg-white/3">
      <div className="flex items-center justify-between mb-2">
        <div className="text-[10px] text-foreground/40">Anthropic Services</div>
        {status.fetchedAt && (
          <div className="text-[9px] text-foreground/20">
            {formatRelativeTime(status.fetchedAt)}
          </div>
        )}
      </div>
      <div className="grid grid-cols-2 gap-2">
        {status.components.map((comp) => {
          const { label, color } = formatServiceStatus(comp.status);
          return (
            <div
              key={comp.name}
              className="flex items-center gap-2 p-2 rounded-md bg-white/3"
            >
              <div
                className="w-2 h-2 rounded-full shrink-0"
                style={{ background: color }}
              />
              <div className="min-w-0">
                <div className="text-[10px] text-foreground/60 truncate">
                  {comp.name}
                </div>
                <div
                  className="text-[10px] font-medium"
                  style={{ color }}
                >
                  {label}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
