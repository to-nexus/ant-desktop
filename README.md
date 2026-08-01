# ant-desktop

A lightweight desktop bridge that connects [Figma Desktop's MCP server](https://www.figma.com/)
(`127.0.0.1:3845`) to an [Ant](https://github.com/to-nexus/ant) backend, so
Ant's design jobs can use Figma's design-analysis tools
(`figma_get_design_context`, `figma_get_metadata`, `figma_get_screenshot`,
`figma_get_variable_defs`).

Figma's MCP server is local-only by design — it listens on loopback and has no
public endpoint. A cloud worker cannot reach it, and neither can a self-hosted
backend on a different machine. This app is the bridge: it opens an
**outbound** WebSocket to the backend and relays MCP calls to Figma on the
backend's behalf.

```
Ant backend (design job worker)
    ↕  WebSocket — always outbound from your desktop
ant-desktop (your machine)
    ↕  HTTP POST — loopback only
Figma Desktop MCP (127.0.0.1:3845)
```

The bridge relays; it never embeds or launches a backend of its own.

## Which backend?

Both are supported — pick one in Settings:

| Target | Default |
|---|---|
| Managed cloud | `https://ant.crosstoken.io` |
| Self-hosted `ant-realtime` | `http://127.0.0.1:4101` |

Any `http(s)` host works, so a LAN-hosted backend is fine too. Ant can also
hand you a `ant-desktop://` deep link to fill this in — the app **parks** such
a request and shows a confirmation dialog naming the target server before
anything changes, so a web page cannot silently repoint your bridge.

## Prerequisites

| Tool    | Version       | Install                                                           |
|---------|---------------|-------------------------------------------------------------------|
| Rust    | stable latest | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | >= 22.13      | https://nodejs.org or `nvm install 22`                            |
| pnpm    | 11.1.0        | `corepack enable && corepack prepare pnpm@11.1.0 --activate`       |

**macOS:** Xcode CLI Tools (`xcode-select --install`)
**Linux:** see [Tauri prerequisites](https://tauri.app/start/prerequisites/#linux)

## Quick Start

```bash
pnpm install
pnpm tauri dev      # Vite on :1420 (strictPort) + the Rust host
```

The app starts in the system tray; click the tray icon to open the window.
`pnpm dev` runs the web layer alone — useful for UI work, but no Tauri command
is reachable.

## Build

```bash
pnpm tauri build                                  # current platform
pnpm tauri build --target universal-apple-darwin  # macOS universal (ARM + Intel)
```

Artifacts land in `src-tauri/target/release/bundle/`.

> ⚠️ **Release binaries are ad-hoc signed and not notarized.** macOS will warn
> on first launch and you may need to allow the app explicitly in System
> Settings. Verify what you downloaded before running it. See
> [DEPLOYMENT_GUIDE.md](docs/DEPLOYMENT_GUIDE.md).

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
│   │   ├── auth/                # JWT, OS keychain, deep-link handler
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
    └── release.yml              # Multi-platform build on tag push
```

## Documentation

| Document | Purpose |
|---|---|
| [Architecture](docs/ARCHITECTURE.md) | Modules, protocol contract, security model |
| [Build & Release](docs/DEPLOYMENT_GUIDE.md) | Build, code signing, CI/CD, release checklist |
| [Contributing](CONTRIBUTING.md) | Dev setup, checks, conventions |
| [Security](SECURITY.md) | Reporting, threat model, deliberate design decisions |

## Tech Stack

- **Framework:** [Tauri v2](https://tauri.app/)
- **Backend:** Rust (tokio, tokio-tungstenite, reqwest, keyring)
- **Frontend:** React 18 + TypeScript + Tailwind CSS
- **Build:** Vite + Cargo + Tauri Bundler

## Contributing

Like [Ant](https://github.com/to-nexus/ant) itself, this app is
**solo-developed**, so reviews are best-effort — if a PR sits for a week,
bump it. Issues and PRs are welcome, particularly platform coverage (the Linux
and Windows build targets are currently disabled), Figma MCP tool support, and
bug reports that include the Logs tab output.

The scope is deliberately narrow: this relays Figma's MCP server to an Ant
backend and does nothing else. Several behaviours around deep links, URL
validation, and keychain storage are intentional — read
[CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md) before
simplifying any of them.

## License

Apache-2.0 — see [LICENSE](LICENSE). Contributions are accepted under the same
license; there is no CLA to sign.
