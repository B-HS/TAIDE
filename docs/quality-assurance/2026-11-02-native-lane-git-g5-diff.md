# native lane git 묶음 5/8 (g5-diff) — diff 렌더러·작업 트리 diff 탭

> 작성일: 2026-11-03 · worktree `TAIDE-lanes/git`(branch `lane/git`) · 사양 `docs/research/2026-10-28-lane-git-spec.md` 2.0.4·git-39~46·48·3.1 G5
> 결정: `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안: 결정 4 A, 5 A, 6 A, 7 A), `docs/acknowledge/2026-10-26-native-autonomous-continuation.md`

## 1. 대상 ID와 결과

| ID | 결과 | 한 줄 요약 |
| --- | --- | --- |
| git-39 | complete | 작업 트리/staged diff 탭 본문(`git_diff_file` + `diff_hunks`, 가상 문서 2개, fs·git 변경 재조회) |
| git-40 | partial | side-by-side·inline·채움 빗금·줄 배경·스크롤 동기화·분할선·900px 자동 inline. 문자 단위 강조 등 결정 4 A 일부 남음 |
| git-41 | complete | 탭별 side-by-side, focus pane diff 탭만 Alt+\ 토글(결정 5 A), 다시 보면 side-by-side |
| git-42 | partial | 미변경 영역 숨김 설정 완료. `editorDiffShowMoves`(결정 6 A) 남음 |
| git-43 | complete | 수정 쪽 hunk lane 막대와 누름 → stage/unstage hunk |
| git-44 | complete | `is_stageable`(compare 없음 && 충돌 행 아님) |
| git-45 | complete | `compare_with` 파일 대 파일 비교 본문 |
| git-46 | complete | 로딩 빈 배경·실패 문구, 첫 그림 프레임 로딩 검사 |
| git-48 | complete | 파일 탭 메뉴 `tab.openChanges` → preview diff 탭 |
| explorer-search-22 | complete | 탐색기 "비교 위해 선택/선택 항목과 비교" → 비교 탭(본문 git-45) |

## 2. 원본 근거

사양 git-39~48 절을 기준으로 했고 아래 구간만 다시 확인했습니다.

- `src/features/git/diff-view.tsx:26-83`: monaco diff editor, `readOnly`, `ignoreTrimWhitespace: false`, `renderSideBySide`, `hideUnchangedRegions: { enabled }`, `experimental: { showMoves }`.
- `src/shared/lib/diff-view-settings.ts`: 두 설정 기본 false, DiffPane·CommitFileDiff·충돌 비교 공용.
- monaco `diffEditorViewModel.js:346-483` `UnchangedRegion`(`fromDiffs`, `getHiddenModifiedRange`, `showMoreAbove/Below`, `showAll`), `hideUnchangedRegionsFeature.js:96-142`(막대 view zone 24px, `afterLineNumber = 숨김 시작 - 1`), `:266-410`(`CollapsedCodeOverlayWidget`: 위·아래 가장자리 누름 = `revealLineCount` 펼침, 아래 펼침 뒤 `setScrollTop` 보정, `$(unfold)` 누름·`"{0} hidden lines"` 더블클릭 = `showAll`), `style.css:6-102`(13px 글자, 가장자리 띠, 가운데 24px), `editorColors.js:88-90`(`unchangedRegionBackground` = `sideBar.background`, `unchangedRegionForeground` = `foreground`, `unchangedCodeBackground` dark `#74747429`/light `#b8b8b829`).
- TAIDE는 monaco nls 메시지를 바꾸지 않으므로 막대 문구는 monaco 기본 영어 `"{n} hidden lines"`입니다(사양 2.0.5 blame 영어 고정과 같은 처리, locale 카탈로그 키 없음).

## 3. 구현

### 3.1 앞 작업분(같은 묶음, 이어받음)

- `crates/taide-git/src/service.rs`: `DiffHunk`, `diff_hunks(original, modified, ignore_trim_whitespace)`(libgit2 `Patch::from_buffers`, `context_lines(0)`, `indent_heuristic(true)`).
- `native/taide-native-editor`: `DocumentKey::Virtual(VirtualKey { owner, side })`, `open_virtual`, `replace_virtual`, `tests/virtual-document.rs`. 앱의 `DocumentKey` 전수 match(`diagnostics.rs`, `editor-locations.rs`, `persistence.rs`, `editor-json.rs`, `editor-json-validation.rs`, `domains/editor.rs`, `ED/store.rs`)에 "파일 아님" arm.
- `native/taide-native-ui/src/editor_surface.rs`: `ExtraZone`, `SurfaceExtras`, `show_request_with_extras`, `EditorOutput.extra_zones`.
- `native/taide-native-ui/src/diff-view.rs`(신규): side-by-side/inline, 채움 띠와 `diffEditor.diagonalFill` 빗금, 줄 배경, 스크롤 동기화, 분할선 끌기, stage lane과 누름.
- 앱 `git-diff.rs`(신규: 외형, `is_stageable`, Alt+\ 소비, 탭별 모드, 숨은 탭 가상 문서 해제), `git-cache.rs` `Key::Diff`/`Key::Compare`, `domains/git.rs` `show_diff_tab`·`StageHunk`·탭 메뉴 intent, `UI/shell.rs` 탭 메뉴 항목, 탐색기 비교 메뉴.

### 3.2 이번 작업분(git-42 숨김, git-46 검사)

- `UI/editor_surface.rs`(추가만): `SurfaceExtras.hidden_lines: Vec<Range<usize>>`. `Projection.extra_hidden_lines`를 fold 숨김과 `display_map::merged_line_ranges`로 합치고, `InputState.rendered_hidden_lines`로 `reveal` 경로도 같은 숨김을 씁니다.
- `UI/diff-view.rs`:
  - `UnchangedRegion`, `Reveal`, `Hidden`, `unchanged_regions(hunks, original_lines, modified_lines)`(monaco `fromDiffs` 그대로: 문맥 3, 최소 3, 문서 처음/끝 영역은 안쪽만 문맥), `show_more_above/below`(20줄), `show_all`.
  - `Request.hide_unchanged`, `Colors.unchanged_code/unchanged_region/unchanged_region_foreground`.
  - 영역마다 양쪽(inline이면 수정 쪽만)에 숨긴 줄, 24px 막대 zone(숨김 시작 앞 줄 뒤), 영역 전체 줄 배경(`unchangedCodeBackground`).
  - 막대: 배경 `appSidebar.background`, gutter 폭 칸에 codicon `unfold`(16px) = 전부 펼침, 글자 칸 가운데 `"{n} hidden lines"`(13px, `app.foreground`) 더블클릭 = 전부 펼침, 위 가장자리 띠(막대 위 6px~안 4px) = 위로 20줄, 아래 가장자리 띠 = 아래로 20줄. 커서는 monaco `canMoveTop/canMoveBottom` 규칙(ns/s/n-resize), 아이콘은 손가락.
  - 아래 펼침은 영역 뒤 줄의 화면 위치를 유지하도록 양쪽 스크롤을 `펼친 줄 수 × 줄 높이 - (막대가 사라지면 24px)`만큼 내립니다.
  - 펼침 상태는 diff 위젯 egui 임시 상태에 영역 단위로 두고, 설정이 꺼지면 비웁니다.
  - 채움 빗금은 채움 zone에만 칠하도록 순서를 나눴습니다(막대 zone 제외).
- 앱 `git-diff.rs`: `Appearance`에 세 색(다크/라이트 `unchangedCodeBackground` 상수, `appSidebar.background`, `app.foreground`).
- 앱 `domains/git.rs` `show_diff`: 매 프레임 `settings.editor_diff_hide_unchanged_regions`를 읽어 `Request.hide_unchanged`로 넘깁니다(설정 변경 즉시 반영).

### 3.3 실제 앱 경로

`main.rs` → `NativeApplication` → 탭 내용 그리기의 `TabKind::Diff` 분기 → `AppSurfaces::show_diff_tab`(`domains/git.rs`) → 캐시 `Key::Diff`/`Key::Compare` 조회(`Command::Read`, transient task) → `show_diff` → `taide_native_ui::diff_view::show`. 설정은 `git::Command::PatchSettings` → `host::update_control_settings`로 바뀐 공유 설정을 다음 프레임에 읽습니다.

### 3.4 사양과 실제 코드의 차이(실제 코드를 따름)

- 사양 2.0.1의 `DiffView::show(ui, store, sides, hunks, options) -> DiffOutput { lane_press }`는 실제 `diff_view::show(ui, store, editor, Request, tokens) -> Output { lane_press, .. }`이고, lane 누름은 `EditorOutput.lane_press` 필드 대신 diff 위젯이 `EditorOutput.geometry`의 gutter lane 칸으로 판정합니다.
- 사양의 `SurfaceExtras { zones, line_end_texts, scroll_override }`는 이 묶음에서 `{ zones, hidden_lines }`입니다. 스크롤 동기화는 `set_view_state`로 하므로 `scroll_override`가 없고, `line_end_texts`는 G8(blame) 몫입니다.
- 사양 2.0.2의 `git::Command::Diff`/`Reply::Diff` 대신 공용 캐시 `Command::Read(Request<Key>)`의 `Key::Diff`/`Key::Compare`를 씁니다.
- native 표시 맵은 첫 줄을 숨기지 못합니다(`display-map.rs` `FIRST_HIDEABLE_LINE = 1`). 그래서 문서 처음 영역은 첫 줄을 남기고 막대를 그 뒤에 두며 숨긴 줄 수가 monaco보다 1 적습니다. 두 쪽 모두 같은 규칙이라 정렬은 유지됩니다.

## 4. 검증

모든 cargo 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/git`.

| 명령 | 결과 |
| --- | --- |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test diff-view` | 8 passed(신규 2: 영역 계산·펼침 규칙, 양쪽 막대·위 가장자리 20줄·아이콘 전부 펼침) |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test editor_surface -- --test-threads=2` | 87 passed(`SurfaceExtras` 회귀) |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- diff_tests --test-threads=2` | 7 passed(신규 2: 숨김 설정 켜기/끄기 즉시 반영, 첫 그림 로딩 화면) |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- git --test-threads=2` | 62 passed |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --test explorer -- --test-threads=2` | 21 passed(explorer-search-22 회귀) |
| `cargo test --quiet --manifest-path crates/taide-git/Cargo.toml diff_hunks` | 2 passed |
| `cargo test --quiet --manifest-path native/taide-native-editor/Cargo.toml --test virtual-document` | 1 passed |
| `cargo check --quiet --all-targets --manifest-path native/taide-native-app/Cargo.toml` | 성공(경고는 기존 vendor·`zen.rs`·`editor-code-actions.rs`·`application-inlay-hints-tests.rs`) |
| `cargo check --quiet --all-targets --features native-host,inspection --manifest-path native/taide-native-ui/Cargo.toml` | 성공(경고는 기존 `tests/workbench.rs:1816`) |
| `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 성공(동결 crate 기존 경고만) |
| `cargo fmt --check`(ui, app, editor, taide-git) | 차이 없음 |

App 검사 세부:

- `실제_app_미변경_영역_숨김_설정은_열린_diff_탭에_바로_반영되고_끄면_모두_보인다`: 40줄 파일의 20번째 줄 변경 diff 탭 → `PatchSettings { editor_diff_hide_unchanged_regions: Some(true) }` → 양쪽 `"15 hidden lines"`·`"18 hidden lines"`, `line 05` 숨김, 문맥 `line 17` 양쪽 표시 → `Some(false)` → 막대 없음, `line 05` 양쪽 표시.
- `실제_app_diff_탭은_응답_전_첫_그림에서_내용과_오류_없이_빈_배경만_그린다`: diff 조회는 탭이 처음 그려진 프레임 끝에만 나가므로, 탭 제목이 처음 그려진 프레임은 항상 응답 전입니다. 그 프레임 뒤 캐시 값 없음, 내용(`alpha`)·`editor.diffLoadFailed` 없음, 가상 문서 0개, 이후 양쪽 내용 표시를 단언합니다(지연 포트 없이 결정적).

## 5. 남은 범위

- git-40(결정 4 A): 문자 단위 강조(`diff.insertedBackground`/`removedBackground`), ± 표시, diff overview ruler, inline 삭제 줄의 구문 색, inline 원본·수정 두 줄 번호 열.
- git-42: `editorDiffShowMoves`(결정 6 A: monaco `computeMovedLines` 이식 + 이동 블록 표시). 숨김 쪽 세부: 가장자리 끌기, 전부 펼친 뒤 다시 접는 glyph(`fold-unchanged`), 가장자리 hover 강조(`focusBorder`), 막대 안 outline breadcrumb, 막대 그림자, diff 재계산 때 펼침 상태 재매핑(지금은 같은 영역만 유지), 아래 펼침 스크롤 보정의 자동 검사(코드 경로만).
- git-42의 "ClaudeDiff에는 적용 안 함"은 ClaudeDiff 표면(G7 git-50)이 아직 없어 그 묶음에서 확인합니다. CommitFileDiff(git-47)·충돌 비교(git-63)도 같은 `Request.hide_unchanged`를 넘기면 됩니다.
- git-39: rename `before_path`는 백엔드 status가 rename 행에 원래 경로를 주지 않아(G1 QA 4절) 코드 경로만 있습니다.
- git-44: 충돌 파일 diff 탭 App 검사는 G8 충돌 fixture와 함께 합니다.
- editor-76·shell-89 행의 diff 부분 근거는 이 문서이며, 그 행 갱신은 각 행 소유 lane 병합 때 합니다.

## 6. 리뷰 수정(2026-11-03)

### 6.1 차단 1: 내장 JSON 서비스가 diff 가상 문서를 처리하던 결함

- 대상: git-39·40·47·50·54·63(diff 표면 전부), editor-76·shell-89의 diff 부분.
- 원인(실제 코드 확인): `editor-json.rs`와 `editor-json-validation.rs`의 `tick` 필터가 `language_id == "json"` && Normal 등급 && `AppFile`만 제외 && view 있음이라, view가 붙은 `.json` diff 가상 문서가 통과했습니다. 두 서비스의 `build()`·`resource_uri()`는 `Virtual`에 `Err(InvalidIdentity)`를 돌려주고, `domains/editor.rs`의 두 tick 소비가 `self.report(..)`로 오류 toast를 띄웠습니다.
- 기준: 사양 2.0.4는 `editor-json.rs`·`editor-json-validation.rs`를 포함한 전수 match에서 가상 문서를 "파일 아님"(진단·LSP 대상 아님)으로 처리하라고 정했습니다. 원본 monaco diff 모델도 TAIDE 진단 경로를 타지 않습니다.
- 재현: 새 App 검사를 수정 전에 실행해 실패를 확인했습니다. 단언 순서를 바꾼 두 번째 실행에서 `오류 toast: ["native editor: InvalidIdentity" × 4]`(원본·수정 문서 × 구조·검증 서비스)로 리뷰의 증상과 같았습니다.
- 수정: 두 필터를 `!matches!(snapshot.key, DocumentKey::AppFile(_) | DocumentKey::Virtual(_))`로 바꿨습니다. Err를 보고하는 `domains/editor.rs`는 바꾸지 않았습니다. diff 위젯은 folding을 끄므로(3.1) JSON 구조 모델이 빠져도 표시 차이가 없고, 구문 강조는 문서 키를 보지 않는 syntax 파이프라인이라 그대로입니다.
- 검사 `실제_app_json_파일_diff_탭은_구문_강조를_그리고_내장_json_서비스_오류_toast를_띄우지_않는다`(`application-git-diff-tests.rs`): `package.json` 수정 → 작업 트리 diff 탭 → 양쪽 `"version": 1` 줄이 각각 두 색 이상의 section(구문 강조), `"name": "BETA",` 표시, 가상 문서 2개 → 1초(검증 debounce 500ms의 두 배) 동안 프레임 진행 → 오류 toast 0개, 두 서비스의 문서 목록에 가상 문서 없음.
- 범위: 필터는 `DocumentKey`로 거르므로 표면과 무관하게 모든 가상 문서(커밋 diff, 파일 이력 inline, 충돌 비교, ClaudeDiff 수정 쪽)에 같이 적용됩니다. `.json` App 검사는 작업 트리 diff 탭 1건이고, 다른 표면의 `.json` App 검사는 하지 않았습니다.
- 병합 메모: main에 먼저 병합된 editor lane의 CSS·HTML·JSON 내장 서비스 tick도 같은 종류의 필터라 병합 때 `Virtual` 제외를 확인해야 합니다(이 worktree에는 CSS·HTML 서비스가 없음).

### 6.2 과대 판정 정리

- git-39(및 G1 git-19, G2 git-24): rename 행의 `before_path`·tooltip은 백엔드 status가 원래 경로를 주지 않아 코드 경로만 있고, 사양의 rename fixture App 검사는 하지 못했습니다. 대응표 행 근거에 그 한계를 적었습니다.
- git-40·47·50·54·63, editor-76·shell-89 diff 부분: `.json` 파일 diff는 6.1 수정과 검사 뒤에만 근거로 씁니다. 행 근거에 "가상 문서 JSON 서비스 제외, `.json` App 검사는 작업 트리 diff 1건"을 적었습니다.

### 6.3 검증

모든 cargo 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/git`, 앱 lib 검사는 `--test-threads=2`.

| 명령 | 결과 |
| --- | --- |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- 해결_중_문서가_짧아져 해결_대화상자가_열린_동안 json_파일_diff_탭은`(수정 전) | 3 failed(재현: JSON 서비스 문서 목록에 가상 문서, 충돌 슬라이스 범위 초과 패닉 2) |
| 같은 crate `--lib -- json_파일_diff_탭은`(수정 전, 단언 순서 변경) | 1 failed(`InvalidIdentity` 오류 toast 4개) |
| 같은 crate `--lib -- 해결_중_문서가_짧아져 해결_대화상자가_열린_동안 json_파일_diff_탭은`(수정 뒤) | 3 passed |
| 같은 crate `--lib -- git_conflict:: domains::git::editor_tests domains::git::diff_tests application::json_tests editor_json` | 26 passed |
| 같은 crate `--lib -- domains::git:: git_conflict:: editor_json application::json_tests` | 53 passed(G1~G8 git App 검사 전부, JSON 내장 서비스 App·단위 회귀) |
| `cargo check --quiet --all-targets --manifest-path native/taide-native-app/Cargo.toml` | 성공(경고는 기존 vendor·`editor-code-actions.rs`·`zen.rs`·`application-inlay-hints-tests.rs`) |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 차이 없음 |

UI·editor crate는 바꾸지 않아 UI 하네스와 Wasm check는 다시 돌리지 않았습니다.
