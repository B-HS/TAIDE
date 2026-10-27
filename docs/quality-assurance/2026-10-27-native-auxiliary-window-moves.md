# 보조 창 탭 이동·제목줄 branch

## 대상

- shell-7: 탭을 새 창/메인 창/기존 창 N으로 이동. 원본은 `src/widgets/editor-area/pane-tab-bar.tsx` 108-120, 230-233과 `layout_move_tab_to_window`입니다.
- shell-61: 제목줄 텍스트(탭 — 프로젝트 + branch, 폭별 숨김). 원본은 `src/features/window/title-bar.tsx`, `src/widgets/window-chrome/title-bar-content.tsx`, `src/widgets/auxiliary-window-shell/auxiliary-title-bar-content.tsx`입니다.
- shell-78: 탭 메뉴의 창 이동 항목(`tab.moveToNewWindow`, `tab.moveToMainWindow`, `tab.moveToWindowNumbered`)과 보조 창 탭 메뉴의 Explorer 보기 reveal·rename. 원본은 `src/features/tab/tab-context-menu.tsx` 121-150, 175-190입니다.
- 직전 단계(`2026-10-27-native-auxiliary-window.md`)의 보조 viewport·창 수명 구현을 그대로 씁니다.

## 원본 동작

- 창 이동 메뉴(`tab-context-menu.tsx` 175-190, `pane-tab-bar.tsx` 116-120, 230-233)
    - 분할 하위 메뉴 다음 구분선 뒤에 놓입니다. 순서는 새 창 → 메인 창 → 창 N입니다.
    - "새 창으로 이동"은 모든 탭에 보입니다.
    - "메인 창으로 이동"은 보조 창(`windowContext.kind === 'auxiliary'`)에서만 보입니다.
    - "창 N으로 이동"은 그 프로젝트 layout의 `auxiliaryWindows` 기록 순서대로 보이고, 보조 창에서는 자기 slot을 뺍니다.
    - 세 항목 모두 `layout_move_tab_to_window(tabId, target)`을 부르고 실패는 `toast.error(describeIpcError)`입니다(`pane-tab-bar.tsx` 106, 120).
- `layout_move_tab_to_window`(`src-tauri/src/lib.rs` 336-370, `crates/taide-runtime/src/layout_actions.rs` 135-197)
    - mutation guard 안에서 탭의 프로젝트를 찾습니다.
    - `Main`·`Existing { slot }`은 layout만 바꿉니다.
    - `NewAuxiliary`는 `next_window_slot`으로 slot을 정합니다. 이어 layout을 바꾸기 전에 OS 창을 엽니다. 연 창은 가장 작은 빈 `editor-<n>` label이고 `WindowRegistry`에 등록됩니다(`src-tauri/src/domain/window/commands.rs` 46-96, 프로젝트가 열려 있지 않으면 NotFound). 이동이 실패하면 그 창을 닫습니다.
    - 성공 뒤 트리가 빈 보조 창 항목을 지우고, 그 label 창을 닫습니다(`cleanup_emptied_auxiliary_windows`).
- 보조 창 탭 메뉴의 reveal·rename(`pane-tab-bar.tsx` 226-227, `auxiliary-window-shell.tsx` 104-108, `explorer-panel.tsx` 151-172)
    - 파일 탭에서 bridge를 발행합니다. bridge는 창(webview realm)마다 따로라 그 보조 창만 받습니다.
    - 그 창의 탐색기 패널을 펼치고 Files 보기로 바꾼 뒤 경로를 reveal합니다. rename이면 inline rename을 시작합니다.
    - 메인 창 sidebar(`shell_view.sidebarCollapsed`)는 건드리지 않습니다.
- 제목줄(`title-bar.tsx` 12-47)
    - 가운데 정렬입니다. 첫 글은 탭 제목, 없으면 프로젝트명입니다.
    - 탭과 프로젝트가 모두 있으면 `— 프로젝트`를 70% 불투명도로 붙입니다. 이 부분은 `md` 폭(768px) 미만에서 숨깁니다.
    - branch는 `GitBranch` 12px 아이콘과 최대 96px으로 잘린 이름입니다. 색은 `app-sidebar-icon-default`, 간격은 6px/4px이고 `lg` 폭(1024px) 미만에서 숨깁니다.
- branch 공급(`title-bar-content.tsx` 19-29, `auxiliary-title-bar-content.tsx` 21-31, `src/entities/git/git.query.ts` 105-111)
    - 메인 창은 focus slot 프로젝트, 보조 창은 자기 프로젝트의 `git_status`를 씁니다. 오류면 branch는 null입니다(`retry: false`).
    - `git:status-changed`·`git:refs-changed`·`fs:changed`가 그 프로젝트 status를 무효화합니다(`src/app/providers/ipc-sync-provider.tsx` 399-411, 531).
    - runtime `git_status`(`crates/taide-runtime/src/git_actions.rs` 136-153)는 `GitStore` 캐시를 쓰고, branch는 HEAD의 shorthand입니다. 아직 커밋이 없으면 symbolic HEAD의 이름입니다(`crates/taide-git/src/service.rs` 1864-1873).

## 구현

- UI(`native/taide-native-ui`)
    - `ShellIntent::MoveTabToWindow { tab, target }`와 `ShellIntent::RevealInAuxiliaryExplorer { project, slot, path, rename }`를 추가했습니다(`commands.rs`). 동결 remote-web은 `ShellIntent`를 쓰지 않아 그대로입니다.
    - pane 문맥과 탭 메뉴 문맥에 이 창의 보조 slot(`window`, 메인은 None)과 그 프로젝트의 `auxiliary_windows`를 넘깁니다(`shell.rs` `PaneViewContext`, `TabMenuContext`).
    - 탭 메뉴(`shell.rs` `tab_menu`)
        - 분할 하위 메뉴 뒤 구분선 다음에 `tab.moveToNewWindow`를 둡니다. 보조 창이면 `tab.moveToMainWindow`를 붙입니다. 이어 `auxiliary_windows` 기록 순서로 자기 slot을 뺀 `tab.moveToWindowNumbered`(`{slot}`)를 둡니다. 각각 `NewAuxiliary`/`Main`/`Existing { slot }`입니다.
        - 파일 탭의 Explorer 보기 reveal·rename은 메인 slot이면 기존 `RevealInExplorer`, 보조 창이면 `RevealInAuxiliaryExplorer`를 냅니다.
        - 문서 주석의 "표시하지 않는 항목"을 Git 변경·이력만으로 고쳤습니다.
    - 제목줄(`shell.rs` `title_bar`)
        - 탭 제목(없으면 프로젝트명)과 `" — 프로젝트"`(70% 불투명도)를 한 줄 LayoutJob으로 그립니다. 글자 내용은 이전과 같습니다.
        - 창 폭이 768 미만이면 프로젝트 부분을 빼고, 1024 미만이면 branch를 뺍니다.
        - branch는 6px 간격 뒤 12px 아이콘, 4px 간격, 최대 96px으로 잘린 이름입니다. 색은 `appSidebar.iconDefault`(`ShellColors::muted`)입니다.
        - 묶음 전체를 신호등 inset 사이 가운데에 둡니다. 넘치면 탭/프로젝트 부분이 먼저 잘립니다.
    - `ShellSurfaces::title_branch_icon`을 추가했습니다. 기본 구현은 무동작이라 기존 구현체가 그대로 컴파일됩니다. `title_bar`가 이 아이콘을 그리려고 `&mut` surface를 받도록 바꿨고, 두 호출부는 branch를 먼저 `String`으로 복사합니다.
- App(`native/taide-native-app`)
    - `HostCommand::MoveTabToWindow { tab, target, label }`(`host.rs`)는 runtime `layout_actions::layout_move_tab_to_window`를 그대로 부릅니다.
        - 창 열기 port: 프로젝트가 열려 있지 않으면 원본처럼 NotFound입니다. 그 밖에는 앱이 예약해 둔 label을 runtime `WindowRegistry`에 (project, slot)으로 등록하고 돌려줍니다.
        - 창 닫기 port: 이번에 연 label이 실패로 닫힐 때만 registry에서 지웁니다. 빈 창 정리(`cleanup_emptied_auxiliary_windows`)가 넘기는 다른 창 label은 건드리지 않습니다. 그 창은 layout 항목이 사라진 다음 프레임에 앱이 닫습니다(직전 단계의 생존 판정, registry 해제, 탭 복귀 no-op).
        - 응답 `HostReply::TabMovedToWindow { label, result }`는 성공이면 새로 연 (project, slot)을 함께 돌려줍니다.
    - `auxiliary_windows::Windows`에 `reserve`/`release`/`adopt`와 `show_explorer`를 추가했습니다(`auxiliary-windows.rs`).
        - label은 열린 창과 예약 중인 label을 함께 보고 가장 작은 빈 `editor-<n>`입니다. 원본은 Rust가 OS 창 목록에서 label을 정합니다. native에서는 창 목록의 주인이 UI 스레드의 `Windows`이고, runtime registry에는 목록 조회 API가 없습니다. 그래서 새 창 이동 intent 때 UI 스레드가 label을 예약해 command에 싣습니다. 이렇게 하면 이동이 여러 건 겹쳐도 label과 viewport id가 겹치지 않습니다.
        - 성공 응답이면 `adopt`로 그 창을 열린 목록에 넣습니다. 응답 시점에는 runtime layout에 이미 새 slot이 있어 다음 프레임에 viewport가 뜹니다. 실패하거나 command 전송이 실패하면 `release`로 예약을 돌려놓고, 실패는 `report`(toast)로 알립니다.
        - `restore`의 `open`도 같은 label 계산과 `adopt`를 씁니다.
    - `ShellIntent::RevealInAuxiliaryExplorer`(`application.rs`)는 다음 순서입니다.
        - 그 창의 탐색기를 펼칩니다(`show_explorer`, view-local). 메인 `shell_view.sidebar_collapsed`는 쓰지 않습니다.
        - 그 창 탐색기 mount의 symbol sidebar 보기를 Files로 바꿉니다.
        - 기존 `HostCommand::RevealExplorerPath`로 reveal하고, rename이면 inline rename을 시작합니다. 탐색기 선택·rename 상태는 원래 프로젝트 단위입니다.
    - branch 공급(`title-branch.rs`, `application.rs` `background_tick`)
        - `PaintSink`가 `GitStatusChanged`·`GitRefsChanged`·`FsChanged`를 프로젝트별로 모읍니다(`title_branch::Changes`).
        - 매 tick마다 그 프로젝트를 stale로 표시하고, 닫힌 프로젝트 항목을 지웁니다.
        - 제목줄이 있는 플랫폼(mac)에서는 보이는 프로젝트만 읽습니다. 메인 창의 focus 프로젝트와 열린 보조 창들의 프로젝트이고, 처음이거나 stale이면서 읽는 중이 아닌 것만 `HostCommand::GitBranch`를 보냅니다. 전송이 실패하면 다시 stale로 둡니다.
        - host는 직렬 command 루프를 막지 않도록 transient 작업에서 runtime `git_actions::git_status`를 부릅니다. 원본 IPC가 독립 비동기 command인 것에 대응합니다. 성공이면 `status.branch`, 오류면 None을 `HostReply::GitBranch`로 돌려줍니다.
        - status 캐시 무효화는 bootstrap에서 이미 켜진 `GitEvents`(`bootstrap.rs` `connect_shell`)가 맡습니다. 그래서 `install_invalidation_listeners`에는 추가로 설치할 것이 없습니다.
        - `AppSurfaces::branch`가 이 값을 돌려주고, `title_branch_icon`은 기존 glyph 아이콘(`Glyph::GitBranch`, lucide git-branch)을 그립니다.
- 범위 밖 수정과 사유
    - `native/taide-native-ui/tests/workbench.rs`의 기존 검사 `탭_메뉴는_주창_파일탭에_탐색기_reveal_rename과_미리보기파일_다시열기를_요청한다`
        - "새 창으로 이동이 보이지 않는다"는 단언에서 그 항목을 뺐습니다.
        - "보조 창에는 reveal/rename이 없다"는 단언을 "있다"로 바꿨습니다.
        - 둘 다 이번 단계가 원본대로 연결한 동작이라, 원본과 다른 이전 기대를 고친 것입니다.
    - `native/taide-native-app/src/application-auxiliary-window-tests.rs`(직전 단계에서 만든 미커밋 파일)에 실제 App 검사 2개와 helper를 더했습니다. viewport 검사 harness를 재사용하려는 것입니다.

## 검증

- RED
    - UI `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test workbench … -- 탭_메뉴 제목줄`를 구현 전에 돌렸습니다. 새 검사 2개와 갱신한 기존 검사 1개가 실패했습니다(메뉴에 `tab.moveToNewWindow`·보조 창 reveal 없음, 제목줄 branch가 한 Label에 붙음). 나머지 2개는 통과였습니다.
    - App 검사(실제 App 2개, 단위 3개)는 구현과 같은 단계에서 구현 뒤에 작성했습니다. 구현을 끄는 재현은 작업 규칙상 하지 않았습니다.
- UI(`native/taide-native-ui`)
    - 같은 필터에 `보조`를 더한 실행은 구현 뒤 7/7 통과입니다.
    - `cargo test --features native-host,inspection --test workbench --lib`는 lib 163/163, workbench 34/34 통과입니다(`cargo fmt` 뒤 다시 실행). 새 실제 egui 검사는 2개입니다.
        - `탭_메뉴는_새창_메인창_다른창_이동과_보조창_탐색기_reveal_rename을_요청한다`
            - 메인 창: 분할 뒤 새 창 항목, 보조 창이 없으면 창 N이 없고 메인 창 항목도 없습니다. 보조 창 1·2가 있으면 그 순서로 창 N이 보이고 `Existing { slot: 2 }`를 냅니다.
            - 보조 창 1: 메인 창 항목과 창 2만 보이고 자기 창 1은 없습니다. reveal·rename은 `RevealInAuxiliaryExplorer`(slot 1, rename false/true)를 내고 `RevealInExplorer`는 내지 않습니다.
            - 터미널 탭: 이동 항목만 있고 reveal/rename은 없습니다.
        - `제목줄은_branch를_아이콘과_보이고_창폭이_좁으면_branch와_프로젝트를_차례로_숨긴다`(메인·보조 창 모두)
            - 1100폭: `탭 — 프로젝트`, 12×12 아이콘, branch가 이 순서로 겹치지 않게 놓이고 묶음이 가운데입니다(허용 4px).
            - 900폭: branch와 아이콘이 없습니다.
            - 700폭: 탭 제목만 있습니다.
    - `cargo check --all-targets --features native-host,inspection`은 exit 0입니다. 경고(`tests/workbench.rs` 1700 `unused_mut`)는 기존 검사의 것입니다.
    - `cargo fmt -- --check`는 exit 0입니다. `cargo fmt`가 고친 파일은 이번에 바꾼 `commands.rs`·`shell.rs`·`workbench.rs`뿐입니다.
- App(`native/taide-native-app`)
    - `cargo test --lib -- auxiliary_windows:: title_branch::`는 7/7 통과입니다. 새 단위 검사는 3개입니다.
        - 예약 label: `editor-2`/`editor-3`, 채택·해제 뒤 재사용, `show_explorer`가 접힌 탐색기를 펼치고 mount를 돌려줌
        - 변경 기록: git status/ref/fs 변경만 프로젝트별로 모음
        - branch 상태: 보이는 프로젝트 1회 요청, 읽는 중 무효화 뒤 재요청, 오류 None, 전송 실패 재시도, 닫힌 프로젝트 정리와 늦은 응답 무시
    - 실제 App 검사(`application-auxiliary-window-tests.rs`): 실제 `NativeApplication`, 임시 프로젝트, 테스트용 immediate viewport renderer를 씁니다.
        - `실제_app_탭_메뉴는_탭을_새창_기존창_메인창으로_옮기고_보조창_탐색기로_reveal_rename한다`
            - 메인 a.txt 새 창: slot 1, `editor-1` 등록, viewport 제목 `alpha — TAIDE`, 보조 viewport에 탭이 그려짐
            - 메인 c.txt 창 1: 메뉴에 메인 창 항목 없음
            - 메인 b.txt 새 창: slot 2, `editor-2`, 메인 트리 비움
            - 보조 1 c.txt 메인 창: 메뉴에 창 2는 있고 창 1은 없음, c가 메인으로 복귀
            - 보조 1의 마지막 탭 a.txt 창 2: 빈 slot 1이 정리되고, registry 해제와 viewport 닫힘, 메인 트리 불변
            - 보조 2에서 ⌘B로 접고 Outline 보기로 둔 뒤 reveal: 그 창 탐색기 펼침, Files 보기, 선택과 트리 행 확인, 메인 `sidebar_collapsed` 불변
            - 보조 2 rename: inline rename 초안 `a.txt`
        - `실제_app_제목줄은_프로젝트_git_branch를_읽고_git_변경_뒤_다시_읽는다`(mac 전용)
            - 임시 `.git`(HEAD·objects·refs)을 손으로 만들어 git CLI 없이 씁니다.
            - 1280폭에서 branch `synthetic-title`이 제목줄에 보이고, 1000폭에서는 숨습니다.
            - HEAD를 바꾸고 `GitStatusChanged`를 발행하면 `synthetic-next`로 다시 읽습니다. 실제 `.git` watcher 대신 같은 event를 직접 발행했습니다.
        - 마지막 변경(host transient 전환) 뒤 `cargo test --lib -- auxiliary_window_tests:: tab_menu_tests:: auxiliary_windows:: title_branch::`는 11/11 통과입니다. 기존 실제 App 보조 창 검사와 메인 탭 메뉴 reveal/rename 검사를 포함합니다.
    - 관련 회귀 `cargo test --lib -- tab_menu_tests:: 보조 auxiliary title_branch shell_keymap command_dispatch`(39건, host transient 전환 전)는 37 통과, 2 실패였습니다.
        - 실패: `file_loading_tests::실제_거절_캐시는_보조_mirror의_…`(153행 텍스트 단언), `view_state_tests::실제_일괄_닫힘은_주_보조_영역의_…`(inlay hints helper 430행 unwrap).
        - 두 검사를 같은 binary로 단독 재실행하면 2/2 통과(41초)입니다. 무거운 실제 App 검사 10여 개가 동시에 돈 부하에서 난 시간·순서 실패로 판단했습니다.
    - `cargo check --all-targets`는 exit 0입니다. 경고는 모두 기존 것입니다.
    - `cargo fmt -- --check --files-with-diff`는 `experiments/lsp-coordinator-spike/…/mock-server.rs`, `application.rs`, `lib.rs`를 냅니다. 모두 직전 단계 QA에 적힌 기존 차이입니다(`application.rs` 933·10819, `lib.rs` 모듈 순서). 이번에 바꾼 줄은 손으로 맞춰 차이가 없습니다. 기존 차이를 건드리지 않으려고 App에는 `cargo fmt`를 돌리지 않았습니다.
- remote-web: `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib`은 exit 0입니다(기존 dead_code 경고).
- 실제 앱 경로
    - 창 이동: `main.rs` → `NativeApplication` → `NativeShell::show`(Main/Auxiliary) → 탭 메뉴 → `ShellIntent::MoveTabToWindow` → `Windows::reserve` → `HostCommand::MoveTabToWindow` → runtime `layout_move_tab_to_window` → `HostReply::TabMovedToWindow` → `Windows::adopt` → `show_auxiliary_windows` viewport
    - 보조 reveal: `RevealInAuxiliaryExplorer` → `Windows::show_explorer`, symbol sidebar Files → `HostCommand::RevealExplorerPath` → `ExplorerPathRevealed`
    - branch: `PaintSink` → `title_branch::Changes` → `background_tick` → `HostCommand::GitBranch` → `git_actions::git_status` → `HostReply::GitBranch` → `AppSurfaces::branch`/`title_branch_icon` → `NativeShell::title_bar`

## 남은 범위

- shell-78의 "변경 열기"·"파일 이력"은 배치 26입니다. 메뉴에 보이지 않습니다.
- 팔레트 명령 `tab.moveToNewWindow`·`tab.moveToMainWindow`는 별도 행 commands-ui-22/23(unwired)입니다. 이번 단계에서 바꾸지 않았습니다. `command-dispatch.rs`의 미지원 단언도 그대로입니다.
- 새 OS 창이 실제로 OS focus를 받는지와 실제 다중 창 실기는 배치 31입니다.
- branch 갱신의 실제 `.git` watcher(`projects.rs` `build_git`) 경로는 자동 검사에서 직접 돌리지 않았습니다. 같은 `GitStatusChanged` event를 발행해 확인했습니다.
- 제목줄 안쪽 좌우 padding(`px-2`)과 글자 크기(`text-xs`)는 기존 native 배치를 유지했습니다. 이번 단계에서 바꾸지 않았습니다.
- 탐색기 선택·rename/create 초안 자체는 여전히 프로젝트 단위로 메인과 공유합니다(대응표 shell-5 남은 범위). 아래 리뷰 수정은 일회 요청(reveal 스크롤·입력 focus)만 창별로 나눴습니다.

## 리뷰 수정 (차단 1건, 비차단 반영)

### 차단: 보조 창 reveal·rename이 메인 탐색기에서 소비됨

- 확인한 결함
    - `Explorer`는 프로젝트마다 하나입니다(`self.explorers[project]`). 일회 필드 `reveal`과 `RenameDraft.focus`는 그 프레임에 먼저 그린 탐색기가 가져가 지웠습니다.
    - 프레임 순서는 메인 창 패스가 먼저이고 보조 viewport가 뒤입니다. 그래서 메인 sidebar가 같은 프로젝트를 보이고 있으면, 보조 창에서 요청한 스크롤과 rename 입력 focus를 메인 탐색기가 받았습니다.
    - 원본은 bridge가 창(webview realm)마다 따로라 요청한 보조 창만 받습니다(`auxiliary-window-shell.tsx` 104-108, `explorer-panel.tsx` 151-172).
    - 스크롤 계산의 현재 offset도 `Explorer` 하나에 저장(`scroll_offset`)돼, 두 창이 같은 값을 덮어썼습니다.
- 수정(`explorer.rs`, `application.rs`, `host.rs`)
    - 탐색기는 그리는 동안 자기 mount(`ShellSlotId`, 메인은 slot id, 보조 창은 그 창의 탐색기 mount id)를 압니다(`show_with_icons`에 mount 인자, `render`가 `mount`를 세팅·해제).
    - 일회 요청에는 대상 mount(`request_mount`)가 붙습니다.
        - 그리는 중에 생긴 요청(키보드 선택, toolbar·문맥 메뉴 생성, F2/문맥 rename, 입력 확정)은 그 mount가 가집니다(`claim_requests`).
        - 밖에서 오는 요청은 응답이 대상 mount를 싣습니다. `HostCommand::RevealExplorerPath`와 `HostReply::ExplorerPathRevealed`에 `mount`를 더했습니다. 메인 탭 메뉴는 slot id, 보조 창 탭 메뉴는 `Windows::show_explorer`가 돌려준 그 창 탐색기 mount를 보냅니다. 붙여넣기 reveal은 clipboard owner의 mount로 보냅니다(`address_requests`).
    - 대상 mount가 아닌 탐색기는 `reveal`로 스크롤하지 않고 지우지도 않으며, 초안 입력에 focus를 요청하지 않습니다(`owns_requests`). 대상이 없으면(`show`로 mount 없이 그리는 검사) 이전과 같습니다.
    - 초안 입력 id에 mount를 넣어 두 창의 TextEdit 상태가 섞이지 않게 했습니다(`native-rename-input`, `native-create-input`).
    - 스크롤 offset은 `Explorer` 필드 대신 그 ScrollArea의 egui 상태(`scroll_area::State::load`, `id_salt`와 같은 `IdSalt`)에서 읽습니다. 창마다 따로입니다.
- 범위 밖 수정과 사유
    - `native/taide-native-app/tests/explorer.rs`의 `show_with_icons` 호출 1곳에 mount 인자를 더했습니다. 공개 시그니처 변경에 맞춘 것이고 단언은 그대로입니다.
- 검사(실제 App, `application-auxiliary-window-tests.rs`)
    - `실제_app_보조_창_탐색기_reveal_rename은_메인_사이드바가_열려_있어도_그_창에서만_스크롤과_포커스를_받는다`
        - 메인 sidebar를 연 채(`sidebar_collapsed = false`) 같은 프로젝트의 보조 창 1을 띄웁니다. 파일 60개 뒤에 `z.txt`를 둬 스크롤이 필요합니다.
        - 보조 창 탭 메뉴 "Explorer 보기에서 보기"(z.txt): 보조 viewport 탐색기 열에 `z.txt` 행이 나타나고, 메인 출력에는 `z.txt`가 없습니다(메인은 스크롤하지 않음).
        - 이어 `a.txt` reveal: 보조 탐색기가 다시 위로 스크롤해 `a.txt`가 보입니다. 보조 창 자신의 offset으로 계산하는지 확인합니다.
        - 보조 창 탭 메뉴 "이름 바꾸기"(a.txt): 보조 viewport의 accesskit focus가 값 `a.txt`, 전체 선택, 보조 탐색기 열 안의 TextInput입니다. 메인 출력에는 focus된 입력이 없고 메인 트리는 계속 보입니다.
        - 보조 viewport에 글자 입력을 보내면 rename 초안 이름이 바뀝니다.

### 비차단 반영

- 재연결 때 진행 중 추적 정리(`application.rs` 닫기 실패 뒤 bridge 재연결, `title-branch.rs`, `auxiliary-windows.rs`)
    - 닫기 실패 뒤 재연결은 이전 bridge의 응답 채널을 버립니다(`HostBridge::disconnect`가 `replies`를 drop하고, `shutdown`은 worker 종료를 기다림). 그래서 그 사이 응답은 오지 않습니다.
    - `Branches::cancel_fetches`: 읽는 중이던 branch를 stale로 되돌려 다음 tick에 다시 요청합니다. 이미 받은 branch 값은 유지합니다.
    - `Windows::settle_reserved`: 이전 worker가 끝난 뒤의 runtime layout과 `WindowRegistry`를 봅니다. 예약 label이 layout의 (project, slot)에 등록돼 있으면 이동이 성공한 것이라 그 창을 채택합니다. 나머지 예약은 풀고, 등록부에 남은 그 label도 지웁니다. 원본에서는 OS 창을 Rust가 열어 응답 유실과 무관하게 창이 있으므로, 이렇게 맞췄습니다.
    - 재연결 경로 자체(닫기 실패)는 자동 검사로 일으키지 않았습니다. 두 helper의 단위 검사가 근거입니다.
        - `연결을_다시_맺으면_읽는_중이던_branch만_다시_요청한다`
        - `연결을_다시_맺으면_응답을_잃은_새_창_이동은_등록부로_채택하고_남은_예약은_푼다`(성공 이동 채택, layout에 없는 등록 label 해제, 미전송 예약 반납)
- 빈 탭 제목(`shell.rs` `title_bar`)
    - 원본 `title-bar.tsx` 15-16, 22, 27, 35는 `tabTitle ?? projectName`이 비면 숨기고, 보조 텍스트는 탭 제목과 프로젝트명이 모두 비어 있지 않을 때만, branch도 비어 있지 않을 때만 그립니다.
    - native는 `Some("")`이어도 `— 프로젝트`를 그렸습니다. 세 값 모두 빈 문자열을 걸러 원본과 맞췄습니다.
    - UI 검사 `제목줄은_탭_제목이_비면_프로젝트_보조_텍스트를_붙이지_않는다`: 빈 제목 탭에서 `window.titleSeparator` 텍스트가 없고, branch 아이콘·이름 묶음이 가운데입니다.
- 대응표 shell-5·shell-6 비고는 이미 이번 단계 사실(탭 메뉴 reveal/rename 연결, branch 공급)로 고쳐져 있습니다. 위 "대응표 shell-5·shell-6 그대로" 기록은 지웠습니다.

### 리뷰 수정 검증

- RED
    - 차단 항목의 실제 App 검사는 수정과 같은 작업 흐름에서 작성됐습니다. 구현을 끄는 재현은 작업 규칙상 하지 않았습니다. 수정 전 코드에서는 메인 탐색기가 먼저 `reveal`을 소비·삭제하므로 "메인 출력에 `z.txt` 없음" 단언과 보조 창 focus 대기가 실패하는 구조입니다.
    - 재연결 helper 단위 검사 2개는 구현 전 실행에서 `settle_reserved`·`cancel_fetches` 없음(E0599)으로 실패했습니다.
    - 빈 제목 UI 검사는 수정 전 실행에서 `" window.titleSeparator prj-…"`가 그려져 실패했습니다(같은 필터의 기존 제목줄 검사 1개는 통과).
- App(`native/taide-native-app`, 최종 코드)
    - `cargo test --lib -- 메인_사이드바가_열려_있어도`는 1/1 통과입니다(44초).
    - `cargo test --lib -- auxiliary_windows:: title_branch::`는 10/10 통과입니다. 새 단위 검사 2개를 포함합니다.
    - `cargo test --lib -- auxiliary_window_tests:: tab_menu_tests:: auxiliary_windows:: title_branch::`는 19/19 통과입니다(130초). 차단 수정 검사, 메인 탭 메뉴 reveal/rename, 보조 창 이동·branch 검사를 포함합니다.
    - `cargo test --lib -- explorer 탐색기 symbol_sidebar context_menu_tests::`는 28/28 통과입니다(241초).
    - `cargo test --test explorer --test explorer-clipboard-owners --test paste-shortcuts`는 21/21, 1/1, 2/2 통과입니다(`paste-shortcuts`는 따로 실행).
    - `cargo check --all-targets`는 exit 0입니다. 경고(`editor-code-actions.rs` 454, `zen.rs` 543, `application-inlay-hints-tests.rs` 234)는 이번 수정 파일이 아닌 기존 것입니다.
    - `cargo fmt -- --check --files-with-diff`는 `mock-server.rs`, `application.rs`, `lib.rs`만 냅니다. 모두 위에 적은 기존 차이(`application.rs` 935·10890, `lib.rs` 모듈 순서)입니다. 이번 수정 줄은 rustfmt 출력에 손으로 맞췄습니다.
- 이전 회귀 실패 2건 재확인(리뷰 비차단 2)
    - 최종 코드(매 tick GitBranch, FsChanged 재조회, host transient 포함)에서 두 검사만 실행하면 2/2 통과입니다(41초).
    - 같은 병렬 필터 `cargo test --lib -- tab_menu_tests:: 보조 auxiliary title_branch shell_keymap command_dispatch`(이번에는 47건)는 46 통과, 1 실패입니다.
        - 실패: `file_loading_tests::실제_거절_캐시는_보조_mirror의_수명_파일_변경과_프로젝트_종료를_따른다`, 153행 `editor.binaryOrTooLarge` 글자 단언입니다. `view_state_tests::실제_일괄_닫힘은_주_보조_영역의_독립_위치와_접기를_보존한다`는 이번에는 통과했습니다.
    - 실패 원인 판단 근거
        - 이 검사는 90행에서 파일을 BINARY로 덮어쓴 직후 열어 `refused`에 들어가기를 기다리고, 한 프레임 뒤 그 글자를 찾습니다.
        - 모든 `FsChanged`는 그 경로를 미리보기 무효화에 넣고(`events.rs` 62-80), 다음 프레임 `reconcile_previews`가 `refused`에서 지웁니다(`application.rs` `reconcile_previews`). 부하로 90행 쓰기의 watcher event가 `refused` 기록 뒤에 오면 그 프레임은 다시 읽는 중이라 글자가 없습니다. 검사 자체의 시간 경합입니다.
        - 이번 단계의 branch 경로는 `PaintSink`에서 `TreeChanges`와 별도인 `title_branch::Changes`에 기록만 하고(`application.rs` 57-64), `refused`·미리보기·탭 상태를 바꾸지 않습니다. GitBranch는 host transient 작업이라 직렬 command 루프를 막지 않습니다. 이 검사는 보조 창을 `Windows`에 등록하지 않고 scope만 바꾸므로 보조 탐색기 수정 경로도 타지 않습니다.
        - 수정 전 코드로 같은 병렬 실행을 돌려 비교하지는 않았습니다(구현 되돌림 금지). 그래서 "이번 변경과 무관"은 위 코드 경로 근거에 따른 판단입니다.
- 범위 밖 실행 기록
    - 탐색기 회귀를 찾다가 `--test explorer-delete`를 2회 실행했습니다. 4건 중 `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다`가 두 번 모두 485행 5초 시간 초과로 실패했습니다.
    - 이 검사는 `taide_file::service::delete_entry`로 임시 디렉터리의 합성 파일 `delete-synthetic.txt`를 macOS 휴지통(NSFileManager)에 보냅니다. 작업 규칙상 실행하지 말았어야 할 검사입니다. 시간 초과가 휴지통 이동 전인지 뒤인지는 확인하지 않았습니다. 사용자 휴지통은 읽거나 바꾸지 않았습니다.
    - 이 검사는 LSP bridge의 삭제 경로만 쓰고, 이번 수정(탐색기 일회 요청, reveal mount, branch, 보조 창 예약)을 지나지 않습니다. 실패 원인은 확인하지 않았습니다.
- UI(`native/taide-native-ui`)
    - `cargo test --features native-host,inspection --test workbench --lib`는 lib 163/163, workbench 35/35 통과입니다. 새 검사는 빈 제목 검사 1개입니다.
    - `cargo check --all-targets --features native-host,inspection`은 exit 0입니다. 경고(`tests/workbench.rs` 1700 `unused_mut`)는 기존 것입니다.
    - `cargo fmt -- --check`는 exit 0입니다.
- remote-web: `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib`는 exit 0입니다(기존 dead_code 경고).
