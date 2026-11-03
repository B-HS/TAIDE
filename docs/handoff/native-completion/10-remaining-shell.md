# 영역 shell 남은 작업 (Rust-native 전환 완료용 인계)

> 작성일: 2026-11-04 · 기준: 저장소 `/Users/hyunseokbyun/development/TAIDE` 브랜치 `to_rust_native` HEAD `0c52b438`(작업 트리 변경 없음). Git 영역은 병합 중인 worktree `/Users/hyunseokbyun/development/TAIDE-lanes/git`(브랜치 `lane/git`, HEAD `50892b59`, `MERGE_HEAD` = `0c52b438`)를 읽었습니다.
> 독자: 이 저장소를 처음 보는 구현 모델. 이 문서만으로 남은 shell 행을 같은 품질로 끝낼 수 있게 썼습니다. 모든 사실은 실제 파일을 열어 확인했고 "경로:줄" 또는 "경로 + 심볼"로 적었습니다. 줄 번호는 위 기준의 값입니다. 바뀌었으면 심볼 이름으로 다시 찾으십시오.

---

## 0. 읽는 법과 전제

### 0.1 표기

| 약어 | 실제 경로 |
| --- | --- |
| `ROOT` | `/Users/hyunseokbyun/development/TAIDE` |
| `GITLANE` | `/Users/hyunseokbyun/development/TAIDE-lanes/git` (Git 영역 최신 코드·QA·기능표, 병합 진행 중) |
| `APP` | `ROOT/native/taide-native-app/src` (`GITLANE/...`로 쓰면 그 worktree의 같은 경로) |
| `UI` | `ROOT/native/taide-native-ui/src` |
| `EDITOR` | `ROOT/native/taide-native-editor/src` |
| `EFRAME` | `ROOT/experiments/native-shell-spike/vendor/eframe` (앱이 쓰는 vendored eframe 0.36.2, `EFRAME/Cargo.toml` 16행) |
| `EGUI` | `ROOT/native/taide-native-app/vendor/egui-input` (앱의 `[patch.crates-io] egui`, `ROOT/native/taide-native-app/Cargo.toml` 111~112행, egui 0.36.2) |
| `TS` | `ROOT/src` (원본 React 앱), `TAURI` = `ROOT/src-tauri` |
| `MONACO` | `ROOT/node_modules/monaco-editor` (0.56.0, `package.json` 3행) |
| 기능표 | `ROOT/docs/quality-assurance/2026-10-09-native-function-matrix.md` |
| shell 사양 | `ROOT/docs/research/2026-10-28-lane-shell-spec.md` |
| 형제 문서 | `ROOT/docs/handoff/native-completion/`의 `14-remaining-git.md`, `16-remaining-commands-ui.md`, `20-gates-31-33.md`, `30-pending-permissions.md` |

### 0.2 GITLANE 병합 상태(2026-11-04 관찰)

- `git -C GITLANE status --short`의 `UU` 8개: 기능표, `APP/application-explorer-views-tests.rs`, `APP/command-dispatch.rs`, `APP/domains/explorer_search.rs`, `APP/explorer.rs`, `APP/navigation-icons.rs`, `APP/shell_keymap.rs`, `UI/command-registry.rs`. 8개 모두 충돌 표식(`<<<<<<<`, `=======`, `>>>>>>>`)이 없습니다(`grep -c` 결과 0). 해결은 끝났고 스테이징만 남은 상태입니다.
- 병합 작업자가 아직 커밋하지 않은 의미 충돌 수정이 있습니다(`git -C GITLANE diff --stat`, 32파일). shell 행에 영향을 주는 것:
    1. IDE diff·close 요청 수신을 shell 수신함으로 합쳤습니다. `GITLANE/APP/provider-events.rs` `Event::IdeDiff(crate::git_claude_diff::Event)`(37행)와 `Inbox::record`의 `IdeDiffRequested`·`IdeCloseTabRequested { request_id: Some }` arm, `GITLANE/APP/domains/shell.rs` `consume_provider_events`의 `Event::IdeDiff(event) => self.accept_claude_diff_event(event)`(1161행), `GITLANE/APP/domains/git.rs` `accept_claude_diff_event`(2263행). 옛 `git_claude_diff::Inbox`와 `git_cache::Changes.ide` 필드는 지웠습니다. 단위 검사 `provider_inbox는_ide_diff_요청과_요청_id가_있는_close_tab만_기록한다`가 `provider-events.rs`에 추가됐습니다.
    2. 제목줄 branch 모듈 `title-branch.rs`를 지우고 `GITLANE/APP/domains/git.rs` `State::title_branch`(git cache status 결과에서 branch를 읽음)로 바꿨습니다. `AppSurfaces.title_branches` 필드와 `PaintSink.title_branches`가 없어졌습니다.
    3. `GITLANE/APP/ide-sync.rs` 91행: IDE 진단 push에서 `DocumentKey::Virtual`(diff 가상 문서)을 뺍니다.
    4. git pull/push 결과가 shell `notify_event`로 OS 알림을 보냅니다(`GITLANE/APP/domains/git.rs` `notify_git_remote`).
    5. `GITLANE/APP/application-notifications-tests.rs`의 `FakePlatform`·`launch_with_platform`이 `pub(in crate::application)`이 됐습니다(다른 영역 검사가 재사용).
- 기능표 행 비고(예: shell-89)는 아직 없는 병합 문서 `2026-11-03-native-git-lane-merge.md`를 가리킵니다. 병합이 끝나면 그 문서가 생기는지 확인하십시오.
- 병합이 끝나 `ROOT`에 들어오면 이 문서의 `GITLANE` 근거를 `ROOT`의 같은 경로에서 다시 확인하십시오. 줄 번호는 병합 결과에서 달라질 수 있습니다.

### 0.3 상태 정의(기능표 머리말 5행)

- complete: 구현 + 실제 앱 경로 + 해당 동작 묶음의 자동 검사. 픽셀·실제 OS 입력·실기(배치 31)는 포함하지 않습니다. 실기만 남으면 complete로 올리고 비고에 실기 항목을 남깁니다.
- partial: 일부 하위 동작·검증이 남음. unwired: 모델·서비스는 있으나 소비자 없음. missing: 사용자 구현 없음. not-applicable/frozen: 범위 밖.

### 0.4 반드시 지킬 운영 규칙(근거 파일)

- 완료 기준: 원본 TS 화면·상태·상호작용 대응 + 실제 앱 경로 + 그 동작의 자동 검사. 원본 결함·내부 수치는 억지로 재현하지 않습니다(`ROOT/docs/memory/2026-10-28-native-lane-operations.md` 4절 59행).
- 금지: GUI 실행, 사용자 실제 데이터·OS 설정·클립보드·Keychain·휴지통(`ROOT/native/taide-native-app/tests/explorer-delete.rs` 실행 금지), 실제 OS 알림, 네트워크, 합성 OS 입력, 저장소 밖 탐색(예외: cargo registry crate 소스 읽기), 동결 `ROOT/native/taide-remote-web` 소스·Cargo 파일 변경(`ROOT/docs/utils/2026-11-02-native-lane-workflow.js`의 `RULES`).
- 코드 주석 금지(공개 API의 영어 `///`만 예외), 의미 있는 숫자는 `const`, `#[allow]`·검사 기대값 조작 금지, 사용자 문자열은 `ROOT/crates/taide-locale/resources/locales/{en,ko,ja}.json`의 원본과 같은 키. 원본이 Monaco 내장 영어 문구를 그대로 보이는 곳은 native도 같은 영어 리터럴을 씁니다(선례: `UI/editor-find-widget.rs` 246~256행 `"Match Case"`, `GITLANE/UI/diff-view.rs` 43행 `HIDDEN_LINES_LABEL = "hidden lines"`).
- 결정 기본값: "큰 변경이 아니면 추천안"(`ROOT/docs/acknowledge/2026-10-26-native-autonomous-continuation.md` 5행). 사용자 허락 보류 목록은 `ROOT/docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` "보류" 표(17~30행)와 형제 문서 `30-pending-permissions.md`의 P번호입니다.
- 새 shell intent는 루트 `ShellIntent`가 아니라 `UI/intents/shell.rs`의 `Intent`에 추가합니다(`ROOT/docs/research/2026-10-28-native-lane-seams.md` 1.3절). 지금 이 enum은 비어 있습니다(`UI/intents/shell.rs` 1~2행 `pub enum Intent {}`), 처리부는 `APP/domains/shell.rs` 687행 `ShellIntent::Shell(intent) => match intent {}`입니다.

### 0.5 형제 문서와의 관계

- shell-89의 남은 구현(ClaudeDiff hunk 되돌리기)은 `14-remaining-git.md` 3.5절(git-50)과 같은 작업입니다. 그 문서가 구현 정본입니다. 이 문서 3.8절은 shell 행 판정에 필요한 요약과 확인 항목만 적습니다.
- shell-92의 diff·close 수신 병합 확인은 `14-remaining-git.md` 3.7절(git-49)과 겹칩니다. 확인은 한 번만 하십시오.
- shell-12의 편집기·루트 경계는 `20-gates-31-33.md` 31-A2절(배치 31 설계 항목)과 같은 작업입니다. 그 절은 A(입력 선처리)/B(전부 지연)/C(체크포인트 복원) 결정을 메인이 사용자에게 묻는다고 적었습니다. 이 문서 3.3절은 같은 라벨을 쓰고 shell 쪽 공통 부분과 각 안의 구현 절차를 더 자세히 적습니다.
- 허락 항목: shell-3 vendored 검사 = P18, shell-5 dialog 부모 창 = P20, shell-15 = P05·P06·P25, 실기 = P23(`30-pending-permissions.md` 1절 표).

---

## 1. 요약

### 1.1 남은 행 수

`grep -n "^| shell-[0-9]"`로 기능표를 전수 확인했습니다.

- `ROOT` 기능표: shell 99행 = complete 86, partial 10, not-applicable 3(shell-13·14·97), unwired·missing·frozen 0.
- `GITLANE` 기능표(병합 작업 트리): complete 87, partial 9, not-applicable 3. 차이는 shell-21(complete)입니다.
- 두 파일 13행의 영역 요약표(`shell | 99 | 54 | 21 | 9 | 12 | 3 | 0`)는 2026-10-14 기준의 낡은 수치입니다. 행별 상태 열이 정본입니다. 행을 바꾸면 요약표를 따로 다시 세십시오.

이 문서가 다루는 행: `ROOT` 기준 complete·not-applicable·frozen이 아닌 10행(모두 partial).

| ID | 기능(기능표 행) | ROOT | GITLANE | 남은 것(한 줄) | 막는 요소 |
| --- | --- | --- | --- | --- | --- |
| shell-3 | 창 위치·크기 복원(96행) | partial | partial | 창 생성 뒤 eframe 치명 오류 종료 코드 경로의 자동 검사 + 실기 | P18(vendored Cargo 변경) + 실기 |
| shell-5 | 보조 창 셸(98행) | partial | partial | 보조 창 WKWebView 배치, 보조 창 터미널 raw input(마우스 보고), 보조 창 theme preview, 파일 선택 dialog 부모 창 | 4번째만 P20 |
| shell-12 | 전역 ErrorBoundary(105행) | partial | partial | 편집기 영역(메인·보조 창) 경계, 루트 `errorBoundary.app`, 픽셀 | 배치 31 설계 항목(31-A2 결정) |
| shell-15 | 기존 사용자 데이터 디렉터리(108행) | partial | partial | 기본 데이터 경로·keychain 승계·Tauri와 공존·롤백 | 사용자 허락 항목(P05) |
| shell-21 | 전역 키맵 toggle-sidebar/explorer/git(114행) | partial | complete | `GITLANE` git-1이 구현. 병합 트리 검증과 `ROOT` 행 반영만 | Git 병합 완료 |
| shell-59 | explorer 4보기(152행) | partial | partial(비고 미갱신) | Git 본문·⌃⇧G가 `GITLANE`에 구현됨. 병합 트리 검증·행 갱신 | Git 병합 완료 |
| shell-78 | 탭 context menu(171행) | partial | partial(비고 미갱신) | 변경 열기(git-48)·파일 이력(git-55)이 `GITLANE`에 구현됨. 낡은 문서 주석 1건 정정 + 보조 창 App 검사 1건 | Git 병합 완료 |
| shell-89 | 탭 콘텐츠 diff/commit diff/claudeDiff/searchEditor(182행) | partial | partial | 네 본문 연결됨(병합 뒤). ClaudeDiff 수정 쪽 hunk 되돌리기(git-50)만 남음 | Git 병합 + git-50 구현 |
| shell-92 | IDE 동기화(185행) | partial | partial | diff·close 수신이 병합 작업 트리에서 shell 수신함으로 합쳐짐. 병합 트리 검증 + 포커스 slot 변경 진단 App 검사 1건 | Git 병합 완료 |
| shell-93 | IPC 동기화(186행) | partial | partial | Git 이벤트 소비(git-64)가 `GITLANE`에 구현됨. 병합 트리 검증·행 판정 | Git 병합 완료 |

상태별 수: partial 10(`ROOT`). Git 병합이 들어오면 shell-21이 complete가 되어 partial 9입니다.

### 1.2 현재 구현 위치(crate·모듈·심볼)

- 앱 crate `native/taide-native-app`(edition 2024, `ROOT/native/taide-native-app/Cargo.toml` 1~5행)
    - 영역 모듈 `APP/domains/shell.rs`(1643줄): `Command`(23행), `Reply`(89행), `State`(110행), `dispatch`(136행), `open_dropped_paths`(522행), `impl NativeApplication`의 `apply_shell_reply`(560행), `apply_shell_intent`(680행), `tick_shell_menus`(835행), `close_project`(1023행), `forget_closed_project`(1061행), `with_external_open`(1085행), `with_relay`(1098행), `consume_provider_events`(1112행), `notify_event`(1175행), `resolve_ide_save`(1273행), `finish_ide_saves`(1311행), `push_ide_diagnostics`(1320행), `window_scopes`(1341행), `is_any_window_focused`(1369행), `is_any_window_tab_dragging`(1415행), `focused_viewport`(1427행), `run_app_menu_action`(1439행), `overlay_viewport`(1473행), `show_auxiliary_windows`(1503행), `close_auxiliary_window`(1574행), `initial_app_menu_model`(1618행). 끝에 App 검사 모듈 4개(1633~1643행: `external_open_tests`, `ide_sync_tests`, `ipc_sync_tests`, `notifications_tests`).
    - shell 보조 모듈: `APP/window-state.rs`(`FILE_NAME` 13행, `main_window_hook` 117행, `MonitorProbe` 127행·`take_result` 144행, `is_zoomed` 202·219행, `Tracker::observe` 236행·`save` 276행, 검사 모듈 288~289행), `APP/stderr-log.rs`, `APP/auxiliary-windows.rs`, `APP/app-menu.rs`, `APP/file-drop.rs`, `APP/file-dialogs.rs`, `APP/toast-layers.rs`, `APP/provider-events.rs`, `APP/external-open.rs`, `APP/single-instance.rs`, `APP/native-notifications.rs`, `APP/ide-sync.rs`, `APP/bootstrap.rs`, `ROOT/native/taide-native-app/src/main.rs`.
    - 공용 통합 지점 `APP/application.rs`: `PaintSink`(50행)·`publish`(62행), `submit`(918행), `background_tick`(1071행), `impl eframe::App for NativeApplication`(1332행: `raw_input_hook` 1333, `raw_input_hook_with_replay` 1342, `logic` 1356, `on_exit` 1365), `WindowPass`(1373행), `frame_ui`(1383행), `finish_window_frames`(1428행), `show_window`(1686행), `finish_frame`(2283행), `AppSurfaces`(2524행부터), `show_unavailable_tab`(2864행), App 검사 모듈 선언(2899~3011행).
- UI crate `native/taide-native-ui`(기능 `native-host`(기본)·`inspection`)
    - `UI/shell.rs`(2795줄): `ShellSurfaces` trait(225행), `NativeShell::show`(366행, 보조 창 분기 383~436행), `main_window`(449행), `sidebar_panel`(577행), `slot_tree`(1251행), `pane_tree`(1536행), `surfaces.tab_content(..)` 호출(1770행), `tab_menu`(2257행).
    - `UI/error-boundary.rs`(374줄): `Region`(42~47행), `label_key`(50행), `is_row`(59행), `is_unmounted_while_hidden`(63행), `Outcome`(75행), `show`(94행), `show_instance`(107행), `capture`(170행), `inject_panic`(356행), `inject_instance_panic`(361행).
    - `UI/surfaces/{editor,explorer_search,git,preview,terminal_agent}.rs`(영역 trait), `UI/intents/*.rs`, `UI/commands.rs`(루트 `ShellIntent`), `UI/tab-drag.rs`, `UI/project-drag.rs`, `UI/presentation.rs`(`apply_visuals` 227행).
- vendored: `EFRAME/src/native/run.rs`(`WinitAppWrapper` 80행, `SharedResult` 88행, `EframeWinitApplication` 495행, `new` 551행, `take_result` 564행), `EFRAME/src/native/wgpu_integration.rs`(`render_immediate_viewport` 1139~1265행), 변경 기록 `EFRAME/TAIDE-CHANGES.md`.

### 1.3 다른 영역과의 의존·소유 경계

| 상대 영역 | 경계 | 이 문서의 행 |
| --- | --- | --- |
| git(`GITLANE`, 병합 중) | Git 보기 본문 `show_git_view`, ⌃⇧G `Run::ShowGit`, diff·commit diff·ClaudeDiff 탭 본문, 탭 메뉴 변경 열기·파일 이력, IDE diff·close 수신, Git 캐시 무효화 | 21, 59, 78, 89, 92, 93 |
| explorer-search(병합됨 `6dba64f9`) | 탐색기 mount별 상태(`explorer_mounts::Mounts`), 검색 패널, 검색 편집기 탭 본문 | 5, 59, 89 |
| editor+preview(병합됨) | `APP/preview_web_view.rs`(WKWebView), `APP/domains/preview.rs` `show_preview_web_views`(626행), 편집기 표면 `UI/editor_surface.rs`, `EDITOR/store.rs` | 5, 12 |
| terminal-agent(병합됨 `446964b7`) | `APP/terminal_surface.rs` raw input(마우스 보고) | 5 |
| settings(병합됨 `3864e1db`) | `APP/domains/settings.rs` `sync_theme_preview`(1218행)·`apply_presentation_appearances`(1258행) | 5 |
| commands-ui(wave 3, 미시작) | commands-ui-82 error-boundary는 shell-12와 같은 기능, 판정은 함께(`ROOT/docs/research/2026-10-28-lane-commands-ui-spec.md` 520~526행) | 12 |
| 메인(wave 4, 배치 31) | 편집기 장애 경계 설계, 실기, 출시 게이트 | 3, 5, 12 |
| 사용자 허락 | shell-15(P05), vendored eframe Cargo 변경(P18), 저장소 밖 crate 소스 확인(P20) | 3, 5, 15 |

소유 규칙(`ROOT/docs/research/2026-10-28-native-lane-seams.md` 1.4~1.5절): 영역 상태는 `self.<영역>_state` 안에 두고, 다른 영역 상태는 읽기만 하며 바꿀 때는 그 영역의 `pub(crate)` 메서드를 부릅니다. 공용 파일(`application.rs`, `lib.rs` 모듈 목록, `host.rs` enum 루트, locale 카탈로그)은 영역 블록 끝에 추가만 합니다. wave 1·2 lane이 대부분 병합됐으므로 shell-5·12 작업은 메인(wave 4)이 다른 영역 파일을 함께 고칠 수 있습니다. 고칠 때는 그 영역의 기존 검사를 함께 실행하십시오.

### 1.4 권장 구현 순서

1. Git 병합 직후 판정 묶음(코드는 이미 있음): shell-21 → shell-59 → shell-78 → shell-93 → shell-92. shell-78은 문서 주석 1줄 정정과 보조 창 App 검사 1건을, shell-92는 포커스 slot 변경 App 검사 1건을 함께 합니다.
2. shell-89: `14-remaining-git.md` 3.5절대로 git-50 hunk 되돌리기를 구현한 뒤 판정합니다.
3. shell-5: 허락이 필요 없는 순서로 (1) 보조 창 theme preview → (2) 보조 창 WKWebView 배치(vendored eframe 수정, 결정 6 A로 허용) → (3) 보조 창 터미널 raw input(vendored eframe 수정) → (4) dialog 부모 창(P20 뒤).
4. shell-3: 앱 쪽 종료 결과 조합 함수 단위 검사(허락 불필요)를 먼저 하고, vendored eframe 셀 공유 검사는 P18 허락 뒤 실행합니다.
5. shell-12: 배치 31 설계 항목. 31-A2 결정 → 편집기 영역 경계 → 루트 경계.
6. shell-15: 사용자 허락 뒤(배치 32 cutover와 붙어 있음).

---

## 2. 영역 공통 지식

### 2.1 egui 0.36·Rust 2024 최소 지식(이 영역에서 실제로 쓰는 것)

1. 즉시 모드: egui는 매 frame UI 전체를 다시 만듭니다. 한 frame은 `egui::Context::run_ui(RawInput, |ui| { .. }) -> FullOutput`입니다. `FullOutput`에는 `shapes`(그린 도형), `platform_output.accesskit_update`(접근성 트리), `textures_delta`, `viewport_output`이 있습니다. 검사는 `output.textures_delta.clear()`를 반드시 부릅니다(비우지 않고 버리면 실패한 선례, `ROOT/docs/quality-assurance/2026-10-27-native-error-isolation-file-drop.md` 검증 절).
2. 다중 pass: `ctx.request_discard(reason)`를 부르면 같은 frame을 한 번 더 그립니다(기본 최대 2 pass). 입력 event는 첫 pass에만 들어갑니다(`EGUI/src/context.rs` 1018행 `run_dyn`, 1049행 `self.begin_pass(new_input.take())`).
3. root `Ui`의 스타일: `run_ui`가 pass 시작에 root `Ui`를 만들 때 그 시점의 `ctx.global_style()`을 복사합니다(`EGUI/src/context.rs` 996행 `Ui::new(..)`, `EGUI/src/ui.rs` 136행 `style.unwrap_or_else(|| ctx.global_style())`). 자식 `Ui`는 부모의 스타일을 물려받습니다(`EGUI/src/ui.rs` 237행 `Arc::clone(&self.style)`). 그래서 pass 도중에 `ctx` 스타일을 바꾸면 이미 만들어진 `ui`와 그 자식에는 반영되지 않고, 새로 만드는 `Area`(popup·tooltip)에만 반영됩니다. shell-5 theme preview에서 중요합니다.
4. viewport: `ctx.show_viewport_immediate(id, builder, |ui, class| ..)`는 부모 pass 안에서 동기로 콜백을 부릅니다(`EGUI/src/context.rs` 4754~4809행). 렌더러가 보조 viewport의 `run_ui`를 시작하기 전까지 `ctx.viewport_id()`는 부모(보통 ROOT)입니다. `begin_pass`가 viewport stack에 새 id를 넣습니다(`EGUI/src/context.rs` 550행 `self.viewport_stack.push(ids)`, 835~837행 `viewport_id`).
5. 층: `egui::Order::{Background, Middle, Foreground, Tooltip, Debug}`. 같은 Order 안의 상하는 `ctx.set_sublayer(parent, child)`로 정합니다(shell-94 선례 `APP/status-error-banner.rs`, `APP/file-drop.rs`).
6. eframe `App` trait(vendored 0.36.2): `logic(&mut self, ctx, frame)`(ROOT, `ui` 전), `ui(&mut self, ui, frame)`, `raw_input_hook(ctx, input)`·`raw_input_hook_with_replay`(TAIDE 추가, `EFRAME/TAIDE-CHANGES.md`), `on_exit`. 앱 hook은 ROOT·deferred viewport 입력 준비에서만 불립니다(`EFRAME/src/native/epi_integration.rs` 389행). 검사용 생성자는 `eframe::CreationContext::_new_kittest(context)`, `eframe::Frame::_new_kittest()`입니다.
7. Rust 2024: `if let Some(x) = a && cond { .. }`(let chain), `let Some(x) = y else { return; };`. kebab-case 파일은 `#[path = "file-name.rs"] mod file_name;`로 선언합니다(`APP/lib.rs` 360~361행 `#[path = "window-state.rs"] pub mod window_state;`).
8. panic 격리: `std::panic::catch_unwind(AssertUnwindSafe(..))`. egui `Context`와 runtime `AppState`는 `parking_lot` 잠금이라 poisoning이 없어 panic 뒤 반쯤 바뀐 상태가 감지되지 않습니다(`ROOT/docs/quality-assurance/2026-10-27-native-error-isolation-file-drop.md` shell-12 절). 그래서 경계는 "render가 공유 상태를 바꾸지 않거나 바꾼 것을 되돌릴 수 있는 영역"에만 둡니다(결정 A, `ROOT/docs/acknowledge/2026-10-26-native-autonomous-continuation.md` 32행).
9. 가시성: 검사 helper를 다른 검사 모듈에서 쓰려면 `pub(super)`로 올립니다. `pub(super)`는 부모 모듈과 그 하위 모듈 전부에서 보입니다. 예: `APP/application-auxiliary-window-tests.rs`의 `install_viewports`(62행)는 `pub(super)`라 `application::domains::git::diff_tests`에서도 씁니다.

### 2.2 상태가 있는 곳

- shell 영역 상태 `NativeApplication.shell_state: domains::shell::State`(`APP/domains/shell.rs` 110~135행): `welcome`, `app_menu`, `open_path_dialog`, `project_display_dialog`, `project_group_dialog`, `project_group_delete`, `system_usage`, `system_usage_appearance`, `system_usage_icon`, `zen_hint`, `zen_hint_colors`, `file_drop_colors`, `auxiliary_windows`, `window_state`(`window_state::Tracker`), `auxiliary_pass_root`, `overlay_viewport`, `zen_fullscreen_state`, `provider_events: Arc<provider_events::Inbox>`, `notifications`, `ide_sync`, `main_window_zoomed: Option<bool>`.
- 공용 루트 필드(seams 문서 1.5절): `runtime`, `services`(`Arc<AppServices>`), `application_ports`, `controller`, `bridge`, `shell: NativeShell`(UI 셸 위젯, `scope: WindowScope`·`colors` 포함), `locale`, `store: EditorStore`, `files`, `untitled`, `toasts: ToastLayers`, `status`, `closing`, `is_exit_ready`, 편집기 appearance 필드들.
- 다른 영역 상태: `explorer_search_state`, `terminal_agent_state`, `settings_state`, `preview_state`, `git_state`, `commands_state`.

### 2.3 흐름

#### 2.3.1 host 명령(`HostCommand::Shell`)

1. 앱 코드가 `self.submit(HostCommand::Shell(Command::X { .. }))`를 부릅니다(`APP/application.rs` 918행). 큐가 차면 `false`를 돌려주므로 호출부가 재시도 플래그를 남깁니다(예: `APP/domains/shell.rs` 1124행 `request_external_opens`).
2. `APP/host.rs`의 직렬 worker(큐 용량 64, `ROOT/docs/architecture.md` 9.2절)가 `domains::shell::dispatch(worker, command)`(136행)를 부릅니다.
3. 결과가 있으면 `Some(HostReply::Shell(Reply::X))`를 돌려주고 다음 frame의 `poll()`이 `apply_shell_reply(context, reply)`(560행)로 반영합니다.
4. 새 명령은 `Command`·`Reply` 끝, `dispatch` match, `apply_shell_reply` match 세 곳에만 추가합니다. `host.rs`·`application.rs`의 라우팅 줄은 손대지 않습니다.

#### 2.3.2 UI intent(`ShellIntent`)

1. `NativeShell::show`(`UI/shell.rs` 366행)가 `Vec<ShellIntent>`를 돌려줍니다.
2. `APP/application.rs` `show_window`의 intent 루프가 영역 wrapper(`ShellIntent::Git(..)` 등)를 영역 함수로 보내고 나머지 shell 변형을 `apply_shell_intent(intent, frame, &snapshot)`(`APP/domains/shell.rs` 680행)로 처리합니다.

#### 2.3.3 렌더 중 요청(`ShellSurfaces`)

- 루트 trait `ShellSurfaces`(`UI/shell.rs` 225행)는 영역 trait들의 supertrait입니다. 구현자는 앱 `AppSurfaces`(`APP/application.rs` 2524행)와 UI 검사 `ROOT/native/taide-native-ui/tests/workbench.rs`의 `Surfaces`(118행, `impl ShellSurfaces for Surfaces` 152행) 두 곳입니다. 메서드를 추가하면 둘 다 고칩니다.
- `show_window`는 매 pass 지역 출력 값을 만들고(`APP/application.rs` 1700~1721행: `command_errors`, `commands`, `settings_output`, `keymap_actions`, `keymap_editor_scope`, `keymap_documents`, `explorer_output`, `load_files`, `preview_output`, `web_placements`, `save_missing_drafts`, `load_untitled`, `banner_actions`, `changed_documents`, `new_focus`, `status`) 루트 필드의 `&mut`과 함께 `AppSurfaces`에 담은 뒤 `self.shell.show(ui, &snapshot, &mut surfaces)`를 부릅니다(1971~1978행, `ui.add_enabled_ui(..)` 안).

#### 2.3.4 provider 이벤트(원본 app provider 대응)

1. 백엔드 `AppEvent`(`ROOT/crates/taide-model/src/app_event.rs`)는 모두 `PaintSink::publish`(`APP/application.rs` 62행)를 지납니다. shell은 `provider_events::Inbox::record`(`APP/provider-events.rs` 53행)에 기록합니다: `AgentStateChanged`, `TerminalCommandFinished`, `LspInstallProgress`(Done/Failed), `IdeSaveRequested`, `ProjectClosed`, `AgentExternalOpen`(플래그). 상한 `MAX_PROVIDER_EVENTS` = 256(11행). 병합 작업 트리는 여기에 `IdeDiffRequested`·`IdeCloseTabRequested`를 더했습니다(0.2절).
2. `background_tick`(1071행)이 `self.poll(context)` 뒤 `consume_provider_events(context, Instant::now())`(1101행)와 `sync_theme_preview(context)`(1102행)를 부릅니다. 종료 중이면 꺼낸 이벤트를 처리하지 않고 버립니다(`APP/domains/shell.rs` 1114~1117행).

#### 2.3.5 한 frame의 순서(창이 여럿일 때)

1. `logic`(1356행): `shell_state.main_window_zoomed = window_state::is_zoomed(frame)` → `background_tick`.
2. `ui` → `frame_ui`(1383행): 공통 tick → `WindowPass` 생성 → `show_window(ROOT)` → `show_auxiliary_windows`(보조 창마다 `show_viewport_immediate` 안에서 `show_window`를 그 창 범위로 다시 부름, `APP/domains/shell.rs` 1535~1552행) → `finish_window_frames`(1428행, `sync_theme_preview(context)`, 이때 context의 viewport는 ROOT) → `finish_frame`(2283행, WKWebView 동기화, 메인 toast).
3. `show_window`는 ROOT일 때만 `frame_pass.web_placements`와 `root_enabled`를 기록합니다(`APP/application.rs` 2047~2050행). 보조 창 pass의 placement는 지역 변수로 버려집니다.

### 2.4 검사 하네스

#### 2.4.1 실행 명령

`<T>`는 target 디렉터리입니다. 메인 게이트는 `ROOT/experiments/native-shell-spike/target`(`ROOT/docs/utils/2026-11-03-native-merge-gate.sh` 5행), lane은 `/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`입니다.

1. LSP mock을 먼저 빌드합니다. 빼면 앱 lib 검사 10건이 실패합니다(`ROOT/docs/memory/2026-10-07-native-transition-operations.md` 6절).
   `cargo build --quiet --manifest-path ROOT/native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir <T>`
2. 앱 lib 필터(실제 App 검사 포함): `cargo test --manifest-path ROOT/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- <필터...> --test-threads=4`(부하가 큰 병렬 환경은 2). 필터는 검사 경로의 부분 문자열입니다. 예: `ipc_sync_tests`, `domains::git::base_tests`, `application::auxiliary_window_tests`, `window_state::tests`.
3. UI 하네스: `cargo test --manifest-path ROOT/native/taide-native-ui/Cargo.toml --features native-host,inspection --locked --offline --target-dir <T> --test workbench -- <필터>`. UI 단위는 `--lib -- <필터>`.
4. 동결 remote-web(UI crate나 vendored eframe을 바꿨을 때 필수): `cargo check --quiet --manifest-path ROOT/native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline --target-dir <T>`. vendored eframe을 바꿨으면 같은 명령에 `--features canvas`도 실행합니다(shell 사양 133행).
5. 편집기 core를 바꿨으면: `cargo test --manifest-path ROOT/native/taide-native-editor/Cargo.toml --locked --offline --target-dir <T> -- <필터>`.
6. 형식: `cargo fmt --manifest-path <바꾼 crate>/Cargo.toml -- --check`.
7. 병합 게이트 전체(메인만): `bash ROOT/docs/utils/2026-11-03-native-merge-gate.sh <로그 디렉터리>`.

#### 2.4.2 UI 하네스(egui 단독, 앱 없이)

- 파일: `ROOT/native/taide-native-ui/tests/workbench.rs`(5871줄). helper: `fixture()`(429행, 두 slot이 있는 `ShellSnapshot`), `rail_frame(context, snapshot, Surfaces, time, events) -> (Surfaces, Vec<ShellIntent>, FullOutput)`(2763행), `welcome_frame`(1536행), `alert_node`(5180행), `has_text`(5196행), 상수 `FRAME_SECONDS`(62행), `PANEL_SETTLE_FRAMES`(96행).
- `rail_frame` 본문의 형태:

```rust
let mut intents = Vec::new();
let mut output = context.run_ui(input, |ui| {
    intents.extend(shell(WindowScope::Main).show(ui, snapshot, &mut surfaces));
});
output.textures_delta.clear();
(surfaces, intents, output)
```

- 경계 검사 예(5574행 `사이드바_패널_fallback은_원본처럼_접었다_펴도_남는다`): `taide_native_ui::error_boundary::inject_instance_panic(&context, Region::SidebarPanel, egui::Id::new(&slot))`(5600행)로 다음 pass에 panic을 주입하고 fallback 문구 `"errorBoundary.sidebarPanel"`을 `has_text`로 단언합니다. 검사 Surfaces는 locale을 쓰지 않아 키 문자열이 그대로 그려집니다.

#### 2.4.3 앱 lib 실제 App 검사(실제 `NativeApplication`)

앱 crate의 dev-dependency로 UI가 `inspection` 기능과 함께 붙으므로 앱 검사에서도 `inject_panic`을 쓸 수 있습니다.

- 기본 fixture(`APP/application-project-rail-tests.rs`): `Fixture`(15행, `Drop`이 `on_exit` 뒤 임시 디렉터리 삭제), `project_ref`(69행), `isolated_state(data)`(80행, IDE 서버·agent hooks 끔), `insert_project`(95행), `launch(fixture, state) -> egui::Context`(120행).
- 가짜 플랫폼(`APP/application-notifications-tests.rs`): `FakePlatform`(26행), `launch_with_platform`(55행, `NativeApplication::new_with_overrides(.., Overrides { platform: Some(platform), ..Default::default() })`), `frame(application, context, focused)`(84행), `frame_until(application, context, focused, phase, ready)`(102행, 시한을 넘으면 `phase=..` 메시지로 실패).
- frame 한 번의 실제 형태(`APP/application-notifications-tests.rs` 84~100행):

```rust
let mut output = context.run_ui(
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(SCREEN[0], SCREEN[1]),
        )),
        focused,
        ..Default::default()
    },
    |ui| {
        eframe::App::logic(application, ui.ctx(), &mut eframe::Frame::_new_kittest());
        eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest());
    },
);
output.textures_delta.clear();
```

- 프로젝트 하나와 파일 탭(`APP/application-ipc-sync-tests.rs`): `Opened`(20행), `open_project(name, files)`(29행), `open_project_with(name, files, setup)`(33행), `edit(application, path, text)`(119행).
- 보조 창(`APP/application-auxiliary-window-tests.rs`): `Viewports`(33행, viewport별 `events`·`outputs`), `Viewports::texts`(41행), `install_viewports(context)`(62행, `context.set_embed_viewports(false)` + `egui::Context::set_immediate_viewport_renderer`로 보조 viewport 출력을 따로 받음), `step_until`(108행), `file_tab`(125행), `auxiliary_window`(139행), `viewport_of`(152행), `choose_tab_menu`(490행, 비공개), `focused_text_input`(832행), `SingleWindow`·`single_window(name)`(1341·1350행).
- 창 상태(`APP/application-window-state-tests.rs`): `frame`(35행), `frame_until`(58행), `render_auxiliary_viewports`(76행), `saved_window_states`(120행).
- 장애 경계(`APP/application-error-boundary-tests.rs`): `fixture`(11행), `label`(23행), `alert`(27행), `step_until`(42행), `retry`(59행), `has_text`(73행), `assert_fallback`(79행), 실제 검사 4건(95·150·172·222행 부근).
- Git 저장소가 필요한 검사(`GITLANE/APP/application-git-fixture.rs`): `fixture::git(root, empty, args)`(10행, 전역 설정·hook 차단), `fixture::init`(33행), `fixture::repository(root, empty, files)`(64행).
- 검사 이름은 한국어로 씁니다(기존 관례).

---

## 3. 행별 상세

각 행은 (a) ID·기능·상태 (b) 원본 TS 동작 (c) native 현재 자산과 정확히 빠진 것 (d) 구현 방법 (e) 자동 검사 (f) 완료 판정 (g) 막는 요소 순서입니다.

---

### 3.1 shell-3 창 위치·크기 복원

#### (a) ID·기능·상태

- shell-3 "창 위치/크기 복원 (window-state 플러그인)", 기능표 96행, `ROOT`·`GITLANE` 모두 partial.

#### (b) 원본 TS 동작

- `TAURI/src/lib.rs` 857~868행: `tauri_plugin_window_state::Builder::new().map_label(domain::window::service::normalize_window_state_label).with_filter(|label| label != domain::window::types::AUXILIARY_WINDOW_STATE_KEY)`. 메인 창의 크기·위치·최대화·전체 화면을 저장·복원하고, 보조 창은 저장하지도 복원하지도 않습니다(860~867행 코드 주석).
- 저장 사각형이 연결된 모니터와 겹칠 때만 위치를 적용하는 판정은 플러그인 내부(저장소 밖)입니다. 기능표 행 비고가 `MonitorExt::intersects` 규칙을 적었습니다.
- 로거: `TAURI/src/lib.rs` 834행 `tauri_plugin_log::Builder::new()`.
- 창 생성 뒤 치명 오류: Tauri는 앱이 오류로 끝나면 0이 아닌 종료 코드를 돌려줍니다(일반 프로세스 동작).
- 사용자 문자열 없음(locale 키 없음).

#### (c) native 현재 자산과 빠진 것

- 구현됨
    - `APP/window-state.rs`: `main_window_hook(data_dir, monitors)`(117행)가 저장 상태를 메인 `ViewportBuilder`에 적용, `MonitorProbe`(127행)가 `resumed`에서 모니터 목록을 읽음, `Tracker::observe`(236행)가 `maximized = info.maximized == Some(true) || is_zoomed == Some(true)`(256행)로 기록, `Tracker::save`(276행) → `--data-dir`의 `native-window-state.json`(`FILE_NAME` 13행).
    - `is_zoomed`(202행, macOS): `eframe::Frame`의 AppKit 핸들 `ns_view` → `[view window]` → `[window isZoomed]`. 다른 OS(219행)는 `None`.
    - `ROOT/native/taide-native-app/src/main.rs`: 첫 줄 `taide_native_app::stderr_log::install()`, `eframe::create_native(..)` → `window_state::MonitorProbe::new(application, monitors)` → `event_loop.run_app(&mut probe)?` → 생성 결과(`startup` 셀, `None`이면 `WINDOW_NOT_CREATED` 상수 = `"native main window was not created"`) → `Ok(probe.take_result()?)`(87~94행).
    - vendored: `EFRAME/src/native/run.rs` `WinitAppWrapper.return_result: SharedResult`(83행, 타입 88행 `Rc<RefCell<Result<(), crate::Error>>>`), 오류 기록(167행), `run_and_return`이 같은 셀에서 꺼냄(389~397행), `EframeWinitApplication { wrapper, control_flow, result }`(495~499행), `new`가 `result: app.return_result.clone()`(551~557행), `take_result`가 `self.result.replace(Ok(()))`(564행).
    - 검사: `APP/window-state-tests.rs` 단위 15건(79~387행, 예: 357행 `appkit_zoom은_egui가_최대화를_모를_때도_최대화로_기록하고_직전_일반_크기와_위치를_지킨다`, 387행 `창_핸들이_없는_frame의_zoom_상태는_알_수_없음이다`), `APP/application-window-state-tests.rs` 실제 App 3건(127·222·424행), `stderr_log` 단위 1건.
- 빠진 것(`ROOT/docs/quality-assurance/2026-11-02-native-lane-shell-windows-layers.md` "shell-3 complete → partial" 절 111~128행을 코드로 재확인)
    1. 창 생성 뒤 eframe 치명 오류가 0이 아닌 종료 코드가 되는 경로의 자동 검사가 없습니다. 이유
        - vendored eframe 단위 검사: eframe은 앱 workspace의 비멤버 path 의존이고 dev-dependency `directories`(`EFRAME/Cargo.toml` 272~273행)가 lock에 없습니다. `cargo test -p eframe`은 `package eframe cannot be tested because it requires dev-dependencies and is not a member of the workspace`(exit 101)로 거부됩니다.
        - 앱 쪽: `EframeWinitApplication`은 `create_native(.., &EventLoop)`로만 만들 수 있고 `WinitAppWrapper::new`·`WinitApp` trait는 crate 비공개입니다. macOS winit event loop는 주 스레드를 요구해 libtest 스레드에서 만들 수 없습니다.
    2. 실기(배치 31): 실제 모니터 읽기·위치 적용·AppKit `isZoomed`, 실제 eframe 오류 종료 코드.

#### (d) 구현 방법

두 검사로 나눕니다. 1은 허락이 필요 없고, 2는 P18 허락이 필요합니다.

1. 앱 쪽 종료 결과 조합(허락 불필요)
    1. `APP/window-state.rs`에 공개 함수 `pub fn exit_result(startup: Option<Result<(), String>>, eframe: eframe::Result) -> Result<(), Box<dyn std::error::Error>>`를 추가합니다. 규칙은 지금 `main.rs` 89~94행과 같습니다: `Some(Err(message))`면 그 문구의 오류, `None`이면 `"native main window was not created"`, 그 밖이면 `eframe` 결과를 `?`로 돌려줍니다. 문구 상수 `WINDOW_NOT_CREATED`를 `main.rs`에서 이 모듈로 옮깁니다.
    2. `main.rs` 89~94행(`let outcome = ..`부터 `Ok(probe.take_result()?)`까지)을 `window_state::exit_result(startup.borrow_mut().take(), probe.take_result())`로 바꿉니다. 호출 순서(`run_app` 뒤 생성 결과 → eframe 결과)는 그대로입니다.
    3. 공개 문서는 영어 `///` 한 단락만 씁니다.
2. vendored eframe 셀 공유 단위 검사(P18 허락 필요)
    1. 허락 선택지(형제 문서 30 P18): (가) `EFRAME/Cargo.toml`의 `[target.'cfg(not(target_arch = "wasm32"))'.dev-dependencies.directories]`(272~273행) 제거, (나) lock에 `directories` 추가, (다) 별도 검사 workspace. 추천은 (가)입니다: 이 dev-dependency를 쓰는 곳은 `EFRAME/src/native/file_storage.rs` 257~270행의 `directories_storage_dir`과 upstream 검사 `storage_path_matches_directories`뿐이라(`grep -n directories EFRAME/src/native/file_storage.rs`) 둘을 함께 지우면 됩니다.
    2. `EFRAME/src/native/run.rs` 끝에 `#[cfg(test)] mod tests`를 둡니다. 가짜 `WinitApp`(trait 정의 `EFRAME/src/native/winit_integration.rs` 89~125행: `egui_ctx`, `window`, `window_id_from_viewport_id`, `save`, `save_and_destroy`, `run_ui_and_paint`, `suspended`, `resumed`, `device_event`, `window_event`, `on_accesskit_event`)를 모든 메서드가 `None`/`Ok(EventResult::Wait)`를 돌려주는 빈 구조체로 구현합니다.
    3. 검사 본문:

```rust
let wrapper = WinitAppWrapper::new(Fake, false);
let cell = wrapper.return_result.clone();
let mut application = EframeWinitApplication::new(wrapper);
*cell.borrow_mut() = Err(crate::Error::AppCreation("synthetic".into()));
assert!(application.take_result().is_err());
assert!(application.take_result().is_ok());
```

- 2단계 3의 요점: 위 검사에서 오류를 `new` 뒤에 넣어야 "같은 셀을 공유한다"를 검증합니다(값 복사로 바뀌면 첫 단언이 실패). `crate::Error::AppCreation`은 `EFRAME/src/lib.rs` 496행의 변형입니다.
- 2단계 4: `EFRAME/TAIDE-CHANGES.md`에 dev-dependency 제거 이유와 검사 추가를 한 절로 적습니다.
- 2단계 5: 실행 전후로 `ROOT/native/taide-native-app/Cargo.lock`이 바뀌지 않는지 `git diff --stat`으로 확인합니다. 바뀌면 중단하고 보고합니다.
- 2단계 6: dev-dependency를 지운 뒤에도 cargo가 비멤버 패키지 검사를 다른 이유로 거부하면 이 방법을 버리고, 메인에게 "컴파일과 코드 확인 + 1번 검사로 인정"을 결정받아 `ROOT/docs/acknowledge/`에 기록합니다.
- 쓰지 말아야 할 대안: 검사 전용 `#[doc(hidden)] pub fn`을 vendored eframe 배포 코드에 열어 앱 검사에서 부르는 방법은 검사를 위해 배포 API를 늘리는 우회라 쓰지 않습니다.

#### (e) 자동 검사

- 1: `APP/window-state-tests.rs`에 `종료_결과는_생성_실패를_먼저_돌려주고_창이_없으면_오류이며_그_뒤_eframe_오류를_돌려준다` 1건. 사례 4개: `Some(Err("x"))`+`Ok` → 오류 문구 `x`, `None`+`Ok` → 오류 문구 `native main window was not created`, `Some(Ok)`+`Err(AppCreation)` → 오류, `Some(Ok)`+`Ok` → `Ok`.
    - 명령: `cargo test --manifest-path ROOT/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- window_state::tests --test-threads=4`
- 2: `cargo test --quiet -p eframe --lib --manifest-path ROOT/native/taide-native-app/Cargo.toml --locked --offline --target-dir <T> -- native::run::tests`
- vendored eframe을 바꿨으면 remote-web Wasm check 두 번(기본, `--features canvas`).

#### (f) 완료 판정

- 1과 2가 모두 통과하면 complete. 비고에 "실기(배치 31): 실제 모니터·위치·AppKit zoom 판정·eframe 오류 종료 코드"를 남깁니다.
- 2를 허락받지 못하면 1만으로 complete로 올리지 않습니다. shell lane 리뷰가 "검사 없이 complete"를 과대 판정으로 되돌린 선례가 있습니다(windows-layers QA 111~123행). 메인이 다른 판정을 명시적으로 결정하면 그 결정을 기록한 뒤 따릅니다.

#### (g) 막는 요소

- 2는 vendored eframe Cargo 파일 변경이라 P18 허락이 필요합니다. 허락 전에는 1만 하고 행 비고에 "셀 공유 검사 대기(P18)"를 적습니다.
- 실기는 배치 31(P23, 사용자 GUI 실행).

---

### 3.2 shell-5 보조 창 셸

#### (a) ID·기능·상태

- shell-5 "보조 창 셸 (고정 project/windowSlot, explorer+editor만, 마지막 탭 이동/닫기 시 창 닫기, layout 오류 시 닫기)", 기능표 98행, partial.

#### (b) 원본 TS 동작

- `TS/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 73~146행 `AuxiliaryWindowShell`
    - 제목줄, view-local 탐색기 `Panel`, 편집기 `Panel`. 둘 다 `ErrorBoundary`(132행 `errorBoundary.sidebarPanel`, 138행 `errorBoundary.editorArea`)로 감쌉니다.
    - 키맵 96~99행: `toggle-sidebar`, `explorer`, `git`(`requestShowExplorerView('git')`). 106~108행: 보기 전환·reveal·rename 요청이 접힌 패널을 펼칩니다.
    - layout 오류이거나 트리가 비면 OS 창을 닫습니다(110~112행 부근 effect).
- 보조 창도 앱 provider 묶음(toast, IPC 동기화, theme, 키맵 런타임, 팔레트, Task Runner)을 mount합니다(`TS/app/app.tsx`, shell 사양 690~691행).
- 보조 창의 `EditorArea`는 메인과 같은 탭 콘텐츠를 그리므로 HTML·오디오·비디오 미리보기(iframe)와 터미널(xterm, 마우스 보고 포함)이 그 창 안에서 동작합니다.
- theme preview는 realm(창)마다입니다: `TS/entities/theme/theme.query.ts` 65~67행 모듈 store, `TS/app/providers/theme-provider.tsx` 30행 `previewTheme ?? resolvedTheme`. 보조 창 설정 탭에서 켠 preview는 그 창에만 적용됩니다(shell 사양 690행).
- 파일 선택 dialog의 부모 창: 원본 `tauri-plugin-dialog`가 부모를 호출 창으로 정하는지는 저장소 밖 crate 소스에 있습니다(shell 사양 848행 허락 4 = P20).
- 사용자 문자열: `errorBoundary.sidebarPanel`, `errorBoundary.editorArea`. 이 행의 남은 범위에는 새 키가 없습니다.

#### (c) native 현재 자산과 빠진 것

- 구현됨(근거 `ROOT/docs/quality-assurance/2026-10-27-native-auxiliary-window.md`, `-moves.md`, `2026-11-02-native-lane-shell-windows-layers.md`): 보조 창 viewport·제목줄·view-local 탐색기·편집기(`show_auxiliary_windows` 1503~1559행, `UI/shell.rs` 383~436행), 닫기 규칙(`close_auxiliary_window` 1574행: overlay 해제, toast 층 정리, mirror flush, 탭 복귀), 창별 키맵·LSP·reveal/rename, 보조 창 toast 층(shell-98), 탭 drag 중 모든 창 web view 숨김(`is_any_window_tab_dragging` 1415행), 보조 창 Emmet, 보조 창 Task Runner(terminal-agent-76), 탐색기 mount별 상태와 보조 창 Search 보기(explorer-search-30), 보조 창 Git 보기(`GITLANE` git-1, `GITLANE/APP/domains/git.rs` 1741~1760행 `Intent::ShowAuxiliaryView`).
- 빠진 것 4건(코드로 재확인)
    1. 보조 창 HTML·미디어 미리보기 WKWebView: `show_window`가 ROOT일 때만 `frame_pass.web_placements = web_placements`를 기록합니다(`APP/application.rs` 2047~2050행). `finish_frame`의 `show_preview_web_views(context, frame, web_placements, is_enabled)`(2295행, `APP/domains/preview.rs` 626~676행)는 메인 `eframe::Frame`에만 `Views::sync`를 합니다. `preview_web_view::Views::sync(frame: &eframe::Frame, ..)`(`APP/preview_web_view.rs` 81행)는 `wry::WebViewBuilder::build_as_child(frame)`(179행)로 메인 창에 붙입니다. 그래서 보조 창 HTML 탭에는 web view가 없습니다.
    2. 보조 창 터미널 raw input(마우스 보고): 앱의 `raw_input_hook`·`raw_input_hook_with_replay`(`APP/application.rs` 1333~1354행)는 ROOT·deferred viewport 입력 준비에서만 불립니다(`EFRAME/src/native/epi_integration.rs` 389행). immediate viewport 경로 `render_immediate_viewport`(`EFRAME/src/native/wgpu_integration.rs` 1139~1265행)는 `take_egui_input`(1188행) 뒤 곧바로 `egui_ctx.run_ui`(1209행)를 부르고 앱 hook을 부르지 않습니다.
    3. 보조 창 theme preview: `sync_theme_preview`(`APP/domains/settings.rs` 1218행)는 `context.viewport_id()`의 preview만 찾습니다. 호출 지점 `background_tick`(1102행)과 `finish_window_frames`(1431행)의 viewport는 ROOT라 보조 창 viewport로 저장된 preview는 적용되지 않습니다.
    4. 파일 선택 dialog 부모 창: `rfd::FileDialog::new().set_parent(frame)`(`APP/domains/shell.rs` 782·801행, `APP/file-dialogs.rs` 15·34행)의 `frame`은 항상 메인 `eframe::Frame`이라 보조 창에서 열어도 메인 창에 붙습니다.

#### (d) 구현 방법

각 하위 항목은 독립입니다. 2·3은 vendored eframe을 고치며 결정 6 A로 허용돼 있습니다(shell 사양 837~839행, `ROOT/docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` 7행 "4.1절 결정은 추천안"). 고칠 때마다 `EFRAME/TAIDE-CHANGES.md`에 절을 추가합니다.

1. 보조 창 theme preview(먼저 합니다. vendored 변경 없음)
    1. 적용 시점: 보조 창 콜백 안이 아니라 `show_auxiliary_windows`의 `context.show_viewport_immediate(..)` 호출 직전에 교환하고 호출이 돌아온 직후 되돌립니다(`APP/domains/shell.rs` 1535행 앞뒤). 이유: 보조 viewport의 root `Ui`는 `run_ui` 시작 때 `ctx.global_style()`을 복사하므로(2.1절 3번) 콜백 안에서 visuals를 바꾸면 그 창 본문에 반영되지 않습니다. `show_viewport_immediate`는 동기 호출이라 교환 구간 동안 ROOT는 아무것도 그리지 않습니다.
    2. `APP/domains/settings.rs`의 비공개 `apply_presentation_appearances`(1258~1307행)를 교환 함수로 바꿉니다: `pub(crate) fn swap_presentation_appearances(&mut self, context: &egui::Context, appearances: Appearances) -> Appearances`. 지금 대입하는 각 필드(`self.shell.colors`, `self.editor.appearance`, `self.find_appearance`, `editor_*_colors` 16종, `banner_appearance`, `lsp_status_appearance`, `status_editor_appearance`, `terminal_agent_state.status_ide_appearance`, `shell_state.system_usage_appearance`, `status_chord_appearance`, `problems_appearance`, `shell_state.zen_hint_colors`, `shell_state.file_drop_colors`, `explorer_search_state.explorer_appearance`·`navigation_appearance`, `tooltip_appearance`, `terminal_agent_state.terminal_appearance`, `settings_state.settings_appearance`, `preview_state.{pdf,presentation,spreadsheet}_appearance`, `toast_theme`)를 `std::mem::replace`로 바꾸고 이전 값을 담은 `Appearances`를 돌려줍니다. visuals는 `context.global_style().visuals.clone()`을 먼저 읽어 이전 값으로 담고 `presentation::apply_visuals(context, &new.visuals)`(`UI/presentation.rs` 227행, `all_styles_mut`)를 부릅니다. `commands_state.keybindings.set_appearance`·`palette.set_appearance`처럼 setter만 있는 필드는 같은 모듈에 `swap_appearance(&mut self, new) -> old`를 추가합니다. `Appearances`(`APP/presentation-refresh.rs` 176행)는 `Clone`이 아니므로 복제 대신 교환으로 되돌립니다. 기존 호출부 두 곳(테마 재적재 응답 처리 1054행, `sync_theme_preview` 1244행)은 반환값을 버리는 형태로 바꿉니다.
    3. 터미널 palette는 교환하지 않습니다. `terminal_views.set_palette`(`APP/terminal_surface.rs` 2050행)는 `Arc<Mutex<Option<Appearance>>>`(1251행)를 바꾸는데, 이 값은 PTY 쪽 OSC 색 질의 응답(`effect_ports`의 `color` 클로저, 4445~4462행)과 원격 관찰(`remote_effects`, 2030행)이 다른 스레드에서 읽습니다. frame 중간에 바꾸면 메인 창 터미널의 색 질의가 보조 창 preview 색으로 답할 수 있습니다. 그리기 색은 교환되는 `terminal_appearance`가 정하므로 보조 창 터미널 표시에는 preview가 적용됩니다. 보조 창 터미널의 OSC 색 질의가 preview가 아닌 메인 테마로 답하는 차이는 QA에 적습니다.
    4. preview 캐시: `settings_state`에 `auxiliary_previews: HashMap<egui::ViewportId, (ResolvedTheme, Option<Appearances>)>`를 둡니다. 보조 창 루프에서 `self.settings_state.settings_views.preview(window.viewport, &self.services.state)`가 `Some(theme)`이면, 캐시의 테마가 같을 때 저장된 `Appearances`를 꺼내고, 다르면 `crate::presentation_refresh::Appearances::new(&theme, &settings)`(229행)로 새로 만듭니다. 매 frame 생성하지 않습니다. 만들기 실패는 `log::warn` 1회(기존 `unreported_failure` 규칙)로 남기고 그 창은 메인 테마로 그립니다.
    5. 교환 순서: `let main = self.swap_presentation_appearances(context, preview);` → `show_viewport_immediate(..)` → `let preview = self.swap_presentation_appearances(context, main);` → 캐시에 `Some(preview)`를 다시 넣습니다.
    6. 메인 창 preview 처리: 지금 `sync_theme_preview`는 메인 preview를 앱 전체 필드에 적용하므로 메인 preview가 켜진 동안 보조 창도 메인 preview 색으로 그려집니다. 원본 보조 창은 자기 preview가 없으면 확정 테마(`resolvedTheme`)로 그려집니다. 그래서 `settings_state`에 확정 테마의 `Appearances` 캐시 `resolved_appearances: Option<Appearances>`를 따로 두고, "메인 preview가 켜져 있고(`settings_state.preview_theme.is_some()`) 그 보조 창에 preview가 없음"이면 이 값으로 교환합니다. 교환 함수와 순서는 5번과 같습니다.
    7. 캐시 무효화: 확정 테마가 바뀌는 곳(테마 재적재 응답 1052행 `self.settings_state.resolved_theme = theme`)과 설정 값이 바뀌어 `Appearances`가 달라지는 곳에서 `auxiliary_previews`와 `resolved_appearances`를 비웁니다. `close_auxiliary_window`에서 그 viewport의 캐시를 지웁니다. 종료 중(`closing.is_some()` 또는 `services.state.is_shutting_down()`)이면 교환하지 않습니다(`sync_theme_preview` 1219행과 같은 조건).
    8. 구문 강조 색: native 구문 토큰 테마는 `editor_syntax` 하나뿐이라 보조 창 preview가 그 창 구문 색까지 바꾸지는 못합니다. 1차 범위는 "셸·편집기 표면 색까지"이고, 구문 색 창별 분리(`EditorSyntax` 테마별 토큰 cache를 viewport별로)는 메인 결정 항목으로 QA에 남깁니다(windows-layers QA 105행이 같은 한계를 적었습니다).
2. 보조 창 WKWebView 배치
    1. vendored eframe: `EFRAME/src/native/wgpu_integration.rs` `render_immediate_viewport`의 `let input = { .. }` 블록(1152~1196행) 안의 `window`(`Arc<winit::window::Window>`)를 `Arc::clone`해 블록 밖으로 꺼냅니다. `egui_ctx.run_ui(..)` 호출(1209행) 동안 thread-local `IMMEDIATE_WINDOW: RefCell<Option<Arc<Window>>>`에 넣고, 들어가기 전 값을 저장해 끝나면 복원합니다(1203행 주석대로 재진입 가능). `EFRAME/src/lib.rs`에 `#[cfg(not(target_arch = "wasm32"))] pub fn immediate_viewport_window() -> Option<std::sync::Arc<winit::window::Window>>`를 둡니다. 앱은 eframe `wgpu` 기능만 켭니다(`ROOT/native/taide-native-app/Cargo.toml` 56행 `features = ["accesskit", "default_fonts", "wgpu", "preserve_empty_paste_shortcuts"]`). glow 경로(`EFRAME/src/native/glow_integration.rs`)는 고치지 않고, 이 함수가 wgpu 백엔드에서만 `Some`을 돌려준다는 사실을 공개 문서와 `TAIDE-CHANGES.md`에 적습니다.
    2. `APP/application.rs` `WindowPass`(1373행)에 `auxiliary_web_placements: HashMap<egui::ViewportId, Vec<preview_web_view::Placement>>`를 추가하고, `show_window`의 ROOT가 아닌 분기(2051행 `else`)에서 그 viewport 키로 지역 `web_placements`를 넣습니다.
    3. `APP/domains/preview.rs`의 `preview_state.web_views`(63행)를 viewport별로 나눕니다: 기존 `web_views`(메인) + `auxiliary_web_views: HashMap<egui::ViewportId, preview_web_view::Views>`. `Views::sync`(`APP/preview_web_view.rs` 81행)의 첫 인자를 `frame: &eframe::Frame`에서 `parent: &impl wry::raw_window_handle::HasWindowHandle`로 일반화합니다. `wry::WebViewBuilder::build_as_child<W: HasWindowHandle>`(wry 0.55.1 `src/lib.rs` 1491행)가 받는 trait와 같습니다. winit `Window`는 이 trait를 구현합니다(vendored eframe이 `window.window_handle()`를 부름, `EFRAME/src/native/epi_integration.rs` 204행). 메인 호출은 `frame`을 그대로 넘깁니다.
    4. 보조 창 콜백 끝(`show_auxiliary_toasts` 뒤)에서 `eframe::immediate_viewport_window()`가 `Some(window)`이면 `show_auxiliary_preview_web_views(context, viewport, &*window, placements, enabled)`를 부릅니다(새 `pub(crate)` 메서드, `APP/domains/preview.rs`). `enabled`는 메인 `show_preview_web_views`의 `web_enabled`(652~662행)와 같은 항목을 그 창 기준으로 계산합니다: 그 창 pass의 `is_enabled`(show_window 반환 전 계산값을 `WindowPass`에 viewport별로 저장), `!commands_state.keybindings.is_open()`, `!commands_state.palette.is_open()`, `pending_tab_close.is_none()`, `explorer_search_state.pending_entry_delete.is_none()`, `!egui::Popup::is_any_open(context)`(콜백 안 context라 그 창 pass), `!egui::DragAndDrop::has_any_payload(context)`, `!taide_native_ui::project_drag::is_dragging(context)`, `!self.is_any_window_tab_dragging(context)`. 메인 전용 항목(`file_drop::is_hovering`, `web_views.drags().is_hovering()`)은 원본 보조 창이 OS drop을 받지 않으므로 넣지 않습니다.
    5. 보조 창 web view의 drag 처리기는 항상 `false`(OS 기본 처리)입니다. 원본 보조 창은 OS 파일 drop을 구독하지 않습니다(shell 사양 706행). `Views`에 "drag 받지 않음" 생성 옵션을 두고 `with_drag_drop_handler(|_| false)`로 만듭니다. 기본 처리로 web view가 drop 파일로 이동하려 해도 기존 `with_navigation_handler(allowed_navigation)`(`APP/preview_web_view.rs` 169행)가 막습니다: `allowed_navigation`(28~38행)은 scheme·host·port·path·query가 원래 source와 모두 같은 URL(조각 이동)만 허용하므로 다른 파일 경로로의 이동은 거부됩니다. 이 근거를 QA에 적고, 단위 검사로 `allowed_navigation(&source, "file:///other.html")`이 거짓임을 단언합니다.
    6. `close_auxiliary_window`에서 그 viewport의 `Views`를 `clear()`(69행) 뒤 제거합니다(WKWebView 해제). `sync` 오류는 메인처럼 `renderer_failed(path)`와 `status`로 처리합니다(`APP/domains/preview.rs` 663~675행과 같은 형태).
3. 보조 창 터미널 raw input
    1. vendored eframe: `EFRAME/src/lib.rs`(비 wasm)에 `pub fn set_immediate_raw_input_hook(hook: Option<Box<dyn FnMut(&egui::Context, &mut egui::RawInput)>>)`를 두고 thread-local에 보관합니다. `render_immediate_viewport`에서 `take_egui_input`과 `input.viewports`·`input.time` 설정이 끝나고 `shared` 차용이 풀린 직후(1196행 블록 뒤, 1198행 `let egui_ctx = ..` 다음) hook이 있으면 `hook(&egui_ctx, &mut input)`을 부릅니다. immediate 경로는 pending 입력 재처리를 하지 않으므로 replay 인자는 0입니다. 이 주장은 구현 때 `EFRAME/src/native/epi_integration.rs` 380~395행과 `render_immediate_viewport` 전체를 다시 읽어 확인하고 QA에 적습니다(shell 사양 711행이 이 확인을 요구).
    2. 앱: hook은 `&mut NativeApplication` 없이 돌아야 합니다. `APP/terminal_surface.rs` `Views`의 raw input이 쓰는 상태(`views`의 view별 `mouse` 필드, `raw_frame`, `capture_mouse`가 쓰는 패킷 대기열)를 `Rc<RefCell<RawInputState>>`로 떼어 `Views`와 hook이 같이 들게 합니다. 메인 `raw_input_hook`은 같은 함수를 부르게 바꿉니다.
    3. 주의: `raw_input_with_replay`(`APP/terminal_surface.rs` 2327행)의 첫 조건 `context.viewport_id() != viewport`(2334행)는 hook 시점에 항상 참이 되어 바로 돌아갑니다. hook 시점의 `context.viewport_id()`는 부모(ROOT)이기 때문입니다(2.1절 4번). immediate 경로용 진입 함수는 `input.viewport_id`만 믿고 이 조건을 빼야 합니다. 이어지는 `context.input_for(viewport, ..)`(2337행)와 `context.cumulative_frame_nr_for(viewport)`(2338행)는 viewport를 명시하므로 그대로 씁니다.
    4. 등록: `NativeApplication` 생성 뒤(`with_relay` 옆에 `with_immediate_input_hook`) `eframe::set_immediate_raw_input_hook(Some(Box::new(move |context, input| state.borrow_mut().raw_input_immediate(context, input))))`. `app_menu.inject`(보조 창)는 지금처럼 viewport 콜백 안에 둡니다(1538~1542행).
    5. `on_exit`에서 hook을 `None`으로 지웁니다.
    6. `terminal_surface.rs` 소유는 terminal-agent 영역입니다. 그 영역 검사(`terminal_surface`, `application::terminal_agent_tests`)를 함께 실행합니다.
4. 파일 선택 dialog 부모 창(P20 허락 뒤)
    1. 저장소 밖 `tauri-plugin-dialog` 소스에서 부모 창 결정 코드를 확인합니다(registry 소스 읽기 예외에 들어가는지 사용자 확인).
    2. 원본이 호출 창을 부모로 쓰면: 보조 창에서 연 dialog에 그 창 핸들을 넘깁니다. rfd `set_parent<W: HasWindowHandle + HasDisplayHandle + ?Sized>`(registry `rfd-0.16.0/src/file_dialog.rs` 96행. 앱 lock은 0.17.2라 구현 때 컴파일로 같은 경계인지 확인)를 쓰므로 `apply_shell_intent`의 `OpenFolder`·`OpenFile`(782·801행)과 `file_dialogs::{save, open}`(`APP/file-dialogs.rs` 13·32행)의 `frame` 인자를 `&(impl HasWindowHandle + HasDisplayHandle)`로 일반화합니다.
    3. intent는 보조 창 콜백 밖에서 처리되므로 intent에 출처 viewport를 싣고, 처리 때 창 핸들을 얻을 수 없으면 메인 frame으로 대체합니다(대체를 QA에 기록). 보조 창 handle은 콜백 안에서만 `immediate_viewport_window()`로 얻을 수 있으므로, dialog 호출을 그 창 콜백 끝으로 옮기는 방식(대기 dialog 요청을 viewport별로 보관)도 검토합니다.

#### (e) 자동 검사

- 1(theme preview, 실제 App, `APP/application-auxiliary-window-tests.rs`): `install_viewports`로 보조 창 출력을 받고, 보조 창 설정 탭에서 테마 preview를 켬 → 같은 frame의 보조 viewport 출력에 preview 테마 `app.background` 색 사각형, ROOT 출력에는 확정 테마 색 → preview를 끄면 보조 창도 확정 색. 단언 helper는 `APP/application-explorer-views-tests.rs` 588행 `shell_background_painted(output, color)`인데 지금 비공개 `fn`이라 `pub(super)`로 올려 재사용합니다. 추가로 frame 뒤 `application.shell.colors`가 메인 값인지(교환 누락 검출), 보조 창 출력의 text 색이 preview visuals인지(root `Ui` 스타일 함정 검출) 단언합니다.
- 2(WKWebView, 실제 App): 보조 창에 HTML 파일 탭을 연 상태로 frame 진행 → `WindowPass.auxiliary_web_placements[viewport]`에 그 탭 placement가 있고 메인 placement에는 없음. web view 생성은 GUI가 필요해 검사하지 않습니다(kittest `Frame`은 창 핸들이 없음). 단위: drag 거부 옵션으로 만든 처리기가 파일 Enter·Drop 모두 `false`(기존 `preview_web_view` 단위 검사 형식).
- 3(raw input, 앱 lib 단위): 공유 `RawInputState`에 보조 viewport id의 합성 `RawInput`(마우스 누름·이동)을 넣고, 그 viewport 소유 터미널 view가 마우스 보고 패킷을 만들며 해당 event를 걸러내는지 단언. 핵심 단언은 "context의 현재 viewport가 ROOT인 상태(hook 시점과 같음)로 불러도 동작"입니다. vendored 호출 경로는 실기입니다.
- 4: P20 뒤 결정에 따라 인자 전달 단위 검사만 가능합니다. 실제 dialog는 GUI라 실기입니다.
- 명령: 앱 `-- auxiliary_window_tests preview_web_view terminal_surface settings --test-threads=4`, vendored 변경 뒤 remote-web Wasm check 두 번, 바꾼 crate fmt.

#### (f) 완료 판정

- 1·2·3 구현과 위 검사 통과, Git 병합 뒤 보조 창 Git 보기 검사(`GITLANE/APP/application-git-base-tests.rs` 327행 `실제_app_보조_창의_git_키는_그_창의_탐색기를_열고_git_보기로_바꾼다`)가 병합 트리에서 통과 → complete. 비고에 "실기(배치 31): 보조 창 WKWebView 표시·터미널 마우스 보고 실제 입력·OS 창 focus", "dialog 부모 창은 P20 확인 전 메인 창 부모 유지(차이)", "보조 창 구문 강조 색은 메인 테마(차이, 결정 대기)", "보조 창 터미널 OSC 색 질의는 메인 테마로 응답(차이)"를 적습니다.
- 4는 원본 근거를 확인할 수 없으므로 complete를 막지 않고 차이로 기록합니다(shell 사양 848행 "확인 전에는 지금(메인 창 부모)을 유지"). 메인이 4를 완료 조건으로 지정하면 따릅니다.

#### (g) 막는 요소

- P20(저장소 밖 crate 소스 확인)은 하위 항목 4만 막습니다.
- 구문 색 창별 분리 범위는 메인 결정입니다((d) 1.8).
- 실기는 배치 31(P23).

---

### 3.3 shell-12 전역 ErrorBoundary (배치 31 설계 항목)

#### (a) ID·기능·상태

- shell-12 "전역 ErrorBoundary(위젯별 fallback)", 기능표 105행, partial. 같은 기능을 commands-ui-82(기능표 721행, missing으로 낡은 비고)가 가리킵니다. 두 행은 함께 판정합니다(`ROOT/docs/research/2026-10-28-lane-commands-ui-spec.md` 520~526행).
- 남은 범위(편집기 영역·루트 경계)는 배치 31 "편집기 장애 경계 설계" 항목입니다(`ROOT/docs/memory/2026-10-28-native-lane-operations.md` 3절 55행 wave 4, `ROOT/docs/PROCESS.md` W4 줄, 형제 문서 `20-gates-31-33.md` 31-A2절).

#### (b) 원본 TS 동작

- 컴포넌트 `TS/shared/ui/error-boundary.tsx` 32~126행: 자식의 render 오류를 잡아 그 영역만 fallback으로 바꿉니다. fallback은 `role='alert'`, `tabIndex=-1`, 영역 이름(`labelKey`), `errorBoundary.description`, 개발 빌드에서만 stack(`import.meta.env.DEV`), `common.retry` 버튼. 전환 때 fallback에 focus. 재시도는 state 초기화로 새 mount.
- 경계 8곳
    - 루트 `TS/main.tsx` 45행 `<ErrorBoundary labelKey='errorBoundary.app' labelFallback='Application' onCaught={revealWindowOnRootCrash}>`. `revealWindowOnRootCrash`(37~41행)가 `data-theme-ready`·`data-locale-ready`를 세우고 `getCurrentWindow().show()`.
    - `TS/widgets/app-shell/app-shell.tsx` 185(Welcome), 195(Activity Bar), 227(상태 표시줄).
    - `TS/widgets/app-shell/project-shell.tsx` 150(사이드바 패널), 156~158(편집기 `errorBoundary.editorArea`가 `EditorArea`를 감쌈. `EditorArea`는 pane tree·탭 바·탭 내용·문제 패널 포함).
    - `TS/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 132(사이드바 패널), 138(편집기).
- 경계 상태의 수명: `ProjectShell`은 `ShellSlotScope` 안에 key 없이 그려지고(`TS/widgets/app-shell/shell-slot-tree-view.tsx` 91~98행), 그 바깥 `Panel`의 React key는 slot 트리 경로 `nodeKeyOf([...path, index])`(22행 `['shell-slot', ...path].join('-')`, 141행)입니다. 그래서 같은 위치에서 프로젝트만 바뀌면 fallback이 유지되고, slot 트리 모양이 바뀌어 경로가 바뀌면 경계가 새로 mount됩니다.
- locale 키(`ROOT/crates/taide-locale/resources/locales/en.json`): `errorBoundary.app`(227행 "Application"), `errorBoundary.description`(228행), `errorBoundary.editorArea`(229행 "Editor"), `errorBoundary.sidebar`(230행), `errorBoundary.sidebarPanel`(231행), `errorBoundary.statusBar`(232행), `errorBoundary.welcome`(233행), `common.retry`(56행).

#### (c) native 현재 자산과 빠진 것

- 구현됨(근거 `ROOT/docs/quality-assurance/2026-10-27-native-error-boundary-foundation.md`, `-panels.md`)
    - 공용 경계 `UI/error-boundary.rs`: `Region { Welcome, ActivityBar, StatusBar, SidebarPanel }`(42~47행), `label_key`(50~57행), `is_row`(StatusBar만), `is_unmounted_while_hidden`(SidebarPanel만 거짓, 63~65행). `show_instance`(107~168행): 상태 id `(STATE_KEY, region, instance, viewport)`(115행)의 egui temp data, 직전 frame에 그려지지 않았으면 generation을 올려 새 mount(121~130행), `catch_unwind`(`capture` 170행), 실패면 `request_discard`·`Outcome::Caught`, 이후 `Outcome::Fallback`, 재시도 버튼이 generation을 올려 새 `id_salt`로 다시 그림. 검사용 `inject_panic`·`inject_instance_panic`(356·361행, `cfg(test)` 또는 `inspection`).
    - 적용: `UI/shell.rs` 479(Welcome)·508(Activity Bar)·534(상태 표시줄)행 `show`, 592행 `show_instance`(사이드바 패널, 메인 slot `Id::new(slot)`과 보조 창 `Id::new((project, slot))`, 586~589행). 사이드바 패널은 `ExplorerSearchSurfaces`의 `begin_explorer`/`explorer`(`&self` 읽기 전용 render)/`finish_explorer`/`discard_explorer`(`UI/surfaces/explorer_search.rs` 7~40행) 트랜잭션으로 바꾼 뒤 경계를 둡니다.
    - 검사: UI workbench 8건(5251~5708행 구간), 실제 App 4건(`APP/application-error-boundary-tests.rs`).
- 빠진 것
    1. 편집기 영역 경계(메인 slot과 보조 창). 이유: 편집기 render `NativeEditor::show_request*`(`UI/editor_surface.rs` 1280·1295·1326·1357·1392·1428행 진입 함수, 핵심 `show_request_inner` 1464행)가 입력 처리 중 공유 `EditorStore`를 직접 바꿉니다. `grep -o -E "store\.(set_composition|set_view_state|take_display|set_display|break_undo_group|set_wrap_affinities|undo|redo|take_selection_reveal|request_selection_reveal)\(" UI/editor_surface.rs | sort | uniq -c` 결과: `set_composition` 9, `set_view_state` 6, `take_display` 3, `set_display` 3, `break_undo_group` 3, `set_wrap_affinities` 2, `undo`·`redo`·`take_selection_reveal`·`request_selection_reveal` 각 1. 편집 적용은 `taide_native_editor::editing` 함수를 거치므로 이 집계는 하한입니다. 키보드 event 루프는 1888행(`std::mem::take(&mut input.events)`)~2191행이고 event마다 `body_input`(4055행) → `input`(4206행)이 `&mut EditorStore`를 씁니다. 앱 `AppSurfaces`도 `&'a mut` 필드 57개를 가집니다(아래 (d) 2.7).
    2. 루트 `errorBoundary.app` 경계: 없음. 경계 밖 panic은 winit `stop_app_on_panic`으로 앱 종료, 다음 실행에서 hot-exit·세션 복원(shell-25)으로 복구합니다.
    3. fallback 픽셀 대조(배치 31 실기).

#### (d) 구현 방법(설계 포함)

결정 A(`ROOT/docs/acknowledge/2026-10-26-native-autonomous-continuation.md` 32행): "영역 render를 'snapshot 읽기 + 영역 소유 상태 + intent 출력'으로 바꾼 영역에만 `catch_unwind` 경계 … 편집기·사이드바 패널은 공유 상태 변경을 render 밖 트랜잭션으로 옮긴 뒤 적용". 아래 2절은 어느 안을 고르든 같은 공통 부분이고, 3절이 31-A2의 안별 절차입니다.

1. 착수 전 결정(메인 → 사용자): `20-gates-31-33.md` 31-A2 "설계(결정 필요)" 표의 A(입력 선처리, 그 문서의 추천)/B(전부 지연)/C(체크포인트 복원). 그 문서가 "큰 구조 변경이라 권장안 자동 채택의 예외"로 정했으므로 묻고 결정을 `ROOT/docs/acknowledge/`에 기록한 뒤 시작합니다. 아래 3.3은 C의 "복원 누락" 위험을 타입으로 막는 변형(C′)을 비교 근거로 덧붙입니다.
2. 공통 부분
    1. 영역 정의(`UI/error-boundary.rs`): `Region::EditorArea`(`label_key` = `"errorBoundary.editorArea"`)와 `Region::App`(`"errorBoundary.app"`)을 추가합니다. `is_unmounted_while_hidden`: EditorArea는 참(slot·보조 창이 사라진 frame만 그려지지 않으므로 원본 조건부 mount와 같음), App은 참.
    2. 메인 slot 경계 위치: `UI/shell.rs` `slot_tree`에서 사이드바를 그린 뒤 남은 `rect`(1454행 `let problems_id = ..`부터 1489행 `self.pane_tree(..)` 호출과 1511~1519행 `problems_resize`까지)를 하나의 경계 rect로 감쌉니다. 원본 `EditorArea`가 문제 패널을 포함하므로 문제 패널(`surfaces.problems_panel`, 1458행)도 경계 안입니다.
    3. 보조 창 경계 위치: `UI/shell.rs` 보조 창 분기의 `self.pane_tree(..)`(420~434행)를 감쌉니다.
    4. instance 규칙: 사이드바 패널과 같게 메인 `egui::Id::new(slot)`, 보조 창 `egui::Id::new((project, slot))`. 원본은 slot 트리 경로 key라 트리 모양이 바뀌면 새 mount되지만 native는 slot id라 유지됩니다. 사이드바 패널과 같은 차이이므로 QA에 함께 적습니다.
    5. 빌림 함정: `show_instance`의 `text: &dyn Fn(&str) -> String` 인자는 경계 안 `contents`와 같은 시점에 살아 있습니다. 사이드바는 `let reader = &*surfaces;`로 둘 다 불변 차용했지만(`UI/shell.rs` 591행), 편집기 `tab_content`는 `&mut self`(`UI/surfaces/editor.rs` 10행)라 같은 방식이 컴파일되지 않습니다. 경계 진입 전에 fallback이 쓰는 세 문구(`region.label_key()`, `"errorBoundary.description"`, `"common.retry"`, `UI/error-boundary.rs` 238·239·299행)를 `surfaces.text(..)`로 미리 만들어 지역 배열에 두고, `text`에는 그 배열을 찾는 클로저를 넘깁니다.
    6. intent: 경계 안에서 나온 intent는 영역 지역 목록에 모았다가 `Outcome::Shown`일 때만 셸 intent에 더합니다(사이드바 선례 `UI/shell.rs` 604~611행). 주의: `slot_tree` 끝의 slot focus 판정이 `!intents[start..].is_empty()`(1527행)를 씁니다. panic frame에는 편집기 intent가 없으므로 포인터 누름(`pointer_in_body`)만으로 focus됩니다. 원본도 fallback이 클릭을 받지 않으므로 같습니다.
    7. 앱 쪽 공유 상태 분류: `APP/application.rs` `AppSurfaces`의 `&'a mut` 필드는 57개입니다(`awk 'NR>=2524 && NR<=2660 && /&.a mut/' APP/application.rs`, 2026-11-04 집계, 괄호는 줄 번호). 아래는 선언 위치와 타입 이름으로 나눈 초안입니다. 편집기 영역 경계 안(`pane_tree`·`problems_panel`)에서 실제로 쓰는 필드만 처리 대상이므로 8번 절차로 확정합니다. 병합 뒤 `GITLANE` 필드를 다시 집계합니다(예: `title_branches`는 병합 작업 트리에서 사라졌고 `git: &domains::git::State`는 불변).
        - (가) pass 출력(show_window 1700~1721행 지역 선언, `WindowPass`, 또는 self의 출력 대기열): `command_errors`(2555), `settings_output`(2559), `keymap_actions`(2564), `keymap_editor_scope`(2565), `keymap_documents`(2567), `banner_actions`(2611), `changed_documents`(2612, `editor_menu_changes`에서 take), `preview_output`(2616), `web_placements`(2625), `load_untitled`(2629), `save_missing_drafts`(2631), `explorer_output`(2639), `commands`(2646), `load_files`(2647), `focused`(2649, `new_focus`), `status`(2651, `self.status.take()`), `document_edits`·`fold_commands`(2553·2554, `frame_pass`), `problem_opens`(2528, `&mut self.problem_opens` 1828행, `domains/editor.rs`가 소비). panic이면 경계 진입 직전 길이로 잘라내거나(Vec) 진입 전 값으로 되돌립니다(Option·Map).
        - (나) 영역 소유 view 상태(원본에서는 `EditorArea` 안 React·Monaco 위젯 상태라 경계가 잡으면 사라짐): `editor_find`(2573), `editor_problems`(2574), `editor_locations`(2575), `editor_rename`(2576), `editor_code_actions`(2577), `editor_documentation`(2578), `editor_completion`(2579), `editor_highlights`(2580), `editor_inlay_hints`(2581), `editor_code_lenses`(2582), `editor_selection_ranges`(2583), `editor_ide_selection`(2584), `editor_view_state`(2585), `editor_markdown`(2586), `editor_keymap_targets`(2566), `problems`(2531), `breadcrumbs`(2538), `app_file_views`(2558), `settings_views`(2560), `terminal_views`(2563)의 view별 UI 상태. panic이면 그 영역에 보이던 view·tab 항목만 제거(새 mount)합니다. 각 타입에 `forget_view(view)`/`forget_tab(tab)`이 있는지 확인하고 없으면 추가합니다.
        - (다) 공유 상태: `store: EditorStore`(2571, 모든 창·pane이 공유하는 문서·undo·view), `reveals`(2650), `terminal_views`의 입력 대기열(PTY로 갈 입력), `open_with`(2614, 사용자 선택 override), `find_history`(2589), `explorers`·`explorer_clipboards`(2637·2638, 탐색기 소유지만 편집기 영역 안 reveal이 쓸 수 있음). 경계를 막는 핵심입니다.
        - (라) 다른 영역 소유 상태: `symbol_sidebars`(2537, slot별 사이드바 보기·검색 패널 상태 `APP/symbol-sidebar.rs` 67행, 사이드바 경계의 `begin_explorer`가 처리), `system_usage`(2545, 상태 표시줄), `status_ide_icons`(2550, 상태 표시줄). 편집기 영역 안 쓰기 지점이 있으면 (다)로 옮깁니다.
        - (마) cache(다음 frame이 같은 값을 다시 쓰므로 되돌리지 않아도 결과가 같은 것): `previews`(2615), `pdf_previews`(2617), `hwp_previews`(2618), `presentation_previews`(2620), `spreadsheet_previews`(2622), `editor_syntax`(2608) 토큰 cache, `explorer_icons`(2534), `navigation_icons`(2536), `system_usage_icon`(2547). 키가 경로·문서 revision·아이콘 이름인지 각 타입에서 확인해 QA 표에 근거를 적습니다. 키가 그렇지 않으면 (나)로 옮깁니다.
    8. 표 작성 방법: 각 필드마다 `grep -n "self\.<필드>" APP/*.rs APP/domains/*.rs`로 `tab_content`·`problems_panel`과 그 하위 함수 안의 쓰기 지점을 찾고, 분류·근거·panic 처리를 QA 문서 표로 남깁니다. 이 표가 리뷰 기준입니다.
3. 안별 절차
    - A안(31-A2 추천, 입력 선처리)
        1. 조사: `show_request_inner`(1464행~4054행)의 구간을 "입력 단계"와 "그리기 단계"로 나눕니다. 입력 단계 후보: 1888~2191행 키보드·IME·붙여넣기 event 루프(각 event → `body_input` 4055행 → `input` 4206행 → `editing`·`language_typing`의 `&mut EditorStore` 함수), 3945행 남은 event 재삽입. 그리기 단계: 나머지(측정·줄 그리기·장식·popup). 포인터 입력(클릭·드래그·hover)은 egui `Response`가 현재 pass의 위젯 rect 할당 뒤에만 나오므로 그리기 단계에서 intent로 기록하고 적용 단계에서 반영합니다.
        2. `show_request*`를 `prepare_input(&self, ui, store: &mut EditorStore, view, ..)`(경계 밖)와 `paint(&self, ui, store: &EditorStore, view, ..) -> EditorFrameOutput`(경계 안)과 `apply_frame_output(store: &mut EditorStore, output)`(경계 밖, `Outcome::Shown`일 때만)로 나눕니다. `EditorFrameOutput`은 view 상태 갱신, 표시 cache(`set_display`), IME 조합(`set_composition`), 포인터 intent, 메뉴·hover 요청을 담습니다.
        3. 입력 단계가 경계 밖에 있으므로 입력 단계 panic은 지금처럼 앱 종료입니다. 그리기 단계 panic만 fallback입니다.
        4. 검사로 확인할 위험: 포인터 결과가 한 frame 늦게 그려지는지(클릭 위치 커서), keypress→paint 지연(31-B4 성능 게이트 p95 16.7ms).
    - B안(전부 지연): A와 같지만 키보드 입력도 그리기 단계에서 action으로 기록하고 적용 단계에서 실행합니다. 키 입력이 한 frame 늦게 그려집니다.
    - C′안(체크포인트 복원의 타입 강제 변형): render 코드는 바꾸지 않고, 경계 진입 전 기록 시작 → panic이면 되돌림 → 성공이면 확정합니다.
        1. `EDITOR/store.rs`의 `DocumentStore.documents`(176~182행)와 `ViewStore.views`(226~231행)의 `HashMap`을 기록 기능이 있는 새 타입 `Journaled<K, V>`로 바꿉니다. 이 타입의 변경 API(`get_mut`, `insert`, `remove`, `values_mut`, `iter_mut`)는 기록 중이면 처음 건드리는 키의 이전 값(`Option<V>`, 없던 키는 `None`)을 복제해 둡니다. `values_mut`·`iter_mut`는 기록 중이면 전체 항목을 먼저 복제합니다. 맵 필드에 직접 접근할 방법이 없으므로 복원 누락이 컴파일 단계에서 막힙니다.
        2. 바꿔야 할 호출 지점(기준 `0c52b438`): `grep -n -E "\.get_mut\(&" EDITOR/store.rs` 38곳, `grep -n -E "values_mut\(" EDITOR/store.rs` 8곳(581·718·827·934·980·1119·1730·1796행), insert/remove 지점(192·831·837·846·849·852·853·908·913·914·1185·1206·1236·1257·1880·1881·1903·1927행, `by_key` 포함). `by_key`·`next_id`·`pending_disposals`도 `RollbackPoint`에 복사합니다(작은 맵이라 시작 때 통째로 복제).
        3. `Document`(88~105행)·`HistoryEntry`(62~69행)·`DocumentIndentation`(71~77행)과 그 필드 타입(`DocumentMetadata`, `SyntaxSnapshot`, `DiskSnapshot`, `ChangeJournal`)에 `Clone`이 없으면 추가합니다. `Rope`(ropey) 복제는 내부 공유라 싸고, `undo: VecDeque<HistoryEntry>`는 항목 수(상한 `EditorLimits.max_undo_groups`, 27행)에 비례합니다. 편집이 일어난 frame에서 문서당 한 번입니다. `ViewState`는 이미 `Clone`입니다(`EDITOR/view.rs` 258~259행).
        4. 공개 API: `pub fn begin_rollback_point(&mut self)`, `pub fn commit_rollback_point(&mut self)`, `pub fn rollback(&mut self)`. 중첩 호출은 `debug_assert!`가 아니라 `Result` 오류(`EditorError`)로 거부합니다.
        5. 원본과 다른 점: 원본 Monaco model은 오류 전에 적용된 편집을 유지하지만 C′는 panic pass의 편집을 되돌립니다. 반쯤 적용된 편집이 디스크에 저장될 위험을 없애기 위한 의도된 차이로 QA·기능표에 적습니다.
        6. 성능: 대형 문서(10만 줄) 입력 frame의 기록 비용을 31-B4에서 측정합니다.
4. 앱 연결(어느 안이든 같은 짝): `UI/surfaces/editor.rs` `EditorSurfaces`에 사이드바와 같은 짝을 추가합니다.

```rust
type EditorRegion;
fn begin_editor_region(&mut self, project: &ProjectId, mount: EditorMount<'_>) -> Self::EditorRegion;
fn finish_editor_region(&mut self, region: Self::EditorRegion);
fn discard_editor_region(&mut self, region: Self::EditorRegion);
```

- `EditorMount`는 `ExplorerMount`(`UI/shell.rs`)와 같은 모양(메인 `Slot(&ShellSlotId)`, 보조 창 `Auxiliary(u32)`)입니다. 앱 구현(`APP/domains/editor.rs`의 `impl EditorSurfaces for AppSurfaces`):
    1. `begin`: (가) 지역 출력의 현재 길이·값, (다) 공유 상태의 기록 시작(C′면 `store.begin_rollback_point()`, A·B면 입력 단계 실행), 그 영역 mount에 보이는 view·tab 목록을 `EditorRegion`에 담습니다.
    2. 경계 안: A·B는 그리기 단계, C′는 지금처럼 `pane_tree`·문제 패널.
    3. `Outcome::Shown`/`Fallback`: `finish` → 확정(C′ `commit_rollback_point`, A·B `apply_frame_output`).
    4. `Outcome::Caught`: `discard` → 공유 상태 되돌림(C′) 또는 출력 폐기(A·B), (가) 잘라내기, (나) 그 mount의 view·tab 항목 제거, 그 영역의 WKWebView placement 폐기(다음 frame에 숨김).
    5. UI 검사 `Surfaces`(`tests/workbench.rs` 152행)에도 같은 메서드를 구현합니다(빈 구현 + 호출 기록).
5. 루트 경계(편집기 영역 다음)
    1. 원본 루트는 앱 전체를 감쌉니다. native에서 셸 밖 앱 코드(`show_window`의 오버레이·대화상자·toast, `background_tick`)는 render 중 `self.submit`으로 즉시 host 명령을 보내고 루트 상태를 바로 바꾸므로 감쌀 수 없습니다.
    2. 추천: 루트 경계 = `NativeShell::show` 전체(`APP/application.rs` 1971~1978행의 `self.shell.show(ui, &snapshot, &mut surfaces)`)를 `error_boundary::show_instance(ui, ui.max_rect(), (Region::App, egui::Id::NULL), ..)`로 감쌉니다. 편집기 영역 경계 뒤 `NativeShell::show` 안에서 경계 밖에 남는 코드는 제목줄·rail 틀·slot 헤더·분할 경계·drag 피드백(`project_drag_feedback`, `tab_drag_feedback`)입니다. 이 부분이 쓰는 `ShellSurfaces` 메서드(`tooltip`, `rail_icon`, `tab_icon`, `title_branch_icon`, `prepare_status_bar` 등)를 `grep -n "surfaces\.[a-z_]*(" UI/shell.rs`로 전수 조사해 사이드바처럼 지연 요청으로 바꾼 뒤 감쌉니다.
    3. panic이면 (가) 지역 출력과 셸 intent를 버리고 창 전체 fallback을 그립니다. 재시도는 generation을 올려 새로 그립니다. 원본 `onCaught`(창 표시)는 메인 창이 이미 보이므로 할 일이 없고, 보조 창은 `ViewportCommand::Visible(true)`를 그 viewport에 보냅니다.
    4. commands-ui 결정 1 A(`window.reload` = 실행한 창의 UI만 다시 적재, `ROOT/docs/research/2026-10-28-lane-commands-ui-spec.md` 4.1절 1번)와 루트 재시도의 의미를 맞출지 31-A2가 함께 결정합니다.
    5. 셸 밖(오버레이·`background_tick`) panic은 앱 종료로 남고 차이로 기록합니다.
6. 검사 주입: `inject_instance_panic(ctx, Region::EditorArea, instance)`은 영역 `contents`가 끝까지 실행된 뒤 panic합니다(`UI/error-boundary.rs` 141~144행, foundation QA 55행). 편집 되돌림을 검사하려면 "편집이 적용된 뒤 panic"이어야 하므로 이 주입이 그대로 맞습니다.

#### (e) 자동 검사

- UI workbench(`--test workbench`)
    - `편집기_영역_panic은_그_slot만_fallback으로_바꾸고_사이드바와_다른_slot을_유지하며_재시도로_새로_그린다`: `fixture()`의 두 slot, 첫 slot에 `inject_instance_panic(&context, Region::EditorArea, egui::Id::new(&slot))` → fallback 문구 `errorBoundary.editorArea`·`alert_node` focus·재시도, 같은 slot 사이드바와 다른 slot 편집기는 그대로, panic frame에 편집기 intent 없음(foundation QA 127행의 discard 대조군 방식).
    - 보조 창 편집기 panic → 보조 창 편집기만 fallback, 보조 창 탐색기 유지.
    - 루트: `Region::App` 주입 → 창 전체 fallback, 재시도.
- 편집기 core(C′일 때): `EDITOR`에 `rollback`이 기록 중의 `apply`·`undo`·`set_view_state`·`set_composition`·view attach·문서 open을 모두 되돌리고 `commit` 뒤에는 유지하는 검사. 명령 `cargo test --manifest-path ROOT/native/taide-native-editor/Cargo.toml --locked --offline --target-dir <T> -- rollback`.
- 실제 App(`APP/application-error-boundary-tests.rs`)
    - 파일 탭을 열고 글자 입력 event와 `inject_instance_panic(Region::EditorArea, ..)`을 같은 frame에 넣음 → fallback, 문서 revision·본문·dirty·undo 가능 여부가 입력 전과 같음(C′) 또는 입력이 적용됨(A, 입력 단계가 경계 밖), 그 frame에 저장·mirror host 명령이 경계 안에서 나가지 않음(`commands` 잘라내기), 다른 slot 편집기 입력은 정상, hot-exit mirror가 panic 뒤에도 정상 저장됨.
    - 재시도 뒤 같은 입력이 적용됨(대조군).
    - 보조 창 편집기 panic(`install_viewports` 사용).
- 성능: 31-B4 keypress→paint 측정.

#### (f) 완료 판정

- 편집기 영역(메인·보조 창)과 루트 경계가 위 검사로 통과하고, 의도된 차이(안별 panic pass 처리, 셸 밖 panic 종료, instance 규칙)가 QA·기능표 비고에 적히면 complete. 픽셀 대조는 배치 31 실기 항목으로 비고에 남깁니다. commands-ui-82를 같은 근거로 갱신합니다(`16-remaining-commands-ui.md`의 commands-ui-82 절).

#### (g) 막는 요소

- 배치 31 설계 항목입니다. A/B/C(C′) 선택은 메인이 사용자에게 묻는 결정입니다(31-A2).
- `native/taide-native-editor`는 동결 remote-web이 의존하는 crate입니다. store 공개 API를 추가하면 remote-web Wasm check로 컴파일만 확인합니다. remote-web 소스·lock은 바꾸지 않습니다.
- 여러 영역 파일(편집기·터미널·설정·미리보기)을 함께 고치므로 wave 4에서 메인이 직렬로 진행합니다.
- 실기(픽셀)는 P23.

---

### 3.4 shell-15 기존 사용자 데이터 디렉터리 사용 (사용자 허락 항목)

#### (a) ID·기능·상태

- shell-15 "기존 사용자 데이터 디렉터리 사용", 기능표 108행, partial. 사용자 허락 보류 항목입니다(`ROOT/docs/acknowledge/2026-10-26-native-autonomous-continuation.md` 12행 "실제 사용자 데이터 경로·keychain 승계(shell-15, 컷오버 정책)", `ROOT/docs/PROCESS.md` 보류 줄, 형제 문서 30 P05·P06·P25).

#### (b) 원본 TS 동작

- 데이터 경로: `TAURI/src/lib.rs` 914~915행 `let data_dir = app.path().app_data_dir().expect(..); let state = AppState::new(AppPaths::new(data_dir));`. identifier `net.gumyo.taide`(`ROOT/src-tauri/tauri.conf.json` 5행), dev 오버레이 `net.gumyo.taide.dev`(`ROOT/src-tauri/tauri.dev.conf.json` 2행). 실제 경로는 `~/Library/Application Support/net.gumyo.taide`입니다(`ROOT/docs/acknowledge/2026-08-29-bundle-identifier-rename.md`: "identifier는 app_data_dir·로그·keyring 서비스명·TCC의 네임스페이스").
- keychain 서비스명: `TAURI/src/lib.rs` 929행 `SecretStoreState::new(app.config().identifier.clone())` = `net.gumyo.taide`. 같은 ack 절차 3: "키체인은 이관 불가(시크릿 비접촉 원칙)".
- 단일 인스턴스: `TAURI/src/lib.rs` 821행 `tauri_plugin_single_instance::init(..)`(두 번째 실행의 argv를 첫 인스턴스로).
- 로그 파일: `TAURI/src/lib.rs` 834행 `tauri_plugin_log::Builder::new()`(출력 위치는 플러그인 기본값, 저장소 밖).
- CLI `taide`: `ROOT/crates/taide-cli/src/main.rs` 15~18행 상수(`TAIDE_APP_PATH`, `/Applications/TAIDE.app/Contents/MacOS/TAIDE`, `target/debug/taide`, `--wait-marker`), `resolve_app_path`(92행), `spawn_app`(119행: 앱 실행 파일을 `<files> [--wait-marker <m>]` 인자로 직접 실행, `--data-dir` 없음).
- 데이터 배치: `ROOT/crates/taide-model/src/paths.rs`(`settings.json`, `session.json`, `themes/`, `snippets/`, `locales/`, `prompts/`, `projects/<id>/{project.json,layout.json,buffers/}`, `plugins/`, `lsp/<server>/<version>`).
- 사용자 문자열 없음.

#### (c) native 현재 자산과 빠진 것

- `APP/bootstrap.rs`: `SECRET_SERVICE = "net.gumyo.taide.native-isolated"`(15행, 133행 `assemble`에서 사용), `Launch::parse`(33~44행), `LaunchConfig::parse_data_dir`(51~72행)가 첫 인자 `--data-dir <절대 경로>`를 강제(상대 경로·루트·`..` 거부), `restore`(74~87행)가 `create_dir_all` 뒤 `AppState::new(AppPaths::new(data_dir))`.
- 단일 인스턴스 소켓 `APP/single-instance.rs` `socket_path(data_dir)`(32행) = `std::env::temp_dir()`의 `taide-native-<hash>.sock`. Tauri 앱의 단일 인스턴스와 서로 모릅니다.
- 창 상태 파일 `native-window-state.json`(data dir 안, Tauri 플러그인 파일과 이름이 달라 공존).
- 로그: stderr만(`APP/stderr-log.rs`, 결정 3 A). 파일 로그는 이 행과 함께 다루기로 했습니다(shell 사양 832행).
- 데이터 형식 호환 근거: 공유 crate(`crates/taide-*`)가 같은 serde 형식을 씁니다. legacy fixture `ROOT/src-tauri/tests/fixtures/rust-native/persistence-v1.json`과 검사 `ROOT/src-tauri/tests/rust_native_phase0_persistence_fixture.rs`.
- 빠진 것: (1) `--data-dir` 없는 실행(실제 제품 경로 해석), (2) keychain 서비스명 승계, (3) 같은 data dir에서 Tauri 앱과 동시 실행 방지, (4) 파일 로그, (5) 롤백(native가 쓴 데이터를 Tauri가 읽는지) 검증, (6) CLI가 native 실행 파일을 찾는 경로(배포 번들).

#### (d) 구현 방법(허락 뒤)

1. 실행 인자
    1. `APP/bootstrap.rs` `Launch::parse`: 첫 인자가 `--data-dir`이면 지금 동작 그대로(격리 실행, 검사·개발용). 아니면 기본 데이터 경로를 쓰고 인자 전체를 `relay_arguments`로 둡니다(CLI의 `<files> --wait-marker <m>`가 그대로 들어옴).
    2. 기본 경로 해석을 순수 함수로 둡니다: `pub fn default_data_dir(home: &Path, flavor: DataFlavor) -> PathBuf` = `home/Library/Application Support/net.gumyo.taide`(`DataFlavor::Release`) 또는 `.../net.gumyo.taide.dev`(`DataFlavor::Dev`). `home`은 `std::env::var_os("HOME")`에서 얻고 없으면 `AppError::Forbidden`. 새 의존성은 추가하지 않습니다. macOS 외 경로는 릴리스 대상 밖이라 `cfg(target_os = "macos")`만 지원하고 나머지는 오류로 둡니다.
    3. dev와 release를 가르는 기준은 메인이 정합니다(예: `cfg(debug_assertions)`). Tauri는 `tauri.dev.conf.json`의 identifier로 갈랐습니다.
2. keychain: 기본 경로 실행이면 `SecretStoreState::new("net.gumyo.taide".into())`, 격리 실행이면 지금 값. `assemble`(`APP/bootstrap.rs` 127행)에 서비스명을 인자로 넘깁니다. 실제 keychain 읽기·쓰기 검사는 금지이므로 서비스명 선택만 단위 검사합니다.
3. Tauri와 동시 실행 방지(결정 필요)
    - A안(추천): native가 data dir에 잠금 파일(`<data_dir>/.native-instance.lock`)을 잡습니다. Tauri 앱 실행 여부는 native가 알 수 없으므로 출시 노트에 "동시 실행 금지"를 적고 베타 동안 Tauri는 rollback 용도로만 둡니다.
    - B안: Tauri 쪽에도 같은 잠금을 추가(Tauri 코드 변경, 배치 32).
4. 파일 로그(결정 필요): Tauri 로그 위치(플러그인 기본값)를 저장소 밖 문서로 확인한 뒤 같은 폴더에 native 로그 파일을 쓸지 정합니다. 확인 전에는 stderr만 유지합니다.
5. 롤백 호환: native가 새로 쓰는 파일(`native-window-state.json`, 잠금 파일)을 Tauri가 무시하는지, native가 쓴 `session.json`·`layout.json`·`settings.json`을 Tauri 파서가 읽는지 검사합니다. 공유 crate 파서를 쓰는 검사를 `ROOT/native/taide-native-app/tests/`에 두어 Tauri 빌드 없이 돌게 합니다.
6. CLI 연결(shell-90 비고와 같은 허락 P06): 배포 번들 실행 파일 경로를 `/Applications/TAIDE.app/Contents/MacOS/TAIDE`로 둘지(Tauri 대체)는 배치 32 cutover입니다. 그 전에는 `TAIDE_APP_PATH`로 native 실행 파일을 가리켜 실기합니다.
7. 번들 identifier: TCC(알림 권한 등)는 번들 identifier 단위입니다(위 ack). native 번들의 identifier를 Tauri와 같게 할지는 배치 31-B6 패키징과 함께 정합니다(`20-gates-31-33.md`).

#### (e) 자동 검사

- 단위(`APP/bootstrap.rs`의 검사 모듈 또는 `ROOT/native/taide-native-app/tests/host.rs`의 실행 설정 검사 옆): `Launch::parse(["a.txt", "--wait-marker", "m"])`가 기본 경로·relay 인자 3개·외부 열기 요청을 만듦, `--data-dir` 실행은 그대로, `default_data_dir(Path::new("/synthetic-home"), Release)` 경로 문자열, `HOME` 없음 오류(환경변수를 바꾸지 말고 `home` 인자로 검사), 서비스명 선택.
- 호환(`ROOT/native/taide-native-app/tests/`): 임시 디렉터리에 legacy fixture를 펼쳐 native `restore` → 세션·프로젝트 복원, native가 저장한 뒤 공유 파서로 다시 읽기.
- 실제 사용자 경로는 자동 검사하지 않습니다(금지).

#### (f) 완료 판정

- 위 구현·검사 + 사용자 실기(실제 데이터로 시작, 프로젝트 목록 승계, keychain 항목 재사용, Tauri로 되돌렸을 때 데이터 정상)가 끝나면 complete.

#### (g) 막는 요소

- 사용자 허락(P05). 허락 전에는 1~5의 코드와 단위 검사까지만 할 수 있습니다. 기본 경로 실행이 실제 사용자 데이터를 열게 되므로 기본 경로 분기를 켜는 커밋 자체를 허락 뒤로 미루는 것을 추천합니다(격리 실행 강제 유지).
- 3·4 결정, 배치 32 cutover(P25).

---

### 3.5 shell-21 전역 키맵 toggle-sidebar/explorer/git

#### (a) ID·기능·상태

- shell-21, 기능표 114행. `ROOT` partial, `GITLANE` complete(같은 행 비고에 git-1 근거, "무프로젝트 키 소비").

#### (b) 원본 TS 동작

- `TS/widgets/app-shell/app-shell.tsx` 143~147행 `useGlobalKeymap({ 'toggle-sidebar': () => requestToggleExplorerSidebar(), explorer: .., git: () => requestShowExplorerView('git') })`. 보조 창도 같은 키맵(`TS/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 96~99행).
- 받는 쪽은 포커스 slot의 패널만 반응합니다(git 사양 197행이 `explorer-panel.tsx` 142~149행 근거로 적음). 키맵 등록은 `AppShell` 수준이라 프로젝트가 없어도 키를 소비합니다(같은 줄).
- 기본 키 ⌃⇧G, 명령 `view.git`(`TS/shared/lib/command-catalog.ts` 94행).

#### (c) native 현재 자산과 빠진 것

- `ROOT`: toggle-sidebar·explorer 완료. git은 미연결(키 미소비·명령 비활성).
- `GITLANE`: `UI/command-registry.rs` `Run::ShowGit`(172행), `"git" => Run::ShowGit`(677행), `("view.git", Run::ShowGit)`(1017행), 단위 단언 1467·1473행. `APP/shell_keymap.rs` `runs_without_focused_project`에 `Run::ShowGit`(72행), `intent`의 `Run::ShowGit` 분기(200~210행: 보조 창이면 `intents::git::Intent::ShowAuxiliaryView`, 아니면 `ShowView { project, slot: 포커스 slot }`), 단위 1463행 `assert!(supports("git") && runs_without_focused_project("git"))`. `APP/command-dispatch.rs` `PROJECTLESS_KEYMAP_ACTIONS`(722~735행, 12개 중 `"git"`). 처리 `GITLANE/APP/domains/git.rs` `apply_git_intent`(1720행)의 `Intent::ShowView`(1727~1740행)·`ShowAuxiliaryView`(1741~1760행). App 검사 `GITLANE/APP/application-git-base-tests.rs` 257행 `실제_app_git_키와_view_git_명령은_포커스_slot만_git_보기로_바꾸고_접힌_사이드바를_펼친다`, 327행 `실제_app_보조_창의_git_키는_그_창의_탐색기를_열고_git_보기로_바꾼다`.
- 빠진 것: 병합 완료와 병합 트리 검증, `ROOT` 행 갱신.
- 문서 불일치: shell 사양 742행은 "프로젝트가 없으면 키를 소비하지 않습니다"라고 썼지만 git 사양 197행·`GITLANE` 구현·`GITLANE` 행은 "프로젝트가 없어도 소비"입니다. 원본 키맵이 `AppShell`에 프로젝트와 무관하게 등록되므로 구현이 원본에 맞습니다. shell 사양 문장은 낡은 기술입니다.

#### (d) 구현 방법

1. 병합이 `ROOT`에 들어온 뒤 `grep -rn "Run::ShowGit" ROOT/native`로 위 심볼이 있는지 확인합니다.
2. 코드 변경은 없습니다. 병합 충돌 파일이었던 `shell_keymap.rs`·`command-dispatch.rs`·`command-registry.rs`에서 `PROJECTLESS_KEYMAP_ACTIONS`와 `runs_without_focused_project`가 같은 집합인지 확인합니다(`command-dispatch.rs` 798행 부근 검사가 단언).

#### (e) 자동 검사

- `cargo test --manifest-path ROOT/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- shell_keymap command_dispatch domains::git::base_tests application::shell_dialogs_tests --test-threads=4`
- `cargo test --manifest-path ROOT/native/taide-native-ui/Cargo.toml --features native-host,inspection --locked --offline --target-dir <T> --lib -- command_registry`

#### (f) 완료 판정

- 위 검사 통과 → complete(`GITLANE` 행 문구 그대로). commands-ui-14(`view.git`, 기능표 653행 unwired)도 같은 근거로 commands-ui 쪽에서 갱신합니다.

#### (g) 막는 요소

- Git 병합 완료(다른 작업자 진행 중).

---

### 3.6 shell-59 explorer 내부 4개 view 전환

#### (a) ID·기능·상태

- shell-59, 기능표 152행, `ROOT`·`GITLANE` 모두 partial. `GITLANE`도 비고를 고치지 않았습니다(남은 범위 "Git 패널 본체·⌃⇧G/`view.git`(배치 26)").

#### (b) 원본 TS 동작

- `TS/widgets/explorer/explorer-panel.tsx`: 보기 목록(files `explorer.title` FolderTree, search `search.title` Search, git `git.title` GitBranch, outline `outline.title` ListTree), 검색·보기 요청·reveal·rename 구독(포커스 slot만), 전환 줄(`role='tablist'`, `aria-label={t('explorer.sidebarSwitchLabel')}`), 본문(`view === 'git' && <GitPanelContainer projectId={projectId} />`).
- locale 키(`en.json`): `explorer.title`(267행), `search.title`(632행), `git.title`(342행), `outline.title`(481행), `explorer.sidebarSwitchLabel`(266행).

#### (c) native 현재 자산과 빠진 것

- `ROOT`: 전환 줄·slot별 보존·tooltip 완료, Search 본문(explorer-search-31), ⌘⇧F/⌘⇧H(explorer-search-37). Git 보기는 전환만 되고 본문이 비어 있습니다(`APP/application-explorer-views-tests.rs` 186행 검사 이름 `..git은_빈_본문만_보인다`).
- `GITLANE`: `APP/domains/explorer_search.rs` 1764~1766행 `if sidebar.view == crate::symbol_sidebar::View::Git { self.show_git_view(ui, project, slot, &mut *icons, errors, intents); .. }`, `show_git_view`(`GITLANE/APP/domains/git.rs` 2922행, `&self` 읽기 전용 + intent 출력이라 사이드바 장애 경계와 맞음). App 검사 이름이 `실제_app_사이드바는_네_보기를_slot별로_전환하고_검색은_패널을_git은_scm_본문을_보인다`(`GITLANE/APP/application-explorer-views-tests.rs` 186행)로 바뀌었습니다.
- 빠진 것: 병합과 병합 트리 검증, 행 갱신.

#### (d) 구현 방법

1. 병합 뒤 `APP/application-explorer-views-tests.rs` 186행 검사 이름이 `..git은_scm_본문을_보인다`인지, 검색 본문 단언과 Git SCM 본문 단언이 함께 남았는지 확인합니다(충돌 파일이었음).
2. 코드 변경 없음.

#### (e) 자동 검사

- `cargo test --manifest-path ROOT/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- application::explorer_views_tests domains::git::base_tests application::search_panel_tests application::explorer_search_tests --test-threads=4`

#### (f) 완료 판정

- 통과 → complete. 비고: "Search 본문 explorer-search-31·37, Git 본문·⌃⇧G git-1(`2026-11-02-native-lane-git-g1-base.md`)". explorer-search-1·30 행의 "Git 보기 본문은 git-1 행이 판정" 문구(기능표 379·408행)도 함께 정리합니다.

#### (g) 막는 요소

- Git 병합 완료.

---

### 3.7 shell-78 탭 context menu

#### (a) ID·기능·상태

- shell-78, 기능표 171행, `ROOT`·`GITLANE` 모두 partial(비고 "변경 열기·파일 이력(배치 26)은 native 소비자가 없어 표시하지 않아 partial").

#### (b) 원본 TS 동작

- `TS/features/tab/tab-context-menu.tsx` 97~192행 항목 순서: 닫기 5종 | Pin/Unpin, Keep Open(preview일 때) | 경로 복사·상대 경로 복사(파일 탭) | Finder reveal, Explorer 보기 reveal, Rename, Open Changes, File History(파일 탭) | Reopen Editor With | 분할 4방향 | 새 창·메인 창·창 N 이동.
- 동작: `TS/widgets/editor-area/pane-tab-bar.tsx` 209행 `onOpenChanges`(파일 탭이면 `diff` 종류 탭 `{ path, staged: false }`, 제목 `${tab.title} (diff)`, 대상 = 현재 창 focus pane, preview), 224행 `onFileHistory`(`requestOpenFileHistory(filePath)`).
- locale 키: `tab.openChanges`("Open Changes", `en.json` 1008행), `git.fileHistory`("File History", 301행).

#### (c) native 현재 자산과 빠진 것

- `ROOT`: 나머지 항목 전부 완료(근거 `2026-10-27-native-tab-icons-menu.md`, `-auxiliary-window-moves.md`).
- `GITLANE`: `UI/shell.rs` `tab_menu`(2257행)에 rename 다음·reopen-with 앞으로 `tab.openChanges` → `ShellIntent::Git(Intent::OpenTabChanges { project, path, title: format!("{} (diff)", tab.title), window: menu.window })`(2329~2338행), `git.fileHistory` → `Intent::OpenFileHistory { project, path }`(2339~2347행). git-48·git-55 complete(`GITLANE` 기능표 525·532행). 검사: UI `GITLANE/native/taide-native-ui/tests/workbench.rs` 2256행 `탭_메뉴는_주창_파일탭에_탐색기_reveal_rename과_미리보기파일_다시열기를_요청한다`의 2291~2305행(순서 rename < openChanges < fileHistory < reopenEditorWith, 선택 intent), App `GITLANE/APP/application-git-diff-tests.rs` 461행 `실제_app_파일_탭_메뉴의_변경_열기는_focus_pane에_preview_diff_탭을_연다`, `GITLANE/APP/application-git-claude-history-tests.rs` 466행 `실제_app_파일_이력은_명령_탭_메뉴로_열리고_항목을_inline_diff로_보여준다`.
- 빠진 것
    1. 병합과 검증, 행 갱신.
    2. 낡은 문서 주석: `GITLANE/UI/shell.rs` 2255~2256행 `/// The tab's own context menu (\`tab-context-menu.tsx\`). Entries whose native consumer does not exist yet (Git file history) are not shown.` 두 항목이 모두 보이므로 사실과 다릅니다.
    3. 보조 창에서 연 메뉴의 변경 열기 대상(그 창의 focus pane) App 검사가 없습니다. git-48 비고는 보조 창 규칙(`window: menu.window`)을 말하지만 검사는 메인 창 1건뿐입니다.

#### (d) 구현 방법

1. 병합 뒤 `UI/shell.rs` `tab_menu` 문서 주석을 `/// The tab's own context menu (\`tab-context-menu.tsx\`).` 한 줄로 줄입니다.
2. 보조 창 검사를 `APP/application-git-diff-tests.rs` 461행 검사 옆에 추가합니다.
    1. `APP/application-auxiliary-window-tests.rs`의 `choose_tab_menu`(490행, 지금 비공개)를 `pub(super)`로 올립니다(같은 파일 helper를 올린 선례 `install_viewports`·`step_until`·`viewport_of`).
    2. Git 저장소 fixture(`fixture::repository`)로 파일 `a.txt`를 커밋한 뒤 수정 → 그 파일 탭을 가진 보조 창 layout(`auxiliary_window(slot, &file_tab(path))`) → `install_viewports` → `choose_tab_menu(application, &context, &viewports, Some(viewport), "a.txt", &text("tab.openChanges"))` → `step_until`로 보조 창 트리의 focus pane에 `TabKind::Diff { path, staged: false, .. }` preview 탭이 있고 메인 트리는 변하지 않음을 단언합니다.

#### (e) 자동 검사

- UI: `cargo test --manifest-path ROOT/native/taide-native-ui/Cargo.toml --features native-host,inspection --locked --offline --target-dir <T> --test workbench -- 탭_메뉴`
- App: `cargo test --manifest-path ROOT/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- domains::git::diff_tests domains::git::claude_history_tests application::tab_menu_tests application::auxiliary_window_tests --test-threads=4`

#### (f) 완료 판정

- 위 검사와 새 보조 창 검사 통과 → complete. 비고에 git-48·git-55 근거.

#### (g) 막는 요소

- Git 병합 완료.

---

### 3.8 shell-89 탭 콘텐츠: diff/commit diff/claudeDiff/searchEditor

#### (a) ID·기능·상태

- shell-89, 기능표 182행. `ROOT` partial("searchEditor 연결, 남은 것 diff/commit diff/claudeDiff"). `GITLANE`(충돌 해결된 작업 트리) partial: "2026-11-03 git lane 병합으로 네 본문이 모두 탭 분기에 연결 … 남은 것: ClaudeDiff 수정 쪽 revert 화살표(git-50)".

#### (b) 원본 TS 동작

- `TS/widgets/editor-area/pane-node-view.tsx` 203~232행
    - `diff` + `rev == null`(203행) → `DiffPane`(projectId, path, staged, compareWith, beforePath)
    - `diff` + `rev != null`(213행) → `CommitFileDiff`(rev, parentRev, path, beforePath)
    - `claudeDiff`(224행) → `ClaudeDiffPane`(projectId, tabId, requestId, path)
    - `searchEditor` → `SearchEditorPane`(242~244행)
- `TS/widgets/claude-diff-pane/claude-diff-pane.tsx`: `monaco.editor.createDiffEditor(container, { automaticLayout: true, readOnly: false })`(58행), Reject `ide.rejectChanges`(145행)·Accept `ide.acceptChanges`(148행), Accept → `ide.diffAccepted` success toast(117행), Reject → `ide.diffRejected` info toast(132행).
- ClaudeDiff hunk 되돌리기(Monaco 0.56.0)
    - 기본값 `renderMarginRevertIcon: true`, `renderGutterMenu: true`(`MONACO/esm/vs/editor/common/config/diffEditor.js` 8~9행). `renderGutterMenu`가 참이면 옛 여백 화살표가 꺼집니다(`MONACO/esm/vs/editor/browser/widget/diffEditor/diffEditorOptions.js` 42~57행). 그래서 보이는 것은 35px gutter(`.../features/gutterFeature.js` 38행 `const width = 35`)의 hunk 도구 막대입니다.
    - 항목은 `diffEditorModifiedWritable`일 때만 있습니다(`.../diffEditor.contribution.js` 42~61행: side-by-side `Codicon.arrowRight` U+EA9C, inline `Codicon.discard` U+EAE2, 제목 "Revert Block"·"Revert Selection"; 코드포인트는 `MONACO/esm/vs/base/common/codiconsLibrary.js` 142·216행). gutter 폭은 항목이 있을 때만 35, 없으면 0입니다(`gutterFeature.js` 52~54행). 그래서 읽기 전용 `DiffPane`에는 gutter가 없고 쓰기 가능한 ClaudeDiff에만 있습니다.
    - 보임: 커서가 있는 hunk 항목은 항상(`showAlways`), 나머지는 gutter hover 때만(`.../diffEditor/style.css` 339~356행). 항목 크기 34×20(`gutterFeature.js` 164행).
    - 동작: `revert(diff)`가 수정 쪽 범위를 원본 범위 텍스트로 바꾸고 앞뒤 undo stop(`.../diffEditorWidget.js` 359~372행), 선택 되돌리기 `revertRangeMappings`(373~385행). diff가 최신이 아니면(`isDiffUpToDate` 거짓) 무동작(361·375행).
    - TAIDE는 Monaco nls를 설정하지 않으므로(`grep -rln "nls.messages\|esm/nls" ROOT/src` 결과 없음) "Revert Block"은 모든 언어에서 영어로 보입니다.

#### (c) native 현재 자산과 빠진 것

- `ROOT`: `APP/domains/editor.rs` 5388~5401행 탭 분기, searchEditor는 `show_search_editor_tab`(`APP/domains/explorer_search.rs` 2271행) 연결, diff·ClaudeDiff는 `APP/domains/git.rs` 136~143행 `show_unavailable_tab` 자리표시.
- `GITLANE`: `APP/domains/editor.rs` 5400~5410행이 `show_search_editor_tab`, `show_diff_tab(ui, project, pane, tab, intents)`, `show_claude_diff_tab(..)`(`GITLANE/APP/domains/git.rs` 2675·2821행, `impl AppSurfaces<'_>` 2672행)을 부릅니다. 공용 그리기 `show_surface`(1085행, `SideText::{Read, Editable}` 1054행, `Surface` 1062행). diff 위젯 `GITLANE/native/taide-native-ui/src/diff-view.rs`(`Request` 71행: `stage_lanes`·`hide_unchanged`, `Output` 216행: `lane_press`, `Hunk` 51행). 되돌리기 관련 코드는 없습니다(`grep -n -i revert diff-view.rs` 결과 없음). App 검사 `application-git-diff-tests.rs`(161·267·339·380·461·516·558·640행), `application-git-claude-history-tests.rs`(218·289·372·466행).
- 빠진 것: ClaudeDiff 수정 쪽 hunk 되돌리기 gutter(git-50 남은 것, `GITLANE` 기능표 527행 "수정 쪽 revert 화살표(결정 4 A)와 git-40 공통 장식").

#### (d) 구현 방법

Git 영역 코드(`diff-view.rs`, `domains/git.rs`)입니다. 구현 정본은 `14-remaining-git.md` 3.5절(git-50)이고 한 번만 구현합니다. 그 설계의 요점과 shell 행 판정에 필요한 확인:

1. `UI/diff-view.rs` `Request`에 `revert_blocks: bool`(ClaudeDiff만 참)과 `is_diff_current: bool`을 추가하고, gutter 열(`REVERT_GUTTER_WIDTH` 35, 항목 34×20), 커서 hunk 항상 표시·hover 시 전부 표시, 누름 → `Output.revert: Option<Revert>`를 냅니다. 같은 위젯의 기존 `stage_lanes`/`lane_press`(76·219·675·761·882·1020행)가 열 배치와 누름 출력의 선례입니다.
2. 아이콘은 같은 파일의 codicon 그리기 방식(42행 `UNFOLD_CODICON = '\u{eb73}'`, 463~464행 `FontId::new(CODICON_SIZE, crate::font_families::codicons(ui))`)으로 `'\u{ea9c}'`(side-by-side)·`'\u{eae2}'`(inline)를 그립니다.
3. 접근성 이름·tooltip 문구는 원본처럼 영어 리터럴 `"Revert Block"`·`"Revert Selection"`입니다(0.4절 선례: 같은 파일 43행 `HIDDEN_LINES_LABEL`). locale 키를 새로 만들지 않습니다.
4. `is_diff_current`는 `git_claude_diff::State`의 `Lines`(`GITLANE/APP/git-claude-diff.rs` 42~47행: `computed`, `wanted`, `in_flight`)에서 "`computed == Some(현재 revisions)`이고 `wanted`·`in_flight`가 없음"으로 계산합니다. 원본 `isDiffUpToDate`와 같은 규칙입니다.
5. 앱: 새 intent `RevertClaudeDiff { tab, original, modified }`를 `apply_git_intent`에서 처리해 수정 가상 문서에 `self.store.apply_separate(document, Transaction { revision, edits: vec![Edit { bytes, text }], group, origin: Some(view), selection_after: None })`(`EDITOR/store.rs` 1606행, `Transaction` 31~37행, `Edit` `EDITOR/document.rs` 39~42행)로 편집 하나를 적용합니다. 같은 패턴 `GITLANE/APP/git-editor.rs` 384행 `apply_conflict_resolution`(407행 `apply_separate`). 적용 뒤 기존 `ClaudeDiffShown` → 200ms 재계산(`RECOMPUTE_DELAY`, `git-claude-diff.rs` 12행)이 hunk를 갱신합니다.
6. 읽기 전용 diff(`DiffPane`, CommitFileDiff)는 `revert_blocks: false`라 gutter 폭이 0입니다(원본과 같음).

#### (e) 자동 검사

- `14-remaining-git.md` 3.5 (e)의 UI·App 검사(`실제_app_claude_diff_revert_block은_수정_쪽_hunk를_원본으로_되돌리고_다시_계산한다`).
- shell-89 판정용 회귀: `cargo test --manifest-path ROOT/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- domains::git::diff_tests domains::git::claude_history_tests git_claude_diff search_editor --test-threads=4`, UI `--test diff-view`.

#### (f) 완료 판정

- 네 본문 연결(병합 트리에서 위 회귀 통과) + hunk 되돌리기 검사 통과 → shell-89 complete, git-50도 같은 근거. git-40 공통 장식(문자 단위 강조·±·overview ruler)은 git-40 행이 추적하며 shell-89 완료를 막지 않습니다. 메인이 "탭 콘텐츠 분기 연결만으로 shell-89 complete"를 택하면 그 결정을 기록하고 hunk 되돌리기는 git-50에만 남깁니다(현재 `GITLANE` 행 문구는 shell-89 남은 것에 revert를 넣었으므로 기본은 위 기준).

#### (g) 막는 요소

- Git 병합 완료. 아이콘 색의 원본 최종값 확인은 `14-remaining-git.md` 3.5 (g)(원본 화면 확인 필요, 그 전에는 `editor.foreground`).

---

### 3.9 shell-92 IDE 동기화

#### (a) ID·기능·상태

- shell-92, 기능표 185행, `ROOT`·`GITLANE` 모두 partial(남은 것 "diff·close 요청 소비(git-49, git lane) 뒤 묶음 3 판정").

#### (b) 원본 TS 동작

- `TS/app/providers/ide-sync-provider.tsx`(주 창만)
    - `ideDiffRequested`(49~68행): 대기 diff 등록, `ideAutoOpenDiff`가 꺼져 있으면(54행) 등록 제거 + `resolveIdeDiff({ outcome: 'rejected' })`, 켜져 있으면 `claudeDiff` 탭(`{ requestId, path: newPath }`, 제목 `tabName`, `target: null`, `preview: false`). 오류는 IPC 오류 toast.
    - `ideSaveRequested`(70~129행): 모든 트리에서 같은 경로 파일 탭 → 없거나 clean이면 `true`, model 없으면 `false`, read-only면 `false`, 그 밖은 저장 후 결과.
    - `ideCloseTabRequested`(131~133행): `requestId`가 있으면 대기 diff 제거.
    - `useIdeStatusSync()`(135행), 진단 push(137~158행: 포커스 slot 프로젝트·IDE 실행 중일 때 300ms debounce 뒤 `publishIdeDiagnostics`, 154행).
- 사용자 문자열: 원본 toast는 IPC 오류 문구만. ClaudeDiff 쪽은 3.8의 키.

#### (c) native 현재 자산과 빠진 것

- `ROOT`: save(`resolve_ide_save` 1273행, `finish_ide_saves` 1311행), 진단 push(`push_ide_diagnostics` 1320행, `APP/ide-sync.rs` 10행 `IDE_DIAGNOSTICS_PUSH_DEBOUNCE` = 300ms), 상태 동기화(terminal-agent-65). App 검사 `APP/application-ide-sync-tests.rs` 51·124·186행, 단위 `ide_진단_push는_포커스_프로젝트가_바뀌면_새_프로젝트로_다시_센다`(`APP/ide-sync.rs` 검사 모듈).
- `GITLANE` lane 커밋: `git_claude_diff::Inbox` + `git_cache::Changes.ide` + `consume_ide_diff_events`(`tick_claude_diffs` 첫 줄). git-49 complete(`GITLANE` 기능표 526행, 비고 "수신은 대체 경로 `git_cache::Changes.ide` … shell lane 수신함 병합 뒤 그 분기로 옮기는 일이 남음").
- `GITLANE` 병합 작업 트리(미커밋, 0.2절 1번): 수신이 shell `provider_events::Inbox`로 옮겨졌습니다. `Event::IdeDiff` → `consume_provider_events` → `accept_claude_diff_event`(대기 등록, `ide_auto_open_diff` 꺼짐이면 `Command::ResolveIdeDiff { outcome: Rejected }`, 아니면 `Command::OpenTab { kind: TabKind::ClaudeDiff { request_id, path }, title, target: None, preview: false }`, `Closed`면 대기 제거). 즉 lane 비고의 "옮기는 일"은 병합 작업 트리에서 끝났습니다.
- 빠진 것
    1. 병합과 병합 트리 검증, 행 판정, git-49 비고 정정.
    2. 진단 push의 "포커스 slot을 바꾸면 새 프로젝트로 다시 push"는 단위 검사만 있고 실제 App 검사가 없습니다(`ROOT/docs/quality-assurance/2026-11-02-native-lane-shell-providers.md` "테스트 공백 (a)" 줄: "두 프로젝트 slot 전환 fixture가 필요해 남김").

#### (d) 구현 방법

1. 병합 뒤 `grep -n "IdeDiff" ROOT/native/taide-native-app/src/provider-events.rs ROOT/native/taide-native-app/src/domains/shell.rs ROOT/native/taide-native-app/src/domains/git.rs`로 세 지점을 확인하고, `grep -n "Inbox\|\.ide\b" ROOT/native/taide-native-app/src/git-cache.rs ROOT/native/taide-native-app/src/git-claude-diff.rs` 결과가 없어야 합니다(이중 기록이면 요청마다 탭이 두 번 열림). 이 확인은 `14-remaining-git.md` 3.7 (d)와 같으므로 한 번만 합니다.
2. 동작 차이 두 가지를 QA에 적습니다(14 문서 3.7 (d) 3번과 같은 내용).
    - 수신함 상한: `MAX_PROVIDER_EVENTS` = 256을 넘으면 가장 오래된(프로젝트 닫힘 제외) 이벤트를 버립니다(`GITLANE/APP/provider-events.rs` 124~130행). IDE diff 요청이 버려지면 IDE는 `IDE_DIFF_TIMEOUT` = 600초(`APP/ide-tools.rs` 27행) 뒤 `Rejected`를 받습니다. 원본은 상한이 없습니다. 한 frame에 256개가 쌓이는 경우만 해당합니다.
    - 종료 중: `consume_provider_events`는 닫는 중·종료 준비·shutdown이면 꺼낸 이벤트를 처리하지 않고 버립니다(1114~1117행). 그 사이 요청은 IDE timeout까지 대기합니다.
3. 포커스 slot 변경 App 검사 추가(`APP/application-ide-sync-tests.rs`)
    1. 두 프로젝트 각각에 진단을 낼 파일을 두고 메인 2 slot 구성으로 띄웁니다. 구성은 `APP/application-explorer-views-tests.rs` 186행 검사(두 slot, `left_half()` 178행)와 같은 방식, IDE 실행 상태와 진단 store 준비는 같은 파일 186행 검사(`실제_app은_ide가_실행중일_때만_진단을_300ms_뒤_push한다`)와 같은 방식을 씁니다.
    2. 첫 slot 포커스로 300ms 뒤 push가 첫 프로젝트 파일 경로만 담는지 단언 → 두 번째 slot을 클릭해 포커스 변경 → 300ms 전에는 push 없음, 뒤에는 두 번째 프로젝트 파일 경로만 담는지 단언(같은 파일 186행 검사가 읽는 값으로).

#### (e) 자동 검사

- `cargo test --manifest-path ROOT/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- ide_sync provider_events domains::git::claude_history_tests git_claude_diff ide_tools ide_server --test-threads=4`

#### (f) 완료 판정

- 병합 트리에서 위 검사 통과 + 3번 검사 추가·통과 → complete. 비고에 save·진단 근거(providers QA)와 diff·close 근거(`2026-11-02-native-lane-git-g7-commit-claude-diff.md`, 수신 경로는 shell 수신함). 같은 근거로 terminal-agent-68(기능표 361행, unwired로 낡음)은 terminal-agent 영역이 갱신합니다.

#### (g) 막는 요소

- Git 병합 완료.

---

### 3.10 shell-93 IPC 동기화

#### (a) ID·기능·상태

- shell-93, 기능표 186행, partial(남은 것 "Git 이벤트 소비(git-64, git lane) 뒤 묶음 3 최종 판정").

#### (b) 원본 TS 동작

- `TS/app/providers/ipc-sync-provider.tsx`(주 창·보조 창 모두 mount). 이벤트별 대응표는 shell 사양 2.1절입니다. Git 관련:
    - `gitStatusChanged`(399행)·`gitRefsChanged`(406행): `QUERY_KEY.GIT.PROJECT(projectId)` 중 `isGitQueryScopeMutable`인 query 무효화.
    - `fsChanged`(489행~): 파일 내용 무효화, `GIT.STATUS` 무효화, `GIT.PROJECT` 중 바뀐 경로의 gutter·diff(`isGitWorktreeQueryForChangedPaths`, 72행), 빠른 열기 색인, tree 동기화.
    - `fsRescanRequired`: 무효화 전부 + 보이는 디렉터리 재읽기.
- 사용자 문자열 없음.

#### (c) native 현재 자산과 빠진 것

- `ROOT`: Git 외 전부 충족(`presentation_refresh::Changes::record`의 `SyncStateChanged`, 앱 밖 `ProjectClosed` 정리 `forget_closed_project`, settings 영역 기록). App 검사 `APP/application-ipc-sync-tests.rs` 158·204·254·352행 부근 4건.
- `GITLANE`: `APP/git-cache.rs` `Changes::record`(54~71행)가 `GitStatusChanged`·`GitRefsChanged`·`FsRescanRequired` → 저장소 범위, `FsChanged` → 그 경로들. `Key`(249행)의 `is_mutable`(315행)·`follows_worktree`(337행)가 경로 키를 무효화합니다. frame당 요청 상한 `MAX_REQUESTS_PER_FRAME` = 16(16행). git-64 complete(`GITLANE` 기능표 541행, 검사 `GITLANE/APP/application-git-base-tests.rs` 485행 `실제_app_변경_목록은_그룹과_건수와_행_표시를_원본대로_그리고_git과_fs_변경_뒤_다시_읽는다`). 병합 작업 트리의 `PaintSink`(`GITLANE/APP/application.rs` 50~71행)에는 `git_cache`와 `providers`가 함께 있고 `title_branches`는 없어졌습니다(제목줄 branch도 git cache status를 읽음, 0.2절 2번).
- 빠진 것: 병합과 검증, 행 판정. App 검사가 실제 OS watcher가 아닌 sink 주입을 쓴다는 점은 git-64 비고에 이미 적혀 있습니다.

#### (d) 구현 방법

1. 병합 뒤 `APP/application.rs` `PaintSink::publish`에 `self.git_cache.record(&event)`와 `self.providers.record(&event)`가 둘 다 있는지 확인합니다(병합 의미 충돌 선례: `PaintSink.providers`, PROCESS 2026-11-03 병합 기록).
2. 제목줄 branch가 Git 변경 뒤 다시 읽히는지 확인하는 기존 검사(`APP/application-auxiliary-window-tests.rs` 1061행 `실제_app_제목줄은_프로젝트_git_branch를_읽고_git_변경_뒤_다시_읽는다`, 병합 트리에서 `application.git_state.title_branch(&alpha)`로 바뀜)를 함께 실행합니다.
3. 코드 변경 없음.

#### (e) 자동 검사

- `cargo test --manifest-path ROOT/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- ipc_sync_tests domains::git::base_tests git_cache presentation_refresh application::auxiliary_window_tests --test-threads=4`

#### (f) 완료 판정

- 통과 → complete. 비고: Git 이벤트 소비는 git-64(`2026-11-02-native-lane-git-g1-base.md`), 나머지는 providers QA.

#### (g) 막는 요소

- Git 병합 완료.

---

## 4. 이 영역에서 반복된 함정과 주의

QA 문서의 실패 기록·흔들림·리뷰 차단 항목과 이번 확인에서 추렸습니다.

### 4.1 판정·문서

1. 과대 판정: shell lane 리뷰가 shell-3을 complete에서 partial로 되돌렸습니다(지정 검사를 실행하지 못함, windows-layers QA 111~128행). shell-92·terminal-agent-70 비고는 실제 분기 순서와 다르게 써서 정정했습니다(providers QA "과대 판정 정정" 1번). 행을 올리기 전에 비고의 모든 문장이 실제 검사 단언과 맞는지 하나씩 대조하십시오.
2. 기능표 머리 요약표(13행)는 행 상태와 따로 갱신해야 합니다. 지금 낡았습니다(1.1절).
3. 사양과 구현이 다르면 실제 코드와 원본 TS를 따르고 QA에 차이를 적습니다. 예: shell-21 무프로젝트 키 소비(3.5), shell-98의 `for_target` 대신 `ToastLayers` 역참조(windows-layers QA 59행), 사양 2.12의 `apply_appearances` 공통화가 이미 `apply_presentation_appearances`로 있음(같은 QA 58행).
4. 이전 작업자 QA의 "통과"를 현재 상태로 믿지 마십시오. 인수 시점에 실제로 실패하던 선례가 있습니다(`2026-10-07-native-transition-operations.md` 6절).
5. `GITLANE` 행 비고가 shell 행(59·78)을 갱신하지 않았고, git-49 비고는 병합 작업 트리의 수신 경로 이동을 아직 반영하지 않았습니다. 병합 뒤 직접 고치십시오.
6. 형제 문서와 같은 작업(git-50, git-49, 31-A2)을 두 번 구현하지 마십시오. 0.5절의 정본을 따릅니다.

### 4.2 검사 실행

1. LSP mock 예제를 먼저 빌드하지 않으면 앱 lib 검사 10건이 실패합니다.
2. 앱 lib 병렬 실행에서 host 큐(64) 포화·종료 대기 시간 초과(`phase=..-exit`)가 납니다. `--test-threads=4`(lane은 2)로 줄이고, 시간 초과는 단독 1회 재실행으로 흔들림과 결함을 구분합니다(PROCESS 단계 E2, 병합 게이트 기록).
3. 기존 실패: `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`, `ROOT/crates/taide-runtime/tests/agent_probe.rs` 소스 문자열 단언 1건(`81e96198`부터). 기존 흔들림: `context_menu_tests::실제_추가_경계_보조창_메뉴는_클립보드와_터미널의_소유_pane을_보존한다`(seams 문서 24~26행). 새 실패와 섞지 마십시오.
4. 하네스: `output.textures_delta.clear()`를 빼면 실패합니다. toast는 등장 애니메이션 첫 frame의 불투명도 0이라 그려지지 않으므로 출력이 나올 때까지 frame을 진행합니다(windows-layers QA 97행). 프레임 밖에서 시각을 넘겨 부르면 egui 입력 focus가 기본값이 됩니다. 비활성 탭의 문서는 로드되지 않습니다. 보조 창의 HTML 파일은 미리보기로 열립니다(providers QA 108행).
5. panic frame은 `request_discard`로 같은 frame에 두 pass가 돌아 검사 Surfaces 기록이 두 번 쌓이고, 입력 event는 첫 pass에만 들어가 마지막 pass intent가 비어 있습니다. 개수 단언 대신 포함 여부와 "discard 대조군"을 쓰십시오(foundation QA 125~127행).
6. `ROOT/native/taide-native-app/tests/explorer-delete.rs`는 실행 금지입니다(사용자 휴지통에 합성 파일이 들어간 선례, PROCESS 단계 D).
7. 일부 파일은 `cfg(test)`에서 UI 소스를 `include!`로 복제 컴파일합니다. 검사가 배포 코드와 다른 컴파일 단위를 검증할 수 있습니다(`2026-10-07-native-transition-operations.md` 6절).
8. vendored eframe은 앱 workspace에서 검사할 수 없고(3.1), macOS winit event loop는 libtest에서 만들 수 없습니다. 창·OS가 필요한 경로는 "컴파일 + 코드 확인 + 실기"로 분리해 적으십시오.
9. 비공개 검사 helper(`choose_tab_menu`, `shell_background_painted`)를 다른 모듈에서 쓰려면 `pub(super)`로 올립니다. 복사해서 두 벌을 만들지 마십시오.

### 4.3 구조

1. 공용 상태를 render 안에서 바꾸는 영역에 `AssertUnwindSafe` 경계를 그냥 두면 panic 뒤 반쯤 바뀐 상태가 저장됩니다(기각된 C안). 경계는 트랜잭션화한 영역에만 둡니다.
2. immediate viewport hook 시점의 `ctx.viewport_id()`는 부모입니다(2.1절 4번, 3.2 (d) 3.3).
3. 보조 viewport의 root `Ui`는 pass 시작 때 스타일을 복사합니다. 창별 visuals는 `show_viewport_immediate` 호출 전에 바꿔야 합니다(2.1절 3번, 3.2 (d) 1.1).
4. 여러 스레드가 읽는 `Arc<Mutex<..>>` 값(터미널 palette)을 frame 중간에 바꿔 되돌리는 방식은 다른 스레드가 중간 값을 읽습니다(3.2 (d) 1.3).
5. 보조 창 pass는 메인 `show_window`를 다시 부르므로 "ROOT일 때만" 분기(`context.viewport_id() == egui::ViewportId::ROOT`)를 놓치면 메인 전용 값(web placement, focus)이 보조 창 값으로 덮입니다. 반대로 메인 전용 처리에 보조 창 분기를 빠뜨리면 기능이 조용히 사라집니다(shell-5 남은 범위 1·3의 원인).
6. 병합 의미 충돌: 2026-11-03 병합에서 `ToastLayers`, `PaintSink.providers`, 탐색기 mount 분리가 의미 충돌을 냈고(PROCESS W1 병합 기록), Git 병합에서는 IDE 수신함과 제목줄 branch가 바뀌었습니다(0.2절). shell 코드가 다른 영역 소유 필드를 직접 지우는 곳(`forget_closed_project` 1061행)은 그 영역 구조가 바뀌면 함께 고쳐야 합니다.
7. 새 shell intent는 `UI/intents/shell.rs`에, 새 host 명령은 `domains::shell::Command` 끝에 둡니다. 공용 파일 루트 블록을 고치지 마십시오(seams 문서 1.4절).
8. 경계의 `text` 클로저와 `&mut` surfaces를 같은 시점에 빌리면 컴파일되지 않습니다. 문구를 미리 만들어 넘기십시오(3.3 (d) 2.5).
9. 단일 인스턴스: 첫 인스턴스 부팅이 `RELAY_ACK_TIMEOUT`(5초, `APP/single-instance.rs` 14행)보다 길면 두 번째가 단독 실행합니다. `Command::Notify`는 osascript를 기한 없이 기다립니다. IDE save 제출 실패 경로에 `ide_sync.saves` 항목이 남을 수 있습니다(providers QA "반영하지 않은 비차단 항목", 후속 과제).

### 4.4 운영

1. workflow 재개 때 이미 실행된 단계의 프롬프트 문자열을 바꾸면 캐시가 깨집니다. 추가 규칙은 아직 실행되지 않은 단계에만 넣습니다(`2026-10-28-native-lane-operations.md` 5절 66행).
2. "implement만 끝내고 멈춰"는 진행 중 구현이 끝나는 즉시 정지하고 리뷰 전 변경은 worktree에 미커밋으로 둡니다. 소요 시간 질문은 중단 지시가 아닙니다(같은 절).
3. 사용자 허락이 필요한 것(shell-15 P05, vendored Cargo 변경 P18, 저장소 밖 소스 확인 P20, 실기 P23)은 진행하지 말고 보류 목록에 남깁니다.
4. `GITLANE`은 다른 작업자가 병합 중입니다. 그 worktree의 파일을 고치거나 스테이징하지 마십시오. 읽기만 합니다.
