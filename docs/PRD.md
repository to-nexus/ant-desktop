# ant-desktop PRD (Product Requirements Document)

## 1. Overview

### 1.1 Product Definition

ant-desktop은 사용자 데스크톱에서 실행되는 경량 컴패니언 앱이다. Figma Desktop App의 MCP 서버(`localhost:3845`)와 Ant Cloud 사이를 중계하여, Ant의 Design Job이 Figma의 고품질 디자인 분석 기능(`get_design_context` 등)에 접근할 수 있게 한다.

### 1.2 Why This Exists

Figma Desktop MCP는 `http://127.0.0.1:3845/mcp`에서만 접근 가능하다. 클라우드에서 실행되는 Ant의 Design Job Worker는 사용자의 로컬 네트워크에 직접 접근할 수 없으므로, 데스크톱에서 실행되는 브릿지 앱이 필요하다.

```
Ant Cloud (Design Job)
    ↕ WebSocket (outbound from desktop)
ant-desktop (User Desktop)
    ↕ HTTP (localhost only)
Figma Desktop MCP (localhost:3845)
```

Figma REST API(OAuth)만으로는 파일 목록·에셋 export 정도만 가능하며, Ant의 Design Job이 필요로 하는 정밀 분석(`get_design_context` — 레이아웃, 토큰 매핑, 컴포넌트 식별, 코드 힌트)은 MCP 경유로만 사용할 수 있다. **Ant 제품에서 "Figma 연동"은 OAuth + ant-desktop(MCP)이 모두 갖춰진 상태만을 의미하며, OAuth만 연결된 중간 상태는 사용자에게 노출되는 기능이 없다.**

### 1.3 Figma 연동 Tier (설정 단계)

내부적으로 Figma 연동 설정 과정을 3단계로 추적한다. **단, Tier 1만으로 동작하는 제품 기능은 없다** — Tier 1은 Tier 2로 가기 위한 전제 조건(OAuth 토큰 확보)일 뿐이다.

| Tier | 상태 | 의미 |
|------|------|------|
| 0 | Figma 미연결 | OAuth 미완료. Figma 기능 전체 비활성. |
| 1 | OAuth 완료 | 인증 토큰 확보됨. **그러나 MCP 없이는 Design Job에서 사용할 수 없음.** UI에서 "Ant Desktop 앱 연결" 단계로 안내. |
| **2** | **OAuth + ant-desktop 연결** | **완전 연동. Design Job이 MCP(get_design_context 등)를 사용할 수 있는 유일한 상태.** |

ant-desktop은 Tier 1(OAuth만 확보) → **Tier 2(완전 연동)** 전환을 담당하는 유일한 수단이다. Tier 1에서 멈춘 사용자에게는 "Figma 연동 미완료 — Ant Desktop 앱을 설치하세요" 안내만 표시된다.


## 2. Target User

Ant를 사용하여 디자인-투-코드 작업을 수행하는 프론트엔드 개발자 또는 디자이너.

전제 조건:
- Ant Cloud 계정 보유 (OAuth 인증 완료)
- Figma Desktop App 설치 및 실행 (Figma Web은 MCP 미지원)
- macOS, Windows, 또는 Linux 데스크톱 환경


## 3. User Flows

### 3.1 초기 설치 및 연결

```
1. [ant-ui] 계정 설정 > Figma 섹션에서 OAuth 완료 확인 (Tier 1 — 이 상태에서는 아직 Design Job 사용 불가)
2. [ant-ui] "ant-desktop 다운로드" 링크 클릭 → 플랫폼별 인스톨러 다운로드
3. [Desktop] 앱 설치 및 실행 → 시스템 트레이에 아이콘 표시
4. [ant-ui] "컴패니언 앱 연결" 버튼 클릭
5. [ant-ui] → [ant-api] POST /api/auth/desktop-token (쿠키 인증)
6. [ant-api] 장기 JWT(90일) 발급, ant-ui에 반환
7. [ant-ui] 딥링크 호출: ant-desktop://connect?token={jwt}&server={realtime-base-url} (예: `https://ant.crosstoken.io` 또는 로컬 `http://127.0.0.1:4101`)
8. [OS] → [ant-desktop] 딥링크 수신, JWT(Keychain) + realtime 베이스 URL(store) 저장; 이후에도 설정에서 동일 값 수정 가능
9. [ant-desktop] → [ant-cloud] WebSocket 연결 (Authorization: Bearer {jwt})
10. [ant-desktop] BridgeRegisterMessage 전송
11. [ant-ui] Bridge 상태 폴링 → Tier 2 확인 → 초록색 연결 상태 표시
```

### 3.2 정상 운영 (MCP 릴레이)

```
1. [ant-desktop] 시스템 트레이에 상주 (초록색 = 연결됨)
2. [ant-cloud]
   a. Design Job Worker가 MCP 호출 필요 → Redis에 요청 publish
   b. ant-realtime이 Redis에서 수신 → 해당 userId의 Bridge WebSocket으로 MCPRequestMessage 전송
      (Worker는 Ant Desktop에 직접 소켓을 열지 않음. 이유: Worker는 단명 자식 프로세스이며 사용자 PC IP를 모름)
3. [ant-desktop] 요청 수신 → http://127.0.0.1:3845/mcp에 HTTP POST (JSON-RPC)
4. [Figma Desktop MCP] 응답 반환
5. [ant-desktop] MCPResponseMessage로 변환 → WebSocket으로 ant-realtime에 전송
6. [ant-cloud]
   a. ant-realtime이 응답을 Redis에 publish
   b. Worker가 대기 중이던 Promise resolve → MCP 결과 획득
7. 30초 간격 heartbeat로 연결 유지
```

> **상세 시퀀스 다이어그램**: SYSTEM_DESIGN.md §2.3 참조

### 3.3 문제 해결 시나리오

**Figma Desktop 미실행:**
- ant-desktop이 localhost:3845 healthcheck 실패 감지
- 트레이 아이콘: 주황색 (경고)
- 트레이 메뉴에 "Figma Desktop이 실행되지 않았습니다" 표시
- Cloud에 MCP 요청이 오면 error 응답: `"Figma Desktop is not running"`

**WebSocket 연결 끊김:**
- 지수 백오프로 자동 재연결 (1s → 2s → 4s → ... → 최대 60s)
- 트레이 아이콘: 빨간색 (연결 끊김)
- 재연결 성공 시 자동으로 BridgeRegisterMessage 재전송

**JWT 만료 (90일 후):**
- WebSocket 연결 시 서버가 401 반환
- 트레이 메뉴에 "인증 만료. ant-ui에서 다시 연결해주세요" 표시
- ant-ui에서 "다시 연결" 클릭 → 동일한 딥링크 플로우 반복


## 4. Functional Requirements

### FR-1: Ant Cloud WebSocket 연결
- outbound-only WebSocket 연결 (방화벽/NAT 친화적)
- 연결 대상은 **ant-realtime 프로세스가 Bridge WebSocket을 제공하는 호스트**이다. ant-cli 기본 포트는 API **4100**, realtime **4101**이므로 로컬에서는 보통 **4101** 쪽에 맞춘다 (리버스 프록시로 한 포트에 묶은 경우는 사용자가 포트를 바꾸면 됨).
- 엔드포인트(상대 경로): `{realtimeOrigin}/bridge/ws` — 예: 베이스가 `https://ant.crosstoken.io`이면 `wss://ant.crosstoken.io/bridge/ws`, 로컬이 `http://127.0.0.1:4101`이면 `ws://127.0.0.1:4101/bridge/ws`
- 인증: Authorization 헤더에 JWT
- 연결 후 `BridgeRegisterMessage` 전송 (userId, machineId, capabilities)
- 30초 간격 `BridgeHeartbeatMessage` 전송
- 연결 끊김 시 지수 백오프 자동 재연결

### FR-2: Figma Desktop MCP 프록시
- `MCPRequestMessage` 수신 → `http://127.0.0.1:3845/mcp`로 JSON-RPC HTTP POST
- MCP 응답 → `MCPResponseMessage`로 변환하여 WebSocket 전송
- 지원 MCP 도구: `get_metadata`, `get_design_context`, `get_screenshot`, `get_variable_defs`
- 요청별 타임아웃: 30초

### FR-3: 딥링크 인증
- 커스텀 URL 스킴: `ant-desktop://`
- `ant-desktop://connect?token={jwt}&server={url}` 형식으로 JWT 수신 (`server`는 **realtime Bridge용 HTTP(S) 오리진**, 아래 FR-3a와 동일 규칙)
- JWT를 OS Keychain에 안전하게 저장
- `server`가 오면 **설정의 서버 베이스 URL을 갱신**한 뒤 WebSocket 연결 시작 (딥링크만으로 URL이 고정되지 않음)

### FR-3a: 서버 주소 설정(수동 + 프리셋)
- 설정 화면에서 **Bridge/realtime 베이스 URL**을 직접 입력하거나, 버튼으로 프리셋을 채울 수 있다.
- **프리셋은 UI 라벨일 뿐**이며, ant-cli의 `ANT_SERVER_MODE` 등을 탐지하지 않는다.
  - **「클라우드」:** 기본값으로 `https://ant.crosstoken.io` 를 필드에 채움 (제품 배포 도메인이 바뀌면 앱 기본값만 수정하면 됨).
  - **「로컬」:** 기본값으로 `http://127.0.0.1` + **포트 입력란 기본 4101** (ant-realtime 기본). 사용자가 4100 등 다른 포트로 바꿀 수 있다.
- 최종 연결 URL은 앱 내부 규칙으로 `http`→`ws`, `https`→`wss` 변환 후 경로 `/bridge/ws`를 붙인다 (리버스 프록시로 경로가 다른 경우는 추후 설정 키로 확장 가능).

### FR-4: 시스템 트레이 상주
- OS 시스템 트레이(macOS 메뉴바, Windows 시스템 트레이, Linux 상태 영역)에 아이콘 상주
- 아이콘 상태: 초록(연결됨), 주황(Figma 미감지), 빨강(연결 끊김), 회색(비활성)
- 트레이 메뉴: 연결 상태, Figma Desktop 상태, 설정 열기, 종료

### FR-5: 설정 윈도우
- 트레이 메뉴 "설정"에서 열리는 작은 윈도우
- 표시 정보: 연결 상태, 서버 베이스 URL, 인증 상태, Figma Desktop 상태
- 설정: OS 로그인 시 자동 시작 on/off; **서버 베이스 URL**(수정 + 클라우드/로컬 프리셋 버튼 + 로컬 포트 필드)
- 작업: 연결 해제, 로그 보기, **저장 후 재연결**(URL 변경 시)

### FR-6: 자동 시작
- OS 로그인 시 자동 실행 (기본값: 활성화)
- 사용자가 설정에서 on/off 토글 가능

### FR-7: 자동 업데이트
- 앱 시작 시 + 24시간 간격으로 업데이트 확인
- 업데이트 가용 시 트레이 알림 → 사용자 확인 후 설치


## 5. Non-Functional Requirements

### NFR-1: Performance
- 메모리 사용: 유휴 시 50MB 이하
- MCP 릴레이 오버헤드: 요청당 10ms 이하 (네트워크 제외)
- 앱 시작 → 트레이 표시: 2초 이내

### NFR-2: Security
- JWT는 OS Keychain에 저장 (macOS Keychain, Windows Credential Manager, Linux libsecret)
- MCP 프록시는 localhost(127.0.0.1) 요청만 수행 (외부 호스트 접근 차단)
- WebSocket 연결은 TLS(wss://) 필수

### NFR-3: Reliability
- WebSocket 자동 재연결 (지수 백오프, 최대 60초 간격)
- Figma Desktop 상태를 10초 간격으로 healthcheck
- 앱 크래시 시 자동 재시작 (OS autostart에 의해)

### NFR-4: Cross-Platform
- macOS (Apple Silicon + Intel), Windows (x64), Linux (x64)
- 동일 코드베이스에서 3 플랫폼 빌드
- 플랫폼별 인스톨러: .dmg (macOS), .msi + .exe (Windows), .AppImage + .deb (Linux)

### NFR-5: Size
- 인스톨러 크기: 10MB 이하
- 디스크 사용: 설치 후 30MB 이하


## 6. Protocol Contract

ant-desktop은 `@ant/shared` 패키지에 정의된 타입을 프로토콜 계약으로 준수한다. 소스 위치: `packages/ant-shared/src/figma.ts`

### 6.1 WebSocket 메시지 타입 (BridgeMessage)

| 메시지 | 방향 | 용도 |
|--------|------|------|
| `BridgeRegisterMessage` | Ant Desktop → cloud | 연결 등록 (userId, machineId, capabilities) |
| `BridgeHeartbeatMessage` | Ant Desktop → cloud | 30초 간격 생존 확인 |
| `BridgeDisconnectMessage` | Ant Desktop → cloud | 정상 종료 알림 |
| `MCPRequestMessage` | cloud → Ant Desktop | MCP 도구 호출 요청 |
| `MCPResponseMessage` | Ant Desktop → cloud | MCP 도구 호출 결과 |

`BridgeRegisterMessage.userId`는 Ant Desktop이 저장된 JWT 페이로드를 **검증 없이 디코딩**하여 ant-cli `JwtService`와 동일한 사용자 ID 클레임(예: `sub`)에서 채운다. 서명·만료 검증은 Cloud(WebSocket 수락 시)가 수행한다.

### 6.2 상수

| 상수 | 값 | 용도 |
|------|------|------|
| `BRIDGE_WS_PATH` | `/bridge/ws` | WebSocket 엔드포인트 경로 |
| `BRIDGE_HEARTBEAT_INTERVAL_MS` | `30_000` | heartbeat 간격 |
| `BRIDGE_HEARTBEAT_TIMEOUT_MS` | `90_000` | Cloud 측 세션 타임아웃 (Ant Desktop 미수신 시) |
| `BRIDGE_MCP_REQUEST_TIMEOUT_MS` | `30_000` | MCP 요청 타임아웃 |
| `BRIDGE_WS_MAX_MESSAGE_BYTES` | (예: `16_777_216`) | WebSocket 단일 프레임 상한 (16MiB, 구현 시 조정) |

> 위 상수들은 아직 `@ant/shared`에 정의되지 않은 것이 있다. ant-cli 측 Bridge 서버 구현 시 함께 추가될 예정이며, ant-desktop은 해당 상수를 참조하거나 동일한 값을 하드코딩한다.

**프로토콜 버전:** MVP에서는 `BridgeRegisterMessage`에 별도 `protocolVersion` 필드나 협상 단계를 두지 않는다. 불일치 시 JSON 파싱 실패나 런타임 에러로 드러나며, 필요해지면 `@ant/shared`와 서버/클라이언트에 필드를 추가한다.

### 6.2a 대용량·바이너리 응답

`get_screenshot`, `get_design_context` 등은 이미지·대용량 텍스트를 포함할 수 있다. WebSocket 페이로드는 **UTF-8 JSON 텍스트 프레임**으로 전송한다. MCP가 반환하는 바이너리는 **Base64 문자열**로 인코딩되어 `MCPResponseMessage.result` 트리 내부에 포함되며, 단일 메시지 크기는 `BRIDGE_WS_MAX_MESSAGE_BYTES`를 초과하지 않도록 Cloud·Ant Desktop 양쪽에서 제한한다. 초과 시 에러 응답 또는 청크/스트리밍 정책은 ant-cli Bridge 구현 시 별도 정의한다.

### 6.2b 동시 MCP 요청

Figma Desktop MCP가 동시 요청을 안정적으로 처리한다는 보장이 없으므로, **MVP에서는 Ant Desktop 측에서 MCP 호출을 단일 뮤텍스로 직렬화**한다. Cloud에서 여러 `MCPRequestMessage`가 동시에 오면 큐에 넣어 순차 처리한다. 병렬화가 필요해지면 Figma 동작 검증 후 Phase 2에서 정책을 재검토한다.

### 6.2c 배포 범위 (Cloud 전용)

`ANT_SERVER_MODE=local` 등 **로컬에서 API·워커·Redis가 동일 머신**에 있을 때는 Design Job이 `127.0.0.1:3845`에 직접 접근할 수 있어 **ant-desktop이 필수는 아니다**. ant-desktop은 **Ant Cloud(원격 워커)** 환경에서만 사용자 데스크톱과 Cloud 사이의 브릿지 역할을 한다.

### 6.3 MCP 도구 (FigmaMCPTool)

| 도구 | 용도 |
|------|------|
| `get_metadata` | 노드 트리 구조 (XML) |
| `get_design_context` | 코드 + 스크린샷 + 컨텍스트 힌트 |
| `get_screenshot` | 노드 스크린샷 (이미지) |
| `get_variable_defs` | 디자인 변수 정의 |


## 7. Out of Scope

- Figma REST API 직접 호출 (ant-cli가 담당)
- 디자인 파일 편집/쓰기 (읽기 전용 프록시)
- Figma 계정 인증/OAuth (ant-ui/ant-cli가 담당)
- 사용자의 디자인 데이터 캐싱 또는 저장
- 모바일 지원
- **로컬 올인원 모드에서의 Bridge 대체:** 로컬 모드 MCP 경로는 ant-cli 워커 직접 호출로 처리하며, Ant Desktop 앱은 설치·연동 대상에서 제외 가능


## 8. Release Strategy

### Phase 1 (MVP)
- macOS + Windows 지원
- 트레이 상주 + WebSocket 연결 + MCP 릴레이
- 딥링크 인증
- 수동 설치 (다운로드 페이지)

### Phase 2
- 자동 업데이트
- OS 자동 시작
- Linux 지원
- 연결 진단 UI

### Phase 3
- 설치 이력/통계
- 다중 Ant Cloud 인스턴스 연결
- MCP 요청 로깅/디버그 모드
