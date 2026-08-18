# 파일 전환 시 4초가량 단축키·힌트 무반응, 탭 닫기 지연 (수정)

> 손 QA 1차 발견 6건 중 항목 4(사용자 실기 보고, 2026-08-18). 진단
> `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md` §1 항목 4·§2.4.

## 증상

파일을 열고(정확히는 다른 언어의 파일로 전환하고) "코드베이스 읽기" 류의 로딩이 끝나기 전까지,
단축키·힌트가 일정 시간(약 4초) 반응하지 않는다. 탭을 닫으려 해도 같은 시간만큼 지연된다.

## 원인

세 가지가 겹쳐서 증상을 만든다.

1. **`lsp_stop`이 전역 뮤테이션 락을 쥔 채 4초 고정 sleep 을 한다.** `AppState::begin_mutation`
   은 앱 전체에서 단 하나뿐인 `tokio::Mutex` 다. 수정 전 `lsp_stop`/`lsp_restart` 는 이 가드를
   함수 **전체**(호출 시작부터 끝까지)에 걸어 두고, 그 안에서 `shutdown_entry` 가
   `shutdown`→(2초 고정 sleep)→`exit`→(2초 고정 sleep)→kill 을 순서대로 수행했다
   (`LSP_SHUTDOWN_TIMEOUT_MS = 2_000`, 두 번). 즉 언어 서버 하나를 정상 종료시키는 데만 **최소
   4초 동안 전역 락이 잡혀 있었다** — 실제 프로세스가 그보다 훨씬 빨리 죽어도 무조건 4초를 다
   기다렸다.
2. **`layout_*`·`file_save`·`lsp_spawn` 등 대부분의 뮤테이션 커맨드가 같은 락 뒤에 줄을 선다.**
   `begin_mutation` 을 쓰는 모든 커맨드가 이 하나의 락을 공유하므로, `lsp_stop` 이 락을 쥔 4초
   동안 탭 닫기·레이아웃 변경·파일 저장·새 LSP 세션 시작이 전부 큐잉된다 — 단축키·힌트가
   "무반응"으로 보이는 것도, 탭 닫기가 지연되는 것도 같은 원인이다.
3. **프론트가 파일을 바꿀 때마다 세션을 파괴하고 새로 만든다.** `use-lsp-session.ts` 의 effect
   의존성 배열에 `path` 가 들어 있어, 파일을 전환할 때마다 이전 파일의 LSP 세션을
   `releaseLspSession` 으로 즉시 해제(refCount 0 → 그 자리에서 dispose)하고 새 파일용 세션을
   새로 `acquireLspSession` 했다. 재사용 가능한 세션(`shares_sessions`)이라도 매 전환마다
   파괴→재생성이 일어나, 매번 위 1·2번 경합을 다시 밟았다.

레이아웃 뮤테이션이 pessimistic(서버 응답을 받은 뒤에야 UI 에 반영)이라, 락 뒤에 줄 선 시간이
그대로 "화면이 안 바뀐다"는 체감으로 이어진다. `lsp_send`(메시지 전송)는 원래부터 이 가드를 쓰지
않는 문서화된 예외였다.

## 수정

Rust 와 프론트 양쪽을 함께 고쳤다(사용자 승인 완료 — 계약 §2.4).

**Rust (`src-tauri/src/domain/lsp/commands.rs`, `src-tauri/src/infra/lsp_proc.rs`)**

- `lsp_stop`/`lsp_restart` 를 재구성해 **가드는 동기 북키핑(refcount 갱신·`LspStore` 에서 엔트리
  선제거)까지만** 쥐고, `shutdown_entry().await` 는 **가드를 놓은 뒤** 실행한다. 엔트리를 먼저
  스토어에서 제거하므로, 가드가 없는 동안에도 다른 `lsp_spawn`/`lsp_stop`/`lsp_send` 가 이
  세션을 다시 붙잡을 수 없다(`find_reusable_entry`/`find_entry` 가 못 찾는다).
- `shutdown_entry` 의 고정 sleep 을 **프로세스 종료 폴링**으로 교체했다. 신설
  `wait_for_process_exit`(50ms 간격, `LSP_SHUTDOWN_POLL_INTERVAL_MS`)가
  `LspProcHandle::is_exited()`(신설 — 프로세스 wait 루프가 실제 종료를 감지하면 세팅하는
  `AtomicBool`)를 폴링하다 종료를 감지하면 즉시 반환한다. 상한은 기존
  `LSP_SHUTDOWN_TIMEOUT_MS`(2초)를 그대로 유지 — 응답하지 않는 서버에 대한 방어는 그대로다.
- 비가드 구간에서 두 `lsp_stop` 호출이 겹치는 경우(예: React effect cleanup 이 수동 종료와
  경합)는 안전하다 — 먼저 실행된 쪽이 스토어에서 엔트리를 제거하므로, 나중 호출은 `find_entry` 에서
  `NotFound` 로 빠르게 실패한다(이전에는 둘 다 `shutdown_entry` 까지 도달해 죽은 프로세스에
  중복으로 shutdown/exit/kill 을 보내는 것으로 우연히 무해했다).

**프론트 (`src/widgets/editor-pane/lsp-session-registry.ts`, `use-lsp-session.ts`)**

- `use-lsp-session.ts` 의 `path` 의존성은 유지한다(파일 단위 문서 attach/detach 는 그대로
  필요) — 대신 세션 자체의 dispose 를 미룬다. `releaseLspSession` 이 refCount 0 이 되면 즉시
  dispose 하지 않고 `LSP_SESSION_DISPOSE_GRACE_MS`(5초) 뒤에 dispose 하는 타이머를 건다. 그
  유예 시간 안에 `acquireLspSession` 이 다시 오면(같은 프로젝트+언어의 다른 파일로 전환) 타이머를
  취소하고 기존 세션을 그대로 재사용한다 — 파일 탐색기 브라우징·⌘P 연속 열기 같은 흔한 패턴에서
  세션 재스폰·재인덱싱이 사라진다.
- **접합부 수정(Phase D)**: 유예 도입 직후엔 즉시-정리 함수(`flushLspSessionDisposal`)가 정의만
  되고 어디서도 호출되지 않아, 프로젝트를 닫거나 앱이 종료돼도 유예 중인 세션이 그대로 5초를 마저
  기다린 뒤에야 정리됐다(`project_close` 는 `LspStore` 를 건드리지 않으므로 프론트의 유예 타이머가
  유일한 정리 경로였다). `ipc-sync-provider.tsx` 의 `events.projectClosed` 핸들러에
  `flushLspSessionsForProject(projectId)`, `HotExitFlushProvider` 에 `flushAllLspSessions()`(신설
  `lsp-session-flush-registry.ts` 경유 — `hot-exit-flush-provider.tsx` 가 monaco worker 를 import
  하는 `lsp-session-registry.ts` 를 직접 참조하면 `bun test` 정적 임포트 그래프 해석이 깨진다)를
  배선해 두 경로 모두 유예 중인 세션을 즉시 정리하도록 마감했다.

## 검토 초점(확인 완료)

- 멀티 윈도우에서 같은 세션에 대한 중복 `lsp_stop` 동시 호출 — 위 "비가드 구간" 항목 참조,
  스토어 제거 순서로 안전.
- root refcount 경합 — 가드 재구성은 root 카운팅 로직 자체를 바꾸지 않았다(남은 root 있으면 여전히
  가드 안에서 조기 반환).
- didClose→didOpen 순서 — 4초 락이 가려주던 경합이 이론상 표면화될 수 있으나, `lsp_send` 자체는
  원래부터 비가드였고 세션 유예로 didClose 자체가 훨씬 덜 발생하게 됐다.

## 대상 파일

- `src-tauri/src/domain/lsp/commands.rs` — `lsp_stop`/`lsp_restart` 가드 재구성,
  `wait_for_process_exit` 신설
- `src-tauri/src/infra/lsp_proc.rs` — `LspProcHandle::is_exited()` 신설
- `src/widgets/editor-pane/lsp-session-registry.ts` — dispose 유예, `flushLspSessionDisposal`/
  `flushLspSessionsForProject`/`flushAllLspSessionDisposals` 신설
- `src/widgets/editor-pane/lsp-session-flush-registry.ts` — 신설(hot-exit 경로 간접 참조용)
- `src/app/providers/ipc-sync-provider.tsx`, `src/app/providers/hot-exit-flush-provider.tsx` —
  접합부 배선(Phase D)
