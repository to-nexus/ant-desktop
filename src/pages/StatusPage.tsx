import { useState, useCallback, useEffect, useRef, type ReactNode } from "react";
import { useAppState } from "../hooks/useAppState";
import { openUrl } from "@tauri-apps/plugin-opener";
import { setWebUrl } from "../lib/tauri";
import StatusIndicator from "../components/StatusIndicator";
import ConnectionCard from "../components/ConnectionCard";
import AntIcon from "../components/icons/AntIcon";
import FigmaIcon from "../components/icons/FigmaIcon";

const FIGMA_DOWNLOAD_URL = "https://www.figma.com/downloads/";
const FIGMA_DEEPLINK_URL = "figma://";

type WebUrlMode = "auto" | "cloud" | "local" | "custom";

const ICON_SIZE = 12;

function ExternalLinkIcon() {
  return (
    <svg width={ICON_SIZE} height={ICON_SIZE} viewBox="0 0 24 24" fill="none"
      stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
      <polyline points="15 3 21 3 21 9" />
      <line x1="10" y1="14" x2="21" y2="3" />
    </svg>
  );
}

function DownloadIcon() {
  return (
    <svg width={ICON_SIZE} height={ICON_SIZE} viewBox="0 0 24 24" fill="none"
      stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="7 10 12 15 17 10" />
      <line x1="12" y1="15" x2="12" y2="3" />
    </svg>
  );
}

function GearIcon() {
  return (
    <svg width={ICON_SIZE} height={ICON_SIZE} viewBox="0 0 24 24" fill="none"
      stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

function Spinner() {
  return (
    <svg className="animate-spin" width={ICON_SIZE} height={ICON_SIZE} viewBox="0 0 24 24"
      fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
      <path d="M12 2a10 10 0 0 1 10 10" />
    </svg>
  );
}

const FIGMA_OPEN_TIMEOUT_MS = 12_000;
const FIGMA_NOTICE_DURATION_MS = 5_000;

const ALLOWED_URL_SCHEMES = ["https:", "http:", "figma:"];

function safeOpenUrl(url: string) {
  try {
    const parsed = new URL(url);
    if (!ALLOWED_URL_SCHEMES.some((s) => parsed.protocol === s)) {
      return;
    }
    return openUrl(url);
  } catch {
    // invalid URL -- silently ignore
  }
}

const badgeBase = "inline-flex items-center gap-1 text-[11px] px-1.5 py-0.5 rounded border transition-colors cursor-pointer";
const badgeAction = `${badgeBase} bg-neutral-800 border-neutral-700 text-neutral-300 hover:bg-neutral-700 hover:text-neutral-100`;
const badgeIconOnly = `${badgeBase} bg-neutral-800 border-neutral-700 text-neutral-400 hover:bg-neutral-700 hover:text-neutral-200`;

function ActionBadge({ icon, label, title, onClick }: {
  icon: ReactNode;
  label?: string;
  title?: string;
  onClick: () => void;
}) {
  return (
    <button onClick={onClick} title={title} className={label ? badgeAction : badgeIconOnly}>
      {icon}
      {label && <span>{label}</span>}
    </button>
  );
}

function resolveWebUrl(
  serverUrl: string | null,
  customWebUrl: string | null,
): string | null {
  if (customWebUrl) return customWebUrl;
  if (!serverUrl) return null;
  if (serverUrl.includes("127.0.0.1") || serverUrl.includes("localhost")) {
    return "http://localhost:4200/app";
  }
  try {
    const u = new URL(serverUrl);
    return `${u.protocol}//${u.hostname}/app`;
  } catch {
    return null;
  }
}

function detectCurrentMode(
  webUrl: string | null,
  serverUrl: string | null,
): WebUrlMode {
  if (!webUrl) return "auto";
  if (webUrl === "https://ant.crosstoken.io/app") return "cloud";
  if (webUrl === "http://localhost:4200/app") {
    const isLocalServer =
      serverUrl?.includes("127.0.0.1") || serverUrl?.includes("localhost");
    if (isLocalServer) return "auto";
    return "local";
  }
  return "custom";
}

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

function WebUrlSettings({
  serverUrl,
  webUrl,
  onClose,
}: {
  serverUrl: string | null;
  webUrl: string | null;
  onClose: () => void;
}) {
  const initialMode = detectCurrentMode(webUrl, serverUrl);
  const [mode, setMode] = useState<WebUrlMode>(initialMode);
  const [customUrl, setCustomUrl] = useState(
    initialMode === "custom" ? (webUrl ?? "") : "",
  );
  const [saving, setSaving] = useState(false);
  const [urlError, setUrlError] = useState<string | null>(null);

  const handleSave = useCallback(async () => {
    setUrlError(null);
    let url = "";
    switch (mode) {
      case "auto":
        url = "";
        break;
      case "cloud":
        url = "https://ant.crosstoken.io/app";
        break;
      case "local":
        url = "http://localhost:4200/app";
        break;
      case "custom":
        url = customUrl.trim();
        break;
    }
    if (url) {
      try {
        const parsed = new URL(url);
        if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
          setUrlError("Only http/https URLs are allowed");
          return;
        }
      } catch {
        setUrlError("Invalid URL format");
        return;
      }
    }
    setSaving(true);
    try {
      await setWebUrl(url);
      onClose();
    } finally {
      setSaving(false);
    }
  }, [mode, customUrl, onClose]);

  const autoResolved = resolveWebUrl(serverUrl, null);

  return (
    <div className="mt-2 bg-neutral-800/60 border border-neutral-700 rounded-lg p-3 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-neutral-300">
          Ant Web URL
        </span>
        <button
          onClick={onClose}
          className="text-neutral-500 hover:text-neutral-300 text-xs"
        >
          ✕
        </button>
      </div>

      <label className="flex items-center gap-2 text-xs text-neutral-300 cursor-pointer">
        <input
          type="radio"
          name="webUrlMode"
          checked={mode === "auto"}
          onChange={() => setMode("auto")}
          className="accent-blue-500"
        />
        <span>
          Auto{" "}
          <span className="text-neutral-500">
            ({autoResolved ?? "—"})
          </span>
        </span>
      </label>

      <label className="flex items-center gap-2 text-xs text-neutral-300 cursor-pointer">
        <input
          type="radio"
          name="webUrlMode"
          checked={mode === "cloud"}
          onChange={() => setMode("cloud")}
          className="accent-blue-500"
        />
        <span>Cloud (ant.crosstoken.io/app)</span>
      </label>

      <label className="flex items-center gap-2 text-xs text-neutral-300 cursor-pointer">
        <input
          type="radio"
          name="webUrlMode"
          checked={mode === "local"}
          onChange={() => setMode("local")}
          className="accent-blue-500"
        />
        <span>Local (localhost:4200/app)</span>
      </label>

      <label className="flex items-center gap-2 text-xs text-neutral-300 cursor-pointer">
        <input
          type="radio"
          name="webUrlMode"
          checked={mode === "custom"}
          onChange={() => setMode("custom")}
          className="accent-blue-500"
        />
        <span>Custom</span>
      </label>

      {mode === "custom" && (
        <input
          type="text"
          value={customUrl}
          onChange={(e) => setCustomUrl(e.target.value)}
          placeholder="https://..."
          className="w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-xs text-neutral-200 focus:outline-none focus:border-blue-500"
        />
      )}

      {urlError && (
        <p className="text-[11px] text-red-400">{urlError}</p>
      )}

      <div className="flex justify-end pt-1">
        <button
          onClick={handleSave}
          disabled={saving || (mode === "custom" && !customUrl.trim())}
          className="text-xs px-3 py-1 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500 text-white rounded transition-colors"
        >
          {saving ? "Saving..." : "Save"}
        </button>
      </div>
    </div>
  );
}

function StatusPage() {
  const { state, error } = useAppState();
  const [showWebUrlSettings, setShowWebUrlSettings] = useState(false);
  const [figmaOpening, setFigmaOpening] = useState(false);
  const [figmaNotice, setFigmaNotice] = useState<string | null>(null);
  const openTimeoutRef = useRef<ReturnType<typeof setTimeout>>();
  const noticeTimeoutRef = useRef<ReturnType<typeof setTimeout>>();

  const showNotice = useCallback((msg: string) => {
    setFigmaNotice(msg);
    clearTimeout(noticeTimeoutRef.current);
    noticeTimeoutRef.current = setTimeout(() => setFigmaNotice(null), FIGMA_NOTICE_DURATION_MS);
  }, []);

  const handleFigmaOpen = useCallback(async () => {
    if (figmaOpening) return;
    setFigmaNotice(null);
    setFigmaOpening(true);
    clearTimeout(openTimeoutRef.current);
    openTimeoutRef.current = setTimeout(() => {
      setFigmaOpening(false);
      showNotice("Figma not detected");
    }, FIGMA_OPEN_TIMEOUT_MS);
    try {
      await safeOpenUrl(FIGMA_DEEPLINK_URL);
    } catch {
      setFigmaOpening(false);
      clearTimeout(openTimeoutRef.current);
      showNotice("Figma not detected");
    }
  }, [figmaOpening, showNotice]);

  useEffect(() => {
    if (state?.figmaStatus === "available" && figmaOpening) {
      setFigmaOpening(false);
      setFigmaNotice(null);
      clearTimeout(openTimeoutRef.current);
      clearTimeout(noticeTimeoutRef.current);
    }
  }, [state?.figmaStatus, figmaOpening]);

  useEffect(() => {
    return () => {
      clearTimeout(openTimeoutRef.current);
      clearTimeout(noticeTimeoutRef.current);
    };
  }, []);

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
  const webUrl = resolveWebUrl(state.serverUrl, state.webUrl);

  return (
    <div className="space-y-4">
      <ConnectionCard title="Ant Server">
        <StatusIndicator
          icon={<AntIcon size={24} />}
          status={conn.status}
          label={conn.label}
          detail={state.serverUrl ?? "No server configured"}
          actions={
            <>
              {webUrl && (
                <ActionBadge
                  icon={<ExternalLinkIcon />}
                  label="Open"
                  title={webUrl}
                  onClick={() => safeOpenUrl(webUrl)}
                />
              )}
              <ActionBadge
                icon={<GearIcon />}
                title="Configure Ant Web URL"
                onClick={() => setShowWebUrlSettings((v) => !v)}
              />
            </>
          }
        />
        {showWebUrlSettings && (
          <WebUrlSettings
            serverUrl={state.serverUrl}
            webUrl={state.webUrl}
            onClose={() => setShowWebUrlSettings(false)}
          />
        )}
      </ConnectionCard>

      <ConnectionCard title="Figma Desktop">
        <StatusIndicator
          icon={<FigmaIcon size={24} />}
          status={figma.status}
          label={figma.label}
          detail="localhost:3845"
          actions={
            state.figmaStatus !== "available" ? (
              figmaOpening ? (
                <span className="inline-flex items-center gap-1.5 text-[11px] text-neutral-400">
                  <Spinner />
                  Opening...
                </span>
              ) : (
                <>
                  <ActionBadge
                    icon={<ExternalLinkIcon />}
                    label="Open"
                    title="Launch Figma Desktop"
                    onClick={handleFigmaOpen}
                  />
                  <ActionBadge
                    icon={<DownloadIcon />}
                    label="Get"
                    title="Download Figma Desktop"
                    onClick={() => safeOpenUrl(FIGMA_DOWNLOAD_URL)}
                  />
                </>
              )
            ) : undefined
          }
          notice={
            figmaNotice ? (
              <p className="text-[11px] text-amber-400 mt-0.5">{figmaNotice}</p>
            ) : undefined
          }
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
