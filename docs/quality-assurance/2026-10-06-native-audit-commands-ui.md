# Native 전환 감사 — 커맨드 팔레트·명령·키맵·공용 UI·i18n·아이콘

감사일 2026-10-06. 읽기 전용 감사이며 빌드·테스트는 실행하지 않았습니다. 판정은 실제 파일과 Grep 호출 체인으로만 했고, 이전 에이전트의 HANDOFF/PROCESS 수치는 사용하지 않았습니다.

## 0. 결론 요약

1. 커맨드 팔레트(5개 모드)는 native 에 화면도 상태도 없습니다. `native/` 아래 `.rs` 에서 팔레트 UI 를 가리키는 심볼은 0건이고(`palette` 검색 결과는 터미널 색 팔레트·테마 JSON 뿐), locale 키 `palette.*` 는 native Rust 코드 어디에서도 참조되지 않습니다.
2. "커맨드 레지스트리"도 없습니다. native 에는 TS `AppCommand` 에 대응하는 `run`/`isEnabled` 레지스트리가 없고, 키바인딩 편집기용 메타데이터 JSON(`keybinding-commands.json`, 비 monaco 53종 + monaco 157종)만 이식돼 있습니다. 실행 가능한 명령은 `shell_keymap.rs` 의 31개 키맵 id 와 터미널 점프 2개뿐입니다.
3. 키맵 엔진(chord, when, IME, 에디터 ⌘K 유예, override 파싱)은 TS 와 동일 수준으로 이식돼 실제 앱에 연결돼 있습니다. 기본 키맵 41개 엔트리는 TS `APP_KEYMAP` 과 id 단위로 일치합니다. 다만 41개 중 8개(⌘P, ⌘⇧P, ⌘T, ⌘F, ⌘⇧F, ⌘⇧H, ⌘⇧E, ⌃⇧G)는 실행 대상이 없습니다. 나머지 33개 중 31개는 `shell_keymap.rs`, 2개(터미널 점프)는 `terminal_surface.rs` 가 처리합니다.
4. 공용 UI 프리미티브(shared/ui)는 모듈로 이식되지 않았고, 화면마다 egui 기본 위젯을 직접 씁니다. 테마 토큰은 `shell_colors` 9개만 egui 에 반영되고 전역 `Visuals` 매핑이 없습니다.
5. i18n 은 양호합니다. TS 도 Rust `taide-locale` 카탈로그(en/ko/ja 각 1,108키)를 받아 쓰던 구조라 native 가 같은 `ResolvedLocale` 을 그대로 씁니다. 아이콘은 약 50개 SVG/수작업 도형뿐이고 lucide 약 100종 사용처 대비 부족하며, 탐색기 트리에는 파일/폴더 아이콘이 없습니다.

## 1. 범위

TS 읽은 경로: `src/app/bootstrap-commands.ts`, `src/widgets/command-palette/*`, `src/features/command-palette/*`, `src/shared/lib/{command-catalog,command-registry,command-palette-query,fuzzy-match,file-icon}.ts`, `src/shared/lib/keymap/{keymap,keybinding-catalog,keymap-category}.ts`, `src/shared/lib/monaco/{monaco-action-commands,monaco-actions}.ts`, `src/entities/{agent,ai,git,sync,task,terminal}/*.commands.ts`, `src/shared/ui/*`, `src/shared/hooks/*`, `src/shared/i18n/i18n.ts`, `src/shared/icons/*`, `src/shared/scroll/*`, `src/shared/styles/global.css`, `docs/features/{command-palette,keymap}.md`.

native 읽은 경로: `native/taide-native-ui/src/{commands,command-score,keymap,keybinding-catalog,keybinding-search,keybinding-commands.json,keymap-defaults.json,icons,tooltips,tooltip-trigger,css-motion,button-color-motion,status-chord,toast,presentation,shell,editor_surface,settings-code-view}.rs`, `native/taide-native-app/src/{keymap,shell_keymap,tooltips,application,terminal_surface,problems-icons,explorer,explorer_toolbar,editor_reveal,main}.rs`, `native/taide-native-app/resources/*`, `crates/taide-locale/src/service.rs` 와 `resources/locales/*.json`, `crates/taide-runtime/src/{layout,git,sync,search,agent_host}_actions.rs`.

## 2. 기능 대응표

판정 표기: done / partial / unwired / missing / n/a. effort 는 native 에 남은 작업량입니다.

### 2.1 커맨드 id 단위 (TS 등록 명령 53종 + monaco 군)

TS 등록 지점: `command-catalog.ts` 41개, `sync.commands.ts` 2, `agent.commands.ts` 3(macOS 한정), `ai.commands.ts` 1, `git.commands.ts` 4, `terminal.commands.ts` 1, `task.commands.ts` 1, `monaco-action-commands.ts` 약 158개(`taide.*` 7개 포함). native 의 id 목록은 `keybinding-commands.json` 에 53종 + `monaco.*` 157종으로 id 는 일치합니다(TS 대비 monaco 1건 차이는 확정하지 못함, 4장 참고).

| 기능 (command id) | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| window.reload | command-catalog.ts:20 | n/a | 웹뷰 새로고침 전용. 카탈로그 JSON 에는 행이 남아 있음(keybinding-commands.json:2) | 키바인딩 편집기에 실행 불가 행이 노출됨(결함 항목 참고) | S |
| settings.open | command-catalog.ts:21 | partial | 사이드바 설정 버튼 `shell.rs:294` → `ShellIntent::OpenSettings` → `application.rs:4130` | 명령/키바인딩 경로 없음. 프로젝트 없으면 `self.status` 문자열만 설정(토스트 아님) | S |
| app.openSettingsFile | command-catalog.ts:22 | partial | 설정 화면의 버튼 → `application.rs:4802-4808` `app_file_views::settings_command` | 명령/키바인딩 경로 없음 | S |
| keybindings.open | command-catalog.ts:28 | done | `shell_keymap.rs:87` → `ShellIntent::OpenKeybindings` → `application.rs:4164` | 없음 | - |
| terminal.new | command-catalog.ts:35 | done | `shell_keymap.rs:103,148` `NewTerminal`, 프로젝트 없으면 `ShowOpenProjectNotice` | 알림이 토스트가 아닌 status 문자열 | S |
| tab.reopenClosed | command-catalog.ts:42 | done | `shell_keymap.rs:108` → `ShellMutation::ReopenClosed` → `commands.rs:108` | 없음 | - |
| file.quickOpen | command-catalog.ts:49 | missing | 검색어: `quick_open`, `QuickOpen`, `file.quickOpen`, `"palette.filePlaceholder"` 전부 native 코드 0건. `quick-open` 은 keymap-defaults.json:3 과 테스트에만 존재 | 팔레트 전체 | L |
| tab.close | command-catalog.ts:56 (TS 명령은 항상 비활성, 키맵 `close-tab` 이 실작동) | done | `shell_keymap.rs:208` `RequestCloseTab` | 없음 | - |
| view.toggleSidebar | command-catalog.ts:64 | done | `shell_keymap.rs:113-121` `SetSidebarCollapsed`(zen 중 차단) | 없음 | - |
| editor.find | command-catalog.ts:71 (monaco 찾기 위젯 위임) | missing | 검색어: `find_bar`, `FindState`, `search_in_document`, `keymap.find`. `find` 키는 `ACTIONS`(shell_keymap.rs:8-40)에 없음 | 에디터 내 찾기/바꾸기 위젯 | L |
| search.find | command-catalog.ts:79 | missing | 검색 패널 UI 없음. 백엔드 `search_actions` 는 `remote-search.rs` 로만 노출 | 패널 열기·포커스 | L |
| search.replace | command-catalog.ts:80 | missing | 상동 | 바꾸기 모드 | L |
| view.explorer | command-catalog.ts:87 | missing | `explorer` 키는 ACTIONS 에 없음. 탐색기 사이드바는 항상 파일 뷰 | 사이드바 files/git 뷰 전환 | S |
| view.git | command-catalog.ts:94 | missing | 검색어: `GitPanel`, `git_panel`, `source_control`, `SidebarView`. native Git 사이드바 없음(git 은 remote-git.rs 만) | Git 뷰 전체 | XL(Git 영역) |
| view.welcome | command-catalog.ts:95 | partial | 빈 pane 에서 `shell.rs:853` `welcome` 렌더 | 명령으로 Welcome 탭 열기, 기존 탭 활성화 규칙 | S |
| editor.split | command-catalog.ts:96 | done | `shell_keymap.rs:209` `SplitTab{Right}` | 없음 | - |
| tab.cycleNext / tab.cyclePrev | command-catalog.ts:103,110 | done | `shell_keymap.rs:214-219` | Ctrl+Tab 이 egui 포커스 이동과 충돌하는지는 실행 확인 못함 | - |
| editor.save | command-catalog.ts:117 | done | `shell_keymap.rs:200` `RequestSaveTab`(File/AppFile/Untitled) | 없음 | - |
| view.toggleTerminal | command-catalog.ts:124 | done | `shell_keymap.rs:154-171` | 없음 | - |
| terminal.copyImeDebug | command-catalog.ts:131 (IME 디버그 플래그 한정) | n/a | WKWebView IME 진단 전용 | 없음. 카탈로그 행만 정리 필요 | S |
| app.showPerfSnapshot | command-catalog.ts:140 (`TAIDE_PERF` 한정) | n/a | TS perf-mark 계측 전용 | 없음. 카탈로그 행만 정리 필요 | S |
| tab.moveToNewWindow | command-catalog.ts:159 | unwired | 런타임 `layout_actions::layout_move_tab_to_window`(layout_actions.rs:156) 존재. native 호출부 0건(검색: `move_tab_to_window`, `MoveToWindow`, `move_focused_tab`) | `ShellMutation`/탭 메뉴/키 경로 | M |
| tab.moveToMainWindow | command-catalog.ts:165 | unwired | 상동. 보조 창(`WindowScope::Auxiliary`)은 존재 | 상동 | M |
| view.toggleZenMode | command-catalog.ts:180 | done | `shell_keymap.rs:90` `SetWindowChrome{zen}` | 보조 창 비활성 규칙은 미확인 | - |
| tab.previousEditor / tab.nextEditor | command-catalog.ts:193,200 | done | `shell_keymap.rs:214-219` | 없음 | - |
| editor.focusGroupLeft/Right/Up/Down | command-catalog.ts:207-234 | done | `shell_keymap.rs:122-142` `adjacent` + `FocusPane` | 없음 | - |
| tab.moveToGroupLeft / Right | command-catalog.ts:235-248 | done | `shell_keymap.rs:180-194` `MoveTab` | 없음 | - |
| tab.closeAllInGroup | command-catalog.ts:249 | done | `shell_keymap.rs:172` `RequestCloseTabs`(pinned 제외) | 없음 | - |
| editor.focusGroup1..9 | command-catalog.ts:263-325 | done | `shell_keymap.rs:129-141` leaf 위치 이동 | 없음 | - |
| sync.uploadNow | sync.commands.ts:41 | unwired | 런타임 `sync_actions::sync_upload`(sync_actions.rs:186) 존재, 호출은 `remote-sync.rs:49-55` 뿐. 네이티브 UI 호출부 0건 | 명령 경로, 성공/실패 토스트 | M |
| sync.downloadNow | sync.commands.ts:42 | unwired | `sync_actions::sync_download`(228), `remote-sync.rs:66-80` 뿐 | 충돌 토스트의 "Pull Remote" 액션(토스트에 액션 없음) | M |
| cli.connectExternalEditor | agent.commands.ts:58 | unwired | `agent_host::cli_install_status`(agent_host.rs:100) 는 `remote-agents.rs` 에서만 사용. 설치/제거 실행 함수는 native 에서 미확인 | 설치·상태 확인·토스트 | M |
| cli.installShellCommand | agent.commands.ts:63 | unwired | 상동 | 상동 | M |
| cli.uninstallShellCommand | agent.commands.ts:64 | unwired | 상동 | 상동 | M |
| ai.inlineEdit | ai.commands.ts:7 | missing | 런타임 `ai_actions` 는 `remote-ai.rs` 에서만 사용. 에디터 인라인 편집 오버레이 없음 | 오버레이 UI 전체 | XL(에디터/AI 영역) |
| git.revertHead | git.commands.ts:32 | unwired | `git_actions::git_revert_commit`(git_actions.rs:617) 존재, native 호출 0건 | 명령 경로·결과 토스트 | S |
| git.createTagOnHead | git.commands.ts:33 | unwired | `git_tag_create`(644) 존재, 태그 다이얼로그 UI 없음 | 다이얼로그 | M |
| git.toggleBlame | git.commands.ts:35 | missing | `git_blame_range`(225)는 있으나 거터/오버레이 없음 | 에디터 blame 표시 | L |
| git.openFileHistory | git.commands.ts:41 | missing | 파일 히스토리 패널 없음 | 패널 | L |
| terminal.runSelectedText | terminal.commands.ts:7 | missing | 검색어: `run_selected`, `run_in_terminal`, `runSelected` 0건 | 선택 텍스트 터미널 전송 | S |
| task.runTask | task.commands.ts:9 | unwired | 런타임 `task_actions.rs` 존재. 태스크 러너 다이얼로그 없음(검색: `TaskRunner`, `task_runner`) | 다이얼로그 | M |
| monaco.* 카탈로그 메타(제목·카테고리·기본 키 라벨) | monaco-action-commands.ts:36-45, monaco-actions.ts | done | `keybinding-commands.json` 157행, `keybinding-catalog.rs:235-319` `rows`, 파리티 픽스처 `tests/fixtures/keybinding-catalog.json` | monaco 158 대비 1건 차이 확정 못함 | - |
| monaco.* 액션 실행(약 157종: 줄 이동/복제/주석/다중 커서/접기/찾기/리팩터 등) | monaco-action-commands.ts:41 | missing | 에디터가 처리하는 키는 select all/undo/redo/방향키/Home/End/Backspace/Delete/Enter/Tab/Escape 뿐(`editor_surface.rs:573-631`). `taide-native-editor/src` 에 액션 enum 없음 | 사실상 전부 | XL |

### 2.2 커맨드 팔레트 UI

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| 팔레트 다이얼로그 셸(⌘P 빈 질의/⌘⇧P `>` 프리필/⌘T `#`, 열릴 때 캐럿 말미, Esc·바깥 클릭 닫기, 닫을 때 질의 초기화, 창 스코프 projectId) | command-palette.tsx:88-462, shared/ui/command.tsx | missing | 검색어: `palette`, `CommandPalette`, `command_palette`, `"palette.title"`. `egui::Modal` 은 close/delete/theme/keybinding 편집기에만 사용 | 전체. 메인 포커스 슬롯/보조 창 프로젝트 스코프 포함 | L |
| 모드 접두어 파싱 `>` `@` `:` `#` | command-palette-query.ts:3-57 | missing | 동명 심볼 0건 | 파서, 줄:열 파서(`^(\d+)(?::(\d+))?$`) | S |
| 파일 퀵오픈(상대경로 매칭, 200건 상한, 파일명+디렉터리 2단 표시, 갱신 중 스피너, 열기 선검증) | command-palette.tsx:305,443; files-group.tsx; docs/features/command-palette.md §3 | missing | 백엔드는 `search_actions::search_list_files`(search_actions.rs:182)와 `taide-search::list_project_files`(service.rs:641). native 호출부는 `remote-search.rs:109` 뿐 | UI·인덱스 캐시·무효화 규약·`splitFileMatchForDisplay` | L |
| 명령 모드(레지스트리 전체 fuzzy, `카테고리: 제목` 라벨, 키 배지, 비활성 행) | commands-group.tsx, command-registry.ts:116 | missing | 레지스트리 자체가 없음(2.1 총평). 라벨 조합 `keybinding-search.rs:90-117 label` 만 존재 | 레지스트리, 실행, `closeAfterAction` 규칙(`file.quickOpen` 예외) | M |
| 줄 이동 `:` | line-group.tsx, command-palette-query.ts:35 | missing | 이동 수단 `editor_reveal.rs`(터미널 파일 링크용 line/column reveal) 는 존재, 팔레트 소비자 없음 | UI | S |
| 심볼 이동 `@` (LSP documentSymbol, 계층 breadcrumb) | symbol-group.tsx, use-document-symbol-loader.ts | missing | `taide-lsp/src/native/{feature,registration,capabilities}.rs` 에 capability 등록만. documentSymbol 요청 소비자 없음 | 요청·세션 대기·평탄화·UI | L |
| 워크스페이스 심볼 `#` (200ms 디바운스, 취소) | workspace-symbol-group.tsx, use-workspace-symbol-search.ts | missing | 상동 | 요청·디바운스·UI | L |
| fuzzy 엔진(공백 8토큰, NFC, 점수·정렬 4단, 강조 인덱스) | fuzzy-match.ts:36-167 | partial | `keybinding-search.rs:63-224` `Search::filter`/`fuzzy_match`/`normalize`. 연결은 `keybinding-editor.rs:7,585,599` 뿐 | 라벨 문자열 슬라이스 전용(`Row` 결합), 하이라이트 세그먼트·파일명/디렉터리 분할·generic 후보 타입 없음 | M |
| cmdk `command-score` | cmdk 내부(shared/ui/command.tsx 경유) | done | `command-score.rs:11-86`, 사용처 `settings-code-view.rs:903`(콤보 검색) | 팔레트에서는 TS 도 `shouldFilter={false}` 라 불필요 | - |
| 선택 행 UI(배경+링 2중 단서, ↑↓ Enter 실행, 방향키 순환, 빈 상태 메시지 분기, 글자색+굵기 강조) | shared/ui/command.tsx:107, docs §4 | missing | 리스트 위젯 없음. 가장 가까운 것은 설정 콤보 `settings-code-view.rs:800-930` | 공용 리스트 컴포넌트 | M |
| 최근 항목(MRU) | docs §3 "MRU 는 미구현" | n/a | TS 도 미구현 | 없음 | - |
| 성능 마크 지표 4 | command-palette.tsx:171,411 | n/a | TS perf-mark 전용 | 없음 | - |

### 2.3 키맵

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| 기본 키맵 41 엔트리 | keymap.ts:97-243 | done | `keymap-defaults.json` 41개, id·키·mods·when·chord 일치 | JSON 을 수동 동기화한 이중 정본(TS 제거 시 해소) | - |
| 엔진: chord 5초 타임아웃, `when`(terminalFocus/editorTextFocus 및 부정), IME composing 무시, repeat/수식키 무시, ⌘K 에디터 유예, override 파싱, 보조 창별 상태 | keymap-dispatch/chord-store/context/when | done | `keymap.rs:164-689` `Keymap::decide`, `Windows::route`(209-311), 창 blur 해제 `application.rs:3621,4268` | 없음 | - |
| 에디터 포커스 중 그룹 chord 미러(monaco 미러) | keymap.md §5.2 | done | `keymap.rs:625-688` `decide_editor`(EDITOR_GROUPS 7개) | 없음 | - |
| 디스패치 대상 31개(shell_keymap `ACTIONS`) + 터미널 점프 2개 | useGlobalKeymap 소비자들 | done | `shell_keymap.rs:8-40`, 터미널 점프 `terminal_surface.rs:4122-4129` | 없음 | - |
| quick-open / command-palette / workspace-symbol | command-palette.tsx:235-241 | missing | `ACTIONS` 에 없음 → `application_keymap_decision`(terminal_surface.rs:1192-1219)이 `false` 반환, 입력이 위젯으로 흘러감 | 팔레트 | (2.2 참고) |
| find / search / search-replace | 에디터·검색 패널 | missing | 상동 | 위젯 | (2.1 참고) |
| explorer / git | explorer-panel-bridge | missing | 상동 | 뷰 전환 | (2.1 참고) |
| 사용자 override 의 비 base id 적용(명령 전용 행 `runsViaCommand`, `monaco.*` 행) | command-palette.tsx:268-283 `decideCommandBindingRun` | missing | `keymap.rs:438-450` 은 base 엔트리 id 와만 매칭. 그 외 override 는 무시(monaco 는 chord prefix 관찰용으로만 `keymap.rs:451-461`) | 명령 레지스트리와 결합된 두 번째 디스패처 | M |
| 키 라벨 포맷(⌘⇧ / Ctrl+) | formatKeymapShortcut | done | `keybinding-catalog.rs:70-108 stage_label` | 없음 | - |
| chord 대기/불일치 상태 표시 | keymap-chord-store | done | `keymap.rs:317-368 chord_status`, `status-chord.rs`, `application.rs:3442` | 없음 | - |
| 키바인딩 편집기·캡처·충돌 판정(소비자) | widgets/keybindings-editor | done | `keybinding-editor.rs`, `keybinding-capture.rs`, `keybinding-catalog.rs:388-` ConflictIndex, `application.rs:4164`. 상세 판정은 설정 영역 | 설정 영역 감사로 이관 | - |

### 2.4 shared/ui 프리미티브·훅

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| alert-dialog | shared/ui/alert-dialog.tsx | partial | 전용 구현만: `close_dialog.rs`, `delete_dialog.rs`(`egui::Modal`) | 범용 컴포넌트(제목/설명/액션/취소 변형), 포커스 트랩 일관성 | M |
| button (variant 6종 × size 8종) | button.tsx:7-35 | partial | 화면별 `ui.button`/`egui::Button::new`(shell.rs:294,351,869,875) | variant/size 체계, 테마 토큰(primary/destructive/secondary…) 매핑, focus ring | M |
| card | card.tsx | partial | 모듈 없음, 화면별 `egui::Frame` | 범용 | S |
| checkbox | checkbox.tsx | partial | `settings-view.rs` 의 `ui.checkbox` 사용 | 테마 스타일, 탐색 목록 행 체크박스 | S |
| command (cmdk 리스트) | command.tsx | missing | 2.2 선택 행 UI 참고 | 입력+그룹+아이템+빈 상태 컴포넌트 | M |
| context-menu | context-menu.tsx | partial | 탐색기 `explorer.rs`, 터미널 `terminal_surface.rs:1804` 등 `context_menu` | 탭 우클릭 메뉴 없음(shell.rs 에 `tab.*` 메뉴 키 없음), 체크/라디오/서브메뉴 항목 | M |
| dialog | dialog.tsx | partial | `egui::Modal`: theme-editor, keybinding-editor, system-usage-view, zen | 범용 컴포넌트, ESC/바깥클릭 규약이 화면별로 따로 구현(delete_dialog.rs:77, snippet-editor.rs:1562, theme-editor.rs:623) | M |
| dropdown-menu | dropdown-menu.tsx | partial | 탭바 "+" 메뉴(shell.rs:712-748), 설정 `ComboBox`(settings-view.rs, settings-code-view.rs) | 범용, 체크·서브메뉴 | M |
| popover | popover.tsx | partial | `egui::Popup` 개별 사용(snippet-editor.rs, keybinding-editor.rs, shell.rs 등) | 범용 | S |
| progress | progress.tsx (lsp-server-status-list.tsx 사용) | missing | 검색어: `ProgressBar`, `progress` 0건(`lsp-status.rs` 는 토큰 카운트만) | 프로그레스 바 | S |
| scroll-area / overlay-scrollbar / scroll-container / use-overlay-scrollbar | scroll/*, hooks/use-overlay-scrollbar.ts (1초 자동 숨김, 트랙 클릭, 썸 드래그) | partial | 전부 egui `ScrollArea` 기본값. 검색어: `ScrollStyle`, `floating`, `ScrollBarVisibility` 0건 | 오버레이 스크롤바 스타일·자동 숨김·thumb 색(테마 토큰) | M |
| separator | separator.tsx | done | `ui.separator`/`painter().hline|vline` 광범위 사용 | 없음 | - |
| switch | switch.tsx | partial | 설정 전용 `settings-view.rs:1746 fn switch` | 범용·테마 토큰 | S |
| tooltip (400ms 지연, 화살표, 위치/충돌 처리, 키보드 닫기, 모달 규칙) | tooltip.tsx, app-providers.tsx:7 | partial | `tooltips.rs`(DELAY 0.4s, 화살표·모션·그레이스 영역), 앱 연결 `application.rs:3748,4266,4279` | 일부 위치가 egui 기본 `on_hover_text` 사용(shell.rs:399,739,938,959)으로 TS 툴팁과 외형/지연이 다름 | S |
| icon-button (비활성에도 툴팁) | icon-button.tsx | partial | `tooltip_trigger::wrap_button`(tooltip-trigger.rs:10-52)로 비활성 포커스+툴팁 이식. 호출부는 일부 화면뿐(테마/스니펫/설정/PDF/HWP) | 탐색기 툴바(explorer_toolbar.rs)·탭 버튼 등 일관 적용, 범용 위젯 | M |
| error-boundary (구역 단위 크래시 격리, 재시도, 포커스 이동) | error-boundary.tsx | missing | 검색어: `catch_unwind`, `panic::set_hook`, `ErrorBoundary` 0건 | 구역 패닉 격리·재시도 UI | M |
| status-error-banner (상단 고정 배너+재시도) | status-error-banner.tsx | missing | 동명 심볼 0건. 유사물 `conflict_banner.rs` 는 파일 충돌용 | 범용 배너 | S |
| file-group-header (체크박스+쉐브론+파일 아이콘+개수) | file-group-header.tsx | missing | 동명 심볼 0건 | 검색/Git 패널 의존 컴포넌트 | S |
| toast(sonner): success/error/warning/info, description, action 버튼 | toast 사용처 다수 (sync.commands.ts:28-31) | partial | `toast.rs` Kind 는 Warning/Error/Success 3종(64-70), description 있음(163), swipe·모션·위치 이식 | Info 종류, action 버튼(conflict "Pull Remote"), loading | M |
| 테마 토큰 → 전역 스타일(CSS 변수 `:root` 150줄, `@theme`) | global.css:6-297 | missing | `presentation.rs:91-103 shell_colors` 9색만. `set_visuals`/`set_style`/`Visuals::` 검색 0건 | egui `Visuals`/`Style` 전역 매핑(위젯 fill·stroke·selection·hyperlink·popup·메뉴·텍스트 입력) | L |
| global.css 보조(reveal 게이트, modal-scrim, shadow-overlay, gutter/blame 클래스, xterm 보정) | global.css:321-467 | n/a | 웹 렌더링 전용. 모달 스크림은 `egui::Modal` 이 대체. gutter/blame 은 에디터 영역 | 없음 | - |
| use-global-keymap / use-keydown-capture | hooks | done | `keymap::Windows::route`(터미널·에디터 양쪽 라우팅) | 없음 | - |
| use-ipc-error-message | hooks/use-ipc-error-message.ts | done | `toast.rs:1384 describe_error`(Localized 키 → 폴백 문자열) | 없음 | - |
| use-reveal-window | hooks/use-reveal-window.ts | n/a | 프레젠테이션을 `NativeApplication::new`(application.rs:238-248)에서 동기 로드 후 첫 프레임 | 없음 | - |
| use-tauri-event / use-object-url / use-monaco-markers / use-remote-connection-revision | hooks/* | n/a | Tauri·Blob URL·Monaco·원격 웹 전용 | 없음 | - |

### 2.5 i18n

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| 카탈로그와 지원 언어 | i18n.ts(리소스는 Rust 가 공급, `applyLocaleMessages`) | done | `crates/taide-locale/resources/locales/{en,ko,ja}.json` 각 1,108줄, 동일 키 | 없음 | - |
| native 가 카탈로그를 사용 | - | done | `presentation.rs:136 message`, `NativeApplication.locale: ResolvedLocale`(application.rs:157, 248). 하드코딩 라벨 검색에서 비테스트 UI 리터럴은 `shell.rs:866` 브랜드 "TAIDE" 1건 | 없음 | - |
| base 언어 병합·사용자 locale pack | LocalePack | done | `taide-locale/service.rs:1356 resolve_pack`(누락 키는 base 로 채움) | 설정 UI 의 pack 가져오기는 설정 영역 | - |
| 언어 선택·즉시 반영 | settings language | done | `settings-view.rs:1340-1466`, `application.rs:609` presentation refresh | 없음 | - |
| 사용자에게 보이는 비국제화 문자열 | - | partial | `application.rs:4918` "native tab surface is not connected", `application.rs:3880-3892` 등 `AppError::Forbidden("native ... host is disconnected or full")` 가 `toast.rs:1398-1402` 로 원문 출력. `main.rs:8` 윈도 제목 "TAIDE Native" | 키 승격(Localized 에러) | S |
| 복수형/보간 | i18next | done | `{{name}}` 단순 치환만 지원하나 카탈로그에 `_one/_other` 키가 0건이라 영향 없음 | 없음 | - |

### 2.6 아이콘

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|
| 앱 전반 lucide 아이콘(소스 약 91개 파일, 고유 약 100종) | `from 'lucide-react'` | partial | 공용 아이콘 모듈 없음. SVG 약 48개가 용도별로 분산: keybindings 5, themes 3, snippets 3, toasts 3, problems 29, status 4, 그리고 탐색기 툴바 4개와 탭 "+" 는 `painter` 수작업 도형(explorer_toolbar.rs:41-90, shell.rs:925-959). `icons.rs` 의 `Icon` enum 은 12종 | 공통 아이콘 레지스트리, 렌더 캐시 일원화, 나머지 약 50종 | L |
| 파일 타입 아이콘 규칙표(`resolveFileIcon`: 특수 파일명/README/LICENSE/.env/확장자 24종 + 색 토큰) | file-icon.ts:80-111, file-icon-registry.ts | partial | 규칙표 이식은 `problems-icons.rs:198-253 file()` 가 유일, 연결은 Problems 패널(`problems.rs:139`)뿐 | 탐색기 트리 행에는 아이콘이 없음(`explorer.rs:912-917` 이름 라벨만), 탭 제목·퀵오픈·검색 결과·FileGroupHeader 에도 없음 | M |
| 폴더 아이콘(`resolveFolderIcon`: folder/folder-open/folder-code/box/flask-conical/git-fork 및 특수 폴더명 11종) | file-icon.ts:96-128 | missing | 검색어: `folder_icon`, `FolderTypeIcon`, `resolve_folder`. 폴더용 Glyph 없음 | 폴더 규칙표+SVG 6종 | S |
| 프로젝트 아이콘 카탈로그(67종 + 선택 그리드) | project-icon-registry.ts:74- | missing | 검색어: `project_icon`, `PROJECT_ICON`. `taide-model` 에는 `icon: Option<String>` 필드 있음(project.rs:37). 프로젝트 레일은 이름 버튼(shell.rs:380-400) | 아이콘 렌더, 선택 UI(프로젝트 영역 감사와 연계) | M |
| 테마 색 연동(`colorClass` → status/git 토큰) | file-icon.ts:34-44 | partial | `problems-icons.rs:FileColor` 가 테마 키로 매핑 | 전 영역 공용화 | S |

### 2.7 집계

총 102행 기준(2.1 42 + 2.2 12 + 2.3 12 + 2.4 25 + 2.5 6 + 2.6 5): done 33, partial 20, unwired 9, missing 32, n/a 8.

## 3. 잘못 구현됐거나 보강이 필요한 native 코드

1. 실행할 수 없는 명령이 편집 가능한 키바인딩 행으로 노출됩니다. `keybinding-catalog.rs:235-319 rows` 는 `keybinding-commands.json` 210종을 모두 행으로 만들지만, 실행 주체가 없어 사용자가 바인딩해도 아무 일도 일어나지 않습니다(`shell_keymap.rs:8-40` 31개만 실행, `keymap.rs:438-450` 이 base id 외 override 를 무시). `window.reload`·`terminal.copyImeDebug`·`app.showPerfSnapshot` 같은 웹 전용 행도 그대로 남아 있습니다. 명령 레지스트리와 디스패처가 생기기 전까지는 사용자를 속이는 UI 입니다.
2. 전역 `egui::Visuals`/`Style` 이 테마와 연결되지 않습니다. `presentation.rs:91-103` 의 9색만 쉘에 쓰이고 버튼·텍스트 입력·팝업·메뉴·선택색은 egui 기본값입니다(`set_visuals`/`set_style` 호출 0건). TS 의 `global.css` 토큰 체계와 시각 동일성을 확보할 수 없습니다.
3. 공용 프리미티브 부재로 같은 규약이 화면마다 중복 구현돼 있습니다. 예: ESC 닫기(`delete_dialog.rs:77`, `snippet-editor.rs:1562`, `theme-editor.rs:623`), 포커스 트랩, 모달 구성. 새 화면(팔레트 등)을 추가할 때마다 같은 코드를 다시 쓰게 됩니다.
4. 툴팁 일관성이 깨져 있습니다. 앱 레벨 `tooltips::Provider` 는 이식됐지만 `shell.rs:399,739,938,959` 가 egui 기본 `on_hover_text` 를 써서 TS 와 지연·외형이 다릅니다.
5. 알림 채널이 TS 와 다릅니다. TS 의 `toast.info(app.openProjectFirst)` 가 native 에서는 매 프레임 `take()` 되는 단일 슬롯 문자열 `self.status`(application.rs:3697,4157-4161,4175-4181)로 처리됩니다. 토스트에 Info 종류와 액션 버튼이 없어(`toast.rs:64-70`) 동기화 충돌 토스트("Pull Remote")를 표현할 수 없습니다.
6. 사용자에게 보이는 원문 영어 오류가 있습니다(`application.rs:4918`, `application.rs:3880-3892` 등 `AppError::Forbidden/Internal` 원문 → `toast.rs:1398-1402`).
7. 테스트가 배포 코드와 다른 컴파일 단위를 검증합니다. `native-app/src/keymap.rs:15` 와 `tooltips.rs` 는 `#[cfg(test)]` 에서 `include!` 로 ui 소스를 복제 컴파일합니다. 실제 앱은 `taide_native_ui::keymap` 을 쓰므로 타입이 둘로 갈라지고, 한쪽만 수정해도 테스트가 통과할 수 있습니다.
8. 키맵 정본이 이중입니다. `keymap-defaults.json`, `keybinding-commands.json` 은 TS `APP_KEYMAP`/카탈로그를 수동 동기화한 사본이며 TS 제거 시점까지 드리프트 위험이 있습니다(파리티 픽스처 `tests/fixtures/keybinding-catalog.json` 으로 부분 방어).
9. TS 에 없는 단축키 ⌘N 이 카탈로그 밖에 하드코딩돼 있습니다(`application.rs:3949-3959`). 재바인딩·충돌 판정 대상이 아니고 `keybinding-commands.json` 에도 없습니다.
10. fuzzy 엔진이 둘로 분리돼 있습니다. `command-score.rs`(cmdk 포트, 호출 1곳 `settings-code-view.rs:903`)와 `keybinding-search.rs`(TS fuzzy-match 포트, `Row`/`ResolvedLocale` 결합)입니다. 팔레트에는 후자를 generic 후보 타입으로 승격해야 하는데 현재는 키바인딩 행 전용입니다.

## 4. 실제 앱 연결이 끊긴 지점

1. 키맵 id 8개: quick-open, command-palette, workspace-symbol, find, search, search-replace, explorer, git. `keymap.rs` 가 `Dispatch` 를 내지만 `shell_keymap::supports`(shell_keymap.rs:82)가 거부해 `terminal_surface.rs:1192-1219` 에서 미처리로 반환됩니다.
2. 런타임 액션은 있으나 native UI 호출부가 없는 것: `layout_move_tab_to_window`, `sync_upload/sync_download`, `agent_host::cli_install_status`(설치/제거 포함), `git_revert_commit`, `git_tag_create`, `git_blame_range`, `task_actions`, `search_actions::search_list_files`, `ai_actions`. 이 중 sync/search/git/ai/agent 는 `remote-*.rs`(브라우저 원격 라우터)에만 연결돼 있습니다.
3. `editor_reveal.rs`(line:column reveal)는 터미널 파일 링크 전용이며 줄 이동 UI 에 연결돼 있지 않습니다.
4. LSP documentSymbol/workspaceSymbol 은 `taide-lsp/src/native/*` 에서 capability 등록까지만 있고 요청·소비 경로가 없습니다.
5. `keybinding_search::Search::filter` 는 키바인딩 편집기에만 연결돼 있습니다(`keybinding-editor.rs:7,585,599`).
6. `problems_icons::file()`(파일 아이콘 규칙)은 Problems 패널에만 연결돼 있고 탐색기·탭·퀵오픈에는 연결돼 있지 않습니다.
7. `keybinding-commands.json` 의 `runsViaCommand` 행(`keybinding-catalog.rs:272`, 475)은 편집기 표시용 플래그일 뿐 디스패치에 쓰이지 않습니다.

## 5. 권장 구현 순서 (의존 관계 포함)

1. 공용 UI 기반 먼저: 테마 토큰 → egui `Visuals`/`Style` 매핑(3장 2번), 공용 Dialog/ListItem/Button/Switch/Popover/Progress 프리미티브를 `taide-native-ui` 에 모듈화합니다. 팔레트·검색 패널·다이얼로그가 모두 여기에 의존합니다.
2. 공용 아이콘 레지스트리(SVG 번들 + 텍스처 캐시 + 테마 색 틴트)와 `resolveFileIcon`/`resolveFolderIcon` 이식을 같은 단계에서 진행합니다. 팔레트 파일 행, 탐색기, 검색 결과가 사용합니다.
3. 명령 레지스트리(`AppCommand` 대응: id, 제목 키, 카테고리 키, `isEnabled`, `run`, `CommandContext`)를 `taide-native-ui` 에 만들고 `ShellIntent`/`HostCommand` 로 실행을 연결합니다. 이후 `Keymap::update` 의 비 base id override 를 레지스트리로 디스패치합니다(`runsViaCommand` 와 `monaco.*`). 실행 불가 명령은 카탈로그에서 숨기거나 비활성 표시합니다.
4. fuzzy 엔진 generic 화(`keybinding-search.rs` → 공용 모듈, 하이라이트 세그먼트와 파일명/디렉터리 분할 포함). 이어서 팔레트 셸과 files/commands/line 모드, 그리고 ⌘P/⌘⇧P 키 연결(`ACTIONS` 에 추가)을 구현합니다. 의존: 1~3.
5. 심볼 모드(`@`, `#`): 먼저 native LSP 클라이언트에 documentSymbol/workspaceSymbol 요청 경로를 만든 뒤 팔레트에 연결합니다. 의존: 4 와 LSP 영역.
6. 이미 런타임이 있는 명령부터 연결합니다: tab.moveToNewWindow/MainWindow, sync.*, git.revertHead, cli.*(macOS). 각각 토스트(Info/액션 포함 확장 필요, 토스트 확장은 1단계에 포함)로 결과를 알립니다.
7. 검색 패널·에디터 찾기·Git 뷰·태스크 러너·태그 다이얼로그는 각 영역 작업이 끝나는 대로 해당 키맵 id/명령을 `ACTIONS` 와 레지스트리에 추가합니다. 이 영역의 책임은 "진입 키/명령 연결"이며 화면 자체는 각 영역 보고서를 따릅니다.
8. monaco 계열 액션 실행(약 157종)은 에디터 영역의 편집 명령 구현 범위에 따라 단계적으로 레지스트리에 등록합니다. 에디터 영역이 구현한 것만 활성화하고 나머지는 카탈로그에서 숨깁니다.
9. 후순위: 오버레이 스크롤바, ErrorBoundary(패닉 격리), StatusErrorBanner, FileGroupHeader, 프로젝트 아이콘 카탈로그.

## 6. 확인하지 못한 것 (불확실성)

- 빌드·실행·테스트를 하지 않았습니다. Ctrl+Tab(tab-cycle)이 egui 포커스 이동에 소비되는지, ⌘1..9 와 `Event::Text` 중복 소비(`keymap.rs:241-247 claimed_text`)가 실기기에서 동작하는지는 코드 읽기로만 판단했습니다.
- monaco 액션 수: TS `MONACO_ACTIONS` 는 grep 으로 158(`actionId:` 계열)으로 세어졌고 native JSON 은 157입니다. 1건 차이가 TS 쪽 집계 오차인지 누락인지 id 단위 대조(파이프 금지 제약으로 수행하지 못함)로 확정하지 못했습니다. 파리티 픽스처 테스트가 이를 방어하는지도 실행해 보지 않았습니다.
- `agent_host` 의 CLI 설치/제거 실행 함수가 native 에 있는지는 `cli_install_status` 만 확인했습니다. 나머지는 미확인이라 unwired 로 판정했습니다.
- native 설정 화면에 sync 업로드/다운로드 UI 가 있는지는 설정 영역 감사 소관이라 확인하지 않았습니다(`settings-view.rs` 에서 "sync" 문자열 1건만 확인).
- 탭 우클릭 메뉴는 쉘 영역 소관이라 `shell.rs` 에서 `tab.*` 메뉴 키가 없다는 사실만 기록했고, 별도 파일에 있을 가능성은 배제하지 못했습니다.
- 팔레트의 접근성(스크린리더, IME 중 Escape, 메인/보조 창 포커스 트랩)은 TS 쪽도 미검증 항목(inventory 문서)이라 기준선이 없습니다.
- 시도한 검색어(missing 판정 근거): `palette`, `CommandPalette`, `command_palette`, `quick_open`, `QuickOpen`, `file.quickOpen`, `"palette.*"` 키, `CommandRegistry`/`command_registry`/`AppCommand`/`run_command`/`execute_command`, `go_to_line`/`goto_line`/`jump_to_line`, `document_symbol`/`workspace_symbol`/`documentSymbol`, `find_bar`/`FindState`/`search_panel`/`SearchPanel`, `GitPanel`/`source_control`/`SidebarView`, `run_selected`/`run_in_terminal`, `ProgressBar`, `catch_unwind`/`ErrorBoundary`/`set_hook`, `ScrollStyle`/`floating`, `folder_icon`/`FolderTypeIcon`, `project_icon`/`PROJECT_ICON`, `lucide`, `set_visuals`/`set_style`/`Visuals::`.
