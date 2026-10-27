# 네이티브 앱 메뉴(macOS 메뉴바)

## 대상

- shell-9: 네이티브 앱 메뉴입니다. `TAIDE`·`File`·`Edit`·`Window` 메뉴바, `File > Open Recent`(최근 프로젝트·`Clear Recent`), `Quit`, 표준 편집·창 항목을 포함합니다.
- 원본:
    - `src-tauri/src/domain/window/commands.rs` 119-165 `build_app_menu`, 167-187 `refresh_app_menu`, 316-331 `request_quit`
    - `src-tauri/src/domain/window/menu.rs` 1-243(메뉴 id·`menu_action`·`recent_menu_entries`·`recent_menu_label`·`build_open_recent_items`·`refresh_recent_menu`)
    - `src-tauri/src/lib.rs` 296-320(`recent_menu_projects`·`menu_label`), 383-438(`dispatch_menu_action`·`recent_project_root`), 453-500(`listen_for_app_menu_refresh`), 876(`on_menu_event`), 973(`set_menu`)
    - `src/entities/project/project.query.ts` 72-96 `useRecentProjectsClearedNotice`(`src/app/providers/ipc-sync-provider.tsx` 295에서 항상 마운트)
    - 설계 기록: `docs/features/window-chrome.md` 7절

## 원본 동작

- 메뉴바 구성(`build_app_menu`):
    - `TAIDE`: About, 구분선, Services, 구분선, Hide, Hide Others, Show All, 구분선, `Quit`(사용자 정의 항목, `CmdOrCtrl+Q`)
    - `File`: `Open Recent ▸` 하나
    - `Edit`: Undo, Redo, 구분선, Cut, Copy, Paste, Select All(모두 OS 표준 항목, 포커스된 입력으로 감)
    - `Window`: Minimize, Zoom, 구분선, Toggle Full Screen
    - 번역 키는 `menu.file`·`menu.openRecent`·`menu.noRecentProjects`·`menu.clearRecent` 4개뿐이고 나머지 문자열은 영어 고정입니다(`window-chrome.md` 7.4).
- `Open Recent`(`build_open_recent_items`):
    - `list_recent_projects`(=`last_opened_at` 내림차순) 상위 `RECENT_PROJECT_MENU_LIMIT`(10)개입니다. 열린 프로젝트도 거르지 않습니다.
    - 라벨은 trim한 `display.label`이 비어 있지 않으면 그것, 아니면 폴더명입니다.
    - `root_missing`이면 목록에 남되 비활성입니다.
    - 비어 있으면 비활성 `No Recent Projects` 한 줄, 그 뒤 구분선, `Clear Recent`(목록이 비면 비활성)입니다.
- 클릭(`menu_action` → `dispatch_menu_action`):
    - id 표: `taide-quit`→Quit, `taide-clear-recent`→ClearRecent, `taide-recent:<projectId>`→OpenRecent, 나머지(제목·안내 줄·표준 항목)는 무시합니다.
    - OpenRecent는 클릭 시점에 `list_recent_projects`를 다시 읽어 root를 찾고, 없으면 로그만 남깁니다. 있으면 `project_open(root)`이고 실패도 로그만 남깁니다.
    - ClearRecent는 `project_forget_recent`이고 실패는 로그만 남깁니다.
    - Quit은 `request_quit`이 메인 창을 닫아 `CloseRequested` hot-exit flush 핸드셰이크를 그대로 거칩니다(`terminate:`를 쓰지 않는 이유가 원본 문서 주석에 있습니다).
- 갱신(`listen_for_app_menu_refresh`): `project:list-changed`·`project:activated`마다 `Open Recent`를 다시 읽어 교체하고, 언어가 실제로 바뀌면 전체 메뉴를 다시 만듭니다.
- `project:recent-cleared`의 `skippedWithDrafts > 0`이면 모든 창이 `project.clearRecentKeptDrafts` info toast를 띄웁니다(메뉴뿐 아니라 어떤 진입점이든).

## 구현

- 의존성: `native/taide-native-app/Cargo.toml`의 macOS 대상 의존성에 `muda = { version = "=0.21.0", default-features = false }`를 추가했습니다(사용자 결정 `docs/acknowledge/2026-10-06-native-transition-decisions.md` 'native 메뉴바').
    - `Cargo.lock`은 `--offline`으로 `muda 0.21.0`·`keyboard-types 0.8.3` 두 항목과 앱 의존 한 줄만 추가됐습니다. 기존 crates.io 항목 버전 변경은 없습니다. remote-web·ui의 `Cargo.lock`은 바뀌지 않았습니다.
    - `default-features = false`는 Linux 전용 `gtk3` 기능을 끕니다. 메뉴바는 릴리스 대상인 macOS에만 설치합니다(`docs/architecture.md`의 "릴리스·CI 타깃이 macOS 단독").
- 순수 모델 `native/taide-native-app/src/app-menu.rs`:
    - 원본과 같은 id 문자열, `action(id) → Action`(`Quit`·`OpenRecent`·`ClearRecent`·`Edit`·`Ignored`), `recent_id`/역파싱
    - `Model::new(locale, projects)`가 `File`·`Open Recent` 제목과 최근 행(상한 10, 라벨 규칙, 비활성, 안내 줄, 구분선, `Clear Recent`)을 만들고, `Model::bar()`가 원본 순서의 메뉴바 전체 명세를 냅니다.
    - `Changes`가 `PaintSink`에서 `ProjectListChanged`·`ProjectActivated`를 revision으로 세고 `ProjectRecentCleared`의 초안 보존 수를 모읍니다.
    - `State`가 revision이 바뀔 때마다 기존 runtime `project_list_recent`를 transient 작업으로 다시 읽고, 모델이 바뀐 경우에만 메뉴바에 다시 그립니다. 읽기 실패는 원본처럼 빈 목록입니다.
- macOS 메뉴바 `native/taide-native-app/src/app-menu-bar.rs`:
    - `Model::bar()`를 `muda` `Menu`/`Submenu`/`MenuItem`/`PredefinedMenuItem`으로 만들고 `init_for_nsapp`으로 설치합니다. 앱 생성은 winit의 `applicationDidFinishLaunching` 기본 메뉴 설치 뒤(`resumed`)이므로 기본 메뉴(`terminate:` Quit 포함)를 대체합니다.
    - 갱신은 `File`/`Open Recent` 제목 `set_text`와 `Open Recent` 행 교체입니다. `muda`가 `&`를 mnemonic으로 지우므로 프로젝트 이름의 `&`는 `&&`로 보존합니다.
    - 클릭은 `MenuEvent::set_event_handler`가 id를 채널로 보내고 `request_repaint`로 루프를 깨웁니다.
- 앱 연결:
    - `main.rs`가 macOS에서 `NativeApplication::with_app_menu(&cc.egui_ctx)`로 설치합니다.
    - `background_tick`이 메뉴 행동을 처리합니다.
        - Quit: `ViewportCommand::Close`를 ROOT로 보내 기존 `close_requested` → `CancelClose` → hot-exit 종료 핸드셰이크를 그대로 탑니다.
        - OpenRecent·ClearRecent: 새 `HostCommand::OpenRecentProject(ProjectId)`·`HostCommand::ClearRecentProjects`입니다. 호스트에서 최근 기록을 다시 읽어 root를 찾고 기존 열기 경로(`validate_open_root`·view state flush·`NativeProjects::open`)를 쓰며, 실패는 원본처럼 로그만 남깁니다. 지우기는 runtime `project_forget_recent`입니다.
        - 종료 중에는 OpenRecent·ClearRecent를 무시합니다.
    - 초안 보존 안내: `PaintSink`가 모은 `skippedWithDrafts`마다 `project.clearRecentKeptDrafts` info toast를 띄웁니다(원격 웹 등 어떤 진입점이든).
- 편집 메뉴(결정):
    - winit의 NSView는 `copy:`·`paste:` 등 편집 selector를 구현하지 않습니다. `muda`는 `setAutoenablesItems(false)`로 항목을 항상 활성으로 두므로, 원본처럼 표준 항목을 쓰면 ⌘C·⌘V 등 key equivalent를 메뉴가 먼저 먹고 아무 응답자도 처리하지 않습니다. 그러면 egui 키 입력(탐색기의 빈 클립보드 붙여넣기 보존, 터미널 raw input, `⌘K ⌘C` 같은 chord)이 깨집니다.
    - 그래서 편집 항목은 단축키를 달지 않은 사용자 정의 항목으로 두고, 키보드 단축키는 지금처럼 egui로 갑니다. 원본에서도 WKWebView가 key equivalent를 페이지에 먼저 주므로 키보드는 포커스 입력이 먼저 받습니다.
    - 클릭은 포커스된 viewport(메인 또는 포커스된 보조 창)에 보냅니다. Cut·Copy·Paste는 egui의 `ViewportCommand::RequestCut/RequestCopy/RequestPaste`("시스템 단축키와 같음")입니다. Undo·Redo·Select All은 egui에 의미 이벤트가 없으므로 그 단축키(⌘Z·⇧⌘Z·⌘A)의 press·release를 한 번 넣습니다. 메인은 `raw_input_hook`(터미널 raw input 앞), 보조 창은 그 창 pass 시작에서 넣고, 그 pass에 보이지 않은 보조 창 대상은 버립니다.
    - 보조 창 대상 Cut·Copy·Paste가 실제 이벤트가 되려면 아래 "리뷰 수정"의 vendored eframe 수정이 필요합니다(보조 창은 immediate viewport라 원본 eframe은 요청을 버립니다).
- 메뉴 라벨 언어: 앱이 쓰는 해석된 locale(`self.locale`)로 그립니다. 원본은 Rust가 OS 언어를 몰라 `system`이면 영어로 그렸고 이를 알려진 제약·후속으로 적었습니다(`window-chrome.md` 7.4). native는 OS 언어를 이미 해석하므로 그 결함을 재현하지 않습니다. 언어가 바뀌면 다음 tick의 모델 비교로 제목과 행을 다시 그립니다.

## 검증

- `Cargo.lock` 갱신: `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --offline --target-dir experiments/native-shell-spike/target`(lock 갱신을 위해 이 1회만 `--locked` 없이)는 exit 0입니다. diff는 `muda 0.21.0`·`keyboard-types 0.8.3` 추가와 앱 의존 `muda` 한 줄뿐입니다.
- `cargo test … --lib app_menu`(`--locked --offline`)는 8통과입니다.
    - 단위 7건(`app-menu-tests.rs`):
        - id 표(Quit·ClearRecent·OpenRecent 왕복·편집 6종, 제목·안내 줄·접두사만·빈 id 무시)
        - 최근 행 상한 10·원래 순서·trim 라벨·빈 라벨 폴더명·root 누락 비활성·구분선·`Clear Recent`
        - 빈 목록의 비활성 안내·비활성 지우기와 ko 번역
        - 메뉴바 전체 명세(원본 순서, Quit만 ⌘Q, 편집 항목 단축키 없음, `&` 보존)
        - 편집 입력 대응과 chord 1회 주입·다른 viewport 무시·보조 창 대상 폐기
        - `Changes`의 revision·초안 보존 수 1회 전달
        - 실제 `AppState`·임시 project 기록으로 연결 메뉴바의 첫 그리기, 같은 목록 재그리기 없음, `ProjectListChanged` 뒤 재조회·재그리기, 클릭 id → 행동, 작업 회수
    - 실제 App 1건(`application-app-menu-tests.rs`): 임시 data dir의 닫힌 기록 3개(하나는 `buffers/` 초안 보유)로 `NativeApplication`을 띄우고 기록용 메뉴바를 연결했습니다.
        - 첫 모델이 디스크 최근 3개와 같음
        - `taide-recent:<id>` → 그 id로 프로젝트가 열림
        - `taide-clear-recent` → 닫힌 일반 기록 삭제, 초안 기록·열린 기록 유지, `Kept 1 recent project(s)…` info toast, 메뉴 재그리기
        - Copy·Paste → ROOT 출력에 `RequestCopy`·`RequestPaste`
        - Undo·Redo·Select All → 다음 `raw_input_hook` 입력에 ⌘Z·⇧⌘Z·⌘A press가 한 번만 들어감
        - Quit → ROOT `Close` 출력, 이어진 `ViewportEvent::Close`에 `CancelClose`와 종료 핸드셰이크 완료(`is_exit_ready`)
- 보조 창 pass에 주입 지점을 넣었으므로 `--lib auxiliary_window` 14통과, `--lib window_state` 7통과를 함께 확인했습니다.
- App `--all-targets` check는 exit 0입니다. 경고는 기존 것(`editor-code-actions.rs` `is_applying`, `zen.rs` 543, `application-inlay-hints-tests.rs` 234, vendored wry-preview)뿐입니다.
- 동결 remote-web `--target wasm32-unknown-unknown --lib` check는 exit 0입니다(기존 dead_code 경고만).
- `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check`에서 이번 파일은 차이가 없습니다. 남은 차이 3개 파일(`application.rs`의 `zen_hint_colors` 들여쓰기·테스트 모듈 순서, `lib.rs` 모듈 순서, `experiments/lsp-coordinator-spike/src/bin/mock-server.rs`)은 HEAD에 이미 있던 것이라 손대지 않았습니다.
- 테스트는 구현과 함께 작성했습니다. 구현을 끄고 실패를 재현하는 편집은 금지라 구현 전 실패 실행 기록은 없습니다(구현 전에는 메뉴 모듈이 없어 테스트가 컴파일되지 않는 상태였습니다).

## 리뷰 수정(보조 창 Edit > Cut·Copy·Paste)

### 차단 항목: 보조 창이 포커스일 때 Cut·Copy·Paste 메뉴가 무동작

- 확인한 원인(실제 결함으로 판정했습니다):
    - `run_app_menu_action`(`native/taide-native-app/src/application.rs` 7173-7181)은 포커스된 보조 창 id로 `RequestCut/RequestCopy/RequestPaste`를 보냅니다.
    - egui는 마지막(root) pass 끝에서 모든 viewport의 명령을 root 출력으로 모읍니다(`native/taide-native-app/vendor/egui-input/src/context.rs` 3045-3057의 `is_last`). eframe은 root 그리기 뒤 `handle_viewport_output` → `Viewport::process_commands`(vendored `wgpu_integration.rs` 1046-1064)로 각 viewport의 `actions_requested`에 쌓습니다.
    - 이를 `egui::Event::Cut/Copy/Paste`로 바꾸는 drain은 root·deferred 그리기 경로(수정 전 834-857행)에만 있었습니다. 보조 창을 그리는 `render_immediate_viewport`(1162행~)는 `actions_requested`를 읽지 않아 요청이 쌓이기만 했습니다. immediate viewport는 자기 창의 redraw도 부모 그리기로 넘기므로(647-661행) root 경로의 drain을 타지 않습니다.
- 수정(vendored eframe, 근본 원인 위치):
    - `experiments/native-shell-spike/vendor/eframe/src/native/wgpu_integration.rs`에 `push_requested_edits`를 두고 root 경로의 기존 drain과 `render_immediate_viewport`가 함께 씁니다(같은 로직 2회 사용).
    - root 경로는 기존처럼 Screenshot을 먼저 꺼낸 뒤 부르므로 동작이 같습니다. immediate 경로는 `take_egui_input` 바로 앞에서 불러, 앞 프레임에 쌓인 요청이 그 보조 창의 이번 입력(raw 이벤트 포함)으로 들어갑니다.
    - Paste는 원본 drain과 같이 클립보드 텍스트의 `\r\n`을 `\n`으로 바꾸고 비어 있지 않을 때만 넣습니다. immediate viewport의 Screenshot 요청은 수정 전처럼 손대지 않습니다.
    - 앱은 wgpu backend만 쓰므로 glow backend는 바꾸지 않았습니다. 기록은 `experiments/native-shell-spike/vendor/eframe/TAIDE-CHANGES.md` "2026-10-27 immediate viewport의 편집 요청"입니다.
    - 앱 쪽에서 보조 창 pass에 이벤트를 직접 만들어 넣는 대안은 Paste에 클립보드 읽기를 앱이 따로 해야 하고 eframe의 결함을 덮는 우회라 택하지 않았습니다.
- 보조 창 App 검사(`application-auxiliary-window-tests.rs`의 `실제_app_메뉴_편집은_포커스된_보조_창_편집기로만_간다`):
    - 메인(main.txt)과 보조 창(notes.txt) 각각 편집기가 있는 실제 `NativeApplication`에 기록용 메뉴바를 연결하고, 메인 입력은 `focused: false`, 보조 창은 포커스·편집기 focus로 둡니다. 보조 창 편집기에서 `synthetic`을 선택합니다.
    - Copy·Paste·Cut: 메뉴 id → root 출력의 보조 창 viewport 명령에 `RequestCopy`·`RequestPaste`·`RequestCut`이 있고 메인 viewport 명령에는 없음을 단언합니다. 이어서 수정된 eframe drain이 그 창 입력에 넣는 `Event::Copy`·`Event::Paste(text)`·`Event::Cut`을 보조 창 RawInput으로 넣어, 보조 창 출력의 `CopyText("synthetic")`, 본문 `pasted\n`, Cut의 `CopyText("pasted\n")`와 빈 본문을 확인합니다.
    - Undo·Redo·Select All: 메뉴 id → 같은 프레임 보조 창 pass 시작 주입(`show_auxiliary_windows`의 `app_menu.inject`)만으로 보조 창 편집기 본문이 `synthetic\n` → `pasted\n`으로 되돌고 다시 바뀌며, 선택이 문서 전체가 됨을 확인합니다. 주입 이벤트는 `input.raw.events`에 없지만 편집기가 포커스 대체 규칙으로 처리함을 실제로 확인했습니다(리뷰 비차단 2).
    - 끝에 메인 편집기 본문이 그대로임을 확인합니다.
    - 테스트 harness는 eframe의 immediate 렌더러를 흉내내므로 `ActionRequested` → 이벤트 변환 자체는 vendored 코드가 아니라 harness 입력으로 대신합니다. winit 창·event loop 없이 `render_immediate_viewport`를 실행할 수 없어(GUI 실행 금지) 이 vendor 경로의 headless 실패 재현·통과 기록은 없습니다. 근거는 위 코드 경로와 컴파일입니다. 실제 보조 창에서 메뉴 Copy가 클립보드를 바꾸는지는 실기(배치 31)입니다.

### 비차단 반영

- 대응표 shell-9·이 문서의 검사 범위 표현을 실제와 맞췄습니다. 메인 App 검사는 Copy·Paste 명령 출력과 Undo·Redo·Select All 입력 주입 5종이고, Cut은 단위 검사(id·입력 대응)와 위 보조 창 App 검사에서만 확인합니다. 메인 Copy·Paste 이벤트가 편집기까지 닿는 것은 root 경로가 원본 eframe drain이라 코드상 근거만 있고 실기(배치 31)로 남깁니다.
- locale 변경 재그리기: `app-menu-tests.rs`의 연결 메뉴바 검사에 en → ko tick 한 번으로 메뉴가 ko 모델로 한 번 더 그려지는 단언을 추가했습니다.

### 검증(리뷰 수정)

- 모든 cargo 명령은 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다.
- App `--all-targets` check(vendored eframe 포함 컴파일)는 exit 0입니다. 경고는 기존 것(`editor-code-actions.rs` `is_applying`, `zen.rs` 543, `application-inlay-hints-tests.rs` 234, vendored wry-preview)뿐입니다.
- `cargo test … --lib 실제_app_메뉴_편집은_포커스된_보조_창`: 1통과(69.05초)입니다.
- `cargo test … --lib auxiliary_window`: 15/15 통과(389.96초)입니다. 새 검사와 기존 보조 창 실제 App 검사 14건을 포함합니다.
- `cargo test … --lib app_menu`: 단위 7건은 통과입니다(locale 재그리기 단언을 넣은 연결 메뉴바 검사 포함). 메인 App 검사 1건은 `phase=clear recent`의 10초 단계 기한 초과로 3회 연속 실패했습니다(42.82초 병렬, 70.84초·52.93초 단독).
    - 이 검사가 실행하는 코드는 이번 수정과 겹치지 않습니다(이 파일의 변경은 `Recording`을 `pub(super)`로 연 것뿐이고, vendored eframe 러너는 테스트에서 쓰지 않습니다).
    - 같은 환경에서 보조 창 15건이 389.96초로, 앞 단계 기록(`2026-10-27-native-window-session-restore.md`의 17건 107.89초)보다 약 3.6배 느렸습니다. `frame_until`은 프레임 뒤 기한을 먼저 단언하므로 느린 프레임에서 준비 직전 프레임이 기한을 넘기면 실패합니다.
    - 진단으로 단계 기한만 90초로 잠시 올려 1회 실행하자 통과(54.66초)했고, 기한은 곧바로 10초로 되돌렸습니다. 기능 결함이 아니라 현재 환경의 프레임 지연으로 판단합니다. 배치 끝 전체 실행에서 다시 확인이 필요합니다.
- 동결 remote-web `--target wasm32-unknown-unknown --lib` check는 exit 0입니다(기존 dead_code 경고만).
- `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check`에서 이번에 바꾼 파일은 차이가 없습니다. 남은 차이(`application.rs` 958행 들여쓰기·테스트 모듈 순서, `lib.rs` 모듈 순서, `experiments/lsp-coordinator-spike/src/bin/mock-server.rs`)는 이전부터 있던 것입니다. vendored eframe은 별도 Cargo.lock이 없어 cargo fmt를 돌리지 않았고 rustfmt 형태로 직접 맞췄습니다.

### 비차단 미반영(후속 후보)

- `State::tick`이 매 프레임 `Model::new`로 비교 모델을 만드는 할당은 정확성 문제가 아니라 이번에 바꾸지 않았습니다.
- `HostCommand::OpenRecentProject`의 열기 순서(`validate_open_root`·view state flush·`NativeProjects::open`)는 `OpenProject`·`OpenProjectByPath`·`open_dropped_paths`와 겹칩니다. 이번 범위 밖의 기존 경로까지 바꾸는 정리라 후속 공통화 후보로 둡니다.
- `TAIDE`·`Edit`·`Window`·`Quit`·편집 항목 문자열은 원본도 영어 고정이고 카탈로그 키가 없습니다(`window-chrome.md` 7.4). 카탈로그 키 없는 사용자 문자열로 기록만 남깁니다.

## 남은 범위

- 실제 NSMenu 생성(`muda` 항목 구성)·`init_for_nsapp`·`MenuEvent` 처리기 경로는 메인 스레드와 OS 메뉴바가 필요해 자동 검사가 없습니다. 메뉴 표시·클릭·⌘Q·Hide·Services·Window 항목 동작은 실기(배치 31)입니다.
- 보조 창 메뉴 Cut·Copy·Paste의 vendored eframe immediate 경로와 메인 Copy·Paste 이벤트의 편집기 도착은 실제 창에서만 확인할 수 있어 실기(배치 31)입니다.
- 편집 항목에 단축키 표시(⌘Z·⇧⌘Z·⌘X·⌘C·⌘V·⌘A)가 없습니다. 위 결정의 대가이며, 표시하려면 메뉴가 키를 먼저 받으므로 egui 키 입력 경로 전체를 메뉴 이벤트로 다시 만들어야 합니다.
- 편집 메뉴의 Paste는 egui `RequestPaste`라 클립보드 텍스트가 비면 아무것도 넣지 않습니다. 키보드 ⌘V의 빈 클립보드 `Key::Paste` 보존(탐색기 파일 붙여넣기)은 메뉴 클릭에는 없습니다. 원본에서도 메뉴 Paste는 DOM 편집 `paste:`라 탐색기 파일 붙여넣기를 부르지 않습니다.
- Undo·Redo·Select All 메뉴 클릭은 그 단축키 입력이므로, 포커스가 텍스트 입력이 아닐 때는 같은 단축키에 묶인 keymap 동작이 실행됩니다(원본은 WebKit 편집 명령이라 이 경우 아무것도 하지 않을 수 있습니다).
- 부팅 직후 첫 최근 목록 조회가 끝나기 전 잠깐 `Open Recent`가 비어 있습니다(원본은 setup에서 동기 조회).
- Windows/Linux 메뉴바는 설치하지 않습니다(릴리스 대상 macOS 단독). 원본의 Dock 메뉴 미구현은 그대로입니다.
