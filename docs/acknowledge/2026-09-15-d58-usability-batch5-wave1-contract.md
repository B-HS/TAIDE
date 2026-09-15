# d-58 — 사용성 배치 5 웨이브 1: 알림 문구 · Open Recent 메뉴 · 퀵오픈 결함 · Welcome 터미널 · 실시간 검색+아이콘 · + 메뉴 · git 패널 그래프 분리 · 메뉴 아이콘 정렬 (2026-09-15)

> 사용자 항목 4·6·7·8·9·10·11·14 (`acknowledge/2026-09-15-usability-batch5-user-decisions.md` §1·§2 결정 전제).
> 조사 정본 `research/2026-09-15-batch5-research.md`(T3·T5·T6·T7·T8). 실행 방식 `docs/agent-operations.md` §2. 이 문서의 §3 은 구현 에이전트가 실시간 기록한다.
> 규칙: 코드 주석 금지(JSDoc 만, 영어)·arrow only·매직넘버 상수화·이모지 금지·`#[allow]`/`@ts-ignore`/`eslint-disable` 금지·신규 의존성 0.
> 신규 로케일 키는 `src-tauri/resources/locales/{en,ko,ja}.json` 3종 + 새 네임스페이스면 `src-tauri/src/domain/locale/service.rs` `MESSAGE_NAMESPACES` 동시 등재(키 집합 일치 테스트가 강제).
> Rust 커맨드 추가 시 `lib.rs` 등록 + `domain/remote/dispatch.rs` `IMPLEMENTED_JSON_COMMANDS` + `REMOTE_ALLOWED_COMMANDS`/`REMOTE_DENIED_COMMANDS` 등재 + `docs/ipc-contract.md`. bindings 재생성은 `cargo test -p taide typescript_바인딩을_생성한다`(`src/shared/api/bindings.ts` 갱신·커밋 대상).

## 0. 근본 원인 · 현행 (메인이 소스로 재확인)

| 항목 | 사실 | 근거 |
|------|------|------|
| 14 메뉴 아이콘 정렬 | `ContextMenuSubTrigger` 클래스에 `gap-2` 가 없다. 일반 `ContextMenuItem`(`gap-2`)·`DropdownMenuSubTrigger`(`gap-2`)와 달라 하위메뉴 트리거(탭 Split·탐색기 Open With·터미널)의 아이콘-라벨 간격이 어긋난다 | `src/shared/ui/context-menu.tsx:44` vs `:97`, `src/shared/ui/dropdown-menu.tsx:149` |
| 9 검색 아이콘 | `FileGroupHeader` 가 아이콘 레지스트리를 안 거치고 lucide `File` 을 하드코딩(`opacity-80`, 색 없음). 탐색기·탭 바는 `FileTypeIcon → resolveFileIcon`. 문제 패널도 같은 헤더를 써 동일 증상 | `src/shared/ui/file-group-header.tsx:29`, `src/features/explorer/file-tree-row.tsx:73`, `src/features/problems/problems-panel.tsx:100` |
| 9 검색 실시간 | 검색 패널은 Enter 전용(`key !== 'Enter'` 즉시 return). `run()` 진입 즉시 이력 push(settings 전체 write) + 정규식 오류마다 `toast.error`. 명시 `searchCancel` 은 과거 레이스 회귀라 금지(generation ref + Rust `begin_search` 토큰 flag 로 supersede) | `src/features/search/search-panel.tsx:118-121`, `src/entities/search/use-search-run.ts:16,45-51,125,133,152-156` |
| 8 Welcome 터미널 | `terminal.new` 실행부가 `command-palette.tsx` 로컬 클로저에 갇혀 재사용 불가. 터미널은 프로젝트 소속(Rust `ensure_project_open`) → 비활성 조건은 기존 `canOpenFile`(`projectId !== null`)과 동일 | `src/widgets/command-palette/command-palette.tsx:166-172`, `src/widgets/welcome/welcome-container.tsx:91` |
| 10 + 버튼 | `IconButton` onClick 이 곧바로 폴더 피커. 드롭다운 정본 패턴은 `tab-bar-add-menu.tsx`(Tooltip↔DropdownMenuTrigger 중첩 순서 포함). `~` 확장은 `terminal/service.rs` `expand_home` 만 존재, `open_project` 는 `canonicalize` 직행 | `src/widgets/app-sidebar/app-sidebar.tsx:78-84`, `src/features/tab/tab-bar-add-menu.tsx:24-33`, `src-tauri/src/domain/project/service.rs:57-68`, `src-tauri/src/domain/terminal/service.rs:212` |
| 6 Open Recent | 앱 메뉴는 TAIDE/Edit/Window 3개(영문 하드코딩), File 메뉴·Open Recent 없음. `handle_menu_event` 는 `MENU_ID_QUIT` 외 early return. `list_recent_projects` 는 `last_opened_at` 내림차순·상한 없음·`root_missing` 재계산. `close_project` 는 디스크 레코드(`projects/<id>/`)를 남겨 Clear Recent 백엔드 없음. Dock 메뉴 API 부재는 결정 문서 §2 | `src-tauri/src/domain/window/commands.rs:155-200,269-276`, `src-tauri/src/domain/project/service.rs:120-132,417-427` |
| 4 알림 | `agentCompleted` body 가 `agent.name`("claude") 한 단어. `AGENT_COMPLETION_ACTIVITIES = ['idle','awaitingInput']` 라 권한 대기도 "작업 완료" 제목으로 나감. `evaluateAgentCompletions` 의 `workedForMs` 는 소비처 없음. payload 에 `projectId` 있음. 프로젝트명·탭 제목은 쿼리 캐시(`QUERY_KEY.PROJECT.LIST`, `layoutQueryOptions`)로 IPC 없이 조달 가능(선례 `git.query.ts:214`). 플러그인 데스크톱은 title/body 만 전달, 클릭 콜백 없음 | `src/app/providers/native-notification-provider.tsx:63`, `src/shared/lib/native-notification-gate.ts:30,76` |
| 7 퀵오픈 | ① 랭킹이 점수 단독 정렬 후 200 상한 → 동점 다수 시 walk 순서가 커트라인(비결정·간헐) ② 코드베이스에 유니코드 정규화 0건, `fuzzyMatch` 는 코드포인트 정확 비교 → NFD 한글 파일명은 NFC 질의로 0건 ③ `target:null` 에 `focused_pane` 이 트리에 없으면 로케일 키 없는 `NotFound("pane not found")` + FE 가 파일 소실로 오인해 인덱스 무효화(백로그 기존 결함) ④ 심링크 파일은 `file_type().is_file()` 필터로 누락 ⑤ `openFile` 이 프로젝트 없으면 조용히 return ⑥ 보조 창엔 팔레트 미마운트라 ⌘P 무반응 ⑦ 선검증 실패 토스트가 "파일을 찾을 수 없습니다: {path}" (사용자 보고 문구) | `src/widgets/command-palette/command-palette.tsx:60,291,324-328`, `src/shared/lib/fuzzy-match.ts:43-56,105`, `src-tauri/src/domain/layout/service.rs:429,1331`, `src-tauri/src/domain/search/service.rs:592-598`, `src/app/app.tsx:57-97`, `docs/backlog.md:154` |
| 11 git 패널 | 그래프 뷰포트가 `VIEWPORT_HEIGHT_PX = 320` 고정 + `overflow-y-auto scrollbar-hidden`(레포 유일의 비-`ScrollContainer` 스크롤). 짧은 패널에선 뷰포트를 삼키고 긴 패널에선 커지지 않으며, 접힌 섹션이 양보한 공간을 그래프가 못 받아 하단이 빈다. 접힘 상태는 모듈 메모리(재시작 초기화, Settings 승격을 의도적으로 거절한 주석) | `src/widgets/git-panel/commit-graph.tsx:45,125`, `src/widgets/git-panel/git-panel.tsx:380-493`, `src/entities/git/git-section-collapse-memory.ts:21-35` |

## 1. 수정 방향

### 1.A 항목 14 — 컨텍스트 메뉴 하위메뉴 트리거 간격 (S, TS)

- `src/shared/ui/context-menu.tsx` `ContextMenuSubTrigger` 클래스에 `gap-2` 추가(일반 Item·Dropdown SubTrigger 와 동일). `ChevronRightIcon` 은 `ml-auto` 유지(크기는 `[&_svg:not([class*='size-'])]:size-4` 가 담당). 다른 변경 없음.
- 검증: 탭 컨텍스트 메뉴 Split·탐색기 Open With·터미널 하위메뉴가 Pin/Move 와 같은 아이콘 열에 정렬. 시각 확인은 사용자 실기.

### 1.B 항목 9 — 검색 패널 실시간 검색 + 결과 아이콘 (M, Rust settings + TS)

**B1 아이콘** — `src/shared/ui/file-group-header.tsx:29` 의 `<File …/>` 를 `<FileTypeIcon fileName={fileNameOf(path)} className='size-3.5 shrink-0' />`(`@shared/icons/file-type-icon`, 동일 레이어 참조) 로 교체. `opacity-80` 제거(레지스트리 `colorClass` 가 색을 준다). 검색·문제 패널 동시 해결. git `StatusRowItem` 은 범위 외.

**B2 설정 필드(Rust)** — `Settings` 에 `search_on_type: bool`(기본 `true`) · `search_on_type_debounce_ms: u32`(기본 `300`, `service.rs` 에서 `[50, 2000]` 클램프, 상수 `SEARCH_ON_TYPE_DEBOUNCE_MIN_MS/MAX_MS/DEFAULT_MS`). `SettingsPatch` 옵션 필드·`Default`·`apply_patch`·클램프 테스트. bindings 재생성. `docs/ipc-contract.md` settings 절 + `docs/features/search.md`(있으면) 갱신.

**B3 설정 UI** — `src/widgets/settings-view/` 에 검색 절이 없으므로 `settings-interface-section.tsx` 에 "검색" 소제목 아래 토글(`settings.searchOnType` / `…Description`) + 숫자 입력(`settings.searchOnTypeDebounceMs` / `…Description`, ms 단위, 기존 숫자 설정 UI 패턴 재사용). 로케일 4키 × 3.

**B4 실시간 실행(TS)** — `src/widgets/search-panel/search-panel-container.tsx`:
- 설정 `searchOnType` 이 켜져 있고 `query.trim()` 이 비어 있지 않으면 질의·옵션(대소문자/정규식/단어/제외 glob)이 바뀔 때 **trailing 전용 디바운스**(`searchOnTypeDebounceMs`)로 `run(buildQuery(), { recordHistory: false, live: true })`. 타이머는 `useEffect` cleanup 으로 취소(외부 시스템=타이머 동기화라 허용). `createLeadingTrailingDebouncer` 는 leading 즉시 실행이라 부적합 — trailing 옵션을 추가하거나 effect 타이머로 구현(2회 이상 쓰이지 않으면 새 공용 유틸 만들지 않는다).
- Enter 는 현행대로 즉시 실행 + `recordHistory: true`(대기 중 디바운스 취소). 이력은 Enter 에서만 쌓인다.
- `RunSearchOptions` 에 `live?: boolean` 추가: live 실행은 (a) 정규식 컴파일 실패 시 `toast.error` 를 띄우지 않고 `status='failed'` + 패널 인라인 문구(기존 실패 표시 재사용, 없으면 `search.invalidRegexInline` 1키)만, (b) `perfMark(SEARCH_RUN_REQUESTED)` 를 찍지 않는다(metric 6 표본 보존). Enter 실행은 현행 유지.
- 이전 실행 취소는 **추가 작업 없음**(generation ref + Rust supersede). 명시 `searchCancel` 을 넣지 않는다(`use-search-run.ts:45-51` 회귀).
- IME: WKWebView 가 composition 이벤트를 주지 않으므로(`src/shared/lib/ime-composition.ts`) keydown 229 가드는 한글 입력 전체를 라이브에서 제외해 버린다 → **디바운스만으로 감수**(문서에 명시). 최소 질의 길이는 두지 않는다(VS Code 동형).
- 안내 문구: 설정 on 이면 `search.liveSearchHint`("입력하면 바로 검색됩니다. Enter 로 이력에 저장") 신규 1키, off 면 기존 `search.pressEnterHint`.
- Search Editor 탭(`search-editor-pane.tsx`)은 범위 외(Enter 유지).
- 테스트: 디바운스 스케줄/취소·Enter 즉시·live 옵션 분기(토스트 억제·이력 미기록)·설정 off 시 현행 — `search-panel-container` 컴포넌트 테스트(RTL) 또는 순수 헬퍼 추출 후 단위 테스트. Rust 클램프·기본값 테스트.
- 문서: `docs/features/search.md`(또는 검색 절이 있는 문서) 실행 트리거 절, `docs/quality-assurance/2026-09-04-perf-baseline.md` metric 6 정의에 "live 실행은 표본 제외" 1줄.

### 1.C 항목 8 — Welcome "터미널 열기" 버튼 (S, TS)

- `src/entities/layout/layout.query.ts` 에 `useOpenTerminalTab(projectId: ProjectId | null)` 추출(선례 `useOpenAppFileTab` :80-92): projectId 없으면 `toast.info(t('app.openProjectFirst'))`, 있으면 `openTab({ kind: { kind: 'terminal', sessionId: '' }, title: t('terminal.title'), target: currentWindowFocusedPane(layout), preview: false })` + 에러 토스트. `command-palette.tsx:166-172` 로컬 클로저를 이 훅으로 교체(동작 동일 — 단 `target` 은 기존 `null` 유지가 팔레트 동작이면 그대로 두고 훅 인자로 받는다; 두 호출부가 같은 target 규칙이면 통일).
- `src/features/welcome/welcome-screen.tsx`: props `canOpenTerminal: boolean` · `onOpenTerminal: () => void` 추가, 기존 "파일 열기" 옆에 `variant='outline'` + lucide `Terminal` 아이콘 + 라벨 `t('keymap.newTerminal')`(기존 키 재사용) 버튼, `disabled={!canOpenTerminal}`. 힌트 줄은 기존 `app.openFileHint` 를 "파일이나 터미널을 열려면 먼저 폴더를 여세요" 로 문안 갱신(en/ja 동일 의미). `welcome-container.tsx` 는 `canOpenTerminal={projectId !== null}`.
- 테스트: `pane-node-view-welcome.test.tsx`·welcome 관련 테스트에 버튼 존재/비활성/클릭 → 훅 호출 단언. `docs/features/*` Welcome 절 1줄.

### 1.D 항목 10 — 사이드바 + 버튼 드롭다운 (S~M, TS + Rust `~` 확장)

- 신규 `src/features/project/sidebar-add-project-menu.tsx`(순수 UI): `tab-bar-add-menu.tsx` 의 `DropdownMenu > Tooltip > TooltipTrigger asChild > DropdownMenuTrigger` 조립을 그대로 따른다(현 `IconButton` 은 자체 Tooltip 을 품어 중첩 충돌 → 사용하지 않음). 항목: **경로로 열기…**(`FolderInput` 아이콘) · **Finder 로 열기…**(`FolderOpen`, 현 폴더 피커) · 구분선 · **최근 프로젝트**(라벨 `app.recentItems` 재사용) 아래 최근 N개(`RECENT_PROJECT_MENU_LIMIT = 10`, `src/shared/constants/project.ts` 신설 — Welcome 의 `RECENT_PROJECT_DISPLAY_LIMIT = 8` 은 그대로 두되 같은 파일로 승격) — 각 항목은 `resolveProjectDisplay` 라벨(없으면 `name`) + 보조 텍스트 `root`, `rootMissing` 이면 disabled + `app.recentProjectRootMissing`. 현재 열려 있는 프로젝트는 목록에서 제외. props 로만 동작(`onOpenByPath`·`onOpenViaFinder`·`recentProjects`·`onSelectRecent`).
- 신규 `src/features/project/open-project-by-path-dialog.tsx`(순수 UI, `create-tag-dialog.tsx` 패턴): 제목 `sidebar.openByPathTitle`, 입력 placeholder `sidebar.openByPathPlaceholder`("/path/to/project 또는 ~/project"), 확인 버튼(기존 공통 키 있으면 재사용), Enter 제출, 빈 값 비활성.
- `src/widgets/app-sidebar/app-sidebar.tsx`: `IconButton` 을 위 메뉴로 교체, `useOpenFolderDialog`(Finder)·`useOpenProject().mutate(path, { onError: toast.error(describeIpcError) })`(경로)·`useQuery(recentProjectsQueryOptions())`·`useActivateProject`/`useOpenProject`(최근 선택 — Welcome `onSelectRecent` 과 같은 규칙) 배선. 열기 성공 시 다이얼로그 닫힘.
- **Rust `~` 확장**: `open_project` 진입에서 `~`/`~/…` 를 홈으로 확장한다. `terminal/service.rs` 의 `expand_home` 을 `infra`(예: `infra/paths` 또는 기존 공용 위치)로 옮겨 두 곳이 공유(2회 사용 → 공통화). 홈은 `HOME`(Windows `USERPROFILE`) — `agent/commands.rs:592 home_dir_env` 와 중복이면 하나로. 원격 dispatch 경로에도 동일 적용됨을 `docs/ipc-contract.md` project_open 절에 명시. 테스트: `~` 단독·`~/x`·`~user`(확장 안 함) 케이스.
- 경로 검증은 `open_project` 기존 에러(`error.project.pathNotDirectory`·io NotFound)를 그대로 토스트. 존재하지 않는 경로의 io 에러가 로케일 키 없이 원문으로 뜨면 `error.project.pathNotFound` 1키를 추가해 `NotFound` 로 감싼다.
- 로케일 신규(3종): `sidebar.addProjectMenu`(aria/툴팁), `sidebar.openByPath`, `sidebar.openViaFinder`, `sidebar.openByPathTitle`, `sidebar.openByPathPlaceholder`(+ 필요 시 `error.project.pathNotFound`). 기존 `sidebar.openFolderAriaLabel` 은 사용처가 없어지면 제거(카탈로그 미참조 테스트 확인).
- 테스트: 메뉴 렌더/항목 클릭 콜백/최근 비활성(RTL), 다이얼로그 제출, Rust `expand_home` 이동 후 기존 테스트 유지.

### 1.E 항목 6 — 메뉴바 `File > Open Recent` + Clear Recent (M, Rust)

- `build_app_menu`(`src-tauri/src/domain/window/commands.rs`): `File` 서브메뉴(id `taide-file`)를 TAIDE 와 Edit 사이에 신설. 내용: `Open Recent ▸`(서브메뉴 id `taide-open-recent`) → 최근 프로젝트 최대 `RECENT_PROJECT_MENU_LIMIT`(= 10, `src-tauri/src/constants.rs`) 항목(id `taide-recent:<projectId>`, 라벨 = `display.label` 있으면 그것, 없으면 `name`; `root_missing` 이면 `enabled(false)`), 없으면 비활성 항목 `No Recent Projects`, 구분선, `Clear Recent`(id `taide-clear-recent`, 목록이 비면 비활성).
- `handle_menu_event`: `taide-recent:` 접두 → 해당 `Project.root` 로 `project::commands::project_open(app.clone(), app.state(), root)` 를 `tauri::async_runtime::spawn` 으로 직접 호출(선례 `remote/dispatch.rs:863`; 프론트 우회 없음 — 창 0개여도 동작). 실패는 `log::warn!`. `taide-clear-recent` → 신규 커맨드 `project_forget_recent` 로직 호출.
- 신규 커맨드 `project_forget_recent`(인자 없음, 원격 정책 `LocalProjectHistoryExposure` 로 `REMOTE_DENIED_COMMANDS`): 현재 세션에 **열려 있지 않은** 프로젝트의 `projects/<id>/` 레코드를 삭제(열린 것은 유지). 완료 후 FE 최근 목록(`QUERY_KEY.PROJECT.RECENT`)이 갱신되도록 기존 `ProjectListChanged` 이벤트를 방출하고, `ipc-sync-provider.tsx:237` 핸들러가 `PROJECT.RECENT` 도 무효화하게 한다(현재 무효화 범위 확인 후 추가). `lib.rs` 등록·dispatch 테이블·bindings·`docs/ipc-contract.md` project 절.
- **동적 갱신**: `project_open`·`project_close`·`project_activate`·`project_forget_recent` 뒤에 `window::menu::refresh_recent_menu(app)`(신규 헬퍼)가 `AppHandle::menu()` → `taide-open-recent` 서브메뉴의 항목을 전부 제거 후 재구성. macOS NSMenu 변경은 메인 스레드여야 하므로 tauri `Submenu::remove_at/append` 가 내부에서 메인 스레드 마샬링을 하는지 `~/.cargo/registry/src/*/tauri-2.11.5/src/menu/submenu.rs` 로 확인하고, 아니면 `app.run_on_main_thread` 로 감싼다(확인 결과를 §3 에 기록). 갱신 실패는 `log::warn!`.
- 라벨 로케일: `domain/locale/service.rs` 가 내장 카탈로그를 Rust 에서 조회할 수 있는 함수를 노출하면 현재 `settings.locale` 로 `menu.file`·`menu.openRecent`·`menu.noRecentProjects`·`menu.clearRecent` 를 조회해 쓴다(키 4개 × 3, 새 네임스페이스 `menu` 등재). 조회 함수가 없으면 카탈로그 소유자인 locale 서비스에 `lookup_builtin_message(locale, key) -> Option<String>` 을 추가한다(다른 도메인이 JSON 을 직접 읽지 않는다). 기존 TAIDE/Edit/Window/Quit 영문 하드코딩은 범위 외(§4 후속). 로케일 변경 시 메뉴 재구성은 `SettingsChanged` 처리 지점에서 `refresh_recent_menu` 호출로 해결(전체 메뉴 재빌드가 필요하면 `set_menu` 재호출 — 1회 비용이라 허용).
- Dock 메뉴는 구현하지 않는다(결정 §2). `docs/features/window-chrome.md`(또는 메뉴 절이 있는 문서)에 File 메뉴·Open Recent·Clear Recent·Dock 제약을 기술.
- 테스트: `list_recent_projects` 상한 적용 헬퍼(`recent_menu_entries`)·`forget_recent`(열린 프로젝트 보존·레코드 삭제)·메뉴 id 파싱. 메뉴 실물은 사용자 실기.

### 1.F 항목 4 — 알림 문구 강화 (S~M, TS 전용)

- `src/shared/lib/native-notification-gate.ts`: `evaluateAgentCompletions` 의 완료 항목이 `activity`('idle' | 'awaitingInput')·`workedForMs`·`sessionId`·`name` 을 싣도록(이미 있는 값 노출). 테스트 갱신.
- `src/app/providers/native-notification-provider.tsx`: `useQueryClient()` 로 (a) `QUERY_KEY.PROJECT.LIST` 캐시에서 `projectId` 의 `ProjectRef` → 표시명 = `resolveProjectDisplay({display}).label ?? name`, (b) `layoutQueryOptions(projectId)` 캐시에서 `sessionId` 를 가진 터미널 탭의 제목(없으면 생략). 새 IPC 없음(선례 `git.query.ts:214`).
- 포맷(결정 §1 #5): **title = 프로젝트 표시명**(프로젝트를 못 찾으면 기존 이벤트 제목으로 폴백), **body = `{에이전트} · {이벤트}: {세부}`**.
  - idle: `notification.agentFinishedBody` = "{{agent}} · 작업 완료 · {{duration}}{{tab}}" (tab 은 " · {탭 제목}" 조각, 없으면 빈 문자열 — i18n 보간으로 처리)
  - awaitingInput: `notification.agentAwaitingInputBody` = "{{agent}} · 입력 대기 (권한 요청 또는 질문){{tab}}"
  - category 는 둘 다 기존 `agentCompleted` 유지(설정 카테고리 분리는 웨이브 3 에서 Rust 타입과 함께).
- 같은 규칙을 다른 카테고리에도 적용: task(`taskCompleted*`) → title 프로젝트명(payload 에 projectId 가 없으면 sessionId 로 레이아웃 캐시 역인덱스해 프로젝트를 찾고, 못 찾으면 현행), body = "{탭 제목 또는 cwd} · 종료 코드 {n} · {duration}"; git → title 프로젝트명, body = "{이벤트} · {브랜치}"; searchReplace → title 프로젝트명, body 현행 요약. lspInstall 은 프로젝트 무관이라 현행 유지.
- 경과 시간 포맷: `src/shared/lib/format-duration.ts` 신설(`formatDurationShort(ms)` → "3분 12초"/"45초"/"1시간 2분", 로케일 키 `duration.hours/minutes/seconds` 또는 `Intl` 기반 — 기존 헬퍼가 없음을 확인함) — task 와 agent 두 곳이 쓰므로 공통화 정당. 테스트 포함.
- 로케일: 신규 body 키들 × 3, 기존 `notification.agentCompleted` 는 폴백 제목으로 유지.
- 테스트: `native-notification-provider.test.ts`(idle/awaitingInput 문구·프로젝트명·탭 제목·폴백)·gate 테스트·format-duration 테스트.
- 문서: `docs/features/notifications.md`(또는 알림 절이 있는 문서)에 포맷 표 + "클릭 라우팅은 플러그인 제약으로 불가" 명시.

### 1.G 항목 7 — 퀵오픈 결함 일괄 (M, TS + Rust)

- **G1 랭킹 결정화(TS)** — `src/shared/lib/fuzzy-match.ts` `fuzzyFilter` 정렬: 점수 내림차순 → 파일명(마지막 세그먼트) 매치 우선 → 대상 문자열 길이 오름차순 → `localeCompare`. 안정·결정적. 테스트(동점 집합의 순서 고정). `docs/features/command-palette.md:84-86` 의 "MRU" 서술을 실제 규칙으로 정정(MRU 는 미구현 — 후속 §4).
- **G2 NFC 정규화(TS)** — 질의와 매칭 대상 라벨을 `normalize('NFC')` 로 통일해 매칭·하이라이트한다(표시 문자열도 정규화본 — 시각 동일). **열기에는 원본 경로**를 그대로 쓴다(디스크 바이트 보존). `fuzzyFilter` 에 정규화를 넣으면 keybindings-editor 등 다른 소비처도 영향 → 팔레트 파일 그룹에서만 적용할지 `fuzzyFilter` 옵션으로 둘지는 "2회 이상" 기준으로 판단(팔레트 커맨드·심볼 그룹도 한글 라벨을 가질 수 있으므로 `fuzzyFilter` 내부 정규화가 자연스럽다). `splitFileMatchForDisplay` 의 인덱스 정합 테스트(NFD 파일명 + NFC 질의).
- **G3 pane 소실 폴백(Rust)** — `layout/service.rs` `open_tab_and_finish`: `target` 이 `None` 이고 `focused_pane` 이 트리에 없으면 `NotFound` 대신 **첫 leaf pane 으로 폴백**하고 `focused_pane` 을 그 pane 으로 갱신(`log::info!`). 명시 `target` 이 없는 pane 이면 현행 `NotFound` 유지하되 로케일 키 `error.layout.paneNotFound` 를 붙인다(원문 노출 제거). 테스트 2건. `docs/backlog.md:154` 항목을 "해결(d-58)" 로, `docs/features/layout-shell.md`(또는 tabs.md) 열기 규칙 1줄.
- **G4 심링크 파일(Rust)** — `search/service.rs::list_project_files`: 엔트리가 심링크면 `std::fs::metadata(path)`(대상 추적)로 파일 여부를 판정해 포함. 디렉토리 심링크는 현행(미추적) 유지 → 순환 위험 없음. 테스트(임시 디렉토리에 파일 심링크).
- **G5 하드 제외 디렉토리 완화 — 보류.** `IGNORED_DIR_NAMES` 가 트리·워처와 공유됨이 확인돼 결정 문서 §2.1 로 재질문 중. 이 계약에서는 **손대지 않는다**(회신 후 별도 항목).
- **G6 무반응 제거(TS)** — `command-palette.tsx` `openFile`: 프로젝트 없으면 `toast.info(t('app.openProjectFirst'))`(같은 파일 `openTerminalTab` 과 동일).
- **G7 보조 창 안내(TS)** — 보조 창 셸(`src/app/app.tsx` aux 분기 또는 `widgets/auxiliary-window-shell`)에 `useGlobalKeymap({ 'quick-open': …, 'command-palette': … })` 로 `toast.info(t('palette.mainWindowOnly'))`(신규 1키 × 3, "명령 팔레트는 메인 창에서 사용할 수 있습니다"). 창 스코프 팔레트는 웨이브 5.
- **G8 진단 로그(Rust)** — `log` 매크로로 ① `layout/commands.rs` `ensure_file_tab_target_exists` 실패 시 `warn!(path, 사유: 경계/부재)` ② G3 폴백 발생 시 `info!(이전 focused_pane)` ③ `search/commands.rs` `search_list_files` 반환 건수·소요 `debug!`. 시크릿 없음(경로만). `docs/debugging.md` §4 에 "퀵오픈 실패 사후 판별" 항목(로그 파일 위치는 §1 참조).
- 테스트·문서는 각 항목에 명시. `command-palette.md` 에 정렬·NFC·심링크 규칙, 보조 창 제약.

### 1.H 항목 11 — git 패널 그래프 하단 Panel 분리 + Settings 영속 (M, TS + Rust settings)

- **레이아웃(TS)** — `src/widgets/git-panel/git-panel.tsx` 본문을 `react-resizable-panels` 세로 `Group` 으로: **Panel A** = 기존 `ScrollContainer`(가상화 변경 3그룹 + 스태시 섹션, sticky 헤더·로빙·`scrollPaddingStart` 전부 현행 유지) / `PaneSeparator`(`src/features/split/pane-separator.tsx` 재사용, 방향 세로) / **Panel B** = 그래프: 헤더(`GitSectionHeader`, `shrink-0` 24px, sticky 아님) + 본문 `flex-1 min-h-0` 을 `ScrollContainer` 로(OverlayScrollbar — `scrollbar-hidden` 원시 뷰포트 제거, `VIEWPORT_HEIGHT_PX` 삭제). Panel B 는 `collapsible` + `collapsedSize` = 헤더 높이(24px), `minSize` = `MIN_PANEL_SIZE_PX`(`@shared/constants/layout`)… 헤더 접힘 토글은 `panelRef.collapse()/expand()` 와 접힘 상태를 동기화(`onLayoutChanged`/`isCollapsed`). 접힘 시 Panel A 가 남은 공간을 전부 받는다.
  - rrp 4.12.2 실 API(명령형 collapse/expand·`collapsedSize` px 문자열·Group 자식 제약)는 `node_modules/react-resizable-panels`(설치 완료) 의 타입/README 로 확인하고 §3 에 기록. 기존 사용처 `app-shell.tsx:221-250`(collapsible 사이드바) 패턴을 우선 따른다.
  - 로빙 포커스: 그래프 헤더는 더 이상 가상화기 인덱스 공간에 없다. `rovingItemCount`/`scrollToIndex` 에서 그래프 항목을 분리하고, 목록 끝(스태시 헤더)에서 ↓ 는 그래프 헤더 버튼으로 DOM 포커스 이동, 그래프 헤더에서 ↑ 는 목록 마지막 항목으로 복귀. `change-row-navigation.ts`·테스트 갱신.
  - e2e `e2e/specs/19-git-sections-collapse.e2e.ts:40` 의 sticky 단언은 변경/스태시 헤더에만 적용되도록 수정(그래프 헤더는 패널 내 고정). e2e 실행은 하지 않는다(사용자 몫) — `typecheck:e2e` 만 통과.
- **영속화(Rust settings 승격, 결정 §2 #3)** — `Settings` 에 `git_sections_collapsed: Vec<GitSectionId 문자열>`(기본 `["stashes"]`) · `git_graph_panel_size_px: u32`(기본 `240`, `[24, 4000]` 클램프) 추가(`SettingsPatch`·`Default`·`apply_patch`·테스트·bindings). 모듈 메모리 `git-section-collapse-memory.ts` 는 제거하고 settings 쿼리를 읽는다(초기 렌더는 settings 캐시가 있으므로 깜빡임 없음 — `settings` 가 아직 없으면 기본값). 쓰기: 접힘 토글은 즉시 `updateSettings({...emptySettingsPatch(), gitSectionsCollapsed})`, 리사이즈는 `onLayoutChanged(isUserInteraction)` 에서 `pane-resize-commit.ts` 와 같은 120ms 트레일링 디바운스로 `gitGraphPanelSizePx` 저장(상수 재사용). UI 노출 없음(`recent_searches` 선례).
  - `git-section-collapse-memory.ts` 의 "승격 거절" JSDoc 근거는 이번 결정으로 폐기 — 파일 삭제 시 참조 문서(`docs/features/git.md`·`architecture.md` §6.4 언급) 갱신.
- 문서: `docs/features/git.md` §2(구조)·§5(로빙)·"스크롤바 하나" 서술을 "변경/스태시 영역 + 그래프 영역 2개" 로 정정, 그래프 Panel·영속화 기술. `docs/ipc-contract.md` settings 절.
- 테스트: `git-sections.test.ts`·`change-row-navigation.test.ts` 갱신, git-panel 컴포넌트 테스트(접힘 시 헤더만·펼침·설정 저장 호출) RTL, Rust settings 테스트.

### 1.I 범위 외

- 항목 1·2(키맵, 웨이브 2)·5(에이전트, 웨이브 3)·13(테마, 웨이브 4)·3·12(프로젝트 split, 웨이브 5). Dock 메뉴(불가). 알림 클릭 라우팅(불가). 권한 요청/질문 구분(웨이브 3). G5(보류). Search Editor 실시간. git `StatusRowItem` 파일 아이콘. 메뉴바 기존 영문 문자열 로케일화. 팔레트 MRU 정렬.

## 2. 실행 계획

- **구현 wf(opus·xhigh)**: Rust 는 한 시점 한 에이전트 — **R** 에이전트가 B2(settings 검색)·H(settings git)·D(`expand_home` 이동+`open_project` 확장)·E(File 메뉴·`project_forget_recent`)·G3/G4/G8 을 순차 구현 후 bindings 재생성·`cargo fmt`·`clippy -D warnings`·`cargo test --workspace` 통과 → 이어서 TS 에이전트 병렬: **T1** A+C, **T2** B1·B3·B4, **T3** D(TS), **T4** F, **T5** G1·G2·G6·G7, **T6** H(TS). 각 TS 에이전트 종료 조건: `bun run typecheck`·`bun run lint`·`bun run format:check`·관련 `bun test` exit 0, §3 자기 항목 기록(아래 플레이스홀더만 교체 — 다른 항목 텍스트 수정 금지).
- **렌즈 검토 wf(sonnet·xhigh)**: 근본성/회귀/경계 3렌즈. major 는 적대적 검증 2표.
- **메인 2차 검증**: `bun run verify` + `bunx vite build` + `bun run typecheck:e2e` → 분할 커밋(항목별 feat/fix) → dev 푸시 → main ff.
- 사용자 실기: 메뉴 정렬·검색 실시간·Welcome 버튼·+ 메뉴·File 메뉴·알림 문구·git 그래프 리사이즈/접힘·퀵오픈.

## 3. 구현 기록 (에이전트 실시간 기록)

<!-- d58-record-R -->
<!-- d58-record-T1 -->
<!-- d58-record-T2 -->
<!-- d58-record-T3 -->
<!-- d58-record-T4 -->
<!-- d58-record-T5 -->
<!-- d58-record-T6 -->

## 4. 후속

- G5 하드 제외 디렉토리 정책(결정 문서 §2.1 회신 후).
- 메뉴바 기존 문자열(TAIDE/Edit/Window/Quit) 로케일화.
- 팔레트 MRU 정렬(문서가 약속했던 동작) — 실수요 확인 후.
- 알림 카테고리 "입력 대기" 분리 + 권한/질문 구분(웨이브 3 d-60 예정).
- 보조 창 팔레트(웨이브 5).
