# native lane terminal-agent 묶음 3/5: commands-run

> 작성일: 2026-11-02 · lane: `terminal-agent` (worktree `TAIDE-lanes/terminal-agent`, 브랜치 `lane/terminal-agent`)
> 기준: `docs/research/2026-10-28-lane-terminal-agent-spec.md` 2.0절·2.16~2.21절과 3절 묶음 3, 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 결정은 추천안: 결정 2 A안, 결정 7 A안), 구조 `docs/research/2026-10-28-native-lane-seams.md`

## 대상 ID와 판정

- [x] terminal-agent-47 Run Selected Text in Terminal → complete (editor-82 근거, 이 worktree에서 재실행)
- [x] terminal-agent-48 run-in-terminal 쓰기 경로 → complete (editor-82·shell-24 근거, 이 worktree에서 재실행)
- [x] terminal-agent-56 CLI 설치 상태·설치·제거·외부 에디터 연결 명령 → partial (명령 3종·host·toast 완료, 설정 행 settings-16 남음)
- [x] terminal-agent-76 Task Runner 대화상자 → complete (보조 창 실제 App 검사 추가)
- [x] terminal-agent-77 `task.runTask` 명령 → complete (shell-24 근거, 이 worktree에서 재실행)
- [x] terminal-agent-78 태스크 감지(npm·make·cargo) → complete (shell-24, taide-task 단위 검사, 76 보조 창 검사의 세 출처)

## 원본 동작 근거

ID별 원본 근거는 사양 2.16~2.21절을 따릅니다(56: `src/entities/agent/agent.commands.ts` 18-66, `src-tauri/src/domain/agent/commands.rs` 86-235, 76: `src/widgets/task-runner/task-runner-dialog.tsx` 29-74, `src/app/app.tsx` 65-89). 구현 중 사양과 다르게 정한 점과 그 근거만 적습니다.

- 56 host 작업 실행 방식: 사양 2.18은 `spawn_transient` 안에서 `run_nonabortable_result`를 쓰라고 적었지만 `run_nonabortable_result`는 async future를 받습니다. CLI 작업은 파일 시스템 호출과 `osascript` 프로세스 대기(관리자 대화상자 응답까지)로 모두 blocking이라, async runtime worker를 막지 않도록 `spawn_transient("native-agent-cli")` 안에서 `run_blocking_result`로 blocking pool에 넘깁니다. host worker는 즉시 `None`을 돌려주고 다음 명령을 처리합니다(2.0.2 규칙). spawn이 거절되면(종료 중) 즉시 `Reply::Cli { result: Err(stopped) }`를 돌려줍니다.
- 56 포트 주입: 사양은 shell 사양 2.5의 override 방식을 가리키지만 그 override는 shell lane worktree에만 있습니다. 이 lane은 `HostIntegrations.agent_cli: Arc<agent_cli::Ports>`(실제 연결 3곳 모두 `Ports::system()`)와 `HostBridge::connect_with_agent_cli`(검사 전용, 리뷰 반영 뒤 `#[cfg(all(test, target_os = "macos"))]`)만 추가했습니다. 검사는 실행 중인 App의 bridge를 이 연결로 교체합니다(`application-context-menu-tests.rs` `clipboard`와 같은 방식).
- 56 비 macOS 분기: 원본 Tauri는 `#[cfg(not(target_os = "macos"))]` 별도 함수입니다. native는 함수 안에서 `cfg!(target_os = "macos")` 상수로 먼저 거절해, 플랫폼마다 쓰이지 않는 필드·함수 경고 없이 같은 오류 문구를 돌려줍니다.
- 56 toast 판정(결정 7 A안)
    - `cli.installShellCommand`: 결과가 `installed && !dangling`일 때만 `settings.cliInstallSuccess`. Ok이지만 아직 미설치(관리자 대화상자 취소, 원본 `is_osascript_user_cancelled`)는 toast 없음. 오류는 `settings.cliInstallFailed`.
    - `cli.uninstallShellCommand`: 결과가 `!installed`일 때 `settings.cliUninstallSuccess`. Ok이지만 아직 설치됨(취소)은 toast 없음. 오류는 `settings.cliUninstallFailed`.
    - `cli.connectExternalEditor`: 원본과 같이 결과 상태를 다시 보고 `installed && !dangling`이면 `settings.cliExternalEditorConnected`, 그 밖(취소·오류)은 `settings.cliInstallFailed`.
    - 모든 Ok 상태는 `terminal_agent_state.cli_status`에 저장합니다(settings-16 행이 읽음). 오류는 `log::warn`에 종류만 남깁니다.
- 56 제출 실패: `apply_terminal_agent_intent`는 `egui::Context`를 받지 않으므로(intent 루프 라우팅 블록은 손대지 않음) 응답 처리와 제출 실패가 같은 `finish_agent_cli`를 부릅니다. 제출 실패면 공용 `submit()`의 `report_once`와 함께 해당 실패 toast가 보입니다(사양 2.18).
- 76: 사양 2.19의 "실패하면 `domains/commands.rs`의 scope를 고침"은 필요하지 않았습니다. 팔레트는 `show_overlays`가 overlay viewport(보조 창)에서 그리고, `Action::RunTask`는 그 pass의 `self.shell.scope`(보조 창 scope)로 `TerminalDestination`을 만들므로 첫 실행에서 통과했습니다. `domains/commands.rs`는 바꾸지 않았습니다.
- 48: 원본 `terminal-write-bridge.ts`의 500건·30초 상한은 내부 메모리 한도이고 native는 탭 닫힘·attach 실패에서 대기를 버려 같은 목적을 이룹니다(사양 2.17 판단). 수치는 재현하지 않았습니다.

## 구현

- 새 `native/taide-native-app/src/agent-cli.rs`(`crate::agent_cli`, `lib.rs`에 모듈 한 줄)
    - `Action { Status, Install, Uninstall, ConnectExternalEditor }`, UI `CliAction` → `Action` 변환
    - `Ports { executable, link, osascript }`, `Ports::system()`(`std::env::current_exe`, `agent_hooks::TAIDE_CLI_TARGET_PATH`, 원본 129-142와 같은 `osascript` 판정)
    - `status`·`install`·`uninstall`·`run`: 원본 분기·오류 문구(dev build, 소유 아님, macOS 전용) 그대로. 공유 crate(`taide_agent::service`, `taide_runtime::agent_host::cli_install_status`)는 바꾸지 않고 재사용(결정 2 A안)
- `domains/terminal_agent.rs`: `Command::Cli(Action)`, `Reply::Cli { action, result }`, `State.cli_status`, worker arm(위 방식), `apply_terminal_agent_reply`의 `Reply::Cli`, `apply_terminal_agent_intent`의 `Intent::Cli`, `finish_agent_cli`
- 공용 파일(추가만): `host.rs` `HostIntegrations.agent_cli` 필드와 3개 리터럴 초기값, `#[cfg(test)] connect_with_agent_cli`. `application.rs` `terminal_agent_state` 리터럴 끝 `cli_status: None`
- `shell_keymap.rs` `intent`: `Run::Cli(action)` → `ShellIntent::TerminalAgent(Intent::Cli(action))`(프로젝트 없이도 실행, 원본과 같음)
- UI: `intents/terminal_agent.rs` `CliAction`과 `#[cfg(feature = "native-host")] Intent::Cli`, `command-registry.rs` `#[cfg(feature = "native-host")] Run::Cli`와 `execution`의 세 id 연결. 활성 조건은 기존 `Always`, mac 전용은 카탈로그 `platform`
- 47·48·77·78: 제품 코드 변경 없음. 근거 검사를 이 worktree에서 다시 실행했습니다.

## 실제 앱 경로

- 56: `main.rs` → `NativeApplication::new` → `HostBridge::connect_with_application_ports`(`HostIntegrations.agent_cli = Ports::system()`) → ⇧⌘P 팔레트 `Action::RunCommand(id)` → `commands_state.palette_commands` → 다음 overlay pass의 `command_dispatch::intent_with_editor` → `shell_keymap::intent(Run::Cli)` → `ShellIntent::TerminalAgent(Intent::Cli)` → `apply_terminal_agent_intent` → `HostCommand::TerminalAgent(Command::Cli)` → host worker `dispatch` → `spawn_transient` + `run_blocking_result` → `agent_cli::run` → `HostReply::TerminalAgent(Reply::Cli)` → `poll()` → `apply_terminal_agent_reply` → `finish_agent_cli`(toast·`cli_status`). 사용자 키 재지정(`actionId: "cli.*"`)도 `command_dispatch::accepts`가 프로젝트 없이 받습니다.
- 76: 보조 창 viewport 키 ⇧⌘P → 보조 창 pass 키맵 → `Intent::OpenPalette` → overlay viewport(보조 창)에서 팔레트 → `task.runTask` → `Intent::OpenTaskRunner(보조 창 프로젝트)` → `Command::DetectTasks` → `show_tasks` → 선택 → `Action::RunTask`(보조 창 scope의 포커스 pane) → `Command::RunTask` → `destination_terminal_target`(활성 > 첫 > 새 탭, `cwd` = task cwd) → `Reply::EditorTerminalTarget` → `editor_terminal_writes`.
- 47·48·77·78: 근거 QA의 실제 앱 경로(`2026-10-12-native-batch23-terminal-run.md`, `2026-10-11-native-batch23-context-menu-app.md`, `2026-10-28-native-shell-keymap-dialogs-slot-header.md`).

## 자동 검사

- 앱 lib 단위 `agent_cli::tests`(macOS 6건, 임시 폴더·가짜 osascript)
    - `설치는_번들_밖_실행파일을_dev_build_오류로_거절하고_관리자_스크립트를_부르지_않는다`: 설치·제거 모두 dev build 오류, 스크립트 0회
    - `실제_포트는_원본_링크_경로를_쓰고_번들이_아닌_개발_빌드에서_링크를_건드리기_전에_거절한다`: `Ports::system()`의 링크가 `/usr/local/bin/taide`, 검사 실행 파일은 번들 밖이라 설치·제거가 링크·osascript 전에 dev build 오류(실제 osascript 미호출)
    - `설치는_미설치일_때만_관리자_스크립트로_번들_cli에_연결하고_결과_상태를_돌려준다`: 스크립트 1회(대상·링크 경로, `with administrator privileges`), 결과 installed·!dangling·`resolved_path`=번들 `taide-cli`. 같은 대상이면 스크립트 0회
    - `관리자_대화상자_취소는_오류없이_미설치_상태를_돌려주고_실패는_그대로_전한다`
    - `제거는_없는_링크를_그대로_두고_taide_소유가_아닌_링크를_지우지_않으며_소유_링크만_지운다`
    - `외부_에디터_연결은_살아있는_링크면_설치하지_않고_끊긴_링크면_다시_설치한다`: 살아있는 링크는 번들 밖 실행 파일이어도 설치하지 않음, dangling은 설치 1회
    - 비 macOS 분기 검사 1건은 `cfg(not(target_os = "macos"))`라 이 macOS 호스트에서는 실행되지 않습니다.
- 앱 lib 실제 App `application::terminal_agent_tests`(가짜 osascript 포트, 실제 팔레트)
    - `실제_app_cli_셸_명령은_팔레트에서_실행되어_원본_toast를_보이고_취소는_성공_문구를_보이지_않는다`: 설치 취소 → toast 없음·미설치 / 설치 실패 → `cliInstallFailed` / 설치 → `cliInstallSuccess`·링크 생성 / 연결(살아있음) → `cliExternalEditorConnected`·스크립트 추가 없음 / 제거 → `cliUninstallSuccess`·링크 제거 / 연결 취소 → `cliInstallFailed`. 마지막으로 설치 osascript를 channel로 붙잡은 동안 팔레트 `task.runTask` → `DetectTasks` 결과(npm `build`)가 먼저 보이고 CLI 응답·toast는 아직 없음, 풀면 `cliInstallSuccess`(host worker 비차단)
    - `실제_app_보조창_task_runner는_그_창에_세_출처를_보이고_그_창_포커스_pane_새_터미널로_실행한다`: 메인 창 미포커스·보조 창(같은 프로젝트, 파일 탭만) 포커스. 보조 viewport에 ⇧⌘P → overlay viewport가 보조 창. `task.runTask` → placeholder `task.runTask`, 행 7개(`build` npm, `lint` make, `cargo build/test/run/check/clippy` cargo)와 출처 문구가 보조 viewport 출력에만 그려지고 메인 출력에는 없음. `cargo check` 선택 → 팔레트 닫힘, 보조 창 포커스 pane에 `terminal.title`·`cwd`=프로젝트 root 새 터미널 탭(활성), `editor_terminal_writes`에 `cargo check\n`, 메인 트리에는 터미널 탭 없음
- 앱 lib 단위: `command_dispatch` 검사에 `cli.*` 3종이 프로젝트 없이 수락되고 메인·보조 문맥 모두 `ShellIntent::TerminalAgent(Intent::Cli(..))`로 해석되는 단정 추가
- UI 단위 `command_registry::tests::cli_세_명령은_mac_전용_셸_명령_범주로_native_host에서만_cli_실행경로를_갖는다`: mac 전용·셸 명령 범주·`Always`, native-host에서 `Run::Cli`, 아니면 `Unavailable`. 기존 실행 경로 표 검사에 `Run::Cli` 제외 arm 추가
- 재실행한 근거 검사: `application::context_menu_tests`의 `실제_터미널_` 4건(같은 pane 기존 탭 우선·새 탭 준비 보존, 팔레트·재지정 키, 추가 문서 종류·readonly, peek·보조 창), `실제_추가_경계_터미널_연결_실패는_전달을_회수하고_다음_터미널로_재전송하지_않는다`, `실제_추가_경계_종료한_터미널과_앱은_대기_전달을_회수한다`, `application::shell_dialogs_tests::실제_app_태스크_실행은_팔레트_명령에서_포커스_프로젝트_태스크를_감지해_포커스_pane_터미널로_보낸다`, `crates/taide-task` lib 16건(cargo 고정 명령·세 출처 합침 포함)

## 검증 명령과 결과

모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/terminal-agent`.

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --manifest-path native/taide-native-ui/Cargo.toml --all-targets --features native-host,inspection` | 통과, 기존 경고 1개(`tests/workbench.rs` `mut`)만 |
| `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets` | 통과, 기존 경고 3개(`is_applying`, `zen.rs` `mut`, `fixture_with_emmet_peek`)와 vendor `wry-preview` 경고만 |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib -- command_registry` | 16 통과 |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --no-default-features --features inspection --lib -- command_registry` | 6 통과(새 검사의 native-host 아닌 분기 포함) |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --features inspection --lib -- agent_cli command_dispatch shell_keymap --test-threads=2` | 21 통과(실제 포트 검사 추가 전) |
| `cargo test ... --features inspection --lib -- application::terminal_agent_tests::실제_app_cli application::terminal_agent_tests::실제_app_보조창_task_runner --test-threads=2` | 2 통과 |
| `cargo test ... --features inspection --lib -- application::context_menu_tests::실제_터미널_ application::context_menu_tests::실제_추가_경계_터미널_연결_실패 application::context_menu_tests::실제_추가_경계_종료한_터미널 application::shell_dialogs_tests::실제_app_태스크_실행 agent_cli --test-threads=2` | 13 통과(`agent_cli` 6건 포함) |
| `cargo test ... --features inspection --lib -- application::terminal_agent_tests --test-threads=2` | 8 통과(묶음 1·2 검사 6건과 이 묶음 2건, 공유 검사 파일 회귀 확인) |
| `cargo test --quiet --manifest-path crates/taide-task/Cargo.toml --lib` | 16 통과 |
| `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 통과, remote-web 기존 dead code 경고만 |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check`, UI 같은 명령 | 처음 실행에서 이 묶음이 추가한 줄만 차이 → `cargo fmt`로 맞춘 뒤 통과(다른 파일 변경 없음) |

- 실패 검사 순서: 56은 구현 전 `cli.*`가 `Execution::Unavailable`이고 host 명령이 없어 새 검사들이 컴파일되지 않는 상태였습니다. 검사와 구현을 함께 작성했고, 구현 전 상태로 되돌려 빨간 실행을 따로 하지는 않았습니다. 76 검사는 제품 변경 없이 첫 실행에서 통과했습니다(위 원본 동작 근거 76).
- 필터 없는 앱 전체 테스트는 메인 병합 때 실행합니다.

## 남은 범위

- 56: 설정 화면 CLI 행(settings-16)은 settings lane이 만듭니다. 상태 조회는 `HostCommand::TerminalAgent(Command::Cli(agent_cli::Action::Status))`, 결과는 `terminal_agent_state.cli_status`를 읽으면 됩니다(결정 2 A안, 공유 crate 무변경). 행이 붙으면 56을 complete로 바꿉니다.
- 56 실기(허락 필요, 사양 4.2 1): 실제 `/usr/local/bin/taide` 설치·제거와 관리자 암호 대화상자. 검사는 가짜 포트로만 했고 실제 osascript는 부르지 않았습니다.
- commands-ui-32~34·40·41 행은 commands-ui lane이 같은 근거로 갱신합니다(사양 1.4). 이 묶음은 terminal-agent 행만 갱신했습니다.
- 실제 PTY의 task 출력과 실제 키보드 입력은 배치 31 실기입니다.

## 수정 (lane 리뷰 반영, 2026-11-03)

차단 항목은 없었습니다. 이 묶음과 lane 공통에 해당하는 비차단 항목을 실제 코드로 확인해 처리했습니다. 판정(56 partial, 나머지 complete)은 바뀌지 않습니다.

### 검사 이식성 (실제 결함, 수정)

- `agent-cli-tests.rs`는 `Fixture::linking_ports`와 여러 검사에서 `std::os::unix::fs::symlink`를 조건 없이 쓰는데 `agent-cli.rs`의 `mod tests` 연결이 `#[cfg(test)]`뿐이라 unix가 아닌 대상의 검사 빌드가 컴파일되지 않습니다. 연결 조건을 `#[cfg(all(test, unix))]`로 바꿨습니다. macOS가 아닌 unix에서도 `macos가_아닌_빌드는_설치와_제거를_지원하지_않는다는_오류를_돌려준다`가 계속 컴파일·실행됩니다.
- `application-terminal-agent-tests.rs`의 CLI App 검사는 macOS에서만 설치가 허용되는 것을, task runner 보조 창 검사의 `command_shift()`(`MAC_CMD | COMMAND | SHIFT`)는 macOS 키 배치를 가정하는데 연결 조건이 `cfg(all(test, unix))`였습니다. 모듈 연결(`application.rs`)을 `#[cfg(all(test, target_os = "macos"))]`로 바꾸고, 이 모듈만 쓰는 `HostBridge::connect_with_agent_cli`(`host.rs`)도 같은 조건으로 맞춰 다른 플랫폼 검사 빌드에서 쓰이지 않는 함수가 남지 않게 했습니다. 기존 선례는 `motion-preference.rs` 82의 `cfg(all(test, target_os = "macos"))`입니다.
- 남은 것: macOS가 아닌 대상으로 실제 컴파일하지는 않았습니다(해당 target std가 없고 오프라인). 근거는 위 cfg 조건의 정적 확인입니다. 이 모듈이 쓰는 다른 검사 모듈의 helper(`project_rail_tests` 등)가 macOS가 아닌 unix에서 쓰이지 않게 되는지는 확인하지 않았습니다.

### 검사 helper의 `///` 설명 제거

- `application-terminal-agent-tests.rs`의 `status_message`·`call_ide_tool` 위 `///` 설명 3줄을 지웠습니다(공개 API가 아닌 검사 helper, 주석 규칙). lane이 새로 만든 다른 검사 파일(`application-commands-tests.rs`, `agent-cli-tests.rs`, 터미널 crate `tests/history.rs`)에는 `///`가 없음을 확인했습니다.

### `Action::Status`·`State.cli_status`의 dead_code 여부

- 비검사 코드에서 `Action::Status`를 만드는 곳과 `cli_status`를 읽는 곳은 없습니다(`agent-cli.rs` 112의 match, `domains/terminal_agent.rs` 475·490의 match·쓰기뿐). settings lane이 settings-16 행에서 만들고 읽습니다.
- 이 worktree에서 `cargo check --quiet --all-targets --features inspection`(앱) 출력에 이 두 항목의 경고는 없었습니다. 출력된 경고는 vendor `wry-preview` deprecated·unused unsafe, `editor-code-actions.rs` 454 `is_applying`, `zen.rs` 543 `unused mut`뿐입니다. 메인 병합 게이트에서도 같은 출력을 확인합니다.

### 설치 Ok이지만 dangling일 때 toast 없음 (코드 유지)

- 리뷰 지적처럼 원본 `runCliInstall`(`agent.commands.ts` 32-39)은 결과 상태와 관계없이 성공 toast를 띄웁니다. native는 `installed && !dangling`일 때만 성공을 띄웁니다.
- `install`(`agent-cli.rs` 62-78)이 Ok인데 dangling인 경로는 두 가지입니다. (1) 이미 끊긴 링크가 있는 상태에서 관리자 대화상자를 취소한 경우: `run_osascript`가 취소를 Ok로 돌려주고 링크는 끊긴 채 남습니다. 이것은 결정 7(취소 시 성공 문구 없음)이 다루는 취소 경우입니다. (2) 링크가 이미 번들 대상과 같은데 번들 안 `taide-cli`가 없는 경우: 설치 스크립트를 부르지 않고 끊긴 상태를 돌려줍니다. 이때 성공 문구는 사실과 다릅니다.
- native는 취소와 성공을 상태로만 구분할 수 있어 (1)을 결정 7대로 처리하려면 dangling도 성공에서 빼야 합니다. 코드는 바꾸지 않았고, (2)는 손상된 번들에서만 생기는 사양 밖 경우로 남깁니다.

### 수정 검증

명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/terminal-agent`를 붙였습니다.

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --all-targets --features inspection --manifest-path native/taide-native-app/Cargo.toml` | 오류 없음. 경고는 위 기존 경고만 |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- agent_cli::tests terminal_agent_tests::실제_app_cli --test-threads=2` | 7 passed, 0 failed, 907 filtered out (cfg 변경 뒤 두 모듈이 macOS에서 그대로 연결·실행됨) |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 차이 없음 |
