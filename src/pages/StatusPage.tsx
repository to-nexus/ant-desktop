import { useAppState } from "../hooks/useAppState";
import StatusIndicator from "../components/StatusIndicator";
import ConnectionCard from "../components/ConnectionCard";
import AntIcon from "../components/icons/AntIcon";
import FigmaIcon from "../components/icons/FigmaIcon";

function connectionStatusToIndicator(status: string) {
  switch (status) {
    case "connected":
      return { status: "connected" as const, label: "Connected" };
    case "connecting":
      return { status: "warning" as const, label: "Connecting..." };
    case "reconnecting":
      return { status: "warning" as const, label: "Reconnecting..." };
    case "authRequired":
      return { status: "inactive" as const, label: "Authentication Required" };
    case "disconnected":
      return { status: "error" as const, label: "Disconnected" };
    default:
      return { status: "inactive" as const, label: "Not Connected" };
  }
}

function figmaStatusToIndicator(status: string) {
  switch (status) {
    case "available":
      return { status: "connected" as const, label: "Connected" };
    case "unavailable":
      return { status: "error" as const, label: "Not Connected" };
    default:
      return { status: "inactive" as const, label: "Unknown" };
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

  const conn = connectionStatusToIndicator(state.connectionStatus);
  const figma = figmaStatusToIndicator(state.figmaStatus);

  return (
    <div className="space-y-4">
      <ConnectionCard title="Ant Server">
        <StatusIndicator
          icon={<AntIcon size={24} />}
          status={conn.status}
          label={conn.label}
          detail={state.serverUrl ?? "No server configured"}
        />
      </ConnectionCard>

      <ConnectionCard title="Figma Desktop">
        <StatusIndicator
          icon={<FigmaIcon size={24} />}
          status={figma.status}
          label={figma.label}
          detail="localhost:3845"
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
