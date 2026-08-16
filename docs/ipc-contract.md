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
- mutation: `file_save(path, content)`, `file_create(path, isDir)`, `file_rename(from, to)`,
  `file_delete(path)`(휴지통), `file_copy(from, to)`
- Hot Exit(기능 확장 3차 — §"기능 확장 3차" 참조): mutation `file_mirror_dirty(projectId, path,
  content, diskModifiedMs)`, `file_clear_mirror(projectId, path)`, `file_prune_mirrors(projectId,
  keepPaths)`, `file_mirror_untitled(projectId, tabId, content)`,
  `file_clear_untitled_mirror(projectId, tabId)`, `file_prune_untitled_mirrors(projectId,
  keepTabIds)`(재시작 후 레이아웃 기준 authoritative GC — untitled 탭이 열려 있거나 닫힘 스택에
  남아 있지 않으면 미러 삭제), `file_flush_complete()`(**원격 세션에서는 거부** — `dispatch.rs`
  가 명시 arm 으로 차단, 데스크톱 자신의 종료만 재개 가능); query `file_list_mirrors(projectId)`
  → `MirrorEntry[]`, `file_list_untitled_mirrors(projectId)` → `UntitledMirrorEntry[]`
  - 모든 project-scoped 미러 커맨드(`file_list_mirrors`·`file_prune_mirrors`·
    `file_mirror_untitled`·`file_list_untitled_mirrors`·`file_clear_untitled_mirror`·
    `file_prune_untitled_mirrors`)는 `projectId` 가 현재 열린 프로젝트인지
    `root_guard::project_root` 로 검증하고, `tabId` 를 파일명 컴포넌트로 쓰는 두 커맨드는 추가로
    `root_guard::ensure_safe_component` 로 경로 탈출 문자(`/`·`\`·`..`)를 거부한다(경로 조작 방지).
- event: `fs:changed(paths[], kind, origin)` (watcher — debounce·배치·echo 플래그),
  `app:hot-exit-flush-requested(timeoutMs)` (Hot Exit — 종료 인터셉트)

### tree / search (`explorer-sidebar.md`)

- query: `tree_rows(projectId, offset, limit)`, `tree_node(projectId, nodeId)`
- mutation: `tree_expand(nodeId)`, `tree_collapse(nodeId)`
- event: `tree:changed(projectId, revision)`
- mutation(C): `search_start(projectId, query, opts, onResult: Channel)` → `searchId`,
  `search_cancel(searchId)`

### git (`git.md`)

- query: `git_status`, `git_gutter(path)`, `git_blame_range(path, from, to)`,
  `git_diff_file(path, mode)`, `git_diff_staged_text(projectId) → StagedDiffText{ diffText,
  truncated, skippedFiles, usedFallback }`(Wave G — `ai.md` §4/§7 의 AI 커밋 메시지 생성 전용
  소비처, unified diff 텍스트 + 32KiB 상한 + 바이너리/lock 파일 제외; `usedFallback` 은 Wave H 신설
  — staged 델타 0건이면 HEAD↔워킹트리(untracked 포함) 전체 변경으로 폴백했다는 표시, 동일
  제외·상한 규칙이 폴백 diff 에도 적용된다),
  `git_show_file(rev, path)`, `git_log(skip, take)`, `git_refs`, `git_ahead_behind`, `git_remotes`,
  `git_stash_list`
- mutation: `git_init(projectId)`, `git_stage(paths)`, `git_unstage(paths)`, `git_discard(paths)`,
  `git_commit(message, opts)`, `git_push`, `git_pull`, `git_fetch`, `git_undo_last_commit`,
  `git_stash_push/apply/pop/drop`
- event: `git:status-changed`, `git:refs-changed`, `git:operation-progress`, `git:operation-finished`
- (이 목록은 stage/unstage hunk·conflict 해결·tag·revert·file log 등 이후 Wave 에서 추가된 git
  커맨드를 전부 반영하지 못한 기존 doc debt — Wave G 범위 밖이라 이번엔 `git_diff_staged_text` 만
  추가했다. 전체 재정리는 별도 문서화 작업으로 유예.)

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
  응답은 둘 다 실패가 아니라 `text: null`(빈 응답/취소)로 표현될 수 있다.
- 원격 dispatch: ai 8종 전부 허용(§4 공통 원칙과 별개로 명시 거부 arm 없음) — `ai.md` §8.

### terminal (`terminal.md`)

- mutation(C): `pty_spawn(opts, onData, onExit)` → sessionId, `pty_attach(sessionId, onData)`
- mutation: `pty_write(sessionId, data)`, `pty_resize(sessionId, cols, rows)`, `pty_kill(sessionId)`,
  `pty_set_paused(sessionId, paused)`, `terminal_report_cwd(sessionId, cwd)`
- query: `shell_profiles`, `terminal_sessions(projectId)`, `resolve_terminal_path(path, cwd)`
- event: `terminal:exited`, `agent:state-changed`

### task (Wave E 추가 — `tasks.md`)

- query: `detect_tasks(projectId)` → `Task[]`(`{ label, command, source, cwd }`) — 프로젝트 루트를
  훑기만 하는 부수효과 없는 조회. `source` 는 `npm`/`make`/`cargo`. 실행은 별도 command 없이
  프론트가 `Task.command` 를 기존 `pty_write` 경로로 포커스(또는 신규) 터미널 탭에 흘려보낸다
  (`terminal.md` §9 재사용 — Run Selected Text 와 동일 전달 경로).

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
  savedAtMs }`. mutation `file_mirror_dirty(projectId, path, content, diskModifiedMs)`(기존 시그니처에
  baseline 인자 추가 — 호출부는 `file.modifiedMs` 를 그대로 전달), `file_clear_mirror(projectId,
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
  자가 차단=자기 접근 상실이라 스트립 대상에서 제외).
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
- Remote 도메인: 비밀번호·해시·세션/nonce 토큰 원문은 로그(`log::info!`/`log::warn!`)·IPC 응답·
  이벤트 payload 어디에도 실리지 않는다(`RemoteStatus` 는 `passwordConfigured: bool` 만 노출).
  비교는 전부 `constant_time_eq` 경유(토큰·비밀번호 동일 경로). 자세한 흐름은 §3 "Remote 비밀번호"
  절 참조.
