import { invoke } from "@tauri-apps/api/core";

export interface AppStateSnapshot {
  connectionStatus: string;
  figmaStatus: string;
  serverUrl: string | null;
  webUrl: string | null;
  machineId: string;
  mcpRequestCount: number;
  lastHeartbeatAgoMs: number | null;
  lastMcpRequestAgoMs: number | null;
}

export interface ConnectionInfo {
  serverUrl: string | null;
  webUrl: string | null;
  hasJwt: boolean;
  userId: string | null;
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

export async function setRealtimeBaseUrl(url: string): Promise<void> {
  return invoke("set_realtime_base_url", { url });
}

export async function setWebUrl(url: string): Promise<void> {
  return invoke("set_web_url", { url });
}
