# Security Policy

Ant Desktop is a Tauri bridge that exposes the locally-running **Figma Desktop
MCP server** to an Ant backend over an outbound WebSocket. It holds a session
token and can reach a design tool — so we take reports seriously.

The broader Ant security policy lives in the
[main repository](https://github.com/to-nexus/ant/blob/main/SECURITY.md); this
document covers the desktop app's own surface.

## Supported Versions

Pre-1.0. Security fixes land on `main` and ship in the next tagged release.

| Version | Supported          |
|---------|--------------------|
| `main`  | :white_check_mark: |
| < 0.x   | :x:                |

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

1. **GitHub Security Advisories** (preferred): the
   [Report a vulnerability](../../security/advisories/new) button on the
   Security tab.
2. **Email**: `probe@to.nexus`.

Include the affected component (bridge / MCP / auth / deep link / tray), a
minimal reproduction, the OS and app version, and the impact you observed.

We acknowledge reports within **3 business days**; targets match the main
repository (Critical 7d / High 14d / Medium 30d / Low best effort).

## Threat Model Notes

Areas that are first-class in our threat model, and where we have made explicit
design decisions worth knowing before you report:

- **Deep links (`ant-desktop://`).** A `connect` request is *parked*, never
  auto-applied — the user must confirm a dialog showing the target server
  before any connection state changes. This is deliberate: it blocks drive-by
  account/server swaps from a malicious web page. Reports of an auto-applying
  deep link are high severity.
- **Server URL validation.** Only `http://` and `https://` are accepted;
  `javascript:`, `data:`, and friends are rejected. Note that `http://` is
  accepted for **any** host, not just loopback, so a confirmed deep link can
  point the bridge at a plaintext remote server. We accept this today for
  self-hosted LAN setups.
- **JWT handling.** Tokens are stored in the OS keychain, never on disk in
  plaintext. The app decodes the payload to display an account identity and
  check `exp`, but **does not verify the signature** — the server is the
  verifying party. Anything that treats the decoded `sub` as an authorization
  decision would be a bug; please report it.
- **MCP scope.** The bridge relays to Figma Desktop's local MCP endpoint. A
  path that lets a remote peer drive arbitrary local MCP servers, or reach
  beyond the Figma endpoint, is in scope.
- **Capabilities.** The Tauri capability set is intentionally narrow; the only
  widened permission is `opener:allow-open-url` scoped to `figma://*`. Any
  bypass that opens other schemes or arbitrary URLs is in scope.

## Out of Scope

- Scanner output with no working proof-of-concept.
- **Unsigned/un-notarized builds.** Release binaries are currently ad-hoc
  signed (`signingIdentity: "-"`) and not notarized — this is a known,
  documented gap (see `docs/DEPLOYMENT_GUIDE.md`), not a report. macOS will
  warn on first launch; verify your download before running it.
- Issues requiring an attacker who already has local code execution as the
  same user.

Thank you for helping keep Ant users safe.
