# native lane terminal-agent 묶음 4/5: commands-ui-entry

> 작성일: 2026-11-02 · lane: `terminal-agent` (worktree `TAIDE-lanes/terminal-agent`, 브랜치 `lane/terminal-agent`)
> 기준: `docs/research/2026-10-28-lane-commands-ui-spec.md` 1.4절 W1-A, 2.0절, 2.1절(13·15·22·23), 2.2절(35·40·41), 3절 묶음 W1-A. 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 결정은 추천안, 이 묶음은 결정 2 A안), 구조 `docs/research/2026-10-28-native-lane-seams.md`

## 대상 ID와 판정

- [x] commands-ui-13 `view.explorer` → complete (코드 변경 없음, 팔레트 경로 실제 App 검사 추가)
- [x] commands-ui-15 `view.welcome` → complete (`Run::OpenWelcomeTab`와 intent, 실제 App 검사 2건)
- [x] commands-ui-22 `tab.moveToNewWindow` → complete (`Run::MoveFocusedTab(NewWindow)`, 실제 App 검사)
- [x] commands-ui-23 `tab.moveToMainWindow` → complete (`Run::MoveFocusedTab(MainWindow)`, 보조 창에서만 활성, 실제 App 검사)
- [x] commands-ui-35 `ai.inlineEdit` → complete (코드 변경 없음, editor-78 근거 검사 재실행)
- [x] commands-ui-40 `terminal.runSelectedText` → complete (코드 변경 없음, editor-82 근거 검사 재실행, 결정 2 A안 차이 기록)
- [x] commands-ui-41 `task.runTask` → complete (코드 변경 없음, shell-24·terminal-agent-77 근거 검사 재실행)

## 원본 동작 근거

ID별 원본 근거는 사양 2.1절·2.2절을 따릅니다. 요약과 구현 중 정한 점만 적습니다.

- 13: `src/shared/lib/command-catalog.ts` 87-93, 실행 `requestShowExplorerView('files')`, 활성 조건 없음. native는 이미 `Run::ShowExplorer` → `ShellIntent::ExplorerSearch(ShowExplorer{project, slot})`(보조 창은 `ShowAuxiliaryExplorer`)였고 팔레트 경로 App 검사만 없었습니다.
- 15: `command-catalog.ts` 95 `context.openWelcomeTab()`, `src/widgets/command-palette/command-palette.tsx` 202-214. 프로젝트가 없으면 `toast.info(app.openProjectFirst)`, 있으면 그 창 focused pane에 `kind: welcome`, 제목 상수 `WELCOME_TAB_TITLE`("Welcome", 번역 안 함), `preview: false`. 같은 pane에 Welcome이 이미 있으면 runtime `open_tab`의 dedupe가 새로 만들지 않고 활성화합니다(`crates/taide-layout/src/service.rs` `is_dedupable`).
- 22·23: `command-catalog.ts` 159-179, 받는 쪽 `src/widgets/editor-area/editor-area.tsx` 139-144. 그 창 pane 트리의 focused leaf에 활성 탭이 없으면 무동작·무알림, 있으면 `moveTabToWindow({ tabId: leaf.active, target })`. 23만 `isEnabled: getWindowContext().kind === 'auxiliary'`.
- 35: `src/entities/ai/ai.commands.ts` 6-14, `isEnabled: activeEditorActionIds !== null`. native는 `execution = EditDocument(Completion(TriggerEdit))`, `enablement = ActiveEditor`, 팔레트는 `domains/commands.rs` `show_palette`의 `ai.inlineEdit` 분기.
- 40: `src/entities/terminal/terminal.commands.ts` 5-12(활성 조건 없음), `editor-area.tsx` 257-288. 원본은 편집기가 없을 때 활성 행·무동작이고 native는 `ActiveEditor`라 비활성 행입니다. 결정 2 A안(무동작 명령은 비활성 행)대로 코드는 바꾸지 않고 대응표 비고에 차이를 적었습니다.
- 41: `src/entities/task/task.commands.ts` 7-15, 활성 프로젝트 필요, `requestOpenTaskRunner()`. native는 shell-24·terminal-agent-77과 같은 경로(`Run::OpenTaskRunner` → `Intent::OpenTaskRunner(project)`).

사양과 다르게 한 점

- 사양 2.1의 15·22·23 검사는 UI `NATIVE_COMMANDS`(실행 경로 표)에 행을 더하라고 적었습니다. 그러나 새 `Run` 변형은 `#[cfg(feature = "native-host")]`라 고정 길이 const 표에 cfg 행을 넣을 수 없습니다. 같은 처지의 `OpenTaskRunner`·`Cli` 선례대로 표 비교에서는 제외(`Execution::Native(Run::OpenWelcomeTab | Run::MoveFocusedTab(_)) => None`)하고, 세 명령의 실행 경로·활성 조건을 따로 단언하는 검사 `welcome과_탭_창이동_명령은_native_host에서만_실행되고_메인창_이동은_보조창에서만_활성이다`를 추가했습니다(native-host가 없으면 세 명령 모두 `Unavailable`).
- 사양 2.0.5는 새 App 검사 연결을 `#[cfg(test)]`로 적었지만 이 검사가 쓰는 공용 helper(`project_rail_tests`·`selection_ranges_tests`·`inlay_hints_tests`)가 모두 `#[cfg(all(test, unix))]`라 같은 조건 `#[cfg(all(test, unix))]`로 연결했습니다.
- `WindowMove` enum도 `Run` 변형과 같이 `#[cfg(feature = "native-host")]`로 막았습니다. 동결 `taide-remote-web`(`default-features = false`)의 컴파일 표면이 바뀌지 않습니다.

## 구현

- UI `native/taide-native-ui/src/command-registry.rs`
    - `Run` 끝에 `#[cfg(feature = "native-host")] OpenWelcomeTab`, `MoveFocusedTab(WindowMove)`. 새 `pub enum WindowMove { NewWindow, MainWindow }`(Copy, native-host). `Run`이 `Copy`라 `TabWindowTarget`(Clone만)을 직접 담지 않습니다.
    - `execution()`의 마지막 `match id` 끝(와일드카드 앞)에 `"view.welcome"`, `"tab.moveToNewWindow"`, `"tab.moveToMainWindow"` 세 줄. 활성 조건은 기존 `enablement()` 그대로(15·22 `Always`, 23 `AuxiliaryWindow`).
- APP `native/taide-native-app/src/shell_keymap.rs` `intent`
    - 첫 match 끝: `Run::OpenWelcomeTab if project.is_none()` → `ShellIntent::ShowOpenProjectNotice`(info toast `app.openProjectFirst`).
    - `(root, focused)` 뒤 match 끝: `Run::OpenWelcomeTab` → `ShellIntent::OpenWelcome { project, pane: focused }`. 보조 창은 `window_tree`가 그 창 트리를 주므로 그 창 focused pane입니다.
    - leaf를 찾은 뒤 `MoveTabToGroup` 다음: `Run::MoveFocusedTab(target)` → `ShellIntent::MoveTabToWindow { tab: active?, target: NewAuxiliary | Main }`. 활성 탭이 없으면 `None`(host 명령 없음, 원본과 같이 무알림).
    - 받는 쪽(`domains/shell.rs` `apply_shell_intent`의 `OpenWelcome`·`MoveTabToWindow`, shell-7 경로)은 읽기만 했고 바꾸지 않았습니다(사양 1.3).
- APP `native/taide-native-app/src/domains/commands.rs`: 파일 끝에 `#[cfg(all(test, unix))] #[path = "../application-commands-tests.rs"] mod tests;` 한 줄 묶음(검사 연결만).
- 새 `native/taide-native-app/src/application-commands-tests.rs`: 실제 App 검사 4건(아래).
- 실제 앱 경로: `main.rs` → `NativeApplication::new` → 팔레트 `command_palette::Action::RunCommand(id)` → `domains/commands.rs` `show_palette`가 `commands_state.palette_commands`에 넣음 → `application.rs` 프레임의 `keymap_actions.append(palette_commands)`(overlay viewport) → `command_dispatch::intent_with_editor` → `intent` → `command_run`(registry `runnable`) → `shell_keymap::intent` → `ShellIntent` → `apply_shell_intent`/`apply_explorer_search_intent` → host. 검사는 이 경로를 `run_palette`(메인)와 보조 viewport 키 입력(⌘⇧P·글자·Enter)으로 그대로 탑니다.

## 검증

모든 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/terminal-agent`, 앱 lib는 `--test-threads=2`로 실행했습니다.

- UI lib `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib -- command_registry::tests`: 17 passed(새 검사 1건, 기존 `실행경로는_…`·`enabled_판정은_…` 갱신 포함)
- 앱 lib `-- command_dispatch::tests::명령_ shell_keymap::tests::welcome`: 3 passed
    - `명령_gate는_…`: `view.welcome`·`tab.moveToNewWindow`를 unavailable 목록에서 빼고 무프로젝트 수락 단언, `tab.moveToMainWindow`는 메인 비수락·보조 창 수락
    - `명령_실행은_…`: `view.welcome` → focused pane `OpenWelcome`, 무포커스 → `ShowOpenProjectNotice`, `tab.moveToNewWindow` → 활성 탭 `NewAuxiliary`, 메인 창 `tab.moveToMainWindow` → None
    - 새 `welcome과_탭_창이동_명령은_그_창의_포커스_pane과_활성_탭만_대상으로_한다`: 메인 분할(focused 오른쪽)·보조 창 각각 focused pane·활성 탭, 무포커스, 빈 focused pane
- 앱 lib `-- domains::commands::tests`: 4 passed(새 실제 App 검사)
    - `실제_app_팔레트_view_explorer는_포커스_슬롯만_접힘을_풀고_파일_보기로_바꾼다`(13): 메인 2 slot, 두 sidebar Outline·두 프로젝트 접힘 → 팔레트 `view.explorer` → focused slot만 Files·접힘 해제, 다른 slot은 Outline·접힘 유지
    - `실제_app_팔레트_view_welcome은_포커스_pane에_한번만_열고_재실행은_활성화한다`(15): 분할 레이아웃 focused 오른쪽 pane에 `TabKind::Welcome`·제목 "Welcome"·preview 아님·활성, 왼쪽 pane 불변. 다른 탭 활성 후 재실행 → 탭 수 그대로 Welcome 재활성
    - `실제_app_프로젝트가_없으면_팔레트_view_welcome은_프로젝트_먼저_열기_안내만_보인다`(15): info toast `app.openProjectFirst` 1건, 레이아웃 수 불변
    - `실제_app_팔레트_탭_창이동은_새창과_메인으로_옮기고_빈_pane은_무시한다`(22·23): 메인 팔레트 `tab.moveToNewWindow` → slot 1 보조 창(label `editor-1`) 채택·탭 이동, 빈 focused pane에서 재실행 → layout revision·창 수 불변(host 명령 없음), 보조 창 viewport 팔레트 `tab.moveToMainWindow`(overlay = 그 창) → 메인 focused pane 복귀·활성, 빈 보조 창 닫힘·label 해제
- 근거 재실행(앱 lib) `-- ai_edit_tests::실제_ai_edit의_팔레트는 context_menu_tests::실제_터미널_실행의_팔레트와 shell_dialogs_tests::실제_app_태스크_실행은_팔레트 terminal_agent_tests::실제_app_보조창_task_runner shell_dialogs_tests::실제_app_탐색기_키는`: 5 passed(35, 40, 41 메인·보조 창, 13 키 경로)
- 회귀(앱 lib) `-- shell_keymap::tests command_dispatch::tests auxiliary_window_tests::실제_app_탭_메뉴는`: 18 passed
- 형식: `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check`, `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` 통과(첫 실행에서 나온 줄바꿈 차이를 Edit로 반영하고 긴 검사 이름 2개를 줄인 뒤 재실행)
- 이름·형식 수정 뒤 재실행(앱 lib) `-- domains::commands::tests shell_keymap::tests::welcome command_dispatch::tests::명령_`: 7 passed
- UI lib 전체 `--features native-host,inspection --lib`: 169 passed. UI 하네스 `--test workbench` 전체: 45 passed
- `cargo check --quiet --all-targets`: UI(`--features native-host,inspection`)·앱 모두 오류 없음. 남은 경고(`tests/workbench.rs` 1816 `unused mut`, 앱 `editor-code-actions.rs` `is_applying`, `zen.rs` 543, `application-inlay-hints-tests.rs` `fixture_with_emmet_peek`, vendor `wry-preview`)는 이 묶음이 고치지 않은 파일의 기존 경고입니다
- 동결 remote-web `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib`: 통과(기존 dead_code 경고만)
- `Cargo.toml`·`Cargo.lock` 변경 없음(새 의존성 없음)

## 대응표

`docs/quality-assurance/2026-10-09-native-function-matrix.md`에서 commands-ui-13·15·22·23·35·40·41 일곱 행만 `unwired` → `complete`로 바꾸고 비고에 실행 경로·검사·QA 경로를 적었습니다. 상단 합계 표는 바꾸지 않았습니다. terminal-agent-77은 앞 묶음(commands-run)이 이미 같은 근거로 complete로 갱신해 확인만 했습니다.

## 남은 범위

- 22·23 실제 OS 창 focus 이동 실기는 사양 4.2-5(배치 31) 허락 항목이라 하지 않았습니다.
- 40의 "편집기 없을 때 원본 활성 행·무동작 vs native 비활성 행" 차이는 결정 2 A안으로 유지합니다.
- 실패 검사 선행: 구현과 검사를 같은 편집 묶음에서 작성해 구현 전 빨간 실행은 따로 하지 않았습니다. 바뀐 기존 단언(UI `tab.moveToMainWindow` `Unavailable`, 앱 `view.welcome`·`tab.moveToNewWindow` unavailable 목록)이 이전 동작의 기대값이었습니다.
