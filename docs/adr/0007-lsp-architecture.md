# ADR-0007 — LSP 아키텍처: Rust 가 서버 관리, 클라이언트는 경량 경로

- 상태: 승인 (2026-08-06 — 결정 3 포함 사용자 확정: 자체 경량 클라이언트)
- 관련: `docs/PRD.md` FR-E, `docs/features/lsp.md`, `docs/research/lsp-servers.md`, `docs/research/monaco.md` §10

## 결정 1 — 서버 프로세스는 Rust 코어가 소유한다 (승인)

- spawn·stdio `Content-Length` 프레이밍·종료·크래시 재기동(supervisor) 전부 Rust `domain/lsp`.
  view 로는 파싱된 JSON-RPC 객체만 Channel 로 오간다(research monaco §10 Rust 예제 기반).
- 프론트에서 shell 권한으로 직접 spawn 하지 않는다(임의 인자 실행 표면 차단 — research lsp-servers 권장).

## 결정 2 — 1차 서버 선정 (승인, research 확정안)

| 언어 | 서버 | 비고 |
|------|------|------|
| JS/TS | **vtsls 0.3.0** | workspaceFolders 정식 지원·VSCode 동급 코드액션. TS7 `tsc --lsp` 는 옵션 백엔드로 추상화(7.1 후 재평가) |
| Rust | **rust-analyzer** (주간 릴리스) | 설정은 `initializationOptions` 에 prefix 제거 트리(가장 흔한 실수 주의). Cargo 워크스페이스당 프로세스 1개 |
| Python | **basedpyright** + `ruff server` | 타입/린트·포맷 분담. `typeCheckingMode: standard` 기본, 프로젝트 설정 존중 |
| Markdown | **marksman** | `.git`/`.marksman.toml` 필요. clientInfo 는 "TAIDE"(VSCode 위장 금지 — 기능 꺼짐) |

- 서버는 `LanguageServerSpec { id, cmd, args, initializationOptions, settingsNamespace, shareable }`
  데이터로 기술한다 — 플러그인(ADR-0010)이 같은 스키마로 언어를 추가한다.
- 프로세스 모델: 세션 키 = (서버 id × 폴더 집합). workspaceFolders 지원 서버(vtsls·basedpyright·
  marksman)는 `didChangeWorkspaceFolders` 로 공유, rust-analyzer·TS7 은 루트당 분리.
- 배포: **로컬 감지 우선 하이브리드** — 프로젝트 로컬 → PATH/rustup → TAIDE 관리 디렉토리 →
  온디맨드 다운로드(SHA256 고정). 앱 번들에 동봉하지 않는다(용량·업데이트 종속 회피).
  미설치 시 언어 기능 없이 동작 + 설치 안내 UI.

## 결정 3 — 에디터측 클라이언트 경로: 자체 경량 클라이언트 (승인)

후보 (research monaco §10):

| 경로 | 평가 |
|------|------|
| (A) monaco 0.56 내장 `monaco.lsp` | `initialize` 의 `rootUri: null` 하드코딩 — 워크스페이스 서버(vtsls·rust-analyzer) 사용 불가. **기각** |
| (B) monaco-languageclient 10.7 | 완전한 클라이언트지만 monaco-editor 를 codingame 포크로 교체(서비스 오버라이드 30+ 패키지), 버전 정합 취약(의존 codingame 25.x vs 최신 36.x). 무겁다 |
| (C) **자체 경량 클라이언트 (추천)** | Monaco provider API(`registerCompletionItemProvider` 등)에 LSP 요청을 어댑팅. `vscode-languageserver-protocol` 타입만 의존. 초기 구현 비용은 있지만 번들·수명주기·버전을 TAIDE 가 완전 통제 — 누수 없는 최적화 원칙에 부합 |

- (C) 확정. 구현 범위·어댑터 목록은 `features/lsp.md` §3 이 정본. 필요 기능이 전부 Monaco
  provider 표면에 존재함을 research 로 확인했다(diagnostics 는 `setModelMarkers`).

## 공통 정책 (승인)

- **positionEncoding 협상 필수**(기본 UTF-16 — 한글/이모지 오프셋 사고 방지).
- **진단은 한 경로만 소비**: pull 지원 서버(rust-analyzer·basedpyright·TS7)는 pull 로 통일하고
  push 를 끄거나 무시. push 전용(vtsls·marksman)은 push 소비.
- LSP 메시지 스트림은 Channel(ADR-0008 §3). didOpen/didChange 는 incremental sync.

## 구현 노트 (2026-08-28 현행화)

- `vscode-languageserver-protocol` 타입 의존도 도입하지 않았다 — JSON-RPC/LSP 타입은
  `src/shared/lib/lsp/protocol.ts` 에 자체 정의(필요 표면만). Monaco 어댑터는 20종으로 확장됐다.
