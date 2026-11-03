# 13. 남은 작업: terminal-agent 영역

> 작성일 2026-11-04 · 대상 영역 `terminal-agent`(터미널 표면·터미널 코어·agent 표시·IDE 도구·CLI 명령·hooks·알림 설정 일부)
> 기준 코드: 메인 작업 트리 `/Users/hyunseokbyun/development/TAIDE`(브랜치 `to_rust_native`, HEAD `0c52b438`. terminal-agent lane 병합 `446964b7`, shell lane 병합 `4fa4beff`, settings lane 병합 `3864e1db` 포함, 미커밋 변경은 `docs/handoff/`뿐).
> Git 영역은 병합 중인 worktree `/Users/hyunseokbyun/development/TAIDE-lanes/git`(브랜치 `lane/git`, 마지막 커밋 `50892b59`, 지금 `to_rust_native`를 받아들이는 병합을 다른 작업자가 해결 중)을 읽었습니다. 그 worktree의 `git status`에서 `UU` 표시 파일 8개를 grep했을 때 충돌 표식(`<<<<<<<`)은 남아 있지 않았고, terminal-agent 행 5개의 기능표 문구는 메인과 글자 단위로 같았습니다.
> 이 문서는 정적 읽기로만 작성했습니다(cargo·앱 실행 없음). 줄 번호는 위 기준이며, git lane 병합 뒤 `application.rs`는 약 14줄 밀립니다(예: `frame_ui` 1383 → 1397). 항상 함께 적은 심볼 이름으로 다시 찾으십시오.
> 2026-11-03판을 다시 검증해 바꾼 점: 19의 2단계 설계를 shell-5의 vendored eframe 설계(`10-remaining-shell.md` 3.2 (d) 2.1)와 맞춤, 19의 미검증 위험(AppKit 동작)과 완료 조건 명시, 64를 `15-remaining-settings.md` settings-22 설계와 맞춤, 68의 git 행 위치·ClaudeDiff revert 표시(monaco 0.56은 화살표가 아니라 gutter 도구 막대)와 병합 진행 상태 갱신.

---

## 0. 읽는 법과 전제 지식

### 0.1 기준 문서

| 문서 | 쓰는 이유 |
| --- | --- |
| `docs/quality-assurance/2026-10-09-native-function-matrix.md` | 행 목록과 상태의 단일 출처. 머리말 5행의 상태 정의를 따릅니다 |
| `docs/research/2026-10-28-lane-terminal-agent-spec.md` | 이 영역 lane 사양. 1.2절 ID 분류, 2.3절(19), 4.1절 결정 1~7, 4.2절 허락 항목 |
| `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` | 사양 결정 채택(4.1 전부 추천안), 중복 범위 소유, CLI 취소 문구 규칙(13행), 보류 표 |
| `docs/quality-assurance/2026-11-02-native-lane-terminal-agent-{terminal-core,ide-agent-display,commands-run,commands-ui-entry,commands-ui-evidence}.md` | lane 구현 근거와 리뷰 수정 기록 |
| `docs/quality-assurance/2026-11-02-native-lane-settings-s1-interface-performance.md` | settings-16·17·21·22·82 구현 근거(이 영역 55·56·64가 기대는 행) |
| `/Users/hyunseokbyun/development/TAIDE-lanes/git/docs/quality-assurance/2026-11-02-native-lane-git-g7-commit-claude-diff.md` | git-49~52 구현 근거(이 영역 68이 기대는 행) |
| 같은 폴더의 `10-remaining-shell.md`(shell-5), `14-remaining-git.md`(git-49~52), `15-remaining-settings.md`(settings-22), `30-pending-permissions.md`(P01~P26) | 다른 영역이 소유한 선행 작업의 상세 절차. 이 문서는 그 절차를 반복하지 않고 가리킵니다 |
| `docs/memory/2026-10-28-native-lane-operations.md`, `docs/memory/2026-10-07-native-transition-operations.md` 5·6절 | 운영 규칙과 반복된 함정 |
| `docs/utils/2026-11-03-native-merge-gate.sh` | 병합 게이트 명령 |

### 0.2 경로 약어 (이 문서 안에서만)

| 약어 | 실제 경로 |
| --- | --- |
| `APP` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app` |
| `UI` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui` |
| `TERM` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-terminal` |
| `CR` | `/Users/hyunseokbyun/development/TAIDE/crates` |
| `TS` | `/Users/hyunseokbyun/development/TAIDE/src`(원본 React 앱) |
| `TAURI` | `/Users/hyunseokbyun/development/TAIDE/src-tauri/src`(원본 Rust 백엔드) |
| `NM` | `/Users/hyunseokbyun/development/TAIDE/node_modules` |
| `GITWT` | `/Users/hyunseokbyun/development/TAIDE-lanes/git`(병합 중 worktree) |
| `EFRAME` | `/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/vendor/eframe`(vendored eframe 0.36.2) |
| `EGUI` | `APP/vendor/egui-input`(vendored egui, `APP/Cargo.toml` 111-112 `[patch.crates-io] egui = { path = "vendor/egui-input" }`) |
| `REG` | `/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f`(이 기계의 `CARGO_HOME`은 `~/.cargo`가 아니라 `/Users/hyunseokbyun/development/rust/cargo`입니다. winit·egui-winit 소스가 여기 있습니다) |
| `TARGET` | `/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`(병합 게이트와 같은 target. lane worktree에서는 `/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`) |

### 0.3 egui 0.36·eframe 최소 지식 (이 저장소에서 쓰는 형태)

- egui는 즉시 모드입니다. 화면 상태는 egui가 아니라 앱 구조체 `NativeApplication`에 둡니다. 매 프레임 `eframe::App::logic(&mut self, ctx, frame)`(`EFRAME/src/epi.rs` 167) 다음 `eframe::App::ui(&mut self, ui, frame)`(같은 파일 182)이 불립니다. 구현은 `APP/src/application.rs` `impl eframe::App for NativeApplication`(1332-1371)입니다. `logic`이 먼저 불리므로 그 안의 `context.input(..)`은 직전 프레임 입력을 읽습니다.
- 입력은 `egui::RawInput.events: Vec<egui::Event>`입니다. 키는 `Event::Key { key, physical_key, pressed, repeat, modifiers }`, 글자는 `Event::Text(String)`, IME는 `Event::Ime(ImeEvent::Preedit { text, active_range_chars } | ImeEvent::Commit(text) | ..)`입니다. macOS에서 egui-winit는 winit `KeyboardInput` 하나를 `Event::Key`(`REG/egui-winit-0.36.2/src/lib.rs` 1038) 다음 `Event::Text`(같은 파일 1064) 순서로 넣습니다. ctrl·cmd가 눌리면 `Text`는 넣지 않고(1061-1065), alt는 막지 않습니다.
- `eframe::App::raw_input_hook(ctx, &mut RawInput)`(`EFRAME/src/epi.rs` 279)은 egui가 입력을 처리하기 전에 이벤트를 고치는 hook입니다. 이 앱은 메뉴 주입과 터미널 raw 입력에 씁니다(`APP/src/application.rs` 1333-1354). ROOT viewport에서만 불리고 보조 창(immediate viewport)에서는 불리지 않습니다(`EFRAME/src/native/wgpu_integration.rs` `render_immediate_viewport` 1139-, shell-5 남은 범위).
- `eframe::Frame::winit_window()`(`EFRAME/src/epi.rs` 776)는 `Option<&Arc<winit::window::Window>>`를 돌려줍니다. 그 값은 통합 객체를 만들 때 루트 창으로 한 번 정해지고(`EFRAME/src/native/epi_integration.rs` 202 `window: Some(Arc::clone(window))`) 바뀌지 않습니다. 그래서 보조 창 pass 안에서도 루트 창입니다. 검사용 `eframe::Frame::_new_kittest()`(같은 파일 730)에서는 `None`입니다.
- 창: 메인 창은 `egui::ViewportId::ROOT`입니다. 보조 창은 `context.show_viewport_immediate(..)`로 그리는 immediate viewport이고(`APP/src/domains/shell.rs` `show_auxiliary_windows` 1503-1560), 같은 `NativeApplication`과 같은 `frame`으로 `show_window`를 다시 부릅니다. immediate pass가 끝나면 egui가 memory의 viewport를 부모(ROOT)로 되돌립니다(`EGUI/src/context.rs` 3083-3084 `self.memory.set_viewport_id(viewport_id)`).
- 위젯 키보드 포커스는 viewport별 `memory.focused()`(`EGUI/src/memory/mod.rs` 921)입니다.

### 0.4 Rust 2024·저장소 관례

- native crate는 `edition = "2024"`, `rust-version = "1.95"`입니다(`APP/Cargo.toml` 4-5). let-chain(`if let A = b && cond && let Some(c) = d { .. }`)을 씁니다. 예: `APP/src/terminal_surface.rs` `input_event` 4276-4285.
- winit는 앱의 직접 의존성입니다(`APP/Cargo.toml` 57 `winit = { version = "=0.30.13", default-features = false }`, 결정 근거 `docs/acknowledge/2026-10-26-native-autonomous-continuation.md` 단계 E 결정 "shell-3 모니터 목록 A안"). macOS 확장 `winit::platform::macos`는 feature 없이 macOS에서 열립니다(`REG/winit-0.30.13/src/platform/mod.rs` 9-10 `#[cfg(any(macos_platform, docsrs))] pub mod macos;`). lockfile의 winit은 0.30.13 하나뿐입니다(`APP/Cargo.lock` 8326-8328).
- 검사 모듈은 `#[cfg(..)] #[path = "파일.rs"] mod 이름;`으로 붙습니다. cargo 필터는 파일 이름이 아니라 모듈 경로입니다. 예: `APP/src/application.rs` 3001-3003 `#[cfg(all(test, target_os = "macos"))] #[path = "application-terminal-agent-tests.rs"] mod terminal_agent_tests;` → 필터 `application::terminal_agent_tests::`.
- 검사 이름은 한국어입니다. 코드 주석은 쓰지 않습니다(공개 API의 영어 `///`만 예외. 검사 helper의 `///`는 lane 리뷰에서 지웠습니다, commands-run QA 수정 절). 매직넘버는 의미 있는 `const`로 둡니다.
- 공용 파일(`application.rs`, `host.rs`의 enum 루트, `lib.rs` 모듈 목록, `UI/src/commands.rs`의 `ShellIntent` 루트, locale 카탈로그)은 영역 블록 끝에 추가만 합니다(`docs/memory/2026-10-28-native-lane-operations.md` 2.2). 다른 영역 파일을 고치면 그 영역의 기존 검사를 함께 돌립니다.
- 원본 TS 화면·상태·상호작용과 같게 만들고, 새 기능·디자인은 넣지 않으며, 원본 결함과 내부 수치는 재현하지 않습니다(lane 사양 0절, workflow 규칙 `docs/utils/2026-11-02-native-lane-workflow.js` 35행).
- 작업자 금지(같은 파일 45-52행): GUI 실행, 사용자 실제 데이터·OS 설정·클립보드·Keychain·휴지통(`APP/tests/explorer-delete.rs` 실행 금지), 실제 OS 알림, 네트워크, 합성 OS 입력, git 상태 변경, `#[allow]`·기대값 조작 같은 우회.

---

## 1. 요약

### 1.1 남은 행과 상태별 수

행 목록은 기능표를 직접 읽어 만들었습니다. 다시 만들 때는 아래 명령을 씁니다(상태 열에 공백 패딩이 있어 `grep -v "| complete |"` 방식은 틀립니다).

```
awk -F'|' '/^\| terminal-agent-/ {s=$5; gsub(/ /,"",s); if (s!="complete" && s!="not-applicable" && s!="frozen") print NR": "$2"|"s}' /Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-10-09-native-function-matrix.md
```

결과(2026-11-04, 메인 HEAD `0c52b438`): terminal-agent 78행 = complete 69, not-applicable 4(7·26·45·46), 남은 5행.

| 상태 | 수 | ID(기능표 줄) |
| --- | --: | --- |
| partial | 2 | terminal-agent-19(312), terminal-agent-56(349) |
| unwired | 2 | terminal-agent-55(348), terminal-agent-68(361) |
| missing | 1 | terminal-agent-64(357) |
| 합계 | 5 | |

주의: 기능표 머리말의 영역 집계 표(15행 `terminal-agent | 78 | 44 | 12 | 13 | 7 | 2 | 0`)와 9행 문장은 2026-10-14 값이고 갱신되지 않았습니다. 수는 위 명령으로 셉니다.

### 1.2 행별 한눈 표 (실제 코드로 다시 확인한 결과)

| ID | 기능 | 표 상태 | 실제로 남은 것 | 코드 소유 | 선행·막는 요소 | 작업 뒤 판정 |
| --- | --- | --- | --- | --- | --- | --- |
| 19 | Option-as-Meta | partial | macOS dead key(US 배열 Option+E/U/I/N/\`)가 `Ime::Preedit`로만 와서 ESC 접두가 나가지 않음. winit `OptionAsAlt` 전환 경로가 저장소에 없음. 보조 창은 창 핸들 접근이 없음 | terminal-agent(+ shell의 vendored eframe 변경) | 2단계는 shell-5 vendored eframe 변경 선행. dead key 효과는 실기(P23)로만 확인 가능 | 1·2단계 + 실기 확인 → complete. 실기 전에는 partial |
| 55 | hooks 설치·제거·상태 UI(설정) | unwired | 코드 없음. settings-17(complete)이 같은 기능을 구현·검사했고 이 행 비고만 낡음 | settings | 없음 | complete |
| 56 | CLI 설치 상태·설치·제거·외부 에디터 연결 명령 | partial | 비고의 남은 범위(설정 CLI 행 = settings-16)가 complete. 행 갱신만 남음. 같은 정책이 두 군데 구현돼 있어 정리를 권장(선택) | terminal-agent(+ settings) | 정리는 메인 결정 3건(3.3 (g)) | complete |
| 64 | 설정 화면 테스트 알림·시스템 설정 열기 | missing | settings-21 complete, settings-22 partial(전달 성공·플랫폼 오류의 실제 App 검사 없음). 메인에는 플랫폼 주입(`Overrides.platform`)이 있어 검사를 쓸 수 있음 | settings | settings-22 검사(15번 문서) | complete |
| 68 | openDiff → Claude diff 탭 | unwired | 메인에는 ClaudeDiff 본문이 자리표시(`show_unavailable_tab`)이고 `IdeDiffRequested` 소비자가 없음. git lane이 git-49~52로 구현했고 병합 중 | git | git lane 병합, git-50(수정 쪽 Revert Block)·git-52(보조 창 App 검사) | 병합 직후 partial, git-50·52 complete 뒤 complete |

### 1.3 현재 구현 위치

| 묶음 | 파일 | 핵심 심볼(줄) |
| --- | --- | --- |
| 영역 진입(호스트 명령·응답·intent·tick) | `APP/src/domains/terminal_agent.rs`(694줄) | `AGENT_CLI_TASK`(19), `pub enum Command`(21, `Cli(crate::agent_cli::Action)` 64), `pub enum Reply`(67, `Cli { action, result }` 94), `pub(crate) struct State`(100), `dispatch`(112, `Command::Cli` arm 348-371), `impl NativeApplication { apply_terminal_agent_reply(376), apply_terminal_agent_intent(449), finish_agent_cli(467), has_terminal_agent_session(505), tick_terminal_agent_fonts(509), tick_terminal_agent_cursors(540), tick_terminal_agent_live_sessions(553), tick_terminal_agent_views(568), tick_terminal_agent_ide_selection(592), finish_terminal_agent_frame(621) }`, `impl TerminalAgentSurfaces for AppSurfaces`(632), `AppSurfaces::show_terminal_tab`(646) |
| 터미널 표면 | `APP/src/terminal_surface.rs`(8,985줄) | `Appearance`(96, `command_colors` 124), `View`(269, `preedit` 290, `meta_text_pending` 291), `Views`(1250), `Request`(1265), `Views::raw_input`(2323), `Views::finish_frame`(2578), `Views::show`(2990), `Views::has_keyboard_focus`(3283), `Views::show_with_keymap`(3306, 포커스 요청 3342-3344), `meta_character`(4222), `input_event`(4261), `buffer_starting_input`(4342), `handle_input`(4368), `key_input`(4562), IME 출력(5226 `if focused` 안 5241), 검사 모듈 `mod tests`(5269) |
| 세션 허브·탭 | `APP/src/terminal_host.rs`, `APP/src/terminal_tabs.rs` | `Hub`(`get`, `configure_cursors`, `configure_history`, `configure_command_colors`), `Tabs`(`hub`, `attach`, `restart`, `menu`) |
| 터미널 코어 | `TERM/src/lib.rs`, `TERM/src/session.rs`, `TERM/src/input.rs` | `TerminalCore`, `SharedTerminal`, `input::character`(alt면 ESC 접두) |
| CLI 명령 백엔드 | `APP/src/agent-cli.rs`(모듈 `crate::agent_cli`, `APP/src/lib.rs` 1-2) | `OSASCRIPT_PROGRAM = "osascript"`(9), `Action`(13), `Osascript` 타입(31), `Ports`(33), `Ports::system`(40), `status`(49), `install_target`(53), `install`(62), `uninstall`(80), `run`(110), `run_osascript`(125), 검사 연결 `#[cfg(all(test, unix))] mod tests`(139-141) |
| CLI 공유 정책(settings lane이 추가) | `CR/taide-runtime/src/agent_host.rs` | `cli_install_status`(102), `AdminPrompt`(116), `CliCommandOutcome { status, is_cancelled }`(123), `cli_install_target`(136), `cli_install`(142), `cli_uninstall`(155), `run_admin_osascript`(181) |
| 설정 화면 CLI·hooks·알림(settings 소유) | `APP/src/settings-ports.rs`, `APP/src/domains/settings.rs`, `UI/src/settings-sections-data.rs`, `UI/src/settings-sections-view.rs` | `OSASCRIPT_PROGRAM = "/usr/bin/osascript"`(14), `CliCommand`(120), `CliPorts`(125), `impl Default for CliPorts`(130), `shell_command`(144), `open_notification_settings`(157), `Ports`(183); `execute_sections`(470)의 `Query::CliStatus`(730)·`Action::CliInstall`(742)·`Query::HooksStatus`(749)·`Action::Hooks`(763)·`Action::OpenNotificationSettings`(799)·`Action::SendTestNotification`(808), `change_shell_command`(846), `accept_sections_reply`의 `Notice` → toast(1108-1138); UI data `STALE_TIME = 60초`(22), `HookTarget`(54), `suppression_label`(198), `cli` 슬롯(568), `send_test_notification`(664), `Reply::TestNotification` → `Notice`(1168-1177), `finish_cli`(1209); UI view `interface`(501), `cli_row`(605), `hooks_list`(688), `hook_project_row`(738), `hook_user_row`(828, CLI 누락 경고 865), `notifications`(891) |
| IDE 도구 | `APP/src/ide-tools.rs` | `IDE_DIFF_TIMEOUT = 600_000ms`(27), `open_diff`(174-226, `AppEvent::IdeDiffRequested` 발행 209), `close_tab`·`closeAllDiffTabs`(`IdeCloseTabRequested` 337·413) |
| ClaudeDiff 탭 닫기 응답 | `APP/src/tabs.rs` | 136-140 `TabKind::ClaudeDiff` 닫힘 → `services.ide.take_pending_diff` → `IdeDiffOutcome::TabClosed` |
| ClaudeDiff 표면(메인) | `APP/src/domains/git.rs` | `AppSurfaces::show_claude_diff_tab(ui, tab)`(141) = `show_unavailable_tab` 자리표시. 호출 `APP/src/domains/editor.rs` 5397-5398 |
| ClaudeDiff 표면(git lane) | `GITWT/native/taide-native-app/src/git-claude-diff.rs`, `GITWT/.../domains/git.rs`, `GITWT/.../provider-events.rs`, `GITWT/.../domains/shell.rs` | `RECOMPUTE_DELAY`(12), `FALLBACK_LANGUAGE_ID`(13), `BAR_HEIGHT = 32.0`(14), `Event { Requested, Closed }`(27), `State`(57), `bar`(249); `Command::ResolveIdeDiff`, `Reply::IdeDiffResolved`, `accept_claude_diff_event`, `tick_claude_diffs`, `show_claude_diff_tab(ui, project, pane, tab, intents)`; `provider_events::Event::IdeDiff`(37, 기록 90-107); `consume_provider_events`의 `Event::IdeDiff(event) => self.accept_claude_diff_event(event)`(1161) |
| UI intent·surface | `UI/src/intents/terminal_agent.rs`, `UI/src/surfaces/terminal_agent.rs` | `CliAction { ConnectExternalEditor, Install, Uninstall }`, `Intent::{NewTerminal, #[cfg(feature = "native-host")] Cli}`, `TerminalAgentSurfaces { project_agents, is_agent_status_badge_enabled }` |
| 명령 레지스트리·키맵 | `UI/src/command-registry.rs`, `APP/src/shell_keymap.rs` | `Run::Cli`(193), `execution`의 `cli.*` 3종(463-471), `Run::Cli(action) => ShellIntent::TerminalAgent(Intent::Cli(action))`(`shell_keymap.rs` 142-146) |

### 1.4 다른 영역과의 의존·소유 경계

- terminal-agent가 제품 코드를 쓰는 행: 19(전부. 2단계의 vendored eframe 함수는 shell-5가 만들고 이 영역은 호출만), 56(정리를 하기로 결정하면 `agent-cli.rs`와 그 검사, 그리고 settings 파일 `settings-ports.rs`).
- terminal-agent가 행만 갱신하는 행(lane 사양 4.1 결정 1 A안, `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` "중복 범위 소유"):
    - 55 ← settings-17(settings 영역).
    - 64 ← settings-21·22·82(settings 영역). 22의 남은 검사는 settings 검사 파일에 씁니다(15번 문서 settings-22 절).
    - 68 ← git-49~52(git 영역, "IDE diff·diff 설정 소비는 git lane"). 69(`ideAutoOpenDiff`, 지금 complete)의 비고도 git-49 근거로 바꿔야 합니다(14번 문서 3.7 (f)).
- 공용 파일 경계
    - 19 1단계는 `application.rs`의 생성자 리터럴에 필드 1줄, `frame_ui`에 호출 1줄을 추가합니다(추가만).
    - 19 2단계는 shell 영역 파일 `APP/src/domains/shell.rs`의 `show_auxiliary_windows` 콜백과 `close_auxiliary_window`에 한 줄씩 추가합니다. vendored eframe(`EFRAME`)의 창 접근 함수는 shell-5가 만듭니다(10번 문서 3.2 (d) 2.1 `eframe::immediate_viewport_window()`). 같은 함수를 두 번 설계하지 않습니다.
    - 56 정리는 settings 파일(`settings-ports.rs`)도 바꿀 수 있으므로 settings 검사(`application::domains::settings::settings_interface_tests::실제_cli_행…`)를 함께 돌립니다.
- 공유 crate(`CR/taide-*`)는 이 영역 남은 작업에서 바꾸지 않습니다. 56 정리는 이미 있는 `CR/taide-runtime/src/agent_host.rs` 함수를 호출하는 쪽으로만 바꿉니다.

### 1.5 권장 구현 순서

1. 55: 근거 재실행과 기능표 행 갱신(막는 것 없음, 코드 없음).
2. 56: 근거 재실행과 행 갱신(필수). CLI 정책 이중 구현 정리는 메인 결정 뒤 별도 커밋(선택).
3. 64: settings-22가 아직 partial이면 15번 문서 settings-22 절의 검사를 먼저 끝내고(settings 소유), 같은 근거로 64를 갱신합니다.
4. 68: git lane 병합 커밋이 `to_rust_native`에 들어온 뒤 확인·행 갱신(partial). git-50·52가 끝나면 complete.
5. 19 1단계(메인 창 `OptionAsAlt`): 이 영역에서 제품 코드가 바뀌는 유일한 작업이므로 1~4와 섞지 않고 따로 커밋합니다.
6. 19 2단계(보조 창): shell-5의 `eframe::immediate_viewport_window()`가 병합된 뒤.
7. 19 실기(P23, 사용자 허락): dead key 동작 확인 뒤 complete 판정.

---

## 2. 영역 공통 지식

### 2.1 상태가 있는 곳

- `NativeApplication.terminal_agent_state: domains::terminal_agent::State`(`APP/src/application.rs` 150, 생성자 리터럴 541-551). 필드(`APP/src/domains/terminal_agent.rs` 100-110):

```rust
pub(crate) struct State {
    pub(crate) terminals: Arc<crate::terminal_tabs::Tabs>,
    pub(crate) terminal_views: crate::terminal_surface::Views,
    pub(crate) terminal_appearance: crate::terminal_surface::Appearance,
    pub(crate) terminal_fonts: crate::terminal_fonts::Loader,
    pub(crate) editor_ide_selection: crate::editor_ide_selection::State,
    pub(crate) status_ide_appearance: crate::status_ide::Appearance,
    pub(crate) status_ide_icons: crate::status_ide::Icons,
    pub(crate) command_colors: Option<taide_native_terminal::CommandColors>,
    pub(crate) cli_status: Option<CliInstallStatus>,
}
```

- 세션은 `terminals.hub()`(`terminal_host::Hub`)가 소유합니다. 원격 spawn 세션도 같은 Hub에 들어갑니다(terminal-core QA 수정 절 "11·49").
- 화면별 터미널 상태(선택, 스크롤, IME preedit, meta 대기 플래그, 포커스 프레임)는 `Views.views: HashMap<(egui::ViewportId, PaneId, TabId), View>`(`terminal_surface.rs` 1252)에 있습니다. 같은 탭이라도 창(viewport)마다 다른 `View`입니다.
- agent 목록은 `services.agents`(`AgentStore`)이고, 화면은 매 프레임 `TerminalAgentSurfaces::project_agents`(`terminal_agent.rs` 633)로 직접 읽습니다.
- 설정 값은 `services.state.settings`(RwLock)입니다. 검사는 `application.services.state.settings.write().필드 = 값;`으로 바꿉니다.
- CLI 상태는 두 곳에 있습니다. 명령 응답이 쓰는 `terminal_agent_state.cli_status`(제품 코드는 `finish_agent_cli` 490에서 쓰기만 하고 읽지 않음. 읽는 곳은 검사 `application-terminal-agent-tests.rs` 1495·1498·1598·1626뿐)와 설정 화면의 `sections::Data.cli`(`UI/src/settings-sections-data.rs` 568, 60초 stale 재조회). 둘은 서로 갱신하지 않습니다. 원본도 명령(`TS/entities/agent/agent.commands.ts`)은 Query 캐시를 바꾸지 않으므로 같은 동작입니다.

### 2.2 호스트 명령·응답·intent·surface 흐름

1. UI 표면·키맵·팔레트가 `ShellIntent::TerminalAgent(intents::terminal_agent::Intent::…)`를 만듭니다(`UI/src/commands.rs` 105).
2. intent 루프가 `ShellIntent::TerminalAgent(intent) => self.apply_terminal_agent_intent(intent)`로 보냅니다(`APP/src/application.rs` 2217 → `terminal_agent.rs` 449).
3. `self.submit(HostCommand::TerminalAgent(Command::…))`가 host 큐에 넣습니다(`APP/src/host.rs` 21, 용량 `HOST_COMMAND_CAPACITY = 64`(10)). 제출이 실패하면 `submit`이 `false`를 돌려주고 호출자가 실패 처리를 합니다(예: 460-462).
4. host worker가 `HostCommand::TerminalAgent(command) => domains::terminal_agent::dispatch(..)`(`host.rs` 342)로 처리하고 `Option<HostReply>`를 돌려줍니다.
5. 다음 프레임 `poll`이 `HostReply::TerminalAgent(reply) => self.apply_terminal_agent_reply(context, reply)`(`application.rs` 764)로 반영합니다.

host worker는 명령을 한 번에 하나씩 처리합니다. 사람이 응답할 때까지 걸리는 작업(관리자 암호 대화상자)을 worker 안에서 기다리면 모든 영역 명령이 멈춥니다. 그런 작업은 `services.tasks.spawn_transient(이름, async { .. tasks.run_blocking_result(이름, move || ..).await .. sender.send(reply).await; repaint(); })`로 떼어 내고 `dispatch`는 `None`을 돌려줍니다. spawn이 거절되면(종료 중) 즉시 오류 응답을 돌려줍니다. 실제 코드(`terminal_agent.rs` 348-371):

```rust
Command::Cli(action) => {
    let ports = integrations.agent_cli.clone();
    let tasks = services.tasks.clone();
    let sender = worker.sender.clone();
    let repaint = worker.repaint.clone();
    if services.tasks.spawn_transient(AGENT_CLI_TASK, async move {
        let result = tasks
            .run_blocking_result(AGENT_CLI_TASK, move || {
                crate::agent_cli::run(&ports, action)
            })
            .await;
        let reply = HostReply::TerminalAgent(Reply::Cli { action, result });
        if sender.send(reply).await.is_ok() {
            repaint();
        }
    }) {
        None
    } else {
        Some(HostReply::TerminalAgent(Reply::Cli {
            action,
            result: Err(crate::host::stopped()),
        }))
    }
}
```

- `run_nonabortable_result`는 async future를 받으므로 blocking 파일 시스템·프로세스 대기에는 쓰지 않습니다(commands-run QA "56 host 작업 실행 방식"). settings 영역도 같은 이유로 `Command::Sections`를 `spawn_transient("native-settings-sections", ..)`로 떼어 냅니다(`APP/src/domains/settings.rs` 430-466).
- 검사용 포트 주입: `HostIntegrations`(`host.rs` 51-56)에 포트 필드를 두고, 검사에서는 실행 중인 App의 bridge를 교체합니다. 예: `HostBridge::connect_with_agent_cli`(`host.rs` 193-, `#[cfg(all(test, target_os = "macos"))]`), 사용 예 `connect_cli`(`application-terminal-agent-tests.rs` 1429-1474).
- 생성 시 포트 교체: `crate::application::Overrides { secret_store, platform }`(`application.rs` 76-79)와 `NativeApplication::new_with_overrides`(313)가 `services.platform`을 바꿉니다(348-356).

### 2.3 한 프레임의 흐름 (이 영역이 끼는 곳)

1. `raw_input_hook`(`application.rs` 1333): 메뉴 이벤트 주입 → `terminal_views.raw_input`(ROOT만).
2. `logic`(1356) → `background_tick`(1071): 호스트 응답 반영, `tick_terminal_agent_fonts`(1092), `tick_terminal_agent_cursors`(1097), `tick_terminal_agent_live_sessions`(1098).
3. `ui`(1361) → `frame_ui`(1383): `tick_terminal_agent_views`(1406, 사라진 탭의 `View` 정리) → `show_window(ui, frame, ..)`(1419, 메인) → `show_auxiliary_windows(&context, frame, ..)`(1420, 보조 창마다 `show_window` 재호출) → `finish_window_frames`(1422 → 1428, IDE 선택 제출).
4. `show_window`(1686) 안: 탭 내용 분기(`APP/src/domains/editor.rs` 5370 `show_terminal_tab`, 5397 `TabKind::ClaudeDiff` → `show_claude_diff_tab`) → `finish_terminal_agent_frame(&context)`(2076) → 키맵 범위 계산 `terminal_views.has_keyboard_focus(&context)`(2085) → overlay(팔레트 등) → `finish_terminal_agent_frame(&context)` 한 번 더(2263, `show_window` 끝).

### 2.4 터미널 키 입력 경로 (19 관련)

- `AppSurfaces::show_terminal_tab`(`terminal_agent.rs` 646) → `Views::show_with_keymap`(3306) → 입력 루프에서 이벤트마다 `input_event(view, event)`(4261). spawn 전 입력은 `buffer_starting_input`(4342)이 같은 함수를 지나 `view.pending`에 쌓습니다.
- 포커스: 그 탭의 pane이 창의 포커스 대상(`target`, `application.rs` `window_command_context` 1452)이고 직전 포커스 탭과 다르면 `request_focus`가 참이 되고(`terminal_agent.rs` 654-658), `show_with_keymap`이 `response.request_focus()`를 부릅니다(`terminal_surface.rs` 3342-3344). 클릭·드래그 시작으로도 포커스를 얻습니다(3337-3341).
- `Views::has_keyboard_focus(context)`(3283-3291): 그 프레임에 그 viewport의 어떤 `View`가 `focused`이고 `focus_id == memory.focused()`이며 `focus_frame == 현재 프레임 번호`일 때만 참입니다. 그래서 그 프레임에 터미널을 그린 뒤에만 의미가 있습니다.
- `input_event`의 meta 분기(4276-4300): `Event::Key { pressed: true }`이고 `alt && !ctrl && !mac_cmd`이며 preedit이 비었고 `meta_character(key, shift)`(4222, 논리 키 → 없으면 물리 키)가 `Some(c)`면 `NativeInput::Key { Key::Character(c), alt }`를 보내고 `view.meta_text_pending = true`로 둡니다. 플래그가 선 동안 첫 `Text`·`Ime::Commit`은 버리고 `Ime::Preedit`이면 preedit을 비웁니다(4262-4275). 입력 루프 3곳(3432·3467·3707)이 끝날 때 플래그를 내립니다.
- 터미널 crate의 `input::character`(`TERM/src/input.rs`)가 `alt`면 ESC 접두를 붙입니다.
- 포커스된 터미널은 매 프레임 `output.ime = Some(IMEOutput { .. })`를 채웁니다(`terminal_surface.rs` 5226 `if focused` 안 5241). egui-winit는 이것을 보고 `window.set_ime_allowed(true)`를 부릅니다(`REG/egui-winit-0.36.2/src/lib.rs` 1138-1153).

### 2.5 검사 하네스

#### 2.5.1 터미널 표면 단위 검사 (`APP/src/terminal_surface.rs` `mod tests`, 필터 `terminal_surface::tests::`)

`View::default()`를 만들고 `buffer_starting_input`에 egui 이벤트를 넣어 쌓인 UTF-16 단위를 확인합니다. 실제 예(8923 `option_as_meta는_시작전_버퍼에서도_esc_접두와_조합문자_억제를_보존한다`, 발췌):

```rust
let alt = |key, shift| Event::Key {
    key,
    physical_key: Some(key),
    pressed: true,
    repeat: false,
    modifiers: egui::Modifiers {
        alt: true,
        shift,
        ..Default::default()
    },
};
let mut view = View::default();
let events = [
    alt(egui::Key::B, false),
    Event::Text("∫".into()),
    alt(egui::Key::E, false),
    Event::Ime(ImeEvent::Preedit {
        text: "´".into(),
        active_range_chars: None,
    }),
];
for event in &events {
    assert!(buffer_starting_input(&mut view, event));
}
assert!(view.preedit.is_empty());
assert!(!view.meta_text_pending);
let units = view.pending.iter().copied().collect::<Vec<_>>();
```

셀 그리기 shape 수집 예는 8856 `셀_그리기는_원본처럼_프로그래밍_합자를_만들지_않는다`입니다.

#### 2.5.2 통합 검사 (`APP/tests/terminal-host.rs`, `--test terminal-host`)

합성 PTY fixture(`Fixture::new()`, 95)와 실제 `Hub`·`Views`로 egui 프레임을 직접 돌립니다(`views.raw_input` → `context.run_ui` → `views.show` → `views.finish_frame`). 자식 프로세스는 bin `native-terminal-queue-fixture`(`APP/Cargo.toml` 13-15, 소스 `TERM/tests/fixtures/session.rs`)이고 환경 변수로 모드를 고릅니다. Option-as-Meta 검사(6368 `option_as_meta는_실제_views에서_esc_접두를_보내고_조합문자를_억제한다`)는 `TAIDE_NATIVE_FIXTURE_META=1`이면 fixture가 정확히 `META_INPUT = b"\x1bb\x1bB\x1b!\x1b \x1bex\x1b[1;3D\x1b\x01"`(`session.rs` 72)를 읽어야 제목 `native-meta-finished`(73)를 출력합니다. 이 바이트는 터미널 crate 검사와 앱 bin이 공유하므로 바꾸면 fixture와 검사를 같은 커밋에서 바꿉니다.

#### 2.5.3 앱 lib 실제 App 검사 (`APP`, `--lib`)

- 공용 fixture(`APP/src/application-project-rail-tests.rs`, 모듈 `application::project_rail_tests`)
    - `Fixture { directory, application }`(15, `Drop`에서 `on_exit` 후 디렉터리 삭제).
    - `isolated_state(data)`(80): IDE 통합·hooks를 끈 settings 파일을 씁니다.
    - `insert_project(state, id, root, name, root_missing)`(95), `project_ref`(69).
    - `launch(&mut fixture, state) -> egui::Context`(120): `NativeApplication::new(&CreationContext::_new_kittest(..), ..)`.
    - `step(application, context, events) -> FullOutput`(142): `eframe::App::logic(.., &mut eframe::Frame::_new_kittest())` 후 `inlay_hints_tests::paint`(`App::ui`, 화면 1000×700, `textures_delta.clear()`).
    - `pointer(position, button)`(192): 이동·누름·뗌 이벤트.
- 다른 공용 helper: `super::inlay_hints_tests::key(key, modifiers)`(누름·뗌 2개), `super::selection_ranges_tests::run_palette(application, context, command_id)`(`APP/src/application-selection-ranges-tests.rs` 74: 팔레트 열기 → 2프레임 → 입력 포커스 단언 → 명령 이름 입력 → 실행).
- terminal-agent 검사 파일 `APP/src/application-terminal-agent-tests.rs`(1,930줄, 모듈 `application::terminal_agent_tests`, macOS 전용)
    - `launch_terminals(name, count) -> Terminals { fixture, context, project, tabs }`(71): 빈 `session_id` 터미널 탭 `count`개를 한 pane에 두고 첫 탭을 활성, 사이드바 접힘, 단일 slot 포커스로 App을 띄웁니다.
    - `quiet_shell(directory)`(132): `stty -echo` 뒤 `/bin/cat`을 exec하는 셸 스크립트. 셸 선택은 `application.services.state.settings.write().shell_override = Some(..)`(예: 392행 `Some("/bin/cat".into())`).
    - `step_until(application, context, phase, ready)`(143): 10초(`DEADLINE`) 안에 조건이 설 때까지 프레임을 돌립니다.
    - `session(application, project, tab)`(164), `activate(application, context, tab)`(197, `layout_activate_tab` 후 `SETTLE_FRAMES`=4 프레임).
    - 탭 구성: `workspace_fixture(name)`(579), `file_tab(path)`(603), `terminal_tab(title, session_id)`(612), `place_tabs(state, project, tabs, active)`(622), `focus_single_slot(state, project, root, name)`(637).
    - CLI: `CliScript { Link, Cancel, Fail, Hold }`(1383), `CliProbe`(1390, `run`이 가짜 osascript), `connect_cli(application, context, directory) -> Arc<CliProbe>`(1429, 임시 `.app` 번들과 링크 경로로 `agent_cli::Ports`를 만들어 bridge 교체), `visible_toasts`(1476), `run_cli_command`(1486), `is_live_cli`(1504).
    - 보조 창: `route_viewports(context)`(1656, immediate viewport를 직접 `run_ui`로 그리고 viewport별 이벤트 주입·출력 저장), `unfocused_main_step`(1682, 메인 `focused: false`), `auxiliary_until`(1702).
- settings 검사(모듈 `application::domains::settings::settings_sections_tests`, `APP/src/application-settings-sections-tests.rs`): `Scene::launch(secrets, configure)`(36), `Scene::launch_with(.., is_attached)`(45), 내부 `assemble`(58-146, `NativeApplication::new_with_secret_store` 사용이라 플랫폼 주입 없음), `scene.ports(settings_ports::Ports)`(160), `scene.text(key)`(164), `scene.step(events)`(168), `scene.until(phase, ready)`(186, 30초), `scene.click(pos)`(209), helper `nodes(output, label)`(263, accesskit 노드), `texts_of`(255), `errors`(287), `successes`(297), `missing_cli_agents()`(309). 상호작용 검사는 `APP/src/application-settings-interface-tests.rs`(모듈 `settings_interface_tests`, `click_label`(47), `warnings`(37)).
- 알림 검사(모듈 `application::domains::shell::notifications_tests`, `APP/src/application-notifications-tests.rs`, `#[cfg(all(test, unix))]`): `FakePlatform`(26, `pub(super)`라 settings 모듈에서 보이지 않음), `launch_with_platform`(55, `new_with_overrides` + `Overrides { platform }`), `frame(application, context, focused)`(84).
- git Claude diff 검사(병합 뒤 모듈 `application::domains::git::claude_history_tests`, `GITWT/native/taide-native-app/src/application-git-claude-history-tests.rs`, `domains/git.rs` 3018-3019에서 연결): `open_diff(application, path) -> JoinHandle<Value>`(116, 실제 IDE `openDiff` 도구 호출), `answered`(169), `claude_tab`(180), `click`(205), `type_text`(213). 단위 `git_claude_diff::tests::`.

#### 2.5.4 UI crate 검사 (`UI`, `--features native-host,inspection --lib`)

- 명령 레지스트리(`command_registry::tests::`): `cli_세_명령은_mac_전용_셸_명령_범주로_native_host에서만_cli_실행경로를_갖는다`(`UI/src/command-registry.rs` 1221).
- 설정 섹션 view(`settings_view::sections_view::tests::`, `UI/src/settings-sections-view-tests.rs`): `cli_행은_조회중_미설치_설치됨_끊어진링크를_원본_문구와_색과_버튼으로_그린다`(1986), `hooks_토글은_꺼지면_목록이_없고_켜면_에이전트_5종과_프로젝트_없음_문구를_원본_순서로_그린다`(2036), `hooks_프로젝트_행은_조회중_설치됨_미설치와_동의_대화상자를_원본대로_처리한다`(2113), `hooks_사용자_행은_cli_요구와_조회상태로_비활성_4경우와_경고를_원본대로_정한다`(2260), `알림_버튼은_스위치_아래에서_시스템_설정과_시험_알림을_요청하고_dev_안내는_그_아래에만_있다`(2328).
- 설정 섹션 data(`settings_sections_data::tests::`, `UI/src/settings-sections-data-tests.rs`): `cli_설치_해제는_진행을_표시하고_응답상태로_교체하며_취소는_성공문구가_없다`(943), `hooks_조회는_대상별로_나뉘고_설치_해제_응답이_그_대상만_교체한다`(1010), `시험_알림은_포커스를_싣고_사유별_스위치_라벨과_전달_문구로_알린다`(1077).

### 2.6 실행 명령 (한 줄, 절대 경로)

workflow 작업자 셸 규칙(변수 대입·`cd`·파이프 금지)에 맞춰 모두 풀어 적습니다.

| 목적 | 명령 |
| --- | --- |
| LSP mock 준비(앱 lib를 넓게 돌리기 전) | `cargo build --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` |
| 앱 lib 필터 | `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- <필터…> --test-threads=2` |
| 앱 통합 | `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --test terminal-host --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- <필터>` |
| UI | `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --features native-host,inspection --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- <필터>` |
| 컴파일·경고 | `cargo check --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --all-targets --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` |
| 형식 | `cargo fmt --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml -- --check` |
| 동결 remote-web(UI crate나 vendored eframe을 바꾼 경우만) | `cargo check --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` |
| 병합 게이트 전체 | `bash /Users/hyunseokbyun/development/TAIDE/docs/utils/2026-11-03-native-merge-gate.sh <로그 디렉터리>` |

- 앱 crate의 `inspection` feature는 `application::emmet_tests`·`sticky_definition_tests`(`application.rs` 2953-2959)만 가릅니다. 이 영역 검사는 없어도 됩니다(dev-dependency가 UI `inspection`을 켬, `APP/Cargo.toml` 105-106). lane QA처럼 `--features inspection`을 붙여도 결과는 같습니다.
- `--test-threads`는 필터 실행 2, 게이트 4입니다. 실제 App 검사가 동시에 많이 돌면 host 큐(64)가 차서 흔들립니다(4절 8번).
- lane 사양 seams의 terminal_agent 영역 필터(`docs/research/2026-10-28-native-lane-seams.md` 250): lib `application::ide_selection_tests terminal_ agent_hooks ide_server ide_tools remote_agents status_ide`, 통합 `terminal-dispatch terminal-environment terminal-file-links terminal-frames terminal-host terminal-links terminal-writer`.

### 2.7 기능표·QA·PROCESS 갱신 규칙

- 기능표 행 형식: `| ID | 요구사항 | TS 근거 | 현재 판정 | 근거 묶음 | 범위/잔여 동작 |`. 판정은 `complete`·`partial`·`unwired`·`missing`·`not-applicable`·`frozen` 중 하나. 비고에는 근거 검사 이름과 QA 문서 파일명을 적고, 남은 범위가 있으면 "남은 것:"으로 시작하는 문장을 둡니다(예: 기능표 349행).
- complete의 뜻: 구현 + 실제 앱 경로 + 해당 동작 묶음의 자동 검사 근거(머리말 5행). 픽셀·실제 OS 입력·실기·성능·출시 통과는 포함하지 않습니다. 예외는 19(3.1 (f)).
- QA 문서: `docs/quality-assurance/<YYYY-MM-DD>-native-terminal-agent-<주제>.md`. 기존 lane 문서처럼 "대상 ID와 판정 / 원본 동작 근거 / 구현 / 실제 앱 경로 / 자동 검사 / 검증 명령과 결과 / 남은 범위" 순서. 실행하지 않은 검사를 통과로 쓰지 않습니다.
- `docs/PROCESS.md` 맨 위 진행 절의 체크리스트와 기능표 집계 문장(31행 "599행 중 complete 405…")을 함께 갱신합니다.
- 커밋·푸시는 메인 오케스트레이터만 합니다. 작업자는 git 상태를 바꾸지 않습니다.

---

## 3. 행별 상세

### 3.1 terminal-agent-19 Option-as-Meta

#### (a) ID·기능·상태

- `terminal-agent-19` "Option-as-Meta(`macOptionIsMeta`)", `partial`, 근거 묶음 `terminal-base`(기능표 312행).
- 비고 요약: 일반 Option 문자 키는 ESC 접두로 보내고 조합 문자를 버림(검사 통과). 남은 것: dead key는 터미널 포커스 중 IME 허용 상태라 `Ime::Preedit`만 와서 ESC 접두가 나가지 않음. 근본 수정 후보는 winit `OptionAsAlt` 전환. 근거 QA `2026-11-02-native-lane-terminal-agent-terminal-core.md` 수정 절 "19: complete → partial".

#### (b) 원본 TS 동작

- 설정: `TS/features/terminal/terminal-view.tsx` 271 `macOptionIsMeta: true`. 사용자 설정이 아니라 상수입니다. 보조 창도 `EditorArea`(`TS/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 139) → `PaneNodeView`(`TS/widgets/editor-area/pane-node-view.tsx` 182-183 `TerminalSession`)로 같은 `TerminalView`를 마운트하므로 같은 규칙입니다.
- 키 인코딩 `NM/@xterm/xterm/src/common/input/Keyboard.ts`
    - 325: `(!isMac || macOptionIsMeta) && ev.altKey && !ev.metaKey`이면 Option을 Meta로 씁니다. 좌우 Option 구분 없음.
    - 326-330: `KEYCODE_KEY_MAPPINGS`(11-36)의 숫자·기호 키는 `ESC + [기본, shift]`.
    - 331-337: A~Z는 `ESC + 소문자`(shift면 대문자, ctrl이면 제어 문자).
    - 338-339: Space는 `ESC + ' '`(ctrl이면 `ESC + NUL`).
    - 340-351: `ev.key === 'Dead'`이고 `ev.code`가 `Key`로 시작하면 `ESC + 글자`를 보내고 `result.cancel = true`.
- 조합 무시: `NM/@xterm/xterm/src/browser/CoreBrowserTerminal.ts` 1030 `shouldIgnoreComposition = isMac && macOptionIsMeta && event.altKey` → 1032 조합 도우미를 건너뛰고 1038-1040 dead key 표식을 세우지 않습니다.
- 결과: Option+B → `ESC b`, US 배열 Option+E(dead key) → `ESC e`이고 `´` 조합이 생기지 않습니다. Cmd+Option은 meta 규칙 밖, Ctrl+Option+문자는 `ESC + 제어 문자`.
- 화면 변화·사용자 문자열·locale 키: 없음. 오류·빈 상태·취소 분기: 없음. 수명: 터미널 인스턴스가 살아 있는 동안 항상(그 터미널이 키보드 포커스를 가질 때만 키를 받음). 편집기(Monaco) 등 다른 입력 위젯에서는 Option 조합 문자를 그대로 입력합니다.

#### (c) native 현재 자산과 정확히 빠진 것

- 있는 것: `terminal_surface.rs` `meta_character`(4222), `input_event` meta 분기(4276-4300), `View.meta_text_pending`(291), 플래그 내림(3432·3467·3707). 검사: 단위 `terminal_surface::tests::option_as_meta는_시작전_버퍼에서도_esc_접두와_조합문자_억제를_보존한다`(8923), 통합 `option_as_meta는_실제_views에서_esc_접두를_보내고_조합문자를_억제한다`(`APP/tests/terminal-host.rs` 6368).
- 빠진 것 1: macOS dead key 경로. 실제 소스로 확인한 사건 순서입니다.
    1. 터미널이 포커스면 egui-winit가 IME를 허용합니다(2.4 마지막 항목).
    2. winit 0.30.13 `REG/winit-0.30.13/src/platform_impl/macos/view.rs` `key_down`(442-497): 458 `let event = replace_event(event, self.option_as_alt());` → IME 허용이면 466-468 `interpretKeyEvents` → dead key는 그 안에서 `set_marked_text`(270-)가 불려 `Ime::Preedit`을 보내고 상태가 Preedit이 됩니다 → 479-488 `had_ime_input`이 참 → 490 조건 때문에 `KeyboardInput`을 보내지 않습니다.
    3. 그래서 egui에는 `Event::Key` 없이 `Event::Ime(Preedit("´"))`만 옵니다. meta 분기는 `Event::Key`에서만 시작하므로 ESC 접두가 나가지 않고 preedit이 다음 키와 조합됩니다. 기존 검사가 합성한 `[Key{E, alt}, Preedit]` 순서는 macOS에서 생기지 않습니다.
    4. 일반 Option 문자(Option+B)는 `insert_text`(393-416)만 불리고 marked text가 없으면 Ime 이벤트를 보내지 않아 `KeyboardInput`이 갑니다. 이 경로는 이미 맞습니다.
- 수정 수단: winit macOS 확장 `winit::platform::macos::WindowExtMacOS::set_option_as_alt(&self, OptionAsAlt)`(`REG/winit-0.30.13/src/platform/macos.rs` 161 선언, 237-239 구현 `self.window.maybe_queue_on_main(move |w| w.set_option_as_alt(option_as_alt))`)와 `OptionAsAlt { OnlyLeft, OnlyRight, Both, None }`(518-532, 기본 `None`). 값은 NSView ivar에 저장되고(`view.rs` 913-919) 다음 `key_down`부터 쓰입니다. 켜면 `replace_event`(`view.rs` 1109-1145)가 alt가 눌렸고 ctrl·cmd가 없을 때 사건의 `characters`를 `charactersIgnoringModifiers`(Option 없는 기본 글자)로 바꾼 새 `NSEvent`를 만들고, 그 사건이 `interpretKeyEvents`와 `KeyboardInput` 생성에 쓰입니다. 기대 결과는 dead key 대신 `KeyboardInput{ text: "e" }` → egui `[Event::Key{E, alt}, Event::Text("e")]` → 기존 meta 분기가 `ESC e`를 보내고 `Text("e")`를 버리는 것입니다. 즉 `input_event`는 바꿀 필요가 없습니다.
- 미검증 위험(실기로만 확인 가능): `replace_event`가 만든 새 `NSEvent`도 원래 `modifierFlags`(Option 포함)와 `keyCode`를 그대로 가집니다(`view.rs` 1125-1136 `keyEventWithType_…_keyCode(.., event.modifierFlags(), .., event.keyCode())`). AppKit 입력 문맥이 `interpretKeyEvents`에서 이 사건의 `characters`가 아니라 `keyCode`+`modifierFlags`로 dead key 조합을 시작하면 OptionAsAlt를 켜도 `Ime::Preedit`이 계속 옵니다. 이것은 AppKit 내부 동작이라 저장소와 registry 소스로는 확인할 수 없습니다. winit 문서(`platform/macos.rs` 155-160 `set_option_as_alt` 설명 "This will ignore diacritical marks and accent characters from being processed as received characters")는 이 용도를 의도하지만 IME 허용 상태에서의 결과는 적혀 있지 않습니다. 그래서 1단계 결과는 실기 확인 전 complete 근거로 쓰지 않습니다((f)).
- 확인한 사실: `native/`, `EFRAME/src`, `REG/egui-winit-0.36.2/src`에 `option_as_alt`·`OptionAsAlt` 사용이 0건입니다. 앱은 `frame.winit_window()`를 아직 쓰지 않고, 창 접근 선례는 raw-window-handle 경유 `window_state::is_zoomed(frame)`(`APP/src/window-state.rs` 202-215)뿐입니다.
- 빠진 것 2: 보조 창. `frame.winit_window()`는 루트 창만 줍니다(0.3). 보조 창 터미널에는 `OptionAsAlt`를 켤 수단이 없습니다.
- 거절한 대안
    - `Ime::Preedit`이 alt와 함께 오면 ESC를 합성: preedit 글자(`´`)에서 원래 키를 추측해야 해 배열마다 달라지고, AppKit에 남은 marked text를 따로 취소해야 하는 우회입니다.
    - Option을 누른 동안 `output.ime = None`으로 IME를 끔: Option 눌림(flagsChanged)과 키 사이에 프레임이 한 번 돌아야만 동작하는 시점 의존이고, 진행 중인 한글 조합을 끊습니다.
    - winit를 vendor해 OptionAsAlt 사건에서 `interpretKeyEvents`를 건너뜀: 새 vendored crate라 허락 항목(P22)입니다. 실기에서 위 미검증 위험이 실제로 드러날 때만 메인 결정으로 올립니다((g)).

#### (d) 구현 방법

1단계(메인 창):

1. `APP/src/domains/terminal_agent.rs`에 상태 타입을 추가하고 `State` 끝에 필드를 둡니다. 이름은 winit의 `OptionAsAlt`와 겹치지 않게 합니다.

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct OptionAsAltState {
    pub(crate) is_wanted: bool,
    pub(crate) applied: Option<bool>,
}
```

    - `State`(100-110) 끝: `pub(crate) option_as_alt: std::collections::HashMap<egui::ViewportId, OptionAsAltState>,`
    - `APP/src/application.rs` 생성자 리터럴(541-551)의 `cli_status: None,` 다음 줄: `option_as_alt: Default::default(),`(공용 파일, 추가만).
2. 같은 파일 `impl NativeApplication`(375-630)의 `finish_terminal_agent_frame` 다음에 메서드를 둡니다. `has_keyboard_focus`는 그 viewport의 그 프레임에 터미널을 그린 뒤에만 의미가 있으므로 창 pass가 끝난 뒤 부릅니다.

```rust
pub(crate) fn apply_terminal_option_as_alt(
    &mut self,
    context: &egui::Context,
    window: Option<&winit::window::Window>,
) {
    let is_wanted = self
        .terminal_agent_state
        .terminal_views
        .has_keyboard_focus(context);
    let state = self
        .terminal_agent_state
        .option_as_alt
        .entry(context.viewport_id())
        .or_default();
    state.is_wanted = is_wanted;
    #[cfg(target_os = "macos")]
    if let Some(window) = window
        && state.applied != Some(is_wanted)
    {
        use winit::platform::macos::{OptionAsAlt, WindowExtMacOS};
        window.set_option_as_alt(if is_wanted {
            OptionAsAlt::Both
        } else {
            OptionAsAlt::None
        });
        state.applied = Some(is_wanted);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = window;
}
```

    - `Both`인 이유: xterm은 `ev.altKey`(좌우 구분 없음)로 판정합니다(Keyboard.ts 325).
    - 키보드 포커스일 때만 켜는 이유: `OptionAsAlt`는 NSView 전체에 적용되므로 항상 켜면 편집기·팔레트·검색창에서 Option+E → `é` 같은 문자 입력이 사라집니다. 원본도 xterm 밖에서는 Option 조합 문자를 입력합니다.
    - 값이 바뀔 때만 부릅니다(`set_option_as_alt`는 `maybe_queue_on_main`, `platform/macos.rs` 237-239).
3. `APP/src/application.rs` `frame_ui`(1383)에서 `let auxiliary_proceeded = self.show_auxiliary_windows(&context, frame, &mut pass);`(1420) 바로 다음 줄에 한 줄을 추가합니다(공용 파일, 추가만). 이 시점의 `context.viewport_id()`와 memory 포커스는 ROOT입니다(0.3 마지막 항목).

```rust
self.apply_terminal_option_as_alt(&context, frame.winit_window().map(|window| &**window));
```

4. `input_event`·`meta_character`는 바꾸지 않습니다. 아래 (e) 2번 단위 검사로 OptionAsAlt 아래의 사건 모양을 고정합니다.

2단계(보조 창, shell-5의 vendored eframe 변경 병합 뒤):

5. 선행: `10-remaining-shell.md` 3.2 (d) 2.1이 `EFRAME/src/lib.rs`에 `pub fn immediate_viewport_window() -> Option<std::sync::Arc<winit::window::Window>>`(immediate viewport `run_ui` 동안 thread-local로 보관)를 추가합니다. 이 함수가 메인에 들어왔는지 `grep -n "fn immediate_viewport_window" /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/vendor/eframe/src/lib.rs`로 확인합니다. 없으면 2단계를 시작하지 않습니다(같은 vendored 변경을 이 영역에서 따로 만들지 않습니다).
6. `APP/src/domains/shell.rs` `show_auxiliary_windows`의 viewport 콜백(1535-1550)에서 `self.show_auxiliary_toasts(ui.ctx(), window.viewport);` 다음에 두 줄을 추가합니다(shell 영역 파일, 추가만). 콜백 안이므로 `ui.ctx().viewport_id()`가 그 보조 창입니다.

```rust
let native_window = eframe::immediate_viewport_window();
self.apply_terminal_option_as_alt(ui.ctx(), native_window.as_deref());
```

7. 같은 파일 `close_auxiliary_window`(1574)에서 `self.toasts.retain(|viewport| viewport != window.viewport);`(1585) 다음 줄에 `self.terminal_agent_state.option_as_alt.remove(&window.viewport);`를 추가합니다. 창이 닫히면 NSView도 사라지므로 적용 기록을 남기지 않습니다.

#### (e) 자동 검사 방법

1. 앱 lib 실제 App 검사(새 검사, `APP/src/application-terminal-agent-tests.rs`). 이름 예 `실제_app_터미널_키보드_포커스는_메인창_option_as_alt를_원하고_팔레트와_편집기_포커스에서는_원하지_않는다`.
    1. 준비(같은 파일 `실제_app_cli_셸_명령은…` 1516-1529와 같은 모양): `let mut fixture = workspace_fixture("option-as-alt");`, `let root = fixture.directory.join("alpha");`, `std::fs::create_dir_all(&root)`, `let main_file = root.join("main.rs");`에 `SYNTHETIC_SOURCE`를 씀, `let project = ProjectId::new();`, `let state = isolated_state(fixture.directory.join("data"));`, `insert_project(&state, &project, &root, "alpha", false);`, `let terminal = terminal_tab("option-as-alt", "");`, `let file = file_tab(&main_file);`, `place_tabs(&state, &project, vec![terminal.clone(), file.clone()], &terminal.id);`, `focus_single_slot(&state, &project, &root, "alpha");`, `let context = launch(&mut fixture, state);`, 그다음 `application.services.state.settings.write().shell_override = Some("/bin/cat".into());`.
    2. `step_until(application, &context, "terminal focus", |application, _| session(application, &project, &terminal.id).is_some() && wanted(application))`. `wanted`는 `application.terminal_agent_state.option_as_alt.get(&egui::ViewportId::ROOT).is_some_and(|state| state.is_wanted)`를 돌려주는 지역 클로저입니다. 자동 포커스가 서지 않으면(2.4 포커스 조건) 터미널 영역을 클릭합니다: `step(application, &context, pointer(egui::pos2(500.0, 400.0), egui::PointerButton::Primary))`(`pointer`는 `project_rail_tests`에서 이미 import). 화면은 1000×700이고 이 점은 탭 바 아래 본문입니다. 사이드바가 이 점을 덮으면 `layout.shell_view.sidebar_collapsed = true`(`launch_terminals` 111행과 같은 설정)를 준비 단계 `state.layouts`에 넣습니다.
    3. 단언 1: `applied == None`(kittest `Frame`에는 창이 없어 적용하지 않음을 고정).
    4. 단언 2: `application.commands_state.palette.open(&context, taide_native_ui::command_registry::PaletteEntry::Commands);` 후 `step` 2회 → `application.commands_state.palette.is_open()`이고 `is_wanted == false`.
    5. 단언 3: `step(application, &context, super::inlay_hints_tests::key(egui::Key::Escape, egui::Modifiers::NONE))` 후 `!palette.is_open()`. 그다음 다시 터미널을 클릭하고 `step_until`로 `is_wanted == true`. 팔레트를 닫은 뒤 포커스가 자동으로 돌아오는지는 이 행의 범위가 아니므로 단언하지 않습니다.
    6. 단언 4: `activate(application, &context, &file.id)`(`layout_activate_tab` + `SETTLE_FRAMES`) → `is_wanted == false`. 다시 `activate(application, &context, &terminal.id)` → `step_until`로 `is_wanted == true`. 탭 복귀 때 자동 포커스는 `NativeApplication.focused`(편집기가 포커스를 얻을 때만 그 탭으로 바뀜, `domains/editor.rs` 5985·6046)에 달려 있어 보장되지 않으므로, 몇 프레임 안에 서지 않으면 2번과 같은 클릭으로 포커스를 줍니다.
2. 단위 검사(새 검사, `terminal_surface.rs` `mod tests`). 이름 예 `option_as_alt_사건_모양은_dead_key_자리에서도_esc_접두를_보내고_기본_글자를_버린다`. `View::default()`에 `[alt(E), Text("e"), alt(U, shift), Text("U"), alt(N), Text("n"), alt(Backtick), Text("`")]`를 `buffer_starting_input`으로 넣고 `pending`이 `"\x1be\x1bU\x1bn\x1b`"`, `meta_text_pending == false`, `preedit` 비어 있음을 단언합니다. 이 검사는 지금 코드로도 통과해야 정상입니다(회귀 고정용). 통합 fixture의 `META_INPUT`은 바꾸지 않습니다(2.5.2).
3. 2단계 App 검사(새 검사, 같은 파일): `실제_app_보조창_task_runner는…`(1725) 검사처럼 보조 창 slot 1에 같은 프로젝트를 열고 그 창에 터미널 탭을 둔 뒤 `route_viewports`로 그리고, 보조 viewport 이벤트로 터미널 영역을 클릭합니다. 단언: `option_as_alt[보조 viewport].is_wanted == true`, `option_as_alt[ROOT].is_wanted == false`(메인은 `unfocused_main_step`), 창을 닫으면(`close_auxiliary_window`) 그 키가 없어짐. 검사용 immediate renderer는 eframe thread-local을 채우지 않으므로 `applied`는 `None`입니다. 실제 창 적용은 실기입니다.
4. 실행
    - `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- terminal_surface::tests::option_as application::terminal_agent_tests --test-threads=2`(기존 terminal-agent App 검사 회귀 포함)
    - `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --test terminal-host --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- option_as_meta`
    - `cargo check --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --all-targets --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`(새 경고 없음, 비 macOS cfg 분기 오타는 이 기계에서 컴파일되지 않으므로 코드 리뷰로 확인)
    - `cargo fmt --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml -- --check`
    - 2단계는 shell 영역 파일을 바꾸므로 `application::auxiliary_window_tests`도 함께 돌립니다.

#### (f) 완료 판정 기준

- 1단계만 끝남: 위 검사 통과, 행은 `partial` 유지. 비고를 "메인 창은 터미널 키보드 포커스 동안 winit `OptionAsAlt::Both`(`apply_terminal_option_as_alt`)를 켜고 그 밖에서는 끔. 남은 것: 보조 창 창 핸들(shell-5 vendored eframe), dead key 실기"로 고칩니다.
- 2단계까지 끝남: 보조 창 검사 통과. 행은 여전히 `partial`이고 남은 것은 실기만입니다.
- complete 조건(이 행만의 예외): 1·2단계 검사 통과 + 실기(P23)에서 포커스된 터미널의 `cat -v`로 US 배열 Option+E → `^[e`, Option+B → `^[b` 확인. 이유: 이 행은 실제 macOS 사건 순서(dead key) 때문에 partial로 내려갔고, 수정 효과가 AppKit 동작에 달려 있어 자동 검사가 그 위험을 덮지 못합니다((c) 미검증 위험). 실기 없이 complete로 올리면 lane 리뷰가 잡았던 과대 판정을 반복합니다.

#### (g) 막는 요소와 처리

- 2단계 선행(shell-5): vendored eframe 변경은 shell 영역이 만듭니다(10번 문서 3.2). 그 전에는 1단계만 하고 행을 partial로 둡니다.
- 실기(배치 31, P23 사용자 허락): GUI 실행·합성 OS 입력은 작업자 금지입니다. 확인 항목: 포커스된 터미널에서 `cat -v`로 Option+B → `^[b`, Option+E/U/I/N/\` → `^[e` 등, Option+Shift+E → `^[E`, 편집기·팔레트에서 Option+E는 여전히 `é` 조합, 한글 입력 소스에서 Option+키(원본은 keyCode 기준 `ESC + 라틴 글자`), 오른쪽 Option, 터미널 포커스 전환 직후 첫 키, 보조 창 터미널.
- 실기에서 dead key가 여전히 조합되면: (c) 미검증 위험이 실제라는 뜻입니다. 행을 partial로 두고 메인에게 한 번에 묻습니다 — A안(추천: winit를 vendor해 OptionAsAlt가 적용된 사건은 `interpretKeyEvents`를 건너뛰게 함, P22 허락 필요) / B안(차이를 비고에 남기고 partial 유지).
- 기존 동작 보존: Ctrl+Option(`key_input`의 ctrl 분기), Cmd 조합, Option+방향키(`\x1b[1;3D`)는 `replace_event`가 ctrl·cmd 조합을 바꾸지 않고(1116-1117) 방향키 글자는 기본 글자와 같아 그대로입니다. 기존 통합 검사가 이 바이트를 계속 확인합니다.

---

### 3.2 terminal-agent-55 hooks 설치·제거·상태 UI(설정)

#### (a) ID·기능·상태

- `terminal-agent-55` "훅 설치·제거·상태 UI(설정)", `unwired`, 근거 묶음 `integrations`(기능표 348행). 비고는 "서비스/프로토콜/카탈로그는 있으나 … 소비자는 연결되지 않았습니다"라는 오래된 일반 문구입니다.
- 같은 기능 행: settings-17 `complete`(기능표 566행, lane settings S1, 병합 `3864e1db`). lane 사양 결정 1 A안대로 settings 영역이 구현했고 이 행은 갱신만 남았습니다.

#### (b) 원본 TS 동작

- 위치: Interface 섹션 `TS/widgets/settings-view/settings-interface-section.tsx` 67-76. 순서는 배지 스위치(62-66) → CLI 행(`AgentCliStatusRow`, 67) → hooks 토글(`AgentHooksToggle`, 라벨 `settings.agentHooks`, 힌트 `settings.agentHooksHint`, 값 `agentHooksEnabled ?? false`, 69-74) → 토글이 켜졌을 때만 `AgentHooksProjectList`(75) → IDE 통합(77-82) → diff 자동 열기(83-87).
- 목록 `TS/widgets/settings-view/agent-hooks-project-list.tsx`
    - 29-35 에이전트 5종 순서: claude(project 범위, `settings.agentHooksAgentClaude`), codex·gemini·opencode·pi(user 범위, `settings.agentHooksAgent{Codex,Gemini,Opencode,Pi}`).
    - 42-59 claude 섹션: 라벨, 프로젝트가 없으면 `app.noProjectOpen`, 있으면 프로젝트마다 `AgentHooksProjectRow`.
    - 66-100 user 행: 상태 조회 `agentHooksStatusQueryOptions('', agentName)`, CLI 조회 `cliInstallStatusQueryOptions()`. `requiresTaideCli`가 참이고 CLI 조회가 끝났는데 미설치면 경고 `settings.agentHooksCliMissing`(status-warning 색, 92). 비활성(96): 상태 없음, 설치·해제 진행 중, CLI 조회 중, CLI 미설치이면서 hooks도 미설치. 힌트 `settings.agentHooksUserLevelDescription`(90)과 경고 `settings.agentHooksUserLevelWarning`(91). 실패 `toast.error(describeIpcError)`(80).
- 프로젝트 행 `TS/widgets/settings-view/agent-hooks-project-row.tsx` 27-79: 조회 중 `settings.loading`, 설치됨이면 `agent.hooksInstalled`(agent-running 색) + outline xs `agent.hooksUninstall`(진행 중 비활성), 아니면 outline xs `agent.hooksInstall` → 동의 AlertDialog(제목 `agent.hooksConsentTitle`, 설명 `agent.hooksConsentDescription`, 취소 `common.cancel`, 확인 `agent.hooksInstall`). 확인하면 설치 요청 후 즉시 닫힘(36-39). 실패 `toast.error(describeIpcError)`.
- 데이터 `TS/entities/agent/agent.query.ts` 37-54: user 범위 projectId는 `''`(37), 조회 키 `QUERY_KEY.AGENT.HOOKS(projectId, agentName)`, 설치·해제 성공 응답으로 그 대상 캐시만 교체(48).
- 백엔드 오류 문구: `CR/taide-runtime/src/agent_hook_actions.rs` 101 `agent hooks are disabled in settings`, 141 `taide CLI is not installed yet`.

#### (c) native 현재 자산과 빠진 것

- UI: `UI/src/settings-sections-view.rs` `interface`(501, 동의 대화상자 상태 `OwnerState.hooks_consent`(202), 처리 564-602), `hooks_list`(688), `hook_project_row`(738, 설치 버튼이 대화상자를 엶 805-806), `hook_user_row`(828, CLI 누락 경고 865). 데이터: `UI/src/settings-sections-data.rs` `HookTarget`(54), `Query::HooksStatus`(74), 대상·종류별 진행 맵.
- 앱: `APP/src/domains/settings.rs` `execute_sections`의 `Query::HooksStatus`(749-762, `agent_hook_actions::agent_hooks_status`)와 `Action::Hooks`(763-798, `AgentHookInstallPorts::new(resolve_home, project_emitter, cli 설치 여부, ensure_started, cli_target)`). 토글은 `HostedChange::AgentHooks` → 기존 `UpdateSettings` → reconcile.
- 검사(모두 기존): UI 2.5.4의 hooks 3건(view)과 1건(data), 앱 `application::domains::settings::settings_interface_tests::실제_hooks는_토글_뒤_임시_프로젝트와_가짜_home에_설치_해제하고_원본_오류를_알린다`(`APP/src/application-settings-interface-tests.rs` 255).
- 빠진 것: 기능표 348행 문구와 판정뿐입니다. 코드는 없습니다.

#### (d) 구현 방법

1. 근거 재실행:
    - `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- application::domains::settings::settings_interface_tests::실제_hooks --test-threads=2`
    - `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --features native-host,inspection --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- hooks_`
2. 기능표 348행: 판정 `complete`, 근거 묶음 `integrations` 유지, 비고 예: "settings-17과 같은 구현(lane settings S1, 병합 `3864e1db`): hooks 토글 → 켜졌을 때 claude 프로젝트별 행(조회 중·설치됨·미설치, 동의 대화상자)과 codex·gemini·opencode·pi user 행(CLI 요구·조회 상태 비활성, CLI 누락 경고), 대상별 조회와 성공 응답 교체. [U] `hooks_토글은…`·`hooks_프로젝트_행은…`·`hooks_사용자_행은…`, [A] `실제_hooks는_토글_뒤_임시_프로젝트와_가짜_home에_설치_해제하고_원본_오류를_알린다`. 실제 사용자 홈·실제 프로젝트 파일 쓰기는 허락 항목(P07). `2026-11-02-native-lane-settings-s1-interface-performance.md`, `<새 QA 문서>`".
3. QA 문서(2.7)에 재실행 명령과 결과 수를 적고, `docs/PROCESS.md` 집계를 갱신합니다.

#### (e) 자동 검사 방법

- 새 검사는 없습니다. (d) 1의 두 명령 결과(통과 수)를 QA에 적습니다. 실패하면 행을 바꾸지 않고 settings 영역에 실패 이름·출력을 넘깁니다.

#### (f) 완료 판정 기준

- 두 명령이 통과하고 행 비고가 위 근거로 바뀌면 `complete`.

#### (g) 막는 요소

- 없음. 실제 `~/.codex` 등 사용자 홈과 실제 프로젝트 `.claude/settings.local.json` 쓰기는 허락 항목(P07)이며 complete 조건이 아닙니다. 검사 App은 프로젝트를 `root_missing`으로 넣어(`Scene::launch_with(.., false)`) 시작 watcher가 실제 `HOME`으로 reconcile하지 않게 합니다. 이 장치를 지우지 마십시오(4절 4번).

---

### 3.3 terminal-agent-56 CLI 설치 상태·설치·제거·외부 에디터 연결 명령

#### (a) ID·기능·상태

- `terminal-agent-56`, `partial`, 근거 묶음 `integrations`(기능표 349행). 비고의 남은 것: "설정 화면 CLI 행(settings-16, `terminal_agent_state.cli_status` 소비), 실제 `/usr/local/bin` 설치는 허락 뒤 실기".
- settings-16은 `complete`(기능표 565행)입니다. 다만 lane 사양 결정 2 A안("settings lane S1은 행 UI만 만들고 `agent_cli`를 재사용", 사양 4.1 2번)과 달리 settings lane은 공유 crate `CR/taide-runtime/src/agent_host.rs`에 `cli_install`·`cli_uninstall`·`run_admin_osascript`를 추가해 썼고(settings S1 QA 3.1), `terminal_agent_state.cli_status`를 읽지 않습니다. 비고의 "`cli_status` 소비"는 실제와 다릅니다.

#### (b) 원본 TS 동작

- 명령(`TS/entities/agent/agent.commands.ts`, macOS에서만 등록 55-66, 범주 `keymap.category.shellCommand`)
    - 18-30 `cli.connectExternalEditor`(`keymap.cliConnectExternalEditor`): 상태 조회 → `installed && !dangling`이면 그대로, 아니면 설치 → 결과가 `!installed || dangling`이면 `toast.error(settings.cliInstallFailed)`, 아니면 `toast.success(settings.cliExternalEditorConnected)`. 예외도 `cliInstallFailed`.
    - 32-39 `cli.installShellCommand`(`keymap.cliInstall`): 성공 `settings.cliInstallSuccess`(결과 상태와 무관), 실패 `settings.cliInstallFailed`.
    - 41-48 `cli.uninstallShellCommand`(`keymap.cliUninstall`): 성공 `settings.cliUninstallSuccess`, 실패 `settings.cliUninstallFailed`.
    - 명령은 IPC를 직접 부르고 Query 캐시를 바꾸지 않습니다.
- 설정 행(settings-16, `TS/widgets/settings-view/agent-cli-status-row.tsx` 9-53): macOS만, 제목 `keymap.category.shellCommand`, 설치됨이면 `settings.cliStatusInstalled`(끊어진 링크면 `settings.cliStatusDangling`, status-warning 색), 버튼 `settings.cliInstallButton`(미설치이거나 끊어짐)·`settings.cliUninstallButton`(설치됨), 성공 응답으로 캐시 교체(`agent.query.ts` 25-31).
- 백엔드(`TAURI/domain/agent/commands.rs`): osascript는 PATH의 `"osascript"`로 실행(131), 성공 또는 사용자 취소(`is_osascript_user_cancelled`, 136)는 `Ok`, 그 밖은 `osascript failed: …`. 설치(185·200)는 같은 대상 symlink면 생략, 아니면 관리자 스크립트. 제거(209·241)는 TAIDE 소유 링크만, 권한 거부면 관리자 `rm`(229).
- 결정(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` 13행, 사양 결정 7 A안): 관리자 대화상자 취소 시 성공 문구를 보이지 않습니다. 명령(commands-ui-33)과 설정 행(settings-16) 모두 같은 규칙입니다.

#### (c) native 현재 자산과 빠진 것

- 명령 경로(완료): `UI/src/command-registry.rs` `Run::Cli`(193, 463-471) → `APP/src/shell_keymap.rs` 142-146 → `apply_terminal_agent_intent`(449) → `Command::Cli` → `spawn_transient` + `run_blocking_result` → `agent_cli::run` → `Reply::Cli` → `finish_agent_cli`(467-503, toast 판정: 설치는 `installed && !dangling`일 때만 성공, 제거는 `!installed`일 때 성공, 연결은 살아 있으면 연결 성공 아니면 `cliInstallFailed`). 검사: `agent_cli::tests` macOS 6건(`APP/src/agent-cli-tests.rs` 101·118·138·160·176·205), App `실제_app_cli_셸_명령은_팔레트에서_실행되어_원본_toast를_보이고_취소는_성공_문구를_보이지_않는다`(`application-terminal-agent-tests.rs` 1513), UI 레지스트리 검사.
- 설정 행(완료, settings 영역): UI `cli_row`(605) → `Action::CliInstall`/`CliUninstall` → `APP/src/domains/settings.rs` 742-747 → `change_shell_command`(846, `run_blocking_result`) → `settings_ports::CliPorts`(125) 기본값 `shell_command`(144: 비 macOS 거절 → `agent_host::cli_install/cli_uninstall` + `run_admin_osascript(Path::new("/usr/bin/osascript"), ..)`). 상태 조회는 `ports.agents.cli_status`(`APP/src/remote-agents.rs` 38·60). 응답 처리 `finish_cli`(`UI/src/settings-sections-data.rs` 1209-1223): `Ok`이고 `!is_cancelled`면 성공 문구.
- 확인한 이중 구현과 차이
    1. 같은 Tauri 정책이 두 곳에 있습니다: `APP/src/agent-cli.rs` 53-123(`install_target`·`install`·`uninstall`)과 `CR/taide-runtime/src/agent_host.rs` 136-174(`cli_install_target`·`cli_install`·`cli_uninstall`). 오류 문구는 글자까지 같습니다(`agent_host.rs` 12-13 상수, `agent-cli.rs` 56·65·83·96).
    2. osascript 실행 파일: `agent-cli.rs` 9 `"osascript"`(원본과 같음) / `settings-ports.rs` 14 `"/usr/bin/osascript"`.
    3. 취소 판별: `agent_cli`는 취소를 `Ok(())`로 받아 결과 상태로만 추정합니다. runtime은 `CliCommandOutcome.is_cancelled`로 명시합니다. 그래서 "취소가 아닌데 끊어진 링크"(링크가 이미 번들 대상과 같은데 번들 안 `taide-cli`가 없는 손상 번들)에서 명령은 toast 없음, 설정 행은 성공 문구로 서로 다릅니다(commands-run QA 수정 절 "설치 Ok이지만 dangling일 때 toast 없음").
    4. `terminal_agent_state.cli_status`는 제품 코드에서 읽는 곳이 없습니다(2.1).
- 빠진 것: 기능표 349행 갱신. 1~4는 기능 누락이 아니라 같은 정책의 이중 구현입니다.

#### (d) 구현 방법

필수:

1. 근거 재실행: `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- agent_cli::tests application::terminal_agent_tests::실제_app_cli application::domains::settings::settings_interface_tests::실제_cli_행 --test-threads=2`(기대 8 passed).
2. 기능표 349행: 판정 `complete`. 비고 끝의 "남은 것" 문장을 "설정 화면 CLI 행은 settings-16(lane settings S1, `settings_ports::CliPorts` → runtime `agent_host::cli_install`/`cli_uninstall`, App `실제_cli_행은_가짜_포트로_취소_무음_설치_실패_해제를_원본_문구로_알리고_버튼을_바꾼다`). 실제 `/usr/local/bin` 설치는 허락 뒤 실기(P03)"로 바꿉니다. 정리를 하지 않으면 "명령과 설정 행이 같은 정책을 두 곳(`agent-cli.rs`, runtime `agent_host.rs`)에 두고 취소 판별 방식이 다름"을 한 문장 덧붙입니다.
3. 같은 근거로 commands-ui-32·33·34(기능표 671-673행, 지금 `unwired`)도 갱신 대상입니다. 소유는 commands-ui 영역이고 절차는 `16-remaining-commands-ui.md` 354행 절입니다. 이 영역 작업자가 함께 갱신할지는 메인이 정합니다.

선택(정리, 메인 결정 뒤. 결정 3건은 (g)):

4. `APP/src/agent-cli.rs`가 runtime 정책을 호출하게 바꿉니다.
    - `Osascript` 타입(31)을 `Arc<dyn Fn(&str) -> AppResult<taide_runtime::agent_host::AdminPrompt> + Send + Sync>`로 바꾸고, `Ports::system()`(40-46)은 `osascript: Arc::new(|script| taide_runtime::agent_host::run_admin_osascript(std::path::Path::new(OSASCRIPT_PROGRAM), script))`를 씁니다. `OSASCRIPT_PROGRAM`은 (g) 결정 2에 따릅니다.
    - `install`(62)·`uninstall`(80) 본문을 플랫폼 거절(`IS_SUPPORTED_PLATFORM`) 뒤 `taide_runtime::agent_host::cli_install(&(ports.executable)()?, &ports.link, |script| (ports.osascript)(script))` / `cli_uninstall(..)` 호출로 바꾸고 `AppResult<CliCommandOutcome>`을 돌려줍니다. 쓰이지 않게 된 `install_target`·`run_osascript`는 삭제합니다.
    - `run`(110)과 `Reply::Cli.result`(`terminal_agent.rs` 96)를 `AppResult<CliCommandOutcome>`로 바꿉니다. `Status`·살아 있는 `ConnectExternalEditor`는 `CliCommandOutcome { status, is_cancelled: false }`.
    - `finish_agent_cli`(467): 취소(`is_cancelled`)는 toast 없음(연결 명령은 원본처럼 결과 상태로 다시 판정해 `cliInstallFailed`). 취소가 아닐 때의 설치 규칙은 (g) 결정 3에 따릅니다. `terminal_agent_state.cli_status`에는 `outcome.status`를 넣습니다(검사 helper가 완료 신호로 씀).
5. `APP/src/settings-ports.rs` `CliPorts::default()`(130-139)가 `crate::agent_cli::install(&crate::agent_cli::Ports::system())`/`uninstall`을 부르게 바꾸면 osascript 경로·플랫폼 거절 문구가 한 곳이 됩니다. 이때 쓰이지 않게 된 `shell_command`(141-155), `ShellCommandChange`, `CLI_INSTALL_UNSUPPORTED`·`CLI_UNINSTALL_UNSUPPORTED`·`OSASCRIPT_PROGRAM`을 지웁니다.
6. 검사 갱신: `agent-cli-tests.rs`의 가짜 osascript가 `Ok(AdminPrompt::Completed | Cancelled)`를 돌려주게, `application-terminal-agent-tests.rs` `CliProbe::run`(1400-1418)이 `CliScript::Cancel => Ok(AdminPrompt::Cancelled)`, `Link | Hold => Ok(AdminPrompt::Completed)`를 돌려주게 바꿉니다. 단언(문구·호출 수·링크 상태)은 (g) 결정 3이 바꾸는 경우만 고칩니다. settings 검사(`CliPorts`를 클로저로 주입, `application-settings-interface-tests.rs` 103-106)는 바뀌지 않아야 합니다.

#### (e) 자동 검사 방법

- 필수 단계: (d) 1 명령의 통과 수를 QA에 적습니다.
- 선택 단계: 같은 명령 + `cargo check … --all-targets`(새 dead code 경고 없음) + `cargo fmt … -- --check`. 공유 crate는 바꾸지 않으므로 runtime 검사는 필요 없습니다. 결정 3이 A안이면 `agent-cli-tests.rs`에 "취소가 아닌데 끊어진 링크 → 설치 성공 문구" 단언 1건을 추가합니다(임시 폴더에 번들 대상 경로를 가리키는 symlink를 미리 만들고 번들에 `taide-cli` 파일을 만들지 않음 → `cli_install`이 스크립트를 부르지 않고 `Completed` → `is_cancelled == false`, `dangling == true`).

#### (f) 완료 판정 기준

- 필수 단계 명령이 통과하고 349행이 갱신되면 `complete`. 정리는 판정 조건이 아니지만, 하지 않으면 QA "남은 위험"에 이중 구현과 차이 1~4를 적습니다.

#### (g) 막는 요소와 결정

- 허락: 실제 `/usr/local/bin/taide` 설치·해제와 관리자 암호 대화상자(P03). 이 기계에는 실제 `/usr/local/bin/taide`가 있습니다(settings S1 QA 57행). 기본 포트를 쓰는 검사는 그 symlink 메타데이터를 읽기만 합니다.
- 메인 결정(정리를 할 때만, 한 번에 묻습니다)
    1. 정리 방향 — A안(추천: 공유 crate의 이미 검사된 함수 하나로 모음, 공유 crate 무변경) / B안(정리하지 않고 차이를 QA에 남김)
    2. osascript 실행 파일 — A안(추천: `/usr/bin/osascript` 절대 경로, PATH 바꿔치기 위험 제거, 설정 행이 이미 사용) / B안(원본과 같은 `"osascript"`)
    3. 취소가 아닌데 끊어진 링크의 설치 응답 — A안(추천: 설정 행과 같게 `!is_cancelled`면 성공 문구, 원본 `runCliInstall`도 결과 상태와 무관하게 성공) / B안(지금 명령처럼 살아 있는 링크일 때만 성공)

---

### 3.4 terminal-agent-64 설정 화면의 테스트 알림·시스템 설정 열기

#### (a) ID·기능·상태

- `terminal-agent-64` "설정 화면의 테스트 알림·시스템 설정 열기", `missing`, 근거 묶음 `terminal-base`(기능표 357행). 비고는 "… 알림 설정 버튼/안내의 실제 native 표면은 없습니다"라는 낡은 문구입니다.
- 같은 기능 행: settings-21 `complete`(570행), settings-22 `partial`(571행, 남은 것: 전달 성공·플랫폼 오류 [A]), settings-82 `complete`(631행, DEV 안내).

#### (b) 원본 TS 동작

- `TS/widgets/settings-view/settings-notification-section.tsx`
    - 115-124: 스위치 9개 아래 가로 묶음(`gap-2` = 8px)에 outline xs 버튼 2개. 첫째 ExternalLink + `settings.notificationsOpenSystemSettings`, 둘째 Bell + `settings.notificationsSendTest`. 항상 보입니다.
    - 46: 시스템 설정 열기 실패는 `toast.error(describeIpcError(error))`.
    - 48-63: 시험 알림은 `sendNativeNotification({ category: 'error'(35), title: t('settings.notificationsSendTest'), body: t('notification.enableHint') })`. 결과 `suppressed`면 `toast.warning(t(SUPPRESSION_REASON_LABEL_KEY[reason]))`(23-27: `notificationsDisabled` → `settings.notificationsEnabled`, `categoryDisabled` → `settings.notificationsError`, `windowFocused` → `settings.notificationsOnlyWhenUnfocused`), 전달이면 `toast.success(t('notification.enableHint'))`, 예외면 `toast.error(describeIpcError(error))`.
    - 125: DEV 빌드면 `settings.notificationsDevHint`(settings-82).
- `TS/entities/notification/notification.ipc.ts` 14-15: 시험 알림은 `notifyNative`를 거치지 않고 백엔드 게이트를 직접 부르므로 자르기·창 종류 검사·첫 전달 안내(`notification.enableHint` info toast)가 없습니다.
- 백엔드: `TAURI/domain/notification/commands.rs` `notification_notify`(29-40, 아무 창 포커스 계산 → `notification_actions::notification_notify`), `notification_open_system_settings`(53-67, macOS URL, 그 밖 `error.notification.settingsUnsupported`). 게이트는 `CR/taide-notification/src/service.rs` `decide_delivery`(17-28): 마스터 → 범주 → (`notifications_only_when_unfocused` && 포커스) 순서. 전달이면 `platform.send_notification`(`CR/taide-runtime/src/notification_actions.rs` 27-29), 그 오류는 그대로 올라옵니다.

#### (c) native 현재 자산과 빠진 것

- UI: `UI/src/settings-sections-view.rs` `notifications`(891-, 버튼 2개와 DEV 안내), `UI/src/settings-sections-data.rs` `Action::SendTestNotification`(166), `suppression_label`(198-204), `send_test_notification`(664, 제출 시점의 `is_any_window_focused`), 응답 → 알림(1164-1177: 열기 오류 → `Notice::Error`, 시험 결과 → `Delivered` = `Notice::Success("notification.enableHint")`, `Suppressed(reason)` = `Notice::Warning(suppression_label(reason))`, 오류 = `Notice::Error`).
- 앱: `APP/src/domains/settings.rs` 799-807(열기, `ports.open_notification_settings`), 808-829(시험, `notification_notify(&worker.state, worker.platform.0.as_ref(), NotificationCategory::Error, &title, &body, || any_window_focused)`를 `run_blocking_result`로). `Notice::Error`는 `taide_native_ui::toast::describe_error`(`UI/src/toast.rs` 1589-1609)로 제목을 만들며 `AppError::Internal(value)`는 `value` 그대로입니다. 포커스는 `tick_settings_reads`(`domains/settings.rs` 1403, `background_tick`에서 호출 `application.rs` 1124)가 `is_any_window_focused`(`domains/shell.rs` 1369-1377)로 `Data`에 매 프레임 기록합니다.
- 기존 검사: [C] `settings_ports::tests::알림_설정_열기는_macos에서만_원본_url을_열고_그밖은_지역화_오류로_거절한다`, [U] `알림_버튼은…`, data `시험_알림은…`, [A] `실제_알림_버튼은_가짜_설정열기와_시험알림_억제_사유를_원본_문구로_알린다`(`application-settings-interface-tests.rs` 443-518: 열기 1회·열기 오류, 억제 사유 3종 warning, 플랫폼 미호출).
- 빠진 것: 전달 성공과 플랫폼 오류의 실제 App 검사(settings-22 남은 것). settings lane worktree에는 플랫폼 주입이 없었지만 지금 메인에는 `Overrides { secret_store, platform }`(`application.rs` 76-79)과 `new_with_overrides`(313, 교체 348-356)가 있습니다. 설정 검사 `Scene::assemble`(58-146)은 아직 `new_with_secret_store`를 써서 플랫폼을 넣지 못합니다.
- 주의: 기본 플랫폼 `NativePlatform::send_notification`(`APP/src/bootstrap.rs` 171-184)은 macOS에서 실제 `/usr/bin/osascript`로 OS 알림을 띄웁니다. 플랫폼을 주입하지 않은 검사에서 시험 알림이 "전달"되면 실제 알림이 뜹니다. 기존 [A] 검사는 포커스·스위치 때문에 억제돼 플랫폼을 부르지 않았을 뿐입니다.

#### (d) 구현 방법 (검사 파일만 바뀝니다. settings 영역 파일)

이 행의 코드 작업은 settings-22의 남은 검사와 같습니다. 상세 절차는 `15-remaining-settings.md` settings-22 절 (d)·(e)를 그대로 따르고, 두 문서가 다른 설계를 내지 않도록 여기서는 요약만 적습니다.

1. `APP/src/application-settings-sections-tests.rs` `Scene`: `is_focused: bool` 필드와 `set_focused`, `step`의 `RawInput.focused`, `assemble`에 `platform: Option<Arc<dyn taide_runtime::PlatformServices>>` 인자, `NativeApplication::new_with_overrides(.., crate::application::Overrides { secret_store: Some(..), platform })`, `launch_with_platform(platform, configure)`, 공용 `RecordingPlatform { sent, refusal }`(15번 문서 코드 블록 그대로). 기존 `launch`·`launch_with`·`launch_empty_pane`은 `None`을 넘겨 동작이 같아야 합니다.
2. `APP/src/application-settings-interface-tests.rs`에 검사 1개: 포커스 없는 프레임을 한 번 지난 뒤 시험 버튼 클릭 → 플랫폼 기록 `[(settings.notificationsSendTest, notification.enableHint)]`, success `notification.enableHint` 1개, info 안내 없음, warning 없음 → `refusal = Some("fixture platform refused")` 뒤 다시 클릭 → error `fixture platform refused`, 기록 1개 유지.
3. 기능표: settings-22(571행) → `complete`(비고에 새 [A] 검사 이름, "남은 것" 삭제). terminal-agent-64(357행) → `complete`, 비고 예: "settings-21·22·82와 같은 구현(lane settings S1): 시스템 설정 열기(가짜 opener 1회·오류 toast, 비 macOS 지역화 오류), 시험 알림(범주 error, 억제 사유별 warning, 전달 success `notification.enableHint`·첫 전달 안내 없음, 플랫폼 오류 toast), DEV 안내. [A] `실제_알림_버튼은…`, `<settings-22 새 검사 이름>`. 실제 OS 알림·시스템 설정 앱은 허락 항목(P01·P02). `2026-11-02-native-lane-settings-s1-interface-performance.md`, `<새 QA 문서>`". 근거 묶음 열은 지금 값(`terminal-base`)을 유지합니다.

순서 판단: 기능표 571행이 이미 `complete`면 1·2를 건너뛰고 3의 64 행만 갱신합니다. 아직 partial이면 settings 영역 작업으로 1·2를 먼저 하고(같은 커밋에서 settings-22와 64를 함께 갱신), 다른 작업자가 settings-22를 진행 중이면 그 결과를 기다립니다.

#### (e) 자동 검사 방법

- `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- application::domains::settings:: --test-threads=2`(새 검사와 `Scene`을 쓰는 settings 검사 전체 회귀)
- `cargo fmt --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml -- --check`

#### (f) 완료 판정 기준

- 새 검사와 기존 settings 검사가 통과 → settings-22와 terminal-agent-64를 `complete`로 갱신.

#### (g) 막는 요소

- 없음. 실제 OS 알림 표시(P01)와 시스템 설정 앱 열기(P02)는 허락 항목이며 complete 조건이 아닙니다. 새 검사는 반드시 플랫폼을 주입해 만듭니다. 주입을 빠뜨리면 실제 알림이 뜹니다(작업자 금지 위반).

---

### 3.5 terminal-agent-68 openDiff → Claude diff 탭

#### (a) ID·기능·상태

- `terminal-agent-68` "openDiff → Claude diff 탭(원본·제안 비교, 제안 편집, 수락·거절, 탭 닫힘 시 거절)", `unwired`, 근거 묶음 `integrations`(기능표 361행, `GITWT` 기능표 361행도 같은 문구).
- 같은 기능 행(git 영역, `GITWT` 기능표 526-529행. 메인 기능표의 같은 줄은 아직 lane 이전 판정 unwired·missing): git-49 `complete`(요청 수신·탭 생성·`ideAutoOpenDiff`), git-50 `partial`(표면, 남은 것: 수정 쪽 revert 표시와 git-40 공통 장식), git-51 `complete`(Accept/Reject), git-52 `partial`(탭 닫힘 처리, 남은 것: 보조 창 이동 뒤 응답 없음·보조 창 Accept App 검사).

#### (b) 원본 TS 동작

- 수신 `TS/app/providers/ide-sync-provider.tsx` 49-68: `ide:diff-requested`마다 대기 목록에 `{ oldPath, newContents, tabName }` 기록(`TS/entities/ide/claude-diff-registry.ts` 9-11, 앱 전역 `Map`). 설정 `ideAutoOpenDiff`가 꺼져 있으면 목록에서 지우고 `resolveIdeDiff({ outcome: 'rejected', content: null })`(탭 없음, toast 없음). 켜져 있으면 `openTabInProject({ kind: claudeDiff { requestId, path: newPath }, title: tabName, target: null, preview: false })`. 실패는 `toast.error(describeIpcError)`. 131-133: `ide:close-tab-requested`에 `requestId`가 있으면 대기 목록에서 지움.
- 표면 `TS/widgets/claude-diff-pane/claude-diff-pane.tsx`
    - 141-152: 높이 32px(`h-8`) 막대(`border-app-border bg-editor-background border-b px-2`), 오른쪽 정렬, 간격 8px, Reject(outline xs, `ide.rejectChanges`) → Accept(default xs, `ide.acceptChanges`), 진행 중 둘 다 비활성.
    - 55-85: Monaco diff 편집기(`readOnly: false`). 원본 = 디스크 파일(`fileQueryOptions(path)`, retry 없음, 실패면 빈 문자열), 수정 = 마운트 때 대기 목록 내용(없으면 빈 문자열), 언어 `file.languageId ?? 'plaintext'`를 양쪽에 동기화.
    - 110-124 Accept: 수정 편집기의 현재 텍스트로 `saved` → 대기 목록 제거 → `toast.success(ide.diffAccepted)` → 탭 닫기. 실패 `toast.error`.
    - 126-139 Reject: `rejected` → 제거 → `toast.info(ide.diffRejected)` → 탭 닫기.
    - 26-36, 101-108: 언마운트 때 모든 창 트리(`collectAllPaneTabs`)에 탭이 없고 아직 미해결이면 `rejected`. 다른 탭으로 전환하거나 보조 창으로 옮긴 경우는 거절하지 않음.
    - 수정 쪽 hunk 되돌리기 표시: monaco 0.56(`NM/monaco-editor/package.json` 3)은 `renderGutterMenu: true`가 기본(`NM/monaco-editor/esm/vs/editor/common/config/diffEditor.js` 9)이라 예전 margin revert 화살표(`shouldRenderOldRevertArrows`)는 그려지지 않고(`.../widget/diffEditor/diffEditorOptions.js` 42-56), gutter 도구 막대의 "Revert Block" 항목이 보입니다. 상세는 `14-remaining-git.md` 3.5 (b).
- 백엔드 탭 닫힘: 사용자가 탭을 닫으면 대기 diff는 `TabClosed`(native `APP/src/tabs.rs` 136-140이 이미 같음).

#### (c) native 현재 자산과 빠진 것

- 메인(`to_rust_native`, HEAD `0c52b438`)
    - 생산: `APP/src/ide-tools.rs` `open_diff`(174-226)가 프로젝트 밖 경로면 즉시 `Rejected`(194-198), 아니면 대기 diff를 넣고 `AppEvent::IdeDiffRequested`(209)를 발행한 뒤 응답을 `IDE_DIFF_TIMEOUT`(27, 600초)까지 기다리고, 시간이 지나면 대기를 꺼내고 `Rejected`.
    - 소비: 없습니다. `IdeDiffRequested`를 읽는 제품 코드는 원격 중계 `APP/src/event-relay.rs` 170뿐이고(나머지는 검사), `APP/src/provider-events.rs`는 이 사건을 기록하지 않습니다. `ide_auto_open_diff`를 읽는 제품 코드는 설정 스위치(`UI/src/settings-controls.rs` 203·241)뿐입니다.
    - 표면: `APP/src/domains/editor.rs` 5397 → `APP/src/domains/git.rs` 141 `show_claude_diff_tab(ui, tab)` = `show_unavailable_tab`.
    - 결과: 지금 IDE의 `openDiff`는 탭 없이 10분 뒤 거절됩니다.
- git lane(`GITWT`, 커밋 `5195d415`·`50892b59` + 병합 중 미커밋 변경)
    - `git-claude-diff.rs`: `Event::{Requested, Closed}`(27-35), `State`(대기 목록·진행 중·줄 diff, 57-), `bar`(249, 막대), 상수 `BAR_HEIGHT` 32, `RECOMPUTE_DELAY` 200ms, `FALLBACK_LANGUAGE_ID` `"plaintext"`.
    - `domains/git.rs`: `accept_claude_diff_event`(설정 false → 대기 제거 + `Command::ResolveIdeDiff { tab: None, Rejected }`, true → `Command::OpenTab { kind: TabKind::ClaudeDiff { request_id, path }, title, target: None, preview: false }`), `tick_claude_diffs`(200ms 뒤 `Command::Hunks`), `show_claude_diff_tab(ui, project, pane, tab, intents)`, `Command::ResolveIdeDiff` → `Reply::IdeDiffResolved`(성공이면 대기 제거·`ide.diffAccepted` success 또는 `ide.diffRejected` info·탭 닫기 요청, 실패면 오류 toast).
    - 수신 경로: 병합 작업 트리에서 lane 원래의 `git_cache::Changes.ide`가 shell의 공용 수신함으로 옮겨져 있습니다(`GITWT/.../provider-events.rs` 37 `IdeDiff(crate::git_claude_diff::Event)`, 기록 90-107, `GITWT/.../domains/shell.rs` 1161 `Event::IdeDiff(event) => self.accept_claude_diff_event(event)`, `git-cache.rs`에는 `IdeDiff` 기록이 없음). 병합 커밋 뒤 다시 확인합니다(14번 문서 3.7).
    - 검사: `application-git-claude-history-tests.rs`의 `실제_app_ide_open_diff는_claude_diff_탭을_열고_수정_쪽_편집_뒤_accept로_저장한다`(289), `실제_app_claude_diff는_자동_열기_끄기_reject_탭_닫기_ide_close_tab을_원본대로_처리한다`(372), 단위 `git-claude-diff-tests.rs`.
- 빠진 것: (1) git lane 병합, (2) git-50 수정 쪽 "Revert Block" gutter 도구 막대·git-40 공통 장식, (3) git-52 보조 창 App 검사, (4) 이 행과 69행 갱신.

#### (d) 구현 방법 (이 영역은 확인과 행 갱신. 코드는 git 영역)

1. 병합 확인: `git -C /Users/hyunseokbyun/development/TAIDE log --oneline -10`에 git lane 병합 커밋(`merge(native): git lane 병합` 형식)이 있고 `ls /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/git-claude-diff.rs`가 있으면 진행합니다. 없으면 기다립니다(병합 worktree는 건드리지 않습니다).
2. 단일 소비자 확인: `grep -rn "IdeDiffRequested" /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src`의 검사 아닌 결과가 생산(`ide-tools.rs`)·원격 중계(`event-relay.rs`)·수신함 기록(`provider-events.rs`) 3곳뿐이어야 합니다. `git-cache.rs`나 `git-claude-diff.rs`에 기록이 남아 있으면 한 요청이 두 번 소비돼 탭이 둘 생기므로 git 영역에 보고합니다.
3. 표면 확인: `grep -n "fn show_claude_diff_tab" /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/domains/git.rs`가 5인자 버전 하나이고, 그 본문이 ClaudeDiff가 아닐 때만 `show_unavailable_tab`을 부르는지 봅니다.
4. 검사 실행((e)).
5. 행 갱신(메인이 14번 문서 3.7 (f)와 같은 커밋에서 하는 것을 권장)
    - git-50·git-52가 아직 partial이면 68은 `partial`. 비고 예: "git-49~52와 같은 구현(git lane g7): IDE `openDiff` → 공용 수신함(`provider_events::Event::IdeDiff`) → 대기 목록 → `ideAutoOpenDiff` 꺼짐이면 탭 없이 Rejected, 켜짐이면 ClaudeDiff 탭(제목 tab_name, preview false). 표면은 32px Reject/Accept 막대와 diff(원본 = 디스크 읽기 전용, 수정 = 요청 내용 편집 가능, 200ms 재계산), Accept → Saved(편집 결과)·success·탭 닫힘, Reject → Rejected·info, 사용자 탭 닫기 → TabClosed, IDE close_tab → 대기 제거. [A] `실제_app_ide_open_diff는…`, `실제_app_claude_diff는…`. 남은 것: 수정 쪽 Revert Block gutter 도구 막대(git-50), 보조 창 이동 뒤 응답 없음·보조 창 Accept App 검사(git-52). `2026-11-02-native-lane-git-g7-commit-claude-diff.md`."
    - git-50·52가 complete가 되면 68을 `complete`로 바꾸고 "남은 것"을 지웁니다.
6. 69행(`ideAutoOpenDiff` 반영, 지금 complete) 비고를 git-49 근거(설정 false → 탭 없이 Rejected App 검사 `실제_app_claude_diff는_자동_열기_끄기…`)로 바꿉니다. 지금 비고는 "배치 13 회귀 통과" 일반 문구이고 메인에는 실제 소비자가 없었습니다(lane 사양 1.3 "git 사양 1.3은 terminal-agent-69가 실제로는 로컬 소비자가 없다고 적었습니다").

#### (e) 자동 검사 방법

- 병합 뒤 메인 트리에서: `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- application::domains::git::claude_history_tests git_claude_diff provider_events application::terminal_agent_tests::실제_app_ide_close_tab --test-threads=2`
    - 마지막 필터는 이 영역 terminal-agent-71 검사(`application-terminal-agent-tests.rs` 1181, ClaudeDiff 탭 2개 `closeAllDiffTabs` → `TabClosed`)로, ClaudeDiff 본문이 실제 표면으로 바뀐 뒤에도 닫힘 경로가 그대로인지 확인합니다.
- 새 검사는 이 영역에서 쓰지 않습니다. git-50·52의 남은 검사는 git 영역이 씁니다(14번 문서 3.5 (e), 3.6 (e)).

#### (f) 완료 판정 기준

- 병합 + 위 검사 통과 → `partial`(git-50·52 남음).
- git-50·52 complete → `complete`.

#### (g) 막는 요소와 처리

- git lane 병합: 다른 작업자가 `GITWT`에서 충돌을 해결 중입니다. 이 영역 작업자는 그 worktree를 고치지 않고, 병합 커밋이 `to_rust_native`에 들어온 뒤 (d)를 진행합니다.
- git-50 Revert Block: git 사양 결정 4 A안(`docs/research/2026-10-28-lane-git-spec.md` 722행)으로 범위에 포함됐고 git 영역 소유입니다. 아이콘 색은 원본 화면 확인이 필요합니다(14번 문서 3.5 (g)).
- 실기(P08·P23): 실제 Claude Code 클라이언트와의 openDiff 왕복. complete 조건이 아닙니다.

---

## 4. 이 영역에서 반복된 함정과 주의

1. 합성한 입력 순서를 실제 OS 순서로 믿지 않습니다. 19의 첫 판정(complete)은 `[Key{E, alt}, Preedit]`를 합성한 검사로 냈다가 리뷰에서 macOS는 dead key에 `Key`를 보내지 않는다는 사실(winit `view.rs` 466-497)이 드러나 partial로 내려갔습니다. 입력 검사는 winit·egui-winit 소스(`REG` 아래)에서 사건 순서를 먼저 확인하고, AppKit 내부에 달린 부분은 실기 항목으로 분리합니다.
2. 이 기계에는 실제 `/usr/local/bin/taide`가 있습니다. 기본 포트를 쓰는 설정 App 검사는 "Installed" 문구가 섞여 다른 검사의 문구 수·대기 조건이 흔들렸습니다(settings S1 QA 57행). CLI 상태가 화면에 나오는 검사는 `missing_cli_agents()` 또는 주입 포트를 씁니다.
3. 기본 플랫폼은 실제 OS 알림을 띄웁니다(`bootstrap.rs` 171-184). 시험 알림·알림 생산자 검사는 반드시 `Overrides.platform`으로 가짜 플랫폼을 넣습니다.
4. hooks App 검사의 프로젝트는 `root_missing`으로 넣습니다. 붙은 프로젝트면 시작 watcher가 `agent_hooks::reconcile_installed`를 실제 `HOME`으로 돌려 사용자 홈 hooks를 고칠 수 있습니다(settings S1 QA 57행).
5. 실제 osascript·`/usr/local/bin` 쓰기·Keychain·휴지통·GUI 실행·합성 OS 입력은 작업자 금지입니다. `APP/tests/explorer-delete.rs`는 실행하지 않습니다(이전에 합성 파일이 사용자 휴지통에 들어간 기록, `docs/PROCESS.md` 19행. 병합 게이트 스크립트 9행은 이 검사를 빼는 인자가 없으므로 허락(P10) 전에는 `--skip 실제_탐색기_삭제는`를 붙입니다, `30-pending-permissions.md` R1).
6. host worker를 막지 않습니다. 관리자 대화상자처럼 사람이 응답할 때까지 걸리는 작업은 `spawn_transient` + `run_blocking_result`로 떼어 냅니다. 56 App 검사는 osascript를 붙잡은 동안 `task.runTask` 감지가 먼저 끝나는지로 이것을 확인합니다.
7. 새 `egui::Context`는 시작 때 repaint 1회가 예약돼 있습니다(`EGUI/src/context.rs` 362 `outstanding: 1`). "repaint 요청 없음"을 단언하기 전에 프레임을 몇 번 돌려 예약을 비웁니다. 직접 `run_ui`한 출력은 `output.textures_delta.clear()`를 하지 않으면 epaint가 drop 때 panic합니다(ide-agent-display QA "51 검사 수정").
8. 앱 lib 병렬 실행에서 실제 App 검사가 host 큐(64)를 채워 흔들립니다(PROCESS.md E2 관찰, 2026-11-03 병합 게이트에서 lib 실패 4~5건이 직렬 재실행에서 통과). 필터 실행은 `--test-threads=2`, 게이트는 4로 하고, 실패하면 그 검사만 단독으로 다시 돌려 흔들림과 결함을 가립니다.
9. 알려진 범위 밖 실패: `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`(기준 커밋부터 있던 결정적 실패로 판단, terminal-core QA 수정 절), `CR/taide-runtime/tests/agent_probe.rs` 소스 문자열 단언 1건(`81e96198`부터, PROCESS.md 32행). 이 둘을 새 실패로 보고하지 않습니다.
10. 앱 lib를 넓게 돌리기 전에 `native-lsp-mock` 예제를 빌드합니다. 없으면 LSP 검사 10건이 실패합니다(`docs/memory/2026-10-07-native-transition-operations.md` 6절).
11. terminal-agent App 검사 모듈과 `HostBridge::connect_with_agent_cli`는 `#[cfg(all(test, target_os = "macos"))]`이고 `agent-cli-tests.rs`는 `#[cfg(all(test, unix))]`입니다. 이 조건은 lane 리뷰에서 비 unix·비 macOS 검사 빌드 컴파일 오류를 고친 결과입니다(commands-run QA 수정 절 "검사 이식성"). 새 helper도 같은 조건 안에 둡니다.
12. 통합 검사 fixture 바이트(`TERM/tests/fixtures/session.rs`의 `META_INPUT` 등)는 bin `native-terminal-queue-fixture`와 공유됩니다. 바꾸면 검사와 같은 커밋에서 함께 바꿉니다.
13. 기능표 판정이 실제 코드보다 앞서거나 뒤처진 사례가 이 영역에 여럿 있었습니다. 다른 lane 병합 직후 관련 행을 다시 확인합니다.
    - 뒤처짐: 55·56·64는 settings lane 병합 뒤에도 갱신되지 않았습니다. commands-ui-32~34도 56 구현 뒤 `unwired` 그대로입니다.
    - 앞섬: 69(`ideAutoOpenDiff`)는 `complete`이지만 메인에는 소비자가 없습니다(3.5 (c)). 6·26·45는 lane 전에는 원본에 없는 기능이 missing으로 남아 있었습니다.
    - 비고가 실제와 다름: 56 비고의 "`terminal_agent_state.cli_status` 소비"는 일어나지 않았습니다(3.3 (a)).
    - 기능표 머리말 집계(9행 문장, 15행 표)는 2026-10-14 값입니다. 영역 수는 1.1의 명령으로 셉니다.
14. 사양과 다른 소유·설계가 실제로 들어간 경우(56의 CLI 백엔드 위치, 68의 수신함 위치)는 사양 문서보다 실제 코드를 따르고 차이를 QA에 남깁니다.
15. git lane 병합이 이 영역 검사 파일도 바꿉니다: `application-terminal-agent-tests.rs`의 `PaintSink` 리터럴(51 검사, 873행 부근)에서 `title_branches` 필드가 `git_cache`로 바뀝니다(`git -C /Users/hyunseokbyun/development/TAIDE-lanes/git diff`로 확인). 병합 뒤 `application.rs` 줄 번호도 약 14줄 밀립니다. 병합 뒤 이 영역 작업을 시작할 때는 줄 번호가 아니라 심볼로 다시 찾습니다.
16. 같은 vendored 변경을 두 영역이 따로 설계하지 않습니다. 보조 창 창 핸들은 shell-5가 `eframe::immediate_viewport_window()`로 만들고(10번 문서 3.2 (d) 2.1), 19 2단계는 그것을 호출만 합니다. 보조 창 raw input hook(마우스 보고)도 shell-5 범위입니다.
17. 실행 중 scrollback의 `rejected_history`는 값 기준이라, 좁은 창에서 거절된 값은 창을 넓혀도 설정값이 바뀌기 전까지 다시 적용되지 않습니다(terminal-core QA 테스트 부채). 이 영역 남은 행은 아니지만 터미널 코어를 고칠 때 함께 보는 위험입니다.
18. 검사 helper에 `///` 설명을 쓰지 않습니다(공개 API가 아님). 기존 settings 검사 helper에 남은 `///`(`application-settings-sections-tests.rs` 43-44·53·307-308)는 범위 밖 기존 주석이라 지우지 않습니다.

---

## 5. 같은 근거로 함께 갱신할 다른 영역 행

| 행 | 지금 | 근거 | 갱신 조건·주체 |
| --- | --- | --- | --- |
| settings-22 | partial | 3.4의 새 App 검사 | settings 영역 행. 15번 문서 settings-22 절. 64와 같은 커밋에서 메인이 갱신 |
| commands-ui-32·33·34(`cli.*`) | unwired | 56의 명령 경로와 App 검사(`2026-11-02-native-lane-terminal-agent-commands-run.md`) | commands-ui 영역(16번 문서 354행 절) 또는 메인 |
| terminal-agent-69 | complete(근거 낡음) | git-49(`실제_app_claude_diff는_자동_열기_끄기…`) | git lane 병합 뒤 비고만 갱신(14번 문서 3.7 (f)) |
| git-49~52 | 메인 기능표는 lane 이전 판정, `GITWT`는 49·51 complete, 50·52 partial | `GITWT` g7 QA | git 영역. 68의 판정이 이 행들을 따름 |
| shell-5 | partial | 19 2단계가 쓰는 vendored eframe 창 접근 | shell 영역(10번 문서 3.2). 19 2단계의 선행 |
