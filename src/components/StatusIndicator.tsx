interface StatusIndicatorProps {
  status: "connected" | "warning" | "error" | "inactive";
  label: string;
  detail?: string;
}

const colorMap = {
  connected: "bg-status-connected",
  warning: "bg-status-warning",
  error: "bg-status-error",
  inactive: "bg-status-inactive",
};

function StatusIndicator({ status, label, detail }: StatusIndicatorProps) {
  return (
    <div className="flex items-center gap-3 py-2">
      <span className={`w-3 h-3 rounded-full ${colorMap[status]} shrink-0`} />
      <div className="min-w-0">
        <p className="text-sm font-medium text-neutral-200">{label}</p>
        {detail && (
          <p className="text-xs text-neutral-500 truncate">{detail}</p>
        )}
      </div>
    </div>
  );
}

export default StatusIndicator;
