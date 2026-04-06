import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PeakLevel } from "../types/peak";
import { onPeakLevelChanged } from "../lib/events";

export function usePeakLevel() {
  const [peakLevel, setPeakLevel] = useState<PeakLevel | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Get initial state
    invoke<PeakLevel>("get_peak_level")
      .then((level) => {
        setPeakLevel(level);
        setIsLoading(false);
      })
      .catch(() => setIsLoading(false));

    // Subscribe to updates
    const unlisten = onPeakLevelChanged((level) => {
      setPeakLevel(level);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return { peakLevel, isLoading };
}
