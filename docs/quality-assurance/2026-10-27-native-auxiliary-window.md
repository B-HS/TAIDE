# 보조 창 (eframe 다중 viewport)

## 대상

- shell-4: 보조 창 분기. 원본은 `src/app/app.tsx` 62-89의 `getWindowContext()` aux 분기에서 `AuxiliaryWindowShell`을 그립니다.
- shell-5: 보조 창 셸. 원본은 `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 73-146입니다.
- shell-6: 보조 창 타이틀바. 원본은 `src/widgets/auxiliary-window-shell/auxiliary-title-bar-content.tsx`입니다.
- shell-8: 보조 창 복원. 원본은 `crates/taide-window/src/service.rs` `restorable_project_ids`·`plan_auxiliary_window_restorations`와 `src-tauri/src/domain/window/commands.rs` `restore_auxiliary_windows`입니다.
- 결정: 보조 창은 eframe 다중 viewport입니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md` 16행).

## 원본 동작

- 창 문맥(`src/shared/lib/window-context.ts` 13-34): 보조 창은 `(projectId, windowSlot)`에 고정되고 메인 창의 활성 프로젝트를 따르지 않습니다.
- 창 생성(`src-tauri/src/domain/window/commands.rs` 46-96):
    - label은 Rust가 정하는 가장 작은 빈 `editor-<n>`입니다(`crates/taide-window/src/service.rs` 29-40).
    - 제목 `"{프로젝트명} — TAIDE"`, 크기 1000×700, 최소 640×420, Overlay 타이틀바와 숨은 제목입니다(`src-tauri/src/domain/window/types.rs` 5-8).
    - 생성 뒤 `WindowRegistry`에 label→(project, slot)을 등록합니다.
- 셸(`auxiliary-window-shell.tsx` 118-145):
    - mac에서만 타이틀바를 그립니다(120-124).
    - 탐색기 패널(기본 240px, 최소 180px, 최대 40%, 접힘 가능)과 편집기 영역만 있습니다. 사이드바(rail)·상태바·Problems·Zen은 없습니다(`zen={false}`, `isProblemsOpen={false}`).
    - 탐색기 접힘은 창마다의 view-local 상태이고 `shell_view.sidebarCollapsed`를 쓰지 않습니다(50-55, 82). 처음에는 펼쳐져 있습니다.
    - 키맵 `toggle-sidebar`는 이 창의 패널을 접고 펼칩니다(96-102).
    - 빈 편집기에서 Welcome을 보이지 않습니다(`pane-node-view.tsx` 130).
- 창 수명(`auxiliary-window-shell.tsx` 110-116, `src-tauri/src/lib.rs` 1104-1118, 373-381):
    - 자기 트리가 비거나(마지막 탭 이동·닫기) layout 조회가 실패하면(프로젝트 닫힘) 창을 닫습니다.
    - 창이 닫히면 창 범위 hot-exit flush 뒤 registry에서 지우고 `return_auxiliary_window_tabs`로 남은 탭을 메인 트리로 돌려보냅니다(`crates/taide-runtime/src/layout_actions.rs` 199-226).
- 타이틀바(`auxiliary-title-bar-content.tsx` 21-31): 이 창 트리의 활성 탭 — 프로젝트명, git branch입니다.
- 키맵(`src/widgets/editor-area/editor-area.tsx` 109-321): close-tab·split·cycle·focus-group·move-tab-to-group·toggle-terminal·close-all-tabs가 이 창의 `paneTree`와 `focusedPane`에 적용됩니다.
- 탐색기 열기(`src/entities/layout/layout.query.ts` 83-109): 대상 pane이 없으면 이 창의 focused pane으로 엽니다.
- 복원(`crates/taide-window/src/service.rs` 55-79, `commands.rs` 333-360): 부팅 때 root가 사라지지 않은 열린 프로젝트의 `auxiliary_windows`를 기록 순서대로 창으로 다시 만듭니다.

## 구현

- UI(`native/taide-native-ui`):
    - `ShellSurfaces`에 `is_auxiliary_explorer_open`과 `auxiliary_explorer`를 추가했습니다. 기본 구현(닫힘·무동작)이 있어 기존 구현체와 동결 remote-web은 그대로입니다.
    - `WindowScope::Auxiliary` 셸은 제목줄 → 열려 있으면 탐색기 패널(메인과 같은 240/180/40% 상수) → 그 창의 pane 트리 순서로 그립니다. rail·상태바·Problems·slot 머리글은 없고 Welcome-on-empty도 끈 채입니다(`shell.rs` Auxiliary 분기).
    - `ShellIntent::ToggleAuxiliaryExplorer { project, slot }`를 추가했습니다.
    - 탭 drag 기억을 viewport별 id로 나눴습니다(`tab-drag.rs` `with_memory`). 같은 egui `Context`를 쓰는 보조 viewport 패스가 메인 창의 진행 중 drag를 `end_frame`에서 끝내 버리지 않게 하기 위해서입니다.
- App(`native/taide-native-app`):
    - `auxiliary-windows.rs`가 열린 보조 창 목록을 가집니다. 각 창은 project, slot, `editor-<n>` label, viewport id, 탐색기 mount id, view-local 탐색기 열림, 창별 마지막 편집기 focus를 들고 있습니다.
        - 복원 계획은 원본 `restorable_project_ids` + `plan_auxiliary_window_restorations`와 같은 규칙을 session 프로젝트 순서로 적용합니다(root 누락 제외, 프로젝트 안 기록 순서 유지, 닫힌 프로젝트의 layout 무시). `taide-window` 크레이트는 native 의존 그래프에 없어 Cargo 변경 없이 같은 규칙을 옮겼습니다.
        - label은 원본 `next_auxiliary_label`처럼 가장 작은 빈 번호입니다. 열고 닫을 때 runtime `WindowRegistry`(`services.windows`)에 등록·삭제합니다.
        - viewport builder는 원본 제목 형식 `"{프로젝트명} — TAIDE"`, 1000×700, 최소 640×420과 메인 창과 같은 overlay 타이틀바(전체 크기 내용·제목/타이틀바 숨김)입니다.
    - `NativeApplication::new`가 부팅 상태의 session·projects·layouts로 복원 계획을 세워 창을 등록합니다(shell-8).
    - 프레임 구조를 나눴습니다(`application.rs` `frame_ui`):
        - 프레임 1회: chord 정리(어느 창도 focus가 없을 때만), markdown·탐색기 clipboard mount·zen 전체화면·터미널 retain, LSP 기능 reconcile, settings `begin_frame`.
        - 창마다(`show_window`): breadcrumbs reconcile, 그 창의 `command_context`·target pane·focused view, 키맵 캡처/라우팅, 셸, intent 처리.
        - 보조 창(`show_auxiliary_windows`): 메인 창 패스 뒤 각 창을 `Context::show_viewport_immediate`로 띄우고, 그 패스 동안 `shell.scope`를 그 창의 `Auxiliary`로 바꿔 같은 `show_window`를 실행합니다. 앱 상태는 하나라 모든 창이 공유합니다.
            - 메인 패스나 앞 보조 패스의 결과(`flush_dirty` 실패의 `false`)와 관계없이 살아 있는 보조 창은 매 프레임 viewport를 요청합니다. egui는 그 패스에서 요청되지 않은 자식 viewport를 지우고(`vendor/egui-input/src/context.rs` 3020-3031 "never used this pass") eframe은 그 OS 창을 파괴하기 때문입니다. 모든 패스가 진행됐을 때만 프레임 끝 처리를 합니다.
            - 보조 패스 동안에는 바꿔 둔 메인 범위를 `auxiliary_pass_root`에 기록합니다. 그래서 그 패스 안에서도 `window_scopes`가 메인 + 모든 보조 창을 돌려줍니다.
        - 프레임 끝 1회: IDE 선택 `finish_frame`, settings `finish_frame`, theme preview, 영속 flush, WKWebView 동기화(메인 창 배치만), toast.
    - LSP 활성 판정은 현재 창 하나가 아니라 열린 모든 창 범위(`window_scopes`: 메인 + 보조 창들) 중 하나에서 활성인지로 봅니다. 한 창의 패스나 logic 단계가 다른 창의 요청을 취소하지 않게 하기 위해서입니다.
        - 프레임 단계: inlay hint, code lens, highlight, documentation, completion, inline completion/edit, selection range, peek 위치(`reconcile_editor_features`, `editor_locations::retain_active`).
        - logic 단계(`background_tick`, 범위는 항상 메인): `reconcile_lsp`는 모든 창의 `lsp::visible_files`를 합쳐 동기화하고 `retain_bindings`합니다. code action 활성 판정도 모든 창입니다. `poll_lsp`의 InlayHints·Highlights·Completion·Documentation·SymbolLocations 수락, `is_current_code_lens_request`, `is_current_selection_range_request`, `dispatch_completion`(요청·clipboard), `HostReply::CompletionClipboard`, 입력 서식(`request_editor_input_formatting`의 owner 확인)도 같은 `active_window_scope`를 씁니다.
        - 위치 열기(`consume_location_model`, `consume_editor_command_navigation`)는 요청 owner가 활성인 창의 범위를 `OpenSymbolLocation`에 실어 보냅니다(`request_window`). 그런 창이 없으면 기존처럼 현재 범위입니다. host의 `symbol_location_host::open`이 그 범위의 트리에서 owner pane을 다시 확인하기 때문입니다.
        - 원본에서는 창마다 Monaco와 LSP 요청이 그 창 realm에서 돌므로, 보조 창 편집기도 메인과 같은 LSP 기능을 받습니다(`auxiliary-window-shell.tsx`의 `EditorArea`).
    - 모달(키바인딩 편집기·팔레트·탭 닫기 확인·삭제 확인·경로 열기·프로젝트 dialog·시스템 사용량)은 OS focus를 가진 창에서 그립니다. 모달이 열려 있는 동안에는 그 창에 고정합니다(`overlay_viewport`). 팔레트가 낸 명령도 그 창의 범위로 실행합니다.
    - 키맵은 창 범위를 따릅니다(`shell_keymap::action/intent`, `command_dispatch::intent`):
        - close-tab·split·cycle·focus-group·move-tab-to-group·toggle-terminal·close-all-tabs·new-terminal은 그 창 트리의 focused pane에 적용됩니다.
        - toggle-sidebar는 보조 창이면 `ToggleAuxiliaryExplorer`로 그 창만 접고 펼칩니다. 메인 `shell_view.sidebar_collapsed`는 쓰지 않습니다.
        - zen 토글은 보조 창에서 무동작입니다(원본 `view.toggleZenMode` MainWindow 전용).
    - 보조 창 탐색기는 그 창의 mount id로 기존 탐색기 표면을 재사용합니다. 파일 열기는 그 창의 focused pane을 대상으로 합니다. 팔레트 파일 열기도 보조 창이면 그 창의 focused pane입니다(원본 `withCurrentWindowTarget`). 탐색기 clipboard·Files/Outline 선택 reconcile은 보조 창 mount를 유지합니다.
    - 창별 마지막 편집기 focus를 따로 둡니다. 두 창이 번갈아 `focused`를 덮어 매 프레임 서로의 편집기에 focus를 다시 요청하던 문제를 막습니다.
    - 창 수명(shell-5):
        - runtime layout에서 그 창 항목이 없거나 트리가 비면 닫습니다. 마지막 탭 닫기·이동, 프로젝트 닫힘이 여기에 해당합니다.
        - 보조 viewport의 OS 닫기 요청도 닫습니다.
        - 닫을 때는 그 창 탭의 문서 중 mirror 대기분을 즉시 `MirrorDraft`로 보냅니다(원본 `FlushScope::Window`). 이어서 registry에서 지우고 `HostCommand::ReturnAuxiliaryWindowTabs` → runtime `return_auxiliary_window_tabs`로 남은 탭을 메인 트리로 돌려보냅니다.
        - 복귀는 `Windows::close`가 대기열에 넣고, 제출에 성공할 때만 지웁니다(`submit_auxiliary_window_returns`). host 큐가 가득 찼거나(`HOST_COMMAND_CAPACITY`) bridge가 없어 제출이 실패하면 다음 프레임마다 다시 보냅니다. 같은 (project, slot) 창을 다시 채택하면 그 대기는 취소합니다.
        - host worker가 직렬이라 mirror 쓰기가 복귀보다 먼저 끝납니다.
        - 앱 종료 중에는 닫지 않으므로, 다음 부팅에서 그대로 복원됩니다.
- 보조 창 경로를 위해 함께 고친 파일과 사유
    - `editor-locations.rs`(+tests): `retain_active`가 창 범위 목록을 받습니다.
    - `symbol-sidebar.rs`(+tests), `explorer_clipboard_owners.rs`: reconcile이 보조 창 탐색기 mount를 유지합니다. `reconcile`은 기존 시그니처를 남겼습니다.
    - `shell_keymap.rs`·`command-dispatch.rs`: 창 범위 인자를 추가했습니다. Main 범위만 쓰던 호출부(`presentation.rs`·`save.rs`·`zen.rs`·`tab_close_batch.rs`·`command-palette.rs`의 검사)에 `WindowScope::Main`을 넘깁니다.
    - `host.rs`: `ReturnAuxiliaryWindowTabs` 명령을 추가했습니다.
    - 리뷰 수정 단계에서 함께 고친 검사 파일
        - `application-inlay-hints-tests.rs`: LSP mock 연결부를 `connect_lsp_mock`으로 뽑았습니다. 기존 fixture와 보조 창 LSP 검사가 같이 씁니다. 동작은 그대로입니다.
        - `application-auxiliary-window-tests.rs`: 테스트용 viewport renderer에 viewport별 focus 요청을 넣었습니다(egui focus가 viewport별이라 패스 밖 `memory_mut`로는 보조 창 편집기에 focus를 줄 수 없습니다). shell-7 헬퍼 `choose_tab_menu`의 `///` 주석 2줄은 주석 규칙에 따라 지웠습니다(이름이 동작을 설명합니다).

## 리뷰 수정 (2026-10-27)

- 차단 1 — logic 단계 LSP 경로가 메인 범위만 보던 결함
    - 증상: 보조 창에서만 열린 파일은 `reconcile_lsp`가 `visible_files(Main)`에서 찾지 못해 동기화되지 않고 매번 `retain_bindings`로 닫혔습니다. 응답도 `is_active(layout, Main)`이 false라 버려졌고, `dispatch_completion`은 completion을 닫았습니다. 보조 패스 안에서 `reconcile_lsp`가 불리면 반대로 메인 파일 binding이 닫혔습니다.
    - 수정: `window_scopes`가 보조 패스 안에서도 메인 + 모든 보조 창을 돌려주게 했고(`auxiliary_pass_root`), logic 단계 판정을 모두 `active_window_scope`(그 목록 중 하나에서 활성)로 바꿨습니다. 위치 열기는 owner가 활성인 창의 범위를 실어 보냅니다(구현 절 참고).
- 차단 2 — 메인 패스 실패가 보조 OS 창을 파괴하던 결함
    - 증상: `show_window(..) && show_auxiliary_windows(..)` 단락 평가로 `flush_dirty` 실패 프레임에 보조 viewport를 요청하지 않아, egui가 그 viewport를 지우고 eframe이 OS 창을 다시 만들었습니다(위치·크기·drag·focus 소실).
    - 수정: 두 호출을 모두 실행하고, 보조 창끼리도 앞 창의 실패와 관계없이 모두 띄웁니다. 프레임 끝 처리만 모든 패스 성공 시 실행합니다.
- 차단 3 — 탭 복귀 제출 실패를 버리던 결함
    - 증상: `close_auxiliary_window`가 창을 목록·registry에서 먼저 지운 뒤 `ReturnAuxiliaryWindowTabs` 제출 실패를 무시해, 탭이 그 세션 내내 보이지 않았습니다.
    - 수정: `Windows`가 복귀 대기열을 갖고 성공할 때까지 매 프레임 다시 제출합니다(구현 절 참고).
- 비차단 반영
    - shell-6: 같은 작업 트리의 shell-61이 보조 창 프로젝트 branch도 조회하므로 대응표 상태를 complete로 올리고, 보조 창 전용 프로젝트 branch 조회 실제 App 검사를 더했습니다.
    - shell-5: 대응표 남은 범위에 search/git 보기, 탐색기 상태 공유를 적고, shell-78에서 해소된 탭 메뉴 reveal/rename을 뺐습니다.
    - 기존 실제 App 검사의 OS 닫기 단계는 host 복귀가 닫기 프레임 안에 끝나면 그 프레임 출력(닫기 요청을 읽느라 viewport를 아직 요청한 출력)으로 viewport 부재를 단언하는 경쟁이 있었습니다. 부하 실행에서 한 번 실패해, 복귀 확인 뒤 다음 프레임 출력으로 단언하도록 고쳤습니다. 단언 내용(닫힌 뒤 viewport 요청 없음)은 그대로입니다.

## 검증

- App `cargo test --lib -- auxiliary shell_keymap command_dispatch symbol_sidebar`는 마지막 변경 뒤 24/24 통과입니다.
    - `auxiliary_windows` 단위 4개
        - 복원 계획: root 누락·layout 없음·목록 밖 프로젝트 제외, 프로젝트 안 기록 순서
        - 가장 작은 `editor-<n>`, registry 등록·해제와 label 재사용
        - viewport id 구분, 탐색기 토글, 빈 트리·layout 없음의 생존 판정
        - builder의 제목·크기·최소 크기·overlay 타이틀바
    - `shell_keymap` 보조 창 키맵 1개: close-tab·split·cycle·focus-group·new-terminal이 메인 focus 프로젝트가 없어도 보조 창 트리에 적용되고, toggle-sidebar는 `ToggleAuxiliaryExplorer`, zen은 무동작, 없는 slot은 무동작입니다.
    - 실제 App 1개(`application-auxiliary-window-tests.rs`): 실제 `NativeApplication`, 임시 프로젝트, 테스트용 immediate viewport renderer(`Context::set_immediate_viewport_renderer`, OS 창 없음)를 씁니다.
        - 부팅 복원: `editor-1`·`editor-2`이고 root 누락 프로젝트 창은 없습니다.
        - 메인 패스의 viewport 요청: parent ROOT, immediate, 제목 `alpha — TAIDE`, 크기와 최소 크기, 타이틀바 숨김
        - 보조 패스 출력: 제목줄 `notes.txt — alpha`, 메인 sidebar가 접혀 있어도 두 창 모두 탐색기 표시
        - 보조 창의 ⌘B: 그 창 탐색기만 접히고 다른 창과 메인 `sidebar_collapsed`는 그대로입니다.
        - 보조 창 탐색기 클릭: 파일이 그 창 트리에 열리고 메인 트리에는 없습니다.
        - 보조 창 OS 닫기: 두 탭이 메인 트리로 돌아오고 registry와 viewport가 사라집니다.
        - 마지막 탭 ⌘W: 창 항목이 지워지고 viewport가 닫힙니다.
    - `command_dispatch`·`symbol_sidebar` 기존 검사(보조 mount 유지 단언 추가)
- App `cargo test --lib -- 보조`(이름에 보조가 든 기존·신규 실제 App 검사)는 마지막 변경 뒤 16/16 통과입니다. 메뉴·파일 로딩·초안·IDE 선택·view state의 보조 범위 회귀가 여기에 들어갑니다.
- App `cargo test --lib -- shell_keymap command_dispatch editor_locations symbol_sidebar zen:: save:: tab_close_batch presentation:: auxiliary_windows explorer_clipboard`는 45/45 통과입니다(키맵 시그니처와 `retain_active` 범위 변경 회귀).
- App `--test explorer-clipboard-owners`는 1/1 통과입니다.
- UI `cargo test --features native-host,inspection --test workbench --lib`: lib 163/163, workbench 32/32 통과입니다. 새 실제 egui 검사는 2개입니다.
    - 보조 창 셸: 제목줄 아래 탐색기(기본 폭)와 그 오른쪽 편집기만 있습니다. rail·상태바·Problems·slot 탐색기는 없고, 접으면 편집기가 0부터 시작합니다.
    - viewport별 탭 drag: 메인 창 drag 도중 같은 프레임에 보조 immediate viewport 패스가 돌아도 drag가 이어지고 `MoveTab`으로 끝납니다.
- App `cargo test --lib -- application::` 96건을 한 번 돌렸습니다(마지막 focus·팔레트 수정 전 binary, 다른 검사와 동시 실행). 결과는 88 통과, 8 실패입니다.
    - 단독 재실행에서 통과: inlay hints 2, json peek 1, IDE 보조창 1, code actions 다른 uri 1. 동시 실행 부하의 시간 초과입니다.
    - 단독 재실행에서도 실패: `startup_tests::actual_app_constructor…`, `startup_tests::actual_app의_아웃라인…`, `view_state_tests::실제_정상과_직접_종료는…`.
        - 셋 다 `close()` 뒤 `is_exit_ready`를 기다리는 단계에서 `tasks=2`로 10초 시간 초과합니다. 이 셋이 이 대기를 하는 테스트 전부입니다.
        - startup 쪽 대기는 `App::logic`만 부르고 `ui`는 부르지 않습니다. 이번 변경 중 `logic`에 닿는 것은 보조 mount를 유지하는 reconcile뿐입니다.
        - 작업 규칙상 HEAD 기준선을 따로 실행할 수 없어(되돌리기·별도 target 금지) 기존 실패인지 확정하지 못했습니다. unresolved로 넘깁니다.
    - 같은 실행의 `exit_tests` 3/3은 통과입니다.
- check와 형식
    - App `cargo check --all-targets`, UI `cargo check --all-targets --features native-host,inspection`, remote-web Wasm lib check는 exit 0입니다. 경고는 모두 HEAD에 있던 것입니다.
    - UI `cargo fmt -- --check`는 exit 0입니다.
    - App `cargo fmt -- --check`의 차이는 이번 변경 밖의 기존 차이뿐입니다: `application.rs` 925·10734 부근, `lib.rs` 모듈 순서, `experiments/lsp-coordinator-spike`.
- RED 기록
    - 새 검사는 구현과 같은 단계에서 작성했습니다. 구현을 끄는 재현은 작업 규칙상 하지 않았습니다.
    - 처음 실행에서는 다음이 실패했고 각각 고쳐 통과했습니다.
        - workbench viewport 검사의 RawInput에 viewport 정보가 빠져 있었습니다(검사 쪽 수정).
        - 탐색기 폭 단언을 검사 표면의 내용 폭에 맞췄습니다(검사 쪽 수정).
        - App 검사의 `ViewportClass` 단언: immediate viewport는 ctx에 class를 기록하지 않아 `viewport_ui_cb`가 없는지로 바꿨습니다(검사 쪽 수정).
    - 창별 편집기 focus 분리(제품 수정)는 검토 중 찾은 결함입니다. 두 창이 번갈아 서로의 편집기에 focus를 다시 요청합니다. 이를 재현하는 별도 검사는 없습니다.
- 실제 앱 경로: `main.rs` → `NativeApplication::new`(복원) → `eframe::App::ui` → `frame_ui` → `show_window`/`show_auxiliary_windows` → `NativeShell::show`(Auxiliary) → `HostBridge` `ReturnAuxiliaryWindowTabs` → runtime `return_auxiliary_window_tabs`입니다.
- 리뷰 수정 검증
    - RED(구현 전 실행, `cargo test --lib -- 보조_창에서만_열린_파일도 dirty_반영에_실패해도 탭_복귀는_host_제출이`): 새 실제 App 검사 3개가 모두 실패했습니다.
        - LSP: `phase=auxiliary-only document binding`에서 10초 시간 초과, `lsp=[None]`(binding 없음).
        - 메인 패스 실패: `failed.viewport_output.contains_key(&viewport)` 단언 실패.
        - 탭 복귀: 창 닫힘·registry 해제·layout 잔류 단언은 통과하고 `phase=tab return resubmitted`에서 시간 초과.
        - 첫 실행에서는 LSP 검사를 inlay fixture의 setup으로 꾸몄다가 embedded viewport(renderer 설치 전) 때문에 메인과 보조 탐색기 위젯 id가 한 viewport에 겹쳐 egui debug assert로 실패했습니다. renderer를 첫 프레임 전에 설치하는 전용 fixture로 바꿔 위 결과를 얻었습니다.
    - GREEN(구현 뒤): 위 3개와 `auxiliary_windows` 단위 `닫힌_창의_탭_복귀는_제출될_때까지_대기하고_같은_슬롯을_다시_열면_취소된다`가 4/4 통과입니다.
    - 새 검사 내용
        - `실제_app_보조_창에서만_열린_파일도_lsp_동기화와_응답_수락을_받는다`: 메인 트리는 비고 보조 창에만 `current.rs`가 있습니다. LSP mock(`--native-inlay-hints`) 연결 뒤 completion·inlay provider가 생기고, 보조 viewport의 inlay hint가 표시되며, 보조 편집기 completion 후보 `method`가 수락되고 요청 viewport가 보조 창입니다.
        - `실제_app_메인_패스가_dirty_반영에_실패해도_보조_창_viewport를_유지한다`: bridge를 잠시 떼고 `pending_dirty`를 넣어 `flush_dirty`를 실패시킵니다. 그 프레임에도 보조 viewport가 요청되고 보조 패스가 그려지며, bridge를 되돌리면 dirty 반영이 이어집니다.
        - `실제_app_보조_창_닫기의_탭_복귀는_host_제출이_실패하면_다시_보낸다`: bridge가 없는 동안 OS 닫기 → 창·viewport·registry는 사라지고 layout 항목은 남습니다. bridge를 되돌리면 복귀가 다시 제출되어 탭이 메인 트리로 오고 창은 다시 열리지 않습니다.
        - `실제_app_보조_창_제목줄_branch는_메인에_보이지_않는_프로젝트도_읽는다`(macOS): 메인 slot에는 alpha, 보조 창에만 git 저장소 beta가 있을 때 beta branch를 읽습니다. shell-61 구현이 이미 있어 RED는 없습니다.
    - `cargo test --lib -- auxiliary 보조 --test-threads=4`: 26/26 통과(기존 OS 닫기 단언 경쟁 수정 뒤). 수정 전 같은 실행에서 그 단언 1건이 실패했습니다.
    - LSP 판정 변경 회귀: `application::inlay_hints_tests application::emmet_tests application::code_actions_tests application::code_lenses_tests application::selection_ranges_tests application::sticky_definition_tests application::ai_completion_tests` 24/24 통과입니다.
    - 범위·키맵 회귀: `application::context_menu_tests application::ide_selection_tests application::file_loading_tests application::missing_draft_tests application::tab_menu_tests application::tab_drag_tests editor_locations shell_keymap command_dispatch symbol_sidebar explorer_clipboard` 63/63 통과입니다.
    - 종료 대기 검사: `application::startup_tests application::view_state_tests application::exit_tests --test-threads=4`는 18 통과, 4 실패입니다.
        - 기존 3건(`startup_tests::actual_app_constructor…` app-exit `tasks=0`, `startup_tests::actual_app의_아웃라인…` `tasks=2`, `view_state_tests::실제_정상과_직접_종료는…` `tasks=2`)은 이전과 같은 `close()` 뒤 종료 대기 시간 초과입니다.
        - `view_state_tests::실제_일괄_닫힘은…`은 부하 실행에서 한 번 실패했고 단독 재실행에서 1/1 통과했습니다.
        - 귀속(shell-61 `GitBranch` transient 여부)은 HEAD 비교나 요청 비활성 재현이 작업 규칙상 금지라 확정하지 못했습니다. `GitBranch` 요청은 `closing.is_none()` 안에서만 나가고(`background_tick`), 비 git 임시 디렉터리에서는 `resolve_repo_root`가 곧바로 실패합니다.
    - check와 형식: App `cargo check --all-targets` exit 0(경고는 기존 `is_applying`·`zen.rs` `mut`·`fixture_with_emmet_peek`), remote-web Wasm lib check exit 0. App `cargo fmt -- --check`의 남은 차이는 기존 차이뿐입니다(`application.rs` 935·10879 부근, `lib.rs` 모듈 순서, `experiments/lsp-coordinator-spike`). UI 크레이트는 이번 수정에서 바꾸지 않았습니다.

## 남은 범위

- 런타임에 새 보조 창을 여는 진입은 shell-7(탭을 새 창/메인 창/기존 창으로 이동, `layout_move_tab_to_window`)입니다. 이 단계의 `Windows::open`과 닫힘 수명이 그 진입의 대상입니다.
    - 원본 `cleanup_emptied_auxiliary_windows`의 `close_window(label)`은 native에서 할 일이 없습니다. layout 항목이 사라지면 다음 프레임에 viewport가 닫히기 때문입니다.
- 보조 창 탐색기는 Files/Outline 보기만 있습니다. 원본 `ExplorerContainer`의 search/git 보기 전환(`auxiliary-window-shell.tsx` 96-105의 `git` 키맵과 search panel bridge 포함)은 아직 없습니다.
- 탐색기 선택·inline rename·create 입력은 프로젝트당 하나인 `Explorer` 상태라, 같은 프로젝트를 메인 slot과 보조 창이 함께 보이면 두 창에 같이 나타납니다. 원본은 창마다 React 상태가 분리됩니다. 창별(mount별) 상태로 나누는 것은 탐색기 상태 구조 변경이라 따로 다룹니다.
- 원본 보조 창은 `TaskRunnerDialog`도 띄웁니다(`src/app/app.tsx` 78). native에는 Task Runner 대화상자 자체가 없어(terminal-agent-76 unwired, shell-24) 그 행에서 함께 다룹니다.
- 탭 drag 중 메인 WKWebView 비활성 판정은 ROOT 패스의 `tab_drag::is_dragging`만 봅니다. 보조 창에서 진행 중인 drag는 반영되지 않습니다.
- 보조 창 안 HTML·오디오·비디오 미리보기(WKWebView)는 배치하지 않습니다. `preview_web_view::Views::sync`가 메인 `eframe::Frame`의 창에만 붙습니다.
- 보조 immediate viewport 입력에는 eframe이 `App::raw_input_hook`을 부르지 않습니다(`vendor/eframe/src/native/wgpu_integration.rs` `render_immediate_viewport`). 보조 창 터미널의 마우스 보고 패킷 경로는 vendored eframe 수정이 필요합니다.
- 보조 창의 설정 탭에서 고른 theme preview는 적용하지 않습니다. `sync_theme_preview`가 메인 viewport 기준입니다.
- 복원 계획은 `taide-window` 크레이트를 의존하지 않고 같은 규칙을 옮겼습니다(Cargo 변경 없이). 원본 규칙이 바뀌면 어긋날 수 있어, 의존 추가와 복제 유지 중 결정이 필요합니다.
- toast와 파일 선택 dialog(rfd)의 부모는 메인 창입니다.
- OS 창 위치·크기 영속(`tauri-plugin-window-state`의 `editor` 키)은 shell-3 범위입니다.
- 실제 OS 다중 창 실기(VoiceOver, 창 전환 focus, 닫기 버튼)는 배치 31입니다.
