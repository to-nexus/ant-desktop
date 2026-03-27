# Architecture

## Overview

ant-desktop is a Tauri v2 desktop app that bridges Figma Desktop MCP (`localhost:3845`) with Ant Cloud. It runs in the system tray and relays MCP requests over WebSocket.

```
Ant Cloud (ant-realtime)
    ↕  WebSocket (wss://, outbound from desktop)
ant-desktop (User Desktop)
    ↕  HTTP POST (localhost only)
Figma Desktop MCP (127.0.0.1:3845)
```

All connections are outbound — no inbound ports are opened.

---

## Process Model

Tauri runs two processes:

- **Core Process (Rust)** — always running. Handles WebSocket, MCP proxy, auth, health checks, system tray. The app's "brain."
- **WebView (React)** — shown only when the user opens the settings window from the tray. Uses the OS native WebView (no bundled Chromium).

Communication between the two is via Tauri IPC (`invoke` / events).

---

## Rust Modules

```
src-tauri/src/
├── main.rs              # Minimal entry point (calls lib::run)
├── lib.rs               # Tauri builder, plugin registration, deep link setup, task spawn
├── error.rs             # Unified AppError enum (Bridge | Auth | Mcp | Health | Internal)
├── validation.rs        # URL validation (server URL, web URL scheme/host checks)
├── constants.rs         # Timeouts, intervals, endpoints, keychain keys
├── commands.rs          # Tauri IPC commands exposed to frontend
├── bridge/
│   ├── mod.rs           # BridgeError enum
│   ├── client.rs        # WebSocket client (connect, reconnect, message loop)
│   └── protocol.rs      # BridgeMessage serde (register, heartbeat, disconnect, mcp, statusProbe)
├── mcp/
│   ├── mod.rs           # McpError enum
│   └── proxy.rs         # Figma MCP HTTP proxy (JSON-RPC, MCP session management)
├── auth/
│   ├── mod.rs           # AuthError enum
│   ├── jwt.rs           # JWT payload decode (sub extraction, exp validation)
│   ├── keychain.rs      # OS Keychain read/write (JWT storage)
│   └── deeplink.rs      # ant-desktop:// URL parsing + server URL validation
├── health/
│   ├── mod.rs           # HealthError enum
│   └── figma_check.rs   # Periodic Figma Desktop MCP health check (10s interval)
├── tray/
│   ├── mod.rs           # Tray icon update logic
│   └── menu.rs          # Tray menu construction, event handling, tab navigation
└── state/
    ├── mod.rs           # Re-exports
    └── app_state.rs     # AppState (Arc<Mutex>), ConnectionStatus, FigmaStatus enums
```

### Module Dependencies

```
commands  → state, bridge, auth, validation
lib       → all modules (setup + task spawn)
bridge    → mcp, state, health (for statusProbe)
mcp       → constants (Figma endpoint)
auth      → validation, constants (keychain keys)
health    → state, mcp (session reset on Figma restart)
tray      → state
```

No circular dependencies allowed.

---

## Background Tasks

Three long-running async tasks are spawned at startup via `lib.rs`:

| Task | Interval | Purpose |
|------|----------|---------|
| `bridge::client::run_loop` | Continuous | WebSocket connection, message dispatch, exponential backoff reconnect |
| Health check (inside `run_loop`) | 30s heartbeat | `BridgeHeartbeatMessage` with `figmaDesktopReachable` flag |
| `health::figma_check::check_loop` | 10s | HTTP POST to `127.0.0.1:3845/mcp`, status change events |

Task cancellation uses `tokio_util::sync::CancellationToken`. On disconnect or reconnect, the token is cancelled and a new one is issued.

---

## Connection State Machine

```
Initial ──(JWT+server OK)──→ Connecting ──(success)──→ Connected
                                 │                        │
                     (connect fail)│                        │ socket closed
                                 ↓                        ↓
                           Reconnecting ←─────────────────┘
                                 │
                                 └──(backoff: 1s→2s→4s→...→60s max)──→ Connecting

Connected ──(401/403)──→ AuthRequired
(any state) ──(user disconnect / app quit)──→ Disconnected
```

Reconnecting never stops — the app retries indefinitely until success or user action.

---

## Authentication Flow

```
ant-ui (Browser)
   │── POST /api/auth/desktop-token (cookie auth)
   │←── { token (JWT, 90-day), server URL }
   │── window.open("ant-desktop://connect?token={jwt}&server={url}")
   │
   ↓ (OS deep link)
ant-desktop
   ├── Parse & validate URL (scheme, host whitelist)
   ├── Save JWT → OS Keychain
   ├── Save server URL → tauri-plugin-store (config.json)
   └── Start WebSocket connection
```

On restart, the app restores JWT from Keychain + server URL from store and auto-connects.

JWT handling:
- **Storage**: OS Keychain (`keyring` crate) — macOS Keychain, Windows Credential Manager, Linux libsecret
- **Validation**: Base64 decode only (no signature verification — server handles that). `exp` claim is checked locally with 60s grace period.
- **Config storage**: `tauri-plugin-store` (`config.json`) — server URL, web URL. Not secrets.

---

## MCP Relay

```
MCPRequestMessage (WebSocket from cloud)
  ↓
mcp::proxy::handle_request()
  ├── Ensure MCP session initialized (initialize + notifications/initialized handshake)
  ├── Build JSON-RPC request: { method: "tools/call", params: { name, arguments } }
  ├── POST http://127.0.0.1:3845/mcp (30s timeout)
  ├── Parse response (JSON or SSE format)
  └── Return MCPResponseMessage (WebSocket to cloud)
```

- MCP calls are serialized via `tokio::sync::Mutex` (one at a time to Figma)
- Max response size: 16 MiB (`BRIDGE_WS_MAX_MESSAGE_BYTES`)
- Incoming WebSocket messages also capped at 16 MiB
- MCP session auto-resets when Figma Desktop restarts (detected by health check)

---

## Tauri IPC Commands

| Command | Purpose |
|---------|---------|
| `get_app_state` | Current connection/figma status, stats snapshot |
| `get_connection_info` | Server URL, web URL, JWT presence, user ID |
| `connect` | Save JWT + server URL, start WebSocket |
| `disconnect` | Clear JWT, stop WebSocket |
| `set_realtime_base_url` | Change server URL, reconnect |
| `set_web_url` | Change Ant Web URL (for "Open" button) |

All URL inputs are validated via `validation.rs` (http/https scheme only, valid host required).

---

## Frontend (React)

Minimal UI — most interaction is via tray icon. Settings window is 480x600px, non-resizable.

```
src/
├── App.tsx              # Tab navigation (Status / Settings / Logs) + tray event listener
├── pages/
│   ├── StatusPage.tsx   # Connection status, Figma status, stats, Web URL config, Figma open/get
│   ├── SettingsPage.tsx # Server URL config (cloud/local presets), disconnect
│   └── LogsPage.tsx     # Event log (connection, MCP, auth, figma events)
├── components/
│   ├── StatusIndicator.tsx  # Status dot + label + actions layout
│   ├── ConnectionCard.tsx   # Card container
│   └── icons/               # AntIcon, FigmaIcon (SVG components)
├── hooks/
│   └── useAppState.ts   # Tauri invoke + event subscription (5s poll + event-driven refresh)
└── lib/
    └── tauri.ts         # IPC invoke wrappers + TypeScript type definitions
```

State management: No external library. Rust `AppState` is the single source of truth. React reads via `invoke("get_app_state")` + Tauri events.

---

## Security Model

### Credential Storage

| Data | Storage | Access |
|------|---------|--------|
| JWT token | OS Keychain | OS-level app isolation |
| Server URLs | tauri-plugin-store | App data directory |

### Network

- WebSocket: TLS (`wss://`) for cloud, `ws://` for local dev
- MCP proxy: hardcoded `127.0.0.1:3845` only (no external host access)
- Deep link: local OS IPC, no network exposure

### CSP (Content Security Policy)

```
default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' asset: http://asset.localhost data:
```

Blocks external scripts, CDN, eval. Tauri auto-injects IPC-required sources.

### Input Validation

- All server/web URLs validated on both Rust and frontend (http/https scheme, valid host)
- Deep link `server` parameter validated via `validation::validate_server_url`
- Frontend `openUrl` calls go through scheme whitelist (`https:`, `http:`, `figma:`)
- WebSocket incoming messages capped at 16 MiB
- JWT `exp` checked with 60s grace period
- Error messages sanitized before IPC serialization (no internal paths/stacks)

---

## Protocol Contract

Rust structs mirror `@ant/shared` TypeScript types. JSON wire format uses `camelCase` fields and `type` tag for enum dispatch.

| Message | Direction | Purpose |
|---------|-----------|---------|
| `bridge.register` | Desktop → Cloud | Connection registration (userId, machineId, capabilities) |
| `bridge.heartbeat` | Desktop → Cloud | 30s keep-alive with `figmaDesktopReachable` flag |
| `bridge.disconnect` | Desktop → Cloud | Graceful shutdown notification |
| `bridge.statusProbe` | Cloud → Desktop | On-demand status check trigger |
| `mcp.request` | Cloud → Desktop | MCP tool call request |
| `mcp.response` | Desktop → Cloud | MCP tool call result |

### Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `BRIDGE_WS_PATH` | `/bridge/ws` | WebSocket endpoint path |
| `BRIDGE_HEARTBEAT_INTERVAL_MS` | 30,000 | Heartbeat interval |
| `BRIDGE_HEARTBEAT_TIMEOUT_MS` | 90,000 | Server-side session timeout |
| `BRIDGE_MCP_REQUEST_TIMEOUT_MS` | 30,000 | MCP request timeout |
| `BRIDGE_WS_MAX_MESSAGE_BYTES` | 16,777,216 | Max message size (16 MiB) |
| `FIGMA_MCP_ENDPOINT` | `http://127.0.0.1:3845/mcp` | Figma MCP address |
| `FIGMA_HEALTH_CHECK_INTERVAL_MS` | 10,000 | Health check interval |
| `RECONNECT_BASE_DELAY_MS` | 1,000 | Initial reconnect delay |
| `RECONNECT_MAX_DELAY_MS` | 60,000 | Max reconnect delay |

Compatibility is verified by JSON fixture round-trip tests in `src-tauri/tests/bridge_protocol.rs`.
