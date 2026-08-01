# Contributing to Ant Desktop

Ant Desktop is the Tauri companion app that bridges a locally-running **Figma
Desktop MCP server** to an Ant backend — either the managed cloud or your own
self-hosted `ant-realtime`. It is a thin, outbound-only client: no server, no
embedded backend.

The platform itself lives in [to-nexus/ant](https://github.com/to-nexus/ant);
its [Code of Conduct](https://github.com/to-nexus/ant/blob/main/CODE_OF_CONDUCT.md)
governs this repository too.

---

## Prerequisites

- **Node.js** >= 22.13 (matches the main repo's `engines`)
- **pnpm** 11.1.0 (`corepack enable && corepack prepare pnpm@11.1.0 --activate`)
- **Rust** stable toolchain + the
  [Tauri v2 system dependencies](https://tauri.app/start/prerequisites/) for
  your OS
- **Figma Desktop** with the MCP server enabled, to exercise the bridge

## Local Setup

```bash
pnpm install
pnpm tauri dev     # Vite on :1420 (strictPort) + the Rust host
```

`pnpm dev` alone runs only the web layer, which is useful for UI work but
cannot reach any Tauri command.

## Layout

```
src/                    React UI (pages, components, hooks, lib/tauri.ts)
src-tauri/src/
├── lib.rs              app setup, deep-link handling, task spawning
├── bridge/             outbound WebSocket client to ant-realtime
├── mcp/                Figma Desktop MCP relay
├── auth/               keychain-backed JWT storage
├── health/             Figma reachability polling
├── tray/               menu-bar surface
├── commands.rs         Tauri command surface exposed to the UI
├── validation.rs       server-URL validation
└── constants.rs        default cloud/local endpoints
src-tauri/tests/        Rust integration tests (bridge protocol + fixtures)
```

## Checks

These four are exactly what CI runs
([.github/workflows/ci.yml](.github/workflows/ci.yml)):

```bash
pnpm build                                   # tsc + vite build
cargo fmt     --manifest-path src-tauri/Cargo.toml --all --check
cargo clippy  --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test    --manifest-path src-tauri/Cargo.toml
```

If a dependency's `postinstall` script needs to run, declare it in
[`pnpm-workspace.yaml`](pnpm-workspace.yaml) under `allowBuilds` with an
explicit `true` / `false`. pnpm 11 turns an undeclared or unresolved entry into
a hard error, so leaving pnpm's generated placeholder in place breaks
`pnpm install --frozen-lockfile` in CI.

## Conventions

- **Commit messages in English**, Conventional Commits style
  (`fix(bridge): …`, `feat(tray): …`).
- **Keep the security posture.** Several behaviours are deliberate and must not
  be "simplified" away — see [SECURITY.md](SECURITY.md) for the reasoning:
  - deep-link `connect` requests are parked behind a user confirmation dialog,
    never auto-applied;
  - server URLs are validated against an http/https allowlist;
  - JWTs live in the OS keychain, and the locally-decoded payload is never
    treated as an authorization decision;
  - the Tauri capability set stays minimal — widening it needs a rationale in
    the PR description.
- **No new outbound hosts** without discussion. The app talks to the configured
  Ant backend and the local Figma MCP endpoint, nothing else.

## Pull Requests

Describe what changed and why, note any capability/permission change
explicitly, and confirm you ran the checks above. Security-sensitive findings
should go through [SECURITY.md](SECURITY.md) instead of a public PR.

## License

By contributing you agree that your contributions are licensed under the
[Apache License 2.0](LICENSE).
