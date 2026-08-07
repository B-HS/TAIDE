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

- query: `project_list`, `project_get`, `project_get_active`
  (`project_get_active` 는 구현 중 추가 — 부팅 시 view 가 활성 프로젝트를 알 방법이 없었다.
  `project:activated` 이벤트는 전환 시점에만 오므로 초기 조회용 query 가 별도로 필요하다.)
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

- query: `theme_get(themeId)`, `theme_get_current(systemTheme)`, `theme_list`, `settings_get`
- mutation: `settings_set_theme(themeId)`, `settings_update(patch)`
  (구현 시 정정: 초안의 `theme_set(id)` 을 **`settings_set_theme`** 로 확정.
  선택된 테마 id 는 `Settings` 가 소유하는 값이라 mutation 도 settings 도메인에 두는 것이 맞다.
  theme 도메인은 읽기 전용(로드·해석)만 담당한다. `theme_get_current` 는 view 부팅 시
  "현재 설정의 테마"를 한 번에 받기 위한 query 로 추가.)
- mutation(7.5-D 추가): `theme_save(theme)`, `theme_delete(themeId)`
  — 테마 편집기(`theme-system.md` §7.3)용. 내장 테마 덮어쓰기를 거부하고,
  id 에 경로 구분자(`/ \ .`)가 있으면 거부한다(경로 탈출 방지).
- event: `theme:changed`, `settings:changed`

**`theme_get_current` 가 `systemTheme` 을 받는 이유(7.5-D 정정)**: `followSystemTheme` 판단은
Rust 가 하되 OS 다크/라이트 감지는 view 가 센서 역할을 한다. Rust 가 웹뷰 없이 OS 테마를
읽으려면 플랫폼 분기가 늘어나는데, view 에는 이미 `prefers-color-scheme` 이 있다.
locale 의 `locale_get_current(systemLanguage)` 도 같은 이유로 같은 형태다.

### locale (7.5-H 신설 — `acknowledge/2026-08-06-i18n-and-session-findings.md`)

- query: `locale_list`, `locale_get(localeId)`, `locale_get_current(systemLanguage)`
- 메시지는 **flat dotted key**(`"common.cancel"`). 중첩 객체가 아니다 —
  사용자 언어팩이 키 단위로 부분 오버라이드하기 쉬운 구조를 택했다.
- 사용자 팩은 `{app_data}/locales/*.json`. `extends` 로 내장(en/ko/ja)을 상속하고 바꾼 키만 담는다.

### font (7.5-D 신설)

- query: `font_list` → `FontFamily { name, monospaced }[]`
- `fontdb` 로 시스템 폰트를 열거한다. `monospaced` 플래그로 에디터·터미널 목록을 기본 필터링한다.

### agent 연동 (`agent-integration.md`)

- mutation: `release_wait_marker(marker)`
- event: `editor-bridge:open-file(path, waitMarker?)` (CLI/single-instance 유입)

### plugin (`plugins.md`)

- query: `plugin_list`
- mutation: `plugin_reload`, `plugin_set_lsp_enabled(pluginId, lspId, enabled)`

### 7.6 추가 (IDE 핵심 루프)

- git: `git_branches`, `git_branch_create(name, checkout)`, `git_branch_checkout(name)`,
  `git_branch_delete(name, force)`, `git_stash_list`, `git_stash_push(message?)`,
  `git_stash_apply(index)`, `git_stash_drop(index)`, `git_discard_hunk(path, hunkStart, hunkEnd)`
- search: `search_replace(query, replacement, paths?)` → `SearchReplaceResult { changedFiles, replacedMatches }`

**`git_discard_hunk` 의 좌표 계약**: hunk 경계는 `git_gutter` 가 반환한 `GutterHunk { start, end }`
를 그대로 넘긴다. Rust 쪽에서 두 함수가 **같은 diff 옵션과 같은 경계 계산 헬퍼**를 공유하므로
프론트가 받은 좌표와 어긋날 수 없다. 이 불변식이 깨지면 엉뚱한 줄이 되돌려진다.

### 7.7 계약 확정 추가

- system(신설): query `system_usage_get` → `SystemUsage { cpuPercent: number | null, memoryBytes: number }`.
  TAIDE 메인 프로세스만 측정한다(자손 프로세스 미포함). 첫 호출은 `cpuPercent: null`
  (CPU 사용률은 두 번째 샘플부터 유효). 프론트는 `refetchInterval` 폴링으로 주기를 소유하고,
  설정 토글이 꺼지면 호출 자체를 하지 않는다(`enabled`).
- layout: `layout_set_preview(tabId, preview)` 신설 — `layout_pin_tab` 과 동일한 형태.
  `TabKind::Terminal` 에 `cwd?: string | null` 필드, `TabKind::Diff` 에 `compareWith?: string | null`
  필드 추가(둘 다 기존 데이터와 하위 호환). `TabKind::ClaudeDiff { requestId, path }` variant 신설 —
  **레이아웃 영속 저장에서 제외되는 휘발성 탭**이다(저장 직전 필터링, 사라지는 활성 탭은 인접 탭으로 대체).
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
- capabilities: `opener:allow-reveal-item-in-dir` 추가 — "Finder 에서 보기" 계열 기능의 전제.

### raw 커맨드 (specta 밖)

`RAW_CHANNEL_COMMANDS`(`src-tauri/src/lib.rs`)에 등록된 3종은 specta 를 통과하지 못해
`bindings.ts` 에 생성되지 않는다. `invoke()` 로 직접 호출한다.

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
