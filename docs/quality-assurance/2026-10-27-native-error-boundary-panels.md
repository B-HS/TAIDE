# 사이드바 패널 장애 격리·미리보기 위 OS drag (배치 24 단계 E2 3/3)

## 대상

- shell-12: 전역 ErrorBoundary(위젯별 fallback). 이번 범위는 사이드바 패널 경계(메인 창 slot과 보조 창)입니다. 편집기 영역 경계는 남은 범위입니다.
- shell-22 남은 범위: HTML/미디어 미리보기(WKWebView) 위로 들어온 OS drag를 앱 drop 처리로 넘깁니다.
- 사용자 결정: A안(`2026-10-27-native-error-isolation-file-drop.md` shell-12 절). render 안의 공유 상태 변경을 render 밖 적용으로 옮긴 영역에만 경계를 둡니다.
- 원본: `src/widgets/app-shell/project-shell.tsx`, `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx`, `src/widgets/explorer/explorer-container.tsx`

## 원본 동작

### shell-12 사이드바 패널 경계

- 위치:
    - `project-shell.tsx` 142-153: 탐색기 `Panel`(기본 240px, 최소 180px, 최대 40%, 접기 가능) 안에 `ErrorBoundary labelKey='errorBoundary.sidebarPanel'`이 `ExplorerContainer`를 감쌉니다. 패널 크기·접힘과 구분선은 경계 밖입니다.
    - `auxiliary-window-shell.tsx` 131-135: 보조 창도 같은 경계를 둡니다.
    - 편집기 영역 경계(`errorBoundary.editorArea`)는 `project-shell.tsx` 155-159, `auxiliary-window-shell.tsx` 137-141입니다.
- 경계 안의 상태(잡히면 함께 사라지고 재시도는 새 mount):
    - `explorer-container.tsx` 58-61: 보기(files/outline), 선택, 경로 선택 요청, 비교 원본
    - `use-explorer-clipboard.ts` 36: 탐색기 clipboard
    - `use-explorer-entry-crud.ts` 57-64: 새 항목 draft, 이름 바꾸기 대상과 오류, 삭제 대상
    - `explorer-container.tsx` 316-320: 삭제 확인 `EntryDeleteDialog`도 경계 안이라 함께 닫힙니다.
    - `explorer-panel.tsx` 113: 검색 요청
- 경계 밖에 남는 상태: tree 펼침(Rust tree 서비스), layout·설정 query, 열린 탭과 문서
- fallback·재시도·focus·개발 빌드 stack은 공용 경계와 같습니다(`2026-10-27-native-error-boundary-foundation.md`).

## 구현

### 공용 경계(`native/taide-native-ui/src/error-boundary.rs`)

- `Region::SidebarPanel`(`errorBoundary.sidebarPanel`)을 더했습니다.
- `show_instance(ui, rect, (region, instance), ..)`: 한 창에 여러 번 나오는 영역(slot마다 하나인 사이드바 패널)이 instance마다 따로 실패 상태와 generation을 갖습니다. 기존 `show`는 `Id::NULL` instance입니다.
- 검사 전용 주입도 instance 단위입니다(`inject_instance_panic`). 기존 `inject_panic`은 `Id::NULL`입니다.

### 셸(`native/taide-native-ui/src/shell.rs`)

- `ShellSurfaces`의 탐색기 API를 경계용으로 바꿨습니다.
    - `type Explorer`: 패널 render가 바꿀 수 있는 영역 소유 상태와 그 render의 요청을 담는 값
    - `begin_explorer(&mut, project, mount)`: 경계 밖에서 상태를 꺼냅니다.
    - `explorer(&self, ui, project, mount, &mut Explorer, intents)`: 경계 안 render입니다. 호스트는 읽기만 합니다.
    - `finish_explorer(&mut, ctx, ..)`: render가 끝났거나(`Shown`) 경계가 fallback이라 render를 건너뛴 경우(`Fallback`) 상태를 돌려놓고 요청을 적용합니다.
    - `discard_explorer(&mut, .., Explorer)`: render가 panic한 경우(`Caught`)입니다. 상태와 요청을 버리고 새 mount로 시작합니다.
    - `ExplorerMount::{Slot, Auxiliary}`: 메인 창 slot과 보조 창을 같은 경로로 다룹니다. 기존 `auxiliary_explorer`는 없앴습니다.
- `sidebar_panel`: 탐색기 `egui::Panel` 안쪽 rect에 경계를 둡니다. 패널 너비·접힘은 원본처럼 경계 밖입니다. fallback 색은 `fallback_colors`로 기존 세 경계와 공유합니다.
- instance id는 메인 창 `slot`, 보조 창 `(project, slot)`입니다.

### 앱(`native/taide-native-app/src/application.rs` `AppSurfaces`)

- `ExplorerRegion`이 영역 소유 상태와 요청을 담습니다.
    - 꺼내는 상태: slot의 보기 상태(`symbol_sidebar::Views::take`, 보기·outline), 프로젝트 탐색기 상태(`explorers`에서 제거), clipboard owner(`explorer_clipboards.mount`)
    - render 안에서는 꺼낸 상태만 바꿉니다. 호스트는 읽기(`trees`, `store.documents()`, `editor_symbols`, `layouts`, clipboard 조회)만 합니다.
    - 요청: 탐색기 `Action` 목록과 보조 창 대상 pane, outline reveal(`OutlineReveal`), tree 조회 요청, clipboard reveal 소비 표시, 오류 문구, 지연 아이콘
- 적용(`finish_explorer`): 기존 render 안의 처리 순서를 그대로 옮겼습니다.
    - 상태 반납, 오류 문구, 지연 아이콘 채우기, tree 조회
    - outline reveal: untitled 문서는 `editor_reveal::reveal_document`(공유 `EditorStore` 선택 변경)와 편집기 본문 focus, 파일은 `reveals.queue_position`
    - clipboard reveal 소비(`take_reveal`), `Action` 처리(명령, rename·create·paste·delete 요청, `open_with`), clipboard 반영
- 버림(`discard_explorer`):
    - 보기 상태는 돌려놓지 않습니다. 다음 render가 새 상태(files 보기)로 시작합니다.
    - clipboard owner를 내립니다(`ClipboardOwners::unmount`). 다음 mount는 새 owner라 이전 owner로 온 붙여넣기 응답은 닿지 않습니다. 원본 unmount 뒤 응답 처리가 무시되는 것에 대응합니다.
    - 탐색기 상태는 `Explorer::remounted`(새 상태, 요청 token 번호만 이어감)로 바꿉니다. token이 다시 1부터 시작하면 panic 전 요청의 응답이 새 요청과 맞을 수 있어 번호만 잇습니다.
    - 그 프로젝트의 삭제 확인 대화상자를 닫습니다(원본 `EntryDeleteDialog`가 경계 안).
- 아이콘: 탐색기 행·보기 전환·outline 아이콘은 공유 texture cache(`&mut`)로 그렸습니다. render가 cache를 바꾸지 않도록 `navigation_icons::Deferred`가 rail과 같은 방식으로 자리(`Shape::Noop`)만 예약하고, 적용 때 `prepare`+`shape`로 채웁니다(`Painter::set`).
    - `PaintIcon<T>` trait로 즉시 그리기(texture cache)와 예약을 같은 호출로 씁니다. 기존 단위·통합 검사(`tests/explorer.rs`, symbol 검사)는 즉시 그리기를 그대로 씁니다.
    - navigation 아이콘에 `prepare`/`shape`를 나눴습니다. mesh는 기존 `Image::paint_at`과 같은 계산입니다(glyph 아이콘과 같은 근거).

### 원본과 다르게 둔 점(근거)

- 탐색기 상태 공유: native 탐색기 상태는 프로젝트 단위로 메인 slot과 보조 창이 함께 씁니다(`mount`로 요청 주인을 가림). 원본은 패널마다 따로입니다. 한 패널이 잡히면 같은 프로젝트의 다른 패널 선택·draft도 새로 시작합니다.
- 아이콘 오류: 즉시 그리기에서는 보기 전환 아이콘 오류가 나머지 전환 버튼 그리기를 멈췄습니다. 지연 채우기에서는 모든 자리를 채워 보고 첫 오류를 상태 문구로 보입니다.
- 검색·git·비교 원본: native 사이드바에는 아직 이 보기가 없어 해당 상태도 없습니다.
- 적용 단계(`finish_explorer`) 자체의 panic은 경계 밖이라 지금처럼 앱 종료입니다. 공유 상태를 바꾸는 코드는 모두 이 단계에 있어, 경계가 잡은 panic 뒤에는 반쯤 바뀐 공유 상태가 남지 않습니다.

### shell-22 미리보기 위 OS drag

- 원본 근거:
    - 원본 미리보기는 앱 webview 안의 문서라 OS drag가 `AppShell`의 `onDragDropEvent`(`app-shell.tsx` 124-135)로 갑니다.
    - Tauri는 webview마다 wry `with_drag_drop_handler`를 연결해 Enter·Over·Drop·Leave를 창 이벤트로 넘기고 `true`(OS 기본 처리 막음)를 돌려줍니다(`tauri-runtime-wry` 2.11.4 `src/lib.rs` 4862-4896).
- vendored wry(`native/taide-native-app/vendor/wry-preview/src/wkwebview/drag_drop.rs` 35-104): 처리기가 `false`면 WKWebView 기본 처리(`super`)를 부르고, `true`면 drag를 받아 `Copy`로 답합니다. 경로는 `NSFilenamesPboardType`에서 읽습니다.
- 구현:
    - `file_drop::WebDrags`(`file-drop.rs`): web view(탭)별 hover와 놓인 경로를 담는 공유 상태입니다.
        - `enter`는 파일 경로가 있을 때만 hover로 받습니다. 경로가 없는 drag(텍스트 등)는 OS 기본 처리에 둡니다. winit 창 drag와 같은 기준입니다(앞 단계 "파일 경로가 없는 drag" 근거).
        - `over`는 받은 drag만 계속 받습니다. `leave`와 drop은 hover를 끝냅니다. drop은 받은 drag의 경로(UTF-8)만 쌓습니다.
        - `forget`은 web view가 사라져 더 이상 이벤트가 오지 않는 탭의 hover를 지웁니다.
    - `preview_web_view.rs`: 미리보기 web view를 만들 때 처리기를 연결하고(`handle_drag`), 이벤트마다 repaint를 요청합니다. web view를 없앨 때(`retain`, `clear`) `forget`을 부릅니다.
    - `application.rs` 메인 창 범위: overlay 조건을 `file_drop::is_hovering || web_views.drags().is_hovering()`로, drop 경로를 winit drop + `take_dropped()`로 넓혔습니다. 같은 `HostCommand::OpenDroppedPaths`와 종료 중 차단 조건을 씁니다. `finish_frame`의 `web_enabled`도 web view hover 동안 미리보기를 숨깁니다.
- web view는 메인 창(ROOT)에만 붙으므로(`frame_pass.web_placements`) 보조 창 drop을 무시하는 기존 규칙과 겹치지 않습니다.

## 검증

- 실제 앱 경로:
    - 사이드바 패널: `main.rs` → `NativeApplication::ui` → `show_window` → `NativeShell::show` → `slot_tree`/보조 창 범위 → `sidebar_panel` → `error_boundary::show_instance` → `AppSurfaces::{begin_explorer, explorer, finish_explorer, discard_explorer}`
    - 미리보기 drag: `finish_frame` → `preview_web_view::Views::sync`(처리기 연결) → wry 처리기 → `WebDrags` → 다음 `show_window` 메인 범위의 overlay·`OpenDroppedPaths`
- 검사 순서: 새 검사는 구현과 같은 단계에서 썼습니다. 구현 전에는 새 trait 메서드·`Region::SidebarPanel`·`WebDrags`가 없어 컴파일되지 않으므로 구현 전 실패 실행은 따로 하지 않았습니다(구현을 끄는 재현은 작업 규칙상 하지 않음). 첫 실행의 실패 2건은 검사 쪽 결함이었습니다. panic frame은 discard 재처리로 같은 frame에 두 pass를 그려 검사 surfaces 기록이 두 번 쌓이는데, 개수를 1로 단언했습니다. 포함 여부 단언으로 고친 뒤 통과했습니다.
- `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test workbench --locked --offline --target-dir …`: 40/40 통과(22.25초). 새 2건:
    - 메인 창 두 slot: 첫 slot 패널 render가 실제로 panic하면 그 slot의 render 기록이 버려지고 `discard_explorer`가 한 번 불립니다. 패널 rect 그대로의 `Alert`에 focus가 있고, 영역 이름·설명·재시도와 개발 빌드 stack이 보입니다. 다른 slot 패널·편집기·상태 표시줄은 그대로입니다. panic이 계속되는 동안 fallback이 유지되고 `begin_explorer`는 slot마다 불리며 버림은 다시 불리지 않습니다. 재시도 뒤 두 패널이 같은 rect로 다시 그려집니다.
    - 보조 창: 주입 panic은 보조 창 탐색기만 fallback으로 바꾸고 편집기를 유지합니다. 재시도 뒤 탐색기가 돌아옵니다.
- `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib …`: 163/163 통과(0.54초)
- `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir … -- error_boundary_tests`: 3/3 통과(54.22초). 새 실제 App 1건:
    - 행 `a.txt`가 보이고, 보기 전환 아이콘이 지연 경로로 채워집니다(`native-navigation-FolderTree-*` texture를 쓴 mesh).
    - 행을 누르면 선택이 확정됩니다. 이어서 패널 panic 주입과 같은 frame에 `⌘⌫`(삭제 요청)를 보내면 fallback이 보이고 rail과 문제 토글은 남습니다. 삭제 확인은 열리지 않고(그 pass 요청 폐기), 선택은 비고(탐색기 상태 새 mount), clipboard owner는 바뀝니다.
    - 재시도 뒤 행이 다시 보입니다. 같은 행 선택 뒤 `⌘⌫`는 삭제 확인을 엽니다(대조군). outline 보기에서 다시 panic하면 열린 삭제 확인이 닫히고 보기는 files로 돌아옵니다.
- `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib … -- file_drop preview_web_view`: 8/8 통과(54.63초). 새 2건:
    - `preview_web_view` 단위(macOS): 파일 없는 Enter·그 뒤 Over·Drop은 기본 처리(`false`)이고 상태가 없습니다. 파일 Enter→Over→Leave, Enter→Drop은 받아서(`true`) 경로를 한 번만 넘깁니다. `forget` 뒤 Over는 기본 처리입니다.
    - 실제 App: web view hover 동안 `app.dropToOpen` overlay를 그리고, web view drop은 overlay를 지우고 폴더를 프로젝트로 엽니다.
- 바꾼 경로와 맞닿은 기존 검사:
    - `cargo test … --lib -- tab_menu_tests auxiliary_window_tests 아웃라인 json_tests symbol_ navigation_icons explorer error_boundary_tests`: 53건 중 52 통과(203.44초). 실패 1건은 `startup_tests::actual_app의_아웃라인_workspace_심볼과_구문_접기는_현재_pane과_reveal을_소비한다`의 마지막 종료 대기(`phase=navigation-exit`, 10초 제한, tasks=2)입니다. 아웃라인 reveal 단언을 모두 지난 뒤 종료 drain의 시간 초과입니다. 같은 검사를 단독으로 다시 실행한 `-- 아웃라인`은 2/2 통과(30.81초)라 병렬 부하에 따른 기존 시간 초과 유형으로 분류했습니다.
    - 위 묶음에는 탐색기 reveal·rename(메인·보조 창), outline reveal(untitled·파일), JSON outline, symbol sidebar·outline 단위, navigation 아이콘, 탐색기 단위 검사가 들어 있습니다.
    - `cargo test … --test explorer`: 21/21 통과(즉시 그리기 `RowIcons`의 texture 래스터화 개수 단언 포함)
    - `cargo test … --test explorer-clipboard-owners`: 1/1 통과
- `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets --locked --offline …`: exit 0(기존 경고만)
- `cargo check --quiet --manifest-path native/taide-native-ui/Cargo.toml --all-targets --features native-host,inspection …`와 기본 feature `--lib`: exit 0(기존 경고만)
- `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline …`: exit 0(기존 경고만). remote-web `Cargo.lock`은 바뀌지 않았습니다.
- `cargo fmt … -- --check`: UI crate는 차이 없음입니다. 앱 crate의 남은 차이는 모두 기존 코드입니다(`experiments/lsp-coordinator-spike/src/bin/mock-server.rs`, `application.rs` 960행 들여쓰기·테스트 모듈 순서, `lib.rs` 모듈 순서).
- 실행하지 않은 것: 필터 없는 앱 전체 테스트(배치 끝 메인 담당), GUI 실기, 실제 Finder drag.

## 변경 파일

- `native/taide-native-ui/src/error-boundary.rs`: `Region::SidebarPanel`, `show_instance`, `inject_instance_panic`
- `native/taide-native-ui/src/shell.rs`: 탐색기 trait API(`type Explorer`, `begin/finish/discard_explorer`, `ExplorerMount`), `sidebar_panel`, `fallback_colors`
- `native/taide-native-ui/tests/workbench.rs`: 검사 surfaces의 새 trait 형태, 경계 검사 2건
- `native/taide-native-app/src/application.rs`: `ExplorerRegion`·`OutlineReveal`, 탐색기 render 분리, 삭제 확인 닫기, web view drag 연결
- `native/taide-native-app/src/application-error-boundary-tests.rs`, `application-file-drop-tests.rs`: 실제 App 검사
- `native/taide-native-app/src/explorer.rs`: `PaintIcon<T>` trait, `RowIcons.glyphs`를 trait object로, `Explorer::remounted`
- `native/taide-native-app/src/navigation-icons.rs`: `prepare`/`shape`, `PaintIcon` 구현, `Deferred`
- `native/taide-native-app/src/symbol-sidebar.rs`, `symbol-outline.rs`(+ 두 검사 파일 import): `take`/`put`, 아이콘 인자를 `PaintIcon<Icon>`로
- `native/taide-native-app/src/explorer_clipboard_owners.rs`: `reveal`(조회), `unmount`
- `native/taide-native-app/src/file-drop.rs`: `WebDrags`
- `native/taide-native-app/src/preview_web_view.rs`: drag 처리기, `forget` 호출, 단위 검사
- 지정 범위 밖 최소 수정: navigation 아이콘·symbol sidebar·outline·clipboard owners는 패널 render를 읽기 전용으로 바꾸는 데 필요한 만큼만 바꿨습니다.
- Cargo.toml·Cargo.lock, 동결 remote-web은 바꾸지 않았습니다.

## 남은 범위

- 편집기 영역 경계(메인·보조 창): 원본 `EditorArea`는 pane tree, 탭 바, 탭 내용(편집기·터미널·미리보기·설정), 문제 panel을 포함합니다. native 편집기 render는 `NativeEditor::show_request*`(`taide-native-ui/src/editor_surface.rs`) 안에서 입력 처리 중 공유 `EditorStore`를 직접 바꿉니다(`set_view_state`, `set_display`, `set_composition`, `undo`/`redo`, `break_undo_group` 등 40곳 이상). 앱 `tab_content`도 편집기별 상태 20여 종, 터미널, 미리보기 cache, WKWebView 배치를 `&mut`로 씁니다. 이 경계는 편집기 입력 처리를 render 밖 트랜잭션으로 나누는 별도 설계가 필요해 이번 작업에서 두지 않았습니다. 그 전까지 이 영역의 panic은 앱 종료와 hot-exit·세션 복원(shell-25)입니다.
- 루트 `errorBoundary.app` 경계
- fallback 픽셀 대조(배치 31)
- shell-22:
    - overlay가 미리보기 web view를 숨긴 뒤 AppKit이 drag 대상을 창으로 바꾸는지(web view `draggingExited` → winit `HoveredFile`)는 실기 확인이 필요합니다. 대상이 바뀌지 않아도 drop은 web view 처리기로 같은 경로에 옵니다. `draggingExited`가 오지 않는 경우에는 그 web view가 사라질 때까지 overlay가 남을 수 있습니다.
    - drag 중 repaint 요청이 AppKit drag 추적 run loop 안에서 바로 그려지는지, 실제 Finder drag, Zen 안내·편집기 popup이 overlay 위에 그려지는 순서 차이, 점선 픽셀은 배치 31입니다.
