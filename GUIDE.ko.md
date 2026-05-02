이 문서는 [English](GUIDE.md) | **한국어** | [日本語](GUIDE.ja.md) 로도 이용 가능합니다.

# LUX (Linalab Unity X) 개발자 가이드

LUX는 유니티 에디터와 AI 코딩 도구(Claude Code, OpenAI Codex, OpenCode 등)를 연결하는 통합 어댑터이자 자동화 툴킷입니다. 이 가이드는 LUX의 구조를 이해하고 활용하려는 개발자를 위해 작성되었습니다.

## 1. 소개

LUX는 유니티 에디터 내부의 작업을 외부 AI 도구가 제어할 수 있도록 브릿지 역할을 수행합니다. 단순한 명령 전달을 넘어, 웹 기반의 컨트롤 서피스, 시각적 파이프라인 에디터, WebRTC를 이용한 원격 제어 기능을 제공하여 유니티 개발 환경의 자동화 수준을 높입니다.

## 2. 설치

### 유니티 패키지 설치
1. 유니티 프로젝트의 `Packages/manifest.json`에 LUX 패키지를 추가합니다.
2. `com.unity.webrtc` 패키지가 설치되어 있는지 확인하십시오 (원격 스트리밍 기능에 필요).

### Rust CLI 설치
LUX의 게이트웨이와 CLI 도구는 Rust로 작성되었습니다.
```bash
cd Packages/com.linalab.lux/RustGateway~
cargo build --release
# 빌드된 실행 파일을 PATH에 등록하거나 직접 실행합니다.
./target/release/lux --help
```

## 3. 빠른 시작

5분 안에 LUX 서버를 켜고 유니티와 연결하는 방법입니다.

1. **유니티 에디터 실행**: 프로젝트를 열고 `Window > Linalab > Lux Workbench`를 엽니다.
2. **서버 실행**: 터미널에서 다음 명령어를 입력합니다.
   ```bash
   # 기본 실행 (30분 idle 시 자동 종료)
   lux serve --port 8080

   # idle 타임아웃 변경 (0 = 비활성화)
   lux serve --port 8080 --idle-timeout 60
   ```
3. **웹 UI 접속**: 브라우저에서 `http://localhost:8080`에 접속합니다.
4. **연결 확인**: `Tools > Linalab > Lux > Server Status` 창에서 서버 상태를 확인합니다. 초록색이면 연결됨, 노란색이면 서버 미실행, 빨간색이면 에러입니다.
5. **서버 수명주기**: 서버는 Unity 에디터가 활동을 유지하는 동안 계속 실행됩니다. 30분간 활동이 없으면 자동으로 종료됩니다 (`--idle-timeout`으로 조절 가능).

## 4. 아키텍처

LUX는 여러 핵심 모듈로 구성되어 있습니다.

| 모듈 | 설명 |
| :--- | :--- |
| **LuxEditor** | 메인 어댑터. 워크벤치 윈도우, 자동화 게이트웨이, WebRTC 프로듀서 포함. |
| **AiBridgeEditor** | AI 도구와의 통신을 위한 TCP 서버 및 프로토콜 핸들러. |
| **UnityGitEditor** | 유니티 내부에서 Git 상태 확인, 스테이징, 브랜치 관리를 지원. |
| **CodexImage** | 노드 기반 이미지 생성 파이프라인 엔진. |
| **RustGateway** | Axum 기반의 웹 서버 및 CLI. 웹 UI와 API 엔드포인트 제공. |
| **Skills** | 유니티 제어를 위한 핵심 스킬 세트 및 참조 문서. |

## 5. CLI 레퍼런스

`lux` 커맨드라인 도구를 통해 서버 관리 및 유니티 제어가 가능합니다.

| 명령어 | 설명 |
| :--- | :--- |
| `lux serve` | 웹 서버 및 게이트웨이 실행. |
| `lux compile` | 유니티 프로젝트 컴파일 실행. |
| `lux test` | 플레이모드 및 에디트모드 테스트 실행. |
| `lux unity status` | 유니티 에디터 연결 상태 확인. |
| `lux unity screenshot` | 현재 에디터 화면 캡처. |
| `lux unity logs` | 유니티 콘솔 로그 스트리밍. |
| `lux unity dynamic-code` | 유니티 내부에서 C# 코드 동적 실행. |
| `lux skill list` | 설치된 스킬 목록 확인. |
| `lux skill install <name>` | 새로운 스킬 설치. |

## 6. 웹 UI

게이트웨이 서버 실행 후 브라우저를 통해 다음 기능을 사용할 수 있습니다.

- **AI 터미널 (AITerminal)**: Claude, Codex 등 다양한 AI 도구를 전환하며 사용.
- **파이프라인 에디터 (NodeEditor)**: ReactFlow 기반의 시각적 도구로 이미지 생성 워크플로우 설계.
- **원격 뷰어 (RemoteViewer)**: WebRTC를 통해 유니티 화면을 실시간으로 보며 마우스/키보드 입력 전달.
- **세션 매니저**: 현재 활성화된 AI 도구 세션 및 명령 이력 관리.

## 7. 스킬 시스템

스킬은 AI가 유니티를 제어하는 방법을 정의한 단위입니다.

- **코어 스킬**: `lux-unity` 스킬이 기본 포함되어 컴파일, 테스트, 로그 확인 등을 지원합니다.
- **스킬 관리**:
  ```bash
  # 스킬 정보 확인
  lux skill info lux-unity
  # 외부 스킬 설치
  lux skill install my-custom-skill --source https://github.com/user/repo
  ```

## 8. API 레퍼런스

외부 도구와의 연동을 위한 주요 엔드포인트입니다.

| 엔드포인트 | 메서드 | 설명 |
| :--- | :--- | :--- |
| `/health` | GET | 서버 상태 및 프로토콜 버전 확인. |
| `/api/health` | GET | 서버 uptime 및 상태 리포팅. |
| `/api/heartbeat` | POST | Unity 에디터에서 주기적 호출, idle 타이머 갱신. `{ "status": "alive", "uptime_seconds": N }` 반환. |
| `/api/sessions` | GET/POST | AI 도구 세션 관리. |
| `/api/graphs` | GET/POST | 파이프라인 그래프 저장 및 로드. |
| `/api/tools/execute` | POST | 특정 AI 도구에 명령 전달. |
| `/api/remote/signaling` | POST | WebRTC 시그널링 데이터 교환. |
| `/events` | WS | 실시간 이벤트 스트리밍 (WebSocket). |

## 9. 원격 접속 (WebRTC)

LUX는 유니티 화면을 웹 브라우저로 스트리밍합니다.

- **설정**: 유니티 에디터의 Lux Workbench에서 해상도와 프레임 레이트를 조절할 수 있습니다.
- **네트워크**: 로컬 네트워크 외부에서 접속하려면 STUN/TURN 서버 설정이 필요합니다. 게이트웨이 설정 파일에서 ICE 서버 정보를 입력하십시오.

## 10. 개발 가이드

### 테스트 실행
- **Rust**: `cargo test` (유닛 테스트 및 스모크 테스트 포함)
- **C#**: 유니티 Test Runner에서 `AiBridgeTests`, `LuxTests` 등을 실행합니다.

### 기여 방법
1. 새로운 기능을 추가할 때는 `LuxEditor` 모듈의 게이트웨이 정책을 먼저 확인하십시오.
2. 웹 UI 수정 시 `RustGateway~/ui-src` 경로의 React 컴포넌트를 수정합니다.
3. 변경 사항 적용 후 반드시 `lux test`를 통해 회귀 테스트를 수행하십시오.

## 11. 트러블슈팅

- **연결 실패**: 유니티 에디터가 실행 중인지, AI Bridge TCP 서버가 활성화되었는지 확인하십시오.
- **서버가 자꾸 종료됨**: `--idle-timeout 0`으로 idle 타임아웃을 비활성화하거나, Unity 에디터에서 Server Status 창이 열려 있는지 확인하십시오 (60초마다 heartbeat 전송).
- **WebRTC 화면 안 나옴**: `com.unity.webrtc` 패키지 버전 호환성을 확인하고, 브라우저의 콘솔 로그에서 시그널링 오류를 체크하십시오.
- **권한 오류**: 자동화 명령 실행 시 유니티 에디터에서 승인 팝업이 떠 있는지 확인하십시오.
- **TypeScript 에러**: `cd RustGateway~/ui-src && npx tsc --noEmit`으로 확인. strict 모드가 활성화되어 있으므로 타입 오류를 반드시 수정해야 합니다.
