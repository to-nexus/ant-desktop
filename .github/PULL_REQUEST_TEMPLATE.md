<!--
Thanks for contributing to Ant Desktop.
Please read CONTRIBUTING.md first — especially the "Conventions" section, which
lists behaviours that are deliberate and must not be simplified away.
-->

## Summary

<!-- What this PR changes and why, in a sentence or two. -->

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Refactor (no functional change)
- [ ] Documentation
- [ ] CI / build / tooling

## Affected area

- [ ] `src/` — React UI
- [ ] `src-tauri/bridge/` — WebSocket client to `ant-realtime`
- [ ] `src-tauri/mcp/` — Figma MCP relay
- [ ] `src-tauri/auth/` — keychain / JWT / deep links
- [ ] `src-tauri/tray/`, window, or app lifecycle
- [ ] Build, release, or CI

## Security surface

<!--
Answer explicitly. If any box is checked, explain the rationale here — a
reviewer will not infer it from the diff.
-->

- [ ] Adds or widens a Tauri capability
- [ ] Adds a new outbound host
- [ ] Changes deep-link handling or the confirmation dialog
- [ ] Changes server-URL validation
- [ ] Changes how JWTs are stored or interpreted
- [ ] None of the above

## Test plan

<!-- What you ran and what you exercised manually (Figma connected? backend reachable?). -->

```
pnpm build
cargo fmt   --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test  --manifest-path src-tauri/Cargo.toml
```

## Linked issue

<!-- e.g. Closes #12 -->

## Checklist

- [ ] The four checks above pass locally.
- [ ] I updated the README or `docs/` if this changes user-visible behaviour.
- [ ] No secrets, tokens, or private hostnames in this diff.
- [ ] Commit messages are in English and follow Conventional Commits.
