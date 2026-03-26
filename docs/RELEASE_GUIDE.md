# Ant Desktop Release Guide

데브옵스팀이 Ant Desktop의 GitHub Release를 수행하기 위한 step-by-step 가이드.

빌드/서명/번들링의 기술 상세는 [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md)를 참조.

---

## 목차

1. [사전 준비](#1-사전-준비)
2. [v0.1.0 초기 릴리스 (unsigned, 내부 배포)](#2-v010-초기-릴리스)
3. [CI/CD 자동 릴리스](#3-cicd-자동-릴리스)
4. [코드 서명 설정 (향후)](#4-코드-서명-설정)
5. [Auto-Update 설정 (향후)](#5-auto-update-설정)
6. [Troubleshooting](#6-troubleshooting)

---

## 1. 사전 준비

### 1.1 GitHub 저장소

- 조직 내에 `ant-desktop` 저장소가 생성되어 있어야 합니다.
- 저장소에 `main` 브랜치가 존재해야 합니다.

### 1.2 로컬 빌드 환경

| 도구 | 버전 | 설치 |
|------|------|------|
| Rust | stable latest | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 20+ | https://nodejs.org |
| pnpm | 9+ | `npm install -g pnpm` |
| Xcode CLI Tools | latest | `xcode-select --install` (macOS only) |

### 1.3 GitHub Actions Secrets (CI 사용 시)

Repository → Settings → Secrets and variables → Actions에 등록:

| Secret | 용도 | 필수 여부 |
|--------|------|----------|
| `GITHUB_TOKEN` | 자동 제공됨 (별도 설정 불필요) | 자동 |
| `APPLE_SIGNING_IDENTITY` | macOS 코드 서명 ID | 선택 (서명 시) |
| `APPLE_CERTIFICATE` | Base64 인코딩된 .p12 파일 | 선택 (서명 시) |
| `APPLE_CERTIFICATE_PASSWORD` | .p12 비밀번호 | 선택 (서명 시) |
| `APPLE_ID` | Apple ID 이메일 | 선택 (공증 시) |
| `APPLE_PASSWORD` | App-Specific Password | 선택 (공증 시) |
| `APPLE_TEAM_ID` | 10자리 Team ID | 선택 (공증 시) |
| `WINDOWS_CERTIFICATE_PASSWORD` | PFX 비밀번호 | 선택 (Windows 서명 시) |

> 코드 서명 없이도 빌드 및 릴리스는 가능합니다. 미설정 시 unsigned 빌드가 생성됩니다.

---

## 2. v0.1.0 초기 릴리스

코드 서명 없이 내부 테스트용으로 배포하는 방법입니다.

### 2.1 버전 확인

세 파일의 버전이 모두 `0.1.0`으로 일치하는지 확인:

```bash
grep '"version"' package.json src-tauri/tauri.conf.json
grep '^version' src-tauri/Cargo.toml
```

### 2.2 방법 A: 수동 빌드 + GitHub Release 업로드

#### Step 1: 로컬 빌드

```bash
pnpm install
pnpm tauri build
```

빌드 산출물 위치:

| 플랫폼 | 파일 | 경로 |
|--------|------|------|
| macOS (ARM) | `.dmg` | `src-tauri/target/release/bundle/dmg/ant-desktop_0.1.0_aarch64.dmg` |
| macOS (ARM) | `.app` | `src-tauri/target/release/bundle/macos/ant-desktop.app` |

macOS universal binary (ARM + Intel):

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm tauri build --target universal-apple-darwin
```

#### Step 2: GitHub Release 생성

1. GitHub 저장소 → Releases → "Draft a new release"
2. Tag: `v0.1.0` (새 태그 생성)
3. Title: `Ant Desktop v0.1.0`
4. Description:

```markdown
## Ant Desktop v0.1.0

Figma Desktop MCP와 Ant Cloud를 연결하는 데스크톱 브릿지 앱의 첫 릴리스입니다.

### 설치 방법 (macOS, unsigned)

1. `.dmg` 파일을 다운로드합니다.
2. DMG를 열고 `ant-desktop.app`을 Applications 폴더로 드래그합니다.
3. **최초 실행 시** (Gatekeeper 우회):
   - 터미널에서 `xattr -cr /Applications/ant-desktop.app` 실행
   - 또는: 앱을 우클릭 → "열기" → "열기" 클릭
4. 시스템 트레이에서 앱 아이콘을 확인합니다.

### 기능
- Figma Desktop MCP 연결 (localhost:3845)
- Ant Cloud WebSocket 브릿지
- 딥링크 인증 (ant-desktop://connect)
- 시스템 트레이 상태 표시
```

5. 빌드 산출물 파일(`.dmg`)을 업로드합니다.
6. "Publish release" 클릭

### 2.3 방법 B: CI/CD 자동 릴리스

```bash
git tag v0.1.0
git push origin main --tags
```

GitHub Actions가 자동으로:
1. macOS (ARM + Intel), Windows, Linux 빌드 실행
2. Draft Release 생성 및 산출물 첨부
3. Releases 페이지에서 Draft를 확인 후 "Publish" 클릭

---

## 3. CI/CD 자동 릴리스

### 3.1 워크플로우

`.github/workflows/release.yml`이 `v*` 태그 푸시 시 자동 실행됩니다.

빌드 매트릭스:

| Runner | Target | 산출물 |
|--------|--------|--------|
| `macos-latest` | `aarch64-apple-darwin` | `.dmg` (Apple Silicon) |
| `macos-latest` | `x86_64-apple-darwin` | `.dmg` (Intel) |
| `ubuntu-22.04` | default | `.deb`, `.AppImage` |
| `windows-latest` | default | `.msi`, `.exe` |

### 3.2 릴리스 프로세스

```bash
# 1. 버전 범프 (세 파일 모두)
#    - package.json
#    - src-tauri/Cargo.toml
#    - src-tauri/tauri.conf.json

# 2. 커밋
git add -A && git commit -m "release: vX.Y.Z"

# 3. 태그 + 푸시
git tag vX.Y.Z
git push origin main --tags
```

### 3.3 릴리스 후 확인

1. GitHub Actions → 워크플로우 실행 확인
2. Releases → Draft Release 확인
3. 각 플랫폼 산출물 다운로드 및 테스트:
   - macOS: `.dmg` 열기 → 앱 설치 → 트레이 아이콘 확인
   - Windows: `.msi` 또는 `.exe` 설치 → 앱 실행
   - Linux: `.deb` 설치 또는 `.AppImage` 실행
4. 딥링크 테스트: `ant-desktop://connect?token=test&server=http://127.0.0.1:4101`
5. "Publish" 클릭

---

## 4. 코드 서명 설정

> 현재 v0.1.0은 unsigned 배포입니다. 외부 배포 시 아래를 설정합니다.

### macOS

1. Apple Developer Program 가입 ($99/year)
2. "Developer ID Application" 인증서 생성
3. App-Specific Password 생성 (appleid.apple.com)
4. GitHub Secrets에 등록 (위 표 참조)

상세 절차: [DEPLOYMENT_GUIDE.md §3](./DEPLOYMENT_GUIDE.md#3-macos-code-signing--notarization)

### Windows

1. OV/EV 코드 서명 인증서 구매 (DigiCert, Sectigo 등)
2. PFX로 내보내기
3. GitHub Secrets에 등록

상세 절차: [DEPLOYMENT_GUIDE.md §4](./DEPLOYMENT_GUIDE.md#4-windows-code-signing)

---

## 5. Auto-Update 설정

> v0.1.0에서는 미적용. 향후 릴리스에서 설정합니다.

```bash
# 업데이트 서명 키 생성
pnpm tauri signer generate -w ~/.tauri/ant-desktop.key
```

1. `tauri.conf.json`에 `updater` 플러그인 설정 추가
2. `TAURI_SIGNING_PRIVATE_KEY` → GitHub Secrets 등록
3. 릴리스 시 `latest.json`이 자동 생성됨

상세 절차: [DEPLOYMENT_GUIDE.md §6](./DEPLOYMENT_GUIDE.md#6-auto-update)

---

## 6. Troubleshooting

### macOS에서 "손상되었기 때문에 열 수 없습니다" 오류

```bash
xattr -cr /Applications/ant-desktop.app
```

### macOS에서 앱이 Gatekeeper에 의해 차단됨

시스템 설정 → 개인 정보 보호 및 보안 → "ant-desktop이(가) 차단되었습니다" → "무시하고 열기"

### CI에서 macOS 빌드 실패: "No signing identity found"

코드 서명 Secrets가 미설정된 경우입니다. 서명 없이 빌드하려면 워크플로우에서 `APPLE_SIGNING_IDENTITY` 등 관련 환경 변수를 제거합니다.

### Rust 빌드 실패: "linker 'cc' not found" (Linux)

```bash
sudo apt-get install -y build-essential libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

### 버전 불일치 오류

세 파일의 버전이 일치하는지 확인:

```bash
grep '"version"' package.json src-tauri/tauri.conf.json
grep '^version' src-tauri/Cargo.toml
```

---

## 릴리스 체크리스트

```
사전:
  [ ] 버전 범프 (package.json, Cargo.toml, tauri.conf.json) — 세 파일 일치
  [ ] cargo test 통과 (cd src-tauri && cargo test)
  [ ] pnpm tauri build 로컬 빌드 성공

릴리스:
  [ ] git tag vX.Y.Z && git push origin main --tags
  [ ] CI 워크플로우 완료 (또는 수동 빌드 업로드)
  [ ] Draft Release에 산출물 첨부 확인

테스트:
  [ ] macOS: DMG 설치 → 트레이 아이콘 표시
  [ ] 딥링크: ant-desktop://connect?token=...&server=... 동작
  [ ] WebSocket: Ant Cloud 연결 성공
  [ ] Figma MCP: Figma Desktop 실행 시 프록시 동작

게시:
  [ ] Release 게시 (Publish)
  [ ] 팀 공지
```
