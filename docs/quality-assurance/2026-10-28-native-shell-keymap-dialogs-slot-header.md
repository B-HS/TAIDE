# 셸 전역 키맵·태스크 실행기·slot 헤더

## 대상

- shell-21: 전역 키맵 `toggle-sidebar`/`explorer`/`git`
    - 원본: `src/widgets/app-shell/app-shell.tsx` 143-147, `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 89-108
- shell-24: `MainWindowDialogs`의 CommandPalette·TaskRunner(포커스 프로젝트 기준)
    - 원본: `src/app/main-window-dialogs.tsx` 21-29, `src/widgets/task-runner/task-runner-dialog.tsx` 29-74
- shell-54: slot 헤더(프로젝트명, 닫기, rootMissing 경고, 단일 slot이면 숨김)
    - 원본: `src/features/shell-slot/shell-slot-header.tsx` 32-59, `src/widgets/app-shell/shell-slot-tree-view.tsx` 83-90

## 원본 동작

### shell-21 전역 키맵

- `app-shell.tsx` 143-147은 세 키를 창 수준에서 한 번만 등록합니다.
    - `toggle-sidebar`는 `requestToggleExplorerSidebar()`, `explorer`는 `requestShowExplorerView('files')`, `git`은 `requestShowExplorerView('git')`입니다.
    - 기본 키는 `keymap-defaults.json`의 ⌘B, ⌘⇧E, ⌃⇧G입니다. 명령 팔레트의 `view.explorer`·`view.git`(`src/shared/lib/command-catalog.ts` 92-94)도 같은 bridge를 부릅니다.
- 받는 쪽은 포커스 slot만 반응합니다.
    - `project-shell.tsx` 98-101·121: 포커스 slot이면 접힌 explorer 패널을 `expand()`합니다. Zen은 보지 않고, imperative 호출이라 `sidebarCollapsed`를 저장하지 않습니다(84-90).
    - `explorer-panel.tsx` 142-149: 포커스 slot의 sidebar 보기를 요청한 보기(`files`/`git`)로 바꿉니다.
    - 토글은 `project-shell.tsx` 110-118입니다. 포커스 slot이고 Zen이 아닐 때만 접고 펴며 저장합니다.
- 보조 창(`auxiliary-window-shell.tsx` 89-108)도 같은 세 키를 등록합니다. 자기 창의 view-local 패널만 접고 펴며, 보기 요청은 자기 패널을 `expand()`합니다.
- `git` 보기는 원본에서 항상 존재합니다(`explorer-panel.tsx` 24-31, 259). 그래서 "git 보기가 없을 때"의 원본 동작은 따로 없습니다.

### shell-24 CommandPalette·TaskRunner

- `main-window-dialogs.tsx` 21-29는 `useFocusedProjectId()`(포커스 slot의 프로젝트)를 두 대화상자에 넘깁니다.
- `task.runTask` 명령(`src/entities/task/task.commands.ts` 7-15)은 활성 프로젝트가 있을 때만 켜지고, task runner를 엽니다. 기본 키는 없습니다.
- `task-runner-dialog.tsx` 29-74의 동작은 다음과 같습니다.
    - 화면에 보이지 않는 제목과 입력 placeholder는 모두 `task.runTask`입니다. 목록은 그룹 제목 없이 한 묶음입니다.
    - 열린 동안만 `detect_tasks`를 조회합니다. 경로는 `task.query.ts` 6-11 → `src-tauri/src/domain/task/commands.rs` 9-13 → `crates/taide-runtime/src/task_actions.rs` 29-34 → `taide_task::service::detect_tasks`입니다.
    - 빈 목록 문구는 조회 중이면 `common.loading`, 아니면 `task.noTasksFound`입니다. 조회 실패도 데이터가 없어 `task.noTasksFound`가 되고 toast는 없습니다.
    - 질의는 `fuzzyFilter(query, tasks, label)`로 거릅니다. 행은 Terminal 아이콘, 이름(말줄임), 같은 줄의 출처 문구(`task.sourceNpm`/`task.sourceMake`/`task.sourceCargo`, 작은 muted 글자)입니다.
    - 선택하면 `requestEditorPaneCommand({ type: 'run-in-terminal', text: command, cwd })` 후 닫고, 닫으면 질의를 비웁니다.
- 실행은 포커스 slot의 `editor-area.tsx` 257-281·323-331입니다.
    - 포커스 pane에서 활성 터미널, 없으면 첫 터미널 탭을 고릅니다(`terminal-tab-targets.ts` 19-23). 그 탭을 활성화하고 `text + "\n"`을 씁니다.
    - 터미널이 없으면 포커스 pane에 `cwd`를 가진 새 터미널 탭(`terminal.title`)을 열고, 만들어진 뒤 씁니다.

### shell-54 slot 헤더

- `shell-slot-tree-view.tsx` 83-90: slot이 2개 이상이고 Zen이 아닐 때만 그립니다. 이름은 `project.name ?? projectId`입니다.
- `shell-slot-header.tsx` 32-59의 표면은 다음과 같습니다.
    - 높이 24px, `appSidebar.background`, 아래 `tabBar.tabBorder` 1px 선, 좌우 8px, 간격 4px, 세로 가운데 정렬입니다.
    - root 누락이면 이름 앞에 12px TriangleAlert(`appSidebar.iconDefault`)를 둡니다. `role=img`·`aria-label`과 아래쪽 tooltip은 모두 `app.recentProjectRootMissing`입니다.
    - 이름은 `text-xs`(12px), `app.foreground` 70%, 말줄임입니다.
    - 닫기는 `shellSlot.close` 버튼입니다. focus-ignore wrapper 안에 있습니다.
- 포커스 추적은 `shell-slot-provider.tsx` 53-69와 `shell-slot.ts` 50-58입니다. slot 요소 안의 pointerdown·focusin이 그 slot을 포커스하고, 헤더도 여기에 포함됩니다. 닫기 버튼(focus-ignore)은 제외합니다.

## 구현

### UI(`taide-native-ui`)

- `command-registry.rs`
    - `Run::ShowExplorer`를 추가하고 keymap `explorer`와 명령 `view.explorer`에 연결했습니다.
    - `Run::OpenTaskRunner`(native-host 전용)를 추가하고 `task.runTask`에 연결했습니다. 기존 `ActiveProject` 조건은 그대로입니다. 동결 remote-web 빌드(native-host 없음)에서는 실행 경로가 없는 상태를 유지합니다.
    - `git`은 계속 실행 경로가 없습니다.
- `commands.rs`: `ShellIntent::ShowExplorer{project, slot}`, `ShowAuxiliaryExplorer{project, slot}`, `OpenTaskRunner(project)`(native-host)를 추가했습니다.
- `command-palette.rs`: 코드 렌즈·들여쓰기 선택과 같은 picker 방식으로 task runner를 추가했습니다.
    - 상태: `open_tasks(generation)`, `show_tasks(generation, tasks)`(세대가 다른 응답은 무시), `is_picking_tasks`.
    - 표면: placeholder와 입력 접근성 이름은 `task.runTask`이고, 목록은 제목 없는 한 묶음입니다.
    - 빈 목록: 응답 전 `common.loading`, 응답 후 `task.noTasksFound`.
    - 행: 이름 fuzzy 필터, Terminal 아이콘, 같은 줄 출처 문구.
    - 선택: `Action::RunTask{generation, task}` 후 포커스를 복원하며 닫습니다.
    - 팔레트에서 `task.runTask`를 고르면 팔레트를 닫지 않고 task 목록으로 바꿉니다(들여쓰기 선택과 같은 처리). 그래서 원래 포커스로 돌아갑니다.
    - 파일·심볼·워크스페이스 심볼 관찰은 picker 동안 멈춥니다. 세 조건을 `has_picker`로 합쳤습니다.
- `shell.rs` slot 헤더
    - 아래 `tab_border` 1px 선을 그립니다.
    - 내용을 헤더 높이 전체에서 세로 가운데 정렬하고, 간격은 4px입니다.
    - root 누락이면 12px 경고 glyph(`ShellSurfaces::rail_icon`의 `RailIcon::RootMissing`, `muted` 색)를 그립니다. Image 접근성 이름과 아래쪽 tooltip은 `app.recentProjectRootMissing`입니다.
    - 이름은 12px, foreground 70%입니다.
    - 헤더를 누르면 slot 포커스 대상이 되고, 닫기 버튼 영역은 제외합니다. 이전에는 헤더 아래 본문만 포커스 대상이었습니다.
    - 기존 `ShellSurfaces` 기본 구현을 그대로 써서 remote-web은 바뀌지 않습니다.

### App(`taide-native-app`)

- `shell_keymap.rs`
    - `explorer`는 메인 창에서 `ShowExplorer{포커스 프로젝트, 포커스 slot}`, 보조 창에서 `ShowAuxiliaryExplorer`입니다.
    - 프로젝트가 없어도 키를 소비합니다(`runs_without_focused_project`). 원본의 창 수준 handler와 같습니다.
    - `OpenTaskRunner`는 대상 창의 프로젝트(메인은 포커스 slot)로 intent를 만듭니다.
- `application.rs`
    - `ShowExplorer`: 그 slot의 sidebar 보기를 Files로 바꾸고, 접혀 있으면 `SetSidebarCollapsed{false}`를 보냅니다.
    - `ShowAuxiliaryExplorer`: 보조 창 explorer를 열고 Files로 바꿉니다.
    - 위 두 처리는 기존 `RevealInExplorer`·`RevealInAuxiliaryExplorer`와 같아서 `show_explorer_files`·`show_auxiliary_explorer_files`로 공통화했습니다.
    - `OpenTaskRunner`: 세대를 올리고 `palette.open_tasks` 후 `HostCommand::DetectTasks`를 보냅니다. 제출이 실패하면 빈 목록을 표시합니다.
    - `HostReply::TasksDetected`: 결과를 `show_tasks`에 넘기고, 실패는 로그만 남기고 빈 목록으로 둡니다.
    - `Action::RunTask`: 같은 세대의 프로젝트와 그 창의 포커스 pane으로 `TerminalDestination{cwd: task.cwd, payload: command + "\n", title: terminal.title}`을 만들어 `HostCommand::RunTask`를 보냅니다.
- `host.rs`
    - `DetectTasks`는 기존 `taide_runtime::task_actions::detect_tasks`를 씁니다.
    - `RunTask`는 기존 `HostReply::EditorTerminalTarget` → `editor_terminal_writes` → 세션 준비 후 `WriteEditorText` 전달 경로를 그대로 씁니다.
- `editor-context-menu.rs`
    - 터미널 대상 결정(활성 터미널 > 첫 터미널 > 새 탭, mutation guard·layout 이벤트)을 `open_terminal_target`으로 분리했습니다.
    - 편집기 선택 실행은 `cwd: None`, 태스크는 `cwd: Some(task.cwd)`로 같은 코드를 씁니다.

### 판단

- explorer 키의 접힘 해제 저장: 원본은 imperative `expand()`라 다음 재시작 때 다시 접힙니다. native는 패널 표시가 저장된 `sidebar_collapsed` 하나로 정해집니다. 그래서 기존 `RevealInExplorer`처럼 해제를 저장합니다.
- Zen 중 explorer 키: 원본은 Zen 강제 접힘을 무시하고 패널을 폅니다. 원본 토글 handler가 명시적으로 피하는 desync입니다. native는 Zen 동안 숨김을 유지하고, Files 보기와 접힘 해제만 반영해 Zen을 나가면 보입니다.
- task 목록은 열 때마다 다시 감지합니다. 원본의 TanStack 60초 캐시 수치는 재현하지 않았습니다.
- 팔레트 안의 task 선택은 대화상자 닫힘·열림 애니메이션 없이 내용만 바뀝니다. 포커스 복원은 원래 위치로 돌아갑니다.

### 기존 검사 조정

- `command-registry.rs` 실행 경로 표와 `command-dispatch.rs` gate 표:
    - `explorer`·`view.explorer`·`task.runTask`를 미실행 목록에서 실행 목록으로 옮겼습니다.
    - native keymap 수는 34에서 35로 바뀌었습니다.
- `tests/workbench.rs`
    - rail glyph 검사를 rail 열 안의 glyph로 한정했습니다. slot 헤더 경고 glyph가 같은 `rail_icon` 표면을 쓰기 때문입니다.
    - 탭바 하단선 검사는 slot 헤더 하단선 아래만 셉니다. 기대값 자체는 그대로입니다.
- `application-auxiliary-window-tests.rs`: 보조 창 ⌘⇧E 검사를 기존 보조 창 검사에 추가했습니다.

## 검증

- UI: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --locked --offline … --lib`는 165통과입니다.
    - 새 palette 2건: 로딩, 다른 세대 무시, 이름·출처, fuzzy, Enter 실행·포커스 복원, 빈 결과, 취소, 명령에서 이어받은 포커스, 클릭 실행, 다른 모드 전환을 확인합니다.
    - registry 검사: `task.runTask` 실행 경로·조건, `explorer` keymap을 확인합니다.
- UI: 같은 명령의 `--test workbench`는 44통과입니다.
    - 새 실제 egui 검사 1건이 다음을 확인합니다.
        - 2 slot에서 root 누락 slot만 12px 경고, 위치·색, Image 접근성, 아래 tooltip
        - 이름 12px·70%·간격, 하단선, 닫기 2개
        - 헤더 이름 클릭의 `FocusSlot`, 닫기 클릭의 `CloseSlot`만
        - 단일 slot·Zen에서 헤더 없음
- App: `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib -- <필터>` 실행 결과입니다.
    - 새 `shell_dialogs_tests` 2건:
        - 실제 `NativeApplication`에 ⌃⇧G를 보내면 아무것도 바뀌지 않습니다. ⌘⇧E는 포커스 slot만 접힘 해제·Files로 바꾸고, 다른 slot은 그대로입니다.
        - 실제 ⇧⌘P 팔레트에서 "Run Task"를 고르면 합성 `package.json`·`Makefile`의 build·lint·check와 출처 문구가 보입니다. lint를 실행하면 포커스 pane에 `cwd`=project root, `terminal.title` 터미널 탭이 활성화되고 `npm run 'lint'\n` 전달이 대기열에 들어갑니다.
        - 실행 뒤에는 그리지 않고 `logic`만 돌려 실제 셸을 띄우지 않았습니다.
    - `shell_dialogs_tests`와 `shell_keymap::` 7건(보조 창 `explorer`·`git`·task, 메인 Zen·포커스 없음 포함)을 함께 돌린 첫 실행은 8통과·1실패입니다.
        - 실패는 task 검사였습니다. 기본 layout에 이미 터미널 탭이 있어 원본 규칙대로 그 탭이 재사용된 것을 fixture가 놓쳤습니다.
        - 터미널 없는 fixture로 고친 뒤 단독 1통과입니다. 아래 동시 실행에서도 `shell_dialogs_tests` 2건이 모두 통과했습니다.
        - 기존 터미널 재사용 경로는 공통화한 `open_terminal_target`의 기존 검사가 덮습니다.
    - 보조 창 검사와 탭 메뉴 reveal 검사 2통과, 공통화한 터미널 대상·reveal 경로 검사 4통과입니다.
    - `command_dispatch::`·`keymap::`·`shell_keymap::`·`command_palette::`·`presentation::` 39통과, `keybinding` 22통과입니다.
    - `project_rail_tests`·`window_state_tests`·`shell_dialogs_tests` 동시 실행은 8통과·1실패입니다.
        - 실패: `실제_app_그룹은…`의 60초 OS watcher 대기 시간 초과
        - 단독 재실행은 1통과입니다. 알려진 병렬 부하 시간 초과 유형입니다.
    - 보조 창 검사 파일은 마지막에 서식만 바꿨고 재실행하지 않았습니다.
- `cargo check --all-targets`
    - App: exit 0
    - UI(`native-host,inspection`): exit 0
    - 동결 remote-web `--target wasm32-unknown-unknown --lib`: exit 0
- `cargo fmt -- --check`
    - UI는 exit 0입니다.
    - App은 이번 변경과 무관한 기존 차이만 남아 exit 1입니다: `experiments/lsp-coordinator-spike` mock-server, `application.rs` 962·11332, `lib.rs` 모듈 순서. HEAD에도 있는 차이입니다. 이번에 바꾼 파일 구간에는 차이가 없습니다.

## 남은 범위

- shell-21의 `git` 키(⌃⇧G)와 `View: Git` 명령은 실행 경로가 없습니다. 키를 소비하지 않고 명령은 비활성입니다. git 보기 자체를 만드는 배치 26에서 `requestShowExplorerView('git')`에 해당하는 보기 전환과 접힘 해제를 연결합니다.
- task 실행의 실제 PTY 출력, slot 헤더 픽셀·tooltip 지연, 실제 OS 키 입력은 배치 31 실기입니다.
