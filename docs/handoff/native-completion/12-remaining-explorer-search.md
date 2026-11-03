# 12. 남은 작업: explorer-search 영역

> 작성일 2026-11-03 · 대상 영역 `explorer-search`(탐색기·검색 패널·검색 편집기·아웃라인·Problems)
> 기준 코드: 메인 작업 트리 `/Users/hyunseokbyun/development/TAIDE`(브랜치 `to_rust_native`, HEAD `7be1ad14`, explorer-search lane 병합 `6dba64f9`·shell lane 병합 `4fa4beff`·settings lane 병합 `3864e1db`·git lane 병합 `f3a5b5d0` 포함).
> 이 문서는 정적 읽기로 작성했습니다. cargo·앱 실행은 하지 않았습니다. 줄 번호는 `0c52b438` 기준이며 현재 HEAD에서는 심볼 이름으로 찾습니다.

---

## 0. 읽는 법과 전제 지식

### 0.1 이 문서가 기준으로 삼는 문서

| 문서 | 쓰임 |
| --- | --- |
| `docs/quality-assurance/2026-10-09-native-function-matrix.md` 373-423행(`## explorer-search` 절) | 행 상태의 단일 출처. 행마다 `| ID | 요구사항 | TS 근거 | 현재 판정 | 근거 묶음 | 범위/잔여 동작 |` 6열입니다 |
| `docs/research/2026-10-28-lane-explorer-search-spec.md` | 이 영역 lane 사양. 2.0절(공통 설계), 2.1~2.3절(ID별 원본 동작·설계), 4장(결정 D1~D10, 허락 P1~P3) |
| `docs/quality-assurance/2026-11-02-native-lane-explorer-search-explorer-rows.md` | 묶음 A(7·8·25·26·27·29·30·44·18) 구현·검증 기록 |
| `docs/quality-assurance/2026-11-02-native-lane-explorer-search-search-panel.md` | 묶음 B(1·30·31·32·34·35·38) 기록 |
| `docs/quality-assurance/2026-11-02-native-lane-explorer-search-search-entry-editor.md` | 묶음 C(16·20·21·33·36·37) 기록과 리뷰 반영 |
| `docs/research/2026-10-28-native-lane-seams.md` | 영역 모듈·`HostCommand::<영역>`·`ShellIntent::<영역>`·공용 파일 규칙 |
| `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` | 사양 결정 채택(전부 추천안)과 허락 보류 목록 |
| `docs/memory/2026-10-28-native-lane-operations.md` | lane 운영·작업자 공통 금지 |
| `docs/memory/2026-10-07-native-transition-operations.md` 5·6절 | 화면 캡처 방법, 반복된 함정 |
| `docs/utils/2026-11-03-native-merge-gate.sh` | 병합 게이트 명령 |
| `docs/quality-assurance/2026-11-02-native-lane-git-g1-base.md`·`-g5-diff.md`·`-g7-commit-claude-diff.md` | explorer-search-7·22·23의 Git 쪽 근거 |

### 0.2 egui 0.36 최소 지식 (이 저장소에서 쓰는 형태)

- egui는 immediate mode입니다. 매 프레임 UI 코드를 처음부터 다시 실행해 위젯을 그리고, 위젯을 그리는 호출이 곧 입력 판정(`Response`)을 돌려줍니다. 상태는 앱 구조체나 `ui.memory()`에 둡니다.
- `Response::clicked()`는 주 버튼, `secondary_clicked()`는 보조(오른쪽) 버튼 클릭입니다. `Response::context_menu(|ui| ..)`는 `egui::Popup::context_menu(response)`로 이어지며, 이 저장소에 vendored된 egui(`native/taide-native-app/vendor/egui-input/src/containers/popup.rs` `pub fn context_menu`)는 `response.secondary_clicked()`이면 메뉴를 열고 `response.clicked()`이면 닫습니다. 메뉴 열림 상태는 egui memory에 남습니다.
- 접근성 트리(AccessKit): `context.enable_accesskit()`을 켜면 `FullOutput.platform_output.accesskit_update.nodes`에 위젯마다 노드(label·role·bounds·description)가 나옵니다. 이 저장소의 App 검사는 이 노드의 `label()`과 `bounds()`로 클릭 위치를 찾습니다.
- `egui::RawInput::default()`의 `focused`는 `true`입니다(`vendor/egui-input/src/data/input/raw_input.rs` 106행 `focused: true`). 검사 프레임에 `focused`를 따로 주지 않으면 "창에 포커스가 있음"으로 처리됩니다. OS 알림 게이트(`notifications_only_when_unfocused`)에 영향이 있습니다(3.1절).
- `egui` 크레이트는 `native/taide-native-app/Cargo.toml` `[patch.crates-io] egui = { path = "vendor/egui-input" }`로 저장소 안 사본을 씁니다. egui 동작을 확인할 때는 저장소 밖 cargo registry가 아니라 이 사본을 읽습니다.

### 0.3 Rust 2024·저장소 관례

- 두 앱 crate는 `edition = "2024"`, `rust-version = "1.95"`입니다(`native/taide-native-app/Cargo.toml` 4-5행). `if let Some(x) = a && cond { .. }` 같은 let chain과 `let Some(x) = y else { return; };`(let-else)를 흔히 씁니다.
- 파일 이름은 kebab-case이고 모듈 이름은 snake_case라 `#[path = "search-panel.rs"] mod search_panel;`처럼 `#[path]`로 잇습니다. 단위 검사는 같은 이름 뒤에 `-tests.rs`를 붙인 파일을 `#[cfg(test)] #[path = "..-tests.rs"] mod tests;`로 붙입니다(예: `explorer-git-decorations.rs` 201-203행).
- 실제 App 검사 파일은 `application-<주제>-tests.rs`이고 `application.rs` 끝에서 `#[cfg(test)] #[path = "application-explorer-search-tests.rs"] mod explorer_search_tests;`처럼 선언합니다(`application.rs` 3004-3011행). git 영역 검사는 `domains/git.rs` 끝에서 선언합니다(`domains/git.rs` 끝).
- 검사 함수 이름은 한국어 문장입니다(예: `실제_app_삭제_뒤_부모_갱신이_실패하면_오류_toast로_알린다`). 검사 필터는 이 이름의 앞부분이나 모듈 경로(`application::explorer_search_tests`)를 씁니다.
- 코드 주석은 쓰지 않고 공개 API에만 영어 `///`를 씁니다. 매직넘버는 `const`로 둡니다. 사용자 문자열은 `crates/taide-locale/resources/locales/{ko,en,ja}.json`의 원본과 같은 키를 `crate::presentation::message(locale, "키", &[("이름", 값)])`로 꺼냅니다. 카탈로그 키는 점이 들어간 평면 키입니다(예: `"notification.searchReplaceDone": "파일에서 바꾸기 완료"`, ko.json 476행).
- 금지: GUI 실행, 사용자 데이터·OS 설정·클립보드·Keychain·휴지통, 실제 OS 알림, 합성 OS 입력, `@ts-ignore`류 우회와 `#[allow]`, 검사 기대값 조작(`docs/memory/2026-10-28-native-lane-operations.md` 4절, `docs/utils/2026-11-02-native-lane-workflow.js` `RULES`).

---

## 1. 요약

### 1.1 남은 행과 상태별 수

기능표 explorer-search 절은 45행입니다. 절 머리의 합계 표(기능표 16행 `| explorer-search | 45 | 22 | 7 | 4 | 12 | 0 | 0 |`)는 2026-10-14 기준이라 낡았습니다. 아래는 행을 직접 Grep해 센 값입니다.

| 기준 | complete | partial | unwired | missing | not-applicable | frozen | 남은 행 |
| --- | --: | --: | --: | --: | --: | --: | --- |
| 메인 기능표 | 40 | 4 | 0 | 0 | 1 | 0 | 4행: 16, 20, 21, 33 |

- 실제 남은 구현 작업은 4행입니다. 이 문서는 4행을 3장에서 같은 형식으로 다룹니다.
- explorer-search-7, 22, 23은 complete입니다(`2026-11-03-native-git-lane-merge.md` 3절 (a)).

| ID | 기능 | 남은 것(실제 코드로 재확인) | 막는 요소 |
| --- | --- | --- | --- |
| explorer-search-33 | 검색 교체 | OS 알림 한 줄(`notify_event` 호출)과 그 검사 | 없음. shell lane의 `notify_event`가 이미 병합됨(`domains/shell.rs` 1175행) |
| explorer-search-20 | 행 메뉴 "터미널에서 열기" | 실제 App에서 행 우클릭 → 메뉴 항목 → `finish_explorer` 구간 자동 검사 | 없음 |
| explorer-search-21 | 행 메뉴 "폴더에서 찾기" | 20과 같은 구간 자동 검사 | 없음 |
| explorer-search-16 | 삭제 + 확인 대화상자 | 실제 휴지통을 쓰는 `tests/explorer-delete.rs` 실행 | 사용자 허락 P1 |

### 1.2 현재 구현 위치

| 층 | 경로·심볼 | 역할 |
| --- | --- | --- |
| 앱 영역 진입점 | `native/taide-native-app/src/domains/explorer_search.rs`(2,487줄) | `Command`(30-90행)·`Reply`(99-150행)·`State`(152-172행)·`ExplorerInputs`·`FrameOutput`, host 작업자 `dispatch`(197-583행), `apply_explorer_search_reply`(650행), `apply_search_replaced`(810행), `apply_explorer_search_intent`(882행), `apply_explorer_search_lsp_reply`(1019행), tick 함수들, 사이드바 region `ExplorerRegion`(1581행)과 `impl ExplorerSearchSurfaces for AppSurfaces`(1603-2240행), `show_search_editor_tab`(2271행) |
| 탐색기 위젯 | `src/explorer.rs`(2,226줄) | `Action`(72-94행), `Output`(134행), `Appearance`(166행, `git_color`), `RowIcons`(228행), `Explorer`(451행), `show`/`show_with_icons`/`show_missing`(948-972행), `render_mounted`(1083행, 행 우클릭 1527-1530행, 행 메뉴 1531-1744행) |
| 탐색기 보조 | `explorer-mounts.rs`(mount별 `Explorer`), `explorer-git-decorations.rs`(`Decoration`·`build`·`GitDecorations`), `explorer-auto-reveal.rs`, `explorer_clipboard*.rs`, `explorer_delete.rs`(휴지통 이동 작업), `delete_dialog.rs`(확인 모달), `explorer_move.rs`, `explorer_toolbar.rs`, `explorer_source_missing.rs` | |
| 사이드바 4보기 | `symbol-sidebar.rs`(`View`, `Sidebar{view, search, search_request, open_nonce, outline..}`, `Views`, `request_search` 186행, `window_tree` 152행), `symbol-outline.rs` | |
| 검색 | `search-panel.rs`(`Panel`, `Request`, `Run`, `Replace`, `skip_report`, `is_replacing` 407행, `replace_finished` 412행), `search-results.rs`, `search-history.rs`, `search-stream.rs`, `search-editor.rs` | |
| Problems | `problems.rs`, `problems-icons.rs` | 41-45행(모두 complete) |
| UI crate | `native/taide-native-ui/src/intents/explorer_search.rs`(`Intent` 8변형), `src/surfaces/explorer_search.rs`(`ExplorerSearchSurfaces` trait) | 셸이 영역에 넘기는 의도와 렌더 경계 |
| 공용 crate(수정 금지 대상) | `crates/taide-runtime/src/{search_actions,tree_actions,layout_actions,notification_actions}.rs`, `crates/taide-search`, `crates/taide-file/src/service.rs` `delete_entry`(190행, macOS 휴지통) | crate를 바꾸면 `src-tauri`도 영향을 받으므로 바꿀 일이 생기면 멈추고 메인에 알립니다(사양 2.0.1) |
| Git 쪽 | `src/git-cache.rs`(`GitCache::observe`·`status_result`·`fetched_at`, `Key::Status`), `src/domains/git.rs`(`tick_git_cache`, `compare_tab`) | explorer-search-7의 공급자 |

### 1.3 다른 영역과의 의존·소유 경계

| 필요한 것 | 소유 | 현재 상태 | 영향 행 |
| --- | --- | --- | --- |
| OS 알림 공통 진입점 `NativeApplication::notify_event` | shell(shell-91) | to_rust_native에 있음(`domains/shell.rs` 1175-1198행) | 33 |
| 프로젝트별 git status 캐시 `GitCache` | git(git-64, git-38) | 메인에 있음 | 7(complete) |
| 비교 탭 본문(git-45), 파일 히스토리 dialog(git-53~55) | git | complete | 22, 23 |
| 설정 범주 7종 묶음 알림 검사(settings-23) | 메인 통합 | 33·git-7 연결 뒤 메인이 추가(기능표 settings-23 비고) | 33 완료 뒤 settings-23 비고 갱신 |
| 실제 휴지통 실행 허락 | 사용자 | 미허락(P1) | 16 |

- 결정 D1 A(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` 10행): 탐색기 Git 장식의 파생·행 렌더·주입점은 explorer-search, `git-cache` 공급 연결은 git lane. 결정 D2·D3 A: 파일 히스토리·비교 메뉴는 git lane.
- 결정 2.0.4(사양)와 git 사양이 말하는 `git-status-supply.rs`는 만들어지지 않았습니다. 실제 공개 API는 `GitDecorations::{set, mounted_projects, get}`입니다(`explorer-git-decorations.rs` 149-198행). git QA(`-git-g1-base.md` 55·95행)의 "그 lane의 `git-status-supply.rs`·`GitStatus(project)` host 명령"은 존재하지 않는 대상이니 무시합니다.
- 공용 파일 규칙(`docs/research/2026-10-28-native-lane-seams.md` 1.4절): `application.rs`·`lib.rs`·enum 루트·locale 카탈로그는 영역 블록 끝에 추가만 합니다. lane이 모두 병합된 뒤의 남은 작업은 메인 작업 트리에서 하므로 이 규칙은 충돌 예방 지침으로 따릅니다.

### 1.4 권장 구현 순서

1. explorer-search-33(3.1절): 한 줄 연결 + App 검사. 가장 작고 독립적입니다.
2. explorer-search-20·21(3.3·3.4절): 기존 App 검사의 직접 주입 단계를 실제 행 우클릭 → 메뉴 클릭으로 바꿉니다.
3. explorer-search-16(3.5절): 사용자 허락을 받은 경우에만 실행합니다. 허락이 없으면 partial로 둡니다.
4. 각 단계 뒤 기능표 해당 행·QA 문서를 갱신하고, 마지막에 settings-23 비고(검색 치환 생산자 연결)를 갱신합니다.

---

## 2. 영역 공통 지식

### 2.1 상태가 있는 곳

- 앱 루트 `NativeApplication`의 필드 `explorer_search_state: domains::explorer_search::State`(`domains/explorer_search.rs` 152-172행)에 영역 상태가 모여 있습니다. 주요 필드:
    - `trees: HashMap<ProjectId, TreeRowPage>`: 프로젝트별 트리 page(원본 `TREE.ROWS` 쿼리 캐시와 같은 역할, 프로젝트 공유).
    - `explorers: explorer_mounts::Mounts`: mount(`ShellSlotId`)별 `Explorer`(선택·초안·이름 변경·자동 reveal 기록). 메인 창은 slot id, 보조 창은 그 창의 `explorer` mount id입니다(결정 D6 A).
    - `symbol_sidebars: symbol_sidebar::Views`: mount별 사이드바(보기 Files/Search/Git/Outline, 검색 패널 `Panel`, 검색 요청과 nonce).
    - `tree_failures`, `tree_loading`, `tree_changes`(watcher 무효화 수신함).
    - `git_decorations: explorer_git_decorations::GitDecorations`: 탐색기 Git 장식 입력 저장소(7행).
    - `search_streams`(검색 배치 버퍼), `search_history`(최근 검색 쓰기 큐), `search_editors`(검색 편집기 탭별 패널).
    - `pending_entry_delete: Option<delete_dialog::Confirmation>`, `deleting_entry`: 삭제 확인 모달(16행).
- 영역 바깥 공유 상태 접근은 `self.services.state.settings.read()`(설정), `self.controller.snapshot()`(셸 스냅숏), `self.toasts`(토스트), `self.locale`(문구) 등 경로만 바꿔 씁니다(seams 문서 1.5절).

### 2.2 한 프레임의 흐름

`eframe::App::logic` → `background_tick`(`application.rs` 1071행), 이어서 `eframe::App::ui` → `frame_ui`(1383행) 순서입니다.

1. `background_tick` 안
    - `tick_explorer_search_trees`(1310행 호출): watcher가 기록한 `tree_changes`마다 `HostCommand::ExplorerSearch(Command::RefreshTree{origin: Watcher})`를 제출합니다.
    - `tick_git_title_branches`(1323행 호출) → 그 안에서 `tick_git_cache`가 git 캐시 무효화·요청을 처리합니다. explorer-search-7의 공급 연결 지점입니다(complete, `domains/git.rs`의 `git_decorations.set`).
2. `frame_ui` 안
    - `tick_explorer_search_clipboards`(1398행 호출, 정의 `domains/explorer_search.rs` 1150행): mount 정리(`explorers.reconcile`), 지금 Files 보기로 보이는 프로젝트를 `git_decorations.mount(..)`에 알림(`visible_explorer_files` 1192행), 사라진 검색 패널·검색 편집기 탭의 실행 취소(`CancelSearch`).
    - 셸 렌더가 사이드바 region을 그립니다(아래 2.3).
    - 렌더 중 요청은 `explorer_output: domains::explorer_search::FrameOutput`(186-195행)에 쌓이고 `show_window`가 `load_explorer_search_trees`·`submit_explorer_search_entries`·`queue_explorer_search_delete`로 소비합니다.
3. host 작업자(별도 tokio 작업)는 `HostCommand`를 하나씩 처리합니다(`host.rs` `dispatch` 325행, `HostCommand::ExplorerSearch(command)` arm 336행 → `domains::explorer_search::dispatch`). 오래 걸리는 일(검색 실행·교체·감시 복원)은 `services.tasks.spawn_transient`로 넘기고 바로 반환합니다. 명령·응답 채널 용량은 `HOST_COMMAND_CAPACITY = 64`(`host.rs` 10행)이고 다른 영역과 공유합니다.
4. 응답은 다음 프레임의 `poll()`에서 `HostReply::ExplorerSearch(reply)` → `apply_explorer_search_reply(context, reply)`(`application.rs` 761행)로 반영됩니다. LSP 작업자 응답(생성·이름 변경·붙여넣기·삭제 완료)은 `crate::lsp::Reply`이며 `apply_explorer_search_lsp_reply`(`domains/explorer_search.rs` 1019행)로 갑니다.
5. 셸 의도: UI crate가 만든 `ShellIntent::ExplorerSearch(Intent)`(예: 누락 화면의 "프로젝트 다시 열기", ⌘⇧F)는 `application.rs` 2214행에서 `apply_explorer_search_intent(intent, &context, &snapshot)`로 갑니다.

### 2.3 사이드바 region 규칙 (영역 렌더 경계)

UI crate의 `ExplorerSearchSurfaces`(`native/taide-native-ui/src/surfaces/explorer_search.rs`)는 네 단계입니다.

1. `begin_explorer(&mut self, project, mount) -> ExplorerRegion`: 그 mount의 `Sidebar`와 `Explorer`를 앱 상태에서 **꺼냅니다**(`symbol_sidebars.take`, `explorers.take`, 1610-1639행).
2. `explorer(&self, ui, project, mount, &mut region, &mut intents)`: `&self`라 앱 상태를 읽기만 하고, 그리기와 결과 기록만 합니다. 결과는 `ExplorerRegion` 필드(`actions`, `search_run`, `search_replace`, `auto_reveal`, `window_pane` 등)와 `intents`에 남깁니다(1640-1945행).
3. `finish_explorer(&mut self, context, project, mount, region)`: 꺼낸 상태를 되돌리고 기록된 결과를 `HostCommand`로 바꿔 `self.commands`에 넣습니다(1946-2212행). 탐색기 `Action`의 처리 표는 2084-2207행입니다(예: `Action::OpenInTerminal(row)` 2161행, `Action::FindInFolder(scope_dir)` 2185행).
4. `discard_explorer`: 렌더가 panic이면 결과를 버리고 실행 중 검색을 취소한 뒤 `explorer.remounted()`로 새로 시작합니다(2213-2239행).

이 규칙 때문에 렌더 함수 안에서 `self.explorer_search_state`를 직접 바꾸는 코드를 넣으면 컴파일되지 않거나(빌림 규칙), panic 격리가 깨집니다. 새 요청은 `ExplorerRegion` 필드 또는 `FrameOutput` 필드로 추가합니다.

### 2.4 검사 하네스

#### 2.4.1 UI egui 하네스 (`native/taide-native-ui`)

- 실행: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --features native-host,inspection --test workbench --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`
- `tests/workbench.rs`는 가짜 `Surfaces`(118행 `struct Surfaces`, 152행 `impl ShellSurfaces for Surfaces`)로 실제 `NativeShell::show`를 그립니다. 탐색기 영역 예시(44행 판정 근거, 519행):

```rust
#[test]
fn 보조_창은_같은_프로젝트의_메인_슬롯이_problems를_열어도_problems와_상태바를_그리지_않는다() {
    let mut snapshot = fixture();
    let slot = snapshot.shell.focused.clone().unwrap();
    // ...보조 창 layout 추가...
    let draw = |scope: WindowScope| {
        let shell = shell(scope);
        let mut surfaces = Surfaces {
            problems_open: std::collections::HashSet::from([slot.clone()]),
            ..Default::default()
        };
        let mut output = egui::Context::default().run_ui(
            egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(WINDOW_SIZE[0], WINDOW_SIZE[1]))), ..Default::default() },
            |ui| { shell.show(ui, &snapshot, &mut surfaces); },
        );
        // ...텍스트 shape 수집...
    };
    // main은 problems 1회 호출, Auxiliary는 0회를 단언
}
```

- 남은 4행은 UI crate를 바꾸지 않으므로 UI 하네스 검사는 회귀 확인용으로만 씁니다.

#### 2.4.2 앱 lib 실제 App 검사 (`native/taide-native-app`, `--lib`)

실제 `NativeApplication::new`와 합성 임시 프로젝트로 `logic` + `ui`를 돌립니다. 이 영역이 쓰는 fixture와 helper:

| helper | 위치 | 쓰임 |
| --- | --- | --- |
| `Fixture{directory, application}` | `application-project-rail-tests.rs` 15행 | 임시 디렉터리와 App. drop 때 `on_exit` 후 디렉터리 삭제 |
| `isolated_state(data)` | 같은 파일 80행 | 합성 데이터 디렉터리, IDE 서버·agent hooks 끔, 설정 파일 기록 |
| `insert_project(state, id, root, name, root_missing)` / `project_ref(..)` | 95행 / 69행 | 프로젝트·기본 layout 삽입 |
| `launch(fixture, state) -> egui::Context` | 120행 | `NativeApplication::new` + `context.enable_accesskit()` |
| `step(app, context, events) -> FullOutput` | 142행 | `logic` + `paint`(한 프레임) |
| `pointer(pos, button) -> Vec<Event>` | 192행 | 같은 프레임에 이동·누름·뗌 |
| `node_rect(output, label, within)` | 153행 | 접근성 label이 같은 첫 노드의 rect |
| `step_until(app, context, phase, ready)` | `application-auxiliary-window-tests.rs` 108행 | 조건이 될 때까지 프레임 반복(10초 한도) |
| `Launched{fixture, context, project, slot, root}` / `launch_project(name, files, with_auxiliary)` | `application-search-panel-tests.rs` 25·33행 | 파일을 쓴 한 프로젝트·한 slot·사이드바 펼침으로 기동 |
| `click(app, context, rect)`, `role_rect(output, role, label)`, `message(app, key, args)`, `texts(output)`, `settle_pointer(app, context, at)` | 같은 파일 121·92·113·81·632행 | 클릭(포인터를 먼저 옮김), role로 구분, 문구, 텍스트 shape, tooltip 닫힘 대기(40프레임) |
| `FakePlatform{sent}` / `launch_with_platform(fixture, state, platform)` / `frame(app, context, focused)` / `frame_until(..)` | `application-notifications-tests.rs` 25·55·84·102행(모듈 `domains::shell::notifications_tests`, `#[cfg(all(test, unix))]`, 지금은 `pub(super)`) | OS 알림을 기록하는 가짜 플랫폼 |
| git 영역: `Workspace`, `frame`, `until`, `nodes`, `node`, `settle`, `text`, `has_text` | `application-git-base-tests.rs` 40·118·136·185·208·153·164·181행(모듈 `domains::git::base_tests`) | git 저장소 fixture와 프레임 helper |
| git 영역: `fixture::{git, init, repository}` | `application-git-fixture.rs` | 전역 설정을 끈 git CLI로 합성 저장소 생성(결정 문서 15행이 허용) |

실제 예시(explorer-search-20·21의 현재 App 검사, `application-explorer-search-tests.rs` 572행):

```rust
let Launched { mut fixture, context, project, slot, root } =
    launch_project("explorer-menu", &MENU_FILES, false);
let application = fixture.application.as_mut().unwrap();
application.services.state.settings.write().search_on_type = false;
step_until(application, &context, "tree", |_, output| {
    node_rect(output, IGNORED_FOLDER, egui::Rect::EVERYTHING).is_some()
});
```

실행(메인 작업 트리, 게이트 스크립트와 같은 target):

1. LSP mock 예제를 먼저 빌드합니다(없으면 LSP 검사 10건 정도가 실패합니다, 운영 문서 6절): `cargo build --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`
2. 영역 필터 실행: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- --test-threads=2 search explorer symbol_ navigation command_dispatch shell_keymap application::tab_menu_tests application::auxiliary_window_tests application::explorer_views_tests application::error_boundary_tests application::context_menu_tests toast tooltip_controlled breadcrumb`
   (묶음 C 리뷰 반영 때 이 필터로 251개 통과, QA `-search-entry-editor.md` 120행)
3. 실제 App 검사가 부하로 시간 초과하면 그 검사만 단독으로 1회 다시 돌려 흔들림과 결함을 구분합니다.

#### 2.4.3 통합 검사 (`native/taide-native-app/tests/`)

- `tests/explorer.rs`: `Explorer`를 직접 그리는 하네스. `menu_frame`(41행), `menu_click`(66행, **누름과 뗌을 서로 다른 프레임에 보냄**), `open_row_menu`(2962행), `separators_between`(2982행). 행 메뉴 순서·구분선 검사는 `행_메뉴는_원본_구분선_구조로_터미널에서_열기와_폴더에서_찾기를_두고_그_동작을_낸다`(2999행).
- `tests/explorer-delete.rs`: 4건 중 `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다`(401행)가 실제 macOS 휴지통을 씁니다. **허락 없이 실행 금지**(3.5절).
- 실행: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --test explorer --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- --test-threads=2`

#### 2.4.4 순수 단위 검사

- `explorer-git-decorations-tests.rs`(원본 9사례 + 저장소 1), `explorer-tests.rs`(`행_바탕은_hover_primary_focus_선택_순서로_탐색기_토큰을_칠한다` 139행, `git_데코레이션은_이름_chevron_색과_배지와_디렉터리_점과_접근성_설명을_그린다` 194행), `search-panel-tests.rs`, `search-results-tests.rs`, `search-history-tests.rs`, `search-editor-tests.rs`, `explorer-auto-reveal-tests.rs`.
- 필터: `-- --test-threads=2 explorer_git_decorations explorer::tests search_panel::tests`

#### 2.4.5 마무리 검증

- `cargo check --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --all-targets --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`(기존 경고: `zen.rs` unused mut, `editor-code-actions.rs` `is_applying`, inlay 검사 `fixture_with_emmet_peek`, vendor `wry-preview`)
- `cargo fmt --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml -- --check`
- UI crate를 바꿨다면 remote-web Wasm check(`docs/utils/2026-11-03-native-merge-gate.sh` 17행). 남은 4행은 UI crate를 바꾸지 않을 것으로 봅니다.
- 병합 단위 전체 게이트는 `docs/utils/2026-11-03-native-merge-gate.sh <로그 디렉터리>`(메인이 실행). 알려진 기존 실패: 앱 lib `editor_fonts` 1건, `crates/taide-runtime/tests/agent_probe.rs` 소스 문자열 단언 1건(PROCESS.md 상단).

---

## 3. 행별 상세

### 3.1 explorer-search-33 교체

#### (a) ID·기능·상태

- explorer-search-33 · 교체(토글, 입력, 파일별 체크, 확인 대화상자, 질의 일치 전에는 비활성+안내, 결과 요약 토스트, 건너뜀 보고, 네이티브 알림, 교체 후 재검색) · **partial**(기능표 411행)

#### (b) 원본 TS 동작

- 바꾸기 줄 `src/features/search/search-panel.tsx`
    - 108-117행: `replaceOpen`·`replaceText` 상태, 새 nonce의 `openReplace`면 열기만.
    - 123행: `canReplaceAll = replaceOpen && selectedGroups.length > 0 && !isReplacing && queryMatchesResults`.
    - 130-131행: 확정 시 `onReplaceAll({ replacement, paths: 선택 그룹 경로 })`.
    - 175-196행: 교체 입력(placeholder `search.replacePlaceholder`), 24px `ReplaceAll` 버튼(tooltip `search.replaceAll`, 비활성이면 불투명도 0.4), 결과가 있고 질의가 다르면 `search.replaceStaleHint`.
    - 268-281행: AlertDialog(작은 크기) `search.replaceConfirmTitle`, `search.replaceConfirmDescription{files, matches}`, `common.cancel`, `common.confirm`.
- 실행 `src/widgets/search-panel/search-panel-container.tsx` 124-162행 `handleReplaceAll`
    - 입력이 아니라 표시 결과를 만든 질의(`ranQuery`)로 교체합니다.
    - `onSuccess`: 순서대로
        1. `toast.success(t('search.replaceDone', { files: changedFiles, matches: replacedMatches }))`
        2. `notifyNative({ category: 'searchReplace', title: resolveProjectNotificationTitle({ projects, projectId, fallbackTitle: t('notification.searchReplaceDone') }), body: replaceSummary })` (body는 1의 요약 문자열 그대로)
        3. 건너뜀이 있으면 `toast.warning(t('search.replaceSkipped', { count }))`, 설명은 앞 5개 `경로 — 사유`와 `search.replaceSkippedMore{count}`(`src/entities/search/replace-skip-report.ts` 8-33행)
        4. `run(replacedQuery, { recordHistory: false })`
    - `onError`: `toast.error(describeIpcError(error))`만. 알림 없음.
    - mutate 단위 콜백이라 패널이 내려간(unmount) 뒤에는 위 콜백이 실행되지 않습니다.
- 알림 게이트(원본·native 공통 backend): `crates/taide-notification/src/service.rs` `decide_delivery`(17-28행) — `notifications_enabled` 꺼짐 → 억제, 범주 스위치(`notify_search_replace`) 꺼짐 → 억제, `notifications_only_when_unfocused && any_window_focused` → 억제.
- locale 키: `search.replaceToggle`, `search.replacePlaceholder`, `search.replaceAll`, `search.replaceStaleHint`, `search.replaceConfirmTitle`, `search.replaceConfirmDescription`, `search.replaceDone`, `search.replaceSkipped`, `search.replaceSkippedMore`, `search.replaceSkipReason.{tooLarge,binary,notUtf8,unreadable,writeFailed}`, `notification.searchReplaceDone`, `common.cancel`, `common.confirm`.

#### (c) native 현재 자산과 빠진 것

- 있음(QA `-search-entry-editor.md` 35-39행, 68행):
    - `search-panel.rs`: 바꾸기 줄·체크·확인 모달·`is_same_query`·`skip_report`·`is_replacing`·`replace_finished`.
    - `domains/explorer_search.rs`: `Command::ReplaceSearch` → `spawn_transient` → `search_actions::search_replace` → `Reply::SearchReplaced`(470-513행), `apply_search_replaced`(810-865행: 바꾸기를 시작한 패널에만 적용, 실패 `report`, 성공 `toasts.success(summary, now)`(844행), 건너뜀 warning, 같은 질의 재검색).
    - 실제 App 검사 `실제_app_모두_바꾸기는_확인한_파일만_바꾸고_요약과_건너뜀_toast를_띄운_뒤_같은_질의로_다시_검색한다`(`application-search-panel-tests.rs` 643행), `실제_app_바꾸기를_시작하지_않은_새_패널은_늦게_온_바꾸기_응답을_무시한다`(1193행).
- 빠진 것(코드로 재확인): `apply_search_replaced` 성공 분기에 OS 알림 호출이 없습니다. `grep -n "notify_event(" native/taide-native-app/src` 결과는 `domains/shell.rs`의 3곳과 git pull/push 알림(`domains/git.rs` `notify_git_remote`)뿐이고 `domains/explorer_search.rs`에는 없습니다. 묶음 C 당시 이 worktree에 `notify_event`가 없어 넣지 않았고(QA 77행), 지금은 shell lane 병합으로 `NativeApplication::notify_event(&mut self, context, category, project: Option<&ProjectId>, fallback_title_key: &str, body: String)`이 있습니다(`domains/shell.rs` 1175-1198행). 이 함수는 원본 `resolveProjectNotificationTitle`과 같은 제목 규칙(`native-notifications.rs` `project_title` 225행: 표시 라벨 → 이름 → 대체 키)과 200 코드 포인트 자르기를 적용해 `HostCommand::Shell(Command::Notify(..))`를 제출합니다.

#### (d) 구현 방법

1. `git -C /Users/hyunseokbyun/development/TAIDE log --oneline -1`이 `7be1ad14` 이후인지 확인합니다(git lane 병합 포함).
2. `native/taide-native-app/src/domains/explorer_search.rs`의 `apply_search_replaced`에서 성공 요약 toast 줄을 찾습니다(`grep -n "self.toasts.success(summary, now);"`). 그 줄을 다음으로 바꿉니다. 순서는 원본처럼 success toast → OS 알림 → 건너뜀 warning → 재검색입니다.

```rust
self.toasts.success(summary.clone(), now);
self.notify_event(
    context,
    taide_model::notification::NotificationCategory::SearchReplace,
    Some(&project),
    "notification.searchReplaceDone",
    summary,
);
```

   - 빌림 확인: 이 위치에서 `panel`(= `self.explorer_search_state`의 가변 빌림)은 바로 위 `panel.replace_finished(..)`가 마지막 사용이라 끝났으므로 `&mut self` 메서드를 부를 수 있습니다. 컴파일이 빌림 오류로 실패하면 `let run = panel.replace_finished(..)` 이후에 `panel`·`state`를 쓰는 줄이 남았는지 확인합니다.
   - 실패 분기(`Err(error) => { ..; self.report(&error); return; }`)와 패널이 없는 조기 반환에는 넣지 않습니다(원본 `onError`·unmount와 같음).
3. 검사에서 가짜 플랫폼을 재사용하기 위해 가시성을 올립니다(기본안). 이유: settings-23 범주 묶음 검사도 같은 가짜 플랫폼이 필요해 2회 이상 쓰입니다.
    - `native/taide-native-app/src/domains/shell.rs` 1641-1643행 `#[cfg(all(test, unix))] #[path = "../application-notifications-tests.rs"] mod notifications_tests;` → `pub(crate) mod notifications_tests;`
    - `native/taide-native-app/src/application-notifications-tests.rs`: `pub(super) struct FakePlatform`(25행) → `pub(crate)`, `pub(super) fn sent`(31행) → `pub(crate)`, `pub(super) fn launch_with_platform`(55행) → `pub(crate)`.
    - 대안(가시성을 바꾸지 않을 때): `application-settings-plugins-tests.rs` 263-277행처럼 검사 함수 안에 기록용 `PlatformServices` 구현을 둡니다. shell 소유 파일을 건드리지 않아야 하는 상황(예: shell 쪽 작업과 동시 진행)에서만 이 안을 택합니다.
4. `application-search-panel-tests.rs`의 `launch_project`(33-79행)에서 상태 구성 부분을 `pub(super) fn project_state(name, files, with_auxiliary) -> (Fixture, AppState, ProjectId, ShellSlotId, std::path::PathBuf)`로 꺼내고, `launch_project`는 `project_state` 뒤 `launch`를 부르게 바꿉니다(동작 불변). 새 검사가 같은 상태를 `launch_with_platform`으로 띄우기 위해서입니다.

#### (e) 자동 검사

- 위치: `native/taide-native-app/src/application-search-panel-tests.rs`에 새 검사 1건. `notifications_tests`가 unix 전용이므로 검사에 `#[cfg(unix)]`를 붙입니다.
- 이름 예: `실제_app_모두_바꾸기_성공은_프로젝트_제목과_요약으로_os_알림을_한_번_보낸다`
- 절차
    1. `let (mut fixture, state, project, slot, root) = project_state("search-replace-notify", &MAIN_FILES[..2], false);`
    2. 설정: `state.settings.write()`에서 `notifications_enabled = true`, `notify_search_replace = true`, `notifications_only_when_unfocused = false`, `search_on_type = false`. `only_when_unfocused`를 끄는 이유는 0.2절(검사 프레임의 `focused` 기본값이 true라 켜 두면 `WindowFocused`로 억제됨)입니다.
    3. `let platform = Arc::new(FakePlatform::default()); let context = launch_with_platform(&mut fixture, state, platform.clone()); context.enable_accesskit();` (`launch_with_platform`은 accesskit을 켜지 않으므로 직접 켭니다. `node_rect`·`role_rect`가 accesskit 노드를 읽습니다.)
    4. 기존 643행 검사와 같은 순서로: tree 대기 → `open_search_view` → `search(application, &context, NEEDLE)` → 결과 `search.matchCount{count: 3, files: 2}` 대기 → 바꾸기 전환 클릭(`settle_pointer` 먼저) → 교체 입력 클릭·`Event::Text(REPLACEMENT)` → `search.replaceAll` 클릭 → `common.confirm` 버튼(`role_rect(.., Role::Button, ..)`) 클릭.
    5. `step_until(.., "notification", |_, _| !platform.sent().is_empty())`
    6. 단언: `platform.sent() == vec![("alpha".to_owned(), message(application, "search.replaceDone", &[("files", "2"), ("matches", "3")]))]`. 근거: `MAIN_FILES[..2]`의 `a.txt`에 needle 1건, `b.txt`에 2건, 둘 다 체크된 상태로 교체. 제목 "alpha"는 기본 표시의 프로젝트 이름(알림 검사 `application-notifications-tests.rs` 237행의 "alpha"와 같은 규칙).
    7. 첫 전달 안내 toast(`notification.enableHint`, `domains/shell.rs` 648-656행)가 1개 뜨는지 `toasts.inspection()`으로 확인합니다(같은 진입점을 쓴다는 근거).
- 선택 보강(완료 판정에는 불필요): 늦은 응답 무시 검사(1193행)를 가짜 플랫폼으로 띄우고 `platform.sent().is_empty()`를 더합니다.
- 실행: 2.4.2절 1번 뒤 `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- --test-threads=2 search_panel_tests notifications_tests`, 이어서 2.4.5절.

#### (f) 완료 판정

- 새 검사와 기존 `search_panel_tests`·`notifications_tests` 통과, `cargo check --all-targets`·fmt 통과.
- 기능표 explorer-search-33 → complete(비고에 `notify_event` 연결·가짜 플랫폼 검사 이름·실제 OS 표시는 P2 실기). settings-23 비고에서 "검색 치환(explorer-search lane) 범주 연결"을 완료로 갱신(행 상태는 git-7과 묶음 검사가 끝날 때 메인이 판정).
- QA 문서 하나(`docs/quality-assurance/<날짜>-native-explorer-search-33-notify.md` 등)에 실행 명령과 통과 수를 적습니다.

#### (g) 막는 요소

- 실제 OS 알림 센터 표시는 자동 검사로 보지 않습니다(허락 P2, 배치 31 실기). 가짜 플랫폼 기록까지가 자동 검사 범위입니다.

---

### 3.2 explorer-search-7 Git 데코레이션

explorer-search-7 complete(`tick_git_cache`가 `GitDecorations::set` 공급, 검사 `실제_app_탐색기_git_장식은_git_cache_status로_파일_배지와_상위_폴더를_그리고_git_변경_뒤_갱신한다`) — 작업 없음.

---

### 3.3 explorer-search-20 컨텍스트 메뉴: 터미널에서 열기

#### (a) ID·기능·상태

- explorer-search-20 · 컨텍스트 메뉴: 터미널에서 열기 · **partial**(기능표 398행)

#### (b) 원본 TS 동작

- 메뉴 `src/features/explorer/file-tree-context-menu.tsx` 147-165행: 행이 있으면(파일·폴더 모두) 구분선 → "Finder에서 보기"(`explorer.revealInFinder`) → "터미널에서 열기"(`explorer.openInTerminal`, 16px `SquareTerminal`) → 폴더면 "폴더에서 찾기". 전체 구조는 96-216행(새 파일·새 폴더 / [파일] 구분선, 나란히 열기, 열기 방식, 브라우저에서 열기, 비교 대상 선택, 선택 항목과 비교, 파일 히스토리 / [행] 구분선, Finder, 터미널, [폴더] 폴더에서 찾기 / 구분선, 잘라내기, 복사, 붙여넣기, 경로 복사, 상대 경로 복사 / [행] 구분선, 이름 바꾸기, 삭제).
- 실행 `src/widgets/explorer/explorer-container.tsx` 160-167행 `openInTerminal`: `resolveTargetDir`(`src/widgets/explorer/explorer-path.ts` 24-27행: 디렉터리는 자기, 파일은 부모 `parentDirOf`, page에 없는 행이면 root, root도 없으면 무동작)로 cwd를 정하고 `openTab({ kind: { kind: 'terminal', sessionId: '', cwd: dir }, title: t('terminal.title'), target: null, preview: false }, { onError: notifyError })`. `target: null`은 그 창의 focused pane입니다.
- locale: `explorer.openInTerminal`(ko "통합 터미널에서 열기", ko.json 251행), `terminal.title`.

#### (c) native 현재 자산과 빠진 것

- 있음:
    - 메뉴 항목 `explorer.rs` 1637-1646행(행 묶음 안 Finder 다음, 항목 응답을 `output.menu["explorer.openInTerminal"]`에 기록), 원본 구분선 구조(1548·1623·1661·1719행 `ui.separator()`).
    - 동작 `finish_explorer`의 `Action::OpenInTerminal(row)`(`domains/explorer_search.rs` 2161-2184행): 폴더면 자기 경로, 파일이면 `Path::parent()`, 없으면 프로젝트 root, `Command::OpenTerminalAt{project, pane: window_pane, cwd, title: terminal.title}`. host `dispatch`(529-546행)가 `layout_open_tab(.., TabKind::Terminal{session_id: "", cwd: Some(cwd)}, title, pane, false)`.
    - 검사: 통합 `tests/explorer.rs` 2999행(메뉴 순서·구분선·`Action::OpenInTerminal`), App `실제_app_터미널에서_열기는_그_폴더를_cwd로_한_고정_터미널_탭을_열고_폴더에서_찾기_요청은_기본_제외_폴더_안만_검색한다`(`application-explorer-search-tests.rs` 572행). 이 App 검사는 메뉴를 거치지 않고 `application.submit(HostCommand::ExplorerSearch(Command::OpenTerminalAt{..}))`를 직접 제출합니다(592-599행).
- 빠진 것: 실제 App에서 "행 우클릭 → 메뉴 표시 → 항목 클릭 → `finish_explorer` → host → 터미널 탭"을 한 검사로 잇는 것. QA(`-search-entry-editor.md` 85행)는 App 하네스로 행을 우클릭했을 때 `egui::Popup::is_any_open`이 false였고 원인을 확인하지 못했다고 적었습니다.
- 새 근거(이 문서 작성 중 확인): 메인의 git 영역 실제 App 검사 두 건이 같은 하네스로 탐색기 행 메뉴를 열고 항목을 눌렀습니다.
    - `application-git-diff-tests.rs` `실제_app_탐색기_비교_메뉴는_선택한_파일과_대상_파일의_preview_비교_탭을_연다`(380행, helper 391행): `sidebar_row`(같은 label 노드 중 `left()`가 가장 작은 rect) → `frame(pointer(rect.center(), Secondary))` → `until(.., |_, output| node(output, item).is_some())`로 메뉴 항목 노드가 나타날 때까지 프레임 반복 → `frame(pointer(item.center(), Primary))` → `settle`.
    - `application-git-claude-history-tests.rs` `실제_app_파일_이력은_명령_탭_메뉴로_열리고_항목을_inline_diff로_보여준다`(466행, 탐색기 단계 629-670행): 같은 방식.
  따라서 메뉴는 App 하네스에서도 열립니다. explorer-search lane 시도가 실패한 원인은 기록이 없어 확정할 수 없지만, 같은 이름 노드가 여러 개일 때 `node_rect(.., Rect::EVERYTHING)`가 첫 노드(탭 제목·breadcrumb·행 안의 Label 등)를 고를 수 있다는 점과, 우클릭 직후 한 프레임만 보고 판정했을 가능성이 차이입니다.

#### (d) 구현 방법

제품 코드 변경은 예상하지 않습니다. 검사를 실제 경로로 바꿉니다.

1. 병합은 완료됐으므로 메인 트리에서 바로 작업합니다(메뉴에 비교·히스토리 항목이 이미 더해져 있습니다).
2. `application-explorer-search-tests.rs`에 지역 helper를 둡니다(git 검사 helper는 `domains::git::base_tests`의 비공개 모듈이라 가져올 수 없습니다).

```rust
fn sidebar_node(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(label))
        .filter_map(|(_, node)| node.bounds())
        .map(|bounds| {
            egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
        .min_by(|left, right| left.left().total_cmp(&right.left()))
}

fn choose_row_menu(
    application: &mut NativeApplication,
    context: &egui::Context,
    row: &str,
    item: &str,
) {
    let output = step_until(application, context, row, |_, output| {
        sidebar_node(output, row).is_some()
    });
    step(
        application,
        context,
        pointer(sidebar_node(&output, row).unwrap().center(), egui::PointerButton::Secondary),
    );
    let output = step_until(application, context, item, |_, output| {
        node_rect(output, item, egui::Rect::EVERYTHING).is_some()
    });
    step(
        application,
        context,
        pointer(
            node_rect(&output, item, egui::Rect::EVERYTHING).unwrap().center(),
            egui::PointerButton::Primary,
        ),
    );
}
```

3. 572행 검사의 `application.submit(HostCommand::ExplorerSearch(Command::OpenTerminalAt{..}))` 단계를 `choose_row_menu(application, &context, IGNORED_FOLDER, &message(application, "explorer.openInTerminal", &[]))`로 바꾸고, 기존 대기 조건(`terminal_cwds`가 하나 늘고 활성 탭이 고정·제목 `terminal.title`·`cwd == root/node_modules`)을 그대로 둡니다.
4. 파일 사례를 더합니다: `choose_row_menu(.., "note.txt", ..)` 뒤 새 터미널 탭의 cwd가 프로젝트 root(`root.to_str()`). `launch_project`의 root는 정규화하지 않은 경로이고 트리 행 경로도 같은 root에서 만들어지므로 그대로 비교합니다.
5. 메뉴가 열리지 않으면 아래 순서로 진단합니다(같은 가정으로 세 번 실패하면 멈추고 보고).
    1. 우클릭 프레임 뒤 `application.explorer_search_state.explorers.get(&slot).and_then(|explorer| explorer.selected.clone())`가 그 행 경로인지 확인합니다. 행 우클릭은 `select_only`를 부르므로(`explorer.rs` 1527-1530행) 아니면 클릭이 행에 닿지 않은 것입니다(다른 노드를 고름, tooltip이 가로챔). tooltip이면 우클릭 전에 `settle_pointer(application, &context, rect.center())`를 씁니다.
    2. 선택은 됐는데 메뉴가 없으면 `egui::Popup::is_any_open(&context)`를 몇 프레임 동안 기록합니다. 열렸다가 닫히면 같은 프레임의 다른 입력(포커스 요청 등)이 닫는 것이고, 한 번도 안 열리면 `secondary_clicked()`가 거짓인 것입니다. 후자면 `tests/explorer.rs` `menu_click`처럼 누름과 뗌을 두 프레임에 나눠 보냅니다.

#### (e) 자동 검사

- 위 3·4번의 App 검사(기존 검사 수정)와 기존 `tests/explorer.rs` 2999행.
- 실행: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- --test-threads=2 application::explorer_search_tests`, `--test explorer -- --test-threads=2`, 2.4.5절.
- 단언 목록: 폴더 → 고정(`!preview`) Terminal 탭, `session_id` 빈 문자열, `cwd == <root>/node_modules`, 제목 `terminal.title`, 그 pane 활성. 파일 → cwd가 root.

#### (f) 완료 판정

- 메뉴를 거치는 App 검사 통과. 기능표 20 → complete(비고: 검사 이름, 메뉴 아이콘 없음은 결정 D10 A로 19의 기존 차이).

#### (g) 막는 요소

- 없음. 터미널 탭은 기존 572행 검사와 같은 경로로 열리므로 새 허락이 필요 없습니다.

---

### 3.4 explorer-search-21 컨텍스트 메뉴: 폴더에서 찾기

#### (a) ID·기능·상태

- explorer-search-21 · 컨텍스트 메뉴: 폴더에서 찾기 · **partial**(기능표 399행)

#### (b) 원본 TS 동작

- 메뉴 `file-tree-context-menu.tsx` 159-164행: 디렉터리만, `explorer.findInFolder`(ko "폴더에서 찾기", 247행), 16px `Search` 아이콘.
- 실행 `explorer-container.tsx` 169-180행 `findInFolder`: 디렉터리이고 프로젝트가 있을 때 `requestOpenSearchPanel({ scopeDir: toRelativePath(project.root, row.path) })`. glob이 아니라 `scopeDir`라 기본 제외 폴더(`node_modules` 등) 안도 검색합니다(같은 함수 주석).
- 수신 `src/widgets/explorer/explorer-panel.tsx` 131-140행: 보기를 search로 바꾸고 요청을 대체하며 nonce를 올립니다. 범위 칩 문구 `explorer.searchScopeLabel{path}`(ko "폴더: {{path}}"), 해제 `search.clearScope`.

#### (c) native 현재 자산과 빠진 것

- 있음: 메뉴 항목 `explorer.rs` 1647-1660행(폴더만, `relative_path`로 상대 경로), `finish_explorer` `Action::FindInFolder(scope_dir)`(2185-2191행) → `symbol_sidebars.entry(project, &slot).request_search(Request{scope_dir: Some(..), ..})`(`symbol-sidebar.rs` 186-190행: 보기 Search, 요청 대체, nonce 증가). 검사: `tests/explorer.rs` 2999행(폴더만, `FindInFolder("node_modules")`), App 572행 검사 후반(요청을 `request_search`로 직접 넣은 뒤 범위 칩·`node_modules` 안 1건·칩 해제 뒤 기본 제외 재검색).
- 빠진 것: 20과 같이 실제 행 우클릭 → 메뉴 → `finish_explorer` 구간. 572행 검사는 `.symbol_sidebars.entry(..).request_search(..)`를 직접 호출합니다(618-625행).

#### (d) 구현 방법

1. 3.3절 (d)의 helper를 같이 씁니다.
2. 572행 검사의 `application.explorer_search_state.symbol_sidebars.entry(&project, &slot).request_search(..)` 단계를 `choose_row_menu(application, &context, IGNORED_FOLDER, &message(application, "explorer.findInFolder", &[]))`로 바꿉니다. 뒤 단계(보기 Search·범위 칩 대기, "needle" 입력·Enter, `search.matchCount{1,1}`·`inner.txt`만, 칩 해제 뒤 재검색은 `note.txt`만)는 그대로 둡니다.
3. 메뉴는 그 행이 있는 mount의 사이드바에 요청을 넣으므로 `symbol_sidebars.view(&project, &slot) == View::Search`를 추가로 단언합니다.

#### (e) 자동 검사

- 3.3절과 같은 검사 함수(한 검사에서 20·21을 함께 다룹니다)와 같은 실행 명령.

#### (f) 완료 판정

- 메뉴를 거치는 App 검사 통과. 기능표 21 → complete.

#### (g) 막는 요소

- 없음.

---

### 3.5 explorer-search-16 삭제와 확인 대화상자

#### (a) ID·기능·상태

- explorer-search-16 · 삭제(⌘⌫, 메뉴) + 확인 대화상자 · **partial**(기능표 394행)

#### (b) 원본 TS 동작

- 대화상자 `src/features/explorer/entry-delete-dialog.tsx`: AlertDialog, 제목 `explorer.deleteConfirmTitle{name}`(ko "{{name}} 을(를) 삭제할까요?"), 설명 `explorer.deleteConfirmDescription{name}`(ko "{{name}} 을(를) 휴지통으로 이동합니다."), `common.cancel`, 파괴 버튼 `explorer.delete`. Radix Action이라 확정 순간 닫히고, `onOpenChange(false)`(Escape·배경)는 취소입니다.
- 실행 `src/widgets/explorer/use-explorer-entry-crud.ts` 180-190행 `confirmDelete`: `deleteEntryAsync(path)` 뒤 `refreshTreeDir({ projectId, dir: parentDir })`까지 성공하면 대상 해제, 둘 중 실패는 `notifyError`(`toast.error(describeIpcError(error))`).
- 백엔드 삭제는 macOS 휴지통 이동입니다(복구 가능).

#### (c) native 현재 자산과 빠진 것

- 있음
    - 진입: ⌘⌫(`explorer.rs` 2015-2017행)·행 메뉴(1731-1743행) → `Action::RequestDelete(row)` → `FrameOutput.delete_entries` → `queue_explorer_search_delete`(1357행) → `delete_dialog::Confirmation`(취소 버튼 첫 focus, Escape 취소, 모달) → `show_explorer_search_delete_dialog`(1367-1396행)에서 확정이면 `flush_dirty()` 뒤 `LspBridge::delete_entry`(`lsp.rs` 1239행) → `explorer_delete::apply`(project root 안인지 `root_guard`로 확인, root 자체·밖 거부, 문서 준비·회수, 공유 초안 보존) → `taide_file::service::delete_entry`(휴지통) → `lsp::Reply::ExplorerDeleteFinished` → `apply_explorer_search_lsp_reply`(1096-1121행)가 부모를 `RefreshTree{origin: Delete}`로 갱신 → 실패면 `report`(toast, 700-711행).
    - 검사: App `실제_app_삭제_뒤_부모_갱신이_실패하면_오류_toast로_알린다`(`application-explorer-search-tests.rs` 689행, 휴지통을 쓰지 않도록 `ExplorerDeleteFinished{result: Ok}`를 직접 주입), 통합 `tests/explorer-delete.rs` 4건.
- 빠진 것: `tests/explorer-delete.rs`의 현재 코드 기준 실행 기록. 4건 중 실제 휴지통을 쓰는 것은 `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다`(401행, 합성 파일 `delete-synthetic.txt` 1개가 사용자 휴지통으로 이동) 1건입니다. 나머지 3건(`탐색기_삭제확인은_..` 94행, `삭제_ack는_..` 212행, `삭제후_공유초안_save_as는_..` 305행)은 휴지통을 쓰지 않지만, 운영 규칙이 파일 단위로 실행을 금지하고 있습니다(`2026-11-02-native-lane-workflow.js` `RULES`, 운영 문서 4절).
- `cargo check --all-targets`가 통과했으므로 이 파일은 현재 코드와 함께 컴파일됩니다(QA `-search-entry-editor.md` 121행).

#### (d) 구현 방법

코드 변경은 없습니다.

1. 사용자에게 허락을 받습니다. 질문 예: "explorer-search-16 완료를 위해 `tests/explorer-delete.rs`를 실행해도 될까요? 합성 파일 `delete-synthetic.txt` 1개가 macOS 휴지통으로 이동합니다(복구 가능). A. 허락 / B. 보류(16은 partial 유지)".
2. 허락을 받으면 휴지통을 먼저 확인하라고 사용자에게 알립니다. 2026-10-27에 작업자가 금지를 어기고 이 파일을 2회 실행해 같은 이름의 파일이 이미 휴지통에 있을 수 있습니다(PROCESS.md 배치 24 D 항목). 휴지통은 에이전트가 건드리지 않습니다.
3. 실행: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --test explorer-delete --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- --test-threads=1`

#### (e) 자동 검사

- 위 명령의 4건 통과. 단언 내용(401행 검사): 삭제 준비 이벤트에서 dirty 문서 스냅숏, 삭제 이벤트에서 원본 파일 없음·선택 프로젝트 탭만 닫힘·다른 프로젝트 탭 유지·공유 초안 mirror `source_missing`, 완료 응답, 프로젝트 root와 프로젝트 밖 경로 삭제 거부, 작업 수 0.

#### (f) 완료 판정

- 허락 뒤 4건 통과 → 기능표 16 → complete(비고: 실행 날짜·명령·통과 수, 휴지통 이동 1건).
- 허락이 없으면 partial 유지, 비고 "휴지통 검사 미실행(허락 P1)" 유지.

#### (g) 막는 요소

- 사용자 허락 P1(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` 27행). 허락 없이 실행하지 않습니다. 휴지통 이동을 주입 포트로 바꾸는 우회는 `crates/taide-file` 변경이라 사양 범위 밖이며, 하더라도 "실제 휴지통 경로" 검증을 대신하지 못합니다.

---

### 3.6 explorer-search-22 컨텍스트 메뉴: 비교 대상 선택 / 선택 항목과 비교

explorer-search-22 complete(`2026-11-03-native-git-lane-merge.md` 5절) — 작업 없음.

### 3.7 explorer-search-23 컨텍스트 메뉴: 파일 히스토리

explorer-search-23 complete(`2026-11-03-native-git-lane-merge.md` 5절) — 작업 없음.

---

## 4. 이 영역에서 반복된 함정과 주의

### 4.1 검사 하네스

- 같은 이름 노드: 영어 locale에서 Search 사이드바 탭(`search.title`)과 검색 입력 placeholder(`search.placeholder`)가 모두 "Search"입니다. label만으로 찾으면 다른 위젯을 누릅니다. `role_rect(.., Role::TextInput, ..)`처럼 role을 함께 봅니다(QA `-search-entry-editor.md` 89행). 탐색기 행 이름도 탭 제목·breadcrumb·행 안 Label과 겹치므로 가장 왼쪽 노드를 고릅니다(3.3절).
- tooltip이 클릭을 가로챔: 공용 tooltip(`taide_native_ui::tooltips`)은 닫힘 지연 동안 아래 위젯 클릭을 막습니다. 버튼을 누른 직후 근처 위젯을 누르면 첫 클릭이 사라집니다. `settle_pointer`(포인터를 먼저 옮기고 40프레임)를 씁니다(QA `-search-panel.md` 55·100행). 원본 Radix tooltip은 포인터를 막지 않으므로 제품 쪽 차이이지만 이 영역 파일 밖입니다.
- 한 프레임 늦음: 클릭한 프레임의 접근성 상태나 popup의 첫 sizing pass 위치를 읽으면 틀립니다. 한 프레임 더 그린 뒤 읽습니다(QA `-search-panel.md` 54행).
- 우클릭 메뉴: 3.3절 (c)·(d). 열렸는지 한 프레임만 보고 판단하지 않고 `step_until`로 메뉴 항목 노드를 기다립니다.
- 검사 fixture가 다른 기능에 걸림: `.rs` fixture는 LSP document symbols 실패(`TransportClosed`)를 상태줄에 남겨 "조용한 reveal" 단언을 깨뜨렸습니다. 이 영역 검사 fixture는 `.txt`를 씁니다(QA `-search-panel.md` 90행). 자동 reveal(25) 기본값 켜짐이 다른 검사의 스크롤 위치를 바꾼 사례가 있어, 수동 reveal을 보는 검사는 `explorer_auto_reveal`을 끕니다(QA `-explorer-rows.md` 85행).
- macOS 경로 정규화: `/var` ↔ `/private/var`. git status `abs_path`는 정규 경로입니다(3.2절 (e)).
- 검사 프레임의 포커스: `RawInput::default().focused == true`(0.2절). 알림·포커스 관련 단언은 `focused`를 명시하거나 설정으로 포커스 영향을 끕니다.
- 부하 흔들림: 앱 lib 실제 App 검사를 병렬로 많이 돌리면 host 큐(64) 포화·종료 대기 시간 초과로 실패합니다. 영역 필터는 `--test-threads=2`, 게이트는 `--test-threads=4`, 시간 초과 실패는 단독 1회 재실행으로 구분합니다(운영 문서 1장, PROCESS.md W1 병합 기록).
- 앞 작업자 코드가 컴파일되지 않은 채 넘어온 적이 있습니다(묶음 C "이어받은 상태"). 이어받으면 먼저 `cargo check --all-targets`부터 돌립니다. 과거 QA의 "통과" 기록을 현재 상태로 쓰지 않습니다(운영 문서 6절).

### 4.2 구조

- region 규칙: 렌더(`explorer`, `&self`) 안에서 앱 상태를 바꾸지 않습니다. 새 결과는 `ExplorerRegion` 또는 `FrameOutput` 필드로 내보내고 `finish_explorer`/`show_window`에서 적용합니다(2.3절). 편집기 위젯 focus처럼 영역 밖에서만 알 수 있는 값은 그 영역이 `FrameOutput` 필드를 켜게 합니다(선례: `is_editor_widget_focused`, `domains/editor.rs` 6043행 → `application.rs` 2116행).
- 검색 generation은 프로세스 전체에서 고유해야 합니다. 패널을 내렸다 다시 만들면 session이 같아 늦은 배치·응답이 새 실행에 섞입니다(QA `-search-panel.md` 80행).
- 검색 배치를 파일마다 `HostReply`로 보내지 않습니다. `search-stream.rs` 버퍼에 쌓고 50ms마다 꺼냅니다(사양 2.0.2). 같은 session의 새 실행 전에 명시 취소를 보내지 않습니다(backend `begin`이 대체, 원본에서 명시 취소가 새 토큰을 끄는 경합이 있었음).
- 사양이 말하는 `git-status-supply.rs`·`GitStatus(project)` host 명령은 존재하지 않습니다(1.3절). 실제 API는 `GitDecorations`입니다.
- 결정 D10 A: 탐색기 컨텍스트 메뉴 항목은 원본의 16px 아이콘 없이 글자만 둡니다(19의 기존 차이). 20·21을 complete로 올려도 아이콘을 넣지 않습니다.
- 공용 crate(`crates/*`)를 바꿔야 할 것 같으면 멈추고 메인에 알립니다. `src-tauri`도 영향을 받습니다(사양 2.0.1).
- 바꾸기 건너뜀 검사는 binary fixture로 만들 수 없습니다. backend 검색이 binary를 결과에서 빼므로 교체 대상이 될 수 없습니다. 검색 결과에는 나오고 교체에서 건너뛰는 비UTF-8 파일을 씁니다(QA `-search-entry-editor.md` 78행).

### 4.3 보안·허락·사용자 데이터

- `tests/explorer-delete.rs`는 실제 휴지통을 씁니다. 허락 없이 실행하지 않습니다(3.5절). 2026-10-27에 금지를 어긴 실행이 있었습니다.
- 실제 OS 알림을 보내지 않습니다. 검사는 가짜 플랫폼(`FakePlatform`)만 씁니다.
- git 검사는 `application-git-fixture.rs` 방식(전역·시스템 설정 차단, hooks 경로 고정)만 씁니다.

### 4.4 남은 테스트 부채·관찰 (행 상태와 별개, 결함 보고가 오면 처리)

- `reconnect_explorer_search`(`domains/explorer_search.rs` 1146-1148행)는 `tree_loading`만 비웁니다. host 재연결 때 유실된 `SearchFinished`·`SearchReplaced`·`RecentSearchesSaved` 때문에 패널 Running·`is_replacing`·이력 대기 목록이 남을 수 있습니다(QA `-search-entry-editor.md` 129행).
- `search-stream.rs` `Streams`는 검색 편집기 탭 id별 항목을 탭 닫힘 때 지우지 않습니다(같은 QA 130행).
- 사이드바를 접은 동안 실행 중이던 검색은 region이 그려지지 않아 패널이 남습니다. 원본 접힘 시 unmount 여부와 비교가 필요합니다(QA `-search-panel.md` 99행).
- host 큐가 가득 차 `RecordRecentSearches` 제출이 실패하면 쓰기 큐가 settle되지 않습니다(같은 줄).
- 어느 slot에도 표시되지 않는 프로젝트의 활성 검색 편집기 탭은 unmount·실행 취소되지 않습니다(기능표 editor-81 비고).
- 관찰(미확인): 보조 창 탐색기에서 새 파일을 만들면 그 파일 탭이 메인 창 활성 탭이 되는 것으로 보였습니다(QA `-explorer-rows.md` 107행). 원본 생성 뒤 열기 대상 창과 비교가 필요합니다.
- 기능표 explorer-search 절 머리 합계 표는 낡았습니다(1.1절). lane 규칙상 행 갱신 작업자는 합계 표를 고치지 않으므로, 전체 집계를 다시 낼 때 메인이 행을 Grep해 다시 셉니다.
