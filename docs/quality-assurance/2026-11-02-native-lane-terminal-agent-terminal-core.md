# native lane terminal-agent 묶음 1/5: terminal-core

> 작성일: 2026-11-02 · lane: `terminal-agent` (worktree `TAIDE-lanes/terminal-agent`, 브랜치 `lane/terminal-agent`)
> 기준: `docs/research/2026-10-28-lane-terminal-agent-spec.md` 2.1~2.9절, 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 결정은 추천안), 구조 `docs/research/2026-10-28-native-lane-seams.md`

## 대상 ID와 판정

- [x] terminal-agent-6 리거처 → complete (결정 4 A안)
- [x] terminal-agent-11 실행 중 scrollback 반영 → complete
- [x] terminal-agent-19 Option-as-Meta → partial (리뷰 반영으로 하향: macOS dead key는 ESC 접두가 나가지 않음, 아래 수정 절)
- [x] terminal-agent-26 터미널 내 검색 → not-applicable (결정 4 A안)
- [x] terminal-agent-27 OSC 133 명령 블록 장식 → complete (결정 6 A안)
- [x] terminal-agent-32 터미널 context menu → complete (결정 5 A안, 아이콘은 commands-ui-98)
- [x] terminal-agent-38 spawn 실패 화면·재시작·toast → complete
- [x] terminal-agent-45 클립보드 가용성 probe → not-applicable (결정 4 A안)
- [x] terminal-agent-49 터미널 설정 → complete (settings-30 scrollback 부분과 같은 근거)

## 원본 동작 근거

ID별 원본 근거는 사양 2.1~2.9절을 그대로 따릅니다. 이번 작업에서 사양과 실제 코드가 다르게 확인된 점만 적습니다.

- 38의 재시작 버튼 배경: 사양은 "배경 터미널 배경"이라고 적었지만 원본 `src/shared/ui/button.tsx` 14의 outline variant는 `border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground`입니다. `src/shared/styles/global.css` 138·150·151·153에 따라 `--background`=`app.background`, `--accent`=`list.hoverBackground`, `--accent-foreground`=`list.foreground`, `--border`=`app.border`입니다. 실제 코드를 따라 버튼 배경은 `app.background`, 테두리는 `app.border`, hover 배경·글자는 `list.hoverBackground`·`list.foreground`로 했습니다. 버튼 크기는 `size='sm'`(`h-8 px-3 rounded-md`)를 따라 높이 32, 좌우 여백 12, 모서리 6입니다. 버튼 글자색은 outline variant에 색 지정이 없어 부모(`text-status-error`/`text-muted-foreground`)를 상속하므로 상태 글자색과 같게 했습니다.
- 19의 표: `node_modules/@xterm/xterm/src/common/input/Keyboard.ts` 11-36(`KEYCODE_KEY_MAPPINGS`)과 325-351을 직접 확인했고 사양 표와 같습니다.

## 구현

### 11 실행 중 scrollback (터미널 crate·앱)

- vendored `native/taide-native-terminal/vendor/alacritty-terminal/src/term/mod.rs`: `Term::native_set_scrolling_history(lines)`(native-retained 전용). `set_options`와 달리 Title/ResetTitle 이벤트를 보내지 않고, alt screen이면 `inactive_grid`(primary), 아니면 `grid`에 `update_history`, 전체 damage.
- `native/taide-native-terminal/src/lib.rs`: `TerminalCore::history()`, `TerminalCore::set_history(lines) -> AppResult<bool>`(같은 값이면 false, 현재 grid 크기로 `admit` 실패면 상태 그대로 `Err`, 통과하면 vendored 함수·`history` 갱신·`enforce_retained`). `admit`이 `&impl Dimensions`를 받도록 바꿔 현재 grid로 바로 검사합니다(호출부 2곳은 `&size`).
- `native/taide-native-terminal/src/session.rs`: `SharedTerminal::configure_history(lines) -> AppResult<bool>`. `configure_cursor`와 같은 잠금 순서, `Failed`면 false, 예산 거절 값은 `rejected_history`에 기억해 같은 값이면 즉시 false(매 frame 재시도·경고 반복 방지), 다른 값이 오거나 적용되면 기억을 지웁니다. 적용 중 core가 폐기되면 `Failure::Parser`로 실패 처리합니다.
- `native/taide-native-app/src/terminal_host.rs`: `Hub::configure_history() -> bool`. 설정 `terminal_scrollback`을 모든 세션에 적용하고 한 세션의 거절은 `log::warn!`(세션 id·값·오류 종류만)로 남긴 뒤 계속합니다. 사양 시그니처(`AppResult<bool>`)와 다른 점: 세션별 오류를 로그로 처리하므로 돌려줄 오류가 없어 `bool`만 돌려줍니다.
- `native/taide-native-app/src/domains/terminal_agent.rs`: `tick_terminal_agent_live_sessions(context)`가 `Hub::configure_history()`를 부르고 바뀌면 repaint를 요청합니다. `application.rs` `background_tick`의 기존 `self.tick_terminal_agent_cursors(context);` 바로 뒤에 한 줄로 호출합니다(공용 파일 규칙의 "자기 기존 호출 줄 바로 뒤에 한 줄").
- 실행 중 세션의 PTY replay ring 바이트는 바꾸지 않습니다(원본과 같음). core history는 Hub의 모든 세션에 적용되므로 같은 Hub를 쓰는 원격 spawn 세션(`application.rs` 387 `remote_terminal::Ports.terminals`, `remote-terminal.rs` 104-116)도 갱신됩니다. 원격 spawn의 `history` 클로저 값은 spawn 시점 core 초기값일 뿐입니다(사양 2.2절 구현 설계 6번의 "원격 터미널은 spawn 값이라 그대로"와 다름, 수정 절 참고).

### 27 숨은 세션 명령 색

- `terminal_surface::Appearance::command_colors()` 접근자.
- `Hub::configure_command_colors(colors)`: 모든 세션에 적용, 세션 오류는 `log::warn!` 후 계속. 사양 시그니처(`AppResult<()>`)와 다른 점은 11과 같은 이유입니다.
- `domains::terminal_agent::State.command_colors: Option<CommandColors>`(마지막 전파 값, 생성자 리터럴 끝에 `command_colors: None`). `tick_terminal_agent_live_sessions`가 현재 appearance 색과 비교해 바뀐 때만 Hub에 전달합니다. 보이는 세션의 기존 frame별 `configure_command_colors` 호출은 그대로입니다.
- 결정 6 A안: 단일 Core 유지. 탭을 갔다 와도 기존 블록·색이 남고, 숨은 동안 끝난 명령은 현재 테마 색을 씁니다.

### 19 Option-as-Meta

- `terminal_surface.rs`
    - `View.meta_text_pending`
    - `meta_character(key, shift)`: xterm 표(숫자·기호 21개, Space, A~Z). 논리 키가 표에 없으면 `physical_key`로 다시 찾습니다(사양의 `meta_character(key, physical, shift)`를 호출부의 `or_else`로 나눈 형태).
    - `input_event` 맨 앞: alt 단독(ctrl·cmd 없음)·preedit 없음·표에 있는 키면 `NativeInput::Key { Key::Character(c), alt }`를 보내고 플래그를 세웁니다. 플래그가 선 동안 다음 `Text`/`Ime::Commit`은 버리고, `Ime::Preedit`은 버리며 preedit을 비웁니다. 다른 `Event::Key`가 오면 플래그를 내립니다.
    - 입력 처리 루프 3곳(시작 전 버퍼 2곳, 실행 중 1곳)이 끝날 때 플래그를 내려 다음 frame의 정상 텍스트를 삼키지 않습니다.
    - 방향키·Enter·Backspace 등과 ctrl+alt는 기존 `key_input` 경로 그대로입니다.

### 38 실패·종료 화면

- `Appearance.status: StatusColors`(실패 `statusIndicator.error`, 종료 `appSidebar.iconDefault`, 버튼 `app.background`·`app.border`·`list.hoverBackground`·`list.foreground`). 기존 테마 맵에서 `presentation::color`로 읽으며 다른 appearance처럼 필수 키입니다.
- `status(.., kind: StatusKind)`: 글자 14px, 문구와 버튼 사이 8px, outline 버튼(테두리 1px, 높이 32, 좌우 12, 모서리 6, hover 색). 호출부는 attach 실패면 `Failed`, 그 밖의 종료 문구(`Phase::Exited`, 실행 중 내부 실패 `Phase::Failed`)는 `Exited`입니다. `ended_message` 반환형은 그대로 두고 호출부에서 `failed` 보유 여부로 종류를 정했습니다(기존 문구 검사 무변경).

### 6·26·32·45·49

- 6: 코드 변경 없음. 셀 단위 그리기를 검사로 고정했습니다.
- 26·45: 재판정만(사양 2.4·2.8).
- 32: 코드 변경 없음. 데스크톱 앱에서는 원본도 붙여넣기 항상 켜짐(`terminal-clipboard-availability.ts`의 secure context 판정은 평문 HTTP 원격 미러 전용)이라 대응표의 "붙여넣기 gate 없음"은 차이가 아닙니다. 메뉴 아이콘 11종은 commands-ui-98(결정 5 A안).
- 49: 11 완료와 App 검사로 실행 중 반영 전체를 고정했습니다.

### 범위 밖 최소 수정

- `native/taide-native-app/tests/terminal-host.rs`의 합성 테마 2곳(`terminal_appearance`, headless surface 검사)에 `status_theme_colors()`를 넣었습니다. 38의 상태 색이 필수 테마 키가 되어 `"colors":{}` 합성 테마로는 `Appearance::new`가 실패하기 때문입니다. 실제 테마는 이 키를 모두 갖습니다(`presentation_refresh::Appearances::new`가 `appSidebar.iconDefault`를 이미 필수로 읽음).
- `native/taide-native-terminal/tests/fixtures/session.rs`(앱 `native-terminal-queue-fixture`와 터미널 crate가 함께 쓰는 PTY fixture)에 두 모드를 추가했습니다: `TAIDE_NATIVE_FIXTURE_META`(raw 입력이 `\x1bb\x1bB\x1b!\x1b \x1bex\x1b[1;3D\x1b\x01`와 정확히 같아야 완료 제목), `TAIDE_NATIVE_FIXTURE_LINES`(echo 없는 줄 반사, OSC 133 명령을 숨은 세션에서 만들기 위함). 기존 모드와 상호 배타 목록에 넣었습니다.

## 실제 앱 경로

- `main.rs` 59 `NativeApplication::new` → `eframe::App::logic`(application.rs 1297) → `background_tick` → `tick_terminal_agent_live_sessions` → `Hub::configure_history`/`Hub::configure_command_colors` → `SharedTerminal::configure_history`/`configure_command_colors` → `TerminalCore::set_history`.
- `eframe::App::ui` → `AppSurfaces::show_terminal_tab` → `Views::show` → `status(.., StatusKind)`(실패·종료 화면), 입력 루프 → `handle_input`/`buffer_starting_input` → `input_event`(Option-as-Meta).
- App 검사 3건은 위와 같은 `eframe::App::logic` + `eframe::App::ui` 경로(`project_rail_tests::step`)로 실제 PTY를 띄워 확인했습니다.

## 검증

명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/terminal-agent`를 붙였습니다.

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --all-targets --manifest-path native/taide-native-terminal/Cargo.toml` | 경고 없음 |
| `cargo test --quiet --manifest-path native/taide-native-terminal/Cargo.toml --test history` | 6 passed (11: 축소·확대·예산 거절·alt screen·OSC 133 잘림·SharedTerminal 같은값/거절/실패) |
| `cargo check --quiet --all-targets --features inspection --manifest-path native/taide-native-app/Cargo.toml` | 기존 경고만(vendor `wry-preview`, `editor-code-actions.rs` `is_applying`, `zen.rs` `mut`) |
| `cargo test ... native/taide-native-app ... --lib -- terminal_surface::tests --test-threads=2` | 37 passed (새 검사 6 `셀_그리기는_원본처럼_프로그래밍_합자를_만들지_않는다`, 19 `option_as_meta는_시작전_버퍼에서도_esc_접두와_조합문자_억제를_보존한다` 포함) |
| `cargo test ... --lib -- application::terminal_agent_tests --test-threads=2` | 3 passed (38 실패·재시작·종료 색, 49 scrollback·커서·글자 크기·셸, 27 탭 전환 gutter·숨은 세션 새 테마 색) |
| 같은 App 검사, `tick_terminal_agent_live_sessions` 호출 줄을 잠시 뺀 상태 | 2 failed(27 `returned gutters`, 49 `live scrollback`), 1 passed(38). 호출 줄 복원 |
| `cargo test ... --test terminal-host -- option_as_meta 실행중_scrollback 숨은_세션 --test-threads=2` | 3 passed (19 실제 Views 바이트, 11 Hub 실행 중 세션, 27 Hub 숨은 세션 색) |
| `cargo test ... --test terminal-host -- --test-threads=2` | 55 passed (32 근거 `terminal_menu…` 14건·`terminal_menu_actions…` 2건 포함) |
| `cargo fmt --manifest-path <crate>/Cargo.toml -- --check` (터미널·앱) | 첫 실행에서 새 코드 줄바꿈 차이 → `cargo fmt` 적용(바뀐 파일은 이번 작업 파일만) → 재검사 두 crate 모두 차이 없음 |
| 아래는 `cargo fmt` 적용 뒤 실행 | |
| `cargo test --quiet --manifest-path native/taide-native-terminal/Cargo.toml` | 13개 대상 39 passed (공유 PTY fixture에 모드를 더해 crate 전체 실행) |
| `cargo build --quiet ... --example native-lsp-mock` | 성공 |
| `cargo test ... --lib -- terminal_ agent_hooks ide_server ide_tools remote_agents status_ide application::ide_selection_tests 터미널 --test-threads=2` (seams 3절 terminal_agent 영역 필터 + 한국어 이름 "터미널") | 84개 중 83 passed, 1 failed: `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`(`left: [0, 2, 3, 5, 6, 2, 3, 6]`, `right: [0, 2, 3, 5, 6]`). 단독 재실행도 같은 값으로 실패. 이 검사는 메모리 내 합성 글꼴만 쓰는 `terminal_fonts::prepare_requested` 순수 함수 검사이고 이번 묶음은 그 경로의 파일을 바꾸지 않았습니다. 처음에는 "기존 실패"로 적었지만 기준 커밋에서 실행해 재현한 것은 아닙니다. 기준 커밋부터 있던 결정적 실패로 보는 근거는 수정 절에 있습니다 |
| `cargo test ... --test terminal-host -- --test-threads=2` | 55 passed |

## 남은 범위

- 배치 31 실기: 실제 Option 키·비 US 배열·dead key·IME 켜짐 상태(19, 특히 macOS가 dead key에서 `Key`와 `Preedit` 중 무엇을 어떤 순서로 보내는지와 원본 xterm이 Option+E에 `ESC e`를 보내는지), 합자 글꼴을 고른 터미널 화면(6), 실패·종료 화면 버튼 픽셀과 hover(38), 실제 셸 PTY.
- 27: xterm과의 전체 VT·reflow marker matrix와 성능은 다른 complete 행처럼 별도 게이트입니다. 탭 전환 뒤 블록 보존·기존 블록 색 유지는 단일 파싱 계약의 의도된 차이입니다(결정 6 A안).
- 11·49: settings-30 행의 scrollback 부분은 settings lane이 이 근거로 갱신합니다(결정 3 A안).
- 32: 메뉴 아이콘은 commands-ui-98(wave 3).
- 이번 묶음 범위 밖 실패: `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`가 이 worktree에서 단독으로도 실패합니다. 재현: 위 필터 또는 이름 필터 단독 실행. 기준 커밋 실행 재현은 하지 않았고(작업 규칙상 checkout 금지), 기준 커밋부터 있던 결정적 실패로 보는 코드 근거는 수정 절에 있습니다. 남은 위험: 편집기 글꼴 체인 꼬리에 UI 코드 글꼴이 섞이는 순서가 의도인지 확인되지 않음. 필요 시점: 메인 병합 게이트에서 기준 커밋 `87bf1f51`과 같은 필터로 비교 실행.
- 검사 공백: 6·19·38의 단위·App 검사는 구현과 함께 작성했습니다. 실패 선행은 11·27·49의 App 검사에서 새 tick 호출 줄을 뺀 상태로 2건 실패를 확인했고, 11의 터미널 crate 검사는 구현 전에는 API가 없어 컴파일되지 않는 형태로만 확인했습니다.

## 수정 (lane 리뷰 반영, 2026-11-03)

차단 항목은 없었습니다. 과대 판정 중 이 묶음 해당 3건(11·49 원격 history 문구, 19 complete, editor_fonts "기존 실패")과 관련 비차단 항목을 실제 코드로 확인했습니다. 코드 변경은 없고 판정·문구만 고쳤습니다.

### 11·49: 원격 터미널 history 문구 정정

- 리뷰 지적이 맞습니다. `Hub::configure_history`(`terminal_host.rs` 624-640)는 `self.sessions`의 모든 세션에 적용합니다. 원격 `pty_spawn`은 `remote-terminal.rs` 104-116에서 `ports.terminals.spawn_observed_with_initial_sink`로 같은 Hub에 세션을 넣고, 그 Hub는 `application.rs` 387-388의 `terminals.hub().clone()`입니다. 따라서 원격 spawn 세션의 core history도 실행 중에 갱신됩니다. spawn 값으로 남는 것은 PTY replay ring 바이트뿐입니다.
- 구현 2절의 문구와 대응표 11·49 행을 이 사실대로 고쳤습니다. 사양 2.2절 구현 설계 6번("원격 터미널은 spawn 값이라 그대로")은 core history에 대해서는 실제 코드와 다르며, 실제 코드를 따릅니다. 원격 spawn 세션이 설정값을 따르는 것은 로컬 세션과 같은 규칙이라 동작 변경은 하지 않았습니다. 별도 원격 경로 검사는 추가하지 않았습니다(Hub 검사 `실행중_scrollback…`과 같은 `sessions` 순회이고 spawn 경로만 다름).
- 판정: 11·49 complete 유지.

### 19: complete → partial

- 실제 macOS 이벤트 순서를 소스로 확인했습니다.
    - 포커스된 터미널은 매 frame `output.ime`를 채우고(`terminal_surface.rs` 5241), egui-winit 0.36.2 `lib.rs` 1138-1153이 이를 보고 `set_ime_allowed(true)`를 부릅니다.
    - winit 0.30.13 `platform_impl/macos/view.rs` 466-497: IME 허용이면 `interpretKeyEvents`를 먼저 부르고, 그 안에서 `setMarkedText`(302-334)가 불리면 `Ime::Preedit`을 보내고 상태를 Preedit으로 바꿉니다. 이 경우 `had_ime_input`이 참이라 `KeyboardInput`을 보내지 않습니다.
    - 일반 Option 문자(예: Option+B → `∫`)는 `setMarkedText` 없이 `insertText`만 불리고(393-416, marked text가 없으면 아무 이벤트도 보내지 않음) 상태가 그대로라 `KeyboardInput`이 갑니다. egui-winit는 이것을 `Event::Key`(1038) 뒤 `Event::Text`(1064) 순서로 넣습니다. 자동 검사가 합성한 `[Key, Text]`와 같은 순서이므로 이 경로는 근거가 보강됐습니다.
    - dead key(US 배열 Option+E/U/I/N/\`)는 `setMarkedText`로 `Ime::Preedit`만 오고 `Key`가 오지 않습니다. `input_event`의 meta 분기는 `Event::Key`에서만 시작하므로 ESC 접두가 나가지 않고 preedit이 쌓여 다음 키와 조합됩니다. 검사가 합성한 `[Key, Preedit]` 순서는 macOS에서 생기지 않습니다.
- 원본: `node_modules/@xterm/xterm/src/browser/CoreBrowserTerminal.ts` 1029-1043은 macOS에서 macOptionIsMeta이고 alt가 눌렸으면 조합 처리를 건너뛰고 `evaluateKeyboardEvent`로 meta 키를 보냅니다. 따라서 dead key에서 원본과 다릅니다(WKWebView의 실제 keydown 값은 실기 확인 필요).
- 근본 수정 후보: winit `OptionAsAlt`(macOS `set_option_as_alt`)를 터미널 포커스·설정에 맞춰 전환하면 `replace_event`(view.rs 458)가 Option 조합 문자를 없애 dead key도 `KeyboardInput`으로 옵니다. egui `ViewportBuilder`·`ViewportCommand`·egui-winit·vendored eframe 어디에도 이 설정 경로가 없어(`option_as_alt` 검색 0건) vendored eframe 변경이 필요하고, 창 전체 설정이라 편집기 Option 문자 입력과의 전환 규칙도 정해야 합니다. 이번 수정 범위를 넘어 구현하지 않았습니다.
- 판정: partial. 대응표 19 행을 고쳤습니다. 배치 31 항목에 이벤트 순서 확인을 명시했습니다.

### editor_fonts 실패의 분류 근거 보강

- 리뷰 지적대로 기준 커밋에서 실행해 재현하지는 않았습니다(작업 규칙상 checkout·되돌리기 금지). 문구를 "기존 실패"에서 "범위 밖 실패, 기준 커밋부터 있던 결정적 실패로 판단"으로 낮췄습니다.
- 코드 근거
    - 실패 값 `left: [0, 2, 3, 5, 6, 2, 3, 6]`은 `editor-fonts.rs` 529의 편집기 일반 체인입니다. 기대 `[0, 2, 3, 5, 6]` 뒤에 `[2, 3, 6]`이 더 붙었습니다.
    - `terminal_fonts::prepare_requested`(`terminal_fonts.rs` 244-255)는 `ui_fonts::prepare`를 먼저 부르고 그 다음 `editor_fonts::prepare`를 부릅니다. `ui_fonts::prepare`는 `FontFamily::Monospace`를 `taide-ui/mono/...` 체인 + 기본 fallback으로 바꿉니다(`ui-fonts.rs` 69, 98-161). 그 체인은 `CODE_FONT_STACK`(`ui-fonts.rs` 14, `SFMono-Regular`·`Menlo`…)과 monospace 질의로 찾은 합성 글꼴 2(SF Mono)·3(Menlo)·6(generic)입니다. `editor_fonts::prepare`는 바뀐 `Monospace`를 체인 꼬리로 붙이고(`editor-fonts.rs` 133-135·194), 검사 helper `chain`(290-297)은 egui 기본 `Monospace` 길이만 잘라내므로 `[2, 3, 6]`이 남습니다. 관측값과 정확히 맞습니다.
    - 이 경로의 파일(`terminal_fonts.rs`·`ui-fonts.rs`·`editor-fonts.rs`)은 이 worktree에서 바뀌지 않았습니다(`git diff --stat` 목록에 없음). `ui-fonts.rs`의 마지막 변경은 `b4a67346`(2026-10-13, `Monospace` 재구성 포함 184줄), 검사의 마지막 변경은 `361786d3`(2026-10-09)이고 둘 다 기준 커밋 `87bf1f51`의 조상입니다. `2026-10-16-native-json-schema-app.md` 64행도 변경을 stash한 상태에서 같은 검사가 실패했다고 기록합니다.
    - 단, `ui_fonts::system_face`(macOS, `NSFont`)는 실제 시스템 글꼴을 읽으므로 결과가 기계 상태에 따라 달라질 수 있습니다. 이 기계에서는 시스템 face가 체인에 들어가지 않았습니다.
- 수정하지 않은 이유: 편집기 글꼴 체인 꼬리에 UI 코드 글꼴이 들어가는 것이 의도인지(검사 기대값이 낡은 것인지, 제품 순서가 회귀인지) 이 lane 범위에서 판단할 근거가 없습니다. 메인 병합 게이트에서 기준 커밋과 같은 필터로 비교합니다.

### 테스트 부채: `rejected_history`가 값 기준

- 재현 조건: 창이 좁을 때 scrollback 값 N이 core 예산(`admit`)에 걸려 거절되면 `SharedTerminal.rejected_history = Some(N)`(`session.rs` `configure_history`)이 남습니다. 이후 창을 넓혀 같은 N을 받아들일 수 있게 돼도, 설정값이 다른 값으로 바뀌기 전까지 `configure_history(N)`은 바로 `Ok(false)`를 돌려 다시 시도하지 않습니다.
- 생략 이유: 매 frame 재시도·경고 반복을 막기 위한 기억이고, 예산 거절은 매우 큰 scrollback과 매우 큰 grid가 겹칠 때만 생깁니다. 사용자가 값을 다시 바꾸면 적용됩니다.
- 남은 위험: 그 사이 실행 중 세션이 이전 history 값으로 남습니다(오류는 없음).
- 필요해지는 시점: scrollback 상한을 늘리거나 resize 시 history 재적용 요구가 생길 때. 수정 방향은 resize가 grid 크기를 바꿀 때 `rejected_history`를 지우는 것입니다.

### 수정 검증

- 이 묶음 문서·대응표 문구만 바뀌었고 코드 변경은 없습니다. 검사는 다시 실행하지 않았습니다(앞 표의 결과 재사용). lane 공통 수정의 검증은 `2026-11-02-native-lane-terminal-agent-commands-run.md` 수정 절에 있습니다.
