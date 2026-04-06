import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ServiceStatus } from "../../types/peak";
import { onServiceStatusUpdated } from "../../lib/events";
import { formatServiceStatus } from "../../lib/format";

export function ServiceStatusRow() {
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
      <div className="flex items-center gap-2 px-1">
        <span className="text-[10px] text-foreground/30">
          Service status loading...
        </span>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-3 px-1 flex-wrap">
      {status.components.map((comp) => {
        const { label, color } = formatServiceStatus(comp.status);
        // Shorten long component names
        const shortName = comp.name
          .replace("Claude ", "")
          .replace("platform.claude.com", "Platform")
          .replace("claude.ai", "Web");

        return (
          <div key={comp.name} className="flex items-center gap-1">
            <div
              className="w-1.5 h-1.5 rounded-full"
              style={{ background: color }}
            />
            <span className="text-[10px] text-foreground/50">{shortName}:</span>
            <span
              className="text-[10px] font-medium"
              style={{ color }}
            >
              {label}
            </span>
          </div>
        );
      })}
    </div>
  );
}
