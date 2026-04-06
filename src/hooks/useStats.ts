import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ClaudeStats } from "../types/stats";
import { onStatsUpdated } from "../lib/events";

export function useStats() {
  const [stats, setStats] = useState<ClaudeStats | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Get initial state
    invoke<ClaudeStats>("get_stats")
      .then((s) => {
        setStats(s);
        setIsLoading(false);
      })
      .catch(() => setIsLoading(false));

    // Subscribe to updates
    const unlisten = onStatsUpdated((s) => {
      setStats(s);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return { stats, isLoading };
}
