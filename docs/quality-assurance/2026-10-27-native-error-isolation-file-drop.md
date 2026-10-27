# 장애 격리·OS 파일 drop (배치 24 단계 E 3/3)

## 대상

- shell-12: 전역 ErrorBoundary(위젯별 fallback). 원본은 `src/shared/ui/error-boundary.tsx`, `src/widgets/app-shell/app-shell.tsx`입니다.
- shell-22: OS 파일/폴더 drop(폴더는 프로젝트 열기, 파일은 preview 탭)과 DragDropOverlay. 원본은 `src/widgets/app-shell/app-shell.tsx` 99-135·149·176, `src/features/window/drag-drop-overlay.tsx`입니다.

## 원본 동작

### shell-22 OS 파일/폴더 drop

- 이벤트(`app-shell.tsx` 37, 124-135, 149): 메인 창 `AppShell`만 `getCurrentWebview().onDragDropEvent`를 구독합니다. 보조 창(`auxiliary-window-shell.tsx`)에는 구독이 없습니다.
    - `enter`·`over`는 overlay를 켭니다. `leave`는 끕니다. `drop`은 끄고 `handleDroppedPaths(paths)`를 부릅니다.
- 처리(`app-shell.tsx` 99-122): 경로를 순서대로 하나씩 기다리며 처리합니다. 대상 프로젝트의 시작값은 focused slot의 프로젝트(`useShellSlotFocus`)입니다.
    - 먼저 `openProject(path)`(`project_open`)를 시도합니다. 성공하면 그 프로젝트가 이후 경로의 대상이 됩니다.
    - 실패하면(오류 종류를 가리지 않음) 대상이 없을 때 `toast.info(t('app.openProjectFirst'))`만 띄웁니다. 대상이 있으면 `openTabInProject({ kind: file, path, title: fileNameOf(path), target: null, preview: true })`로 파일 탭을 엽니다. 이 열기가 실패하면 `toast.error(describeIpcError(error))`입니다.
    - `project_open` 실패 자체는 알리지 않습니다(`catch`로 삼킴).
- 열기 경로:
    - `project_open`(`crates/taide-runtime/src/project_actions.rs` 336-366)은 `taide-project` `open_project` → `validate_open_root`(`crates/taide-project/src/service.rs` 414-435)를 거칩니다. 디렉터리가 아니면 `error.project.pathNotDirectory`로 실패합니다. 이미 열린 root는 다시 활성화합니다.
    - 파일 탭은 `layout_open_tab`(`crates/taide-runtime/src/layout_actions.rs` 238-274)입니다. `ensure_file_tab_target_exists`가 열린 프로젝트 안의 기존 파일인지 확인합니다. 프로젝트 밖이면 Forbidden, 없으면 NotFound입니다.
    - `target: null`은 메인 창에서 layout `focused_pane`으로 해석됩니다(`layout.query.ts` 101-104·134-138, `pane-tree.ts` 145-149·181-182).
- overlay(`drag-drop-overlay.tsx` 9-15, `app-shell.tsx` 176): app-shell 전체(`absolute inset-0`, 제목줄 포함)를 덮습니다. `pointer-events-none`, `z-50`입니다. Zen 안내(`zen-mode-hint.tsx` 28, `z-20`)보다 위이고, body portal(대화상자·메뉴)과 toast보다 아래입니다.
    - 배경 `bg-app-background/85`, 테두리 `border-4 border-dashed border-app-accent`입니다.
    - 가운데 세로 배치(`gap-3`)로 `FolderInput` 아이콘(`size-10`, `text-app-accent`)과 `app.dropToOpen` 문구(`text-sm font-medium text-app-foreground`)를 보입니다.

### shell-12 ErrorBoundary

- 컴포넌트(`error-boundary.tsx` 32-83, 86-126): 자식의 render/effect 오류를 잡아 그 영역만 fallback으로 바꿉니다.
    - fallback은 `role='alert'`, 영역 이름(`labelKey`), `errorBoundary.description`, 개발 빌드에서만 stack, `common.retry` 버튼입니다. 전환 때 fallback에 focus를 옮깁니다.
    - 재시도는 state를 초기화해 자식을 새로 mount합니다. React가 잡은 순간 그 subtree의 fiber를 버리므로, 재시도는 깨진 인스턴스를 재사용하지 않는 새 mount입니다(115-124).
- 경계 위치:
    - 루트 `errorBoundary.app`(`main.tsx` 45, `onCaught`로 창 표시)
    - `app-shell.tsx`: Welcome 185, Activity Bar 195(`w-14`), 상태 표시줄 227(`h-6`)
    - `project-shell.tsx`: 사이드바 패널 150, 에디터 156
    - 보조 창 `auxiliary-window-shell.tsx` 132·138(사이드바 패널, 에디터)

## 구현

### shell-22

- 입력 근거(설치·vendored 소스):
    - winit 0.30.13 macOS `window_delegate.rs` 367-430은 NSWindow의 drag 대상 메서드입니다. `draggingEntered:`에서 파일마다 `HoveredFile`, `performDragOperation:`에서 파일마다 `DroppedFile`, `draggingExited:`에서 `HoveredFileCancelled`를 보냅니다. 665-669에서 창을 `NSFilenamesPboardType`에 등록합니다. 파일 경로가 없는 drag(텍스트 등)는 거절합니다.
    - egui-winit 0.36.2 `lib.rs` 457-483은 `HoveredFile`을 `RawInput.hovered_files`에 쌓고, 취소 때 비우고, drop 때 hovered를 비운 뒤 `dropped_files`에 넣습니다.
    - vendored egui `raw_input.rs` 122-139의 `RawInput::take`는 hovered를 복제하고 dropped를 옮깁니다. 그래서 같은 프레임의 재처리 pass에는 drop이 다시 오지 않고, hover는 유지됩니다.
- 새 모듈 `native/taide-native-app/src/file-drop.rs`:
    - `is_hovering`: 현재 viewport의 `hovered_files`가 비어 있지 않은지입니다. 원본 `enter/over → true`, `leave/drop → false`와 같은 수명입니다.
    - `dropped_paths`: 그 pass의 `dropped_files`를 OS 순서대로 문자열로 읽습니다. UTF-8이 아닌 경로는 건너뜁니다(아래 차이 참고).
    - `paint_overlay`: 창 전체를 `app.background` 85% 불투명도로 채우고, `app.accent` 4px 점선 테두리, 가운데 40px `FolderInput` 아이콘, 12px 간격, 14px medium `app.dropToOpen` 문구(줄 높이 20px)를 그립니다. 아이콘은 lucide-react 1.28.0 `folder-input` 노드를 `resources/icons/folder-input.svg`로 옮겨 기존 `navigation_icons::Icons` 래스터화 경로에 추가했습니다(ISC 고지는 같은 폴더의 `LICENSE.txt`).
    - 색은 `presentation_refresh::Appearances.file_drop`으로 테마마다 다시 계산합니다(Zen 안내 색과 같은 갱신 지점 2곳).
- 앱 연결(`application.rs`):
    - `show_window`의 메인 창 범위(`WindowScope::Main`)에서 셸과 Zen 안내를 그린 다음 overlay를 그립니다. 보조 창 범위에서는 아무것도 하지 않습니다(원본 보조 창에 구독 없음).
    - drop 경로가 있으면 `HostCommand::OpenDroppedPaths { focused, paths }`를 보냅니다. `focused`는 snapshot의 focused slot 프로젝트입니다. 종료 중(`closing`·`is_exit_ready`·shutdown)에는 보내지 않습니다. 앱 메뉴 `Open Recent`와 같은 조건입니다.
    - `HostReply::DroppedPathsOpened`의 알림을 순서대로 `app.openProjectFirst` info toast와 기존 IPC 오류 toast(`report`)로 보입니다.
    - overlay가 보이는 동안 WKWebView 미리보기를 숨깁니다(`web_enabled`에 `!file_drop::is_hovering` 추가). 기존 탭·프로젝트 drag 중 숨김과 같은 방식입니다. 그래서 overlay가 미리보기 영역도 덮고, 그 위치에 놓아도 winit drop으로 처리됩니다.
- 호스트(`host.rs` `open_dropped_paths`): 원본 루프를 worker 한 번의 직렬 처리로 옮겼습니다.
    - 경로마다 기존 `HostCommand::OpenProject`와 같은 순서(`validate_open_root` → `flush_view_states` → `NativeProjects::open`)로 프로젝트를 엽니다. 성공하면 대상을 바꿉니다.
    - 실패하면 대상이 없을 때 `Notice::OpenProjectFirst`, 있으면 기존 `HostCommand::OpenFileTab`과 같은 순서(`flush_view_states` → `open_file_tab(.., pane: None, preview: true)`)로 엽니다. 실패는 `Notice::Failed(error)`입니다.
    - 원본의 순차 await와 같은 순서를 지키고, 알림은 한 응답에 모아 순서대로 보입니다.
- 원본과 다르게 둔 점(근거):
    - 레이어: overlay는 egui `Order::Middle` 전용 레이어입니다. 셸 본문(Background)과 Middle area 위, 모달·메뉴(Foreground area)와 toast(Tooltip) 아래라 원본의 portal·toast 순서와 같습니다. Zen 안내는 Foreground 비-area 레이어라 overlay 위에 그려집니다. 원본(`z-20 < z-50`)과 반대인데, overlay를 Foreground로 올리면 모달·메뉴 위로 올라가는 다른 차이가 생겨 이 배치를 골랐습니다. Zen 진입 뒤 3초 동안만 겹칩니다. 편집기 completion·문서 hover popup도 `Order::Tooltip`(`taide-native-ui/src/editor-completion.rs` 1340·1666, `editor-documentation.rs` 589·734)이라 overlay 위에 남습니다(원본은 Monaco DOM 안이라 overlay 아래).
    - 점선 길이: CSS `dashed`의 dash·간격 길이는 UA가 정합니다. 굵기의 3배(12px) dash와 같은 간격으로 그리고 각 변을 모서리에서 시작했습니다. 픽셀 대조는 배치 31입니다.
    - 파일 경로가 없는 drag: Tauri/wry는 경로가 비어도 `enter`를 보내 원본은 텍스트 drag에도 overlay를 보입니다(놓아도 아무 일 없음). winit은 이런 drag를 거절하므로 native는 overlay를 보이지 않습니다. 원본 결함은 재현하지 않았습니다.
    - UTF-8이 아닌 경로: 원본은 Tauri 이벤트 직렬화(serde `PathBuf`)가 실패해 그 drop 전체가 전달되지 않습니다. native는 그 경로만 건너뛰고 나머지를 처리합니다. macOS는 `NSString` 경로라 항상 UTF-8입니다.

### shell-12 (구현하지 않음, 결정 필요)

native에서 원본 "render 오류"에 해당하는 것은 두 가지입니다. 하나는 `Result` 오류인데, 이미 상태줄·toast로 보이고 앱은 계속 동작합니다. 다른 하나는 panic(버그)입니다. 영역 경계는 panic을 잡아야 의미가 있어 `std::panic::catch_unwind`를 검토했습니다. 다음 이유로 지금 구조에서는 안전하지 않다고 판단했습니다.

- 영역 render가 공유 상태를 직접 바꿉니다. 그래서 React의 "subtree를 버리고 새로 mount"에 해당하는 버릴 상태가 영역 안에 없습니다.
    - 에디터 영역은 `NativeEditor::show_request(ui, store: &mut EditorStore, ..)`(`taide-native-ui/src/editor_surface.rs` 1280-1292)로 그리는 중에 입력 편집을 공유 `EditorStore`(모든 창·pane의 문서와 undo)에 적용합니다. 편집 중 panic을 잡고 계속하면 일관성이 깨진 문서가 이후 저장·hot-exit mirror로 디스크에 쓰일 수 있습니다.
    - 탐색기·상태바·rail도 `AppSurfaces`의 `&mut` 앱 상태(프로젝트별 explorer 상태, status, 아이콘 캐시, 명령 큐)를 씁니다.
- `&mut` 상태와 `egui::Ui`는 `UnwindSafe`가 아닙니다. 잡으려면 `AssertUnwindSafe`로 "panic 뒤에도 일관적"이라고 단언해야 하는데, 위 근거로 그 단언이 성립하지 않습니다.
- egui `Context`와 runtime `AppState`는 `parking_lot` 잠금(`epaint-0.36.2/src/mutex.rs` 1-109)이라 poisoning이 없습니다. 잠금 안에서 panic하면 반쯤 바뀐 상태가 감지되지 않고 다음 프레임에 그대로 쓰입니다. `Area`·`Panel`의 끝 등록도 건너뜁니다.
- runtime `AppState`도 같은 `parking_lot` 잠금입니다(`crates/taide-runtime/src/state.rs` 8).
- 지금 panic은 앱을 끝냅니다. winit 0.30.13 macOS `event_loop.rs` 436-460의 `stop_app_on_panic`이 NSApplication을 멈추고, 307·374에서 panic을 다시 전파합니다. 앱 `Cargo.toml`에 `panic = "abort"` 설정은 없습니다. 다음 실행은 hot-exit mirror와 세션 복원(shell-25)으로 복구합니다.

선택지:

- A안(추천): 영역 render를 "snapshot 읽기 + 영역 소유 상태 + intent 출력"으로 바꾼 영역에만 `catch_unwind` 경계를 둡니다. panic 때는 영역 소유 상태와 그 pass의 intent를 버리고 fallback을 그리며, 재시도는 새 상태로 다시 그립니다. Welcome·Activity Bar·상태 표시줄부터 적용합니다. 에디터·사이드바 패널은 공유 상태 변경(문서 편집, explorer 상태)을 render 밖 트랜잭션으로 옮기는 별도 설계 배치 뒤에 적용합니다.
- B안: in-process 격리를 하지 않습니다. panic은 앱 종료로 두고 hot-exit·세션 복원으로 복구합니다. 원본 fallback UI와 다르므로 대응표에 차이로 기록합니다.
- C안(비추천): 모든 경계를 `AssertUnwindSafe`로 감쌉니다. 에디터 편집 중 panic 뒤 깨진 문서가 저장될 수 있습니다.

fallback UI(영역 이름 키 `errorBoundary.*`, `errorBoundary.description`, `common.retry`, 개발 빌드 stack, fallback focus)는 결정 뒤 같은 locale 키로 구현합니다.

## 변경 파일

- 새 파일: `native/taide-native-app/src/file-drop.rs`, `src/file-drop-tests.rs`, `src/application-file-drop-tests.rs`, `resources/icons/folder-input.svg`
- `src/lib.rs`: `file_drop` 모듈 연결. `HostReply`가 `Notice`를 실어 `pub mod`입니다.
- `src/host.rs`: `HostCommand::OpenDroppedPaths`, `HostReply::DroppedPathsOpened`, `open_dropped_paths`
- `src/application.rs`: 색 필드와 테마 갱신 2곳, `show_window` 메인 범위의 overlay·drop 제출, 응답 처리, `web_enabled` 조건, 검사 모듈 연결
- `src/presentation-refresh.rs`: `Appearances.file_drop`
- `src/navigation-icons.rs`: 기존 lucide 래스터화 경로에 `FolderInput` 1개 추가. 세 번째 래스터화 복제를 만들지 않으려고 이 경로를 재사용했습니다(지정 범위 밖 최소 수정).
- Cargo.toml·Cargo.lock, `taide-native-ui`, 동결 remote-web은 바꾸지 않았습니다.

## 검증

- 실제 앱 경로: `main.rs` → `NativeApplication::ui` → `frame_ui` → `show_window`(메인 범위) → `file_drop::paint_overlay`·`HostCommand::OpenDroppedPaths` → `HostBridge` worker `open_dropped_paths` → `HostReply::DroppedPathsOpened` → toast. 미리보기 숨김은 `finish_frame`의 `web_enabled`입니다.
- 검사 순서: 새 검사는 구현과 같은 단계에서 썼습니다. 구현 전에는 `file_drop` 모듈과 명령이 없어 컴파일되지 않으므로, 구현 전 실패 실행은 따로 하지 않았습니다(구현을 끄는 재현은 작업 규칙상 하지 않음). 첫 실행에서 단위 2건이 검사 하네스 결함(`run_ui` 결과의 `TexturesDelta`를 비우지 않고 drop)으로 실패했고, 하네스를 고친 뒤 통과했습니다. 같은 실행에서 실제 App 2건은 통과했습니다.
- `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir … -- file_drop`는 4/4 통과입니다(39.24초).
    - 단위 2건(`file-drop-tests.rs`):
        - hovered 있음/없음의 안내 상태와 drop 경로의 OS 순서
        - UTF-8이 아닌 경로만 건너뜀(unix)
    - 실제 App 2건(`application-file-drop-tests.rs`, `RawInput.hovered_files`/`dropped_files` 합성):
        - 프로젝트 0개 Welcome에서 hover 동안 `app.dropToOpen` 문구, 화면 전체 85% 배경 사각형, accent 4px 점선 조각을 그립니다. hover가 끝나거나 drop 프레임이면 지웁니다. 프로젝트 없이 파일을 놓으면 `app.openProjectFirst` info toast이고 프로젝트는 열리지 않습니다.
        - `[폴더, 그 안 파일]`을 놓으면 폴더를 프로젝트로 열고 활성화(focused)한 뒤 그 파일을 preview 탭으로 엽니다. 다음 파일 drop은 focused 프로젝트의 preview 탭입니다. 프로젝트 밖 파일은 Error toast이고 탭·프로젝트가 늘지 않습니다.
- 변경과 맞닿은 기존 검사: `cargo test … --lib -- native_settings_p`는 2/2 통과입니다(0.20초). 모든 내장 테마에서 `Appearances::new`가 해석되는지(새 `app.accent` 등 포함)와 설정 미리보기 appearance 검사입니다.
- `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets --locked --offline …`는 exit 0입니다(기존 경고만 있음).
- `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline …`는 exit 0입니다(기존 경고만). remote-web `Cargo.lock`은 바뀌지 않았습니다.
- `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check`: 이번에 추가하거나 바꾼 부분에는 차이가 없습니다. 남은 차이는 모두 기존 코드입니다(`experiments/lsp-coordinator-spike/src/bin/mock-server.rs`, `application.rs` 958행 들여쓰기·테스트 모듈 순서, `lib.rs` 모듈 순서).
- 실행하지 않은 것: 필터 없는 앱 전체 테스트(배치 끝 메인 담당), GUI 실기와 실제 Finder drag(배치 31). `web_enabled` 숨김 조건은 실제 WKWebView 자식 창이 필요해 자동 검사가 없습니다.

## 남은 범위

- shell-22:
    - 창 가장자리에 붙은 HTML/미디어 미리보기(WKWebView) 위로 곧장 들어온 drag는 WKWebView가 대상이 됩니다. winit 창 delegate가 이벤트를 받지 못해 overlay와 열기가 없습니다. 앱은 wry `with_drag_drop_handler`를 설정하지 않아 기본 처리기(`false`)로 WKWebView 기본 동작이 실행되고, navigation 처리기가 파일 이동을 막아 아무 일도 일어나지 않습니다(`vendor/wry-preview/src/wkwebview/drag_drop.rs`). 처리기를 연결해 경로를 앱으로 넘기고, 미리보기를 숨긴 뒤 AppKit이 창으로 대상을 바꾸는지 실기로 확인해야 합니다.
    - Zen 안내·편집기 popup이 overlay 위에 그려지는 순서 차이, 점선 dash 길이·모서리 픽셀, 실제 OS drag는 배치 31입니다.
- shell-12: 결정 필요(위 A/B/C안). 결정 전에는 구현하지 않습니다.
