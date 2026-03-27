import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import StatusPage from "./pages/StatusPage";
import SettingsPage from "./pages/SettingsPage";
import LogsPage from "./pages/LogsPage";

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
