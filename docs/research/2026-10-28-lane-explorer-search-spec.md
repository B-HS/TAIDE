# lane explorer-search 사양

> 작성일 2026-10-28 · lane `explorer-search`(wave 1) · 사양 단계 산출물
> 운영 기준: `docs/memory/2026-10-28-native-lane-operations.md`. 이후 구현 작업자는 이 문서와 여기서 가리키는 파일 구간만 읽습니다.
> 같은 날 04시 개정: git·shell·settings lane 사양(`docs/research/2026-10-28-lane-{git,shell,settings}-spec.md`)과 겹치는 경계(7·22·23, OS 알림, shell-37·shell-5)를 맞추고 아이콘 자산과 원본 경로를 정정했습니다.

## 0. 읽는 법

- 기준은 원본 TS 화면·상태·상호작용입니다. 새 기능·디자인은 넣지 않고, 원본 결함·웹 기술 한계는 재현하지 않습니다(2.0.7에 판단 목록).
- 원본 근거는 `경로:줄`로 적었습니다. 줄은 HEAD `72065136` 기준입니다. `src/...`는 저장소 루트 기준입니다.
- 단계 0(lane 경계) 작업이 진행 중이라 `native/taide-native-app/src/application.rs`·`host.rs`, `native/taide-native-ui/src/commands.rs`·`shell.rs`는 심볼 이름으로만 인용합니다(작성 중에도 `application.rs` 줄이 움직였습니다). 작업 트리에는 앱 `src/domains/{mod,commands,shell}.rs`, UI `src/intents/{mod,commands}.rs`가 생기고 있습니다. 이 문서의 "영역 파일"은 그 규칙을 따른 앱 `src/domains/explorer_search.rs`, UI `src/intents/explorer_search.rs`, `HostCommand::ExplorerSearch(..)`/`HostReply::ExplorerSearch(..)` 하위 enum을 뜻합니다. `docs/research/2026-10-28-native-lane-seams.md`가 생기면 이름은 그 문서를 따릅니다.
- 이 문서는 정적 읽기로 작성했습니다. cargo·앱 실행은 하지 않았습니다.

## 1. 요약

### 1.1 대상 ID와 상태

대상은 23개입니다. 상태는 기능 대응표(`docs/quality-assurance/2026-10-09-native-function-matrix.md` 379-422행) 기준입니다.

| 상태 | 수 | ID |
| --- | --: | --- |
| partial | 7 | 1, 8, 16, 27, 29, 30, 44 |
| unwired | 4 | 18, 25, 26, 38 |
| missing | 12 | 7, 20, 21, 22, 23, 31, 32, 33, 34, 35, 36, 37 |

lane 처리 예상(4장 추천안 기준)

| 결과 | ID |
| --- | --- |
| 이 lane이 구현하고 complete 후보 | 1, 8, 20, 21, 25, 26, 27, 29, 30, 31, 32, 34, 35, 36, 37, 38, 44 |
| 문서 정정만 | 18(not-applicable) |
| 이 lane 몫을 끝내도 partial로 남음 | 7(공급은 git-38), 16(휴지통 검사 허락 P1), 33(OS 알림 진입점 병합 전이면) |
| git lane으로 이관 | 22(=git-45 진입), 23(=git-55 진입) |

### 1.2 현재 코드와 대조해 바뀌는 판정

대응표 행 일부는 최근 단계(shell-5·shell-59·shell-78) 이후 갱신되지 않았습니다.

| ID | 대응표 | 실제 | 남은 것 |
| --- | --- | --- | --- |
| 1 | 파일/아웃라인 2개 | 4개 tablist·전환·접근성은 연결됨(`docs/quality-assurance/2026-10-28-native-explorer-views-theme-providers.md`) | 검색 본문(31), Git 본문(git lane), 버튼 모서리 4px |
| 18 | unwired | 원본에 drag 기능이 없음 | not-applicable로 정정 |
| 26 | unwired | shell-78에서 메인·보조 창 모두 연결, 실제 App 검사 있음 | 대응표 갱신, 30 변경 뒤 회귀 |
| 30 | 보조 창 미생성 | 보조 창 탐색기 패널(240/180/40%, 접힘, 창별)은 shell-5로 연결 | mount별 탐색기 상태, 보조 창 검색 본문 |
| 44 | 보조 창 미연결 | 보조 창 셸은 Problems·상태바를 그리지 않음 | 고정 닫힘을 단언하는 검사 |

같은 기능을 다른 영역 행이 함께 가리킵니다: 7 = git-38, 22의 탭 본문 = git-45, 23 = git-55의 탐색기 진입, 36의 탭 본문 = editor-81 = shell-89의 searchEditor 부분, 37 = commands-ui-11·12의 실행, 25·34 = settings-80의 두 소비자, 33의 알림 = settings-23의 검색 치환 생산자.

### 1.3 다른 lane 의존

| 필요한 것 | 제공 | 시점 | 영향 ID | 없을 때 |
| --- | --- | --- | --- | --- |
| 영역 파일·하위 enum·영역 trait(단계 0) | 메인 | wave 0 | 전부 | 시작하지 않음 |
| 프로젝트별 Git status 조회 캐시 `git-cache.rs`(git-64, git 사양 2.3) | git lane G1 | wave 2 | 7 | 4장 D1. 이 lane은 파생·행 렌더·주입점까지 만들고 7은 partial로 둡니다. git-38이 공급을 연결하고 행을 닫습니다 |
| diff 탭 본문(git-45) | git lane | wave 2 | 22 | 4장 D3 |
| 파일 히스토리 패널(git-53~55) | git lane | wave 2 | 23 | 4장 D2 |
| Git 보기 본문(git-1) | git lane | wave 2 | 1·30의 Git 보기 | 1·30은 Git 본문을 git-1 행으로 넘기고 판정 |
| 앱 공통 OS 알림 진입점 | shell lane shell-91(shell 사양 4.1 결정 1, = terminal-agent-59~63) 또는 settings lane settings-23(`domains::settings::notify`) | wave 1~2 | 33의 알림 한 줄 | 33을 알림 한 줄만 남긴 partial로 두고, 병합 뒤 호출 1곳 추가. 두 사양이 같은 진입점을 따로 설계하고 있어 메인 조율이 필요합니다(4.1 마지막 항목) |
| 탭 내용 분기에서 `TabKind::SearchEditor`를 영역 렌더러로 넘기는 hook | 단계 0 또는 editor lane | wave 1 | 36(+editor-81) | 4장 D4 |
| shell-37 프로젝트 닫기의 앱 캐시 정리(`trees`, `explorers`, `tree_loading`, `symbol_sidebars` 항목 제거) | shell lane | wave 1 동시 | 30(D6 키 변경) | 이 lane이 `domains::explorer_search`에 `forget_project(&ProjectId)`(그 프로젝트 mount의 탐색기·검색 세션·`tree_failures` 제거)를 공개합니다. 나중에 병합되는 쪽이 shell-37의 `explorers` 직접 제거를 이 호출 하나로 바꿉니다 |
| shell-5 잔여(탐색기 선택·rename·create의 창별 분리) | 이 lane의 D6 A가 해소 | - | 30 | shell 사양 4.1 결정 5 A(이 lane 병합 뒤 shell 묶음 3은 검증·행 갱신만)와 같은 방향입니다 |
| settings-80의 탐색기 자동 reveal·searchOnType 소비자 | 이 lane(25·34)이 제공 | - | - | settings lane은 병합 뒤 App 증거만 추가합니다(settings 사양 52행) |
| 아이콘 `check.svg`·`history.svg` | 이 lane 묶음 B가 추가 | - | 31·32 | git lane은 같은 파일을 재사용합니다(git 사양 2.4 목록) |
| 팔레트 `search.find`/`search.replace`(commands-ui-11·12) | 이 lane의 37이 부수적으로 해소 | - | 37 | 4장 D8 |

## 2. ID별 사양

### 2.0 공통 설계

#### 2.0.1 파일 소유

- 새 앱 모듈(`native/taide-native-app/src`, kebab 파일 + `#[path]` 관례)
    - `domains/explorer_search.rs`: 탐색기·검색 영역 진입점. region 준비/적용, host 응답 반영, intent 처리, 영역 상태(`tree_failures`, mount별 탐색기, 검색 세션, Git 데코레이션 입력), `forget_project`를 둡니다.
    - `search-panel.rs`(+`search-panel-tests.rs`): 검색 패널 상태·렌더·Output, 질의 비교.
    - `search-results.rs`(+tests): 결과 누적기, 행 평탄화, 높이 추정, 가상 목록 렌더.
    - `search-history.rs`(+tests): 최근 검색 계산과 쓰기 큐.
    - `search-stream.rs`: host 배치 버퍼(2.0.2).
    - `search-editor.rs`(+tests): 4장 D4가 A일 때.
    - `explorer-git-decorations.rs`(+tests): 7의 순수 파생과 데코레이션 입력 저장소.
    - `explorer-auto-reveal.rs`(+tests): 25의 결정.
    - 실제 App 검사: `application-search-panel-tests.rs`, `application-explorer-search-tests.rs`.
- 수정하는 영역 파일: `explorer.rs`, `symbol-sidebar.rs`, `navigation-icons.rs`와 `resources/icons/*.svg`, UI `command-registry.rs`(`Run` 추가와 그 검사), `command-dispatch.rs`(find 대체).
- 공용 파일은 영역 블록 끝에 추가만 합니다: `application.rs`(영역 호출), `lib.rs`(모듈 목록), 공용 enum 루트, `presentation-refresh.rs`(Appearance 필드).
- 손대지 않는 파일: `title-branch.rs`(제목줄 branch는 그대로, git lane이 `git-cache.rs`로 옮길지 정함), git lane 파일 전부.
- crates 변경은 없을 것으로 봅니다. 필요한 backend는 `crates/taide-runtime/src/search_actions.rs`, `crates/taide-search`, `crates/taide-runtime/src/tree_actions.rs`, `crates/taide-runtime/src/layout_actions.rs`, `crates/taide-settings`에 모두 있습니다. crate를 바꾸면 `src-tauri`도 영향을 받으므로, 바꿔야 할 일이 생기면 멈추고 메인에 알립니다.

#### 2.0.2 검색 실행 경로(31·32·33·36·38 공통)

- host 작업자는 명령을 하나씩 처리합니다(`HostBridge::connect_with_integrations`의 직렬 루프). 응답 채널 용량은 `HOST_COMMAND_CAPACITY`(64)이고 다른 응답과 공유합니다. 그래서 검색 배치를 파일마다 `HostReply`로 보내면 안 됩니다. 운영 문서 1장의 host 큐 포화 흔들림을 키웁니다.
- 실행: `HostCommand::ExplorerSearch(Run{owner, session, generation, project, query})`를 받으면 `HostCommand::GitBranch` 처리처럼 `services.tasks.spawn_transient`로 넘기고 바로 반환합니다(spawn이 거부되면 그 자리에서 `Finished{result: Err}`). 작업 안에서 `search_actions::search_run(SearchRunContext{state, store: &services.search, tasks}, project, owner, session, query, on_match)`를 부릅니다(호출 예: `remote-search.rs`의 `"search_run"` 분기). `search_run`은 future가 drop되면 `SearchRequestCancellation`으로 토큰을 표시합니다.
- `on_match`는 blocking 스레드에서 불립니다(`Send + 'static`). `search-stream.rs`의 `Streams`(`Arc<Mutex<HashMap<(owner, session), Stream>>>`, `Stream{generation, batches: Vec<SearchFileMatches>}`)에 넣고 repaint를 요청합니다. UI는 원본처럼 50ms마다(`src/entities/search/use-search-run.ts:31` `RESULT_FLUSH_INTERVAL_MS`) 꺼내 누적기에 붙입니다. 시각은 egui input time과 `request_repaint_after`로 정합니다. 원본이 rAF 대신 타이머를 쓴 이유(보조 창이 그려지지 않아도 결과가 쌓여야 함, 같은 파일 24-30 주석)와 같게, 버퍼는 창 그리기와 무관하게 쌓이고 그 창이 다음에 그릴 때 꺼냅니다.
- 끝: `HostReply::ExplorerSearch(Finished{owner, session, generation, result: AppResult<u32>})` 하나만 보냅니다. 받은 뒤 남은 버퍼를 비우고 total을 반영합니다(`use-search-run.ts:152-158`).
- 대체: 같은 `(owner, session)`의 새 실행 전에 취소를 따로 보내지 않습니다. backend `begin`이 이전 토큰을 표시합니다(`use-search-run.ts` 52-58 주석, `search_actions.rs`의 `search_run` 안 `store.begin`). 명시 취소를 먼저 보내면 새 토큰을 끄는 경합이 원본에서 실제로 있었습니다. UI는 generation으로 늦은 배치와 응답을 버립니다(`use-search-run.ts:131-164`).
- 취소: 표면이 사라질 때만 보냅니다(`use-search-run.ts:166-175`, 실행 중일 때만). `Cancel{owner, session}` → `search_actions::search_cancel`. 실행 중이던 상태는 completed로 기록합니다.
- 교체: `Replace{owner, session, generation, project, query, replacement, paths}` → spawn_transient → `search_actions::search_replace` → `Replaced{.., result: AppResult<SearchReplaceResult>}`.
- owner는 창 label입니다. 메인은 `main`, 보조 창은 그 창의 `editor-<n>` label입니다(`src/entities/search/search.ipc.ts:7-14`). session은 패널이면 `search-panel-<mount ShellSlotId>`, 검색 편집기면 탭 id입니다.
- 일치 열기: `open_problem`(host)과 같은 경로를 씁니다. 탭은 preview, 대상 pane은 그 창 pane(원본 `withCurrentWindowTarget`, `src/entities/layout/layout.query.ts:106-109`; 메인은 layout focused pane, 보조 창은 그 창 focused pane), 응답은 `HostReply::ProblemOpened` 처리처럼 `Reveals::queue`로 위치를 보입니다. `SearchLineMatch.column`은 1부터 시작하는 UTF-16 열입니다(`crates/taide-model/src/search.rs:115-119`). reveal 위치와 같은 단위라 변환하지 않습니다.
- 미리보기 강조: `match_start/match_end`는 `preview` 안의 0부터 시작하는 UTF-16 오프셋입니다(`search.rs:120-124`). egui 문자열을 자르기 전에 `char::len_utf16` 누적으로 byte 오프셋으로 바꿉니다. surrogate 경계 안쪽 값은 다음 문자 경계로 올립니다.

#### 2.0.3 mount별 탐색기 상태(8·25·30 공통, 4장 D6)

- 원본은 `ExplorerContainer`가 창·slot마다 하나라 선택·초안·이름 변경·비교 대상·자동 reveal 기록이 컨테이너별입니다(`src/widgets/explorer/explorer-container.tsx:58-61`, `src/widgets/explorer/use-explorer-auto-reveal.ts:55`). 트리 page(`TREE.ROWS`)는 프로젝트별로 공유합니다(`src/entities/tree/tree.query.ts:18-23`).
- native는 `explorers: HashMap<ProjectId, Explorer>`(앱 상태)라 같은 프로젝트의 메인 slot과 보조 창이 선택·초안을 공유합니다(`docs/quality-assurance/2026-10-27-native-auxiliary-window.md` 165). `begin_explorer`가 `explorers.remove(project)`로 꺼내고, `Explorer::address_requests`/`claim_requests`/`owns_requests`가 그 공유를 보정하고 있습니다.
- 설계(D6 A): 키를 mount(`ShellSlotId`, 보조 창은 `auxiliary-windows.rs`의 그 창 `explorer` mount)로 바꿉니다. 트리 page(`trees`)·`tree_loading`은 `ProjectId` 키를 유지합니다. `CreateRequest`·`RenameRequest`·`explorer_clipboard::Request`에 mount를 넣고, 응답(`HostReply::TreeToggled`/`ExplorerPathRevealed`, LSP reply의 create·rename·paste 완료)은 mount로 찾습니다. mount 수명은 `symbol_sidebar::Views::reconcile`처럼 slot 트리와 보조 창 목록으로 정리합니다. 삭제 확인(`pending_entry_delete`)은 앱 하나라 그대로 둡니다. `forget_project`는 그 프로젝트를 보던 mount 항목을 모두 지웁니다(1.3 shell-37).
- region 규칙은 유지합니다. `explorer`(region 렌더, `&self`)는 host를 읽기만 하고, 변경은 `ExplorerRegion` 출력으로 남겨 `finish_explorer`가 적용합니다. panic이면 `discard_explorer`가 새 mount로 시작합니다(shell-12). 검색 패널 상태도 같은 규칙으로 `symbol_sidebar::Sidebar` 안에 둡니다(`begin_explorer`의 `symbol_sidebars.take`로 꺼내짐).

#### 2.0.4 Git 데코레이션 경계(7, 4장 D1)

- 원본: 탐색기는 `gitStatusQueryOptions(projectId)`(`src/entities/git/git.query.ts:105-111`, `retry: false`)를 직접 구독합니다. `fs:changed`·`git:statusChanged`에 무효화됩니다(`src/app/providers/ipc-sync-provider.tsx:531`). 실패하거나 저장소가 아니면 data가 없어 데코레이션이 비고, 이 탐색기 경로에서는 오류 표시가 없습니다(`explorer-container.tsx:65,90-91`).
- 현재 native: 제목줄만 `HostCommand::GitBranch(project)` → `git_actions::git_status`를 transient로 부르고 branch만 남깁니다(`title-branch.rs`의 `Branches`, 무효화는 `Changes`가 `GitStatusChanged`·`GitRefsChanged`·`FsChanged`를 기록). 프로젝트별 status 캐시는 앱에 없습니다.
- git lane 설계(git 사양 2.3·git-38): `git-cache.rs`의 `GitCache`가 프로젝트별 status를 관찰 기반으로 조회·무효화하고 `status(project) -> Option<&AppResult<GitStatus>>`를 줍니다. 원본처럼 Git 패널·gutter·탐색기가 같은 캐시를 읽습니다.
- 경계(D1 A)
    - 이 lane(wave 1)이 제공
        - `explorer-git-decorations.rs`: `Decoration`(6종), `build(rows: &[StatusRow], root: Option<&str>) -> HashMap<String, Decoration>`(원본 규칙, 7 설계).
        - 입력 저장소 `GitDecorations`(영역 상태): `set(project, Option<&GitStatus>)`가 상태 행 목록이 바뀐 때만 다시 계산해 프로젝트별로 보관합니다. `None`이면 그 프로젝트 항목을 지웁니다(원본: 오류·비저장소면 데코레이션 없음). `mounted_projects()`는 지금 Files 보기로 그려지는 탐색기의 프로젝트 목록을 줍니다.
        - 행 렌더: `Explorer::show_with_icons`가 데코레이션 맵 참조를 받아 색·배지·접근성 설명을 그립니다.
        - wave 1에는 공급자가 없으므로 입력 저장소가 늘 비고, 화면은 원본의 "status 없음"과 같습니다.
    - git lane(wave 2)이 제공
        - 매 tick `mounted_projects()`의 각 프로젝트에 `GitCache::observe(status)`를 걸고, 값이나 세대가 바뀌면 `GitDecorations::set`을 부릅니다. 호출 지점은 `domains/git.rs`이고, 이 lane 파일에는 손대지 않습니다.
        - git-38의 실제 App 검사와 7·git-38 행 판정.
    - 이 lane은 공개 API(`GitDecorations::set`, `mounted_projects`)만 고정하고 Git 패널용 기능은 넣지 않습니다.

#### 2.0.5 표시 토큰·아이콘·문구

- Tailwind 환산: `text-xs` 12px/줄 16px, `size-6` 24px, `size-5` 20px, `size-4` 16px, `size-3.5` 14px, `size-3` 12px, `size-1.5` 6px, `px-2` 8px, `px-1.5` 6px, `px-3` 12px, `px-4` 16px, `py-1.5` 6px, `py-2` 8px, `py-0.5` 2px, `gap-1` 4px, `gap-1.5` 6px, `gap-2` 8px, `rounded-sm` 4px(`src/shared/styles/global.css` 156 `--radius: 0.375rem`, 293 `--radius-sm: calc(var(--radius) - 2px)`).
- 테마 키(모두 `crates/taide-theme/src/service.rs` 기본 테마에 있음): `panel.background`, `panel.inputBackground`, `panel.inputBorder`, `panel.matchHighlight`, `app.focusBorder`, `app.border`, `app.foreground`, `appSidebar.iconDefault`, `explorer.background`, `explorer.itemHover`, `explorer.itemSelected`, `explorer.itemFocused`, `explorer.gitAdded`, `explorer.gitModified`, `explorer.gitDeleted`, `explorer.gitUntracked`, `git.renamed`, `git.conflicted`, `editor.background`, `input.border`, `button.primaryBackground`, `button.primaryForeground`, `tabBar.tabBorder`.
- 아이콘 등록부는 셋입니다.
    - 앱 `navigation_icons::Icon`: `resources/icons/*.svg`(`EXTRA_SOURCES`)와 `base()`로 잇는 `problems_icons::Glyph`. 사이드바 region이 쓰는 등록부(`ExplorerRegion.icons`)라 검색 패널 아이콘은 여기에 variant를 더합니다. 회전(`ROTATION_CENTER`)을 지원합니다.
    - UI `glyph_icons::Glyph`: `resources/problems/*.svg`. 이미 `X`, `ChevronRight`, `TriangleAlert`, `Terminal`, `FileDiff`, `FileSearchCorner`가 있습니다.
    - UI `icons::Icon`: `Loader`(`resources/icons/loader-circle.svg`, 원본 `Loader2`), `Search`, `Close` 등.
- 새로 옮길 lucide SVG(원본 `node_modules/lucide-react/dist/esm/icons/<name>.mjs`의 iconNode): `case-sensitive`, `whole-word`, `regex`, `eye-off`, `history`, `replace-all`, `check`. 라이선스는 이미 있는 `resources/icons/LICENSE.txt`(Lucide ISC) 범위라 새 의존성이 아닙니다. 그대로 쓰는 것: `search.svg`, `loader-circle.svg`, `file-search-corner.svg`(`FileSearch2`의 원본, `file-search-2.mjs`가 재수출), `Glyph::X`·`ChevronRight`·`TriangleAlert`.
- 문구 키는 `crates/taide-locale/resources/locales/{ko,en,ja}.json`에 모두 있습니다(검색·검색 편집기·탐색기 메뉴·알림 키 확인). 새 키는 만들지 않습니다.

#### 2.0.6 검사 방식

- UI egui 하네스: `native/taide-native-ui`, `--features native-host,inspection`, `--lib` 또는 `--test workbench`.
- 앱 lib 실제 App: `native/taide-native-app` `--lib -- <필터> -- --test-threads=4`. 실제 `NativeApplication`과 합성 temp 프로젝트를 씁니다. fixture는 `application-auxiliary-window-tests.rs`의 `Fixture`·`isolated_state`·`insert_project`·`project_ref`·`launch`를 재사용합니다.
- 순수 모듈 단위: 앱 lib의 `#[cfg(test)]` `-tests.rs`. 원본 TS 단위 검사(아래 각 ID에 적은 파일)의 사례를 그대로 옮깁니다.
- target은 lane worktree의 `target-lane`입니다.
- 금지: GUI 실행, 사용자 데이터·설정·clipboard·Keychain, 실제 휴지통(`tests/explorer-delete.rs` 실행 금지, 16은 허락 뒤), 실제 OS 알림 전송, git CLI(`taide_git::service::init`은 `run_git`이라 쓰지 않음), 전역 git 설정을 읽는 commit(`service::commit`의 `repo.signature()`).

#### 2.0.7 원본 결함·기술 한계 비재현 판단

- 34: WKWebView는 IME 조합 이벤트를 주지 않아 원본은 조합 중 글자로도 검색합니다(`src/widgets/search-panel/search-panel-container.tsx:191-198` 주석). native는 조합 중(preedit가 비지 않음)에는 디바운스를 시작하지 않고 commit 뒤에 시작합니다(D9).
- 35: 원본은 표면마다 쓰기 큐가 있어 패널과 검색 편집기가 같은 왕복 안에 쓰면 서로 덮을 수 있습니다. native는 앱 하나의 큐를 씁니다.
- 26·25: 원본 reveal 실패는 처리하지 않은 promise라 사용자 표시가 없습니다. 26은 native 기존처럼 상태줄 기록을 유지하고(차이로 기록), 25의 조용한 reveal은 원본처럼 아무것도 남기지 않습니다.

---

### 2.1 묶음 A: 탐색기 행·상태 보정

#### explorer-search-18 드래그로 이동

- 원본 동작: 없습니다. `src/features/explorer`, `src/widgets/explorer`에서 drag/dnd 처리는 0건입니다(`rg -i "drag|dnd"` 결과는 `open-to-the-side-plan.ts:7` 주석 1건). 2026-10-06 감사(`docs/quality-assurance/2026-10-06-native-audit-explorer-search.md` 3.1)도 n/a입니다.
- native 자산: `explorer_move.rs`는 이름 변경의 이동·LSP 조율이며 drag 소비자가 아닙니다.
- 빠진 것: 없음.
- 설계: 코드 변경 없음. 대응표 행을 `not-applicable`로 바꾸고 근거를 비고에 적습니다. 머리 집계를 함께 고칩니다: 웹 기술/내부 지표 전용 10→11행, 기능 분모 588→587, 미완료 274→273, 미연결 110→109, explorer-search 영역 행 unwired 4→3·not-applicable 0→1.
- 검사: 없음(문서 변경).
- 완료 판정: 행 상태와 집계가 갱신됨.

#### explorer-search-26 탭 메뉴의 "탐색기에서 보기"·"이름 바꾸기"

- 원본 동작
    - 발행: `src/widgets/editor-area/pane-tab-bar.tsx:226-227`. 파일 탭만 `requestRevealInExplorer(path)`·`requestRenameInExplorer(path)`를 보냅니다.
    - 수신: `src/widgets/explorer/explorer-panel.tsx:151-173`. 포커스 slot만(보조 창은 항상) 보기를 files로 바꾸고 요청을 넘깁니다.
    - 처리: `explorer-container.tsx:230-241`. `tree_reveal` 응답 page로 선택·스크롤합니다(`selectPathRequest`). 이름 바꾸기는 응답 page에서 행을 찾아 `crud.startRename`, 없으면 무동작입니다.
    - 펼침: `src/widgets/project-shell/project-shell.tsx:98-101,122-123`(포커스 slot만), 보조 창 `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx:107-108`.
    - 실패: `void revealTreePath(...)`라 표시 없음.
- native 자산
    - UI: `ShellIntent::RevealInExplorer{project, slot, path, rename}`, `RevealInAuxiliaryExplorer`.
    - App: `show_explorer_files`, `show_auxiliary_explorer_files`(보기 Files, 접힘 해제).
    - host: `HostCommand::RevealExplorerPath{project, mount, path, rename}` → `tree_actions::tree_reveal` → `HostReply::ExplorerPathRevealed` → `Explorer::address_requests`·`select_path`·`start_rename`.
    - 검사: `application-tab-menu-tests.rs` `실제_app_탭은_에이전트_터미널_glyph와_다시열기_탐색기_reveal_rename을_반영한다`, `application-auxiliary-window-tests.rs` `실제_app_탭_메뉴는_탭을_새창_기존창_메인창으로_옮기고_보조창_탐색기로_reveal_rename한다`·`실제_app_보조_창_탐색기_reveal_rename은_메인_사이드바가_열려_있어도_그_창에서만_스크롤과_포커스를_받는다`, UI `tests/workbench.rs`의 `RevealInExplorer` 단언. 기록은 `docs/quality-assurance/2026-10-27-native-tab-icons-menu.md`, `2026-10-27-native-auxiliary-window-moves.md`입니다.
- 빠진 것: 대응표 갱신만 남았습니다. 실패 표시 차이는 2.0.7에 적었습니다.
- 설계: 30의 mount별 상태로 응답 주소가 `mount` 키 조회로 바뀌는 것 외에는 변경이 없습니다.
- 검사: 30 변경 뒤 `-- tab_menu auxiliary_window` 필터와 workbench를 1회 실행합니다.
- 완료 판정: 위 검사 통과, 행 complete.

#### explorer-search-44 Problems 열기/닫기(보조 창 고정 닫힘)

- 원본 동작: 상태바 오류 개수 버튼(`src/widgets/window-chrome/status-bar-content.tsx`), slot별 열림(`app-shell.tsx:79`, `shell-slot-tree-view.tsx:96-97`), 편집기 영역 분할(`editor-area.tsx:529-532`). 보조 창은 상태바가 없고 `isProblemsOpen={false}`로 고정입니다(`auxiliary-window-shell.tsx:139`).
- native 자산: `problems.rs` `Views::is_open/toggle/reconcile/status_button/show_panel`(slot별), `ShellSurfaces::problems_open/problems_panel`. 셸의 `WindowScope::Auxiliary` 분기는 제목줄·탐색기·pane 트리만 그리고 Problems·상태바를 호출하지 않습니다.
- 빠진 것: 보조 창 고정 닫힘을 단언하는 검사.
- 설계: 코드 변경은 예상하지 않습니다.
- 검사
    - UI workbench: 같은 프로젝트의 메인 slot을 `problems_open`에 넣고 `WindowScope::Auxiliary`로 그렸을 때 `problems_panel` 호출 0회, 상태바 영역 없음.
    - App: 메인 focused slot에서 Problems를 열고 보조 창 viewport 출력에 Problems 제목·심각도 버튼 문구가 없고, 메인 slot에는 있음.
- 완료 판정: 두 검사 통과, 행 complete.

#### explorer-search-8 선택/포커스 시각 구분

- 원본 동작
    - `src/features/explorer/file-tree-row.tsx:61-64`: 모든 행에 `hover:bg-explorer-item-hover`, 선택이면 `focused ? bg-explorer-item-selected : bg-explorer-item-focused`입니다.
    - `focused`는 "primary 선택 행이면서 트리 컨테이너에 focus가 있음"입니다(`file-tree.tsx:116,355-356,404`). 그래서 트리에 focus가 있어도 다중 선택의 나머지 행은 `explorer.itemFocused`입니다.
    - focus 판정은 React `onFocus/onBlur`(버블링)라 트리 안 입력(초안 행)도 포함하고, 메뉴가 열려 focus가 portal로 가면 빠집니다.
    - hover 우선: Tailwind v4에서 `hover:bg-*`(클래스+`:hover`, 0,2,0)가 `bg-*`(0,1,0)보다 우선이라 선택 행에 포인터를 올리면 hover 색입니다. 4장 D7.
- native 자산: `explorer.rs` `render_mounted`가 `selected || hovered`일 때 `ui.visuals().selection.bg_fill` 또는 `widgets.hovered.weak_bg_fill`을 칠합니다. focus는 `memory.has_focus(tree_id)`(`native-tree-focus`), primary는 `Explorer.selected`입니다. `explorer::Appearance`에는 파일 색만 있습니다. 같은 규칙의 선례는 `symbol-outline.rs` `Appearance{selected, focused, hover}`입니다.
- 빠진 것: 테마 토큰과 primary·focus 규칙.
- 설계: `explorer::Appearance::new`에서 `explorer.itemSelected/itemFocused/itemHover`를 읽습니다. 행 바탕은 D7 A면 hover > (primary && 트리 focus → selected) > (선택 → focused) 순서입니다. 트리 focus에는 트리 안 초안·이름 변경 입력의 focus도 포함합니다. 7의 글자색 변경과 같은 함수(`render_mounted`)라 같은 묶음에서 한 번에 고칩니다.
- 검사: App lib에서 `Explorer::show`를 egui로 그려 행 rect의 바탕 RectShape 색을 확인합니다(`symbol-sidebar-tests.rs` 선택색 확인 방식). 경우는 4가지입니다: focus 있는 primary, focus 없는 선택, 다중 선택의 primary 아닌 행, 선택 행 hover.
- 완료 판정: 4가지가 테마 토큰과 일치, 기존 `tests/explorer.rs` 회귀 통과.

#### explorer-search-30 보조 창의 탐색기 패널

- 원본 동작: `auxiliary-window-shell.tsx:82`(창별 접힘 상태), 96-108(⌘B·explorer·git 키와 4개 bridge의 펼침), 125-141(기본 240px, 최소 180px, 최대 40%, 접힘 가능, `ExplorerContainer zen=false`). 컨테이너가 창마다 따로라 선택·초안·검색 패널이 창별입니다.
- native 자산: 셸 `WindowScope::Auxiliary` 분기의 `egui::Panel::left`(`EXPLORER_DEFAULT_WIDTH`, 최소, `EXPLORER_MAX_FRACTION`), `ShellSurfaces::is_auxiliary_explorer_open`·`auxiliary_explorer`, `auxiliary-windows.rs`(`explorer` mount, `is_explorer_open`, `show_explorer`, `toggle_explorer`, `explorer_mounts`). 보조 창 실제 App 검사 6건이 `application-auxiliary-window-tests.rs`에 있습니다.
- 빠진 것: (a) mount별 탐색기 상태(2.0.3), (b) 보조 창 Search 보기 본문(묶음 B), (c) Git 보기 본문(git lane).
- 설계: 2.0.3을 그대로 적용합니다. 보조 창 Search 보기는 묶음 B의 패널이 `ExplorerMount::Auxiliary`에서도 같은 코드로 그립니다. 보조 창 owner label은 `auxiliary-windows.rs` 창의 label입니다.
- 검사(App)
    - 같은 프로젝트를 메인 slot과 보조 창에 띄웁니다. 메인에서 행 선택·이름 변경 시작 → 보조 창 행은 선택 바탕·입력이 없습니다.
    - 보조 창에서 새 파일 초안을 만들고 확정 → 그 창만 새 행 선택·reveal, 메인 선택은 그대로입니다.
    - `forget_project` 뒤 그 프로젝트 mount 항목이 남지 않음.
    - 기존 보조 창 검사 6건 회귀.
    - 묶음 B 뒤: 보조 창 Search 보기에서 검색하고 일치를 열면 그 창 focused pane에 열립니다.
- 완료 판정: 위 검사 통과. Git 보기 본문은 git-1 행에 남긴다고 비고에 적고 complete. 판정은 묶음 B 끝에 합니다. shell-5 비고의 "탐색기 창별 분리" 항목도 같은 근거로 갱신합니다.

#### explorer-search-7 Git 데코레이션(이 lane 몫: 파생·렌더·주입점)

- 원본 동작
    - 파생 `src/widgets/explorer/file-tree-git-status.ts`
        - 7-15: 종류 매핑. modified·added·deleted·renamed·untracked는 같은 이름, `typeChange`→modified, `conflicted`→conflicted.
        - 24-31: 우선순위 conflicted 6 > added 5 > untracked 4 > renamed 3 > modified 2 > deleted 1.
        - 33-38: 행 하나는 `isConflicted`면 conflicted, 아니면 staged·unstaged 중 높은 쪽, 둘 다 없으면 없음.
        - 48-66: 키는 `StatusRow.absPath`(절대 경로). 파일 자기 자리와, 프로젝트 root 아래(`startsWith(root + '/')`)의 모든 조상 디렉터리에 더 높은 우선순위만 기록합니다. root 자신은 제외하고, 형제 접두사(`/a/b-c`와 `/a/b`)는 섞이지 않습니다. root가 없으면 빈 맵입니다.
    - 표시 `src/features/explorer/file-tree-row.tsx`
        - 24-32: 행 글자색(행 컨테이너 class라 이름·chevron·배지에 같이 적용, `currentColor`). added→`explorer.gitAdded`, modified→`explorer.gitModified`, deleted→`explorer.gitDeleted`, renamed→`git.renamed`, untracked→`explorer.gitUntracked`, conflicted→`git.conflicted`. 파일·폴더 타입 아이콘은 자기 색을 유지합니다.
        - 34-41, 77-81: 오른쪽 끝(`ml-auto`, 왼쪽 4px, 행 오른쪽 여백 8px). 파일은 글자 A/M/D/R/U/!(12px), 디렉터리는 지름 6px 원(`bg-current`). 접근성 이름은 상태 이름(`modified` 등)입니다.
        - `ignored`는 타입·색만 있고 파생이 만들지 않습니다. 넣지 않습니다.
    - 공급: `explorer-container.tsx:65,90-91`(`gitStatus?.rows ?? []`, 오류·저장소 아님이면 데코레이션 없음).
- native 자산: `taide_model::git::{GitStatus{rows}, StatusRow{abs_path, staged, unstaged, is_conflicted}, GitChangeKind}`. `TreeRow`에는 Git 필드가 없습니다. 상태 조회는 제목줄용(`HostCommand::GitBranch`, `title-branch.rs`)뿐입니다. 행 렌더는 `explorer.rs` `render_mounted`와 `RowIcon::paint`(chevron 색은 `ui.visuals().text_color()`)입니다.
- 빠진 것: 파생, 행 색·배지, 입력 주입점, 공급(git lane).
- 설계(D1 A)
    - 파생: `explorer-git-decorations.rs`의 `Decoration`(6종)과 `build(rows, root)`. 원본 규칙을 그대로 옮깁니다. 경로 비교는 문자열 그대로(`root + "/"` 접두사)입니다.
    - 입력 저장소: 2.0.4의 `GitDecorations`. region 렌더는 `&self`에서 그 프로젝트 맵을 읽기만 합니다.
    - 렌더: `Explorer::show_with_icons`에 `Option<&HashMap<String, Decoration>>`를 넘깁니다(`RowIcons` 확장 또는 인자 추가). 행 이름과 chevron은 데코레이션 색, 배지는 오른쪽에 그리고 이름 truncate 폭에서 배지 폭(왼쪽 4px 포함)을 뺍니다. 접근성은 행 노드 description에 상태 이름을 둡니다.
    - 색은 `explorer::Appearance`에 6개 토큰을 더합니다.
- 검사(이 lane)
    - 단위: `src/widgets/explorer/file-tree-git-status.test.ts`의 9개 사례(unstaged 매핑, untracked·typeChange, staged added 우선, conflicted, 등재 안 함, 조상 전파와 root 제외, 디렉터리 최우선, 형제 접두사, root 없음).
    - 렌더(App lib, egui): 합성 맵(`src/new.rs`=untracked, `src`=untracked 전파, 다른 파일 modified)을 넣고 `Explorer::show_with_icons`를 그려 이름 galley 색, 배지 글자 `U`/`M`, 디렉터리 원 shape와 색, 배지가 이름과 겹치지 않음, 행 접근성 description을 확인합니다. `set(project, None)` 뒤 데코레이션이 사라짐.
    - 주입점: `mounted_projects()`가 Files 보기 mount의 프로젝트만 돌려줌(Search·Outline 보기, 접힌 사이드바 제외).
- 검사(git lane, git-38에서 실행)
    - 제목줄 검사(`실제_app_제목줄은_프로젝트_git_branch를_읽고_git_변경_뒤_다시_읽는다`)처럼 `.git` 골격(HEAD·objects·refs)을 손으로 만든 fixture에 `src/new.rs`를 둡니다. untracked가 되면 행 글자색 `explorer.gitUntracked`, 배지 `U`, `src` 디렉터리 점 같은 색, root 행 없음. 파일을 지우고 FsChanged → 다시 조회 → 데코레이션이 사라짐.
    - 경로 정규화: macOS temp(`/var` → `/private/var`)에서 `abs_path`와 프로젝트 root가 어긋날 수 있으므로 fixture root를 `canonicalize`해 넣습니다.
    - added 사례는 `taide_git::service::stage`가 git CLI를 쓰지 않을 때만 추가합니다.
- 완료 판정: 이 lane에서는 단위·렌더·주입점 검사 통과 후 행을 partial로 두고 비고에 "파생·렌더·주입점 완료, 공급 연결은 git-38"을 적습니다. complete는 git lane이 git-38과 함께 판정합니다.

#### explorer-search-25 활성 탭 자동 reveal(`explorerAutoReveal`)

- 원본 동작
    - 결정 `src/widgets/explorer/explorer-auto-reveal.ts:30-41`: 설정 꺼짐·활성 경로 없음·사이드바 안 보임·files 보기 아님이면 skip, 직전에 실행한 경로와 같으면 skip, 행이 page에 있으면 select-only, 없으면 reveal-then-select.
    - 입력 `use-explorer-auto-reveal.ts:57-65`
        - 활성 경로는 이 창 pane 트리의 focused pane 활성 탭이 파일일 때의 경로입니다(`pane-tree.ts:102-105`).
        - 프로젝트 root 아래일 때만 씁니다(`toRelativePath`가 바뀌는 경우).
        - enabled는 `settings.explorerAutoReveal ?? true`, 사이드바 보임은 `!zen && !sidebarCollapsed`(보조 창은 창별 접힘)입니다.
    - 실행 73-91: 결정 전에 경로를 기록합니다. select-only는 선택·최소 스크롤만 하고 focus는 옮기지 않습니다. reveal-then-select는 `tree_reveal` 뒤 선택하며, 실패는 조용히 무시합니다. 사이드바를 펼치거나 보기를 바꾸지 않습니다(`project-shell.tsx:92-97`).
    - 재계산 93-96: 설정·활성 경로·rows·보임·보기가 바뀔 때마다.
- native 자산: 설정 모델·설정 화면 스위치(`crates/taide-model/src/settings.rs` 261, UI `settings-controls.rs` `ExplorerAutoReveal`). 활성 탭 조회 `symbol_sidebar::window_tree` + `taide_native_ui::snapshot::active_tab`(아웃라인이 사용, `explorer` region의 Outline 분기). 선택·스크롤 `Explorer::select_path`(최소 스크롤 `revealing_scroll_offset`). reveal host 경로 `HostCommand::RevealExplorerPath`.
- 빠진 것: 결정 모듈, mount별 마지막 경로, 조용한 reveal.
- 설계
    - `explorer-auto-reveal.rs`에 `decide(Input) -> Decision{Skip, SelectOnly, RevealThenSelect}`를 둡니다.
    - mount별 `last_revealed: Option<String>`은 mount별 `Explorer`(D6)에 둡니다.
    - native는 사이드바가 안 보이면 탐색기 region 자체를 그리지 않으므로, 그때는 결정이 skip이고 기록도 남지 않습니다. 원본과 결과가 같습니다.
    - 보임 판정: 메인은 window zen과 `layout.shell_view.sidebar_collapsed`, 보조 창은 `is_explorer_open`입니다.
    - reveal은 `RevealExplorerPath`에 `quiet`(또는 origin) 플래그를 더해 실패 시 상태줄·toast를 남기지 않고, 보기 전환·펼침을 하지 않습니다. 성공 응답은 그 mount에만 `select_path`하고 focus는 요청하지 않습니다.
- 검사
    - 단위: `src/widgets/explorer/explorer-auto-reveal.test.ts` 13개 사례.
    - App(합성 프로젝트 `src/deep/a.rs`, 폴더 접힘)
        - a.rs 탭 활성 → 조상 펼침·행 선택, 편집기 focus 유지.
        - 설정 끄고 탭 전환 → 선택 불변.
        - 사이드바 접힘에서 탭 전환 → 변화 없음, 펼치면 reveal.
        - Search 보기 → 변화 없음.
        - 두 slot이 같은 프로젝트면 각자 자기 활성 파일을 따릅니다.
        - 보조 창은 그 창 활성 탭을 따릅니다.
        - 없는 경로 reveal 실패 → toast·상태줄 없음.
- 완료 판정: 단위·App 통과. settings-80 비고에 근거 위치를 남깁니다(행 갱신은 settings lane).

#### explorer-search-27 프로젝트 루트 누락 안내와 "프로젝트 다시 열기"

- 원본 동작
    - 조건 `explorer-container.tsx:88`: `rootUnavailable = project.rootMissing || isTreeRowsError`.
    - 화면 `explorer-panel.tsx:202-221`: 머리줄(프로젝트명 대문자·툴바)은 그대로입니다. 본문은 가운데 정렬 세로 배치(간격 8px, 좌우 16px, 12px, `appSidebar.iconDefault`)입니다. 20px `TriangleAlert`, `explorer.projectRootMissing`(`app.foreground`), 프로젝트가 있으면 root 전체 경로(`break-all`), 밑줄 버튼 `explorer.reopenProject`(`app.foreground`) 순서입니다.
    - 다시 열기 `explorer-container.tsx:243-253`: `close_project(projectId)` 성공 뒤 `open_project(root)`. 오류는 각각 toast(`describeIpcError`)입니다. 다시 여는 프로젝트는 새 id입니다.
- native 자산
    - `ProjectRef.root_missing`(앱 `projects` 목록).
    - 트리 실패: `HostReply::Tree` Err와 `HostReply::TreeSynced` result Err는 상태줄 문자열만 남깁니다.
    - page가 없고 로딩 중이 아니면 region이 매 프레임 `is_tree_requested`를 켜서, 실패한 프로젝트를 계속 다시 요청하고 스피너를 그립니다(`explorer` region의 Files 분기, `tree_loading`·`load_trees`).
    - host: `HostCommand::CloseProject`(view state·layout flush 뒤 `project_actions::project_close`)와 `HostCommand::OpenProject(root)`(`validate_open_root` 뒤 `NativeProjects::open`).
- 빠진 것: 누락 화면, 재요청 반복 중지, 다시 열기.
- 설계
    - 영역 상태 `tree_failures: HashSet<ProjectId>`를 둡니다. 설정·해제 규칙은 29에 적었습니다.
    - Files 보기에서 `root_missing || tree_failures.contains`이면 트리 대신 누락 화면을 그립니다. 머리줄은 `Explorer`의 머리줄 코드를 분리해 그대로 씁니다.
    - 버튼은 `HostCommand::ExplorerSearch(ReopenProject{project, root})`입니다. host가 `CloseProject` 본문을 실행하고 성공하면 `OpenProject(root)` 본문을 실행합니다. 실패는 `HostReply::Failed` → `report`(toast)입니다. shell-37이 먼저 병합되면 닫기 경로는 그쪽 `close_project`(mirror flush 포함)를 거치게 맞춥니다.
    - 아이콘은 기존 `TriangleAlert` glyph입니다.
- 검사(App)
    - 부팅 때 `root_missing=true`이고 폴더가 없는 프로젝트 → 화면 문구·root·버튼(접근성 Button)이 보이고 `TreeRows` 재요청이 반복되지 않습니다.
    - 버튼 → 프로젝트가 닫히고 열기 실패 toast. 원본도 닫힌 뒤 열기가 실패합니다.
    - 폴더는 있는데 플래그만 켜진 경우 → 버튼 뒤 새 프로젝트 id로 트리가 보입니다.
- 완료 판정: 검사 통과, 29와 함께 complete.

#### explorer-search-29 트리 로드 오류 표시

- 원본 동작
    - `explorer-container.tsx:63,88`: `TREE.ROWS` 조회 실패면 27과 같은 화면입니다.
    - 조회 기본값은 `retry: 0`, `refetchOnWindowFocus: false`(`src/app/query-client.ts` 9-14)입니다. 실패는 다음 무효화 성공이나 mutation의 `setQueryData`가 올 때까지 남고, 이전 data가 있어도 화면은 오류입니다.
    - watcher 갱신은 `refreshTreeDir`가 실패하면 `TREE.ROWS`를 무효화하고 다시 조회합니다(`ipc-sync-provider.tsx:188-205`). 그 조회가 실패해야 오류 화면입니다.
    - 툴바 새로고침 실패는 처리하지 않아 표시가 없습니다(`explorer-container.tsx:305`). 삭제 뒤 부모 갱신 실패는 toast입니다(`use-explorer-entry-crud.ts:180-190`).
- native 자산: 27과 같습니다. `HostCommand::RefreshTree{dirs: None}`도 내부에서 `tree_rows`를 다시 읽습니다. 삭제 성공 뒤 부모 갱신은 LSP reply `ExplorerDeleteFinished` 처리에서 `RefreshTree{dirs: Some(parent)}`로 보냅니다.
- 빠진 것: 실패 상태, 반복 요청 중지, 출처별 처리.
- 설계
    - `tree_failures` 설정: `HostReply::Tree{offset: 0}` Err, watcher 갱신에서 온 `TreeSynced`의 page 조회 Err.
    - 해제: 그 프로젝트의 page가 하나라도 성공으로 들어올 때(`Tree`, `TreeSynced`, `TreeToggled`, `ExplorerPathRevealed`, collapse).
    - 실패 중에는 region이 `is_tree_requested`를 켜지 않습니다. 재조회는 watcher 갱신·다시 열기만 합니다.
    - `RefreshTree`에 출처(watcher·toolbar·delete)를 붙입니다. toolbar 실패는 표시하지 않고, delete 실패는 toast(16), watcher 실패는 위 규칙입니다.
- 검사(App)
    - page가 있는 프로젝트의 폴더를 실행 중에 지우고 watcher 갱신 → 누락 화면, 재요청이 반복되지 않음.
    - 폴더를 다시 만들고 갱신 → 트리 복귀.
    - 툴바 새로고침 실패 → 상태줄·toast 없음.
- 완료 판정: 검사 통과.

---

### 2.2 묶음 B: 검색 패널 핵심

#### explorer-search-38 검색 백엔드 연결

- 원본 동작: `src/entities/search/search.ipc.ts`
    - 19-28: `search_run(projectId, owner, sessionId, query, channel)`. 채널은 파일 하나에 메시지 하나이고, 파일 순서는 정해지지 않습니다.
    - 30: `search_cancel(owner, sessionId)`. 32-33: `search_replace(projectId, query, replacement, paths)`. 43: `search_list_files`.
    - backend 동작(전체 순회, 정규식·단어·대소문자, gitignore, `scopeDir`, 취소, 교체와 건너뜀 보고, 10,000건 상한)은 `crates/taide-search/src/service.rs`·`crates/taide-runtime/src/search_actions.rs`에 있고, 크레이트 검사가 있습니다.
- native 자산: `search_actions::{search_run, search_replace, search_cancel, search_list_files}`, `services.search`(`SearchStore`), 원격 경로 `remote-search.rs`(검사 `remote-content-tests.rs`), 팔레트용 `HostCommand::ListProjectFiles`.
- 빠진 것: run·cancel·replace의 host 명령과 UI 소비자.
- 설계: 2.0.2입니다. 명령은 `Run`, `Cancel`, `Replace`, 응답은 `Finished`, `Replaced`입니다.
- 검사(App, 합성 프로젝트)
    - run → 배치가 버퍼에 쌓이고 `Finished` total이 맞음.
    - 같은 session으로 두 번째 run → 첫 run의 늦은 배치·응답이 반영되지 않음.
    - 패널 제거(보기 전환) → `Cancel`이 나가고 더 이상 배치가 쌓이지 않음.
    - replace → fixture 파일이 바뀌고 binary·NUL fixture가 `skipped`에 들어감.
- 완료 판정: 31·32·33이 이 경로를 실제로 쓰면 complete입니다(`list_files`는 팔레트가 이미 사용).

#### explorer-search-31 검색 패널 본체

- 원본 동작
    - 요청과 상태 수명
        - `explorer-panel.tsx:113-115,126-140,246-258`: 요청(`includeGlob`, `scopeDir`, `seedText`, `openReplace`)과 `openNonce`는 패널 컨테이너가 들고, 프로젝트가 바뀌면 요청을 버립니다. 검색 보기가 아니면 `SearchPanelContainer`를 mount하지 않습니다. 보기를 벗어나면 입력·결과가 사라지고 실행 중 검색은 취소됩니다(4장 D5).
        - 요청 기본값은 `src/shared/lib/bridge/search-panel-bridge.ts`의 `requestOpenSearchPanel`(모든 필드 null/false)입니다. 새 요청은 이전 요청 전체를 대체합니다.
    - 컨테이너 `src/widgets/search-panel/search-panel-container.tsx`
        - 51-65: 질의, 토글 4개(기본 대소문자·단어·정규식 꺼짐, gitignore 켜짐, `src/entities/search/search.type.ts`), 제외 glob, session(`search-panel-<useId>`).
        - 73: 범위 칩 문구 `scopeDir ?? includeGlob`.
        - 77-80: 새 nonce면 `seedText`가 있을 때만 질의를 바꿉니다.
        - 82-91: 질의는 text, 토글, `includeGlob`, `excludeGlob.trim() || null`, `respectGitignore`, `scopeDir`. contextLines는 넣지 않아 0입니다.
        - 109-115: Enter 제출은 공백뿐이면 무시, 대기 중 입력 검색을 취소, 이력을 기록합니다.
    - 표시 `src/features/search/search-panel.tsx`
        - 141-142: 바탕 `panel.background`, 머리 영역 아래선 `app.border`, 좌우 8px, 상하 6px, 줄 간격 6px.
        - 144-152: 바꾸기 전환 버튼. 24px, 14px `ChevronRight`(열리면 90도), 위쪽 2px, `aria-pressed`, tooltip `search.replaceToggle`(아래), 색 `appSidebar.iconDefault`, hover `explorer.itemHover`, 4px 모서리.
        - 154-174: 질의 상자. `panel.inputBackground`, 1px `panel.inputBorder`, focus-within이면 `app.focusBorder`, 4px 모서리. 입력은 12px, 좌우 8px, 상하 6px, placeholder `search.placeholder`. 오른쪽에 최근 검색 버튼, 토글 4개 순서입니다.
        - `src/features/search/search-option-toggles.tsx:18-70`: 각 24px·14px 아이콘(`CaseSensitive`, `WholeWord`, `Regex`, `EyeOff`). 켜지면 `explorer.itemSelected` 바탕과 `app.foreground`, 꺼지면 `appSidebar.iconDefault`와 hover `explorer.itemHover`. 마지막만 오른쪽 4px. tooltip은 `search.caseSensitive`, `search.wholeWord`, `search.regex`, `search.respectGitignore`(아래).
        - `src/features/search/search-history-dropdown.tsx:18-35`: `History` 아이콘 버튼. 이력이 없으면 비활성(불투명도 0.4)이고 tooltip `search.recentSearches`. 열면 시작점 정렬 메뉴에 항목을 원래 순서로 보이고, 고르면 질의만 바꿉니다(실행하지 않음).
        - 197: 제외 glob 입력(`src/features/search/search-exclude-glob-input.tsx:9-19`). 자체 상자, placeholder `search.excludeGlobPlaceholder`, Enter 처리 없음.
        - 199-206: 검색 편집기 열기 버튼(36).
        - 208-219: 범위 칩. `explorer.itemSelected` 바탕, `app.foreground`, 좌우 6px, 상하 2px, 4px 모서리, 12px, 최대 폭에서 truncate. 문구 `explorer.searchScopeLabel{path}`, 12px `X` 버튼(tooltip `search.clearScope`)은 요청의 `includeGlob`·`scopeDir`만 지웁니다(`explorer-panel.tsx:253`).
        - 220-233: 상태 줄(12px, `appSidebar.iconDefault`).
            - 실행 중: 12px 회전 `Loader2`와 `search.searching`, 간격 6px.
            - 완료: 일치가 있으면 `search.matchCount{count, files}`(files는 파일 그룹 수), 없으면 `search.noMatches`.
            - 완료이면서 잘림(total ≥ 10,000, `src/entities/search/search-run-state.ts` `isSearchResultTruncated`, `src/shared/constants/search.ts` `SEARCH_MATCH_LIMIT`): `panel.matchHighlight` 글자로 `search.truncated{limit: 10000}`.
        - 236-266: 본문(`search-run-state.ts` `resolveSearchResultsView`). 결과가 있으면 상태와 무관하게 목록(32)입니다. 아니면 상태별로 다릅니다.
            - idle: 20px `Search`(불투명도 0.6), 간격 8px, 좌우 16px, 가운데 정렬, 문구는 입력 검색이 켜졌으면 `search.liveSearchHint`, 아니면 `search.pressEnterHint`.
            - running: 빈 본문.
            - completed: `search.noResults`. failed: `search.failed`.
        - 125-128: Enter 제출. IME 조합 중이면 무시합니다.
        - 135-138: nonce가 바뀌면(첫 mount 포함) 질의 입력에 focus하고 전체 선택합니다.
    - 실패 toast: 명시 실행 실패만 `describeIpcError` toast입니다(`use-search-run.ts:159-163`).
- native 자산: `symbol_sidebar::View::Search`(전환 버튼만, 본문 비어 있음; `explorer` region이 Search·Git이면 전환 줄만 그리고 반환), `Sidebar`(slot별, 프로젝트 바뀌면 새로 만듦), 설정 읽기(`services.state.settings`), toast(`toasts.ipc_error`), tooltip provider(`tooltips.show`).
- 빠진 것: 본문 전부.
- 설계
    - 상태: `Sidebar`에 `search: Option<search_panel::Panel>`, `search_request: Option<Request>`, `open_nonce: u64`를 둡니다. 보기가 Search가 아니게 되는 frame에 `Panel`을 drop하고, 실행 중이면 `Cancel`을 출력합니다(D5 A).
    - 렌더: `explorer` region의 Search 분기에서 `Panel::show(ui, &Inputs{settings, history, request, nonce, appearance, icons, locale}) -> Output`를 부릅니다. render는 읽기만 하고 `Output{run, cancel, open_match, open_editor, replace, clear_scope, record_history}`를 `finish_explorer`에서 적용합니다.
    - 질의 비교·정규화: `src/entities/search/search-query.ts:10-42`를 `search-panel.rs`에 옮깁니다.
    - 입력: egui `TextEdit` 단일 줄. Enter·IME는 `Explorer`의 composing 추적과 같은 방식으로 막습니다. focus·전체 선택은 nonce 변경 frame에 `request_focus`와 선택 범위로 합니다.
    - 최근 검색 메뉴: 기존 egui 팝업 메뉴(탐색기 컨텍스트 메뉴와 같은 스타일)입니다.
    - Appearance: `search_panel::Appearance::new(theme)`를 `presentation-refresh.rs`의 Appearances에 추가합니다.
    - 아이콘: 2.0.5(`navigation_icons::Icon`에 variant 추가, 스피너는 `Loader` 회전).
- 검사
    - 단위: `src/entities/search/search-query.test.ts`(8), `src/entities/search/search-run-state.test.ts`(7) 사례.
    - App
        - Search 보기 → 질의 입력·Enter → 상태 줄 `search.searching` → 완료 문구·결과.
        - 토글 4개의 `aria-pressed` 상당(selected)과 질의 반영(대소문자·정규식·gitignore·제외 glob).
        - 공백 질의 Enter 무시.
        - 이력 버튼 비활성·활성.
        - 범위 칩 표시·해제.
        - 실패 질의(잘못된 정규식) Enter → toast와 `search.failed`.
        - 보기 전환 뒤 복귀하면 빈 패널.
        - 프로젝트 전환 시 요청 폐기.
    - UI 수치: 머리 영역 rect·버튼 24px·상자 테두리 색을 shapes로 확인합니다.
- 완료 판정: 단위·App 통과, 메인·보조 창 모두에서 동작. shell-59 비고의 검색 부분을 같은 근거로 갱신합니다.

#### explorer-search-32 결과 목록

- 원본 동작
    - 누적 `src/entities/search/search-result.ts`
        - 49-67: 같은 파일에서 인접 일치의 before/after 줄이 겹치면 겹친 만큼 잘라 같은 줄을 두 번 보이지 않습니다.
        - 80-101: 배치를 도착 순서로 그룹에 붙이고, 같은 경로가 다시 오면 기존 그룹에 합친 뒤 중복을 다시 제거합니다. 누적 수를 셉니다. 경로→그룹 index 맵으로 배치당 상수 비용이며 전체 재그룹을 하지 않습니다(`search-result-budget.test.ts`).
    - 평탄화 `src/features/search/search-result-rows.ts:29-56`: 그룹 헤더와 일치 행을 한 목록으로 펼칩니다. 접힌 그룹은 헤더만 남깁니다. 행 id는 `group:<path>`·`match:<path>:<line>:<column>`. 높이는 헤더 20px, 일치 행 4 + 16 + 컨텍스트 줄 수 × 20px입니다.
    - 목록 `src/features/search/search-results-list.tsx:35-86`
        - 가상 스크롤(overscan 12), 자체 스크롤 영역과 overlay 스크롤바.
        - 접힘 상태는 목록 mount 동안만 유지됩니다(새 실행이 시작되면 결과가 비어 목록이 내려가므로 초기화).
        - 바꾸기가 열렸을 때만 헤더에 체크박스를 둡니다(`search-panel.tsx:241-245`).
    - 헤더 `src/shared/ui/file-group-header.tsx`
        - 바탕 hover `explorer.itemHover`, 12px. 체크박스가 있으면 왼쪽 8px 뒤 14px 체크박스(`input.border` 테두리, 켜지면 `button.primaryBackground` 바탕과 12px `Check`), 이어서 왼쪽 4px.
        - 버튼 부분: 상하 2px, 오른쪽 8px, 간격 4px. 12px `ChevronRight`(펼치면 90도), 14px 파일 아이콘, 파일 이름(굵게), 전체 경로(`appSidebar.iconDefault`, truncate), 오른쪽 끝 개수(tabular).
        - 클릭·Enter·Space는 접기 전환. 체크박스는 `search.selectFile{path}` 접근성 이름.
    - 일치 행 `src/features/search/search-match-row.tsx`
        - 왼쪽 32px, 상하 2px, 오른쪽 8px, 12px.
        - 줄 번호는 24px 폭 오른쪽 정렬 `appSidebar.iconDefault`, 간격 8px.
        - 미리보기는 truncate, 일치 부분은 `panel.matchHighlight` 굵게(바탕 없음).
        - 컨텍스트 줄은 `appSidebar.iconDefault` 불투명도 0.7, 상하 2px, 번호는 before가 `line - before.len + i`, after가 `line + i + 1`.
        - 클릭·Enter·Space면 일치 열기입니다. 패널은 preview 탭, 그 창 pane, reveal(`explorer-container.tsx:109-110`)로 엽니다.
- native 자산: 파일 아이콘 `problems_icons::file(name)`(탐색기와 같은 glyph), 파일 그룹·가상 목록 선례 `problems.rs`, 열기·reveal 경로 `open_problem`·`Reveals::queue`.
- 빠진 것: 전부.
- 설계: `search-results.rs`에 누적기(`Accumulator{groups, index_by_path: HashMap<String, usize>, total}`, 증분 append), `rows(groups, collapsed)`, `estimate_height`, 가상 목록 렌더(`ScrollArea::show_viewport`, overscan 12)를 둡니다. 측정 전 높이는 추정값을 쓰고, 그려진 행은 실제 높이로 보정합니다(원본 `measureElement`). 체크박스는 14px 직접 그리기(`Check` 아이콘)입니다. 일치 열기는 2.0.2의 경로입니다.
- 검사
    - 단위: `search-result.test.ts`(12), `search-result-rows.test.ts`(6) 사례, `search-result-budget.test.ts`(4)는 연산 수 대신 구조로 옮김(200그룹을 두 배치로 붙여 그룹·index 수, 같은 경로 재도착은 새 그룹을 만들지 않음, 빈 배치 무변화), UTF-16 강조 변환(BMP 밖 문자 fixture).
    - App: 두 파일에 일치 → 헤더 2개·개수·경로, 헤더 클릭으로 접힘·펼침, 일치 클릭 → preview 탭이 열리고 그 줄·열로 reveal, 2,000행 이상 fixture에서 viewport 밖 행이 그려지지 않음(출력 label 수 상한).
- 완료 판정: 단위·App 통과.

#### explorer-search-35 검색 이력 저장

- 원본 동작: `src/entities/search/search-history.ts`
    - 6: 상한 20개.
    - 8-13: 앞뒤 공백 제거, 빈 문자열 무시, 이미 맨 앞이면 무변화, 있으면 앞으로 이동.
    - 39-53: 저장 왕복 중에는 내가 마지막에 쓴 목록을 기준으로 이어 쓰고, 진행 중 쓰기가 0이 되면 서버 값으로 돌아갑니다.
    - 60-71: 쓰기는 설정 patch `recentSearches`입니다.
    - 기록하는 실행: 패널 Enter(`search-panel-container.tsx:109-115`), 검색 편집기 Enter. 기록하지 않는 실행: 입력 검색, 교체 뒤 재검색, 검색 편집기 자동 실행.
    - backend는 20개로 다시 자릅니다(`crates/taide-settings/src/service.rs` 202, 296-297).
- native 자산: `Settings.recent_searches`, 설정 쓰기 경로(`update_control_settings`, `HostCommand::UpdateSettings`).
- 빠진 것: 계산·큐·쓰기 명령.
- 설계: `search-history.rs`에 `add_recent_term`과 `Queue{pending, in_flight}`(앱 하나)를 둡니다. 쓰기는 `HostCommand::ExplorerSearch(RecordRecentSearches(Vec<String>))` → `update_control_settings(SettingsPatch{recent_searches: Some(..)})` → `HostReply::ExplorerSearch(RecentSearchesSaved(result))`로 settle합니다. 실패는 settle만 하고 표시하지 않습니다(원본 mutation 실패도 표시 없음). 목록 표시는 `pending.unwrap_or(settings.recent_searches)`를 씁니다. settings lane 파일(`settings_controls.rs`)은 건드리지 않습니다.
- 검사: 단위 `search-history.test.ts` 10개 사례. App: Enter 두 번을 연달아 → 설정 파일(테스트 `--data-dir` 아래)에 두 개 모두, 입력 검색만 한 질의는 없음.
- 완료 판정: 통과.

#### explorer-search-34 입력 시 검색

- 원본 동작: `search-panel-container.tsx:184-203`
    - 설정 `searchOnType`(기본 켜짐), `searchOnTypeDebounceMs`(기본 300, backend가 50~2000으로 보정; `src/shared/constants/search.ts`)입니다.
    - 질의·토글·제외 glob·`includeGlob`·`scopeDir` 중 하나가 바뀌면 타이머를 다시 겁니다. 질의가 공백뿐이면 타이머를 걸지 않고 결과는 그대로 둡니다.
    - 발화 때 그 순간의 입력으로 실행합니다. 이력은 남기지 않고 실패 toast도 없이 `failed` 본문만 보입니다(`use-search-run.ts:12-21`).
    - Enter는 대기 타이머를 취소하고 즉시 실행합니다.
    - 이전 실행은 backend 대체로 끝냅니다(명시 취소 없음).
- native 자산: 설정 값(`search_on_type`, `search_on_type_debounce_ms`)과 설정 화면 컨트롤이 있습니다.
- 빠진 것: 디바운스.
- 설계: `Panel`에 `live_deadline: Option<f64>`를 둡니다. 입력 변화가 있는 frame에 `now + debounce`로 다시 잡고 `request_repaint_after`를 겁니다. 마감이 지나면 `run(live: true, record: false)`입니다. IME 조합 중에는 시작하지 않습니다(2.0.7).
- 검사
    - 단위·App 사례 출처: `src/widgets/search-panel/search-panel-container.test.tsx` 실시간 검색 9건.
    - App(입력 시간 주입): 300ms 전에는 run 없음, 지나면 1회, 연속 입력은 마지막 기준 1회, 옵션 변경도 같은 디바운스로 재실행, 설정 끄면 없음, Enter가 대기 run을 취소하고 이력 기록, 반쪽 정규식 입력 → toast 없음·`search.failed`, 켜지면 안내 문구가 `search.liveSearchHint`, 조합 중 preedit만으로는 run 없음.
- 완료 판정: 통과. settings-80 비고에 근거 위치를 남깁니다.

#### explorer-search-1 사이드바 4개 보기 tablist

- 원본 동작: `explorer-panel.tsx:24-31,177-200`(순서·아이콘·`explorer.sidebarSwitchLabel`·선택 바탕), 각 버튼은 24px, 16px 아이콘, 4px 모서리(`rounded-sm`, 2.0.5)입니다. 202-260은 보기별 본문입니다.
- native 자산: `symbol-sidebar.rs`(`View` 4개, `Sidebar::switch`, TabList/Tab 접근성, 누름 frame focus, `BUTTON_RADIUS`), 실제 App 검사 `application-explorer-views-tests.rs` `실제_app_사이드바는_네_보기를_slot별로_전환하고_검색과_git은_빈_본문만_보인다`.
- 빠진 것: Search 본문(31), Git 본문(git lane), 버튼 모서리(`BUTTON_RADIUS` 2 → 4).
- 설계: 모서리 상수만 바꿉니다. 위 App 검사의 "검색 본문이 비어 있다" 단언은 31의 패널 표시 단언으로 바꿉니다(Git 본문 빈 단언은 git lane이 바꿀 때까지 유지).
- 검사: 위 App 검사 수정본, `symbol_sidebar` 단위 검사.
- 완료 판정: Search 본문까지 통과하면 complete입니다. Git 본문은 git-1 행이 판정한다고 비고에 적습니다.

---

### 2.3 묶음 C: 검색 진입·교체·편집기·메뉴

#### explorer-search-33 교체

- 원본 동작
    - 열기: `search-panel.tsx:108-117` 전환 버튼 또는 요청 `openReplace`(새 nonce에서 열기만, 닫지는 않음).
    - 교체 줄 175-193: 질의 상자와 같은 모양, placeholder `search.replacePlaceholder`, 24px `ReplaceAll` 버튼(tooltip `search.replaceAll`, 오른쪽 4px, 비활성이면 불투명도 0.4)입니다.
    - 활성 조건 119-123: 바꾸기 열림, 선택 파일 1개 이상, 진행 중 아님, 표시 결과의 질의와 현재 입력이 같음(`isSameSearchQuery`, contextLines 제외). 다르고 결과가 있으면 `search.replaceStaleHint`(194-196)를 보입니다.
    - 파일별 선택: 제외 경로 집합. 새 검색에도 초기화하지 않습니다.
    - 확인 268-281: AlertDialog(작은 크기). `search.replaceConfirmTitle`, `search.replaceConfirmDescription{files, matches}`(선택 파일 수·선택 일치 수), `common.cancel`, `common.confirm`.
    - 실행 `search-panel-container.tsx:124-162`
        - 입력이 아니라 표시 결과를 만든 질의(`ranQuery`)로, 선택 파일 경로만 교체합니다.
        - 성공: `toast.success(search.replaceDone{files, matches})`, OS 알림(범주 `searchReplace`, 제목은 프로젝트 표시 이름 또는 이름, 없으면 `notification.searchReplaceDone`, 본문은 같은 요약).
        - 건너뜀이 있으면 `toast.warning(search.replaceSkipped{count: 총수})`, 설명은 앞 5개를 `경로 — 사유`(`search.replaceSkipReason.*`)로, 남으면 `search.replaceSkippedMore{count}`(`src/entities/search/replace-skip-report.ts:8-33`; backend 목록 상한 50). 줄마다 truncate.
        - 그 질의로 다시 검색(이력 없음). 실패는 `describeIpcError` toast.
- native 자산: backend `search_replace`, toast(`Toasts::notify(Kind::Warning, title, Options{description})`), 확인 모달 선례 `delete_dialog.rs`(취소 초기 focus, Escape, 배경 클릭).
- 빠진 것: 전부, OS 알림 진입점(1.3).
- 설계
    - `Panel`에 `replace_open`, `replace_text`, `excluded: BTreeSet<String>`, `confirm: Option<..>`를 둡니다.
    - 확인 모달은 `delete_dialog.rs` 구조를 따릅니다.
    - 실행은 2.0.2 `Replace`, 응답 `Replaced`를 그 session의 `Panel`에 적용합니다. 패널이 이미 없으면 UI 반영(toast·재검색)을 하지 않습니다. 원본도 unmount 뒤 mutate 콜백이 실행되지 않습니다.
    - 건너뜀 설명은 `\n`으로 이은 한 문자열입니다. 줄마다 truncate가 필요하면 toast 렌더의 설명 줄바꿈을 확인합니다.
    - OS 알림은 병합된 앱 공통 진입점(shell-91 또는 settings-23 설계의 `notify(category, project, fallback_title_key, body)` 형태)을 한 곳에서 부릅니다. 진입점이 아직 없으면 이 줄만 남깁니다.
- 검사(App, 합성 fixture)
    - 바꾸기 열기 → 버튼 비활성(결과 없음) → 검색 → 활성, 입력을 바꾸면 비활성과 안내.
    - 파일 하나 체크 해제 → 확인 문구의 수, 확정 → 그 파일만 바뀜, 성공 toast 문구, 재검색 결과.
    - binary fixture → warning toast와 설명.
    - 취소·Escape → 변화 없음.
    - OS 알림은 진입점의 주입 포트(기록 포트)로 범주·제목·본문을 확인합니다. 실제 OS 알림은 보내지 않습니다.
- 완료 판정: 통과. 진입점 미병합이면 partial(알림 1줄 남음). 통과하면 settings-23 비고의 검색 치환 생산자 부분도 같은 근거로 갱신합니다.

#### explorer-search-36 검색 편집기 탭으로 열기

- 원본 동작
    - `search-panel-container.tsx:164-176`: 현재 입력으로 만든 질의(빈 text 허용)로 `TabKind::SearchEditor{query}` 탭을 엽니다. 제목은 `query.text || searchEditor.title`, preview 아님, 그 창 pane입니다. 실패는 toast입니다.
    - 버튼: `search-panel.tsx:199-206`, 14px `FileSearch2`, tooltip `searchEditor.title`.
    - 탭 본문 `src/widgets/search-editor/search-editor-pane.tsx`(editor-81)
        - 42-74: 탭별 기억(`src/entities/search/search-editor-memory.ts`, 16개 LRU, 프로젝트가 다르면 버림)에서 입력·결과를 복원하고, 없으면 탭 질의로 시작합니다.
        - 76-89: Enter 실행은 이력을 기록하고 `includeGlob`·`scopeDir`를 유지하며 contextLines를 포함합니다.
        - 96-97: 일치 열기는 그 창 focused pane, preview, reveal입니다.
        - 103-109: 기억이 없고 탭 질의가 비지 않았으면 mount 때 한 번 자동 실행합니다(이력 없음).
        - 111-113: mount 때 입력에 focus합니다(전체 선택 없음).
        - 115-122: 화면에서 내려갈 때 입력·결과를 기억합니다. 실행 중이면 취소하고 completed로 남깁니다(`use-search-run.ts` `readSnapshot`).
        - 125-200: 바탕 `editor.background`, 머리 좌우 12px·상하 8px.
            - 질의 상자와 이력·토글은 패널과 같습니다.
            - 둘째 줄(간격 12px, 줄바꿈 허용)은 제외 glob과 `searchEditor.contextLinesLabel`, 숫자 입력(56px, 0~10, `src/widgets/search-editor/search-editor-context-lines.ts`)입니다.
            - 상태 줄은 0건이면 `searchEditor.noResults`입니다.
            - 본문 안내는 항상 `search.pressEnterHint`이고, 빈 결과는 `searchEditor.noResults`, 실패는 `search.failed`입니다.
            - 바꾸기·범위 칩·입력 검색은 없습니다.
- native 자산: 탭 아이콘 `Glyph::FileSearchCorner`(shell-75), 탭 본문은 미지원 안내(`tab.contentUnavailable`, `tab_content`의 File 외 분기)입니다.
- 빠진 것: 열기 명령, 탭 본문(D4).
- 설계
    - 열기: `HostCommand::ExplorerSearch(OpenEditor{project, pane, query, title})` → `layout_actions::layout_open_tab(sink, state, project, TabKind::SearchEditor{query}, title, pane, false)`.
    - 본문(D4 A): `search-editor.rs`가 31·32 모듈을 재사용합니다. 탭 id별 상태를 16개 LRU로 둡니다. 직전 frame에 그렸는데 이번 frame에 그리지 않은 탭은 실행 중이면 `Cancel`하고 completed로 기록합니다. 탭 내용 분기에서 `TabKind::SearchEditor`를 이 렌더러로 넘기는 hook이 필요합니다(1.3).
- 검사
    - 단위: `search-editor-memory.test.ts`(5), `search-editor-context-lines.test.ts`(5) 사례.
    - App: 패널에서 질의·토글 설정 → 버튼 → `SearchEditor` 탭이 열리고 제목과 query가 맞음. 탭 본문이 자동 실행해 결과를 보임. contextLines 2 → 컨텍스트 줄과 중복 제거. 일치 클릭 → 같은 pane에 파일이 열리고, 검색 탭으로 돌아오면 입력·결과가 그대로이고 다시 실행하지 않음. 빈 질의로 연 탭은 자동 실행하지 않음.
- 완료 판정: 통과. editor-81과 shell-89의 searchEditor 부분을 같은 근거로 갱신합니다. D4가 B면 탭 생성까지만 판정하고 본문은 editor-81에 넘깁니다.

#### explorer-search-37 ⌘⇧F, ⌘⇧H, 선택 seed, focus·전체 선택, openReplace

- 원본 동작
    - 키: `src/shared/lib/keymap/keymap.ts:103-105`(`find` ⌘F, `search` ⌘⇧F, `search-replace` ⌘⇧H).
    - `src/widgets/editor-area/editor-area.tsx:154-168,291-297`
        - 포커스 slot에서만 동작하고, 아니면 키를 소비하지 않습니다(`whenFocused`).
        - ⌘⇧F는 텍스트 focus가 있는 편집기의 비지 않은 primary 선택이 한 줄일 때 `seedText`로 넣습니다.
        - ⌘⇧H는 `openReplace: true`이고 seed가 없습니다.
        - ⌘F는 편집기(찾기 위젯 포함, `hasWidgetFocus`)에 focus가 없으면 편집기 찾기 대신 검색 패널을 엽니다(seed 없음).
    - 팔레트: `command-catalog.ts:79-86`(`search.find`, `search.replace`, seed 없음).
    - 수신: `explorer-panel.tsx:131-140`(포커스 slot만, 보기 search, 요청 대체, nonce 증가), `project-shell.tsx:98-101,120`(접힘 펼침), 보조 창 `auxiliary-window-shell.tsx:105`.
    - 패널: 새 nonce면 seed가 있을 때 질의를 바꾸고, `openReplace`면 바꾸기를 열며, 입력 focus와 전체 선택을 합니다(31 참조).
- native 자산: 키 정의 `keymap-defaults.json`의 `search`·`search-replace`, 명령 정의 `keybinding-commands.json` `search.find`·`search.replace`(UI `src/`). UI `command-registry.rs` `keymap_run`에는 Run이 없고, 그 검사가 `search`·`search-replace`를 pending으로 단언합니다(`NATIVE_KEYMAP_COUNT`). `command-dispatch.rs`의 `accepts`/`intent`는 `find`를 `editor.find`로만 봅니다. `shell_keymap.rs`의 `ShowExplorer` 처리가 slot·보조 창 분기의 선례입니다.
- 빠진 것: Run, intent, 처리, find 대체.
- 설계
    - UI: `Run::OpenSearch{replace: bool}`을 추가하고 `keymap_run`의 `search`/`search-replace`를 연결합니다. 검사의 pending 목록과 `NATIVE_KEYMAP_COUNT`를 고칩니다. 팔레트 실행은 같은 Run으로 자동 연결됩니다(D8).
    - intent: `intents::explorer_search::Intent::OpenSearch{project, slot | aux, seed: Option<String>, replace}`.
    - 앱 처리: `show_explorer_files`처럼 보기를 Search로 바꾸고 접힘을 풉니다. 보조 창은 그 창 탐색기를 엽니다. `Sidebar.search_request`를 새 요청으로 바꾸고 nonce를 올립니다.
    - seed: `intent_with_editor`의 focused view가 텍스트 focus일 때 primary 선택 텍스트를 문서 snapshot에서 읽습니다. 비었거나 `\n`이 있으면 None입니다.
    - find 대체: `command-dispatch.rs`의 `find` 분기에서 편집기 focus(본문 또는 찾기 위젯)가 없으면 `OpenSearch{replace: false}`로 갑니다.
- 검사
    - 단위: `search-panel-bridge.test.ts`의 요청 기본값 사례(부분 요청은 나머지 기본값, 폴더 범위는 `scopeDir`만).
    - UI: `command-registry` 단위에서 `search`/`search-replace`가 Run을 돌려주고, 팔레트 명령 실행 가능.
    - App
        - 편집기에서 한 줄 선택 → ⌘⇧F → 포커스 slot만 Search 보기, 질의가 선택 텍스트, 입력 focus·전체 선택.
        - 여러 줄 선택 → seed 없음.
        - ⌘⇧H → 바꾸기 열림.
        - 접힌 사이드바 → 펼쳐짐.
        - 폴더에서 찾기 뒤 ⌘⇧F → 범위 해제.
        - 편집기 focus 없는 ⌘F → 검색 패널, 편집기 focus 있는 ⌘F → 편집기 찾기.
        - 보조 창 ⌘⇧F → 그 창 탐색기.
- 완료 판정: 통과. commands-ui-11·12, shell-59의 ⌘⇧F/⌘⇧H 부분을 같은 근거로 갱신합니다(D8).

#### explorer-search-20 컨텍스트 메뉴: 터미널에서 열기

- 원본 동작
    - 메뉴 `src/features/explorer/file-tree-context-menu.tsx:147-158`: 행이 있으면 파일·폴더 모두 보입니다. 행 묶음(구분선 → Finder에서 보기 → 터미널에서 열기 → 폴더에서 찾기) 안이며 문구는 `explorer.openInTerminal`, 아이콘 16px `SquareTerminal`입니다.
    - 실행 `explorer-container.tsx:160-167`: `resolveTargetDir`(`src/widgets/explorer/explorer-path.ts:24`)로 디렉터리는 자기, 파일은 부모, page에 없는 행이면 root를 cwd로 쓰고, root도 없으면 무동작입니다. `TabKind::Terminal{sessionId: '', cwd}`, 제목 `terminal.title`, preview 아님, 그 창 pane입니다. 실패는 toast입니다.
    - 메뉴 전체 구조(20-23 공통): 새 파일·새 폴더 / [파일] 구분선, 나란히 열기, 열기 방식, 브라우저에서 열기, 비교 대상 선택, 선택 항목과 비교, 파일 히스토리 / [행] 구분선, Finder에서 보기, 터미널에서 열기, [폴더] 폴더에서 찾기 / 구분선, 잘라내기, 복사, 붙여넣기, 경로 복사, 상대 경로 복사 / [행] 구분선, 이름 바꾸기, 삭제(96-216).
- native 자산: `explorer.rs` 행 컨텍스트 메뉴(`render_mounted`, 항목 응답을 `output.menu`에 기록). 지금 구분선 위치가 원본과 다릅니다: 새 폴더 뒤 항상 구분선이 있고, 파일 묶음 뒤 구분선은 html 파일의 브라우저 항목 뒤에만 있어 html이 아닌 파일은 열기 방식과 Finder에서 보기 사이에 구분선이 없습니다. 항목은 글자만 그리고 아이콘이 없습니다(19 complete 판정 범위의 기존 차이, D10). host `NewTerminal{project, pane, title}`은 `cwd: None` 고정이고, 터미널 attach는 탭 cwd를 씁니다(`terminal_tabs.rs` 272-283, 407-421). 대상 디렉터리 helper는 `explorer_clipboard::parent_dir`가 있습니다.
- 빠진 것: 항목, cwd 탭 열기.
- 설계
    - `explorer::Action::OpenInTerminal(TreeRow)`. 대상 디렉터리는 원본 `resolveTargetDir` 규칙입니다.
    - host `ExplorerSearch(OpenTerminalAt{project, pane: Option<PaneId>, cwd})` → `layout_open_tab(.., TabKind::Terminal{session_id: "", cwd: Some(cwd)}, terminal.title, pane, false)`.
    - 메뉴 구분선을 원본 구조로 맞춥니다: 파일 묶음은 그 앞 구분선 하나로 시작하고, 행 묶음은 항상 구분선으로 시작합니다. 22·23 항목이 들어갈 자리(파일 묶음의 브라우저 항목 뒤)를 비워 둡니다.
- 검사(App): 폴더 우클릭 → 항목 → 새 터미널 탭의 `TabKind::Terminal.cwd`가 그 폴더. 파일 → 부모. 보조 창 → 그 창 pane. 메뉴 항목 순서·구분선 위치를 `output.menu`와 출력 shape 순서로 확인(html·비html 파일, 폴더).
- 완료 판정: 통과.

#### explorer-search-21 컨텍스트 메뉴: 폴더에서 찾기

- 원본 동작
    - 메뉴 `file-tree-context-menu.tsx:159-164`: 디렉터리만, `explorer.findInFolder`, 아이콘 16px `Search`.
    - 실행 `explorer-container.tsx:169-180`: 프로젝트가 있을 때 `requestOpenSearchPanel({scopeDir: toRelativePath(root, row.path)})`입니다. glob이 아니라 `scopeDir`라 `node_modules` 같은 기본 제외 폴더 안도 찾습니다.
- native 자산: `Explorer::relative_path`(같은 규칙, 비공개 메서드), backend `SearchQuery.scope_dir`.
- 빠진 것: 항목과 요청.
- 설계: `explorer::Action::FindInFolder(String)` → 같은 mount의 `Sidebar`에 `Request{scope_dir: Some(rel), ..}`, nonce 증가, 보기 Search입니다. 메뉴는 그 mount에서 열렸으므로 mount를 직접 지정합니다.
- 검사
    - 단위·App 사례 출처: `search-panel-container.test.tsx` 폴더 범위 3건.
    - App: `node_modules/pkg` fixture 폴더에서 항목 → Search 보기, 범위 칩 `폴더: node_modules/pkg`, 검색 결과가 그 폴더 안 파일만, 칩 해제 뒤 재검색은 전체(기본 제외 적용).
- 완료 판정: 통과.

#### explorer-search-22 컨텍스트 메뉴: 비교 대상 선택 / 선택 항목과 비교

- 원본 동작
    - 메뉴 `file-tree-context-menu.tsx:132-139`: 파일만. `explorer.selectForCompare`(아이콘 `FileDiff`), `explorer.compareWithSelected`(아이콘 `GitCompare`, 대상이 없으면 비활성).
    - 실행 `explorer-container.tsx:182-195,211-213`: 대상 경로는 컨테이너 상태입니다. 비교하면 `TabKind::Diff{path: row.path, staged: false, compareWith: 대상}` 탭을 엽니다. 제목은 `<대상 이름> vs <행 이름>`, preview, 그 창 pane이고, 호출 직후 대상을 지웁니다. 실패는 toast입니다.
- native 자산: `TabKind::Diff{path, staged, compare_with}`(모델), 탭 아이콘 `FileDiff`. diff 본문은 없습니다(git-45·editor-76).
- 빠진 것: 메뉴·상태·탭 열기, 본문(git lane).
- 설계
    - D3 A(git lane 소유, git 사양 4.1 결정 2 A와 같음): 이 lane은 mount별 `Explorer`(D6)와 20의 메뉴 구조(파일 묶음 안 자리)만 남깁니다. git lane이 `Explorer.compare_source: Option<String>`, `Action::SelectForCompare`/`CompareWithSelected(TreeRow)`, host `OpenDiffTab{project, pane, kind, title}` → `layout_open_tab(.., preview: true)`, 본문(git-45)을 함께 넣습니다.
    - D3 B: 이 lane이 위 메뉴·대상 상태·Diff 탭 생성까지 하고 본문은 git-45가 판정합니다.
- 검사(구현하는 lane): 파일 A 선택 → B에서 비교 → Diff 탭의 kind·제목·preview가 맞고 대상이 지워져 다시 비활성. 다른 mount에는 대상이 없음. git lane은 본문 표시까지.
- 완료 판정: D3 A면 이 lane은 행의 lane 표기를 panel-git으로 옮기고 git-45와 함께 판정되게 합니다. D3 B면 탭 생성까지 통과 후 partial(본문 git-45).

#### explorer-search-23 컨텍스트 메뉴: 파일 히스토리

- 원본 동작
    - 메뉴 `file-tree-context-menu.tsx:140-143`: 파일만, `git.fileHistory`, 아이콘 `History`.
    - 실행 `explorer-container.tsx:214`: `requestOpenFileHistory(path)`.
    - 패널 `src/widgets/file-history/file-history-panel.tsx`(git-53·54): 포커스 slot만, 오른쪽 도킹 대화상자.
- native 자산: 없음. 패널은 git lane 범위입니다.
- 빠진 것: 메뉴와 패널.
- 설계(D2 A, git 사양 4.1 결정 2 A와 같음): git lane이 git-53~55와 함께 구현합니다. 메뉴 위치는 20의 구조(파일 묶음 끝)입니다. 이 lane은 코드를 넣지 않습니다. 소비자 없는 메뉴는 원본에 없는 무동작 항목이 됩니다.
- 검사: git lane에서 실행합니다.
- 완료 판정: git lane이 판정합니다. 이 lane에서는 행의 lane 표기를 panel-git으로 옮깁니다.

#### explorer-search-16 삭제와 확인 대화상자

- 원본 동작
    - 대화상자 `src/features/explorer/entry-delete-dialog.tsx`: `explorer.deleteConfirmTitle{name}`, `explorer.deleteConfirmDescription{name}`, 취소, 파괴 `explorer.delete`. Radix Action이라 확정 순간 닫힙니다.
    - 실행 `use-explorer-entry-crud.ts:180-190`: 삭제 뒤 부모 디렉터리 갱신까지 성공하면 끝이고, 둘 중 실패는 `describeIpcError` toast입니다.
- native 자산: `delete_dialog.rs`, `explorer_delete.rs`(project 범위 확인, 문서 준비·회수, 초안 보존, macOS 복구 가능한 휴지통 `taide_file::service::delete_entry`), LSP reply `ExplorerDeleteFinished` → 성공이면 부모 `RefreshTree`, 실패면 `report`. 검사는 `tests/explorer-delete.rs` 4건입니다(실제 휴지통을 쓰는 것 포함, 실행 금지).
- 빠진 것
    - (a) 실제 휴지통 경로 검사의 현재 코드 기준 실행(허락 필요).
    - (b) 삭제 뒤 부모 갱신 실패 toast. 지금은 `TreeSynced` Err가 상태줄로만 갑니다.
- 설계: (b)는 29의 `RefreshTree` 출처 `delete`로 실패를 `report`(toast)합니다. (a)는 4장 P1입니다.
- 검사
    - App: 부모 갱신을 실패시키는 fixture(삭제 직후 부모 폴더 제거) → error toast. 휴지통에 닿지 않도록 확인 모달·dispatch까지만 쓰는 기존 비휴지통 검사 방식을 따르고, 삭제 완료는 `ExplorerDeleteFinished{result: Ok}` reply를 직접 넣어 만듭니다.
    - 허락 뒤: `tests/explorer-delete.rs` 전체(합성 fixture만 휴지통으로 이동).
- 완료 판정: (b) 통과와 (a) 실행 통과. 허락이 없으면 partial을 유지하고 비고에 "휴지통 검사 미실행"을 적습니다.

## 3. 작업 묶음 계획

| 묶음 | ID(순서) | 선행 | 수정 예상 파일 | 새 의존성 |
| --- | --- | --- | --- | --- |
| A 탐색기 행·상태 | 18, 26, 44, 8, 30, 7, 25, 27, 29 (9개) | 단계 0, D1·D6·D7 결정 | 앱: `explorer.rs`(mount 상태·선택 색·Git 색·배지·누락 화면 머리줄 분리), `domains/explorer_search.rs`, `explorer-git-decorations.rs`(+tests), `explorer-auto-reveal.rs`(+tests), `symbol-sidebar.rs`, `auxiliary-windows.rs`(mount 조회만), `application-explorer-search-tests.rs`, 추가만: `application.rs`·`lib.rs`·host·intent 하위 enum 루트·`presentation-refresh.rs`. UI: `tests/workbench.rs`(44). 문서: 대응표 9행·머리 집계, QA 기록 1개 | 없음 |
| B 검색 패널 핵심 | 38, 31, 32, 35, 34, 1 (6개) | A(30의 mount 상태), 아이콘 SVG | 앱: `search-panel.rs`(+tests), `search-results.rs`(+tests), `search-history.rs`(+tests), `search-stream.rs`, `domains/explorer_search.rs`, `symbol-sidebar.rs`(Sidebar 필드·모서리), `navigation-icons.rs`, `resources/icons/{case-sensitive,whole-word,regex,eye-off,history,replace-all,check}.svg`, `application-search-panel-tests.rs`, `application-explorer-views-tests.rs`(1 단언 수정), 추가만: `presentation-refresh.rs`·`lib.rs`·host enum 루트. 문서: 대응표 6행(+30 판정, shell-59 비고), QA 기록 | 없음(lucide SVG는 `resources/icons/LICENSE.txt` 범위) |
| C 진입·교체·편집기·메뉴 | 33, 36, 37, 20, 21, 16 (6개, D2·D3가 B면 22·23 추가해 최대 8개) | B, D4·D8·D10 결정, 알림 진입점(33), 탭 내용 hook(36) | 앱: `search-panel.rs`, `search-editor.rs`(+tests, D4 A), `explorer.rs`(메뉴 2항목·구분선), `domains/explorer_search.rs`, `command-dispatch.rs`(find 대체), `shell_keymap.rs`(필요 시), `application-search-panel-tests.rs`, `application-explorer-search-tests.rs`. UI: `command-registry.rs`(Run·검사), `intents/explorer_search.rs`. 문서: 대응표 6행(+22·23 lane 표기, editor-81·shell-89·commands-ui-11·12·settings-23 비고), QA 기록 | 없음 |

- 묶음마다 순서: 실패 검사 작성 → 구현 → 영역 검사 → 대응표 행 갱신(운영 문서 2.2).
- 영역 검사(묶음 끝 1회)
    - 앱: `--lib -- explorer search symbol_sidebar auxiliary_window tab_menu explorer_views -- --test-threads=4`(필터는 테스트 이름에 맞춰 조정)
    - 앱 통합: `--test explorer`(A의 8·30), `--test explorer-clipboard`(A의 D6 변경)
    - UI(C, 44): `--features native-host,inspection --lib -- command_registry`, `--test workbench`
    - remote-web: Wasm check(UI 공용 enum·trait 변경이 있는 묶음)
    - 각 crate `cargo fmt -- --check`
- 묶음 C에서 16이 허락 대기로 남아도 나머지 5개로 끝냅니다.
- 다른 lane과 같은 파일: `explorer.rs`는 이 lane 소유입니다. git lane은 wave 2에 이 lane 병합 뒤 D1 A면 손대지 않고, D3·D2 A면 메뉴 자리에 항목만 추가합니다. shell-37과의 `explorers` 정리는 1.3의 `forget_project`로 맞춥니다.

## 4. 결정·허락

### 4.1 결정 필요

1. D1 탐색기 Git 데코레이션 분담 — **A안(추천: 이 lane이 파생·행 렌더·주입점을 만들고 git lane이 `git-cache.rs` 공급 연결과 git-38·7 판정. `render_mounted`를 8·D6와 함께 한 번만 고치고 status 조회 경로는 git-cache 하나)** / B안(git 사양 4.1 결정 2 A 그대로 7 전부 git lane, 이 lane은 손대지 않음) / C안(이 lane이 임시 status 조회를 만들어 wave 1에 닫음, git-cache와 중복이라 비추천). A안은 git 사양 결정 2의 "탐색기 Git 장식은 git lane이 닫음"과 같은 방향이고, 메인이 두 lane 지시에 같은 문장으로 넣어야 합니다.
2. D2 explorer-search-23 파일 히스토리 메뉴 — **A안(추천: git-53~55와 함께 git lane, git 사양 결정 2 A와 같음, 소비자 없는 무동작 메뉴를 만들지 않음)** / B안(이 lane이 메뉴와 요청 큐를 만들고 git lane이 소비)
3. D3 explorer-search-22 비교 — **A안(추천: git lane이 메뉴·대상 상태·Diff 탭·본문까지, git 사양 결정 2 A와 같음, 본문 없는 탭이 중간에 노출되지 않음)** / B안(이 lane이 메뉴·대상 상태·Diff 탭 생성, 본문은 git-45)
4. D4 검색 편집기 탭 본문(editor-81, shell-89의 searchEditor 부분) — **A안(추천: 검색 패널 모듈을 그대로 쓰므로 이 lane이 구현, git 사양 1.3도 searchEditor 부분을 이 lane으로 봄)** / B안(editor lane이 구현, 36은 탭 생성까지만)
5. D5 검색 보기에서 벗어날 때 상태 — **A안(추천: 원본대로 패널을 내리고 입력·결과 초기화, 실행 취소)** / B안(slot별로 유지)
6. D6 탐색기 상태 단위 — **A안(추천: mount별로 나눔, 원본 컨테이너 단위와 일치, shell 사양 4.1 결정 5 A와 같은 방향)** / B안(프로젝트 공유 유지, 30을 차이 기록과 함께 판정)
7. D7 선택 행 hover 바탕 — **A안(추천: 원본 CSS 결과대로 hover가 선택보다 우선, 같은 규칙의 아웃라인(explorer-search-39) 차이는 후속 기록)** / B안(아웃라인처럼 선택 우선)
8. D8 commands-ui-11·12 — **A안(추천: 37 구현으로 실행 가능해지므로 같은 묶음에서 행 갱신)** / B안(commands-ui lane이 wave 3에 갱신)
9. D9 검색 입력 IME — **A안(추천: 조합 중에는 입력 검색 타이머를 시작하지 않음, 원본은 WebView 한계)** / B안(원본처럼 조합 중에도 실행)
10. D10 탐색기 컨텍스트 메뉴 항목 아이콘 — **A안(추천: 기존 native 메뉴처럼 글자만 두고 19의 기존 차이로 QA에 기록, complete 행을 이 lane에서 다시 열지 않음)** / B안(원본처럼 모든 항목에 16px 아이콘을 넣음, 19를 함께 재검증)
11. (메인 조율) 앱 공통 OS 알림 진입점 — shell 사양(shell-91, wave 1, `ShellCommand::Notify`)과 settings 사양(settings-23, wave 2, `domains::settings::notify`)이 같은 진입점을 따로 설계하고 있습니다. **추천: 먼저 도는 shell lane의 경로 하나로 정하고 settings lane은 그 진입점을 씀.** 이 lane의 33은 정해진 진입점을 한 곳에서 부르기만 합니다.

### 4.2 허락 필요

- P1 explorer-search-16: `tests/explorer-delete.rs`의 실제 휴지통 검사 실행. 합성 fixture 파일 1~3개가 사용자 macOS 휴지통으로 이동합니다(복구 가능). 허락이 없으면 16은 partial로 둡니다.
- P2 explorer-search-33: OS 알림 센터 실제 전달은 자동 검사로 보지 않습니다. 실기 확인(배치 31)이나 사용자 확인이 필요합니다. 자동 검사는 진입점의 주입 포트 호출까지입니다.
- P3 화면 픽셀·실제 입력(IME 조합, 실제 키) 확인은 GUI 실행이 필요해 이 lane에서 하지 않습니다(배치 31 메인).
- 새 외부 의존성은 없습니다. 아이콘은 이미 라이선스가 있는 lucide SVG를 옮깁니다.
