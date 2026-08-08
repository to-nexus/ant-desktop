import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import StatusPage from "./pages/StatusPage";
import SettingsPage from "./pages/SettingsPage";
import LogsPage from "./pages/LogsPage";
import ConnectConfirmModal from "./components/ConnectConfirmModal";
import { useAppState } from "./hooks/useAppState";
import { confirmConnect, cancelConnect } from "./lib/tauri";

type Tab = "status" | "settings" | "logs";

const REJECTED_NOTICE_DURATION_MS = 10_000;

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("status");
  const { state, error, refresh } = useAppState();
  const [rejectedNotice, setRejectedNotice] = useState<string | null>(null);
  const noticeTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );

  useEffect(() => {
    const unlisten = listen<string>("navigate-tab", (event) => {
      const tab = event.payload;
      if (tab === "settings" || tab === "logs") {
        setActiveTab(tab);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // A parked deep-link connect lives in backend state, not in this event — the
  // event is only a "look now" nudge, so a cold-start deep link that fired
  // before this webview existed still surfaces on the next poll.
  useEffect(() => {
    const unlisteners = [
      listen("auth-connect-request", () => refresh()),
      listen("auth-received", () => refresh()),
      listen<{ server?: string; reason?: string }>(
        "auth-connect-rejected",
        (event) => {
          const server = event.payload?.server ?? "an unknown server";
          setRejectedNotice(
            `Refused a connection request for ${server}. If this is your own server, add it in Settings first.`,
          );
          clearTimeout(noticeTimeoutRef.current);
          noticeTimeoutRef.current = setTimeout(
            () => setRejectedNotice(null),
            REJECTED_NOTICE_DURATION_MS,
          );
        },
      ),
    ];
    return () => {
      clearTimeout(noticeTimeoutRef.current);
      unlisteners.forEach((p) => p.then((fn) => fn()).catch(() => {}));
    };
  }, [refresh]);

  const approveConnect = useCallback(async () => {
    await confirmConnect();
    await refresh();
  }, [refresh]);

  const rejectConnect = useCallback(async () => {
    await cancelConnect();
    await refresh();
  }, [refresh]);

  const tabs: { id: Tab; label: string }[] = [
    { id: "status", label: "Status" },
    { id: "settings", label: "Settings" },
    { id: "logs", label: "Logs" },
  ];

  return (
    <div className="flex flex-col h-screen bg-[#0f0f0f]">
      <nav className="flex border-b border-neutral-800 px-4 pt-3">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2 text-sm font-medium transition-colors rounded-t-md ${
              activeTab === tab.id
                ? "text-white bg-neutral-800 border-b-2 border-blue-500"
                : "text-neutral-400 hover:text-neutral-200"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      <main className="flex-1 overflow-y-auto p-4">
        {rejectedNotice && (
          <p className="mb-3 rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-[11px] text-amber-400">
            {rejectedNotice}
          </p>
        )}
        {activeTab === "status" && <StatusPage state={state} error={error} />}
        {activeTab === "settings" && <SettingsPage />}
        {activeTab === "logs" && <LogsPage />}
      </main>

      {state?.pendingConnect && (
        <ConnectConfirmModal
          pending={state.pendingConnect}
          onApprove={approveConnect}
          onReject={rejectConnect}
        />
      )}
    </div>
  );
}

export default App;
