# 창·세션 복원 (배치 24 단계 E 1/3)

## 대상

- shell-3: 창 위치/크기 복원. 원본은 `tauri-plugin-window-state`입니다.
- shell-25: Hot-exit·앱 종료 후 세션 복원(프로젝트·slot 트리·레이아웃·zen).
- shell-8: 보조 창 복원. 이번에는 복제했던 원본 규칙을 `taide-window` 직접 호출로 바꿨습니다.
- 원본:
    - `src-tauri/src/lib.rs` 856-870(플러그인 등록·`map_label`·`with_filter`), 889-895·976-980(메인 창 생성과 지연 복원)
    - `tauri-plugin-window-state` 2.4.1(`Cargo.lock` 고정) `src/lib.rs`
    - `src-tauri/tauri.conf.json` 13-26(메인 창 1400×900, 최소 720×480, `visible: false`)
    - `src-tauri/src/domain/window/commands.rs` 340-360, `crates/taide-window/src/service.rs`
    - `src/widgets/app-shell/use-window-chrome.ts` 75-97, `src/entities/session/session.query.ts`, `src/app/providers/ipc-sync-provider.tsx` 348-360

## 원본 동작

- 창 상태 플러그인(`tauri-plugin-window-state` 2.4.1 `src/lib.rs`):
    - 플래그는 기본값 `StateFlags::all()`입니다(50-67). 크기·위치·최대화·표시·장식·전체 화면을 모두 다룹니다.
    - 저장 파일은 app config 디렉터리의 `.window-state.json`이고, 창 label을 키로 한 map입니다(36, 121-147). 앱 종료(`RunEvent::Exit`) 때 저장합니다(502-506). 읽기 실패는 빈 map으로 시작합니다(397, 511-521).
    - 창이 준비되면(`on_window_ready`, 407-501) 저장된 상태를 복원합니다(166-268).
        - 위치는 저장된 사각형의 네 모서리 중 하나가 연결된 모니터 안에 있을 때만 적용합니다. 아니면 OS가 위치를 정합니다(189-210, 527-548).
        - 크기(212-217)를 적용한 뒤 최대화였다면 최대화하고(219-221), 전체 화면 여부를 맞춥니다(223-225).
    - 추적은 `Moved`/`Resized`/`CloseRequested` 이벤트입니다(447-500). 최소화·최대화 중에는 위치와 크기를 갱신하지 않고, 0 크기는 저장하지 않습니다(282-321).
- TAIDE 설정(`src-tauri/src/lib.rs` 856-870):
    - `map_label`이 `editor-<n>` 보조 창 label을 `editor` 하나로 모읍니다. `with_filter`가 그 키를 제외하므로 보조 창은 위치/크기를 저장하지도 복원하지도 않습니다.
    - 결과적으로 메인 창(`main`)만 복원합니다. 보조 창은 항상 1000×700 기본 크기로 엽니다(`domain/window/commands.rs` 72-73).
- 전체 화면 복원과 Zen(`use-window-chrome.ts` 84-97): 첫 렌더에서는 fullscreen 명령을 보내지 않습니다. 플러그인이 복원한 전체 화면을 Zen 동기화가 풀지 않게 하기 위해서입니다.
- 세션 복원: 부팅 때 `restore_state`가 세션(프로젝트 순서·활성 프로젝트·slot 트리와 크기·focused slot·window chrome)과 프로젝트별 layout을 읽습니다. 그다음 보조 창(`restore_auxiliary_windows`)과 watcher를 지연 복원합니다(`lib.rs` 913-980). 화면은 `session_get_shell_state`와 `session:*` 이벤트로 그 상태를 그립니다(`session.query.ts`, `ipc-sync-provider.tsx` 348-360).
- 보조 창 복원 규칙(`taide-window/src/service.rs` 29-79): `restorable_project_ids`는 root가 사라진 프로젝트를 뺍니다. `plan_auxiliary_window_restorations`는 프로젝트별 `auxiliary_windows` slot을 기록 순서대로 펼칩니다. `next_auxiliary_label`은 비어 있는 가장 작은 `editor-<n>`을 고릅니다.

## 구현

- 의존성: `native/taide-native-app/Cargo.toml`에 `taide-window = { path = "../../crates/taide-window" }`만 추가했습니다. `Cargo.lock`은 `--offline` 1회 해석으로 `taide-window` 항목과 앱의 의존 목록 한 줄만 늘었습니다. crates.io 항목 버전은 바뀌지 않았습니다. 동결 `taide-remote-web/Cargo.lock`도 바뀌지 않았습니다.
- shell-8 중복 제거(`auxiliary-windows.rs`):
    - 복제했던 `next_label`과 `LABEL_PREFIX`/`FIRST_LABEL_NUMBER`를 지우고 `taide_window::service::next_auxiliary_label`을 씁니다.
    - `restoration_plan`은 원본 `restorable_project_ids`로 복원 대상을 고르고 `plan_auxiliary_window_restorations`로 펼칩니다.
    - 원본은 프로젝트 map 순서로 돌아 순서 계약이 없습니다. native는 label 배정이 실행마다 같도록 세션 프로젝트 순서로 거른 목록을 넘깁니다. 그래서 앞 단계 검사가 그대로 유효합니다.
    - 복제 함수만 단언하던 단위 검사 1건(`보조_창_라벨은_사용중인_가장_작은_editor_번호를_쓴다`)은 지웠습니다. 같은 규칙은 원본 crate 검사와 native `Windows` 검사(`editor-1/2`, 예약 `editor-2/3`, 해제 후 재사용)가 덮습니다.
- shell-3(`window-state.rs`, `main.rs`, `application.rs`):
    - `window_state::main_viewport`가 `--data-dir` 아래 `native-window-state.json`의 `main` 항목을 읽습니다. 저장된 크기·최대화·전체 화면을 eframe 메인 `ViewportBuilder`에 적용합니다. 항목이 없거나 파일이 손상됐으면 기본 1400×900 창을 씁니다. `main.rs`는 런타임 복원 전에 이 함수를 부릅니다.
    - `NativeApplication`의 `window_state::Tracker`는 매 `logic`에서 root viewport의 `ViewportInfo`만 읽습니다. 그래서 보조 창은 기록되지 않고, 원본 `with_filter`처럼 기본 크기로 엽니다.
        - 최소화 중이거나 inner rect가 없거나 크기가 0이면 갱신하지 않습니다.
        - 최대화·전체 화면은 상태만 기록하고 직전 창 크기를 유지합니다.
        - 크기는 `inner_rect × zoom_factor`, 즉 egui 확대 배율과 무관한 논리 point입니다.
    - `App::on_exit`는 정상 닫기와 직접 종료 모두에서 `exit_application`보다 먼저 상태를 원자적으로 저장합니다(`taide_infra::persist::write_json`). 저장 실패는 원본처럼 종료를 막지 않고 경고 로그만 남깁니다.
    - 원본과 다르게 둔 점(근거):
        - 단위는 물리 픽셀이 아니라 논리 point입니다. eframe은 모니터와 배율을 알기 전에 point 단위 `ViewportBuilder`로 창을 만들기 때문입니다.
        - 파일 이름은 원본 `.window-state.json`과 다릅니다. 단위가 다른 파일을 서로 잘못 읽지 않게 하기 위해서입니다.
        - 원본 `prev_x/prev_y`는 이벤트 순서 보정용 내부 값이라 쓰지 않았습니다. 전체 화면 중의 화면 크기를 창 크기로 저장하는 원본 동작도 재현하지 않았습니다.
        - 표시·장식 플래그는 TAIDE 메인 창에서 바뀌지 않으므로 저장하지 않습니다.
    - Zen과의 관계: `zen::Fullscreen::new(zen, zenFullscreen)`가 부팅 첫 프레임에 fullscreen을 보내지 않습니다(shell-19). 그래서 복원된 전체 화면이 원본처럼 유지됩니다.
- shell-25: 세션·layout·보조 창·editor view state 복원 경로는 이미 공유 `restore_state`와 native 부팅에 연결돼 있었습니다. 이번에는 창 크기(shell-3)를 같은 재시작 경로에 붙였습니다. 실제 App 종료 → 디스크 → 새 App 재시작 검사로 묶음 전체를 확인했습니다.

## 검증

- 실제 앱 경로:
    - 크기 복원: `main.rs` → `window_state::main_viewport` → eframe `NativeOptions.viewport`
    - 추적·저장: `NativeApplication::logic`(`background_tick`) → `Tracker::observe`(root `ViewportInfo`) → `App::on_exit` → `Tracker::save`
    - 세션: `bootstrap::restore` → `project_actions::restore_state` → `NativeApplication::new`(`ShellController::connect`, `Windows::restore`, `zen::Fullscreen::new`, `Tracker::new`)
- 검사 순서: 새 검사는 구현과 같은 단계에서 썼습니다. 구현 전에는 `window_state` 모듈과 저장 경로가 없어 컴파일되지 않는 상태였으므로, 구현 전 실패 실행은 따로 하지 않았습니다(구현을 끄는 재현은 작업 규칙상 하지 않음).
- `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline … -- window_state auxiliary_windows::`(중복 제거 직후)는 11/11 통과입니다.
- `cargo test … --lib -- window_state`는 7/7 통과입니다(60.76초).
    - 단위 5건(`window-state-tests.rs`):
        - 저장 없음 → 기본 창, 저장 안 함
        - 크기 추적·`main` 키 하나만 저장·다음 viewport 복원(위치 없음)
        - 최대화·전체 화면 중 직전 크기 유지와 최소화 무시
        - zoom 배율 곱·0 크기 무시
        - 손상 파일 → 기본 창, 다음 저장이 덮어씀
    - 실제 App 2건(`application-window-state-tests.rs`):
        - 메인 root 1180×760과 보조 viewport 900×600을 각각 공급합니다. 종료 뒤 파일에는 `main` 1180×760만 있고, 보조 창 builder는 원본 기본 1000×700에 위치가 없습니다.
        - App1에서 `ResizeSlots`(35/65), `FocusSlot`(beta), `SetWindowChrome`(zen·rail 접힘)을 실제 controller로 제출하고 메인 크기를 공급한 뒤 종료합니다. 같은 `--data-dir`로 `bootstrap::restore`(경고 0) → 새 App을 만들면 프로젝트 순서·활성 beta·slot 분할과 크기·focused slot·zen·rail 접힘·snapshot focused 프로젝트·보조 창 `editor-1`·메인 크기가 그대로입니다. 새 App이 root 정보 없이 종료해도 저장값이 유지됩니다.
    - 첫 실행에서 재시작 검사가 egui 디버그 단언(같은 탐색기 위젯 id가 메인 slot과 embed된 보조 창에 동시에 그려짐)으로 실패했습니다. 실제 앱처럼 보조 창을 별도 immediate viewport로 그리게 검사 하네스를 고친 뒤 통과했습니다. 그 전 실행에서는 App을 fixture 밖으로 꺼낸 채 panic해 종료 정리 없이 drop이 멈췄으므로, App을 fixture 안에 두도록 바꿨습니다(멈춘 실행은 중단).
- 변경과 직접 맞닿은 기존 검사: `cargo test … --lib -- auxiliary_window exit_tests`는 17/17 통과입니다(107.89초). 보조 창 실제 App 검사 전부, `auxiliary_windows` 단위, `on_exit`를 쓰는 종료 검사를 포함합니다.
- `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets --locked --offline`은 exit 0입니다(기존 경고만 있음).
- `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline`은 exit 0입니다. remote-web `Cargo.lock`은 바뀌지 않았습니다.
- `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check`: 이번에 추가하거나 바꾼 부분에는 차이가 없습니다. 남은 차이는 모두 기존 코드입니다(`experiments/lsp-coordinator-spike/src/bin/mock-server.rs`, `application.rs` 937행 들여쓰기·테스트 모듈 순서, `lib.rs`의 `editor_schema_transport`/`open_path_dialog`/`welcome` 순서). 요청 밖 재포맷이라 손대지 않았습니다.
- 실행하지 않은 것: 필터 없는 앱 전체 테스트(배치 끝 메인 담당), GUI 실기(실제 창 크기·전체 화면 복원 확인은 배치 31).

## 남은 범위

- 후속: 아래 위치 복원과 잔여 mirror 정리는 단계 E 결정 A안에 따라 `2026-10-27-native-window-position-mirror-prune.md`에서 구현했습니다.
- shell-3 위치 복원(결정 필요): 원본은 저장 사각형이 연결된 모니터와 겹칠 때만 위치를 적용합니다. native에서 같은 보정을 하려면 창 생성 전에 모니터 목록이 필요합니다.
    - 현재 의존성으로는 얻을 방법이 없습니다. eframe은 `persistence` 기능이 꺼져 있고, winit은 dev-dependency뿐이고, `objc2-app-kit`의 `NSScreen` 기능은 앱이 선언하지 않았습니다(다른 crate의 기능 통합에 기대는 것은 우회라 쓰지 않음).
    - 보정 없이 위치를 복원하면 모니터를 뺀 뒤 창이 화면 밖에 열릴 수 있습니다. 그래서 위치는 아직 복원하지 않고 OS 기본 배치를 씁니다(winit 0.30.13 macOS `window_delegate.rs` 629-630의 `center`). 원본의 "모니터 없음" 분기와 같은 결과입니다.
- shell-3 macOS zoom: egui-winit 0.36.2(`lib.rs` 1382-1388, egui#3494)는 macOS에서 실행 중 `maximized`를 갱신하지 않습니다. 그래서 zoom된 창 크기를 일반 크기로 기록합니다. Windows/Linux는 egui가 갱신하는 값을 그대로 씁니다.
- shell-25 잔여 hot-exit mirror 정리: 원본 `src/widgets/editor-area/editor-area.tsx` 409-443은 프로젝트가 editor 영역에 처음 보일 때 열린 파일 탭 밖의 path mirror와, 열린 탭·최근 닫힘 밖의 untitled mirror를 지웁니다(`file_prune_mirrors`, `file_prune_untitled_mirrors`). native에는 이 sweep이 없습니다.
    - native mirror는 canonical 경로를 표시 경로로 기록합니다(`persistence.rs` `write_mirror`). 그래서 탭 경로 문자열만 비교하는 원본 keep 목록을 그대로 넘기면, 열린 탭의 mirror를 지울 위험이 있습니다. canonical 비교(`root_guard::canonicalize_lenient`)와 쓰기 경쟁(mutation guard 안에서 keep 목록 계산)을 함께 설계해야 해서 이번 단계에서는 구현하지 않았습니다.
- 실기(실제 OS 창 크기·전체 화면 복원, 재실행)는 배치 31입니다.
