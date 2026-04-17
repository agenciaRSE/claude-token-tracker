import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SubscriptionUsage } from "../types/subscription";
import { onSubscriptionUpdated } from "../lib/events";

export function useSubscriptionUsage() {
  const [usage, setUsage] = useState<SubscriptionUsage | null>(null);

  useEffect(() => {
    invoke<SubscriptionUsage>("get_subscription_usage")
      .then((u) => setUsage(u))
      .catch(() => {});

    const unlisten = onSubscriptionUpdated((u) => setUsage(u));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return usage;
}
