# Rust-native lane 경계(seam) 설계와 영역 이동 지침

> 작성일: 2026-10-28 · 근거: `docs/memory/2026-10-28-native-lane-operations.md` 2.1절(단계 0)
> 범위: 앱 crate(`native/taide-native-app`)와 UI crate(`native/taide-native-ui`)의 통합 지점을 영역 파일로 나누는 **동작 불변 구조 이동**
> 영역: `shell`, `editor`, `explorer_search`, `git`, `terminal_agent`, `settings`, `commands`, `preview`

## 0. 현재 상태

| 항목 | 상태 |
| --- | --- |
| 골격 | 완료: `src/domains/mod.rs`(application의 자식 모듈), `lib.rs`의 `pub use application::domains;`, `host.rs`의 `HostWorker`, UI `src/intents/mod.rs`·`intents/shell.rs`(빈 `Intent`, 새 shell intent 자리)·`intents/commands.rs` |
| shell | 완료: `src/domains/shell.rs` (상태 17필드, HostCommand 22변형, HostReply 9변형, worker 처리, 응답 반영, ShellIntent 처리, tick·dialog·보조 창 메서드) |
| commands | 완료: `src/domains/commands.rs` (상태 6필드, HostCommand 5변형, HostReply 3변형, 팔레트·키바인딩 overlay, `taide_native_ui::intents::commands::Intent`) |
| preview | 완료(작업 2/4): `src/domains/preview.rs` (상태 13필드 `preview_state`, HostCommand 5변형, HostReply 9변형, worker 처리, `apply_preview_reply`, `intents::preview::Intent::ReopenWith`, `surfaces::preview::PreviewSurfaces`, `poll_preview_web`·`tick_preview_media`·`show_preview_web_views`, `AppSurfaces::show_preview_tab`, `reconcile_previews` 등 5메서드, `connect_web`·`media_appearance`·`WEB_*` 상수). 남은 접점은 2.7절 |
| editor | 일부(작업 2/4 + 리뷰 반영 작업): `src/domains/editor.rs` (HostCommand 33변형, HostReply 20변형, worker 처리와 `open_problem`·`open_marker`, `apply_editor_reply`, `intents::editor::Intent` 9변형과 `apply_editor_intent`, `surfaces::editor::EditorSurfaces`(`tab_content`·`tab_icon`·`problems_open`·`problems_panel`), `AppSurfaces::show_document`·`tokens_supplier`, **5.9절 1단계 메서드 이동 완료**(`impl NativeApplication` 두 번째 블록, 2.6절 editor 행 전부와 `reconcile_editor_features`), `submit()` 가로채기 `intercept_editor_command`·`prepare_editor_submission`). **상태(`editor_state`)와 tick·show 이동(5.9절 2~4단계)은 미완료** |
| explorer_search | 완료(explorer_search·git 작업): `src/domains/explorer_search.rs` (상태 14필드 `explorer_search_state`, HostCommand 8변형, HostReply 6변형, worker 처리, `apply_explorer_search_reply`, `intents::explorer_search::Intent` 5변형과 `apply_explorer_search_intent`, `surfaces::explorer_search::ExplorerSearchSurfaces`(`type Explorer`·`begin/explorer/finish/discard_explorer`·`is_auxiliary_explorer_open`), `ExplorerRegion`·`OutlineReveal` 타입, `AppSurfaces::show_breadcrumbs`, tick·show 메서드 9개와 영역 메서드 7개(2.6절)). 5.7절 hook `show_search_editor_tab` 유지. 남은 접점은 2.7절 |
| git | 완료(explorer_search·git 작업): `src/domains/git.rs` (상태 2필드 `git_state`, HostCommand 1변형, HostReply 1변형, worker 처리, `apply_git_reply`, 빈 `intents::git::Intent`와 `apply_git_intent`(seam), `surfaces::git::GitSurfaces`(`branch`·`title_branch_icon`), `tick_git_title_branches`). 5.7절 hook `show_diff_tab`·`show_claude_diff_tab` 유지 |
| terminal_agent | 완료(terminal_agent·settings 작업): `src/domains/terminal_agent.rs` (상태 7필드 `terminal_agent_state`, HostCommand 11변형, HostReply 7변형, worker 처리와 `TERMINAL_CLIPBOARD_BYTES`, `apply_terminal_agent_reply`, `intents::terminal_agent::Intent::NewTerminal`과 `apply_terminal_agent_intent`, `surfaces::terminal_agent::TerminalAgentSurfaces`(`project_agents`·`is_agent_status_badge_enabled`), tick·finish 5개, `AppSurfaces::show_terminal_tab`). 남은 접점은 2.7절 |
| settings | 완료(terminal_agent·settings 작업): `src/domains/settings.rs` (상태 9필드 `settings_state`, HostCommand 15변형, HostReply 8변형, worker 처리, `poll_settings_revisions`, `apply_settings_reply`(Presentation 갱신 arm 포함), `intents::settings::Intent` 4변형과 `apply_settings_intent`, `presentation_inputs`·`sync_theme_preview`, `tick_settings_reads`, `load_settings_requests`, `AppSurfaces::show_settings_tab`·`show_app_file_tab`). settings는 ShellSurfaces 메서드가 없어 UI surfaces 파일을 만들지 않았다. 남은 접점은 2.7절 |
| 나머지 영역 | 없음. editor의 상태·tick 이동(5.9절 2~4단계)만 남았다 |
| 리뷰 반영 작업 | 남은 충돌 지점 정리(2.8절): poll_lsp의 explorer 응답 분리, 재연결 블록 영역 훅, `submit()` 가로채기 분리, presentation 대입 단일화, `FrameOutput` 3개(preview·settings·explorer_search), editor 메서드 블록 이동 |
| 줄 수 | `application.rs` 11,582 → 10,573 → 8,440 → 7,525 → 7,063 → 3,010, `host.rs` 2,477 → 1,977 → 1,225 → 965 → 444 (`domains/editor.rs` 2,441 → 2,319 → 6,085, `domains/preview.rs` 795 → 917, `domains/explorer_search.rs` 1,310 → 1,476, `domains/git.rs` 140 → 144, `domains/terminal_agent.rs` 597 → 601, `domains/settings.rs` 841 → 882, `domains/shell.rs` 1,179 → 1,185, `domains/commands.rs` 643 → 647). UI `commands.rs` 249 → 211 → 189 → 180, UI `shell.rs` 2,820 → 2,792 → 2,795 |

검증 결과(리뷰 반영 작업 끝, 2.8절): 앱 `cargo check --all-targets`(기존 경고 3개와 vendor `wry-preview` 경고만), UI `cargo check --all-targets --features native-host,inspection`(기존 경고 1개, UI 코드는 이번에 바꾸지 않음), remote-web Wasm check(remote-web 기존 dead code 경고만), UI lib·workbench 테스트(167·45 통과), 앱 lib 영역 필터 649개 중 647개 통과(`application::{code_actions,code_lenses,inlay_hints,selection_ranges,context_menu,json,view_state,file_loading,missing_draft,markdown,ai_completion,ai_edit,emmet,sticky_definition,startup,ide_selection,tab_menu,tab_drag,file_drop,window_state,project_rail,close_all,explorer_views,shell_dialogs,auxiliary_window,error_boundary,exit,app_menu}_tests editor_ lsp file_loading preview shell_keymap command_dispatch save tab_close_batch symbol_location explorer workspace_symbol breadcrumb problems settings_ theme_ snippet_host_tests app_file_views presentation terminal_ toast`, `--test-threads=4`). 실패 2개는 3절 끝의 기존 흔들림 `context_menu_tests::실제_추가_경계_보조창_메뉴는_클립보드와_터미널의_소유_pane을_보존한다`(`--exact` 단독 재실행 통과)와 작업 2/4에 기록한 손대지 않은 `editor_fonts` 시스템 글꼴 단언(같은 값 `[0, 2, 3, 5, 6, 2, 3, 6]`)이다. 영향 통합 테스트 `explorer`·`lsp`·`preview`·`save`·`settings-controls`·`tabs` 31개 통과, 두 crate `cargo fmt --check` 통과, `git status`에 remote-web·Cargo 파일 없음.

검증 결과(terminal_agent·settings 작업 끝): 앱 `cargo check --all-targets`(기존 경고 3개와 vendor `wry-preview` 경고만), UI `cargo check --all-targets --features native-host,inspection`(기존 경고 1개만, `tests/workbench.rs` 1816 `mut`로 줄 이동), remote-web Wasm check(remote-web 기존 dead code 경고만), UI lib·workbench 테스트(167·45 통과), 앱 lib 영역 필터 164개 통과(`application::ide_selection_tests application::startup_tests application::context_menu_tests application::explorer_views_tests application::markdown_tests application::ai_completion_tests terminal_ agent_hooks ide_server ide_tools remote_agents status_ide settings_ theme_ snippet_host_tests app_file_views remote_preferences presentation status_editor toast shell_keymap command_dispatch application_ports`, `--test-threads=4`), 경로를 고친 통합 테스트 `terminal-host`·`settings-controls`(52·2개 통과), 두 crate `cargo fmt --check` 통과, `git status`에 remote-web·Cargo 파일 없음. 재확인(같은 작업 재실행, 코드 변경 없음): 원본(HEAD) host.rs·application.rs와 영역 파일을 대조해 변형 정의·worker arm·응답·intent·tick 위치·`submit()` 가로채기·생성자 순서가 경로만 다름을 확인했고, 위 검사를 다시 실행해 같은 결과를 얻었다. 단 앱 lib 영역 필터는 163개 통과·1개 실패였고, 실패는 3절 끝의 기존 흔들림 `context_menu_tests::실제_추가_경계_보조창_메뉴는_클립보드와_터미널의_소유_pane을_보존한다`(`body_view(..).unwrap()`)이며 단독 재실행에서 통과했다.

검증 결과(explorer_search·git 작업 끝): 앱 `cargo check --all-targets`(기존 경고 3개와 vendor `wry-preview` 경고만), UI `cargo check --all-targets --features native-host,inspection`(기존 경고 1개만, `tests/workbench.rs` 1812 `mut`로 줄 이동), remote-web Wasm check(remote-web 기존 dead code 경고만), UI lib·workbench 테스트(167·45 통과), 앱 lib 영역 필터 117개 통과(`application::explorer_views_tests explorer symbol_ workspace_symbol breadcrumb problems title_branch remote_git application::auxiliary_window_tests application::tab_menu_tests application::shell_dialogs_tests application::error_boundary_tests application::startup_tests application::json_tests shell_keymap`, `--test-threads=4`, 마지막 추출 뒤 재실행도 117개 통과), 경로를 고친 통합 테스트 `projects`·`host`·`explorer`(4·9·21개 통과, `explorer-delete` 미실행), 두 crate `cargo fmt --check` 통과, `git diff --stat`에 remote-web·Cargo 파일 없음.

검증 결과(작업 2/4 끝): 앱 `cargo check --all-targets`(기존 경고 3개만), UI `cargo check --all-targets --features native-host,inspection`(기존 경고 1개만), remote-web Wasm check, UI lib·workbench 테스트(167·45 통과), 앱 lib editor·preview 영역 필터 509개 중 508개 통과(실패 1개는 이동과 무관한 `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`: 손대지 않은 `editor-fonts.rs`가 시스템 글꼴 fallback 체인을 단언하며 단독 재실행에서도 같은 값 `[0, 2, 3, 5, 6, 2, 3, 6]` ≠ `[0, 2, 3, 5, 6]`으로 실패), shell·commands 보조 필터 87개 통과, 경로를 고친 통합 테스트 대상(5.9절 끝) 실행, 두 crate `cargo fmt --check` 통과, remote-web·Cargo 파일 무변경.

검증 결과(작업 1/4 끝): 앱 `cargo check --all-targets`, UI `cargo check --all-targets --features native-host,inspection`, remote-web Wasm check, UI lib·workbench 테스트(167·45 통과), 앱 lib shell·commands 영역 필터 190개 중 188~189개 통과(실패는 회차마다 다른 3절 끝의 기존 흔들림 검사 1~2개, 각각 단독 재실행 통과), 변경 통합 테스트 4개 대상(host·projects·settings-controls·explorer-system) 통과, 두 crate `cargo fmt --check` 통과, remote-web·Cargo 파일 무변경.

## 1. 구조 결정

### 1.1 (a) 영역 파일 위치와 구성

- `domains`는 `application.rs` 안에 `#[path = "domains/mod.rs"] pub mod domains;`로 선언한 **application의 자식 모듈**이다. 그래서 영역 파일은 `NativeApplication`의 비공개 필드·메서드와 `application.rs`의 비공개 항목(`WINDOW_LABEL`, `APPLICATION_NAME`, `editor_error`, `paint_rail_icon`, `WindowPass`, `AppSurfaces` 등)에 가시성 변경 없이 접근한다.
- `lib.rs` 끝의 `pub use application::domains;`로 경로를 줄인다: 크레이트 안 `crate::domains::<영역>`, 통합 테스트 `taide_native_app::domains::<영역>`. `application.rs`와 그 자식(테스트 포함)은 `domains::<영역>`로 쓴다.
- 영역 파일 `src/domains/<영역>.rs`의 항목 순서:
    1. `pub enum Command` — 이 영역의 host 요청 (`HostCommand::<영역>(Command)`)
    2. `pub enum Reply` — 이 영역의 host 응답 (`HostReply::<영역>(Reply)`)
    3. `pub(crate) struct State` — 이 영역 소유 상태 (`NativeApplication.<영역>_state`)
    4. `pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply>` — worker 처리
    5. 영역 전용 비동기 보조 함수 (예: shell의 `open_dropped_paths`)
    6. `impl NativeApplication { ... }` — `apply_<영역>_reply`, `apply_<영역>_intent`, `tick_<영역>_*`, `show_<영역>_*`, 영역 전용 메서드
    7. (영역 trait를 쓰는 경우) `impl <영역>Surfaces for AppSurfaces<'_> { ... }`
- 가시성: 영역 파일의 메서드·함수는 `pub(crate)`, 영역 파일 안에서만 쓰는 것은 비공개 그대로. 영역 메서드 시그니처에 들어가는 `application.rs` 타입은 `pub(crate)`로 올린다(`WindowPass`가 선례).
- 필드 이름 규칙: 영역 상태 필드는 `<영역>_state` (`self.shell`은 이미 `NativeShell`이므로 접미사로 구분).

### 1.2 (b) HostCommand·HostReply와 worker

- 루트 enum(`host.rs`)의 맨 위 블록이 영역 wrapper다. 영역당 한 줄:

```rust
pub enum HostCommand {
    Shell(crate::application::domains::shell::Command),
    Commands(crate::application::domains::commands::Command),
    ...미이동 변형...
}
```

- worker 분배: `host.rs`의 `dispatch(worker: &HostWorker<'_>, command)` 맨 위 match 블록에 영역당 한 줄 `HostCommand::<영역>(command) => crate::application::domains::<영역>::dispatch(worker, command).await,`.
- `HostWorker<'a>`는 worker 루프의 공용 의존을 묶는다: `services`, `clipboard`, `read_clipboard`, `sender`, `repaint`, `integrations`. 영역 `dispatch`는 필요한 것만 `let services = worker.services;`처럼 꺼낸다.
- 영역 응답은 `HostReply::<영역>(Reply::X { .. })`로 감싼다. 공용 실패 응답(`HostReply::Failed`, `HostReply::SettingsFailed`)과 다른 영역 응답(예: commands의 `RunTask` → `HostReply::EditorTerminalTarget`)은 원래 변형을 그대로 돌려준다(동작 동일).
- 여러 영역이 쓰는 `host.rs` 보조 함수는 `host.rs`에 두고 `pub(crate)`로 연다(현재: `open_file_tab`, `write_clipboard`, `update_control_settings`, `stopped`, `flush_layouts`, `HostIntegrations::{flush_view_states, projects}`, `HostIntegrations.reconcile`, `HostIntegrations.terminals`(editor의 `WriteEditorText`·`CloseTab`이 사용, 작업 2/4)). 한 영역만 쓰는 보조 함수는 영역 파일로 옮긴다(editor: `open_problem`·`open_marker`, preview: `connect_web`·`media_appearance`·`WEB_*` 상수).
- 응답 반영: `NativeApplication::poll()`의 `match reply` 맨 위 블록에 영역당 한 줄 `HostReply::<영역>(reply) => self.apply_<영역>_reply(context, reply),`.
- 영역이 다른 변형을 or-패턴 한 arm으로 처리하던 곳은 arm 본문을 `application.rs`의 공용 메서드로 꺼내고 각 영역 arm이 호출한다(작업 2/4): `apply_file_link_reply`(root `TerminalFileOpened` + editor `ProblemOpened`), `apply_current_file_link_reply`(root `WorkspaceSymbolOpened` + editor `DocumentationFileOpened`·`BreadcrumbOpened`). terminal_agent·explorer_search 작업자는 자기 arm을 `apply_<영역>_reply`로 옮기며 같은 공용 메서드를 호출한다.
- 이름 충돌: 옛 `HostReply::Preview { request, result }`는 `domains::preview::Reply::Preview`가 되었고 `HostReply::Preview(..)`는 wrapper다.
- terminal_agent·settings 작업 뒤 wrapper 블록은 `Shell`·`Commands`·`Editor`·`ExplorerSearch`·`Git`·`TerminalAgent`·`Settings`·`Preview` 순이다(HostCommand·HostReply·`dispatch`·`poll()` 라우팅 동일). 루트 `HostCommand`는 wrapper만 남았고, 루트 `HostReply`는 wrapper와 공용 `SettingsFailed`·`Failed`뿐이다. `host.rs` `dispatch`는 라우팅만 하므로 `HostWorker` 구조 분해와 지역값을 지웠다. 공용 보조 함수: `authorize_terminal_link`는 `terminal_tabs.rs`도 호출해 `host.rs`에 남겼고, `TERMINAL_CLIPBOARD_BYTES`는 terminal_agent `ReadTerminalClipboard`만 써서 영역 파일로 옮겼다. `HostBridge::submit`의 `SnippetEdit` admit 가로채기는 `HostCommand::Settings(Command::SnippetEdit(..))` 패턴으로 감쌌다. settings `ReadSettingsResource` arm은 블록 안 `use taide_native_ui::settings_resources::{Kind, Reply}`가 영역 `Reply`를 가리므로 wrapper 쪽만 `crate::domains::settings::Reply::SettingsResource`로 적었다. terminal_agent `TerminalMenu`는 `HostReply::Editor(editor::Reply::Closed)`, `NewTerminal`·settings `OpenAppFile`·`OpenSettings`는 공용 `HostReply::Failed`, settings 제어 변경은 공용 `HostReply::SettingsFailed`를 그대로 돌려준다.
- explorer_search·git 작업 뒤 wrapper 블록은 `Shell`·`Commands`·`Editor`·`ExplorerSearch`·`Git`·`Preview` 순이었다(HostCommand·HostReply·`dispatch`·`poll()` 라우팅 동일). explorer_search `RestoreWatchers`의 실패 응답은 공용 `HostReply::Failed`를 그대로 돌려준다. `host.rs` `dispatch`의 `HostWorker` 구조 분해는 `sender`·`repaint`를 쓰는 arm이 모두 영역으로 옮겨져 `..`로 바꿨다(남은 루트 arm이 다시 필요하면 필드를 추가한다).

### 1.3 (c) ShellIntent와 ShellSurfaces

- `ShellIntent` 루트(`taide-native-ui/src/commands.rs`)는 **shell 영역 소유**다. 기존 shell 변형(2.4절, 생성처 약 150곳)은 루트에 그대로 둔다. 루트 끝 wrapper 블록에는 영역당 한 줄을 둔다: `Shell(crate::intents::shell::Intent)`, `Commands(crate::intents::commands::Intent)`, 이후 `Editor`, `ExplorerSearch`, `Git`, `TerminalAgent`, `Settings`, `Preview`. **새 shell intent는 루트가 아니라 `intents/shell.rs`의 `Intent`에 추가**한다(shell lane 사양 0절과 같음). 지금 `intents::shell::Intent`는 빈 enum이고 `apply_shell_intent`가 `ShellIntent::Shell(intent) => match intent {}`로 받는다.
- wrapper 이름은 영역 이름의 PascalCase다: `HostCommand::TerminalAgent(domains::terminal_agent::Command)`, `ShellIntent::ExplorerSearch(intents::explorer_search::Intent)`처럼 HostCommand·HostReply·ShellIntent에서 같게 쓴다.
- 영역 하위 enum은 `taide-native-ui/src/intents/<영역>.rs`의 `pub enum Intent` (`#[derive(Debug, Clone)]`, 변형의 `#[cfg(feature = "native-host")]`는 그대로 유지). `intents/mod.rs`에 `pub mod <영역>;`.
- 처리 위치:
    - shell 변형: `domains/shell.rs`의 `apply_shell_intent(intent, frame, snapshot)`. 이 match의 마지막 arm은 **다른 영역 변형 목록**(현재 `ShellIntent::Commands(_)`와 미이동 변형)이며 아무것도 하지 않는다. 모든 변형을 나열해 shell 변형 추가 시 처리 누락을 컴파일러가 잡는다.
    - 영역 wrapper: `application.rs` 화면 pass의 intent 루프에서 catch-all 직전 영역 라우팅 블록 `ShellIntent::<영역>(intent) => self.apply_<영역>_intent(intent, ...),` (루프 지역값 `context`·`frame`·`snapshot`·`keymap_documents`·`command_context`를 필요한 만큼 넘길 수 있도록 라우팅은 application.rs에 둔다).
- remote-web 호환: 동결 `taide-remote-web`은 `ShellIntent`·`ShellSurfaces`를 쓰지 않고 `commands::ShellMutation`, `snapshot::ShellSnapshot`, 설정·편집기 위젯만 쓴다(`rg -n "ShellIntent|ShellSurfaces" native/taide-remote-web/src` 결과 없음). 따라서 `ShellMutation`의 경로와 정의, remote-web이 import하는 UI 모듈 경로는 바꾸지 않는다. UI 공개 경로를 옮겨야 하면 옛 경로에 `pub use`를 남긴다.
- `ShellSurfaces`(UI `shell.rs`)의 루트 trait도 shell 영역 소유다. 다른 영역 메서드는 `taide-native-ui/src/surfaces/<영역>.rs`의 `pub trait <영역>Surfaces`로 옮기고 루트를 `pub trait ShellSurfaces: crate::surfaces::<영역>::<영역>Surfaces + ... {}` 형태의 supertrait로 묶는다(5.6절). 구현자는 두 곳뿐이다: 앱 `application.rs`의 `AppSurfaces`, UI `tests/workbench.rs`의 `Surfaces`.
- 현재 상태(작업 2/4): `ShellIntent` 루트 끝 wrapper 블록은 `Shell`·`Commands`·`Editor`·`Preview`. intent 루프 라우팅 블록은 `Commands`·`Editor`(`apply_editor_intent(intent, &context, frame, &snapshot, &command_context, &keymap_documents)`)·`Preview`. UI `surfaces/mod.rs`(첫 생성)와 `surfaces/editor.rs`·`surfaces/preview.rs`가 있고 `ShellSurfaces: EditorSurfaces + PreviewSurfaces`. 이후 영역은 supertrait 목록 끝에 `+ crate::surfaces::<영역>::<영역>Surfaces`를 추가한다.
- 현재 상태(explorer_search·git 작업 뒤): wrapper 블록은 `Shell`·`Commands`·`Editor`·`ExplorerSearch`·`Git`·`Preview`. git은 옮길 `ShellIntent` 변형이 없지만 git lane이 루트 블록을 고치지 않도록 shell과 같은 빈 `intents::git::Intent`와 `ShellIntent::Git` wrapper, 라우팅 `ShellIntent::Git(intent) => self.apply_git_intent(intent)`(`match intent {}`)를 미리 두었다. 인자가 더 필요하면 git lane이 그 라우팅 한 줄만 고친다. 라우팅 블록은 `Commands`·`Editor`·`ExplorerSearch`(`apply_explorer_search_intent(intent, &context, &snapshot)`)·`Git`·`Preview`. supertrait는 `EditorSurfaces + ExplorerSearchSurfaces + GitSurfaces + PreviewSurfaces`(알파벳 순). `S::Explorer` 명시 사용처가 없어 5.6절 3번의 경로 변경은 필요 없었다. 앱 `AppSurfaces`의 `type Explorer = ExplorerRegion`이 다른 모듈의 trait 구현 연관 타입이 되어 `ExplorerRegion`을 `pub(crate)`로 올렸다(필드는 비공개).
- 현재 상태(terminal_agent·settings 작업 뒤): 루트 `ShellIntent`는 shell 변형 24개와 wrapper 블록 `Shell`·`Commands`·`Editor`·`ExplorerSearch`·`Git`·`TerminalAgent`·`Settings`·`Preview`뿐이다. `intents/terminal_agent.rs`(`NewTerminal`), `intents/settings.rs`(`OpenSettings`·`OpenSettingsFile`·`ChangeEditorFontSize`·`#[cfg(feature = "native-host")] ToggleEditorMinimap`). intent 루프 라우팅 블록은 `Commands`·`Editor`·`ExplorerSearch`·`Git`·`TerminalAgent`(`apply_terminal_agent_intent(intent)`)·`Settings`(`apply_settings_intent(intent, &snapshot)`)·`Preview`. `apply_shell_intent`의 다른 영역 목록은 wrapper 7개뿐이다. supertrait는 `EditorSurfaces + ExplorerSearchSurfaces + GitSurfaces + PreviewSurfaces + TerminalAgentSurfaces`(알파벳 순). settings는 ShellSurfaces 메서드가 없어 `surfaces/settings.rs`를 만들지 않았다. settings lane이 surface 메서드가 필요하면 그때 파일과 supertrait 한 줄을 추가한다.

### 1.4 (d) 공용 파일의 영역 블록 규칙

lane은 자기 영역 파일만 고치고, 아래 공용 파일은 **자기 영역 블록 안에서 추가만** 한다. 단계 0이 끝나면 영역 블록 자체는 더 늘지 않는다.

| 공용 파일 | 영역 블록 | lane이 하는 일 |
| --- | --- | --- |
| `application.rs` `NativeApplication` struct | `<영역>_state: domains::<영역>::State` 한 줄 | 새 상태는 `domains/<영역>.rs`의 `State`에 추가. 루트 struct에는 추가하지 않는다 |
| `application.rs` 생성자 `new_with_secret_store` | `<영역>_state: domains::<영역>::State { ... }` 리터럴 | 새 필드 초기화식을 자기 리터럴 끝에 추가 |
| `application.rs` `poll()` | `HostReply::<영역>(reply) => ...` 라우팅 블록 | 손대지 않음. `Reply` 변형과 `apply_<영역>_reply` arm만 추가 |
| `application.rs` intent 루프 | `ShellIntent::<영역>(intent) => ...` 라우팅 블록 | 손대지 않음. `Intent` 변형과 `apply_<영역>_intent` arm만 추가 |
| `application.rs` 프레임 흐름(`background_tick`, `show_window`, `show_overlays`, `finish_frame`) | `self.tick_<영역>_*()` / `self.show_<영역>_*()` 호출 줄 | 기존 tick 함수 안에 추가. 새 호출 위치가 꼭 필요하면 자기 기존 호출 줄 바로 뒤에 한 줄 |
| `application.rs` `AppSurfaces`·`show_window` 렌더 중 요청 | `<영역>_output: &mut domains::<영역>::FrameOutput` 한 줄(preview·settings·explorer_search) | 새 요청은 자기 `FrameOutput` 필드와 기존 소비 함수 안에 추가. 새 소비 시점이 꼭 필요할 때만 `show_window`에 한 줄 |
| `application.rs` 재연결 블록·`submit()`·poll_lsp | `self.reconnect_<영역>()`, `intercept_*`/`prepare_editor_submission`, `apply_explorer_search_lsp_reply` 라우팅 | 자기 영역 함수 안에 추가(2.8절) |
| `host.rs` `HostCommand`/`HostReply`/`dispatch` | 맨 위 wrapper·라우팅 블록 | 손대지 않음. 자기 `Command`/`Reply`/`dispatch`에 추가 |
| UI `commands.rs` `ShellIntent` | 루트 끝 wrapper 블록 | 손대지 않음. shell lane은 `intents/shell.rs`, 다른 lane은 `intents/<영역>.rs`에 추가 |
| UI `shell.rs` `ShellSurfaces` | supertrait 목록 | shell lane만 루트 메서드 추가. 다른 lane은 `surfaces/<영역>.rs`에 추가 |
| `lib.rs` 모듈 목록, `domains/mod.rs`, `intents/mod.rs` | 알파벳 순(rustfmt `reorder_modules`) | 새 모듈 한 줄을 정렬 위치에 추가. 충돌해도 정렬 병합이면 된다 |
| locale 카탈로그 `crates/taide-locale/resources/locales/{en,ko,ja}.json` | 키 접두사 묶음(평면 JSON, 키 정렬) | 자기 접두사 묶음 안에서 정렬 위치에 추가. 세 언어 파일에 같은 키를 함께 추가 |

locale 접두사와 영역: `project.*`·`projectGroup.*`·`welcome.*`·`zen.*`·`window.*`·`systemUsage.*` → shell, `palette.*`·`keybindings.*`·`task*` → commands, `editor.*` → editor, `explorer.*`·`search.*`·`problems.*`·`outline.*` → explorer_search, `git.*` → git, `terminal.*`·`agent.*`·`ide.*` → terminal_agent, `settings.*`·`theme.*`·`snippet*` → settings, `preview.*` → preview. 이 목록에 없는 접두사는 처음 쓰는 lane이 이 문서에 한 줄 추가한다.

### 1.5 (e) 영역 간 공유 상태 접근

- 영역 코드는 `impl NativeApplication` 블록 안에 있으므로 `&mut self`로 모든 상태에 접근한다. 빌림 분리는 기존 코드처럼 필드 경로 단위로 한다(`let locale = &self.locale; let icons = &mut self.explorer_icons;`). `self.<영역>_state.x`와 `self.y`는 서로 다른 필드이므로 원래 코드의 분리 빌림이 그대로 성립한다.
- 소유 규칙:
    - 영역 소유 상태는 `self.<영역>_state` 안에 둔다.
    - 공용 상태는 루트에 남긴다: `runtime`, `services`, `application_ports`, `controller`, `bridge`, `shell`(`NativeShell`), `locale`, `toasts`, `toast_theme`, `tooltips`, `tooltip_appearance`, `motion_preference`, `status`, `closing`, `close_request_active`, `is_exit_ready`, 그리고 여러 영역이 읽는 문서 저장소 `store`·`files`·`untitled`(editor가 주 소유자이지만 shell·commands·explorer·preview가 읽고 쓴다).
    - 다른 영역 상태는 **읽기만** 한다(예: shell의 `overlay_viewport`가 `self.commands_state.palette.is_open()`을 읽음). 바꿔야 하면 소유 영역의 `pub(crate)` 메서드를 호출한다(예: shell intent가 `self.request_tab_close(tab)`(editor), commands가 `self.request_editor_formatting(..)`(editor)을 호출).
    - 단계 0의 이동에서는 기존 교차 접근을 고치지 않는다. 동작 불변이 우선이고, 교차 쓰기 정리는 각 lane의 별도 작업이다.
- 메서드 경계를 넘어 여러 필드를 동시에 `&mut`로 넘겨야 하면(예: `AppSurfaces`처럼) 영역 파일에 인자 묶음 struct를 정의해 필드 참조를 담는다. 현재 shell·commands 이동에서는 필요하지 않았다.

## 2. 분류표

표의 미이동 행은 **제안**이다. 각 작업자는 옮기기 전에 `rg -n "<이름>\b"`로 실제 사용처를 확인하고, 다른 영역에서 더 많이 쓰면 그 영역으로 옮기거나 공용으로 남긴 뒤 이 표를 갱신한다.

### 2.1 `NativeApplication` 필드

| 영역 | 필드 |
| --- | --- |
| shell (완료, `shell_state`) | `welcome`, `app_menu`, `open_path_dialog`, `project_display_dialog`, `project_group_dialog`, `project_group_delete`, `system_usage`, `system_usage_appearance`, `system_usage_icon`, `zen_hint`, `zen_hint_colors`, `file_drop_colors`, `auxiliary_windows`, `window_state`, `auxiliary_pass_root`, `overlay_viewport`, `zen_fullscreen_state` |
| commands (완료, `commands_state`) | `keybindings`, `palette`, `palette_files`, `palette_file_changes`, `palette_commands`, `task_runner` |
| 공용(루트 유지) | `runtime`, `services`, `application_ports`, `controller`, `bridge`, `shell`, `locale`, `store`, `files`, `untitled`, `toasts`, `toast_theme`, `tooltips`, `tooltip_appearance`, `motion_preference`, `status`, `closing`, `close_request_active`, `is_exit_ready`, `mirror_sweeps` |
| editor (미이동, 5.9절) | `save_picker`, `lsp`, `pending_format_saves`, `workspace_activity`, `workspace_suspended`, `deleted_documents`, `resumed_save_epochs`, `lsp_diagnostics`, `editor_*`(symbols·folding·json·json_validation·locations·documentation·completion·highlights·inlay_hints·code_lenses·selection_ranges·view_state·markdown·clipboard·menu_changes·terminal_writes·formatting·rename·code_actions·find·problems·sticky_scroll·syntax·keymap_targets 및 `*_colors`), `editor`, `view_state_flush`, `peek_models`, `lsp_status_appearance`, `status_editor_appearance`, `status_chord_appearance`, `indentation_request`(commands와 공유), `find_history`, `find_appearance`, `banner_appearance`, `restore_notices`, `loading_file_tabs`, `pending_disk_choice`, `pending_file_observations`, `observing_files`, `untitled_loading`, `untitled_failed`, `saving_untitled`, `missing_drafts`, `saving_missing_draft`, `loading`, `failed`, `refused`, `pending_dirty`, `persistence`, `last_edited_views`, `pending_tab_close`, `focused`, `reveals`, `document_edits`, `fold_commands`, `problem_opens`, `problems`, `problems_appearance` |
| explorer_search (완료, `explorer_search_state`, 루트 위치는 옛 `workspace_symbols` 자리) | `workspace_symbols`, `explorer_appearance`, `explorer_icons`, `navigation_appearance`, `navigation_icons`, `symbol_sidebars`, `breadcrumbs`, `trees`, `explorers`, `explorer_clipboards`, `pending_entry_delete`, `deleting_entry`, `tree_loading`, `tree_changes` (State 필드 순서는 옛 선언 순서). 다른 영역의 읽기·쓰기: `explorer_icons`는 shell(`show_shell_dialogs`의 `paint_rail_icon`, `AppSurfaces::rail_icon`·`tooltip`), editor(`AppSurfaces::tab_icon`·`show_document`), `show_window`의 `status_error_banner::show`가 쓰고, `navigation_icons`는 `show_window`의 파일 drop overlay가, `pending_entry_delete`·`deleting_entry`는 commands·preview·shell·`toast_interaction_enabled`·`show_window`의 활성 조건이, `tree_changes.take_previews()`는 preview `reconcile_previews`가, `workspace_symbols`는 commands 팔레트가 읽는다. 1.5절대로 경로만 바꿨다 |
| git (완료, `git_state`, 루트 위치는 옛 `title_branch_changes` 자리) | `title_branch_changes`, `title_branches` |
| terminal_agent (완료, `terminal_agent_state`, 루트 위치는 옛 `terminals` 자리) | `terminals`, `terminal_views`, `terminal_appearance`, `terminal_fonts`, `editor_ide_selection`, `status_ide_appearance`, `status_ide_icons` (State 필드 순서는 옛 선언 순서). `editor_ide_selection`은 IDE(에이전트) 연동 선택 알림이고 요청 변형 `EditorIdeSelection`·테스트 `application::ide_selection_tests`·locale `ide.*`가 terminal_agent여서 여기 두었다(5.9절 3번의 `begin_frame`은 terminal_agent 문장). 다른 영역의 읽기·쓰기: `terminal_views`는 commands `show_commands_overlays`의 `clear_keymap_chord`, `frame_ui`의 `clear_keymap_chord`, `show_window`의 `set_command_context`·키맵 캡처·`chord_status`·`route_window_keys`·`has_keyboard_focus`·`submission_failed`, `raw_input_hook(_with_replay)`, `background_tick`의 입력 flush·`chord_status`, settings `apply_settings_reply`·`sync_theme_preview`의 `set_palette`, editor `submit()`의 `cancel_file_links`가 쓰고, `terminals`는 editor `flush_dirty`와 재연결 블록이 읽는다. 1.5절대로 경로만 바꿨다 |
| settings (완료, `settings_state`, 루트 위치는 옛 `settings_views` 자리) | `settings_views`, `app_file_views`, `settings_appearance`, `resolved_theme`, `preview_theme`, `presentation_changes`, `presentation_refresh`, `presentation_failures`, `presentation_system_language`. 다른 영역의 읽기·쓰기: `app_file_views`는 editor(`run_editor_context_menu`의 `changed`, `release_project_documents`의 `clear`, `request_tab_save`의 쓰기, `release_tab`의 `release_closed`), `settings_views.snippet_catalog()`는 editor `dispatch_completion`, `preview_theme`·`resolved_theme`은 `background_tick`의 editor syntax tick과 `show_window`의 `status_error_banner`, `presentation_refresh.retry()`는 `status_error_banner` 결과가 쓴다. 1.5절대로 경로만 바꿨다 |
| preview (완료, `preview_state`, 루트 위치는 옛 `web_bridge` 자리) | `web_bridge`, `helper_executable`, `web_previews`, `web_views`, `open_with`, `previews`, `hwp_previews`, `pdf_previews`, `pdf_appearance`, `presentation_previews`, `presentation_appearance`, `spreadsheet_previews`, `spreadsheet_appearance` |

### 2.2 `HostCommand` 변형

| 영역 | 변형 |
| --- | --- |
| shell (완료) | `OpenWelcome`, `CopyText`, `RevealPath`, `OpenInBrowser`, `OpenPath`, `OpenProject`, `OpenProjectByPath`, `OpenDroppedPaths`, `OpenRecentProject`, `ClearRecentProjects`, `CloseProject`, `OpenProjectInSlot`, `SetProjectGroupMembers`, `ReorderProjects`, `ReorderProjectGroups`, `ReturnAuxiliaryWindowTabs`, `MoveTabToWindow`, `CreateProjectGroup`, `EditProjectGroup`, `DeleteProjectGroup`, `OpenProjectGroup`, `SetProjectDisplay` |
| commands (완료) | `RunTask`, `DetectTasks`, `SetKeymapOverrides`(설정 화면도 사용), `ListProjectFiles`, `OpenPaletteFile` |
| editor (완료) | `EditorContextMenu`, `EditorClipboard`, `RunEditorText`, `WriteEditorText`, `CodeActions`, `RenameEditor`, `FormatEditorInput`, `OpenDocumentationFile`, `OpenSymbolLocation`, `ReadPeekModels`, `ReadCompletionClipboard`, `AutoSave`, `MirrorDraft`, `SaveTracked`, `ObserveFile`, `ChooseDisk`, `CleanupFileMirror`, `NewUntitled`, `OpenUntitled`, `SaveUntitled`, `CleanupUntitledMirror`, `PruneMirrors`, `SaveMissingDraft`, `CloseTab`, `SaveMirroredTab`, `OpenFileTab`, `OpenFileToSide`, `OpenProblem`, `OpenMarker`, `OpenBreadcrumbFile`, `OpenDocument`, `Save`, `SetDirty` |
| explorer_search (완료) | `RestoreWatchers`, `RefreshTree`, `TreeRows`, `TreeToggle`, `TreeCollapse`, `RevealExplorerPath`, `RevealBreadcrumbTree`, `OpenWorkspaceSymbol`(commands 팔레트도 생성) |
| git (완료) | `GitBranch` |
| terminal_agent (완료) | `EditorIdeSelection`, `ResolveTerminalFileLinks`, `OpenTerminalFile`, `OpenTerminalUrl`, `TerminalMenu`, `ReadTerminalClipboard`, `RestartTerminal`, `ResizeTerminal`, `AttachTerminal`, `NewTerminal`, `CopyTerminalSelection` (생성처: `terminal_surface.rs`, `shell_keymap.rs` 테스트, `toast-tests.rs`, 통합 `terminal-host`) |
| settings (완료) | `ReadAppFile`, `WriteAppFile`, `OpenAppFile`(`app_file_views::settings_command`도 생성), `RefreshPresentation`, `ReadSettingsCatalog`, `ReadSettingsResource`, `ThemeEdit`, `SnippetEdit`, `ReadSnippetCatalog`, `OpenSettings`, `OpenSettingsFolder`, `SetEditorFontSize`(`status_editor`도 생성), `SetTerminalFontSize`(`status_editor`도 생성), `ToggleEditorMinimap`(editor `run_editor_context_menu`도 생성), `UpdateSettings` |
| preview (완료) | `ReadHwpPreview`, `ReadPresentationPreview`, `ReadSpreadsheetPreview`, `ReadPreview`, `ReadPdfPreview` |

### 2.3 `HostReply` 변형

| 영역 | 변형 |
| --- | --- |
| shell (완료) | `TabMovedToWindow`, `CopiedText`, `SystemFinished`, `ProjectOpenedByPath`, `DroppedPathsOpened`, `ProjectDisplaySet`, `ProjectClosed`, `ProjectGroupSaved`, `ProjectGroupOpened` |
| commands (완료) | `TasksDetected`, `ProjectFiles`, `PaletteFileOpened` |
| 공용(루트 유지) | `Failed`, `SettingsFailed`(settings가 주 사용자이지만 commands의 `SetKeymapOverrides`도 반환) |
| editor (완료. commands `RunTask`·terminal_agent `TerminalMenu`도 각각 `Reply::EditorTerminalTarget`·`Reply::Closed`를 그대로 돌려준다) | `EditorTerminalTarget`, `EditorClipboard`, `DocumentationFileOpened`, `SymbolLocationOpened`, `PeekModels`, `BreadcrumbOpened`, `ProblemOpened`, `MarkerOpened`, `CompletionClipboard`, `DraftMirrored`, `ObservedFile`, `DiskChosen`, `UntitledOpened`, `UntitledSaved`, `MissingDraft`, `MissingDraftSaved`, `Closed`, `MirrorSaved`, `Opened`, `Saved` |
| explorer_search (완료. `WorkspaceSymbolOpened`는 공용 `apply_current_file_link_reply`를 호출) | `WorkspaceSymbolOpened`, `BreadcrumbTree`, `TreeSynced`, `Tree`, `TreeToggled`, `ExplorerPathRevealed` |
| git (완료) | `GitBranch` |
| terminal_agent (완료. `TerminalFileOpened`는 공용 `apply_file_link_reply`를 호출) | `TerminalFileLinks`, `TerminalFileOpened`, `TerminalUrlOpened`, `TerminalClipboard`, `TerminalResized`, `TerminalAttached`, `TerminalSelectionCopied` |
| settings (완료. `Presentation`은 모든 영역의 appearance를 대입하는 arm을 통째로 옮겼다) | `AppFileWritten`, `AppFile`, `ThemeEdit`, `SnippetEdit`, `SnippetCatalog`, `SettingsCatalog`, `SettingsResource`, `Presentation` |
| preview (완료) | `HwpSourceReady`, `HwpPreview`, `SpreadsheetSourceReady`, `SpreadsheetPreview`, `PresentationSourceReady`, `PresentationPreview`, `PdfSourceReady`, `PdfPreview`, `Preview` |

### 2.4 `ShellIntent` 변형

| 영역 | 변형 |
| --- | --- |
| shell (루트 유지, `apply_shell_intent`; 새 변형은 `intents::shell::Intent`) | `Mutate`, `OpenFolder`, `OpenProjectByPath`, `SelectRecentProject`, `OpenFile`, `OpenWelcome`, `CopyText`, `RevealPath`, `MoveTabToWindow`, `OpenPath`, `CloseProject`, `OpenProjectInSlot`, `SetProjectGroupMembers`, `ReorderProjects`, `ReorderProjectGroups`, `NewProjectGroup`, `OpenProjectGroup`, `EditProjectGroup`, `DeleteProjectGroup`, `EditProjectDisplay`, `SetProjectDisplay`, `ShowOpenProjectNotice`, `RequestCloseTab`, `RequestCloseTabs` |
| commands (완료, `intents::commands::Intent`) | `OpenPalette`, `OpenKeybindings`, `OpenTaskRunner` |
| editor (완료, `intents::editor::Intent`) | `NewUntitled`, `ChooseIndentation`, `FormatEditor`, `RenameEditor`, `CodeActions`, `ToggleEditorStickyScroll`, `RequestSaveTab`, `EditDocument`, `FoldDocument` |
| explorer_search (완료, `intents::explorer_search::Intent`. 생성처: UI `shell.rs` 탭 메뉴, 앱 `shell_keymap.rs`) | `RevealInExplorer`, `RevealInAuxiliaryExplorer`, `ToggleAuxiliaryExplorer`, `ShowExplorer`, `ShowAuxiliaryExplorer` |
| git (빈 `intents::git::Intent`, seam만) | 없음 |
| terminal_agent (완료, `intents::terminal_agent::Intent`. 생성처: UI `shell.rs` 탭 추가 메뉴·빈 편집기·탭 메뉴, 앱 `shell_keymap.rs`) | `NewTerminal` |
| settings (완료, `intents::settings::Intent`. 생성처: UI `shell.rs` rail 설정 버튼, 앱 `shell_keymap.rs`·`command_dispatch`(테스트)·`presentation.rs`(테스트)) | `OpenSettings`, `OpenSettingsFile`, `ChangeEditorFontSize`, `ToggleEditorMinimap` |
| preview (완료, `intents::preview::Intent`) | `ReopenWith` |

### 2.5 `ShellSurfaces` 메서드

| 영역 | 메서드 |
| --- | --- |
| shell (루트 유지) | `text`, `text_with_args`, `welcome_content`, `rail_recent_projects`, `is_reduced_motion`, `rail_icon`, `tooltip`, `type StatusOutput`, `prepare_status_bar`, `status_bar`, `apply_status_bar`, `discard_status_bar` |
| editor (완료, `surfaces::editor::EditorSurfaces`) | `tab_content`, `tab_icon`, `problems_open`, `problems_panel` |
| explorer_search (완료, `surfaces::explorer_search::ExplorerSearchSurfaces`) | `type Explorer`, `begin_explorer`, `explorer`, `finish_explorer`, `discard_explorer`, `is_auxiliary_explorer_open` |
| git (완료, `surfaces::git::GitSurfaces`) | `branch`, `title_branch_icon` |
| terminal_agent (완료, `surfaces::terminal_agent::TerminalAgentSurfaces`) | `project_agents`, `is_agent_status_badge_enabled` |
| preview (완료, `surfaces::preview::PreviewSurfaces`) | `can_reopen_with` |
| commands·settings | 없음 |

### 2.6 `NativeApplication` 메서드

| 영역 | 메서드 |
| --- | --- |
| shell (완료, `domains/shell.rs`) | `apply_shell_reply`, `apply_shell_intent`, `tick_shell_menus`, `tick_shell_system_usage`, `show_shell_usage_detail`, `show_shell_dialogs`, `project_group_dialogs`, `close_project`, `window_scopes`, `active_window_scope`, `is_any_window_focused`, `focused_viewport`, `run_app_menu_action`, `overlay_viewport`, `show_auxiliary_windows`, `submit_auxiliary_window_returns`, `close_auxiliary_window` |
| commands (완료, `domains/commands.rs`) | `apply_commands_reply`, `apply_commands_intent`, `tick_commands_files`, `show_commands_overlays`, `show_palette` |
| 공용 라우터·기반(application.rs 유지) | `new`, `with_app_menu`, `new_with_secret_store`, `poll`(라우팅), `report`, `report_close_failure`, `report_once`, `report_save_failure`, `notify_open_project_first`, `run_toast_actions`, `submit`(가로채기 포함), `toast_interaction_enabled`, `background_tick`(tick 순서), `frame_ui`, `finish_window_frames`, `window_command_context`, `show_window`(intent 루프 라우팅), `show_overlays`, `finish_frame`, `eframe::App` 구현 |
| shell (남은 후보) | `close`, `exit_application` |
| editor (완료분, `domains/editor.rs`) | `apply_editor_reply`, `apply_editor_intent`, `AppSurfaces::show_document`(+`tokens_supplier`), `EditorSurfaces` 구현 |
| 공용(application.rs, 작업 2/4 추가) | `apply_file_link_reply`, `apply_current_file_link_reply`, `AppSurfaces::show_unavailable_tab` |
| editor (리뷰 반영 작업에서 `domains/editor.rs`로 이동 완료. 다른 모듈·테스트가 부르는 것만 `pub(crate)`) | `run_editor_context_menu`, `apply_editor_menu_clipboard`, `flush_dirty`, `has_layout_tab`, `capture_view_states`, `fulfill_view_states`, `request_tab_close`, `is_dirty_close_tab`, `request_close_tabs`, `advance_close_batch`, `cancel_failed_batch_prepare`, `request_tab_save`, `release_tab`, `tab_close_dialog`, `save_untitled`, `save_missing_draft`, `release_project_documents`, `drafts`, `choose_disk`, `queue_project_observations`, `flush_observations`, `draft_project`, `auto_save_delay`, `flush_persistence`, `save_view`, `focused_editor_source`, `request_editor_code_actions(_from_source)`, `request_editor_rename(_from_source)`, `formatted_document`, `save_formatting_options`, `prepare_untitled_cleanup`, `queue_native_save`, `poll_native_formatting`, `request_editor_formatting`, `request_editor_input_formatting`, `reconcile_lsp`, `clear_builtin_json`, `reconcile_builtin_json`, `dispatch_completion`, `consume_code_action_navigation`, `consume_editor_command_navigation`, `request_window`, `consume_location_model`, `is_current_code_lens_request`, `is_current_selection_range_request`, `poll_lsp`, `finish_formatted_save`, `workspace_busy`, `suspend_workspace_documents`, `reconcile_editor_features` |
| explorer_search (완료, `domains/explorer_search.rs`) | `show_explorer_files`·`show_auxiliary_explorer_files`(비공개), `report_create_failure`·`report_rename_failure`(poll_lsp도 호출), `create_entry`·`rename_entry`(`run_toast_actions`도 호출), `observe_workspace_symbols`(팔레트도 호출), `apply_explorer_search_reply`, `apply_explorer_search_intent`, 새로 꺼낸 `tick_explorer_search_clipboards`(frame_ui의 clipboard window 정리), `tick_explorer_search_breadcrumbs`(show_window 첫 breadcrumbs reconcile), `tick_explorer_search_trees`(background_tick의 `tree_changes` 루프), `load_explorer_search_trees`(show_window의 `load_trees` 제출 루프), `submit_explorer_search_entries`(show_window의 create·rename·paste 루프), `close_explorer_search_discarded_delete`(show_window의 버려진 패널 삭제 확인 닫기), `queue_explorer_search_delete`(show_window의 `delete_entries` → 확인 dialog), `show_explorer_search_delete_dialog`(show_overlays의 삭제 확인), `AppSurfaces::show_breadcrumbs`(editor `show_document`가 호출, `pub(crate)`), `ExplorerSearchSurfaces` 구현과 `ExplorerRegion`·`OutlineReveal` 타입 |
| git (완료, `domains/git.rs`) | `apply_git_reply`, `apply_git_intent`(빈 match), 새로 꺼낸 `tick_git_title_branches`(background_tick의 title branch 무효화·요청 블록), `GitSurfaces` 구현 |
| settings (완료, `domains/settings.rs`) | `presentation_inputs`(비공개), `sync_theme_preview`(background_tick·finish_window_frames가 호출), `apply_settings_reply`, `apply_settings_intent`, 새로 꺼낸 `poll_settings_revisions`(poll 앞쪽의 theme·settings revision 관찰과 app file reconcile), `tick_settings_reads`(background_tick else 분기 끝의 snippet catalog 읽기와 presentation refresh 제출), `load_settings_requests`(show_window의 settings catalog·resource·app file 제출 루프), `AppSurfaces::show_settings_tab`·`show_app_file_tab`(editor `tab_content`의 `Settings`·`AppFile` 분기) |
| terminal_agent (완료, `domains/terminal_agent.rs`) | `apply_terminal_agent_reply`, `apply_terminal_agent_intent`, 새로 꺼낸 `tick_terminal_agent_fonts`(background_tick의 terminal font 적재·크기), `tick_terminal_agent_cursors`(background_tick의 `configure_cursors`), `tick_terminal_agent_views`(frame_ui의 live tab·session `retain`), `tick_terminal_agent_ide_selection`(finish_window_frames의 IDE selection 제출·deadline), `finish_terminal_agent_frame`(show_window 두 곳의 같은 `terminal_views.finish_frame` 블록), `AppSurfaces::show_terminal_tab`(editor `tab_content`의 `Terminal` 분기), `TerminalAgentSurfaces` 구현 |
| preview (완료, `domains/preview.rs`) | `reconcile_previews`, `cancel_web`, `pdf_preview_paths`, `web_preview_paths`(비공개), `preview_tab_paths`, 새로 꺼낸 `poll_preview_web`(poll 맨 앞 web bridge 루프), `tick_preview_media`(frame_ui의 media appearance 갱신), `show_preview_web_views`(finish_frame의 web view sync), `apply_preview_reply`, `apply_preview_intent`, `AppSurfaces::show_preview_tab`(tab_content의 preview 분기) |

`AppSurfaces` 보조 메서드: `show_breadcrumbs`(explorer_search로 옮김. breadcrumbs·trees·navigation 상태를 쓰는 explorer_search 렌더러이고 editor `show_document`가 `self.show_breadcrumbs(..)`로 호출한다), `show_document`(editor, 완료), `show_unavailable_tab`(공용, application.rs).

### 2.7 shell·commands에 의도적으로 남긴 접점

아래는 다른 영역 코드와 섞인 문장이거나 프레임 흐름 순서 자체여서 이번 이동에서 application.rs에 남겼다. shell·commands lane이 필요할 때 자기 tick·show 함수로 옮긴다(호출 위치·순서 유지).

- shell: `PaintSink.app_menu` 기록, `with_app_menu`, 생성자의 `app_menu_changes`·`system_usage`·`auxiliary_windows`·`zen_fullscreen_state` 계산, `background_tick`의 `window_state.observe`와 보조 창 `settle_reserved`, `frame_ui`의 `zen_fullscreen_state.reconcile`과 `app_menu.finish_child_passes`, `raw_input_hook`의 `app_menu.inject`, `on_exit`의 `window_state.save`, `show_window`의 파일 drop 제출·zen hint·zen escape·status bar 출력, `AppSurfaces`의 shell 메서드.
- commands: `show_window`의 `palette_commands` 키맵 합류와 `keybindings.observe_closed`, editor intent `ChooseIndentation`의 `palette.open_indentation`/`dismiss`, 여러 곳의 "키바인딩·팔레트가 열려 있지 않음" 조건, `indentation_request`(editor와 공유).
- preview(작업 2/4): `show_window`의 `load_previews`·`load_pdf_previews`·`load_hwp_previews`·`load_presentation_previews`·`load_spreadsheet_previews`·`load_web_previews` 제출 루프(지역 Vec 6개를 넘겨야 해 함수로 꺼내면 인자 8개가 되어 남김, 경로만 `HostCommand::Preview(..)`·`self.preview_state.*`로 변경), `show_window`의 `AppSurfaces` 생성 리터럴 preview 필드, 파일 drop overlay의 `web_views.drags()`, `background_tick`의 재연결 블록(`reset_pending`·`connect_web`, shell·explorer 문장과 섞임), `close`·`exit_application`의 `web_bridge.take()`·`invalidate_all`·`web_views.clear()`, 생성자의 `connect_web` 호출, `release_tab`의 preview 무효화(editor 메서드 안), presentation 갱신 arm·`sync_theme_preview`의 `pdf/presentation/spreadsheet_appearance` 대입(settings 문장과 섞임).
- editor(작업 2/4): `submit()`의 가로채기(`EditorContextMenu`·`CodeActions`·`RenameEditor`·`FormatEditorInput`·`SetDirty`·`SaveUntitled`·`Save`/`AutoSave`→`SaveTracked`)는 패턴만 `HostCommand::Editor(domains::editor::Command::..)`로 감싸고 검사 순서를 유지했다. `show_window`의 `HostCommand::Editor(Command::SetDirty { .. })` 즉시 제출 분류도 같다.
- explorer_search(explorer_search·git 작업, 경로만 `self.explorer_search_state.*`로 변경): `poll()` Presentation arm과 `sync_theme_preview`의 `explorer_appearance`·`navigation_appearance` 대입(settings 문장과 섞임), `poll_lsp`의 `crate::lsp::Reply` arm(`WorkspaceSymbols`, `ExplorerPasted`, `ExplorerCreated`, `ExplorerRenamed`, 이름 변경 완료의 `explorers.moved`, `ExplorerDeleteFinished`의 `deleting_entry`·`RefreshTree` 제출. HostReply가 아닌 LSP 응답 enum이고 poll_lsp는 editor 5.9절 이동 대상), `background_tick`의 `explorer_clipboards`·`symbol_sidebars`·`breadcrumbs` reconcile(가운데 editor `problems.reconcile`이 있고 지역 `auxiliary_explorers`를 함께 씀)과 재연결 블록의 `tree_loading.clear()`, `run_toast_actions`의 `RetryCreate`·`RetryRename`(공용 라우터), `show_window`의 `is_enabled`·zen escape 조건(`pending_entry_delete`·`deleting_entry`), `AppSurfaces` 생성 리터럴의 explorer 필드, 파일 drop overlay·`status_error_banner`의 아이콘 대여, `flush_dirty` 실패 블록의 create·rename 실패 표시(terminal `submission_failed`와 바깥 `return false`가 섞임), 생성자 리터럴, `toast_interaction_enabled` 조건, shell `overlay_viewport`·`show_shell_dialogs`, commands `show_commands_overlays`·`show_palette`, preview `reconcile_previews`·`show_preview_web_views`의 읽기.
- git(explorer_search·git 작업): 재연결 블록의 `git_state.title_branches.cancel_fetches()`(editor·shell·commands·preview 문장과 섞임), `AppSurfaces` 생성 리터럴의 `title_branches`. `PaintSink.title_branches`·`PaintSink.trees`는 이벤트 기록용 공유 `Arc`라 그대로 둔다.
- 생성자 순서(explorer_search·git 작업): `explorer_search_state` 리터럴은 옛 `explorer_appearance` 자리(`problems: ..?` 뒤)에 두어 `?` 순서 `problems` → `explorer_icons` → `navigation_icons` → `shell_state.window_state`(경고 로그)가 그대로다. 앞쪽으로 당겨진 `workspace_symbols`는 `Default`, 뒤에서 당겨진 `trees`~`tree_changes`는 기본값·이동·`egui.viewport_id()`(읽기)뿐이다. `git_state`는 옛 `title_branch_changes` 자리에 둔다. drop 순서: 루트 필드는 옛 `workspace_symbols` 자리라 뒤에 선언됐던 explorer 필드가 원래보다 먼저 drop된다. 직접 `Drop`은 `workspace_symbols::Entry`(자기 watch 취소)뿐이고 나머지는 데이터·텍스처·`Arc`라 순서 의존이 없다.

### 2.8 리뷰 반영 작업(남은 충돌 지점 정리)

리뷰가 남은 충돌 지점으로 지적한 곳을 영역 블록으로 나눴다. 동작은 바꾸지 않았고, 문장은 원문 그대로 옮기며 경로(`domains::editor::Command::X` → 영역 파일 안 `Command::X`, settings 파일 안 `crate::domains::commands::Command`)와 가시성만 바꿨다. 2.7절의 해당 항목은 아래로 대체된다.

- poll_lsp: `crate::lsp::Reply`의 explorer 응답 5개(`WorkspaceSymbols`, `ExplorerPasted`, `ExplorerCreated`, `ExplorerRenamed`, `ExplorerDeleteFinished`)를 `domains/explorer_search.rs`의 `apply_explorer_search_lsp_reply`로 옮기고, poll_lsp match 맨 위에 `reply @ (..5개..) => self.apply_explorer_search_lsp_reply(reply)` 라우팅 한 arm을 두었다. 원래 arm들은 `while let` 본문의 유일한 문장인 match의 arm이고 `continue`가 없어 그대로 옮겨진다. `lsp::Reply`는 lsp.rs·통합 테스트가 직접 만드는 공용 enum이라 wrapper로 바꾸지 않았고, 영역 함수 끝의 `_ => {}`는 라우팅 패턴 밖 변형이 들어올 수 없는 자리다. explorer lane이 lsp 응답을 추가하면 영역 함수 arm과 라우팅 패턴 한 줄을 고친다. `Renamed` arm 중간의 `explorers.moved` 루프는 `move_explorer_search_entries(from, to)`로 꺼냈다(같은 자리에서 호출).
- 재연결 블록(`background_tick`의 `Ok(Err(error))` 분기): 영역 문장 하나씩을 `reconnect_explorer_search`(`tree_loading.clear`), `reconnect_git`(`title_branches.cancel_fetches`), `reconnect_shell`(`settle_reserved`), `reconnect_commands`(`palette_files.cancel_fetches`), `reconnect_preview(context)`(`reset_pending` 5개와 `connect_web`)로 꺼내 같은 순서로 호출한다. editor의 `loading.clear()`와 bridge 재연결 뒤 editor 문장은 editor 상태 이동 전이라 그대로 남겼다.
- `submit()`: 가로채기 사슬을 같은 순서의 라우팅 4줄로 바꿨다. `intercept_editor_command`(EditorContextMenu·CodeActions·RenameEditor·FormatEditorInput) → `intercept_settings_command`(SnippetEdit List) → `domains::settings::is_settings_command(&command)`(옛 `is_settings` 목록, settings 파일 소유) → `prepare_editor_submission`(SetDirty·SaveUntitled format-on-save·Save/AutoSave → SaveTracked, `tracked_save` 반환). 각 함수는 `std::ops::ControlFlow<bool, _>`를 돌려주고 `Break(v)`는 옛 `return v`다. 제출 실패 처리(snippet catalog·snippet·theme·terminal file link·tracked save·settings toast)는 bridge 제출 결과와 묶여 있어 application.rs에 남겼다.
- presentation 대입: `Reply::Presentation` arm과 `sync_theme_preview`가 같은 순서로 반복하던 `apply_visuals` + 대입 39줄을 settings 파일의 비공개 `apply_presentation_appearances(context, appearances)` 하나로 합쳤다(두 곳의 앞뒤 문장 `resolved_theme`·`preview_theme`·`presentation_failures`·`request_repaint` 순서는 그대로). 테마 색 추가 시 고칠 곳이 `presentation_refresh::Appearances`(정의·`new`)와 이 함수 두 곳으로 줄었다. 영역별 훅으로 나누려면 대입 순서가 영역끼리 섞여 있어 순서를 바꿔야 하고 `Appearances`를 영역 하위 struct로 쪼개야 해서(`?` 평가 순서 변경) 하지 않았다.
- 렌더 중 요청(`AppSurfaces`): 영역 전용 지역 Vec를 영역 파일의 `FrameOutput`(`#[derive(Default)]`)으로 묶었다. `domains::preview::FrameOutput`(`load_previews`·`load_pdf_previews`·`load_hwp_previews`·`load_presentation_previews`·`load_spreadsheet_previews`·`load_web_previews`, 필드 비공개, 소비는 `load_preview_requests(context, output)`로 옛 6개 루프를 그대로 옮김), `domains::settings::FrameOutput`(`load_settings`·`load_settings_resources`·`load_app_files`·`settings_errors`·`theme_errors`·`snippet_notices`), `domains::explorer_search::FrameOutput`(`rename_entries`·`paste_entries`·`create_entries`·`delete_entries`·`discarded_explorers`·`load_trees`). `AppSurfaces` 필드·생성 리터럴·`show_window` 지역 변수는 영역당 한 줄(`preview_output`·`settings_output`·`explorer_output`)이고, 렌더 코드는 `self.<영역>_output.<필드>.push(..)`로 쓴다. settings·explorer의 소비 지점은 프레임 흐름 순서에 묶여 있어 `show_window`에서 `settings_output.x`/`explorer_output.x`로 꺼내 기존 영역 함수에 넘긴다. `flush_dirty` 실패 분기의 create·rename 실패 표시는 `fail_explorer_search_entries`로 꺼냈다. 지역 Vec가 struct 필드로 묶여 소비되지 않은 요청의 drop 순서가 바뀔 수 있지만, 필드 타입(경로·`TabId`·`ProjectId`·explorer/clipboard/delete 요청·settings 요청·`AppError`·snippet notice)에는 `Drop` 구현이 없어(`rg -n "impl Drop for"` 확인) 관찰되지 않는다. lane은 새 요청을 자기 `FrameOutput` 필드로 추가하고, 기존 소비 함수 안에서 처리하면 공용 파일을 고치지 않는다. 새 소비 시점이 필요할 때만 `show_window`에 한 줄을 추가한다.
- editor 메서드의 다른 영역 문장: `release_tab`의 preview 무효화 블록 → `release_preview_tab(&closed, path)`(preview), AppFile 분기 → `release_settings_app_file(&tab)`(settings), `request_tab_save`의 app file 저장 블록 → `save_settings_app_file(&tab, document) -> bool`(settings, 옛 `return`은 `return true`), `flush_dirty`의 terminal 세션 확인 → `has_terminal_agent_session(&id)`(terminal_agent).
- editor 메서드 이동(5.9절 1단계): `application.rs`의 editor 메서드를 `domains/editor.rs`의 두 번째 `impl NativeApplication` 블록으로 원문 그대로 옮겼다. 첫 블록의 `run_editor_context_menu`~`flush_persistence`(shell 후보 `close`는 남김), 두 번째 블록의 `save_view`~`suspend_workspace_documents`, 세 번째 블록의 `reconcile_editor_features`. 바이트 대조: 조각마다 옮기기 전후 두 파일 크기 합의 차이가 경로 변경 길이(`domains::editor::Command::` → `Command::` 한 곳당 −17, settings 경로 +7, `pub(crate) ` +11)와 정확히 같았다. `TabKind`는 application.rs에서 테스트(`use super::*`)만 쓰게 되어 `#[cfg(test)] use`로 남겼고, `drafts()`가 `pub(crate)`가 되어 `Draft`를 `pub(crate)`로 올렸다.

남은 충돌 지점(이번 작업에서 하지 않음):
- editor 상태(약 60필드)는 여전히 루트 `NativeApplication`에 있다(5.9절 2단계). 그래서 editor 필드를 추가하는 lane은 루트 struct와 생성자 리터럴을 고친다. `AppSurfaces`의 editor 필드(약 50개)도 같다.
- `background_tick`의 `reveals`·`explorer_clipboards`·`problems`·`symbol_sidebars`·`breadcrumbs` reconcile 순서(editor·explorer 섞임), 재연결 블록의 editor 문장, `frame_ui`·`show_window`·`finish_frame`의 editor 문장(5.9절 3단계).
- `submit()`의 제출 실패 처리 묶음, `close`·`exit_application`의 preview·settings 문장(shell 후보 메서드).
- `presentation_refresh::Appearances`와 `apply_presentation_appearances`(위 설명).
- `AppSurfaces`의 shell·terminal_agent·editor 렌더 중 요청 Vec(`commands`, `command_errors`, `keymap_*`, `load_files`, `load_untitled`, `save_missing_drafts`, `banner_actions`, `changed_documents`)은 공용이거나 editor 상태 이동과 함께 정리할 대상이다.
- `domains/editor.rs`의 `tab_content` 탭 종류 분기 사슬(1.4절 공용 블록, 한 줄씩 추가하는 구조).

## 3. 영역별 테스트 파일 패턴

이름 필터는 `cargo test --lib -- --test-threads=4 <필터...>`에 여러 개를 함께 넘긴다(libtest는 여러 필터 중 하나라도 맞으면 실행). 표의 중괄호는 줄임 표기이며 실제로는 `application::auxiliary_window_tests application::window_state_tests ...`처럼 하나씩 넘긴다. 작업 1/4에서 쓴 shell·commands 필터 전체는 5.8절 5번 예시와 같다.

| 영역 | 앱 lib 테스트(필터) | 앱 통합 테스트(`--test`) | UI 테스트 |
| --- | --- | --- | --- |
| shell | `application::{auxiliary_window,window_state,app_menu,file_drop,shell_dialogs,project_rail,error_boundary,exit,tab_menu,tab_drag}_tests`, `welcome`, `app_menu`, `auxiliary_windows`, `window_state`, `file_drop`, `open_path_dialog`, `system_usage`, `zen`, `toast`, `bootstrap` | `projects`, `host`(일부), `explorer-system`(시스템 명령) | `--test workbench`, `--test controller` |
| commands | `application::shell_dialogs_tests`(팔레트), `command_palette`, `command_dispatch`, `command_registry`, `keybinding_editor`, `keybinding_search`, `keymap`, `shell_keymap`, `presentation::` | `host`(파일 색인), `paste-shortcuts` | `--test keymap-platform` |
| editor | `application::{code_actions,code_lenses,inlay_hints,selection_ranges,context_menu,json,view_state,file_loading,missing_draft,markdown,ai_completion,ai_edit,emmet,sticky_definition,startup}_tests`, `editor_`, `lsp`, `file_loading` | `save`, `save-syntax`, `disk`, `untitled`, `missing_draft`, `persistence`, `tabs`, `editor-reveal`, `lsp`, `lsp-recovery`, `lsp-workspace`, `lsp-workspace-worker` | `--test editor_surface`, `--test editor-*`, `--test document_admission`, `--test conflict_banner` |
| explorer_search | `application::explorer_views_tests`, `explorer`, `symbol_`, `workspace_symbol`, `breadcrumb`, `problems` (explorer 상태 경로를 고친 `application::{auxiliary_window,tab_menu,shell_dialogs,error_boundary,startup,json}_tests`, `shell_keymap`도 함께) | `explorer`, `explorer-clipboard`, `explorer-clipboard-owners`, `explorer-system` (`explorer-delete`는 실제 휴지통 사용으로 실행 금지). 경로를 고친 대상은 `explorer`·`projects`·`host` | `--test workbench`(탐색기 경로) |
| git | `title_branch`, `remote_git` (`application::auxiliary_window_tests`의 title branch 검사 포함) | `projects`(git 무효화) | 없음 |
| terminal_agent | `application::ide_selection_tests`, `terminal_`, `agent_hooks`, `ide_server`, `ide_tools`, `remote_agents`, `status_ide` | `terminal-dispatch`, `terminal-environment`, `terminal-file-links`, `terminal-frames`, `terminal-host`, `terminal-links`, `terminal-writer` | 없음 |
| settings | `settings_`, `theme_`, `snippet_host_tests`, `app_file_views`, `remote_preferences`, `presentation` | `settings-controls`, `app-file` | `--test settings-code-controls`, `--test theme-portable`, `--test snippet-*` |
| preview | `preview` | `preview*`, `open-with` | 없음 |

공통 전제: 앱 lib 테스트의 실제 LSP 경로는 `native-lsp-mock` example 실행 파일을 쓴다. target이 비어 있으면 `cargo build --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`를 먼저 실행한다(없으면 `assertion failed: mock.is_file()`로 수십 개가 실패한다). 흔들림(기존 검사 설계의 경쟁, 이번 이동과 무관한 경로): `context_menu_tests::실제_추가_경계_보조창_메뉴는_클립보드와_터미널의_소유_pane을_보존한다`, `view_state_tests::실제_일괄_닫힘은_주_보조_영역의_독립_위치와_접기를_보존한다`, `file_loading_tests::실제_거절_캐시는_보조_mirror의_수명_파일_변경과_프로젝트_종료를_따른다`는 보조 창 layout을 손으로 만든 직후 한 번 그린 뒤 단언한다. 뒤의 두 개는 이번 이동 전 `docs/quality-assurance/2026-10-27-native-auxiliary-window-moves.md`에도 병렬 부하 실패·단독 통과로 기록돼 있다. 앞의 두 개는 `body_view(..).unwrap()`에서 실패한다. 임시 진단(제거함)에서 실패 회차는 그리기 전 `controller.snapshot()`이 이미 보조 창 없는 revision 1이었다. fixture 중 비동기로 끝난 다른 layout 변경이 같은 revision 1을 먼저 차지해 `ShellController::apply_layouts`가 검사의 revision 1 layout을 건너뛴 것이다(`present.revision >= layout.revision`). 한 프레임 뒤에는 보조 본문이 생긴다. 실행마다 0~2개가 실패하고 단독·재실행에서는 통과한다. 고치려면 검사가 `finish_mutation` 전에 현재 revision을 다시 읽거나 보조 본문이 생길 때까지 `wait`해야 하며, 이는 테스트 기대값 변경이라 이번 범위에서 하지 않았다.

## 4. 이동 규칙(모든 영역 공통)

- 원문 그대로 옮긴다: 대상 구간을 Read로 확인하고 같은 문장을 영역 파일에 쓴 뒤 원래 위치를 지운다. 문장 순서, 조건, 반환값, 로그 문자열, 기존 doc 주석을 바꾸지 않는다.
- 허용되는 기계적 변경만 한다.
    1. 경로 접두사: `self.x` → `self.<영역>_state.x`, `HostCommand::X` → `HostCommand::<영역>(Command::X)`, `HostReply::X` → `HostReply::<영역>(Reply::X)`, `ShellIntent::X` → `ShellIntent::<영역>(intents::<영역>::Intent::X)`.
    2. 함수 경계: 루프 안 match arm을 함수로 옮길 때 arm의 `continue`는 그 arm이 루프 본문의 마지막 문장일 때만 `return`으로 바꾼다. `break`나 바깥 함수의 `return`이 있는 arm은 옮기지 않고 남긴다.
    3. 인자 전달: arm이 쓰던 루프 지역값은 매개변수로 받는다(`&context` → `context: &egui::Context`, `frame`, `snapshot`).
    4. 가시성: 옮긴 함수는 `pub(crate)`, 시그니처에 나오는 application 타입은 `pub(crate)`.
    5. 이동으로 쓰이지 않게 된 import·매개변수 제거, `cargo fmt`.
- 테스트 기대값은 바꾸지 않는다. 테스트 파일은 경로 컴파일 수정만 한다.
- `NativeApplication::submit()`은 일부 변형을 큐에 넣기 전에 가로챈다(`EditorContextMenu`, `CodeActions`, `RenameEditor`, `FormatEditorInput`, `SnippetEdit`(List), `SetDirty`, `SaveUntitled` 처리, `is_settings` 분류). 이 변형을 옮기는 작업자는 가로채기 패턴을 감싼 형태로 바꾸고 검사 순서를 유지한다. `is_settings`에는 이미 `HostCommand::Commands(domains::commands::Command::SetKeymapOverrides(_))`가 있다.
- `poll()`에서 옮길 arm이 루프 제어(`continue`/`break`)나 arm 밖 지역값을 쓰면 위 2·3 규칙으로 처리하고, 불가능하면 남긴 뒤 이 문서 2.7절 형식으로 기록한다.
- remote-web 동결: `native/taide-remote-web`의 소스·`Cargo.toml`·`Cargo.lock`, 모든 `Cargo.toml`·`Cargo.lock`을 바꾸지 않는다.

## 5. 남은 영역 작업자 단계별 지침

각 영역마다 아래 순서로 진행하고, 단계마다 앱 `cargo check --all-targets`로 컴파일 상태를 확인한다. 선례는 `domains/shell.rs`(큰 영역)와 `domains/commands.rs`(작은 영역)다.

### 5.1 준비

1. 이 문서 1·2·4절과 `domains/commands.rs`를 읽는다.
2. 2절 표의 자기 영역 행을 `rg -n "\bVariant\b" native/` 와 `rg -n "self\.field\b" native/taide-native-app/src`로 확인해 확정한다. 다른 영역에서 쓰는 이름은 공용 또는 사용 비중이 큰 영역으로 정하고 표를 갱신한다.

### 5.2 상태

1. `src/domains/<영역>.rs`를 만들고 `domains/mod.rs`에 `pub mod <영역>;`를 알파벳 위치에 추가한다.
2. 영역 필드를 `pub(crate) struct State`로 옮긴다(타입 그대로). 루트 struct에는 `<영역>_state: domains::<영역>::State,` 한 줄을 원래 첫 필드 위치에 둔다.
3. 생성자에서 그 필드들의 초기화식을 `<영역>_state: domains::<영역>::State { ... }` 리터럴로 옮긴다. struct 리터럴은 적힌 순서대로 평가되므로 `?`·로그·I/O가 있는 식의 **상대 순서**를 원래와 같게 둔다. 영역 리터럴 하나로 묶으면 순서가 어긋날 때는, 원래 리터럴 앞쪽 식이 모두 순수(기본값·이동)인지 확인한 뒤 해당 `?` 식을 리터럴 바로 앞 `let`으로 원래 순서대로 올린다. 선례: shell은 `window_state: Tracker::new(..)`(읽기 실패 시 `log::warn`)가 `problems`·`explorer_icons`·`navigation_icons`의 `?`보다 뒤여야 해서 `shell_state` 리터럴을 원래 `window_state` 자리(`shell` 다음)에 두고, 그보다 앞서야 하는 `status_ide_icons`·`system_usage_icon`의 `?` 식을 같은 순서로 `let`에 올렸다.
4. 필드 선언 순서는 drop 순서다. 영역 필드를 `<영역>_state` 하나로 모으면 drop 시점이 바뀌므로, 옮기는 타입과 그 하위 필드에 순서 의존 `Drop`이 없는지 `rg -n "impl Drop for" native/taide-native-app/src`로 확인한다(shell·commands 이동 타입은 직접 `Drop` 구현이 없고 채널·`Arc`·데이터만 가진다).
5. `self.field` 접근을 `self.<영역>_state.field`로 바꾼다. 여러 줄 체인(`self\n    .field`)과 `application-*-tests.rs`의 `application.field`도 함께 바꾼다. 같은 이름의 `AppSurfaces` 필드(예: `AppSurfaces.welcome`, `AppSurfaces.auxiliary_windows`)는 다른 struct이므로 건드리지 않는다.

### 5.3 HostCommand·HostReply·worker

1. 영역 변형을 `pub enum Command`/`pub enum Reply`로 옮긴다(필드 그대로).
2. `host.rs` 루트 enum 맨 위 wrapper 블록에 `<영역>(crate::application::domains::<영역>::Command)`/`Reply`를 추가한다.
3. `host.rs` `dispatch`의 해당 arm을 영역 `dispatch(worker, command)`로 옮기고, 라우팅 블록에 한 줄 추가한다. arm이 쓰던 `services`·`clipboard`·`integrations`·`reconcile`(`&worker.integrations.reconcile`) 등은 `HostWorker`에서 꺼낸다. 한 영역만 쓰는 host 보조 함수·상수는 함께 옮기고, 공용은 `pub(crate)`로 연다.
4. `poll()`의 응답 arm을 `apply_<영역>_reply`로 옮기고 라우팅 블록에 한 줄 추가한다.
5. 생성 위치를 모두 바꾼다: `application.rs`(`domains::<영역>::...`), 다른 모듈(`crate::domains::<영역>::...`), 단위 테스트 파일, 통합 테스트(`taide_native_app::domains::<영역>`). `rg -n "HostCommand::(A|B|...)\b|HostReply::(A|B|...)\b" native/`가 빈 결과가 될 때까지.

### 5.4 ShellIntent

1. `taide-native-ui/src/intents/<영역>.rs`에 `pub enum Intent`를 만들고 변형을 옮긴다(cfg 속성 포함). `intents/mod.rs`에 `pub mod <영역>;`.
2. `ShellIntent` 루트 끝 wrapper 블록에 `<영역>(crate::intents::<영역>::Intent)`를 추가한다.
3. 생성·매칭 위치를 바꾼다: UI `shell.rs` 등 UI 모듈, UI `tests/workbench.rs`, 앱 `shell_keymap.rs`·`command-dispatch.rs`·`command-palette.rs`·`application.rs`(`AppSurfaces` 포함)·`tab_close_batch.rs`·`zen.rs`·`save.rs`·`presentation.rs`. 닫는 괄호가 필요하므로 한 곳씩 Edit한다.
4. `application.rs` intent 루프의 해당 arm을 `apply_<영역>_intent`로 옮기고, catch-all 직전 라우팅 블록에 `ShellIntent::<영역>(intent) => self.apply_<영역>_intent(intent, ...),`를 추가한다.
5. `domains/shell.rs` `apply_shell_intent`의 다른 영역 목록에서 옮긴 변형을 지우고 `ShellIntent::<영역>(_)`를 추가한다.
6. `rg -n "ShellIntent|ShellSurfaces" native/taide-remote-web/src`가 여전히 빈 결과인지 확인한다.

### 5.5 프레임 tick·overlay

1. `background_tick`·`show_window`·`show_overlays`·`finish_frame` 안에서 **영역 문장만 연속된 구간**을 `tick_<영역>_*`/`show_<영역>_*`로 옮기고 같은 자리에서 호출한다(선례: `tick_shell_menus`, `tick_shell_system_usage`, `tick_commands_files`, `show_shell_usage_detail`, `show_commands_overlays`, `show_shell_dialogs`).
2. 다른 영역 문장과 섞인 구간은 남기고 2.7절에 기록한다.

### 5.6 ShellSurfaces (editor·explorer_search·git·terminal_agent·preview)

1. 첫 작업자가 UI `src/surfaces/mod.rs`를 만들고 UI `lib.rs`에 `pub mod surfaces;`를 알파벳 위치에 추가한다.
2. `src/surfaces/<영역>.rs`에 `pub trait <영역>Surfaces`를 만들고 2.5절의 메서드를 원문 그대로(기본 구현·doc 주석·연관 타입 포함) 옮긴다. 시그니처가 `ShellIntent` 등 UI 타입을 쓰면 `crate::` 경로로 import한다.
3. UI `shell.rs`: `pub trait ShellSurfaces: crate::surfaces::<영역>::<영역>Surfaces {`처럼 supertrait를 추가한다(여러 개면 `+`). `impl ShellSurfaces` 인자로 받는 함수에서 supertrait 메서드·연관 타입은 그대로 호출된다. `S::Explorer`처럼 연관 타입을 명시한 곳이 모호해지면 `<S as crate::surfaces::explorer_search::ExplorerSearchSurfaces>::Explorer`로 바꾼다.
4. 구현자 두 곳을 나눈다: 앱 `application.rs`의 `impl ShellSurfaces for AppSurfaces<'_>`에서 해당 메서드를 떼어 `domains/<영역>.rs`의 `impl <영역>Surfaces for AppSurfaces<'_>`로 옮기고, UI `tests/workbench.rs`의 `Surfaces`에도 같은 분리를 한다(기본 구현만 쓰면 빈 `impl` 블록).
5. 앱 쪽에서 trait 메서드를 메서드 호출로 쓰는 파일은 trait를 import한다(`use taide_native_ui::surfaces::<영역>::<영역>Surfaces;`).

### 5.7 영역별 주의점

- 탭 내용 분기 hook(단계 0 범위로 결정됨: `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`, editor-preview 사양 1.5절·결정 5): `AppSurfaces::tab_content`의 `let TabKind::File { path } = &tab.kind else { 내용 없음 화면 }` 앞에 `TabKind::SearchEditor { .. }`(explorer_search), `TabKind::Diff { .. }`(git, `rev`가 있으면 commit diff), `TabKind::ClaudeDiff { .. }`(git, IDE diff) arm을 두고 각 영역 함수(`domains::<영역>::show_*_tab`)로 넘긴다. 지금 세 종류는 모두 `tab.contentUnavailable` 화면이므로 그 렌더링을 그대로 공용 함수로 꺼내 세 영역 함수가 호출하게 해 동작을 유지한다. 작업 1/4에서는 하지 않았다. editor·preview 이동 작업자가 맡고, 단계 0에서 못 하면 shell lane 묶음 1의 첫 작업이다. 이 분기 블록은 1.4절 표의 공용 영역 블록으로 취급한다(각 lane은 자기 arm의 영역 함수 안만 고친다). **작업 2/4에서 완료**: `domains/editor.rs`의 `tab_content`에서 `Untitled` 분기 뒤, `let TabKind::File { path } = &tab.kind else` 앞에 `match &tab.kind { SearchEditor => show_search_editor_tab, Diff => show_diff_tab, ClaudeDiff => show_claude_diff_tab, _ => {} }` 블록을 두었고, else 분기와 세 영역 함수는 모두 `application.rs`의 `AppSurfaces::show_unavailable_tab`(옛 else 본문 그대로)을 호출한다. `Diff`의 `rev` 구분은 git lane이 `show_diff_tab` 안에서 한다.

- editor: 변형 수가 가장 많다. `submit()` 가로채기(4절), `SaveUntitled`의 format-on-save 경로, `EditDocument` arm의 `continue`, LSP 응답 루프(`poll_lsp`, `LspBridge::poll`)는 별도 enum이므로 HostReply와 구분한다. `AppSurfaces::tab_content`·`show_document`가 1,000줄 이상이라 `EditorSurfaces` 분리로 가장 큰 감소가 난다. `store`·`files`·`untitled`는 공용으로 루트에 둔다.
- preview: `reconcile_previews`가 `background_tick`의 순서에 묶여 있다. `ShellIntent::ReopenWith`, `can_reopen_with`, `open_with`를 함께 옮긴다.
- explorer_search(완료): `AppSurfaces::{begin_explorer, explorer, finish_explorer, discard_explorer}`가 크고 `self.commands` 버퍼에 `HostCommand::Shell(...)`(CopyText 등)·`HostCommand::Editor(...)`(OpenFileTab 등)를 넣는다(영역 파일에서는 `crate::domains::<영역>::Command`). `explorer_icons`는 shell의 `paint_rail_icon`도 쓴다. 남은 접점은 2.7절.
- git(완료): 가장 작다. `GitBranch` 요청·응답, `title_branches`, `branch`·`title_branch_icon` surfaces, 빈 intent seam.
- terminal_agent: `terminal_views`는 commands(`clear_keymap_chord`)와 shell 흐름에서도 쓰인다. 공용 읽기·호출은 유지한다.
- settings: `submit()`의 `is_settings`와 `SnippetEdit`(List) 가로채기, `SettingsFailed` 응답, theme·presentation 갱신이 `poll()`의 여러 arm에 흩어져 있다. `SetKeymapOverrides`는 commands 소유로 두고 settings 화면은 `HostCommand::Commands(...)`로 보낸다.

### 5.8 검증(영역마다)

1. `cargo check --quiet --all-targets --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`
2. `cargo check --quiet --all-targets --features native-host,inspection --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`
3. `cargo check --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`
4. UI를 바꿨으면 `cargo test --quiet --features native-host,inspection --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --lib --test workbench --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`
5. 3절의 영역 필터로 앱 lib 테스트(`-- --test-threads=4`)와, 경로를 고친 통합 테스트 대상만 실행. shell·commands 예: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- --test-threads=4 application::auxiliary_window_tests application::window_state_tests application::app_menu_tests application::file_drop_tests application::shell_dialogs_tests application::project_rail_tests application::error_boundary_tests application::exit_tests application::tab_menu_tests application::tab_drag_tests application::code_lenses_tests application::code_actions_tests application::selection_ranges_tests application::context_menu_tests application::json_tests application::ai_edit_tests application::ide_selection_tests application::view_state_tests application::startup_tests command_palette keybinding_editor shell_keymap command_dispatch toast snippet_host_tests file_loading presentation:: welcome app_menu auxiliary_windows window_state open_path_dialog system_usage zen`
6. `cargo fmt --manifest-path <변경 crate>/Cargo.toml -- --check`
7. `git -C /Users/hyunseokbyun/development/TAIDE diff --stat`에 `native/taide-remote-web`과 `Cargo.toml`·`Cargo.lock`이 없는지 확인
8. 새 경고가 없는지 확인(기존 경고: `editor-code-actions.rs` `is_applying`, `zen.rs` 543 `mut`, `application-inlay-hints-tests.rs` `fixture_with_emmet_peek`, UI `tests/workbench.rs` 1803 `mut`(작업 2/4에서 impl 분리로 줄 이동), vendor `wry-preview`)

### 5.9 editor 남은 이동(작업 2/4에서 못 한 것)

작업 2/4는 editor의 host 변형·응답 반영·intent·surfaces·`show_document`까지 옮겼다. 아래가 남았다. 순서대로 진행하고 단계마다 앱 `cargo check --all-targets`를 돌린다.

1. **메서드 이동**(동작 무관, 비용 큼. **리뷰 반영 작업에서 완료**, 2.8절): `application.rs`의 두 번째 `impl NativeApplication` 블록 중 `save_view`부터 `suspend_workspace_documents`까지(약 1,960줄, `poll_lsp` 약 700줄 포함)는 전부 editor다. 블록째 `domains/editor.rs`의 `impl NativeApplication`으로 옮기고 다른 모듈에서 부르는 메서드만 `pub(crate)`로 연다(현재 호출처: application.rs 프레임 흐름, `domains/commands.rs`의 `request_editor_code_actions`·`request_editor_rename`·`request_editor_formatting`·`focused_editor_source`, `domains/shell.rs`). 첫 번째 블록의 `run_editor_context_menu`(약 1,682행)부터 `flush_persistence`까지도 editor지만 중간의 `close`(shell 후보)는 남긴다. `reconcile_editor_features`(세 번째 블록)도 editor다.
2. **상태 이동**(`editor_state`, 위험 큼): 2.1절 editor 행 약 60필드. `self.<필드>` 치환은 `AppSurfaces`의 같은 이름 필드(`problems`, `problem_opens`, `editor_*`, `lsp`, `peek_models`, `find_history`, `find_appearance`, `banner_appearance`, `restore_notices`, `untitled_*`, `missing_drafts`, `loading`, `failed`, `refused`, `focused`, `reveals`, `document_edits`, `fold_commands`, `editor`)와 겹치므로 파일 전체 일괄 치환을 쓰지 않는다. 1단계로 `NativeApplication` 메서드를 먼저 `domains/editor.rs`에 모은 뒤 그 파일의 `impl NativeApplication` 구간과 `application.rs`의 `impl NativeApplication` 구간만 치환하고, `impl AppSurfaces`·`impl ShellSurfaces for AppSurfaces` 구간은 건드리지 않는다. `self.editor`는 `self.editor_*`의 접두사이므로 `self.editor.`·`self.editor,` 형태로 따로 치환한다. 같은 이유로 `self.focused`는 `self.focused_editor_source(` 메서드와 겹친다. `store`·`files`·`untitled`는 공용으로 남긴다(1.5절). 생성자 리터럴 순서: editor 초기화식 중 `?`는 `problems: Views::with_tooltips(..)?` 하나이고 그 앞뒤 식은 `Default`/이동뿐이다. drop 순서: `lsp`(`LspBridge`의 `Drop`)와 editor 상태의 `Entry`/`Slot` `Drop`(취소 토큰)은 서로 독립이다.
3. **tick·show 이동**: `background_tick`·`frame_ui`·`show_window`·`finish_frame`에서 editor 문장만 연속된 구간(예: `frame_ui`의 `editor_ide_selection.begin_frame`·`editor_view_state.begin_frame`·`editor_markdown` 준비, `finish_frame`의 `flush_persistence`)을 `tick_editor_*`/`show_editor_*`로 꺼낸다.
4. 상태 이동 뒤 단위 테스트의 `application.<editor 필드>` 접근(약 20개 테스트 파일)을 `application.editor_state.<필드>`로 고친다.

drop 순서 메모(preview, 완료분): `preview_state`를 옛 `web_bridge` 자리(`terminal_fonts` 다음)에 두어 `open_with`·`previews`·`*_previews`·`*_appearance`가 원래보다 먼저 drop된다. 모두 `#[derive(Default)]` 데이터 캐시이고 `preview_web_*`의 `Drop`(취소·서버 정리)은 `web_bridge`/`web_previews` 안에 있어 원래 위치와 같다. 생성자의 preview 초기화식은 전부 `Default`/이동이라 평가 순서가 관찰되지 않는다.

경로를 고친 통합 테스트 대상(작업 2/4, 모두 통과 122개): `host`, `projects`, `lsp`, `missing_draft`, `untitled`, `terminal-host`, `disk`, `save`, `tabs`, `persistence`, `preview`, `preview-animation`, `preview-formats`, `preview-hwp-host`, `preview-hwp3-boundary`, `preview-macos`, `preview-pdf`, `preview-presentation-host`, `preview-spreadsheet-host`.

editor·preview 앱 lib 필터(작업 2/4): `application::code_actions_tests application::code_lenses_tests application::inlay_hints_tests application::selection_ranges_tests application::context_menu_tests application::json_tests application::view_state_tests application::file_loading_tests application::missing_draft_tests application::markdown_tests application::ai_completion_tests application::ai_edit_tests application::startup_tests application::ide_selection_tests application::tab_menu_tests application::file_drop_tests application::window_state_tests application::project_rail_tests application::close_all_tests editor_ lsp file_loading preview shell_keymap command_dispatch save tab_close_batch symbol_location`.
