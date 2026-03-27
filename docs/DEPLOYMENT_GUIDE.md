# Build & Release Guide

How to build, sign, and release ant-desktop. Covers local builds, code signing, CI/CD, and auto-update.

---

## Table of Contents

1. [Local Build](#1-local-build)
2. [Distributing Without Code Signing (Dev / Internal)](#2-distributing-without-code-signing)
3. [macOS Code Signing + Notarization](#3-macos-code-signing--notarization)
4. [Windows Code Signing](#4-windows-code-signing)
5. [GitHub Actions CI/CD](#5-github-actions-cicd)
6. [Auto-Update (tauri-plugin-updater)](#6-auto-update)
7. [Release Process](#7-release-process)

---

## 1. Local Build

### Build Command

```bash
pnpm tauri build
```

This runs the full pipeline:

1. `pnpm build` — TypeScript check + Vite production build → `dist/`
2. `cargo build --release` — Rust release binary
3. Bundler — wraps the binary into platform-specific installers

### Useful Flags

| Flag | Purpose | Example |
|------|---------|---------|
| `--debug` | Include debug symbols, enable DevTools | `pnpm tauri build --debug` |
| `--target` | Cross-compile to a specific target | `pnpm tauri build --target universal-apple-darwin` |
| `--no-bundle` | Skip installer creation (binary only) | `pnpm tauri build --no-bundle` |
| `-- --features custom-protocol` | Enable Tauri custom protocol | `pnpm tauri build -- --features custom-protocol` |

### Build Output Location

All artifacts are placed under:

```
src-tauri/target/release/bundle/
```

### Platform-Specific Artifacts

#### macOS

```
src-tauri/target/release/bundle/
├── macos/
│   └── ant-desktop.app          # Application bundle
└── dmg/
    └── ant-desktop_0.1.0_aarch64.dmg   # Disk image installer
```

For a **universal binary** (runs natively on both Apple Silicon and Intel):

```bash
# Install both targets first (one-time)
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin

# Build universal
pnpm tauri build --target universal-apple-darwin
```

Output: `ant-desktop_0.1.0_universal.dmg`

#### Windows

```
src-tauri/target/release/bundle/
├── msi/
│   └── ant-desktop_0.1.0_x64_en-US.msi   # WiX installer
└── nsis/
    └── ant-desktop_0.1.0_x64-setup.exe    # NSIS installer
```

To build only one installer type, set in `tauri.conf.json`:

```json
{
  "bundle": {
    "targets": ["msi"]
  }
}
```

#### Linux

```
src-tauri/target/release/bundle/
├── deb/
│   └── ant-desktop_0.1.0_amd64.deb    # Debian package
└── appimage/
    └── ant-desktop_0.1.0_amd64.AppImage  # Portable
```

---

## 2. Distributing Without Code Signing

When you don't have a signing certificate yet (development, internal testing), the OS will block unsigned apps. Here's how to work around it.

### macOS — Gatekeeper Bypass

macOS quarantines downloaded apps. Three ways to open an unsigned app:

**Option A: Right-click → Open (simplest)**

1. Right-click (or Control-click) `ant-desktop.app`
2. Select "Open" from the context menu
3. Click "Open" in the warning dialog

**Option B: Remove quarantine attribute**

```bash
xattr -cr /path/to/ant-desktop.app
```

Then double-click to open normally.

**Option C: System Settings**

1. Try to open the app (it will be blocked)
2. Go to System Settings → Privacy & Security
3. Scroll down — you'll see "ant-desktop was blocked"
4. Click "Open Anyway"

### Windows — SmartScreen Bypass

1. When SmartScreen appears, click "More info"
2. Click "Run anyway"

### Why This Won't Work for Production

- Users won't trust an app that triggers security warnings
- macOS may completely block unsigned apps in future versions
- IT departments often enforce policies that prevent unsigned installs
- App stores require signing

**For anything beyond internal testing, you need code signing (sections 3 & 4).**

---

## 3. macOS Code Signing + Notarization

Two steps: **sign** (proves it's from you) and **notarize** (Apple scans for malware and approves it).

### 3.1 Prerequisites

1. **Apple Developer Program** membership ($99/year)
   - Enroll at https://developer.apple.com/programs/
   - Takes 24-48 hours to activate

2. **Developer ID Application certificate**
   - Open Xcode → Settings → Accounts → Manage Certificates
   - Click "+" → "Developer ID Application"
   - Or use the Apple Developer portal: Certificates, Identifiers & Profiles → Create

3. **Verify certificate is installed:**

```bash
security find-identity -v -p codesigning
```

You should see something like:

```
1) ABCDEF1234... "Developer ID Application: Your Name (TEAMID)"
```

### 3.2 Tauri Configuration

Add to `tauri.conf.json`:

```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Developer ID Application: Your Name (TEAMID)"
    }
  }
}
```

Or set via environment variable (preferred for CI):

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
```

### 3.3 Notarization

Notarization submits the app to Apple's automated security scan. Without it, Gatekeeper will still block the app on other Macs.

**Method A: Apple ID + App-Specific Password (simpler)**

```bash
export APPLE_ID="your@apple.id"
export APPLE_PASSWORD="xxxx-xxxx-xxxx-xxxx"   # App-Specific Password
export APPLE_TEAM_ID="XXXXXXXXXX"
```

Generate an App-Specific Password at https://appleid.apple.com → Sign-In and Security → App-Specific Passwords.

**Method B: App Store Connect API Key (better for CI)**

1. Go to https://appstoreconnect.apple.com → Users and Access → Integrations → Keys
2. Generate a new key with "Developer" access
3. Download the `.p8` file

```bash
export APPLE_API_KEY="XXXXXXXXXX"          # Key ID
export APPLE_API_ISSUER="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
export APPLE_API_KEY_PATH="/path/to/AuthKey_XXXXXXXXXX.p8"
```

### 3.4 Build with Signing + Notarization

With environment variables set, just run the normal build:

```bash
pnpm tauri build
```

Tauri automatically:

1. Signs the `.app` with your Developer ID
2. Submits to Apple for notarization
3. Waits for approval (typically 2-10 minutes)
4. Staples the notarization ticket to the `.app`
5. Creates the `.dmg`

You can verify:

```bash
# Check signing
codesign --verify --deep --strict ant-desktop.app

# Check notarization
spctl --assess --type exec ant-desktop.app
# Expected: "ant-desktop.app: accepted"
```

---

## 4. Windows Code Signing

### 4.1 Get a Certificate

Purchase an **OV (Organization Validation)** or **EV (Extended Validation)** code signing certificate from a trusted CA:

- DigiCert, Sectigo, GlobalSign, SSL.com
- OV: ~$200-400/year (may show SmartScreen warning for a few weeks)
- EV: ~$300-600/year (immediate SmartScreen trust, but requires USB token)

### 4.2 Export as PFX

If you received separate `.crt` and `.key` files:

```bash
openssl pkcs12 -export -out certificate.pfx \
  -inkey private.key -in certificate.crt \
  -certfile ca-chain.crt
```

### 4.3 Configure Tauri

**Option A: Certificate thumbprint (for certificates installed in Windows cert store)**

```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

**Option B: PFX file (for CI)**

```bash
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your-pfx-password"
```

```json
{
  "bundle": {
    "windows": {
      "certificate": "path/to/certificate.pfx",
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

### 4.4 Azure Trusted Signing (Alternative)

Microsoft's cloud-based signing service — no USB token needed:

```json
{
  "bundle": {
    "windows": {
      "signCommand": "AzureSignTool sign -kvu https://your-vault.vault.azure.net -kvc your-cert -tr http://timestamp.digicert.com -td sha256 %1"
    }
  }
}
```

---

## 5. GitHub Actions CI/CD

### 5.1 Workflow File

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: macos-latest
            args: '--target aarch64-apple-darwin'
          - platform: macos-latest
            args: '--target x86_64-apple-darwin'
          - platform: ubuntu-22.04
            args: ''
          - platform: windows-latest
            args: ''

    runs-on: ${{ matrix.platform }}

    steps:
      - uses: actions/checkout@v4

      - name: Install pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 9

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }}

      - name: Install Linux dependencies
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libappindicator3-dev \
            librsvg2-dev \
            patchelf

      - name: Install frontend dependencies
        run: pnpm install

      - name: Build and release
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          # macOS signing (remove if not yet set up)
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
          # Windows signing (remove if not yet set up)
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'ant-desktop ${{ github.ref_name }}'
          releaseBody: 'See the assets below to download and install.'
          releaseDraft: true
          prerelease: false
          args: ${{ matrix.args }}
```

### 5.2 Required GitHub Secrets

Go to Repository → Settings → Secrets and variables → Actions → New repository secret:

| Secret | Purpose | Required When |
|--------|---------|---------------|
| `APPLE_SIGNING_IDENTITY` | `"Developer ID Application: ..."` | macOS signing |
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` file | macOS signing |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` | macOS signing |
| `APPLE_ID` | Apple ID email | macOS notarization (Method A) |
| `APPLE_PASSWORD` | App-specific password | macOS notarization (Method A) |
| `APPLE_TEAM_ID` | 10-char Team ID | macOS notarization |
| `WINDOWS_CERTIFICATE_PASSWORD` | PFX password | Windows signing |

**To export your macOS certificate as base64 for CI:**

```bash
# Export from Keychain to .p12
security export -k ~/Library/Keychains/login.keychain-db \
  -t identities -f pkcs12 -o certificate.p12

# Encode as base64
base64 -i certificate.p12 -o certificate-base64.txt

# Copy the content of certificate-base64.txt into APPLE_CERTIFICATE secret
```

### 5.3 Triggering a Release

```bash
# 1. Update version in package.json, Cargo.toml, tauri.conf.json
# 2. Commit
git add -A && git commit -m "release: v0.2.0"

# 3. Tag and push
git tag v0.2.0
git push origin main --tags
```

The workflow:
1. Builds on macOS (ARM + Intel), Windows, Linux in parallel
2. Signs binaries (if secrets are configured)
3. Creates a **draft** GitHub Release with all installers attached
4. You review the draft, edit release notes, and click "Publish"

### 5.4 Without Signing (CI only builds)

If you haven't set up signing yet, remove the signing env vars from the workflow. The build will still produce unsigned installers — useful for internal testing.

---

## 6. Auto-Update

`tauri-plugin-updater` lets the app check for new versions and update itself.

### 6.1 Generate Update Signing Keys

This is **separate from code signing** — it ensures updates come from you, not a MITM.

```bash
pnpm tauri signer generate -w ~/.tauri/ant-desktop.key
```

This creates:
- `~/.tauri/ant-desktop.key` — private key (keep secret)
- `~/.tauri/ant-desktop.key.pub` — public key (embed in app)

You'll be prompted for a password. Remember it.

### 6.2 Configure tauri.conf.json

```json
{
  "bundle": {
    "createUpdaterArtifacts": "v2Compatible"
  },
  "plugins": {
    "updater": {
      "pubkey": "CONTENTS_OF_ant-desktop.key.pub",
      "endpoints": [
        "https://github.com/YOUR_ORG/ant-desktop/releases/latest/download/latest.json"
      ]
    }
  }
}
```

### 6.3 Add CI Environment Variables

Add to the GitHub Actions workflow:

```yaml
env:
  TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_KEY_PASSWORD }}
```

Add to GitHub Secrets:
- `TAURI_SIGNING_PRIVATE_KEY`: contents of `~/.tauri/ant-desktop.key`
- `TAURI_SIGNING_KEY_PASSWORD`: the password you chose

### 6.4 How Auto-Update Works

When `createUpdaterArtifacts` is enabled, `pnpm tauri build` produces an additional `latest.json` file alongside the installer. `tauri-action` uploads this to the GitHub Release.

The update flow:

1. App starts → checks the endpoint URL for `latest.json`
2. Compares `latest.json` version with current version
3. If newer, downloads the update artifact (`.tar.gz` on macOS, `.nsis.zip` on Windows)
4. Verifies the update signature against the embedded public key
5. Applies the update and restarts

### 6.5 Add Updater Plugin to Rust

```bash
cd src-tauri
cargo add tauri-plugin-updater@2
```

Register in `lib.rs`:

```rust
.plugin(tauri_plugin_updater::Builder::new().build())
```

Frontend check (optional — for a manual "Check for updates" button):

```typescript
import { check } from '@tauri-apps/plugin-updater';

const update = await check();
if (update) {
  await update.downloadAndInstall();
  await relaunch();
}
```

---

## 7. Release Process

### 7.1 Version Bump

Three files must have matching versions:

```bash
# Check current versions
grep '"version"' package.json src-tauri/tauri.conf.json
grep '^version' src-tauri/Cargo.toml
```

Update all three to the new version (e.g., `0.2.0`).

### 7.2 Release via CI (Recommended)

```bash
git add -A && git commit -m "release: vX.Y.Z"
git tag vX.Y.Z
git push origin main --tags
```

GitHub Actions builds all platforms, creates a **draft** release with installers attached. Review the draft on GitHub and click "Publish".

### 7.3 Release via Manual Build

```bash
pnpm install && pnpm tauri build
```

Upload artifacts from `src-tauri/target/release/bundle/` to a new GitHub Release manually.

### 7.4 Post-Release Verification

```
Pre-release:
  [ ] Version bumped in package.json, Cargo.toml, tauri.conf.json (all match)
  [ ] cargo test passes
  [ ] pnpm tauri build succeeds locally

Release:
  [ ] git tag + push triggers CI (or manual upload)
  [ ] Draft release has all platform artifacts

Test:
  [ ] macOS: .dmg install → tray icon appears
  [ ] Windows: .msi/.exe install → app runs
  [ ] Linux: .deb/.AppImage runs
  [ ] Deep link: ant-desktop://connect?token=...&server=...
  [ ] WebSocket connects to Ant Cloud
  [ ] Figma MCP relay works (with Figma Desktop running)

Post:
  [ ] Publish release on GitHub
  [ ] Auto-update works from previous version (if configured)
  [ ] Announce to team
```

### 7.5 Unsigned Build Notes (Internal Testing)

macOS users must bypass Gatekeeper for unsigned builds (see [Section 2](#2-distributing-without-code-signing)).

---

## Quick Reference

| Task | Command |
|------|---------|
| Dev mode | `pnpm tauri dev` |
| Production build | `pnpm tauri build` |
| macOS universal | `pnpm tauri build --target universal-apple-darwin` |
| Debug build | `pnpm tauri build --debug` |
| Binary only (no installer) | `pnpm tauri build --no-bundle` |
| Generate updater keys | `pnpm tauri signer generate -w ~/.tauri/ant-desktop.key` |
| Tag a release | `git tag v0.1.0 && git push origin main --tags` |
| Verify macOS signing | `codesign --verify --deep --strict ant-desktop.app` |
| Verify notarization | `spctl --assess --type exec ant-desktop.app` |
