import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { type AppStateSnapshot, getAppState } from "../lib/tauri";

export function useAppState() {
  const [state, setState] = useState<AppStateSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const snapshot = await getAppState();
      setState(snapshot);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();

    const unlistenConnection = listen("connection-status-changed", () => {
      refresh();
    });
    const unlistenFigma = listen("figma-status-changed", () => {
      refresh();
    });
    const unlistenMcp = listen("mcp-request-processed", () => {
      refresh();
    });

    const interval = setInterval(refresh, 5000);

    return () => {
      clearInterval(interval);
      unlistenConnection.then((fn) => fn());
      unlistenFigma.then((fn) => fn());
      unlistenMcp.then((fn) => fn());
    };
  }, [refresh]);

  return { state, error, refresh };
}
