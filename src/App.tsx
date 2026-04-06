import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { PopupShell } from "./components/popup/PopupShell";
import { DashboardShell } from "./components/dashboard/DashboardShell";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { onShowNotification, onTokenAlert } from "./lib/events";
import { formatTokens } from "./lib/format";

export default function App() {
  const [windowLabel, setWindowLabel] = useState<string>("");

  useEffect(() => {
    // Detect which window we're in
    const label = getCurrentWindow().label;
    setWindowLabel(label);
  }, []);

  // Handle notifications from Rust backend
  useEffect(() => {
    const setupNotifications = async () => {
      let permitted = await isPermissionGranted();
      if (!permitted) {
        const permission = await requestPermission();
        permitted = permission === "granted";
      }

      if (permitted) {
        const unlistenNotif = onShowNotification(({ title, body }) => {
          sendNotification({ title, body });
        });

        const unlistenAlert = onTokenAlert((tokens) => {
          sendNotification({
            title: "Daily Token Alert",
            body: `You've used ${formatTokens(tokens)} tokens today.`,
          });
        });

        return () => {
          unlistenNotif.then((fn) => fn());
          unlistenAlert.then((fn) => fn());
        };
      }
    };

    setupNotifications();
  }, []);

  if (!windowLabel) {
    return null; // Wait for label detection
  }

  if (windowLabel === "popup") {
    return <PopupShell />;
  }

  if (windowLabel === "dashboard") {
    return <DashboardShell />;
  }

  // Fallback
  return <PopupShell />;
}
