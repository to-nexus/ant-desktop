# ant-desktop Development Guide

## 문서 역할 (빠른 참고)

| 문서 | 용도 |
|------|------|
| [PRD.md](./PRD.md) | 범위·FR/NFR·사용자 플로우·MVP 판단 |
| [SYSTEM_DESIGN.md](./SYSTEM_DESIGN.md) | 아키텍처·모듈·프로토콜·상태·URL 규칙 — **일상 구현의 기준** |
| 이 파일 | 스캐폴딩·명령어·Phase·테스트·릴리즈 |

## 저장소 모델 및 레이어링

### 문서 소유권 원칙

이 `docs/` 디렉터리(**PRD.md**, **SYSTEM_DESIGN.md**, 본 파일)가 **ant-desktop 제품·아키텍처·개발 규칙의 정본(SSOT)** 이다.

- Ant 메인 레포의 구현 계획서(`.cursor/plans/...`)는 **Bridge 서버 측 구현 + Figma 파이프라인** 작업 순서 문서이며, Ant Desktop 제품 스펙을 바꾸려면 **여기 `docs/`를 먼저 수정**한 뒤 계획서 요약을 맞춘다.
- `@ant/shared/src/figma.ts`의 타입·상수가 **크로스 레포 프로토콜 계약**의 단일 소스이다. 필드 추가·변경 시 이 문서의 §6(PRD)·§8(SYSTEM_DESIGN)을 같이 갱신한다.

### 저장소 경계

| 저장소 | 성격 | 언어 | 빌드 |
|--------|------|------|------|
| **`ant`** (메인) | pnpm workspace 기반 **TypeScript 모노레포** — `ant-cli`, `ant-ui`, `ant-shared` 등 | TypeScript | pnpm + Vite + tsc |
| **`ant-desktop`** (여기) | **별도 Git 저장소** — Tauri 데스크톱 앱 | Rust + TypeScript(React) | Cargo + pnpm + Tauri Bundler |

Ant Desktop을 Ant 모노레포에 편입하지 않는 이유: 배포 채널(GitHub Releases 인스톨러 vs Docker/K8s), 코드 서명, Tauri 빌드 체인이 근본적으로 다르다.

### 이 레포는 모노레포인가?

**아니다.** Ant의 pnpm workspace처럼 **여러 npm 패키지**가 공존하는 모노레포가 아니다.

**한 저장소 안에 React(TypeScript)와 Rust가 함께** 들어가는 **Tauri 표준 단일 제품 레포**이다:

```
ant-desktop/
├── src/              # React 프론트엔드 (TS)
├── src-tauri/        # Rust 백엔드 (Tauri Core)
├── docs/             # 제품·설계·개발 문서 (정본)
├── package.json      # 프론트 의존성 + Tauri CLI
└── ...
```

이후 프론트가 복잡해지면 `packages/*`로 쪼갤 수 있으나, MVP에서는 위 구조를 유지한다.

### 레이어링 (코드 계층 규칙)

```
┌──────────────────────────────────────────────────┐
│              React WebView (src/)                │  UI 계층
│  pages/ components/ hooks/ lib/tauri.ts          │
└─────────────────────┬────────────────────────────┘
                      │ Tauri IPC (invoke / events)
                      │ ← 유일한 경계. UI는 이 아래를 직접 호출하지 않는다.
┌─────────────────────┴────────────────────────────┐
│        commands.rs  (#[tauri::command])           │  IPC 경계
│  UI가 호출하는 안전한 명령 단위 API               │
└─────────────────────┬────────────────────────────┘
                      │
┌─────────────────────┴────────────────────────────┐
│           Rust Core (src-tauri/src/)              │  Core 계층
│                                                  │
│  bridge/   ← WebSocket 클라이언트, 프로토콜       │
│  mcp/      ← Figma MCP HTTP 프록시               │
│  auth/     ← JWT·Keychain·딥링크                 │
│  health/   ← Figma Desktop 헬스체크              │
│  tray/     ← 시스템 트레이 아이콘·메뉴            │
│  state/    ← 앱 전역 상태 (Arc<Mutex<AppState>>) │
│  constants.rs  ← 상수                            │
└──────────────────────────────────────────────────┘
```

**규칙:**

| 규칙 | 설명 |
|------|------|
| **UI → Core 금지** | `src/` 코드는 `src-tauri/src/`의 Rust를 직접 import/호출할 수 없다. 반드시 `invoke()` 경유. |
| **시크릿은 Core 전용** | JWT·WebSocket 토큰·MCP 프록시 등 민감 로직은 Rust에만 둔다. |
| **Core 모듈 간 의존** | `bridge` → `mcp`, `bridge` → `auth`, `commands` → `state` 등 자유롭지만, **순환 의존 금지**. |
| **상태 단일 소스** | `AppState`(Arc<Mutex>)가 런타임 상태의 유일한 소스. UI는 `get_app_state` invoke + 이벤트 구독으로 조회. |
| **외부 I/O는 Core** | 네트워크(WebSocket, HTTP), 파일시스템(store), OS API(Keychain, 트레이)는 모두 Rust Core에서 수행. |

### Ant와의 계약

- JSON 메시지 스키마·상수 등 **크로스 레포 타입**은 Ant의 `packages/ant-shared`에서 정의된다. Ant Desktop은 **동일 계약을 Rust struct로 재구현**하고, JSON fixture round-trip 테스트로 호환성을 검증한다 (§7.3 참조).
- `@ant/shared`가 변경되면 이 레포의 `src-tauri/src/bridge/protocol.rs` + `tests/fixtures/` + 이 문서들을 같은 타이밍에 갱신한다.

---

## 코딩 컨벤션

이 프로젝트는 **소규모 Tauri 단일 제품**이다. 복잡한 아키텍처 패턴보다 **명확한 소유권, 짧은 모듈, 직접적인 에러 전파**를 우선한다.

### Rust 컨벤션

**네이밍:**
- Rust 표준을 따른다: `snake_case`(함수·변수), `CamelCase`(타입·struct·enum), `SCREAMING_SNAKE_CASE`(상수).
- 모듈 이름은 단수형 (`bridge`, `auth`, `health` — 복수형 `bridges` 아님).

**에러 처리:**
- **`thiserror`** 로 모듈별 에러 enum을 정의한다 (`BridgeError`, `McpError`, `AuthError` 등).
- 최상위 `error.rs`에 `AppError` enum을 두어 모듈 에러를 통합한다.
- `anyhow`는 쓰지 않는다 — 모든 에러가 타입으로 구분되어야 Tauri command에서 직렬화 가능하다.
- 함수 시그니처는 `Result<T, ModuleError>` (모듈 내부) 또는 `Result<T, AppError>` (command 경계).

```rust
// src-tauri/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Bridge(#[from] BridgeError),
    #[error(transparent)]
    Mcp(#[from] McpError),
    #[error(transparent)]
    Auth(#[from] AuthError),
}

// Tauri command에서 사용하려면 Serialize 필요
impl serde::Serialize for AppError { ... }
```

**모듈 캡슐화:**
- 각 모듈의 `mod.rs`에서 **pub으로 노출할 것만 re-export**한다. 내부 struct·함수는 `pub(crate)` 또는 private.
- 모듈 간 의존 방향: `bridge → mcp`, `bridge → auth`, `commands → state`, `commands → bridge`. **역방향 금지, 순환 금지.**

**비동기 태스크 구조:**
- 장기 실행 루프(WebSocket, heartbeat, health check)는 `lib.rs`의 Tauri `setup` 훅에서 `tokio::spawn`으로 시작한다.
- 태스크 취소는 `tokio_util::sync::CancellationToken`(또는 `tokio::sync::watch`)으로 관리한다. 앱 종료·연결 해제 시 토큰을 cancel하여 루프를 깨끗이 종료한다.
- 모듈 간 비동기 통신이 필요하면 `tokio::sync::mpsc` 채널을 쓴다 (예: bridge → mcp 요청 전달). 공유 상태 mutation은 `Arc<Mutex>` 경유.

```
setup() 에서 spawn:
  ├── bridge::run_loop(token, state, mcp_sender)   // WS 연결 + 수신 루프
  ├── bridge::heartbeat_loop(token, state)          // 30초 heartbeat
  └── health::check_loop(token, state, app_handle)  // 10초 Figma 헬스체크
```

**로깅:**
- `tracing` 매크로만 사용 (`tracing::info!`, `tracing::error!` 등). `println!`/`eprintln!` 금지.
- 구조화된 필드 사용: `tracing::info!(tool = %tool, duration_ms = elapsed, "MCP request completed")`.

**Clippy:**
- `cargo clippy -- -D warnings`를 CI 게이트로 건다. 경고를 에러로 취급.

### React/TypeScript 컨벤션

이 앱의 React UI는 **3페이지, 소수 컴포넌트**의 얇은 레이어이다. 과도한 추상화를 피한다.

**컴포넌트:**
- 함수형 컴포넌트만 사용. class 컴포넌트 금지.
- 파일 하나에 컴포넌트 하나. 파일명 = 컴포넌트명 (`StatusPage.tsx`).

**상태 관리:**
- 외부 상태 라이브러리(Zustand, Redux 등)를 쓰지 않는다.
- `useAppState` 커스텀 훅이 Tauri `invoke()` + `listen()` 이벤트로 Rust 상태를 구독·반영한다. React 쪽 "진짜 상태"는 없다 — Rust `AppState`가 단일 소스이다.
- 로컬 UI 상태(폼 입력 등)만 `useState`로 관리.

**스타일링:**
- Tailwind CSS utility-first. CSS 파일·CSS Modules·styled-components 사용하지 않는다.
- 공통 색상·간격은 `tailwind.config.js`에 테마로 정의.

**타입:**
- `strict: true`. `any` 금지, `as` 캐스팅 최소화.
- Tauri IPC 커맨드의 인자·반환 타입은 `lib/tauri.ts`에 한 곳에서 정의.

---

## 1. Prerequisites

### 1.1 Required Tools

| Tool | Version | Installation |
|------|---------|-------------|
| Rust | stable (latest) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 20+ | https://nodejs.org or `nvm install 20` |
| pnpm | 9+ | `npm install -g pnpm` |
| Figma Desktop | latest | https://www.figma.com/downloads |

### 1.2 Platform-Specific Dependencies

**macOS:**
- Xcode Command Line Tools: `xcode-select --install`
- 추가 설치 불필요 (WebKit은 macOS 내장)

**Windows:**
- Visual Studio Build Tools (C++ workload)
- WebView2 (Windows 10/11에 기본 포함, 구버전은 수동 설치)
- https://tauri.app/start/prerequisites/#windows

**Linux:**
- 시스템 패키지:
  ```bash
  # Ubuntu/Debian
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

  # Fedora
  sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
    libappindicator-gtk3-devel librsvg2-devel
  ```
- https://tauri.app/start/prerequisites/#linux

### 1.3 Verify Installation

```bash
rustc --version      # rustc 1.XX.0
cargo --version      # cargo 1.XX.0
node --version       # v20.X.X
pnpm --version       # 9.X.X
```


## 2. Project Setup

### 2.1 Scaffolding

Tauri v2 + React + TypeScript 프로젝트를 생성한다:

```bash
cd /path/to/ant-desktop
pnpm create tauri-app@latest . -- --template react-ts
```

**비어 있지 않은 레포(`docs/`, `.git` 등이 이미 있을 때):** `create-tauri-app`이 현재 디렉터리 거부 시 다음 중 하나를 택한다.

- 상위 폴더에 임시 이름으로 프로젝트를 만든 뒤 `src/`, `src-tauri/`, 설정 파일만 이 레포로 옮긴다.
- 또는 CLI가 `--force`를 지원하는 버전이면 `pnpm create tauri-app@latest . -- --template react-ts --force` (실행 전 `create-tauri-app --help`로 옵션 확인).

생성 후 추가 설정:

```bash
# Tailwind CSS 추가
pnpm add -D tailwindcss @tailwindcss/vite

# Tauri 플러그인 설치 (Rust + JS 양쪽 모두)
pnpm add @tauri-apps/plugin-autostart \
         @tauri-apps/plugin-updater \
         @tauri-apps/plugin-store \
         @tauri-apps/plugin-deep-link \
         @tauri-apps/plugin-single-instance \
         @tauri-apps/plugin-log

# Rust 의존성은 Cargo.toml에 직접 추가 (아래 참조)
```

### 2.2 Directory Structure

```
ant-desktop/
├── docs/                          # 문서 (PRD, System Design, 이 파일)
├── src/                           # React 프론트엔드
│   ├── App.tsx
│   ├── main.tsx
│   ├── pages/
│   │   ├── StatusPage.tsx
│   │   ├── SettingsPage.tsx
│   │   └── LogsPage.tsx
│   ├── components/
│   │   ├── StatusIndicator.tsx
│   │   └── ConnectionCard.tsx
│   ├── hooks/
│   │   ├── useAppState.ts
│   │   └── useAutostart.ts
│   └── lib/
│       └── tauri.ts
├── src-tauri/                     # Rust 백엔드 (Tauri Core)
│   ├── Cargo.toml
│   ├── tauri.conf.json            # Tauri 설정 (윈도우, 권한, 번들링)
│   ├── capabilities/              # Tauri v2 권한 설정
│   │   └── default.json
│   ├── icons/                     # 앱 아이콘 + 트레이 아이콘
│   │   ├── icon.png
│   │   ├── tray-connected.png
│   │   ├── tray-warning.png
│   │   ├── tray-error.png
│   │   └── tray-inactive.png
│   ├── src/
│   │   ├── main.rs               # 최소 진입점 (lib::run 호출)
│   │   ├── lib.rs                 # Tauri 빌더, 플러그인, 태스크 spawn
│   │   ├── error.rs               # AppError 통합 에러 enum
│   │   ├── constants.rs
│   │   ├── commands.rs
│   │   ├── bridge/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs
│   │   │   └── protocol.rs
│   │   ├── mcp/
│   │   │   ├── mod.rs
│   │   │   └── proxy.rs
│   │   ├── auth/
│   │   │   ├── mod.rs
│   │   │   ├── jwt.rs
│   │   │   ├── keychain.rs
│   │   │   └── deeplink.rs
│   │   ├── health/
│   │   │   ├── mod.rs
│   │   │   └── figma_check.rs
│   │   ├── tray/
│   │   │   ├── mod.rs
│   │   │   └── menu.rs
│   │   └── state/
│   │       ├── mod.rs
│   │       └── app_state.rs
│   └── tests/                     # Rust 통합 테스트 (src/ 밖)
│       ├── bridge_protocol.rs     # fixture 기반 serde round-trip
│       └── fixtures/
│           └── bridge_messages.json
├── package.json
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.js
└── README.md
```

### 2.3 Key Configuration Files

**src-tauri/Cargo.toml** — Rust 의존성:

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-autostart = "2"
tauri-plugin-updater = "2"
tauri-plugin-store = "2"
tauri-plugin-deep-link = "2"
tauri-plugin-single-instance = "2"
tauri-plugin-log = "2"

tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-webpki-roots"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

> crate 버전은 scaffolding 시점 기준 최신으로 조정한다.

**src-tauri/tauri.conf.json** — 핵심 설정 항목:

```json
{
  "productName": "ant-desktop",
  "identifier": "com.ant.desktop",
  "app": {
    "windows": [
      {
        "title": "ant-desktop",
        "width": 480,
        "height": 600,
        "resizable": false,
        "visible": false
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/icon.png"]
  },
  "plugins": {
    "deep-link": {
      "desktop": {
        "schemes": ["ant-desktop"]
      }
    }
  }
}
```

> `"visible": false` — 앱 시작 시 윈도우를 숨기고 트레이만 표시한다.


## 3. Development Workflow

### 3.1 Commands

```bash
# 개발 서버 (Rust + Vite HMR)
pnpm tauri dev

# 프론트엔드만 개발 (Rust 재컴파일 없이 UI 수정)
pnpm dev

# 프로덕션 빌드 (현재 플랫폼용 인스톨러 생성)
pnpm tauri build

# Rust 코드만 타입 체크
cd src-tauri && cargo check

# Rust 테스트
cd src-tauri && cargo test

# Rust 린트
cd src-tauri && cargo clippy
```

### 3.2 Development Tips

**Rust 변경 시:** `pnpm tauri dev`가 자동으로 Rust를 재컴파일하지만, 초기 빌드는 1-2분 소요. 이후 증분 빌드는 빠름.

**React 변경 시:** Vite HMR이 즉시 반영.

**딥링크 테스트:**
```bash
# macOS — 클라우드 프리셋과 동일한 베이스
open "ant-desktop://connect?token=test-jwt&server=https://ant.crosstoken.io"

# 로컬 realtime (ant-cli 기본 realtime 포트 4101)
open "ant-desktop://connect?token=test-jwt&server=http://127.0.0.1:4101"
```

`server`는 `SYSTEM_DESIGN.md` **§2.5 서버 베이스 URL**의 **`realtime_base_url`** 과 동일 형식(HTTP(S) 오리진, Bridge는 `/bridge/ws`)이어야 한다. API 포트(4100)가 아니라 **realtime 포트(기본 4101)** 를 쓴다. 앱 설정 화면에서도 같은 값을 수동·프리셋으로 바꿀 수 있다.

**Figma Desktop MCP 테스트:**
```bash
# Figma Desktop이 실행 중일 때 MCP 서버 확인
curl -X POST http://127.0.0.1:3845/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":"test","method":"tools/list","params":{}}'
```

### 3.3 Debugging

- **Rust 로그**: `RUST_LOG=debug pnpm tauri dev`로 상세 로그 활성화
- **WebView DevTools**: `pnpm tauri dev`에서 자동 활성화 (프로덕션에서는 비활성화)
- **트레이 이벤트**: Rust 쪽 `tracing` 로그로 확인


## 4. Implementation Roadmap

### Phase 1: Scaffolding + Tray App (1주)

**목표:** 빈 앱이 시스템 트레이에 상주하고, 설정 윈도우를 열 수 있는 상태.

**작업:**
1. Tauri v2 + React 프로젝트 생성 (`pnpm create tauri-app`)
2. Tailwind CSS 설정
3. `tray-icon` 설정: 트레이 아이콘 + 메뉴 (설정/종료)
4. 설정 윈도우 기본 레이아웃 (StatusPage 껍데기)
5. `tauri-plugin-single-instance` 설정
6. `tauri-plugin-log` 설정
7. `AppState` 기본 구조 구현
8. macOS/Windows 빌드 확인

**완료 조건:**
- `pnpm tauri dev` → 트레이 아이콘 표시
- 트레이 메뉴에서 "설정" → 윈도우 열림
- "종료" → 앱 종료

### Phase 2: WebSocket + Bridge Protocol (1-2주)

**목표:** Ant Cloud에 WebSocket으로 연결하고 Bridge 프로토콜 메시지를 주고받는다.

**작업:**
1. `bridge/protocol.rs` — `BridgeMessage` enum + serde 직렬화
2. `bridge/client.rs` — WebSocket 연결, 메시지 송수신
3. `BridgeRegisterMessage` 전송 로직
4. `BridgeHeartbeatMessage` 30초 타이머
5. 지수 백오프 재연결
6. `MCPRequestMessage` 수신 핸들링 (로깅만, 프록시는 Phase 3)
7. 트레이 아이콘 상태 연동 (연결됨/끊김)
8. `commands.rs` — `get_app_state`, `disconnect`

**선행 조건:**
- ant-cli에 `/bridge/ws` WebSocket 엔드포인트가 구현되어 있어야 함
- 없으면 로컬 모크 WebSocket 서버로 테스트

**완료 조건:**
- 하드코딩된 JWT + 서버 URL로 WebSocket 연결 성공
- heartbeat 30초 간격 전송
- 연결 끊기면 재연결 시도
- 트레이 아이콘 색상 변경

### Phase 3: MCP Proxy (1주)

**목표:** Cloud의 MCP 요청을 Figma Desktop으로 중계하고 응답을 돌려보낸다.

**작업:**
1. `mcp/proxy.rs` — `MCPRequestMessage` → JSON-RPC → `MCPResponseMessage`
2. `health/figma_check.rs` — localhost:3845 healthcheck
3. Figma 미실행 시 에러 응답 생성
4. 요청 타임아웃 (30초) 처리
5. 트레이 아이콘: Figma 상태 반영 (주황색)
6. `mcp_request_count`, `last_mcp_request` 통계

**테스트 방법:**
- Figma Desktop 실행 + 모크 WebSocket에서 `MCPRequestMessage` 전송
- 또는 Rust 단위 테스트로 JSON-RPC 변환 검증

**완료 조건:**
- `get_metadata` 요청 → Figma MCP 응답 → Cloud로 전달 성공
- Figma 미실행 시 에러 응답 반환
- 트레이 아이콘이 Figma 상태 반영

### Phase 4: Deep Link Auth + Settings UI (1주)

**목표:** ant-ui에서 딥링크로 인증하고, 설정 UI에서 **서버 베이스 URL(프리셋 + 수동)** 과 상태를 확인한다.

**작업:**
1. `tauri-plugin-deep-link` 설정
2. `auth/deeplink.rs` — URL 파싱, JWT 추출, `server` → store `realtime_base_url`
3. `auth/keychain.rs` — OS Keychain 읽기/쓰기
4. 딥링크 수신 → JWT·베이스 URL 저장 → WebSocket 연결 자동 시작
5. StatusPage 구현 (연결 상태, Figma 상태, 통계)
6. SettingsPage 구현: **realtime 베이스 URL 입력**, **「클라우드」**(`https://ant.crosstoken.io`), **「로컬」**(`http://127.0.0.1` + 포트 기본 4101), 저장·재연결, 연결 해제
7. LogsPage 기본 구현 (최근 이벤트)
8. (주의) 프리셋 라벨은 **UI 표시용**이며 `ANT_SERVER_MODE` 등을 탐지하지 않음

**테스트 방법:**
- `open "ant-desktop://connect?token=test&server=http://127.0.0.1:4101"` (macOS)
- 설정만으로 URL 변경 후 재연결
- Keychain에 JWT 저장 확인
- 앱 재시작 후 store+Keychain에서 복원하여 자동 연결

**완료 조건:**
- 딥링크로 JWT 수신 → 자동 연결
- 설정에서 프리셋·수동 URL 변경 후 재연결
- 설정 윈도우에서 연결 상태 확인
- "연결 해제" → JWT 삭제 + WebSocket 종료

### Phase 5: Auto-Start + Auto-Update (1주)

**목표:** OS 로그인 시 자동 시작, 새 버전 자동 업데이트.

**작업:**
1. `tauri-plugin-autostart` 설정
2. SettingsPage에 자동 시작 토글 추가
3. `tauri-plugin-updater` 설정
4. 업데이트 확인 로직 (시작 시 + 24시간 간격)
5. 업데이트 다운로드 + 설치 UI

**완료 조건:**
- OS 재부팅 → 앱 자동 시작 → 트레이 표시 → 자동 연결
- 업데이트 서버에 새 버전 배포 시 알림 표시

### Phase 6: CI/CD + Release (1-2주)

**목표:** GitHub Actions로 3 플랫폼 빌드 + 릴리즈 자동화.

**작업:**
1. GitHub Actions 워크플로우:
   - macOS (Apple Silicon + Intel universal)
   - Windows (x64)
   - Linux (x64)
2. Tauri 서명 설정 (macOS: codesign, Windows: signtool)
3. GitHub Releases에 인스톨러 업로드
4. 업데이트 서버 설정 (GitHub Releases 기반 또는 S3)
5. README.md 작성
6. 다운로드 페이지 (또는 GitHub Releases 링크)

**완료 조건:**
- `git tag v0.1.0 && git push --tags` → 3 플랫폼 인스톨러 자동 빌드
- 다운로드 페이지에서 플랫폼별 인스톨러 제공


## 5. Testing Strategy

### 5.1 Rust Unit Tests

```bash
cd src-tauri && cargo test
```

**테스트 대상:**
- `bridge/protocol.rs` — BridgeMessage 직렬화/역직렬화. TypeScript 타입과 JSON 호환성
- `mcp/proxy.rs` — MCPRequest → JSON-RPC 변환, JSON-RPC → MCPResponse 변환
- `auth/deeplink.rs` — 딥링크 URL 파싱
- `state/app_state.rs` — 상태 전환 로직

### 5.2 Integration Tests

**WebSocket 통합 테스트:**
- 모크 WebSocket 서버를 Rust 테스트에서 구동
- 연결 → 등록 → heartbeat → MCP 요청/응답 → 재연결 전체 흐름 테스트

**MCP 프록시 통합 테스트:**
- Figma Desktop이 없는 환경에서는 모크 HTTP 서버로 대체
- CI에서는 모크 서버 사용, 로컬에서는 실제 Figma Desktop으로 수동 테스트

### 5.3 Manual Test Checklist

각 릴리즈 전 수행:

- [ ] macOS: 설치 → 트레이 표시 → 딥링크 인증 → 연결 → MCP 릴레이
- [ ] Windows: 동일
- [ ] Linux: 동일
- [ ] Figma Desktop 종료 → 트레이 주황색 → 재실행 → 초록색 복구
- [ ] WebSocket 끊김 → 빨간색 → 자동 재연결 → 초록색 복구
- [ ] OS 재부팅 → 자동 시작 → 자동 연결
- [ ] 업데이트 설치 → 정상 동작


## 6. Release & Distribution

### 6.1 Versioning

Semantic Versioning: `MAJOR.MINOR.PATCH`
- 0.x.x — pre-release (Phase 1-5)
- 1.0.0 — 첫 안정 릴리즈 (Phase 6 완료)

### 6.2 Distribution Channels

| 플랫폼 | 인스톨러 형식 | 배포 |
|--------|-------------|------|
| macOS | `.dmg` (universal: ARM + x64) | GitHub Releases |
| Windows | `.msi` + `.exe` (x64) | GitHub Releases |
| Linux | `.AppImage` + `.deb` (x64) | GitHub Releases |

### 6.3 Code Signing

**macOS:**
- Apple Developer ID 인증서로 서명
- Notarization (Apple 공증) 필요 (미서명 시 Gatekeeper 차단)
- CI에서 `tauri-action`이 자동 처리

**Windows:**
- EV Code Signing Certificate 또는 Standard Certificate
- SmartScreen 경고 방지를 위해 서명 권장
- 초기에는 미서명으로 시작, 사용자 수 증가 시 서명 추가

### 6.4 Auto-Update Flow

```
앱 시작
  │
  ▼
업데이트 확인 (GitHub Releases API)
  │
  ├── 새 버전 없음 → 정상 진행
  │
  └── 새 버전 있음 → 트레이 알림
                       │
                       ▼
                  사용자 "업데이트" 클릭
                       │
                       ▼
                  다운로드 + 설치 + 재시작
```

Tauri의 `tauri-plugin-updater`가 플랫폼별 업데이트 메커니즘을 자동으로 처리한다. 업데이트 매니페스트는 GitHub Releases의 latest release 정보를 사용한다.


## 7. Relationship to Other Projects

### 7.1 ant-cli 의존성

ant-desktop이 작동하려면 ant-cli 측에 다음이 구현되어 있어야 한다:

| ant-cli 구현 | 용도 | Phase 연관 |
|-------------|------|-----------|
| `POST /api/auth/desktop-token` | 장기 JWT 발급 엔드포인트 | Phase 4 |
| JWT 미들웨어: Authorization 헤더 지원 | WebSocket 인증 | Phase 2 |
| `/bridge/ws` WebSocket 엔드포인트 | Bridge 연결 수신 | Phase 2 |
| Bridge Session Manager (Redis) | 세션 등록/관리 | Phase 2 |
| MCP 요청 라우팅 (Design Job → Bridge) | MCP 릴레이 트리거 | Phase 3 |

### 7.2 ant-ui 의존성

| ant-ui 구현 | 용도 | Phase 연관 |
|------------|------|-----------|
| "컴패니언 앱 연결" 버튼 + 딥링크 생성 | 인증 트리거 | Phase 4 |
| Bridge 상태 폴링 (`GET /api/figma/bridge/status`) | Tier 2 상태 표시 | Phase 4 |
| 다운로드 페이지 링크 | 앱 배포 | Phase 6 |

### 7.3 @ant/shared 계약

`packages/ant-shared/src/figma.ts`가 유일한 프로토콜 계약 소스이다. ant-desktop의 Rust 코드는 이 파일의 TypeScript 타입과 JSON 직렬화가 완벽히 호환되어야 한다.

**동기화 절차 (권장):**

1. **공유 JSON fixture:** `ant-desktop/src-tauri/tests/fixtures/bridge_messages.json` (또는 동등 경로)에 각 메시지 타입별 **골든 샘플** JSON을 둔다. 소스는 TypeScript에서 `JSON.stringify`로 생성하거나 수동으로 `figma.ts` 주석과 맞춘다.
2. **Rust round-trip 테스트:** `serde_json::from_str::<BridgeMessage>(fixture)` → 다시 `to_string` → 파싱이 깨지지 않는지, 키 이름이 camelCase인지 검증한다.
3. **선택:** 소규모 Node 스크립트로 `figma.ts`에서 샘플 객체를 export해 JSON 파일을 덤프하고, CI에서 Rust 테스트가 그 파일을 읽게 하면 TS↔Rust 드리프트를 한 번에 잡을 수 있다.
4. `@ant/shared/src/figma.ts` 수정 시 반드시 fixture와 Rust struct를 같은 PR에서 갱신한다.

프로토콜 변경 시 체크리스트:
1. `@ant/shared/src/figma.ts` 수정 (TypeScript)
2. `ant-desktop/src-tauri/src/bridge/protocol.rs` 동기화 (Rust)
3. fixture + `cargo test`로 호환성 검증
