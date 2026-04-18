import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { PopupShell } from "./components/popup/PopupShell";
import { DashboardShell } from "./components/dashboard/DashboardShell";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import {
  onShowNotification,
  onTokenAlert,
  onSubscriptionWarning,
  onPlaySound,
} from "./lib/events";
import { formatTokens, formatDuration } from "./lib/format";
import { playSound, isValidSoundId } from "./lib/sounds";

// Detect window label synchronously at module load time so the first render
// already knows which shell to mount (avoids blank window flashes).
function detectWindowLabel(): string {
  try {
    return getCurrentWindow().label || "popup";
  } catch (err) {
    console.error("Failed to detect window label:", err);
    return "popup";
  }
}

export default function App() {
  const [windowLabel] = useState<string>(detectWindowLabel);

  // Handle notifications from Rust backend
  useEffect(() => {
    let cleanup: (() => void) | undefined;

    const setupNotifications = async () => {
      try {
        let permitted = await isPermissionGranted();
        if (!permitted) {
          const permission = await requestPermission();
          permitted = permission === "granted";
        }

        if (!permitted) return;

        const unlistenNotif = await onShowNotification(({ title, body }) => {
          sendNotification({ title, body });
        });

        const unlistenAlert = await onTokenAlert((tokens) => {
          sendNotification({
            title: "Daily Token Alert",
            body: `You've used ${formatTokens(tokens)} tokens today.`,
          });
        });

        const unlistenSub = await onSubscriptionWarning(({ scope, pct, secondsToReset }) => {
          const scopeLabel = scope === "session" ? "5-hour session" : "weekly";
          sendNotification({
            title: `Claude ${scopeLabel} usage at ${pct}%`,
            body: `Resets in ${formatDuration(secondsToReset)}. Consider deferring heavy tasks.`,
          });
        });

        // Sound alerts: only registered in the popup window so that we
        // don't play each tone twice when the dashboard is also open.
        // Both windows listen for OS notifications (they're deduplicated
        // by the system); sound is single-owner.
        let unlistenSound: (() => void) | undefined;
        if (windowLabel === "popup") {
          unlistenSound = await onPlaySound(({ soundId, volume }) => {
            if (isValidSoundId(soundId)) {
              playSound(soundId, volume);
            }
          });
        }

        cleanup = () => {
          unlistenNotif();
          unlistenAlert();
          unlistenSub();
          unlistenSound?.();
        };
      } catch (err) {
        console.error("Failed to setup notifications:", err);
      }
    };

    setupNotifications();

    return () => {
      cleanup?.();
    };
  }, [windowLabel]);

  if (windowLabel === "dashboard") {
    return <DashboardShell />;
  }

  return <PopupShell />;
}
