# T0 감사 24항목 구현에 대한 어드버서리얼 검증 — confirmed 3건 근본 수정 + minor 6건 처리

> 계약 `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` 구현물(§2, 아직 미커밋 워킹트리)을
> 대상으로 한 적대적 재검증(코드 리뷰 에이전트) 결과 confirmed 3건·minor 6건을 받아 처리한 세션의
> 기록. 개별 항목의 수정/기각 근거는 아래 각 절, 종합 목록은 `## 결과 요약`.

## §1 confirmed — `project_close` 의 `forbid_directory` 는 재오픈을 영구히 막는다 (되돌림)

### 증상

T0 #15(`docs/bug/2026-08-18-audit-t0-fixes.md` 의 15번 행)가 추가한
`project_close` 의 `asset_protocol_scope().forbid_directory(root, true)` 호출 이후, 같은 프로젝트를
닫았다 같은 세션에서 다시 열면 `project_open` 이 재호출하는 `allow_directory` 가 아무 효과가
없다 — 해당 루트 하위 파일은 `convertFileSrc`(`asset://`) 로 다시는 읽히지 않는다(video/audio
미리보기가 계속 빈 화면). 중첩 프로젝트(부모를 닫으면 아직 열려 있는 자식도 함께 막힘) 도 같은
근본 원인으로 깨진다.

### 원인

Tauri 2.11.5 의 `tauri::scope::fs::Scope`(`asset_protocol_scope()` 가 반환하는 타입)는
`allowed_patterns`/`forbidden_patterns` 두 `HashSet` 을 감싸는데, 공개 API 가
`allow_directory`/`allow_file`/`forbid_directory`/`forbid_file`(전부 add-only)뿐이고 **forbidden
패턴을 제거하는 API 가 없다**(`tauri-2.11.5/src/scope/fs.rs` 실물 확인 — `forbid_directory` 는
`forbidden_patterns.insert(...)` 만 한다). `is_allowed` 는 forbidden 을 먼저 검사해 매치되면
allowed 여부와 무관하게 즉시 `false` 를 반환하고(`forbid_directory` 자체의 doc 도 "takes
precedence... denied always" 라고 명시), `Scopes`(앱 전역 상태)의 `asset_protocol` 필드는
`pub(crate)` 라 스코프 객체 자체를 교체할 수도 없다. 즉 **한 번 forbid 된 경로는 앱을 재시작하기
전까지 그 세션 동안 영구히 막힌다** — "닫을 때 회수, 열 때 재부여"라는 #15 의 설계 자체가 이
버전의 Tauri API 로는 구현 불가능하다.

대안(자체 `asset` URI 스킴 프로토콜을 통째로 재구현해 앱 상태 기반으로 매 요청마다 검사)도
검토했으나, `tauri::protocol` 모듈 전체가 `pub(crate)`(비공개)라 Tauri 의 기존 파일 서빙·HTTP
range(비디오 탐색) 로직을 재사용할 수 없어 처음부터 다시 구현해야 한다 — 이는 이번 수정의 범위를
크게 벗어나는 아키텍처 변경이고, 앱을 직접 구동해 webview 레벨에서 검증할 수 없는 상태로는 안전하게
투입할 수 없다고 판단했다.

### 수정

`forbid_directory` 호출과, 그 호출에만 쓰이던 `closed_root` 지역변수를 `project_close` 에서
제거했다(`src-tauri/src/domain/project/commands.rs`) — #15 이전의 동작(닫은 프로젝트 트리도 그
세션 동안은 `asset://` 로 계속 읽힘)으로 되돌아간다. 이 표면 자체를 되돌리지 않고 막으려면 Tauri
의 asset 스코프 메커니즘을 우회하는 별도 아키텍처가 필요하므로, T1 후속 트랙으로 남긴다(§1 하단
"후속" 참조).

`docs/bug/2026-08-18-audit-t0-fixes.md` 15번 행에 취소선 처리 + 이 문서로 교차 참조를 남겼다.

### 후속 (범위 밖)

재오픈과 양립하는 형태로 이 표면을 다시 좁히려면, Tauri 의 `Builder::register_uri_scheme_protocol`
로 `"asset"` 스킴을 앱 레벨에서 직접 재등록(등록되면 Tauri 내장 asset 프로토콜은 스킵된다)하고,
요청마다 현재 열린 프로젝트 목록(`AppState::projects`, 이미 open/close 마다 정확히 갱신됨)을 직접
조회해 검사하는 방식이 유일한 재사용 가능 경로로 보인다 — 단 파일 서빙·mime·HTTP range(비디오
탐색) 로직을 전부 새로 작성해야 해서 T1 아키텍처 작업으로 분류해야 한다.

## §2 confirmed — `editor-pane.tsx` 의 `isConflicted` 가 절대경로 탭 path 를 상대경로 `row.path` 와 비교한다

### 증상

T0 #18(git `StatusRow`/`CommitFile` 에 절대경로 동봉, `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md`
§1 결정 8)로 git 패널·탐색기 양쪽이 파일 탭을 절대경로로 열도록 이관됐지만, `editor-pane.tsx` 의
충돌 표시 판정(`isConflicted`)만 이관에서 빠졌다 — `StatusRow.path`(저장소-상대)를 탭의 절대경로
`path` 와 비교해 항상 `false` 였다. 병합 충돌 파일을 열어도 인라인 충돌 데코레이션·gutter 충돌
해소 다이얼로그·"미해소 충돌 파일은 hunk 스테이징 차단" 가드가 전부 죽는다.

### 수정

비교 로직을 `src/widgets/editor-pane/conflict-status.ts` 의 `isPathConflicted(rows, absolutePath)`
로 추출(같은 위젯 폴더의 `breadcrumb-path.ts`/`lsp-session-registry.ts` 와 동일한 "순수 로직은
별도 파일 + 단위 테스트" 패턴)해 `row.absPath === path` 로 비교하도록 정정하고, `editor-pane.tsx`
는 이 함수를 호출만 한다. 회귀 테스트는 `conflict-status.test.ts`.

## §3 minor — `search_replace` 의 파일별 치환이 async 워커 스레드에서 블로킹 I/O 를 돈다

두 렌즈(contract·design)가 같은 결함을 보고했다. `search_replace` 의 파일별 루프가
`state.begin_mutation().await` 로 가드만 async 로 잡고, 실제 `service::replace_one_file`(동기
`std::fs` 읽기/쓰기)은 `spawn_blocking` 밖 — 커맨드 자신의 async 태스크 — 에서 그대로 실행했다.
대상 파일이 수백~수천 개인 전역 치환이면 그동안 하나의 tokio 워커 스레드가 블로킹 I/O 에 점유된다.

**타당해 수정**: 파일별 재획득 가드까지 통째로 `tokio::task::spawn_blocking` 안으로 옮겼다. async
`AppState::begin_mutation`(`tokio::sync::Mutex::lock().await`)은 블로킹 클로저 안에서 쓸 수
없으므로, `AppState::begin_mutation_blocking`(`Mutex::blocking_lock()`)을 신설했다 — 이 메서드는
async 실행 컨텍스트에서 호출하면 패닉하는 tokio 자신의 계약을 그대로 따르고, `spawn_blocking`
클로저는 정확히 그 계약이 요구하는 "블로킹 전용 스레드"이므로 안전하다. 가드가 있는 `&AppState`
를 얻으려면 `'static` 이 필요해(`spawn_blocking` 클로저 제약) `search_replace` 에 `app: AppHandle`
파라미터를 추가하고 클로저 안에서 `app.state::<AppState>()` 로 조회한다(`project_close` 등 기존
커맨드와 동일 패턴). 원격 `dispatch.rs` 의 `search_replace` 호출부도 `app.clone()` 인자를 추가해
맞췄다.

회귀 테스트: `state.rs` 에 `begin_mutation_blocking_은_비동기_begin_mutation_과_동일한_락을_공유한다`
— async 쪽이 `begin_mutation().await` 로 가드를 쥔 동안 `spawn_blocking` 안의
`begin_mutation_blocking()` 이 통과하지 못함을 확인해, 두 진입점이 같은 뮤텍스를 공유함(=
`file_save` 등 다른 뮤테이션과 실제로 직렬화됨)을 검증한다.

## §4 minor — 프로덕션이 더는 호출하지 않는 `service::replace` 를 삭제

`search_replace` 커맨드가 파일별 오케스트레이션(§3)으로 재구성된 뒤, 구 monolith
`service::replace`(대상 탐색 + 치환을 한 번에 하던 함수)는 프로덕션 호출부가 0건이었고, 자기
자신의 단위 테스트 7건에서만 불렸다. **타당해 수정**: 프로덕션 `pub fn replace` 를 삭제하고, 그
테스트 7건이 부르던 `replace(...)` 를, 프로덕션이 실제로 쓰는 분해
(`resolve_replace_targets`+`replace_one_file` 루프)를 그대로 미러링하는 테스트 전용 헬퍼로
교체했다 — 테스트 바디(호출부·assert)는 그대로 두고 무엇을 검증하는지만 프로덕션 경로에 맞춰
정정한 것이라 커버리지 손실이 없다.

## §5 minor — `commit-graph.tsx` 의 revert 충돌 파일 열기가 상대경로를 파일 도메인에 넘긴다

`onOpenFile`(git-panel-container 의 `openFileTab` 로 배선, `file_open` 경유 — #18 이 절대경로
전제로 재정의한 바로 그 sink)에 `RevertOutcome.conflictedPaths[0]`(`git2` 인덱스 경로, 저장소
상대)를 그대로 넘기고 있었다 — repo 루트와 활성 프로젝트 루트가 다른 경우(다중 루트, worktree,
백그라운드 프로젝트의 git 패널)에 엉뚱한 파일을 열거나 저장 대상이 어긋난다.

**타당해 수정**: `RevertOutcome` 에 `conflicted_abs_paths`(workdir 루트 join, `StatusRow.abs_path`/
`CommitFile.abs_path` 와 동일 패턴)를 신설해 `revert_commit`(Rust) 에서 함께 계산하고,
`commit-graph.tsx` 는 `conflictedAbsPaths[0]` 을 넘기도록 정정했다. `docs/data-model.md`·
`docs/ipc-contract.md` 갱신, `revert_commit는_충돌이_생기면_conflicted를_반환한다` 테스트에 절대
경로 assert 추가.

## §6 minor — wait-marker `localStorage` 가 앱 재시작을 가로질러 미회수 마커를 누적한다

T0 #11(`docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` §2.4)이 창 간 공유를 위해
모듈 스코프 `Map` 을 `localStorage` 로 교체하면서, 등록됐지만 `takeWaitMarkers` 로 회수되지 못한
마커(예: `taide --wait` 대기 중 앱 강제 종료)가 다음 실행으로 그대로 넘어가 무기한 쌓이는 부작용이
생겼다.

**타당해 수정**: `clearStaleWaitMarkersOnStartup`(`agent-wait-marker-registry.ts`)을 신설해
`AgentExternalOpenProvider`(마커를 등록하는 유일한 곳 — 메인 창 전용)의 마운트 effect 맨 앞에서
호출한다. `sessionStorage` 는 실제 앱 재시작(새 OS 프로세스 → 새 webview → 빈 `sessionStorage`)
에서는 비어 있지만 창 내 `location.reload()`(`window.reload` 커맨드)에서는 유지되는 성질을
이용해, "이 세션에서 이미 정리했는가"를 `sessionStorage` 플래그로 추적한다 — 그래서 진짜 재시작
직후에만 `localStorage` 의 마커 맵을 비우고, reload 로 그대로 열려 있는 탭들의 유효한 대기 마커는
건드리지 않는다. 회귀 테스트 2건(`agent-wait-marker-registry.test.ts`).

## §7 minor — `terminal.md` 가 "foregroundPid 관련 코드 자체가 없다"고 잘못 단언

`docs/features/terminal.md` §10 의 괄호 설명이 "탭 닫기 확인 다이얼로그가 미구현"이라는 사실과
"`foregroundPid` 관련 코드 자체가 코드베이스에 없다"는 주장을 함께 적었는데, 후자는 사실이 아니다
— `PtySession::foreground_pid`/`TerminalStore::foreground_pids` 는 실재하며 시스템 리소스
사용량(`system/commands.rs`)과 에이전트 감지 폴링(`lib.rs::poll_agents`, `agent/commands.rs`)
용도로 쓰인다. **타당해 수정**: 문구를 "탭 닫기용 프론트 확인 다이얼로그가 미구현"으로 한정하고,
`foreground_pid`/`foreground_pids` 가 별도 목적으로 실재한다는 사실을 덧붙였다.

## 결과 요약

| # | 등급 | 판정 | 대상 파일 |
|---|------|------|-----------|
| project_close forbid_directory 재오픈 영구 차단 | confirmed | 근본 수정(되돌림) | `src-tauri/src/domain/project/commands.rs` |
| editor-pane isConflicted repo-relative 비교 | confirmed | 근본 수정 | `src/widgets/editor-pane/{editor-pane.tsx,conflict-status.ts,conflict-status.test.ts}` |
| search_replace 블로킹 I/O가 async 워커에서 실행 (×2 렌즈) | minor | 수정 | `src-tauri/src/domain/search/commands.rs`, `src-tauri/src/domain/remote/dispatch.rs`, `src-tauri/src/state.rs` |
| service::replace 죽은 코드 | minor | 수정 | `src-tauri/src/domain/search/service.rs` |
| commit-graph revert 충돌 파일 상대경로 | minor | 수정 | `src-tauri/src/domain/git/{types.rs,service.rs}`, `src/widgets/git-panel/commit-graph.tsx`, `docs/data-model.md`, `docs/ipc-contract.md` |
| wait-marker localStorage 재시작 간 누적 | minor | 수정 | `src/entities/agent/agent-wait-marker-registry.ts`(+test), `src/app/providers/agent-external-open-provider.tsx` |
| terminal.md foregroundPid 오기술 | minor | 수정 | `docs/features/terminal.md` |

모든 항목을 수정으로 처리했다(기각 없음) — 판단 근거는 각 절 참조.

## 검증

- `bunx tsc --noEmit` 클린.
- `bun test` 1195/1195 그린(신규 6건: `conflict-status.test.ts` 4건 + `agent-wait-marker-registry.test.ts`
  신규 2건).
- `cargo fmt --all -- --check`/`cargo clippy --workspace --all-targets -- -D warnings` 클린.
- `cargo test --workspace` 936 lib(+1 신규 `begin_mutation_blocking` 테스트) + 6 session_restore +
  17 cli = 959/959 그린.
- `src/shared/api/bindings.ts` 는 `cargo test`(`typescript_바인딩을_생성한다`)로 재생성 —
  `RevertOutcome.conflictedAbsPaths` 신규 필드 외에, 이 세션 시작 시점에 이미 커밋 전 상태였던
  `CommitFile.absPath`/`origAbsPath` 필드 누락(T0 #18 워킹트리 작업이 이 파일을 마지막으로
  재생성한 이후 생긴 기존 드리프트로 보인다 — 이번 세션이 만든 결함은 아니다)도 함께 정정됐다.

## 대상 파일

Rust: `src-tauri/src/domain/project/commands.rs`, `src-tauri/src/domain/search/commands.rs`,
`src-tauri/src/domain/search/service.rs`, `src-tauri/src/domain/remote/dispatch.rs`,
`src-tauri/src/state.rs`, `src-tauri/src/domain/git/types.rs`, `src-tauri/src/domain/git/service.rs`.

프론트: `src/widgets/editor-pane/editor-pane.tsx`, `src/widgets/editor-pane/conflict-status.ts`(신설),
`src/widgets/editor-pane/conflict-status.test.ts`(신설), `src/widgets/git-panel/commit-graph.tsx`,
`src/entities/agent/agent-wait-marker-registry.ts`, `src/entities/agent/agent-wait-marker-registry.test.ts`,
`src/app/providers/agent-external-open-provider.tsx`, `src/shared/api/bindings.ts`(자동 생성).

문서: `docs/bug/2026-08-18-audit-t0-fixes.md`(15번 행 정정), `docs/data-model.md`,
`docs/ipc-contract.md`, `docs/features/terminal.md`.
