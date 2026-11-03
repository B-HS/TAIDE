# explorer-search lane 묶음 A: 탐색기 행·상태 (explorer-rows)

> 작성일 2026-11-02 · lane `explorer-search` 1/3 · worktree `TAIDE-lanes/explorer-search`(브랜치 `lane/explorer-search`)
> 기준: 사양 `docs/research/2026-10-28-lane-explorer-search-spec.md` 2.1절, 결정 D1·D6·D7 추천안(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`), 구조 `docs/research/2026-10-28-native-lane-seams.md`

## 대상 ID와 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| explorer-search-18 | not-applicable | 원본에 drag 기능 없음(사양 2.1). 코드 변경 없음 |
| explorer-search-26 | complete | 기존 연결 + D6 뒤 회귀 통과, 보조 창 reveal이 그 창 mount만 선택함을 단언 |
| explorer-search-44 | complete | 보조 창 고정 닫힘 단언(UI workbench + 실제 App) |
| explorer-search-8 | complete | 탐색기 토큰과 hover > primary+focus > 선택 규칙 |
| explorer-search-30 | partial | D6(mount별 상태) 완료. 보조 창 Search 본문은 묶음 B, Git 본문은 git-1 |
| explorer-search-7 | partial | 파생·행 렌더·주입점 완료. 공급 연결은 git-38(git lane) |
| explorer-search-25 | complete | 결정 모듈 + mount별 기록 + 조용한 reveal |
| explorer-search-27 | complete | 누락 화면 + 다시 열기 |
| explorer-search-29 | complete | 실패 상태·반복 요청 중지·출처별 처리 |

## 원본 근거

사양 2.1의 ID별 "원본 동작"을 따랐습니다. 직접 다시 확인한 구간은 `src/widgets/explorer/file-tree-git-status.ts`, `src/features/explorer/file-tree-row.tsx:20-90`, `src/widgets/explorer/explorer-auto-reveal.ts`와 두 단위 검사 파일입니다.

## 구현

### 8 선택/포커스 시각 구분 (`explorer.rs`)

- `explorer::Appearance`가 `explorer.itemHover/itemSelected/itemFocused`를 읽습니다.
- 행 바탕: hover → `itemHover`, 아니면 선택 행 중 primary(`Explorer.selected`)이면서 트리 focus이면 `itemSelected`, 나머지 선택 행은 `itemFocused`(D7 A: 원본 CSS 우선순위대로 hover가 선택보다 우선).
- 트리 focus = `native-tree-focus` 또는 이 mount의 새 항목·이름 변경 입력이 focus를 가짐, 단 popup(메뉴)이 열려 있으면 제외(원본은 메뉴 portal로 focus가 빠짐).
- 아이콘 없는 `Explorer::show`(통합 검사용)는 기존 visuals 색을 유지합니다.

### 30 mount별 탐색기 상태 (D6 A)

- `explorer-mounts.rs` `Mounts`: `ShellSlotId`(메인 slot 또는 보조 창 `explorer` mount) 키, 프로젝트가 바뀐 mount는 새 상태로 시작합니다. 트리 page(`trees`)·`tree_loading`은 프로젝트 키 그대로입니다.
- `CreateRequest`·`RenameRequest`에 `mount: Option<ShellSlotId>`를 넣어 생성·이름 변경 완료/실패와 toast 재시도(`application.rs` `run_toast_actions`)를 그 mount로 보냅니다. 붙여넣기는 기존 clipboard owner가 이미 mount 단위라 그대로입니다. `TreeToggled`는 그 프로젝트 mount 모두에 전달하고 각자 `pending_parent`로 거릅니다. `ExplorerPathRevealed`는 `mount`로 찾습니다.
- 공유 보정 `address_requests`·`claim_requests`·`owns_requests`와 `RenameDraft::show`의 `owns_focus_request`는 쓰이지 않아 지웠습니다.
- 수명: `tick_explorer_search_clipboards`(매 프레임 영역 tick)에서 slot 트리·보조 창 목록으로 정리합니다(`symbol_sidebar::Views::reconcile`과 같은 규칙).
- `forget_explorer_search_project(&ProjectId)`: 그 프로젝트의 mount 상태·page·로딩·실패·데코레이션을 지웁니다. 지금은 다시 열기 완료(`Reply::ProjectReopened`)에서 부르고, shell-37 병합 때 프로젝트 닫힘 처리에서 이 호출 하나로 정리합니다(사양 1.3).

### 7 Git 데코레이션 (D1 A: 파생·렌더·주입점)

- `explorer-git-decorations.rs`: `Decoration`(6종, 배지 글자·접근성 이름·테마 키), `build(rows, root)`(원본 규칙: typeChange→modified, 우선순위, staged/unstaged 중 높은 쪽, 조상 전파는 `root + "/"` 아래만, root 없으면 빈 맵).
- `GitDecorations`(영역 상태): `set(project, Option<&GitStatus>)`는 행 목록이 바뀐 때만 다시 계산하고 `None`이면 지웁니다. `mounted_projects()`는 지금 Files 보기로 보이는 mount의 프로젝트(메인: zen 아님·사이드바 펼침, 보조 창: 탐색기 열림)입니다. root는 매 프레임 `mount`로 갱신해 root가 바뀌면 다시 파생합니다.
- 렌더: `RowIcons.decorations`. 이름·chevron 색 = 데코레이션 색, 파일은 오른쪽 끝 배지 글자(12px), 디렉터리는 지름 6px 점, 배지 왼쪽 4px·행 오른쪽 8px, 이름 truncate 폭에서 배지 폭을 뺍니다. 행 접근성 description = 상태 이름. 타입 아이콘은 자기 색을 유지합니다.
- 공급자는 없습니다(wave 1). git lane이 `domains/git.rs`에서 `mounted_projects()` 각 프로젝트에 `GitCache`를 걸고 `explorer_search_state.git_decorations.set(..)`을 부릅니다.

### 25 활성 탭 자동 reveal

- `explorer-auto-reveal.rs` `decide(Input) -> Decision{Skip, SelectOnly, RevealThenSelect}`(원본 결정 그대로).
- region 렌더(Files 보기·page 있음)에서: 활성 경로 = 이 창 pane 트리의 focused pane 활성 파일 탭, 프로젝트 root 아래만. 사이드바가 안 보이면 region 자체를 그리지 않으므로 skip·기록 없음(원본과 같은 결과). 결정 전에 `Explorer.auto_revealed`에 기록합니다.
- SelectOnly는 `select_path`(최소 스크롤, focus 요청 없음). RevealThenSelect는 finish에서 `RevealExplorerPath{quiet: true}`를 보내고, 성공 응답은 그 mount에만 선택, 실패는 상태줄·toast 없음. 보기 전환·사이드바 펼침은 하지 않습니다.
- 설정은 `services.state.settings.explorer_auto_reveal`을 렌더 입력(`ExplorerInputs`)으로 읽습니다.

### 27·29 루트 누락·트리 로드 오류

- 영역 상태 `tree_failures`. 설정: `Tree{offset: 0}` Err, `TreeSynced{origin: Watcher}` Err. 해제: 그 프로젝트 page가 성공으로 들어오는 모든 경로(`accept_explorer_tree`: 조회·watcher·툴바·삭제 갱신, 펼침, reveal, breadcrumb, 생성·이름 변경·붙여넣기).
- region: `root_missing || tree_failures`이면 page가 있어도 트리 대신 `Explorer::show_missing`(머리줄 분리 재사용 + 가운데 세로 배치: TriangleAlert 20px `appSidebar.iconDefault`, `explorer.projectRootMissing` `app.foreground`, root 경로, 밑줄 버튼 `explorer.reopenProject`, 간격 8px·좌우 16px·12px). 이 상태에서는 `is_tree_requested`를 켜지 않습니다.
- `RefreshOrigin{Watcher, Toolbar, Delete}`를 `RefreshTree`/`TreeSynced`에 붙였습니다. 툴바: 실패 무표시(원본 처리 없음). 삭제: 부모 갱신 오류·결과 오류를 toast(`report`). watcher: page 실패만 실패 상태(디렉터리별 갱신 오류는 원본처럼 재조회로 흡수).
- 다시 열기: 버튼 → `ShellIntent::ExplorerSearch(Intent::ReopenProject{project, root})`(UI `intents/explorer_search.rs`) → `close_project(project)`(shell 경로, mirror flush 포함) + `Command::ReopenProject`. host 큐가 직렬이라 닫힘 뒤에 처리되고, 프로젝트가 아직 있으면(닫기 실패·미제출) 아무것도 하지 않습니다(원본: 닫기 실패면 열지 않음). 닫혔으면 shell `OpenProject(root)`를 실행하고 `Reply::ProjectReopened{closed, failure}` → 이전 프로젝트 상태 정리, 실패는 toast.

### 44 Problems 보조 창 고정 닫힘

- 코드 변경 없음. 검사만 추가.

## 검증 (실제 명령과 결과)

| 명령 | 결과 |
| --- | --- |
| `cargo build --quiet --manifest-path native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir targets/explorer-search` | 성공 |
| `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets --locked --offline --target-dir …` | 성공, 기존 경고 3개(`is_applying`, `zen.rs` mut, `fixture_with_emmet_peek`)와 vendor `wry-preview` 경고만 |
| `cargo test … --lib -- explorer_git_decorations explorer_auto_reveal --test-threads=2` | 23개 통과(파생 9 + 저장소 1 + 결정 13) |
| `cargo test … --lib -- explorer::tests --test-threads=2` | 2개 통과(8 바탕 4경우, 7 렌더) |
| `cargo test … --lib -- application::explorer_search_tests --test-threads=2` | 5개 통과(44+30, 27 폴더 없음, 27 플래그만, 29, 25) |
| `cargo test … --lib -- explorer symbol_ application::tab_menu_tests application::auxiliary_window_tests application::error_boundary_tests application::shell_dialogs_tests toast tooltip_controlled breadcrumb problems title_branch --test-threads=2` | 136개 중 135개 통과, 1개 실패(아래) → 수정 뒤 `application::auxiliary_window_tests application::tab_menu_tests` 10개 통과 |
| `cargo test … --lib -- application::startup_tests application::context_menu_tests application::project_rail_tests application::file_drop_tests application::tab_drag_tests application::window_state_tests application::app_menu_tests application::close_all_tests application::exit_tests --test-threads=2` | 37개 통과 |
| `cargo test … --test explorer` | 21개 통과 |
| `cargo test … --test explorer-clipboard --test projects --test paste-shortcuts` | 5·4·2개 통과 |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test workbench` | 46개 통과(새 44 검사 포함) |
| `cargo check --quiet --manifest-path native/taide-native-ui/Cargo.toml --all-targets --features native-host,inspection` | 성공, 기존 경고 1개(`workbench.rs` 1880 mut) |
| `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 성공, remote-web 기존 dead code 경고만 |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 통과(이번 파일만 `cargo fmt`로 정리, 다른 파일 변경 없음을 `git status`로 확인) |
| `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | 통과 |

- 회귀 1건: `실제_app_보조_창_탐색기_reveal_rename은_메인_사이드바가_열려_있어도_그_창에서만_스크롤과_포커스를_받는다`가 "both explorers" 단계에서 시간 초과. 원인: 25로 보조 창이 자기 활성 탭(z.txt)을 자동 reveal해 목록이 아래로 스크롤되어 m00.txt가 보이지 않음. 원본도 `explorerAutoReveal` 기본값 true라 같은 동작이므로, 수동 reveal을 검사하는 이 검사의 fixture에서 설정을 끄도록 했습니다(기대값은 그대로). 같은 검사의 `is_selected`는 보조 창 mount만 선택됐는지와 선택한 mount가 하나뿐인지로 강화했습니다.
- 실제 앱 경로: 새 App 검사는 `launch`(실제 `NativeApplication::new`)와 `eframe::App::logic` + 렌더로 사이드바 region(`begin_explorer → explorer → finish_explorer`)과 host 작업자를 거칩니다.

## 범위 밖 최소 수정 (사유)

- `application.rs`: `AppSurfaces.explorers` 타입, `explorer_inputs` 한 줄과 초기화, 생성자 영역 리터럴 두 필드, toast 재시도의 mount 조회, 테스트 모듈 한 줄(영역 블록 안 추가·변경).
- `explorer_clipboard.rs`(잘라내기 이동의 `RenameRequest{mount: None}`), `toast-tests.rs`·`tooltip-controlled-tests.rs`·`tests/explorer.rs`·`tests/projects.rs`: 구조체·enum 필드 추가에 따른 리터럴 보정만.
- `application-auxiliary-window-tests.rs`: 보조 창 검사 도구(`Viewports`, `install_viewports`, `step_until`, `file_tab`, `auxiliary_window`, `viewport_of`)를 `pub(super)`로 열어 새 App 검사가 재사용, 위 회귀 1건의 fixture 설정, explorer 상태 조회를 mount 기준으로 변경. `application-tab-menu-tests.rs`·`application-error-boundary-tests.rs`: explorer 상태 조회를 slot 기준으로 변경.
- `symbol-sidebar.rs`: `Views::view(project, slot)` 읽기 함수 추가(주입점의 Files 보기 판정).

## 원본과의 차이·판단

- 26: native reveal 실패는 상태줄에 남깁니다(원본은 처리 안 한 promise라 표시 없음, 사양 2.0.7 유지). 25의 조용한 reveal만 무표시.
- 27: 다시 열기의 닫기 단계는 앱의 `close_project`(mirror flush 포함)를 그대로 쓰고, 열기는 host에서 닫힘을 확인한 뒤 실행합니다. 원본의 "닫기 성공 뒤 열기"와 같은 결과입니다.
- 29: watcher 갱신 중 루트가 사라져도 backend `tree_rows`가 캐시 page를 돌려주면 실패가 아닙니다. 원본도 같은 backend 재조회라 같은 결과이며, 실패 경로는 "첫 조회 실패 → watcher 갱신 성공 복귀"로 검사했습니다.

## 남은 범위·테스트 부채

- 30: 보조 창 Search 보기 본문은 묶음 B, Git 보기 본문은 git-1. shell-5 비고 갱신은 30 판정 때(묶음 B).
- 7: status 공급 연결과 실제 App 판정은 git-38(git lane). 공개 API: `GitDecorations::{set, mounted_projects, get}`(영역 상태 `explorer_search_state.git_decorations`).
- shell-37 병합 시 프로젝트 닫힘 처리에서 `forget_explorer_search_project` 호출(지금은 다시 열기 경로만 호출).
- 자동 검사로 직접 다루지 않은 하위 경우(테스트 부채): 25의 Search 보기 중 변화 없음(결정 단위 검사와 region의 Search 분기 조기 반환으로만 확인), 같은 프로젝트를 두 slot이 볼 때, 29의 삭제 뒤 부모 갱신 실패 toast(삭제가 실제 휴지통을 쓰므로 허락 P1 전에는 실행 불가). 실행이 필요해지는 시점: 묶음 C(16) 허락 뒤.
- 관찰(이 묶음 범위 밖, 미확인): 보조 창 탐색기에서 새 파일을 만들면 그 파일 탭이 메인 창 활성 탭이 되는 것으로 보입니다(25 자동 reveal이 메인에서 그 파일을 선택한 것으로 관찰). 원본 생성 뒤 열기 대상 창과 비교가 필요합니다.
