# 14. 남은 작업: git 영역

> 작성일 2026-11-04(2026-11-03판을 실제 파일로 다시 대조해 갱신) · 대상 영역 `git`(Source Control 패널, diff 탭·커밋 diff·Claude diff, 파일 이력, blame·gutter·충돌, 탐색기 Git 장식 공급, Sync 알림)
> 기준 코드: Git 코드·QA·기능표는 worktree `/Users/hyunseokbyun/development/TAIDE-lanes/git`(브랜치 `lane/git`, HEAD `50892b59` "docs(native): git lane 검증 기록과 기능표 갱신")를 읽었습니다. 이 worktree는 `to_rust_native`(HEAD `0c52b438`)를 `lane/git`으로 받아들이는 병합 중입니다(`/Users/hyunseokbyun/development/TAIDE/.git/worktrees/git/MERGE_HEAD` = `0c52b438340cff56285fa8b4fc75eee76b4e5fd6`). 다른 작업자가 충돌을 해결하고 있으며 미커밋 변경이 계속 바뀝니다(0.3절). 메인 저장소 `/Users/hyunseokbyun/development/TAIDE`(브랜치 `to_rust_native`)에는 git lane이 아직 병합되지 않았습니다(메인에는 `native/taide-native-app/src/git-cache.rs`가 없고 `title-branch.rs`가 남아 있음).
> 이 문서는 정적 읽기로 작성했습니다. cargo·앱 실행은 하지 않았습니다. 줄 번호는 작성 시점 값입니다. 병합으로 계속 바뀌므로 항상 심볼 이름으로 다시 찾으십시오.

---

## 0. 읽는 법과 전제 지식

### 0.1 표기

| 표기 | 뜻 |
| --- | --- |
| `LANE/` | `/Users/hyunseokbyun/development/TAIDE-lanes/git/` (병합 뒤에는 메인 저장소의 같은 상대 경로) |
| `APP/` | `native/taide-native-app/src/` |
| `UI/` | `native/taide-native-ui/src/` |
| `ED/` | `native/taide-native-editor/src/` |
| `CR/` | `crates/` |
| `TS/` | 원본 앱 `src/` (메인 저장소와 lane worktree 모두에 있음) |
| `MONACO/` | `/Users/hyunseokbyun/development/TAIDE/node_modules/monaco-editor/esm/vs/` (monaco-editor 0.56.0, `node_modules/monaco-editor/package.json`의 `"version": "0.56.0"`). lane worktree에는 `node_modules`가 없습니다 |
| `[C]` | crate·순수 모듈 단위 검사 |
| `[U]` | `native/taide-native-ui/tests/*.rs` egui UI 하네스 검사(`--features native-host,inspection`) |
| `[A]` | 앱 lib 안의 실제 `NativeApplication` 검사(`application-*-tests.rs`, `--lib`) |

### 0.2 이 문서가 기준으로 삼는 문서

| 문서 | 쓰임 |
| --- | --- |
| `LANE/docs/quality-assurance/2026-10-09-native-function-matrix.md` 478-542줄(`git-1`~`git-65`) | 행 상태의 단일 출처. git 표 머리(476-477줄)의 열은 `| ID | 요구사항 | TS 근거 | 현재 판정 | 근거 묶음 | 범위/잔여 동작 |` 6개입니다 |
| `LANE/docs/research/2026-10-28-lane-git-spec.md` | git lane 사양(2.0 공통 설계, ID별 사양, 4.1 결정). 이후 코드는 사양과 다른 부분이 있고 실제 코드가 우선합니다(각 QA 문서의 "사양과 실제 코드 차이" 절) |
| `LANE/docs/quality-assurance/2026-11-02-native-lane-git-g1-base.md` ~ `g8-blame-conflicts.md`(8개) | 묶음 G1~G8의 구현·검증·남은 범위·리뷰 수정 기록 |
| `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` | 7개 lane 사양의 4.1 결정은 모두 추천안 채택, OS 알림 진입점은 shell lane `notify_event` 하나, 탐색기 Git 장식은 D1 A(파생·렌더·주입점 explorer-search, 공급 git), git 검사의 git CLI 전역 설정 읽기 허용 |
| `docs/acknowledge/2026-10-26-native-autonomous-continuation.md` | "결정사항은 큰 변경사항 아니면 다 권장사항대로", 허락이 필요한 것은 보류 |
| `docs/memory/2026-10-28-native-lane-operations.md` | lane 운영(영역 소유, 공용 파일은 영역 블록 끝에 추가만, 병합 게이트) |
| `docs/research/2026-10-28-native-lane-seams.md` 1.4절 | 공용 파일 영역 블록 규칙(locale 접두사 `git.*` → git 영역) |
| `docs/memory/2026-10-07-native-transition-operations.md` 5·6절 | 화면 캡처 방법과 반복된 함정 |
| `docs/utils/2026-11-03-native-merge-gate.sh` | 병합 게이트 명령 |
| `docs/utils/2026-11-02-native-lane-workflow.js` `BACKGROUND`·`RULES`·`DONE` 문자열 | lane 작업자 공통 규칙(셸·git·금지 사항·주석·매직넘버·검증 문서) |

git 사양 4.1 결정(모두 A 채택, `LANE/docs/research/2026-10-28-lane-git-spec.md` 717-725줄)은 이 문서 전체에서 다음 이름으로 부릅니다.

| 결정 | 채택 내용 |
| --- | --- |
| 결정 1 A | git-15(amend·fetch·branch delete·undo)는 원본 화면이 없어 not-applicable |
| 결정 4 A | monaco 기본 장식 재현 범위: 900px 자동 inline, 분할선 끌기, filler 빗금, ± 표시, 문자 단위 강조, overview ruler(기존 `DecorationKind::Overview` 재사용), ClaudeDiff 수정 쪽 revert 화살표. 근거 문구는 "원본에서 보이고 누를 수 있는 것은 모두, native에 기반이 이미 있음" |
| 결정 5 A | Alt+\는 focus pane의 diff 탭만 토글 |
| 결정 6 A | showMoves는 monaco `computeMovedLines` 규칙을 libgit2 hunk 위에 이식 |
| 결정 7 A | diff 줄 계산은 libgit2 hunk(`taide_git::service::diff_hunks`)로, stage 백엔드와 같은 경계 |

### 0.3 지금 바뀌고 있는 것 (병합 진행 중) — 꼭 먼저 확인

작성 시점 `git -C LANE status --short`에 `UU`(충돌 미해결) 파일이 있고(예: 기능표, `application-explorer-views-tests.rs`, `command-dispatch.rs`, `domains/explorer_search.rs`, `explorer.rs`, `navigation-icons.rs`, `shell_keymap.rs`, UI `command-registry.rs`), 병합 작업자가 git 영역 파일을 미커밋(` M`)으로 고치고 있습니다. `git -C LANE diff`로 확인한 git 관련 미커밋 변경은 다음과 같습니다. 이 문서의 "병합 결과 확인 필요"는 이 변경을 말합니다.

| 변경 | 파일·심볼 | 관련 행 |
| --- | --- | --- |
| pull/push OS 알림 연결 | `APP/domains/git.rs` 상수 `GIT_PULL_NOTIFICATION`·`GIT_PUSH_NOTIFICATION`(35-42줄), 메서드 `notify_git_remote`(1513-1544줄 부근), `apply_git_reply`의 `MutationOp::Pull`·`Push` 분기 첫 줄 호출 | git-7 |
| 알림 App 검사 | `APP/application-git-header-tests.rs` `실제_app_sync의_pull과_push는_성공과_실패를_프로젝트_제목의_os_알림으로_보낸다`(575줄, 파일 수정 시각 2026-11-04 01:12), helper `commit_ahead`(210줄). `APP/domains/shell.rs` 끝 `pub(in crate::application) mod notifications_tests;`, `APP/application-notifications-tests.rs`의 `FakePlatform`·`FakePlatform::sent`·`launch_with_platform`이 `pub(in crate::application)` | git-7 |
| 탐색기 장식 공급 | `APP/domains/git.rs` `tick_git_cache` 안 `explorer_search_state.git_decorations.mounted_projects()` 반복 → `cache.observe(.., Key::Status, now)` → `git_decorations.set(.., status_result(..).and_then(Result::ok))`(2631-2644줄) | git-38 |
| 제목줄 branch를 캐시로 이동 | `APP/title-branch.rs`·`title-branch-tests.rs` 삭제(staged `D`), `State`의 `title_branch_changes`·`title_branches` 필드 삭제, `HostCommand::Git(Command::GitBranch)`·`Reply::GitBranch` 삭제, 새 `State::title_branch(&ProjectId) -> Option<&str>`(299-304줄), `tick_git_title_branches`가 `Key::Status`를 관찰(2509-2529줄), `application.rs`의 `PaintSink.title_branches`·`AppSurfaces.title_branches` 삭제 | git-38(사양 2.0.3) |
| 장식 App 검사 | `APP/application-git-base-tests.rs` `실제_app_탐색기_git_장식은_git_cache_status로_파일_배지와_상위_폴더를_그리고_git_변경_뒤_갱신한다`(637줄) | git-38 |
| IDE diff 수신함 이전 | `APP/provider-events.rs` `Event::IdeDiff(crate::git_claude_diff::Event)`와 `Inbox::record`의 `IdeDiffRequested`·`IdeCloseTabRequested { request_id: Some }` arm, 단위 검사 `provider_inbox는_ide_diff_요청과_요청_id가_있는_close_tab만_기록한다`; `APP/domains/shell.rs` `consume_provider_events`의 `Event::IdeDiff(event) => self.accept_claude_diff_event(event)`(1161줄); `APP/domains/git.rs` `accept_claude_diff_event`(2263줄, 옛 `consume_ide_diff_events` 본문을 이벤트 하나 단위로); `APP/git-cache.rs` `Changes.ide` 삭제; `APP/git-claude-diff.rs` `Inbox` 삭제; `tick_claude_diffs` 첫 줄의 `consume_ide_diff_events` 호출 삭제 | git-49 |
| 탐색기 비교 탭 target | `domains::git::compare_tab(project, source, path, target: Option<PaneId>)`(1185줄), `application-git-diff-tests.rs`의 비교 메뉴 검사가 `explorers.of_project(..)`로 바뀜 | explorer-search-22(본문 git-45) |
| CSS·HTML diff 가상 문서 검사 | `APP/application-git-diff-tests.rs` `실제_app_css와_html_파일_diff_탭의_가상_문서는_내장_구조_서비스_대상이_아니고_오류_toast가_없다`(상수 `MARKUP_DIFFS`) | 4절 함정 7 |
| shell lane `LayoutActions` 필드 추가 반영 | `APP/application-git-claude-history-tests.rs` `ide_tool`의 `crate::ide_tools::LayoutActions { .., wait_markers: Arc::default(), .. }` | git-49~52 검사 |
| 기능표 행 | lane worktree 기능표의 shell-89 행이 이미 "2026-11-03 git lane 병합으로 네 본문이 모두 탭 분기에 연결" 문구와 병합 문서 `2026-11-03-native-git-lane-merge.md`를 참조합니다. 그 QA 파일은 작성 시점에 lane·메인 어디에도 없습니다(`ls docs/quality-assurance | grep merge` 결과 없음) | shell-89 |

구현을 시작하기 전에 다음 순서로 병합 결과를 확인하십시오.

1. `git -C /Users/hyunseokbyun/development/TAIDE log --oneline -5` — `merge(native): git lane 병합` 같은 커밋이 있는지 봅니다. 없으면 아직 병합 전이고, 이 문서의 경로는 `LANE/`에서만 맞습니다.
2. `grep -n "fn notify_git_remote\|fn accept_claude_diff_event\|git_decorations.set\|fn title_branch" /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/domains/git.rs` — 네 심볼이 모두 나오면 병합 결과가 이 절과 같습니다.
3. `grep -n "^| git-" /Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-10-09-native-function-matrix.md` — 메인 기능표의 git 행이 lane 값(대부분 complete)으로 바뀌었는지 봅니다. 작성 시점 메인 기능표는 lane 작업 전 값(예: `git-1 | ... | missing`)입니다.
4. `ls /Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/ | grep git-lane-merge` — shell-89 비고가 가리키는 병합 문서가 생겼는지 봅니다.

### 0.4 egui 0.36·Rust 2024 최소 지식 (이 영역 코드에서 쓰는 형태)

- egui는 즉시 모드 GUI입니다. 매 프레임 `eframe::App::logic`(상태 갱신)과 `eframe::App::ui`(그리기)가 불리고, 위젯은 그릴 때마다 다시 만들어집니다. 위젯 간 상태는 앱 구조체나 egui 임시 메모리(`ui.ctx().data(|data| data.get_temp::<T>(id))`, `ui.ctx().data_mut(|data| data.insert_temp(id, value))`)에 둡니다. diff 위젯의 분할 비율·스크롤·펼침 상태가 임시 메모리 예입니다(`UI/diff-view.rs` `show`의 `let memory = Id::new(("taide-diff-view", request.modified));`와 `struct State { split, scroll, reveals }`).
- 상호작용은 `ui.interact(rect, id, Sense::click() | Sense::drag() | Sense::hover())`가 돌려주는 `Response`로 판정합니다(`clicked()`, `double_clicked()`, `dragged()`, `hovered()`, `interact_pointer_pos()`). 접근성 이름은 `response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label))`로 붙이며, 앱 검사는 이 이름으로 accesskit 노드를 찾습니다(예: `APP/git-claude-diff.rs` `bar`).
- 그리기는 `ui.painter()`(`rect_filled`, `text`, `galley`, `line_segment`)입니다. `Painter::with_clip_rect(rect)`는 부모 clip과 교집합을 만듭니다. 자식 영역은 `ui.new_child(egui::UiBuilder::new().max_rect(rect))`로 만듭니다.
- 이 저장소의 egui는 crates.io 0.36.2를 고친 fork입니다(`native/taide-native-app/Cargo.toml`의 `egui = { path = "vendor/egui-input" }`, `vendor/egui-input/Cargo.toml` `version = "0.36.2"`, 차이 기록 `vendor/egui-input/UPSTREAM.md`). 검사에 영향을 주는 차이: `RawInput::default()`의 `focused`는 `true`입니다(`vendor/egui-input/src/data/input/raw_input.rs` 106줄 `focused: true, // integrations opt into global focus tracking`). 그래서 알림 설정 `notifications_only_when_unfocused`를 끄지 않으면 검사에서 알림이 모두 `WindowFocused`로 억제됩니다.
- 앱 crate의 파일 이름은 kebab-case이고 모듈은 `#[path = "git-cache.rs"] mod git_cache;` 형태로 등록합니다(`APP/lib.rs`). 검사 파일도 `#[cfg(test)] #[path = "../application-git-base-tests.rs"] mod base_tests;`처럼 영역 파일(`APP/domains/git.rs` 끝, 2989-3027줄)에 등록해야 컴파일됩니다(4절 함정 4).
- Rust 2024 edition입니다(`native/taide-native-editor/Cargo.toml` 4줄 `edition = "2024"`, 5줄 `rust-version = "1.95"`). 코드는 `let ... else`, `if let ... && let ...`(let chain), `Option::is_none_or`/`is_some_and`를 씁니다. 예: `UI/diff-view.rs` `show`의 `if sash_response.dragged() && let Some(position) = sash_response.interact_pointer_pos() && rect.width() > 0.0 { .. }`.
- 각 native crate는 루트 workspace 밖의 독립 workspace라 모든 cargo 명령에 `--manifest-path`가 필요합니다(`docs/architecture.md` 9.1절).
- 코드 규칙(lane workflow `RULES`): 새 코드 주석 금지(공개 API 영어 `///`만 예외), 의미 있는 리터럴은 `const`, `#[allow]`·테스트 기대값 조작 같은 우회 금지, 사용자 문자열은 원본과 같은 locale 카탈로그 키, 요청 밖 리팩터링·리네임·재포맷 금지, `Cargo.toml`·`Cargo.lock` 변경 금지(새 의존성 없음).

---

## 1. 요약

### 1.1 남은 행과 상태별 수

기능표 git 65행의 현재 판정을 직접 셌습니다. 사용한 명령(4열 = 현재 판정):

```
awk -F'|' 'NR>=478 && NR<=542 {gsub(/^ +| +$/,"",$2); gsub(/^ +| +$/,"",$5); print $2" => "$5}' /Users/hyunseokbyun/development/TAIDE-lanes/git/docs/quality-assurance/2026-10-09-native-function-matrix.md
```

git-61은 요구사항 문구에 `\|`가 있어 이 명령으로는 열이 밀립니다(4절 함정 14). `sed -n 538p`로 따로 읽어 complete를 확인했습니다.

| 상태 | 수 | ID |
| --- | --: | --- |
| complete | 58 | 1~6, 8~14, 16~37, 39, 41, 43~49, 51, 53~65 |
| partial | 6 | 7, 38, 40, 42, 50, 52 |
| unwired | 0 | 없음 |
| missing | 0 | 없음 |
| not-applicable | 1 | 15 (결정 1 A) |
| frozen | 0 | 없음 |

이 문서가 다루는 행은 partial 6행입니다. 추가로 complete인 git-49는 병합 때 수신 경로를 옮겨야 해서 3.7절에서 확인 절차를 적습니다.

| ID | 기능 | 상태 | 기능표 줄 | 남은 범위(기능표 비고 요약) | 처리 |
| --- | --- | --- | --: | --- | --- |
| git-7 | Sync 버튼(pull 성공 후 push, 진행 스피너, 오류 toast) | partial | 484 | pull/push OS 알림 4종 | 병합 작업자 처리 중 → 병합 결과 확인 필요(3.1) |
| git-38 | 탐색기 파일 트리 Git 데코레이션 | partial | 515 | 탐색기 표시 프로젝트를 git 캐시에 관찰시키는 공급 연결과 장식 검사 회귀 | 병합 작업자 처리 중 → 병합 결과 확인 필요(3.2) |
| git-40 | Diff 렌더 | partial | 517 | 문자 단위 강조, ± 표시, diff overview ruler, inline 삭제 줄 구문 색, inline 원본·수정 두 줄 번호 열 | 구현 필요(3.3) |
| git-42 | `editorDiffHideUnchangedRegions`/`editorDiffShowMoves` 반영 | partial | 519 | showMoves, 가장자리 끌기, 다시 접기 glyph, 가장자리 hover 강조, 막대 outline breadcrumb, diff 재계산 때 펼침 상태 | 구현 필요, 기능표에 없는 확인된 결함 1건 포함(3.4) |
| git-50 | ClaudeDiffPane 렌더 | partial | 527 | 수정 쪽 revert 화살표(결정 4 A)와 git-40 공통 장식 | 구현 필요(3.5) |
| git-52 | 탭 닫힘/다른 창 이동 시 암묵적 reject 구분 | partial | 529 | 보조 창 이동 뒤 응답 없음·보조 창 Accept 실제 App 검사 | 검사 추가(3.6) |

주의: 기능표 머리의 영역 요약표(`LANE/...function-matrix.md` 18줄 `| git | 65 | 1 | 1 | 34 | 29 | 0 | 0 |`)는 lane 작업 전 값이라 행 판정과 맞지 않습니다. lane workflow `DONE` 규칙이 "상단 합계 표는 건드리지 않습니다"라서 생긴 차이입니다. 병합 뒤 메인이 행을 다시 세어 요약표를 갱신해야 합니다(5절).

### 1.2 현재 구현 위치

| 역할 | 위치·심볼 | 줄 수(작성 시점, `wc -l`) |
| --- | --- | --: |
| 영역 진입점: 명령·응답·상태·intent·tick·표면 | `APP/domains/git.rs` — `pub enum Command`(93줄), `pub enum MutationOp`(222줄), `pub enum Reply`(242줄), `pub(crate) struct State`(280줄), `pub(crate) async fn dispatch`(515줄), `impl NativeApplication { apply_git_reply(1253), notify_git_remote, apply_git_intent(1720), accept_claude_diff_event(2263), tick_claude_diffs(2297), show_git_file_history(2357), reconnect_git(2498), tick_git_title_branches(2509), tick_git_cache(2531) }`, `impl AppSurfaces { show_diff_tab(2675), show_diff(2754), show_claude_diff_tab(2821), show_git_view(2922) }`, 자유 함수 `read`(338), `with_hunks`(501), `show_surface`(1085), `compare_tab`(1185) | 3,027 |
| 조회 캐시·무효화 | `APP/git-cache.rs` — `Invalidation`, `Changes`, `Request<K>`, `GitCache<K, V>`(`observe` 117, `invalidate` 146, `requests` 159, `begin` 195, `finish` 206, `refetch` 224), `CacheKey`, `Key`(249), `Diff { sides, hunks }`(390), `Value`(396), `status_result`(415) | 569 |
| SCM 패널 그리기·모델 | `APP/git-scm.rs`, `APP/git-scm-model.rs` | 2,154 / 464 |
| 브랜치 스위처·커밋·그래프·상세 | `APP/git-branch.rs`, `APP/git-commit.rs`, `APP/git-graph.rs`, `APP/git-commit-detail.rs` | 611 / 230 / 632 / 284 |
| diff 탭 보조(외형·stageable·Alt+\·탭별 모드·숨은 문서 해제) | `APP/git-diff.rs` — `Appearance`, `is_stageable`, `take_side_by_side_toggle`, `Tabs`, `release_hidden`, `window`, `SyntaxTokens` | 164 |
| ClaudeDiff 대기 목록·줄 diff 일정·막대 | `APP/git-claude-diff.rs` — `Event`, `Revisions`, `State`(`pending`, `resolving`, `lines`), `Appearance`, `Choice`, `bar`, `RECOMPUTE_DELAY = 200ms`, `FALLBACK_LANGUAGE_ID = "plaintext"`, `BAR_HEIGHT = 32.0` | 340 |
| 파일 이력 dialog | `APP/git-history.rs` | 377 |
| 편집기 blame·gutter·충돌 | `APP/git-editor.rs`(`domains::git::editor` 모듈: `apply_git_editor_intent` 95, `apply_conflict_resolution` 384, `show_git_editor` 472, `show_conflict_compare` 632), `APP/git-blame.rs`, `APP/git-gutter.rs`, `APP/git-conflict.rs` | 735 / 265 / 264 / 198 |
| 공용 diff 위젯 | `UI/diff-view.rs` — `Hunk`(51), `Colors`(59), `Request`(71), `UnchangedRegion`(86), `Reveal`, `Hidden`, `side_lines`(149), `unchanged_regions`(160), `Tokens`, `Output`(216), `fillers`(266), `inline_zones`(298), `stage_ranges`(321), `is_side_by_side`(335), `layer`(339), `show_side`(389), `unchanged_bar`(437), `paint_removed_lines`(625), `lane_line`(695), `show`(741) | 1,028 |
| git intent | `UI/intents/git.rs` — `Section`, `Intent`(약 45변형: `DiffShown`, `ToggleDiffSideBySide`, `StageHunk`, `OpenTabChanges`, `ClaudeDiffShown`, `ResolveClaudeDiff`, `OpenFileHistory`, `EditorShown` ...), `ConflictAction` | 275 |
| 명령 등록 | `UI/command-registry.rs` — `Run::ShowGit`, `Run::Git(GitRun)`(`RevertHead`, `CreateTagOnHead`, `OpenFileHistory`, `ToggleBlame`, `StageSelection`) | — |
| 가상 문서 | `ED/document.rs` `DocumentKey::Virtual(VirtualKey { owner, side })`, `ED/store.rs` `open_virtual`(647줄)·`replace_virtual`(671줄)·`apply`(1681줄)·`apply_separate`(1689줄) | — |
| 편집기 표면 확장점 | `UI/editor_surface.rs` `ExtraZone`(184줄), `SurfaceExtras { zones, hidden_lines }`(193줄), `NativeEditor::show_request_with_extras`(1424줄), `EditorOutput.extra_zones` | — |
| 장식 종류 | `ED/decoration.rs` `DecorationKind::{Inline(InlineStyle), LineBackground([u8;4]), Lane { mark: LaneMark::{Bar, DeletedTriangle}, color }, Overview { lane: OverviewLane::{Left, Center, Right, Full}, color, minimap }}` | — |
| 줄 hunk 계산 | `CR/taide-git/src/service.rs` `DiffHunk`(628줄), `diff_hunks`(638줄) | — |
| 백엔드 명령 | `CR/taide-runtime/src/git_actions.rs`(`git_status`, `git_pull`, `git_push`, `git_diff_file`, `git_show_file`, ...), `CR/taide-runtime/src/ide_actions.rs::ide_resolve_diff` | — |

### 1.3 다른 영역과의 의존·소유 경계

| 다른 영역 | 그 영역이 소유하는 것 | git 영역이 하는 것 | 근거 |
| --- | --- | --- | --- |
| shell | OS 알림 진입점 `NativeApplication::notify_event`(`APP/domains/shell.rs` 1176줄), provider 수신함 `APP/provider-events.rs` `Inbox`·`consume_provider_events`(1112줄), 보조 창 수명·탭 창 이동(`Command::MoveTabToWindow`, 362줄), `APP/tabs.rs`의 ClaudeDiff 닫기 → `TabClosed`(136-139줄) | `notify_event`를 부르는 git 생산자(git-7), 수신함 IDE diff 분기의 처리 함수 `accept_claude_diff_event`(git-49) | `2026-10-28-native-lane-spec-decisions.md` 채택 2·3번째 항목 |
| explorer-search | 장식 파생·행 렌더·주입점 `APP/explorer-git-decorations.rs`(`Decoration`, `build`, `GitDecorations::{set, mounted_projects, get, mount, forget}`), 탐색기 "비교" 메뉴 | `GitDecorations::set` 공급(git-38), 비교 탭 본문(git-45, complete), 파일 이력 메뉴(git-55, complete) | 같은 문서 4번째 항목(D1·D2·D3 A) |
| editor | 편집기 표면·`EditorStore`·구문 파이프라인, 내장 JSON/CSS/HTML 서비스 | diff 위젯이 `SurfaceExtras`로 추가만, 내장 서비스 tick 필터에서 `DocumentKey::Virtual` 제외 유지 | `g5-diff.md` 6.1절 |
| settings | 설정 UI(settings-27 complete) | 설정 소비자: `editor_diff_hide_unchanged_regions`(완료), `editor_diff_show_moves`(git-42 남음), `ide_auto_open_diff`(git-49 완료) | git 사양 git-42 "settings 사양은 소비자를 git lane 몫으로 둠", 기능표 settings-28 비고 "소비자 없음: ... diff 2종(git lane)" |
| terminal-agent | IDE MCP 서버·`APP/ide-tools.rs`(`open_diff` 도구가 `AppEvent::IdeDiffRequested` 발행, `IDE_DIFF_TIMEOUT = 600_000ms`, 27줄) | ClaudeDiff 표면 전부(terminal-agent-68 = git-49~52) | git 사양 1.2 표 |
| commands-ui | 팔레트·키맵 행의 판정 | git 명령 실행 경로는 이미 있음. commands-ui-14·36~39·62·87 행은 그 영역이 닫음 | git 사양 1.3 표 |

git 행을 닫으면 함께 갱신할 다른 영역 행(lane 기능표 기준 상태): editor-76(partial, 남은 것 = git-40·50과 같음), shell-89(partial, 남은 것 = git-50 revert), shell-92(partial, "diff·close 요청 소비(git-49) 뒤 묶음 3 판정"), settings-23(partial, git push/pull 범주 = git-7), settings-28(partial, diff 2종 = git-42), settings-80(partial, IDE diff 자동 열기), terminal-agent-68(unwired로 남아 있음 — git-49~52 근거로 갱신 대상), explorer-search-7(partial, git-38과 같은 근거).

### 1.4 권장 구현 순서

1. **병합 결과 확인(git-7, git-38, git-49)** — 병합 작업자가 끝낸 뒤 0.3절 확인 명령, 3.1·3.2·3.7절 확인 목록, 병합 게이트를 돌립니다. 이 셋은 코드가 이미 쓰여 있어 검증과 기능표 갱신만 남는 것이 정상입니다.
2. **git-52(검사만)** — 코드 변경 없이 App 검사 1개를 추가합니다. 실패하면 그때 원인을 고칩니다. 다른 작업과 독립이라 먼저 끝내 행을 닫습니다.
3. **git-42의 확인된 결함과 펼침 상태 규칙** — 3.4.1(전체 파일 영역 판정 누락)과 3.4.2(내용이 바뀌면 펼침 초기화)는 `UI/diff-view.rs` 안의 작은 수정입니다. 이후 작업이 diff 위젯을 크게 바꾸기 전에 고칩니다.
4. **git-40 기반: 문자 단위 내부 변경 계산 이식**(3.3.1) — monaco `refineDiff`를 `taide-native-editor`에 옮기고 원본 oracle로 검증합니다. git-40 문자 강조, inline 삭제 줄의 문자 삭제 강조, hunk 안 정렬, git-50 선택 영역 revert, git-42 이동 블록의 "with changes" 판정이 모두 이 결과를 씁니다.
5. **git-40 표시**: 문자 강조 → ± 표시와 gutter 배경 → inline 원본 줄 번호 열 → inline 삭제 줄 구문 색 → hunk 안 정렬 → overview ruler 순서(3.3.2~3.3.7). 앞 단계일수록 다른 단계의 입력이 됩니다.
6. **git-50 revert gutter**(3.5) — 4·5단계의 내부 변경과 gutter 배치를 씁니다.
7. **git-42 showMoves**(3.4.3) — `computeMovedLines` 이식과 이동 표시. 4단계의 refine을 이동 블록 내부 diff에 씁니다.
8. **git-42 숨김 막대 세부**(3.4.4) — 가장자리 끌기, 다시 접기 glyph, hover 강조, 그림자, tooltip, breadcrumb. breadcrumb는 symbol 출처 확인이 필요해 마지막입니다.

---

## 2. 영역 공통 지식

### 2.1 상태가 있는 곳

`NativeApplication.git_state: domains::git::State`(`APP/domains/git.rs` 280-297줄) 하나에 영역 상태가 모여 있습니다. 병합 미커밋 상태 기준 필드:

| 필드 | 타입 | 내용 |
| --- | --- | --- |
| `cache_changes` | `Arc<git_cache::Changes>` | 이벤트 sink(`PaintSink`)가 다른 스레드에서 기록하는 무효화 입력. lane 커밋 `50892b59`에는 `ide: git_claude_diff::Inbox` 필드도 있었고 병합 미커밋에서 삭제됨 |
| `cache` | `GitCache<Key, Value>` | 프로젝트별 조회 캐시(status, log, diff, 파일, blame 등) |
| `panels` | `git_scm::Panels` | slot별 SCM 패널 로컬 상태(접힘, 선택, 그래프 높이, 태그 대화상자) |
| `appearance`, `diff_appearance`, `claude_appearance`, `history_appearance` | `Option<..Appearance>` | 테마 토큰에서 만든 색 묶음. `tick_git_cache`에서 필요할 때 다시 만듦 |
| `initializing`, `stashing`, `syncing` | `HashSet`/`HashMap` | 진행 중 표시(init, stash, Sync) |
| `init` | `InitPort` | `git init` 주입 포트(검사에서 가짜로 교체) |
| `commit` | `git_commit::State` | 커밋 초안·AI 요청 |
| `diffs` | `git_diff::Tabs` | diff 탭별 side-by-side 여부와 이번 프레임에 그려진 탭 |
| `claude` | `git_claude_diff::State` | IDE diff 대기 목록, 진행 중 resolve, 탭별 줄 hunk와 재계산 일정 |
| `histories` | `git_history::Panels` | 창별 파일 이력 dialog |
| `editors` | `git_gutter::Editors` | 편집기 gutter·blame·충돌 상태 |

lane 커밋 `50892b59`에는 `title_branch_changes: Arc<title_branch::Changes>`·`title_branches: title_branch::Branches`가 더 있었고, 병합 미커밋에서 `APP/title-branch.rs`와 함께 삭제되어 `State::title_branch(&ProjectId) -> Option<&str>`(캐시 status의 `branch`, 오류면 `None`)로 바뀌었습니다.

다른 영역 상태 중 git이 읽거나 쓰는 것: `explorer_search_state.git_decorations`(git-38 공급, 쓰기는 `set` 메서드로만), `store: EditorStore`(diff 가상 문서), `editor_syntax`(가상 문서 토큰), `services.state.settings`(`editor_diff_hide_unchanged_regions`, `editor_diff_show_moves`, `ide_auto_open_diff`, 알림 설정), `services.state.layouts`(ClaudeDiff 탭이 남은 창 트리), `shell_state.auxiliary_windows`(창 목록).

### 2.2 요청·응답·intent의 흐름

1. **그리기**: 표면 함수는 `&self` 또는 대여한 부분만 받아 상태를 직접 바꾸지 않고 `ShellIntent::Git(Intent::..)`를 `intents: &mut Vec<ShellIntent>`에 넣습니다. 예: `AppSurfaces::show_diff_tab`이 매 프레임 `Intent::DiffShown { project, tab, path, staged, compare_with, before_path, rev, parent_rev }`를 냅니다. 같은 intent가 캐시 관찰 신호입니다.
2. **intent 적용**: `APP/application.rs` intent 루프의 `ShellIntent::Git(intent) => self.apply_git_intent(intent, &context, &snapshot)`(2230줄)가 보냅니다. `apply_git_intent`는 상태를 바꾸고 필요하면 `self.submit(HostCommand::Git(Command::..))`를 부릅니다. 편집기 관련 intent(`EditorShown`, `EditorLanePress`, `ResolveHunkDiscard`, `ResolveConflict`, `CloseConflictCompare`, `ToggleBlame`, `StageSelection`)는 `apply_git_editor_intent`(`APP/git-editor.rs`)로 넘깁니다(`domains/git.rs` 2250-2256줄).
3. **host 제출**: `NativeApplication::submit`이 host 큐(용량 `HOST_COMMAND_CAPACITY = 64`, `APP/host.rs` 10줄)에 넣습니다. 큐가 가득이면 `false`를 돌려주며, 호출 측은 상태를 되돌리고 다음 프레임에 다시 냅니다(예: `Intent::ResolveClaudeDiff` 처리에서 `submit` 실패면 `claude.finish_resolve`, 2234-2241줄).
4. **worker**: host worker는 명령을 하나씩 await합니다. `HostCommand::Git(command) => crate::application::domains::git::dispatch(worker, command).await`(`APP/host.rs` 339-340줄). git `dispatch`는 모든 git 작업을 `services.tasks.spawn_transient(..)`로 띄우고 즉시 `None`을 돌려줍니다(`spawn_reply` 319줄, `change` 1227줄). 그래서 pull/push/AI가 worker를 막지 않습니다. spawn이 거절되면 `Err(stopped())`인 응답을 바로 돌려줍니다.
5. **응답**: task가 `HostReply::Git(Reply::..)`를 `worker.sender`로 보내고 `repaint()`를 부릅니다. 다음 프레임 `poll`이 `HostReply::Git(reply) => self.apply_git_reply(context, reply)`(`APP/application.rs` 777줄)로 넘깁니다.
6. **조회(캐시)**: 읽기 전용 조회는 개별 명령이 아니라 `Command::Read(Request<Key>)` 하나로 갑니다. 표면이 `cache.observe(project, key, now)`로 관찰하고, `tick_git_cache` 끝에서 `cache.requests(now)`(이번 라운드에 관찰됐고 값 없음·stale·재부착 후 stale 시간 경과, 프레임당 `MAX_REQUESTS_PER_FRAME` 16개)를 `Command::Read`로 내고 `cache.begin(&request)`합니다. 응답 `Reply::Read { request, result }` → `cache.finish(&request, result, now)`. 무효화는 `Changes::record`(이벤트 sink) → `tick_git_cache`의 `cache_changes.take()` → `cache.invalidate`. `invalidate`는 값을 지우지 않고 `is_stale`과 `generation`만 바꿉니다(`APP/git-cache.rs` 146-153줄).
7. **변경**: stage·commit·pull 같은 변경은 `Reply::Mutated { project, op: MutationOp, result }`로 돌아오고, 성공이면 `cache.invalidate(&project, &Invalidation::repository())`(가변 범위 전부 stale), 실패면 `self.toasts.ipc_error(&self.locale, &error, now)`.

### 2.3 한 프레임의 순서

| 단계 | 함수(작성 시점 위치) | git 관련 동작 |
| --- | --- | --- |
| `logic` | `NativeApplication::background_tick`(`APP/application.rs` 1085줄) | host 응답 `poll` → `apply_git_reply` |
| | 같은 함수 안 `consume_provider_events`(1115줄 호출, 정의 `APP/domains/shell.rs` 1112줄) | 병합 뒤: IDE diff 요청·close-tab 알림 → `accept_claude_diff_event` |
| | 같은 함수의 `if self.closing.is_none() && !self.is_exit_ready && !self.services.state.is_shutting_down()` 블록 끝 `tick_git_title_branches`(1337줄) → `tick_git_cache` → `tick_claude_diffs` | 무효화 소비, 닫힌 프로젝트 정리, 외형 갱신, 숨은 diff 문서 해제, ClaudeDiff 재계산 요청, 병합 뒤 탐색기 장식 공급, 캐시 요청 제출 |
| `ui` | `NativeApplication::frame_ui`(1397줄) | 앞부분 `tick_explorer_search_clipboards(&snapshot)`(1412줄)가 `git_decorations.mount(..)`(이번 프레임에 Files 보기로 보이는 프로젝트, `domains/explorer_search.rs` 1164-1165줄) |
| | 셸 그리기 → 탭 내용 `APP/domains/editor.rs` `tab_content`의 `match &tab.kind { .. Diff => show_diff_tab(5405줄), ClaudeDiff => show_claude_diff_tab(5409줄) .. }` | diff 표면 그리기, intent 수집 |
| | intent 루프 → `apply_git_intent` | 관찰·명령 제출 |
| | `show_git_file_history(&context)`(`application.rs` 2243줄) | 창별 파일 이력 dialog |

`logic`의 탐색기 장식 공급은 직전 프레임의 `mount` 결과를 봅니다(한 프레임 늦음). 응답이 오면 repaint가 요청되므로 다음 프레임에 반영됩니다. 닫는 중·종료 준비 중에는 git tick 전체가 돌지 않습니다.

### 2.4 diff 표면 공통 구조

모든 diff 표면(작업 트리·staged diff 탭, 파일 비교 탭, 커밋 diff 탭, ClaudeDiff 탭, 파일 이력 inline diff, 충돌 비교 대화상자)이 같은 함수를 지납니다.

1. 데이터: `git_cache::Key::{Diff, Compare, CommitDiff, ConflictSides}` 조회의 `read`(338줄)가 `with_hunks`(501-513줄)에서 `taide_git::service::diff_hunks(&original, &modified, false)`를 `services.tasks.run_blocking_result("native-git-diff-hunks", ..)`로 계산해 `Value::Diff(git_cache::Diff { sides: DiffSides { original, modified, language_id }, hunks: Vec<diff_view::Hunk> })`로 돌려줍니다. ClaudeDiff만 예외로 `Command::Hunks { tab, revisions, original, modified }`(dispatch 876-906줄)가 `diff_hunks(.., true)`(monaco 기본 `ignoreTrimWhitespace: true`)를 계산하고 `git_claude_diff::State::finish`가 문서 revision이 같을 때만 받습니다.
2. 문서: `show_surface(ui, Parts, Surface, Colors)`(1085-1172줄)가 두 쪽 가상 문서를 엽니다. 키는 `DocumentKey::Virtual(VirtualKey { owner: 탭 id 또는 표면이 만든 TabId::new(), side: Original | Modified })`. 쪽마다 `SideText::Read { text, fresh }`(응답이 새로 왔을 때만 `replace_virtual`, 같은 내용·언어면 무동작이고 revision도 그대로, `ED/store.rs` 685-687줄)나 `SideText::Editable(text)`(처음 한 번만 편집 가능하게 열고 이후 사용자 편집 소유, 언어만 맞춤). view는 `ViewKey { window: git_diff::window(side)("taide-diff-original"/"taide-diff-modified"), pane, tab: owner }`로 붙입니다.
3. 위젯: `taide_native_ui::diff_view::show(ui, store, editor, Request { original, modified, hunks, side_by_side, stage_lanes, hide_unchanged, presentation, colors }, &mut git_diff::SyntaxTokens(syntax))`. 위젯은 `presentation`을 복제해 folding·minimap·sticky scroll을 끄고(752-755줄), 폭이 `INLINE_BREAKPOINT = 900.0` 이하면 inline으로 그립니다.
4. 수명: 그려진 diff 탭은 `git_state.diffs.shown(tab)`으로 기록되고, `tick_git_cache`가 `diffs.finish_frame()` 뒤 `git_diff::release_hidden(store, &shown)`으로 이번 프레임에 그려지지 않은 owner의 가상 문서·view를 해제합니다(원본 pane 언마운트와 같음).
5. 구문 색: `show_surface` 끝에서 `syntax.show_lines(document, output.rendered_lines)`로 보이는 줄만 토큰화를 요청합니다. inline 모드에서는 `output.original`이 `None`이라 원본 문서 줄 토큰을 요청하지 않습니다(git-40 3.3.5의 출발점).

표면별 설정값(실제 호출처):

| 표면 | 호출처 | `side_by_side` | `stage_lanes` | `hide_unchanged` |
| --- | --- | --- | --- | --- |
| 작업 트리·staged·비교 diff 탭 | `AppSurfaces::show_diff`(2754줄) | `git.diffs.is_side_by_side(tab)`(Alt+\ 토글) | `!is_commit && git_diff::is_stageable(..)` | 설정값 |
| 커밋 diff 탭 | 같은 함수(`is_commit`) | 항상 `true` | `false` | 설정값 |
| ClaudeDiff | `show_claude_diff_tab`(2821줄) | `true` | `false` | `false` |
| 파일 이력 inline | `show_git_file_history`(2357줄 이하, 2406-2455줄) | `false` | `false` | 설정값 |
| 충돌 비교 대화상자 | `APP/git-editor.rs` `show_conflict_compare`(632줄) | `true` | `false` | 설정값 |

`diff_view::Hunk`(`UI/diff-view.rs` 51줄)는 libgit2 hunk 머리 숫자 그대로입니다. 1부터 세며, 줄 수가 0인 쪽의 `*_start`는 "그 줄 뒤"를 뜻합니다. monaco `LineRange`(1부터, 끝 제외)로 바꾸는 규칙은 `side_lines`(149줄)입니다.

```rust
fn side_lines(start: u32, lines: u32) -> Range<u32> {
    if lines == 0 {
        start + 1..start + 1
    } else {
        start..start + lines
    }
}
```

현재 `diff_view::Colors`(59-69줄): `inserted_line`(`diff.insertedLineBackground`), `removed_line`(`diff.removedLineBackground`), `diagonal_fill`(상수 dark `#cccccc33`/light `#22222233`), `border`(`diff.border`), `stage`(`git.staged`), `foreground`(`editor.foreground`), `unchanged_code`(상수 dark `#74747429`/light `#b8b8b829`), `unchanged_region`(`appSidebar.background`), `unchanged_region_foreground`(`app.foreground`). 값은 `APP/git-diff.rs` `Appearance::new`(32-55줄)가 테마에서 만듭니다. 새 색을 더하면 이 두 곳과 `native/taide-native-ui/tests/diff-view.rs`의 `COLORS` 상수(48줄)를 함께 고칩니다. TAIDE 테마의 diff 토큰은 `CR/taide-theme/src/service.rs` 381-385줄(dark)·519-523줄(light)의 `diff.insertedBackground`·`diff.insertedLineBackground`·`diff.removedBackground`·`diff.removedLineBackground`·`diff.border`이고, 원본 monaco 색 매핑은 `TS/shared/lib/monaco/theme.ts` 28-32줄(`diffEditor.insertedTextBackground` ← `diff.insertedBackground` 등)입니다.

### 2.5 검사 하네스

#### 2.5.1 `[C]` crate·순수 모듈

- `CR/taide-git/src/service.rs`의 tests 모듈(임시 저장소). 예: `diff_hunks는_같은_내용의_인덱스_대비_작업트리_hunk와_같고_그_범위로_stage된다`(3037줄), `diff_hunks는_앞뒤_공백_무시면_공백만_바뀐_줄을_hunk에서_뺀다`(3078줄). 실행: `cargo test --quiet --manifest-path crates/taide-git/Cargo.toml diff_hunks`.
- 앱 crate 순수 모듈 검사는 모듈 옆 `git-*-tests.rs`를 `#[cfg(test)] #[path = "git-cache-tests.rs"] mod tests;`로 등록합니다. 예: `APP/git-claude-diff-tests.rs` `그려진_요청이_어느_창에도_없으면_대기_목록에서_지운다`.
- `taide-native-editor`의 monaco 이식은 원본 코드를 bun으로 실행해 만든 기준 자료와 비교합니다. 기준 자료 생성 스크립트는 `docs/utils/<날짜>-monaco-<기능>-oracle.js`입니다(예: `docs/utils/2026-10-10-monaco-completion-preview-oracle.js`: `import { .. } from '../../node_modules/monaco-editor/esm/vs/..'`, `const ROOT = resolve(import.meta.dir, '../..')`, 결과를 `native/taide-native-editor/tests/fixtures/<기능>-reference.tsv`에 `writeFileSync`). 검사는 `native/taide-native-editor/tests/<기능>.rs`가 `include_str!("fixtures/<기능>-reference.tsv")`를 줄마다 읽어 비교합니다(`tests/completion-preview.rs` 119줄). 실행은 메인 저장소(node_modules가 있는 곳)에서 `bun /Users/hyunseokbyun/development/TAIDE/docs/utils/<파일>.js` 한 줄입니다. 생성 JSON을 `native/` 아래 두면 루트 `prettier --check .` 대상이므로 `.tsv`를 쓰거나 `.prettierignore`에 넣습니다(`docs/memory/2026-10-07-native-transition-operations.md` 6절). monaco 코드를 옮기면 `THIRD_PARTY_LICENSES.md`의 "Ported source in `native/taide-native-editor`" 절(629줄부터)에 항목을 더하고 MIT 문구는 `native/taide-native-editor/LICENSE-MONACO-SNIPPET`을 가리킵니다(같은 절 기존 항목 형식).

#### 2.5.2 `[U]` UI egui 하네스 (`native/taide-native-ui/tests/diff-view.rs`)

파일 첫 줄이 `#![cfg(all(feature = "native-host", feature = "inspection"))]`이라 feature 없이 돌리면 검사가 0개입니다. `Fixture`(70-205줄)가 `EditorStore`에 가상 문서 두 개를 열고 `diff_view::show`를 직접 부릅니다.

```rust
fn frame(&mut self, events: Vec<Event>) -> (Output, FullOutput) {
    self.time += FRAME_STEP;
    let mut shown = None;
    let mut output = self.context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.screen)),
            time: Some(self.time),
            events,
            focused: true,
            ..Default::default()
        },
        |ui| {
            shown = Some(
                diff_view::show(
                    ui,
                    &mut self.store,
                    &self.editor,
                    Request {
                        original: self.original,
                        modified: self.modified,
                        hunks: &self.hunks,
                        side_by_side: self.side_by_side,
                        stage_lanes: true,
                        hide_unchanged: self.hide_unchanged,
                        presentation: &self.presentation,
                        colors: COLORS,
                    },
                    &mut NoTokens,
                )
                .unwrap(),
            );
        },
    );
    output.textures_delta.clear();
    (shown.unwrap(), output)
}
```

단언 도구: `filled_tops(&output, color)`(그 색으로 채운 사각형의 top 목록, 207줄), `painted_texts(&output)`(그려진 글자, 221줄), `Output.original/modified.extra_zones`(zone 화면 사각형), `Fixture::press(pos)`(누름·뗌 두 프레임). 화면 크기 `WIDE`(1200×600)는 side-by-side, `NARROW`(900×600)는 inline입니다. 기존 검사 예(실제 코드):

```rust
#[test]
fn 좌우_보기는_채움_띠로_같은_줄을_같은_높이에_두고_줄_배경과_빗금을_그린다() {
    let mut fixture = Fixture::new(WIDE);
    fixture.frame(Vec::new());
    let (shown, painted) = fixture.frame(Vec::new());
    assert!(shown.is_side_by_side);
    // ... 원본 쪽 extra_zones, 줄 배경 top, 빗금 LineSegment, 같은 줄 높이 단언
}
```

`Request`에 필드를 더하면 이 `frame`의 리터럴도 고칩니다. 실행:

```
cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test diff-view --locked --offline --target-dir <TARGET>
```

#### 2.5.3 `[A]` 앱 lib 실제 App 검사

검사 파일은 `APP/application-git-<묶음>-tests.rs`이고 `APP/domains/git.rs` 끝(2989-3027줄)에 등록되어 있습니다: `fixture`(`application-git-fixture.rs`), `base_tests`, `changes_tests`, `header_tests`, `commit_tests`, `graph_tests`, `diff_tests`, `claude_history_tests`, `editor`(`git-editor.rs`, 검사 아님), `editor_tests`. 모듈 경로는 `crate::application::domains::git::<이름>`이므로 `application`의 비공개 항목(`submit`, `git_state`, `store`, `services`, `toasts`)을 검사에서 바로 씁니다.

공통 도구(`APP/application-git-base-tests.rs` 40-235줄, `pub(super)`):

| 도구 | 하는 일 |
| --- | --- |
| `Workspace::new(label, &["alpha"])` | 임시 디렉터리(`taide-native-git-<label>-<id>`)에 데이터 디렉터리·빈 template 디렉터리·프로젝트 root를 만들고 `ProjectId`·`ShellSlotId`를 정함 |
| `workspace.state(isolated_state(workspace.data.clone()))` | 프로젝트·세션·slot 트리를 넣은 `AppState` |
| `launch(&mut workspace.fixture, state)` | `project_rail_tests::launch`(`APP/application-project-rail-tests.rs` 120줄): tokio runtime + `NativeApplication::new` + `context.enable_accesskit()` |
| `frame(app, ctx, events)` | `logic` 뒤 화면 1280×800으로 `ui` 한 번, `FullOutput` 반환 |
| `until(app, ctx, "phase", \|app, output\| 조건)` | 조건이 참이 될 때까지 빈 프레임(20초 제한, 실패 메시지에 phase) |
| `settle(app, ctx)` | 6프레임 |
| `show_git(app, ctx)` | ⌃⇧G로 Git 보기 |
| `text(app, key)` | `presentation::message(&app.locale, key, &[])` |
| `node(output, label)`, `nodes(..)`, `has_text(..)`, `text_shapes(..)` | accesskit 노드와 그려진 글자 찾기 |
| `workspace.git(0, &["add", "--", "a.txt"])` | 검사 쪽 git CLI(`GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, `GIT_TERMINAL_PROMPT=0`, hooksPath 빈 디렉터리) |

Git 저장소 fixture(`APP/application-git-fixture.rs`): `fixture::init(root, empty)`(local config로 이름·hook·서명·자격 증명·protocol 고정), `fixture::repository(root, empty, &[("a.txt", "one\n")])`(init + 첫 커밋 `test(fixture): initial`), `fixture::git(root, empty, args)`. 이벤트 주입은 `application.services.events.publish(AppEvent::GitStatusChanged { project_id })`(같은 `PaintSink`로 들어감).

ClaudeDiff 도구(`APP/application-git-claude-history-tests.rs` 35-215줄, 파일 안 비공개 fn): `real(path)`(macOS `/var` → `/private/var` 정규화), `launch_git(workspace)`, `open_diff(app, path)`(실제 `ide_tools::dispatch("openDiff", ..)`를 runtime에 spawn해 `JoinHandle<Value>` 반환), `ide_tool(app, name, args)`, `answered(app, ctx, handle)`(IDE 응답이 올 때까지 프레임), `claude_tab(app, project)`, `virtual_text(app, owner, side)`, `click`, `type_text`, `rightmost`/`leftmost`(글자 위치). 실제 검사 첫 부분(이 파일 288-312줄):

```rust
#[test]
fn 실제_app_ide_open_diff는_claude_diff_탭을_열고_수정_쪽_편집_뒤_accept로_저장한다() {
    let mut workspace = Workspace::new("claude-accept", &["alpha"]);
    let root = workspace.roots[0].clone();
    std::fs::write(root.join("a.txt"), DISK).unwrap();
    let context = launch_git(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let path = real(&root.join("a.txt"));

    let pending = open_diff(application, &path);
    let output = until(
        application,
        &context,
        "claude diff tab",
        |application, output| {
            claude_tab(application, &project).is_some()
                && has_text(output, "GAMMA")
                && has_text(output, "gamma")
        },
    );
    // ...
}
```

보조 창 도구(`APP/application-auxiliary-window-tests.rs`, `pub(super)`): `install_viewports(&context)`(62줄, immediate viewport renderer를 설치해 보조 창 출력을 `Viewports.outputs[viewport]`에 모음, 입력은 `viewports.borrow_mut().events.insert(viewport, events)`), `step_until`(108줄), `file_tab`(125줄), `auxiliary_window(slot, tab)`(139줄), `viewport_of(app, project, slot)`(152줄), `focused_text_input`(832줄). 탭 메뉴 항목 선택 `choose_tab_menu(app, ctx, viewports, viewport: Option<ViewportId>, title, item) -> Vec<String>`(490줄)와 `tab_bar`(471줄)·`labels`(475줄)·`is_window_open`(455줄)은 비공개 `fn`이라 git 검사에서 쓰려면 `pub(super)`로 올려야 합니다(G1에서 같은 파일 helper 4개와 `Viewports`를 `pub(super)`로 올린 선례, `g1-base.md` 4절).

알림 도구(병합 미커밋 기준): `crate::application::domains::shell::notifications_tests::{FakePlatform, launch_with_platform}`(`pub(in crate::application)`). `launch_with_platform`은 accesskit을 켜지 않으므로 호출 뒤 `context.enable_accesskit()`를 부릅니다. 모듈이 `#[cfg(all(test, unix))]`입니다. `FakePlatform::sent()`는 `(제목, 본문)` 목록입니다.

실행(필터 실행, 동시 2개):

```
cargo build --quiet --manifest-path native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir <TARGET>
cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <TARGET> -- domains::git:: git_cache git_claude_diff --test-threads=2
```

`<TARGET>`: 메인 저장소에서는 `/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`(병합 게이트와 같음), git lane worktree에서는 `/Users/hyunseokbyun/development/TAIDE-lanes/targets/git`. 첫 줄 mock 빌드를 빼면 LSP를 쓰는 앱 검사가 실패합니다(`docs/memory/2026-10-07-native-transition-operations.md` 6절). `--test-threads=2`는 lane workflow 규칙, 병합 게이트는 4입니다.

#### 2.5.4 마무리 검증

UI·editor crate를 바꾸면 동결 Wasm 클라이언트 compile을 확인합니다(remote-web은 `taide-native-ui`(기본 feature 끔)·`taide-native-editor`에 의존, `native/taide-remote-web/Cargo.toml` 28-29줄).

```
cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline --target-dir <TARGET>
cargo fmt --manifest-path native/taide-native-<crate>/Cargo.toml -- --check
```

병합 뒤 전체 게이트는 `bash /Users/hyunseokbyun/development/TAIDE/docs/utils/2026-11-03-native-merge-gate.sh <로그 디렉터리>`입니다(lsp-mock 빌드, 앱 전체 `--no-fail-fast -- --test-threads=4`, UI `--features native-host,inspection` 전체, editor·syntax·terminal 전체, remote-web Wasm, 5개 crate fmt, 로그별 `test result: ok` 수와 `FAILED`·`error` 줄 요약). `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다` 1건은 git과 무관한 기존 실패입니다(`docs/PROCESS.md` W0·W1 기록, `g8-blame-conflicts.md` 4절).

### 2.6 금지·허락

- 금지(`LANE/docs/research/2026-10-28-lane-git-spec.md` 2.0.7, lane workflow `RULES`): GUI 실행, 실제 OS 알림 발송, 실제 Keychain·AI provider·clipboard·Finder reveal, **추적 안 된 파일 discard**(백엔드가 사용자 휴지통으로 옮김), 네트워크 원격, 저장소 밖 탐색, 작업 저장소 git 상태 변경, 허용 밖 의존성, `native/taide-native-app/tests/explorer-delete.rs` 실행, 동결 `native/taide-remote-web` 소스·Cargo 파일 변경.
- 허용(`2026-10-28-native-lane-spec-decisions.md` 마지막 채택 항목): git 검사의 git CLI가 사용자 전역 설정을 읽기 전용으로 참조하는 수준(fixture local config가 hook·서명·자격 증명을 덮음).
- 새 crate 의존성 없이 진행합니다. 문자 단위 diff용 crate(`similar`, `imara-diff` 등)는 lockfile에 없습니다(`Cargo.lock`·`native/*/Cargo.lock`의 diff 관련 항목은 `git2 0.21.0`·`libgit2-sys`뿐). monaco 알고리즘은 순수 Rust로 옮기고 `taide-native-editor`에 정규식·외부 의존을 더하지 않습니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md` "기각된 대안과 이유": editor crate에 정규식 의존성을 더하면 동결 remote-web lockfile과 의존 그래프가 바뀜).

---

## 3. 행별 상세

### 3.1 git-7 Sync — pull/push OS 알림 (병합 결과 확인 필요)

#### (a) ID·기능·상태

`git-7` Sync 버튼(pull 성공 후 push, 진행 스피너, 오류 toast) — **partial**. 남은 범위: pull/push OS 알림 4종(기능표 484줄, `g3-header-branches.md` 6절).

#### (b) 원본 TS 동작

- 버튼: `TS/widgets/git-panel/git-panel.tsx` 526-535줄. `hasRemote &&`일 때만 `IconButton`(label `t('git.sync')`, 진행 중 `Loader2 animate-spin` 아니면 `RefreshCw`, `disabled={isSyncing}`, tooltip 아래).
- 실행: `TS/widgets/git-panel/git-panel-container.tsx` 93-94줄(`usePushGit`, `usePullGit`), 206줄 `handleSync = () => pull(projectId, { onSuccess: () => push(projectId, { onError: notifyError }), onError: notifyError })`. pull이 실패하면 push를 하지 않고 오류 toast.
- 알림: `TS/entities/git/git.query.ts` 189-192줄 `GIT_REMOTE_NOTIFICATION_TITLE_KEY`(push/pull × succeeded/failed), 213-238줄 `useGitRemoteMutation`. pull·push 각각:
  - 성공: `notifyNative({ category: 'gitRemote', title: notificationTitle(t(succeeded)), body: joinNotificationBody([t(succeeded), queryClient.getQueryData<GitStatus>(QUERY_KEY.GIT.STATUS(projectId))?.branch]) })`. branch는 새로 조회하지 않고 캐시 값만 씁니다(없으면 본문은 문구만).
  - 실패: `category: 'error'`, 제목 규칙 같음(`failed` 키), 본문 `[t(failed), describeIpcError(error)]`.
  - 제목 `resolveProjectNotificationTitle`(`TS/shared/lib/notification-text.ts` 42줄): 프로젝트 표시 label → 프로젝트 이름 → 없으면 fallback 문구.
  - 본문 연결 `joinNotificationBody`(같은 파일 9·19줄): 빈 값·공백만 있는 값을 빼고 `' · '`로 연결.
  - 발송은 `TS/entities/notification/notify.ts` 37줄 `notifyNative` → 백엔드가 설정 gate(`notifications_enabled`, 범주 스위치 `notify_git_remote`/`notify_error`, `notifications_only_when_unfocused`)를 적용합니다(`CR/taide-notification/src/service.rs` 17-43줄 `decide_delivery`·`is_category_enabled`).
- locale 키: `git.sync`, `notification.gitPullSucceeded`("풀 완료"), `notification.gitPullFailed`("풀 실패"), `notification.gitPushSucceeded`("푸시 완료"), `notification.gitPushFailed`("푸시 실패")(`CR/taide-locale/resources/locales/ko.json` 470-473줄, en·ja 같은 키).
- 순서: hook 수준 `onSuccess`/`onError`(알림)가 먼저, `mutate` 수준 콜백(push 제출, toast)이 뒤입니다.

#### (c) native 현재 자산과 빠진 것

- 완료(g3 문서): `Intent::Sync(project)` → `syncing.insert` → `Command::Pull` → `Reply::Mutated { op: MutationOp::Pull }`에서 성공이면 `Command::Push` 제출(제출 실패면 `syncing` 해제), 실패면 `syncing` 해제·오류 toast. `MutationOp::Push` 응답에서 해제(`domains/git.rs` 1388-1427줄). 진행 중 아이콘 회전(`APP/git-scm.rs` `spinner`, 병합 미커밋에서 `Icon::LoaderCircle` → `Icon::Loader`로 이름만 바뀜).
- 진입점: shell lane이 만든 `NativeApplication::notify_event(context, category, project: Option<&ProjectId>, fallback_title_key, body)`(`APP/domains/shell.rs` 1176줄). 내부에서 `native_notifications::project_title`(표시 label → 이름 → fallback)과 `truncate(.., NOTIFICATION_TEXT_MAX_CODE_POINTS = 200)`(`APP/native-notifications.rs` 18줄)를 적용하고 `HostCommand::Shell(Command::Notify(Request { category, title, body, any_window_focused }))`를 제출합니다. 본문 연결은 `native_notifications::join_body(&[Option<String>])`(128줄, `NOTIFICATION_BODY_SEPARATOR = " · "`, 원본 `joinNotificationBody`).
- 병합 미커밋에서 확인한 연결: `notify_git_remote(&mut self, context, project, [succeeded, failed]: [&str; 2], result: &AppResult<()>)`가 성공이면 `(NotificationCategory::GitRemote, succeeded, cache.status_result(project).and_then(Result::ok).and_then(|status| status.branch.clone()))`, 실패면 `(NotificationCategory::Error, failed, Some(crate::toast::describe_error(&self.locale, error)))`로 본문을 만들고 `notify_event`를 부릅니다. `apply_git_reply`의 Pull 분기 첫 줄(1393줄), Push 분기 `syncing.remove` 다음 줄(1418줄)에서 호출합니다.

#### (d) 구현 방법

병합 작업자가 아래와 같이 처리 중입니다. 병합 결과가 다르면 이 단계대로 맞춥니다.

1. `APP/domains/git.rs` 상단 상수 블록에 두 상수를 둡니다.

```rust
const GIT_PULL_NOTIFICATION: [&str; 2] = [
    "notification.gitPullSucceeded",
    "notification.gitPullFailed",
];
const GIT_PUSH_NOTIFICATION: [&str; 2] = [
    "notification.gitPushSucceeded",
    "notification.gitPushFailed",
];
```

2. `impl NativeApplication`(같은 파일)에 `notify_git_remote`를 둡니다. 범주는 `taide_model::notification::NotificationCategory::{GitRemote, Error}`, 본문은 `crate::native_notifications::join_body(&[Some(presentation::message(&self.locale, title_key, &[])), detail])`, 마지막에 `self.notify_event(context, category, Some(project), title_key, body)`.
3. `apply_git_reply`의 `Reply::Mutated { op: MutationOp::Pull, .. }` 분기 첫 줄에 `self.notify_git_remote(context, &project, GIT_PULL_NOTIFICATION, &result);`, `MutationOp::Push` 분기에는 `syncing.remove` 다음 줄에 `GIT_PUSH_NOTIFICATION`으로 같은 호출. 원본처럼 알림이 toast·push 제출보다 먼저입니다. branch는 무효화 전에 읽어도 뒤에 읽어도 같습니다(`invalidate`는 값을 지우지 않고 stale만 표시).
4. 검사에서 쓰도록 `APP/domains/shell.rs`의 `mod notifications_tests;`를 `pub(in crate::application) mod notifications_tests;`로, `APP/application-notifications-tests.rs`의 `FakePlatform`·`FakePlatform::sent`·`launch_with_platform`을 `pub(in crate::application)`로 올립니다(shell 영역 파일의 가시성만 바꾸는 최소 수정, QA에 사유 기록).
5. 판단 기록용 차이: TanStack v5는 status 재조회가 실패해도 `getQueryData`가 마지막 성공 값을 돌려줍니다. native 캐시는 오류도 값으로 저장하므로 직전 status 조회가 실패했으면 성공 알림 본문에 branch가 빠집니다. 원본에서도 실패 직후 branch가 낡은 값이라 의미 있는 차이는 아니며, QA에 적습니다.

#### (e) 자동 검사

`APP/application-git-header-tests.rs`에 `[A]` 1개(병합 미커밋에 같은 이름으로 있음, 실행 결과는 이 문서 작성 시 확인하지 않음): `실제_app_sync의_pull과_push는_성공과_실패를_프로젝트_제목의_os_알림으로_보낸다`.

1. `Workspace::new("sync-notify", &["alpha"])`, `fixture::repository`, `bare_remote(&workspace)`(177줄, 같은 임시 디렉터리의 bare 원격), `branch.main.remote/merge` 설정, `push`·`fetch`, `commit_ahead(&workspace, "b.txt")`.
2. 상태 설정: `notifications_enabled = true`, `notifications_only_when_unfocused = false`(0.4절의 `focused: true` 때문), `notify_git_remote = true`, `notify_error = true`.
3. `launch_with_platform(&mut workspace.fixture, state, platform.clone())` 뒤 `context.enable_accesskit()`, `settle`, `show_git`.
4. Sync 버튼(accesskit label `git.sync`, tooltip 노드 제외 helper `button`)을 누르고 `syncing`이 비고 `platform.sent().len() == 2`가 될 때까지 진행 → `sent == [("alpha", "<gitPullSucceeded> · main"), ("alpha", "<gitPushSucceeded> · main")]`(문구는 `text(app, key)`로 만들어 locale 독립).
5. push 실패: bare 원격 `objects` 디렉터리 권한을 `0o555`로 바꾸고 다시 Sync → 3번째 `<gitPullSucceeded> · main`, 4번째 제목 `alpha`·본문이 `"<gitPushFailed> · "`로 시작하고 더 김. 권한 복구.
6. pull 실패: 원격 경로를 `rename`으로 옮기고 Sync → 5번째 본문이 `"<gitPullFailed> · "`로 시작, push 알림 없음(개수 5). 원래 경로로 복구.

실행: `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <TARGET> -- domains::git::header_tests --test-threads=2`. 이 검사는 `#[cfg(unix)]`입니다.

#### (f) 완료 판정

- 위 검사와 기존 `header_tests`(sync 순서·진행 중 비활성·pull 실패 push 미실행 포함) 통과.
- 기능표 git-7을 complete로, 비고에 알림 근거(검사 이름, 범주·제목·본문 규칙)를 적습니다. settings-23 행의 "git push/pull" 남은 범위를 이 근거로 지웁니다(settings-23 전체 complete는 검색 치환 범주와 범주 7종 묶음 검사 `application-settings-notifications-tests.rs`가 더 필요하다고 settings-23 비고에 적혀 있음).

#### (g) 막는 요소

- 실제 OS 알림 표시는 사용자 허락 항목(`2026-10-28-native-lane-spec-decisions.md` 보류 표 "실제 OS 알림 표시")이라 가짜 platform으로만 검사합니다. 실기 확인은 하지 않습니다.
- 병합 진행 중: 병합 작업자가 끝내기 전에는 이 행을 고치지 않습니다. 끝난 뒤 위 검사만 다시 돌려 확인합니다.

### 3.2 git-38 탐색기 Git 장식 공급 (병합 결과 확인 필요)

#### (a) ID·기능·상태

`git-38` Explorer 파일 트리 Git 데코레이션(변경 종류별 색, 폴더로 우선순위 전파) — **partial**(기능표 515줄). explorer-search-7과 같은 기능입니다(D1 A: 파생·렌더·주입점은 explorer-search, 공급은 git).

#### (b) 원본 TS 동작

- 공급: `TS/widgets/explorer/explorer-container.tsx` 65줄 `const { data: gitStatus } = useQuery(gitStatusQueryOptions(projectId))`, 90-91줄 `buildFileTreeGitStatusByPath(gitStatus?.rows ?? [], project?.root ?? null)`로 행마다 `gitStatus`.
- 파생: `TS/widgets/explorer/file-tree-git-status.ts` 48-66줄. 충돌이면 conflicted, 아니면 staged·unstaged 종류(typeChange→modified) 중 우선순위 높은 것(conflicted 6 > added 5 > untracked 4 > renamed 3 > modified 2 > deleted 1, 24-31줄). 프로젝트 root 아래 조상 디렉터리로 같은 규칙 전파(`startsWith(root + '/')`). 키는 절대 경로.
- 렌더: `TS/features/explorer/file-tree-row.tsx` 25-42·77-81줄. 글자색 `explorer.gitAdded/gitModified/gitDeleted`, `git.renamed`, `explorer.gitUntracked`, `git.conflicted`. 파일은 오른쪽 배지 A/M/D/R/U/!, 디렉터리는 6px 점.
- 갱신: git status 쿼리이므로 git 이벤트·fs 이벤트·변경 성공 무효화를 그대로 받습니다(`TS/entities/git/git.query.ts`의 `useGitMutation`, `TS/app/providers/ipc-sync-provider.tsx`).

#### (c) native 현재 자산과 빠진 것

- explorer-search 영역(메인 병합분): `APP/explorer-git-decorations.rs` — `Decoration`(6종, `badge`·`name`·`theme_key`), `build(rows, root)`(114줄), `GitDecorations::{set(project, Option<&GitStatus>)(160줄), mounted_projects()(175줄), get(project), mount(..), forget(project)}`. `set(None)`은 그 프로젝트 장식을 지우고, 같은 행 목록이면 다시 계산하지 않습니다. `mount`는 `tick_explorer_search_clipboards`가 매 프레임 `visible_explorer_files(snapshot)`(Files 보기로 보이는 mount: 메인은 zen 아님·사이드바 펼침, 보조 창은 탐색기 열림)로 부릅니다. 행 렌더는 `APP/explorer.rs`의 `decorations: Option<&HashMap<String, Decoration>>`(234줄)·`icons.decorations?.get(&row.path)`(1441줄)·accesskit description `decoration.name()`(1508줄), 색은 `git_color`(213줄).
- explorer-search QA(`docs/quality-assurance/2026-11-02-native-lane-explorer-search-explorer-rows.md` 46줄): "공급자는 없습니다(wave 1). git lane이 `domains/git.rs`에서 `mounted_projects()` 각 프로젝트에 `GitCache`를 걸고 `explorer_search_state.git_decorations.set(..)`을 부릅니다." 메인 저장소에는 `set` 호출자가 없습니다(`grep -rn git_decorations native/taide-native-app/src` 결과에 `.set(`이 없음).
- git 사양 2.0.3과 g1 QA 6절은 `APP/git-status-supply.rs`(공개 API `status`, `generation`)를 전제로 했지만 그 파일은 만들어지지 않았습니다. 실제 API는 위 `GitDecorations`입니다.
- 병합 미커밋 상태: `tick_git_cache`에 공급 반복이 들어갔고(0.3절), 제목줄 branch도 캐시로 옮겨졌으며(`State::title_branch`), App 검사가 추가됐습니다.

#### (d) 구현 방법 (병합 결과가 다르면 이대로 맞춤)

1. `APP/domains/git.rs` `tick_git_cache`에서 캐시 요청을 내기 직전(`for request in self.git_state.cache.requests(..)` 앞)에 다음 반복을 둡니다. `now`는 함수 앞부분의 `let now = Instant::now();`를 재사용합니다.

```rust
for project in self.explorer_search_state.git_decorations.mounted_projects() {
    self.git_state.cache.observe(&project, Key::Status, now);
    self.explorer_search_state.git_decorations.set(
        &project,
        self.git_state
            .cache
            .status_result(&project)
            .and_then(Result::ok),
    );
}
```

2. 관찰은 매 프레임 해야 합니다. `GitCache::requests`는 이번 라운드에 관찰된 항목만 요청하고 관찰 표시를 지웁니다(`std::mem::take(&mut entry.is_observed)`, `APP/git-cache.rs` 159-193줄). 탐색기가 계속 보이는 동안은 "재부착"이 아니므로 시간 stale이 걸리지 않고 이벤트 무효화로만 다시 읽습니다(TanStack 관찰자와 같음, g1 QA 3절 설계 판단).
3. 제목줄 branch(사양 2.0.3 "공급이 흡수되지 않았으면 이 캐시의 status를 읽도록"): `APP/title-branch.rs`와 `HostCommand::Git(Command::GitBranch)` 경로를 지우고, `tick_git_title_branches`가 제목줄에 보이는 프로젝트(focus 프로젝트 + 보조 창 프로젝트)를 `Key::Status`로 관찰하며, `GitSurfaces::branch`가 `State::title_branch`를 읽습니다. 기존 검사의 `application.git_state.title_branches.branch(&p)`는 `application.git_state.title_branch(&p)`로 바뀝니다(`application-git-base-tests.rs` 416줄, `application-auxiliary-window-tests.rs` 1107·1196줄 — 병합 미커밋에서 이미 바뀜).
4. 오류 처리 차이(판단 기록용): TanStack v5는 재조회가 실패해도 `data`에 마지막 성공 값을 남기므로 원본 탐색기는 오류 뒤에도 이전 장식을 그립니다. native 캐시는 오류도 값으로 저장해 `status_result`가 `Err`가 되고 `set(None)`으로 장식이 지워집니다. 오류가 성공 뒤에 오는 경우(예: `.git` 삭제)에만 다르며, 원본의 낡은 장식은 결함으로 보고 재현하지 않는 것을 권장합니다. 채택한 쪽을 QA에 적습니다.

#### (e) 자동 검사

- `[A]` `APP/application-git-base-tests.rs` `실제_app_탐색기_git_장식은_git_cache_status로_파일_배지와_상위_폴더를_그리고_git_변경_뒤_갱신한다`(병합 미커밋): root를 `canonicalize`(4절 함정 2), `a.txt` 수정·`docs/new.txt` 추가 → Files 보기에서 `a.txt` 행 accesskit description `"modified"`, `docs` 행 `"untracked"`, 캐시 status `Ok`, 배지 `M`의 글자색 = 테마 `explorer.gitModified` → 테스트 쪽 `git commit -am` + `GitStatusChanged` 주입 → `a.txt` description 없음, `docs`는 그대로.
- `[C]`·`[U]` 회귀: `APP/explorer-git-decorations-tests.rs`(파생 9사례 + 저장소 1), `APP/explorer-tests.rs` 장식 렌더 검사.
- 제목줄 회귀: `application::auxiliary_window_tests`의 title branch 검사 2개(1061·1133줄 부근), `base_tests`의 비저장소 검사.

```
cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <TARGET> -- domains::git::base_tests explorer_git_decorations explorer::tests application::auxiliary_window_tests --test-threads=2
```

#### (f) 완료 판정

- 위 검사 통과. 기능표 git-38과 explorer-search-7을 같은 근거로 complete. 비고에 공급 경로(`tick_git_cache` → `GitDecorations::set`)와 검사 이름을 적습니다. 오류 처리 차이(d-4)를 QA에 기록합니다.

#### (g) 막는 요소

- 병합 진행 중(0.3절). 병합 작업자가 끝낸 결과를 확인만 합니다.

### 3.3 git-40 Diff 렌더 — 남은 monaco 기본 장식

#### (a) ID·기능·상태

`git-40` Diff 렌더(side-by-side, 줄 단위 변경 하이라이트, 공백 변경도 diff, 읽기 전용, 자동 레이아웃, 언어별 하이라이트) — **partial**(기능표 517줄). 남은 범위(결정 4 A): 문자 단위 강조, ± 표시, diff overview ruler, inline 삭제 줄의 구문 색과 원본·수정 두 줄 번호 열(`g5-diff.md` 5절). editor-76 행의 남은 것과 같습니다.

#### (b) 원본 TS 동작

- `TS/features/git/diff-view.tsx` 46-50줄: `monaco.editor.createDiffEditor(container, { automaticLayout: true, readOnly: true, ignoreTrimWhitespace: false })`, 이후 76줄 `updateOptions({ renderSideBySide })`, 80줄 `updateOptions({ hideUnchangedRegions: { enabled }, experimental: { showMoves } })`. 그 밖은 monaco 기본값입니다.
- monaco 0.56 기본값(`MONACO/editor/common/config/diffEditor.js` 5-36줄): `enableSplitViewResizing: true`, `renderSideBySide: true`, `renderMarginRevertIcon: true`, `renderGutterMenu: true`, `maxComputationTime: 5000`, `ignoreTrimWhitespace: true`, `renderIndicators: true`, `originalEditable: false`, `renderOverviewRuler: true`, `diffAlgorithm: 'advanced'`, `experimental.showEmptyDecorations: true`, `hideUnchangedRegions { contextLineCount: 3, minimumLineCount: 3, revealLineCount: 20 }`, `renderSideBySideInlineBreakpoint: 900`, `useInlineViewWhenSpaceIsLimited: true`, `compactMode: false`.
- 문자 단위 강조(`MONACO/editor/browser/widget/diffEditor/components/diffEditorDecorations.js` 19-109줄): 각 줄 mapping에
  - 원본 쪽 비지 않으면 `line-delete`(줄 배경) + indicator, 수정 쪽 비지 않으면 `line-insert`.
  - 한쪽이 비면(순수 삽입·삭제) 다른 쪽 줄 전체에 `diffWholeLineAddDecoration`/`diffWholeLineDeleteDecoration`(`className: 'char-insert'`/`'char-delete'`, **`isWholeLine: true`**, `MONACO/.../diffEditor/registrations.contribution.js` 58-76줄). 즉 줄 배경 위에 문자 배경 색이 줄 폭 전체로 한 겹 더 칠해집니다.
  - 둘 다 비지 않으면 `innerChanges` 각각에 `char-delete`(원본)·`char-insert`(수정)(`diffDeleteDecoration`/`diffAddDecoration`, `shouldFillLineOnLineBreak: true`). 범위가 비었고 `showEmptyDecorations`면 `diff-range-empty` 클래스(왼쪽 3px 테두리, `style.css` 147-154줄).
  - 색: `char-insert` = `diffEditor.insertedTextBackground` → TAIDE `diff.insertedBackground`, `char-delete` = `diffEditor.removedTextBackground` → `diff.removedBackground`(`TS/shared/lib/monaco/theme.ts` 28-32줄).
- 내부 변경 계산: `MONACO/editor/common/diff/defaultLinesDiffComputer/defaultLinesDiffComputer.js` 160-182줄 `refineDiff`. 줄 mapping 범위를 `LinesSliceCharSequence`(공백 무시 여부 `considerWhitespaceChanges = !ignoreTrimWhitespace`, 38줄)로 만들고, 길이 합이 500 미만이면 `DynamicProgrammingDiffing`, 아니면 `MyersDiffAlgorithm`, 이어서 `optimizeSequenceDiffs` → `extendDiffsToEntireWordIfAppropriate`(단어 = `[a-zA-Z0-9]`, `linesSliceCharSequence.js` 153-157줄 `isWordChar`) → (`extendToSubwords`는 diff 편집기가 넘기지 않아 끔, `MONACO/editor/browser/widget/diffEditor/diffEditorViewModel.js` 209-213줄) → `removeShortMatches` → `removeVeryShortMatchingTextBetweenLongDiffs`. 시간 제한(`maxComputationTime` 5000ms의 `DateTimeout`)을 넘으면 `DiffAlgorithmResult.trivialTimedOut`(범위 전체 한 덩어리)입니다(`algorithms/myersDiffAlgorithm.js` 39-40줄).
- ± 표시(`registrations.contribution.js` 25-40줄): `diffLineAddDecorationBackgroundWithIndicator`는 `linesDecorationsClassName: 'insert-sign codicon-add'`(U+EA60, `MONACO/base/common/codiconsLibrary.js` 11줄)와 `marginClassName: 'gutter-insert'`, 삭제는 `delete-sign codicon-remove`(U+EB3B, 308줄)·`gutter-delete`. CSS `style.css` 200-208줄: 11px, `opacity: 0.7`, 세로 가운데. 글자색은 지정이 없어 `.monaco-editor { color: var(--vscode-editor-foreground) }`(`MONACO/editor/browser/widget/codeEditor/editor.css` 20-28줄)을 상속합니다. `gutter-insert`·`gutter-delete`는 margin(줄 번호 칸) 배경을 `diffEditorGutter.*LineBackground` → 없으면 `diffEditor.insertedLineBackground`/`removedLineBackground`로 칠합니다(`style.css` 274-294줄, TAIDE는 gutter 전용 색을 매핑하지 않음).
- overview ruler(`MONACO/.../diffEditor/features/overviewRulerFeature.js`): 전체 폭 30px(`ONE_OVERVIEW_WIDTH = 15` × 2, 42-43줄)를 diff 편집기 오른쪽 끝에 둡니다(`diffEditorWidget.js` 92-102줄: 수정 편집기 폭 = 전체 − 수정 시작 − ruler 폭). 왼쪽 15px = 원본 편집기 ruler(삭제 범위), 오른쪽 15px = 수정 편집기 ruler(삽입 범위)(130-150줄). 색(58-59줄): `diffEditorOverview.insertedForeground`가 없으면 `diffEditor.insertedTextBackground`의 alpha를 2배(`transparent(2)`), 삭제도 같음(TAIDE는 overview 색을 매핑하지 않음). 수정 편집기 스크롤 위치에 맞는 viewport 사각형(`diffViewport`, 배경 `scrollbarSlider.background` → TAIDE `scrollbar.thumb`, hover/active `scrollbar.thumbHover`, `theme.ts` 175-177줄). ruler 배경 dark `rgba(255,255,255,0.01)`, light `rgba(0,0,0,0.03)`(`style.css` 183-184줄). ruler에서 누르면 수정 편집기 세로 스크롤바 누름으로, 휠은 수정 편집기 스크롤로 넘깁니다(68-74줄).
- inline 삭제 줄(`MONACO/.../components/diffEditorViewZones/diffEditorViewZones.js` 172-237줄): 원본 줄의 토큰(`originalModel.tokenization.getLineTokens`, 186줄)으로 구문 색을 칠하고, 내부 변경 원본 범위에 `char-delete`, margin에 줄마다 `delete-sign`(198줄, renderIndicators). margin의 전구 버튼(`inlineDiffDeletedCodeMargin.js`)은 3.3.8 참고.
- inline 두 줄 번호 열(`diffEditorWidget.js` 104-114줄): inline이면 원본 편집기를 폭 `max(5, layoutInfoDecorationsLeft)`(= 원본 편집기의 gutter 폭)로 수정 편집기 왼쪽에 둡니다. 그래서 원본 줄 번호 열과 수정 줄 번호 열이 나란히 보입니다. 원본 쪽에는 변경마다 수정 쪽 높이의 빈 zone(`gutter-delete` margin, 230줄)을 넣어 두 열의 줄을 맞춥니다. inline에서 원본 편집기 glyph margin은 꺼집니다(`components/diffEditorEditors.js` 119줄 `result.glyphMargin = renderSideBySide`).
- hunk 안 정렬(side-by-side, `diffEditorViewZones.js` 387-492줄 `computeRangeAlignment`, 95줄 `innerHunkAlignment = renderSideBySide`): 내부 변경이 줄 중간에서 시작하거나(원본·수정 시작 열이 둘 다 1보다 큼) 줄 끝 전에 끝나면 그 줄 경계에서 정렬을 끊어, 한 hunk 안에서도 같은 줄끼리 같은 높이에 오도록 filler를 여러 개로 나눕니다(`emitAlignment` 정의 443-475줄, 내부 변경 정렬 476-490줄, hunk 끝 강제 정렬 491줄).

#### (c) native 현재 자산과 정확히 빠진 것

| 항목 | 현재 코드 | 빠진 것 |
| --- | --- | --- |
| 내부 변경 | `git_cache::Diff { sides, hunks }`(`APP/git-cache.rs` 390-393줄), `with_hunks`는 `diff_hunks`만 계산 | 문자 단위 범위 계산과 전달 경로 전부 |
| 문자 강조 | `diff_view::show`의 장식은 `LineBackground`(줄 배경), `Lane { mark: Bar }`(stage), 미변경 영역 배경뿐(`UI/diff-view.rs` 761-832·895-905줄) | `DecorationKind::Inline(InlineStyle { background })` 장식, 순수 삽입·삭제 줄의 두 번째 줄 배경, 빈 범위 3px 표시 |
| ± 표시 | 없음 | lane 칸의 codicon 글자 |
| gutter 배경 | `UI/editor-paint.rs` `line_backgrounds`(204-217줄)가 `self.text_rect.x_range()`만 칠함 | 줄 번호 칸 배경 |
| overview ruler | `show`가 `ui.available_rect_before_wrap()` 전체를 두 편집기에 나눔. 각 편집기의 기존 스크롤바 overview(`UI/editor-overview.rs` `marks`·`paint`)는 `DecorationKind::Overview` 장식만 읽음 | 30px 열(또는 결정 4 A 문구대로 기존 Overview 장식 재사용), 두 ruler, viewport 사각형, 누름·휠 |
| inline 삭제 줄 | `paint_removed_lines`(625-673줄)가 `colors.foreground` 단색 galley로 그림. `show_surface`는 inline에서 원본 문서 토큰을 요청하지 않음(`output.original`이 `None`) | 원본 토큰 색, 문자 삭제 강조, margin `-` 표시 |
| inline 줄 번호 | inline이면 수정 편집기 하나만 그림(`Output.original: None`, 883-889줄) | 원본 gutter 열 |
| hunk 안 정렬 | `fillers`(266-295줄)가 hunk 끝에 차이만큼 한 개 | 내부 변경 기준 정렬 분할 |

#### (d) 구현 방법

작업을 3.3.1~3.3.7로 나눕니다. 각 단계 끝에서 그 단계 검사를 통과시킨 뒤 다음으로 갑니다.

##### 3.3.1 내부 변경 계산 이식 (`taide-native-editor`)

1. 새 파일 `ED/diff-refine.rs`, `ED/lib.rs`에 `#[path = "diff-refine.rs"] pub mod diff_refine;`(알파벳 순서 위치, 기존 kebab-case 파일 관례).
2. 공개 타입(monaco 좌표계 그대로, 1부터, 열은 UTF-16 단위):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: u32,
    pub end_exclusive: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeMapping {
    pub original: TextRange,
    pub modified: TextRange,
}

pub struct Refined {
    pub mappings: Vec<RangeMapping>,
    pub hit_timeout: bool,
}

pub fn refine(
    original_lines: &[&str],
    modified_lines: &[&str],
    (original, modified): (LineRange, LineRange),
    consider_whitespace_changes: bool,
    is_valid: &mut dyn FnMut() -> bool,
) -> Refined
```

   `is_valid`는 monaco `ITimeout.isValid()` 대응입니다. editor crate는 Wasm으로도 컴파일되므로(remote-web 의존) `std::time::Instant`를 직접 쓰지 않고 호출 측이 마감 시각 클로저를 넘깁니다. 앱은 `let deadline = Instant::now() + Duration::from_millis(MAX_COMPUTATION_MS); move || Instant::now() < deadline`(`MAX_COMPUTATION_MS = 5000`, monaco `maxComputationTime`)을 씁니다.
3. 옮길 원본(`MONACO/editor/common/diff/defaultLinesDiffComputer/`): `algorithms/diffAlgorithm.js`(`SequenceDiff`, `OffsetPair`, `DiffAlgorithmResult`), `algorithms/dynamicProgrammingDiffing.js`(101줄), `algorithms/myersDiffAlgorithm.js`(159줄), `linesSliceCharSequence.js`(205줄: 공백 trim, `translateRange`, `findWordContaining`, `countLinesIn`, `isStronglyEqual`, 경계 점수), `heuristicSequenceOptimizations.js`(374줄: `optimizeSequenceDiffs`, `extendDiffsToEntireWordIfAppropriate`, `removeShortMatches`, `removeVeryShortMatchingTextBetweenLongDiffs`), `defaultLinesDiffComputer.js`의 `refineDiff`(160-182줄), `../rangeMapping.js`의 `toRangeMapping2`. 문자열은 줄마다 `encode_utf16()`으로 바꿔 `Vec<u16>`에서 계산합니다(원본 `charCodeAt`). 재귀는 명시 스택으로 바꾸고 입력 길이·작업 예산 상한을 둡니다(JSON 이식의 관례, `docs/PROCESS.md` b3 항목들).
4. libgit2 hunk → `LineRange` 변환은 `UI/diff-view.rs`의 `side_lines`와 같은 규칙입니다. 이제 두 곳에서 쓰므로 `diff_refine::LineRange::from_hunk_side(start, lines)`로 옮기고 `side_lines`를 그것으로 바꿉니다(공통화 규칙 "2회 이상").
5. oracle: `docs/utils/<날짜>-monaco-diff-refine-oracle.js`가 `DefaultLinesDiffComputer`·`SequenceDiff`·`OffsetRange`·`InfiniteTimeout`을 import해 `new DefaultLinesDiffComputer().refineDiff(originalLines, modifiedLines, new SequenceDiff(new OffsetRange(o0, o1), new OffsetRange(m0, m1)), InfiniteTimeout.instance, considerWhitespaceChanges, {})`를 사례마다 실행하고 `mappings`의 좌표를 `native/taide-native-editor/tests/fixtures/diff-refine-reference.tsv`에 씁니다. 사례: 단어 하나 변경, 공백만 변경(두 모드), 줄 추가·삭제가 섞인 hunk, 빈 줄, 탭, BMP 밖 문자(UTF-16 2단위), 한글, 500자 경계 양쪽(DP/Myers 전환), 짧은 일치 제거가 일어나는 입력.
6. 라이선스: `THIRD_PARTY_LICENSES.md` "Ported source in `native/taide-native-editor`" 절에 `src/diff-refine.rs` 항목(원본 파일 목록, 사례 수, MIT 문구 위치 `native/taide-native-editor/LICENSE-MONACO-SNIPPET`).

##### 3.3.2 계산 결과 전달과 문자 강조

1. `APP/git-cache.rs` `Diff`에 `pub inner: Vec<taide_native_editor::diff_refine::RangeMapping>`를 더합니다.
2. `APP/domains/git.rs` `with_hunks`의 blocking task에서 `diff_hunks(&original, &modified, false)` 다음에, 두 쪽 줄 수가 모두 0이 아닌 hunk마다 `refine(.., consider_whitespace_changes: true, ..)`를 부르고 결과를 모읍니다. 순수 삽입·삭제 hunk는 refine하지 않습니다(원본도 줄 전체 장식만).
3. ClaudeDiff: `Command::Hunks`의 계산을 `diff_hunks(.., true)` + `refine(.., consider_whitespace_changes: false, ..)`로 넓히고, `Reply::Hunks.result`와 `git_claude_diff::State`의 `Lines`에 `inner`를 더해 `State::inner(&TabId) -> &[RangeMapping]`을 둡니다. revision 검사(`finish`)는 그대로입니다.
4. `UI/diff-view.rs` `Request`에 `pub inner: &'a [RangeMapping]`, `Colors`에 `inserted_text`·`removed_text`를 더합니다. `APP/git-diff.rs` `Appearance::new`에서 `color("diff.insertedBackground")?`·`color("diff.removedBackground")?`.
5. `show` 안에서 쪽별 장식 목록에 더합니다.
   - 한쪽 줄 수가 0인 hunk: 다른 쪽 줄 범위에 `DecorationKind::LineBackground(rgba(colors.inserted_text 또는 removed_text))`를 기존 줄 배경과 별도로 하나 더 넣습니다(원본 `isWholeLine: true` 문자 배경). `UI/editor-paint.rs` `line_backgrounds`는 같은 줄의 `LineBackground` 장식마다 사각형을 따로 칠하므로 두 반투명 색이 원본처럼 겹칩니다.
   - 내부 변경: 원본 범위 → 원본 문서, 수정 범위 → 수정 문서에 `DecorationKind::Inline(InlineStyle { background: Some(rgba(..)), ..Default::default() })`. 줄바꿈을 넘는 범위(`shouldFillLineOnLineBreak`)가 줄 끝까지 칠해지는지는 `UI/editor-paint.rs` `range_overlays`의 `row.extent(&decoration.bytes)`(235줄)로 결정되므로 `[U]` 검사로 확인합니다.
   - 빈 범위(원본 `diff-range-empty`, 3px 왼쪽 테두리): native 빈 장식 폭은 2px(`UI/editor-paint.rs` 25줄 `EMPTY_DECORATION_WIDTH`)라 다릅니다. 편집기를 그린 뒤 `output.geometry.caret_rect(byte)`(공개, `UI/editor-geometry.rs` 74줄) 왼쪽에서 폭 3px 사각형을 같은 색으로 칠합니다(상수 `EMPTY_CHANGE_WIDTH: f32 = 3.0`).
   - (line, UTF-16 column) → 바이트: `let line_char = rope.line_to_char(line - 1); let unit = rope.char_to_utf16_cu(line_char) + (column - 1) as usize; rope.char_to_byte(rope.utf16_cu_to_char(unit))`(ropey `=1.6.1`, `ED/Cargo.toml` 14줄). 줄 끝을 넘으면 줄 끝으로 자릅니다.
   - 장식 층: 기존 `layer(store, document, items)`(339-360줄)는 줄 범위를 받아 줄 시작 바이트로 바꿉니다. 문자 범위용으로 바이트 범위를 그대로 받는 별도 helper를 둡니다(`Decoration { bytes, kind, stickiness: Stickiness::NeverGrowsWhenTypingAtEdges }`, 같은 `snapshot.revision`과 `DECORATION_Z_ORDER`).
6. inline 모드: 수정 쪽은 같고, 원본 쪽 문자 삭제는 3.3.5에서 삭제 줄 그림에 넣습니다.

##### 3.3.3 ± 표시와 gutter 배경

1. ±: 편집기를 그린 뒤 `diff_view::show`에서 각 쪽 `EditorOutput.geometry.rows`(`pub(crate) rows: Arc<[Row]>`, `UI/editor-geometry.rs` 36줄, 같은 crate라 접근 가능, `lane_line`이 이미 씀)를 돌며, 행의 `segment.line + 1`이 변경 줄 집합(원본 = `side_lines(old)`, 수정 = `side_lines(new)`)에 있고 줄의 첫 segment인 행마다 lane 칸(`crate::editor_gutter::lane_column(geometry.gutter_rect, has_folding)`, `UI/editor-gutter.rs` 184줄) 가운데에 codicon 글자를 그립니다. 글자: 수정 `'\u{ea60}'`(add), 원본 `'\u{eb3b}'`(remove). 폰트 `FontId::new(INDICATOR_SIZE, crate::font_families::codicons(ui))`(기존 `unchanged_bar`와 같은 방식), `INDICATOR_SIZE = 11.0`, 색 `colors.foreground.gamma_multiply(INDICATOR_OPACITY)`, `INDICATOR_OPACITY = 0.7`. stage 막대(lane 칸 왼쪽 3px)와 같은 칸에 겹쳐 그리는 것이 원본과 같습니다(원본도 `taide-diff-gutter-stage::before`와 `insert-sign`이 같은 lines-decorations 칸, `TS/shared/styles/global.css` 424-446줄).
2. gutter 배경: 줄 번호 글자 아래에 칠해야 하므로 편집기 그리기 뒤 덧칠하면 안 됩니다. `UI/editor_surface.rs` `SurfaceExtras`에 `pub gutter_backgrounds: Vec<(Range<usize>, Color32)>`(0부터 줄 범위)를 더하고, 편집기 gutter를 그리기 직전에 그 행들의 `gutter_rect` 폭 사각형을 칠합니다. diff 위젯은 변경 줄에 `colors.inserted_line`/`removed_line`을 넘깁니다. `SurfaceExtras`는 `Default`를 derive하므로 다른 생성처는 바뀌지 않고, 리터럴 생성처인 `diff_view::show_side`(428-431줄)만 고칩니다.
3. 색·크기 상수는 `UI/diff-view.rs` 상단 `const`로 둡니다.

##### 3.3.4 inline 원본 줄 번호 열

1. inline 분기(`if !is_side_by_side { .. }`, 834줄)에서 원본 편집기도 그립니다. 폭은 원본 편집기의 gutter 폭(`EditorGeometry.gutter_rect.width()`의 직전 프레임 값, 첫 프레임은 수정 쪽 gutter 폭과 같은 값)이고 최소 `ORIGINAL_GUTTER_MIN = 5.0`(원본 `max(5, ..)`), 수정 편집기는 그 오른쪽 나머지입니다.
2. 줄 맞춤: 원본 쪽 zone = hunk마다 원본 범위 끝 뒤에 `new_lines × 줄 높이`(수정 쪽 삽입 줄 높이), 수정 쪽 zone = 기존 `inline_zones`(원본 삭제 줄 높이). 두 편집기의 스크롤은 side-by-side와 같은 동기화 코드(992-1018줄)를 씁니다.
3. 원본 열은 줄 번호만 보이고 본문은 잘립니다(폭이 gutter뿐). 삭제 줄 배경(`removed_line`)이 원본 줄 번호 칸에도 보이도록 3.3.3의 gutter 배경을 원본 쪽에 넣습니다.
4. `Output.original`이 inline에서도 `Some`이 되므로, 앱 `show_surface`의 `syntax.show_lines` 호출과 inline을 가정한 기존 검사(`폭_900_이하는_수정_쪽만_그리고_삭제_줄을_띠_안에_보인다`, `tests/diff-view.rs` 327줄)의 단언을 새 동작에 맞게 바꿉니다. 바꾼 단언과 이유를 QA에 적습니다.

##### 3.3.5 inline 삭제 줄의 구문 색·문자 삭제·`-` 표시

1. `show_surface`가 inline일 때도 원본 문서 토큰을 요청합니다: 삭제 hunk의 원본 줄 범위들의 합(최소~최대)을 `syntax.show_lines(documents[0], range)`.
2. `paint_removed_lines`에 `tokens: &mut dyn Tokens`를 넘겨 `tokens.tokens(store, original)`이 있으면 줄마다 `RowText::expanded(line_text, RowColumns { tab_size, start_column: 0.0, indent_columns: 0 }, colors.foreground)` → `text.highlight(RowTokens { spans: tokens.lines.spans(line), styles: tokens.styles, row_start_byte: 0 })` → `text.layout_job(&editor.appearance.font)`로 galley를 만듭니다(`UI/editor-row-text.rs` 94·384·479줄, 같은 패턴 `UI/editor_surface.rs` 3495-3510줄). 토큰이 없으면 지금처럼 단색입니다.
3. 문자 삭제: 그 줄에 걸친 내부 변경 원본 범위를 배경 사각형으로 글자 아래에 칠합니다(galley를 그리기 전에 `galley.pos_from_cursor`로 x 범위 계산, 색 `colors.removed_text`).
4. margin `-`: 삭제 줄마다 lane 칸에 `'\u{eb3b}'`(3.3.3과 같은 크기·색).

##### 3.3.6 hunk 안 정렬

1. `fillers(hunks, line_height)`를 `fillers(hunks, inner, original_line_max_columns, line_height)`처럼 넓혀 원본 `computeRangeAlignment`의 `emitAlignment` 규칙을 옮깁니다: 내부 변경 원본·수정 시작 열이 둘 다 1보다 크면 그 시작 줄에서 정렬, 원본 끝 열이 그 줄 최대 열보다 작으면 끝 줄에서 정렬, 마지막에 hunk 끝에서 강제 정렬. 같은 줄 재정렬은 건너뜁니다(`forceAlignment`가 아니면 `origLineNumberExclusive === lastOrigLineNumber || modLineNumberExclusive === lastModLineNumber`일 때 무시). 정렬 구간마다 짧은 쪽에 차이만큼 filler.
2. side-by-side에서만 적용합니다(원본 `innerHunkAlignment = renderSideBySide`). inline은 기존 `inline_zones` 그대로.
3. 내부 변경이 없으면(순수 삽입·삭제, 계산 시간 초과) 지금 동작과 같아야 합니다. 기존 `채움_띠는_짧은_쪽_hunk_뒤에_줄_수_차만큼_놓이고_맨_위_삽입은_첫_줄_위다` 검사가 그대로 통과해야 합니다.

##### 3.3.7 overview ruler

결정 4 A의 문구가 "overview ruler(기존 `DecorationKind::Overview` 재사용)"입니다. 원본 배치(오른쪽 30px 별도 열)와 결정 문구를 둘 다 만족하는 순서로 적습니다.

1. `UI/diff-view.rs` 상단 상수 `OVERVIEW_WIDTH: f32 = 15.0`, `OVERVIEW_TOTAL_WIDTH: f32 = 30.0`. `show`에서 `rect`의 오른쪽 30px를 ruler 열로 떼고 나머지를 지금처럼 나눕니다(side-by-side·inline 모두, 원본 `renderOverviewRuler`는 모드와 무관).
2. 줄 → y 변환은 편집기 내부 레이아웃이 알고 있습니다. `UI/editor-overview.rs`의 기존 `paint`(100줄 부근)가 `display.row_of_byte`, `layout.row_top/row_bottom`, `projected(rect, top, bottom, content_height, MIN_MARK_HEIGHT)`로 같은 일을 합니다. 그래서 `SurfaceExtras`에 `pub diff_overview: Option<DiffOverview>`(`pub struct DiffOverview { pub column: Rect, pub clip: Rect, pub zones: Vec<(Range<usize>, Color32)> }`, 줄 범위는 0부터)를 더하고, `editor_surface.rs`에서 기존 `crate::editor_overview::paint(..)` 호출(3856줄) 뒤에 같은 투영으로 zones를 `column`에 칠하는 새 함수(예: `crate::editor_overview::paint_zones`)를 부릅니다. 자식 Ui의 painter는 편집기 rect로 잘려 열 밖에 못 그리므로, diff 위젯이 자기 clip과 교집합으로 계산한 `clip`을 넘겨 `egui::Painter::new(ui.ctx().clone(), ui.layer_id(), clip)`로 그립니다. 이렇게 하면 장식 데이터는 기존 `Overview` 투영 규칙을 재사용합니다.
3. 원본 편집기 → 열 왼쪽 15px에 삭제 줄 범위, 수정 편집기 → 오른쪽 15px에 삽입 줄 범위. 색은 `Colors`에 `inserted_overview`·`removed_overview`로 더하고 `Appearance::new`에서 `diff.insertedBackground`/`removedBackground`의 alpha를 2배(255에서 자름)로 만듭니다.
4. viewport 사각형: 수정 편집기 `geometry.scroll`·보이는 높이·content 높이로 위치와 크기를 계산해 `scrollbar.thumb`(hover면 `scrollbar.thumbHover`)로 칠합니다. content 높이가 `EditorOutput`에 없으므로 `EditorOutput`(native-host)에 `pub content_height: f32`를 더합니다(`EditorOutput`은 `editor_surface.rs` 안에서만 생성).
5. 상호작용: 열을 `Sense::click_and_drag()`로 잡고, 누르거나 끌면 누른 y의 비율로 수정 편집기 스크롤을 정합니다(`set_view_state`, 기존 동기화가 원본 쪽을 따라감). 휠은 열 위에서도 수정 편집기 스크롤로 갑니다.
6. 열 배경: dark `Color32::from_rgba_unmultiplied_const(255, 255, 255, 3)`(0.01×255≈3), light `(0, 0, 0, 8)`(0.03×255≈8)를 `APP/git-diff.rs`에 상수로(기존 `DIAGONAL_FILL_DARK`/`LIGHT`와 같은 방식, `ThemeType`으로 고름).

##### 3.3.8 기능표에 없는 원본 차이 (구현 전 사용자 확인 권장)

조사 중 원본에서 보이거나 누를 수 있지만 기능표 git-40 비고에 없는 것을 찾았습니다. 결정 4 A 문구("보이고 누를 수 있는 것은 모두")에 들어갈 수 있으나 범위를 넓히는 일이라 구현 전에 확인합니다(ai-process 3절).

| 원본 | 근거 | native |
| --- | --- | --- |
| inline 삭제 zone margin의 전구(lightBulb, U+EA61) 버튼과 메뉴 "Copy deleted lines/line", "Copy changed line (n)", 편집 가능하면 "Revert this change" | `MONACO/.../diffEditorViewZones/inlineDiffDeletedCodeMargin.js` 전체(40줄 `Codicon.lightBulb`) | 없음. 복사는 실제 clipboard라 자동 검사는 intent 단언까지만 |
| side-by-side 두 편집기 사이 그림자(`box-shadow` `scrollbar.shadow` → TAIDE `app.shadow`) | `MONACO/.../diffEditor/style.css` 296-304줄 | 1px 경계선만 |
| diff 편집기 안 sticky scroll | monaco 기본 `stickyScroll.enabled: true`(`MONACO/editor/common/config/editorOptions.js` 1382줄). 원본 `DiffView`는 이 옵션을 넘기지 않고, 숨김 설정이 켜졌을 때만 꺼짐(`components/diffEditorEditors.js` 162-167줄) | native 위젯은 항상 끔(`UI/diff-view.rs` 755줄 `presentation.options.sticky_scroll = false`). 원본 앱 화면에서 실제로 보이는지 확인 필요(정적 읽기로는 확정 불가) |

### 3.4 git-42 diff 설정 반영 — showMoves와 숨김 세부

#### (a) ID·기능·상태

`git-42` `editorDiffHideUnchangedRegions`/`editorDiffShowMoves` 설정이 diff에 반영 — **partial**(기능표 519줄). 남은 범위: `editorDiffShowMoves`(결정 6 A), 가장자리 끌기, 전부 펼친 뒤 다시 접기 glyph, 가장자리 hover 강조, 막대의 outline breadcrumb, diff 재계산 때 펼침 상태 재매핑. QA(`g5-diff.md` 5절)는 막대 그림자와 아래 펼침 스크롤 보정 자동 검사도 남은 것으로 적었습니다.

#### (b) 원본 TS 동작

- 설정 소비: `TS/shared/lib/diff-view-settings.ts` 12-15줄 `resolveDiffViewSettingsProps`(두 설정 기본 false)를 DiffPane·CommitFileDiff·충돌 비교가 같은 값으로 씁니다. ClaudeDiff는 적용하지 않습니다(`TS/widgets/claude-diff-pane/claude-diff-pane.tsx` 58줄, 옵션 `{ automaticLayout: true, readOnly: false }`만).
- `DiffView`는 `[original, modified, languageId]`가 바뀔 때마다 새 model을 만들어 `setModel`합니다(`TS/features/git/diff-view.tsx` 61-73줄). monaco `DiffEditorWidget.setModel`은 새 `DiffEditorViewModel`을 만듭니다(`MONACO/.../diffEditor/diffEditorWidget.js` 294-303줄 `createViewModel`·`setModel`). 그래서 **내용이나 언어가 바뀌면 미변경 영역의 펼침 상태가 처음(모두 접힘)으로 돌아갑니다**. 같은 문자열로 다시 읽히면 effect가 다시 돌지 않아 유지됩니다.
- showMoves: `showMoves = experimental.showMoves && renderSideBySide`(`diffEditorOptions.js` 63줄). 즉 inline(직접 끈 경우와 900px 이하 자동 inline 모두)에서는 이동 계산·표시가 없습니다(`diffEditorViewModel.js` 212줄 `computeMoves: this._options.showMoves`). 계산 `computeMovedLines`(`MONACO/editor/common/diff/defaultLinesDiffComputer/computeMovedLines.js` 16-32줄): 단순 삭제(원본 3줄 이상, 수정 빈)와 단순 삽입(수정 3줄 이상)의 유사도(`LineRangeFragment.computeSimilarity`, 문자 히스토그램, `utils.js`) > 0.90 짝 → 나머지 변경에서 3줄 해시 일치로 이동 후보(`computeUnchangedMoves`) → 위·아래 비슷한 줄로 확장(`areLinesSimilar` 186줄: trim 같음, 또는 둘 다 300자 초과가 아니고 Myers 공통 비공백 문자 비율 > 0.6 && 긴 줄 비공백 > 10) → 가까운 이동 합치기(`joinCloseConsecutiveMoves` 217줄, 거리 합 ≤ 2) → 원본 텍스트 15자 이상 && 2자 이상 줄 2개 이상만 → 같은 diff 안 이동 제거(`removeMovesInSameDiff` 237줄). 이동마다 `refineDiff`로 내부 변경을 구해 `MovedText(m, changes)`(`defaultLinesDiffComputer.js` 151-159줄).
- 이동 표시(`MONACO/.../features/movedBlocksLinesFeature.js`, `components/diffEditorDecorations.js` 75-107줄, `style.css` 104-145줄):
  - 이동 블록 테두리 2px `diffEditor.move.border`(`#8b8b8b9c`), 활성 이동은 `diffEditor.moveActive.border`(`#FFA500`)(`registrations.contribution.js` 22-23줄, TAIDE 매핑 없음). block padding 원본 쪽 `[4,0,4,4]`(`movedCodeBlockPadding = 4`), 수정 쪽 `[4,0,4,4]`.
  - 블록 시작 줄 위에 양쪽 18px placeholder zone과 오른쪽 정렬 문구: 원본 쪽 "Code moved to line {0}-{1}"(또는 "Code moved with changes to line {0}-{1}"), 수정 쪽 "Code moved from line {0}-{1}"("with changes from"), 그 옆 "Compare" 토글 버튼(`MovedBlockOverlayWidget`, 236-277줄). TAIDE는 monaco nls를 바꾸지 않으므로 영어 그대로입니다(`g5-diff.md` 2절과 같은 처리).
  - 두 편집기 경계에 연결선: 원본 블록 세로 가운데에서 시작해 트랙 x까지 가로, 수정 블록 가운데 y까지 세로, 오른쪽 끝(화살표 앞)까지 가로. 트랙 최대 6(`LinesLayout.compute`, 204줄), 간격 10px, 좌우 padding 10px, 화살표 15×15 다각형, 화살표 뒤 높이 18px 사각형(편집기 배경색). 선 두께 2, 활성 이동은 활성 색(58-120줄). 이 열의 폭만큼 원본 편집기가 좁아집니다(`diffEditorWidget.js` 97-99줄).
  - 활성 이동: 마지막으로 focus된 편집기의 커서 줄이 들어 있는 이동(170-201줄).
  - Compare: 그 이동의 내부 diff만 장식으로 보이고 일반 장식은 숨김(`diffEditorDecorations.js` 25·30·75-90줄), 다시 누르면 해제.
- 숨김 막대 세부(`MONACO/.../features/hideUnchangedRegionsFeature.js`, `style.css` 6-102줄):
  - 막대가 하나도 없게 되는 경우: 영역이 하나이고 수정 문서 1줄부터 전체 줄 수만큼이면 영역 없음(89-95줄). 변경이 없는 diff는 아무것도 숨기지 않습니다.
  - 위 가장자리 띠: 누르고 2px 넘게 끌면 `visibleLineCountTop = clamp(시작값 + round(dy / 줄 높이), 0, lineCount − bottom)`, 끌지 않고 떼면 20줄 펼침(318-348줄). 아래 띠: `bottom = clamp(시작값 − round(dy / 줄 높이), 0, lineCount − top)`이고 영역 뒤 첫 줄의 화면 위치가 유지되도록 스크롤을 보정(349-389줄). 끄는 동안 커서는 `canMoveTop/canMoveBottom` 규칙의 n/s/ns-resize.
  - hover 강조: 끌지 않는 동안 띠 위에 마우스가 있거나 끄는 중이면 띠 배경 `focusBorder`(TAIDE `app.focusBorder`), 띠 높이 4px(위 테두리 4px·아래 2px 투명, `style.css` 17-32줄).
  - 그림자: 막대 가운데(24px)에 위·아래 안쪽 그림자 `diffEditor.unchangedRegionShadow`(dark `#000000`, light `#737373BF`, `registrations.contribution.js` 24줄, `style.css` 84줄).
  - tooltip(브라우저 `title`): 위 띠 "Click or drag to show more above"(278줄), 아래 띠 "Click or drag to show more below"(283줄), unfold 아이콘 "Show Unchanged Region"(280줄), 숨긴 줄 수 문구 "Double click to unfold"(396줄).
  - 다시 접기 glyph: 영역이 전부 펼쳐지고(top + bottom = lineCount) 끄는 중이 아니면 막대를 숨기고(`shouldHideControls`, `diffEditorViewModel.js` 390줄) 영역 첫 줄 glyph margin에 `codicon-fold`(U+EAF5, `codiconsLibrary.js` 235줄), hover 문구 "Fold Unchanged Region"; 그 glyph에서 오른쪽 버튼이 아닌 mouseup이면 `collapseAll`(top = bottom = 0)(hideUnchangedRegionsFeature.js 146-232줄).
  - breadcrumb: 숨긴 수정 범위 안에서 시작하고 밖에서 끝나는 symbol(큰 것부터)을 "{n} hidden lines" 뒤 `  |  `와 아이콘·이름·chevron으로 잇고, 누르면 그 줄이 보이도록 펼침(391-424줄). symbol 출처는 `diffEditorBreadcrumbs` contribution(`MONACO/editor/contrib/diffEditorBreadcrumbs/browser/contribution.js` 47-75줄, outline model = 그 언어에 등록된 document symbol provider)입니다. TAIDE는 `import * as monaco from 'monaco-editor'`(`TS/shared/lib/monaco/setup.ts` 1줄, `editor.main.js`가 이 contribution을 포함)라 활성입니다. TAIDE의 provider는 LSP adapter(`TS/shared/lib/lsp/adapters/document-symbol.ts` 74-80줄, 모델 URI로 요청)와 monaco 내장 언어 worker입니다.
  - 커서 이동 펼침: 원본·수정 편집기의 커서가 숨긴 줄로 가면 그 줄이 보이게 가까운 쪽에서 펼칩니다(65-88줄 `ensureOriginalLineIsVisible`/`ensureModifiedLineIsVisible`, `RevealPreference.FromCloserSide`).

#### (c) native 현재 자산과 빠진 것

- 완료: `unchanged_regions`(monaco `fromDiffs`와 같은 식, `diffEditorViewModel.js` 347-375줄과 대조), 양쪽 숨긴 줄·24px 막대·영역 배경, 위·아래 띠 누름 20줄, 아이콘 누름·문구 더블클릭 전부 펼침, 아래 펼침 스크롤 보정, 설정 즉시 반영(`domains/git.rs` `show_diff`가 매 프레임 `settings.editor_diff_hide_unchanged_regions` 읽음). 문서 첫 줄은 native 표시 맵이 숨기지 못해(`FIRST_HIDEABLE_LINE`, `UI/diff-view.rs` 46줄) 문서 처음 영역은 1줄 덜 숨깁니다(QA에 기록된 native 차이).
- **확인된 결함(기능표 비고에 없음)**: `UI/diff-view.rs` `show`가 `unchanged_regions(..)` 결과를 그대로 씁니다(791-807줄). 변경이 없는 diff(같은 내용의 두 쪽, hunk 없음)에서 `unchanged_regions`는 `runs`가 `(1, 1, N)` 하나라 `at_start && at_end`가 참이고 잘라내기 0이므로 문서 전체를 영역 하나로 돌려줍니다(178-199줄). 그러면 `hidden(Reveal::default())`가 첫 줄만 남기고 나머지를 막대로 숨깁니다(111-120줄). 원본은 이 경우 영역을 없앱니다(b 첫 항목).
- **펼침 상태 규칙 차이**: native는 영역이 같은 값(시작·줄 수)으로 다시 계산되면 펼침을 유지하고 다르면 버립니다(`state.reveals.iter().find(|(known, _)| known == region)`, 808-817줄). 원본은 내용·언어가 바뀌면 영역이 같아도 모두 초기화합니다. 기능표의 "재계산 때 펼침 상태 재매핑"은 monaco 내부 기능(decoration 추적·`setVisibleRanges`, `diffEditorViewModel.js` 95-160줄) 기준이지만, TAIDE 원본은 model을 새로 만들어 그 기능을 쓰지 않습니다.
- showMoves: `settings.editor_diff_show_moves`의 소비자가 없습니다(`grep -rn "editor_diff_show_moves" native` 결과는 `UI/settings-code-controls.rs` 98·130·161·265줄뿐, 모델 `CR/taide-model/src/settings.rs` 184줄).
- 숨김 세부: 띠는 `Sense::click()`이라 끌기가 없고(484-497줄), hover 배경·그림자·tooltip·fold glyph·breadcrumb·커서 이동 펼침이 없습니다(`unchanged_bar` 437-505줄).

#### (d) 구현 방법

##### 3.4.1 전체 파일 영역 결함 수정

1. `UI/diff-view.rs`에 순수 함수 `pub fn visible_regions(regions: Vec<UnchangedRegion>, modified_lines: u32) -> Vec<UnchangedRegion>`를 둡니다. 영역이 하나이고 `modified_start == 1 && line_count == modified_lines`이면 빈 목록, 아니면 그대로(원본 89-95줄과 같은 조건, `modified_lines`는 수정 문서 `rope.len_lines()`).
2. `show`에서 `regions`를 만든 직후 이 함수를 거칩니다. `[U]` 검사에서 직접 부를 수 있게 공개합니다.

##### 3.4.2 펼침 상태: 내용이 바뀌면 초기화

1. `State`(`UI/diff-view.rs` 225-240줄)에 `revisions: Option<(u64, u64)>`를 더합니다. `show`에서 두 문서 snapshot의 `revision`을 읽어 저장값과 다르면 `reveals`를 비웁니다. `replace_virtual`은 내용·언어가 같으면 revision을 올리지 않으므로(`ED/store.rs` 685-687줄) "같은 문자열로 다시 읽음"은 유지, "다른 내용"은 초기화로 원본과 같아집니다.
2. 편집 가능한 ClaudeDiff는 `hide_unchanged: false`라 영향이 없습니다.
3. 기능표 비고의 "재매핑"은 이 규칙으로 대체하고 원본 근거(b 둘째 항목)를 QA에 적습니다.

##### 3.4.3 showMoves

1. 계산: 새 파일 `ED/diff-moves.rs`(`pub mod diff_moves`). 원본 `computeMovedLines.js` 전체(249줄)와 그 보조(`utils.js`의 `LineRangeFragment.computeSimilarity`, `LineRangeSet`, `SetMap`, `MonotonousArray`·`findLastMonotonous`, `areLinesSimilar`의 Myers 문자 diff)를 옮깁니다. 입력 `changes`는 libgit2 hunk에서 만든 `(LineRange, LineRange)` 목록(결정 7 A), 해시는 원본처럼 `line.trim()`의 완전 해시(같은 문자열 = 같은 번호, `defaultLinesDiffComputer.js` 39-49줄). 출력:

```rust
pub struct MovedText {
    pub original: LineRange,
    pub modified: LineRange,
    pub changes: Vec<RangeMapping>,
}
```

   `changes`는 3.3.1 `refine`을 이동 범위 전체에 적용한 결과입니다(비면 "Code moved", 있으면 "Code moved with changes").
2. oracle: `docs/utils/<날짜>-monaco-moved-lines-oracle.js`가 `computeMovedLines(changes, originalLines, modifiedLines, hashedOriginal, hashedModified, InfiniteTimeout.instance)`를 실행해 `native/taide-native-editor/tests/fixtures/diff-moves-reference.tsv`를 만듭니다. 사례: 함수 하나 이동(변경 없음), 이동 + 한 줄 수정, 3줄 미만 이동(제외), 15자 미만(제외), 같은 hunk 안 이동(제외), 가까운 이동 합치기, 유사도 0.90 경계.
3. 전달: `git_cache::Diff`에 `pub moves: Vec<MovedText>`. `with_hunks`에서 항상 계산합니다(설정을 켜고 끄는 즉시 반영해야 하고 캐시 키에 설정이 없음, 4절 함정 12). ClaudeDiff는 계산하지 않습니다.
4. 위젯 입력: `diff_view::Request`에 `pub moves: &'a [MovedText]`, `pub show_moves: bool`. 위젯 안에서 `show_moves && is_side_by_side`일 때만 그립니다. 앱은 `Surface`(1062-1071줄)에 `show_moves: bool`을 더해 diff 탭(`show_diff`)·커밋 diff·충돌 비교(`APP/git-editor.rs` `show_conflict_compare`)에서 `settings.editor_diff_show_moves`를, ClaudeDiff·파일 이력(inline 고정)에서 `false`를 넘깁니다.
5. 그리기(`UI/diff-view.rs`, side-by-side 분기):
   - 이동 블록 시작 줄 앞에 양쪽 18px `ExtraZone`(`MOVED_BLOCK_ZONE_HEIGHT = 18.0`)을 filler·막대 zone과 같은 목록에 더하고, zone 사각형 오른쪽에 문구와 "Compare" 버튼을 그립니다. 문구는 원본 영어 형식(`format!("Code moved to line {}-{}", start, end_exclusive - 1)` 등 네 가지).
   - 블록 테두리: 편집기를 그린 뒤 블록 줄들의 행 사각형 합에 padding(위 4, 오른쪽 0, 아래 4, 왼쪽 4)을 더해 2px 선(`MOVE_BORDER`, 활성이면 `MOVE_ACTIVE_BORDER`).
   - 연결선 열: 원본 편집기 폭을 `lineAreaWidth = (트랙 수 − 1) × 10 + 20`만큼 줄이고 그 자리에 선을 그립니다. 트랙 배정은 원본 `LinesLayout.compute`(최대 6, 넘치면 가장 덜 겹치는 트랙). `painter.line_segment`로 세 구간, `painter.add(Shape::convex_polygon(..))`로 화살표, 그 뒤 사각형은 편집기 배경.
   - 활성 이동: 각 편집기 view의 `selection.head` 줄과 마지막으로 focus를 가진 편집기(`EditorOutput.response.has_focus()`를 프레임마다 위젯 임시 상태에 기억)로 정합니다.
   - Compare: 위젯 임시 상태 `compare: Option<usize>`(이동 index). 켜지면 그 이동의 `changes`만 문자 장식으로 쓰고 일반 줄·문자 장식은 그리지 않습니다. 다시 누르면 해제.
6. 색: `MOVE_BORDER = Color32::from_rgba_unmultiplied_const(0x8b, 0x8b, 0x8b, 0x9c)`, `MOVE_ACTIVE_BORDER = Color32::from_rgb(0xff, 0xa5, 0x00)`(원본 기본값, TAIDE 테마 매핑 없음).

##### 3.4.4 숨김 막대 세부

1. 띠를 `Sense::click_and_drag()`로 바꾸고 위젯 임시 상태에 `dragging: Option<(region_index, Edge, start_y, start_value, moved)>`를 둡니다. `drag_started`에서 기록, `dragged`이면 `|dy| > DRAG_THRESHOLD(2.0)`부터 원본 식으로 top/bottom을 바꾸고 아래 끌기는 영역 뒤 첫 줄의 y 변화만큼 양쪽 스크롤을 보정, `drag_stopped`에서 한 번도 2px를 넘지 않았으면 지금의 20줄 펼침.
2. hover 강조: 끄는 중이 아닐 때 띠 `hovered()`이거나 끄는 중이면, 띠 안 4px 띠를 `Colors.focus_border`(`app.focusBorder`)로 칠합니다. 위치는 원본 `transform`(위 띠 `translate(0, 4px)`, 아래 띠 `translate(0, -6px)`, `style.css` 54-60줄)에 맞춰 계산합니다.
3. 그림자: 막대 위·아래 안쪽에 `UNCHANGED_REGION_SHADOW_DARK = #000000`/`_LIGHT = #737373BF` 색의 얇은 그라데이션(원본 `inset 0 ±5px 5px -7px`)을 `Shape::mesh` 또는 반투명 사각형 두 줄로 근사합니다. 근사 방식은 `[U]` 검사로 색 존재만 단언하고 QA에 적습니다.
4. tooltip: diff 위젯은 UI crate라 앱 tooltip 정책(`APP/domains/git.rs`의 `tooltips`/`tooltip_appearance`, 예 `git_scm::View.tooltips`)을 직접 못 씁니다. `Output`에 hover 대상(문구 종류와 사각형)을 돌려주고 앱이 그 tooltip 경로로 그립니다. 문구는 원본 영어 네 가지.
5. 다시 접기 glyph: 영역이 전부 펼쳐지고 끄는 중이 아니면 막대 zone을 넣지 않고(지금은 `hidden.count > 0`인 영역만 막대가 있음 — 같은 결과, 818-826줄), 영역 첫 줄 lane 칸에 `'\u{eaf5}'`(fold, 16px)를 그리고 pointer 커서. 오른쪽 버튼이 아닌 누름이면 그 영역 reveal을 `Reveal::default()`로. stage 막대는 변경 줄에만 있어 미변경 영역 첫 줄과 겹치지 않습니다. 원본은 glyph margin 칸이지만 native diff 편집기는 glyph margin이 따로 없어 lane 칸을 씁니다. 이 차이를 QA에 적습니다.
6. 커서 이동 펼침: 그린 뒤 각 view의 selection head 줄이 숨긴 줄 범위에 있으면 원본 `ensure..IsVisible(FromCloserSide)`처럼 위·아래 중 가까운 쪽 reveal을 늘립니다. 키보드로 읽기 전용 diff를 탐색할 때 필요합니다. 기능표 비고에 없는 항목이므로 3.3.8과 같이 범위 확인 뒤 진행합니다.
7. breadcrumb(마지막, 확인 필요): 원본 symbol 출처가 "그 언어의 document symbol provider"라 native에서 같은 결과를 내려면 가상 문서용 symbol 공급이 필요합니다. native 순수 symbol 함수는 JSON(`ED/json-symbols.rs`)·HTML(`ED/html-symbols.rs`)·`ED/document-symbols.rs`가 있고, 가상 문서는 LSP·내장 서비스 대상에서 빠집니다(`g5-diff.md` 6.1, 4절 함정 7). TS/JS는 내장 서비스가 없습니다(editor-57의 임베디드 JS 엔진은 허락 보류 항목). 구현 전에 원본 앱에서 TS·JSON·Markdown diff의 막대에 breadcrumb가 실제로 보이는지 확인하고(LSP adapter가 inmemory 모델 URI에 결과를 주는지 정적 읽기로 불명), 그 결과로 범위를 정합니다.

#### (e) 자동 검사

- `[U]` `native/taide-native-ui/tests/diff-view.rs`:
  1. `변경이_없는_diff는_숨김_설정에도_아무것도_숨기지_않는다`: `Fixture::with(WIDE, 같은 40줄, 같은 40줄, vec![])`(기존 `long_text(None)` helper, 446줄), `hide_unchanged = true` → `painted_texts`에 "hidden lines"가 없고 `line 05` 있음. 수정 전 실패를 먼저 확인합니다.
  2. `내용이_바뀌면_펼침_상태가_처음으로_돌아간다`: 막대 아이콘으로 전부 펼친 뒤 수정 문서를 `store.replace_virtual`로 바꾸고(같은 영역이 나오게 변경 줄 안에서 한 글자만 변경) 다음 프레임에 막대가 다시 있음. 같은 문자열 `replace_virtual`이면 펼침 유지.
  3. 끌기: 위 띠를 3줄 높이만큼 끌기(`Event::PointerButton` 누름 → `PointerMoved` → 뗌) → 숨긴 줄 수가 3 줄어듦, 2px 이하 이동 뒤 뗌 → 20줄.
  4. hover: 띠 위로 포인터 → `filled_tops(output, COLORS.focus_border)`가 띠 위치.
  5. 다시 접기: 전부 펼침 → fold 글자(U+EAF5)가 영역 첫 줄 lane 칸에 그려짐 → 누름 → 막대 복귀.
  6. 이동 표시: 6줄 함수가 아래로 옮겨진 두 텍스트와 그 hunk, `moves`(oracle과 같은 함수로 계산), `show_moves = true`, `WIDE` → "Code moved to line" 문구, 이동 테두리 색 선, 연결선(`Shape::LineSegment`·`Shape::Path` 색 `MOVE_BORDER`); `NARROW` → 없음; Compare 누름 → 일반 줄 배경 사각형이 사라지고 이동 내부 문자 배경만.
- `[C]` `native/taide-native-editor/tests/diff-moves.rs`(oracle 비교).
- `[A]` `APP/application-git-diff-tests.rs`: `실제_app_이동_표시_설정은_열린_diff_탭에_바로_반영되고_좁으면_보이지_않는다` — 함수 이동을 작업 트리에 만들고 diff 탭 → `Command::PatchSettings(SettingsPatch { editor_diff_show_moves: Some(true), .. })` → 이동 문구, `Some(false)` → 없음. 기존 `실제_app_미변경_영역_숨김_설정은_열린_diff_탭에_바로_반영되고_끄면_모두_보인다` 검사와 같은 구조(`g5-diff.md` 4절).

```
cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test diff-view --locked --offline --target-dir <TARGET>
cargo test --quiet --manifest-path native/taide-native-editor/Cargo.toml --test diff-moves --locked --offline --target-dir <TARGET>
cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <TARGET> -- domains::git::diff_tests --test-threads=2
```

#### (f) 완료 판정

- 위 검사 전부 통과, 기존 `diff-view` 검사(작성 시점 9개 정의: 233·282·327·357·383·418·460·564줄 등)·`diff_tests` 회귀 통과, Wasm check·fmt.
- 기능표 git-42 complete(남은 native 차이 "문서 첫 줄은 숨기지 못함"은 비고에 유지), settings-28의 "diff 2종" 근거 갱신. breadcrumb를 범위에서 빼기로 했으면 그 결정과 근거를 QA·`docs/acknowledge`에 적고, 빼지 않았다면 구현 근거가 있어야 complete입니다.

#### (g) 막는 요소

- breadcrumb: 원본 앱 실행으로 symbol 출처를 확인해야 합니다(GUI 실행은 작업자 금지 항목이라 사용자 확인 또는 허락 필요). 확인 전까지 이 하위 항목만 보류합니다.
- 커서 이동 펼침·tooltip은 기능표 비고 밖이라 범위 확인 뒤 진행합니다.

### 3.5 git-50 ClaudeDiffPane — 수정 쪽 revert

#### (a) ID·기능·상태

`git-50` ClaudeDiffPane 렌더(원본 = 디스크 파일, 수정 = 요청 내용(편집 가능), 언어 동기화) — **partial**(기능표 527줄). 남은 것: 수정 쪽 revert 화살표(결정 4 A)와 git-40 공통 장식.

#### (b) 원본 TS 동작

- `TS/widgets/claude-diff-pane/claude-diff-pane.tsx` 55-85줄: `monaco.editor.createDiffEditor(container, { automaticLayout: true, readOnly: false })`, 원본 model은 빈 문자열·`plaintext`로 만들고 디스크 내용이 오면 `setValue`, 언어는 `file.languageId ?? 'plaintext'`로 양쪽 동기화. 수정 쪽이 편집 가능하므로 monaco 문맥 키 `diffEditorModifiedWritable`이 참이고, 원본 쪽은 `originalEditable: false`(기본값)라 읽기 전용입니다.
- monaco 0.56은 `renderGutterMenu: true`가 기본이라 예전 margin revert 화살표(`shouldRenderOldRevertArrows`)는 그려지지 않습니다(`MONACO/.../diffEditor/diffEditorOptions.js` 42-57줄: `shouldRenderGutterMenu`가 참이면 false). 실제로 보이는 것은 **diff gutter 도구 막대**입니다(`MONACO/.../features/gutterFeature.js`).
  - 도구 막대 항목은 `MenuId.DiffEditorHunkToolbar`에 등록된 "Revert Block"(`diffEditor.contribution.js` 40-65줄): side-by-side는 아이콘 `arrowRight`(U+EA9C), inline은 `discard`(U+EAE2). 조건 `diffEditorModifiedWritable && (inline 여부)`. 선택 영역이 있으면 `DiffEditorSelectionToolbar`의 "Revert Selection".
  - 항목이 있을 때만 폭 35px의 gutter 열이 생깁니다(`gutterFeature.js` 38줄 `const width = 35`, 52-54줄 `_hasActions`·`width`). side-by-side: 열은 분할선 바로 왼쪽(`gutterLeft = sashLeft − gutterWidth`, 원본 편집기 폭 = sashLeft − gutterWidth − 이동선 폭), inline: 맨 왼쪽(`diffEditorWidget.js` 93-115줄). git diff(`readOnly: true`)는 항목이 없어 열이 없습니다.
  - 항목: 각 줄 mapping(순수 삭제 포함)마다 하나, 범위 = 수정 쪽 줄 범위(100-118줄). 항목 크기 34×20(`DiffToolBar` 160-164줄), 항목 범위 세로 가운데 `menu.separatorBackground`(TAIDE `menu.separator`) 2px 선, 버튼은 항목 범위 가운데에 두되 화면과 항목 범위 안으로 clamp(`DiffToolBar.layout` 215-232줄). 원본 1줄부터 시작하고 항목 범위가 30px 미만이면 작은 모드(최대 1개).
  - 보임: 수정 편집기 커서가 들어 있는 hunk 항목은 항상 보임(`showAlways`, `_currentDiff` 56-67줄), 나머지는 gutter 열에 마우스가 있을 때만 보임(opacity 0.1초 등장, 0.7초 사라짐, `style.css` 339-356줄). tooltip "Revert Block"(오른쪽, 즉시).
  - 누름: `RevertHunkOrSelection.runViaToolbarContext` → `revertRangeMappings(mapping.withInnerChangesFromLineRanges().innerChanges)`(`commands.js` 150-175줄, `diffEditorWidget.js` 373-385줄) → 수정 쪽 hunk 줄 범위 전체를 원본 쪽 줄 범위의 텍스트로 바꾸는 편집(앞뒤 `pushUndoStop`). diff가 최신이 아니면(`isDiffUpToDate` 거짓) 무동작.
  - 선택 영역 revert: 수정 쪽 선택이 비지 않고 내부 변경과 겹치면 그 선택 범위에 항목 하나("Revert Selection"), 겹치는 내부 변경만 되돌림(`_selectedDiffs` 68-99줄, `gutterItems` 100-118줄).
  - `editorGutter.itemBackground`·`editorGutter.itemGlyphForeground`는 monaco 0.56 standalone에 등록되어 있지 않습니다(`grep -rln "editorGutter.itemBackground" MONACO`의 결과가 `diffEditor/style.css`뿐). 그래서 CSS 변수가 정의되지 않아 버튼 배경은 칠해지지 않고 아이콘 색은 상속값입니다. 상속 체인의 최종 색은 원본 앱 화면으로 확인해야 합니다(g 참고).
- 언어·원본 갱신·Accept/Reject는 이미 완료(git-50·51 비고).

#### (c) native 현재 자산과 빠진 것

- 완료: 32px 막대(`git_claude_diff::bar`), `show_claude_diff_tab`(2821-2920줄)이 `show_surface`(`SideText::Read` 원본, `SideText::Editable` 수정, `side_by_side: true`, `stage_lanes: false`, `hide_unchanged: false`), 200ms 재계산(`git_claude_diff::State::shown/due/finish`, `Command::Hunks`).
- 빠진 것: diff 위젯에 gutter 열·항목·버튼·되돌리기 출력이 없음, 앱에 되돌리기 편집 경로 없음, git-40의 문자 강조·±·overview ruler(ClaudeDiff에도 적용).

#### (d) 구현 방법

1. `UI/diff-view.rs`:
   - `Request`에 `pub revert_blocks: bool`(ClaudeDiff만 참)과 `pub is_diff_current: bool`(앱이 hunk가 현재 문서 revision 기준인지 알려줌).
   - 상수 `REVERT_GUTTER_WIDTH: f32 = 35.0`, `REVERT_ITEM_WIDTH: f32 = 34.0`, `REVERT_ITEM_HEIGHT: f32 = 20.0`, `REVERT_SEPARATOR_WIDTH: f32 = 2.0`, `REVERT_SMALL_RANGE: f32 = 30.0`.
   - 레이아웃: `revert_blocks`면 side-by-side는 `original_rect` 오른쪽 끝을 35px 줄이고 그 자리(분할선 왼쪽)를 gutter 열로, inline은 `rect` 왼쪽 35px를 열로 떼고 나머지를 원본 줄 번호 열(3.3.4)과 수정 편집기에 줍니다. 분할선 끌기 계산(`state.split`)은 열을 포함한 위치 기준으로 원본과 같게 둡니다.
   - 항목: hunk마다 수정 쪽 줄 범위의 화면 y 범위를 `output.modified.geometry`(행 목록)에서 구합니다(보이는 행만 있으므로 화면 밖 항목은 그리지 않음, 순수 삭제는 경계 y 한 점). 범위 가운데 2px 세로선(`Colors.gutter_separator` = `menu.separator`), 버튼은 원본 `layout` clamp 식대로.
   - 보임: 수정 편집기 `selection.head`가 그 hunk 줄 범위에 있으면 항상, 아니면 포인터가 열 위에 있을 때만. 보이지 않는 프레임에는 버튼을 만들지 않습니다(accesskit 노드도 없음). 애니메이션은 즉시 표시로 두고 QA에 차이를 적습니다(egui `ctx.animate_value_with_time`으로 원본 시간을 재현할 수도 있음).
   - 버튼: `ui.interact(rect, id.with(("taide-diff-revert", index)), Sense::click())`, `widget_info`로 이름 "Revert Block"(원본 영어), 아이콘 side-by-side `'\u{ea9c}'`/inline `'\u{eae2}'` 16px codicon, 색은 (g)의 확인값.
   - `Output`에 `pub revert: Option<Revert>`(`pub struct Revert { pub original: Range<u32>, pub modified: Range<u32> }`, 1부터 끝 제외 줄 범위). `is_diff_current`가 거짓이면 누름을 무시합니다.
2. `APP/domains/git.rs`와 `UI/intents/git.rs`:
   - `UI/intents/git.rs` `Intent`에 `RevertClaudeDiff { tab: TabId, revisions: (u64, u64), original: Range<u32>, modified: Range<u32> }`. UI crate는 앱 타입 `git_claude_diff::Revisions`를 모르므로 기존 `ClaudeDiffShown { .., revisions: (u64, u64) }`(211-217줄)와 같은 튜플을 씁니다.
   - `git_claude_diff::State`에 `is_current(&TabId, Revisions) -> bool`(그 탭 `Lines.computed == Some(revisions)`이고 `wanted`·`in_flight`가 없음).
   - `show_claude_diff_tab`에서 `Surface`에 `revert_blocks: true`, `is_diff_current: git.claude.is_current(&tab.id, current)`(`current`는 `claude_revisions(self.store, &tab.id)`). `output.revert`가 있으면 intent.
   - `apply_git_intent`의 새 분기: 탭의 수정·원본 가상 문서를 `DocumentKey::Virtual(VirtualKey { owner: tab, side })`로 찾고, revision이 intent의 값과 같은지 확인(다르면 무동작 — 4절 함정 8), 원본 줄 범위 텍스트를 잘라 수정 문서 줄 범위를 바꾸는 `Transaction`을 `self.store.apply_separate(document, Transaction { revision, edits: vec![Edit { bytes, text }], group: UndoGroup(revision), origin: Some(view), selection_after: None })`로 적용합니다(같은 패턴 `APP/git-editor.rs` `apply_conflict_resolution` 407-419줄). 끝 줄이 문서 끝을 넘으면(마지막 줄에 줄바꿈 없음) 바이트를 문서 끝으로 자르고 원본 텍스트도 같은 규칙으로 자릅니다(원본 `toExclusiveRange` 동작은 `[A]` 사례로 고정).
   - 편집 뒤 문서 revision이 바뀌므로 기존 `ClaudeDiffShown` → 200ms 재계산이 그대로 hunk를 갱신합니다. 가상 문서는 `editor_menu_changes`(파일 문서 dirty·mirror 경로)에 넣지 않습니다.
3. 선택 영역 revert(3.3.1 이후): `git_claude_diff::State`가 inner를 가지면 수정 view 선택이 비지 않고 inner 수정 범위와 겹칠 때 항목 하나("Revert Selection")만 선택 범위에 두고, 누르면 겹치는 inner마다 수정 범위 → 원본 범위 텍스트 편집을 한 `Transaction`(여러 `Edit`, 뒤에서부터)으로 적용합니다.
4. git-40 공통 장식은 3.3에서 diff 위젯 공통으로 들어가므로 ClaudeDiff에도 자동 적용됩니다(`ignore_trim_whitespace: true`에 맞춰 refine은 `consider_whitespace_changes: false`).

#### (e) 자동 검사

- `[U]` `diff-view.rs`: `revert_blocks: true`, `WIDE` → 분할선 왼쪽 35px 열, 포인터를 열 위로 → hunk 수만큼 "Revert Block" 노드(`fixture.context.enable_accesskit()` 필요), 열 밖 → 커서 hunk 하나만, 버튼 누름 → `Output.revert == Some(Revert { original, modified })`, `is_diff_current: false`면 `None`. `NARROW` → 열이 왼쪽, 아이콘 글자 U+EAE2. 이 하네스의 `Fixture::with`는 문서를 `read_only = true`로 열지만(`open_virtual(.., true)`) 위젯 단위 검사에는 영향이 없습니다.
- `[A]` `application-git-claude-history-tests.rs` `실제_app_claude_diff_revert_block은_수정_쪽_hunk를_원본으로_되돌리고_다시_계산한다`: `DISK = "alpha\nbeta\ngamma\n"`, `PROPOSED = "alpha  \nbeta\nGAMMA\n"`(기존 상수 28-29줄) → hunk `[Hunk { old_start: 3, old_lines: 1, new_start: 3, new_lines: 1 }]` 대기(기존 검사와 같은 단언) → gutter 열 위로 포인터 → "Revert Block" 누름 → 수정 가상 문서 = `"alpha  \nbeta\ngamma\n"`, 200ms 뒤 `claude.hunks(&tab.id)` 비어 있음, Accept → IDE 응답 Saved·파일 = 되돌린 내용. 수정 쪽 문서에 undo 한 번 → PROPOSED 복귀(undo 그룹 1개). undo는 `application.store.undo(document)`(G8 충돌 검사 `application-git-editor-tests.rs` 609줄과 같은 방식)로 부르거나, 키 입력이면 `crate::application::inlay_hints_tests::key(egui::Key::Z, egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND)`(`application-ai-completion-tests.rs` 812·894줄과 같은 수식키)를 씁니다.

```
cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <TARGET> -- domains::git::claude_history_tests git_claude_diff --test-threads=2
```

#### (f) 완료 판정

- 위 검사와 기존 `claude_history_tests` 4개 통과, 3.3의 공통 장식 검사 통과.
- 기능표 git-50 complete, editor-76·shell-89 claudeDiff 부분 근거 갱신. 애니메이션·색 확인 결과를 QA에 적습니다.

#### (g) 막는 요소

- 아이콘 색: 원본 CSS가 색을 정하지 않아 상속 체인의 최종 값을 정적 읽기로 확정하지 못했습니다. 후보는 `.monaco-editor { color: var(--vscode-editor-foreground) }` 같은 조상 글자색입니다. 원본 앱 화면 확인(사용자) 또는 TS 앱의 개발자 도구 확인이 필요하며, 그 전에는 `editor.foreground`로 두고 QA에 "확인 필요"로 적습니다.
- 선택 영역 revert는 3.3.1 이후에만 가능합니다.

### 3.6 git-52 탭 닫힘·다른 창 이동의 암묵적 reject 구분

#### (a) ID·기능·상태

`git-52` 탭 닫힘/다른 창 이동 시 암묵적 reject 구분(모든 창 트리 검사) — **partial**(기능표 529줄). 남은 것: 보조 창 이동 뒤 응답 없음·보조 창 Accept 실제 App 검사.

#### (b) 원본 TS 동작

- `TS/widgets/claude-diff-pane/claude-diff-pane.tsx` 26-36줄 `isTabStillOpenInLayout(layout, tabId)`(`collectAllPaneTabs`로 메인·보조 창 트리 전부), 87-108줄 언마운트 effect: 탭이 어느 창에든 남아 있으면(탭 전환·다른 창 이동) 무동작, 없고 미해결이면(`takePendingClaudeDiffIfUnresolved`) `rejected`. 사용자가 탭을 닫으면 백엔드 닫기 observer가 먼저 `TabClosed`를 보냅니다.
- 다른 창으로 옮기면 pane이 메인에서 언마운트되고 보조 창에서 다시 마운트됩니다. 수정 model은 `getPendingClaudeDiff(requestId)?.newContents`로 새로 만들어지므로(44줄 `requestedContentsRef`) 옮기기 전 편집은 사라집니다.
- `TS/app/providers/ide-sync-provider.tsx` 131-133줄: IDE `close_tab` 요청(`ideCloseTabRequested`, `requestId` 있음)이면 대기 목록에서 지움.

#### (c) native 현재 자산과 빠진 것

- 완료(g7 문서 3.3절): 표면 해제 때 응답 없음, 사용자 탭 닫기는 `APP/tabs.rs` 136-139줄(`TabKind::ClaudeDiff { request_id, .. }` → `services.ide.take_pending_diff(request_id)` → `IdeDiffOutcome::TabClosed`), IDE `close_tab`은 대기 목록 제거, `tick_claude_diffs`(2297-2353줄)가 전 창 트리(`taide_layout::service::all_roots` + `collect_claude_diff_tab_ids`, `CR/taide-layout/src/service.rs` 39·220줄)에 탭이 없을 때만 그려진 요청을 지움(`git_claude_diff::State::retain`, 200-208줄). 탭 창 이동(`APP/domains/shell.rs` `Command::MoveTabToWindow` 362-397줄 → `layout_actions::layout_move_tab_to_window`)은 IDE 응답을 보내지 않습니다. 탭 내용 분기는 창과 무관(`domains/editor.rs` `tab_content`).
- 검사: `[C]` `git-claude-diff-tests.rs` `그려진_요청이_어느_창에도_없으면_대기_목록에서_지운다`(다른 창 트리 유지), `[A]` 탭 닫기·IDE close_tab(`실제_app_claude_diff는_자동_열기_끄기_reject_탭_닫기_ide_close_tab을_원본대로_처리한다`, 372줄). 보조 창 이동·보조 창 Accept·탭 전환 복귀의 `[A]`가 없습니다(사양 git-52 검사 목록의 "다른 탭 전환 후 복귀 시 유지"도 App 근거 없음).
- 탭 메뉴의 창 이동 항목은 탭 종류와 무관하게 모든 탭에 있습니다(`UI/shell.rs` 2366-2390줄 `tab.moveToNewWindow`·`tab.moveToMainWindow`·`tab.moveToWindowNumbered`).

#### (d) 구현 방법

코드 변경 없이 검사를 먼저 씁니다. 실패하면 실패한 단언의 원인을 고칩니다.

1. `APP/application-auxiliary-window-tests.rs`의 `choose_tab_menu`(490줄)를 `pub(super)`로 올립니다(같은 파일 helper를 올린 G1 선례). 이 함수는 `project_rail_tests::step`으로 프레임을 돌리고 탭 우클릭 뒤 메뉴 항목을 누르며, 내부에서 비공개 `tab_bar()`·`labels()`를 쓰지만 같은 파일 안이라 그대로 둡니다.
2. `APP/application-git-claude-history-tests.rs`에 검사를 추가합니다((e)).
3. 예상 위험: 이동 프레임에 탭이 어느 창에도 그려지지 않으면 `release_hidden`이 가상 문서를 해제하고 보조 창에서 대기 목록 내용으로 다시 엽니다. 원본도 편집을 잃으므로 같은 결과입니다. 이동 직후 `retain`의 `open` 집합에 보조 창 트리가 들어가야 대기 목록이 남습니다(layout 변경은 `services.state.layouts` 쓰기 한 번이라 중간 상태가 없어야 정상).

#### (e) 자동 검사

`[A]` `실제_app_claude_diff_탭을_보조_창으로_옮기면_응답_없이_유지되고_그_창에서_accept한다`:

1. `Workspace::new("claude-move", &["alpha"])`, `a.txt = DISK`, `launch_git(&mut workspace)`, 바로 이어 `let viewports = crate::application::auxiliary_window_tests::install_viewports(&context);`(첫 프레임 전에 설치 — `launch_git`이 `settle`을 부르므로 `install_viewports`를 쓰려면 `launch`·`install_viewports`·`settle` 순으로 이 검사 안에서 직접 부릅니다).
2. `let pending = open_diff(application, &path);` → `until(.. claude_tab(..).is_some() && has_text(output, "GAMMA"))`.
3. 수정 쪽에 글자 하나 입력(`click` + `type_text`) → 이동 뒤 사라지는지 확인용.
4. `choose_tab_menu(application, &context, &viewports, None, TITLE, &text(application, "tab.moveToNewWindow"))` → `step_until`으로 보조 창 slot 1에 그 탭이 있고(`services.state.layouts.read()[&project].auxiliary_windows`) 메인 트리에 없음.
5. `settle`을 몇 번 돌린 뒤 `assert!(!pending.is_finished())`, `application.git_state.claude.contents(request_id) == PROPOSED`.
6. `let viewport = viewport_of(application, &project, 1);` → `viewports.borrow().outputs[&viewport]`에서 "GAMMA" 글자와 `ide.acceptChanges` 노드를 찾을 때까지 프레임. 수정 쪽 텍스트가 PROPOSED(이동 전 편집이 사라짐, 원본과 같음).
7. 보조 창 입력: `viewports.borrow_mut().events.insert(viewport, pointer(accept.center(), egui::PointerButton::Primary));` 다음 프레임.
8. `answered(application, &context, pending) == text_content(diff_outcome_text(IdeDiffOutcome::Saved))`, 파일 = PROPOSED, 모든 창 트리에 ClaudeDiff 탭 없음(`taide_layout::service::all_roots` + `collect_claude_diff_tab_ids`).
9. 탭 전환 복귀(같은 검사 또는 별도): 새 요청을 열고 같은 pane에 파일 탭을 하나 더 열어 활성화 → `virtual_text(.., Modified)`가 `None`(문서 해제), 대기 목록 유지, `pending` 미완료 → ClaudeDiff 탭 다시 활성화 → 수정 쪽 PROPOSED로 다시 열림.

```
cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <TARGET> -- domains::git::claude_history_tests application::auxiliary_window_tests --test-threads=2
```

#### (f) 완료 판정

- 새 검사와 기존 `claude_history_tests`·`auxiliary_window_tests` 통과. 기능표 git-52 complete, terminal-agent-68의 "탭 닫힘 시 거절" 부분 근거.

#### (g) 막는 요소

- 없음. 실제 다중 창 focus 실기는 배치 31 실기 범위라 하지 않습니다.

### 3.7 git-49 Claude diff 요청 수신 (complete, 병합 결과 확인 필요)

#### (a) ID·기능·상태

`git-49` Claude diff 요청 수신(`ide:diff-requested` → ClaudeDiff 탭, `ideAutoOpenDiff` 존중) — **complete**(기능표 526줄). 단 complete 판정은 사양이 허용한 대체 수신 경로(`git_cache::Changes.ide`) 기준이며 shell lane 수신함 병합 뒤 그 분기로 옮기는 일이 남았습니다(`g7-commit-claude-diff.md` 5절).

#### (b) 원본 TS 동작

`TS/app/providers/ide-sync-provider.tsx` 49-68줄: 요청을 `setPendingClaudeDiff`로 기록 → 설정 `ideAutoOpenDiff`가 false면 지우고 `resolveIdeDiff(rejected)` → 아니면 `openTabInProject({ projectId, kind: claudeDiff { requestId, path: newPath }, title: tabName, target: null, preview: false })`, 실패면 오류 toast. 131-133줄 close-tab 요청이면 대기 목록에서 지움. 대기 목록은 `TS/entities/ide/claude-diff-registry.ts`의 앱 전역 `Map` 하나.

#### (c) native 현재 자산과 병합 미커밋 변경

- lane 커밋: `APP/git-claude-diff.rs` `Inbox::record`가 `AppEvent::IdeDiffRequested`·`IdeCloseTabRequested { request_id: Some }`를 `Event`로 기록, `git_cache::Changes.ide`가 그 `Inbox`, `tick_claude_diffs` 첫 줄 `consume_ide_diff_events`.
- 병합 미커밋(0.3절): `APP/provider-events.rs` `Event::IdeDiff(git_claude_diff::Event)`(37줄)와 `Inbox::record`의 두 arm(90-107줄), `consume_provider_events`의 `Event::IdeDiff(event) => self.accept_claude_diff_event(event)`, `accept_claude_diff_event`(2263-2295줄), `git_cache::Changes.ide`·`git_claude_diff::Inbox` 삭제.

#### (d) 확인 목록 (병합 결과)

1. `grep -n "IdeDiff" native/taide-native-app/src/provider-events.rs native/taide-native-app/src/domains/shell.rs native/taide-native-app/src/domains/git.rs`로 위 세 지점을 확인합니다.
2. `grep -n "Inbox\|ide:" native/taide-native-app/src/git-cache.rs native/taide-native-app/src/git-claude-diff.rs` 결과가 없어야 합니다(이중 기록이면 요청마다 탭이 두 번 열림).
3. 동작 차이 두 가지를 QA에 적습니다.
   - 수신함 상한: `provider_events::MAX_PROVIDER_EVENTS = 256`(11줄)을 넘으면 가장 오래된(프로젝트 닫힘 제외) 이벤트를 버리고 `log::warn!`합니다(117-127줄). IDE diff 요청이 버려지면 IDE는 `IDE_DIFF_TIMEOUT = 600_000ms`(`APP/ide-tools.rs` 27줄) 뒤 `Rejected`를 받습니다. 원본은 상한이 없습니다. 한 프레임 사이에 256개가 쌓이는 경우만 해당합니다.
   - 종료 중: `consume_provider_events`는 닫는 중·종료 준비·shutdown이면 꺼낸 이벤트를 처리하지 않고 버립니다(`APP/domains/shell.rs` 1114-1117줄). 그 사이 요청은 IDE 쪽 timeout까지 대기합니다.
4. `provider-events.rs` 단위 검사 `provider_inbox는_ide_diff_요청과_요청_id가_있는_close_tab만_기록한다`(병합 미커밋에 있음: 요청 1개·`request_id: None`인 close_tab은 무시·`Some`은 `Closed`)가 남아 있는지 봅니다.

#### (e) 자동 검사

기존 `[A]` `실제_app_ide_open_diff는_claude_diff_탭을_열고_수정_쪽_편집_뒤_accept로_저장한다`(289줄), `실제_app_claude_diff는_자동_열기_끄기_reject_탭_닫기_ide_close_tab을_원본대로_처리한다`(372줄)가 새 경로를 그대로 지납니다(이벤트가 같은 `PaintSink`의 `providers` 기록으로 들어감). 실행: 3.6 (e)와 같은 명령에 `provider_events` 필터를 더합니다.

#### (f) 완료 판정

- 위 검사 통과 후 기능표 git-49 비고에서 "대체 경로·이전 남음" 문장을 지우고 새 경로를 적습니다. terminal-agent-68(lane 기능표에서 unwired)·terminal-agent-69·settings-80 IDE diff 부분·shell-92 diff·close 부분을 git-49~52 근거로 갱신합니다(`g7-commit-claude-diff.md` 5절 병합 메모).

#### (g) 막는 요소

- 병합 진행 중. 끝난 결과를 확인만 합니다.

---

## 4. 이 영역에서 반복된 함정과 주의

1. **accesskit 이름 충돌**: 버튼과 그 tooltip 노드가 같은 label을 가져 `node(output, label)`이 tooltip을 집고 클릭이 빗나갔습니다(`g3-header-branches.md` 5절, `g2-changes.md` 5절). 버튼은 `node.role() != Role::Tooltip`, 스위처 트리거는 expanded 상태가 있는 노드, 목록 항목은 toggled 상태로 고릅니다(`application-git-header-tests.rs` 60-96줄의 `item`·`trigger`·`buttons`·`button`).
2. **macOS 임시 경로 정규화**: `std::env::temp_dir()` 아래 경로는 `/var/...`이고 백엔드는 `/private/var/...`로 정규화합니다. 비정규화 경로로 만든 기대값이 틀리고, 그 경로의 `git_file_log`가 `pathOutsideRepository`가 됐습니다(`g7` 4절, `g2` 5절). root를 `canonicalize`하거나 `real(path)`를 씁니다.
3. **egui 더블 클릭**: 0.3초 안에 같은 입력을 두 번 누르면 단어 선택이 되어 입력이 교체됐습니다(`g4-commit.md` 검증 표). 이미 focus가 있으면 다시 누르지 않습니다.
4. **검사 파일 미등록**: 새 `application-git-*-tests.rs`를 `domains/git.rs`에 `#[cfg(test)] #[path]`로 등록하지 않으면 컴파일조차 되지 않아 "통과"처럼 보입니다(`g3` 5절: header_tests가 등록되지 않은 채로 있었음). 추가 뒤 `--lib -- domains::git::<이름>`으로 실행 개수가 0이 아닌지 확인합니다.
5. **LSP mock 미빌드**: `native-lsp-mock` 예제가 없으면 LSP를 쓰는 앱 검사가 실패합니다(`g1-base.md` 5절). 먼저 빌드합니다.
6. **부하 흔들림**: 여러 lane·App 검사가 동시에 돌면 host 큐(64) 포화·시간 초과로 실패합니다. `--test-threads=2`(lane) 또는 `4`(게이트)로 실행하고, 시간 초과는 단독 1회 재실행으로 결함과 구분합니다(`docs/PROCESS.md` W1 기록, lane workflow `RULES`).
7. **가상 문서와 내장 서비스**: view가 붙은 `.json` diff 가상 문서를 내장 JSON 서비스가 처리해 `InvalidIdentity` 오류 toast 4개가 떴습니다(`g5-diff.md` 6.1). 새 문서 소비 서비스(tick 필터)를 만들거나 병합하면 `DocumentKey::Virtual` 제외를 확인합니다. 병합 미커밋에서 `editor-css.rs` 38줄, `editor-html.rs` 58줄, `editor-completion-json.rs` 54줄, `editor-json.rs` 118·218줄, `editor-json-validation.rs` 205·313줄, `editor-json-formatting.rs` 23줄, `editor-problems.rs` 389줄, `editor-selection-ranges.rs` 164줄이 `DocumentKey::AppFile(_) | DocumentKey::Virtual(_)`를 제외하며, CSS·HTML diff App 검사가 추가됐습니다(0.3절).
8. **오래된 영역으로 슬라이스 패닉**: 대화상자가 열린 동안 문서가 짧아지면 저장해 둔 충돌 영역으로 자르다 패닉했습니다(`g8-blame-conflicts.md` 6.1). 사용자 동작을 비동기로 적용할 때는 적용 직전 현재 문서로 다시 검증합니다(3.5의 revert 편집도 revision을 확인).
9. **원본 백엔드 특성 착각**: rename 행 path가 옛 경로로 와서 rename tooltip·`before_path`의 App 검사를 할 수 없었습니다(`g1-base.md` 4절, git2 0.21 `StatusEntry::path`). blame은 HEAD 기준이라 미커밋 줄 문구가 나오지 않습니다(`g8` 3.5절). 기대값은 원본 백엔드 동작으로 정합니다.
10. **사양과 실제 코드의 차이**: 사양의 `git-status-supply.rs`, `SurfaceExtras.line_end_texts`·`scroll_override`, `EditorOutput.lane_press`, `git::Command::Diff/Show/FileLog/Status` 등은 실제로 만들어지지 않았거나 다른 형태입니다(각 QA "사양과 실제 코드 차이" 절). 사양 문장보다 실제 코드를 따릅니다.
11. **UTF-16 길이**: 원본 JavaScript 길이는 UTF-16 단위입니다. blame 요약 50자 자르기에서 char 수로 세어 결과가 달랐습니다(`g8` 6.2). monaco 이식(3.3.1·3.4.3)은 처음부터 UTF-16 단위로 계산합니다.
12. **즉시 반영 설정**: diff 설정은 매 프레임 `services.state.settings.read()`로 읽어야 열린 탭에 바로 반영됩니다(`show_diff`). 캐시 키에 설정을 넣지 않습니다(showMoves 이동 계산을 항상 하는 이유).
13. **병합 중 worktree**: 작성 시점 lane worktree에 `UU` 파일이 있고 병합 작업자가 git 파일을 고치고 있습니다. 그 worktree에서 파일을 고치거나 git 상태를 바꾸지 않습니다.
14. **기능표 셀 안의 `|`**: git-61 요구사항 문구에 `` `\|\|\|\|\|\|\|` ``가 있어 `awk -F'|'` 같은 열 분리로 행을 세면 그 행만 열이 밀립니다(1.1절). 행 상태는 `grep "^| git-61 "`로 따로 확인합니다.
15. **요약표 갱신 누락**: lane 규칙이 "상단 합계 표는 건드리지 않습니다"라 기능표 18줄 git 요약(1/1/34/29)이 행 판정(58/6/0/0)과 다릅니다. 병합 뒤 메인이 다시 세어야 하며, 요약표 숫자를 진척 근거로 쓰지 않습니다.
16. **아직 없는 QA 문서 참조**: 병합 미커밋 기능표의 shell-89 비고가 `2026-11-03-native-git-lane-merge.md`를 근거로 적었지만 그 파일이 없습니다(0.3절). 병합 커밋 전에 문서를 쓰거나 비고를 고쳐야 합니다.

---

## 5. 행을 닫을 때 갱신할 문서

1. 기능표(`docs/quality-assurance/2026-10-09-native-function-matrix.md`): 해당 git 행의 판정·비고(근거 QA 파일명과 검사 이름), 같은 근거를 쓰는 다른 영역 행(1.3절 끝 목록), 머리 요약표의 영역별 수와 합계(병합 뒤 행을 다시 세어 갱신, 4절 함정 15).
2. 새 QA 문서 `docs/quality-assurance/<날짜>-native-git-<주제>.md`: 대상 ID와 판정, 원본 근거, 구현 파일, 사양·대응표와 실제 코드 차이, 실행한 명령과 실제 결과(실패 → 수정 → 재실행 포함), 남은 범위.
3. monaco 이식이 있으면 `THIRD_PARTY_LICENSES.md`의 "Ported source in `native/taide-native-editor`" 절과 oracle 스크립트(`docs/utils/`).
4. 범위 결정(3.3.8, 3.4.4의 breadcrumb·커서 펼침·tooltip·fold glyph 위치, 3.5 아이콘 색·애니메이션)을 내렸으면 `docs/acknowledge/<날짜>-native-git-remaining-decisions.md`.
5. `docs/PROCESS.md` 맨 위 절의 진행 항목(W1·W2 줄의 "실행 중: git" 항목과 "병합 뒤 연결: git-38 → explorer-search-7 Git 장식 공급, git-7 → shell `notify_event`").
