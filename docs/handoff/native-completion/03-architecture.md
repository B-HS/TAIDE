# 03. native 앱 구조와 새 기능을 끝까지 연결하는 방법

> 작성일: 2026-11-04(2026-11-03 판을 실제 코드로 다시 확인해 갱신) · 기준: `to_rust_native` `7be1ad14`. Git 예시는 `f3a5b5d0`에서 병합된 메인 트리 코드입니다.
> 근거는 "경로 + 심볼 이름"으로 적었습니다. 줄 번호는 기준 커밋의 값이며 바뀔 수 있습니다. 구조 이동의 설계 정본은 `docs/research/2026-10-28-native-lane-seams.md`(이하 "seams 문서")입니다. `docs/architecture.md` 9절은 2026-10-09 기준이라 이 문서가 대체합니다(예: 9.2절의 `eframe::run_native` 시작과 "제출이 overlay 뒤" 순서는 현재 코드와 다릅니다).
> 경로 약어: "앱 `x.rs`" = `native/taide-native-app/src/x.rs`, "UI `x.rs`" = `native/taide-native-ui/src/x.rs`.

## 1. crate 책임과 의존 방향

### 1.1 native crate

| crate | 책임 | 의존 |
| --- | --- | --- |
| `native/taide-native-app` | 실행 파일. `src/main.rs` → `src/application.rs`의 `NativeApplication`(eframe App). 셸 조립, host worker(`src/host.rs`), 영역 모듈(`src/domains/*.rs`), 탐색기·검색, 터미널 표면, 미리보기, LSP bridge, 원격 HTTP/WS·IDE 서버 | 아래 crate 전부 + `crates/taide-*` 16개 |
| `native/taide-native-ui` | 화면을 그리는 egui 코드: 셸(`shell.rs`), 편집기 표면(`editor_surface.rs`·`editor-*.rs`), 설정·테마·키바인딩·스니펫 편집기, 팔레트, toast, modal, tooltip, 장애 경계, 명령 레지스트리, 키맵, 셸 상태 controller(`controller.rs`)·snapshot(`snapshot.rs`), 영역 intent(`intents/`)·surface trait(`surfaces/`) | `taide-native-editor`, `taide-model`; feature `native-host`일 때 `taide-runtime`·`taide-layout`·`taide-project`·`taide-infra`·`taide-file`·`tokio` |
| `native/taide-native-editor` | egui 비의존 편집 코어: rope 문서·뷰 스토어(`store.rs` `EditorStore`·`DocumentStore`·`ViewStore`), 트랜잭션·undo, 편집 저널(`change-journal.rs`), 줄 토큰 자료형, 표시 줄 매핑(`display-map.rs`), 장식, 접기, 들여쓰기·괄호 규칙 인터페이스, 스니펫, 내장 JSON·CSS·HTML 언어 서비스 이식(`json-*.rs`, `css-*.rs`, `html-*.rs`) | `taide-model`, `ropey`, `lsp-types`, `unicode-segmentation` |
| `native/taide-native-syntax` | TextMate 구문 토큰화(`ferriki-textmate`+`ferroni`), 문법 자산, 토큰 테마, 토큰화 스레드(`token-pipeline.rs` `TokenPipeline`), 플러그인 문법, Monaco 언어 구성, JS 정규식(`regress`, `js-regex.rs`) | `taide-native-editor`, `taide-model` |
| `native/taide-native-terminal` | `alacritty_terminal`(vendored) 기반 터미널 코어, 세션, 입력 인코딩, 선택 | `taide-infra`, `taide-model`, `taide-native-retained` |
| `native/taide-native-retained` | retained 메모리 계측 유틸 | 없음 |
| `native/taide-remote-web` | 브라우저 Wasm 원격 클라이언트. 동결(컴파일만 유지) | `taide-native-ui`(`default-features = false`), `taide-native-editor`, `taide-remote-wire`, `taide-model` |

의존 방향: `taide-native-app` → (`taide-native-ui`, `taide-native-syntax`, `taide-native-terminal`) → `taide-native-editor` → `taide-model`. `taide-native-ui`와 `taide-native-syntax`는 서로 의존하지 않습니다. UI가 구문 결과를 쓸 때는 editor crate의 자료형(`LineTokens`, 스타일 표, 장식 층)만 보고, 앱이 둘을 이어 줍니다(9절).

### 1.2 동결 remote-web이 만드는 제약

`taide-remote-web`이 `taide-native-ui`와 `taide-native-editor`를 소비하고, 그 `Cargo.lock`과 소스를 바꾸면 안 됩니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md` 결정 표). 그래서 다음을 지킵니다.

1. `taide-native-ui`·`taide-native-editor`에 새 외부 의존(특히 정규식)을 넣지 않습니다. 넣으면 remote-web의 lockfile과 의존 그래프가 바뀝니다. 정규식이 필요한 규칙은 `taide-native-syntax`에서 평가합니다(같은 문서 기각 대안 표 39줄).
2. remote-web이 import하는 UI 모듈의 공개 경로를 옮기지 않습니다. 실제로 쓰는 모듈(`rg -o "taide_native_ui::[a-z_]+" native/taide-remote-web/src`의 고유값): `commands`(`ShellMutation`), `conflict_banner`, `editor_surface`, `font_preview`, `keybinding_editor`, `keymap`, `presentation`, `settings_controls`, `settings_owner`, `settings_resources`, `settings_view`, `shell`, `snapshot`, `snippet_catalog`, `snippet_edit`, `snippet_editor`, `theme_edit`, `toast`, `tooltips`. 옮겨야 하면 옛 경로에 `pub use`를 남깁니다(seams 1.3절).
3. remote-web은 `ShellIntent`·`ShellSurfaces`를 쓰지 않습니다(`rg -n "ShellIntent|ShellSurfaces" native/taide-remote-web/src` 결과 0줄). 그래서 이 둘의 영역 분리는 자유롭습니다.
4. 새로 만드는 UI 코드가 `taide-runtime` 등 host 전용 crate를 쓰면 `#[cfg(feature = "native-host")]`로 묶습니다(UI `lib.rs`의 `#[cfg(feature = "native-host")] pub mod controller;`, `pub mod document_admission;`, `editor_ai_edit` 등과 같은 형태). remote-web은 이 feature를 끄고 빌드합니다.
5. 공개 구조체 리터럴을 깨지 않습니다. 예: `EditorAppearance`·`EditorPresentation`에 필드를 늘리지 않고 새 진입 함수(`show_presented`, `show_tokenized`, `show_request`)를 추가했습니다(같은 문서 기각 대안 표 40줄).
6. UI·editor crate를 바꿀 때마다 remote-web Wasm check를 돌립니다(02 문서 8.2절).

### 1.3 공용 crate(`crates/taide-*`)

원본 Tauri 앱과 native 앱이 함께 쓰는 Rust 코어입니다. native 앱이 직접 쓰는 것(`native/taide-native-app/Cargo.toml` 66~87줄): `taide-model`(ID·`AppError`·`AppEvent`·DTO·`AppPaths`), `taide-runtime`(`AppServices`, `AppState`, `TaskSupervisor`, `EventSink`, `PlatformServices`, `*_actions.rs` facade), `taide-layout`, `taide-file`, `taide-git`(libgit2), `taide-lsp`, `taide-terminal`, `taide-infra`(PTY·watcher·secret·perf), `taide-settings`, `taide-plugin`, `taide-ide`, `taide-remote`, `taide-font`, `taide-system`, `taide-window`, `taide-agent`. 공용 crate는 edition 2021이며, 바꾸면 Tauri 앱도 같이 검사합니다(02 문서 8.6절). 원칙적으로 native 화면 작업은 공용 crate를 바꾸지 않고, 필요한 facade가 없을 때만 최소로 추가합니다.

## 2. 시작 순서(`main.rs`)

`native/taide-native-app/src/main.rs` `main`의 실제 순서입니다.

1. `taide_native_app::stderr_log::install()`.
2. 첫 인자가 `preview_web_helper::FLAG`(`--html-preview-helper`)면 HTML helper로 stdin/stdout을 서비스하고 끝납니다.
3. `taide_infra::perf::init(..)`, `bootstrap::Launch::parse(arguments)?`(`--data-dir` 필수, 02 문서 9.1절).
4. unix: `single_instance::acquire(..)`. 이미 같은 데이터 디렉터리의 인스턴스가 있으면 인자를 전달하고 종료합니다.
5. `window_state::Monitors`와 `window_state::main_window_hook(data_dir, monitors) -> eframe::WindowBuilderHook`(앱 `window-state.rs` 117줄: 초기 `ViewportBuilder`에 저장된 창 위치·크기를 모니터 목록 기준으로 적용하는 hook. `NativeOptions.window_builder`에 넣음).
6. Tokio multi-thread runtime, `TaskSupervisor::new(runtime.handle().clone())`, `runtime.block_on(bootstrap::restore(config, &tasks))?`로 `AppState`와 경고를 복원합니다.
7. `eframe::NativeOptions`(초기 1400×900, 최소 720×480, fullsize content view, 제목 숨김)와 `winit::event_loop::EventLoop::<eframe::UserEvent>::with_user_event().build()?`.
8. `eframe::create_native(WINDOW_TITLE, options, Box::new(|context| { NativeApplication::new(..)?.with_app_menu(..).with_relay(..).with_external_open(..) }), &event_loop)`. macOS에서만 `with_app_menu`(muda 메뉴바 설치), unix에서만 `with_relay`.
9. `window_state::MonitorProbe::new(application, monitors)`로 감싼 뒤 `event_loop.run_app(&mut probe)`, 끝나면 생성 결과와 `probe.take_result()`(vendored eframe 수정 4, 02 문서 3.1절)를 돌려줍니다.

`NativeApplication::new` → `new_with_secret_store` → `new_with_overrides`(`application.rs` 257·290·313줄) 순으로 위임합니다. 실제 생성 본문은 `new_with_overrides`이고, 검사는 `Overrides { secret_store, platform }`(74~79줄)로 Keychain과 OS 플랫폼 포트를 가짜로 바꿔 이 함수를 직접 부릅니다. 생성자 안에서 `bootstrap::connect_shell(state, tasks, Arc::new(PaintSink { .. }), repaint)`가 `AppServices` 조립과 `ShellController` 연결을 합니다(`bootstrap.rs` 104줄).

## 3. 앱 crate 내부 구조

### 3.1 파일 규칙

- 모듈 파일 이름은 kebab-case이고 `lib.rs`에서 `#[path = "editor-fonts.rs"] mod editor_fonts;`처럼 snake_case 모듈로 선언합니다(UI·editor·syntax crate도 같음). 일부 오래된 파일은 snake_case 이름입니다(`terminal_host.rs`, `preview_web_view.rs`, `shell_keymap.rs`).
- 실제 App 검사 파일은 `src/application-<주제>-tests.rs`이고, `application.rs` 끝에서 `#[cfg(all(test, unix))] #[path = "application-<주제>-tests.rs"] mod <주제>_tests;`로 붙습니다. 영역 파일에 붙은 것도 있습니다(`domains/shell.rs` 끝의 `#[path = "../application-notifications-tests.rs"] mod notifications_tests;`, `ide_sync_tests`, `ipc_sync_tests`). 이렇게 붙으면 검사가 `NativeApplication`의 비공개 필드에 접근할 수 있습니다.
- `lib.rs` 마지막 줄 `pub use application::domains;` 덕분에 crate 안에서는 `crate::domains::<영역>`, 통합 검사에서는 `taide_native_app::domains::<영역>`으로 씁니다.

### 3.2 `NativeApplication`과 영역 모듈

`application.rs`의 `pub struct NativeApplication`(144~254줄, 필드 109개)은 공용 상태와 영역 상태 묶음을 가집니다.

| 종류 | 필드 |
| --- | --- |
| 영역 상태 묶음 7개 | `shell_state`, `commands_state`, `explorer_search_state`, `git_state`, `terminal_agent_state`, `settings_state`, `preview_state`(각 `domains::<영역>::State`) |
| 공용(루트 유지, seams 1.5절) | `runtime`, `services`(`Arc<AppServices>`), `application_ports`, `controller`(`ShellController`), `bridge`(`Option<HostBridge>`), `shell`(`NativeShell`), `locale`, `store`(`EditorStore`), `files`, `untitled`, `toasts`(`ToastLayers`), `toast_theme`, `tooltips`, `tooltip_appearance`, `motion_preference`, `status`, `closing`, `close_request_active`, `is_exit_ready` |
| 그 밖(대부분 편집기, 아직 루트) | `lsp`, `editor`, `editor_*`(약 40개), `problems`, `persistence`, `loading`, `failed`, `refused`, `pending_tab_close`, `focused`, `reveals`, `mirror_sweeps`, `save_picker` 등 |

편집기 상태는 아직 `editor_state`로 묶이지 않았습니다(seams 5.9절 2~4단계 미완료). 그래서 편집기 필드를 추가하면 루트 struct와 생성자 `new_with_overrides`의 `let application = Self { .. }` 리터럴(535줄부터)을 직접 고칩니다. seams 1.4절 표는 생성자를 `new_with_secret_store`라고 적었지만 영역 리터럴(`terminal_agent_state: domains::terminal_agent::State { .. }` 등, 541~723줄)은 `new_with_overrides` 안에 있습니다.

영역 모듈 `src/domains/<영역>.rs`(`domains/mod.rs`에 알파벳 순: `commands`, `editor`, `explorer_search`, `git`, `preview`, `settings`, `shell`, `terminal_agent`)는 `application.rs` 35줄의 `#[path = "domains/mod.rs"] pub mod domains;`로 선언된 application의 자식 모듈입니다. 그래서 `NativeApplication`의 비공개 필드·메서드와 `application.rs`의 비공개 항목(`WINDOW_LABEL`, `APPLICATION_NAME`, `AppSurfaces`, `WindowPass`, `editor_error`, `paint_rail_icon` 등)에 가시성 변경 없이 접근합니다(seams 1.1절).

영역 파일의 항목 순서(seams 1.1절. 가장 작은 실제 예: 메인의 `domains/git.rs` 144줄 전체):

1. `pub enum Command`: 이 영역의 host 요청(`HostCommand::<영역>(Command)`).
2. `pub enum Reply`: 이 영역의 host 응답(`HostReply::<영역>(Reply)`).
3. `pub(crate) struct State`: 영역 소유 상태(`NativeApplication.<영역>_state`).
4. `pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply>`: host worker에서 실행되는 처리.
5. 영역 전용 비동기 보조 함수.
6. `impl NativeApplication { ... }`: `apply_<영역>_reply`, `apply_<영역>_intent`, `tick_<영역>_*`, `show_<영역>_*`, `reconnect_<영역>`, 영역 메서드. 다른 모듈이 부르는 것만 `pub(crate)`.
7. `impl <영역>Surfaces for AppSurfaces<'_> { ... }`: UI가 그리는 중 앱에 묻는 질의의 구현.
8. 필요하면 `#[derive(Default)] pub(crate) struct FrameOutput`(그리는 중 쌓는 요청)과 입력 묶음(`domains/explorer_search.rs` `ExplorerInputs<'a>`).

## 4. host 호출: `HostCommand`/`HostReply`/`HostBridge`/`HostWorker`

### 4.1 흐름

1. UI 스레드가 `self.submit(HostCommand::<영역>(Command::X { .. }))`를 부릅니다(`application.rs` `NativeApplication::submit`, 918줄). 반환값 `bool`은 큐에 넣었는지입니다.
2. `submit`은 일부 명령을 큐에 넣기 전에 가로챕니다. 순서: `intercept_editor_command`(EditorContextMenu·CodeActions·RenameEditor·FormatEditorInput) → `intercept_settings_command`(SnippetEdit List) → `domains::settings::is_settings_command` 분류 → `prepare_editor_submission`(SetDirty·SaveUntitled·Save/AutoSave → SaveTracked). 각 가로채기는 `std::ops::ControlFlow`를 돌려주고 `Break(v)`면 그 값으로 끝냅니다(seams 2.8절).
3. `HostBridge::submit`(`host.rs` 257줄)이 `tokio::sync::mpsc` 채널(`HOST_COMMAND_CAPACITY = 64`, 10줄)에 `try_send`합니다. 가득 차면 `AppError::Internal("native host command queue is full")`이 되고, `submit`은 실패 처리(snippet catalog·snippet·theme 응답 실패 전달, 터미널 파일 링크 취소, 추적 저장 실패 표시, 설정이면 `toasts.settings_failed`, 그 밖에는 `report_once`) 뒤 `false`를 돌려줍니다. 호출자는 `false`면 다음 프레임에 다시 시도하도록 상태를 되돌리고 `context.request_repaint()`를 부릅니다(예: 메인 `domains/git.rs` `tick_git_title_branches`의 `self.git_state.title_branches.retry(&project)`).
4. worker 하나(`tasks.spawn_transient_handle("native-host-commands", ..)`, `host.rs` 226줄)가 명령을 한 번에 하나씩 `host.rs` `dispatch`로 처리합니다. `dispatch`는 영역 wrapper로 라우팅만 합니다: `HostCommand::Git(command) => crate::application::domains::git::dispatch(worker, command).await`. 종료 중(`state.is_shutting_down()`)이면 루프를 끝냅니다.
5. 영역 `dispatch`가 `Some(HostReply)`를 돌려주면 worker가 응답 채널에 보내고, 응답이 있든 없든 `repaint()`를 부릅니다.
6. 다음 프레임 `background_tick` → `poll`(749줄)이 `HostBridge::poll`로 응답을 꺼내 `HostReply::<영역>(reply) => self.apply_<영역>_reply(..)`로 라우팅합니다. 공용 응답은 `HostReply::SettingsFailed(error)`(설정 실패 toast)와 `HostReply::Failed(error)`(`report` → IPC 오류 toast)입니다.

### 4.2 `HostWorker`와 오래 걸리는 작업(transient task)

`HostWorker<'a>`(`host.rs` 59줄)는 worker 루프의 공용 의존을 빌려 줍니다: `services`(`&Arc<AppServices>`), `clipboard`, `read_clipboard`, `sender`(응답 채널), `repaint`, `integrations`(`HostIntegrations`: `view_states` flush, `terminals`, 원격 설정 `reconcile`, `wait_markers`, `agent_cli` 포트). 영역 `dispatch`는 필요한 것만 꺼냅니다(`let services = worker.services;`).

worker는 직렬이라 한 명령이 오래 걸리면 뒤 명령이 모두 기다리고 큐가 찹니다. 그래서 처리 방식이 둘입니다.

| 방식 | 언제 | 실제 예 |
| --- | --- | --- |
| worker 안에서 `await` | 짧은 작업, 순서 보장이 필요한 변경 | `domains/shell.rs` `Command::Notify`: `services.tasks.run_blocking_result("native-notification", move || ...)`를 `await`하고 `Some(HostReply::Shell(Reply::NotificationSent(result)))` |
| transient task로 분리 | 오래 걸리는 읽기(Git 상태, 검색 등) | 메인 `domains/git.rs` `Command::GitBranch`: `services.tasks.spawn_transient("native-title-branch", async move { ...; if sender.send(reply).await.is_ok() { repaint(); } })`. 시작에 성공하면 `None`(응답은 task가 직접 보냄), 실패(종료 중)하면 즉시 `Some(실패 응답)` |

`TaskSupervisor`(`crates/taide-runtime/src/task_supervisor.rs`)의 `spawn_transient(name, future) -> bool`(94줄)은 감독이 멈췄으면(`is_stopped`) `false`를 돌려줍니다. `run_blocking_result(name, closure)`(132줄)는 blocking 작업을 감독 아래 실행합니다. 감독 밖에서 `tokio::spawn`·`std::thread::spawn`을 새로 쓰지 않습니다. 종료 때 회수되지 않는 작업이 생기기 때문입니다. Git lane은 이 패턴을 `spawn_reply(worker, name, future, refused) -> Option<HostReply>` 보조 함수로 묶었습니다(`lane/git` 커밋 `5195d415`의 `domains/git.rs` 311줄).

LSP는 별도 채널입니다. `crate::lsp::LspBridge`가 자기 worker와 `lsp::Reply` enum을 가지며, `background_tick`의 `poll_lsp`(편집기 영역 메서드)가 꺼냅니다. 탐색기 관련 LSP 응답 5종(`WorkspaceSymbols`, `ExplorerPasted`, `ExplorerCreated`, `ExplorerRenamed`, `ExplorerDeleteFinished`)만 `apply_explorer_search_lsp_reply`로 넘깁니다(seams 2.8절).

## 5. `ShellIntent`·`ShellSurfaces`·`AppSurfaces`·`FrameOutput`

### 5.1 `ShellIntent`(UI → 앱, 그린 결과로 생긴 의도)

- 정의: UI `commands.rs` `pub enum ShellIntent`(53~108줄). 루트에 shell 변형(`Mutate`, `OpenFolder`, `OpenFile`, `RequestCloseTab` 등 24개)이 있고, 끝에 영역 wrapper 블록이 있습니다: `Shell(crate::intents::shell::Intent)`, `Commands(..)`, `Editor(..)`, `ExplorerSearch(..)`, `Git(..)`, `TerminalAgent(..)`, `Settings(..)`, `Preview(..)`.
- 영역 하위 enum: UI `intents/<영역>.rs`의 `#[derive(Debug, Clone)] pub enum Intent`. 메인에서 `intents/shell.rs`와 `intents/git.rs`는 빈 enum(`pub enum Intent {}`)입니다. Git lane에서는 `intents/git.rs`가 275줄(`ShowView`, `ToggleSection` 등)로 채워졌습니다. 새 shell intent는 루트가 아니라 `intents/shell.rs`에 추가합니다(seams 1.3절).
- 생성: `NativeShell::show(&self, ui, snapshot, surfaces: &mut impl ShellSurfaces) -> Vec<ShellIntent>`(UI `shell.rs` 366줄)가 그리면서 모은 intent를 돌려줍니다. 키맵 action도 `crate::command_dispatch::intent_with_editor(..)`(앱 `command-dispatch.rs` 90줄)로 intent가 됩니다.
- 처리: `application.rs` `show_window`의 intent 루프가 영역별로 라우팅합니다.

    ```rust
    for intent in intents {
        match intent {
            ShellIntent::Commands(intent) => self.apply_commands_intent(intent, &context),
            ShellIntent::Editor(intent) => self.apply_editor_intent(
                intent,
                &context,
                frame,
                &snapshot,
                &command_context,
                &keymap_documents,
            ),
            ShellIntent::ExplorerSearch(intent) => {
                self.apply_explorer_search_intent(intent, &context, &snapshot)
            }
            ShellIntent::Git(intent) => self.apply_git_intent(intent),
            ShellIntent::TerminalAgent(intent) => self.apply_terminal_agent_intent(intent),
            ShellIntent::Settings(intent) => self.apply_settings_intent(intent, &snapshot),
            ShellIntent::Preview(intent) => self.apply_preview_intent(intent, &context),
            intent => self.apply_shell_intent(intent, frame, &snapshot),
        }
    }
    ```

    `domains/shell.rs` `apply_shell_intent`(680줄)는 `ShellIntent::Shell(intent) => match intent {}`부터 shell 변형을 모두 나열하고 마지막 arm(825줄 부근 `ShellIntent::Commands(_) | ...`)에서 다른 영역 wrapper를 무시합니다. 변형 추가 시 처리 누락을 컴파일러가 잡게 하려는 구조입니다.
- `ShellIntent::Mutate(ShellMutation)`은 `self.controller.submit(command)`로 셸 controller에 보내고 실패하면 `report`합니다(6절).

### 5.2 `ShellSurfaces`(UI가 그리는 중 앱에 묻는 질의)

- 정의: UI `shell.rs` 225줄 `pub trait ShellSurfaces: EditorSurfaces + ExplorerSearchSurfaces + GitSurfaces + PreviewSurfaces + TerminalAgentSurfaces`. 루트 메서드는 shell 소유입니다(`text`, `text_with_args`, 상태 표시줄 4종 `prepare_status_bar`·`status_bar`·`apply_status_bar`·`discard_status_bar`와 연관 타입 `StatusOutput`, `welcome_content`, `rail_recent_projects`, `is_reduced_motion`, `rail_icon`, `tooltip`). 영역 메서드는 UI `surfaces/<영역>.rs`의 `pub trait <영역>Surfaces`입니다(예: `surfaces/git.rs` `GitSurfaces { fn branch(..); fn title_branch_icon(..) {} }`). commands·settings는 surfaces 파일이 없습니다. 필요해지면 파일을 만들고 supertrait 목록 끝에 한 줄 추가합니다.
- 구현자는 두 곳뿐입니다: 앱 `application.rs`의 `struct AppSurfaces<'a>`(2524줄. 루트 trait 구현은 `impl ShellSurfaces for AppSurfaces<'_>` 2656줄, 영역 trait 구현은 각 `domains/<영역>.rs`), UI 검사 `native/taide-native-ui/tests/workbench.rs`의 `Surfaces`. 영역 trait에 기본 구현 없는 메서드를 추가하면 두 곳 모두 구현해야 컴파일됩니다.
- `AppSurfaces`는 `show_window`가 창 pass마다 `NativeApplication` 필드의 빌림(`&mut self.store`, `&self.locale` 등)을 모아 만드는 struct 리터럴입니다(`show_window`의 `let mut surfaces = AppSurfaces { .. }`). 그리는 동안 생긴 요청은 지역 Vec나 영역 `FrameOutput`에 쌓고, `shell.show`가 끝난 뒤 소비합니다. 그리는 도중에 host로 제출하지 않습니다.
- 탭 내용 분기: `domains/editor.rs` `AppSurfaces::tab_content`(5361줄 부근)가 `TabKind`별로 `show_settings_tab`·`show_app_file_tab`(settings), `show_terminal_tab`(terminal_agent), 문서(`show_document`, editor), `show_search_editor_tab`(explorer_search), `show_diff_tab`·`show_claude_diff_tab`(git), 미리보기(`show_preview_tab`, preview)로 넘깁니다. 각 lane은 자기 함수 안만 고칩니다. 메인의 `show_diff_tab`·`show_claude_diff_tab`은 아직 `show_unavailable_tab`을 부르는 자리이고, Git lane에서 실제 본문으로 바뀌었습니다.

### 5.3 `FrameOutput`

그리는 중 쌓는 영역 요청을 영역 파일의 `#[derive(Default)] pub(crate) struct FrameOutput`으로 묶습니다: `domains::preview::FrameOutput`(76줄, 미리보기 적재 요청), `domains::settings::FrameOutput`(183줄: `load_settings`, `load_settings_resources`, `load_app_files`, `settings_errors`, `theme_errors`, `snippet_notices`), `domains::explorer_search::FrameOutput`(187줄: `rename_entries`, `paste_entries`, `create_entries`, `delete_entries`, `discarded_explorers`, `load_trees`, `is_editor_widget_focused`). `show_window`에 지역 변수(`preview_output`, `settings_output`, `explorer_output`)가 있고 `AppSurfaces`에 같은 이름 필드가 있습니다. 렌더 코드는 `self.<영역>_output.<필드>.push(..)`로 쓰고, `show_window`가 `shell.show` 뒤 기존 소비 함수(`load_preview_requests`, `load_settings_requests`, `submit_explorer_search_entries` 등)로 넘깁니다(seams 2.8절).

## 6. 셸 상태: `ShellController`·`ShellMutation`·`ShellSnapshot`

- UI `controller.rs` `ShellController`는 셸 상태(프로젝트·layout·슬롯·탭) 변경을 맡는 별도 worker(`spawn_transient_handle("native-shell-controller", ..)`, 70줄)를 가집니다.
- 변경: `controller.submit(ShellMutation::X)`(107줄)가 큐에 `try_send`합니다. 가득 차면 `native shell command queue is full`. worker가 `AppState`를 바꾼 뒤 `ShellSnapshot::read(&state)`를 `watch` 채널로 게시하고 repaint합니다. 탭 활성화·이동 등 일부 변경 전에는 `set_before_mutation`(120줄)으로 등록한 캡처(편집기 view state 저장)를 먼저 실행합니다.
- 읽기: `controller.snapshot()`(127줄)이 최신 `Arc<ShellSnapshot>`(projects, layouts, shell tree, window chrome 등)을 돌려줍니다. host 응답이 layout을 이미 바꿨을 때는 `apply_layouts`(152줄)로 더 새 revision을 먼저 반영합니다. 같은 revision이면 건너뛰므로(`present.revision >= layout.revision`) 검사에서 경쟁이 생길 수 있습니다(02 문서 12.2절 첫 행).
- 오류: `controller.take_error()`(170줄)를 `poll`이 꺼내 `report`합니다.
- 이벤트: `NativeShellEvents::publish`(25~35줄)가 받은 `AppEvent`를 `forward`(앱의 `PaintSink`)로 넘기고 snapshot 갱신을 깨웁니다(`refresh.notify_one()`).

## 7. 이벤트: `AppEvent` → `PaintSink` → 영역 inbox

백엔드(공용 crate)가 내는 `taide_model::app_event::AppEvent`는 `AppServices.events`(controller의 `NativeShellEvents`)를 거쳐 `application.rs`의 `struct PaintSink`(50줄, `impl EventSink for PaintSink` 61줄)에 도착합니다. `PaintSink::publish`는 어느 스레드에서든 불릴 수 있으므로 UI 상태를 직접 바꾸지 않습니다. 각 영역의 `Mutex` 기반 inbox에 기록만 하고 `context.request_repaint()`를 부릅니다. UI 스레드가 다음 tick에 inbox를 비워 처리합니다.

| `PaintSink` 필드(메인) | 타입 | 기록하는 이벤트 | 소비 위치 |
| --- | --- | --- | --- |
| `trees` | `crate::events::TreeChanges` | `FsChanged`, `FsRescanRequired`(미리보기 무효화는 `record_preview`) | `tick_explorer_search_trees`, `reconcile_previews`(`take_previews`) |
| `file_indexes` | `crate::command_palette::FileIndexChanges` | `FsRescanRequired`, `FsChanged`(kind가 `Modified`가 아닌 것) | `tick_commands_files` |
| `presentation` | `crate::presentation_refresh::Changes` | `SettingsChanged`, `ThemeChanged`, `SyncStateChanged` | `poll_settings_revisions`, `sync_theme_preview` |
| `git_cache` | `crate::git_cache::Changes` | `GitStatusChanged`, `GitRefsChanged`, `FsRescanRequired`(저장소 전체), `FsChanged`(경로 단위) | `tick_git_cache`(`domains/git.rs` 2530줄) |
| `app_menu` | `crate::app_menu::Changes` | `ProjectListChanged`, `ProjectActivated`, `ProjectRecentCleared` 등 | `tick_shell_menus` |
| `providers` | `crate::provider_events::Inbox` | `AgentStateChanged`, `TerminalCommandFinished`, `LspInstallProgress`(Done·Failed), `IdeSaveRequested`, `ProjectClosed`, `AgentExternalOpen` | `consume_provider_events`(13.1절) |
| `settings` | `domains::settings::Events`(영역 상태에서는 `settings_state.sections_events`) | `LspInstallProgress`, `SyncStateChanged`, `RemoteStateChanged` | `tick_settings_reads` |

제목줄 branch는 `GitCache::status_result`를 읽는 `State::title_branch`(`domains/git.rs` 301줄)이며 `title_branch` 모듈은 삭제됐습니다. `tick_git_title_branches`(2508줄)는 제목줄에 보이는 프로젝트를 `Key::Status`로 관찰하는 함수로 남아 있습니다.

새 inbox를 추가하는 순서:

1. 영역 모듈(또는 영역 전용 파일)에 `#[derive(Default)] pub(crate) struct <이름> { 내부: Mutex<..> }`와 `pub(crate) fn record(&self, event: &AppEvent)`, `pub(crate) fn take(&self) -> ..`를 만듭니다. 다른 스레드의 panic으로 `Mutex`가 poison돼도 기록을 잃지 않게 `lock().unwrap_or_else(|error| error.into_inner())`로 잠급니다(Git lane `git-cache.rs` `Changes::record`·`take`와 같은 형태).
2. 생성자 `new_with_overrides`에서 `let x = Arc::new(<이름>::default());`를 만들어 `PaintSink { <필드>: x.clone(), .. }` 리터럴과 영역 `State` 리터럴에 같은 `Arc`를 넣습니다.
3. `PaintSink` struct에 필드 한 줄, `PaintSink::publish`에 `self.<필드>.record(&event);` 한 줄을 추가합니다.
4. 영역 tick에서 `take()`로 비웁니다.
5. 버스트로 무한히 자라지 않게 상한을 둡니다(`provider-events.rs` `MAX_PROVIDER_EVENTS = 256`, 넘치면 가장 오래된 비종료 이벤트를 버리고 `log::warn!("native provider events overflowed; ...")`).

## 8. 한 프레임 처리 순서

eframe가 매 프레임 `App::logic` → `App::ui`를 부릅니다(`application.rs` `impl eframe::App for NativeApplication`, 1332줄). 입력 hook(`raw_input_hook`, `raw_input_hook_with_replay`)은 그 전에 불려 메뉴바 클릭 주입(`shell_state.app_menu.inject`)과 터미널 raw 입력 복사(`terminal_views.raw_input`)를 합니다. `logic`은 `shell_state.main_window_zoomed`를 갱신한 뒤 `background_tick`을 부르고, `ui`는 `frame_ui`를 부릅니다.

### 8.1 `logic` → `background_tick`(그리기 없음, 1071줄)

1. `tick_shell_menus`(메뉴바 모델), toast 감속 설정, 메인 toast의 위치 설정(`set_position`)과 시간 진행(`tick`).
2. 터미널 입력 flush(종료 중이면 취소와 `reveals.clear()`), `tick_terminal_agent_fonts`, 편집기 글꼴 크기 반영, `tick_terminal_agent_cursors`, `tick_terminal_agent_live_sessions`.
3. `reconcile_previews`(미리보기 무효화).
4. `poll(context)`: `poll_preview_web` → controller 오류 `report` → `poll_settings_revisions` → host 응답 루프(4.1절 6번).
5. `consume_provider_events`(7절 `providers` inbox, shell 영역).
6. `sync_theme_preview`. 종료 중이 아니면 플러그인 문법 추적과 구문 tick(`editor_syntax.follow_plugins`, `editor_syntax.tick(&self.store, theme, now)`), `tick_settings_reads`.
7. reveal·탐색기 클립보드·문제 패널·심볼 사이드바·breadcrumb reconcile, `reconcile_builtin_json`, selection range tick, `reconcile_lsp`, `poll_native_formatting`, `dispatch_completion`, 심볼·접기 갱신 예약, `poll_lsp`, 이름 변경·code action poll.
8. 단축키 chord 상태, `tick_shell_system_usage`, 작업 공간 일시정지 해제, `fulfill_view_states`.
9. 종료 진행(`closing` 수신) 처리. 종료가 실패하면 재연결: `reconnect_explorer_search`, `reconnect_git`, `reconnect_shell`, `reconnect_commands`, `reconnect_preview` 뒤 `HostBridge`·`LspBridge`·`EditorSyntax` 재연결과 dirty 문서 영속화 예약.
10. 창 상태 관찰(`window_state.observe`), 닫기 요청 처리(`CancelClose` 후 `close`).
11. `tick_commands_files`. 종료 중이 아니면 `tick_explorer_search_trees`, mirror sweep 제출(`HostCommand::Editor(Command::PruneMirrors)`), `tick_git_title_branches`.
12. 작업 공간이 바쁘지 않으면 `flush_observations`, `flush_persistence`.

### 8.2 `ui` → `frame_ui`(1383줄)

1. 창 focus가 없으면 키맵 chord 해제, `tick_preview_media`, snapshot 읽기, IDE selection·view state·Markdown 준비, `tick_explorer_search_clipboards`, Zen 전체 화면 reconcile, `tick_terminal_agent_views`, `reconcile_editor_features`(문서 편집·접기 명령 수집), `settings_views.begin_frame`.
2. `WindowPass { snapshot, document_edits, fold_commands, web_placements, root_enabled, overlay }`(1373줄) 생성. `overlay`는 이번 프레임 overlay를 그릴 viewport입니다(`domains/shell.rs` `overlay_viewport` 1473줄).
3. `show_window(ui, frame, &mut pass)`: 메인 창(8.3절).
4. `show_auxiliary_windows(..)`: 보조 창마다 같은 `show_window`(9절).
5. `app_menu.finish_child_passes`, `finish_window_frames`(`tick_terminal_agent_ide_selection`, `settings_views.finish_frame`, `sync_theme_preview`).
6. 두 pass가 모두 진행됐으면 `finish_frame`(2283줄): `flush_persistence` → `show_preview_web_views`(WKWebView 위치 동기화, 11.2절) → 메인 toast 그리기 → `run_toast_actions` → `fulfill_view_states`.

### 8.3 `show_window`(창 하나의 pass, 1686줄)

1. `tick_explorer_search_breadcrumbs`, `window_command_context`(대상 pane·편집기 프로젝트·focus view·`CommandContext`), 지역 요청 버퍼와 `FrameOutput` 3개 생성, `is_enabled` 계산(종료 중·시스템 사용량 상세·삭제 확인·탭 닫기 대기·저장 대기 등이면 셸 입력 비활성).
2. 키맵 1단계(그리기 전): 입력 가능하고 창에 focus가 있으면 `terminal_views.capture_window_keymap_with_local(..)`로 창 수준 단축키를 가로채 `keymap_actions`에 넣습니다. 편집기 AI 편집 입력·code action·이름 변경·peek 위치 목록이 자기 키를 먼저 소유하는지 클로저로 확인합니다.
3. `tooltips.begin_frame`, `AppSurfaces` 리터럴 생성.
4. `ui.add_enabled_ui(is_enabled && 키바인딩 편집기·팔레트가 닫힘, |ui| self.shell.show(ui, &snapshot, &mut surfaces))` → `intents`.
5. `close_explorer_search_discarded_delete`. 메인 창이면 Zen 안내, 파일 drop overlay와 drop 제출(`HostCommand::Shell(Command::OpenDroppedPaths)`). 상태 오류 배너(`status_error_banner::show`, 재시도면 `presentation_refresh.retry()`).
6. 설정 오류·테마 오류·명령 오류·스니펫 안내 toast, `pick_settings_files`, `load_settings_requests`, `finish_terminal_agent_frame`.
7. 키맵 2단계(입력 가능할 때): `keybindings.observe_closed`, `terminal_views.route_window_keys(..)`, overlay 창이면 팔레트 명령 합류, `keymap_actions`를 `command_dispatch::intent_with_editor`로 intent로 바꿔 `intents`에 추가.
8. 편집된 문서의 영속화 표시, 전역 ⌘N(원본에 없는 단축키로 미결 결정 항목, 01 문서 5.2절), untitled·missing draft·파일 적재 제출, `load_preview_requests`, `load_explorer_search_trees`.
9. 쌓인 `commands` 제출: `SetDirty`·`ThemeEdit`·`SnippetEdit`는 즉시 `submit`, 나머지는 `flush_dirty()`가 성공한 뒤 `submit_explorer_search_entries`(생성·이름 변경·붙여넣기) → 나머지 명령 순서대로. `flush_dirty()`가 실패하면 탐색기 요청을 실패 처리하고 이 pass는 `false`로 끝납니다.
10. intent 루프(5.1절), 디스크 충돌 배너 action(`choose_disk`), 삭제 확인 대기열(`queue_explorer_search_delete`), `flush_observations`.
11. overlay 창이면 `show_overlays`(2267줄: `show_shell_usage_detail` → `show_commands_overlays`(팔레트·키바인딩 편집기·태스크 러너) → `tooltips.finish_frame` → `show_explorer_search_delete_dialog` → `tab_close_dialog` → `show_shell_dialogs`), 아니면 `tooltips.finish_frame`만.
12. 메인 창의 Zen Escape 처리, `finish_terminal_agent_frame`, `true` 반환.

요약: "background_tick → 키맵 가로채기 → shell.show → 배너 → 키맵 라우팅 → host 제출 → intent 처리 → overlay → (보조 창마다 반복) → finish_frame(영속화·WebView·toast)".

## 9. 보조 창 viewport

- 결정: eframe 다중 viewport로 구현(`docs/acknowledge/2026-10-06-native-transition-decisions.md` 결정 표 16줄).
- 상태: `shell_state.auxiliary_windows`(앱 `auxiliary-windows.rs` `Windows` 52줄). 창마다 `Window { project, slot, label, viewport, .. }`(16줄)이고 viewport ID는 `egui::ViewportId::from_hash_of((VIEWPORT_SALT, &label))`(`VIEWPORT_SALT = "taide-auxiliary-window"`)입니다.
- 그리기: `domains/shell.rs` `show_auxiliary_windows`(1503줄)가 살아 있는 창마다 `self.toasts.mount(window.viewport)` 뒤 `context.show_viewport_immediate(window.viewport, auxiliary_windows::builder(title), |ui, _| { .. })` 안에서 메뉴 이벤트 주입, `tick_auxiliary_toasts`, `self.shell.scope`를 잠시 `window.scope()`(`WindowScope::Auxiliary { project, slot }`)로 바꾸고 같은 `show_window`를 부른 뒤 원래 scope 복구, `show_auxiliary_toasts`, OS 닫기 요청 확인을 합니다. 닫기 요청이 있으면 `close_auxiliary_window`. 레이아웃에서 사라진 창도 `close_auxiliary_window`로 정리합니다.
- UI 쪽: `NativeShell::show`가 `WindowScope::Auxiliary`면 제목 표시줄, 선택적 탐색기(`egui::Panel::left(Id::new(("auxiliary-explorer", project, *slot)))`, `is_auxiliary_explorer_open`), 그 창의 pane tree만 그립니다(UI `shell.rs` 401줄 부근).
- 창별 상태: focus된 편집기(`auxiliary_windows.focused_editor(viewport)`), toast 층(`toast-layers.rs` `ToastLayers::auxiliary(viewport)`), overlay 창(`overlay_viewport`).
- vendored eframe 수정 1·3(02 문서 3.1절)이 보조 창 접근성 트리와 편집 메뉴 요청을 가능하게 합니다. 남은 범위는 `shell-5` partial(기능표 "범위/잔여 동작" 열, 단계 D 기록 `docs/PROCESS.md` 19줄: 보조 창 WKWebView·터미널 raw input hook·theme preview·탐색기 선택 상태의 창별 분리).

## 10. 편집기: 코어·표면·구문 파이프라인

1. 문서·뷰: `store: EditorStore`(`native/taide-native-editor/src/store.rs` 253줄)가 `DocumentStore`(177줄, `DocumentId`별 rope·revision·dirty·read-only·크기 등급·undo)와 `ViewStore`(227줄, `ViewId`별 선택·스크롤·접기·IME composition)를 가집니다. 편집은 `EditorStore::apply(..)`(1598줄)로 트랜잭션(편집·선택·undo 묶음)을 적용하고, 저장은 `EditorStore::save_snapshot(document)`(1828줄)로 저장할 내용을 얻습니다. 파일 탭과 문서의 대응은 앱의 `files: HashMap<String, FileDocument>`·`untitled: HashMap<TabId, UntitledDocument>`입니다(여러 영역이 읽으므로 루트 공용). 한도는 `application.rs` 상수 `MAX_DOCUMENTS = 256`, `MAX_VIEWS = 512`, `MAX_UNDO_GROUPS = 200`입니다.
2. 표면: UI `editor_surface.rs`의 `NativeEditor`가 `show`·`show_with_keymap`·`show_with_input_route`·`show_presented`·`show_tokenized`·`show_request`·`show_request_with_editor_keymap` 진입점을 가집니다(1164줄부터). 새 입력이 필요하면 `EditorRequest`(181줄)를 쓰는 진입을 늘리고 `EditorAppearance` 필드를 늘리지 않습니다(1.2절 5번).
3. 앱 연결: `domains/editor.rs` `AppSurfaces::show_document`가 탭의 문서를 표면에 그리고, 그 안에서 breadcrumb(`show_breadcrumbs`, explorer_search), 찾기, 문제, peek, 완성, inlay hint, code lens 등의 상태를 함께 넘깁니다.
4. 구문: 앱 `editor-syntax.rs`의 `EditorSyntax`(333줄)가 문서 열기·편집(저널)·닫기·테마 변경을 `taide-native-syntax`의 `TokenPipeline` 토큰화 스레드 요청으로 바꿉니다. `background_tick`의 `editor_syntax.tick(&store, theme, now)`(437줄)가 응답을 받고, 그리는 중에는 `SyntaxLease::frame_tokens`(203·218줄)가 줄 토큰을 표면에 빌려 줍니다(`domains/editor.rs` `tokens_supplier`). 저장 직전에는 정확히 토큰화된 줄까지를 저장 정리에 공급합니다(`supply_save_cleanup`, 585줄). 테마가 바뀌면 다시 토큰화합니다. 테마는 `settings_state.preview_theme`이 있으면 그것, 없으면 `resolved_theme`입니다.
5. LSP: `lsp: Option<crate::lsp::LspBridge>`, `reconcile_lsp`, `poll_lsp`, 요청별 상태(`editor_completion`, `editor_highlights`, `editor_inlay_hints`, `editor_code_lenses`, `editor_rename`, `editor_code_actions` 등)가 루트에 있습니다. 내장 언어 서비스(JSON·CSS·HTML)는 editor crate의 순수 Rust 이식을 앱 `editor-json*.rs`·`editor-css.rs`·`editor-html.rs`가 공급합니다.
6. 키: 편집기 기본 키는 UI의 `*-keymap-defaults.json`(`editor-keymap-defaults.json`, `editor-find-keymap-defaults.json` 등)과 앱 `command-registry.rs`·`command-dispatch.rs`의 같은 실행 경로를 씁니다(키맵·팔레트·컨텍스트 메뉴 공용).

## 11. 터미널과 미리보기

### 11.1 터미널 Hub

- 앱 `terminal_host.rs` `Hub`(528줄)가 세션과 프레임 큐를 소유합니다. 한도는 `application.rs` 상수(`MAX_TERMINAL_SESSIONS = 64`, `TERMINAL_FRAME_BYTES = 4 * 1024 * 1024`, `TERMINAL_QUEUE_COUNT = 64`, `TERMINAL_FRAME_VISITS = 1_000_000`, `TERMINAL_WRITER_BYTES = 1024 * 1024`)로, 생성자에서 `crate::terminal_tabs::Tabs::new(services.clone(), crate::terminal_host::Limits { sessions, core, frames: terminal_frames::Limits { .. }, writer: terminal_writer::Limits { .. } })`(411줄)에 넘깁니다. `Tabs::hub(&self) -> &Arc<Hub>`(앱 `terminal_tabs.rs` 86줄)로 Hub를 얻습니다(예: `background_tick`의 `terminal_views.flush_inputs(self.terminal_agent_state.terminals.hub(), ..)`).
- `terminal_agent_state.terminals`(`Arc<terminal_tabs::Tabs>`)를 host worker(`HostIntegrations.terminals`)와 원격 터미널 포트가 함께 씁니다.
- 그리기와 입력: `terminal_surface::Views`(1250줄, `terminal_agent_state.terminal_views`). 전역 키맵 라우팅(`capture_window_keymap_with_local` 3013줄, `route_window_keys` 3174줄, `chord_status` 3297줄)도 이 파일(약 9,000줄)에 있습니다. 탭 내용은 `AppSurfaces::show_terminal_tab`(terminal_agent 영역).
- 코어는 `native/taide-native-terminal`(vendored alacritty_terminal + vte, 02 문서 4.7절).

### 11.2 미리보기와 WKWebView

- 이미지·SVG·PDF·스프레드시트·프레젠테이션·HWP는 앱이 직접 디코드해 egui texture로 그립니다(앱 `preview*.rs`, `preview_state`의 캐시들).
- HTML·비디오·오디오는 vendored wry의 WKWebView(macOS 전용)입니다. 그리는 중 `AppSurfaces::show_preview_tab`(`domains/preview.rs` 754줄)이 탭 영역을 `web_placements`(`preview_web_view::Placement`)로 기록하고, `finish_frame`의 `show_preview_web_views`(626줄)가 `preview_state.web_views.sync(frame, context, &live, &web_placements, web_enabled)`로 native view를 그 위치에 맞춥니다. `web_enabled`는 키바인딩 편집기·팔레트·탭 닫기 확인·삭제 확인·`egui::Popup::is_any_open`·`egui::DragAndDrop::has_any_payload`·프로젝트 drag·탭 drag·파일 drop hover 중에는 `false`입니다. 실패하면 web view를 모두 비우고 렌더러 실패로 표시합니다. HTML은 같은 실행 파일을 `--html-preview-helper`로 띄운 helper 프로세스(`preview_web_helper`)와 `preview_web_host::Bridge`로 통신합니다.
- WebView는 egui 위에 겹치는 OS view라 egui 층 순서를 따르지 않습니다. overlay가 열리면 비활성화하는 이유입니다.

## 12. 설정·테마·locale, toast·modal·tooltip·장애 경계

### 12.1 설정·테마·locale

- 설정 값: `services.state.settings`(`taide_runtime::AppState`의 `RwLock`). 읽기는 `self.services.state.settings.read().<필드>`(예: `background_tick`의 `toast_position`, `editor_font_size`). 쓰기는 host 명령(`HostCommand::Settings(..)`)으로만 합니다. 변경은 `AppEvent::SettingsChanged`로 돌아와 `presentation` inbox와 `poll_settings_revisions`(`domains/settings.rs` 939줄)가 반영합니다.
- 테마·외형: `presentation_refresh::Inputs`(66줄)/`Appearances`(176줄)가 설정·테마에서 모든 영역의 색 묶음을 만들고, `domains/settings.rs` `apply_presentation_appearances`(1258줄)가 한 곳에서 각 `*_appearance`·`*_colors` 필드에 대입합니다. 테마 색을 추가할 때 고치는 곳은 `Appearances`(정의·`new`)와 이 함수입니다(seams 2.8절). 테마 미리보기는 `settings_state.preview_theme`이 `resolved_theme`보다 우선합니다. 테마·로케일 적재가 실패하면 `settings_state.presentation_failures`가 상태 오류 배너를 띄웁니다.
- 색: `presentation::color(theme, "<테마 키>") -> AppResult<Color32>`(UI `presentation.rs` 59줄, 앱 `presentation.rs`가 재수출). 없는 키는 오류이므로 합성 테마 fixture에도 키를 넣어야 합니다(예: QA `2026-11-02-native-lane-terminal-agent-terminal-core.md` 66줄).
- 문자열: `presentation::message(&self.locale, "<키>", &[("<인자>", "<값>")])`(UI `presentation.rs` 538줄, 앱이 `pub use`로 재수출). 카탈로그는 `crates/taide-locale/resources/locales/{en,ko,ja}.json`의 평면 키(각 1,115개, 키 정렬)이며 원본 TS와 같은 키를 씁니다. 사용자에게 보이는 문자열을 코드에 하드코딩하지 않습니다(lane workflow `RULES` 52줄).

### 12.2 toast·modal·tooltip·장애 경계

| 요소 | 위치 | 쓰는 법 |
| --- | --- | --- |
| toast | UI `toast.rs` `Toasts<A>`의 메서드 `success`, `error`, `warning`, `info`, `notify`, `ipc_error`, `settings_failed`, `snippet`과 자유 함수 `describe_error(locale, error) -> String`(1589줄, 앱 `toast.rs`가 `pub use`로 재수출); 앱 `toast-layers.rs` `ToastLayers`(메인 1개 + 보조 창별, `main()`·`auxiliary(viewport)`·`mount`·`retain`, `Deref`로 메인 `Toasts`); 앱 `toast.rs`(앱 action enum `Action`과 `error_once`, `save_failed`, `open_project_first`) | 메인 창 toast는 `self.toasts.main().info(title, Instant::now())`. IPC 오류는 `self.report(&error)`. action 버튼이 있는 toast는 `crate::toast::Action`을 쓰고 `run_toast_actions`가 처리 |
| modal | UI `modal.rs`(`Chrome`, `Presence`, `FocusReturn`, `Layer`, `takes_escape`, `trap_focus`) | 대화상자는 `show_overlays` 단계에서 overlay 창에만 그립니다 |
| tooltip | UI `tooltips.rs` `Provider`. 앱 `tooltips.rs`는 배포 빌드에서 이것을 `pub use`로 재수출하고, 검사 빌드(`cfg(test)`)에서는 같은 UI 소스를 `include!`로 다시 컴파일한 `compatibility` 모듈을 씁니다. 셸 쪽 진입은 `ShellSurfaces::tooltip` | 모든 셸 tooltip은 host의 한 owner를 거칩니다(400ms 지연, 키보드 focus로도 열림, UI `shell.rs` `tooltip`의 doc 주석). 규범: `docs/memory/tooltip-conventions.md` |
| 장애 경계 | UI `error-boundary.rs`(`show`, `show_instance`, `Region::{Welcome, ActivityBar, StatusBar, SidebarPanel}`) | 영역 render를 `catch_unwind`로 감싸 panic 시 그 영역 상태와 그 pass의 intent를 버리고 fallback·재시도를 그립니다. "snapshot 읽기 + 영역 소유 상태 + intent 출력"으로 바뀐 영역에만 적용합니다. 편집기·루트 경계는 배치 31 설계 대상(`docs/acknowledge/2026-10-26-native-autonomous-continuation.md` 단계 E 결정) |
| 상태 오류 배너 | 앱 `status-error-banner.rs` | 테마·로케일 적재 실패 시 상단 배너와 재시도(`presentation_refresh.retry()`) |

## 13. 새 기능을 끝까지 연결하는 방법

### 13.1 실제 예: OS 알림 진입점 `notify_event`(shell-91, 2026-11-02 shell lane, 메인에 병합됨)

원본: 앱 수준 provider가 에이전트 완료·명령 완료·LSP 설치 완료 이벤트를 받아 OS 알림을 보냅니다(`src/app/providers/native-notification-provider.tsx`, `src/entities/notification/`, 기능표 `settings-23` TS 근거). QA: `docs/quality-assurance/2026-11-02-native-lane-shell-providers.md` shell-91 절. 결정: OS 알림 진입점은 shell lane의 경로 하나(`notify_event`)이고 settings·git·explorer-search가 그것을 씁니다(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` 채택 절).

파일별 단계(데이터가 흐르는 순서):

1. 공용 이벤트 확인: 원하는 이벤트가 `taide_model::app_event::AppEvent`에 이미 있는지 봅니다(`TerminalCommandFinished`, `LspInstallProgress`, `AgentStateChanged`). 없으면 공용 crate를 바꿔야 하므로 범위를 먼저 확인합니다(1.3절).
2. inbox: 앱 `provider-events.rs`에 `pub(crate) enum Event`(16줄)와 `Inbox::record`(53줄)의 매핑(필요한 필드만 복사), 상한 `MAX_PROVIDER_EVENTS`(11줄), `take`(129줄)를 둡니다. 같은 파일 끝 `#[cfg(test)]`에 순수 단위 검사 2건(`provider_inbox는_알림_이벤트만_순서대로_기록하고_비운다`, `provider_inbox는_상한을_넘으면_닫힘을_남기고_가장_오래된_것을_버린다`)이 있습니다.
3. 연결: `application.rs` `new_with_overrides`에서 `let provider_events = Arc::new(crate::provider_events::Inbox::default());`(331줄)를 만들어 `PaintSink { providers: provider_events.clone(), .. }`(343줄)와 `shell_state`의 `provider_events`(643줄)에 넣고, `PaintSink::publish`에 `self.providers.record(&event);`(68줄)를 둡니다.
4. 소비: `background_tick`의 `self.poll(context);` 바로 뒤 `self.consume_provider_events(context, Instant::now());`(1101줄, 정의는 `domains/shell.rs` 1112줄). 종료 중이면 비운 이벤트를 버리고, 이벤트마다 `notify_agent_completions`, `notify_command_finished`, LSP 설치 분기, IDE 저장, 프로젝트 닫힘으로 나눕니다. 원본 규칙(명령 10초 미만 무시, 에이전트 10초 이상 일한 뒤 완료)은 앱 `native-notifications.rs`의 원본 이름 상수(`TASK_COMPLETION_NOTIFY_MIN_DURATION_MS = 10_000`, `AGENT_COMPLETION_NOTIFY_MIN_WORKING`, `NOTIFICATION_TEXT_MAX_CODE_POINTS = 200`, 각각 `/// Original ...` doc 주석)로 둡니다.
5. 단일 진입점: `domains/shell.rs` 1175줄

    ```rust
    pub(crate) fn notify_event(
        &mut self,
        context: &egui::Context,
        category: taide_model::notification::NotificationCategory,
        project: Option<&ProjectId>,
        fallback_title_key: &str,
        body: String,
    ) {
        use crate::native_notifications::{
            NOTIFICATION_TEXT_MAX_CODE_POINTS, project_title, truncate,
        };
        let title = project_title(
            &self.controller.snapshot().projects,
            project,
            presentation::message(&self.locale, fallback_title_key, &[]),
        );
        let request = crate::native_notifications::Request {
            category,
            title: truncate(&title, NOTIFICATION_TEXT_MAX_CODE_POINTS),
            body: truncate(&body, NOTIFICATION_TEXT_MAX_CODE_POINTS),
            any_window_focused: self.is_any_window_focused(context),
        };
        self.submit(HostCommand::Shell(Command::Notify(request)));
    }
    ```

    제목 규칙·200 코드 포인트 자르기·focus 입력을 한 곳에서 처리합니다. 다른 영역은 이 함수만 부릅니다.
6. host 명령: `domains/shell.rs`의 `pub enum Command`에 `Notify(crate::native_notifications::Request)`(77줄), `pub enum Reply`에 `NotificationSent(AppResult<taide_model::notification::NotificationDelivery>)`(106줄)를 추가했습니다. `host.rs`는 고치지 않습니다(영역 wrapper가 이미 있음).
7. worker 처리: 같은 파일 `dispatch`의 `Command::Notify(request)` arm(477줄)이 `services.tasks.run_blocking_result("native-notification", ..)` 안에서 종료 중이면 `Err(stopped())`, 아니면 `taide_runtime::notification_actions::notification_notify(&state, platform, category, &title, &body, || request.any_window_focused)`를 부릅니다. 설정(알림 켜짐·범주·창 focus 조건)은 공용 facade가 판단하고, 실제 전송은 `PlatformServices::send_notification`(`crates/taide-runtime/src/platform_services.rs`. native 구현은 앱 `bootstrap.rs`의 `/usr/bin/osascript -e NOTIFICATION_SCRIPT -- <title> <body>`)입니다.
8. 응답 반영: `apply_shell_reply`의 `Reply::NotificationSent(result)` arm(648줄)이 첫 `Delivered` 때만 `notification.enableHint` 안내 toast(`self.toasts.main().info(..)`)를 띄우고 `shell_state.notifications.has_announced_delivery`를 기록하며, 오류는 `log::warn!`만 합니다.
9. 문자열: `crates/taide-locale/resources/locales/{en,ko,ja}.json`의 `notification.*` 키(`notification.agentCompleted`, `notification.taskCompletedSucceeded`, `notification.lspInstallSucceeded`, `notification.enableHint` 등). 세 파일에 같은 키를 정렬 위치에 넣습니다.
10. 검사(실제 App): 앱 `application-notifications-tests.rs`(`domains/shell.rs` 끝에서 `notifications_tests`로 붙음). 핵심 구조:
    - 가짜 플랫폼 `FakePlatform`(26줄)이 `send_notification`을 메모리 `sent`에 기록합니다. 실제 OS 알림은 보내지 않습니다.
    - `launch_with_platform`(55줄)이 Tokio runtime·`TaskSupervisor`를 만들고 `NativeApplication::new_with_overrides(&eframe::CreationContext::_new_kittest(context.clone()), .., crate::application::Overrides { platform: Some(platform), ..Default::default() })`로 실제 앱을 띄웁니다.
    - `frame(application, &context, focused)`(84줄)가 `context.run_ui(..)` 안에서 `eframe::App::logic`·`ui`를 한 번 돌리고, `frame_until(.., "phase", |app| 조건)`(102줄)이 `DEADLINE`(10초) 안에 조건을 기다립니다. 실패 메시지에 `phase=..`와 `application.status`가 남습니다.
    - 검사 본문(`실제_app은_명령완료와_lsp설치를_os알림으로_보내고_포커스_범주_첫전달_안내를_지킨다`, 200줄): `let events = application.services.events.clone(); events.publish(command(Some(1), LONG_COMMAND_MS));`처럼 이벤트를 넣고 `platform.sent()`와 `toasts.inspection()`(안내 toast 수)을 단언합니다. 짧은 명령·focus된 창·진행 중 LSP 설치는 알림이 없어야 한다는 부정 분기도 같은 검사에 있습니다.
11. 실행: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- --test-threads=2 provider_events native_notifications domains::shell::` 그리고 앱 `cargo fmt .. -- --check`.
12. 기록: QA 문서(대상 ID, 원본 근거, 구현, 실행한 명령과 결과 수, 남은 범위), 기능표 해당 행(`shell-91`, 소비하는 `settings-23`)의 판정과 "범위/잔여 동작" 열.

다른 영역에서 이 진입점을 쓰는 실제 코드(메인 트리, `git-7`): 원본 `src/entities/git/git.query.ts` `useGitRemoteMutation`(214줄 부근)은 push/pull 성공 시 `category: 'gitRemote'`, 제목 `notificationTitle(i18next.t(titleKey.succeeded))`, 본문 `joinNotificationBody([성공 문구, 캐시된 현재 branch])`, 실패 시 `category: 'error'`, 본문 `[실패 문구, describeIpcError(error)]`로 알립니다. `domains/git.rs` `notify_git_remote`(1515줄)는 이를 그대로 옮겨 `NotificationCategory::GitRemote`/`Error`, 본문 `crate::native_notifications::join_body(&[Some(문구), detail])`(branch는 `self.git_state.cache.status_result(project)`에서, 오류는 `crate::toast::describe_error`)를 만든 뒤 `self.notify_event(context, category, Some(project), title_key, body)`를 부릅니다. 키는 카탈로그에 이미 있는 `notification.gitPushSucceeded`·`gitPushFailed`·`gitPullSucceeded`·`gitPullFailed`입니다.

### 13.2 두 번째 예: 원본 query cache를 대신하는 `git-cache`

원본 TanStack Query의 규칙(`src/app/query-client.ts`: staleTime 60초, 보존 10분, 재시도 없음)을 native로 옮긴 읽기 캐시입니다. 큰 화면 하나를 여러 읽기(상태·branch·log·diff·blame)로 그릴 때 이 형태를 따릅니다. 코드는 `native/taide-native-app/src/git-cache.rs`를 직접 읽습니다.

1. 무효화 기록: `git-cache.rs` `Changes::record`(55줄)가 `GitStatusChanged`·`GitRefsChanged`·`FsRescanRequired`는 저장소 전체(`invalidation.repository = true`), `FsChanged`는 경로 단위(`invalidation.paths`)로 프로젝트별 `Invalidation`에 모읍니다. `PaintSink`에 `git_cache` 필드로 연결합니다(7절).
2. 관찰: 그리는 코드가 필요한 읽기를 `git_state.cache.observe(&project, Key::Status, now)`처럼 표시합니다(`GitCache::observe` 119줄). `Key`(251줄)는 읽기 종류 enum(`Status`, `Remotes`, `Branches`, `Stashes`, `Log`, `Diff { path, staged, before_path }`, `Compare`, `Tags`, `CommitFiles { rev }`, `CommitDiff`, `FileLog`, `File`, `Gutter`, `CurrentUser`, `BlameLine { path, line }`, `BlameOverlay`, `ConflictSides`)이고, `trait CacheKey`의 `is_mutable`, `follows_worktree(paths)`, `stale_after`로 원본 규칙(`git.query.ts` `isGitQueryScopeMutable`, `ipc-sync-provider.tsx` `isGitWorktreeQueryForChangedPaths`)을 답합니다. 상수는 `STALE_AFTER = 60초`, `DISCARD_AFTER = 10분`입니다.
3. tick: `domains/git.rs` `tick_git_cache`(2530줄)가 `cache_changes.take()`의 무효화를 `cache.invalidate`로 적용하고, 열린 프로젝트만 남기고(`retain`), `cache.requests(now)`(프레임당 최대 `MAX_REQUESTS_PER_FRAME = 16`)를 `HostCommand::Git(Command::Read(request))`로 제출한 뒤 `cache.begin(&request)`. 제출이 실패하면 `request_repaint` 후 중단합니다.
4. worker: `Command::Read(request)` arm(518줄)이 `spawn_reply(worker, "native-git-read", async move { HostReply::Git(Reply::Read { request, result: read(&services, &request).await }) }, refused)`로 transient task에서 읽습니다. `refused`는 감독이 멈췄을 때 즉시 돌려줄 `Err(stopped())` 응답입니다.
5. 반영: `apply_git_reply`의 `Reply::Read { request, result }` → `cache.finish(&request, result, now)`. 요청의 `generation`이 현재와 다르면(읽는 동안 무효화) 값을 저장해도 다시 stale로 둡니다(`finish` 208·219줄). 늦은 응답이 새 값을 덮지 않게 하는 장치입니다.
6. 읽기: 화면은 `cache.status_result(&project)`, `cache.branches(..)`, `cache.diff(..)`, `cache.blame(..)` 같은 접근자로 마지막 응답을 읽습니다. 재연결 때는 `reconnect_git`이 `cache.cancel_fetches()`를 부릅니다.
7. 검사: `git-cache-tests.rs`(순수 상태기계), `application-git-*-tests.rs`(실제 App).

이 캐시를 다른 영역이 소비하는 실제 코드(메인 트리, `git-38` → `explorer-search-7`): `tick_git_cache` 안에서 `for project in self.explorer_search_state.git_decorations.mounted_projects() { self.git_state.cache.observe(&project, Key::Status, now); self.explorer_search_state.git_decorations.set(&project, self.git_state.cache.status_result(&project).and_then(Result::ok)); }`(`domains/git.rs` 2632~2636줄). 주입점 `GitDecorations::set(&mut self, project, status: Option<&GitStatus>)`(`None`이면 그 프로젝트 장식 제거, status 행이 같으면 다시 만들지 않음)과 `mounted_projects()`(Files 보기로 보이는 mount의 프로젝트)는 explorer-search lane이 먼저 만들어 메인에 병합됐습니다(앱 `explorer-git-decorations.rs` 149·150줄 doc 주석 "A status supplier calls [`GitDecorations::set`] for each of [`GitDecorations::mounted_projects`]", `set` 160줄 부근, `mounted_projects` 175줄). 한 영역이 주입점을 만들고 다른 영역이 공급하는 결정 D1 A(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`)의 실제 형태입니다.

### 13.3 일반 체크리스트(새 기능 하나)

1. 원본 확인: 해당 lane 사양(`docs/research/2026-10-28-lane-<영역>-spec.md`)의 ID 절과 원본 TS 경로·줄을 읽습니다. 원본 동작의 정상·빈 상태·오류·비활성·취소·수명(닫힘·종료·프로젝트 제거) 분기를 모두 적어 둡니다. 사양이 실제 코드와 다르면 실제 코드를 따르고 QA에 차이를 적습니다.
2. 실패 검사 먼저: 순수 로직은 해당 모듈 단위 검사, 화면은 UI egui 하네스(`--features native-host,inspection`), 앱 경로는 실제 App 검사(13.1절 10번 구조)로 실패를 확인합니다. 실제 OS 포트(알림·외부 열기·Keychain)는 `Overrides`나 주입 포트로 가짜를 씁니다.
3. 상태: 영역 `State`에 필드를 추가하고 생성자의 영역 리터럴 끝에 초기화식을 넣습니다(편집기 필드는 아직 루트).
4. host 작업이 필요하면 영역 `Command`/`Reply`·`dispatch` arm·`apply_<영역>_reply` arm을 추가합니다. 오래 걸리면 transient task(4.2절). 제출 실패(`submit`이 `false`)에서 상태를 되돌리고 다음 프레임에 다시 시도하게 합니다.
5. UI 입력이 필요하면 `intents/<영역>.rs`에 변형, 처리는 `apply_<영역>_intent` arm. 그리는 중 앱 질의가 필요하면 `surfaces/<영역>.rs`에 메서드와 두 구현자(앱 `AppSurfaces`, UI `tests/workbench.rs` `Surfaces`).
6. 백엔드 이벤트가 필요하면 inbox(7절).
7. 프레임마다 할 일은 기존 `tick_<영역>_*`/`show_<영역>_*` 안에 넣습니다. 새 호출 위치가 꼭 필요할 때만 프레임 흐름 함수에 자기 기존 호출 줄 바로 뒤 한 줄을 추가합니다.
8. 문자열은 locale 키, 색은 테마 키(`presentation::color(theme, "<키>")`), 의미 있는 숫자는 원본 이름을 딴 상수(`/// Original ...` 영어 doc 주석만 허용, 그 밖의 코드 주석 금지).
9. 실제 앱 경로 도달 확인: `main.rs` → `NativeApplication::new` → 프레임 흐름(8절) → 새 코드까지 호출 체인을 `rg`로 따라갑니다. 검사가 `new_with_overrides`로 실제 앱을 띄워 그 경로를 지나야 complete 근거가 됩니다.
10. 검증(02 문서 8.8절), QA 문서, 기능표 행 갱신. 상단 합계 표는 건드리지 않습니다.

## 14. 공용 파일의 영역 블록 규칙

여러 lane이 동시에 고치는 공용 파일은 "자기 영역 블록 안에서 추가만" 합니다(seams 문서 1.4절). 목적은 병합 충돌을 줄이는 것입니다.

| 공용 파일·위치 | 영역 블록 | 할 일 |
| --- | --- | --- |
| `application.rs` `pub struct NativeApplication` | `<영역>_state: domains::<영역>::State` 한 줄 | 새 상태는 영역 `State`에. 루트에 추가하지 않음(편집기 예외, 3.2절) |
| `application.rs` `new_with_overrides`의 `Self { .. }` 리터럴 | `<영역>_state: domains::<영역>::State { .. }` 리터럴 | 자기 리터럴 끝에 초기화식 추가. 리터럴은 적힌 순서로 평가되므로 `?`·로그·I/O가 있는 식의 상대 순서를 바꾸지 않음(seams 5.9절 2번) |
| `application.rs` `poll()` 응답 라우팅, `show_window` intent 라우팅 | `HostReply::<영역>(..)`, `ShellIntent::<영역>(..)` 한 줄씩 | 손대지 않음. 영역 `Reply`·`Intent` 변형과 영역 함수 arm만 추가(라우팅에 인자가 더 필요하면 그 한 줄만 고침) |
| `application.rs` `background_tick`·`frame_ui`·`show_window`·`show_overlays`·`finish_frame` | `self.tick_<영역>_*()`/`self.show_<영역>_*()` 호출 줄 | 기존 영역 함수 안에 추가. 새 줄은 자기 기존 호출 바로 뒤에만 |
| `application.rs` 재연결 블록·`submit()`·`poll_lsp` | `self.reconnect_<영역>()`, `intercept_*`/`prepare_editor_submission`, `apply_explorer_search_lsp_reply` 라우팅 | 자기 영역 함수 안에 추가(seams 2.8절) |
| `application.rs` `AppSurfaces`와 그 생성 리터럴 | `<영역>_output`·영역 입력 묶음 한 줄 | 새 요청은 자기 `FrameOutput` 필드로 |
| `application.rs` `PaintSink` | 필드·`publish`의 `record` 줄 | 자기 inbox 한 줄씩 추가. 2026-11-03 병합에서 `PaintSink.providers`가 의미 충돌을 냈음(`docs/PROCESS.md` 30줄) |
| `host.rs` `HostCommand`/`HostReply`/`dispatch` | 맨 위 wrapper·라우팅 블록 | 손대지 않음 |
| UI `commands.rs` `ShellIntent` | 루트 끝 wrapper 블록 | 손대지 않음. shell은 `intents/shell.rs`, 다른 영역은 `intents/<영역>.rs` |
| UI `shell.rs` `ShellSurfaces` | supertrait 목록 | shell만 루트 메서드 추가. 다른 영역은 `surfaces/<영역>.rs` |
| `lib.rs` 모듈 목록, `domains/mod.rs`, `intents/mod.rs`, `surfaces/mod.rs` | 알파벳 순(rustfmt `reorder_modules`) | 정렬 위치에 한 줄 |
| `crates/taide-locale/resources/locales/{en,ko,ja}.json` | 키 접두사 묶음(평면, 정렬) | 세 파일에 같은 키를 정렬 위치에. 접두사 소유: `project.*`·`projectGroup.*`·`welcome.*`·`zen.*`·`window.*`·`systemUsage.*` → shell, `palette.*`·`keybindings.*`·`task*` → commands, `editor.*` → editor, `explorer.*`·`search.*`·`problems.*`·`outline.*` → explorer_search, `git.*` → git, `terminal.*`·`agent.*`·`ide.*` → terminal_agent, `settings.*`·`theme.*`·`snippet*` → settings, `preview.*` → preview. 목록에 없는 접두사(예: `notification.*`, `app.*`)는 처음 쓰는 lane이 seams 문서 1.4절에 한 줄 추가 |
| `docs/quality-assurance/2026-10-09-native-function-matrix.md` | 자기 영역 표의 행 | 이번 묶음 ID 행만 갱신. 상단 합계 표는 손대지 않음 |

영역 블록 밖에서 실제로 충돌한 파일(2026-11-04 `f3a5b5d0` 병합에서 해결한 8개, 근거 `docs/quality-assurance/2026-11-03-native-git-lane-merge.md` 1절)은 기능표, `application-explorer-views-tests.rs`, `command-dispatch.rs`, `domains/explorer_search.rs`, `explorer.rs`, `navigation-icons.rs`(아이콘 enum), `shell_keymap.rs`, UI `command-registry.rs`(명령 등록·키 action 목록)입니다. 이 파일들은 match arm·목록 끝에만 추가하고, 기존 항목 순서를 바꾸거나 재포맷하지 않습니다(`docs/research/2026-10-28-lane-commands-ui-spec.md` 3절 "묶음별 주의" 1번).

다른 영역 상태를 다루는 규칙(seams 1.5절):

1. 다른 영역 상태는 읽기만 합니다(예: shell의 `overlay_viewport`가 `self.commands_state.palette.is_open()`을 읽음).
2. 바꿔야 하면 소유 영역의 `pub(crate)` 메서드를 부릅니다(예: shell intent가 `self.request_tab_close(tab)`(editor)를, commands가 `self.request_editor_formatting(..)`(editor)을, git이 `self.notify_event(..)`(shell)을 부름). 13.2절 마지막 예처럼 다른 영역이 만든 주입점(`GitDecorations::set`)에 값을 넣는 것은 그 영역이 공개한 API를 쓰는 것이므로 허용됩니다.
3. 빌림 충돌은 필드 경로 단위로 나눠 피합니다(`let locale = &self.locale; let icons = &mut self.explorer_search_state.explorer_icons;`).
4. 메서드 경계를 넘어 여러 필드를 동시에 `&mut`로 넘겨야 하면 영역 파일에 인자 묶음 struct를 정의해 필드 참조를 담습니다(`AppSurfaces`, `ExplorerInputs<'a>`와 같은 형태).
