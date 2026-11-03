# explorer-search lane 묶음 B: 검색 패널 핵심 (search-panel)

> 작성일 2026-11-02 · lane `explorer-search` 2/3 · worktree `TAIDE-lanes/explorer-search`(브랜치 `lane/explorer-search`)
> 기준: 사양 `docs/research/2026-10-28-lane-explorer-search-spec.md` 2.0.2·2.0.5·2.2절, 결정 D5·D9 추천안(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`), 구조 `docs/research/2026-10-28-native-lane-seams.md`

## 대상 ID와 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| explorer-search-38 | partial | run·cancel·배치 버퍼·늦은 응답 무시 연결. 교체(`search_replace`) host 명령은 33(묶음 C)과 함께 |
| explorer-search-31 | complete | 검색 패널 본체(입력·토글·이력 메뉴·제외 glob·범위 칩·상태 줄·본문·실패 toast) |
| explorer-search-32 | complete | 누적기·행·가상 목록·헤더 접기·일치 열기(preview + reveal) |
| explorer-search-35 | complete | 최근 검색 계산·쓰기 큐·설정 patch |
| explorer-search-34 | complete | 입력 시 검색 디바운스, IME 조합 중 보류(D9 A) |
| explorer-search-1 | complete | 4개 보기 tablist + Search 본문 + 4px 모서리. Git 본문은 git-1 |
| explorer-search-30 | complete | 묶음 A의 mount별 상태 + 보조 창 Search 본문(사양 2.1: 판정은 묶음 B 끝). Git 본문은 git-1 |

대응표 행 비고 갱신: shell-5(창별 분리·보조 창 Search 해소), shell-59(Search 본문), settings-80(searchOnType·debounce 소비자 위치).

## 원본 근거

사양 2.2절의 ID별 "원본 동작"을 따랐습니다. 직접 다시 확인한 구간: `src/entities/search/{search-query,search-run-state,search-result,search-history,use-search-run}.ts`와 각 단위 검사, `src/features/search/{search-result-rows.ts,search-panel.tsx,search-option-toggles.tsx,search-history-dropdown.tsx,search-exclude-glob-input.tsx,search-results-list.tsx,search-match-row.tsx}`, `src/shared/ui/file-group-header.tsx`, `src/widgets/search-panel/search-panel-container.tsx`(+test), `src/widgets/explorer/explorer-panel.tsx:113-140,246-258`.

## 구현

### 새 모듈 (`native/taide-native-app/src`)

- `search-stream.rs`: `Session{owner, id}`, `Streams`(`Mutex<HashMap<Session, Stream{generation, batches}>>`). `begin`·`push`(현재 generation만, 비어 있다가 처음 쌓이면 true)·`has`·`take`·`forget`.
- `search-results.rs`: `Accumulator`(경로→그룹 index 색인, 증분 append, 인접 컨텍스트 중복 제거, 누적 수), `rows`·`row_id`·`estimate_height`(헤더 20, 일치 4+16+컨텍스트×20), `utf16_byte_offset`(서로게이트 안쪽은 다음 경계), 미리보기 공백 접기(`white-space: nowrap`과 같은 결과), 가상 목록 `show`(`ScrollArea::show_viewport`, overscan 12).
- `search-history.rs`: `add_recent_term`(20개, 공백 정리, 맨 앞이면 무변화), `Queue{pending, in_flight}`(`enqueue`·`settle`·`list`).
- `search-panel.rs`: `Appearance`(사양 2.0.5 토큰), `Status`·`resolve_view`·`is_truncated`, `Request`(includeGlob·scopeDir·seedText·openReplace), `Panel`(입력·토글·제외 glob·바꾸기 열림·체크 제외 집합·접힘·실행 상태·50ms 배치 반영·입력 검색 마감·IME 조합 추적)과 `show`.

### 영역 연결 (`domains/explorer_search.rs`)

- 명령: `RunSearch{run, mount, project, streams}` → `spawn_transient`에서 `search_actions::search_run`, 배치는 `Streams::push` 후 첫 배치에만 repaint, 끝에 `SearchFinished{mount, generation, result}` 하나. spawn 거부면 그 자리에서 `Err(stopped())`. `CancelSearch(session)` → `search_cancel`(오류 무시, 원본 `.catch(() => undefined)`). `OpenSearchMatch{project, pane, open, viewport}` → preview 탭 + `apply_file_link_reply`(reveal). `RecordRecentSearches(list)` → `settings_update(SettingsPatch{recent_searches})` → `RecentSearchesSaved`(결과 무관 settle).
- region: Search 보기에서 `Sidebar.search`(없으면 생성, owner = 메인 `WINDOW_LABEL`/보조 창 label, session id = `search-panel-<mount>`)를 그립니다. 설정(`search_on_type`, debounce)과 이력(`Queue::list(settings.recent_searches)`)은 렌더 입력으로 읽습니다. 보기가 Search가 아니게 된 frame에 패널을 내리고 실행 중이면 취소를 남깁니다(D5 A). 출력(run·open)은 `finish_explorer`에서 명령으로 바꿉니다(이력 기록 → run 순서). 범위 칩 해제는 sidebar 요청의 includeGlob·scopeDir만 지웁니다.
- 패널이 sidebar와 함께 사라지는 경우(slot 닫힘·프로젝트 교체·region panic)도 `symbol_sidebar::Views`가 취소 목록에 모으고 tick이 `CancelSearch`를 보냅니다.
- 응답: `SearchFinished` → 그 mount 패널의 `finish`(남은 배치 반영·total·completed, 명시 실행 실패만 `report` toast), `SearchMatchOpened` → `apply_file_link_reply`, `RecentSearchesSaved` → `Queue::settle`.

### 수정한 영역 파일

- `symbol-sidebar.rs`: `Sidebar.search`·`search_request`·`open_nonce`, `running_search`, `Views::get_mut`·`take_cancelled`, 교체·정리·take 때 실행 중 검색 수집, `BUTTON_RADIUS` 2 → 4(1).
- `navigation-icons.rs`: `CaseSensitive`·`WholeWord`·`Regex`·`EyeOff`·`History`·`Check`·`Loader` variant(SVG는 묶음 A에서 추가된 lucide 파일과 기존 `loader-circle.svg`).
- `explorer.rs`: `explorer::Appearance`에 `search: search_panel::Appearance`와 `search()` 접근자.
- `lib.rs`: 모듈 4줄(알파벳 위치), `search_panel`·`search_stream`은 공개 `Command` 변형이 쓰므로 `pub mod`(기존 `breadcrumbs`·`workspace_symbol_host`와 같은 방식).

## 검증 (실제 명령과 결과)

모든 cargo 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/explorer-search`를 붙였습니다.

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets` | 성공. 기존 경고 3개(`is_applying`, `zen.rs` mut, `fixture_with_emmet_peek`)와 vendor `wry-preview` 경고만 |
| `cargo test --quiet … --lib -- search_history search_results search_panel::tests symbol_sidebar --test-threads=2` | 1차 57 통과·2 실패(검사 쪽 결함: 클릭한 frame의 접근성 상태를 읽음, popup 첫 sizing pass 위치를 읽음) → 한 frame 더 그리게 고친 뒤 59 통과 |
| `cargo test --quiet … --lib -- application::search_panel_tests --test-threads=2` | 1차 3 통과·2 실패 → 원인: 앞 클릭이 남긴 공용 tooltip 층(닫힘 지연)이 입력 상자 위를 덮어 입력 클릭이 막힘(포인터를 먼저 옮기고 닫힐 때까지 그린 뒤 클릭하도록 검사 수정), 보조 창은 입력 클릭 추가 → 5 통과 |
| `cargo test --quiet … --lib -- search_panel search_history search_results symbol_sidebar --test-threads=2` | 65 통과(단위 + 실제 App 5, 머리 영역 수치·이력 파일 검사 추가 후) |
| `cargo test --quiet … --lib -- search explorer symbol_ navigation application::tab_menu_tests application::auxiliary_window_tests application::error_boundary_tests application::shell_dialogs_tests toast tooltip_controlled breadcrumb problems presentation --test-threads=2` | 206개 중 205 통과, 1 실패(아래 "묶음 A 검사 보정") |
| `cargo test --quiet … --lib -- application::explorer_search_tests::실제_app_활성_파일_자동 --test-threads=2` | 보정 뒤 1 통과 |
| 위 영역 필터 + `application::startup_tests application::context_menu_tests` | 228 통과 |
| `cargo test --quiet … --test explorer -- --test-threads=2` | 21 통과 |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 통과(새 파일과 이번에 고친 파일만 `cargo fmt`로 정리, `git status`로 다른 파일 변경 없음 확인) |

UI crate(`taide-native-ui`)와 remote-web은 이번 묶음에서 바꾸지 않아 UI·Wasm 검사는 실행하지 않았습니다.

### ID별 자동 검사

- 38: App `실제_app_검색_보기는_enter로_…`(run → 배치 → `SearchFinished` 4건/3파일), `실제_app_잘못된_정규식_…`(보기 전환 시 실행 중 패널 폐기, 버퍼 forget으로 늦은 배치 거부), 단위 `결과는_50ms_뒤에_…`(배치 반영 시점, 다른 generation 응답 무시), sidebar `검색_요청은_slot의_프로젝트가_바뀌면_…`(교체·정리 때 취소 목록).
- 31: 단위 run-state 7사례(원본 그대로), 머리 영역 수치(버튼 24px 6개, 질의 상자 focus 테두리 `app.focusBorder`, 제외 상자 `panel.inputBorder`, 상자 바탕 `panel.inputBackground`, 아래 구분선 `app.border`, 바탕 `panel.background`), 토글 selected·질의 반영(정규식·gitignore·제외 glob·scopeDir, contextLines 0), 공백·조합 중 Enter 무시, 이력 버튼 비활성·활성·선택은 질의만 변경, 시드·openReplace·범위 칩 해제. App: Enter 실행·완료 문구, 범위 칩 실행·해제, 잘못된 정규식 Enter → toast + `search.failed`, 보기 전환 뒤 빈 패널.
- 32: 단위 누적 10사례(원본 12 중 복원 2 제외), 행 6사례, 예산 3사례(구조), UTF-16 변환(BMP 밖 문자), 공백 접기, 일치 행 클릭 → 열기 출력, 바꾸기 열림 때 체크박스. App: 헤더 접기·펼침, 일치 클릭 → preview 탭·caret(2행 1열 = 오프셋 9), 2,100행 결과에서 그린 일치 행 수 < 100, 보조 창 일치가 그 창 pane에 열림.
- 35: 단위 10사례(원본 그대로). App: 입력 검색은 이력 없음, 이어진 Enter 2회 → 설정 파일 `recentSearches = ["needle one", "needle"]`.
- 34: 단위 원본 실시간 9사례 + 조합 1사례(입력 시간 주입). App: 디바운스 전 run 없음, 뒤 1회, 이력 없음.
- 1: App `실제_app_사이드바는_네_보기를_slot별로_전환하고_검색은_패널을_git은_빈_본문만_보인다`(Search 본문에 `search.liveSearchHint`), sidebar 단위 검사의 선택 바탕 모서리 4px.
- 30: App `실제_app_보조_창_검색_보기의_일치는_그_창의_pane에_preview로_열린다`.
- 실제 앱 경로: App 검사는 `launch`(실제 `NativeApplication::new`) → `eframe::App::logic` + 렌더 → 사이드바 region(`begin_explorer → explorer → finish_explorer`) → host 작업자(`dispatch` → `search_actions`)를 거칩니다.

## 사양과 다르게 한 것 (근거)

- Appearance 위치: 사양은 `presentation-refresh.rs` Appearances에 추가였지만, 적용 지점이 settings lane 파일(`domains/settings.rs`)이라 `explorer::Appearance`(이 lane 파일) 안에 넣었습니다. 테마 갱신 때 explorer Appearance와 함께 바뀝니다.
- owner: 메인은 native 창 label `WINDOW_LABEL`("native-main"), 보조 창은 그 창 label입니다(원본 "main"과 문자열만 다르고 backend 세션 키 역할은 같습니다).
- generation: 세션별이 아니라 프로세스 전체에서 고유하게 잡습니다. 패널을 내렸다 다시 만들면 같은 session에서 번호가 다시 시작해, 끊긴 실행의 늦은 배치·응답이 새 실행에 섞일 수 있기 때문입니다(원본은 unmount 때 generation을 올려 같은 효과).
- 응답 `SearchFinished`는 session 대신 mount를 싣습니다(패널을 찾는 키, generation이 고유하므로 충분).
- 높이 보정: 행을 추정 높이 그대로 그리므로 측정 보정이 필요 없습니다.
- 일치 열기: editor 영역의 `open_problem`은 pane을 받지 않는 비공개 함수라, 같은 절차(preview `layout_open_tab` → 그 pane 활성 탭 → `OpenedFileLink`)를 pane을 받는 `open_search_match`로 이 영역에 두었습니다.
- 입력 시 검색 단위 검사는 패널 단위(입력 시간 주입)로 옮기고, 실제 App은 대표 경로(디바운스·이력)만 확인했습니다.

## 범위 밖 최소 수정 (사유)

- `application.rs`: `explorer_search_state` 생성 리터럴 두 필드, `ExplorerInputs` 리터럴 두 필드, 테스트 모듈 선언 한 줄(모두 영역 블록 안 추가).
- `application-explorer-views-tests.rs`(사양 1의 단언 수정): Search 본문 빈 단언을 패널 표시 단언으로 바꾸고 테스트 이름을 맞춤.
- 묶음 A 검사 보정 `application-explorer-search-tests.rs` `실제_app_활성_파일_자동_reveal은_…`: `.rs` fixture가 LSP document symbols 실패(`native document symbols: TransportClosed`)를 상태줄에 남겨 "조용한 reveal은 상태줄을 비운다" 단언이 깨졌고(단독 재실행에서도 재현), "시끄러운 reveal 실패" 단계는 그 LSP 상태 덕분에 통과하던 것이었습니다(루트 바로 아래 없는 파일은 `tree_reveal`이 실패하지 않음). fixture를 `.txt`로 바꾸고 없는 경로를 `missing-dir/missing.txt`(조상 디렉터리 읽기가 실패)로 바꿔 두 단언이 각자 의도대로 검사되게 했습니다. 기대값은 바꾸지 않았습니다.
- `symbol-sidebar-tests.rs`: 모서리 단언 추가와 검색 요청·취소 목록 검사 1개.

## 남은 범위·테스트 부채

- 38: 교체 host 명령(`Replace`/`Replaced`)과 binary·NUL 건너뜀 App 검사는 33(묶음 C). 2026-11-03 리뷰 반영에서 묶음 C 연결·검사를 근거로 대응표 38을 complete로 올렸습니다(`2026-11-02-native-lane-explorer-search-search-entry-editor.md` "리뷰 반영 수정").
- 33(묶음 C)으로 넘긴 것: 바꾸기 입력 줄·모두 바꾸기·질의 불일치 안내·확인 대화상자, `search-query.ts` 비교(`isSameSearchQuery`) 이식과 그 8사례(소비자가 33뿐이라 지금 넣으면 쓰이지 않는 코드), `replace-all.svg` 아이콘(사양 B 파일 목록에 있으나 33만 씀).
- 36(묶음 C): 검색 편집기 열기 버튼, 복원 누적기(`restoreSearchResultAccumulator`) 2사례, 검색 편집기 Enter 이력 기록.
- 37·21(묶음 C): 요청(`search_request`·`open_nonce`)을 채우는 진입점. 지금은 검사에서 직접 채워 범위 칩·시드·openReplace를 확인했습니다.
- 테스트 부채(자동 검사로 직접 다루지 않음): 사이드바를 접은 동안 실행 중이던 검색(region이 그려지지 않아 패널이 남음, 원본 접힘 시 unmount 여부와 비교 필요), 제외 glob 입력의 IME 조합(조합 추적은 질의 입력만), host 큐가 가득 차 `RecordRecentSearches` 제출이 실패하면 쓰기 큐가 settle되지 않음, 사용자 실제 IME·화면 픽셀(P3, GUI 필요). 실행이 필요해지는 시점: 배치 31 실기 확인 또는 관련 결함 보고 때.
- 관찰(다른 영역): 공용 tooltip(`taide_native_ui::tooltips`)이 닫힘 지연 동안 아래 위젯 클릭을 가로챕니다. 사이드바 Search 탭을 누른 직후 바로 아래 질의 입력을 누르면 첫 클릭이 막힐 수 있습니다(원본 Radix tooltip은 포인터 이벤트를 막지 않음). 이 lane 파일 밖이라 고치지 않았습니다.
