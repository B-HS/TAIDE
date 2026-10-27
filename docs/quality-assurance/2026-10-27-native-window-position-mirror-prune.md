# 창 위치 복원·잔여 mirror 정리 (배치 24 단계 E2 1/3)

## 대상

- shell-3: 메인 창 위치 복원과 원본의 모니터 교차 보정. 사용자 결정은 `docs/acknowledge/2026-10-26-native-autonomous-continuation.md`의 '단계 E 결정' A안입니다(이미 lock에 있는 `winit =0.30.13`을 일반 의존으로 올리고 `eframe::create_native`로 이벤트 루프를 직접 만듭니다).
- shell-25: 프로젝트가 editor 영역에 보일 때의 잔여 hot-exit mirror 정리 sweep. 직전 단계 `2026-10-27-native-window-session-restore.md`의 남은 범위를 이었습니다.
- 원본:
    - `tauri-plugin-window-state` 2.4.1 `src/lib.rs` 166-268(복원), 281-321(`update_state`), 447-500(이벤트 추적), 523-549(`MonitorExt::intersects`)
    - `src-tauri/src/lib.rs` 856-870(`map_label`·`with_filter`로 보조 창 제외)
    - `src/widgets/editor-area/editor-area.tsx` 86-87·409-443(sweep), `src/widgets/app-shell/shell-slot-tree-view.tsx` 81-107(slot마다 `ProjectShell` → `EditorArea`), `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 139(보조 창의 `EditorArea`)
    - `crates/taide-file/src/service.rs` 421-453(`prune_mirrors`), 472-505(`prune_untitled_mirrors`), `crates/taide-runtime/src/file_actions.rs` 204-242

## 원본 동작

- 창 위치(`tauri-plugin-window-state`):
    - 위치는 창 바깥 좌상단(`outer_position`)입니다. 최대화·최소화 중에는 갱신하지 않습니다(314-318). `Moved` 이벤트는 `prev_x/prev_y`에 직전 값을 남기고, 최대화 상태 복원 때 그 직전 값을 씁니다(197-206, 455-472).
    - 복원 때 저장 사각형(위치 + 안쪽 크기)이 연결된 모니터 중 하나와 겹칠 때만 위치를 적용합니다. 겹치지 않으면 OS가 위치를 정합니다(189-210).
    - 겹침 판정은 사각형의 네 모서리 중 하나가 모니터 영역 `[left, right) × [top, bottom)` 안에 있는지입니다(527-548).
    - TAIDE는 `with_filter`로 보조 창(`editor`)을 제외하므로 메인 창만 저장·복원합니다.
- 잔여 mirror sweep(`editor-area.tsx` 409-443):
    - `EditorArea`마다 `prunedProjectIdRef`를 둡니다. layout이 준비됐고 그 영역의 프로젝트가 마지막 sweep 프로젝트와 다를 때 한 번 실행합니다. 같은 프로젝트의 이후 layout 변경으로는 다시 돌지 않습니다.
    - `EditorArea`는 main 창의 slot마다, 그리고 보조 창마다 하나씩 mount됩니다. 그래서 slot이 다른 프로젝트로 바뀌거나 새 slot·보조 창이 프로젝트를 보이면 다시 실행됩니다.
    - keep 목록:
        - path mirror: main tree와 모든 보조 창 tree(`collectAllPaneTabs`)의 열린 file 탭 경로입니다. 닫힌 탭 스택은 넣지 않습니다.
        - untitled mirror: 열린 탭과 닫힌 탭 스택(`closedTabs`)의 untitled 탭 id입니다.
    - `prune_mirrors`는 keep에 없는 path mirror와 읽을 수 없는 mirror 파일을 지웁니다(원본 파일 존재 여부와 무관). `prune_untitled_mirrors`도 같은 방식입니다. 두 호출은 서로 독립이고, 실패는 조용히 무시합니다(`.catch(() => undefined)`).

## 구현

- 의존성: `native/taide-native-app/Cargo.toml`에서 `winit = { version = "=0.30.13", default-features = false }`를 dev-dependencies에서 dependencies로 옮겼습니다. `Cargo.lock`은 이미 앱 패키지의 의존 목록에 `winit`이 있어 바뀌지 않았습니다. crates.io 항목 버전도 동결 `taide-remote-web/Cargo.lock`도 그대로입니다.
- shell-3 위치 복원:
    - 이벤트 루프(`main.rs` 33-72):
        - vendored eframe 0.36.2에서 확인한 `create_native`(`lib.rs` 376-407)를 씁니다. 기존 `run_native`는 기본값 `run_and_return: true`라 같은 `WinitAppWrapper(…, true)` 경로였습니다(`native/run.rs` 450-484, `epi.rs` 456). 그래서 종료 의미는 바뀌지 않습니다.
        - winit 0.30.13은 모니터 목록을 실행 중인 `ActiveEventLoop`에서만 줍니다(`event_loop.rs` 396-417). 그래서 `window_state::MonitorProbe`가 eframe 앱을 감싸 `resumed`에서 eframe보다 먼저 `available_monitors()`를 읽어 `Monitors`(`Rc<OnceCell>`)에 넣습니다(`window-state.rs` 127-189, 리뷰 수정 뒤 줄 번호). 그다음 eframe이 같은 `resumed`에서 창을 만듭니다(`wgpu_integration.rs` 456-486).
        - 창 생성 때 eframe의 `NativeOptions.window_builder` hook(`epi_integration.rs` 86-89)이 `window_state::main_window_hook`을 부릅니다(`window-state.rs` 117-125). 이 hook이 저장 상태와 그 모니터 목록으로 `ViewportBuilder`를 만듭니다.
        - 오류 전달: `create_native` 경로는 eframe 내부 오류를 반환하지 않고 로그만 남깁니다. 그래서 `main.rs`가 app 생성 실패 문구를 보관했다가 루프 종료 뒤 `Err`로 돌려줍니다. 창이나 wgpu를 만들지 못해 app이 생성되지 않았으면 `native main window was not created`로 실패합니다. 기존 `run_native`처럼 시작 실패가 0이 아닌 종료 코드가 되게 하기 위해서입니다.
        - 이 경로의 회귀(리뷰 지적, 미해결): 앱에는 로거 초기화가 없어 eframe이 `log::error!`로만 남기는 원인(wgpu 어댑터 없음 등)이 어디에도 출력되지 않습니다. 이전 `run_native`는 `eframe::Error`를 `main`의 `Err`로 돌려 원인이 stderr에 나왔지만, 이제는 `native main window was not created`만 나옵니다. 또 창과 app이 만들어진 뒤의 eframe 치명 오류(`run_ui_and_paint`의 `Err` 등)는 eframe 내부 `return_result`에만 담겨 호출자에게 오지 않으므로, 루프가 끝나면 `startup`이 `Some(Ok)`라 종료 코드 0으로 끝납니다. `run_native`는 이 경우 `Err`를 돌려줬습니다.
    - 저장(`Tracker::observe`, `window-state.rs` 204-238): root viewport의 `outer_rect.min × zoom_factor`(논리 point)를 `position`으로 기록합니다. 최대화·전체 화면·최소화 중에는 직전 일반 창의 위치와 크기를 유지합니다. `outer_rect`가 없으면(위치를 모르는 플랫폼) 이전 값을 유지합니다.
    - 최대화 상태 복원의 판정 기준은 원본과 다릅니다. 리뷰에 따르면 원본은 최대화 상태에서 현재(최대화) 위치 x,y로 `intersects`를 판정한 뒤 `prev_x/prev_y`를 적용합니다. native는 직전 일반 위치로 판정하고 그 위치를 적용합니다. 그래서 직전 일반 위치가 연결된 모든 모니터 밖이고 최대화 위치는 안쪽인 드문 경우, 원본은 그 위치를 적용하고 native는 적용하지 않습니다(OS가 위치를 정함). 그 밖의 경우 적용되는 위치는 같습니다. native 쪽은 화면 밖 위치를 적용하지 않는 방향이라 그대로 두었습니다. 이 단계 규칙상 저장소 밖 plugin 소스를 다시 읽지 않았으므로, 원본 쪽 서술은 리뷰 판독에 기댄 것입니다.
    - 복원(`WindowState::apply`, 27-61): 크기·최대화·전체 화면을 적용한 뒤, 저장 사각형(위치 + 저장 크기)의 네 모서리 중 하나가 어떤 모니터의 반열린 영역 안에 있을 때만 `with_position`을 적용합니다. 모니터 목록이 비었거나 겹치지 않으면 위치를 넣지 않아 OS가 정합니다.
    - 모니터 좌표(`Monitor::from_physical`·`From<MonitorHandle>`, 리뷰 수정 뒤 70-88): winit 물리 위치·크기를 그 모니터 배율로 나눠 논리 point로 바꿉니다. `From<MonitorHandle>`은 이 순수 함수에 `position()`·`size()`·`scale_factor()`를 넘깁니다. macOS에서 이 값은 `CGDisplayBounds`(주 화면 좌상단 기준 point)입니다(winit `macos/monitor.rs` 234-249). 창의 `outer_position`도 같은 전역 point 공간을 씁니다(`flip_window_screen_coordinates`, 401-407). 그래서 macOS에서는 배율이 다른 모니터를 섞어도 비교가 맞습니다.
    - 파일 형식: `position`은 `#[serde(default)]`라 직전 단계에서 저장한 위치 없는 파일도 크기를 그대로 복원합니다. 보조 창은 계속 기록하지 않습니다(원본 `with_filter`).
    - 원본과 다르게 둔 점: 원본은 물리 픽셀을 비교하고 native는 논리 point를 비교합니다. eframe이 point 단위 `ViewportBuilder`로 창을 만들고 크기도 point로 저장하기 때문입니다. macOS가 아닌 플랫폼에서 배율이 다른 모니터를 섞으면 판정이 크게 다를 수 있습니다. 각 모니터의 물리 원점을 그 모니터 배율로 나눈 논리 좌표는 서로 이어지지 않아(예: 배율 1.0 모니터 옆 배율 2.0 모니터의 논리 원점이 절반으로 당겨짐) 모니터 영역이 겹치거나 비고, 오판 폭이 모니터 폭 단위일 수 있습니다(TAIDE 1차 대상은 macOS).
- shell-25 잔여 mirror sweep:
    - `mirror-sweep.rs` `MirrorSweeps::due`(18-41): 원본 `EditorArea` 한 개를 host 하나로 봅니다. host는 main slot id와 보조 창의 explorer slot id(`auxiliary_windows.explorer_mounts`)입니다. host마다 마지막 sweep 프로젝트를 기억합니다.
        - layout이 있는 프로젝트를 처음 보이거나 다른 프로젝트로 바뀐 host만 sweep 대상이 됩니다.
        - 사라진 host는 잊습니다. 그래서 다시 mount되면 다시 실행됩니다.
        - 같은 프로젝트가 여러 host에서 한 pass에 예약되면 한 번만 보냅니다.
        - `retry`(42-44): 제출에 실패한 프로젝트를 기억한 host 항목을 모두 지웁니다. 다음 `due`는 그 프로젝트를 다시 한 번만 냅니다. 원본(`editor-area.tsx` 428-441)은 `prunedProjectIdRef`를 먼저 기록하고 IPC 결과의 실패를 `.catch(() => undefined)`로 버려 다시 시도하지 않습니다. native도 host에서 실행된 prune의 실패는 다시 시도하지 않습니다(`host.rs` 1529-1534, 로그만). 다만 native에는 유한 host 명령 큐(`HOST_COMMAND_CAPACITY` 64, `host.rs` 16·693)가 있고, 큐가 차서 제출이 실패하면 prune은 실행조차 되지 않습니다. 원본에는 이 상태가 없으므로 '활성화마다 한 번 실행'을 지키려고 다시 예약합니다. 같은 함수의 `RefreshTree`(실패 시 `tree_changes.merge`)·`GitBranch`(실패 시 `title_branches.retry`)와 같은 재시도 방식입니다. `submit`은 실패를 `report_once`(`application.rs` 2505·2035, `toast.rs` 76-81)로 알리고, 이 함수는 같은 오류 toast가 떠 있는 동안 새 toast를 쌓지 않습니다. 그래서 큐가 비워질 때까지의 재시도가 toast를 늘리지 않습니다.
    - App 연결(`application.rs` 6974-6983): `background_tick`에서 종료 중이 아닐 때 `due` 결과를 `HostCommand::PruneMirrors(project)`로 보냅니다. 제출이 실패하면(host 명령 큐 포화 등) `MirrorSweeps::retry`로 그 프로젝트의 sweep 기록을 지우고 repaint를 요청해, 다음 pass에 다시 예약합니다(리뷰 수정).
    - 호스트(`host.rs` 1529-1534, `mirror-sweep.rs` 89-123, 리뷰 수정 뒤 줄 번호): `begin_owned_mutation` guard 안에서 다음을 합니다.
        - 프로젝트가 열려 있는지 확인합니다.
        - 그 순간의 layout으로 keep 목록을 계산합니다(`keep`, 63-87).
        - `taide_file::service::prune_mirrors`와 `prune_untitled_mirrors`를 둘 다 실행합니다.
        - 실패는 원본처럼 화면에 보이지 않고 경고 로그만 남깁니다.
    - canonical 비교: native 편집 중 mirror는 canonical 경로를 표시 경로로 씁니다(`persistence.rs` 247-254). 그래서 keep에는 탭 경로 원문과 `root_guard::canonicalize_lenient` 결과를 함께 넣습니다. 원문은 종료 flush(`file_mirror_dirty`)나 원본 앱이 쓴 mirror를 위해 넣었습니다. 원본 파일이 지워진 탭도 부모 canonical + 파일 이름으로 맞습니다.
    - 쓰기 경쟁: native mirror 쓰기(`persistence::write_mirror`)도 같은 `begin_owned_mutation` guard 안에서 탭 존재를 확인한 뒤 씁니다. sweep은 같은 guard 안에서 keep을 계산하므로, 쓰기와 sweep 사이에 열린 탭의 mirror를 지우는 순서가 생기지 않습니다.
    - 저장 위치는 `--data-dir`의 `buffers/<project>`뿐입니다(`AppPaths::buffers_dir`).

## 검증

- 실제 앱 경로:
    - 위치: `main.rs` → `EventLoop::with_user_event` → `eframe::create_native` → `MonitorProbe::resumed`(모니터 읽기) → eframe `create_window` → `window_builder` hook → `main_viewport` → `WindowState::apply`
    - 기록: `NativeApplication::logic`(`background_tick`) → `Tracker::observe` → `App::on_exit` → `Tracker::save`
    - sweep: `NativeApplication::logic`(`background_tick`) → `MirrorSweeps::due` → `HostCommand::PruneMirrors` → `mirror_sweep::prune`
- 검사 순서:
    - sweep은 App 검사(아래 3번째)를 먼저 쓰고, 모듈·호스트 명령만 있고 `background_tick` 연결이 없는 상태에서 실행했습니다. `phase=alpha mirror sweep`의 10초 단계 기한 초과로 실패했습니다(같은 실행에서 단위 2건은 통과). 연결을 넣은 뒤 통과했습니다.
    - 위치 단위 검사는 구현과 같은 단계에서 썼습니다. 구현 전에는 `position` 필드·`Monitor`·hook이 없어 컴파일되지 않는 상태였습니다. 구현을 끄는 재현은 작업 규칙상 하지 않았습니다.
- `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline … -- mirror_sweep:: window_state`는 15/15 통과입니다(68.44초, 서식 정리 뒤 재실행).
    - `window_state` 단위 10건(`window-state-tests.rs`):
        - 기존 5건(크기·최대화·zoom·손상 파일). 새 인자인 모니터 목록만 넘깁니다.
        - 위치 추적·저장 뒤, 겹치는 모니터가 있을 때만 `position`을 넣고 모니터가 없거나 겹치지 않으면 넣지 않음
        - 모서리 판정: 왼쪽 밖에서 오른쪽 모서리만 걸침 → 적용, 오른쪽 경계 바로 안 → 적용, 경계와 같은 x → 미적용(반열린), 두 번째 모니터가 있으면 적용, 모든 모니터 밖 → 미적용
        - 최대화·전체 화면·최소화 중 직전 일반 위치 유지, `outer_rect` 없음 → 유지, zoom 배율 곱
        - 위치 없는 이전 파일 → 크기·전체 화면 복원, 위치 없음
        - `main_window_hook`이 hook 생성 뒤 `MonitorProbe`처럼 나중에 채운 모니터 목록으로 위치를 정함, 목록이 없으면 위치 없음
    - `mirror_sweep` 단위 2건(`mirror-sweep-tests.rs`):
        - host별 한 번: 첫 pass 두 프로젝트(보조 창과 겹친 alpha는 한 번), 같은 상태 재실행 없음, slot 프로젝트 전환·복귀 때마다 재예약, layout 없는 프로젝트는 생길 때까지 보류, 보조 창 unmount 뒤 재mount 재예약, 전체 unmount 뒤 재예약
        - keep 목록: symlink 경로 탭은 원문과 canonical, 원본 파일이 없는 보조 창 탭도 canonical, 닫힌 file 탭 제외, 열린·닫힌 untitled id 포함
    - 실제 App 3건(`application-window-state-tests.rs`):
        - 직전 단계 2건: 메인 root 정보(원점 64,48)를 공급하면 종료 때 `main` 항목에 크기와 위치가 저장되고, 모니터가 겹치면 같은 위치·크기로 복원됩니다. 재시작 검사도 위치를 확인합니다. 첫 검사의 이름은 `…메인_창_상태만_추적해…`로 바꿨습니다(위치도 기록하므로).
        - 새 검사 `실제_app은_editor_영역이_프로젝트를_보일_때_열린_탭_밖의_잔여_mirror만_정리한다`(unix):
            - alpha의 main 탭은 symlink 경로이고, 보조 창 탭(`notes.txt`)·열린 untitled 탭이 있고, 닫힌 file·untitled 탭이 있습니다. mirror는 native처럼 canonical 경로로 미리 씁니다.
            - App 부팅 뒤 alpha mirror는 열린 file 탭 2개의 canonical mirror만 남고, untitled mirror는 열린·닫힌 탭 2개만 남습니다(stale·닫힌 file·stale untitled 삭제).
            - 어떤 영역에도 보이지 않은 beta의 stale mirror는 남습니다. `ActivateProject(beta)`로 slot에 보이면 지워지고, alpha의 남은 mirror는 그대로입니다.
- 제거·표시 경로를 바꾸는 변경이라 mirror를 단언하는 기존 App 검사를 함께 실행했습니다: `cargo test … --lib -- application::exit_tests application::missing_draft_tests application::project_rail_tests application::app_menu_tests`는 12건 중 10건 통과, 2건 실패입니다(208.39초, 병렬).
    - `app_menu_tests::실제_app_메뉴는_…`: `phase=clear recent` 10초 단계 기한 초과입니다. 이 단계 이전부터 같은 위치에서 3회 연속 실패한 기록이 있습니다(`2026-10-27-native-app-menu.md` 119-121). 이 검사는 열리지 않은 프로젝트의 buffers만 쓰므로 sweep 대상이 아닙니다.
    - `project_rail_tests::실제_app_그룹은_…`: 병렬 실행에서 `native host command queue is full` toast와 함께 결과 toast 기한을 넘겼습니다. 같은 검사를 단독으로 다시 돌리면 1/1 통과입니다(52.84초). 이 변경이 보내는 명령은 host가 프로젝트를 처음 보일 때 1건뿐이고, 같은 상태에서는 다시 보내지 않습니다(단위 검사로 확인). 병렬 부하에서 host 처리가 늦어진 것으로 판단했습니다.
- `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets --locked --offline`은 exit 0입니다(기존 경고만 있음). `main.rs`의 `create_native`/`run_app` 경로와 `tests/paste-shortcuts.rs`(winit 사용)도 이 check에 포함됩니다.
- `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline`은 exit 0입니다(기존 경고만 있음). remote-web `Cargo.lock`은 바뀌지 않았습니다.
- `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check`: 이번에 추가하거나 바꾼 부분에는 차이가 없습니다. 남은 차이는 직전 단계 기록과 같은 기존 코드뿐입니다(`experiments/lsp-coordinator-spike/src/bin/mock-server.rs`, `application.rs` 960행 들여쓰기·테스트 모듈 순서, `lib.rs`의 `editor_schema_transport`/`open_path_dialog`/`welcome` 순서). 요청 밖 재포맷이라 손대지 않았습니다.
- 실행하지 않은 것: 필터 없는 앱 전체 테스트(배치 끝 메인 담당), GUI 실기. 실제 OS 모니터 목록 읽기와 창 위치 적용은 창을 띄워야 해서 headless 검사가 없습니다. 근거는 위 코드 경로·winit/eframe 소스 확인·컴파일입니다.

## 리뷰 수정 (1차)

- 차단 항목(실제 결함으로 확인): `MirrorSweeps::due`는 반환 전에 `swept`에 (host, project)를 기록했고, `background_tick`은 `self.submit(HostCommand::PruneMirrors(project))`의 `bool`을 버렸습니다. `submit`은 `bridge.submit` 실패(`host.rs` 693의 `native host command queue is full` 등) 때 toast를 띄우고 `false`를 돌려줍니다(`application.rs` 2475-2507). 그래서 큐가 찬 틱에 처음 보인 프로젝트는 slot 전환이나 host unmount 전까지 다시 sweep되지 않았습니다.
- 수정:
    - `mirror-sweep.rs` `MirrorSweeps::retry`(42-44): 그 프로젝트를 기억한 host 항목을 모두 지웁니다.
    - `application.rs` 6974-6983: 제출이 실패하면 `retry` 뒤 `context.request_repaint()`를 부릅니다. 같은 함수의 `RefreshTree`·`GitBranch` 실패 처리와 같은 형태입니다.
- 비차단 반영:
    - `Monitor::from_physical`(`window-state.rs` 70-82)로 물리→논리 변환을 순수 함수로 분리했습니다. `From<MonitorHandle>`은 이 함수만 부릅니다. 단위 검사 1건을 추가했습니다(배율 2.0·음수 원점 `[-3456, -400]`/`[3456, 2234]` → `[-1728, -200]`/`[1728, 1117]`, 배율 1.0 → 그대로).
    - 문서 정정: 최대화 상태 판정 기준 차이, 비macOS 배율 혼합 판정 폭, `create_native` 경로의 원인 문구 손실과 생성 뒤 치명 오류의 종료 코드 0을 위 '구현' 절과 기능표 shell-3에 적었습니다.
- 비차단 미반영:
    - 로거 추가: 앱 전체의 `log::*` 출력이 처음으로 stderr에 나오게 되는 동작 변경이고, `env_logger` 같은 새 의존이 필요하거나 직접 `log::Log` 구현을 새로 들여야 합니다. 이 수정 범위 밖이라 남은 위험으로 기록했습니다.
    - 생성 뒤 eframe 치명 오류의 종료 코드: vendored eframe(`experiments/native-shell-spike/vendor/eframe/src/native/run.rs`)에서 오류는 `log::error!` 뒤 `return_result`에만 담깁니다(161-165). `create_native`가 돌려주는 `EframeWinitApplication`은 그 앱을 `Box<dyn ApplicationHandler>`로 감싸고(492-494) 결과를 꺼내는 공개 API가 없습니다(546-562). 그래서 vendored eframe을 고치지 않고는 전달할 방법이 없어 남은 위험으로 기록했습니다.
- 검사 순서: 재예약 단위 검사를 먼저 추가하고 실행했습니다. `retry`가 없어 컴파일 오류(E0599 `no method named retry`)로 실패했고, 구현 뒤 통과했습니다. 모니터 변환 검사는 같은 이유로 구현 전 컴파일 실패가 분명해 구현 전 실행은 생략했습니다.
- 바꾼 파일: `native/taide-native-app/src/mirror-sweep.rs`, `mirror-sweep-tests.rs`, `application.rs`(6979-6982만), `window-state.rs`, `window-state-tests.rs`, 이 문서, 기능표(`2026-10-09-native-function-matrix.md` shell-3·shell-25 행). 모두 이 단계의 범위(shell-3·shell-25) 안입니다.
- 검증 결과: 아래 '리뷰 수정 검증'에 적습니다.

## 리뷰 수정 검증

- `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline … -- mirror_sweep::`(수정 전): 컴파일 실패(E0599 2건, `retry` 없음).
- 같은 명령(수정 뒤): 3/3 통과.
- `cargo test … --lib … -- mirror_sweep:: window_state`: 17/17 통과(75.41초). 단위 `mirror_sweep` 3건, `window_state` 11건, 실제 App `window_state_tests` 3건입니다.
    - 새 단위 `sweep_제출에_실패한_프로젝트는_다음_pass에_한번_다시_예약한다`: 첫 pass 두 프로젝트 → alpha `retry` → 다음 pass에 alpha 한 번(main slot과 보조 창 두 host가 alpha를 보여도 한 번) → 그다음 pass 없음 → 기록 없는 프로젝트 `retry`는 영향 없음.
    - 새 단위 `모니터의_물리_위치와_크기는_그_모니터_배율로_나눈_논리_point가_된다`: 위 '비차단 반영'의 두 경우입니다.
- `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets --locked --offline …`: exit 0(기존 경고만 있음).
- `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline …`: exit 0(기존 경고만 있음). remote-web는 `taide-native-app`에 의존하지 않고 이번 수정은 앱 크레이트 안이라 영향이 없지만 다시 확인했습니다.
- `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check`: 이번에 바꾼 부분에는 차이가 없습니다. 남은 차이는 위 '검증'에 적은 기존 코드와 같습니다.
- 리뷰 권고에 따라 mirror를 다루는 App 묶음을 수정 뒤 다시 병렬로 돌렸습니다: `cargo test … --lib -- application::exit_tests application::missing_draft_tests application::project_rail_tests application::app_menu_tests`는 12건 중 10건 통과, 2건 실패입니다(519.51초. 수정 전 같은 묶음은 208.39초로, 이번 실행 동안 기계 부하가 컸습니다).
    - `app_menu_tests::실제_app_메뉴는_…`: `phase=clear recent status=None`. 수정 전 기록과 같은 위치의 기존 실패입니다.
    - `project_rail_tests::실제_app_그룹은_…`: `group open result toast "Opened 1, skipped 2": {} open=3`. 이번에는 기한 동안 toast가 하나도 없었고 `native host command queue is full` toast도 없었습니다. 그래서 이번 실패는 큐 포화가 아니라 부하로 인한 기한 초과로 봅니다. 같은 검사를 단독으로 다시 돌리면 1/1 통과입니다(48.41초).
    - 수정 전 실패에서 PruneMirrors가 큐 포화에 기여했는지는 이번에도 직접 측정하지 못했습니다. 다만 이 변경이 보내는 명령은 host가 프로젝트를 처음 보일 때 프로젝트당 1건이고, 이제 제출이 실패해도 sweep이 사라지지 않고 다음 pass에 다시 갑니다.

## 남은 범위

- shell-3: 실제 창에서의 위치 복원(모니터 분리·배율 혼합 포함)과 `create_native` 종료 경로 실기는 배치 31입니다. macOS 실행 중 zoom 상태를 egui-winit이 갱신하지 않는 문제(직전 단계 기록)는 위치에도 같게 적용됩니다. zoom된 창의 위치·크기를 일반 창 값으로 기록할 수 있습니다.
- shell-3 남은 위험(리뷰 지적):
    - 시작 실패 원인 문구가 출력되지 않습니다(로거 없음). 종료 코드는 0이 아닙니다.
    - 창 생성 뒤의 eframe 치명 오류는 호출자에게 오지 않아 종료 코드 0으로 끝납니다.
    - 최대화 상태에서 직전 일반 위치만 모든 모니터 밖이면 원본은 위치를 적용하고 native는 OS에 맡깁니다.
- shell-25: 남은 기능 범위는 없습니다. 실기는 배치 31입니다.
- shell-25 테스트 부채(리뷰 지적, 실행하지 않음):
    - 재현 조건: 탭 경로가 심볼릭 링크 파일이고, 그 링크의 대상 파일이 삭제된 뒤 sweep이 돕니다.
    - 결과: mirror는 대상의 canonical 경로로 기록돼 있습니다. 삭제 뒤 `canonicalize_lenient`는 링크의 부모 canonical + 링크 이름으로 풀려 keep과 맞지 않습니다. 그래서 열린 탭의 mirror가 지워질 수 있습니다. 원본은 탭 경로 원문으로 비교하고 원문으로 mirror를 쓰므로 해당 사항이 없습니다.
    - 생략 이유: 파일 단위 심볼릭 링크 + 대상 삭제 + 그 사이 sweep이라는 세 조건이 겹치는 드문 경로입니다. 고치려면 mirror 기록 경로 규칙(`persistence.rs` 247-254)과 keep 계산을 함께 바꿔야 해서 이번 수정 범위를 넘습니다.
    - 실행이 필요해지는 시점: 파일 단위 심볼릭 링크 탭의 hot-exit 복원을 다루는 작업, 또는 배치 31 실기에서 해당 증상이 보일 때입니다.
