# Native 전환 배치 2 단계 2/3 — 명령 레지스트리·디스패처

작성일 2026-10-06. 기준 커밋 `0973248b` 위의 미커밋 작업 트리(앞 단계 test-health 변경 포함)에서 진행했습니다. GUI 실행·실기 확인은 하지 않았고, 아래 결과는 전부 이 문서에 적은 명령의 실제 출력입니다.

## 0. 결론

- 검증 계약 V1~V5 의 명령은 모두 exit 0 입니다. native-app lib 테스트는 322 → 329(신규 7), native-ui lib 테스트는 52 → 57(신규 5)이며 실패 0 입니다.
- TS 가 등록하는 명령 212개(비 monaco 55 + monaco 157)가 등록 순서·id·titleKey·categoryKey·keymapId·titleDefaultValue 그대로 Rust 레지스트리에 들어 있고, 각 명령은 `Execution::Native(Run)` 또는 `Execution::Unavailable` 로 타입이 갈립니다. Native 34개, Unavailable 178개입니다.
- `shell_keymap.rs` 의 문자열 목록 `ACTIONS` 와 문자열 `match` 는 없어졌고, 키맵 액션과 명령이 같은 typed 요청(`Run`) → `shell_keymap::intent` → `ShellIntent` 경로로 실행됩니다.
- 사용자 override 가 keymap id 없는 명령 행(`runsViaCommand`)에도 적용됩니다. 앱 키맵이 이기고, 실행 가능한 명령일 때만 키를 소비합니다.
- 사용자 결정이 필요한 사항 2건(하드코딩 ⌘N, 실행 불가 행의 키바인딩 편집기 노출)과 미구현 4건은 7장에 적었습니다.

## 1. 항목별 변경

### R1 명령 모델 (순수 타입·함수)

신규 `native/taide-native-ui/src/command-registry.rs` (`taide_native_ui::command_registry`).

| 대상 | 내용 |
|---|---|
| `Command` | `id` `title_key` `category_key` `keymap_id` `title_default_value` `default_binding_label` `is_mac_only` `execution` `enablement` |
| `CommandContext` | `active_project`(activeProjectId) · `focused_shell_slot`(focusedShellSlotId) · `active_editor_actions`(activeEditorActionIds, `None` = 활성 편집기 없음) · `window`(TS 의 전역 `getWindowContext().kind`) |
| `Run` | typed 실행 요청 17종. TS `CommandContext` 의 콜백과 `EditorPaneCommand` 를 클로저 대신 값으로 표현했습니다 |
| `Execution` | `Native(Run)` / `Unavailable` |
| `Enablement` | TS `isEnabled` 의 1:1 이식. `Always` `Never` `WebviewDiagnostics` `MainWindow` `AuxiliaryWindow` `ActiveProject` `ActiveEditor` `SupportedEditorAction` |
| `Command::runnable` / `is_runnable` | 실행 경로가 있고 `Enablement` 가 허용할 때만 `Some(Run)` / `true` |
| `format_categorized_label` / `Command::label` | `카테고리: 제목`. 제목은 번역 → `titleDefaultValue` → 키 순서, 카테고리 번역이 없으면 키를 그대로 씁니다 |
| `keymap_run` | 키맵 액션 id → `Run` (31개) |
| `Registry::editor_action_ids` | native 편집기가 수행할 수 있는 monaco 액션 id 집합. 읽기 전용 문서에서는 문서 편집 액션을 뺍니다 |
| `registry()` | 프로세스에서 한 번만 파싱하는 `&'static Registry` |

기존 `keybinding_search::label` 은 본문을 지우고 `format_categorized_label` 을 호출합니다(구현 1개).

실행 실패 보고: 명령·키맵에서 나온 intent 의 `ShellMutation` 제출 실패와 문서 편집 실패는 `Toasts::ipc_error` 로 보고합니다(`application.rs` 의 intent 루프, `command_errors`). 셸 UI 클릭에서 나온 intent 는 기존대로 `status` 를 씁니다.

### R2 카탈로그와 실행 경로

메타데이터 정본은 `native/taide-native-ui/src/keybinding-commands.json` 하나이고 이제 레지스트리만 읽습니다. 실행 경로와 enabled 조건은 `command-registry.rs` 의 `execution` / `enablement` 가 id 로 부여합니다.

Native 34개

| 명령 | Run |
|---|---|
| settings.open | OpenSettingsTab |
| app.openSettingsFile | OpenSettingsFile |
| keybindings.open | OpenKeybindingsEditor |
| terminal.new | OpenTerminalTab |
| tab.reopenClosed | ReopenClosedTab |
| tab.close | CloseTab (TS 가 항상 비활성이라 `Enablement::Never`. 키맵 `close-tab` 만 실행됩니다) |
| view.toggleSidebar | ToggleSidebar |
| editor.split | Split |
| tab.cycleNext · tab.nextEditor | CycleTab(Next) |
| tab.cyclePrev · tab.previousEditor | CycleTab(Previous) |
| editor.save · monaco.taide.saveFile | SaveActiveTab |
| view.toggleTerminal | ToggleTerminal |
| view.toggleZenMode | ToggleZenMode (보조 창에서는 비활성) |
| editor.focusGroupLeft/Right/Up/Down | FocusGroup(Direction) |
| editor.focusGroup1..9 | FocusGroup(Position) |
| tab.moveToGroupLeft/Right | MoveTabToGroup |
| tab.closeAllInGroup | CloseAllTabs |
| monaco.deleteAllLeft | EditDocument(DeleteAllLeft) |
| monaco.editor.action.outdentLines | EditDocument(OutdentLines) |

Unavailable 178개

- 비 monaco 24개: window.reload, file.quickOpen, editor.find, search.find, search.replace, view.explorer, view.git, view.welcome, terminal.copyImeDebug, app.showPerfSnapshot, tab.moveToNewWindow, tab.moveToMainWindow, sync.uploadNow, sync.downloadNow, cli.connectExternalEditor, cli.installShellCommand, cli.uninstallShellCommand, ai.inlineEdit, git.revertHead, git.createTagOnHead, git.toggleBlame, git.openFileHistory, terminal.runSelectedText, task.runTask
- monaco 154개: native 편집기에 대응 동작이 없는 나머지 전부

키맵 액션 41개 분류: `Run` 31개, 터미널 뷰가 직접 처리하는 2개(terminal-jump-to-previous/next-command, 변경 없음), 실행 대상 없음 8개(quick-open, command-palette, workspace-symbol, find, search, search-replace, explorer, git). font-size-up/down 은 TS 에서도 명령이 없는 키맵 전용 액션이라 `keymap_run` 에만 있습니다.

monaco 연결 판단 근거: `taide-native-editor/src/editing.rs` 의 공개 함수와 `editor_surface.rs` 의 `KeyAction` 을 `MONACO_ACTIONS` 의 id 와 대조했습니다. select all·undo·redo·커서 이동은 monaco 카탈로그에 행이 없고(`cursorUndo`/`cursorRedo` 는 커서 위치 되돌리기라 다른 동작입니다), 1:1 로 대응하는 것은 `deleteAllLeft` = `delete_to_line_start`, `editor.action.outdentLines` = `outdent`(monaco 가 Shift+Tab 과 같은 `CoreEditingCommands.Outdent` 를 실행), `taide.saveFile` = 저장 3개뿐입니다. `editor.action.indentLines` 는 연결하지 않았습니다. monaco 는 빈 선택에서도 줄 들여쓰기를 하는데 native 의 공개 함수 `tab` 은 빈 선택에서 캐럿 위치에 들여쓰기를 넣어 동작이 다르고, 줄 단위 함수 `shift_lines` 는 비공개입니다.

### R3 키맵 일원화와 명령 행 override

- `native/taide-native-app/src/shell_keymap.rs`: `ACTIONS` 삭제. `supports` 는 `keymap_run(..).is_some()`, `action` 은 `intent(keymap_run(..)?, snapshot)` 입니다. 기존 문자열 분기는 `intent(run: Run, snapshot)` 의 typed `match` 로 옮겼고 분기 순서는 그대로입니다. 프로젝트 없이도 소비하던 7개 id 의 문자열 목록은 `runs_without_focused_project` 로 옮겼습니다.
- 신규 `native/taide-native-app/src/command-dispatch.rs` (레지스트리 실행 진입점, `terminal_surface.rs` 를 참조하지 않습니다)
  - `accepts(id, has_focused_shell, context)`: 키를 소비할지 판정. 키맵 액션은 기존 규칙, 명령은 `Command::runnable`
  - `intent(id, context, snapshot)`: 키맵 액션 id 또는 명령 id → `ShellIntent`. 다음 단계의 팔레트가 같은 함수를 씁니다
  - `context(snapshot, scope, active_editor_actions)`, `active_editor_actions(store, focused_view)`, `apply_document_edits(..)`
- `native/taide-native-app/src/terminal_surface.rs` (라우팅 부분만): `application_keymap_decision` 이 `command_dispatch::accepts` 를 쓰고, `Views` 가 프레임마다 받은 `CommandContext` 를 보관합니다(`set_command_context`). `route_keymap` · `capture_window_keymap` · `route_window_keys` 의 시그니처는 그대로입니다.
- `native/taide-native-app/src/application.rs`: 프레임 시작에 `CommandContext` 를 만들어 라우터에 넘기고, 키맵 결과를 `command_dispatch::intent` 로 해석합니다. 터미널 뷰 핸들러의 중복 문자열 목록도 `accepts` 로 바꿨습니다.
- `native/taide-native-ui/src/keymap.rs`: `Keymap::update` 가 keymap id 없는 비 monaco 명령의 override 를 `command_bindings` 로 모으고, `Keymap::decide` 는 앱 키맵이 아무것도 고르지 않은 경우에만 `Decision::Dispatch(명령 id)` 를 냅니다.

TS 와 맞춘 규칙: chord 대기·편집기 유예 중이면 명령이 물러남, 앱 키맵이 `none` 이 아니면 명령이 물러남, chord 가 붙은 명령 행은 이 경로에서 미지정 취급, 같은 키의 명령이 여럿이면 등록 순서 첫 행, 같은 명령의 override 가 여럿이면 첫 유효 항목, macOS 전용 명령은 다른 플랫폼에서 행이 없음, 실행 불가면 키를 소비하지 않음.

기본 키맵 41개의 when·chord·IME·편집기 유예 코드는 건드리지 않았습니다.

### R4 키바인딩 편집기

- `keybinding-catalog.rs` 의 `rows` 가 JSON 을 직접 읽지 않고 `command_registry::registry()` 에서 행을 만듭니다. `rows` 의 시그니처와 결과는 그대로이고 fixture 파리티 테스트(`keybinding_catalog은_전체원본행_…`)가 수정 없이 통과합니다.
- 노출 규칙: TS 편집기에는 필터가 없습니다. `keybindings-editor.tsx:81` 이 `buildKeybindingRows(listRegisteredCommands(), overrides)` 를 그대로 쓰고, 이 파일과 `keybinding-catalog.ts` 어디에서도 `isEnabled` / `isCommandRunnable` 을 읽지 않습니다. monaco 행도 "편집기가 지원하는 액션"으로 거르지 않습니다. 그래서 native 도 전 행을 그대로 노출하며 `keybinding-editor.rs` 는 수정하지 않았습니다. 감사 보고서 3장 1번이 지적한 "실행 불가 행 노출"은 TS 동작과 같으므로 7장의 사용자 결정 사항으로 남겼습니다.
- `keybinding-commands.json` · `keymap-defaults.json` 은 내용을 바꾸지 않았습니다. 파일명도 유지했습니다(`tools/keybinding-catalog/export.ts` 의 `--check` 대상 경로).

### R5 하드코딩 ⌘N

TS 에 대응하는 명령도 전역 키도 없습니다. `DEFAULT_COMMANDS` 와 `APP_KEYMAP` 에 새 untitled 파일 항목이 없고, ⌘N 은 탐색기 트리에 포커스가 있을 때의 "새 파일"(`explorer-shortcuts.ts:43`)뿐이며, untitled 생성은 탭바 "+" 메뉴(`tab-bar-menu-items.ts:44`)로만 합니다. 지시대로 제거하지 않았고 카탈로그에도 넣지 않았습니다. `application.rs` 의 `consume_key(COMMAND, N)` 은 그대로입니다.

### R6 모듈 경계

전역 키맵 라우팅 함수는 `terminal_surface.rs` 의 `Views` 에 그대로 있습니다. 실행 진입점 `command-dispatch.rs` 와 `shell_keymap.rs` 의 비테스트 코드는 `terminal_surface` 를 참조하지 않습니다.

### 그 밖의 배선

- `native/taide-native-ui/src/commands.rs`: `ShellIntent::OpenSettingsFile`, `ShellIntent::EditDocument { tab, edit }` 추가
- `application.rs`: `OpenSettingsFile` 은 창 범위의 프로젝트로 `app_file_views::settings_command` 를 제출하고, 프로젝트가 없으면 `app.openProjectFirst` 를 기존 `OpenSettings` 와 같은 방식으로 알립니다. `EditDocument` 는 `document_edits` 큐에 넣고 다음 프레임 `show_document` 가 편집기 렌더 직전에 적용해 dirty·persistence 처리를 기존 변경 경로와 공유합니다. 그 프레임에 렌더되지 않은 탭의 요청은 버립니다.
- `native/taide-native-app/src/command-registry.rs`: `taide_native_ui::command_registry` 재노출. 테스트 빌드에서 `include!` 되는 ui 소스가 `crate::command_registry` 로 참조하기 때문입니다(`presentation.rs` 와 같은 방식).
- `native/taide-native-app/src/keymap.rs`: 테스트 호환 모듈에 `command_registry` import 1줄
- `native/taide-native-ui/src/lib.rs`, `native/taide-native-app/src/lib.rs`: 모듈 선언

## 2. 범위 밖 수정 1건

`native/taide-native-ui/src/editor_surface.rs`: `NativeEditor::indent_options` 를 `fn` 에서 `pub fn` 으로 바꿨습니다(가시성 한 단어). 명령으로 실행한 `outdentLines` 가 같은 편집기의 Shift+Tab 과 같은 들여쓰기 옵션을 쓰게 하려면 위젯이 쓰는 값을 그대로 받아야 하기 때문입니다. 앱이 따로 계산한 값은 탭 문자 모드에서 탭 크기 기본값이 위젯과 다릅니다.

## 3. TS 근거 경로

| 주제 | 경로 |
|---|---|
| 명령 모델·enabled·실패 보고·제목 포맷 | `src/shared/lib/command-registry.ts` |
| 등록 순서 | `src/app/bootstrap-commands.ts` |
| 기본 명령 43개와 isEnabled | `src/shared/lib/command-catalog.ts` |
| monaco 명령과 지원 액션 게이트 | `src/shared/lib/monaco/monaco-action-commands.ts`, `monaco-actions.ts` |
| 엔티티 명령 | `src/entities/{sync,agent,ai,git,terminal,task}/*.commands.ts` |
| 키맵 엔트리·override 파싱 | `src/shared/lib/keymap/keymap.ts` |
| 명령 행 디스패치 | `src/shared/lib/keymap/command-binding-dispatch.ts`, `keybinding-catalog.ts`(`findRunnableCommandBinding`), `keymap-dispatch.ts`, `src/widgets/command-palette/command-palette.tsx:235-283` |
| 키맵 액션 핸들러 | `src/widgets/editor-area/editor-area.tsx:293-321`, `src/app/providers/keybindings-runtime-provider.tsx:58-61`, `src/widgets/app-shell/app-shell.tsx:143`, `src/widgets/app-shell/use-window-chrome.ts:43` |
| 활성 편집기 범위 | `src/widgets/editor-area/focused-editor-tab.ts:19` (file·appFile·untitled) |
| 키바인딩 편집기 행 | `src/widgets/keybindings-editor/keybindings-editor.tsx:81` |
| monaco 액션 구현 | `node_modules/monaco-editor/esm/vs/editor/contrib/linesOperations/browser/linesOperations.js` (`OutdentLinesAction`, `DeleteAllLeftAction`) |
| ⌘N | `src/features/explorer/explorer-shortcuts.ts:43`, `src/features/tab/tab-bar-menu-items.ts:44` |

## 4. 추가한 테스트

native-ui `command-registry.rs`
- `command_registry는_원본_전체명령의_등록순서_메타데이터와_플랫폼등록을_보존한다`: TS 함수로 생성된 fixture `tests/fixtures/keybinding-catalog.json` 의 default-mac·default-non-mac 행과 id·titleKey·titleDefaultValue·categoryKey·keymapId·defaultBindingLabel·runsViaCommand·source 를 순서까지 비교
- `실행경로는_native동작이_있는_명령과_keymap액션만_typed요청으로_구분한다`: Native 34개의 (id, Run) 전체 목록, 키맵 41개 중 31개, 명령과 키맵의 실행 일치, 편집기 액션 집합
- `enabled_판정은_원본_조건과_실행경로_유무를_함께_적용한다`
- `제목은_카테고리_콜론_제목_형식과_번역_기본값_키_순서를_따른다`

native-ui `keybinding-catalog.rs`
- `keymap의_명령행_dispatch는_catalog의_runnable_command와_같은_행을_앱_keymap_뒤에_고른다`: fixture 6개 시나리오에서 `Keymap::decide` 와 기존 `runnable_command` 이식본의 결과 비교

native-app
- `keymap-tests.rs` `keymap_override는_keymap_id없는_명령행을_앱_keymap_다음_순위와_등록순서로_dispatch한다`
- `shell_keymap.rs` `명령행_override는_window_route에서_실행경로가_있는_명령만_소비한다`
- `command-dispatch.rs` 4개: gate(기존 31개 액션과 무프로젝트 허용 7개를 테스트 데이터로 고정), typed intent 해석과 문맥 생성, 활성 편집기 액션 집합, 문서 편집 요청 적용과 undo 단계

## 5. 실행한 명령과 결과

공통 인자 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` 는 생략해 적었습니다.

| 순서 | 명령 | 결과 |
|---|---|---|
| 1 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib keymap_override는_keymap_id없는` (구현 전) | exit 101. 1 failed, `left: None` `right: Dispatch("settings.open")` |
| 2 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib command_registry` | exit 0. 4 passed, 0.02s |
| 3 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib key` | exit 0. 11 passed, 0.05s |
| 4 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib keymap` | exit 0. 25 passed, 0.10s |
| 5 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib shell` | exit 0. 9 passed, 0.05s |
| 6 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib command_dispatch` | exit 0. 4 passed, 0.01s |
| 7 | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | exit 1. 신규 `command-registry.rs` 만 차이 |
| 8 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | exit 1. 신규 `command-dispatch.rs` 만 차이 |
| 9 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | exit 0 |
| 10 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test keymap-platform` | exit 0. 1 passed |
| 11 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test controller` | exit 0. 2 passed |
| 12 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test workbench` | exit 0. 11 passed, 0.12s |
| 13 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib keybinding` | exit 0. 21 passed, 0.70s |
| 14 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` (V4, 전체 1회) | exit 0. 329 passed, 0 failed, 12.29s |
| 15 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib` | exit 0. 57 passed, 0.40s |
| 16 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | exit 0 |
| 17 | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | exit 0 |
| 18 | `cargo check --manifest-path native/taide-native-app/Cargo.toml` | exit 0. 경고는 vendor `wry` 17건뿐(기존) |
| 19 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | exit 0 |
| 20 | `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml` (계약 외 추가) | exit 0. 경고 8건은 모두 기존 파일(`lib.rs:1`, `settings-code-view.rs`, `settings-view.rs`, `snippet-editor.rs`), 이번에 고친 파일에는 없음 |

- 4·5·6번은 포맷 정리와 `apply_document_edits` 분리 이전 상태에서 실행했습니다. 최종 트리는 14번(전체)이 덮습니다.
- 9번은 포맷 적용 뒤에 실행했고 그 뒤 ui 크레이트 소스는 바뀌지 않았습니다. 19번은 같은 입력의 재확인입니다.

## 6. 실패했다가 고친 내역

1. 1번: 명령 행 override 가 디스패치되지 않음(구현 전 재현). `Keymap::update` / `Keymap::decide` 에 명령 바인딩을 추가한 뒤 4번에서 통과했습니다.
2. 7·8번: 신규 파일 두 개가 rustfmt 형식과 달랐습니다. `cargo fmt --manifest-path …` 로 맞춘 뒤 16·17번에서 통과했습니다. 두 번 모두 신규 파일만 바뀌었습니다.

테스트 선작성에 대해: 구현 전에 실패를 실제로 확인한 것은 1번(명령 행 override)뿐입니다. 레지스트리 테스트 4개와 `command-dispatch.rs` 테스트는 모듈이 없던 상태라 구현과 같은 편집에서 함께 썼고, 실패 상태를 따로 실행해 보지 않았습니다.

`include!` 로만 컴파일되는 `keymap-tests.rs` 는 rustfmt 대상이 아니어서 fmt 검사가 형식을 확인하지 못합니다. 주변 형식에 맞춰 손으로 정리했습니다.

## 7. 남은 위험과 결정 필요 사항

사용자 결정 필요

1. ⌘N: TS 에 없는 전역 단축키가 카탈로그 밖에 남아 있습니다. 재바인딩·충돌 판정 대상이 아닙니다. 유지, 제거, 탐색기 포커스 한정(TS 의 새 파일) 중 선택이 필요합니다.
2. 실행 불가 행 노출: Unavailable 178개(웹 전용 window.reload, terminal.copyImeDebug, app.showPerfSnapshot 포함)가 TS 와 같이 편집 가능한 행으로 보입니다. 바인딩해도 실행되지 않습니다. 레지스트리에 `Execution` 이 있으므로 숨기거나 비활성 표시하는 것은 `rows` 에서 한 곳만 바꾸면 되지만, TS 에 없는 동작이라 구현하지 않았습니다.

미구현

3. monaco 행의 키 override: TS 는 monaco 키바인딩 서비스가 기본 키를 떼고 새 키를 붙입니다. native 편집기의 기본 키는 `editor_surface.rs` 의 `command_action` / `key_action` 에 고정돼 있어 override 가 반영되지 않습니다. Native 로 연결한 3개(deleteAllLeft, outdentLines, taide.saveFile)도 키 재지정은 되지 않고, 다음 단계의 팔레트에서만 실행됩니다.
4. 비동기 실패의 toast: 호스트 응답 `HostReply::Failed` 와 컨트롤러 워커 오류는 어느 명령에서 나왔는지 구분할 수 없어 기존대로 `status` 문자열에 남습니다. `self.submit` 이 `false` 를 돌려주는 경우(호스트 미연결)도 기존처럼 알림이 없습니다. toast API 확장과 status 이전이 비목표여서 손대지 않았습니다.
5. `editor.action.indentLines`, `view.welcome`, `tab.moveToNewWindow` / `tab.moveToMainWindow`: 런타임이나 편집 함수는 일부 있으나 native 실행 경로가 없어 Unavailable 입니다.
6. 명령으로 실행한 문서 편집 뒤의 캐럿 추적 스크롤: 편집이 위젯의 변경 감지 시작점 이전에 적용돼 `moved` 가 거짓입니다. 두 편집 모두 캐럿 줄을 벗어나지 않아 영향은 작다고 봅니다.

실기 확인 필요 (다음 단계에서 팔레트가 붙은 뒤)

- 명령 행에 키를 지정하고(예: settings.open) 터미널 포커스, 편집기 포커스, 프로젝트 없는 상태에서 각각 실행되는지
- `monaco.deleteAllLeft` · `monaco.editor.action.outdentLines` 실행 뒤 탭 dirty 표시, 자동 저장, undo 한 단계
- 읽기 전용 문서에서 두 편집 명령이 비활성으로 보이는지
- 보조 창에서 `view.toggleZenMode` 가 비활성으로 보이는지. 키맵 `toggle-zen-mode` 의 보조 창 동작은 이번에 바꾸지 않았습니다

검증하지 못한 것

- `tools/keybinding-catalog/export.ts --check` / `--check-fixture`: bun 실행이 금지돼 돌리지 않았습니다. `keybinding-commands.json` 과 fixture 는 이번에 바꾸지 않았습니다.
- native-app 대상 clippy 는 실행하지 않았습니다.
- `taide-remote-web` 은 지정된 명령대로 호스트 타깃 `cargo check` 만 했고 wasm 타깃 빌드는 하지 않았습니다.

## 8. 테스트 부채

- [ ] `application.rs` 의 `EditDocument` 큐잉 → 다음 프레임 `show_document` 적용 → dirty/persistence 연결. 재현 조건: 편집기 탭 포커스에서 `monaco.deleteAllLeft` 실행. 생략 이유: `NativeApplication` 전체를 띄우는 테스트 하네스가 없고 실행 수단(팔레트)이 다음 단계입니다. 남은 위험: 큐 적용 자체(`apply_document_edits`)와 intent 해석은 단위 테스트가 있으나 둘을 잇는 앱 배선은 컴파일만 확인했습니다. 필요 시점: 팔레트 단계의 실기 확인.
- [ ] 명령·키맵 intent 의 `controller.submit` 실패가 toast 로 가는 분기. 재현 조건: 컨트롤러 채널이 닫힌 상태에서 키맵 액션 실행. 생략 이유: 같은 하네스 부재. 필요 시점: 종료 경로를 다루는 작업.
