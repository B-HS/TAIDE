# ADR-0004 — 상태 소유권: Rust 코어가 단일 출처, view 는 프로젝션

- 상태: 승인 (사용자 요구사항, 2026-08-06)
- 관련: `docs/PRD.md` §5 원칙 1·NFR-2·NFR-3, `docs/architecture.md`, `docs/ipc-contract.md`

## 맥락

사용자 요구: "정보는 Rust 단이 가지고 view 는 진짜 표시만 해서, view reload 등으로도 누수 없는
최적화된 프로그램". TS/WebView 는 장시간 구동 시 누수·비대화 위험이 있고, IDE 는 며칠씩 켜 두는 앱이다.

## 결정

1. **도메인 상태(프로젝트, 탭/스플릿 레이아웃, 터미널 세션, Git 상태, LSP 세션, 설정/테마)의
   단일 출처는 Rust 코어다.** Rust 의 `AppState`(Tauri managed state)에 상주한다.
2. view(React)는 **프로젝션(표시 계층)** 이다:
   - 읽기: IPC query 명령으로 조회 (TanStack Query 로 캐싱 — ADR-0008)
   - 쓰기: IPC mutation 명령으로 의도(intent)만 전달 — view 가 도메인 상태를 직접 변형하지 않는다
   - 갱신: Rust 가 상태 변경 시 이벤트를 발행하고 view 는 해당 query 를 invalidate
3. view 로컬 상태는 **순수 표시 상태**(호버, 포커스, 드래그 중, 입력 중 텍스트 등)로 한정한다.
4. **view reload = 전체 UI 복원**이 항상 성립해야 한다. 리로드 후 view 는 Rust 상태를 다시 조회해
   동일 화면을 재구성한다. pty·LSP·watcher 프로세스는 Rust 소유이므로 리로드에 영향받지 않는다.
5. 영속화(디스크 저장)도 Rust 만 수행한다(ADR-0009). view 는 localStorage 등에 도메인 상태를 저장하지 않는다.

## 예외 (성능상 view 직결 경로)

- **에디터 버퍼**: 타이핑 문자는 Monaco 모델이 1차 보유(키입력마다 IPC 왕복 불가).
  dirty 파일 내용은 debounce 로 Rust 에 미러링해 reload 시 미저장 변경을 복원한다(`features/editor.md`).
- **터미널 화면**: pty 출력은 Rust→view 스트림으로 흘리되, 스크롤백 원본(ring buffer)은 Rust 가 보유해
  reload 시 재생(replay)한다(`features/terminal.md`).

## 결과

- IPC 계약(`docs/ipc-contract.md`)이 아키텍처의 핵심 문서가 된다. 명령·이벤트·타입을 전부 계약으로 고정하고
  Rust→TS 타입 자동생성으로 드리프트를 방지한다(ADR-0011).
- view 컴포넌트는 구독 해제(unlisten, dispose)만 정확히 하면 누수 표면이 크게 줄어든다.
  각 feature 문서에 "수명주기" 절을 의무화한다.
- 비용: 모든 기능이 query/mutation/event 3종 계약을 정의해야 해 초기 코드량이 늘어난다 — 감수한다.
