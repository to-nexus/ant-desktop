# ant-desktop

A lightweight desktop app that bridges [Figma Desktop MCP](https://www.figma.com/) (`localhost:3845`) with Ant Cloud, enabling Ant's Design Jobs to access Figma's rich design analysis capabilities.

## Why

Figma Desktop MCP is only accessible at `http://127.0.0.1:3845/mcp`. Ant Cloud workers run remotely and cannot reach a user's localhost. ant-desktop runs on the user's desktop, maintains an outbound WebSocket to Ant Cloud, and relays MCP requests/responses between the two.

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

**macOS:** Xcode Command Line Tools (`xcode-select --install`)

**Linux:** System packages required — see [Tauri prerequisites](https://tauri.app/start/prerequisites/#linux).

## Quick Start

```bash
# Install dependencies
pnpm install

# Run in development mode (Rust + Vite HMR)
pnpm tauri dev
```

The app starts in the system tray. Click the tray icon to open the settings window.

### Deep Link Testing (macOS)

```bash
open "ant-desktop://connect?token=test-jwt&server=http://127.0.0.1:4101"
```

### Figma MCP Verification

```bash
curl -X POST http://127.0.0.1:3845/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":"test","method":"tools/list","params":{}}'
```

## Production Build

```bash
# Build for current platform (creates installer in src-tauri/target/release/bundle/)
pnpm tauri build

# macOS universal binary (ARM + Intel)
pnpm tauri build --target universal-apple-darwin
```

See [docs/DEPLOYMENT_GUIDE.md](docs/DEPLOYMENT_GUIDE.md) for signing, CI/CD, and distribution details.

## Project Structure

```
ant-desktop/
├── docs/                        # Product & architecture docs (SSOT)
│   ├── PRD.md                   # Requirements, tiers, protocol contract
│   ├── SYSTEM_DESIGN.md         # Architecture, modules, serde mapping
│   ├── DEVELOPMENT_GUIDE.md     # Scaffolding, conventions, phase roadmap
│   ├── DEPLOYMENT_GUIDE.md      # Build, sign, distribute, auto-update
│   └── RELEASE_GUIDE.md         # Step-by-step release process for DevOps
├── src/                         # React frontend (TypeScript)
│   ├── App.tsx                  # Tab navigation (Status / Settings / Logs)
│   ├── pages/                   # StatusPage, SettingsPage, LogsPage
│   ├── components/              # StatusIndicator, ConnectionCard
│   ├── hooks/                   # useAppState (Tauri event subscription)
│   └── lib/tauri.ts             # IPC invoke wrappers + types
├── src-tauri/                   # Rust backend (Tauri Core)
│   ├── src/
│   │   ├── lib.rs               # App builder, plugin registration, task spawn
│   │   ├── bridge/              # WebSocket client + protocol serde
│   │   ├── mcp/                 # Figma MCP HTTP proxy (JSON-RPC)
│   │   ├── auth/                # JWT decode, OS Keychain, deep link
│   │   ├── health/              # Figma Desktop availability check
│   │   ├── tray/                # System tray icon + status menu
│   │   ├── state/               # AppState (Arc<Mutex>)
│   │   ├── commands.rs          # Tauri IPC commands
│   │   ├── constants.rs         # Timeouts, intervals, URLs
│   │   └── error.rs             # Unified AppError enum
│   └── tests/                   # Integration tests + JSON fixtures
├── package.json
├── tailwind.config.js
├── vite.config.ts
└── tsconfig.json
```

## Development Commands

```bash
pnpm tauri dev              # Full dev server (Rust + Vite HMR)
pnpm dev                    # Frontend only (no Rust recompile)
pnpm tauri build            # Production build + installer

cd src-tauri
cargo check                 # Rust type check
cargo test                  # Run Rust tests
cargo clippy                # Lint
```

## Documentation

| Document | Purpose |
|----------|---------|
| [PRD](docs/PRD.md) | Product requirements, user flows, Figma tier definitions |
| [System Design](docs/SYSTEM_DESIGN.md) | Architecture, module structure, protocol, state machines |
| [Development Guide](docs/DEVELOPMENT_GUIDE.md) | Setup, coding conventions, implementation phases |
| [Deployment Guide](docs/DEPLOYMENT_GUIDE.md) | Build, code signing, CI/CD, auto-update, distribution |
| [Release Guide](docs/RELEASE_GUIDE.md) | Step-by-step release process for DevOps team |

## Tech Stack

- **Framework:** [Tauri v2](https://tauri.app/) (cross-platform desktop)
- **Backend:** Rust (WebSocket, HTTP proxy, OS integration)
- **Frontend:** React 18 + TypeScript + Tailwind CSS v3
- **Build:** Vite 6 + Cargo

## License

Private — see repository access policies.
