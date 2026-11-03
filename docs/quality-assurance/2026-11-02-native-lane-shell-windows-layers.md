# lane shell 묶음 2: 창·viewport·표시 층

> 작성일: 2026-11-02 · lane: `shell`(worktree `TAIDE-lanes/shell`, 브랜치 `lane/shell`) · 사양: `docs/research/2026-10-28-lane-shell-spec.md` 2.7~2.12, 3.2 · 결정: `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안: 결정 2 A, 3 A, 5 A, 6 A)

## 대상

| ID | 기능 | 결과 |
| --- | --- | --- |
| shell-94 | 테마·로케일 오류 배너 | complete(픽셀은 실기) |
| shell-22 | OS 파일/폴더 drop | complete(실제 Finder drag·WKWebView 대상 전환은 실기) |
| shell-9 | 네이티브 앱 메뉴 | complete(NSMenu 실기, 편집 항목 단축키 표시 차이 유지) |
| shell-3 | 창 위치·크기 복원 | partial(수정 절: 창 생성 뒤 치명 오류의 종료 코드 경로에 자동 검사 없음. 실제 zoom·모니터·eframe 치명 오류 경로는 실기) |
| shell-98 | provider 계층(보조 창 toast) | complete(픽셀·지연은 실기) |
| shell-5 | 보조 창 셸 | partial. 이 묶음 4건 중 "탭 drag 중 모든 창 web view 숨김"만 완료. WKWebView 배치·터미널 raw input·theme preview는 남김(아래 남은 범위) |

## 원본 동작

원본 근거는 사양 2.7~2.12에 줄 번호까지 정리돼 있어 그대로 따랐습니다.

## 구현

### shell-94 테마·로케일 시작 대체와 배너 층

- `status_error_banner::startup`: 생성자의 `Inputs::load` 결과에서 테마 로드 또는 `Appearances::new`가 실패하면 `log::warn` 뒤 번들 `taide-dark`(`DEFAULT_THEME_ID`)와 그 `Appearances`로, 로케일이 실패하면 번들 `en`으로 계속 띄우고 `Failures`에 표시합니다. 번들 대체까지 실패할 때만 생성 오류입니다. `Refresh`는 그대로라 배너 재시도가 같은 입력을 다시 요청하고 성공하면 기존 `Presentation` 경로가 적용·배너 해제합니다.
- 배너 층: `status_error_banner::layer_ids(failures)`와 `file_drop::overlay_layer()`를 두고, drop overlay를 그린 frame에 보이는 배너마다 `context.set_sublayer(overlay, banner)`를 부릅니다(메인 `show_window`). 순서는 셸 본문 < overlay < 배너입니다.

### shell-22

- shell-94의 sublayer 지정 외 코드 변경 없음. 결정 5 A: Zen 안내·편집기 popup이 overlay 위인 기존 순서 차이를 유지합니다.

### shell-9

- `initial_app_menu_model`: 설치 때 `project_list_recent`를 바로 읽어 `Model::new(locale, &recent)`를 만들고(실패면 빈 목록 + `log::warn`), `with_app_menu`가 그 모델로 설치합니다. `app_menu::State::connect_installed`가 설치한 모델을 `drawn`으로 기록해 첫 tick 재조회가 같은 목록이면 다시 그리지 않습니다.

### shell-3

- zoom: `window_state::is_zoomed(frame)`가 macOS에서 `eframe::Frame`의 AppKit 핸들 `ns_view` → `[view window]` → `[window isZoomed]`(`objc2::msg_send!`, `wry::raw_window_handle` 재수출)를 읽고, 핸들이 없거나 다른 OS면 `None`입니다. `App::logic`이 매 frame 이 값을 `shell_state.main_window_zoomed`에 두고 `Tracker::observe(info, zoom_factor, is_zoomed)`가 `maximized = info.maximized == Some(true) || is_zoomed == Some(true)`로 기록합니다. 최대화 중 직전 일반 크기·위치 유지 규칙은 그대로입니다.
- 종료 코드(결정 6 A, vendored eframe): `run.rs`의 `WinitAppWrapper.return_result`를 `Rc<RefCell<..>>`(`SharedResult`)로 바꾸고 `EframeWinitApplication`이 같은 셀을 들게 했습니다. `create_wgpu`·`create_glow`는 `EframeWinitApplication`을 직접 돌려주고 `lib.rs` `create_native`는 그것을 그대로 반환합니다. 새 공개 함수 `EframeWinitApplication::take_result`, 앱 `MonitorProbe::take_result`. `main.rs`는 `run_app` 뒤 생성 결과(실패면 그 오류) 다음에 `probe.take_result()?`를 돌려줘 창 생성 뒤 치명 오류가 0이 아닌 종료 코드가 됩니다. `run_and_return` 경로는 같은 셀에서 꺼냅니다. 기록: `experiments/native-shell-spike/vendor/eframe/TAIDE-CHANGES.md`.
- 로거(결정 3 A): 새 모듈 `stderr-log.rs`(`taide_native_app::stderr_log`)의 의존성 없는 `log::Log`(`Warn` 이상, `[LEVEL] target: message` 한 줄을 stderr)를 `main` 첫 줄에서 설치합니다. 다른 로거가 먼저 있으면 그대로 둡니다.

### shell-98 보조 창 toast(결정 2 A)

- 새 모듈 `toast-layers.rs`: `ToastLayers { main, auxiliary: HashMap<ViewportId, Toasts>, focused }`. 루트 필드 `NativeApplication.toasts`의 타입만 바꿨습니다.
    - 역참조(`Deref`/`DerefMut`)가 frame 시작 때 OS focus를 가진 창의 층을 돌려줍니다(사양의 `ToastTarget::Interaction`). 그래서 다른 영역의 기존 `self.toasts.error(..)`·`report`·`toast::*` 호출 30여 곳은 바꾸지 않고 상호작용 규칙을 따릅니다.
    - `main()`이 사양의 `ToastTarget::Main`입니다. 주 창 전용 provider toast를 여기로 보냅니다: drop 결과(`DroppedPathsOpened`), 외부 열기 결과(`ExternalOpensProcessed`), 알림 첫 전달 안내, 앱 메뉴 kept drafts, 프로젝트 그룹 열기 결과.
    - `set_reduced_motion`·`take_actions`는 모든 층에 적용합니다(같은 이름의 고유 메서드라 기존 호출 줄 유지).
- focus 갱신: `tick_shell_menus` 첫 줄에서 `focused_viewport` 규칙(메인 focus 우선, 없으면 focus 가진 보조 창, 없으면 ROOT)으로 정합니다. 층이 없는 창이면 ROOT입니다.
- 보조 창 패스: 패스 전에 `mount`(층 생성, 실패는 상태줄), 콜백 시작에 그 창 입력으로 `tick`, 끝에 `show`. 활성 조건은 종료 중이 아니고, 기존 모달 판정이 거짓이거나 모달이 다른 창(`overlay_viewport`)에 있을 때입니다. 메인 층의 위치·tick·show는 `background_tick`·`finish_frame`에서 `main()`으로 그대로 합니다.
- 창 닫기(`close_auxiliary_window`)에서 그 층을 버립니다.

### shell-5 탭 drag 중 모든 창 web view 숨김

- UI `tab_drag::is_dragging_in(context, viewport)`: 그 창의 drag 기억을 어느 패스에서든 읽기 전용으로 봅니다. 기존 `is_dragging`은 현재 viewport로 위임합니다.
- 앱 `is_any_window_tab_dragging`(shell 영역): ROOT와 열린 보조 창 중 하나라도 탭 drag가 활성이면 참. 메인 web view 활성 판정(`domains/preview.rs` `show_preview_web_views`)의 `tab_drag::is_dragging(context)` 한 조건을 이것으로 바꿨습니다(editor+preview lane 파일, 한 줄).

### 사양과 실제 코드가 달라 실제 코드를 따른 곳

1. 사양 2.12의 `apply_appearances` 공통화는 이미 `domains/settings.rs`의 `apply_presentation_appearances`로 되어 있습니다.
2. shell-98: 사양은 호출부 30여 곳을 `for_target(ToastTarget)`로 바꾸게 했지만, 그 호출부 대부분이 다른 lane 영역 파일(editor·settings·explorer_search·terminal_agent·commands)이라 병합 충돌을 피하려고 루트 필드 타입만 바꾸고 역참조를 상호작용 층으로 두었습니다. 주 창 전용은 `main()`입니다. 다른 lane이 새로 쓰는 `self.toasts.*` 호출도 자동으로 상호작용 규칙을 따릅니다.
3. shell-98: 사양은 보조 층의 tick을 `background_tick`에서 하게 했지만 `Toasts::tick`이 hover·키·focus를 현재 viewport 입력에서 읽으므로 보조 층은 그 창 패스 시작에서 tick합니다. 감속 설정은 모든 층에 같게 둡니다.
4. shell-3: `background_tick`은 `eframe::Frame`을 받지 않으므로 `App::logic`이 zoom 값을 shell 상태에 두고 `observe`가 읽습니다.
5. shell-9: 사양대로 `initial_app_menu_model`을 두었고 `State`에 그린 모델을 미리 넣는 함수 이름은 `connect_installed`입니다.
6. shell-3 vendored 단위 검사: eframe dev-dependency(`directories`)가 spike lock에 없어 `--locked --offline`으로 vendored crate 검사를 돌릴 수 없습니다. 셀 공유는 컴파일(앱 check, remote-web Wasm canvas)과 코드 확인까지이고 실제 오류 종료는 실기입니다.

### 원본과 다르게 둔 점(근거)

- 시작 로케일 실패: 원본은 빈 리소스라 키가 그대로 보이지만 메인 결정대로 번들 `en`을 씁니다.
- 보조 창 toast: 결정 2 A의 기록된 차이(응답 전에 창을 바꾸면 바꾼 창). 프로젝트 그룹 저장 실패 등 대화상자 결과는 상호작용 층(대화상자가 뜬 focus 창)입니다.
- 로거: 원본 `tauri_plugin_log` 파일 출력은 실제 사용자 로그 경로라 하지 않았고(shell-15) stderr만 씁니다.

## 검증

명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/shell`, 앱 manifest는 `native/taide-native-app/Cargo.toml`입니다.

| 명령 | 결과 |
| --- | --- |
| 앱 `cargo check --quiet --all-targets` | 통과. 경고는 이 묶음 밖 기존 3건(`editor-code-actions.rs` `is_applying`, `zen.rs` `mut`, `application-inlay-hints-tests.rs` `fixture_with_emmet_peek`)과 vendored wry 경고 |
| UI `cargo check --quiet --all-targets --features native-host,inspection` | 통과. 기존 경고 1건(`tests/workbench.rs` 1816 `mut`) |
| 앱 `cargo test --lib -- 시작_때 drop_안내는_셸 부팅_직후 app_menu::tests file_drop_tests --test-threads=2` | 15 passed(shell-94·22·9) |
| 앱 `cargo test --lib -- window_state stderr_log --test-threads=2` | 17 passed(shell-3) |
| 앱 `cargo test --lib -- toast는_동작한_보조_창에만 --test-threads=2` | 1 passed(shell-98, 아래 실패 1회 뒤) |
| 앱 `cargo test --lib -- 탭을_끄는_동안에는 toast_layers --test-threads=2` | 2 passed(shell-5 drag, toast 층 단위) |
| UI `cargo test --features native-host,inspection --lib -- tab_drag` | 6 passed |
| 앱 회귀 `cargo test --lib -- window_state stderr_log toast auxiliary_window_tests external_open_tests notifications_tests file_drop_tests explorer_views_tests app_menu exit_tests project_rail_tests preview_web ipc_sync_tests ide_sync_tests startup_tests --test-threads=2` | 91 passed |
| remote-web `cargo check --quiet --target wasm32-unknown-unknown --lib` | 통과(동결 crate의 기존 dead_code 경고만) |
| remote-web `cargo check --quiet --target wasm32-unknown-unknown --lib --features canvas` | 통과(vendored eframe·UI 변경 뒤 재실행) |
| `cargo fmt -- --check` 앱·UI | 통과(앞 묶음 작업자가 남긴 테스트 형식 차이 포함해 Edit로 맞춤) |

새 검사

- 단위: `window_state` 2(zoom 기록과 직전 일반 크기·위치 유지, 핸들 없는 frame은 `None`), `stderr_log` 1, `toast_layers` 1(층 생성 전 대체, focus 창 라우팅, 주 창 분리, 닫힌 창 제거)
- 실제 App(`main.rs` → `NativeApplication` → 각 경로)
    - `explorer_views_tests` 2: 깨진 테마·로케일 파일로 launch → 번들 테마·`en`, 배너 2개(로케일 y≤32, 테마 y≥32), 셸 배경 `#1e1e2e` → 테마 파일을 고친 뒤 테마 재시도 → 테마 배너만 사라지고 사용자 배경 / 열리지만 그릴 수 없는 사용자 테마(`editor.background` 잘못된 색) → 번들 테마 + 테마 배너
    - `file_drop_tests` 1: 두 실패 + hover 파일에서 그리기 순번이 셸 본문 < overlay < 배너 2개
    - `app_menu_tests` 1: 최근 11개 프로젝트 data dir로 launch → `initial_app_menu_model`이 `project_list_recent`와 같은 모델, 상위 10개만, `Clear Recent` 활성
    - `auxiliary_window_tests` 2: 보조 창 focus에서 낸 오류 toast가 보조 viewport 출력에만 있고 메인 층·메인 출력에는 없음, 외부 열기 안내는 보조 창 focus여도 메인 출력에만, 보조 창 닫기 뒤 층 제거 / 보조 창 탭 press + 24px 이동으로 그 창만 drag 활성이고 `is_any_window_tab_dragging` 참, 놓으면 거짓
- 실패 선행 확인: shell-98 App 검사는 처음에 보조 창 출력에 문구가 없어 실패했습니다. 층 라우팅 단언은 통과했고 원인은 toast 등장 애니메이션 첫 frame의 불투명도 0(그리기 생략)이라 출력이 나올 때까지 frame을 진행하도록 검사를 고쳤습니다. 다른 새 검사는 구현과 함께 썼고 구현 전 실패를 따로 실행하지 않았습니다. shell-94·22·9의 구현과 검사는 이 묶음의 앞 작업자가 남긴 미커밋 변경이며 이번에 실행해 통과를 확인했습니다.
- 실행하지 않은 것: 필터 없는 앱 전체 테스트(메인 병합 때), vendored eframe crate 단위 검사와 `cargo fmt`(spike workspace 밖이라 `cargo metadata` 실패, 위 6), GUI·실제 창이 필요한 경로(zoom 판정, eframe 치명 오류 종료 코드, NSMenu, Finder drag).

## 남은 범위

- shell-5(이 묶음에서 남긴 3건, 원인은 범위·충돌 크기)
    1. 보조 창 WKWebView 배치: vendored eframe `render_immediate_viewport`에 콜백 동안의 창(`Arc<winit::Window>`)을 읽는 `immediate_viewport_window()`를 열고, `preview_web_view::Views`를 viewport별로 나눠 `sync`가 `&impl HasWindowHandle`을 받게 해야 합니다. `preview_web_view.rs`는 editor+preview lane 파일이라 그 lane과 구조 합의가 먼저 필요합니다. 보조 창 web view drag 처리기는 항상 기본 처리(`false`), 창 닫기 때 제거.
    2. 보조 창 터미널 raw input(마우스 보고): vendored eframe `SharedState`에 immediate raw input hook을 두고 `take_egui_input` 직후 부르며, `terminal_surface::Views`의 raw input 상태를 `Rc<RefCell<..>>`로 나눠야 합니다. `terminal_surface.rs`는 terminal-agent lane 파일입니다.
    3. 보조 창 theme preview: 보조 창 패스 시작에 `settings_views.preview(window.viewport, ..)`의 `Appearances`를 적용하고 끝에 메인 값으로 되돌립니다. `apply_presentation_appearances`가 settings 영역의 비공개 함수이고 `Appearances`가 `Clone`이 아니어서 캐시 방식(또는 frame마다 두 번 생성)을 settings lane과 정해야 합니다. 원본처럼 그 창의 구문 강조 색까지 바꾸려면 `editor_syntax`의 테마별 캐시도 창별이어야 합니다.
    4. 묶음 3: 보조 창 탐색기 search·git 보기(explorer-search-30, git-1), 창별 탐색기 상태(explorer-search D6), 보조 창 Task Runner(terminal-agent-76), 파일 선택 dialog 부모 창(허락 4)
- 실기(배치 31): shell-3 실제 위치 복원·zoom·모니터·eframe 오류 종료 코드, shell-9 NSMenu, shell-22 Finder drag·WKWebView 대상 전환·점선 픽셀, shell-94 배너 픽셀, shell-98 toast 픽셀·지연

## 수정(shell lane 리뷰 반영)

### shell-3 complete → partial

리뷰가 맞습니다. shell-3의 남은 것 2번(창 생성 뒤 eframe 치명 오류의 종료 코드)은 사양 2.10 완료 판정이 "구현과 검사 통과"이고 검사로 vendored eframe 결과 셀 공유 단위 검사를 지정했는데, 그 검사를 추가·실행하지 못했습니다. 지금 근거는 다음뿐입니다.

- 구현: vendored `run.rs` `SharedResult` 셀 공유, `EframeWinitApplication::take_result`, `MonitorProbe::take_result`, `main.rs`의 `Ok(probe.take_result()?)`(이 문서 구현 절)
- 확인: 앱 `cargo check`·remote-web Wasm check 컴파일과 코드 읽기

자동 검사를 둘 수 없는 이유를 다시 확인했습니다.

1. vendored eframe 단위 검사: eframe은 앱 workspace의 비멤버 path 의존이고 dev-dependency(`directories`, vendored `Cargo.toml` 272행)가 있습니다. `cargo test --quiet --no-run -p eframe --lib --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir ..`는 `package eframe cannot be tested because it requires dev-dependencies and is not a member of the workspace`(exit 101)로 거부되고, lock에도 `directories`가 없습니다. `experiments/native-shell-spike` workspace에서도 eframe은 `[patch.crates-io]` 대상이라 멤버가 아닙니다. 실행하려면 vendored eframe manifest나 lock을 바꿔야 하고, 이번 lane은 Cargo 파일을 바꾸지 않습니다.
2. 앱 쪽 검사: `EframeWinitApplication`은 `create_native(.., &EventLoop)`로만 만들 수 있습니다(`new`와 `WinitApp`은 crate 비공개). event loop를 만들어 돌리는 것은 이 lane에서 금지한 GUI 실행이고, macOS winit event loop는 주 스레드를 요구한다고 문서화돼 있어 libtest 스레드에서도 만들 수 없습니다(이번에 실행으로 확인하지는 않음). `MonitorProbe`도 이 구체 타입을 들고 있어 가짜 안쪽 앱으로 바꿀 수 없습니다.

그래서 대응표 shell-3을 partial로 낮추고, 종료 코드 경로는 "컴파일과 코드 확인까지, 자동 검사 없음(미검증)"이라고 적었습니다. AppKit `isZoomed` 읽기 자체도 실기 항목으로 분리해 적었습니다(기록 규칙과 핸들 없는 frame `None`은 단위 검사). 위 "사양과 실제 코드가 달라" 6의 "셀 공유는 컴파일과 코드 확인까지"는 그대로 유효합니다.

남은 것(shell-3 complete 조건)

1. 종료 코드 셀 공유 단위 검사: 실행 방법 허락이 필요합니다(아래 선택지 중 하나). 검사 내용은 사양 2.10대로 이벤트 루프 없이 `WinitAppWrapper`의 셀에 오류를 넣고 `EframeWinitApplication::take_result`가 그 오류를 돌려준 뒤 `Ok(())`를 남기는지입니다(가짜 `WinitApp`을 `run.rs` `#[cfg(test)]` 모듈에 둠).
2. 실기(배치 31): 실제 모니터 읽기·위치 적용·AppKit zoom 판정, 창 생성 뒤 실제 eframe 오류의 종료 코드

### 검증(수정)

코드 변경이 없는 문서 정정이라 shell-3에 대해 새로 실행한 검사는 없습니다. 위 검증 절의 `cargo test --lib -- window_state stderr_log --test-threads=2`(17 passed)가 zoom 기록·로거 근거로 그대로 유효합니다(관련 파일 변경 없음).
