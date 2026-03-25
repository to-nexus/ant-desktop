import { useEffect, useState, useCallback } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { type AppStateSnapshot, getAppState } from "../lib/tauri";

const isTauri = typeof window !== "undefined" && !!(window as any).__TAURI_INTERNALS__;

export function useAppState() {
  const [state, setState] = useState<AppStateSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!isTauri) return;
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

    const unlisteners: Promise<UnlistenFn>[] = [];

    if (isTauri) {
      unlisteners.push(listen("connection-status-changed", () => refresh()));
      unlisteners.push(listen("figma-status-changed", () => refresh()));
      unlisteners.push(listen("mcp-request-processed", () => refresh()));
    }

    const interval = setInterval(refresh, 5000);

    return () => {
      clearInterval(interval);
      unlisteners.forEach((p) => p.then((fn) => fn()).catch(() => {}));
    };
  }, [refresh]);

  return { state, error, refresh };
}
