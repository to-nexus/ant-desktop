import { type ReactNode } from "react";

interface StatusIndicatorProps {
  status: "connected" | "warning" | "error" | "inactive";
  label: string;
  detail?: string;
  icon?: ReactNode;
  actions?: ReactNode;
  notice?: ReactNode;
}

const dotColor = {
  connected: "bg-status-connected",
  warning: "bg-status-warning",
  error: "bg-status-error",
  inactive: "bg-status-inactive",
};

function StatusIndicator({ status, label, detail, icon, actions, notice }: StatusIndicatorProps) {
  return (
    <div className="flex items-start gap-3 py-2">
      {icon && <div className="shrink-0 mt-0.5">{icon}</div>}
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium text-neutral-200">{label}</p>
          {actions && (
            <div className="flex items-center gap-1.5 ml-auto shrink-0">{actions}</div>
          )}
        </div>
        {detail && (
          <p className="text-xs text-neutral-500 truncate">{detail}</p>
        )}
        {notice}
      </div>
      <span className={`w-2.5 h-2.5 rounded-full ${dotColor[status]} shrink-0 mt-1`} />
    </div>
  );
}

export default StatusIndicator;
