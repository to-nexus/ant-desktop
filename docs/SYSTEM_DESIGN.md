# ant-desktop System Design

## 1. Technology Stack

| Layer | Technology | Version | Purpose |
|-------|-----------|---------|---------|
| Framework | Tauri | v2 | 크로스 플랫폼 데스크톱 앱 |
| Core (Native) | Rust | stable | WebSocket, HTTP 프록시, 시스템 통합 |
| UI | React + TypeScript | React 18+ | 설정 윈도우 |
| Styling | Tailwind CSS | v3+ | 최소 UI 스타일링 |
| Build | Vite | v5+ | 프론트엔드 번들링 |
| Package | Tauri Bundler | 내장 | 플랫폼별 인스톨러 생성 |

### Tauri Plugins

| Plugin | Purpose |
|--------|---------|
| `tray-icon` (빌트인) | 시스템 트레이 아이콘 + 메뉴 |
| `tauri-plugin-autostart` | OS 로그인 시 자동 실행 |
| `tauri-plugin-updater` | 자동 업데이트 |
| `tauri-plugin-store` | 로컬 설정 파일 (`realtime_base_url` 등) |
| `tauri-plugin-deep-link` | 커스텀 URL 스킴 (`ant-desktop://`) |
| `tauri-plugin-single-instance` | 중복 실행 방지 |
| `tauri-plugin-log` | 구조화된 로깅 |

### Rust Crates (Core Dependencies)

| Crate | Purpose |
|-------|---------|
| `tokio` | 비동기 런타임 |
| `tokio-tungstenite` | WebSocket 클라이언트 |
| `reqwest` | HTTP 클라이언트 (MCP 프록시) |
| `serde` / `serde_json` | JSON 직렬화/역직렬화 |
| `keyring` | OS Keychain 접근 (JWT 저장) |
| `uuid` | 요청 ID 생성 |
| `tracing` | 구조화된 로깅 |


## 2. Architecture Overview

### 2.1 Tauri Process Model

Tauri 앱은 두 프로세스로 구성된다:

```
┌─────────────────────────────────────────────────────┐
│ ant-desktop                                       │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │ Core Process (Rust)                           │  │
│  │                                               │  │
│  │  ┌─────────┐  ┌───────────┐  ┌────────────┐  │  │
│  │  │ Bridge  │  │ MCP Proxy │  │ Health     │  │  │
│  │  │ Client  │  │           │  │ Monitor    │  │  │
│  │  └────┬────┘  └─────┬─────┘  └──────┬─────┘  │  │
│  │       │             │               │         │  │
│  │       │       ┌─────┴─────┐         │         │  │
│  │       │       │ Figma MCP │         │         │  │
│  │       │       │ :3845     │         │         │  │
│  │       │       └───────────┘         │         │  │
│  │  ┌────┴────┐  ┌───────────┐  ┌─────┴──────┐  │  │
│  │  │ Auth    │  │ Tray      │  │ App State  │  │  │
│  │  │ Manager │  │ Manager   │  │ Manager    │  │  │
│  │  └─────────┘  └───────────┘  └────────────┘  │  │
│  └───────────────────┬───────────────────────────┘  │
│                      │ Tauri IPC (invoke/events)     │
│  ┌───────────────────┴───────────────────────────┐  │
│  │ WebView (React UI)                            │  │
│  │                                               │  │
│  │  Settings Window (on demand)                  │  │
│  │  - Connection status                          │  │
│  │  - Figma Desktop status                       │  │
│  │  - Configuration                              │  │
│  │  - Logs viewer                                │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Core Process (Rust)**: 항상 실행. 시스템 트레이, WebSocket, HTTP 프록시 등 모든 백그라운드 작업을 수행한다. 앱의 "두뇌"에 해당.

**WebView (React)**: 사용자가 설정 윈도우를 열 때만 표시. 트레이 메뉴에서 "설정"을 클릭하면 나타나고, 닫으면 숨겨진다 (프로세스는 유지). OS 네이티브 WebView를 사용하므로 Chromium을 내장하지 않는다.

**Tauri IPC**: Core Process와 WebView 사이의 통신. Rust 함수를 `#[tauri::command]`로 노출하면 React에서 `invoke()`로 호출할 수 있다.


### 2.2 External Connections

```
                          ┌──────────────────┐
                          │  Ant Cloud       │
                          │                  │
                          │  ant-realtime    │
                          │  /bridge/ws      │
                          └────────┬─────────┘
                                   │
                          WebSocket (wss://, outbound)
                          Authorization: Bearer {jwt}
                                   │
┌──────────────────────────────────┼──────────────────┐
│ User Desktop                     │                  │
│                                  │                  │
│  ┌───────────────────────────────┴───────────────┐  │
│  │           ant-desktop (Core Process)        │  │
│  └───────────────────────────────┬───────────────┘  │
│                                  │                  │
│                    HTTP POST (localhost only)        │
│                                  │                  │
│  ┌───────────────────────────────┴───────────────┐  │
│  │       Figma Desktop MCP (127.0.0.1:3845)     │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

연결은 **항상 outbound** (데스크톱 → 클라우드). 인바운드 포트를 열지 않으므로 방화벽/NAT 뒤에서도 동작한다.

### 2.3 End-to-End MCP 릴레이 (서버 측 포함)

Ant Desktop은 WebSocket 한 쪽 끝만 본다. 반대편(Ant Cloud)에서 어떤 일이 일어나는지 이해해야 프로토콜·타임아웃·에러 처리를 올바르게 설계할 수 있다. 아래는 **한 번의 MCP 호출**이 Cloud 내부에서 거치는 전체 경로이다.

```
┌─ Ant Cloud ────────────────────────────────────────────────────┐
│                                                                │
│  Job Worker              Redis               ant-realtime      │
│  (Design Job)                                                  │
│       │                                                        │
│  (1)  │── publish ──▶ bridge:mcp:req  ──▶ subscribe ──────┐    │
│       │  {requestId,                                      │    │
│       │   userId,                                         │    │
│       │   tool, args}                                     │    │
│       │                                                   ▼    │
│       │                                        해당 userId의   │
│       │                                        Bridge WS 찾기  │
│       │                                             │          │
└───────┼─────────────────────────────────────────────┼──────────┘
        │                                             │
        │                                    (2)  MCPRequestMessage
        │                                        (WebSocket frame)
        │                                             │
        │                                             ▼
        │                                      ┌──────────────┐
        │                                      │ ant-desktop │
        │                                      │  mcp::proxy   │
        │                                      └──────┬───────┘
        │                                             │
        │                                    (3)  HTTP POST
        │                                        127.0.0.1:3845
        │                                             │
        │                                             ▼
        │                                      ┌──────────────┐
        │                                      │ Figma Desktop│
        │                                      │ MCP Server   │
        │                                      └──────┬───────┘
        │                                             │
        │                                    (4)  JSON-RPC Response
        │                                             │
        │                                             ▼
        │                                      ┌──────────────┐
        │                                      │ ant-desktop │
        │                                      └──────┬───────┘
        │                                             │
        │                                    (5)  MCPResponseMessage
        │                                        (WebSocket frame)
        │                                             │
┌───────┼─────────────────────────────────────────────┼──────────┐
│       │                                             ▼          │
│       │                               ant-realtime 수신 후     │
│       │                               Redis publish            │
│  (6)  │◀── subscribe ◀── bridge:mcp:resp:{requestId}           │
│       │                                                        │
│  Worker Promise resolve                                        │
│  (1에서 대기 중이던 callTool 반환)                              │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

**Ant Desktop 관점 요약:**
- Ant Desktop이 관여하는 구간은 **(2)~(5)** 뿐이다. (1)과 (6)은 Cloud 내부(Redis Pub/Sub)이며, Ant Desktop은 이를 알 필요가 없다.
- WebSocket으로 `MCPRequestMessage`가 오면 Figma에 프록시하고 `MCPResponseMessage`를 돌려보내는 것이 전부이다.
- 그러나 **타임아웃**은 양쪽에서 건다: Worker 쪽에서 `BRIDGE_MCP_REQUEST_TIMEOUT_MS`만큼 대기하고, Ant Desktop도 동일 값으로 Figma HTTP 호출을 제한한다. 둘 중 하나라도 먼저 타임아웃되면 에러 응답이 된다.

**왜 Worker가 직접 WebSocket을 안 여는가:**
- Job Worker는 BullMQ 큐에서 디큐된 자식 프로세스이다. 수명이 짧고, 한 사용자의 Ant Desktop에 직접 소켓을 열 방법이 없다(사용자 PC의 IP를 모르고, NAT 뒤에 있다).
- ant-realtime은 이미 Ant Desktop과 **항시 연결된 WebSocket 세션**을 유지하고 있으므로, Worker는 Redis로 메시지를 보내고 Realtime이 중계하는 것이 자연스럽다.

### 2.4 배포 범위 (Cloud vs Local)

ant-desktop은 **Ant Cloud**에서 Design Job 워커가 사용자 `localhost`에 접근할 수 없을 때 필요하다. **로컬 개발 모드**(API·워커·Redis가 같은 머신)에서는 워커가 직접 `http://127.0.0.1:3845/mcp`를 호출할 수 있으므로 Ant Desktop 없이 MCP를 사용할 수 있다. Figma 완전 연동(PRD §1.3 Tier 2)은 Cloud 사용자를 주 대상으로 설명한다.

### 2.5 서버 베이스 URL (설정·프리셋·WebSocket 조합)

Ant Desktop은 ant-cli가 **로컬인지 클라우드인지 자동 탐지하지 않는다.** 사용자가 설정에 넣은 **HTTP(S) 오리진**만 사용한다.

**저장 값 (`tauri-plugin-store`):**

- `realtime_base_url` — Bridge WebSocket이 붙을 **realtime 쪽** 베이스. 스킴+호스트+포트까지. 경로는 비우거나 루트만 (예: `https://ant.crosstoken.io`, `http://127.0.0.1:4101`).
- 로컬 프리셋 UI는 호스트+포트를 합쳐 위 문자열을 만든다. 포트 기본값 **4101** (ant-realtime 기본; API 4100과 혼동 시 사용자가 변경).

**프리셋 버튼(표시 텍스트만 "클라우드" / "로컬"):**

| 프리셋 | 필드에 채우는 기본값 | 비고 |
|--------|---------------------|------|
| 클라우드 | `https://ant.crosstoken.io` | 제품 도메인 변경 시 앱 상수만 갱신 |
| 로컬 | `http://127.0.0.1` + 포트 **4101** | 포트는 숫자 입력으로 조정 가능 |

**WebSocket URL 계산:**

1. `realtime_base_url` 파싱 → 스킴을 `http`→`ws`, `https`→`wss`로 치환.
2. 경로는 **`/bridge/ws`** 를 연결 (이미 경로가 있는 베이스 URL은 구현 시 정책 정의: MVP는 오리진만 저장 권장).
3. 최종 예: `https://ant.crosstoken.io` → `wss://ant.crosstoken.io/bridge/ws`; `http://127.0.0.1:4101` → `ws://127.0.0.1:4101/bridge/ws`.

**딥링크 `server` 쿼리:** ant-ui가 넘기는 값도 위와 동일 형식이면 store의 `realtime_base_url`을 덮어쓴 뒤 연결한다. 사용자는 이후 설정에서 수동 수정 가능.


## 3. Rust Core Modules

### 3.1 Module Structure

```
src-tauri/src/
├── main.rs              # 최소 진입점 (lib::run 호출)
├── lib.rs               # Tauri 앱 빌더, 플러그인 등록, 트레이 설정, 백그라운드 태스크 spawn
├── error.rs             # AppError 통합 에러 enum (모듈별 에러 From 구현)
├── bridge/
│   ├── mod.rs
│   ├── client.rs        # WebSocket 클라이언트 (연결, 재연결, 메시지 송수신)
│   └── protocol.rs      # BridgeMessage 직렬화/역직렬화
├── mcp/
│   ├── mod.rs
│   └── proxy.rs         # Figma Desktop MCP HTTP 프록시
├── auth/
│   ├── mod.rs
│   ├── jwt.rs           # JWT payload Base64URL 디코딩, userId(sub) 추출 (서명 검증 없음)
│   ├── keychain.rs      # OS Keychain 읽기/쓰기 (JWT만)
│   └── deeplink.rs      # ant-desktop:// 딥링크 핸들러
├── health/
│   ├── mod.rs
│   └── figma_check.rs   # Figma Desktop MCP healthcheck
├── tray/
│   ├── mod.rs
│   └── menu.rs          # 트레이 메뉴 구성, 아이콘 상태 변경
├── state/
│   ├── mod.rs
│   └── app_state.rs     # 앱 전역 상태 (연결 상태, Figma 상태 등)
├── commands.rs          # Tauri IPC 커맨드 (#[tauri::command])
└── constants.rs         # 상수 (타임아웃, 간격, 버전 등)
```

**모듈 의존 방향 (위→아래 허용, 역방향 금지):**

```
commands.rs ──▶ state/, bridge/, auth/
bridge/     ──▶ mcp/, auth/, state/
mcp/        ──▶ (외부 HTTP만)
auth/       ──▶ (Keychain, store만)
health/     ──▶ state/
tray/       ──▶ state/
```

순환 의존이 생기면 설계 오류이다. 모듈 간 비동기 조율이 필요하면 공유 상태(`Arc<Mutex<AppState>>`)나 `tokio::sync::mpsc` 채널을 사용한다.

### 3.2 error.rs — 에러 아키텍처

모든 모듈은 자체 에러 enum(`BridgeError`, `McpError`, `AuthError`)을 `thiserror`로 정의한다. 최상위 `error.rs`가 이들을 통합한다.

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Bridge(#[from] bridge::BridgeError),
    #[error(transparent)]
    Mcp(#[from] mcp::McpError),
    #[error(transparent)]
    Auth(#[from] auth::AuthError),
    #[error(transparent)]
    Health(#[from] health::HealthError),
    #[error("internal: {0}")]
    Internal(String),
}
```

Tauri IPC command에서 `Result<T, AppError>`를 반환하려면 `AppError`에 `Serialize`를 구현해야 한다. 프론트엔드에는 `{ error: string }` 형태의 단순 문자열로 전달한다.

**`anyhow`를 쓰지 않는 이유:** Tauri command 직렬화에서 에러 타입이 보존되어야 UI가 에러 종류별 처리(인증 만료 vs 네트워크 vs Figma 미응답)를 할 수 있다.

### 3.3 비동기 태스크 관리

`lib.rs`의 Tauri `setup` 훅에서 장기 실행 백그라운드 태스크를 spawn한다:

```rust
// lib.rs (개념적 구조)
tauri::Builder::default()
    .setup(|app| {
        let state = AppState::new();
        let token = CancellationToken::new();

        // (1) WebSocket 연결 + 수신 루프
        tokio::spawn(bridge::run_loop(token.clone(), state.clone()));

        // (2) 30초 heartbeat 타이머
        tokio::spawn(bridge::heartbeat_loop(token.clone(), state.clone()));

        // (3) 10초 Figma Desktop 헬스체크
        tokio::spawn(health::check_loop(token.clone(), state.clone(), app.handle().clone()));

        app.manage(state);
        app.manage(token);
        Ok(())
    })
```

**태스크 취소:** `tokio_util::sync::CancellationToken`을 공유한다. 앱 종료(`on_exit`) 또는 사용자 "연결 해제" 시 토큰을 cancel하면 모든 루프가 정리된다. 재연결 시 새 토큰으로 태스크를 다시 spawn한다.

**모듈 간 메시지 전달:** bridge가 `MCPRequestMessage`를 수신하면 `mcp::proxy`를 직접 호출(async function call)한다. 별도 채널은 MVP에서는 불필요하다 — MCP 호출이 직렬화(Mutex)되므로 채널 큐의 이점이 없다.

### 3.4 bridge/client.rs — WebSocket Client

앱의 핵심 모듈. Ant Cloud의 `/bridge/ws` 엔드포인트에 WebSocket 연결을 유지한다.

**책임:**
- WebSocket 연결 수립 (TLS, Authorization 헤더)
- JWT 페이로드에서 `sub`(또는 ant-cli가 사용하는 사용자 ID 클레임)를 읽어 **`BridgeRegisterMessage.userId`**에 채움 (서명 검증은 서버가 하며, Ant Desktop은 표시·등록용으로만 디코딩)
- `BridgeRegisterMessage` 전송 (연결 직후)
- `BridgeHeartbeatMessage` 전송 (30초 간격)
- `MCPRequestMessage` 수신 → `mcp::proxy`로 전달 → `MCPResponseMessage` 전송
- 연결 끊김 시 지수 백오프 재연결 (최대 간격까지 도달한 뒤에도 **무한 재시도**)

**userId 획득:** `auth` 모듈에서 저장된 JWT 문자열을 Base64URL 디코딩하여 payload JSON을 파싱하고, ant-cli `JwtService`와 동일한 클레임 키(예: `sub`)를 `userId`로 사용한다. 클레임이 없거나 파싱 실패 시 WebSocket 연결을 시작하지 않고 `AuthRequired` 상태로 안내한다.

**상태 머신:**

```
Initial ──(JWT+userId OK)──▶ Connecting ──(성공)──▶ Connected
                                │                    │
                    (연결 실패)  │                    │ 소켓 끊김
                                ▼                    ▼
                          Reconnecting ◀──────────────┘
                                │
                                └──(백오프 후)──▶ Connecting  (무한 반복, 최대 간격 60s)

Connected ──(401·JWT 무효)──▶ AuthRequired

(임의 상태) ──(사용자 연결 해제·앱 종료)──▶ Disconnected
```

`Disconnected`는 **의도적 종료**로만 진입한다. 네트워크 장애는 `Reconnecting`에서 백오프 재시도를 끝없이 반복한다.

**재연결 전략:**
- 지수 백오프: 1s → 2s → 4s → 8s → 16s → 32s → 60s (최대)
- 재연결 성공 시 백오프 리셋
- 네트워크 변경 감지 시 즉시 재연결 시도

### 3.5 mcp/proxy.rs — MCP Proxy

Cloud에서 수신한 `MCPRequestMessage`를 Figma Desktop MCP로 중계한다.

**요청 흐름:**

```
MCPRequestMessage (WebSocket)
    │
    ▼
┌──────────────────────────────┐
│ mcp::proxy::handle_request() │
│                              │
│ 1. tool + args 추출           │
│ 2. JSON-RPC 요청 구성         │
│ 3. POST http://127.0.0.1:3845/mcp │
│ 4. 응답 파싱                   │
│ 5. MCPResponseMessage 반환    │
└──────────────────────────────┘
    │
    ▼
MCPResponseMessage (WebSocket)
```

**JSON-RPC 요청 형식:**

```json
{
  "jsonrpc": "2.0",
  "id": "{requestId}",
  "method": "tools/call",
  "params": {
    "name": "get_design_context",
    "arguments": {
      "fileKey": "abc123",
      "nodeId": "1:2"
    }
  }
}
```

**에러 처리:**
- Figma Desktop 미실행 (connection refused): `error: "Figma Desktop is not running"`
- 요청 타임아웃 (30초): `error: "MCP request timed out"`
- Figma 내부 에러: MCP 응답의 에러를 그대로 전달

**동시 요청 (MVP):** `tokio::sync::Mutex` 등으로 **Figma MCP HTTP 호출을 전역 직렬화**한다. WebSocket에서 동시에 여러 `MCPRequestMessage`가 도착하면 한 번에 하나만 `127.0.0.1:3845`로 전달한다. 순서는 수신 순서(FIFO)를 기본으로 한다.

**대용량 응답:** MCP JSON-RPC 응답 전체를 문자열로 직렬화한 뒤 WebSocket 텍스트 프레임으로 보낸다. 바이너리 필드는 MCP 스키마에 따라 Base64로 임베드된 상태로 전달된다. 단일 메시지가 구현 상한(예: 16MiB)을 넘으면 Ant Desktop은 `MCPResponseMessage`에 `error`를 설정하고 Cloud에 알린다.

### 3.6 auth/keychain.rs — Keychain Manager

JWT 토큰을 OS 네이티브 자격 증명 저장소에 안전하게 보관한다.

| OS | 저장소 | Crate |
|----|--------|-------|
| macOS | Keychain Access | `keyring` |
| Windows | Credential Manager | `keyring` |
| Linux | libsecret (GNOME Keyring) | `keyring` |

**저장 항목 (Keychain 전용):**
- `ant-desktop/jwt` — JWT 토큰

**비밀 아님:** Ant Cloud 베이스 URL(`https://...`)은 **tauri-plugin-store**에만 저장한다. Keychain에 서버 URL을 중복 저장하지 않는다 (PRD·보안 모델과 일치).

### 3.7 auth/deeplink.rs — Deep Link Handler

`ant-desktop://connect?token={jwt}&server={url}` 형식의 딥링크를 처리한다.

**처리 흐름:**
1. URL 파싱 → `token`, `server` 쿼리 파라미터 추출
2. JWT를 Keychain에 저장
3. `server` 쿼리를 **tauri-plugin-store**의 `realtime_base_url`로 저장 (형식은 §2.5와 동일)
4. `bridge::client`에 연결 시작 신호 (§2.5 규칙으로 WS URL 계산)
5. UI에 연결 상태 업데이트 이벤트

**중복 실행 방지:** `tauri-plugin-single-instance`를 사용하여 이미 실행 중인 인스턴스가 있으면 딥링크 데이터를 기존 인스턴스로 전달한다.

### 3.8 health/figma_check.rs — Health Monitor

Figma Desktop MCP의 가용성을 주기적으로 확인한다.

**체크 방법:** `http://127.0.0.1:3845/mcp`에 경량 요청 전송. 연결 성공/실패로 판별.

**체크 주기 (MVP):** 고정 **10초** 간격. 적응형 간격(실패 후 단축 등)은 Phase 2 이후 최적화 항목으로 분리한다.

**상태 변경 시 이벤트 발행:** `figma_status_changed` → 트레이 아이콘 색상 변경 + UI 업데이트

### 3.9 tray/menu.rs — Tray Manager

시스템 트레이 아이콘과 메뉴를 관리한다.

**아이콘 상태:**

| 색상 | 의미 | 조건 |
|------|------|------|
| 초록 | 정상 | WebSocket 연결됨 + Figma Desktop 감지됨 |
| 주황 | 부분 동작 | WebSocket 연결됨 + Figma Desktop 미감지 |
| 빨강 | 연결 끊김 | WebSocket 연결 안됨 |
| 회색 | 미설정 | JWT 없음 (인증 전) |

**메뉴 항목:**

```
ant-desktop
─────────────────────
● Ant Cloud 연결됨          (상태 표시)
● Figma Desktop 감지됨      (상태 표시)
─────────────────────
  설정...                   (설정 윈도우 열기)
  로그 보기...               (로그 윈도우 열기)
─────────────────────
  종료                      (앱 종료)
```

### 3.10 state/app_state.rs — App State

앱 전역 상태를 관리한다. `Arc<Mutex<AppState>>`로 스레드 안전하게 공유.

```rust
pub struct AppState {
    pub connection_status: ConnectionStatus,  // Connected, Reconnecting, Disconnected, AuthRequired
    pub figma_status: FigmaStatus,            // Available, Unavailable, Unknown
    pub server_url: Option<String>,
    pub machine_id: String,
    pub last_heartbeat: Option<Instant>,
    pub mcp_request_count: u64,               // 총 처리 요청 수
    pub last_mcp_request: Option<Instant>,     // 마지막 요청 시간
}
```

### 3.11 commands.rs — Tauri IPC Commands

React UI에서 호출 가능한 Tauri 커맨드 목록:

| 커맨드 | 용도 |
|--------|------|
| `get_app_state` | 현재 앱 상태 조회 |
| `get_connection_info` | 서버 베이스 URL, 인증 상태 조회 |
| `set_realtime_base_url` | `realtime_base_url` 저장 후 WebSocket 재연결(선택) |
| `apply_preset_cloud` | `https://ant.crosstoken.io` 로 베이스 URL 채움 (표시 라벨만 "클라우드") |
| `apply_preset_local` | `http://127.0.0.1:{port}` 로 채움, `port` 기본 4101 (표시 라벨만 "로컬") |
| `disconnect` | WebSocket 연결 해제 + JWT 삭제 |
| `toggle_autostart` | 자동 시작 on/off |
| `get_autostart_status` | 자동 시작 설정 조회 |
| `get_logs` | 최근 로그 조회 |
| `open_figma_download` | Figma Desktop 다운로드 페이지 열기 |


## 4. React Frontend

### 4.1 UI Structure

UI는 최소한으로 유지한다. 사용자는 대부분 트레이 아이콘으로만 상호작용하며, 설정 윈도우는 드물게 연다.

**윈도우 사양:**
- 크기: 480 x 600px (고정)
- 리사이즈: 불가
- 닫기 동작: 윈도우 숨기기 (앱 종료 아님)

**페이지:**
- 상태 (기본): 연결 상태 + Figma 상태 + 통계 (처리 요청 수 등)
- 설정: 자동 시작, **realtime 베이스 URL**(직접 입력 + 클라우드/로컬 프리셋 + 로컬 포트), 연결 해제, 적용 후 재연결
- 로그: 최근 이벤트/에러 목록

### 4.2 Frontend Structure

```
src/
├── App.tsx              # 라우팅 (탭 네비게이션)
├── pages/
│   ├── StatusPage.tsx   # 연결 상태, Figma 상태, 통계
│   ├── SettingsPage.tsx # 자동 시작, 서버 베이스 URL·프리셋, 연결 관리
│   └── LogsPage.tsx     # 이벤트/에러 로그
├── components/
│   ├── StatusIndicator.tsx  # 연결 상태 원형 표시
│   └── ConnectionCard.tsx   # 연결 정보 카드
├── hooks/
│   ├── useAppState.ts   # Tauri 이벤트 구독 + invoke로 상태 조회
│   └── useAutostart.ts  # 자동 시작 설정
└── lib/
    └── tauri.ts         # invoke 래퍼, 이벤트 리스너 유틸리티
```

### 4.3 Tauri Events (Core → UI)

Core Process가 UI에 상태 변경을 알리는 이벤트:

| 이벤트 | 페이로드 | 트리거 |
|--------|---------|--------|
| `connection-status-changed` | `{ status: string }` | WebSocket 상태 변경 |
| `figma-status-changed` | `{ available: boolean }` | Figma Desktop 감지 변경 |
| `mcp-request-processed` | `{ tool: string, duration_ms: number }` | MCP 요청 완료 |
| `auth-received` | `{ server: string }` | 딥링크 인증 수신 |
| `error` | `{ message: string }` | 에러 발생 |


## 5. Protocol Detail

### 5.1 Connection Lifecycle

```
                ant-desktop              ant-realtime (Cloud)
                     │                           │
                     │──── WebSocket CONNECT ────▶│
                     │     Authorization: Bearer  │
                     │     {jwt}                  │
                     │                           │
                     │◀──── 101 Switching ────────│
                     │      Protocols             │
                     │                           │
                     │──── BridgeRegister ───────▶│
                     │     { type, userId,        │
                     │       machineId,           │
                     │       capabilities }       │
                     │                           │
                     │    ┌─── Heartbeat Loop ──┐ │
                     │    │                     │ │
                     │────┤ BridgeHeartbeat ────▶│ │  (30초 간격)
                     │    │ { timestamp }       │ │
                     │    └─────────────────────┘ │
                     │                           │
                     │◀──── MCPRequest ──────────│  (Design Job 실행 시)
                     │     { requestId, tool,    │
                     │       args }              │
                     │                           │
                     │──── MCPResponse ─────────▶│
                     │     { requestId, result } │
                     │                           │
                     │──── BridgeDisconnect ────▶│  (정상 종료)
                     │     { reason }            │
                     │                           │
```

### 5.2 MCP Proxy Detail

```
MCPRequestMessage                    JSON-RPC Request
{                                    {
  "type": "mcp.request",              "jsonrpc": "2.0",
  "requestId": "uuid-1",    ──▶      "id": "uuid-1",
  "tool": "get_design_context",       "method": "tools/call",
  "args": {                           "params": {
    "fileKey": "abc",                    "name": "get_design_context",
    "nodeId": "1:2"                      "arguments": {
  }                                        "fileKey": "abc",
}                                          "nodeId": "1:2"
                                         }
                                       }
                                     }

JSON-RPC Response                    MCPResponseMessage
{                                    {
  "jsonrpc": "2.0",                    "type": "mcp.response",
  "id": "uuid-1",           ──▶       "requestId": "uuid-1",
  "result": {                          "result": {
    "content": [...]                     "content": [...]
  }                                    }
}                                    }
```

### 5.3 Deeplink Authentication Flow

```
    ant-ui (Browser)          ant-api            OS            ant-desktop
         │                      │                │                  │
         │── POST ─────────────▶│                │                  │
         │   /api/auth/         │                │                  │
         │   desktop-token    │                │                  │
         │   (cookie auth)      │                │                  │
         │                      │                │                  │
         │◀── { token, ────────│                │                  │
         │     expiresAt }      │                │                  │
         │                      │                │                  │
         │── window.open() ────────────────────▶│                  │
         │   ant-desktop://connect             │                  │
         │   ?token={jwt}                        │                  │
         │   &server={url}                       │                  │
         │                      │                │                  │
         │                      │                │──── deeplink ──▶│
         │                      │                │    event         │
         │                      │                │                  │
│                      │                │                  │── JWT → Keychain
│                      │                │                  │── URL → tauri-plugin-store
         │                      │                │                  │
         │                      │◀──── WebSocket Connect ──────────│
         │                      │      Authorization: Bearer {jwt}  │
         │                      │                │                  │
```


## 6. Security Model

### 6.1 Credential Storage

| 데이터 | 저장 위치 | 접근 제어 |
|--------|---------|---------|
| JWT 토큰 | OS Keychain | OS 수준 앱 격리 |
| `realtime_base_url` | tauri-plugin-store | 앱 데이터 디렉토리 (OS 사용자 권한으로 보호; 플러그인 기본 암호화 여부는 Tauri 설정 따름) |
| 기타 앱 설정 | tauri-plugin-store | 앱 데이터 디렉토리 |

### 6.2 Network Security

- **WebSocket**: TLS 필수 (`wss://`). 인증서 검증 활성화.
- **MCP 프록시**: `127.0.0.1:3845`만 접근. 외부 호스트 차단 (하드코딩).
- **딥링크**: JWT가 URL에 포함되지만 로컬 OS 내부 통신이므로 네트워크 노출 없음.

### 6.3 Tauri Security

- CSP (Content Security Policy) 설정으로 WebView에서 외부 스크립트 차단
- IPC 명령어별 권한 설정 (`tauri.conf.json > permissions`)
- WebView에서 Rust 함수 호출은 선언된 command만 가능


## 7. Error Handling Strategy

### 7.1 Error Categories

| 카테고리 | 예시 | 처리 |
|---------|------|------|
| Auth | JWT 만료, 인증 거부 | 트레이 경고 + "다시 연결" 안내 |
| Network | WebSocket 끊김, DNS 실패 | 지수 백오프 재연결 |
| Figma | Desktop 미실행, MCP 에러 | MCP 응답에 에러 포함 + 트레이 경고 |
| Internal | 패닉, OOM | 로그 기록 + 자동 재시작 (OS autostart) |

### 7.2 Reconnection Strategy

```
연결 끊김 감지
    │
    ▼
시도 1: 1초 후 재연결
    │ 실패
    ▼
시도 2: 2초 후 재연결
    │ 실패
    ▼
시도 3: 4초 후 재연결
    │ 실패
    ▼
  ...지수 증가...
    │ 실패
    ▼
시도 N: 60초 후 재연결 (최대 간격)
    │
    ▼
무한 반복 (사용자가 종료하거나 연결 성공할 때까지)
```

재연결 성공 시:
1. 백오프 간격 리셋
2. `BridgeRegisterMessage` 재전송
3. 트레이 아이콘 초록색 복구

`Disconnected`와 `Reconnecting`을 혼동하지 않는다: "최대 재시도 초과 후 영구 오프라인" 상태는 두지 않는다.


## 8. @ant/shared Type Mapping

ant-desktop의 Rust 구조체와 `@ant/shared/src/figma.ts` TypeScript 타입 간의 매핑:

| TypeScript (@ant/shared) | Rust (ant-desktop) | 용도 |
|--------------------------|---------------------|------|
| `BridgeMessage` | `enum BridgeMessage` | WebSocket 메시지 union |
| `BridgeRegisterMessage` | `struct RegisterMessage` | 연결 등록 |
| `BridgeHeartbeatMessage` | `struct HeartbeatMessage` | 생존 확인 |
| `BridgeDisconnectMessage` | `struct DisconnectMessage` | 정상 종료 |
| `MCPRequestMessage` | `struct McpRequest` | MCP 요청 |
| `MCPResponseMessage` | `struct McpResponse` | MCP 응답 |
| `BridgeCapability` | `enum BridgeCapability` | `FigmaMcp` |
| `FigmaMCPTool` | `enum FigmaMcpTool` | MCP 도구 이름 |

JSON 직렬화 형식은 TypeScript와 바이트 단위로 호환되어야 한다.

- **태그 필드:** `type` 값은 `bridge.register`, `mcp.request` 등 **리터럴 문자열**이므로 enum variant마다 `#[serde(rename = "...")]`로 지정한다.
- **필드 이름:** `requestId`, `machineId` 등은 **각 struct**에 `#[serde(rename_all = "camelCase")]`를 붙인다. enum에만 `rename_all = "camelCase"`를 두면 variant 태그에만 영향을 주고 내부 필드에는 적용되지 않는다.

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BridgeMessage {
    #[serde(rename = "bridge.register")]
    Register(RegisterMessage),
    #[serde(rename = "bridge.heartbeat")]
    Heartbeat(HeartbeatMessage),
    #[serde(rename = "bridge.disconnect")]
    Disconnect(DisconnectMessage),
    #[serde(rename = "mcp.request")]
    McpRequest(McpRequest),
    #[serde(rename = "mcp.response")]
    McpResponse(McpResponse),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterMessage {
    pub user_id: String,      // JSON: "userId"
    pub machine_id: String,   // JSON: "machineId"
    pub capabilities: Vec<BridgeCapability>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRequest {
    pub request_id: String,
    pub tool: String,
    pub args: serde_json::Value,
}

// HeartbeatMessage { timestamp }, DisconnectMessage, McpResponse 동일 패턴
```

> Rust 필드명은 스네이크 케이스를 유지하고, serde가 JSON 키를 camelCase로 매핑한다.
