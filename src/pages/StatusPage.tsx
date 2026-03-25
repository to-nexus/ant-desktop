import { useAppState } from "../hooks/useAppState";
import StatusIndicator from "../components/StatusIndicator";
import ConnectionCard from "../components/ConnectionCard";

function connectionStatusToIndicator(
  status: string,
  serverUrl: string | null,
) {
  const serverName = serverUrl
    ? serverUrl.includes("127.0.0.1") || serverUrl.includes("localhost")
      ? "Local Server"
      : "Ant Cloud"
    : "Server";
  switch (status) {
    case "connected":
      return { status: "connected" as const, label: `${serverName} Connected` };
    case "connecting":
      return { status: "warning" as const, label: `${serverName}: Connecting...` };
    case "reconnecting":
      return { status: "warning" as const, label: `${serverName}: Reconnecting...` };
    case "authRequired":
      return {
        status: "inactive" as const,
        label: "Authentication Required",
      };
    case "disconnected":
      return { status: "error" as const, label: "Disconnected" };
    default:
      return { status: "inactive" as const, label: "Not Connected" };
  }
}

function figmaStatusToIndicator(status: string) {
  switch (status) {
    case "available":
      return { status: "connected" as const, label: "Figma Desktop Detected" };
    case "unavailable":
      return {
        status: "warning" as const,
        label: "Figma Desktop Not Running",
      };
    default:
      return { status: "inactive" as const, label: "Figma Status Unknown" };
  }
}

function formatDuration(ms: number | null): string {
  if (ms === null) return "—";
  if (ms < 1000) return "just now";
  const secs = Math.floor(ms / 1000);
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.floor(secs / 60);
  return `${mins}m ago`;
}

function StatusPage() {
  const { state, error } = useAppState();

  if (error) {
    return (
      <div className="text-red-400 text-sm p-4">
        Failed to load state: {error}
      </div>
    );
  }

  if (!state) {
    return <div className="text-neutral-500 text-sm p-4">Loading...</div>;
  }

  const conn = connectionStatusToIndicator(state.connectionStatus, state.serverUrl);
  const figma = figmaStatusToIndicator(state.figmaStatus);

  return (
    <div className="space-y-4">
      <ConnectionCard title="Connection">
        <StatusIndicator
          status={conn.status}
          label={conn.label}
          detail={state.serverUrl ?? "No server configured"}
        />
      </ConnectionCard>

      <ConnectionCard title="Figma Desktop">
        <StatusIndicator
          status={figma.status}
          label={figma.label}
          detail="127.0.0.1:3845 (Figma MCP)"
        />
      </ConnectionCard>

      <ConnectionCard title="Statistics">
        <div className="grid grid-cols-2 gap-y-2 text-sm">
          <div className="text-neutral-500">MCP Requests</div>
          <div className="text-neutral-200 text-right">
            {state.mcpRequestCount}
          </div>
          <div className="text-neutral-500">Last Heartbeat</div>
          <div className="text-neutral-200 text-right">
            {formatDuration(state.lastHeartbeatAgoMs)}
          </div>
          <div className="text-neutral-500">Last MCP Request</div>
          <div className="text-neutral-200 text-right">
            {formatDuration(state.lastMcpRequestAgoMs)}
          </div>
          <div className="text-neutral-500">Machine ID</div>
          <div className="text-neutral-200 text-right font-mono text-xs truncate">
            {state.machineId.slice(0, 8)}...
          </div>
        </div>
      </ConnectionCard>
    </div>
  );
}

export default StatusPage;
