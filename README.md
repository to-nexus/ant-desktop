# ant-desktop

A lightweight desktop bridge that connects [Figma Desktop MCP](https://www.figma.com/) (`localhost:3845`) with Ant Cloud, enabling Ant's Design Jobs to access Figma's design analysis capabilities (`get_design_context`, etc.).

```
Ant Cloud (Design Job Worker)
    ↕  WebSocket (outbound from desktop)
ant-desktop (User Desktop)
    ↕  HTTP POST (localhost only)
Figma Desktop MCP (127.0.0.1:3845)
```

## Prerequisites

| Tool    | Version       | Install                                                    |
|---------|---------------|------------------------------------------------------------|
| Rust    | stable latest | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 20+           | https://nodejs.org or `nvm install 20`                     |
| pnpm    | 9+            | `npm install -g pnpm`                                      |

**macOS:** Xcode CLI Tools (`xcode-select --install`)  
**Linux:** See [Tauri prerequisites](https://tauri.app/start/prerequisites/#linux)

## Quick Start

```bash
pnpm install
pnpm tauri dev
```

The app starts in the system tray. Click the tray icon to open the settings window.

## Build

```bash
pnpm tauri build                                 # Current platform
pnpm tauri build --target universal-apple-darwin  # macOS universal (ARM + Intel)
```

Artifacts: `src-tauri/target/release/bundle/`

## Project Structure

```
ant-desktop/
├── src/                         # React frontend (TypeScript + Tailwind)
│   ├── App.tsx                  # Tab navigation (Status / Settings / Logs)
│   ├── pages/                   # StatusPage, SettingsPage, LogsPage
│   ├── components/              # StatusIndicator, ConnectionCard, icons/
│   ├── hooks/useAppState.ts     # Tauri event subscription + polling
│   └── lib/tauri.ts             # IPC invoke wrappers + types
├── src-tauri/                   # Rust backend (Tauri Core)
│   ├── src/
│   │   ├── lib.rs               # App builder, plugin setup, task spawn
│   │   ├── bridge/              # WebSocket client + protocol serde
│   │   ├── mcp/                 # Figma MCP HTTP proxy (JSON-RPC)
│   │   ├── auth/                # JWT, OS Keychain, deep link handler
│   │   ├── health/              # Figma Desktop health check
│   │   ├── tray/                # System tray icon + menu
│   │   ├── state/               # AppState (Arc<Mutex>)
│   │   ├── commands.rs          # Tauri IPC commands
│   │   ├── validation.rs        # URL validation (scheme, host)
│   │   ├── constants.rs         # Timeouts, intervals, endpoints
│   │   └── error.rs             # Unified AppError enum
│   └── tests/                   # Integration tests + JSON fixtures
├── docs/
│   ├── ARCHITECTURE.md          # System architecture, modules, protocol
│   └── DEPLOYMENT_GUIDE.md      # Build, signing, CI/CD, release process
└── .github/workflows/
    └── release.yml              # CI/CD: multi-platform build on tag push
```

## Documentation

| Document | Purpose |
|----------|---------|
| [Architecture](docs/ARCHITECTURE.md) | System architecture, modules, protocol contract, security model |
| [Build & Release](docs/DEPLOYMENT_GUIDE.md) | Build, code signing, CI/CD, auto-update, release checklist |

## Tech Stack

- **Framework:** [Tauri v2](https://tauri.app/)
- **Backend:** Rust (tokio, tokio-tungstenite, reqwest, keyring)
- **Frontend:** React 18 + TypeScript + Tailwind CSS
- **Build:** Vite + Cargo + Tauri Bundler

## License

Private — see repository access policies.
