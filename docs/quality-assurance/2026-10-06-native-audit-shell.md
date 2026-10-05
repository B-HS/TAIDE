# Native 전환 감사: 앱 셸·창·레이아웃·탭·프로젝트

감사 일자: 2026-10-06. 읽기 전용 감사이며 빌드·테스트·실행은 하지 않았습니다. 이전 에이전트의 HANDOFF.md·PROCESS.md 수치는 근거로 쓰지 않고 실제 파일과 호출 체인으로만 판정했습니다.

## 1. 범위

읽은 TS 경로 (비테스트):
- src/app: app.tsx, main-window-dialogs.tsx, providers/ 전체(agent-external-open, agent-state-sync, app-providers, emmet, external-link, hot-exit-flush, ide-sync, ipc-sync(헤더·역할), keybindings-runtime, locale, native-notification, shell-slot, theme)
- src/widgets: app-shell(app-shell, project-shell, shell-slot-tree-view, use-window-chrome, use-project-drag), app-sidebar(3종), auxiliary-window-shell(2종), window-chrome(2종), welcome(container), editor-area(editor-area, pane-node-view, pane-tab-bar, use-request-close-tab)
- src/features: shell-slot, split, tab(8종), project(7종), window(5종), welcome
- inventory 문서: 2026-09-28-ts-view-inventory, 2026-09-28-ts-overlay-inventory, 2026-09-29-ts-provider-inventory, 2026-09-29-ts-shared-widget-inventory, 2026-09-29-ts-feature-inventory-c, 2026-09-29-m8-entry-and-cutover
- Tauri 시절 창 구성 참조: src-tauri/tauri.conf.json(창 크기·Overlay 타이틀바), src-tauri/src/domain/window/menu.rs, src-tauri/src/lib.rs(menu dispatch, window-state 플러그인)

읽은 native 경로:
- native/taide-native-app/src: main.rs, bootstrap.rs, lib.rs, application.rs(생성자 1-650, 닫기·탭 닫기 1584-1960, background_tick 3360-3600, eframe::App 3599-4405, 종료 4376-4515, AppSurfaces 필드·ShellSurfaces 구현 4516-5214), tabs.rs, close_dialog.rs, zen.rs, shell_keymap.rs(액션 표 + 로직), events.rs, event-relay.rs, application-ports.rs, status-chord.rs, status-editor.rs(show_font), status-ide.rs·lsp-status.rs(키 확인), projects.rs(앞부분), host.rs(HostCommand 열거)
- native/taide-native-ui/src: shell.rs 전체, commands.rs, snapshot.rs, controller.rs, split.rs, lib.rs, icons.rs(아이콘 목록)
- 전체 검색: native/, crates/, src-tauri/src (target·vendor 제외)

읽지 못한 부분은 7장에 적었습니다. native 디렉터리는 전부 미커밋(`git status`: `?? native/`)입니다.

## 2. 기능 대응표

표기: 상태는 done/partial/unwired/missing/n/a, effort는 native 남은 작업량(S 반나절 이하, M 1~2일, L 3~5일, XL 1주 초과). native 근거의 줄 번호는 현재 작업 트리 기준입니다.

### 2.1 앱 진입·창·종료

| # | 기능 | TS 근거 | 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|---|
| 1 | 메인 창 생성과 앱 진입 | src/app/app.tsx 91-120 | done | native-app/src/main.rs 36-49 (eframe::run_native 단일 창) → application.rs 197 NativeApplication::new | 없음. 단 `--data-dir` 필수(bootstrap.rs 27-31) | - |
| 2 | 창 크기·최소 크기·타이틀·배경색 | tauri.conf.json (1400x900, min 720x480, title "TAIDE", backgroundColor) | partial | main.rs 6-7, 32-35: 1280x800, 타이틀 "TAIDE Native" 고정, min size 없음, ViewportBuilder에 배경색 없음 | 1400x900, 최소 720x480, 타이틀 "TAIDE", 배경 #1e1e2e | S |
| 3 | 창 위치/크기 복원 (window-state 플러그인) | src-tauri/src/lib.rs 857 | missing | 검색어: window_state, window-state, with_position, outer_rect → native 내 구현 없음 | 종료 시 창 geometry 저장/복원, 풀스크린 복원 | M |
| 4 | 보조 창 분기 (getWindowContext aux → AuxiliaryWindowShell) | app.tsx 65-89 | unwired | ui/src/shell.rs 133-161 `WindowScope::Auxiliary` 분기는 있으나 application.rs 253-256이 항상 `WindowScope::Main`; Auxiliary 생성처는 application.rs 4138(OpenSettings 소유자 계산)·테스트뿐 | 다중 viewport 생성, 창별 ViewKey.window 라벨(현재 상수 WINDOW_LABEL="native-main" application.rs 31), 창별 상태 | XL |
| 5 | 보조 창 셸 (고정 project/windowSlot, explorer+editor만, 마지막 탭 이동/닫기 시 창 닫기, layout 오류 시 닫기) | auxiliary-window-shell.tsx 73-146 | unwired | shell.rs 133-161이 title_bar+pane_tree만 그림(explorer 없음, 창 닫기 로직 없음) | explorer 패널, auto-close 규칙, 보조 창 키맵 범위 | L |
| 6 | 보조 창 타이틀바 (탭 — 프로젝트 — branch) | auxiliary-title-bar-content.tsx | unwired | shell.rs 140-147 | branch(표면 `branch()`가 None), 창 제목 동기화 | S |
| 7 | 탭을 새 창/메인 창/기존 창 N으로 이동 | pane-tab-bar.tsx 116-120, 230-233 | unwired | runtime에 `layout_actions::layout_move_tab_to_window`(crates/taide-runtime/src/layout_actions.rs 156) 존재하나 native 호출처 없음(검색어: layout_move_tab_to_window, TabWindowTarget, NewAuxiliary, move_tab_to_window → native-ui/app 소스 0건, remote-layout-tests.rs만 문자열) | ShellMutation 변종, 창 열기 closure(open_auxiliary_window), 메뉴 항목 | L |
| 8 | 보조 창 복원 | taide-window plan_auxiliary_window_restorations | missing | native bootstrap.rs 55-67은 restore_state만 호출, 창 복원 호출 없음 | 아래 4장 결함 1 참고(TS 시절 데이터의 보조 창 탭이 보이지 않음) | XL(4~7과 묶음) |
| 9 | 네이티브 앱 메뉴(File > Open Recent·Clear Recent·Quit, 앱/편집/창 표준 메뉴) | src-tauri/src/domain/window/menu.rs, lib.rs 398-420 | missing | 검색어: muda, NSMenu, menu_bar, MenuBar, tray, set_menu → native 소스 0건, Cargo.toml에 muda 없음 | 최근 목록 갱신·Clear Recent·로케일 라벨·Quit→종료 drain | M |
| 10 | 창 닫기·앱 종료 drain (hot-exit 미러·layout flush·서비스 정리) | hot-exit-flush-provider.tsx | done | application.rs 3563-3572(close_requested 가로채기)→1610 close()→4419 shutdown(drafts 미러, flush_layouts, drain_services); on_exit 4376 직접 종료 경로 | 없음(Cmd+Q 직접 경로 실기 미확인, 7장) | - |
| 11 | flush 범위 window/project 응답 | hot-exit-flush-provider.tsx 54-58 | missing | 범위가 필요한 기능(보조 창 닫기·프로젝트 닫기)이 native에 없어 해당 경로 없음. event-relay.rs 203이 `HotExitFlushRequested`를 무시 | 4·32 구현 시 필요 | M |
| 12 | 전역 ErrorBoundary(위젯별 fallback) | shared/ui/error-boundary.tsx, app-shell.tsx | n/a | Rust 패닉 정책이라 렌더 예외 복구 개념이 없음 | - | - |
| 13 | document contextmenu 억제(입력 제외) | app-shell.tsx 137-154 | n/a | 웹 기술 전용 | - | - |
| 14 | 프로젝트 전환 perf mark | app-shell.tsx 167-170 | n/a | 성능 계측 전용 | - | - |
| 15 | 기존 사용자 데이터 디렉터리 사용 | Tauri app data | partial | bootstrap.rs 27-31, 15 SECRET_SERVICE="...native-isolated": 격리 디렉터리만 허용 | 기본 데이터 경로 해석, 기존 앱과 양방향 호환 | M |

### 2.2 Zen·창 크롬·전역 입력

| # | 기능 | TS 근거 | 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|---|
| 16 | Zen 토글(⌘K Z·팔레트 브리지) | use-window-chrome.ts 41-46 | partial | shell_keymap.rs 83-95 + application.rs 4128-4240 | 명령 팔레트 경로(20번 의존) | S |
| 17 | Zen 동안 rail·탭바·slot 헤더·explorer 숨김, 다른 slot 숨김, 상태바 설정 | project-shell.tsx, shell-slot-tree-view.tsx 129-137 | done | shell.rs 179-181, 200, 413-433, 492-497, 545, 674 | 없음 | - |
| 18 | Zen Escape 해제(IME·defaultPrevented 가드) | use-window-chrome.ts 64-73 | done | zen.rs 24-55 escape(), application.rs 4308-4327 | 없음 | - |
| 19 | Zen 풀스크린 opt-in·첫 렌더 보호 | use-window-chrome.ts 93-97 | done | zen.rs 3-22 Fullscreen::reconcile (초기값 new(zen,enabled)) | 없음 | - |
| 20 | Zen 진입 안내 배너(3초) | zen-mode-hint.tsx | missing | 검색어: zen.hint, ZenModeHint, zen.hintExit → native 0건 | 배너 위젯 | S |
| 21 | 전역 키맵: toggle-sidebar/explorer/git | app-shell.tsx 143-147 | partial | shell_keymap.rs 31 액션 중 toggle-sidebar만(97-124); explorer/git 뷰 전환 액션 없음 | explorer 뷰 전환(타 영역) | S |
| 22 | OS 파일/폴더 drop(프로젝트 열기 또는 파일 preview 탭) + DragDropOverlay | app-shell.tsx 99-135, drag-drop-overlay.tsx | missing | 검색어: dropped_files, hovered_files, DroppedFile, drag_drop, on_file_drop → native 소스 0건(vendor/egui-input 정의만) | raw input dropped_files 처리, 폴더면 프로젝트 열기·아니면 파일 탭, overlay, 오류 toast | M |
| 23 | 포커스 slot 추적(포커스 프로젝트가 title/status/dialog 대상) | shell-slot-provider.tsx | partial | shell.rs 613-626 pointer press/위젯 조작 시 FocusSlot 의도 | 키보드 focusin 기반 slot 전환, 포털 열림 중 무시 규칙 | S |
| 24 | MainWindowDialogs: CommandPalette·TaskRunner(포커스 프로젝트 기준) | main-window-dialogs.tsx | missing | 검색어: palette, task_runner, TaskRunner, quick-open → native-app/ui에 dialog 구현 없음(command-score.rs는 설정 검색용, keymap-tests의 "quick-open" 문자열만) | 팔레트·task 실행 다이얼로그 전부(타 영역과 중복) | L |
| 25 | Hot-exit·앱 종료 후 세션 복원(프로젝트·slot 트리·레이아웃·zen) | ipc-sync-provider, session | done | bootstrap.rs 55-67 restore_state, projects.rs attach(레이아웃 로드·watcher 복구), application.rs 328 RestoreWatchers | 없음 | - |

### 2.3 Welcome

| # | 기능 | TS 근거 | 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|---|
| 26 | 프로젝트 0개 전체 Welcome (제목·힌트·열기 버튼) | welcome-screen.tsx 46-70 | partial | shell.rs 196-199, 853-904 | 열기 버튼은 있음. 카드·아이콘·스타일 차이 | S |
| 27 | 최근 프로젝트 목록(limit, rootMissing 비활성, 조회 실패 안내) | welcome-container.tsx 61-62, 76-83 | missing | 검색어: recent_projects, RecentProject, recentItems, app.recentItems, project_list_recent → native UI 0건(runtime project_actions::project_list_recent 존재, 호출처 remote만) | 목록 UI·활성/열기 분기·rootMissing 표시 | M |
| 28 | 단축키 카드(effective keymap 반영) | welcome-container.tsx 24-60 | missing | 검색어: keyboardShortcutsTitle, WELCOME_KEYMAP_HIGHLIGHT_IDS → 0건 | 카드와 effective keymap 계산 | S |
| 29 | 열기 버튼 동작(폴더/파일/터미널, root 밖 파일 거부) | welcome-container.tsx 64-74 | partial | application.rs 4194-4234(rfd 폴더/파일, 파일은 프로젝트 root에서 시작) | `app.openFileOutsideRoot` 토스트 검증, 탭 대상 pane | S |
| 30 | 빈 pane의 Welcome/“파일 없음” 분기(welcomeOnEmptyEditor) | pane-node-view.tsx 130, 267-274 | partial | shell.rs 759-765는 설정과 무관하게 항상 welcome. 설정 컨트롤(settings-controls.rs 137)만 존재 | 설정 반영(꺼짐이면 `editor.noFileOpen` 문구) | S |
| 31 | welcome 종류 탭 | pane-node-view.tsx 198-202, tab-bar-menu "openWelcome" | missing | application.rs 4916-4922: Welcome 포함 미지원 종류는 라벨 + status 문자열 | tab_content 분기 | S |

### 2.4 프로젝트 rail·그룹

| # | 기능 | TS 근거 | 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|---|
| 32 | 프로젝트 버튼 활성화(project_activate) | app-sidebar.tsx 141, project-icon-button.tsx | done | shell.rs 359-401 → ShellMutation::ActivateProject(commands.rs 80-82) | 없음 | - |
| 33 | 프로젝트 glyph(icon/label/color 표시, resolveProjectDisplay) | project-display-glyph.tsx | partial | shell.rs 367-371: label 첫 글자만 | 아이콘 레지스트리·색 token·mode 3종 | M |
| 34 | rootMissing 표시(dim + tooltip) | sortable-project-icon.tsx 114, 148-154 | partial | shell.rs 390-399 tooltip 문구만 | dim 표시, 경고 아이콘 | S |
| 35 | agent 상태 배지·세션 tooltip | agent-status-badge.tsx, app-sidebar.tsx 84-90 | missing | 검색어: agentStatusBadge, AgentStatusBadge, agent_status_badge, agent.sessionTooltip, projectAgents → 0건(rail 데이터에 agents 없음) | 배지, 우선순위 선택, tooltip, 설정 agentStatusBadgeEnabled | M |
| 36 | 프로젝트 context menu(닫기, Open to Right/Below/Left/Above, 그룹 추가/새 그룹/제거, 파일 관리자 열기, 경로 복사, 표시 편집/초기화) | sortable-project-icon.tsx 169-203 | missing | 검색어: project.close, shellSlot.openToTheRight, projectGroup.addTo, project.copyPath, project.displayMenu → 앱 소스 0건 | 메뉴 전부 | M |
| 37 | 프로젝트 닫기(project_close) | sortable-project-icon.tsx 170 | missing | runtime `project_actions::project_close`(taide-runtime project_actions.rs 458)는 remote-projects.rs 143과 projects-tests.rs에서만 호출. ShellMutation 변종 없음(commands.rs 10-47) | UI 진입, 닫을 때 flush(11번), 닫기 확인 | M |
| 38 | 프로젝트 표시 편집 dialog(mode·아이콘 검색·라벨·12색·미리보기) + 초기화 | project-display-dialog.tsx | missing | project_set_display는 remote-projects.rs 152만 | dialog 전체, 아이콘 레지스트리 | L |
| 39 | rail 프로젝트 drag 재정렬(project_reorder) | use-project-drag.ts 97-104 | missing | 검색어: dnd_drag_source, dnd_drop_zone, DragAndDrop(application.rs 4350 한 곳, web 배치 비활성 용) → shell에 DnD 없음. project_reorder는 remote-projects.rs 149만 | egui DnD 재정렬 + ShellMutation | M |
| 40 | rail `+` 메뉴(경로로 열기, Finder로 열기, 최근 프로젝트 N개) | sidebar-add-project-menu.tsx | partial | shell.rs 351-353 "Open Folder" 단일 버튼(= Finder로 열기) | 메뉴, 최근 목록(RECENT_PROJECT_MENU_LIMIT, 열린 것 제외, rootMissing 비활성) | S |
| 41 | 경로로 열기 dialog | open-project-by-path-dialog.tsx | missing | 검색어: openByPath, OpenProjectByPath, sidebar.openByPath → 0건 | dialog | S |
| 42 | rail 설정 버튼 | app-sidebar.tsx 201-208 | done | shell.rs 289-301 → ShellIntent::OpenSettings(application.rs 4130-4163) | 아이콘 대신 텍스트 버튼(외형) | - |
| 43 | 그룹 섹션 구성(그룹 헤더+멤버, 미분류 구분선, 중복 소속 정리) | project-group.ts, app-sidebar.tsx 154-177 | partial | shell.rs 308-350(멤버를 projects 순서로 필터) | 미분류 구분선·aria group, 한 프로젝트가 두 그룹이면 두 번 표시됨 | S |
| 44 | 그룹 접기/펴기 영속 | sortable-project-group-header.tsx 98-100 | done | shell.rs 309-320 → project_group_set_collapsed(commands.rs 84-86) | chevron·aria-expanded 표시 없음(텍스트 버튼) | S |
| 45 | 그룹 색 tint | sortable-project-group-header.tsx 70, 104-107 | missing | 검색어: resolveProjectGroupColorVar, group.color, project_group_set_color → shell 0건 | 색 적용 | S |
| 46 | 그룹 생성/수정 dialog(이름 ≤N 코드포인트, 12색) | project-group-dialog.tsx | missing | project_group_create/rename는 remote-projects.rs 157-171만 | dialog, 새 그룹+멤버 | M |
| 47 | 그룹 context menu(그룹 전체 열기 결과 toast, 수정, 삭제 확인) | sortable-project-group-header.tsx 73-144 | missing | project_group_open/delete는 remote-projects.rs만 | 메뉴, 삭제 AlertDialog, 결과 toast | M |
| 48 | 그룹에 추가/제거(project_group_set_members) | app-sidebar.tsx 111-122 | missing | remote-projects.rs 184만 | 프로젝트 메뉴(36)와 함께 | S |
| 49 | 그룹 header drag 재정렬(project_group_reorder) | use-project-drag.ts 112-119 | missing | remote-projects.rs 196만 | DnD(39와 같은 기반) | M |
| 50 | rail 접힘(sidebarRailCollapsed) 반영 | use-window-chrome.ts 35 | done | shell.rs 200 | 접기 토글 UI는 TS에서도 확인하지 못함(7장) | - |

### 2.5 Shell slot(다중 프로젝트 분할)

| # | 기능 | TS 근거 | 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|---|
| 51 | slot 트리 렌더·분할 리사이즈·영속(session_set_shell_slot_sizes) | shell-slot-tree-view.tsx 113-160 | done | shell.rs 403-479, 1115-1195, split.rs, commands.rs 87-89 | 없음 | - |
| 52 | 프로젝트를 slot 가장자리에 drop해 분할/교체(project_open_in_slot) | shell-slot-tree-view.tsx 54-109, use-project-drag.ts 121-128 | missing | 검색어: project_open_in_slot, OpenProjectInSlot, ShellSlotDropData → 앱 소스 0건(remote-projects.rs 110만). split-drop-zones 없음 | 5-zone drop UI + 미리보기 + 변종 | L |
| 53 | "Open to the Right/Below/…" 메뉴 | sortable-project-icon.tsx 41-46 | missing | 36번과 동일 | 메뉴 | S |
| 54 | slot 헤더(프로젝트명, 닫기, rootMissing 경고, 단일 slot이면 숨김) | shell-slot-header.tsx | partial | shell.rs 492-539 | rootMissing 경고 아이콘·tooltip, 비활성 상태 | S |
| 55 | slot 닫기(shell_slot_close)·problems 상태 정리 | app-shell.tsx 88-89 | done | shell.rs 523-532, commands.rs 83, problems.reconcile(application.rs 3438) | 오류 toast 대신 status(결함 2) | - |
| 56 | 포커스 slot이 타이틀·상태바 대상 | app-shell.tsx 68, 180, 228 | done | application.rs 3664-3667·5129, snapshot.rs focused_project | 없음 | - |
| 57 | ProjectShell: explorer(기본 240px, 최소 180, 최대 40%) + editor 분할 | project-shell.tsx 142-159 | partial | shell.rs 545-555 egui::Panel::left | 드래그로 최소 미만에서 collapse, collapsed 상태 변화의 영속(드래그 경로) | S |
| 58 | explorer 접기 토글(⌘B)과 layout_set_shell_view 영속 | project-shell.tsx 70, 110-118 | done | shell_keymap.rs 112-124 → commands.rs 93-107 | 없음 | - |
| 59 | explorer 내부 4개 view(files/search/git/outline) 전환 | explorer-panel.tsx | partial | shell.rs 552는 `surfaces.explorer`(application.rs 4641)가 파일 트리만 표시(타 영역) | 뷰 전환 탭·검색/Git/outline(타 영역 감사 참조) | - |
| 60 | Problems 패널(slot별, 기본 220px/최소 120/상한 70%, 키보드 리사이즈) | editor-area.tsx 516-535 | done | shell.rs 556-611, 962-1113 + problems.rs | 없음 | - |

### 2.6 타이틀바·상태바

| # | 기능 | TS 근거 | 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|---|
| 61 | 타이틀바 텍스트(탭 — 프로젝트 + branch, 폭별 숨김) | title-bar.tsx, title-bar-content.tsx | partial | shell.rs 222-266, application.rs 4611-4613 `fn branch(...) -> None` | branch(결함 4), 반응형 숨김, 브랜치 아이콘 | S |
| 62 | 타이틀바 창 chrome(Overlay 스타일, 드래그 영역, traffic light 여백 78px) | tauri.conf.json titleBarStyle Overlay | partial | shell.rs 24 TRAFFIC_LIGHT_INSET은 있으나 main.rs 32-35가 fullsize_content_view/titlebar 설정을 하지 않아 OS 타이틀 위에 한 번 더 그림(결함 3) | macOS fullsize content view, 드래그 영역, 비-mac 처리(TS는 IS_MAC일 때만 그림) | M |
| 63 | 상태바: Problems 토글+오류 개수(aria-pressed) | status-bar.tsx 99-115 | done | application.rs 5132-5139 problems.show_status | 없음 | - |
| 64 | 상태바: LSP running/total·crash 색 | status-bar.tsx 116-121 | done | application.rs 5141-5146 lsp::status::show | 없음 | - |
| 65 | 상태바: IDE 연결 3상태·포트 tooltip | status-bar.tsx 122-145 | done | application.rs 5147-5155 status_ide::show | 없음 | - |
| 66 | 상태바: chord 대기·불일치 flash(1.5초) | status-bar-content.tsx 22, 131-141 | done | application.rs 5159-5164, status-chord.rs, terminal_views.chord_status | flash 지속시간 정확성은 단위 확인 못함(7장) | - |
| 67 | 상태바: 커서 줄:열 | status-bar-content.tsx 71-106 | done | application.rs 5203-5208 status_editor::cursor | 없음 | - |
| 68 | 상태바: 시스템 사용량 + 상세 dialog | status-bar-content.tsx 169 | done | application.rs 5188-5202, 3853-3863 system_usage_view | 없음 | - |
| 69 | 상태바: 에디터/터미널 글꼴 stepper(clamp, reset) | font-size-stepper.tsx | done | application.rs 5165-5187 status_editor::show_font | 없음 | - |
| 70 | 상태바 Zen 숨김(zenHideStatusBar) | app-shell.tsx 226 | done | shell.rs 180 | 없음 | - |

### 2.7 Pane·탭

| # | 기능 | TS 근거 | 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|---|
| 71 | pane 트리 렌더·분할 리사이즈 영속(layout_resize) | pane-node-view.tsx 78-120 | done | shell.rs 629-671, commands.rs 87·131 | 없음 | - |
| 72 | pane 포커스(포인터) | pane-node-view.tsx 162-170 | done | shell.rs 766-775 → FocusPane | 키보드 focus 경로 없음(경미) | - |
| 73 | 포커스 pane 시각 표시(탭바 하단 ring) | pane-tab-bar.tsx 264, 281 | missing | shell.rs 679-690에 포커스 구분 없음 | 하단 강조선 | S |
| 74 | 탭 항목(제목 truncate, preview 이탤릭, dirty 점, pin 아이콘, ✕) | tab-item.tsx | partial | shell.rs 780-851, 941-960 | 파일/터미널 종류 아이콘(75), agent tooltip, hover 시 dirty→✕ 전환은 있음 | S |
| 75 | 탭 종류별 아이콘(FileTypeIcon, 터미널·settings·diff 등, agent 터미널 색) | pane-tab-bar.tsx 60-70 | missing | 검색어: file_type_icon, FileTypeIcon, folder_icon, tab icon → ui/icons.rs는 12개 설정용 아이콘뿐(icons.rs 12-25) | 파일 타입 아이콘 레지스트리 전체 | L |
| 76 | 탭 활성화/중클릭 닫기/pinned 닫기 방지 | tab-item.tsx 34-39 | done | shell.rs 809-838, commands.rs 79-86 request_close_tab | 안내가 toast 아닌 status(application.rs 1668-1675) | - |
| 77 | 탭 pin/unpin | tab-context-menu.tsx 109-112, tab-item.tsx 81 | partial | shell.rs 830-835(pinned 버튼=unpin) | pin하는 경로(메뉴) 없음 | S(72와 묶음) |
| 78 | 탭 context menu(닫기·다른 탭·오른쪽·저장된 탭·모두, pin, keep open, 경로·상대 경로 복사, Finder/explorer reveal, rename, 변경 열기, 파일 이력, 다시 열기(에디터/preview), 분할 4방향, 새 창/메인 창/창 N 이동) | tab-context-menu.tsx 97-192 | missing | 검색어: tab.closeOthers, tab.moveToNewWindow, tab.keepOpen, tab.openChanges, tab.reopenEditorWith, tab.copyRelativePath → application/shell 0건(terminal-host.rs의 tab.split은 터미널 우클릭 메뉴) | 메뉴 전체와 각 핸들러 연결(타 영역 브리지 포함) | L |
| 79 | 탭바 빈 공간 context menu(새 파일·새 터미널·닫은 탭 복원·저장된 탭 닫기·모두 닫기·Welcome 열기·분할)와 더블클릭 새 untitled | tab-bar-context-menu.tsx, pane-tab-bar.tsx 275 | missing | 위 검색어 동일 | 메뉴, 더블클릭 | M |
| 80 | 탭바 `+` 메뉴(새 untitled, 새 터미널) | tab-bar-add-menu.tsx | done | shell.rs 707-756 | 없음 | - |
| 81 | 탭 drag 재정렬·pane 간 이동(pinned 경계 규칙) | editor-area.tsx 471-504 | missing | shell에 DnD 없음. runtime ShellMutation::MoveTab(commands.rs 114)은 키맵 move-tab-to-group(shell_keymap.rs 189)만 사용 | DnD 소스/타깃, pinned 경계 계산 | L |
| 82 | pane 5-zone drop(분할/중앙 이동) + DragOverlay 탭 미리보기 | split-drop-zones.tsx, pane-node-view.tsx 275-280 | missing | 검색어: SplitDropZones, split-drop → 0건. ShellMutation::SplitTab(commands.rs 123)은 키맵 split만 | zone UI, 미리보기 | L(81과 같은 기반) |
| 83 | 탭바 가로 스크롤(휠→가로), overlay scrollbar | pane-tab-bar.tsx 122-125, 287 | partial | shell.rs 693 egui ScrollArea::horizontal | 세로 휠→가로 변환, 활성 탭 가시화 | S |
| 84 | 키맵: close-tab·close-all·reopen·cycle·split·focus-group-N·move-tab-to-group·toggle-terminal·new-terminal·save·font-size·toggle-sidebar·zen·open-keybindings | editor-area.tsx 293-321 | done | shell_keymap.rs 6-37 31개 액션 + action() 76-228, 라우팅 application.rs 3934-3938 | `find`/`search`/`search-replace`/`quick-open`/`command-palette`는 타 영역 | - |
| 85 | dirty 탭 닫기 확인(저장/저장 안 함/취소, 다건 묶음 1회 질문, untitled Save As, 저장 실패 시 닫기 취소) | use-request-close-tab.tsx, close-dirty-tab-dialog.tsx | done | application.rs 1647-2110, close_dialog.rs, tab_close_batch.rs, tabs.rs close() 33-143 | 없음(전용 테스트 파일 존재) | - |
| 86 | 일괄 닫기(Close Others/To Right/Saved/All) | pane-tab-bar.tsx 141-151 | partial | request_close_tabs(application.rs 1710)와 close-all-tabs 키맵만 | Others/ToRight/Saved/All 진입 UI(78·79 의존) | S |
| 87 | 닫은 탭 복원 | pane-tab-bar.tsx 173 | partial | shell_keymap.rs 108-111 ReopenClosed(⌘⇧T 키맵) | 탭바 메뉴 항목 | S |
| 88 | 탭 콘텐츠: file/untitled/terminal/settings/appFile | pane-node-view.tsx 178-254 | done | application.rs 4751-5127 tab_content(preview 포함 타 영역) | 없음 | - |
| 89 | 탭 콘텐츠: diff/commit diff/claudeDiff/searchEditor | pane-node-view.tsx 203-232, 242-244 | missing | application.rs 4916-4922: 미지원 종류는 제목 라벨 + status "native tab surface is not connected" (결함 5) | 각 pane(타 영역: Git·IDE·검색) | XL(타 영역 합산) |

### 2.8 Provider 계열(앱 수준)

| # | 기능 | TS 근거 | 상태 | native 근거 | 빠진 것 | effort |
|---|---|---|---|---|---|---|
| 90 | 외부(agent CLI) 열기 요청 처리(프로젝트·파일 탭 열기, `--wait` marker와 pinned 탭 연동) | agent-external-open-provider.tsx | missing | 검색어: external_open, ExternalOpen, agentExternalOpen, pending_external → event-relay.rs 203이 `AgentExternalOpen`을 None으로 버림, PaintSink(application.rs 52-58)는 tree/presentation만 기록. runtime `agent_pending_external_opens`(agent_actions.rs 130)는 remote-agents.rs만 호출 | 시작 시/이벤트 시 큐 drain, 프로젝트 활성화, marker 회수 | M |
| 91 | native(OS) 알림(agent 완료/입력 대기, terminal 명령 완료, LSP 설치 결과) | native-notification-provider.tsx | missing | NativePlatform::send_notification(bootstrap.rs 151)은 있으나 호출 정책이 없음(검색어: notification_actions, native_notification → runtime 함수·bridge만, 이벤트→알림 연결 0건) | 이벤트 구독, 시작 시각·첫 안내, 배경 창 판정 | M |
| 92 | IDE 동기화(diff/save/close 요청, 진단 push) | ide-sync-provider.tsx | partial | ide-tools.rs(IdeDiffRequested 발행 179)·ide-server.rs·상태바 연결 | claudeDiff 탭 pane이 없어 diff 요청 표시 불가(89), 진단 push 연결은 확인 못함 | L(타 영역) |
| 93 | IPC 동기화(프로젝트·layout·설정·Git·terminal push → 화면 갱신) | ipc-sync-provider.tsx | done | controller.rs 40-98(이벤트마다 snapshot 재읽기 후 repaint), application.rs PaintSink | 없음 | - |
| 94 | 테마·로케일 적용과 로드 오류 배너/재시도 | theme-provider.tsx, locale-provider.tsx | partial | application.rs 565-617 presentation 적용(타 영역) | 오류가 status 문자열로만 표시, 재시도 UI 없음 | S |
| 95 | keybindings 런타임(override 적용, 편집기 열기) | keybindings-runtime-provider.tsx | done | application.rs 4164 OpenKeybindings, 4259-4278 keybindings.show | 없음 | - |
| 96 | Emmet 약어 확장 provider | emmet-provider.tsx | missing | 검색어: emmet → 설정 UI(settings-code-*.rs)만, 에디터 엔진 없음 | 편집기 확장(타 영역) | L |
| 97 | 외부 링크 anchor 가로채기 | external-link-provider.tsx | n/a | webview anchor가 없음. native 링크는 terminal_links·open_url 경로 | - | - |
| 98 | 앱 provider(QueryClient·Tooltip·toast 계층) | app-providers.tsx | n/a | 대체 구조(ShellController snapshot, tooltips::Provider, toast::Toasts) | - | - |
| 99 | shell mutation 오류의 사용자 피드백(`toast.error(describeIpcError)`) | app-shell.tsx 89, use-project-drag.ts 127 | partial | application.rs 488-490 controller 오류와 100곳의 `self.status = Some(...)`가 상태바 문자열로 표시(결함 2) | 로컬라이즈된 toast, 자동 소거 | M |

집계(99행): done 33, partial 25, unwired 4, missing 33, n/a 4. 가중치 없는 행 단순 합이며 기능 크기를 반영하지 않습니다.

## 3. 실제 앱 연결이 끊긴 지점

1. `WindowScope::Auxiliary`(native-ui/src/shell.rs 56, 133-161): 렌더 경로가 있으나 NativeApplication이 한 번도 만들지 않습니다(application.rs 253-256). main.rs는 단일 `ViewportBuilder` 창뿐이고 `show_viewport_*`/`ViewportCommand::Title` 호출이 없습니다.
2. runtime `layout_actions::layout_move_tab_to_window`·`taide-window` 복원 계획: native에서 호출처 없음. 레이아웃 모델(`auxiliary_windows`)은 TS 시절 데이터를 그대로 읽습니다.
3. runtime `project_actions::{project_close, project_reorder, project_set_display, project_open_in_slot, project_group_create/rename/set_color/set_members/delete/reorder/open, project_list_recent}`: browser remote 경로(native-app/src/remote-projects.rs 110-196)에서만 호출되고 egui 셸의 ShellMutation(ui/src/commands.rs 10-47)·HostCommand(host.rs 21-)에는 변종이 없습니다. 즉 같은 기능이 원격 브라우저 클라이언트에서는 도달 가능하고 데스크톱 UI에서는 도달 불가입니다.
4. `AppEvent::AgentExternalOpen`·`HotExitFlushRequested`: event-relay.rs 203이 원격 프레임 변환에서 버리고, PaintSink는 소비하지 않습니다.
5. `NativePlatform::send_notification`(bootstrap.rs 151): 호출 정책 없음. `notification_actions`는 src-tauri에서만 연결되어 있습니다.
6. `ShellMutation::{MoveTab, SplitTab, KeepTab, ReopenClosed, SetSidebarCollapsed}`: 키맵 또는 더블클릭 외에는 UI 진입점이 없어 메뉴 기반 기능이 비어 있습니다.
7. 탭 종류 Welcome·Diff·ClaudeDiff·SearchEditor: 데이터와 runtime 동작(tabs.rs 128 ClaudeDiff 닫기 응답)은 있으나 pane 렌더러가 없습니다.

## 4. 잘못 구현됐거나 보강이 필요한 코드(결함·설계)

1. 보조 창 탭이 접근 불가 (high): TS 시절 `ProjectLayout.auxiliary_windows`에 들어 있던 탭은 native에서 렌더되지 않습니다(shell.rs main_window는 `layout.root`만 그림). 그런데 `tabs::close`(tabs.rs 49-55)와 `request_tab_close`(application.rs 1655-1662)는 보조 창 탭도 후보로 다뤄 데이터상 존재·dirty·hot-exit 대상이 되지만 사용자는 볼 수도 닫을 수도 없습니다. 근본 해결은 보조 창 구현(4~8번)이며, 선행 최소 조치로 복원 시 보조 창 탭을 메인 root로 합치는 migration이 필요합니다. 결정 필요 사항입니다.
2. 오류·안내 채널이 상태바 문자열 (medium): application.rs의 `status: Option<String>`은 3697에서 take한 뒤 3852에서 다시 대입되고 소거 코드가 없어(검색: `status = None` 0건) 한번 설정되면 다른 오류가 덮을 때까지 상태바에 남습니다. 대입 지점이 100곳이며 다수가 `error.to_string()` 원문(비로컬라이즈)입니다. TS는 `toast.error(describeIpcError(error))`(app-shell.tsx 89)로 로컬라이즈된 일시 알림을 씁니다. toast 인프라(toast.rs: ipc_error, warning)는 이미 있으므로 shell/host 오류를 그쪽으로 모으고 status 라벨은 제거해야 합니다. pinned 닫기 경고(application.rs 1668-1675)도 TS는 toast.warning입니다.
3. 창 크롬 불일치 (medium): main.rs 32-35는 OS 기본 타이틀바를 쓰는데 shell.rs 24는 Overlay 스타일을 전제한 78px traffic light 여백으로 28px 자체 타이틀바를 항상 그립니다(TS는 IS_MAC에서만). 결과적으로 타이틀이 OS 타이틀과 이중으로 보이고 창 제목은 "TAIDE Native" 고정입니다. macOS는 fullsize content view + 투명 타이틀, 그 외 플랫폼은 자체 타이틀바 생략으로 정리해야 합니다.
4. branch가 항상 None (low/medium): application.rs 4611-4613 `ShellSurfaces::branch`가 상수 None이라 타이틀바에 Git 브랜치가 나오지 않습니다. 선행 데이터는 Git 서비스에 있습니다(타 영역 연결 필요).
5. 미지원 탭 종류의 처리 방식 (medium): application.rs 4916-4922는 라벨만 그리고 status에 "native tab surface is not connected: {:?}"를 계속 씁니다. Diff·ClaudeDiff·SearchEditor·Welcome 탭이 레이아웃에 있으면(IDE 요청·Git·검색이 만든 탭 포함) 빈 화면과 영구 오류 문구가 됩니다. 각 pane 구현 전까지는 최소한 localized 안내 화면이 필요합니다.
6. `welcomeOnEmptyEditor` 무시 (low): shell.rs 759-765가 설정과 무관하게 Welcome을 그립니다. 설정 UI(settings-controls.rs)는 이 값을 저장하지만 읽는 쪽이 없어 토글이 무효입니다.
7. 일관성 없는 tooltip (low): shell.rs 399, 739, 938, 959는 `on_hover_text`를 쓰고 상태바·explorer는 앱 전용 `tooltips::Provider`(지연·배치·모션 규칙)를 씁니다. rail·탭·슬롯 헤더 tooltip이 같은 시스템을 쓰지 않습니다.
8. 그룹 소속 중복 표시 (low): shell.rs 322-326은 한 프로젝트가 두 그룹의 members에 있으면 두 번 그립니다. TS resolveProjectGroupSections는 마지막 그룹 하나로 분할을 보장합니다.
9. 탭바 휠 스크롤 (low): TS는 세로 휠을 가로 스크롤로 변환(pane-tab-bar.tsx 122-125). egui ScrollArea::horizontal은 마우스 휠만으로 가로 스크롤되지 않아 탭이 많을 때 접근이 불편합니다. 활성 탭 가시화도 없습니다.
10. 데이터 디렉터리 격리 고정 (design): bootstrap.rs 27-31이 `--data-dir`을 필수로 하고 SECRET_SERVICE가 native-isolated라서 현재는 기존 앱 데이터를 읽는 구성이 아닙니다. 의도된 격리(docs/acknowledge/2026-09-30-m8-code-first-parity.md)지만 컷오버 전에 양방향 호환 경로를 별도 확정해야 합니다.

## 5. 권장 구현 순서(의존 관계 포함)

1. 결정: 보조 창을 egui 다중 viewport로 갈지, 메인 창 흡수로 갈지 확정(4번 결함·2.1 4~8). 갈 경우 ViewKey.window 라벨·상태(`NativeApplication` 단일 필드들: store/terminal_views/…)의 창별 분리 설계가 선행입니다. 흡수로 가면 복원 시 병합 migration(S~M)만 먼저 넣습니다.
2. ShellMutation/HostCommand 확장: project_close·reorder·set_display·open_in_slot·group CRUD·open group·recent 목록·move_tab_to_window를 기존 runtime 액션에 연결(M). 이후 모든 UI가 이 변종에 의존합니다.
3. 알림 채널 정리(결함 2): status 문자열을 toast로 전환(M). 이후 추가되는 모든 UI의 오류 피드백이 이 위에 올라갑니다.
4. egui DnD 기반 구축(L): rail 프로젝트·그룹 재정렬(2.4 39·49), slot 5-zone drop(52), pane 5-zone drop(82), 탭 정렬/이동(81)을 같은 컬리전 규칙(use-project-drag.ts 59-65, editor-area.tsx 471-504)으로 구현.
5. Context menu와 dialog 묶음(L): 프로젝트 메뉴(36)·그룹 메뉴(47)·탭 메뉴(78)·탭바 메뉴(79)·표시/그룹/경로 dialog(38·41·46)·삭제 확인. 2번 완료 후.
6. Welcome 완성(M): 최근 목록·단축키 카드·welcome 탭·`welcomeOnEmptyEditor`·root 밖 파일 거부(27~31). 2번의 recent 목록에 의존.
7. 창 크롬(M): 타이틀바 방식·창 크기/최소/타이틀·window-state·branch·Zen 힌트·OS 파일 drop·포커스 pane 강조·탭 아이콘(2.1 2, 3, 22; 2.2 20; 61, 62, 73, 75). 탭 아이콘 레지스트리는 explorer 아이콘과 공용이므로 타 영역과 조율.
8. 네이티브 메뉴(M): muda 등 도입은 의존성 추가이므로 사용자 승인 필요. Open Recent·Quit은 2번의 최근 목록에 의존.
9. 보조 창 구현(1번 결정이 viewport일 때, XL): 2.1 4~8, 11, 탭 메뉴의 창 이동, hot-exit flush 범위 응답.
10. Provider 잔여(M): agent 외부 열기(90)·OS 알림(91)·IDE diff 표시(92, claudeDiff pane은 타 영역).
11. 탭 종류 pane 4종(Diff·ClaudeDiff·SearchEditor·Welcome)은 각 타 영역 감사와 합쳐 순서화.

## 6. 시도한 검색어(missing 판정 근거)

- OS drop: dropped_files, hovered_files, DroppedFile, drag_drop, on_file_drop
- 다중 창/메뉴/창 상태: show_viewport, ViewportBuilder, ViewportCommand, ViewportClass, NSMenu, menu_bar, MenuBar, muda, tray, set_menu, window_state, window-state, with_position, with_min_inner_size, layout_move_tab_to_window, TabWindowTarget, NewAuxiliary
- DnD: dnd_drag_source, dnd_drop_zone, DragAndDrop, SplitDropZones, ShellSlotDropData, project_open_in_slot, OpenProjectInSlot
- 메뉴·메시지 키: tab.closeOthers, tab.moveToNewWindow, tab.keepOpen, tab.openChanges, tab.reopenEditorWith, tab.copyRelativePath, project.close, shellSlot.openToTheRight, projectGroup.addTo, project.copyPath, project.displayMenu, sidebar.openByPath, sidebar.addProjectMenu
- Welcome·Zen: recent_projects, RecentProject, recentItems, project_list_recent, keyboardShortcutsTitle, WELCOME_KEYMAP_HIGHLIGHT_IDS, welcomeOnEmptyEditor, app.dropToOpen, zen.hint, ZenModeHint, zen.hintExit
- 배지·아이콘: agentStatusBadge, AgentStatusBadge, agent.sessionTooltip, projectAgents, file_type_icon, FileTypeIcon, folder_icon
- Provider: external_open, ExternalOpen, agentExternalOpen, pending_external, notification_actions, native_notification, emmet, palette, task_runner, TaskRunner, quick-open

## 7. 확인하지 못한 것(불확실성)

- 실행·스크린샷 검증은 하지 않았습니다. 모든 판정은 소스 읽기와 호출 체인입니다.
- application.rs는 요청 지시와 달리 5,534줄 전체를 읽지 못했습니다. 읽은 구간은 1-650, 1584-1960, 3360-3600, 3599-4515, 4516-5214(일부)입니다. 650-1359(poll 응답 처리)·1960-3360(저장/LSP/preview 보조)·5214-5534(show_document)는 grep 함수 목록과 필요한 키워드 검색으로만 확인했습니다. 그 구간에서 이 영역 기능(예: 창 이벤트)을 추가로 처리하지 않는다는 점은 `ViewportCommand`·`dropped_files`·`status` 전수 grep으로 확인했습니다.
- macOS Cmd+Q·Dock 종료가 `close_requested`를 거치는지, 아니면 `on_exit`만 타는지는 winit/eframe 동작 확인이 필요합니다. on_exit(application.rs 4376)에 직접 종료 경로가 있고 application-exit-tests.rs가 있으나 내용은 읽지 않았습니다.
- egui `Sense::drag()` 분할 핸들이 키보드 focus를 받는지(shell.rs 1175 `has_focus` 분기가 실제로 실행되는지)는 egui 소스를 확인하지 않았습니다.
- `sidebarRailCollapsed`를 켜는 TS UI는 이 감사에서 찾지 못했습니다(키맵·팔레트 경로 추정). native의 렌더 반영(shell.rs 200)은 확인했습니다.
- chord 불일치 flash 1.5초(TS CHORD_NO_MATCH_INDICATOR_DURATION_MS)와 native 지속시간 일치 여부는 terminal_views.chord_status 구현을 읽지 않아 미확인입니다.
- IDE 동기화·테마/로케일·Emmet·팔레트는 타 영역 감사 소관이라 연결 여부만 확인했고 동작 동등성은 판정하지 않았습니다.
- 브라우저 Wasm 클라이언트(taide-remote-web)는 같은 NativeShell을 공유하므로 위 shell.rs 결함은 두 쪽에 동시에 적용되나, 이쪽 UI 연결은 감사하지 않았습니다.
