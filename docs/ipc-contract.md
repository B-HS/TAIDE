# IPC 계약 (view ↔ Rust)

> 아키텍처 §4 의 상세. 타입 생성은 ADR-0011(tauri-specta), 전송 원칙 근거는
> `docs/research/tauri-v2.md`·`performance-memory.md`. **이 문서의 목록이 command·event 의 정본이며,
> 구현 시 추가·변경은 이 문서를 먼저 갱신한다.**

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

### project (`layout-shell.md`)

- query: `project_list`, `project_get`
- mutation: `project_open(path)`, `project_close(id)`, `project_activate(id)`, `project_reorder(ids)`
- event: `project:opened`, `project:closed`, `project:activated`, `project:list-changed`,
  `project:focus-kind-changed`

### layout (`tabs.md`)

- query: `layout_get(projectId)`
- mutation: `layout_open_tab(projectId, kind, target?)`, `layout_close_tab(tabId)`,
  `layout_activate_tab(tabId)`, `layout_move_tab(tabId, paneId, index)`,
  `layout_split(paneId, dir, tabId)`, `layout_resize(paneId, sizes)`, `layout_focus_pane(paneId)`,
  `layout_pin_tab(tabId, pinned)`, `layout_reopen_closed(projectId)`,
  `layout_set_view_state(tabId, viewState)`
- event: `layout:changed(projectId, revision)`

### file (`editor.md`)

- query: `file_open(path)` (내용+언어+크기 정책 판정), `file_read_raw(path)`(Response —
  뷰어 모드/대형)
- mutation: `file_save(path, content)`, `file_mirror_dirty(path, content)`, `file_create(path, isDir)`,
  `file_rename(from, to)`, `file_delete(path)`(휴지통), `file_copy(from, to)`
- event: `fs:changed(paths[], kind, origin)` (watcher — debounce·배치·echo 플래그)

### tree / search (`explorer-sidebar.md`)

- query: `tree_rows(projectId, offset, limit)`, `tree_node(projectId, nodeId)`
- mutation: `tree_expand(nodeId)`, `tree_collapse(nodeId)`
- event: `tree:changed(projectId, revision)`
- mutation(C): `search_start(projectId, query, opts, onResult: Channel)` → `searchId`,
  `search_cancel(searchId)`

### git (`git.md`)

- query: `git_status`, `git_gutter(path)`, `git_blame_range(path, from, to)`,
  `git_diff_file(path, mode)`, `git_show_file(rev, path)`, `git_log(skip, take)`, `git_refs`,
  `git_ahead_behind`, `git_remotes`, `git_stash_list`
- mutation: `git_stage(paths)`, `git_unstage(paths)`, `git_discard(paths)`,
  `git_commit(message, opts)`, `git_push`, `git_pull`, `git_fetch`, `git_undo_last_commit`,
  `git_stash_push/apply/pop/drop`
- event: `git:status-changed`, `git:refs-changed`, `git:operation-progress`, `git:operation-finished`

### terminal (`terminal.md`)

- mutation(C): `pty_spawn(opts, onData, onExit)` → sessionId, `pty_attach(sessionId, onData)`
- mutation: `pty_write(sessionId, data)`, `pty_resize(sessionId, cols, rows)`, `pty_kill(sessionId)`,
  `pty_set_paused(sessionId, paused)`, `terminal_report_cwd(sessionId, cwd)`
- query: `shell_profiles`, `terminal_sessions(projectId)`, `resolve_terminal_path(path, cwd)`
- event: `terminal:exited`, `agent:state-changed`

### lsp (`lsp.md`)

- mutation(C): `lsp_spawn(spec, folders, onMessage)` → sessionId
- mutation: `lsp_send(sessionId, message)`, `lsp_stop(sessionId)`, `lsp_restart(sessionId)`
- query: `lsp_sessions(projectId)`
- event: `lsp:session-status-changed`

### theme / settings (`theme-system.md`)

- query: `theme_get`, `theme_list`, `settings_get`
- mutation: `theme_set(id)`, `settings_update(patch)`
- event: `theme:changed`, `settings:changed`

### agent 연동 (`agent-integration.md`)

- mutation: `release_wait_marker(marker)`
- event: `editor-bridge:open-file(path, waitMarker?)` (CLI/single-instance 유입)

### plugin (`plugins.md`)

- query: `plugin_list`
- mutation: `plugin_reload`, `plugin_set_lsp_enabled(pluginId, lspId, enabled)`

## 4. 보안 (NFR-7)

- **view 는 fs/shell 계열 Tauri 플러그인 API 를 직접 호출하지 않는다** — 파일·프로세스 접근은 전부
  위 커스텀 command 경유(Rust 가 경로·인자 검증). capabilities 는 core 기본 + dialog 등 최소만 허용.
- 경로 파라미터는 Rust 에서 canonicalize + 프로젝트 루트/허용 범위 검증(터미널 링크 해석 등
  루트 밖 접근은 명시된 command 만).
- CSP: Monaco 요구(`style-src 'unsafe-inline'`, `worker-src 'self' blob:`)를 포함해 최소로 조인다
  (`editor.md` §1).
