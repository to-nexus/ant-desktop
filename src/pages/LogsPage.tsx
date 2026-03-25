import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import ConnectionCard from "../components/ConnectionCard";

interface LogEntry {
  id: number;
  timestamp: string;
  type: string;
  message: string;
}

let nextId = 1;

function LogsPage() {
  const [logs, setLogs] = useState<LogEntry[]>([]);

  const addLog = useCallback((type: string, message: string) => {
    const entry: LogEntry = {
      id: nextId++,
      timestamp: new Date().toLocaleTimeString(),
      type,
      message,
    };
    setLogs((prev) => [entry, ...prev].slice(0, 100));
  }, []);

  useEffect(() => {
    const isTauri = !!(window as any).__TAURI_INTERNALS__;
    if (!isTauri) return;

    const unlisteners = [
      listen("connection-status-changed", (event) => {
        addLog("connection", `Status: ${JSON.stringify(event.payload)}`);
      }),
      listen("figma-status-changed", (event) => {
        addLog("figma", `Status: ${JSON.stringify(event.payload)}`);
      }),
      listen("mcp-request-processed", (event) => {
        const payload = event.payload as Record<string, unknown>;
        const tool = payload.tool ?? "unknown";
        const durationMs = payload.duration_ms ?? "?";
        addLog("mcp", `Request: ${tool} (${durationMs}ms)`);
      }),
      listen("auth-received", (event) => {
        const payload = event.payload as Record<string, unknown>;
        addLog("auth", `Deep link: ${payload.server}`);
      }),
      listen("error", (event) => {
        const payload = event.payload as Record<string, unknown>;
        addLog("error", String(payload.message));
      }),
    ];

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()).catch(() => {}));
    };
  }, [addLog]);

  return (
    <div className="space-y-4">
      <ConnectionCard title={`Event Log (${logs.length})`}>
        {logs.length === 0 ? (
          <p className="text-sm text-neutral-500">No events recorded yet.</p>
        ) : (
          <div className="space-y-1 max-h-[400px] overflow-y-auto">
            {logs.map((log) => (
              <div
                key={log.id}
                className="flex gap-2 text-xs py-1 border-b border-neutral-800 last:border-0"
              >
                <span className="text-neutral-600 shrink-0 w-16">
                  {log.timestamp}
                </span>
                <span
                  className={`shrink-0 w-20 ${
                    log.type === "error"
                      ? "text-red-400"
                      : log.type === "mcp"
                        ? "text-blue-400"
                        : "text-neutral-400"
                  }`}
                >
                  {log.type}
                </span>
                <span className="text-neutral-300 truncate">{log.message}</span>
              </div>
            ))}
          </div>
        )}
      </ConnectionCard>
    </div>
  );
}

export default LogsPage;
