이 문서는 [English](AGENTS.md) | **한국어** | [日本語](AGENTS.ja.md) 로도 이용 가능합니다.

# LUX (Linalab Unity X) 에이전트 가이드

LUX는 통합 유니티 에디터 AI 어댑터이자 자동화 툴킷입니다. 독립적인 유니티 패키지이며, 단독 실행 애플리케이션이 아닙니다.

## 코드베이스 구조

| 경로 | 설명 | 어셈블리 / 기술 스택 |
| :--- | :--- | :--- |
| `LuxEditor/` | 유니티 에디터 C# 스크립트 | `Linalab.LuxEditor` |
| `AiBridgeEditor/` | TCP 서버 및 프로토콜 | `Linalab.UnityAiBridge.Editor` |
| `UnityGitEditor/` | Git 통합 기능 | `Linalab.UnityGit.Editor` |
| `CodexImage/` | 이미지 생성 파이프라인 | C# 에디터 스크립트 |
| `RustGateway~/` | Rust CLI 및 웹 서버 | Axum 0.7, React 19 |
| `McpHelper~/` | Node.js MCP 헬퍼 | Node.js |
| `Skills/lux-unity/` | 핵심 AI 스킬 | Manifest + SKILL.md |
| `*Tests/` | C# 및 Rust 테스트 스위트 | NUnit / Cargo |

## 주요 컨벤션

### Rust (`RustGateway~/`)
- Axum 0.7, tokio 1, clap 4.5, anyhow, serde를 사용합니다.
- 에러 핸들링: 로직에는 `anyhow`를 사용하고, 사용자 출력에는 `eprintln`을 사용하세요.
- `TODO`, `FIXME`, `HACK` 주석을 남기지 마세요.
- 새로운 엔드포인트는 `server.rs` 또는 `gateway_cli_smoke.rs`에 테스트가 포함되어야 합니다.
- 서버 수명주기: graceful shutdown이 포함된 idle 타임아웃(`--idle-timeout`), 하트비트(`POST /api/heartbeat`), 헬스 체크(`GET /api/health`)를 지원합니다.

### TypeScript (`RustGateway~/ui-src/`)
- React 19와 TypeScript strict 모드를 사용합니다.
- 함수형 컴포넌트와 훅(hooks)을 사용하세요.
- API 훅에 mock 데이터나 fallback 데이터를 넣지 마세요.
- 상태 관리: `useState`, `useRef`, `useCallback`, `useEffect`를 사용합니다.

### C# (에디터 디렉토리)
- 네임스페이스: `UnityEditor`. 어셈블리: `Linalab.LuxEditor`.
- 모든 클래스 이름에는 `Lux` 접두사를 붙여야 합니다.
- 파일이 큰 경우 partial 클래스를 사용하여 로직을 그룹화하세요.
- 거대한 C# 파일은 partial 클래스로 분리되어 있습니다 (예: `LuxAutomationGateway`는 약 10개 파일로, `LuxWebRTCProducer`는 약 7개 파일로 분리).
- 테스트: `*Tests/Editor/` 디렉토리에서 NUnit `[Test]`를 사용하세요.

### 스킬 (Skills)
- 핵심 스킬은 `Skills/`에 위치하며 삭제할 수 없습니다.
- 구조: `manifest.json`, `SKILL.md`, `references/`로 구성됩니다.

## 안티 패턴 (하지 마세요)
- C# 클래스 이름에서 `Lux` 접두사를 제거하지 마세요.
- API 훅에 mock 데이터나 fallback 데이터를 추가하지 마세요.
- TypeScript strict 모드를 비활성화하지 마세요.
- CLI에서 핵심 스킬 보호 기능을 제거하지 마세요.
- `cargo test`를 실행하지 않고 커밋하지 마세요.
- 테스트를 통과시키기 위해 테스트 파일 자체를 수정하지 마세요.
- 호스트 프로젝트(neon-glitch)를 LUX의 일부로 취급하지 마세요.

## 검증 명령어

### Rust
```bash
cd RustGateway~ && cargo build && cargo test
```

### TypeScript
```bash
cd RustGateway~/ui-src && npx tsc --noEmit
```

### CLI 도움말
```bash
cd RustGateway~ && cargo run -- skill install --help
cd RustGateway~ && cargo run -- serve --help
```

### C#
LSP 진단을 사용하여 검증하세요. 별도의 CLI 빌드 명령어는 제공되지 않습니다.
