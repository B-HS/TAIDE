# explorer-search-20·21: 행 우클릭 메뉴 경로로 검사 전환

> 작성일 2026-11-04 · 메인 작업 트리 `/Users/hyunseokbyun/development/TAIDE`(HEAD `062d7560`)
> 근거 문서 `docs/handoff/native-completion/12-remaining-explorer-search.md` 3.3·3.4절
> 제품 코드 변경 없음. 앱 lib 실제 App 검사만 실제 메뉴 클릭 경로로 바꿨습니다.

## 대상 ID와 판정

| ID | 판정 | 근거 |
| --- | --- | --- |
| explorer-search-20 | 자동 검사 범위 완료 | `실제_app_터미널에서_열기는_...`가 행 우클릭 → `explorer.openInTerminal` 클릭 → `Action::OpenInTerminal` → `finish_explorer` → host `OpenTerminalAt` → 고정 Terminal 탭까지 통과 |
| explorer-search-21 | 자동 검사 범위 완료 | 같은 검사가 행 우클릭 → `explorer.findInFolder` 클릭 → `Action::FindInFolder` → 사이드바 검색 범위까지 통과 |

기능표 행 상태(complete)·비고 갱신은 메인 몫입니다(이 작업 범위 밖).

## 바꾼 내용

파일 하나: `native/taide-native-app/src/application-explorer-search-tests.rs`

1. 지역 helper 추가
    - `sidebar_node(output, label)`: 접근성 노드 중 label이 같은 것의 bounds를 rect로 바꿔 `left()`가 가장 작은 것(사이드바 행)을 고릅니다. handoff 3.3절 (d) 2번 코드에 `left()` 최소 선택을 그대로 씁니다.
    - `choose_row_menu(application, context, row, item)`: 행 노드가 뜰 때까지 프레임을 돌린 뒤 그 행의 왼쪽 안쪽(`ROW_CLICK_INSET = 8.0`)에서 보조 버튼 클릭 → 메뉴 항목 노드가 뜰 때까지 프레임 반복 → 항목 중심을 주 버튼 클릭. 문서의 `center()` 대신 왼쪽 안쪽을 쓴 이유는 행 전체 rect 중심이 펼침 chevron·이름 영역과 겹칠 수 있어 메뉴 대상 행 선택을 확실히 하기 위해서입니다.
    - `active_terminal_cwd(application, project, title)`: 활성 탭이 preview가 아니고 제목이 `terminal.title`이며 `session_id`가 빈 Terminal일 때 cwd를 돌려줍니다. 기존 검사의 인라인 판정을 그대로 옮긴 것입니다(의미 불변).
2. `실제_app_터미널에서_열기는_그_폴더를_cwd로_한_고정_터미널_탭을_열고_폴더에서_찾기_요청은_기본_제외_폴더_안만_검색한다` 수정
    - 폴더 사례: `application.submit(HostCommand::ExplorerSearch(Command::OpenTerminalAt{..}))` 직접 주입을 `choose_row_menu(.., IGNORED_FOLDER, explorer.openInTerminal)`로 교체. 대기·단언(cwd `<root>/node_modules`, 고정·제목·빈 `session_id`, 그 pane 활성)은 유지.
    - 파일 사례 추가: `choose_row_menu(.., "note.txt", explorer.openInTerminal)` 뒤 터미널 수가 하나 더 늘고 활성 터미널 cwd가 프로젝트 root(`root.to_str()`)인지 단언합니다.
    - 폴더에서 찾기: `symbol_sidebars.entry(..).request_search(..)` 직접 호출을 `choose_row_menu(.., IGNORED_FOLDER, explorer.findInFolder)`로 교체. 뒤 단계(보기 `View::Search` + 범위 칩 `explorer.searchScopeLabel{path}`, "needle" Enter, `search.matchCount{1,1}`·`inner.txt`만, 칩 해제 뒤 `note.txt`만)는 그대로 둡니다. 보기 전환 단언은 `step_until` 대기 조건에 그대로 들어 있어 실패하면 panic합니다.

직접 주입 경로가 사라졌으므로 세 단계(폴더 터미널·파일 터미널·범위 검색)는 모두 실제 우클릭·메뉴 클릭을 거쳐야만 통과합니다. 메뉴가 열리지 않으면 `step_until`이 `phase=explorer.openInTerminal`·`phase=explorer.findInFolder`로 panic합니다(`application-auxiliary-window-tests.rs` 108행, 10초 한도).

## 실행한 검사 (실제 결과)

1. 계약 지정 검사 — exit 0

```
cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- --test-threads=2 explorer_search_tests
```

```
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1226 filtered out; finished in 1.75s
```

포맷 정리 뒤 같은 명령을 다시 돌려 같은 결과(7 통과)를 확인했습니다. 선택된 7건은 이 모듈의 검사 전부입니다.
2. 포맷 — 이 파일만 확인, exit 0

```
rustfmt --check --edition 2024 native/taide-native-app/src/application-explorer-search-tests.rs
```

3. 메인이 같은 날 묶음 전체로 다시 확인 — 모두 exit 0

| 명령 | 결과 |
| --- | --- |
| `cargo test --lib -- --test-threads=2 search_panel_tests explorer_search_tests explorer::tests notifications_tests` | 24 통과, 0 실패(1209 filtered) |
| `cargo test --lib -- --exact application::explorer_search_tests::실제_app_터미널에서_열기는_...` | 1 통과 |
| `cargo test --test explorer` | 22 통과, 0 실패 |
| `cargo check --all-targets` | exit 0. 경고는 기존 것만(`is_applying`, `zen` mut, `fixture_with_emmet_peek`, vendor `wry-preview`) |
| `cargo fmt -- --check` | exit 0 |

`application-search-panel-tests.rs`(같은 날 explorer-search-33 작업)의 포맷 차이는 메인이 `cargo fmt`로 정리했고, 그 뒤 크레이트 전체 `--check`가 통과했습니다.

## 검증하지 않은 것과 위험

- `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check`는 이 묶음만 따로 실행했을 때 동시 작업 중인 `src/application-search-panel-tests.rs`(다른 작업 소유) 때문에 실패했습니다. 메인이 그 파일을 포맷한 뒤 크레이트 전체 `--check`가 통과했습니다.
- `cargo check --all-targets`, `--test explorer`(통합 `tests/explorer.rs` 2999행), 병합 게이트는 이 작업자 단계에서 실행하지 않았고, 메인이 같은 날 위 표대로 실행해 통과했습니다. 제품 코드를 바꾸지 않았으므로 통합 검사는 회귀 확인용입니다.
- 실제 터미널 프로세스 실행·macOS 창 표시는 보지 않습니다(레이아웃 탭 기술자까지가 자동 검사 범위, 기존 검사와 같은 범위).
- 이 검사는 접근성 label이 같은 노드가 여러 개일 때 `left()` 최솟값을 행으로 간주합니다. 지금 트리 fixture에서는 통과하지만, 사이드바 왼쪽에 같은 label 노드가 생기면 오탐 가능성이 있습니다.
