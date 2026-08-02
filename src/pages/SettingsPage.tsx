import { useState, useEffect, useCallback } from "react";
import ConnectionCard from "../components/ConnectionCard";
import {
  getConnectionInfo,
  setRealtimeBaseUrl,
  disconnect as disconnectCmd,
  type ConnectionInfo,
} from "../lib/tauri";

const CLOUD_URL = "https://ant.crosstoken.io";
const LOCAL_HOST = "http://127.0.0.1";
const DEFAULT_LOCAL_PORT = "4101";

function SettingsPage() {
  const [info, setInfo] = useState<ConnectionInfo | null>(null);
  const [urlInput, setUrlInput] = useState("");
  const [localPort, setLocalPort] = useState(DEFAULT_LOCAL_PORT);
  const [saving, setSaving] = useState(false);
  const [disconnecting, setDisconnecting] = useState(false);
  const [disconnected, setDisconnected] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  const loadInfo = useCallback(async () => {
    try {
      const data = await getConnectionInfo();
      setInfo(data);
      if (data.serverUrl) {
        setUrlInput(data.serverUrl);
        const match = data.serverUrl.match(/:(\d+)$/);
        if (match && data.serverUrl.startsWith("http://127.0.0.1")) {
          setLocalPort(match[1]);
        }
      }
    } catch (e) {
      setErrorMsg(`Failed to load connection info: ${e}`);
    }
  }, []);

  useEffect(() => {
    loadInfo();
  }, [loadInfo]);

  const applyCloud = () => {
    setUrlInput(CLOUD_URL);
  };

  const applyLocal = () => {
    setUrlInput(`${LOCAL_HOST}:${localPort}`);
  };

  const handleSave = async () => {
    const trimmed = urlInput.trim();
    if (!trimmed) return;
    try {
      const parsed = new URL(trimmed);
      if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
        setErrorMsg("Only http:// and https:// URLs are allowed");
        return;
      }
    } catch {
      setErrorMsg("Invalid URL format");
      return;
    }
    setSaving(true);
    setErrorMsg(null);
    setSuccessMsg(null);
    try {
      await setRealtimeBaseUrl(trimmed);
      await loadInfo();
      setSuccessMsg("URL saved. Reconnecting...");
      setTimeout(() => setSuccessMsg(null), 3000);
    } catch (e) {
      setErrorMsg(`Failed to save URL: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDisconnect = async () => {
    setErrorMsg(null);
    setSuccessMsg(null);
    setDisconnecting(true);
    setDisconnected(false);
    try {
      await disconnectCmd();
      await loadInfo();
      setDisconnected(true);
      setTimeout(() => setDisconnected(false), 2000);
    } catch (e) {
      setErrorMsg(`Failed to disconnect: ${e}`);
    } finally {
      setDisconnecting(false);
    }
  };

  return (
    <div className="space-y-4">
      <ConnectionCard title="Server">
        <div className="space-y-3">
          <div>
            <label className="block text-xs text-neutral-400 mb-1">
              Realtime Base URL
            </label>
            <input
              type="text"
              value={urlInput}
              onChange={(e) => setUrlInput(e.target.value)}
              placeholder="https://ant.crosstoken.io"
              className="w-full bg-neutral-800 border border-neutral-700 rounded px-3 py-1.5 text-sm text-neutral-200 placeholder-neutral-600 focus:outline-hidden focus:border-blue-500"
            />
          </div>

          <div className="flex gap-2">
            <button
              onClick={applyCloud}
              className="px-3 py-1 text-xs bg-neutral-800 border border-neutral-700 rounded hover:bg-neutral-700 text-neutral-300 transition-colors"
            >
              Cloud
            </button>
            <button
              onClick={applyLocal}
              className="px-3 py-1 text-xs bg-neutral-800 border border-neutral-700 rounded hover:bg-neutral-700 text-neutral-300 transition-colors"
            >
              Local
            </button>
            <input
              type="number"
              value={localPort}
              onChange={(e) => setLocalPort(e.target.value)}
              className="w-20 bg-neutral-800 border border-neutral-700 rounded px-2 py-1 text-xs text-neutral-300 focus:outline-hidden focus:border-blue-500"
              min="1"
              max="65535"
            />
          </div>

          <button
            onClick={handleSave}
            disabled={saving || !urlInput.trim()}
            className="w-full py-1.5 text-sm bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500 rounded text-white transition-colors"
          >
            {saving ? "Saving..." : "Save & Reconnect"}
          </button>

          {errorMsg && (
            <p className="text-xs text-red-400">{errorMsg}</p>
          )}
          {successMsg && (
            <p className="text-xs text-green-400">{successMsg}</p>
          )}
        </div>
      </ConnectionCard>

      <ConnectionCard title="Authentication">
        <div className="space-y-2 text-sm">
          <div className="flex justify-between">
            <span className="text-neutral-500">JWT Token</span>
            <span className="text-neutral-300">
              {info?.hasJwt ? "Stored" : "None"}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-neutral-500">User ID</span>
            <span className="text-neutral-300 font-mono text-xs">
              {info?.userId ?? "—"}
            </span>
          </div>
        </div>
      </ConnectionCard>

      <ConnectionCard title="Actions">
        <button
          onClick={handleDisconnect}
          disabled={!info?.hasJwt || disconnecting}
          className={`w-full py-1.5 text-sm rounded transition-colors ${
            disconnected
              ? "bg-green-900 text-green-300 cursor-default"
              : "bg-red-900 hover:bg-red-800 disabled:bg-neutral-800 disabled:text-neutral-600 disabled:cursor-not-allowed text-red-200"
          }`}
        >
          {disconnecting
            ? "Disconnecting..."
            : disconnected
              ? "Disconnected"
              : "Disconnect"}
        </button>
      </ConnectionCard>
    </div>
  );
}

export default SettingsPage;
