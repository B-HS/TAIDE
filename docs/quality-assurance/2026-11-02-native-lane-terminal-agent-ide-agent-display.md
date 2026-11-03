# native lane terminal-agent 묶음 2/5: ide-agent-display

> 작성일: 2026-11-02 · lane: `terminal-agent` (worktree `TAIDE-lanes/terminal-agent`, 브랜치 `lane/terminal-agent`)
> 기준: `docs/research/2026-10-28-lane-terminal-agent-spec.md` 2.10~2.15절과 3절 묶음 2, 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 결정은 추천안), 구조 `docs/research/2026-10-28-native-lane-seams.md`

## 대상 ID와 판정

- [x] terminal-agent-51 agent 상태 UI 동기화 → complete
- [x] terminal-agent-52 rail 프로젝트 아이콘 agent 배지·접근성 라벨 → complete (shell-35 근거)
- [x] terminal-agent-53 터미널 탭 Sparkles·활동 색·tooltip → complete (shell-75 근거)
- [x] terminal-agent-67 IDE openFile → complete
- [x] terminal-agent-71 IDE close_tab·closeAllDiffTabs → complete (ClaudeDiff 표면 수명은 git-52)
- [x] terminal-agent-72 IDE 선택 push → complete (editor-83 근거)

## 원본 동작 근거

ID별 원본 근거는 사양 2.10~2.15절을 따릅니다. 이번 작업에서 사양과 실제 코드가 다르게 확인된 점만 적습니다.

- 51 Zen의 메인 창 탭 바: 사양 2.10 자동 검사는 "메인 창 Zen에서 다음 paint에 Sparkles 텍스처"를 적었지만, 원본 `src/widgets/editor-area/pane-node-view.tsx` 174는 `{!zen && <PaneTabBar … />}`로 Zen에서 메인 창 탭 바를 숨깁니다. 보조 창은 `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 139에서 `zen={false}`를 그대로 넘겨 항상 탭 바를 보입니다. 따라서 Zen 동안 관찰할 표면은 보조 창 탭 바이고, 메인 창은 Zen을 끄는 즉시 최신 상태를 그려야 합니다. native도 Zen에서 메인 탭 바를 그리지 않으며(검사가 Zen 동안 메인 창 Sparkles 없음을 함께 고정), 실제 코드를 따라 검사를 이렇게 구성했습니다.
- 51 repaint 확인 방법: 실행 중인 실제 App context는 다른 tick(터미널·애니메이션)도 repaint를 요청하므로 `context.has_requested_repaint()`만으로는 원인을 가릴 수 없습니다. 별도 probe context로 `PaintSink::publish`가 정확히 1개의 repaint 원인(호출 위치)을 남기는지 확인하고, 실제 App의 `services.events.publish(AppEvent::AgentStateChanged)` 뒤 실제 context의 `repaint_causes()`에 같은 원인이 들어오는지 확인했습니다.
- 67 진입 경로: 사양은 `LayoutActions::new(hub).with_view_states(..)`를 새로 조립하라고 적었지만, 실제 앱은 `application-ports.rs` `build`가 만든 `ide_server::Ports.actions`(`Arc<LayoutActions>`)를 `ide-server.rs` 368 `ide_tools::handle_incoming` → `ide_tools::dispatch`에 넘깁니다. 검사는 새로 조립하지 않고 실행 중인 App의 `application.application_ports.ide.actions` 그대로 `ide_tools::dispatch`를 호출합니다(웹소켓 전송 계층만 생략). 그래서 `start_ide`(IDE 서버 시작·lock 파일)는 필요하지 않았습니다.
- 67·71 대응표의 이전 비고("위치/선택 및 창 전면 요청", "dirty 확인")는 원본에 없는 동작입니다(사양 2.13·2.14). 원본 openFile은 창을 앞으로 가져오지 않고 프로젝트도 활성화하지 않으며, close_tab은 dirty 확인 없이 닫습니다.

## 구현

- 제품 코드 변경 없음(사양 3절 묶음 2 예상과 같음). 이 묶음의 변경은 `native/taide-native-app/src/application-terminal-agent-tests.rs`의 실제 App 검사 3건과 보조 함수뿐입니다. 이 파일과 `application.rs`의 테스트 모듈 등록은 묶음 1이 만들었습니다.
- 이 세션 시작 시 같은 파일에 이 묶음의 검사 3건이 미추적 상태로 이미 있었고(앞선 시도), 이어서 고쳐 완성했습니다.
    - 51 검사 수정: 새 `egui::Context`는 시작 시 repaint 1회가 예약된 상태(`vendor/egui-input/src/context.rs` 362 `outstanding: 1`)라 publish 전 `!has_requested_repaint()` 단정이 실패했습니다. probe를 `SETTLE_FRAMES` 안에서 예약이 빌 때까지 돌린 뒤 단정합니다. probe 실행 결과의 `textures_delta`를 비우지 않으면 epaint가 drop 시 panic하므로 `clear()`합니다.
    - 67 검사 강화: 파일마다 `fn <이름>_body() {}` 본문을 써서 `paints_body`(그려진 `Shape::Text`에 표식 포함)로 열린 파일의 본문이 실제로 그려지고 직전 파일 본문은 사라졌음을 확인합니다. 이를 위해 `step_until_collecting`의 조건이 frame 출력도 받도록 바꿨습니다.
- 검사 3건
    - 51 `실제_app_agent_상태_변경은_zen_메인창과_보조창_탭바에_다시_그려진다`: Zen 켬, 메인 창(파일·agent 터미널 탭)과 보조 창 slot 1(파일·agent 터미널 탭). `AgentStore::diff` working → 보조 창 탭 바 Sparkles가 `appSidebar.iconAgentWorking` 색, 메인 창 출력에는 Sparkles 없음. `PaintSink::publish(AgentStateChanged)` repaint 원인이 실제 App context에 기록됨. idle → 보조 창 색이 `appSidebar.iconAgentIdle`. Zen 끔 → 메인 탭 바 Sparkles가 idle 색, 보조 창도 idle.
    - 67 `실제_app_ide_openfile은_대상_프로젝트_포커스_pane에_본문을_그리고_창을_앞으로_가져오지_않는다`: alpha·beta 두 slot, alpha 포커스. openFile(second) → `Opened file: <path>`, 포커스 탭·view·본문. `makeFrontmost:false`(third) → `{success, filePath, languageId:"rust"}` JSON과 같은 탭 결과. 프로젝트 밖 경로 → `RPC_INVALID_PARAMS`, 탭 변화 없음. beta 파일 → beta layout에만 탭과 본문, 포커스 slot·활성 프로젝트·포커스 파일은 alpha 그대로. 전 과정의 viewport 명령에 `Focus`·`Visible(true)` 없음.
    - 71 `실제_app_ide_close_tab은_dirty_확인없이_닫고_터미널_세션과_대기_diff를_화면에서_정리한다`: 없는 제목 → `TAB_CLOSED`, 탭 그대로, close 이벤트 없음. dirty `보관.rs` → `TAB_CLOSED`, 확인 대화상자(`pending_tab_close`) 없이 탭·탭 바 글자 사라짐, 디스크 내용 그대로, `ide:close-tab-requested`(requestId 없음). `/bin/cat` 터미널 탭 제목 → Hub 세션 제거·탭 사라짐·이벤트. ClaudeDiff 탭 2개(`insert_pending_diff_owned` + `layout_open_tab`) → `closeAllDiffTabs` → `CLOSED_2_DIFF_TABS`, 두 대기 응답 `TabClosed`, 화면에 탭 없음, 이벤트 2건(각 requestId).
- 52·53·72: 코드·검사 추가 없음. 다른 영역 행의 complete 근거(shell-35, shell-75, editor-83)를 이 worktree에서 다시 실행해 통과를 확인하고 행을 갱신했습니다.

## 실제 앱 경로

- 51: `main.rs` → `NativeApplication::new`(`application-ports.rs` `start_agent_poll`, `PaintSink`가 `services.events`의 sink) → `eframe::App::ui` → 메인 `shell.rs` 탭 바와 보조 창 viewport 탭 바가 매 frame `AppSurfaces::project_agents` → `services.agents.agents_for(project)`를 읽어 `glyph-icons` `tab(kind, has_agent)` Sparkles를 활동 색으로 그림. 검사는 `project_rail_tests::step`(실제 `eframe::App::logic` + `eframe::App::ui`)과 보조 viewport immediate renderer로 확인.
- 67·71: `main.rs` → `NativeApplication::new` → `application-ports.rs` `build`의 `ide_server::Ports.actions` → (`ide-server.rs` `handle_incoming` →) `ide_tools::dispatch` → `LayoutActions` `open_file_tab`/`close_tab` → `taide_runtime::layout_actions` → 실제 App 다음 frame의 컨트롤러 snapshot·편집기 view·탭 바. 검사는 웹소켓 계층만 빼고 같은 `Arc<LayoutActions>`로 호출하며, 도구가 기다리는 view state 캡처에 frame이 응답하도록 호출 중에도 frame을 돌립니다.
- 52·53·72: 각 근거 QA의 실제 앱 경로(`2026-10-26-native-project-rail-display.md`, `2026-10-27-native-tab-icons-menu.md`, `2026-10-12-native-batch23-ide-selection.md`).

## 검증

명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/terminal-agent`를 붙였습니다. 앱 lib 검사는 `--manifest-path native/taide-native-app/Cargo.toml --features inspection --lib`입니다. LSP mock(`debug/examples/native-lsp-mock`)은 이미 있었습니다.

| 명령 | 결과 |
| --- | --- |
| `cargo test --quiet ... -- terminal_agent_tests::실제_app_ --test-threads=2` (세션 시작 상태) | 2 passed(67, 71), 1 failed(51: `assertion failed: !probe.has_requested_repaint()`), 경고 1(`run_ui` 반환값 미사용) |
| 51 수정 뒤 `-- terminal_agent_tests::실제_app_agent --test-threads=2` | 1 failed(`Dropped TexturesDelta with 1 unapplied deltas`) → `textures_delta.clear()` |
| 같은 명령 재실행 | 1 passed, 이 파일 경고 없음 |
| 67 본문 표식 추가 뒤 `-- terminal_agent_tests::실제_app_ide_openfile --test-threads=2` | 1 passed |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 첫 실행에서 이 묶음 검사 파일만 줄바꿈 차이 → rustfmt 출력대로 Edit 적용 → 재검사 차이 없음 |
| 아래는 형식 적용 뒤 실행 | |
| `cargo test --quiet ... -- tab_menu_tests project_rail_tests::실제_app_rail은_표시_glyph terminal_agent_tests --test-threads=2` | 8 passed: `terminal_agent_tests` 6(묶음 1의 3 + 이 묶음 51·67·71), 53 근거 `tab_menu_tests` 1, 52 근거 `실제_app_rail은_표시_glyph와_폴링된_에이전트_상태와_배지_설정을_반영한다` 1 |
| `cargo check --quiet --all-targets --features inspection --manifest-path native/taide-native-app/Cargo.toml` | 기존 경고만(vendor `wry-preview`, `editor-code-actions.rs` `is_applying`, `zen.rs` `mut`) |
| `cargo test --quiet ... -- ide_selection_tests editor_ide_selection ide_tools --test-threads=2` | 8 passed: 72 근거 `application::ide_selection_tests` 3, 67·71 단위 근거 `ide_tools::tests` 5 |

- UI crate(`taide-native-ui`)와 동결 `taide-remote-web`은 이 묶음에서 바꾸지 않았으므로 UI 하네스·Wasm check는 실행하지 않았습니다. 52·53의 UI workbench·glyph 단위 근거는 근거 QA의 기록이며 이 lane에서 UI crate가 바뀌지 않았습니다(`git status`에 UI 파일 없음).
- 실패 선행: 이 묶음은 제품 코드 변경이 없어 구현 전 실패 단계가 없습니다. 대신 각 검사가 변화 전 상태를 함께 단정합니다(agent 넣기 전 Sparkles 없음, Zen 동안 메인 창 Sparkles 없음, 새 파일을 연 뒤 직전 본문 없음, 없는 제목·밖 경로에서 변화 없음). 제품 코드를 잠시 빼는 변이 실행은 하지 않았습니다.

## 남은 범위

- 배치 31 실기: IDE 클라이언트(Claude Code)와의 실제 웹소켓 openFile·close_tab 왕복, 실제 agent 프로세스 감지로 바뀌는 탭·rail 표시 픽셀.
- 71: ClaudeDiff 표면(git-50)이 생긴 뒤 닫힘 시 표면 상태 해제는 git-52 범위입니다. shell lane이 같은 `close_tab` 경로에 붙이는 wait marker 해제(shell 사양 2.4)는 병합 뒤 이 검사와 함께 다시 확인합니다.
- 묶음 1에서 기록한 범위 밖 실패(기준 커밋 실행 재현은 하지 않음, 근거는 terminal-core 문서 수정 절) `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`는 이번 필터에 포함되지 않았고 이 묶음과 관계없습니다.
