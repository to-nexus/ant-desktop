import { invoke } from "@tauri-apps/api/core";

/**
 * A deep-link connect request awaiting the user's approval. Carries no token —
 * `server` plus `account` is what the user has to judge, and `isAccountSwitch`
 * is the signal that matters most: an attacker's link can name the real server
 * while attaching *their* account.
 */
export interface PendingConnectView {
  server: string;
  account: string | null;
  currentAccount: string | null;
  isAccountSwitch: boolean;
}

export interface AppStateSnapshot {
  connectionStatus: string;
  figmaStatus: string;
  serverUrl: string | null;
  webUrl: string | null;
  machineId: string;
  mcpRequestCount: number;
  lastHeartbeatAgoMs: number | null;
  lastMcpRequestAgoMs: number | null;
  pendingConnect: PendingConnectView | null;
}

export interface ConnectionInfo {
  serverUrl: string | null;
  webUrl: string | null;
  hasJwt: boolean;
  userId: string | null;
  account: string | null;
}

export async function getAppState(): Promise<AppStateSnapshot> {
  return invoke<AppStateSnapshot>("get_app_state");
}

export async function getConnectionInfo(): Promise<ConnectionInfo> {
  return invoke<ConnectionInfo>("get_connection_info");
}

export async function disconnect(): Promise<void> {
  return invoke("disconnect");
}

export async function connect(
  serverUrl: string,
  jwt: string,
  userId: string,
): Promise<void> {
  return invoke("connect", { serverUrl, jwt, userId });
}

export async function confirmConnect(): Promise<void> {
  return invoke("confirm_connect");
}

export async function cancelConnect(): Promise<void> {
  return invoke("cancel_connect");
}

/**
 * Start a pairing and get back the web URL to open. The returned URL carries a
 * one-shot `desktop_pair` nonce that the web app echoes on the deep link, which
 * is how the backend recognises a connect the user began here and skips the
 * approval prompt.
 */
export async function beginPairing(webUrl: string): Promise<string> {
  return invoke<string>("begin_pairing", { webUrl });
}

export async function setRealtimeBaseUrl(url: string): Promise<void> {
  return invoke("set_realtime_base_url", { url });
}

export async function setWebUrl(url: string): Promise<void> {
  return invoke("set_web_url", { url });
}
