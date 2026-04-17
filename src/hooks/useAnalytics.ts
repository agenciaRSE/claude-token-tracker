import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectAnalytics } from "../types/analytics";
import { onAnalyticsUpdated } from "../lib/events";

export function useAnalytics() {
  const [analytics, setAnalytics] = useState<ProjectAnalytics | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    invoke<ProjectAnalytics>("get_project_analytics")
      .then((a) => {
        setAnalytics(a);
        setIsLoading(false);
      })
      .catch(() => setIsLoading(false));

    const unlisten = onAnalyticsUpdated((a) => {
      setAnalytics(a);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return { analytics, isLoading };
}
