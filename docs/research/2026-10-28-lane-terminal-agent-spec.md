# lane terminal-agent 사양 (Rust-native 전환, wave 1)

> 작성일: 2026-10-28 · 대상 lane: `terminal-agent` · 운영 기준: `docs/memory/2026-10-28-native-lane-operations.md`
> 이 문서만 읽고 구현을 시작할 수 있도록 원본 TS 동작, native 현재 자산, 설계, 검사, 완료 기준을 ID마다 적었습니다. 원본 TS를 다시 조사할 필요는 없고, 구현 중에는 여기 적힌 native 심볼 주변만 읽으면 됩니다.

## 0. 읽는 법과 전제

- 경로는 저장소 기준 상대 경로입니다. lane worktree(`/Users/hyunseokbyun/development/TAIDE-lanes/terminal-agent`)에서도 같습니다.
- `native/taide-native-app/src/application.rs`, `native/taide-native-app/src/host.rs`, `native/taide-native-ui/src/commands.rs`, `native/taide-native-ui/src/shell.rs`는 wave 0 구조 이동 중이라 줄 번호를 쓰지 않고 심볼 이름으로만 적었습니다. 이 문서 작성 시점에 `docs/research/2026-10-28-native-lane-seams.md`는 없고, `native/taide-native-app/src/domains/`에는 `commands.rs`·`shell.rs`만 있습니다. 아래의 `domains::terminal_agent`, `HostCommand::TerminalAgent(Command::…)`, `HostReply::TerminalAgent(Reply::…)`, `ShellIntent::TerminalAgent(intents::terminal_agent::Intent::…)`는 현재 `domains/commands.rs`·`intents/commands.rs`의 패턴을 따른 이름입니다. seams 문서가 생기면 그 이름으로 바꿔 읽습니다.
- 그 밖의 native 파일(`terminal_surface.rs`, `terminal_host.rs`, `ide-tools.rs`, `domains/commands.rs` 등)의 줄 번호는 작성 시점 값입니다. 다른 lane 병합으로 조금 밀릴 수 있으니 함께 적은 심볼 이름을 우선합니다.
- 원본 줄 번호는 이 문서를 쓰며 직접 확인한 값입니다. QA 문서 근거는 파일명으로 적었습니다(모두 `docs/quality-assurance/` 아래).
- 기준: 원본 TS 화면·상태·상호작용과 같게 만들고, 새 기능·디자인은 넣지 않으며, 원본 결함은 재현하지 않습니다.
- 상태 정의는 기능 대응표(`2026-10-09-native-function-matrix.md`) 머리말을 따릅니다. complete는 구현 + 실제 앱 경로 + 해당 동작의 자동 검사이고 픽셀·실제 OS 입력·실기(배치 31)는 포함하지 않습니다. not-applicable은 웹 기술/내부 지표 전용입니다.
- 같은 기능을 다룬 다른 lane 사양: `2026-10-28-lane-shell-spec.md`(shell-90~92), `2026-10-28-lane-settings-spec.md`(settings-16·17·21·22·23·30), `2026-10-28-lane-git-spec.md`(git-49~52). 이 문서는 그 설계를 다시 쓰지 않고 절 번호로 가리킵니다.

## 1. 요약

### 1.1 수와 상태

- 대상 32개: terminal-agent-6, 11, 19, 26, 27, 32, 38, 45, 47, 48, 49, 51, 52, 53, 55, 56, 57, 59, 60, 61, 62, 63, 64, 67, 68, 70, 71, 72, 73, 76, 77, 78
- 대응표 현재 상태: partial 12(11, 19, 27, 32, 38, 49, 59, 60, 61, 63, 67, 71), unwired 13(47, 48, 51, 55, 56, 57, 68, 70, 72, 73, 76, 77, 78), missing 7(6, 26, 45, 52, 53, 62, 64)
- 실제 코드로 다시 판정한 분류

| 분류 | 수 | ID |
| --- | --: | --- |
| 이미 구현됨, 근거 갱신만 필요 | 7 | 47, 48, 52, 53, 72, 77, 78 |
| 이미 구현됨, 확인 검사 1건 추가 뒤 갱신 | 3 | 32, 51, 76 |
| 원본에 사용자 기능이 없음(not-applicable 재판정) | 2 | 26, 45 |
| 이 lane이 구현(또는 근거 검사 작성) | 9 | 6, 11, 19, 27, 38, 49, 56, 67, 71 |
| 다른 lane이 같은 기능을 구현, 병합 뒤 행 갱신(4절 결정 1 A안) | 11 | 55, 57, 59, 60, 61, 62, 63, 64, 68, 70, 73 |

- 대응표의 오래된 비고: 47·48(editor-82 complete), 52(shell-35 complete), 53(shell-75 complete), 72(editor-83 complete), 76~78(shell-24 complete)은 다른 행에서 이미 실제 App 검사까지 끝났는데 이 영역 행만 갱신되지 않았습니다. 2026-10-06 감사(`2026-10-06-native-audit-terminal-agent.md`)가 6·26·45를 이미 n/a로 판정했으나 대응표에는 missing으로 남아 있습니다.
- 이 lane(wave 1)에서 끝낼 수 있는 ID: 21개(묶음 1~3). 56은 명령 부분만 끝나고 설정 행(settings-16)이 남아 partial로 끝납니다.
- 다른 lane 병합 뒤에만 닫을 수 있는 ID: 11개(묶음 4)와 56의 설정 행.

### 1.2 ID별 분류

| ID | 기능 | 표 상태 | 이 lane에서 할 일 | 다른 lane·외부 의존 | 묶음 |
| --- | --- | --- | --- | --- | --- |
| 6 | 리거처 | missing | 셀 단위 그리기 근거 검사 1건, complete 재판정 | 없음 | 1 |
| 11 | 실행 중 scrollback 반영 | partial | `TerminalCore::set_history`, Hub 전파 | settings-30이 재사용(결정 3) | 1 |
| 19 | Option-as-Meta | partial | Alt 단독 문자 키 → ESC 접두, 조합 문자 억제 | 실기(배치 31) | 1 |
| 26 | 터미널 내 검색 | missing | not-applicable 재판정 | 없음 | 1 |
| 27 | OSC 133 장식 | partial | 숨은 세션 색 갱신, 탭 전환·테마 변경 App 검사 | 결정 6 | 1 |
| 32 | 터미널 context menu | partial | 붙여넣기 gate 재판정, 근거 갱신 | 아이콘은 commands-ui-98(결정 5) | 1 |
| 38 | spawn 실패 화면·재시작·toast | partial | 화면 글자색, 재시작 App 검사 | 없음 | 1 |
| 45 | 클립보드 가용성 probe | missing | not-applicable 재판정 | 없음 | 1 |
| 49 | 터미널 설정 | partial | 실행 중 반영 App 검사(11 뒤) | settings-30과 같은 근거 | 1 |
| 51 | agent 상태 UI 동기화 | unwired | Zen·보조 창·이벤트 repaint App 검사 | 없음 | 2 |
| 52 | rail agent 배지 | missing | 근거 갱신(shell-35) | 없음 | 2 |
| 53 | 탭 Sparkles·tooltip | missing | 근거 갱신(shell-75) | 없음 | 2 |
| 67 | IDE openFile | partial | App 도달 검사 | 없음 | 2 |
| 71 | IDE close_tab·closeAllDiffTabs | partial | App 도달 검사 | diff 표면 수명은 git-52 | 2 |
| 72 | IDE 선택 push | unwired | 근거 갱신(editor-83) | 없음 | 2 |
| 47 | Run Selected Text in Terminal | unwired | 근거 갱신(editor-82) | commands-ui-40 같은 근거 | 3 |
| 48 | run-in-terminal 쓰기 경로 | unwired | 근거 갱신(editor-82, shell-24) | 없음 | 3 |
| 56 | CLI 설치·제거·연결 명령 | unwired | `agent_cli` 모듈, 명령 3종, toast | 설정 행은 settings-16(결정 2) | 3 |
| 76 | Task Runner 대화상자 | unwired | 보조 창 App 검사 1건, 근거 갱신 | shell 사양 2.12의 보조 창 항목을 이 lane이 가져옴 | 3 |
| 77 | `task.runTask` 명령 | unwired | 근거 갱신(shell-24) | commands-ui-41 같은 근거 | 3 |
| 78 | 태스크 감지 | unwired | 근거 갱신(shell-24, taide-task 단위 검사) | 없음 | 3 |
| 55 | hooks 설치 UI | unwired | 없음 | settings-17(settings lane S1) | 4 |
| 57 | 외부 열기·`--wait` | unwired | 없음 | shell-90(shell 사양 2.4) | 4 |
| 59 | agent 완료·입력 대기 알림 | partial | 없음 | shell-91(shell 사양 2.5) | 4 |
| 60 | 명령 완료 알림 | partial | 없음 | shell-91 | 4 |
| 61 | LSP 설치 알림 | partial | 없음 | shell-91 | 4 |
| 62 | 첫 전달 안내 toast | missing | 없음 | shell-91 | 4 |
| 63 | 알림 게이트·전송 | partial | 없음 | shell-91 | 4 |
| 64 | 테스트 알림·시스템 설정 열기 | missing | 없음 | settings-21·22(settings lane S1) | 4 |
| 68 | Claude diff 탭 | unwired | 없음 | git-49~52 | 4 |
| 70 | IDE saveDocument | unwired | 없음 | shell-92 save(shell 사양 2.6) | 4 |
| 73 | IDE 진단 push | unwired | 없음 | shell-92 진단(shell 사양 2.6) | 4 |

### 1.3 다른 lane 의존과 순서

- 먼저 필요한 것
    - wave 0 구조 이동. 이 lane의 새 코드는 `domains/terminal_agent.rs`, `intents/terminal_agent.rs`, 새 모듈에만 넣고, 공용 파일(`application.rs`, `host.rs`의 enum 루트, `lib.rs` 모듈 목록, `taide-native-ui/src/commands.rs`의 `ShellIntent` 루트, locale 카탈로그)은 영역 블록 끝에 추가만 합니다. wave 0이 `domains/terminal_agent.rs`를 만들지 않았으면 `domains/commands.rs`와 같은 모양(`Command`, `Reply`, `State`, `dispatch`, `impl NativeApplication { apply_terminal_agent_reply, apply_terminal_agent_intent }`)으로 새로 만듭니다.
- 같은 wave의 다른 lane과 조율할 것
    - shell lane
        - shell 사양 4.1 결정 1과 이 문서 결정 1은 같은 결정입니다. A안이면 shell lane이 57·59~63·70·73의 코드를 만들고 이 lane은 묶음 4에서 행만 갱신합니다.
        - `terminal_surface.rs`를 두 lane이 함께 고칩니다. shell lane은 shell-5에서 `Views`의 raw input 상태를 공유 구조로 나누고, 이 lane은 `input_event`·`key_input`·`View`·`status`·`Appearance`만 고칩니다. 함수 단위로 겹치지 않게 하고 충돌은 메인 병합에서 풉니다.
        - `ide-tools.rs`: shell lane이 `close_tab`에 wait marker 해제 훅을 넣습니다. 이 lane은 이 파일을 고치지 않고 검사만 추가합니다(67·71).
        - shell 사양 2.12 "묶음 3으로 넘기는 범위"의 "보조 창 `TaskRunnerDialog`(terminal-agent-76)"는 이 lane 묶음 3이 검사하고 필요하면 고칩니다. shell lane 묶음 3 목록에서 뺍니다.
    - explorer-search lane: 검색 치환 알림은 shell lane의 알림 진입점을 씁니다(explorer-search 사양 1.3 표). 이 lane과 직접 겹치는 파일은 없습니다.
    - editor+preview lane: 겹치는 파일 없음.
- 나중에 이어야 하는 것
    - settings lane(wave 2): 55(settings-17), 56의 설정 행(settings-16), 64(settings-21·22). settings 사양 S1의 "taide-runtime `agent_host.rs`에 cli install/uninstall 추가"는 결정 2 A안이면 이 lane의 `crate::agent_cli` 재사용으로 바뀝니다. settings 사양 결정 7(실행 중 scrollback)은 결정 3 A안이면 "이 lane 구현 재사용"으로 바뀝니다.
    - git lane(wave 2): 68(git-49~52). 71의 ClaudeDiff 표면 해제는 git-52 범위입니다. git 사양 1.3은 terminal-agent-69(complete로 표시)가 실제로는 로컬 소비자가 없다고 적었습니다. 69는 이 lane 대상이 아니며 git lane이 정정합니다.
    - commands-ui lane(wave 3): 32의 메뉴 아이콘(commands-ui-98), commands-ui-32~34·40·41 행 갱신, 묶음 4의 행 갱신(운영 문서 3절 "앞 wave에서 진입만 남은 행").

### 1.4 같은 근거로 함께 갱신할 다른 영역 행

| 다른 영역 행 | 근거 | 갱신 주체 |
| --- | --- | --- |
| settings-30(scrollback 부분) | 묶음 1의 11·49 | settings lane S5 또는 메인 |
| commands-ui-40 `terminal.runSelectedText` | editor-82(`2026-10-12-native-batch23-terminal-run.md`) | commands-ui lane |
| commands-ui-41 `task.runTask` | shell-24(`2026-10-28-native-shell-keymap-dialogs-slot-header.md`) | commands-ui lane |
| commands-ui-32~34 `cli.*` | 묶음 3의 56 | commands-ui lane |
| settings-16 | 묶음 3의 56 백엔드 + settings lane 행 | settings lane |

## 2. ID별 사양

### 2.0 공통 설계

#### 2.0.1 영역 모듈

- `native/taide-native-app/src/domains/terminal_agent.rs`
    - `pub enum Command { Cli(agent_cli::Action) }`
    - `pub enum Reply { Cli { action: agent_cli::Action, result: AppResult<CliInstallStatus> } }`
    - `pub(crate) struct State { pub(crate) cli_status: Option<CliInstallStatus>, pub(crate) command_colors: Option<CommandColors> }`
    - `pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply>`
    - `impl NativeApplication`
        - `apply_terminal_agent_reply(&mut self, reply: Reply)`
        - `apply_terminal_agent_intent(&mut self, intent: Intent)`
        - `tick_terminal_agent(&mut self, context: &egui::Context)`: 11의 history 전파, 27의 명령 색 전파
- 공용 파일 추가(추가만)
    - `host.rs`: `HostCommand::TerminalAgent(domains::terminal_agent::Command)`, `HostReply::TerminalAgent(domains::terminal_agent::Reply)`, dispatch 분기 1줄
    - `application.rs`: `terminal_agent_state` 필드, 생성자 초기값, `HostReply::TerminalAgent(reply) => self.apply_terminal_agent_reply(reply)`, `ShellIntent::TerminalAgent(intent)` 분기, `background_tick`에서 기존 `self.terminals.hub().configure_cursors()` 바로 뒤에 `self.tick_terminal_agent(context)`, 테스트 모듈 목록 끝에 `application-terminal-agent-tests.rs`
    - `lib.rs`: `agent_cli` 모듈
    - `taide-native-ui/src/intents/mod.rs`: `pub mod terminal_agent;`
    - `taide-native-ui/src/commands.rs`: `ShellIntent::TerminalAgent(crate::intents::terminal_agent::Intent)`(native-host 전용 variant)

#### 2.0.2 host worker를 막지 않는 규칙

- host worker는 명령을 한 번에 하나씩 처리합니다(`host.rs`의 `connect_with_integrations` 안 `while let Some(command) = receiver.recv().await`).
- 관리자 암호 대화상자처럼 사람이 응답할 때까지 걸리는 작업(56의 osascript)은 worker 안에서 기다리지 않습니다. 기존 `HostCommand::GitBranch` 처리처럼 `services.tasks.spawn_transient(name, async move { … sender.send(reply) … repaint() })`로 떼어 내고 dispatch는 `None`을 돌려줍니다. spawn이 실패하면(종료 중) 즉시 오류 응답을 돌려줍니다.

#### 2.0.3 검사 공통

- 명령(모두 `CARGO_TARGET_DIR=<worktree>/target-lane`, `--locked --offline`)
    - 앱 lib: `cargo test --manifest-path native/taide-native-app/Cargo.toml --features inspection --lib -- <필터> --test-threads=4`
    - 앱 통합: `cargo test --manifest-path native/taide-native-app/Cargo.toml --features inspection --test terminal-host -- <필터>`
    - 터미널 crate: `cargo test --manifest-path native/taide-native-terminal/Cargo.toml --test core -- <필터>`(새 파일이면 `--test history`)
    - UI: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib -- <필터>`
    - 동결 remote-web: `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline`(UI crate를 바꾸는 묶음 3만)
    - 형식: 바꾼 crate마다 `cargo fmt --manifest-path <crate>/Cargo.toml -- --check`
- 재사용할 fixture와 선례
    - 실제 터미널 App: `application-context-menu-tests.rs`의 `fixture()`, `shell_override = "/bin/cat"`, `terminal_text`, `wait`, `menu_action`(1464행 검사가 잘못된 셸 → 오류 toast 선례)
    - 합성 PTY와 실제 `Views`: `tests/terminal-host.rs`(`terminal_commands`, `terminal_menu…` 검사들, 2250행 바이트 전송 검사)
    - 셀 그리기 shape 수집: `terminal_surface.rs` 테스트 모듈의 `egui::Shape::Text` 수집(8499·8600행 부근)
    - agent 표시: `application-tab-menu-tests.rs`(`AGENT_GLYPH_TEXTURE_PREFIX`, `AgentStore::diff`로 합성 agent 넣기)
    - IDE: `application-ide-selection-tests.rs`의 `start_ide`, `ide-tools-tests.rs`의 `LayoutActions` 조립
    - Task Runner: `application-shell-dialogs-tests.rs`, 보조 창은 `application-auxiliary-window-tests.rs`
- 금지: GUI 실행, 실제 클립보드·OS 알림·osascript·`/usr/local/bin` 쓰기·Keychain·휴지통, 합성 OS 입력, 저장소 밖 탐색, git 상태 변경, 새 의존성. 56은 포트를 주입하고 실제 osascript를 부르지 않습니다.
- 검사 이름은 기존처럼 한국어입니다.

---

### 2.1 terminal-agent-6 리거처

#### 원본 동작

- `src/features/terminal/terminal-view.tsx` 6-10의 addon은 fit, webgl, search, unicode11, web-links뿐이고 287-307에서 이것만 로드합니다. ligatures addon은 설치돼 있지 않습니다(`docs/features/terminal.md` 12).
- xterm WebGL 렌더러는 셀마다 글리프를 그려 `->`, `!=` 같은 프로그래밍 합자를 만들지 않습니다. 터미널 설정(`settings-terminal-section.tsx` 35-97)에도 합자 항목이 없습니다.
- 결과: 원본 화면은 "합자 없음"입니다.

#### native 현재 자산

- `terminal_surface.rs` `paint`(4851-)가 셀마다 `painter.text`를 따로 부릅니다(4927-4937, 커서 셀 5016-5031). 한 셀 글자와 zero-width 결합 문자만 한 shape에 들어가므로 셀 사이 합자가 생기지 않습니다.

#### 빠진 것

- 코드는 없습니다. 원본과 같은 "합자 없음"을 증명하는 검사만 없습니다.

#### 자동 검사

- 앱 lib(`terminal_surface.rs` 테스트 모듈, 기존 shape 수집 방식): core에 `->=!=www`를 출력하고 그린 뒤 `Shape::Text`의 `galley.job.text`가 모두 한 셀 문자(`-`, `>`, `=`, `!`, `w`)이고 두 글자 이상 문자열 galley가 없음을 확인합니다. 한글 wide char는 한 shape, spacer 셀은 그리지 않음을 함께 확인합니다.

#### 완료 판정

- 검사 통과 → complete(결정 4 A안). 비고: 원본도 합자 없음, 사용자가 합자 글꼴을 고른 경우의 실제 화면은 배치 31 실기.

---

### 2.2 terminal-agent-11 실행 중 scrollback 반영

#### 원본 동작

- `terminal-view.tsx` 241-245: 설정 `terminalScrollback`이 바뀌면 `term.options.scrollback = scrollback`으로 실행 중 xterm 버퍼 줄 수를 바로 바꿉니다. 줄이면 가장 오래된 줄이 잘리고, 늘리면 이후 출력이 새 상한까지 쌓입니다.
- `terminal-session.tsx` 137-150: PTY replay ring 바이트(`scrollbackBytes`)는 spawn 때만 정해지고 실행 중에는 바꾸지 않습니다. 설정 힌트 `settings.terminalScrollbackHint`("Applies from the next spawn on…", en.json 919)가 이 ring을 가리킵니다.
- 범위 100~100 000, 기본 10 000(`settings-terminal-section.tsx` 18-20).

#### native 현재 자산

- spawn 때 history: `terminal_tabs.rs` 424-428(`history = settings.terminal_scrollback`, `scrollback_bytes = history * REPLAY_BYTES_PER_LINE`)
- `taide-native-terminal/src/lib.rs`: `TerminalCore::new(size, history, limits)`(310-336), `admit`(286-307, grid 바이트 예산), `validate_resize`(454), `has_scrollback`(665)
- `session.rs` `SharedTerminal::configure_cursor`(255-264): 실행 중 설정 반영의 선례(publishing 잠금 → state 잠금 → Failed면 무시 → 같은 값이면 false)
- `terminal_host.rs` `Hub::configure_cursors`(611-622)와 `NativeApplication::background_tick`의 호출
- vendored alacritty: `term/mod.rs` `Term::set_options`(552-575)는 history와 함께 Title/ResetTitle 이벤트를 보냅니다. `grid/mod.rs` `Grid::update_history`(172-180)는 잘린 줄을 `native_record_trim`으로 기록합니다.

#### 빠진 것

- 실행 중 세션의 history 변경 경로

#### 구현 설계

1. vendored `native/taide-native-terminal/vendor/alacritty-terminal/src/term/mod.rs`에 `pub fn native_set_scrolling_history(&mut self, lines: usize)`를 추가합니다.
    - `self.config.scrolling_history = lines`
    - alt screen이면 `inactive_grid`, 아니면 `grid`에 `update_history(lines)`(primary grid만)
    - `self.damage.full = true`
    - `set_options`를 쓰지 않는 이유: Title 이벤트가 `Listener::send_event`(lib.rs 170-205)를 지나 `ScanEvent::Title` stream effect가 되어 탭 제목·agent 감지 입력에 가짜 제목이 들어갑니다.
2. `TerminalCore`(lib.rs)
    - `pub fn history(&self) -> usize`
    - `pub fn set_history(&mut self, lines: usize) -> AppResult<bool>`
        - 같은 값이면 `Ok(false)`
        - 현재 grid 크기(`Size { columns, rows }`)와 `lines`로 `admit`. 실패면 `Err`, 상태는 그대로
        - 통과하면 vendored 함수 호출, `self.history = lines`, `enforce_retained()?`, `Ok(true)`
3. `SharedTerminal::configure_history(&self, lines) -> AppResult<bool>`(session.rs)
    - `configure_cursor`와 같은 잠금 순서
    - `Phase::Failed`면 `Ok(false)`
    - 예산 초과로 거절한 값을 `State.rejected_history`에 기억합니다. 같은 값이 다시 오면 `Ok(false)`로 즉시 돌아가 매 frame 재시도와 경고 반복을 막습니다.
4. `Hub::configure_history(&self) -> AppResult<bool>`(terminal_host.rs)
    - `services.state.settings.read().terminal_scrollback as usize`를 읽어 모든 세션에 적용합니다.
    - 한 세션의 거절은 `log::warn!`(세션 id·값만, 내용 없음)으로 남기고 나머지 세션은 계속 처리합니다.
5. `tick_terminal_agent`가 `configure_history()`를 부르고, 바뀌면 `context.request_repaint()`
6. 실행 중 세션의 PTY ring 바이트는 바꾸지 않습니다(원본과 같음). 원격 터미널(`remote-terminal.rs`의 `history` 클로저)은 spawn 값이라 그대로입니다.
7. 화면 쪽 정합: `paint`가 `view.offset`을 history로 자르고(4870), 선택은 `native_record_trim`과 `reconcile_view`가 다시 맞춥니다. 별도 코드는 필요 없지만 검사로 확인합니다.

#### 자동 검사

- 터미널 crate(`tests/core.rs` 또는 새 `tests/history.rs`)
    - 50줄 출력 → `set_history(10)` → `grid().history_size() == 10`, 가장 오래된 줄이 사라지고 최신 줄은 남음
    - 늘리기 → 기존 줄 유지, 이후 출력이 새 상한까지 쌓임
    - 예산 초과 값 → `Err`, `history()` 그대로, 이후 `advance` 정상
    - `set_history` 직후 `flush_sync_outcome().effects`에 Title/ResetTitle 없음
    - alt screen 중 변경 → alt 해제 뒤 primary history가 새 상한 이하
    - 잘린 줄에 있던 OSC 133 블록이 `command_blocks()`에서 빠짐
- 터미널 crate(session): `configure_history`가 같은 값이면 false, Failed면 false, 거절 값 재시도 없음
- 앱 통합(`tests/terminal-host.rs` 합성 PTY): 설정 값을 바꾸고 `Hub::configure_history` → 실행 중 세션 `history()`가 새 값, 새로 spawn한 세션도 새 값

#### 완료 판정

- 위 검사 통과 → complete. settings-30의 scrollback 부분 같은 근거(결정 3).

---

### 2.3 terminal-agent-19 Option-as-Meta

#### 원본 동작

- `terminal-view.tsx` 271: `macOptionIsMeta: true`. 설정이 아니라 상수입니다.
- xterm `node_modules/@xterm/xterm/src/common/input/Keyboard.ts`
    - 325: `(!isMac || macOptionIsMeta) && altKey && !metaKey`이면 Option을 Meta로 씁니다. 그래서 모든 플랫폼에서 같은 규칙입니다.
    - 327-330: `KEYCODE_KEY_MAPPINGS`(11-36)의 숫자·기호 키는 `ESC + [기본, shift]` 중 하나입니다. 숫자 `0)` `1!` `2@` `3#` `4$` `5%` `6^` `7&` `8*` `9(`, 기호 `;:` `=+` `,<` `-_` `.>` `/?` `` `~ `` `[{` `\|` `]}` `'"`입니다.
    - 331-337: 영문 A~Z는 `ESC + 소문자`(shift면 대문자, ctrl이면 제어 문자)
    - 338-339: Space는 `ESC + ' '`(ctrl이면 `ESC + NUL`)
    - 340-351: Option이 dead key(`key === 'Dead'`, `code`가 `Key*`)면 `ESC + 글자`를 보내고 조합을 취소합니다(`cancel`).
    - 방향키·Enter·Backspace·Escape의 Alt는 앞 분기에서 수식 키 mask로 처리됩니다.
- 결과: Option+b는 `ESC b`(단어 뒤로), Option 조합 문자(`∫`)는 PTY에 가지 않습니다.

#### native 현재 자산

- `taide-native-terminal/src/input.rs` `character()`(188-232): alt면 `ESC + 문자`, alt+ctrl 조합 처리
- `terminal_surface.rs`
    - `key_input`(4376-4416): 문자 키는 ctrl 또는 mac_cmd가 있을 때만 `Key::Character`로 바꿉니다. Option 단독 문자 키는 `None`
    - `input_event`(4153-4194): 그 뒤 오는 `Event::Text("∫")`를 `CommittedText`로 보냅니다.
    - `buffer_starting_input`(4196-): spawn 전 입력도 `input_event`를 지납니다.

#### 빠진 것

- Alt 단독 문자 키의 ESC 접두 인코딩과, 같은 키 입력이 만든 조합 문자·dead key preedit 억제

#### 구현 설계

- `terminal_surface.rs`
    - `View`에 `meta_text_pending: bool`을 둡니다.
    - 함수 `meta_character(key: egui::Key, physical: Option<egui::Key>, shift: bool) -> Option<char>`
        - 표: A~Z, Num0~Num9, Space, Minus, Equals, OpenBracket, CloseBracket, Backslash, Semicolon, Quote, Comma, Period, Slash, Backtick. 값은 위 원본 표와 같습니다.
        - `key`가 표에 있으면 그것을, 없으면 `physical`을 씁니다. Option 조합 상태의 논리 키는 보통 표 밖 문자라 물리 키로 떨어집니다.
    - `input_event`의 `Event::Key { pressed: true, .. }` 분기 앞에 새 분기를 둡니다. 조건은 `modifiers.alt && !modifiers.ctrl && !modifiers.mac_cmd`이고 preedit이 비어 있을 때입니다.
        - `meta_character`가 `Some(c)`면 `NativeInput::Key { key: Key::Character(c), modifiers: Modifiers { alt: true, ..Default::default() } }`를 돌려주고 `view.meta_text_pending = true`
        - `None`이면 기존 경로(방향키·Enter·Backspace는 `key_input`이 처리)
    - `meta_text_pending`일 때 다음 `Event::Text(_)`, `Event::Ime(ImeEvent::Commit(_))`, `Event::Ime(ImeEvent::Preedit { .. })` 하나를 `InputEvent::Handled`로 버리고 플래그를 내립니다. Preedit이면 `view.preedit`도 비웁니다.
    - 다른 `Event::Key`가 오거나 그 frame의 입력 처리가 끝나면 플래그를 내립니다(다음 frame의 정상 텍스트를 삼키지 않음).
    - ctrl+alt 조합은 지금처럼 `key_input`의 ctrl 분기 → `character()`의 alt+ctrl 처리로 둡니다.

#### 자동 검사

- 앱 통합(`tests/terminal-host.rs`의 실제 `Views` 바이트 검사 방식, 2250행 선례)
    - `[Key{B, alt}, Text("∫")]` → 보낸 바이트 `\x1bb`만
    - `[Key{B, alt+shift}, Text("ı")]` → `\x1bB`
    - `[Key{Num1, alt+shift}, Text("⁄")]` → `\x1b!`
    - `[Key{Space, alt}, Text("\u{a0}")]` → `\x1b `
    - `[Key{E, alt}, Ime(Preedit("´"))]` → `\x1be`, preedit 없음, 다음 frame `Text("x")` → `x`
    - `Alt+ArrowLeft` → `\x1b[1;3D`, `Ctrl+Alt+A` → `\x1b\x01`(기존과 같음)
    - spawn 전 버퍼링(`buffer_starting_input`)도 같은 바이트

#### 완료 판정

- 위 검사 통과 → complete. 비고: 실제 Option 키·비 US 배열·dead key·IME 켜짐 상태는 배치 31 실기.

---

### 2.4 terminal-agent-26 터미널 내 검색

#### 원본 동작

- `terminal-view.tsx` 288에서 `SearchAddon`을 만들어 로드만 하고 437에서 해제합니다. `findNext`·`findPrevious` 호출부, 명령, 키 바인딩이 `src` 어디에도 없습니다(`rg findNext src` 결과: OSC 133 명령 점프 함수뿐).
- 결과: 원본 사용자에게 터미널 검색 기능이 없습니다. 터미널에 포커스가 있을 때 ⌘F도 터미널에서 아무 일도 하지 않습니다.

#### native 현재 자산과 할 일

- 코드 없음. 원본과 같은 상태입니다.
- 대응표를 not-applicable로 바꾸고 비고에 "웹 addon 로드만 있고 사용자 표면 없음(2026-10-06 감사 2.1 표와 같은 판정)"을 적습니다(결정 4).

#### 완료 판정

- 재판정만 합니다. 검사는 추가하지 않습니다.

---

### 2.5 terminal-agent-27 OSC 133 명령 블록 장식

#### 원본 동작

- `src/features/terminal/terminal-osc133.ts`
    - 84-124: A/C/D 상태 전이, C를 본 셸은 빈 프롬프트 블록을 D에서 버림
    - 14: 블록 상한 500
    - 152-157: 테마 색 `#RRGGBB(AA)` → 앞 6자리
    - 164-165: exit 0이면 `statusIndicator.success`, 아니면 `statusIndicator.error`
    - 225-236: D 시점의 색으로 decoration(왼쪽 2px gutter, overview ruler 오른쪽)을 만듭니다. 이미 만든 decoration은 이후 테마가 바뀌어도 색이 그대로입니다.
    - 197-202, 203-273: tracker는 xterm 인스턴스와 함께 살고 죽습니다. 탭이 뒤로 가면 unmount, 돌아오면 새 tracker가 ring replay를 다시 파싱하므로 replay에 남은 블록만 그 시점 테마 색으로 다시 생깁니다.
- `terminal-session.tsx` 423-424: 색은 현재 테마에서 읽습니다.

#### native 현재 자산

- vendored `term/native_commands.rs`: A/C/D 전이·상한 500·빈 프롬프트 폐기·JS Number 규칙 exit code(92-151), D 시점 색 보존(141-147), 표시 행 decoration(153-186)
- `terminal_surface.rs`: 2px gutter(4956-4965), ruler(`terminal_ruler.rs`), 보이는 세션만 매 frame `session.configure_command_colors(appearance.command_colors)`(3420), 색 읽기 `command_color`(222-231)
- 근거 QA: `2026-10-03-m8-native-terminal-commands.md`, `2026-10-03-m8-native-terminal-ruler.md`
- Core는 세션 동안 유지되고 한 번만 파싱합니다(전환 계약 `docs/acknowledge/2026-09-23-rust-native-transition-contract.md` 59 "PTY 출력은 하나의 Rust terminal core가 단 한 번 파싱한다").

#### 빠진 것

1. 뒤로 간(그리지 않는) 세션은 테마가 바뀌어도 이전 명령 색을 계속 씁니다. 원본은 뒤 탭에 tracker가 없고, 돌아올 때 replay가 현재 테마 색으로 D를 처리합니다. 그래서 뒤에서 끝난 명령은 현재 테마 색이어야 합니다.
2. 탭 전환 뒤 장식 유지·테마 변경 전후 색에 대한 App 검사가 없습니다.
3. 원본 replay 수명과의 차이(ring 밖 오래된 블록이 native에서는 남음, 돌아온 탭의 기존 블록 색이 그대로임)는 단일 파싱 계약 때문에 생기는 의도된 차이입니다. 2026-10-03 ruler QA 끝절의 미응답 질문(A: 단일 Core 유지)이 결정 6입니다.

#### 구현 설계

- `terminal_host.rs`에 `Hub::configure_command_colors(&self, colors: CommandColors) -> AppResult<()>`를 둡니다. 모든 세션에 `SharedTerminal::configure_command_colors`를 부르고, 한 세션 오류는 `log::warn!` 후 계속합니다.
- `terminal_surface::Appearance`에 `pub fn command_colors(&self) -> CommandColors` 접근자를 둡니다(필드는 private).
- `tick_terminal_agent`가 `self.terminal_appearance.command_colors()`를 `State.command_colors`와 비교해 바뀐 때만 Hub에 전달합니다. 테마 변경 응답(`HostReply::Presentation`)이 `terminal_appearance`를 바꾸면 다음 tick에 반영됩니다.
- 보이는 세션의 기존 3420 호출은 그대로 둡니다.

#### 자동 검사

- 앱 통합(`tests/terminal-host.rs`의 합성 PTY, 기존 `terminal_commands` 검사 방식)
    - 세션 둘(보이는 것 1, 뒤 탭 1)에서 A·C·D(0) 완료 → 색 S1
    - 테마 색을 S2로 바꾸고 `Hub::configure_command_colors(S2)`
    - 뒤 탭 세션에서 새 명령 D(1) → 그 블록 색은 실패 색 F2
    - 보이는 세션의 기존 블록은 S1 그대로, 새 D(0) 블록은 S2
- 앱 lib(실제 App, `/bin/cat` 셸): 출력에 OSC 133 블록 하나를 만들고 다른 탭으로 갔다 돌아온 뒤 gutter shape이 같은 줄·같은 색으로 남음(결정 6 A안의 동작을 고정)

#### 완료 판정

- 위 검사 통과 + 결정 6 A안 → complete. 비고: 탭 전환 뒤 블록 보존·기존 블록 색 유지는 단일 파싱 계약의 의도된 차이, xterm과의 전체 VT·reflow marker matrix와 성능은 다른 complete 행처럼 별도 게이트.

---

### 2.6 terminal-agent-32 터미널 context menu

#### 원본 동작

- `src/features/terminal/terminal-context-menu.tsx` 67-113: 순서 Copy(`terminal.copy`, Copy 아이콘), Paste(`terminal.paste`, ClipboardPaste), Select All(`terminal.selectAll`, SquareDashedMousePointer), 구분선, Clear(`terminal.clear`, Eraser), 구분선, Split 하위 메뉴(`tab.split`, GitCompare; 하위 `editorArea.splitLeft/Right/Top/Bottom`, PanelLeft/Right/Top/Bottom, 공간 부족이면 비활성), New Terminal(`tab.newTerminal`, SquareTerminal), 구분선, Kill(`terminal.kill`, Trash2). 닫힐 때 터미널로 포커스 복귀(62-65).
- `src/widgets/terminal-pane/terminal-pane.tsx` 142-156: 열 때 한 번 상태를 표본합니다. `canCopy = clipboard.canCopy && 선택 있음`, `canPaste = clipboard.canPaste`.
- `terminal-clipboard-availability.ts` 1-28: 비동기 클립보드 API가 있는지(secure context)만 봅니다. Tauri 창은 secure context라 항상 참이고, 거짓은 평문 HTTP 원격 미러뿐입니다.
- 결과: 데스크톱 앱에서 Paste는 항상 켜져 있고 Copy는 선택이 있을 때만 켜집니다.

#### native 현재 자산

- `terminal_surface.rs` `terminal_context_menu`(1653-1990): 같은 순서·구분선, Copy는 선택 텍스트가 있을 때만(1703-1710), Paste 항상 켜짐(1782), 분할 방향별 비활성(1711-1728), 키보드 탐색·검색·접근성 트리
- 검사: `tests/terminal-host.rs` `terminal_menu…` 14건(1746-2385: secondary popup, 키보드 split/new/kill, Shift+F10, 포커스 복귀, AX, 방향키·Enter·Space, disabled 건너뛰기, 라벨 검색)과 `terminal_menu_actions…` 2건(3113-3126: 클릭·Enter/Space로 disabled copy, select, clear, paste, 포커스)

#### 빠진 것

- 붙여넣기 gate: 데스크톱 앱에서는 원본도 항상 켜짐이라 차이가 없습니다. 대응표 비고의 "clipboard 가용성에 따른 붙여넣기 gate 없음"은 원격 미러(동결 remote-web) 전용 조건입니다.
- 항목 아이콘 11종: native 메뉴는 모두 글자만입니다(shell-78·79도 같은 이유로 아이콘 없이 판정). 앱 전반 lucide 아이콘은 commands-ui-98 행입니다(결정 5).

#### 할 일

- 코드 변경 없음. 대응표 비고를 위 근거로 갱신합니다.

#### 완료 판정

- 결정 5 A안이면 complete(비고: 아이콘은 commands-ui-98). B안이면 아이콘 추가 뒤 complete.

---

### 2.7 terminal-agent-38 spawn 실패 화면·재시작·toast

#### 원본 동작

- `terminal-session.tsx`
    - 177-182 `handleSpawnFailure`: spawn 진행 표시 해제, 대기 입력 버림, 실패 상태 저장, `toast.error(describeIpcError(error))`
    - 285-292 `handleRestart`: 종료·실패 상태를 지우고 마지막으로 잰 크기로 다시 spawn
    - 388-397 실패 화면: 배경 `terminal.background`, 글자색 `text-status-error`(`statusIndicator.error`), 14px(`text-sm`), 가운데 정렬, 문구 = `describeIpcError(failure)`, 아래 8px(`gap-2`) 간격으로 outline 작은 버튼 `terminal.restart`
    - 399-408 종료 화면: 글자색 `text-muted-foreground`(`appSidebar.iconDefault`, `src/shared/styles/global.css` 149), 문구 `terminal.processExited` 또는 `terminal.processExited (code)`, 같은 버튼

#### native 현재 자산

- `terminal_surface.rs`
    - `Views::attached`(2423-2480): attach 실패면 `failed`에 오류 저장, 대기 입력·preedit 비움
    - `show`의 세션 없음 분기(3321-3330): `failed`가 있으면 `status(…, describe_error, terminal.restart)` 화면, 재시작 누르면 `failed` 제거 후 다시 `AttachTerminal`
    - `ended_message`(4330-4348), `status`(4350-4374): 글자색이 실패·종료 모두 터미널 전경색(`appearance.foreground`)이고 버튼은 egui 기본 버튼
- `application.rs`의 `HostReply::TerminalAttached` 처리: 실패면 `report` → `toasts.ipc_error`(원본 toast와 같은 문구)
- 검사: `application-context-menu-tests.rs` 1464행(잘못된 셸 → 오류 toast, 세션 없음, 고친 뒤 다음 실행 성공). 실패 화면의 재시작 버튼 경로는 검사가 없습니다.

#### 빠진 것

1. 실패 문구 색(`statusIndicator.error`)과 종료 문구 색(`appSidebar.iconDefault`)
2. 실패 화면 → 재시작 버튼 → 새 세션 실행의 App 검사

#### 구현 설계

- `terminal_surface::Appearance`에 `status_error: Color32`, `status_muted: Color32`를 둡니다. `Appearance::new`에서 `theme.colors["statusIndicator.error"]`, `theme.colors["appSidebar.iconDefault"]`를 `presentation::parse_color`로 읽습니다(기존 `command_color`와 같은 테마 맵).
- `status(ui, rect, id, appearance, message, restart, kind)`에 `kind: StatusKind { Failed, Exited }`를 받아 글자색을 고릅니다. 글자 14px, 문구와 버튼 사이 8px입니다. 버튼은 테두리 1px·배경 터미널 배경인 outline 모양으로 그립니다(`taide-native-ui/src/settings-view.rs` `outline_buttons`와 같은 시각 규칙을 이 함수 안에서 지역 적용. UI crate의 private 함수를 끌어오지 않음).
- 호출부: `failed` 분기는 `Failed`, `ended_message`의 `Phase::Exited`는 `Exited`. `Phase::Failed`(실행 중 내부 실패)는 원본에 대응 화면이 없어 지금처럼 종료 문구를 쓰되 `Exited` 색입니다.

#### 자동 검사

- 앱 lib(실제 App, `application-context-menu-tests.rs` fixture)
    - `shell_override`를 없는 경로로 두고 터미널 탭 열기 → 오류 toast 1개, 그 탭 영역에 `describe_error` 문구 shape(색 = 테마 `statusIndicator.error`)과 `terminal.restart` 버튼
    - `shell_override = "/bin/cat"`로 고치고 버튼 클릭 → 새 세션 attach, 실패 화면 사라짐, 입력 `abc\n` 에코가 터미널 텍스트에 보임
    - `/bin/cat` 세션에 EOF를 보내 종료 → 종료 문구 색 = `appSidebar.iconDefault`

#### 완료 판정

- 위 검사 통과 → complete. 비고: 버튼 픽셀은 배치 31.

---

### 2.8 terminal-agent-45 클립보드 가용성 probe

#### 원본 동작

- `src/widgets/terminal-pane/terminal-clipboard-availability.ts` 1-28: `navigator.clipboard.writeText/readText`가 있는지로 secure context를 판정합니다. 이 판정이 거짓인 곳은 평문 LAN HTTP로 서빙되는 원격 미러뿐입니다(파일 주석 6-15).

#### 할 일

- native 앱은 OS 클립보드를 직접 쓰므로 probe 대상이 없습니다. 원격 미러는 동결 범위입니다.
- 대응표를 not-applicable로 바꾸고 비고에 "secure context 판정 전용, 데스크톱 앱의 결과(Copy=선택 있음, Paste=항상)는 32의 메뉴 검사가 근거"를 적습니다(결정 4).

#### 완료 판정

- 재판정만 합니다.

---

### 2.9 terminal-agent-49 터미널 설정

#### 원본 동작

- `src/widgets/settings-view/settings-terminal-section.tsx` 35-97: 글꼴 크기(6~48), 글꼴 family picker(로딩 중 `settings.loading`), shell 텍스트(placeholder `/bin/zsh`, blur 시 trim 저장), shell profile 목록(조회 중 `settings.loading`), scrollback(100~100 000, 힌트), cursor style `bar|block|underline`, blink 스위치
- 실행 중 반영: 크기·family(재fit), scrollback, cursorStyle, cursorBlink(`terminal-view.tsx` 217-257). shell override는 새 터미널부터(`pty_default_options`)

#### native 현재 자산

- UI: `taide-native-ui/src/settings-code-view.rs`(scrollback draft 96, shell profiles 152·415-437, 힌트 164), `settings-code-controls.rs`(cursor, blink, shell)
- 앱: `terminal_settings.rs` `cursor`, `Hub::configure_cursors`, `terminal_appearance.set_font_size`(`background_tick`), `terminal_fonts.rs`(family 재적재), shell profiles(`host.rs`의 `taide_runtime::terminal_actions::shell_profiles`), 새 spawn shell override(1464행 검사가 사용)

#### 빠진 것

- 실행 중 scrollback(11)
- 실행 중 반영 전체의 App 증거

#### 자동 검사

- 앱 lib(실제 App, `/bin/cat` 터미널 1개, settings 패치는 기존 설정 저장 경로 사용)
    - scrollback 변경 → 실행 중 세션 `history()` 반영
    - cursor style·blink 변경 → `cursor_style()` 반영
    - 글자 크기 변경 → 셀 크기 변화와 PTY resize 요청
    - shell override 변경 → 기존 세션 셸 그대로, 새 터미널 탭은 새 셸
- 이 검사는 settings 사양 settings-30 `[A]` 검사와 같은 것입니다. 이 lane이 쓰고 settings lane은 재사용합니다.

#### 완료 판정

- 11 완료 + 위 검사 통과 → complete. settings-30 같은 근거.

---

### 2.10 terminal-agent-51 agent 상태 UI 동기화

#### 원본 동작

- `src/entities/agent/agent.query.ts` 56-66: `agent:state-changed` payload(프로젝트의 전체 최신 agent 목록)를 `AGENT.PROJECT(projectId)` 캐시에 그대로 넣습니다.
- `src/app/providers/agent-state-sync-provider.tsx` 1-17: 이 구독을 앱 루트(주 창·모든 보조 창)에 둬서, sidebar가 사라지는 Zen에서도 탭 바 agent 표시가 갱신됩니다.

#### native 현재 자산

- 생산: `application-ports.rs` `start_agent_poll`(30-55, `NativeApplication` 생성에서 시작) → `agent_actions::poll_agents` → `AgentStore::diff` 저장·`AgentStateChanged` 발행. hooks 서버도 같은 저장소를 갱신합니다.
- 소비: rail(`shell.rs` `project_button`, `surfaces.project_agents`)과 탭 바(`shell.rs`의 탭 바 `surfaces.project_agents`)가 매 frame `AppSurfaces::project_agents` → `services.agents.agents_for(project)`를 읽습니다. `PaintSink::publish`는 모든 이벤트에 `request_repaint`를 합니다.
- 결과: 캐시 없이 저장소를 직접 읽으므로 Zen·보조 창에서도 같은 값을 그립니다. 검사는 메인 창 탭·rail만 있습니다(shell-35, shell-75).

#### 빠진 것

- Zen과 보조 창, 이벤트로 인한 다시 그리기의 App 증거

#### 자동 검사

- 앱 lib(`application-terminal-agent-tests.rs`, `application-tab-menu-tests.rs` 방식)
    - 메인 창 Zen 켬, 터미널 탭(세션 id 있음) → `AgentStore::diff`로 working agent 넣고 `AppEvent::AgentStateChanged` 발행 → `context.has_requested_repaint()`, 다음 paint에서 `native-glyph-Sparkles-` 텍스처 생성
    - 같은 세션을 idle로 바꾸면 Sparkles 색이 `appSidebar.iconAgentIdle`로 바뀜(같은 탭 아이콘 색 인자 확인)
    - 보조 창에 같은 프로젝트 터미널 탭 → 그 창 탭 바도 Sparkles

#### 완료 판정

- 위 검사 통과 → complete.

---

### 2.11 terminal-agent-52 rail 프로젝트 아이콘의 agent 배지·접근성 라벨

- 원본: `src/widgets/app-sidebar/app-sidebar.tsx` 84-136, `src/features/project/project-icon-button.tsx` 42-59, `agent-status-badge.tsx`
- native: `taide-native-ui/src/project-rail.rs`(`agent_status_label_key` 215-, `top_priority_agent` 239-, `paint_agent_badge` 277-), `shell.rs` `project_button`(대표 agent, `이름 — 상태` 접근성 이름, 배지 설정 off면 숨김)
- 근거: shell-35 complete, `2026-10-26-native-project-rail-display.md`(UI workbench·unit·실제 App)
- 할 일: 대응표 비고를 shell-35 근거로 갱신, complete

### 2.12 terminal-agent-53 터미널 탭 아이콘(Sparkles·활동 색)·tooltip

- 원본: `src/widgets/editor-area/pane-tab-bar.tsx` 53-62, 185-197, `src/features/tab/tab-item.tsx` 58-65
- native: `taide-native-ui/src/glyph-icons.rs` `tab(kind, has_agent)`, `shell.rs` 탭 바의 agent 색 인자·`agent.sessionTooltip`, `application.rs` `AppSurfaces::tab_icon`
- 근거: shell-75 complete, `2026-10-27-native-tab-icons-menu.md`(UI workbench 2건, glyph 단위, 실제 App AgentStore 합성 agent → Sparkles 텍스처)
- 할 일: 대응표 비고 갱신, complete

---

### 2.13 terminal-agent-67 IDE openFile 도구

#### 원본 동작

- `src-tauri/src/domain/ide/server.rs` 73-97 `tool_open_file`: `filePath` 필수, 열린 프로젝트 안의 실제 파일만, `IdeLayoutActions::open_file_tab` → `src-tauri/src/lib.rs` 246-259 `layout_service::open_tab_and_finish(project, File{path}, title, target None, preview)`
- `makeFrontmost`(기본 true)는 응답 모양만 바꿉니다. true면 `Opened file: <path>` 텍스트, false면 `{success, filePath, languageId}` JSON. 창을 앞으로 가져오거나 커서·선택(`startText`/`endText`)을 옮기는 코드는 없습니다.
- 프로젝트 활성화도 하지 않습니다. 탭은 그 프로젝트 layout의 메인 창 포커스 pane에 열립니다.

#### native 현재 자산

- `ide-tools.rs` `open_file`(119-160)이 같은 검증·응답, `LayoutActions::new`(42-80)의 `open_file_tab`이 `layout_actions::open_tab_and_finish`. 열기 전에 `flush_view_states`(147)
- 근거: `ide-tools-tests.rs`의 layout·응답 검사(420, 574행)
- 대응표 비고의 "위치/선택 및 창 전면 요청"은 원본에 없는 동작입니다.

#### 빠진 것

- IDE 스레드에서 연 탭이 실제 `NativeApplication` 화면(포커스 pane 활성 탭)과 문서 로드까지 닿는 App 증거

#### 자동 검사

- 앱 lib(`application-terminal-agent-tests.rs`)
    - 실제 App, 프로젝트 파일 2개 중 1개 열림. `start_ide` 뒤 `application.runtime.block_on(ide_tools::dispatch(services, &LayoutActions::new(hub).with_view_states(<앱의 shared view states>), "openFile", {"filePath": 두번째, "preview": false}))`
    - 다음 frame들에서 컨트롤러 snapshot의 포커스 탭이 그 파일, 문서가 로드돼 본문이 그려짐, 응답 텍스트 `Opened file: …`
    - `makeFrontmost:false` → JSON 응답과 같은 탭 결과, 창 viewport 명령(포커스·Visible) 없음
    - 열린 프로젝트 밖 경로 → RPC 오류, 탭 변화 없음
    - 다른(포커스 아닌) slot 프로젝트 파일 → 그 프로젝트 layout에만 탭, 포커스 slot 그대로

#### 완료 판정

- 위 검사 통과 → complete.

---

### 2.14 terminal-agent-71 IDE close_tab·closeAllDiffTabs

#### 원본 동작

- `src-tauri/src/domain/ide/server.rs` 299-322 `tool_close_tab`: `tab_name`으로 모든 프로젝트 layout에서 제목이 같은 첫 탭을 찾아 `close_tab_and_finish`. dirty 확인 없이 닫습니다. 닫혔으면 `IdeCloseTabRequested { tab_name, request_id }` 발행. 응답은 항상 `TAB_CLOSED`
- 324-345 `tool_close_all_diff_tabs`: 모든 ClaudeDiff 탭을 닫고 `CLOSED_<n>_DIFF_TABS`
- 닫힘 observer(`src-tauri/src/lib.rs` 326-335): IDE `reconcile_closed_tab`(대기 diff → `TabClosed`) 후 터미널이면 PTY kill
- 대응표 비고의 "dirty 확인"은 원본에 없는 동작입니다.

#### native 현재 자산

- `ide-tools.rs` `close_tab`(304-331), `closeAllDiffTabs`(387-408): 같은 규칙에 닫기 전 `flush_view_states`
- `LayoutActions::new`의 `close_tab`(60-78): `reconcile_closed_tab`, 터미널이면 `kill_session` + `Hub::discard`
- `IdeStore::reconcile_closed_tab`(`crates/taide-ide/src/store.rs` 211-218)
- shell lane이 같은 경로에 wait marker 해제를 붙입니다(shell 사양 2.4).

#### 빠진 것

- IDE가 닫은 탭이 실제 App 화면에서 사라지고 대기 diff가 `TabClosed`로 끝나는 App 증거. ClaudeDiff 표면(git-50)이 생긴 뒤 표면 상태 해제는 git-52 범위입니다.

#### 자동 검사

- 앱 lib(`application-terminal-agent-tests.rs`)
    - dirty 파일 탭(`보관.rs`)과 `/bin/cat` 터미널 탭 → `close_tab {"tab_name": 파일 제목}` → 확인 대화상자 없이 탭 사라짐, 응답 `TAB_CLOSED`, `IdeCloseTabRequested` 발행
    - 터미널 탭 제목으로 `close_tab` → Hub에서 세션 제거, 화면 갱신
    - layout API로 ClaudeDiff 탭 2개(대기 diff 2개 `insert_pending_diff_owned`) → `closeAllDiffTabs` → `CLOSED_2_DIFF_TABS`, 두 대기 응답 `TabClosed`, 화면에 탭 없음
    - 없는 제목 → `TAB_CLOSED`, 변화 없음

#### 완료 판정

- 위 검사 통과 → complete. 비고: ClaudeDiff 표면 수명은 git-52.

---

### 2.15 terminal-agent-72 IDE 선택 push

- 원본: `src/widgets/editor-pane/use-editor-ide-selection.ts` 9-55(IDE 실행 중일 때 커서·선택 변화 300ms debounce 뒤 `ide_set_selection`, 해제 시 `ide_clear_selection`)
- native: `editor-ide-selection.rs`, `ide-tools.rs` `getCurrentSelection`/`getLatestSelection`(348-359)
- 근거: editor-83 complete, `2026-10-12-native-batch23-ide-selection.md`(실제 App 3건: debounce·UTF-16 범위·초기 reveal 제외·readonly·보조 창·닫힘)
- 할 일: 대응표 비고 갱신, complete

---

### 2.16 terminal-agent-47 Run Selected Text in Terminal

- 원본: `src/entities/terminal/terminal.commands.ts` 5-12(`terminal.runSelectedText`, 범주 터미널, `requestEditorPaneCommand('run-selected-text-in-terminal')`), `src/widgets/editor-area/editor-area.tsx` 283-288(포커스 pane의 저장 가능 편집기에서 비어 있지 않은 주 선택, 없으면 현재 줄 → `runInTerminal(text, null)`)
- native: `taide-native-ui/src/command-registry.rs` `execution`(351-356: `terminal.runSelectedText`·`monaco.taide.runSelectedTextInTerminal` → `Run::EditDocument(DocumentEdit::RunSelectedText)`), `editor-context-menu.rs` `Request::run_selected_text`(217-), `application.rs`의 `Action::RunSelectedText` 처리
- 근거: editor-82 complete, `2026-10-12-native-batch23-terminal-run.md`(두 팔레트·재지정 키·현재 줄·Unicode/역방향 선택·같은 세션 재사용·보조 창, 실제 App 3건)
- 할 일: 대응표 비고 갱신, complete. commands-ui-40도 같은 근거(1.4)

### 2.17 terminal-agent-48 run-in-terminal 쓰기 경로

- 원본: `editor-area.tsx` 249-281 `runInTerminal(text, cwd)`
    - 포커스 pane에서 활성 터미널, 없으면 첫 터미널(`resolveRunTargetTerminalTab`) → 활성화 → `text + "\n"`
    - 없으면 포커스 pane에 `cwd` 터미널 탭(`terminal.title`, preview false)을 열고 만들어진 탭에 씀, 실패 `toast.error(describeIpcError)`
    - `src/shared/lib/bridge/terminal-write-bridge.ts` 14, 25: 세션이 준비될 때까지 탭별 대기열(최대 500건, 30초 TTL)
- native: `editor-context-menu.rs` `TerminalDestination`(81-), `open_terminal_target`(활성 > 첫 > 새 탭), `HostReply::EditorTerminalTarget` → `editor_terminal_writes` 대기 → `flush_dirty`에서 Hub에 세션이 생기면 `HostCommand::WriteEditorText`, attach 실패면 대기 제거(`HostReply::TerminalAttached`), 열기 실패 `report` toast
- 근거: editor-82(`2026-10-11-native-batch23-context-menu-app.md`: `/bin/cat` 실제 PTY에서 새·기존 세션 전달, `application-context-menu-tests.rs` 1464행: attach 실패 → 대기 회수·다음 터미널로 재전송 안 함), shell-24(task `cwd` 새 탭)
- 판단: 원본의 500건·30초 상한은 내부 메모리 한도입니다. native는 탭이 닫히거나 attach가 실패하면 대기를 버려 같은 목적을 이룹니다. 수치는 재현하지 않습니다.
- 할 일: 대응표 비고 갱신, complete

---

### 2.18 terminal-agent-56 CLI 설치 상태·설치·제거·외부 에디터 연결 명령

#### 원본 동작

- 명령(`src/entities/agent/agent.commands.ts` 18-66, macOS에서만 등록, 범주 `keymap.category.shellCommand`)
    - `cli.connectExternalEditor`(`keymap.cliConnectExternalEditor`): 상태 조회 → `installed && !dangling`이면 그대로, 아니면 설치 → 결과가 `!installed || dangling`이면 `toast.error(settings.cliInstallFailed)`, 아니면 `toast.success(settings.cliExternalEditorConnected)`. 예외도 `cliInstallFailed`
    - `cli.installShellCommand`(`keymap.cliInstall`): 설치 성공 `settings.cliInstallSuccess`, 실패 `settings.cliInstallFailed`
    - `cli.uninstallShellCommand`(`keymap.cliUninstall`): 성공 `settings.cliUninstallSuccess`, 실패 `settings.cliUninstallFailed`
- 백엔드(`src-tauri/src/domain/agent/commands.rs`)
    - 86-88 상태: `taide_runtime::agent_host::cli_install_status("/usr/local/bin/taide")`
    - 121-127 대상: 실행 파일이 `.app` 번들 안이면 같은 폴더의 `taide-cli`, 아니면 오류 "taide CLI shell command is only available in the installed TAIDE.app (not a dev build)"
    - 129-142 osascript: `-e <script>`, 성공 또는 사용자 취소(exit 1 + stderr `-128`)는 `Ok`, 그 밖은 `Internal("osascript failed: …")`
    - 183-195 설치: 이미 같은 대상 symlink면 생략, 아니면 관리자 권한 `mkdir -p /usr/local/bin && ln -sf`
    - 207-235 제거: 번들 gate, 없으면 현재 상태, TAIDE 소유 symlink(대상 이름 `taide-cli`)가 아니면 "the existing 'taide' command is not managed by TAIDE and was not removed", `remove_file`, 권한 거부면 관리자 `rm`
    - macOS 아니면 "… only supported on macOS"
- 설정 행(`agent-cli-status-row.tsx`)은 settings-16입니다(settings 사양 2A).

#### native 현재 자산

- `crates/taide-agent/src/service.rs` 195-254: `resolve_cli_install_target`, `build_cli_install_apple_script`, `build_cli_uninstall_apple_script`, `build_osascript_args`, `is_osascript_user_cancelled`, `is_cli_symlink_owned`
- `crates/taide-runtime/src/agent_host.rs` 100 `cli_install_status`
- `agent-hooks.rs` 24-26 `TAIDE_CLI_TARGET_PATH`, `terminal_environment.rs`(EDITOR는 이미 주입)
- 명령 카탈로그: `taide-native-ui/src/keybinding-commands.json` 1363-1380(`cli.*` 3종, `"platform": "mac"`), 레지스트리 `is_mac_only`
- 실행 경로 없음(`Execution::Unavailable`), 원격 `remote-agents.rs`는 상태 조회만

#### 구현 설계

- 새 모듈 `native/taide-native-app/src/agent-cli.rs`(`crate::agent_cli`). 공유 crate는 바꾸지 않습니다(결정 2 A안).
    - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Action { Status, Install, Uninstall, ConnectExternalEditor }`
    - `pub(crate) struct Ports { executable: Arc<dyn Fn() -> AppResult<PathBuf> + Send + Sync>, link: PathBuf, osascript: Arc<dyn Fn(&str) -> AppResult<()> + Send + Sync> }`
        - `Ports::system()`: `std::env::current_exe`, `agent_hooks::TAIDE_CLI_TARGET_PATH`, 실제 osascript 실행기(원본 129-142와 같은 판정)
    - `status(&Ports) -> CliInstallStatus`, `install(&Ports) -> AppResult<CliInstallStatus>`, `uninstall(&Ports) -> AppResult<CliInstallStatus>`: 위 원본 분기와 오류 문자열을 그대로 옮깁니다. macOS 판정은 `cfg!(target_os = "macos")`
    - `run(&Ports, Action) -> AppResult<CliInstallStatus>`: `ConnectExternalEditor`는 `status`가 `installed && !dangling`이면 그대로, 아니면 `install`
    - 주입: host 통합(`HostIntegrations`)에 `agent_cli: Arc<agent_cli::Ports>`(기본 `system()`), `#[cfg(test)]` 교체 함수. shell 사양 2.5의 알림 포트와 같은 방식입니다.
- `domains::terminal_agent`
    - `Command::Cli(action)` → 2.0.2 규칙대로 `spawn_transient("native-agent-cli", …)`에서 `run_nonabortable_result`로 `agent_cli::run` → `HostReply::TerminalAgent(Reply::Cli { action, result })`
    - `apply_terminal_agent_reply`
        - 모든 `Ok(status)`는 `State.cli_status`에 저장(settings 행이 읽음)
        - `Install`: Ok이고 `installed && !dangling` → success `settings.cliInstallSuccess`. Ok이지만 미설치(관리자 대화상자 취소) → toast 없음(결정 7 A안). Err → error `settings.cliInstallFailed`
        - `Uninstall`: Ok이고 `!installed` → success `settings.cliUninstallSuccess`, Ok이지만 아직 설치됨(취소) → toast 없음, Err → error `settings.cliUninstallFailed`
        - `ConnectExternalEditor`: Ok이고 `installed && !dangling` → success `settings.cliExternalEditorConnected`, 그 밖(Err 포함) → error `settings.cliInstallFailed`
        - `Status`: toast 없음
- UI(`taide-native-ui`)
    - `intents/terminal_agent.rs`: `pub enum CliAction { ConnectExternalEditor, Install, Uninstall }`, `pub enum Intent { #[cfg(feature = "native-host")] Cli(CliAction) }`
    - `command-registry.rs`: `Run::Cli(CliAction)`(native-host), `execution`에서 세 id → `Run::Cli`. 활성 조건은 기존(`Always`, mac 전용은 카탈로그 `platform`). 실행 경로 표 검사(1182-1230, 1667, 1822 부근)에 세 명령을 추가
- 앱
    - `shell_keymap.rs` `intent`: `Run::Cli(action)` → `ShellIntent::TerminalAgent(Intent::Cli(action))`. 프로젝트 없이도 실행됩니다(원본과 같음).
    - `command-dispatch.rs`의 실행 가능 명령 표 갱신
    - `apply_terminal_agent_intent`: `Intent::Cli(action)` → `HostCommand::TerminalAgent(Command::Cli(action.into()))`, 제출 실패면 해당 실패 toast

#### 자동 검사

- 앱 lib 단위(`agent-cli-tests.rs`, 임시 폴더)
    - 설치: 번들 밖 exe → dev build 오류. `<tmp>/X.app/Contents/MacOS/taide-native` exe + `taide-cli` 파일 → 미설치면 osascript 1회(스크립트에 link·대상 경로), 가짜 실행기가 symlink 생성 → installed. 이미 같은 대상 → osascript 0회. 실행기가 아무것도 안 함(취소) → `Ok`, 미설치
    - 제거: link 없음 → `Ok`, osascript 0회. 다른 이름 대상 symlink → 소유 아님 오류. 소유 symlink → 삭제, 미설치
    - 연결: installed·!dangling → 설치 안 함. dangling(대상 없는 symlink) → 설치 호출
    - macOS 아닌 빌드 분기는 `cfg`로 해당 플랫폼에서만 검사
- 앱 lib(실제 App, 포트 주입, `application-terminal-agent-tests.rs`)
    - ⇧⌘P 팔레트에서 `cli.installShellCommand` 실행 → success toast 문구 `settings.cliInstallSuccess`. 오류 포트 → `settings.cliInstallFailed`. 취소 포트 → toast 없음
    - `cli.connectExternalEditor` → `settings.cliExternalEditorConnected`
    - 설치 포트를 barrier로 붙잡은 동안 `HostCommand::Commands(Command::DetectTasks)` 응답이 먼저 도착(host worker 비차단)
- UI 단위: 레지스트리에서 세 명령의 실행 경로 `Run::Cli`, `is_mac_only`
- 동결 remote-web Wasm check

#### 완료 판정

- 위 검사 통과 → 명령 부분 완료. 행 상태는 partial(비고: 설정 행 settings-16 남음). settings lane이 행을 붙이면 complete. commands-ui-32~34 같은 근거.

---

### 2.19 terminal-agent-76 Task Runner 대화상자

#### 원본 동작

- `src/widgets/task-runner/task-runner-dialog.tsx` 29-74: 숨은 제목·placeholder `task.runTask`, 열린 동안만 `detect_tasks`, 빈 목록 `common.loading`/`task.noTasksFound`, `fuzzyFilter(query, tasks, label)`, 행 = Terminal 아이콘 + 이름 + 같은 줄 출처(`task.sourceNpm|Make|Cargo`, 작은 muted), 선택 → `run-in-terminal(command, cwd)` 후 닫고 질의 비움
- `src/app/app.tsx` 65-89: 보조 창에도 `TaskRunnerDialog projectId={보조 창 프로젝트}`가 mount되어 그 창의 포커스 pane에서 실행합니다.

#### native 현재 자산

- `taide-native-ui/src/command-palette.rs` `open_tasks`(472-)·`show_tasks`(485-)·`is_picking_tasks`(496-), 행·출처 문구(825-856)
- `domains/commands.rs`: `Intent::OpenTaskRunner`(151-169), `Command::DetectTasks`(64-75), `Action::RunTask`(350-372, 현재 창 `self.shell.scope`의 포커스 pane으로 `TerminalDestination`)
- 팔레트는 `show_overlays`가 overlay viewport(포커스 창, `domains::shell`의 `overlay_viewport`)에서만 그립니다.
- 근거: shell-24 complete, `2026-10-28-native-shell-keymap-dialogs-slot-header.md`(UI palette 2건, 실제 App 팔레트 → 감지 → 실행). 보조 창은 keymap intent 검사만 있습니다.

#### 빠진 것

- 보조 창에서 연 Task Runner가 그 창에 보이고 그 창 포커스 pane으로 실행되는 App 증거

#### 자동 검사

- 앱 lib(`application-terminal-agent-tests.rs`, `application-auxiliary-window-tests.rs`·`application-shell-dialogs-tests.rs` 방식)
    - 보조 창(같은 프로젝트, 터미널 없는 pane) 포커스 → `task.runTask` → 팔레트가 보조 viewport 출력에 있고 ROOT에는 없음
    - fixture에 `package.json`·`Makefile`·`Cargo.toml` → 세 출처 문구
    - 항목 선택 → 보조 창 포커스 pane에 `cwd`=root, `terminal.title` 터미널 탭, `editor_terminal_writes`에 `command\n`
    - 실패하면 이 묶음 안에서 `domains/commands.rs` `show_palette`/`Action::RunTask`의 scope를 고칩니다(commands 영역 파일이므로 수정 범위를 QA에 적음).

#### 완료 판정

- 위 검사 통과 → complete.

### 2.20 terminal-agent-77 `task.runTask` 명령

- 원본: `src/entities/task/task.commands.ts`(활성 프로젝트가 있을 때만, 기본 키 없음, 키 바인딩 편집기 목록에 노출)
- native: `command-registry.rs` 334(`ActiveProject`), 445(`Run::OpenTaskRunner`), `keybinding-commands.json` 1411(카탈로그), `command-dispatch.rs` 808-809(활성 조건 검사), `shell_keymap.rs` `Run::OpenTaskRunner`(127-, 1319-1332 Zen·split 검사)
- 근거: shell-24 QA의 registry·dispatch·keymap 검사
- 할 일: 대응표 비고 갱신, complete. commands-ui-41 같은 근거

### 2.21 terminal-agent-78 태스크 감지(npm·make·cargo)

- 원본: `src/entities/task/task.ipc.ts` → `src-tauri/src/domain/task/commands.rs` 9-13 → `crates/taide-runtime/src/task_actions.rs` 29-34 → `taide_task::service::detect_tasks`
- native: `domains/commands.rs` `Command::DetectTasks` → 같은 `task_actions::detect_tasks`. 감지 규칙은 공유 crate(`crates/taide-task/src/service.rs`, cargo 검사 366·384행 등)
- 근거: shell-24 App 검사(package.json·Makefile), taide-task 단위 검사(cargo), 2.19의 보조 창 검사(세 출처)
- 할 일: 대응표 비고 갱신, complete

---

### 2.22 묶음 4: 다른 lane이 구현하는 ID

결정 1 A안 기준입니다. 이 lane은 코드를 쓰지 않고, 소유 lane 병합 뒤 그 QA 근거로 행을 갱신합니다. B안이면 아래 "설계 위치"를 이 lane이 구현합니다.

| ID | 원본 동작(근거) | 설계 위치 | 이 lane의 행 갱신 조건 |
| --- | --- | --- | --- |
| 55 hooks UI | 토글 `settings.agentHooks`(기본 off) 켜짐일 때만 목록, claude는 프로젝트별 행(설치됨 `agent.hooksInstalled` + 해제, 아니면 설치 → 동의 대화상자 `agent.hooksConsentTitle`), codex·gemini·opencode·pi는 user 범위 스위치와 CLI 누락 경고(`agent-hooks-project-list.tsx` 29-100, `agent-hooks-project-row.tsx` 41-77, `agent.query.ts` 39-54) | settings 사양 settings-17 | settings lane S1 병합 |
| 57 외부 열기 | `taide <file>`·Ctrl+G·`--wait` marker, 대상 프로젝트 해석, `app.openProjectFirst`·`app.externalEditorTabHint`, 탭이 모두 닫히면 marker 해제(`agent-external-open-provider.tsx` 36-116, `tab-path-change.ts` 147-152) | shell 사양 2.4(shell-90) | shell lane 묶음 1 병합. 실제 CLI 연결은 shell-15 허락 |
| 59 agent 알림 | working 10초 이상 뒤 idle·awaitingInput이면 범주 `agentCompleted`/`agentAwaitingInput`, 제목 프로젝트 표시 라벨, 본문 차단 사유별 키·탭 제목 꼬리(`native-notification-provider.tsx` 143-161) | shell 사양 2.5(shell-91) | 같음 |
| 60 명령 완료 알림 | `durationMs >= 10000`, 성공 `taskCompleted`/실패 `error`, 본문 `[탭 제목 ?? cwd ?? terminal.title, exitCode, 경과]`(같은 파일 178-194) | shell 사양 2.5 | 같음 |
| 61 LSP 설치 알림 | `done`/`failed`만, `lspInstall`/`error`, 제목 `notification.lspInstallSucceeded|Failed`, 본문 `message ?? serverId`(196-204) | shell 사양 2.5 | 같음 |
| 62 첫 전달 안내 | 세션에 처음 `delivered`면 `notification.enableHint` info toast 1회. `notifyNative`를 거치는 모든 알림(git push/pull, 검색 치환 포함)이 계기이고 설정 시험 버튼은 계기가 아님(137-141, `notify.ts` 37-50) | shell 사양 2.5 | 같음. git·explorer-search 알림도 같은 진입점을 써야 함 |
| 63 게이트·전송 | 메인 창·비미러만, 제목·본문 200 코드 포인트(초과 199 + `…`), Rust 게이트(마스터 → 범주 → 포커스 없을 때만), 비밀 가림, 실패 삼킴(`notify.ts` 37-50, `notification.ipc.ts` 14-15) | shell 사양 2.5 | 같음 |
| 64 시험 알림·시스템 설정 | 버튼 2개: 시스템 설정 열기(`x-apple.systempreferences:com.apple.Notifications-Settings.extension`, 비 macOS `error.notification.settingsUnsupported`), 시험 알림(범주 error, 제목 `settings.notificationsSendTest`, 본문 `notification.enableHint`, suppressed 사유별 warning toast)(`settings-notification-section.tsx` 23-63, 115-125) | settings 사양 settings-21·22·82 | settings lane S1 병합 |
| 68 Claude diff 탭 | `ideAutoOpenDiff` 꺼짐 → 즉시 rejected, 켜짐 → `ClaudeDiff` 탭, 원본·제안 비교, 제안 편집, Accept/Reject, 탭 닫힘 → TabClosed(`ide-sync-provider.tsx` 49-68, `claude-diff-pane.tsx`) | git 사양 git-49~52 | git lane 병합 |
| 70 saveDocument | 모든 트리에서 같은 경로 파일 탭, 없거나 깨끗하면 true, 모델 없음·readOnly면 false, 저장 성공 true(`ide-sync-provider.tsx` 70-129) | shell 사양 2.6(shell-92 save) | shell lane 묶음 1 병합 |
| 73 진단 push | IDE 실행 중·포커스 slot 프로젝트 있을 때 표식 변화 300ms debounce 뒤 전체 진단(LSP 0 기준 범위·severity·source), 없으면 빈 목록(`ide-sync-provider.tsx` 137-158) | shell 사양 2.6(shell-92 진단) | 같음 |

- 행 갱신 시점: 57·59~63·70·73은 wave 1 shell lane 병합 직후, 55·64·68은 wave 2 병합 직후입니다. terminal-agent lane worktree는 wave 1 끝에 닫히므로 이 갱신은 메인이 병합 게이트에서 하거나 wave 3 commands-ui lane이 "진입만 남은 행"으로 처리합니다.
- shell 사양 2.6 완료 판정은 terminal-agent-73만 적었습니다. 70(save 요청)도 같은 근거로 갱신합니다.

## 3. 작업 묶음 계획

| 묶음 | ID | 내용 | 수정 예상 파일 | 새 의존성 |
| --- | --- | --- | --- | --- |
| 1 터미널 표면·코어(wave 1, 먼저) | 6, 11, 19, 26, 27, 32, 38, 45, 49 (9) | set_history와 Hub 전파, 숨은 세션 명령 색, Option-as-Meta, 실패·종료 화면 색, 재판정 4건(6·26·32·45), 설정 실행 중 반영 App 검사 | 터미널: `taide-native-terminal/src/lib.rs`, `src/session.rs`, vendored `vendor/alacritty-terminal/src/term/mod.rs`, `tests/core.rs`(또는 새 `tests/history.rs`). 앱: `terminal_host.rs`(`configure_history`, `configure_command_colors`), `terminal_surface.rs`(`input_event`, `meta_character`, `View`, `Appearance`, `status`, 테스트 모듈), 새 `domains/terminal_agent.rs`(tick), `application.rs`(필드·tick 호출·테스트 모듈 추가만), 새 `application-terminal-agent-tests.rs`, `tests/terminal-host.rs`(검사 추가). 문서: QA `<날짜>-native-terminal-agent-surface.md`, 대응표 9행 | 없음 |
| 2 IDE·agent 표시(wave 1) | 51, 52, 53, 67, 71, 72 (6) | Zen·보조 창 agent 표시 검사, IDE openFile·close App 도달 검사, 근거 갱신 3건 | 앱: `application-terminal-agent-tests.rs`(검사 추가만). 제품 코드 변경 없음 예상. 문서: QA `<날짜>-native-terminal-agent-ide.md`, 대응표 6행 | 없음 |
| 3 명령·실행 경로(wave 1) | 47, 48, 56, 76, 77, 78 (6) | `agent_cli`와 `cli.*` 명령 3종·toast, 보조 창 Task Runner 검사, 근거 갱신 4건 | 앱: 새 `agent-cli.rs`·`agent-cli-tests.rs`, `domains/terminal_agent.rs`(Command·Reply·intent), `host.rs`(enum 루트·dispatch 1줄·통합 포트 필드, 추가만), `shell_keymap.rs`, `command-dispatch.rs`, `lib.rs`(모듈 추가), `application.rs`(분기 추가만), 검사 파일. UI: 새 `intents/terminal_agent.rs`, `intents/mod.rs`, `commands.rs`(`ShellIntent` variant 추가), `command-registry.rs`(`Run::Cli`, 검사 표). 필요하면 `domains/commands.rs`(2.19 실패 시). 문서: QA `<날짜>-native-terminal-agent-commands.md`, 대응표 6행 | 없음 |
| 4 다른 lane 병합 뒤 행 갱신(wave 1 끝~3) | 55, 57, 59, 60, 61, 62, 63, 64, 68, 70, 73 (11) | 코드 없음(결정 1 A안). 소유 lane QA를 근거로 행 갱신 | 대응표 11행, 영역 집계 | 없음 |

- 순서: 1 → 2 → 3. 묶음 2는 제품 코드를 거의 바꾸지 않아 짧고, 묶음 3은 UI crate를 바꾸므로 동결 remote-web Wasm check가 필요한 유일한 묶음입니다.
- 묶음마다: 실패 검사 → 구현 → 영역 검사(2.0.3) → 대응표 행·영역 집계 갱신 → QA 문서. 커밋은 workflow 종료 뒤 메인이 합니다.
- 새 의존성 대안: 56에서 `objc2`·AppleScript 바인딩 대신 기존과 같은 `osascript` 프로세스를 씁니다. 19는 xterm 표를 코드로 옮기며 키보드 배열 crate를 쓰지 않습니다.
- 대응표 영역 집계(terminal-agent 행 78): 묶음 1~3 뒤 대상 32행은 complete 18, not-applicable 2, partial 1(56), 묶음 4 대기 11(현재 상태 유지)입니다. 영역 전체는 complete 44 → 62, not-applicable 2 → 4, partial 12 → 5(59·60·61·63과 56), unwired 13 → 5(55·57·68·70·73), missing 7 → 2(62·64)로 예상합니다(합계 78). 실제 수는 행 갱신 결과로 다시 셉니다.
- 대상 `CliInstallStatus`는 `taide_model::agent::CliInstallStatus`(`crates/taide-model/src/agent.rs` 85-)입니다.

## 4. 결정과 허락이 필요한 항목

### 4.1 사용자(메인) 결정

모두 추천안이면 "전부 추천안대로"로 답하시면 됩니다. 1은 shell 사양 결정 1과 같은 결정이라 한 번만 정하면 두 lane에 같이 적용됩니다. 1·3은 묶음 1 시작 전에, 2·7은 묶음 3 시작 전에, 4~6은 묶음 1 완료 판정 전에 필요합니다.

1. 중복 행 소유(57, 59~63, 70, 73 / 55, 64 / 68)
    - A안(추천): shell lane이 57·59~63·70·73, settings lane이 55·64, git lane이 68을 구현합니다. 이 lane은 묶음 4에서 행만 갱신합니다. 이유: 원본도 provider·설정 화면 한 곳이 소유하고, 같은 wave의 두 lane이 같은 코드를 만들 위험이 없어집니다. 함께 정할 것: settings 사양의 settings-22(`SettingsCommand::Notify`)·settings-23(`taide-notification/src/producer.rs`) 설계는 shell lane의 `native_notifications`·`ShellCommand::Notify`를 재사용하도록 바꿉니다(현재 두 사양이 같은 생산자를 각각 설계).
    - B안: 이 lane이 소유하고 shell lane은 재판정만 합니다. 묶음 4가 shell 사양 2.4~2.6 설계 그대로 구현 묶음이 되어 이 lane 기간이 크게 늘어납니다.
2. CLI 백엔드 위치와 소유(56 ↔ settings-16)
    - A안(추천): 이 lane이 wave 1에서 앱 모듈 `crate::agent_cli`(공유 crate 무변경)와 host 명령, `cli.*` 명령 3종을 만듭니다. settings lane S1은 행 UI만 만들고 `agent_cli`를 재사용합니다(settings 사양 S1의 "taide-runtime `agent_host.rs`에 install/uninstall 추가" 단계는 빠짐).
    - B안: settings 사양처럼 `crates/taide-runtime/src/agent_host.rs`에 둡니다. 루트 workspace(Tauri 포함) crate를 바꾸고 그 검사를 루트 workspace에서 빌드해야 합니다.
    - C안: settings lane이 56 전체를 맡고 이 lane은 재판정만 합니다.
3. 실행 중 scrollback 구현 소유(11 ↔ settings-30, settings 사양 결정 7)
    - A안(추천): 이 lane이 wave 1에서 `set_history`와 전파, 49의 App 검사까지 만듭니다. settings lane은 settings-30 행 갱신만 합니다.
    - B안: settings 사양 결정 7대로 settings lane이 wave 2에 구현합니다. 11·49는 그때까지 partial입니다.
4. 원본에 없는 기능 행의 판정(6, 26, 45)
    - A안(추천): 6은 셀 단위 그리기 검사 뒤 complete(원본도 합자 없음이라는 화면 동작을 검사로 고정), 26·45는 not-applicable(웹 addon 로드만 / secure context 판정만). 2026-10-06 감사 판정과 같습니다.
    - B안: 셋 다 not-applicable
    - C안: missing 유지(비추천, 원본에 없는 기능을 남은 작업으로 셈)
5. 터미널 메뉴 아이콘(32)
    - A안(추천): 아이콘은 앱 전반 lucide 행(commands-ui-98, wave 3)에서 모든 native 메뉴와 함께 처리하고 32는 complete로 판정합니다. shell-78·79가 같은 이유로 글자만 있는 메뉴를 판정한 선례와 같습니다.
    - B안: 이 lane에서 lucide 11종(SVG를 `node_modules/lucide-react/dist/esm/icons/`의 path 데이터로 만들어 `resources/icons/`에 추가)을 터미널 메뉴에만 먼저 넣습니다. 메뉴마다 모양이 달라지는 기간이 생깁니다.
6. OSC 133 탭 전환 뒤 블록 수명(27, `2026-10-03-m8-native-terminal-ruler.md` 끝절의 미응답 질문)
    - A안(추천): 단일 Core를 유지합니다(전환 계약 "PTY 출력은 하나의 Rust terminal core가 단 한 번 파싱"). 탭을 갔다 와도 블록·색이 그대로 남는 차이를 비고에 적고 27을 complete로 판정합니다. 숨은 세션의 이후 완료는 현재 테마 색을 쓰도록 맞춥니다(2.5).
    - B안: 원본처럼 표시 때 ring을 다시 파싱합니다. 단일 파싱 계약을 바꿔야 하고 부수 효과(알림·cwd 이벤트) 중복 방지 설계가 새로 필요해 비추천입니다.
7. CLI 관리자 대화상자를 취소했을 때의 toast(56)
    - A안(추천): toast를 띄우지 않습니다. 원본 명령은 취소도 성공으로 받아 "설치됨" toast를 띄우지만, 원본 백엔드 주석이 취소를 "quiet no-op"로 정의하고 실제로는 설치되지 않았으므로 결함으로 보고 재현하지 않습니다. `connectExternalEditor`는 원본도 상태를 다시 확인하므로 그대로입니다.
    - B안: 원본처럼 취소해도 success toast

### 4.2 허락이 필요한 항목

작업자 자동 검사는 아래를 하지 않습니다. 실기 확인이 필요하면 사용자 허락 뒤 별도로 진행합니다.

1. 실제 CLI 설치·해제(56): `/usr/local/bin/taide` 쓰기와 관리자 암호 프롬프트. 설치된 Tauri 앱의 symlink를 바꿀 수 있습니다. native 개발 빌드는 `.app` 번들이 아니라 원본과 같은 dev build 오류로 실패하는 것이 정상입니다.
2. 실제 OS 알림과 시스템 설정 앱 열기(59~64): 소유 lane에서 포트 주입으로만 검사합니다.
3. 실제 `taide` CLI → native 연결(57): shell-15(실제 사용자 데이터 경로) 허락 뒤입니다.
4. 실제 사용자 홈의 hooks 파일과 실제 프로젝트 `.claude/settings.local.json` 쓰기(55): settings lane 허락 항목입니다.
5. 배치 31 실기: Option-as-Meta(실제 Option 키, 비 US 배열, dead key, IME 켜짐), 합자 글꼴을 고른 터미널 화면, 실패·종료 화면 버튼 픽셀, 실제 PTY의 task 출력, IDE 클라이언트(Claude Code)와의 openFile·close_tab 왕복.
