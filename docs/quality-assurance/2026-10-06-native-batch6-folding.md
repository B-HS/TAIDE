# Native 전환 배치 6 단계 3/3 — 접기(들여쓰기 기반) QA

상태: 검증 계약 V1~V5 전부 exit 0. 접기 화면의 실제 외형과 Monaco 내장 기본 키는 미확인·미연결로 남았습니다(7절).

경로는 저장소 루트(`/Users/hyunseokbyun/development/TAIDE`) 기준 상대 경로입니다. MONACO 는 `node_modules/monaco-editor/esm/vs`(0.56.0)입니다.

## 0. 이어받기 판정

앞선 작업자가 컨텍스트 한도로 중단한 작업 트리를 이어받았습니다. 구현을 다시 만들지 않고, 먼저 검증 계약을 그대로 실행한 뒤 Monaco·TS 소스와 대조해 남은 것만 채웠습니다.

| 항목 | 이어받은 시점 | 이번에 한 일 |
| --- | --- | --- |
| F1 접기 영역 계산 | 완료 | `indentRangeProvider.js`·`utils.js` 와 줄 단위로 대조. 수정 없음 |
| F2 숨김 줄 | 완료 | `hiddenRangeModel.js`·`foldingModel.js`·`foldingRanges.js` 와 대조. 수정 없음 |
| F3 gutter 컨트롤 | 완료 | `folding.css`·`foldingDecorations.js`·`editorOptions.js` 와 대조. 클릭 뒤 머리 줄 reveal 이 빠져 있어 추가(2절 F3) |
| F4 명령 | 완료 | `folding.js` 의 각 `invoke` 와 대조. 수정 없음. Monaco 내장 기본 키는 미연결(5절) |
| F5 옵션 | 완료 | `code-editor.tsx:271`, `editor-pane.tsx:422` 와 대조. 수정 없음 |
| F6 결합 | 토큰·장식·현재 줄·IME·드래그를 접힌 상태에서 고정하는 UI 테스트 없음 | UI 테스트 1건 추가(구현 수정 없이 통과) |
| QA 문서 | 없음 | 이 문서 |

이어받은 시점의 결과(수정 전): V1 124 통과·1 ignored, V2 lib 117·editor_surface 65, V3 check 오류 0·lib 368·save 2·save-syntax 1·paste-shortcuts 4, V4 오류 0, V5 세 크레이트 차이 없음.

## 1. 구조 요약

- 순수 모델(`native/taide-native-editor`): `folding.rs`(영역 계산, 접기 모델, 편집 추적, 스토어 연산), `display-map.rs`(숨김 줄 투영).
- 표면(`native/taide-native-ui`): `editor_surface.rs`(프레임마다 영역 캐시·접힘 맞춤·명령·클릭·reveal), `editor-gutter.rs`(컨트롤 위치·클릭 영역·fade), `editor-paint.rs`(접힌 줄 배경·생략 표식), `editor-geometry.rs`(생략 표식 위치).
- 앱(`native/taide-native-app`): `application.rs`(옵션 전달, chevron 그리기, 명령 큐), `command-dispatch.rs`(`take_fold_commands`), `shell_keymap.rs`(`Run::FoldDocument` → `ShellIntent::FoldDocument`).
- `ViewState.folds` 는 설계대로 "숨겨진 첫 줄의 시작 바이트 .. 숨겨진 마지막 줄의 내용 끝 바이트"입니다. 머리 줄은 범위에 들어가지 않습니다.

## 2. 항목별 변경 내용

### F1. 접기 영역 계산

- `folding.rs` `indent_regions(rope, tab_size, limit)`, `indent_level`, `MAX_FOLDING_REGIONS = 5000`.
- 아래에서 위로 훑으며 더 깊은 들여쓰기 스택을 걷어 내고 `end_line > line` 인 영역만 남깁니다. 공백뿐인 줄은 건너뜁니다(빈 줄은 앞 블록에 붙지 않고, 다음 같은·얕은 줄 바로 앞까지가 영역 끝입니다).
- 한도 초과 시 들여쓰기 깊이별 개수를 얕은 쪽부터 채우고, 경계 깊이는 문서 위에서부터 한도까지만 남깁니다. 깊이 1000 이상은 개수에 넣지 않습니다.
- 줄 번호 상한 `0xFFFFFF`(MONACO `foldingRanges.js` 의 `MAX_LINE_NUMBER`)를 0 기준으로 옮겼습니다.
- 테스트(`native/taide-native-editor/tests/folding.rs`): 영역 경계와 빈 줄, 탭 크기별 깊이, 한도 초과, 큰 문서 시간 기록.

### F2. 숨김 줄

- `display-map.rs`: `HiddenLines`(병합된 줄 범위 + 범위 앞의 보이는 줄 수), `merged_line_ranges`(첫 줄 제외·문서 안으로 자르기·겹침 병합), `DisplayMap::set_hidden_lines`·`hidden_lines`·`hidden_lines_at`. 숨김 먼저, wrap 나중입니다. 숨겨진 줄은 표시 줄 0개이고 `row_of_byte` 는 숨김 줄의 바이트를 머리 줄의 마지막 표시 줄로 돌려줍니다. `RowSegment.ends_folded` 는 머리 줄의 마지막 표시 줄에서만 참입니다.
- `folding.rs`: `hidden_lines`(folds → 줄 범위), `tracked_folds`(편집 추적), `FoldingModel::following_edit`·`reconcile_folds`(편집 뒤 새 영역에 맞춤), `reveal_carets`(선택의 시작점이 숨김 줄에 들어가면 펼침), `store_folds`·`shown_selection`(숨김이 바뀌면 숨김 줄 안의 선택 끝을 머리 줄 끝으로 옮김).
- `store.rs`: `tracked_view_folds`. 일반 편집과 undo·redo 모두 편집 저널의 변경으로 접힘을 옮깁니다. 이전에는 undo·redo 에서 `view.folds.clear()` 였습니다.
- `editing.rs`: `shown_offset`(좌우 이동이 숨김 줄을 건너뜀), `rope_line_content_range`.
- undo·redo 에서의 TS 동작: Monaco 는 접힘을 모델 장식으로 들고 있고(`foldingModel.js:82-104`) undo·redo 도 편집이므로 장식이 따라 움직입니다. native 도 지우지 않고 옮깁니다(`tests/folding.rs` `undo와_redo는_접힌_범위를_지우지_않고_되돌린_문서의_같은_줄로_옮긴다`).
- 테스트: `tests/display-map.rs` 의 숨김 3건(숨김+wrap 왕복 포함), `tests/folding.rs` 의 편집 추적·맞춤·캐럿 진입·undo 등.

### F3. gutter 컨트롤

- 표시 조건: TS 는 `showFoldingControls` 를 지정하지 않으므로 Monaco 기본 `mouseover` 입니다(`editorOptions.js:3357`). 접힌 영역의 컨트롤은 항상 보이고, 펼쳐진 영역의 컨트롤은 gutter 에 포인터가 있는 동안만 0.5초 fade 로 나타납니다(`folding.css:5-31`). `editor-gutter.rs` `FoldControlFade`.
- 크기·위치: 글꼴 크기의 140%, 왼쪽 여백 2px(`folding.css:15-16`), 접기 폭 16px(`editorOptions.js:1221-1224`), 줄 장식 폭 10px(`editorOptions.js:1481`). `Gutter::fold_control_rect`.
- 아이콘: 접힘 chevronRight, 펼침 chevronDown(`foldingDecorations.js:26-27`). 표면은 `FoldControl { rect, chevron_rotation, color }` 만 내보내고 앱이 기존 아이콘 레지스트리의 `Glyph::ChevronRight` 를 회전해 그립니다(`application.rs` 의 `paint_fold_control`). 색은 `icon.foreground`(`foldingDecorations.js:25`)이며 TS 테마가 `appSidebar.iconDefault` 로 매핑하므로(`src/shared/lib/monaco/theme.ts:44`) 같은 값인 `visuals.weak_text_color` 를 씁니다.
- 접힌 줄: 배경은 선택 색의 30%(`foldingDecorations.js:23`, `foldingHighlight` 기본 true), 줄 끝 표식은 `⋯`(U+22EF), 좌우 여백 0.2em, 색 `#808080`(`folding.css:33-40`, `foldingDecorations.js:24`). `editor-paint.rs` `folded_background`·`fold_placeholder`.
- 클릭(`folding.js:301-406`): 줄 장식 영역에서 왼쪽 4px 를 뺀 곳이 컨트롤 클릭이고 캐럿을 옮기지 않습니다. 접힌 줄 끝의 표식 클릭은 캐럿을 줄 끝에 두고 펼칩니다. 그 뒤의 빈 곳은 펼치지 않습니다(`unfoldOnClickAfterEndOfLine` 기본 false, `editorOptions.js:3224`). Shift·가운데 버튼은 재귀, Alt 는 주변 전환입니다. 누른 줄과 뗀 줄이 같아야 합니다.
- 이번에 추가: 클릭으로 접힘이 바뀐 뒤 `this.reveal({ lineNumber, column: 1 })`(`folding.js:403`)에 해당하는 동작이 없었습니다. `editor_surface.rs` 에 `InputState.clicked_fold_line`, `FoldReveal::LineStart` 를 넣어 다음 프레임에 머리 줄이 화면에 다 보이지 않으면 가운데로 옮깁니다. 테스트 `접기_컨트롤_클릭_뒤에_머리_줄이_화면에_다_보이지_않으면_가운데로_보이게_하고_다_보이면_스크롤을_그대로_둔다`.

### F4. 명령

- `command-registry.rs` `fold_command`, `Run::FoldDocument`, `ActiveEditor { is_read_only, has_folding }`. TS 가 등록한 접기 범주 19개(`src/shared/lib/monaco/monaco-actions.ts:746-829`) 중 13개가 실행 가능합니다: fold, unfold, toggleFold, foldRecursively, unfoldRecursively, toggleFoldRecursively, foldAll, unfoldAll, foldAllExcept, unfoldAllExcept, gotoParentFold, gotoPreviousFold, gotoNextFold.
- 실행 경로 없음 6개와 이유: foldAllBlockComments·foldAllMarkerRegions·unfoldAllMarkerRegions(언어 구성 필요), createFoldingRangeFromSelection·removeManualFoldingRanges(수동 범위, 비목표), toggleImportFold(provider 가 주는 imports 종류 필요).
- 접기가 꺼진 편집기에서는 13개 모두 실행 불가입니다(`editor_action_ids` 가 `has_folding` 으로 거름).
- 명령 의미는 `folding.js` 의 각 `invoke` 와 `foldingModel.js:256-531` 을 그대로 옮긴 `FoldingModel::run` 입니다. 명령 뒤에는 선택 시작 위치를 화면 밖이면 가운데로 보이게 합니다(`folding.js:450-453`).
- 키맵: native 키맵 기본값 카탈로그(`native/taide-native-ui/src/keymap-defaults.json`)에는 접기 항목이 없어 연결할 기본값이 없습니다. 사용자가 키 바인딩 편집기에서 지정한 키는 기존 `command_bindings` 경로로 `Run::FoldDocument` 까지 갑니다(`shell_keymap.rs`). Monaco 내장 기본 키는 5절을 보십시오.

### F5. 옵션

- TS 에는 접기 설정 필드가 없습니다. `folding: !largeFile`(`src/features/editor/code-editor.tsx:271`)뿐이고 `largeFile` 은 `file.tier === 'large' || file.tier === 'readOnly'`(`src/widgets/editor-pane/editor-pane.tsx:422`), untitled·app 파일은 false 입니다(`untitled-pane.tsx:200`, `app-file-pane.tsx:165`).
- `presentation.rs` `editor_folding(tier)` 가 같은 조건이고 `application.rs` 가 문서 탭의 `EditorPresentation.options.folding` 에 넣습니다. `editor_presentation(&Settings)` 만으로는 켜지지 않습니다(기본 false). 접기가 꺼진 표시에서는 남아 있던 접힘을 지웁니다(`maintain_folds`).

### F6. 결합

- 기존 테스트: 좌우·세로·page 이동, gutter·표식 클릭, 명령, 캐럿 진입 시 펼침, wrap 된 머리 줄, 머리 줄 끝 입력·줄바꿈·undo, 위쪽 접힘 변경 시 스크롤 유지.
- 이번에 추가한 테스트 `접힌_상태의_토큰과_장식과_현재_줄_강조와_ime_좌표와_드래그_선택은_숨김_줄을_뺀_표시_줄에_놓인다`: 숨김 줄의 토큰·장식은 그려지지 않고, 숨김 뒤 줄의 토큰 색·범위 장식·현재 줄 강조·캐럿·IME 후보 창 좌표가 그 줄의 표시 줄에 놓이며, 접힘을 가로지르는 드래그는 숨김 줄의 바이트까지 선택하고 접힘을 유지합니다. 구현 수정 없이 통과했습니다.
- 저장 정리와 토큰화는 문서 전체가 대상입니다. 표면이 돌려주는 `rendered_lines` 는 보이는 첫 표시 줄의 문서 줄부터 마지막 표시 줄의 문서 줄까지라서 사이의 숨김 줄도 토큰화 요청에 들어갑니다.

## 3. TS·Monaco 근거

| 동작 | 근거 |
| --- | --- |
| 들여쓰기 영역, 한도 5000 | MONACO `editor/contrib/folding/browser/indentRangeProvider.js:8, 26-90, 95-191`, `editor/common/model/utils.js:10-31` |
| 영역 병합·접힘 유지 조건 | `foldingRanges.js:243-324`, `foldingModel.js:77-128` |
| 숨김 범위와 선택 보정 | `hiddenRangeModel.js:30-63, 75-103`, `folding.js:261-271` |
| 캐럿 진입 시 펼침 | `folding.js:272-300`(선택의 시작 줄 기준) |
| 클릭 | `folding.js:301-406` |
| 명령 | `folding.js:437-470` 과 각 액션의 `invoke`, `foldingModel.js:256-354, 394-531` |
| 장식·색·아이콘 | `foldingDecorations.js:23-122`, `folding.css` |
| 옵션 기본값 | `editor/common/config/editorOptions.js:1221-1224, 1481, 3212-3224, 3357` |
| TS 명령 목록 | `src/shared/lib/monaco/monaco-actions.ts:746-829` |
| 대형 파일 예외 | `src/features/editor/code-editor.tsx:271`, `src/widgets/editor-pane/editor-pane.tsx:422` |
| 컨트롤 색 | `src/shared/lib/monaco/theme.ts:44` |

## 4. 설계 문서와 달라진 점

- `DisplayMap::build(document, hidden_lines, wrap)` 대신 `build(document, wrap)` + `set_hidden_lines(&[Range<usize>]) -> bool` 입니다. `refresh` 로 문서가 바뀌면 숨김이 지워지므로 호출부가 매 프레임 다시 줍니다. 접힘만 바뀌었을 때 wrap 계산을 다시 하지 않기 위해서입니다.
- 누적합은 Fenwick 트리가 아니라 배치 4 에서 정한 `first_rows` 배열과 `HiddenLines.shown_before` 입니다. 접힘이 바뀌면 바뀐 첫 줄부터 다시 번호를 매깁니다.
- `rows_of_line` 은 `Option` 이 아니라 빈 범위로 숨김 줄을 나타냅니다.
- 설계 835줄의 TS 근거 `monaco-actions.ts:747-827` 은 실제로 746-829 입니다.
- 범위 밖 수정 1건: `native/taide-native-ui/src/commands.rs` 에 `ShellIntent::FoldDocument { tab, command }` 변형과 import 한 줄이 들어갔습니다(앞선 작업자). 명령 레지스트리의 `Run::FoldDocument` 를 앱의 탭별 명령 큐로 넘기려면 `ShellIntent` 에 변형이 있어야 해서 불가피했고, 그 밖의 변경은 없습니다.
- 설계는 `editor-gutter.rs` 가 아이콘까지 그리는 것으로 읽히지만, 아이콘 레지스트리가 앱 크레이트에 있어 표면은 `FoldControl` 을 콜백으로 내보내고 앱이 그립니다. `taide-remote-web` 은 콜백을 주지 않습니다.

## 5. TS·Monaco 와 다른 점 (넘긴 것 포함)

1. Monaco 내장 기본 키가 연결되지 않았습니다. TS 에서는 Monaco 가 직접 처리하던 키입니다(`folding.js` 의 `kbOpts`): fold ⌥⌘[, unfold ⌥⌘], foldAll ⌘K ⌘0, unfoldAll ⌘K ⌘J, toggleFold ⌘K ⌘L, foldRecursively ⌘K ⌘[, unfoldRecursively ⌘K ⌘], toggleFoldRecursively ⌘K ⇧⌘L, foldAllExcept ⌘K ⌘-, unfoldAllExcept ⌘K ⌘=. native 에는 카탈로그의 `defaultBindingLabel` 을 실제 키로 바꾸는 경로가 없고(표시용), 편집기 문맥의 ⌘K 는 `native/taide-native-ui/src/keymap.rs` 가 편집기로 넘기기만 합니다. `keymap.rs` 는 이 단계의 수정 범위 밖이고, 접기만 따로 표면에 넣으면 사용자 재지정과 어긋나는 두 번째 키 경로가 생겨 넣지 않았습니다. 명령 팔레트와 사용자 지정 키로는 실행됩니다.
2. 접기 표식(region marker)과 offSide 규칙은 넣지 않았습니다(언어 구성 표 없음). offSide 언어에서는 빈 줄이 앞 블록에 붙는 방식이 달라질 수 있습니다. TS 에서 어느 언어에 offSide·marker 가 실제로 적용되는지는 확인하지 못했습니다.
3. LSP folding range, 수동 범위, import 접기는 비목표입니다.
4. 접힘 상태 영속화는 하지 않습니다. TS 는 Monaco view state(`editor.saveViewState()` 의 JSON, 접기 contribution 의 상태 포함 — `folding.js:118-151`)를 탭마다 `layout_set_view_state` 로 저장하고(`src/widgets/editor-pane/use-editor-view-state.ts`), 경로별로 메모리에도 둡니다(`src/entities/editor/model-registry.ts:173-184`). native 의 접힘은 `ViewState` 메모리에만 있습니다.
5. 시점 차이: Monaco 는 영역 계산을 지연 스케줄러로, 캐럿 진입 펼침을 200ms 뒤에 합니다(`folding.js:165-167`). native 는 같은 프레임에 합니다.
6. reveal 은 세로만 맞춥니다. Monaco 의 `revealPositionInCenterIfOutsideViewport` 는 가로도 맞춥니다. 선택이 바뀐 경우의 가로 맞춤은 기존 캐럿 reveal 이 처리합니다.
7. 편집으로 선택이 접힌 영역 안에 들어가 펼쳐졌는데 새 영역 목록에 같은 시작 줄이 없을 때, Monaco 는 그 영역을 다음 갱신까지 펼친 상태로 남기지만(`foldingRanges.js:277-283`) native 는 바로 버립니다. gutter 컨트롤 하나가 잠깐 남느냐의 차이입니다.

## 6. 실행한 명령과 결과

공통 인자: `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`. 표의 수치는 마지막 실행 값입니다.

| 검증 | 명령 | 결과 |
| --- | --- | --- |
| V1 | `cargo test --quiet --manifest-path native/taide-native-editor/Cargo.toml` | exit 0. 124 통과, 1 ignored, 실패 0. `tests/folding.rs` 16건, `tests/display-map.rs` 11건(1 ignored) 포함. 이 크레이트는 이번에 수정하지 않아 한 번만 실행 |
| V2 | `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --lib --test editor_surface` | exit 0. lib 117 통과(0.48s), editor_surface 67 통과(0.14s). 수정 전 65 → 테스트 2건 추가 |
| V3 | `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml` | exit 0. 오류 0. 경고는 `vendor/wry-preview` 의 기존 경고뿐. 재실행 때는 `--message-format short` 를 덧붙였음 |
| V3 | `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib` | exit 0. 368 통과, 실패 0(5.96s) |
| V3 | `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --test save --test save-syntax --test paste-shortcuts` | exit 0. 2 + 1 + 4 통과 |
| V4 | `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml` | exit 0. 출력 없음 |
| V5 | `cargo fmt --manifest-path <editor·ui·app Cargo.toml> -- --check` | 셋 다 exit 0 |
| 추가 | `cargo clippy --quiet` (editor `--all-targets`, ui `--lib --test editor_surface`) | editor 경고 0. ui 는 접기 코드와 무관한 기존 경고(too_many_arguments, collapsible_if 등)만 있음. 고치지 않음 |

V2·V3·V4 는 `editor_surface.rs` 수정 뒤 다시 실행했습니다. native 앱의 필터 없는 전체 테스트는 실행하지 않았습니다.

## 7. 실패했다가 고친 내역

- 새 F6 테스트가 처음에 드래그 선택에서 실패했습니다(기대 `(3, 44)`, 실제 `(45, 44)`). 원인은 구현이 아니라 테스트였습니다. 좌표를 재려고 `caret_offset` 을 편집기와 같은 `Context` 로 호출하면 편집기 없는 프레임이 끼어들어, 다음 프레임의 첫 누름이 직전 프레임 위젯 영역에서 편집기를 찾지 못합니다. 다른 접기 테스트처럼 측정 전용 `Context` 를 따로 두어 해결했습니다. 기대값은 바꾸지 않았습니다.
- 같은 테스트의 첫 컴파일 오류(`store` 동시 대여)는 revision 을 미리 꺼내 해결했습니다.
- `cargo fmt --check` 가 새 테스트 한 곳의 여는 중괄호 위치를 지적해 맞췄습니다.
- 앱 lib 테스트를 한 번 `-- --quiet` 를 겹쳐 붙여 실행해 인자 오류로 끝났고(테스트는 돌지 않음) 인자를 빼고 다시 실행했습니다.

## 8. 남은 위험과 실기 확인

- 실기 확인 필요: 접기 컨트롤의 크기·세로 정렬·fade, 접힌 줄 배경 농도, `⋯` 표식의 글리프(편집기 글꼴 체인에 U+22EF 가 없으면 대체 글꼴로 그려짐)와 간격. GUI 를 실행하지 않아 확인하지 못했습니다.
- Monaco 내장 기본 키 미연결(5절 1번). TS 사용자가 ⌥⌘[ 등으로 접던 동작이 native 에서는 키로 되지 않습니다.
- 영역 계산은 문서 revision 이나 탭 크기가 바뀔 때마다 표면에서 동기 실행합니다. 대형 등급은 접기가 꺼지지만 그 아래 크기의 큰 파일에서 입력마다 전체 줄을 훑습니다. `tests/folding.rs` 에 큰 문서의 계산 시간을 기록하는 테스트가 있으나, 실제 앱에서 입력 지연으로 느껴지는지는 실기로 확인하지 못했습니다.
- `FoldRegion` 캐시와 fade 상태는 표면의 프레임 상태에 있어 같은 문서를 두 뷰로 열면 뷰마다 따로 계산합니다.

## 9. 리뷰 후속

리뷰어가 올린 차단 항목은 1건이고, 실제 파일로 확인한 결과 옳은 지적이라 수정했습니다. Rust 코드·Cargo.toml·Cargo.lock 은 바꾸지 않았습니다.

### 9.1 항목 1 — `folding.rs` 의 출처·라이선스 고지 누락 (수정)

- 확인한 사실
  - `THIRD_PARTY_LICENSES.md` 의 "Ported source in `native/taide-native-editor`" 절에는 `src/line-tokens.rs` 한 건만 있었고 `folding` 이라는 낱말이 파일 어디에도 없었습니다(`grep -n "Ported source\|folding\|line-tokens\|LICENSE-MONACO-SNIPPET\|decoration"` 결과: 476·483·502·504·509줄만 일치).
  - `native/taide-native-editor/src/folding.rs` 는 Monaco 0.56.0 의 구조를 그대로 옮긴 포팅입니다. `indent_regions`·`indent_level` 은 `indentRangeProvider.js:26-90, 95-191` 과 `editor/common/model/utils.js:10-31`, `FoldingModel::merged` 는 `foldingRanges.js:48-71, 243-324` 와 `foldingModel.js:77-128`, `innermost_at`·`enclosing_at`·`inside`·`toggle` 과 `set_collapsed_*`·`toggle_at`·`*_fold_line` 은 `foldingRanges.js:154-186` 과 `foldingModel.js:20, 188-254, 256-357, 394-531`, `shown_selection`·`hidden_lines` 는 `hiddenRangeModel.js:30-63, 75-103`, `click`·`reveal_carets` 는 `folding.js:277-300, 347-406` 에 대응합니다.
  - `node_modules/monaco-editor/package.json` 은 version 0.56.0, license MIT 이고 위 JS 파일 머리말은 "Copyright (c) Microsoft Corporation. All rights reserved. Licensed under the MIT License" 입니다.
  - 전문은 `native/taide-native-editor/LICENSE-MONACO-SNIPPET` 에 이미 있습니다(MIT, Copyright (c) 2016 - present Microsoft Corporation). 설계 문서 668줄이 들여쓰기 접기를 MIT 고지 대상으로 적고 이 파일의 선례를 따르라고 지시합니다.
- 수정: `THIRD_PARTY_LICENSES.md` 의 같은 절에 `src/folding.rs` 항목을 추가했습니다. Monaco Editor 0.56.0, MIT, Copyright (c) Microsoft Corporation, 옮긴 JS 파일과 함수 이름, 전문 위치를 적었습니다. 리뷰어가 든 이름에 더해 실제 코드가 따르는 `RangesCollector.insertFirst`, `computeIndentLevel`, `FoldingRegions.ensureParentIndices`·`findRange`, `FoldingModel.update`·`getAllRegionsAtLine`·`getRegionAtLine`, `getParentFoldLine`·`getPreviousFoldLine`·`getNextFoldLine`, `HiddenRangeModel.updateHiddenRanges`, `FoldingController.onEditorMouseUp`·`revealCursor` 도 넣었습니다. 고지에 적은 이름은 모두 Monaco 소스에서 grep 으로 존재를 확인했습니다.
- 범위 밖 수정 사유: `THIRD_PARTY_LICENSES.md` 는 이 단계의 수정 범위 목록에 없지만 지적 대상이 이 파일 자체라 불가피했습니다. 추가한 것은 목록 항목 하나(19줄)뿐입니다.
- 실패하는 테스트로 재현하지 않은 이유: 고지 문서의 누락이라 동작 결함이 아니고, native 쪽에는 이 파일을 읽는 테스트·빌드 입력이 없습니다. 저장소에서 이 파일을 읽는 것은 `src/shared/lib/theme-convert/bundled-theme-licenses.test.ts` 하나이며 번들 테마 id 의 포함 여부와 `TAIDE ships (\d+) color themes` 문장만 봅니다. 추가한 항목은 두 조건에 닿지 않습니다. 이 TS 테스트는 허용 명령 밖(bun)이라 실행하지 않았습니다.

### 9.2 고지에 넣지 않은 것

- `native/taide-native-ui` 의 접기 코드(`editor-gutter.rs`, `editor-geometry.rs`, `editor-paint.rs`, `editor_surface.rs`)는 Monaco 의 수치와 규칙(`folding.css`, `foldingDecorations.js`, `editorOptions.js`, `folding.js:301-346` 의 누름 판정)을 근거로 native 방식으로 다시 쓴 코드라 소스 포팅 항목으로 적지 않았습니다. 리뷰 지적과 설계 668줄의 대상도 "들여쓰기 접기" 포팅입니다. 이 판단을 바꿔야 하면 `native/taide-native-ui` 절을 새로 만들어야 합니다.
- `display-map.rs` 의 `merged_line_ranges` 는 일반적인 구간 병합이고 지적 대상이 아니라 적지 않았습니다.

### 9.3 실행한 명령과 결과

| 명령 | 결과 |
| --- | --- |
| `grep -n "Ported source\|folding\|line-tokens\|LICENSE-MONACO-SNIPPET\|decoration" THIRD_PARTY_LICENSES.md` (수정 전) | exit 0. 476·483·502·504·509줄만 일치. `folding` 없음 |
| `grep -n` 으로 `indentRangeProvider.js`·`foldingRanges.js`·`foldingModel.js`·`hiddenRangeModel.js` 의 클래스·함수 선언 나열 | exit 0. 고지에 적은 이름이 모두 있음 |
| `rg -n "FoldingController = \|MAX_LINE_NUMBER = "` (`folding.js`, `foldingRanges.js`) | exit 0. `folding.js:47` `FoldingController`, `foldingRanges.js:11` `MAX_LINE_NUMBER = 0xFFFFFF` |
| `grep -n "\"version\"\|\"license\"" node_modules/monaco-editor/package.json` | exit 0. 0.56.0, MIT |
| `rg -n "THIRD_PARTY_LICENSES"` (crates, native, src-tauri, src, scripts, package.json) | exit 0. 파일을 읽는 코드는 위 TS 테스트 하나뿐 |
| `git diff -- THIRD_PARTY_LICENSES.md` (수정 후) | exit 0. 이번 추가는 `src/folding.rs` 항목 19줄. 같은 diff 의 `grammar-registrations.rs` 7줄은 앞선 단계의 변경 |
| `git diff --check -- THIRD_PARTY_LICENSES.md` | exit 0. 출력 없음(공백 오류 없음) |

V1~V5 는 다시 실행하지 않았습니다. 이번 수정이 Markdown 두 파일뿐이고 어느 크레이트의 빌드·테스트 입력도 아니어서 영향받는 검증 명령이 없습니다. 6절의 수치는 이 문서의 구현 단계 시점 값이며, 그 뒤 다른 단계가 바꾼 코드까지 포함한 현재 작업 트리의 결과는 이 후속에서 확인하지 않았습니다.
