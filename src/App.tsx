import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import StatusPage from "./pages/StatusPage";
import SettingsPage from "./pages/SettingsPage";
import LogsPage from "./pages/LogsPage";
import { confirmConnect, cancelConnect } from "./lib/tauri";

type Tab = "status" | "settings" | "logs";

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("status");

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

  // Deep-link connect requires explicit user confirmation — a web page can
  // trigger the `ant-desktop://connect` scheme, so the backend parks the
  // request and we confirm before switching the app's server/account.
  useEffect(() => {
    const unlisten = listen<{ server: string }>(
      "auth-connect-request",
      async (event) => {
        const server = event.payload?.server ?? "an unknown server";
        const approved = window.confirm(
          `Connect this app to:\n\n${server}\n\n` +
            "Approve ONLY if you just started this sign-in. Approving switches " +
            "the app to this server and account.",
        );
        try {
          await (approved ? confirmConnect() : cancelConnect());
        } catch (e) {
          console.error("deep-link connect confirmation failed", e);
        }
      },
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

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
        {activeTab === "status" && <StatusPage />}
        {activeTab === "settings" && <SettingsPage />}
        {activeTab === "logs" && <LogsPage />}
      </main>
    </div>
  );
}

export default App;
