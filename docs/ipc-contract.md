# IPC 계약 (view ↔ Rust)

> 아키텍처 §4 의 상세. 타입 생성은 ADR-0011(tauri-specta), 전송 원칙 근거는
> `docs/research/tauri-v2.md`·`performance-memory.md`. **이 문서의 목록이 command·event 의 정본이며,
> 구현 시 추가·변경은 이 문서를 먼저 갱신한다.**
>
> **실측(2026-08-16)**: command **178종** — `src/shared/api/bindings.ts` 의 `__TAURI_INVOKE("...")` 전수
> (raw 3종 제외) = `src-tauri/src/domain/remote/dispatch.rs` 의 `IMPLEMENTED_JSON_COMMANDS` 배열 원소
> 수와 정확히 일치(파리티 테스트 `bindings와_dispatch_테이블은_커맨드_이름_집합이_일치한다` 가 강제).
> raw 채널 커맨드 3종(specta 밖, 아래 "raw 커맨드" 절)까지 합치면 총 **181종**. event 는 **25종**
> (`src-tauri/src/events.rs` 의 `#[tauri_specta(event_name = ...)]` 전수).

## 1. 공통 규칙

- 3종 패턴: **query**(조회, 부수효과 금지) / **mutation**(의도 전달 → Rust 가 상태 변경 + 이벤트 발행)
  / **event**(Rust→view 변경 알림, 소형 payload). 스트림은 **Channel**.
- 네이밍: command = `{domain}_{action}` snake_case, event = `{domain}:{event-kebab}`.
- 전송 매체 선택 (research 확정):

| 데이터 | 매체 |
|--------|------|
| 일반 조회/조작 | command (`Result<T, AppError>`) |
| 대용량 단발(파일 본문 > 수 MB) | `ipc::Response`(ArrayBuffer) 또는 Channel 청크 |
| 스트림(pty 출력·LSP 메시지·검색 결과) | `ipc::Channel` — pty 는 `InvokeResponseBody::Raw` |
| 상태 변경 알림(저빈도) | event — 대형 데이터 금지, "변경됨" 신호만 |

- 이벤트 payload 에는 가능하면 `revision`(단조 증가)을 실어 늦게 도착한 이벤트로 인한 stale 갱신을
  차단한다(revision gate — performance research §3).
- 에러: 전 command 는 `Result<T, AppError>` — `AppError { code, message, details? }`
  (thiserror + Serialize + specta Type, 코드는 `error.rs` 중앙화). 프론트는 코드 기반 분기 + 토스트.
- view 구독은 `useTauriEvent`(취소 플래그 내장 — StrictMode 이중 마운트 안전) 훅만 사용.
  Channel 은 소비 위젯이 소유하고 unmount 시 대응 detach command 호출.

## 2. 타입 생성 (ADR-0011)

- 모든 command 에 `#[specta::specta]`, 이벤트는 `tauri_specta::Event` derive.
- `src/bindings.ts` 는 debug 빌드에서 재생성·커밋. entities 레이어만 bindings 를 import 하고
  widgets/features 는 entities 경유(FSD).

## 3. command·event 목록 (도메인별 정본)

각 상세 시맨틱은 해당 `features/*.md` 참조. (C)=Channel 파라미터 포함.

### app (최상위 도메인 — 이전 판 `app_get_info` 누락)

- query: `app_get_info() → AppInfo{ name, version, platform, arch }`(부팅 시 About/타이틀바용).
- `app_file_read`/`app_file_write` 는 Wave I 신설(§"Wave I 계약 확정 추가" 절)이라 그쪽에서 다룬다.
- event: `app:ready(version)` — 타입·`collect_events!`·`fanout_remote_events!` 등록까지는 돼 있으나
  **Rust 쪽에서 실제로 `.emit()` 호출하는 지점이 없고 프론트도 구독하지 않는다**(정찰 확인 — 죽은
  이벤트 배선. 제거 여부는 이 문서 범위 밖이라 사실만 기록한다).

### project (`layout-shell.md`)

- query: `project_list`, `project_get`, `project_get_active`
  (`project_get_active` 는 구현 중 추가 — 부팅 시 view 가 활성 프로젝트를 알 방법이 없었다.
  `project:activated` 이벤트는 전환 시점에만 오므로 초기 조회용 query 가 별도로 필요하다.)
- mutation: `project_open(path)`, `project_close(id)`, `project_activate(id)`, `project_reorder(ids)`
- event: `project:opened`, `project:closed`, `project:activated`, `project:list-changed`,
  `project:focus-kind-changed`

### layout (`tabs.md`)

- query: `layout_get(projectId)`
- mutation: `layout_open_tab(projectId, kind, title, target, preview)`(이전 판은 `target?` 뒤에
  `title`·`preview` 두 인자가 빠져 있었다 — 정정), `layout_close_tab(tabId)`,
  `layout_activate_tab(tabId)`, `layout_move_tab(tabId, paneId, index)`,
  `layout_split(paneId, edge: DropEdge, tabId)`, `layout_resize(paneId, sizes)`,
  `layout_focus_pane(paneId)`, `layout_pin_tab(tabId, pinned)`, `layout_set_preview(tabId, preview)`
  (7.7 후속 — 아래 절 참조), `layout_reopen_closed(projectId)`,
  `layout_set_view_state(tabId, viewState)`, `layout_set_dirty(tabId, dirty)`,
  `layout_set_terminal_session(tabId, sessionId)`, `layout_open_untitled(projectId, target)`,
  `layout_convert_untitled(tabId, path)`(이 4종은 이전 판에 전혀 없던 기재 누락 — 정정),
  `layout_move_tab_to_window(tabId, target: TabWindowTarget)`,
  `layout_set_shell_view(projectId, patch: ShellViewPatch)`(둘 다 Wave I — 아래 절 참조)
- event: `layout:changed(projectId, revision)`

### file (`editor.md`)

- query: `file_open(path)` (내용+언어+크기 정책 판정), `file_read_raw(path)`(Response —
  뷰어 모드/대형, raw 커맨드 — 아래 절)
- mutation: `file_save(path, content)`, `file_create(path, isDir)`, `file_rename(from, to)`,
  `file_delete(path)`(휴지통), `file_copy(from, to)`
- Hot Exit(기능 확장 3차 — §"기능 확장 3차" 참조): mutation `file_mirror_dirty(projectId, path,
  content) → number | null`(반환값은 Rust 가 쓰기 시점에 직접 stat 한 디스크 mtime baseline — 원래
  시그니처는 프론트가 `diskModifiedMs` 인자로 baseline 을 넘겼으나 Wave B 하드닝의 미러 부활 수정에서
  빠졌다, 아래 "기능 확장 3차 계약 확정 추가 (Hot Exit — B1 Rust)" 절 참조), `file_clear_mirror(projectId,
  path)`, `file_prune_mirrors(projectId, keepPaths)`, `file_mirror_untitled(projectId, tabId, content)`,
  `file_clear_untitled_mirror(projectId, tabId)`, `file_prune_untitled_mirrors(projectId,
  keepTabIds)`(재시작 후 레이아웃 기준 authoritative GC — untitled 탭이 열려 있거나 닫힘 스택에
  남아 있지 않으면 미러 삭제), `file_flush_complete()`(**원격 세션에서는 거부** — §"원격 dispatch
  정책" 참조, 데스크톱 자신의 종료만 재개 가능); query `file_list_mirrors(projectId)`
  → `MirrorEntry[]`, `file_list_untitled_mirrors(projectId)` → `UntitledMirrorEntry[]`
  - 모든 project-scoped 미러 커맨드(`file_list_mirrors`·`file_prune_mirrors`·
    `file_mirror_untitled`·`file_list_untitled_mirrors`·`file_clear_untitled_mirror`·
    `file_prune_untitled_mirrors`)는 `projectId` 가 현재 열린 프로젝트인지
    `root_guard::project_root` 로 검증하고, `tabId` 를 파일명 컴포넌트로 쓰는 두 커맨드는 추가로
    `root_guard::ensure_safe_component` 로 경로 탈출 문자(`/`·`\`·`..`)를 거부한다(경로 조작 방지).
- event: `fs:changed(paths[], kind, origin)` (watcher — debounce·배치·echo 플래그),
  `app:hot-exit-flush-requested(timeoutMs)` (Hot Exit — 종료 인터셉트)

### tree / search (`explorer-sidebar.md`)

> 이전 판의 `tree_expand`/`tree_collapse`/`tree_node`·`tree:changed` 이벤트·`search_start`/`searchId`
> 는 전부 코드에 없는 이름이었다(정정).

- query: `tree_rows(projectId, offset, limit) → TreeRowPage`
- mutation: `tree_toggle(projectId, path)`, `tree_reveal(projectId, path)`,
  `tree_refresh(projectId, dir)` — 셋 다 갱신된 `TreeRowPage` 를 **반환값으로 직접** 돌려준다. 트리
  갱신을 알리는 별도 이벤트는 없다(옛 `tree:changed` 는 실재하지 않는다 — 정정).
- mutation(C): `search_run(projectId, sessionId, query: SearchQuery, onMatch: Channel<SearchMatch>) →
  number`(정정: 이전 판이 말한 Rust 발급 `searchId` 는 없다 — `sessionId` 는 **호출자(프론트)가
  검색 표면마다 하나씩 발급**해 넘기고, 반환값은 매치 총수다. 같은 `sessionId` 의 새 실행은 이전
  실행을 자동 취소한다 — `docs/acknowledge/2026-08-15-wave-d-search-nav-contract.md` §3.4),
  `search_cancel(sessionId)`
- mutation: `search_replace(projectId, query: SearchQuery, replacement, paths?) →
  SearchReplaceResult { changedFiles, replacedMatches }`(이전 판은 `projectId` 인자가 빠져 있었다 —
  정정)
- Wave D: `TabKind::SearchEditor { query }`(신규 tab variant, "Search Editor") — 결과 목록은 저장하지
  않고 쿼리만 레이아웃에 영속화한다. 복원 시 같은 `search_run` 으로 재검색한다(대량 `SearchMatch[]`
  를 레이아웃 JSON 에 싣지 않기 위함).

### git (`git.md`)

> 커맨드 **41종**(query 16 · mutation 25). Wave C(`docs/acknowledge/2026-08-15-wave-c-git-contract.md`)
> 가 기존 27종에 13종(3-way 충돌 해소·hunk/line 단위 stage·커밋 상세·파일 히스토리·revert·tag·원격
> 브랜치 checkout)을 더했다. 이전 판은 이 13종 전부와 `git_current_user`가 누락돼 있었고, **`git_refs`
> 라는 query 커맨드는 애초에 코드에 존재한 적이 없다**(정정 — refs 변경 "알림"은 `git:refs-changed`
> 이벤트가 담당하고, 실제 목록은 `git_branches`/`git_tags`/`git_stash_list` 를 각각 호출해 받는다).

- query: `git_status(projectId)`, `git_gutter(projectId, path)`,
  `git_blame_range(projectId, path, from, to)`, `git_diff_file(projectId, path, mode: DiffMode)`,
  `git_diff_staged_text(projectId) → StagedDiffText{ diffText, truncated, skippedFiles, usedFallback }`
  (Wave G — `ai.md` §4/§7 의 AI 커밋 메시지 생성 전용 소비처, unified diff 텍스트 + 32KiB 상한 +
  바이너리/lock 파일 제외; `usedFallback` 은 Wave H 신설 — staged 델타 0건이면 HEAD↔워킹트리(untracked
  포함) 전체 변경으로 폴백했다는 표시, 동일 제외·상한 규칙이 폴백 diff 에도 적용된다),
  `git_show_file(projectId, rev, path)`, `git_log(projectId, skip, take)`, `git_ahead_behind(projectId)`,
  `git_remotes(projectId)`, `git_stash_list(projectId)`, `git_branches(projectId)`,
  `git_current_user(projectId) → string | null`, `git_tags(projectId) → TagInfo[]`(Wave C),
  `git_conflict_sides(projectId, path) → ConflictSides{ base?, ours?, theirs?, workdir }`(Wave C —
  index conflict entry 의 stage 1/2/3 blob + workdir 현재 내용, base 없는 add/add 충돌은
  `base: null`), `git_commit_files(projectId, rev) → CommitFile[]`(Wave C — 부모 tree 대비 변경 파일,
  병합 커밋은 first-parent 기준·초기 커밋은 empty tree 대비), `git_file_log(projectId, path, skip,
  take) → LogEntry[]`(Wave C — 파일 단위 히스토리, `--follow` rename 추적은 범위 밖)
- mutation: `git_init(projectId)`, `git_stage(projectId, paths)`, `git_unstage(projectId, paths)`,
  `git_discard(projectId, paths)`, `git_commit(projectId, message, opts: CommitOptions{ amend?,
  stageAll? })`, `git_push(projectId)`, `git_pull(projectId)`, `git_fetch(projectId)`,
  `git_undo_last_commit(projectId)`, `git_branch_create(projectId, name, checkout)`,
  `git_branch_checkout(projectId, name)`, `git_branch_delete(projectId, name, force)`,
  `git_stash_push(projectId, message?)`, `git_stash_apply(projectId, index)`,
  `git_stash_drop(projectId, index)`, `git_discard_hunk(projectId, path, hunkStart, hunkEnd)`,
  `git_stage_hunk(projectId, path, hunkStart, hunkEnd)` / `git_unstage_hunk(projectId, path, hunkStart,
  hunkEnd)`(Wave C — git2 `Repository::apply` 로 선택 hunk 만 index 에/에서 적용, stage/unstage 대칭),
  `git_stage_lines(projectId, path, lineStart, lineEnd)` / `git_unstage_lines(projectId, path,
  lineStart, lineEnd)`(Wave C — hunk 가 아니라 선택 라인 단위로 patch 재합성), `git_resolve_conflict(
  projectId, path, content)`(Wave C — workdir 기록 + index stage 0 으로 충돌 자동 해소),
  `git_revert_commit(projectId, rev) → RevertOutcome{ conflicted, conflictedPaths[], conflictedAbsPaths[] }`
  (Wave C — 충돌 시 conflicted 상태로 착지해 위 3-way 흐름과 연계; `conflictedAbsPaths` 는 절대경로
  동봉본으로, 호출부(`commit-graph.tsx`)가 파일 도메인이 절대경로를 요구하는 `onOpenFile` 에 넘긴다),
  `git_tag_create(projectId, name, target, opts:
  TagCreateOptions{ message?, annotated? })` / `git_tag_delete(projectId, name)`(Wave C — annotated
  가 기본값, `message` 없으면 lightweight), `git_checkout_remote_branch(projectId, remoteRef)`(Wave C
  — 로컬 추적 브랜치 생성 + upstream 설정 + checkout, 동명 로컬 브랜치가 있으면 에러 없이 그냥
  checkout)
- event: `git:status-changed`, `git:refs-changed` — **`git:operation-progress`/`git:operation-finished`
  이벤트는 코드에 존재하지 않는다**(이전 판의 기재 오류, 정정).

**`git_discard_hunk` 의 좌표 계약**: hunk 경계는 `git_gutter` 가 반환한 `GutterHunk { start, end }`
를 그대로 넘긴다. Rust 쪽에서 두 함수가 **같은 diff 옵션과 같은 경계 계산 헬퍼**를 공유하므로
프론트가 받은 좌표와 어긋날 수 없다. 이 불변식이 깨지면 엉뚱한 줄이 되돌려진다. (`git_stage_hunk`/
`git_unstage_hunk`/`git_stage_lines`/`git_unstage_lines` 는 파라미터 형태는 같지만 diff 기준선이
다르므로 — 예: `git_stage_hunk` 는 index↔workdir, `git_gutter`/`discard_hunk` 는 HEAD↔workdir-with-index
— 이 불변식을 그대로 확장 적용하지 않는다.)

**커밋 diff 의 에디터 탭 승격(손 QA 1차 수정, 2026-08-18)**: 커밋 상세 패널에서 파일을 클릭하면
패널 내부가 아니라 `layout_open_tab` 으로 에디터 탭(`TabKind::Diff`, `rev` 필드 있음)을 연다 —
신규 git 커맨드는 없다. 탭이 그 안에서 부르는 데이터도 `git_show_file(rev, path)` /
`git_show_file(parentRev, beforePath ?? path)` 이 두 기존 커맨드 그대로다(`rev`/`parentRev` 는 각각
커밋의 `id`/첫 부모, `beforePath` 는 rename 파일의 커밋 이전 경로 — `origPath`). 상세는 아래 손 QA
절과 `git.md` §"커밋 diff" 참조.

### ai (Wave G 신설 — `ai.md`)

> Wave G 이전부터 자동완성(auto-tab) 커맨드 6종이 존재했으나 이 문서에 정본 항목이 없었다 — 이번에
> 처음 문서화하며, 신규 3종·리네임 1건을 함께 반영한다. 상세 시맨틱은 `ai.md`.

- query: `ai_token_status()` → `AiTokenStatus{ollamaCloud, codex, omlx}`,
  `ai_list_models(provider)` → `AiModelInfo[]`
- mutation: `ai_set_token(provider, token)`, `ai_clear_token(provider)`
- mutation(취소 가능, `requestId` 기반 — 셋이 `AiRequestStore` 하나를 공유):
  `ai_inline_complete(request: AiInlineCompleteRequest)` → `AiInlineCompleteResponse{requestId, text}`
  (auto-tab, FIM-우선/chat-폴백) · **`ai_inline_edit(request: AiInlineEditRequest)`**(신규) →
  `AiInlineEditResponse{requestId, text}` · **`ai_commit_message(request: AiCommitMessageRequest)`**
  (신규) → `AiCommitMessageResponse{requestId, text}` · **`ai_request_cancel(requestId)`**
  (`ai_inline_cancel` 에서 리네임 — auto-tab 전용이던 취소가 세 커맨드 공용으로 일반화됐다)
- `AiInlineEditRequest{requestId, provider?, model?, selection, instruction, language, filePath,
  prefix, suffix}` / `AiCommitMessageRequest{requestId, provider?, model?, diffText, recentCommits}`
  — `provider`/`model` 을 생략하면 `Settings.aiProvider`/`aiModel` 로 폴백한다(`ai.md` §1). 텍스트
  응답은 둘 다 실패가 아니라 `text: null`(빈 응답/취소)로 표현될 수 있다. (`AiInlineCompleteRequest`
  는 `provider`/`model` 이 필수다 — 폴백 대상이 아니다.)
- 원격 dispatch: ai 8종 전부 허용(§"원격 dispatch 정책" 참조) — `ai.md` §8.

### terminal (`terminal.md`)

- mutation(C): `pty_spawn(opts: PtySpawnOptions, onData) → sessionId`, `pty_attach(sessionId, onData)
  → subscriptionId`(둘 다 raw 커맨드 — 아래 절)
- mutation: `pty_write(sessionId, data)`, `pty_resize(sessionId, cols, rows)`, `pty_kill(sessionId)`,
  `pty_set_paused(sessionId, paused)`, `pty_detach(sessionId, subscriptionId)`(Wave I 채널 다중화 —
  세션당 여러 창이 구독할 수 있게 되면서 신설. 세션/구독이 이미 없으면 에러가 아니라 already-detached
  로 취급한다)
- query: `shell_profiles`, `terminal_sessions(projectId)`, `resolve_terminal_path(path, cwd)`,
  `pty_default_options(projectId, cwd?)`(7.7 후속 — 아래 절 참조)
- event: `terminal:exited(sessionId, ...)`, `terminal:cwd-changed(sessionId, cwd)`,
  `agent:state-changed(projectId, agents: DetectedAgent[])` — **`terminal_report_cwd` 라는 mutation
  은 코드에 없다**(정정: cwd 보고는 프론트→Rust mutation 이 아니라 Rust→view 이벤트
  `terminal:cwd-changed` 로 흐른다)

### task (Wave E 추가 — `tasks.md`)

- query: `detect_tasks(projectId)` → `Task[]`(`{ label, command, source, cwd }`) — 프로젝트 루트를
  훑기만 하는 부수효과 없는 조회. `source` 는 `npm`/`make`/`cargo`. 실행은 별도 command 없이
  프론트가 `Task.command` 를 기존 `pty_write` 경로로 포커스(또는 신규) 터미널 탭에 흘려보낸다
  (`terminal.md` §9 재사용 — Run Selected Text 와 동일 전달 경로).

### lsp (`lsp.md`)

> `lsp_detect_servers`·`lsp_resolve_root`·`lsp_install`·`lsp_install_cancel` 4종과 `lsp:install-progress`
> 이벤트는 이 캠페인(Wave A~I) 이전(2026-08-11 전후)부터 있었지만 이 문서에 실린 적이 없었다 —
> 이번에 처음 문서화한다.

- mutation(C): `lsp_spawn(request: LspSpawnRequest, onMessage) → sessionId`
- mutation: `lsp_send(sessionId, message)`, `lsp_stop(sessionId, root?, owner)`,
  `lsp_restart(sessionId)`, `lsp_install(serverId)`(서버 바이너리 자동 다운로드·설치, 진행률은
  `lsp:install-progress` 이벤트로), `lsp_install_cancel(serverId)`
- query: `lsp_sessions(projectId)`, `lsp_detect_servers() → LspServerDetection[]`(설치된 LSP 바이너리
  감지), `lsp_resolve_root(serverId, filePath) → string | null`
- event: `lsp:session-status-changed`, `lsp:install-progress(serverId, phase, receivedBytes?,
  totalBytes?, message?)`

### theme / settings (`theme-system.md`)

- query: `theme_get(themeId)`, `theme_get_current(systemTheme)`, `theme_list`, `settings_get`
- mutation: `settings_set_theme(themeId)`, `settings_update(patch)`
  (구현 시 정정: 초안의 `theme_set(id)` 을 **`settings_set_theme`** 로 확정.
  선택된 테마 id 는 `Settings` 가 소유하는 값이라 mutation 도 settings 도메인에 두는 것이 맞다.
  theme 도메인은 읽기 전용(로드·해석)만 담당한다. `theme_get_current` 는 view 부팅 시
  "현재 설정의 테마"를 한 번에 받기 위한 query 로 추가. **손 QA 1차 수정(2026-08-18)**:
  `settings_set_theme` 이 이제 `follow_system_theme` 을 `false` 로 함께 끈다 — 이전에는 이 플래그가
  켜진 채로 테마를 골라도 `theme_get_current` 가 계속 OS 테마로 재해석해 선택이 조용히 무시됐다.
  아래 손 QA 절·`data-model.md` §Settings 참조.)
- mutation(7.5-D 추가): `theme_save(theme)`, `theme_delete(themeId)`
  — 테마 편집기(`theme-system.md` §7.3)용. 내장 테마 덮어쓰기를 거부하고,
  id 에 경로 구분자(`/ \ .`)가 있으면 거부한다(경로 탈출 방지).
- event: `theme:changed`, `settings:changed`

**`theme_get_current` 가 `systemTheme` 을 받는 이유(7.5-D 정정)**: `followSystemTheme` 판단은
Rust 가 하되 OS 다크/라이트 감지는 view 가 센서 역할을 한다. Rust 가 웹뷰 없이 OS 테마를
읽으려면 플랫폼 분기가 늘어나는데, view 에는 이미 `prefers-color-scheme` 이 있다.
locale 의 `locale_get_current(systemLanguage)` 도 같은 이유로 같은 형태다.

**페이로드 확장(7.10-W7)**: `Theme`·`ResolvedTheme` 에 `tokenColors?: TokenColorRule[]`(원본
TextMate 룰 전량 — 없으면 필드 자체가 생략) 필드가 추가됐다. `ResolvedTheme` 에는 추가로
`syntaxOverrides: string[]`(자식 테마가 스스로 명시한 `syntax` 키 목록 — 프론트가 오버레이 합성
근거로 쓴다)이 붙는다. colors/syntax/terminal 의 토큰 집합·타입은 불변이다 — `theme_get`/
`theme_get_current`/`theme_save` 세 커맨드 모두 이 확장된 형태를 주고받는다(자동 생성 타입은
`bindings.ts`, 상세 스키마는 `docs/theme-system.md` §2.1).

### snippet (Wave F 신설 — `editor.md` §10)

- query: `snippet_list` → `SnippetFile[]`(`{ fileName, snippets: Record<string, SnippetEntry> }`,
  `SnippetEntry = { prefix, body, description?, scope? }` — `prefix`/`body`/`description` 은
  `string | string[]`). `paths.snippets_dir()` 아래 `<languageId>.json` + `*.code-snippets` 를
  스캔하고, 파싱 실패 파일은 스킵 + `log::warn!`(테마 목록과 동일한 견고성 원칙 — 하나가 깨져도
  나머지는 계속 로드).
- mutation: `snippet_save(fileName, content)` → `SnippetFile`, `snippet_delete(fileName)`.
  `fileName` 은 `theme_save` 와 동일한 경로 탈출 방지(`/`·`\`·`..` 거부)에 더해 확장자
  화이트리스트(`.json`/`.code-snippets`)까지 강제한다 — 리스트 스캔(`snippet_list`)과 같은
  predicate 를 공유시켜 "저장은 되는데 목록에 안 보이는 파일"이 생기지 않게 하기 위함
  (`theme_save` 대비 한 단계 더 엄격). `content` 는 JSON 스키마 검증(`SnippetFile.snippets` 형태)
  후 **재직렬화 없이 원문 그대로** 저장 — 프론트가 조립한 문자열이 그대로 디스크에 남는다.
- Rust 는 스니펫을 캐시하지 않는다 — 프론트 TanStack Query(`QUERY_KEY.SNIPPET`)가 캐시를 소유하고
  save/delete 성공 시 invalidate 로 다시 받는다. 파일 워처(외부 편집 즉시 반영)는 1차 제외.
- `system_open_app_data_path(kind)` 의 `AppDataPathKind` 에 `"snippets"` 추가(아래 7.10 항목 갱신).
- settings: `editorSemanticHighlighting`(기본 true) · `editorFormatOnType`(기본 false) ·
  `editorFormatOnPaste`(기본 false) · `emmetEnabled`(기본 true) 필드 4종 추가(전부
  `SettingsPatch`/`emptySettingsPatch`/sync 동반 — 상세는 `data-model.md`).

### locale (7.5-H 신설 — `acknowledge/2026-08-06-i18n-and-session-findings.md`)

- query: `locale_list`, `locale_get(localeId)`, `locale_get_current(systemLanguage)`
- 메시지는 **flat dotted key**(`"common.cancel"`). 중첩 객체가 아니다 —
  사용자 언어팩이 키 단위로 부분 오버라이드하기 쉬운 구조를 택했다.
- 사용자 팩은 `{app_data}/locales/*.json`. `extends` 로 내장(en/ko/ja)을 상속하고 바꾼 키만 담는다.

### font (7.5-D 신설)

- query: `font_list` → `FontFamily { name, monospaced }[]`
- `fontdb` 로 시스템 폰트를 열거한다. `monospaced` 플래그로 에디터·터미널 목록을 기본 필터링한다.

### agent (`agent-integration.md`)

> 커맨드 9종. 이전 판은 `release_wait_marker(marker)` 1건만 실었으나 **이 이름의 커맨드는 코드에
> 없다** — 실제 이름은 `agent_release_marker`(정정)이고, 나머지 8종(감지·CLI 설치·hooks 설치)도
> 통째로 누락돼 있었다.

- query: `agent_list(projectId) → ProjectAgents`(pty 프로세스 트리에서 에이전트 감지, unix 1s/
  Windows 2s 폴링), `agent_cli_status() → CliInstallStatus`(`taide` CLI 심링크 설치 여부, macOS
  전용), `agent_hooks_status(projectId, agentName) → AgentHooksStatus`,
  `agent_pending_external_opens() → ExternalOpenRequest[]`(drain — "기능 확장 1차" 절 참조)
- mutation: `agent_release_marker(marker)`(외부 에디터 왕복 탭 닫힘 → 마커 삭제,
  `agent-integration.md` §2.2), `agent_cli_install()` / `agent_cli_uninstall() → CliInstallStatus`
  ("기능 확장 1차" 절 참조), `agent_hooks_install(projectId, agentName)` /
  `agent_hooks_uninstall(projectId, agentName) → AgentHooksStatus`
- event: `agent:state-changed(projectId, agents: DetectedAgent[])`,
  `agent:external-open(request: ExternalOpenRequest{ path, waitMarker? })`(콜드스타트 argv·
  single-instance 유입 — **이전 판의 `editor-bridge:open-file` 이라는 이벤트 이름은 코드에 존재하지
  않는다**, 정정: 실제 이벤트명은 `agent:external-open`)

### ide — Claude Code IDE MCP 연동 (`agent-integration.md` §3, 신설 도메인 — 이전 판 전체 누락)

- query: `ide_get_status() → IdeStatus{ running, port, connected, clientCount }`
- mutation: `ide_start()` / `ide_stop()`(내장 MCP 서버 시작/중단), `ide_set_selection(input:
  IdeSelectionInput{ projectId, path, text, startLine, startCharacter, endLine, endCharacter,
  isEmpty })` / `ide_clear_selection()`, `ide_publish_diagnostics(projectId, items: IdeDiagnostic[])`
  (LSP 진단을 MCP `getDiagnostics` 도구로 노출), `ide_resolve_diff(requestId, outcome: "saved" |
  "rejected" | "tabClosed", content)`, `ide_resolve_save(requestId, saved)`,
  `ide_notify_at_mention(path, lineStart, lineEnd)`
- event: `ide:status-changed(status: IdeStatus)`, `ide:diff-requested(requestId, projectId, oldPath,
  newPath, newContents, tabName)`(Claude 의 `openDiff` 도구 호출 → TAIDE 가 diff 탭을 연다 —
  `TabKind::ClaudeDiff`, "7.7 계약 확정 추가" 절 참조), `ide:save-requested(requestId, projectId,
  path)`, `ide:close-tab-requested(tabName, requestId: string | null)`(Claude 가 `close_tab`/
  `closeAllDiffTabs` 로 스스로 diff 탭을 닫을 때 — `requestId` 는 "7.7 계약 확정 추가" 절에서 이미
  다룬 pending 레지스트리 정리용 필드)

### plugin (`plugins.md`)

- query: `plugin_list`, `plugin_read_grammar(pluginId, languageId) -> string`(7.10-W7 신설 — grammar
  파일 본문을 온디맨드로 가져온다. `plugin_list` 응답에 본문을 인라인하지 않는 이유는 매니페스트
  스캔마다 수 MB 를 흘리지 않기 위함이다. 프론트는 highlighter 생성 시 이 query 로 각 플러그인의
  language 기여를 fetch 해 shiki `LanguageRegistration` 으로 주입한다 — 실패한 플러그인은 스킵 +
  경고)
- mutation: `plugin_reload`, `plugin_install(sourcePath) → LoadedPlugin`,
  `plugin_uninstall(pluginId) → LoadedPlugin[]`(둘 다 Wave I 신설 — "Wave I 계약 확정 추가" 절
  참조). **이전 판의 `plugin_set_lsp_enabled(pluginId, lspId, enabled)` 는 코드에 존재하지 않는
  커맨드였다**(기재 오류, 정정 — 삭제).

### vsix (7.10-W5 신설 — `vsix-theme-import.md`)

- query: `vsix_extract_themes(vsixPath)` → `VsixThemeExtractionResult { extension, themes[] }`.
  `extension`(`VsixExtensionInfo { displayName, publisher, version }`)은 `extension/package.json`
  에서 그대로 읽는다(임포트 출처 표기용 — extension.vsixmanifest XML 은 파싱하지 않는다, 신규
  의존성 회피). `themes[]`(`VsixExtractedTheme { label, uiTheme, rawJson, includeChain[] }`)의
  `rawJson`/`includeChain[].rawJson` 은 **파싱하지 않은 원본 문자열**이다 — 변환(색상 매핑·jsonc
  주석 제거)은 `scripts/convert-vscode-theme.ts` 의 기존 로직을 재사용하는 프론트 몫이다(Rust 는
  추출만). 손상되거나 확장 루트를 벗어나는 개별 테마 항목은 전체 호출을 실패시키지 않고
  건너뛴다(`plugins.md` §3 "하나가 깨져도 나머지는 계속 로드된다"와 동일한 견고성 원칙).
  **`vsixPath` 는 `resolve_within_open_project`(프로젝트 루트 가드)를 거치지 않는다** — 프론트
  `dialog` 로 사용자가 직접 고른 파일 경로이고 Rust 는 그 파일을 읽기만 한다(쓰기 없음). 이는
  `project_open(path)` 가 이미 사용자 선택 경로를 루트 가드 없이 받는 것과 같은 성격이다(프로젝트
  루트를 정하는 진입점 자체이므로 검증할 "루트"가 아직 없다). **원격 dispatch 는 Wave I 에서
  허용→거부로 전환됐다** — §"원격 dispatch 정책" 참조.
- **grammar 는 다루지 않는다**: `vsix_extract_themes` 는 `contributes.themes` 만 읽고
  `contributes.grammars`(TextMate 문법)는 추출하지 않는다 — grammar 는 `contributes.languages`
  (언어 id·확장자)와 쌍으로 가져와야 의미가 있고, TAIDE 의 확장자→언어id 매핑
  (`LANGUAGE_ID_BY_EXTENSION`)이 Rust 컴파일 타임 상수라 런타임 오버레이화가 선행돼야 한다 —
  W7 에서도 범위 밖(`docs/backlog.md`). (Wave I 의 `vsix_import_plugin` 은 grammar/language 기여를
  다룬다 — 아래 "Wave I 계약 확정 추가" 절.)
- 도메인 분리 근거: `domain/plugin` 은 TAIDE 자체 선언적 확장 체계(`{app_data}/plugins/*/
  taide-plugin.json`, 부팅 시 스캔, ADR-0010 "코드 실행 없음")이고 `domain/vsix` 는 VS Code
  확장(.vsix) 이라는 **별개 포맷**을 사용자가 그때그때 선택한 파일에서 1회성으로 추출하는
  기능이다. 스키마·트리거·검증 규칙이 전혀 겹치지 않아(taide-plugin.json 의 경로 탈출 검증은
  플러그인 루트 기준, vsix 는 zip 아카이브 내 `extension/` 루트 기준) `domain/plugin` 에
  얹으면 두 스키마가 뒤섞인다 — 신규 `domain/vsix` 로 분리했다.

### remote (`remote-control.md`)

> 커맨드 7종. 이전 판은 `remote_set_password`/`remote_clear_password` 2종만 개별 절에서 서술했고,
> 나머지 5종(상태 조회·서버 시작/중단·링크 발급·세션 전체 폐기)은 도메인으로 묶여 문서화된 적이
> 없었다.

- query: `remote_status() → RemoteStatus{ running, port, clientCount, passwordConfigured }`(5초 폴링)
- mutation: `remote_start()` / `remote_stop()`(로컬 HTTP/WS 서버 시작·중단), `remote_issue_link() →
  RemoteLinkInfo{ url }`(**원격에서 거부** — §"원격 dispatch 정책"), `remote_revoke_sessions()`
  (모든 세션 즉시 무효화), `remote_set_password(password)` / `remote_clear_password()`(둘 다
  **원격에서 거부** — "기능 확장 3차 계약 확정 추가 (Remote 비밀번호 — C1 Rust)" 절 참조)
- event: `remote:state-changed(status: RemoteStatus)`

### sync (설정 gist 동기화 — 신설 도메인, 이전 판 전체 누락. `data-model.md` §6 참조)

- query: `sync_status() → SyncStatus{ connected, hasGist, lastSyncedAt, remoteNewer }`
- mutation: `sync_connect(pat)`(GitHub PAT 로 연결), `sync_disconnect()`, `sync_upload()`(현재 설정을
  gist 로 업로드 — `strip_non_syncable` 로 `remoteAccessEnabled`·`remotePasswordOnlyLogin`·
  `shellOverride`·`remoteAllowedHosts` 를 제외하고 업로드, Wave B §3.1), `sync_download(force) →
  SyncDownloadResult`(`{ kind: 'applied', status } | { kind: 'conflict', remoteUpdatedAt }` — 로컬이
  더 최신이면 `force` 없이는 충돌로 보고, 다운로드 페이로드도 동일 4필드를 강제 제외한다)
- event: `sync:state-changed(status: SyncStatus)`
- `schemaVersion` 게이트(`sync::service::ensure_supported_schema_version`)는 페이로드의
  `schemaVersion` 이 이 빌드의 `SETTINGS_SCHEMA_VERSION`(정책상 `1` 로 동결)보다 **클 때만** 거부한다.
  같은 버전인데 이 빌드가 모르는 필드가 섞인 페이로드는 게이트를 통과하고, 그 필드는 `serde(default)`
  에 의해 조용히 버려진다 — 미지 필드 감지 장치가 아니다(2026-08-18 T1-E, X1#6).

### 7.6 추가 (IDE 핵심 루프)

> git·search 신규 커맨드 목록은 위 `### git`·`### tree / search` 절로 통합했다 — 이 절에 다시 따로
> 적으면 두 출처가 갈라져 이번처럼 문서 부채가 쌓인다(SSOT 유지).

**`git_discard_hunk` 의 좌표 계약**은 위 `### git` 절 말미로 옮겼다.

### 7.7 계약 확정 추가

- system(신설): query `system_usage_get` → `SystemUsage { cpuPercent: number | null, memoryBytes: number }`.
  TAIDE 메인 프로세스만 측정한다(자손 프로세스 미포함). 첫 호출은 `cpuPercent: null`
  (CPU 사용률은 두 번째 샘플부터 유효). 프론트는 `refetchInterval` 폴링으로 주기를 소유하고,
  설정 토글이 꺼지면 호출 자체를 하지 않는다(`enabled`).
- system(7.7 후속): mutation `system_open_path(path)`(기본 앱으로 열기),
  `system_reveal_path(path)`(파일 관리자에서 표시), `system_open_in_browser(path)`(`file://` URL 을
  기본 브라우저로) 3종 신설. 셋 다 실행 전에 경로가 **열린 프로젝트 루트 내부(루트 자신 포함)** 인지
  검증하고, 벗어나면 `InvalidArgument` 를 반환한다. 내부적으로 `tauri_plugin_opener` 의 Rust API
  (`open_path` / `reveal_item_in_dir` / `open_url`)를 호출하므로 capability 권한이 필요 없다.
  프론트는 `@tauri-apps/plugin-opener` 를 import 하지 않는다(§4).
- layout: `layout_set_preview(tabId, preview)` 신설 — `layout_pin_tab` 과 동일한 형태.
  `TabKind::Terminal` 에 `cwd?: string | null` 필드, `TabKind::Diff` 에 `compareWith?: string | null`
  필드 추가(둘 다 기존 데이터와 하위 호환). `TabKind::ClaudeDiff { requestId, path }` variant 신설 —
  **레이아웃 영속 저장에서 제외되는 휘발성 탭**이다(저장 직전 필터링, 사라지는 활성 탭은 인접 탭으로 대체).
  7.7 후속: `layout_reopen_closed` 의 닫은 탭 스택에도 넣지 않는다 — 닫는 순간 pending diff 요청이
  해소되므로 되살리면 수락/거부가 항상 실패하는 좀비 탭이 된다.
  `ide:close-tab-requested` payload 에 `requestId: string | null` 추가 — Claude 가 `close_tab` /
  `closeAllDiffTabs` 로 스스로 diff 탭을 닫는 경로에서 프론트 pending 레지스트리를 정리하기 위한 값이다.
- terminal: `pty_default_options(projectId, cwd)` — `cwd` 가 주어지면 프로젝트 루트 하위인지 검증 후
  사용하고, `null` 이면 기존처럼 project.root 를 쓴다.
- file: `file_delete(path)` 를 실제로 휴지통 이동(`trash` 크레이트)으로 구현
  (기존엔 영구 삭제였고 문서만 휴지통이라 적혀 있었다 — 문서-코드 불일치 해소).
  `file_create(path, isDir)` 는 대상 경로가 이미 존재하면 오류를 반환한다
  (기존엔 폴더 중복 생성이 `create_dir_all` 때문에 조용히 성공했다).
- settings: `editorMinimap`, `showSystemUsage`, `agentStatusBadgeEnabled`, `agentHooksEnabled`,
  `ideIntegrationEnabled`, `ideAutoOpenDiff` 필드 6종 추가(전부 `SettingsPatch`/`emptySettingsPatch` 동반).
  `SETTINGS_SCHEMA_VERSION` 은 1 유지(전 필드 serde default).
- agent: `DetectedAgent` 에 `activity: AgentActivity`(`"idle" | "working" | "awaitingInput" | "unknown"`)
  필드 추가 — 기존 `agent:state-changed` 이벤트 payload 가 그대로 확장된다(신규 이벤트 없음).
  `AgentHooksStatus { installed }` 타입만 추가(커맨드는 후속 단계).
- capabilities: opener 계열 권한(`opener:allow-open-path` / `opener:allow-open-url` /
  `opener:allow-reveal-item-in-dir`)은 7.7 후속에서 **전부 제거**했다. 광역 스코프(`path: "**"`)가
  §4 의 "view 는 fs/shell 플러그인 API 를 직접 호출하지 않는다" 선언과 충돌했기 때문이다 —
  같은 기능은 위 `system_*` 커맨드가 루트 검증을 거쳐 제공한다.

### 7.10 계약 확정 추가 (dead-end UX 스파인)

- system(신설): mutation `system_open_app_data_path(kind)` — `kind` 는 `"plugins" | "themes" | "locales"`
  열거값만 받는다(`AppDataPathKind`). 대상 디렉토리를 `create_dir_all` 로 없으면 만든 뒤
  `tauri_plugin_opener::reveal_item_in_dir` 로 Finder/파일 관리자에 표시한다.
  **`system_reveal_path` 와 달리 `resolve_within_open_project`(프로젝트 루트 가드)를 거치지 않는다** —
  임의 경로 문자열을 프론트에서 받지 않고 열거값 3종(=앱 데이터 디렉토리 3종)으로 입력 자체를
  고정했으므로 경로 탈출이 애초에 불가능한 설계다. 프론트는 플러그인/테마/로케일 설정 패널의
  "폴더 열기" 버튼에서 사용한다. (Wave F: `"snippets"` 추가 — 4종. 설정 SNIPPETS 섹션의
  "Open Snippets Folder" 버튼이 사용한다.)
- git(신설): mutation `git_init(projectId)` — 프로젝트 루트에서 시스템 `git init` 을 실행한다.
  이미 초기화된 저장소에 다시 호출해도 git 자체가 재초기화를 정상 종료(exit 0)로 처리하므로
  에러가 아니라 무해한 동작이다(별도 존재 여부 분기를 두지 않는다). 성공 시 `git:status-changed` /
  `git:refs-changed` 를 발행하고, 캐시된 저장소 루트(`GitStore`)를 무효화해 다음 조회가 새로
  초기화된 저장소를 다시 discover 하게 한다. `git_status` 가 `notARepository` 상태를 반환하는
  화면에서 "저장소 초기화" 버튼으로 이어진다.
- plugin: `LoadedPlugin.error` 타입이 자유 형식 한국어 문자열에서 **안정된 코드 union**
  (`PluginErrorCode = "parse-failed" | "id-mismatch" | "version-mismatch" | "path-escape"`) 으로
  바뀌었다. 프론트는 `settings.pluginError.{parseFailed,idMismatch,versionMismatch,pathEscape}`
  로케일 키로 코드를 문구로 매핑해 렌더링한다(하드코딩된 한국어 메시지를 그대로 노출하던 기존
  동작은 다국어 요구사항과 충돌해 제거).
- plugin(7.10-W7 추가): `PluginErrorCode` 에 grammar 검증 3종 `"grammar-missing" | "grammar-invalid"
  | "grammar-conflict"` 추가(파일 미존재 / JSON 파싱·`scopeName` 누락 / scopeName·languageId 충돌).
  `settings.pluginError.{grammarMissing,grammarInvalid,grammarConflict}` 로케일 키를 en/ko/ja
  4곳(스키마+3언어)에 동기 추가한다.

### 기능 확장 1차 계약 확정 추가 (taide CLI·sticky scroll)

- agent(신설 3종, 디스패치 파리티 137→140): mutation `agent_cli_install` / `agent_cli_uninstall`
  → `CliInstallStatus`(+`dangling: boolean` — 심링크는 있으나 canonicalize 실패 = 재설치 필요).
  macOS 전용(그 외 플랫폼·dev 빌드는 `InvalidArgument` 거부). install 은 멱등(이미 우리 target
  이면 osascript 생략), uninstall 은 `remove_file` → `PermissionDenied` 시에만 osascript,
  `read_link` 가 `taide-cli` 로 끝나지 않으면 거부(타 도구 심링크 보호), 사용자 취소(exit 1 +
  "-128")는 조용한 no-op. query `agent_pending_external_opens` → `ExternalOpenRequest[]`(drain) —
  콜드스타트 argv 와 single-instance 콜백이 적재한 pending 열기 요청을 프론트가 부팅 시 1회 +
  `AgentExternalOpen` 이벤트 수신 시마다 소진한다(이벤트 payload 직접 처리 금지 — 중복 재오픈 방지).
- settings: `Settings`/`SettingsPatch` 에 `editorStickyScrollEnabled: boolean`(기본 true),
  `aiOmlxBaseUrl: string | null`(QA6 후속 1차 — sanitize: http/https 만, trailing slash 제거,
  빈 문자열 패치 = 명시 해제) 추가.

### 기능 확장 2차 계약 확정 추가 (system usage breakdown)

- system(신설): query `system_usage_breakdown` → `SystemUsageProcess[]`.
  `SystemUsageProcess { pid: number, kind: SystemUsageProcessKind, label: string,
  cpuPercent: number | null, memoryBytes: number }`,
  `SystemUsageProcessKind = "app" | "terminal" | "lsp" | "agent" | "other"`.
  TAIDE 프로세스(root pid)와 그 자손 프로세스 전체를 반환한다(터미널 pty·LSP 서버·감지된 에이전트
  포함). pid 분류는 **도메인 라벨 우선**(터미널 foreground pid·LSP 서버 pid·에이전트 pid — 각
  도메인이 이미 알고 있는 매핑)이고, 도메인이 모르는 pid 는 `classify_process_name` 프로세스명
  폴백으로 분류한다(`SHELL_PROCESS_NAMES` → terminal, `KNOWN_LSP_BINARY_NAMES` → lsp, 그 외 other).
  `system_usage_get` 과는 **완전히 분리된 별도 `System` 인스턴스**로 갱신한다 — 두 커맨드가 하나의
  `System` 을 공유하면 두 폴러의 위상차가 sysinfo 의 200ms 최소 갱신 간격보다 좁을 때 앱 pid 의
  CPU 델타가 붕괴한다. 처음 등장한 pid(이전 breakdown 호출에서 본 적 없는 pid)는
  `cpuPercent: null` 로 반환한다(sysinfo 는 한 번도 refresh 하지 않은 pid 의 `cpu_usage()` 를 0.0 으로
  주는데, 이는 진짜 idle 과 구분이 안 되므로 `system_usage_get` 의 `has_previous_sample` 게이트와
  같은 원칙을 pid 단위로 적용한 것). 프론트는 모달이 열려 있는 동안만(`enabled`) `refetchInterval` 로
  폴링하고 `gcTime: 0` 이라 모달을 닫으면 캐시를 버리지만, 백엔드 `System` 인스턴스 자체는 앱
  생애주기 동안 유지되므로 이미 알려진 pid 는 재오픈 즉시 유효한 값을 돌려준다(모달을 반복해 열어도
  값이 붕괴하지 않는다). 디스패치 파리티: 140→141.

### 기능 확장 3차 계약 확정 추가 (Hot Exit — B1 Rust)

> 계약: `docs/acknowledge/2026-08-14-hotexit-remote-password-contract.md` §3.1. 상세 스키마·판정
> 기준은 `docs/data-model.md` §6.

- file(신설 8종, 디스패치 파리티 138→146 — `file_prune_untitled_mirrors` 는 QA6 결함 수정으로
  후속 추가, 아래 "QA6 확정 결함 수정" 참조): query `file_list_mirrors(projectId)` →
  `MirrorEntry { path, content, savedAtMs, diskModifiedMs, conflict }`(디스크 현재 mtime 을
  baseline 과 비교해 Rust 가 `conflict` 를 판정 — 원본이 사라진 항목은 목록에서 제외, 유령 복원
  금지), `file_list_untitled_mirrors(projectId)` → `UntitledMirrorEntry { tabId, content,
  savedAtMs }`. mutation `file_mirror_dirty(projectId, path, content) → number | null`(**Wave B
  하드닝에서 정정**: B1 원안은 프론트가 `diskModifiedMs` 인자로 baseline 을 넘기는 4번째 인자
  구조였으나, 미러 부활 버그 수정(Wave B 하드닝 패키지 §3.2 "미러 부활 A안" — `docs/acknowledge/
  2026-08-15-wave-b-hardening-contract.md`) 과정에서 baseline 을 프론트가 신뢰해 넘기는 대신 Rust
  가 쓰기 시점에 직접 stat 해 반환값으로 돌려주는 구조로 바뀌었다 — 인자에서 빠지고 반환값
  `number | null` 이 됐다), `file_clear_mirror(projectId,
  path)`, `file_prune_mirrors(projectId, keepPaths)`(`keepPaths` 밖의 경로 미러 일괄 삭제 —
  프로젝트 열 때 GC), `file_mirror_untitled(projectId, tabId, content)`,
  `file_clear_untitled_mirror(projectId, tabId)`, `file_flush_complete()`(아래 종료 플러시 완료
  신호 — 인자 없음).
- event(신설): `app:hot-exit-flush-requested(timeoutMs)` — `WindowEvent::CloseRequested` 를
  가로채 emit. view 는 모든 dirty 모델을 미러 mutation 으로 밀어넣은 뒤 `file_flush_complete` 를
  호출해야 실제 종료가 재개된다. `timeoutMs`(= `HOT_EXIT_FLUSH_TIMEOUT_MS`, 2.5초)는 Rust 상수를
  이벤트로 실어 보내 view 가 같은 매직넘버를 따로 들지 않게 한다 — 미응답 시 Rust 가 타임아웃
  폴백으로 강제 종료한다(하드행 방지).
- layout: `TabKind::Untitled` 를 `is_volatile` 대상에서 제외 — untitled 탭이 레이아웃에 영속되고
  닫힌 탭 undo 스택에도 쌓인다(이전에는 재시작 시 소실). `ClaudeDiff` 만 남은 유일한 volatile
  kind.

### 기능 확장 3차 계약 확정 추가 (Remote 비밀번호 — C1 Rust)

> 계약: `docs/acknowledge/2026-08-14-hotexit-remote-password-contract.md` §3.2. 파일:라인 근거는
> 세션 정찰 `recon3-remoteAuth.md`.

- remote(신설 2종, 디스패치 파리티 145→147): mutation `remote_set_password(password)` — 비밀번호를
  `salt$sha256(salt+password)` 형식으로 해시해 keyring(`SecretAccount::RemoteAccess`)에 저장하고
  기존 세션을 전부 무효화한다(신규 의존 0 — 저장은 `settings.json` 이 아니라 keyring, gist 동기화
  화이트리스트에도 없음). `remote_clear_password()` — 비밀번호를 keyring 에서 삭제하고 링크만으로
  접속하던 현행으로 되돌리며, 마찬가지로 기존 세션을 전부 무효화한다. **두 커맨드는 원격 세션에서
  호출하면 항상 거부된다** — `dispatch.rs` 의 `match` arm 이 실제 핸들러를 부르지 않고
  `AppError::Forbidden` 을 즉시 반환한다(원격 세션이 자신의 게이트를 바꾸는 것을 막기 위함).
  파리티 테스트(`bindings와_dispatch_테이블은_커맨드_이름_집합이_일치한다`)는 이름 집합만 보므로
  `IMPLEMENTED_JSON_COMMANDS` 에는 정상 등록되어 있다.
- `RemoteStatus` 에 `passwordConfigured: boolean` 추가(5초 폴링에 편승 — 별도 쿼리 불필요).
  `Settings.remotePasswordOnlyLogin: boolean`(기본 false, sync 화이트리스트 비포함)은 기존
  `settings_get`/`settings_update` 로 그대로 노출된다(Remote 전용 커맨드 신설 불필요). 단
  `settings_update` 는 원격 세션에서도 호출 가능한 커맨드이므로, `dispatch.rs` 의 `settings_update`
  arm 이 원격 요청의 patch 에서 `remotePasswordOnlyLogin` 필드를 `None` 으로 스트립한 뒤 위임한다
  (원격 세션이 자기 게이트 모드는 못 바꾸되 그 외 설정은 정상 변경 가능 — `remoteAccessEnabled` 는
  자가 차단=자기 접근 상실이라 스트립 대상에서 제외). **Wave B 하드닝에서 확장(정정)**: 스트립
  대상이 `remotePasswordOnlyLogin` 1필드에서 `remoteAllowedHosts`·`shellOverride` 를 더한 **3필드**
  로 늘었다 — gist 인바운드(§sync)의 `shellOverride` 미필터가 RCE 급 결함으로 확인되면서
  (`docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §2·§3.1), `settings_update` 원격 경로도
  같은 필드를 동일 원칙으로 스트립하도록 맞췄다. 자세한 허용/거부/스트립 전체 그림은
  §"원격 dispatch 정책" 참조.
- 인증 흐름 재배치(`auth_middleware`, 예외 0개였던 종전 구조에 최초로 예외 추가):
  1. Origin/Host 검사(그대로, Origin 헤더 부재는 허용 — 브라우저 GET 내비게이션 대응).
  2. 유효한 세션 쿠키(`taide_remote_session`, 이제 `HashMap<digest, 만료 Instant>` 로 저장되어
     `REMOTE_SESSION_TTL_MS`(7일)가 지나면 자동 무효화)면 통과.
  3. `GET`/`POST /__taide/login` 은 인증 없이 핸들러로 통과.
  4. `?t=<link_token>` 유효 시: **비밀번호 미설정**이면 (구현상) 토큰을 nonce 로 소모한 뒤 그
     nonce 를 즉시 세션으로 승격해 원 요청을 계속 처리 — 바깥에서 관찰되는 동작은 종전과 동일한
     "링크 클릭 = 즉시 세션". **비밀번호 설정됨**이면 nonce 쿠키(`taide_remote_login_nonce`,
     `REMOTE_LOGIN_NONCE_TTL_MS`=5분)를 발급하고 로그인 폼으로 303.
  5. 그 외: `remotePasswordOnlyLogin` on 이면 로그인 폼으로 303, off 면 401.
  - `consume_link_token` 은 더 이상 세션을 직접 발급하지 않는다 — "토큰 검증·소모 → nonce 발급"
    으로 책임이 좁혀졌고, "nonce 검증 → 세션 발급"은 신설 `promote_nonce_to_session` 이 맡는다
    (기존 세션 승격 테스트 3건이 이 2단 호출로 갱신됨).
- 로그인 `POST`: Origin **필수**(부재 시 즉시 403 — 미들웨어의 관대한 검사와 별개로 CSRF 방어를
  이중화), 잠금 중이면 비밀번호 검증 없이 429 + 남은 초, `remotePasswordOnlyLogin` 이 아니면 nonce
  쿠키 필수, `service::verify_password`(constant-time) 로 검증. 실패 시 지수 백오프 잠금
  (`REMOTE_LOGIN_MAX_ATTEMPTS`=5회 초과부터, `REMOTE_LOGIN_LOCKOUT_BASE_MS`=1초씩 배가,
  `REMOTE_LOGIN_LOCKOUT_MAX_MS`=60초 상한 — 전역 카운터, 세션이 아니라 서버 단위). 성공 시
  세션 쿠키 발급 + nonce 쿠키 제거 + `/` 로 303. axum 이 `form` feature 없이 빌드되어
  (`default-features = false, features = ["http1", "tokio", "ws"]`) `Form` 추출기를 쓸 수 없으므로
  `application/x-www-form-urlencoded` 바디는 `server.rs` 가 신규 의존 없이 직접 파싱한다.
- `login_page.rs`(신설): self-contained HTML(인라인 `<style>`, 스크립트 0 — CSP
  `script-src 'none'; connect-src 'none'`). 문자열은 앱의 `locale::service::builtin_by_id` 를
  그대로 재사용(로그인 페이지 전용 복제본을 만들지 않음) — `settings.language` 로 선택하되
  `"system"`이거나 en/ko/ja 가 아니면 영어로 대체(로그인 페이지는 OS 로케일을 알 방법이 없는
  순수 Rust 서버 렌더라 프론트의 `resolve_language` 흐름과 다름). 팔레트는 중립 다크 기본 +
  `prefers-color-scheme: light` 오버라이드. `X-Forwarded-Proto` 헤더가 `https` 가 아니면
  "이 연결은 암호화되지 않았습니다" 고지를 보여준다(서버 자체는 TLS 를 종단하지 않음 — 루프백
  평문 바인드가 전제, 터널 뒤에 있을 때만 이 헤더로 판별 가능).

### QA6 확정 결함 수정 (Hot Exit·Remote 비밀번호, 2026-08-14)

> 위 두 절("기능 확장 3차 계약 확정 추가")의 초판 구현에서 발견된 결함을 원인 해결한 기록.

- **경로 탈출**: `file_list_mirrors`/`file_prune_mirrors`/`file_mirror_untitled`/
  `file_list_untitled_mirrors`/`file_clear_untitled_mirror` 5종이 `projectId` 검증 없이 바로
  `buffers_dir(project_id)` 를 조립해, 열린 프로젝트가 아닌 임의 `projectId`(`"../.."` 등)로
  호출하면 앱데이터 밖 경로를 읽고/쓰고/삭제할 수 있었다(`file_mirror_dirty`/`file_clear_mirror`
  만 `ensure_within_root` 를 거치던 비대칭). 5종 모두 `root_guard::project_root` 로 "열린
  프로젝트인가"를 먼저 확인하도록 수정했고, `tabId` 를 파일명 컴포넌트로 쓰는 2종
  (`file_mirror_untitled`·`file_clear_untitled_mirror`)에는 신설
  `root_guard::ensure_safe_component`(경로 구분자·`.`/`..` 거부)를 추가했다.
- **원격 종료 제어**: `HotExitFlushRequested` 이벤트가 `fanout_remote_events!` 목록에 있어 원격
  브라우저에도 전파되었고, `file_flush_complete` 는 원격 디스패치가 허용되어 있어 dirty 모델이
  없는 원격 세션이 먼저 회신하면 데스크톱의 플러시가 끝나기 전에 `app.exit(0)` 이 실행될 수
  있었다. 이벤트를 fanout 목록에서 제거하고, `file_flush_complete` 는
  `remote_set_password`/`remote_clear_password` 와 같은 패턴으로 `dispatch.rs` 의 `match` arm 이
  `AppError::Forbidden` 을 즉시 반환하도록 막았다(`IMPLEMENTED_JSON_COMMANDS` 에는 파리티
  유지를 위해 그대로 남긴다).
- **비밀번호 게이트 fail-open**: `server.rs::is_password_configured` 가 키링 조회 오류를 `Ok(None)`
  과 동일 취급해(`.ok().flatten()`), 키체인 접근이 일시 실패하면 비밀번호가 설정돼 있어도
  링크 토큰만으로 즉시 세션이 발급됐다. `Ok(None)` 만 "미설정"으로, 그 외(`Ok(Some)`·`Err`)는 전부
  "설정됨"으로 fail-closed 하도록 수정.
- **⌘Q 가 hot-exit 핸드셰이크를 우회**: macOS 앱 메뉴의 `PredefinedMenuItem::quit` 은
  `NSApplication terminate:` 로 직행해 `WindowEvent::CloseRequested` 를 전혀 발생시키지 않는다
  (tao 의 macOS 델리게이트에 `applicationShouldTerminate:` 훅이 없음 — `applicationWillTerminate`
  가 곧장 `RunEvent::Exit`, 가로챌 지점이 없다). Quit 항목을 커스텀 `MenuItem`(accelerator
  `CmdOrCtrl+Q`)으로 교체해 클릭 시 메인 윈도우의 `close()` 를 호출하도록 바꿔, X 버튼과 동일한
  `CloseRequested` 경로를 타게 했다.
- **심볼릭 링크 경로의 미러 소실**: `mirror_dirty` 가 `MirrorFile.path` 에 `ensure_within_root` 가
  돌려준 canonicalize 된 실경로를 저장해, 심볼릭 링크로 열린 탭의 경로(프론트가 보내는 keepPaths·
  복원 비교 기준)와 어긋나 `prune_mirrors` 가 열려 있는 탭의 미러를 지워버렸다. 저장 위치(파일명
  해시)는 canonicalize 된 경로를 계속 쓰되, `MirrorFile.path`(비교용 필드)는 호출자가 IPC 로 보낸
  원래 경로 문자열을 저장하도록 분리(`storage_path`/`display_path` 파라미터 분리).
- **untitled 미러 GC 부재**: `prune_mirrors` 는 `buffers/` 최상위 파일만 훑어 `untitled/` 하위
  디렉터리를 건드리지 않고, 프론트의 `pruneUntitledContents` 는 재시작 후 비어 있는 메모리
  레지스트리에만 의존해 재시작 이후 닫힌 untitled 탭의 미러가 영구 잔존했다. 신설
  `file_prune_untitled_mirrors(projectId, keepTabIds)` 가 복원된 레이아웃(열린 탭 + 닫힌 탭
  undo 스택)을 authoritative keep 목록으로 삼아 프로젝트 활성화 시 `file_prune_mirrors` 와 함께
  스윕한다.
- **`remote_status` 5초 폴링마다 키링 접근**: `RemoteStatus.passwordConfigured` 조회가 매 폴링마다
  OS 키링을 때렸다. `RemoteStore` 에 `password_configured: bool` 캐시를 추가해 앱 부팅 시 1회,
  `remote_set_password`/`remote_clear_password` 호출 시에만 갱신하고 폴링은 캐시만 읽는다.
- **로그인 nonce 슬롯 1개**: `pending_login_nonce: Option<(Vec<u8>, Instant)>` 단일 슬롯이라 두
  기기가 연달아 링크를 발급받으면 먼저 발급받은 기기의 nonce 가 무효화돼 올바른 비밀번호를
  입력해도 실패로 집계됐다(잠금 카운터 오염). `session_token_digests` 와 같은 방식의
  `pending_login_nonces: HashMap<digest, 만료 Instant>` 로 교체해 여러 개의 진행 중 로그인을
  동시에 허용한다.
- **미러 쿼리 캐시 미동기화**(`FILE.MIRRORS`/`FILE.UNTITLED_MIRRORS`, `staleTime: Infinity`): 저장·
  view-disk·prune 시점에만 무효화되고 500ms 디바운스·flush 쓰기에는 캐시 갱신이 없어, `EditorPane`
  이 `key` 없이 같은 pane 안에서 재사용되는 탭 왕복(A→B→A)에서 (a) 프로젝트 활성화 이후 쓰인
  미러가 캐시에 없어 복원되지 않거나 (b) 캐시된 **오래된** 미러가 최신 편집 위에 재적용되는 두
  경로로 미저장 편집이 소실될 수 있었다. `editor-pane.tsx`/`untitled-pane.tsx` 의 미러 쓰기 경로를
  `persistMirror` 로 통일해, IPC 쓰기가 성공할 때마다 `queryClient.setQueryData` 로 해당 엔트리를
  직접 갱신한다(매 디바운스마다 전체 목록을 `invalidateQueries` 로 재조회하지 않고 캐시만 갱신 —
  I/O 비용 없이 신선도 유지).
- **flush 콜백이 쓰기 완료를 기다리지 않음**: `mirror-flush-registry` 의 `flushAllMirrors` 는
  `await` 를 지원하는데, 등록된 flush 콜백(`editor-pane.tsx`/`untitled-pane.tsx`)이
  `void mirrorDirty(...).catch(...)` 로 Promise 를 버리고 즉시 반환해 `HotExitFlushProvider` 가
  실제 쓰기 완료 전에 `file_flush_complete` 를 호출할 수 있었다. flush 콜백을 `async` 로 바꿔
  `persistMirror`(내부에서 `mirrorDirty`/`mirrorUntitled` 를 `await`)를 반환하도록 수정.

### Wave I 계약 확정 추가 (셸·워크스페이스 — 멀티 윈도우·Zen·AppFile·플러그인)

> 계약: `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md`. 상세: `layout-shell.md`
> §7, `window-chrome.md` §5, `tabs.md` §3.1/§4.4, `plugins.md` §6, `data-model.md` §8.

- **window(신설)**: mutation `window_open_auxiliary(projectId, windowSlot) → AuxiliaryWindowInfo{
  label, projectId, windowSlot }`(Rust 가 `editor-<n>` 라벨을 발급 — 호출부는 어떤 라벨이 나올지
  모른다), `window_set_fullscreen(fullscreen)`(호출한 창 자체가 대상 — `window: tauri::Window` 가
  Tauri 에 의해 자동 주입되므로 라벨 파라미터가 없다, `file_flush_complete` 와 동일 패턴).
- **layout(추가)**: mutation `layout_move_tab_to_window(tabId, target: TabWindowTarget)` —
  `TabWindowTarget = {kind:'main'} | {kind:'newAuxiliary'} | {kind:'existing', slot}`, 대상 보조
  창이 없으면(`newAuxiliary`) `window_open_auxiliary` 코어를 내부적으로 재사용해 새로 연다. 이동으로
  빈 보조 창은 서버에서 자동으로 닫힌다(`cleanup_emptied_auxiliary_windows`) — 단 탭을 그냥
  닫아서(이동이 아니라) 비게 된 보조 창은 이 경로를 타지 않으므로 자동으로 닫히지 않는다(프론트
  `auxiliary-window-shell.tsx` 가 자기 트리가 비면 스스로 `getCurrentWindow().close()` 하는 것으로
  대칭을 완성한다). `layout_set_shell_view(projectId, patch: ShellViewPatch{zen?, sidebarCollapsed?})`
  — main 창 전용 표시 상태(§`ShellViewState`, `data-model.md` §8).
- **app(신설 도메인)**: query `app_file_read(target: AppFileTarget)` → `string`, mutation
  `app_file_write(target, content)` → (성공 시 `settings:changed` 를 함께 발신하는 것은 `target.kind
  === 'settings'` 일 때만 — Prompt 타깃은 순수 쓰기). `AppFileTarget = {kind:'settings'} |
  {kind:'prompt', id: PromptTemplateId}`(`'auto-tab-default' | 'inline-edit-default' |
  'commit-message-default'`, `ai.md` §5 의 기존 파일과 동일 화이트리스트). 두 커맨드 모두
  `root_guard` 의 프로젝트 루트 검증 대상이 **아니다** — 경로 자체를 Rust `AppPaths` 가 유도하고
  프론트/레이아웃에는 절대 경로가 노출되지 않는다. `app_get_info` 도 같은 `app` 도메인에 속한다
  (§"app" 절 — 이 커맨드는 Wave I 이전부터 있었다).
- **settings(이벤트 실제 배선)**: `event: settings:changed` 는 이 문서에 이미 등재돼 있었지만 Wave I
  이전에는 실제로 발신되지 않았다(§2 확정 사실 7 — "load_settings 는 부팅 1회, 설정 변경 이벤트
  없음"). `settings::commands::apply_and_broadcast` 가 `settings_update`·`sync_download`·
  `app_file_write`(Settings 타깃) 세 진입점의 공용 코어가 되면서 셋 다 이제 실제로 이 이벤트를
  전 창(+원격, `lib.rs` 의 `fanout_remote_events!` 목록에 등록됨) 발신한다. 프론트는
  `ipc-sync-provider.tsx` 에서 이 이벤트로 `SETTINGS.CURRENT` 를 직접 `setQueryData` 한다(무효화가
  아니라 페이로드를 즉시 반영).
- **plugin(추가)**: mutation `plugin_install(sourcePath)` → `LoadedPlugin`(디렉토리 또는
  `.zip`/`.vsix` 아카이브 — 아카이브는 `vsix::service::extract_hardened_zip` 으로 추출, 이미 설치된
  id 면 거부), `plugin_uninstall(pluginId)` → `LoadedPlugin[]`(빌트인 보호 없음).
- **vsix(추가)**: mutation `vsix_import_plugin(vsixPath)` → `LoadedPlugin` — VS Code 확장의
  `contributes.languages`/`contributes.grammars` 를 읽어 `taide-plugin.json` 을 합성한 뒤
  `plugin_install` 과 동일한 검증·등록 경로로 착지시킨다(`plugins.md` §6). 기존
  `vsix_extract_themes` 는 그대로 있지만 **원격 dispatch 는 이번에 허용→거부로 전환**(아래 참조).
- **원격 dispatch 명시 거부(신규 6종)**: `window_open_auxiliary`·`window_set_fullscreen`·
  `layout_move_tab_to_window`·`plugin_install`·`plugin_uninstall`·`vsix_import_plugin` — 전부
  "원격 세션은 로컬 디스플레이/파일시스템 다이얼로그가 없다"는 기존 `file_flush_complete`/
  `remote_issue_link` 류 거부와 같은 근거. **`vsix_extract_themes` 도 이번에 허용→거부로 전환**했다
  — 이전에는 원격에서도 임의 로컬 파일 경로를 zip 으로 열어 읽을 수 있었다(§2 확정 사실 8, 제한적
  임의 파일 읽기 표면). `plugin_list`/`plugin_reload` 는 원격에서도 그대로 허용(읽기 전용). 전체
  거부 목록 전수는 §"원격 dispatch
  정책" 참조.
- **채널 다중화(내부 구현, 새 IPC 표면 아님)**: `lsp_spawn`(reuse 경로)과 `pty_attach` 는 이제 세션당
  구독자 목록을 유지해 여러 창이 같은 LSP 세션/pty 세션을 동시에 구독할 수 있다 — 메시지/출력은 전
  구독자에 브로드캐스트되고 `send` 가 실패한(창이 닫힌) 구독자는 다음 브로드캐스트에서 자동
  제거된다. 커맨드 시그니처·응답 타입은 변경되지 않았다.

### 손 QA 1차 발견 6건 수정 (2026-08-18)

> 계약: `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md`. 사용자 실기 QA 6건 중 IPC 표면에
> 영향을 준 4건(터미널 링크·와일드카드·파일 열기 블로킹·커밋 diff)을 이 절에 담는다. 부트 테마·
> peek 트위스티 2건은 IPC 변경이 없다(`window-chrome.md`/`docs/bug` 참조).

- **system(신설)**: mutation `system_open_external_url(url)` — `http://`/`https://` 접두(ASCII
  대소문자 무시) 화이트리스트 + 제어문자·공백 거부만 검증하고(스킴 파싱용 `url` 크레이트는 도입하지
  않았다), 통과하면 `tauri_plugin_opener::open_url` 로 OS 기본 브라우저를 연다. 터미널
  (`terminal.md` §"링크")의 xterm `WebLinksAddon` 이 ⌘/⌥ 클릭으로 인식한 URL 이 유일한 호출부다.
  **원격에서는 명시 거부**(`system_open_external_url` 행 — 위 "원격 dispatch 정책" 표) — 원격
  세션이 데스크톱 자신의 OS 브라우저를 대신 열게 할 수는 없다는, `remote_issue_link` 자가 확장
  거부와 같은 근거.
- **layout**: `TabKind::Diff` 에 옵션 필드 3종 `rev`/`parentRev`/`beforePath`(전부
  `#[serde(default)]` — 구 `layout.json` 은 필드 없이도 그대로 역직렬화된다) 추가. `rev` 가 있으면
  그 diff 는 워킹트리/스테이지 비교가 아니라 **커밋 diff**(`git_show_file(rev, path)` vs
  `git_show_file(parentRev, beforePath ?? path)`)를 뜻한다 — 전용 `TabKind` 신설은 검토에서
  기각됐고(Wave C L0-2, `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md` §3) 기존 variant
  확장으로 처리했다. 신규 커맨드는 없다 — 위 `### git` 절의 "커밋 diff 의 에디터 탭 승격" 참조.
- **settings**: `remote_allowed_hosts` 항목에 `*.` 접두 와일드카드를 허용한다(RFC 6125, 선두
  1레이블만 매칭 — `*.example.com` 은 `foo.example.com` 은 매칭하지만 `example.com` 자신·
  `a.b.example.com` 은 매칭하지 않는다). sanitize(`is_valid_allowed_host`)는 맨몸 `*`·접미사가
  1레이블뿐인 와일드카드(`*.com`)·선두가 아닌 와일드카드(`foo.*.com`)·레이블에 섞인 와일드카드
  (`*foo.com`)를 전부 걸러낸다. 매칭(`remote::service::host_matches_allowed_entry`)은
  `is_allowed_host`(원격 Host 헤더 검증)와 `is_insecure_connection`(HTTPS 강제 예외) 양쪽이 공유하는
  단일 함수다 — `ends_with` 계열 유사 도메인(`evil-trycloudflare.com` 이 `*.trycloudflare.com` 을
  통과하는 함정)을 구조적으로 배제하려 `split_once('.')` 로 선두 1레이블만 떼어 비교한다.
  `remote_issue_link` 가 쓰는 `format_issue_link_url` 은 **첫 비와일드카드 항목**을 우선하고(와일드
  카드 패턴은 브라우저가 실제로 열 수 있는 호스트가 아니다), 등록된 호스트가 전부 와일드카드면
  루프백 링크로 폴백한다. 프론트 `remote-allowed-hosts-row.tsx` 의 `isValidAllowedHost` 가 같은
  형태 검증을 미러링한다(`remote.allowedHostsDescription` locale 문구도 함께 갱신).
- **lsp**: IPC 표면(커맨드 시그니처)은 변경 없음 — `lsp_stop`/`lsp_restart` 내부의 전역 뮤테이션
  가드 보유 구간이 줄고(가드 하 처리는 동기 북키핑 + `LspStore` 선제거까지만, `shutdown_entry` 는
  가드 밖에서 대기) `shutdown_entry` 의 고정 2s+2s sleep 이 프로세스 종료 폴링(`wait_for_process_exit`,
  상한은 기존 `LSP_SHUTDOWN_TIMEOUT_MS`)으로 바뀌었을 뿐이다. 프론트 `lsp-session-registry.ts` 의
  `releaseLspSession` 도 refCount 0 즉시 dispose 대신 `LSP_SESSION_DISPOSE_GRACE_MS`(5초) 유예 후
  dispose 하도록 바뀌었다(그 사이 재획득하면 타이머 취소 — 파일 전환 시 세션 재사용). 상세는
  `lsp.md` §5·`docs/bug/2026-08-18-lsp-stop-global-lock.md` 참고. **접합부 수정(Phase D + 메인
  2차)**: 유예 도입 직후엔 `flushLspSessionDisposal`(유예를 건너뛰고 즉시 dispose)이 정의만 되고
  어디서도 호출되지 않아, 유예 중이던 세션이 `project_close`(Rust 가 `LspStore` 를 건드리지 않는다)
  나 hot-exit 이후에도 계속 5초를 마저 기다린 뒤에야 정리됐다. Phase D 가 `ipc-sync-provider.tsx` 의
  `events.projectClosed` 핸들러에 `flushLspSessionsForProject(projectId)` 를, `HotExitFlushProvider`
  에 `flushAllLspSessions()`(간접 참조 이유는 아래)를 배선했지만, 그 시점의 `flushLspSessionsForProject`
  는 여전히 유예 타이머가 걸린 세션에만 작용했다 — `events.projectClosed` 는 Tauri IPC 콜백에서
  동기 도착해 React 가 그 프로젝트의 팬을 아직 언마운트하기 전이므로, 도착 시점의 세션은 전부
  `refCount ≥ 1`(유예 타이머 없음)이라 이 배선은 실제로는 항상 아무것도 하지 않는 구조적 no-op
  이었다. 메인 2차에서 `flushLspSessionsForProject` 를 refcount/유예 상태와 무관하게 해당
  프로젝트의 세션을 강제로 즉시 dispose 하도록 근본 수정했다(프로젝트가 닫히는 중이므로 그 세션을
  다시 참조할 팬도 곧 사라짐). 신설 `lsp-session-flush-registry.ts` 를 하나 더 둔 것은
  `hot-exit-flush-provider.tsx`·`ipc-sync-provider.tsx` 가 `lsp-session-registry.ts`(실제 monaco
  worker 를 import 하는 모듈이라 `bun test` 정적 임포트 그래프 해석이 실패한다)를 직접 참조하지
  않게 하기 위해서다 — `lsp-session-registry.ts` 가 자기 모듈 로드 시 `flushAllLspSessionDisposals`
  ·`flushLspSessionsForProject` 를 이 레지스트리에
  스스로 등록한다.

### 원격 dispatch 정책 (허용 · 거부 · 부분 스트립)

> `dispatch.rs` 의 `IMPLEMENTED_JSON_COMMANDS` 는 179종 전부(raw 3종 제외)를 담고, 파리티 테스트
> (`bindings와_dispatch_테이블은_커맨드_이름_집합이_일치한다`)가 이 목록과 `bindings.ts` 의 커맨드
> 이름 집합이 완전히 같음을 강제한다. **목록에 있다고 전부 원격에서 실제로 실행되는 것은 아니다** —
> `dispatch()` 의 `match` arm 이 실제로 무엇을 하는지에 따라 아래 3갈래로 나뉜다.

- **기본 허용(위 목록에서 언급되지 않은 나머지 전부)**: `match` arm 이 실제 핸들러로 그대로 위임한다.
  예: `git_*` 전종·`file_*`(아래 예외 제외)·`ai_*` 8종·`plugin_list`/`plugin_reload`/
  `plugin_read_grammar`·`remote_status`/`remote_start`/`remote_stop`/`remote_revoke_sessions`·
  `sync_*`·`search_replace`(원격 세션도 파일을 직접 고쳐 쓸 수 있다 — 기존 설계상 허용, 별도 강화
  없음)·`theme_save`/`theme_delete`·`snippet_save`/`snippet_delete`·`git_init`.
- **명시 거부(20종)** — `match` arm 이 핸들러를 부르지 않고 즉시 `AppError::Forbidden` 을 반환한다.
  `IMPLEMENTED_JSON_COMMANDS` 에는 파리티 유지를 위해 그대로 남아 있다(코드는 커맨드별 `deny_remote_*`
  헬퍼):

  | 커맨드 | 거부 사유(요약) |
  |--------|-----------------|
  | `remote_set_password` / `remote_clear_password` | 원격 세션이 자기 접속 게이트를 바꾸지 못하게 |
  | `remote_issue_link` | 이미 인증된 원격 세션이 스스로 새 온보딩 링크를 발급해 접근을 자가 확장하지 못하게(Wave B §6) |
  | `file_flush_complete` | 데스크톱 자신의 `CloseRequested` 종료 시퀀스만 재개 가능(Hot Exit) |
  | `window_open_auxiliary` / `window_set_fullscreen` / `layout_move_tab_to_window` | 원격 세션에는 대응할 로컬 디스플레이/OS 창이 없음(Wave I) |
  | `plugin_install` / `plugin_uninstall` / `vsix_import_plugin` | 데스크톱 로컬 파일시스템의 임의 경로를 이름으로 받음(Wave I) |
  | `vsix_extract_themes` | Wave I 에서 허용→거부로 전환 — 임의 로컬 파일 읽기 표면이었음 |
  | `system_open_external_url` | 원격 세션이 데스크톱 자신의 OS 기본 브라우저를 열게 할 수는 없음(손 QA 1차 수정, 아래 절 참조) |
  | `system_open_path` / `system_reveal_path` / `system_open_in_browser` / `system_open_app_data_path` | `system_open_external_url` 과 동일 계열·동일 사유로 허용→거부 전환(손 QA #12, 2026-08-18) — 넷 다 `tauri_plugin_opener` 로 데스크톱 자신의 화면에 앱 창(기본 앱 열기/Finder·Explorer 표시)을 띄우는데, 원격 세션은 그 창을 보거나 쓸 방법이 없다. `system_open_path` 는 (`system_reveal_path`/`system_open_in_browser` 와 동일하게 `resolve_within_open_project` 로 프로젝트 루트에 가드되어 있었음에도) 애초에 이 3종과 함께 거부됐어야 할 대상이 이번에 뒤늦게 합류했다 — 경로 자체의 안전성이 아니라 "원격이 못 보는 창을 여는가"가 거부 기준이므로, 루트 가드 여부와 무관하게 넷 다 같은 결론이다 |
  | `agent_cli_install` / `agent_cli_uninstall` | macOS 에서 관리자 권한 프롬프트(`osascript`)를 데스크톱 자신의 화면에 띄우거나 `/usr/local/bin` 에 직접 심링크를 건다 — 원격 세션이 보거나 답할 수 없는 권한 승격 창(T0 감사 #12, 2026-08-18) |
  | `agent_pending_external_opens` | `AgentStore` 의 대기 중 외부 열기 큐(`taide open --wait`)는 세션 구분 없는 단일 큐라 먼저 호출한 쪽이 통째로 비운다. 원격 세션이 드레인하면 `waitMarker` 등록이 원격 realm 의 `agent-wait-marker-registry.ts` 에 남아 데스크톱 탭 종료로는 해제되지 않고, 외부 CLI 프로세스가 앱 종료 전까지 블록된다(T0 감사 #14) |
  | `lsp_install` | `plugin_install`/`vsix_import_plugin` 과 동일 계열 — 수백MB 언어서버 아카이브를 데스크톱 로컬에 내려받고 인스톨러 프로세스를 spawn 한다(T0 감사 #16) |

- **스코프 조건부 거부(1종)**: `agent_hooks_install` 은 `agentName` 으로 `hook_scope_for_agent` 가
  결정하는 스코프에 따라 분기한다 — `HookInstallScope::Project`(`claude`, 프로젝트 루트 하위
  `.claude/settings.local.json` 에 `project_root` 로 루트 가드됨)는 원격에서도 그대로 허용,
  `User` 스코프(`codex`/`gemini`, 홈 디렉터리의 `~/.codex/hooks.json` / `~/.gemini/settings.json` 에
  루트 가드 밖 **command 훅**을 주입)는 `AppError::Forbidden` 으로 거부한다. User 스코프 훅은
  TAIDE CLI 가 모든 훅 이벤트마다 실행하는 셸 커맨드를 심는 것과 같아, 원격 세션이 종료돼도 살아남는
  백도어가 된다는 점에서 `settings_update` 가 스트립하는 `shellOverride` 와 같은 근거다(T0 감사 #13).
  `agentName` 을 알 수 없으면(미지원 이름) 이 분기가 아니라 실핸들러의 `InvalidArgument` 로 위임되어
  동일한 에러를 낸다.
- **부분 스트립(핸들러는 호출하되 민감 필드를 지운 뒤 위임, 2종)**:
  - `settings_update`: patch 에서 `remotePasswordOnlyLogin`·`remoteAllowedHosts`·`shellOverride`
    3필드를 `None` 으로 스트립한 뒤 위임(`remoteAccessEnabled` 는 자가 차단=자기 접근 상실이라
    스트립 대상에서 제외) — Wave B 하드닝에서 1필드(`remotePasswordOnlyLogin`)에 나머지 2필드가
    추가됐다.
  - `app_file_write`(`target.kind === 'settings'` 일 때만): 파싱된 전체 `Settings` 에서 같은 3필드를
    현재(적용 전) 값으로 되돌려쓴다 — patch 가 아니라 파일 전체 대치라 "지운다"가 아니라 "현재값
    유지"로 구현된다(`settings_update` 와 동급 게이트 — Wave I §3.3). `target.kind === 'prompt'`
    는 스트립 없이 그대로 위임한다.
- **raw 채널 3종**(`pty_spawn`·`pty_attach`·`file_read_raw`)은 specta 파리티 대상이 아니라
  `IMPLEMENTED_JSON_COMMANDS` 목록 자체에는 없지만, `dispatch()`/`dispatch_raw()` 의 별도 `match`
  arm 으로 **원격에서도 그대로 허용**된다 — 목록에 없는 것이 원격 차단을 뜻하지 않는다(아래 "raw
  커맨드" 절 참조).

### T0 감사 데이터·기능 수정 (2026-08-18)

> 계약: `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` §2.2~§2.4. 보안(§2.1) 5클러스터는
> 위 "원격 dispatch 정책" 절에 이미 반영돼 있다. 이 절은 **IPC 표면(타입·필드·커맨드 시그니처)에
> 영향을 준 항목만** 담는다 — 내부 구현만 바뀐 항목(watcher `.git` 필터, `search_replace`/`pty_write`
> 락 범위, `write_atomic_preserving_mode`, `delete_theme`/`load_theme` 경로 가드)은 커맨드
> 시그니처·반환 타입이 그대로라 여기 없다(상세는 `docs/bug/2026-08-18-audit-t0-fixes.md`).

- **git — `StatusRow`/`CommitFile` 절대경로 동봉(#18)**: 두 타입 모두 `absPath: string` (+ `origPath`
  가 있을 때만 나란히 채워지는 `origAbsPath: string | null`)이 새로 붙는다. `path`/`origPath` 는
  그대로 저장소-상대 경로(git2 반환값 그대로, git 커맨드에 되돌려 넘길 때 계속 쓴다)이고, `absPath`
  는 Rust 가 `repo.workdir()` 로 조인해 계산한 **파일 도메인(`file_open`/`file_save`) 이 요구하는
  절대경로**다. 소비부(`git-panel.tsx`) 는 "파일 열기"/"경로 복사"/"탐색기에 표시" 3곳에서
  `row.absPath` 로 전환됐다 — git 커맨드(`git_stage`/`git_unstage`/`onOpenChanges`(diff 탭) 등)에는
  계속 상대 `row.path` 를 넘긴다(그 경로들은 항상 이 `StatusRow`/`CommitFile` 이 속한 저장소 기준으로
  해석되므로 절대경로 변환이 필요 없다). `commit-detail-panel.tsx` 의 diff 탭 오픈(`TabKind::Diff`)도
  `git_show_file(rev, path)` 에 상대경로를 그대로 쓴다 — 파일 도메인을 거치지 않기 때문이다.
- **settings — `SettingsPatch` 클리어 가능 문자열 필드 일반화(#22, 사용자 결정 7)**: `shellOverride`·
  `editorFontFamily`·`terminalFontFamily`·`uiFontFamily`·`aiProvider`·`aiModel` 6필드가 `aiOmlxBaseUrl`
  이 먼저 쓰던 3상태 규약(빈 문자열=해제)을 그대로 따른다 — **필드 생략(`null`)=건드리지 않음,
  `""`=명시적으로 `None` 으로 되돌림(폰트 "System Default"·AI Provider/Model 미선택 등), 비어있지
  않은 문자열=그 값으로 설정**. `Option<Option<T>>` 은 배제하고 `Option<String>` 단일 레이어를
  유지한다(구 클라이언트가 이 필드들을 생략하면 여전히 "건드리지 않음"으로 해석되어 하위호환). 프론트
  피커(`settings-view.tsx` 의 폰트 패밀리 콤보박스·AI Provider 전환 시 `aiModel` 리셋)는 값이 없을 때
  `null` 이 아니라 `''` 를 patch 에 실어 보내도록 함께 갱신됐다 — 이전에는 `shellOverride:
  value.trim() || null` 처럼 빈 값을 `null` 로 보내던 자리가 있었는데, 새 규약에서 `null` 은
  "건드리지 않음"이라 그 자리들은 전부 해제가 조용히 무시되는 회귀였다.
- **tree/search — `SearchMatch` 좌표를 UTF-16 코드유닛 기준으로 정정(#23)**: `column`(1-based)·
  `matchStart`/`matchEnd`(0-based, `preview` 문자열 안 오프셋) 세 필드 모두 값의 **단위**가 Rust
  UTF-8 바이트에서 monaco `Position`/JS 문자열 slice 가 쓰는 UTF-16 코드유닛으로 바뀌었다(필드
  이름·타입은 `u32` 그대로). ASCII 텍스트는 두 단위가 같아 값이 그대로였지만, 한글 등 비 ASCII
  문자 뒤의 매치는 이전까지 실제보다 큰(바이트 기준) 좌표를 반환해 하이라이트/커서 이동이 밀렸다.
- **lsp — 백그라운드 크래시 재시작 후 상태를 `Crashed` 로 보고(#24)**: `handle_process_exit` 의
  자동 재시작(`RESTART_BACKOFF_LIMIT` 이내)이 프로세스 재기동에 성공해도 `lsp:session-status-changed`
  는 이제 `status: "running"` 이 아니라 `status: "crashed"`(+ 안내 메시지)를 낸다 — 재기동된 프로세스는
  `initialize` 핸드셰이크가 다시 이뤄진 적이 없어 실제로는 요청에 응답하지 못하는데, 이 배경 재시작은
  프론트의 어떤 액션도 기다리고 있지 않아(`lsp_spawn`/`lsp_restart` 와 달리) 재핸드셰이크를 걸어줄
  주체가 없다. `Running` 을 정직하지 못하게 보고하는 대신 `Crashed` 를 유지해 사용자가 수동
  재시작(`lsp_restart`, 이 경로는 프론트가 그 다음 `initialize` 를 스스로 보낸다)을 트리거하게
  한다. 근본 수정(세션 세대 이벤트로 프론트가 자동으로 재핸드셰이크)은 T1-D 로 이월.
- **terminal — 탭/프로젝트 종료 시 pty 세션 회수(#21, 사용자 결정 9)**: 새 커맨드는 없다 — 기존
  `project_close`/`layout_close_tab` 내부 동작이 바뀐다. `project_close` 는 그 프로젝트 소유의
  모든 pty 세션을 `TerminalStore::kill_project` 로 일괄 kill 한다(이전에는 프로젝트를 닫아도
  터미널이 앱 종료까지 계속 살아있었다). 터미널 탭을 닫으면(`layout_close_tab`, 실제로는 둘 다 거치는
  공유 경로 `close_tab_and_finish`) `TabKind::Terminal.sessionId` 를 `TerminalStore::kill_session`
  으로 개별 회수한다(**접합부 수정 — Phase D**: 계약 §3 은 이 절반을 "Rust: 탭 닫기 시 pty_kill"로
  적어뒀으나 R1/R2/F 어느 구현에서도 실제 호출부가 배선되지 않아 `pty_kill`/`killPty` 호출부가 0건인
  채로 남아 있었다 — `close_tab_and_finish` 에 직접 배선해 닫았다). 둘 다 `PtySession::drop` 이
  reader/flusher 스레드와 일시정지 게이트를 정리하므로 중복 kill(예: `pty_kill` 로 먼저 죽은 세션을
  `kill_project` 가 다시 순회) 은 무해하다(`kill` 자체가 멱등 — 이미 종료된 프로세스에 대한 kill 은
  에러를 반환할 뿐 패닉하지 않고, 호출부가 `let _ =` 로 무시한다).
- **project — `project_close` 부수효과 확장(#2·R4#7, IPC 시그니처 불변)**: 레이아웃이
  `dirty_layouts` 에 남아 있으면(2초 주기 flusher 가 아직 못 따라잡은 상태) 제거 전에 동기
  flush 한다(이전엔 이 경합에서 미저장 레이아웃이 경고 없이 버려졌다). `GitStore`(projectId→repo_root
  캐시)·`TreeStore`(트리 캐시)를 각각 `remove(project_id)` 로 회수한다(이전엔 프로젝트를 다시 열면
  옛 repo_root/디렉터리 목록이 부활했고, 앱 수명 내내 메모리에 남았다). 전체 회수 목록의 정본은
  `architecture.md` §6.3.
  **asset 프로토콜 스코프는 회수하지 않는다(X1#7, T0 #15 롤백 후 T1 2차로 재이월)** — Tauri
  `asset_protocol_scope()`(`scope::fs::Scope`)의 `forbid_directory` 는 되돌릴 수 없는 추가 전용
  API 라, 순진하게 호출하면 같은 폴더를 다시 열어도 영원히 `asset://` 를 못 읽는 새 회귀가
  생긴다. 근본 수정(자체 `register_uri_scheme_protocol("asset", ...)` 재구현 + 열린 프로젝트
  root 집합 기반 동적 판정)은 위험도·범위 문제로 이 배치에서 보류했다 — 사유는 `architecture.md`
  §6.3 참고. **현재도 닫힌 프로젝트 트리를 webview 가 계속 읽을 수 있다** (이 항목만 예외).

### raw 커맨드 (specta 밖)

`RAW_CHANNEL_COMMANDS`(`src-tauri/src/lib.rs`)에 등록된 3종은 specta 를 통과하지 못해
`bindings.ts` 에 생성되지 않는다. `invoke()` 로 직접 호출한다. 셋 다 `dispatch()`(`pty_spawn`·
`pty_attach`) 또는 `dispatch_raw()`(`file_read_raw`)의 전용 `match` arm 으로 원격에서도 다뤄진다
(§"원격 dispatch 정책" 참조) — `IMPLEMENTED_JSON_COMMANDS` 배열에 없는 것은 파리티 테스트 대상이
아니기 때문일 뿐, 원격 미지원을 뜻하지 않는다.

| 커맨드 | 이유 |
|--------|------|
| `pty_spawn` / `pty_attach` | raw 바이트 `Channel`(`InvokeResponseBody::Raw`) — `Vec<u8>` 로 보내면 JSON 숫자 배열이 되어 성능이 붕괴한다 |
| `file_read_raw(path)` | 미리보기용 원본 바이트를 `ArrayBuffer` 로 전달. 20MB(`READ_ONLY_FILE_BYTES`) 상한은 Rust 가 강제 |

## 4. 보안 (NFR-7)

- **view 는 fs/shell 계열 Tauri 플러그인 API 를 직접 호출하지 않는다** — 파일·프로세스 접근은 전부
  위 커스텀 command 경유(Rust 가 경로·인자 검증). capabilities 는 core 기본 + dialog 등 최소만 허용.
- 경로 파라미터는 Rust 에서 canonicalize + 프로젝트 루트/허용 범위 검증(터미널 링크 해석 등
  루트 밖 접근은 명시된 command 만).
- CSP: Monaco 요구(`style-src 'unsafe-inline'`, `worker-src 'self' blob:`)를 포함해 최소로 조인다
  (`editor.md` §1).
- Remote 도메인: 비밀번호·해시·세션/nonce 토큰 원문은 로그(`log::info!`/`log::warn!`)·IPC 응답·
  이벤트 payload 어디에도 실리지 않는다(`RemoteStatus` 는 `passwordConfigured: bool` 만 노출).
  비교는 전부 `constant_time_eq` 경유(토큰·비밀번호 동일 경로). 자세한 흐름은 §3 "Remote 비밀번호"
  절, 커맨드별 허용/거부 전체 그림은 §3 "원격 dispatch 정책" 절 참조.
- **zip 아카이브 하드닝(Wave I)**: `plugin_install`(아카이브 경로)·`vsix_import_plugin` 이 공유하는
  `vsix::service::extract_hardened_zip` 은 엔트리 수 상한(`VSIX_ARCHIVE_MAX_ENTRIES` 5000)·누적
  압축 해제 바이트 예산(`VSIX_ARCHIVE_MAX_TOTAL_BYTES` 128MB — zip bomb 방어)·경로는
  `enclosed_name()`(zip-slip 방어, `../` 를 포함한 엔트리는 스킵)·파일 모드는 항상 `0o644`/디렉토리는
  `0o755` 로 고정(zip 안에 담긴 unix 모드 비트를 신뢰하지 않는다)한다. 기존 `infra::lsp_install::
  extract_zip`(신뢰된 자체 배포 아카이브 전용, zip-slip 만 방어)과는 별개 함수로 분리했다 — 사용자가
  임의로 고른 `.vsix`/`.zip` 은 신뢰 전제가 다르기 때문(`plugins.md` §6).
