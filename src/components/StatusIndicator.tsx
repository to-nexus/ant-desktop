import { type ReactNode } from "react";

interface StatusIndicatorProps {
  status: "connected" | "warning" | "error" | "inactive";
  label: string;
  detail?: string;
  icon?: ReactNode;
}

const dotColor = {
  connected: "bg-status-connected",
  warning: "bg-status-warning",
  error: "bg-status-error",
  inactive: "bg-status-inactive",
};

function StatusIndicator({ status, label, detail, icon }: StatusIndicatorProps) {
  return (
    <div className="flex items-center gap-3 py-2">
      {icon && <div className="shrink-0">{icon}</div>}
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-neutral-200">{label}</p>
        {detail && (
          <p className="text-xs text-neutral-500 truncate">{detail}</p>
        )}
      </div>
      <span className={`w-2.5 h-2.5 rounded-full ${dotColor[status]} shrink-0`} />
    </div>
  );
}

export default StatusIndicator;
