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
- mutation: `git_init(projectId)`, `git_stage(paths)`, `git_unstage(paths)`, `git_discard(paths)`,
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

**페이로드 확장(7.10-W7)**: `Theme`·`ResolvedTheme` 에 `tokenColors?: TokenColorRule[]`(원본
TextMate 룰 전량 — 없으면 필드 자체가 생략) 필드가 추가됐다. `ResolvedTheme` 에는 추가로
`syntaxOverrides: string[]`(자식 테마가 스스로 명시한 `syntax` 키 목록 — 프론트가 오버레이 합성
근거로 쓴다)이 붙는다. colors/syntax/terminal 의 토큰 집합·타입은 불변이다 — `theme_get`/
`theme_get_current`/`theme_save` 세 커맨드 모두 이 확장된 형태를 주고받는다(자동 생성 타입은
`bindings.ts`, 상세 스키마는 `docs/theme-system.md` §2.1).

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

- query: `plugin_list`, `plugin_read_grammar(pluginId, languageId) -> string`(7.10-W7 신설 — grammar
  파일 본문을 온디맨드로 가져온다. `plugin_list` 응답에 본문을 인라인하지 않는 이유는 매니페스트
  스캔마다 수 MB 를 흘리지 않기 위함이다. 프론트는 highlighter 생성 시 이 query 로 각 플러그인의
  language 기여를 fetch 해 shiki `LanguageRegistration` 으로 주입한다 — 실패한 플러그인은 스킵 +
  경고)
- mutation: `plugin_reload`, `plugin_set_lsp_enabled(pluginId, lspId, enabled)`

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
  루트를 정하는 진입점 자체이므로 검증할 "루트"가 아직 없다).
- **grammar 는 다루지 않는다**: `vsix_extract_themes` 는 `contributes.themes` 만 읽고
  `contributes.grammars`(TextMate 문법)는 추출하지 않는다 — grammar 는 `contributes.languages`
  (언어 id·확장자)와 쌍으로 가져와야 의미가 있고, TAIDE 의 확장자→언어id 매핑
  (`LANGUAGE_ID_BY_EXTENSION`)이 Rust 컴파일 타임 상수라 런타임 오버레이화가 선행돼야 한다 —
  W7 에서도 범위 밖(`docs/backlog.md`).
- 도메인 분리 근거: `domain/plugin` 은 TAIDE 자체 선언적 확장 체계(`{app_data}/plugins/*/
  taide-plugin.json`, 부팅 시 스캔, ADR-0010 "코드 실행 없음")이고 `domain/vsix` 는 VS Code
  확장(.vsix) 이라는 **별개 포맷**을 사용자가 그때그때 선택한 파일에서 1회성으로 추출하는
  기능이다. 스키마·트리거·검증 규칙이 전혀 겹치지 않아(taide-plugin.json 의 경로 탈출 검증은
  플러그인 루트 기준, vsix 는 zip 아카이브 내 `extension/` 루트 기준) `domain/plugin` 에
  얹으면 두 스키마가 뒤섞인다 — 신규 `domain/vsix` 로 분리했다.

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
  "폴더 열기" 버튼에서 사용한다.
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
