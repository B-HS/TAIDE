# Native 배치 6 단계 2/3 — 편집기 장식·앵커 기반 (2026-10-07)

## 0. 먼저 알아야 할 것 — 본문 x 좌표가 바뀝니다

이 단계부터 편집기 gutter 폭이 Monaco 식으로 계산됩니다. 본문·캐럿·선택·IME 후보 창·줄 번호의 x 좌표가 모두 달라집니다.

| 화면 | 이전 본문 시작 x | 이후 본문 시작 x |
| --- | --- | --- |
| 줄 번호 켬 | `"000"` 의 폭 + 패딩 8 × 2 | `round(자릿수 × 숫자 최대 폭)` + 줄 장식 10 (+ 접기 16) |
| 줄 번호 끔 | 패딩 8 | 줄 장식 10 (+ 접기 16) |
| 줄 번호의 오른쪽 끝 | 본문 시작 − 8 | 줄 번호 폭의 오른쪽 끝(패딩 없음) |

- 앱(`show_document`)은 TS 와 같이 일반 파일에서 접기 영역 16 을 예약합니다(`folding = !largeFile`). 글꼴 14 기준 예: 이전 41.28 → 이후 51.
- 기본 presentation(테스트, 브라우저 클라이언트 `taide-remote-web`)은 접기가 꺼져 있어 줄 장식 10 만 더합니다. 글꼴 14 기준 41.28 → 35.
- 줄 사이 장식·위젯의 실제 소비자(진단, Git, 찾기, LSP)는 연결하지 않았습니다. 앱 화면에서 달라지는 것은 gutter 폭뿐입니다.

상태: D1~D5 구현 완료. 검증 계약 V1~V6 은 최종 작업 트리에서 모두 exit 0 입니다. GUI 는 실행하지 않았습니다.

기준 시점은 HEAD `62f698fe` 이고 앞 단계(플러그인 문법)의 미커밋 변경 위에서 작업했습니다. 경로는 저장소 루트 기준입니다. `MONACO` 는 `node_modules/monaco-editor/esm/vs`, `ES` 는 `native/taide-native-ui/src/editor_surface.rs`, `APP` 은 `native/taide-native-app/src/application.rs` 입니다.

## 1. 한눈에 보는 결과

| 항목 | 결과 |
| --- | --- |
| D1 장식 모델 | `native/taide-native-editor/src/decoration.rs` 신규. 층·범위·종류·revision 태그, Monaco stickiness 규칙으로 저널 따라가기. 순수 테스트 8건 |
| D2 층 렌더 | 줄 배경, 범위 배경, 물결 밑줄(직접 그림), 전경색·직선 밑줄(galley 섹션), lane 표식. UI 테스트 6건 |
| D3 gutter | `editor-gutter.rs` 신규. Monaco 식 폭, 줄 번호 오른쪽 정렬, lane 표식. UI 테스트 1건과 기존 기대값 갱신 |
| D4 좌표 질의·배치기 | `EditorGeometry::{caret_rect, range_rects, byte_at}`, `editor-overlay.rs` 의 `place_overlay`. UI 테스트 1건, 배치기 순수 테스트 3건 |
| D5 입력 경로 | `EditorRequest` 와 `NativeEditor::show_request`. 기존 `show` 계열 서명과 소비처는 그대로 |
| clippy `too_many_arguments` | 시작 8건 → 끝 8건(늘지 않음) |
| 테스트 수 | 편집기 크레이트 97 → 105, UI `editor_surface` 40 → 53, UI lib 116, 앱 lib 367, remote-web 53 |

## 2. 기준 확인

### 2.1 gutter 폭과 줄 번호

| 대상 | 확인한 사실 | 근거 |
| --- | --- | --- |
| 폭 식 | `lineNumbersWidth = round(max(자릿수, lineNumbersMinChars) × maxDigitWidth)`(줄 번호가 꺼지면 0), `lineDecorationsWidth` 에 `folding && showFoldingControls !== 'never'` 이면 16 을 더함, `glyphMarginWidth` 는 glyphMargin 이 꺼지면 0, `decorationsLeft = lineNumbersLeft + lineNumbersWidth`, `contentLeft = decorationsLeft + lineDecorationsWidth` | `MONACO/editor/common/config/editorOptions.js:1220-1239` |
| TS 설정값 | `glyphMargin: false`, `lineNumbersMinChars: 3` | `src/features/editor/code-editor.tsx:102, 185-186` |
| lineDecorationsWidth | TS 가 지정하지 않음. Monaco 기본값 10 | `editorOptions.js:1479-1481` |
| folding | TS 가 `folding: !largeFile` 로 지정. `largeFile` 은 `tier` 가 `large` 또는 `readOnly`, 제목 없는 문서는 false | `code-editor.tsx:271`, `src/widgets/editor-pane/editor-pane.tsx:422`, `src/widgets/editor-pane/untitled-pane.tsx:200` |
| showFoldingControls | TS 가 지정하지 않음. 기본값 `mouseover`(never 가 아니므로 16 을 예약) | `editorOptions.js:3357` |
| 자릿수 | 문서 줄 수의 십진 자릿수 | `MONACO/editor/browser/config/editorConfiguration.js:137-144, 170-177` |
| maxDigitWidth | 숫자 0~9 폭의 최댓값. 읽은 폭이 2 이하이면 5 로 올림 | `MONACO/editor/browser/config/fontMeasurements.js:76-95, 113-138` |
| 줄 번호 위치 | `left: lineNumbersLeft; width: lineNumbersWidth` 인 상자에 `text-align: right`, 패딩 없음. 이어지는 표시 줄에는 번호 없음 | `MONACO/editor/browser/viewParts/lineNumbers/lineNumbers.js:83-87, 158`, `lineNumbers.css:6-15` |
| TS 의 CSS 재정의 | `src/shared/styles/global.css` 에 `.line-numbers`·`.margin` 재정의 없음 | `grep` 결과 0건 |

### 2.2 층 순서와 장식 렌더

| 대상 | 확인한 사실 | 근거 |
| --- | --- | --- |
| 본문 영역 순서 | 현재 줄 강조 → 선택 → 들여쓰기 가이드 → 장식(`DecorationsOverlay`) → 공백 표식 → (rulers, view zone) → 본문 줄 → content widget → 캐럿. DOM 순서가 곧 그리는 순서이고 `view-overlays`·`view-lines` 에 z-index 가 없음 | `MONACO/editor/browser/view.js:129-135, 174-181`, `MONACO/editor/browser/widget/codeEditor/editor.css:64-72` |
| gutter 순서 | 현재 줄 여백 강조 → margin 줄 장식 → 줄 장식 lane → 줄 번호 | `view.js:136-141` |
| 장식 정렬 | zIndex → className → 범위 시작. 줄 전체 장식을 먼저, 범위 장식을 나중에 | `MONACO/editor/browser/viewParts/decorations/decorations.js:63-90` |
| 줄 전체 장식 | 본문 영역의 `left:0; width:100%`. 모델 줄의 모든 뷰 줄에 놓임 | `decorations.js:93-111`, `MONACO/editor/common/viewModel/inlineDecorations.js:98-106` |
| 범위 장식 | 표시 줄마다 범위의 x 구간 rect. 같은 className 이 겹치거나 맞닿으면 하나로 합침 | `decorations.js:112-183` |
| 물결 밑줄의 소속 | 진단은 `className: squiggly-*` 장식입니다. 즉 `DecorationsOverlay` 가 그리며 본문 줄보다 아래 층입니다 | `MONACO/editor/common/services/markerDecorationsService.js:171-237` |
| 물결 모양 | 6 × 3 px SVG 를 줄 상자 아래쪽에 `repeat-x bottom left` 로 반복. 다각형 3개가 45도 지그재그(아래 꼭짓점 x 1.75, 위 꼭짓점 x 4.75, 주기 6, 두께 약 1)를 이룸 | `MONACO/editor/browser/widget/codeEditor/codeEditorWidget.js:1864-1867, 1877` |
| lane 장식 | `left: decorationsLeft; width: decorationsWidth; height: 100%` 인 상자를 범위의 뷰 줄마다 둠. 같은 className 은 한 줄에 한 번만 | `MONACO/editor/browser/viewParts/linesDecorations/linesDecorations.js:54-93`, `linesDecorations.css:15-18`, `MONACO/editor/browser/viewParts/glyphMargin/glyphMargin.js:52-97` |
| TS 의 Git 막대 | lane 상자의 `left: 0; width: 3px; top: 0; bottom: 0` | `src/shared/styles/global.css:371-392, 424-446` |
| TS 의 삭제 삼각형 | `top: -4px; left: 0; border-width: 4px 0 4px 6px`. 꼭짓점은 (lane 왼쪽, 줄 위 −4), (lane 왼쪽 + 6, 줄 위), (lane 왼쪽, 줄 위 + 4) | `global.css:394-408` |
| TS 가 만드는 장식 | conflict 는 `className` + `isWholeLine`, Git·conflict lane 은 `linesDecorationsClassName` + `isWholeLine`. stickiness 는 지정하지 않음 | `src/widgets/editor-pane/use-editor-git-gutter-and-conflicts.ts:207-225` |

### 2.3 편집 시 범위 이동

| 대상 | 확인한 사실 | 근거 |
| --- | --- | --- |
| 이동 규칙 | `nodeAcceptEdit`. 시작 경계는 `AlwaysGrows`·`GrowsOnlyWhenTypingBefore` 에서, 끝 경계는 `NeverGrows`·`GrowsOnlyWhenTypingBefore` 에서 앞 글자에 붙음. 삭제가 있으면 삭제 시작 위치의 경계는 제자리, 바꾸기에서는 공통 길이 안의 경계가 제자리, 그 뒤 경계는 새 글자 끝으로 이동. 시작이 끝을 넘으면 끝을 시작에 맞춤 | `MONACO/editor/common/model/intervalTree.js:226-306` |
| 적용 순서 | 한 번의 편집 묶음을 내용 변경 순서대로 `acceptReplace(offset, length, textLength, forceMoveMarkers)` | `MONACO/editor/common/model/textModel.js:1123-1126` |
| 기본 stickiness | 지정하지 않으면 `AlwaysGrowsWhenTypingAtEdges`(0) | `textModel.js:1928` |
| 내장 장식의 stickiness | 진단·찾기·단어 강조는 `NeverGrowsWhenTypingAtEdges`(1) | `markerDecorationsService.js:228`, `MONACO/editor/contrib/find/browser/findDecorations.js:254-300`, `MONACO/editor/contrib/wordHighlighter/browser/highlightDecorations.js:34-78` |
| forceMoveMarkers | `EditOperation.insert`·`replaceMove` 와 일부 ReplaceCommand 만 true. 일반 입력은 false | `MONACO/editor/common/core/editOperation.js:12, 31`, `MONACO/editor/common/commands/replaceCommand.js:116-124` |
| native 저널 | span 은 편집 전 좌표의 오름차순이고 삽입 길이는 `new_end_byte - start_byte` | `native/taide-native-editor/src/change-journal.rs:142-184`, `native/taide-native-editor/tests/change-journal.rs:365-379` |

### 2.4 content widget 배치

| 대상 | 확인한 사실 | 근거 |
| --- | --- | --- |
| 뷰포트 안 배치 | 위는 `앵커 위 − 높이`, 아래는 `앵커 위 + 앵커 높이`. `위쪽 여유 >= 높이` 면 위에 맞고 `뷰포트 높이 − 아래 위치 >= 높이` 면 아래에 맞음. 가로는 오른쪽을 넘으면 당기고 그다음 왼쪽 아래로 내려가지 않게 함 | `MONACO/editor/browser/viewParts/contentWidgets/contentWidgets.js:213-234` |
| 편집기 밖으로 넘치는 배치 | 창 기준 위 여백 22, 아래 여백 22. 가로 하한은 `max(15, 편집기 왼쪽 − 폭)`, 상한은 `min(편집기 오른쪽 + 폭, 창 폭 − 15)` | `contentWidgets.js:235-282` |
| 선호 순서 | 1차로 선호 순서에서 처음 맞는 쪽, 없으면 2차로 첫 선호. EXACT 는 앵커 위치 그대로 | `contentWidgets.js:361-409` |
| 제품 위젯의 방식 | AI 인라인 편집 입력과 Monaco 내장 완성·rename·parameter hints 는 모두 `allowEditorOverflow: true`. `allowOverflow` 기본값 true, `fixedOverflowWidgets` 기본값 false | `src/features/editor/ai-inline-edit.ts:128-136`, `MONACO/editor/contrib/suggest/browser/suggestWidget.js:833`, `MONACO/editor/contrib/rename/browser/renameWidget.js:61`, `MONACO/editor/contrib/parameterHints/browser/parameterHintsWidget.js:58`, `editorOptions.js:3046, 3211` |

## 3. 항목별 변경 내용

### D1. 장식 모델 (`native/taide-native-editor`)

| 파일 | 내용 |
| --- | --- |
| `src/decoration.rs` (신규) | `UnderlineKind { Straight, Squiggly }`, `Underline { kind, color }`, `InlineStyle { foreground, background, underline }`, `LaneMark { Bar, DeletedTriangle }`, `DecorationKind { Inline, LineBackground, Lane { mark, color } }`, `Stickiness`(Monaco 4종, 기본값 `AlwaysGrowsWhenTypingAtEdges`), `Decoration { bytes, kind, stickiness }`, `DecorationLayer`(`new`·`revision`·`z_order`·`items`·`intersecting`·`apply`·`tracking`) |
| `src/lib.rs` | `pub mod decoration` |
| `tests/decoration.rs` (신규) | 순수 테스트 8건 |

- 색은 RGBA 바이트입니다(egui 비의존).
- `DecorationLayer::new` 는 항목을 시작 바이트 오름차순으로 정렬합니다. `intersecting(bytes)` 는 시작이 범위 끝 이하인 항목까지를 이진 탐색으로 자르고, 그 안에서 끝이 범위 시작 이상인 항목만 돌려줍니다(맞닿은 것 포함).
- `apply(&ChangeSet)`: 층의 revision 이 변경의 `revision_before` 와 같을 때만 적용하고 `revision_after` 로 올립니다. 다르면 아무것도 바꾸지 않고 false 입니다. span 은 뒤에서부터 적용합니다(Monaco 의 내림차순 적용과 같음). 경계 이동은 `moved_marker` 가 `nodeAcceptEdit` 를 그대로 옮긴 것입니다(`forceMoveMarkers = false`, `collapseOnReplaceEdit = false`).
- `tracking(ChangesSince)`: 변경이 없으면 빌린 층 그대로(`Cow::Borrowed`), 변경이 있으면 복제해 차례로 적용한 층, 저널이 끊겼거나(`Lagged`) revision 이 이어지지 않으면 `None` 입니다. `None` 이 무효화입니다.
- 이동은 단조라서 정렬 순서가 유지됩니다(같은 시작끼리의 순서만 달라질 수 있습니다).

### D2. 표면의 층 렌더 (`native/taide-native-ui`)

| 파일 | 내용 |
| --- | --- |
| `src/editor-paint.rs` | `FrameDecoration { z_order, bytes, lines, kind }`, `frame_decorations`, `color32`, `Layers.decorations`, `Layers::{line_backgrounds, range_overlays, range_overlay, squiggle}`. `Layers::row` 의 순서 변경. 줄 번호 그리기는 `editor-gutter.rs` 로 이동 |
| `src/editor-row-text.rs` | `RowInlineStyle { foreground, underline }`, `RowText.underline_colors`, `RowText::decorate`, 비공개 `split_section`. `styled_layout_job` 이 장식 밑줄색을 우선 |
| `src/editor-geometry.rs` | `RowLayout.decorations`, `RowLayout::row` 가 인라인 전경색·직선 밑줄을 `RowText` 에 반영, `Row::extent` |
| `src/editor_surface.rs` | `tracked_layers`(저널 따라가기와 z 순서 정렬), 보이는 줄 범위로 `frame_decorations` 호출 |

한 표시 줄의 그리는 순서(바뀐 뒤):

1. 현재 줄 강조
2. 선택 영역(모든 선택)
3. 줄 배경(줄 전체 장식, z 순서)
4. 범위 배경과 물결 밑줄(범위 장식, z 순서)
5. 캐럿
6. 본문 galley(토큰 색, 장식 전경색, 직선 밑줄 포함)
7. lane 표식
8. 줄 번호

- 프레임마다 `tracked_layers` 가 층별로 `store.changes_since(문서, 층.revision)` 을 읽어 현재 revision 까지 옮깁니다. 따라갈 수 없는 층은 그리지 않습니다. 같은 프레임의 입력으로 문서가 바뀌어도 그 프레임에 맞는 위치에 그려집니다.
- `frame_decorations` 는 보이는 문서 줄의 바이트 범위와 닿는 항목만 모으고, 경계를 문서 길이로 자른 뒤 문자 시작으로 내립니다(바꾸기 편집에서 경계가 여러 바이트 문자의 중간에 남을 수 있기 때문입니다). 줄 전체 장식용으로 시작·끝 줄을 함께 계산합니다.
- 줄 배경: 범위의 시작 줄부터 끝 줄까지의 모든 표시 줄에 본문 폭 전체(`text_rect` 의 x 범위)로 그립니다. 가로 스크롤과 무관합니다.
- 범위 배경: 표시 줄마다 `Row::extent` 로 범위와 표시 줄 조각의 교집합을 x 구간으로 바꿉니다. 탭은 펼친 공백 전체, 이어지는 줄은 들여쓰기 뒤부터, 범위가 다음 표시 줄로 이어지면 표시 줄 글자의 끝까지입니다. 폭이 0 이면 그리지 않습니다(빈 줄 포함. Monaco 도 findMatch 외에는 줄바꿈 폭을 더하지 않습니다).
- 병합: 같은 z 순서에서 배경색과 물결색이 모두 같은 범위 장식을 x 시작순으로 정렬해, 겹치거나 맞닿으면 한 구간으로 합쳐 그립니다(Monaco 의 className 병합).
- 물결 밑줄: 줄 상자 아래 3 높이의 띠에 폭 1 의 꺾은선 하나를 그리고 띠로 clip 합니다. 꼭짓점은 구간 왼쪽 기준 1.75(아래), 4.75(위)에서 시작해 반 주기 3 간격으로 번갈아 놓입니다. SVG 다각형은 이 중심선을 가진 폭 약 1 의 선을 3 높이로 자른 모양입니다.
- 전경색·직선 밑줄: `RowText::decorate` 가 표시 문자 구간의 경계에서 섹션을 나누고 전경색과 밑줄색만 바꿉니다. 토큰의 굵기·기울임·취소선은 남습니다. 직선 밑줄은 `TextFormat.underline`(폭 1)이며 토큰 밑줄보다 장식 밑줄색이 우선합니다.
- lane 표식: 범위의 줄에 속한 모든 표시 줄에 그립니다. 막대는 lane 왼쪽 3 폭에 줄 높이 전체, 삭제 삼각형은 줄 위 경계에 걸친 가로 6·세로 8 의 삼각형입니다. 같은 모양·색은 표시 줄마다 한 번만 그립니다.
- "접힌 표시 줄": 지시의 이 표현은 wrap 으로 나뉜 표시 줄로 해석했습니다. 숨김 줄(접기)은 `DisplayMap` 에 아직 없습니다(단계 7). 장식은 보이는 표시 줄의 조각과 교집합으로만 그리므로, 숨김 줄이 생기면 그 줄의 장식은 그려지지 않습니다.

### D3. gutter (`native/taide-native-ui/src/editor-gutter.rs`, 신규)

- `Gutter::measure(painter, 줄 수, appearance, has_folding)`: 2.1 의 식 그대로입니다. 상수는 `LINE_NUMBERS_MIN_CHARS = 3`, `LINE_DECORATIONS_WIDTH = 10`, `FOLDING_CONTROLS_WIDTH = 16` 입니다. 숫자 최대 폭은 `"0123456789"` 를 한 번 레이아웃해 글리프 전진 폭의 최댓값을 씁니다. 2 이하이면 5 로 올립니다.
- `Gutter::width()` 가 본문 시작 오프셋입니다. `Gutter::paint_row` 가 lane 표식과 줄 번호를 그립니다. 줄 번호는 `줄 번호 폭` 의 오른쪽 끝에 오른쪽 정렬합니다.
- `has_folding` 은 `EditorDisplayOptions.folding` 입니다. 표면(`show_request`), reveal(`reveal_tokenized`), wrap 폭 계산(`Projection`)이 같은 값을 씁니다.
- 기존 `editor-geometry.rs` 의 `gutter_width` 는 삭제했습니다.
- `APP` 5634-5641: `editor_presentation.options.folding = !matches!(tier, Large | ReadOnly)`. TS 의 `folding: !largeFile` 입니다. 지금 이 옵션이 하는 일은 gutter 의 접기 영역 폭 예약뿐입니다.

### D4. 좌표 질의와 오버레이 배치기

| 파일 | 내용 |
| --- | --- |
| `src/editor-geometry.rs` | `EditorGeometry.rows`(그 프레임의 표시 줄 정보, `Arc<[Row]>`, 크레이트 내부 필드), `caret_rect(byte)`, `range_rects(bytes)`, `byte_at(position)`. `Row` 에 `Debug`·`Clone`·`PartialEq` |
| `src/editor-overlay.rs` (신규) | `OverlayPreference { Exact, Above, Below }`, `OverlayBounds { Viewport(Rect), Page { editor, window } }`, `OverlayPlacement { position, preference }`, `place_overlay(anchor, size, preferences, bounds)` |
| `src/lib.rs` | `mod editor_gutter`, `pub mod editor_overlay` |

- `caret_rect(byte)`: 그 바이트가 놓인 표시 줄이 화면에 걸쳐 있으면 x 폭 0, y 는 줄 상자 전체(줄 위 ~ 줄 위 + 줄 높이)인 rect 입니다. Monaco 의 앵커(위, 왼쪽, 줄 높이)와 같습니다. wrap 경계의 바이트는 뒷 표시 줄 시작입니다. 그려지지 않은 줄, 화면 아래로 완전히 벗어난 overscan 줄, 문서 밖은 `None` 입니다. 가로로 화면 밖이어도 값을 돌려줍니다(배치기가 당깁니다).
- `range_rects(bytes)`: 범위 배경과 같은 계산으로 표시 줄마다 rect 하나입니다.
- `byte_at(position)`: 본문 rect 안의 위치만 받습니다. 그 y 의 표시 줄에서 가까운 문자 경계의 문서 바이트입니다(탭은 가까운 쪽 끝). gutter·화면 밖·줄이 없는 곳은 `None` 입니다.
- `place_overlay`: 앵커 rect(x 왼쪽, y 위·아래를 씀), 위젯 크기, 선호 순서, 경계를 받아 왼쪽 위 좌표와 선택된 쪽을 돌려줍니다. `Viewport` 는 `_layoutBoxInViewport`, `Page` 는 `_layoutBoxInPage`(창 여백 22·15)입니다. 선호가 비면 `None` 입니다. 화면 좌표계 하나로 옮겼습니다.

### D5. 장식 입력 경로

- `EditorRequest<'a, Keymap, Route, Tokens> { request_focus, keymap, route, presentation, tokens, decorations }` 와 `NativeEditor::show_request(ui, store, view, request)` 를 추가했습니다. 인자 5개입니다.
- `show_tokenized` 는 `decorations: &[]` 인 요청을 만들어 `show_request` 로 위임합니다. `show`, `show_with_keymap`, `show_with_input_route`, `show_presented`, `show_tokenized`, `reveal`, `reveal_presented`, `reveal_tokenized` 의 서명은 그대로입니다. 앱과 `taide-remote-web` 의 호출부는 바꾸지 않았습니다.
- 장식은 토큰처럼 함수로 받지 않고 빌린 층 목록으로 받습니다. 그 프레임의 입력 뒤 revision 까지는 표면이 저널로 옮기기 때문입니다.

## 4. 바뀐 기존 UI 테스트 기대값

`native/taide-native-ui/tests/editor_surface.rs`. 글꼴은 monospace 14(숫자 전진 폭 약 8.4287), 화면 800 × 200, 기본 presentation(접기 꺼짐)입니다. 새 값은 실행 전에 식으로 계산했고 실행 결과와 같았습니다.

| 기대값 | 이전 | 이후 | 근거 |
| --- | --- | --- | --- |
| 본문 시작 x(본문·캐럿·본문 clip·IME 캐럿) | 41.28 = `"000"` 폭 25.28 + 8 × 2 | 35.00 = round(3 × 8.4287) 25 + 10 | `editorOptions.js:1226-1238`, 1481 |
| 한 자리 줄 번호의 x | 24.84 = 41.28 − 8 − 8.44 | 16.56 = 25 − 8.44 | `lineNumbers.js:158`, `lineNumbers.css:10` |
| 두 자리 줄 번호의 x | 16.44 = 41.28 − 8 − 16.84 | 8.16 = 25 − 16.84 | 같음 |
| `SELECTION_FRAME` 2행 선택 시작 x | 83.28 = 41.28 + 42 | 77.00 = 35 + 42 (같은 문자 인덱스 5) | 본문 시작 이동 |
| `SELECTION_FRAME` 4행 선택 끝·IME 캐럿 x | 125.56 = 41.28 + 84.28 | 119.28 = 35 + 84.28 | 같음 |
| `REVEALED_VIEW`·`SCROLLED_COMPOSITION_TRACE` 의 가로 스크롤 | 85.16 = 842.88 + 1 − (800 − 41.28) | 78.88 = 842.88 + 1 − (800 − 35) | 본문 폭 765 |
| `SCROLLED_COMPOSITION_TRACE` 가로 스크롤바 | 112.28..746.28 | 102.00..747.00 | track [35, 786] 길이 751, slider floor(765 × 751 / 890.59) = 645, 위치 round(78.875 × 106 / 125.59) = 67 |
| 식으로 계산하는 gutter(5곳) | `measure("000") + PADDING * 2.0` | `gutter_width()` = round(3 × 숫자 폭) + 10 | 테스트 도우미 `line_numbers_width`, `gutter_width` |

- 스크롤된 본문 x(−43.88), 캐럿 x(799.00), 세로 스크롤바 rect 는 그대로입니다(본문 오른쪽 끝 기준이라 gutter 와 무관).
- 선택 범위(14..50, 70, 148)와 문서 내용 기대값은 바뀌지 않았습니다.
- 식을 쓰는 곳: `scrollbar는_가로_scroll을_…`, `기본_presentation의_show_presented는_…`, `큰_글꼴에서_…`, `wrap_columns` 도우미(wrap 테스트 8건이 사용), `reveal은_토큰의_굵은_글꼴로_…`.
- 기존 테스트의 본문 로직은 건드리지 않았습니다. `PADDING` 상수는 `EditorAppearance.horizontal_padding` literal 에 남아 있습니다.

## 5. 설계 문서·지시와 달라진 점

| 번호 | 설계·지시 | 실제 | 이유 |
| --- | --- | --- | --- |
| 1 | 지시 D2·설계 4.3: 순서가 "장식 → 텍스트 → 밑줄" | 물결 밑줄은 본문보다 먼저(범위 배경과 같은 층) 그립니다. 직선 밑줄은 본문 galley 의 일부입니다 | Monaco 에서 진단 밑줄은 `className` 장식이고 `DecorationsOverlay` 는 본문 줄보다 앞 DOM 입니다(2.2). 설계의 순서는 이 부분만 실제 소스와 다릅니다. 물결은 줄 상자 아래 3 높이라 글자와 겹치는 것은 descender 일부뿐입니다. 9절 1번 |
| 2 | 설계 4.3: `DecorationLayer { owner, revision, z_order, items }` 공개 필드 | `owner` 없음. 필드는 비공개이고 `new` 가 정렬 | 표면은 호출자가 준 목록 순서와 `z_order` 만 씁니다. 읽는 곳이 없는 식별자는 넣지 않았습니다. 정렬 불변식을 지키려고 필드를 닫았습니다 |
| 3 | 설계 4.3: `InlineStyle` 의 italic·bold·strikethrough·opacity, `UnderlineKind::Dotted`, `Before`·`After` 주입 텍스트, `OverviewMark`, `hover`, `Lane.is_clickable` | 넣지 않음 | 이 단계가 그리는 것(줄 배경, 인라인 배경·전경, 직선·물결 밑줄, lane 표식)만 담았습니다. 나머지는 각 소비 단계(진단 10, minimap·overview 12, 주입 텍스트 13, conflict 클릭)에서 그리는 코드와 함께 추가해야 죽은 필드가 되지 않습니다 |
| 4 | 설계 4.3: `Decoration` 에 stickiness 없음 | `Decoration.stickiness` 추가 | Monaco 는 장식마다 stickiness 를 갖고(2.3), TS 장식(기본값 Always)과 진단·찾기(Never)가 다릅니다. 지시 D1 의 요구입니다 |
| 5 | 설계 4.3: "표면은 보이는 바이트 범위를 이진 탐색" | 위쪽 경계만 이진 탐색, 아래쪽은 선형 걸러내기 | 항목의 끝은 정렬돼 있지 않습니다. 층 하나당 프레임마다 앞쪽 항목 수만큼 비교합니다(11절 5번) |
| 6 | 설계 4.9: `EditorPresentation<'a>` 에 `layers` 필드, `show_presented(.., &presentation)` | `EditorRequest` 와 `show_request` | 직전 배치가 같은 이유(기존 테스트의 `EditorPresentation` literal)로 토큰을 별도 진입 함수로 넣었습니다. 지시 D5 가 요청 구조체로 묶도록 했습니다 |
| 7 | 설계 4.4: `top_right_anchor()` | 넣지 않음. 대신 `byte_at` 추가 | 지시 D4 는 문서 위치와 화면 rect 의 양방향 질의입니다. 우상단 앵커는 찾기 위젯(단계 6) 전용이고, 설계가 인용한 위치 식(`findWidget.js:589-602`, 이번에 확인하지 않음)과 함께 정해야 합니다 |
| 8 | 설계 4.4: 배치기 입력은 경계 rect 하나 | `OverlayBounds` 두 가지(뷰포트, 창) | 제품 위젯은 전부 `allowEditorOverflow: true` 라 실제로 쓰이는 식은 `_layoutBoxInPage` 입니다(2.4). 둘 다 옮겼습니다 |
| 9 | 설계 7절: gutter 는 "줄 번호 폭 + 줄 장식 10 + 접기 16" | 접기 16 은 `options.folding` 일 때만. 앱이 `!largeFile` 을 넘김 | Monaco 식의 조건입니다(2.1). 기본 presentation 은 "전부 꺼짐"(설계 4.8)이라 10 만 더합니다 |
| 10 | 수정 범위: `application.rs` 는 서명 변화에 따른 최소 수정 | 서명은 바뀌지 않았고, 접기 영역 폭을 위해 `folding` 을 넘기는 4줄을 추가 | 앱 화면의 본문 x 를 TS 와 맞추려면 필요합니다. 9절 2번 |
| 11 | 설계 7절: 지금의 gutter 는 ES 1029-1049 | 실제 위치는 `editor-geometry.rs:121-139`(배치 4 에서 이동) | 서술 내용(`줄 번호 폭 + 패딩 8 × 2`)은 맞았습니다 |
| 12 | 설계 4.3: Monaco 가 범위를 옮긴다는 서술에 파일 근거 없음 | `intervalTree.js:226-306`, `textModel.js:1123-1126` 을 근거로 삼음 | 2.3 |
| 13 | (설계에 없음) | `Layers::row` 가 선택을 모두 그린 뒤 장식, 그다음 캐럿을 그림 | 이전에는 선택마다 선택 rect 와 캐럿을 번갈아 그렸습니다. 장식 배경이 캐럿을 가리지 않게 하고 Monaco 순서(선택 → 장식)를 맞추려면 나눠야 합니다. 선택이 하나인 화면의 도형 순서는 같습니다(기존 스냅샷 테스트 통과) |

설계가 인용한 `view.js:114-167`, `editorOptions.js:1220-1239`, 3.3 의 `global.css` 줄 번호는 실제 파일과 일치했습니다.

## 6. 실행한 명령과 실제 결과

cargo 명령은 모두 `--locked --offline --target-dir experiments/native-shell-spike/target` 을 붙였습니다(`cargo fmt` 제외). 출력이 길어 일부는 `--quiet` 를 더했습니다(같은 테스트 집합, 출력 형식만 다름).

| 순서 | 명령 | 결과 |
| --- | --- | --- |
| 1 | `git status --short`, `git diff --stat` (시작) | 앞 단계의 미커밋 변경만 있음. 이 단계 범위 파일 중 변경은 `application.rs`(앞 단계 5줄)뿐 |
| 2 | `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml --lib` (기준선) | exit 0. 경고 11, 그중 `too_many_arguments` 8(`editor_surface.rs` 3건: `reveal_tokenized` 8/7, `show_presented` 8/7, `show_tokenized` 9/7) |
| 3 | `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test decoration` (D1 구현 뒤 첫 실행) | exit 0. 8 통과 |
| 4 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` (배치기 구현 전) | exit 101. `unresolved import taide_native_ui::editor_overlay`(실패하는 테스트 확인) |
| 5 | `cargo check --manifest-path native/taide-native-ui/Cargo.toml --lib` (구현 뒤) | exit 0 |
| 6 | 4번 명령 (gutter 변경 뒤, 기대값 갱신 전) | exit 101. 43건 중 28 통과, 15 실패. 실패는 전부 gutter 좌표 기대값(4절). 출력된 값이 미리 계산한 값과 같았음. 배치기 3건 통과 |
| 7 | 4번 명령 (기대값 갱신 뒤) | exit 0. 43 통과 |
| 8 | 4번 명령 (장식·gutter·좌표 테스트 추가 뒤) | exit 101. 52건 중 50 통과, 2 실패(7절 1·2번) |
| 9 | 4번 명령 (수정 뒤) | exit 0. 52 통과 |
| 10 | `cargo fmt … -- --check` (편집기, UI, 앱) | 편집기 exit 1(신규 테스트 3곳), UI exit 1(소스 3곳, 테스트 13곳), 앱 exit 0 → 손으로 맞춘 뒤 편집기·UI exit 0 |
| 11 | `cargo clippy --manifest-path native/taide-native-editor/Cargo.toml --all-targets` | exit 0. 경고 3(신규 테스트의 `single_range_in_vec_init`) → 단언을 고친 뒤 경고 0 |
| 12 | V1 `cargo test --manifest-path native/taide-native-editor/Cargo.toml` (최종) | exit 0. 105 통과, 1 ignored. 가장 긴 대상 display-map 0.24초 |
| 13 | V2 `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --test editor_surface` (최종) | exit 0. lib 116(0.48초), editor_surface 53(0.12초) |
| 14 | V3 `cargo check --manifest-path native/taide-native-app/Cargo.toml` | exit 0. 경고는 기존 `vendor/wry-preview` 17건뿐 |
| 15 | V3 `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` (최종) | exit 0. 367 통과, 0 실패(5.94초) |
| 16 | V3 `cargo test --manifest-path native/taide-native-app/Cargo.toml --test save --test save-syntax --test paste-shortcuts` (최종) | exit 0. paste-shortcuts 2, save 1, save-syntax 4(3.99초) |
| 17 | V4 `cargo check --manifest-path native/taide-remote-web/Cargo.toml` (최종) | exit 0 |
| 18 | V4 `cargo test --manifest-path native/taide-remote-web/Cargo.toml --features inspection` (최종) | exit 0. 53 통과(대상 18개, 가장 긴 것 0.16초) |
| 19 | V5 `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml --lib --test editor_surface` (최종) | exit 0. lib 경고 11(`too_many_arguments` 8, 기준선과 같음). 테스트 대상 경고 0 |
| 20 | V6 `cargo fmt --manifest-path <편집기·UI·앱 Cargo.toml> -- --check` (최종) | 모두 exit 0 |
| 21 | `git status --short` (끝) | 이 단계가 바꾼 파일은 10절 목록과 같음. `Cargo.toml`·`Cargo.lock` 은 건드리지 않음 |

- 12~20번은 서식 수정과 마지막 테스트 추가까지 끝난 작업 트리에서 실행했습니다. V3·V4 는 그 전에도 한 번 실행했고 결과가 같았습니다(앱 lib 367, 통합 2·1·4, remote-web 53).
- 앱의 필터 없는 전체 테스트와 앱 크레이트의 clippy 는 실행하지 않았습니다(지시).

## 7. 신규 테스트와 실패했다가 고친 내역

신규 테스트:

- `native/taide-native-editor/tests/decoration.rs` (8건)
    - `가장자리_입력은_stickiness에_따라_범위를_늘리거나_민다`, `빈_범위에서의_입력은_stickiness에_따라_글자를_품거나_앞뒤에_남는다`, `삭제는_겹친_부분만_줄이고_범위를_덮으면_삭제_시작의_빈_범위로_남긴다`, `바꾸기는_공통_길이_안의_경계를_제자리에_두고_그_뒤의_경계는_새_글자_끝으로_옮긴다`: stickiness 4종 × 편집 위치. 기대값은 `nodeAcceptEdit` 를 손으로 따라가 정했습니다
    - `한_transaction의_여러_편집은_각_범위가_같은_글자를_가리키게_옮긴다`: span 4개, 여러 바이트 문자, 줄 삽입
    - `undo와_redo의_span으로도_범위가_글자를_따라간다`
    - `저널을_따라가는_층은_변경이_없으면_빌린_그대로이고_뒤처지거나_어긋나면_무효다`: `Borrowed`·`Owned`, revision 이 이어지지 않는 `apply`, 저널 상한 초과, 없는 revision
    - `층은_시작_바이트_순으로_정렬되고_범위와_닿는_장식만_돌려준다`
- `native/taide-native-ui/tests/editor_surface.rs` (13건)
    - 배치기 3건: `오버레이는_선호_순서에서_처음_맞는_쪽에_놓이고_어느_쪽도_맞지_않으면_첫_선호에_놓인다`, `오버레이의_가로_위치는_경계_안으로_당기고_exact_선호는_앵커_위치를_그대로_쓴다`, `편집기_밖으로_넘칠_수_있는_오버레이는_창의_여백을_기준으로_뒤집고_당긴다`
    - `장식_층은_선택과_본문_사이에_그리고_gutter의_lane_표식은_줄_번호_앞에_그린다`: 도형 순서 전체, rect·색, 물결 띠와 꼭짓점, 삼각형 꼭짓점, 전경색·직선 밑줄 섹션, 글자 위치 불변
    - `겹치거나_맞닿은_같은_모양의_범위_장식은_합치고_같은_lane_표식은_줄마다_한_번_그린다`
    - `장식_범위는_탭_전각_문자_wrap_줄에서_글자의_x_범위에_놓이고_줄_장식은_모든_표시_줄에_놓인다`: 탭 2개, 전각 문자, 이어지는 줄 들여쓰기, wrap 경계의 `caret_rect`, `range_rects`
    - `스크롤된_화면의_범위_장식은_글자와_함께_움직이고_줄_장식은_본문과_gutter_폭에_남는다`
    - `뒤처진_장식_층은_그_프레임의_편집까지_저널로_옮겨_그리고_따라갈_수_없는_층은_그리지_않는다`
    - `문자_중간의_장식_경계는_문자_시작으로_내리고_문서_끝을_넘으면_문서_끝에서_자른다`
    - `gutter_폭은_줄_번호와_줄_장식과_접기_폭의_합이고_줄_번호는_그_폭의_오른쪽에_맞춘다`: 기본, 접기, 네 자리 줄 수, 줄 번호 끔, wrap 열, reveal
    - `좌표_질의는_보이는_표시_줄에서_문서_위치와_화면_rect를_서로_바꾼다`: 모든 문자 경계의 왕복, gutter·화면 밖, overscan 줄
    - `장식이_없는_요청은_기존_평문_화면과_같고_보이지_않는_줄의_장식은_도형을_더하지_않는다`: `show_request` 가 기존 `PLAIN_FRAME` 과 같음
    - `표시_줄_장식은_구간을_경계에서_나누어_전경색과_밑줄색만_바꾸고_글꼴_스타일은_남긴다`

실패했다가 고친 내역:

1. 범위 장식 병합. 처음 구현은 목록에서 이웃한 항목끼리만 합쳤습니다. 테스트(같은 층에 배경과 물결 밑줄이 섞인 경우)가 실패했습니다. Monaco 는 className 으로 먼저 정렬하므로 같은 모양끼리는 사이에 다른 장식이 있어도 합쳐집니다(`decorations.js:63-80`). 구현을 "같은 z 순서와 같은 모양으로 묶어 x 순으로 합침"으로 고쳤습니다. 기대값은 바꾸지 않았습니다.
2. wrap 장식 테스트의 문서. 끊을 곳이 없는 `x` 연속을 넣어 두 표시 줄로 나뉜다고 가정했는데, 줄 나눔 규칙은 전각 문자 뒤에서 먼저 끊어 세 줄이 됐습니다. 테스트 문서를 단어 반복으로 바꾸고 표시 줄 수를 `create_line_breaks` 결과에서 읽게 했습니다. 구현과 단언 내용은 그대로입니다.
3. `cargo fmt -- --check` 가 지적한 곳과, rustfmt 가 여는 중괄호를 다음 줄로 내리는 긴 테스트 이름 5개를 손으로 고쳤습니다.
4. clippy 가 신규 편집기 테스트의 `[3..5]` 형태 단언 3곳을 지적했습니다. 상수와 단일 항목 비교로 바꿨습니다.

실패하는 테스트부터 작성했는지:

- 배치기: 테스트를 먼저 쓰고 컴파일 실패를 확인한 뒤 구현했습니다(6절 4번).
- 장식 모델: 테스트 파일을 구현보다 먼저 썼지만 실패 상태로는 실행하지 않았습니다. 구현 뒤 첫 실행에서 8건이 통과했습니다.
- UI 장식·gutter·좌표 테스트: 표면 구현 뒤에 썼습니다. 그 과정에서 1번의 구현 오류가 드러났습니다.

테스트 기대값을 구현에 맞춰 바꾼 것은 없습니다. 4절의 기존 기대값은 Monaco 식으로 다시 계산한 값입니다.

## 8. TS 와 다르게 남는 점

| 항목 | TS(Monaco) | native | 영향 |
| --- | --- | --- | --- |
| 물결의 모양 | SVG 다각형(가장자리 anti-alias) | 폭 1 의 꺾은선을 3 높이 띠로 clip | 꼭짓점 주변의 픽셀이 조금 다를 수 있습니다. 실기 확인 대상 |
| hint 의 점 3개 밑줄 | 12 × 3 SVG 를 한 번만(`no-repeat`) | 모델에 없음 | 진단 단계(10)에서 추가해야 합니다. `codeEditorWidget.js:1869-1872, 1892` |
| 진단의 테두리·배경 | `editorError.border`·`editorError.background` 등이 있으면 이중 밑줄·배경 | 없음 | 해당 색을 정의한 테마에서만 다릅니다. `editor.css:81-116` |
| 직선 밑줄 | 글꼴이 정한 위치·두께 | 글리프 상자 아래, 폭 1 | 큰 글꼴에서 가늘 수 있습니다(배치 5 QA 7절과 같은 한계) |
| `forceMoveMarkers` | 일부 편집은 경계를 강제로 밀어냄 | 저널에 그 정보가 없어 항상 false | 붙여넣기 등으로 장식 경계 위치에 넣을 때 범위가 늘어나는지 여부가 다를 수 있습니다 |
| `collapseOnReplaceEdit` | 장식이 통째로 바뀌면 접는 옵션 | 없음 | TS 는 이 옵션을 쓰지 않습니다 |
| 바꾸기의 공통 길이 | UTF-16 단위 | 바이트 단위. 경계가 문자 중간에 남으면 그릴 때 문자 시작으로 내림 | 비 ASCII 를 포함한 바꾸기에서 경계가 한 글자 안쪽으로 다를 수 있습니다 |
| undo·redo | 편집 목록대로 이동 | 전후 문서의 차이 span 하나로 이동 | 떨어진 여러 곳을 한 번에 고친 편집을 되돌리면 그 사이의 장식 범위가 뭉개집니다. 공급자가 다시 계산하기 전까지입니다(설계 4.3 이 받아들인 한계) |
| `showIfCollapsed`, `shouldFillLineOnLineBreak`, findMatch 의 줄바꿈 폭 | 있음 | 없음 | 찾기 단계(6)에서 필요하면 추가합니다 |
| gutter 클릭 | 줄 번호 클릭은 줄 선택, lane 클릭은 `GUTTER_LINE_DECORATIONS` 대상 | 이전과 같이 gutter 클릭도 본문 클릭으로 처리 | 이 단계의 비목표입니다 |
| 현재 줄 강조의 폭 | `renderLineHighlight: line` 은 본문만 | 이전과 같이 gutter 까지 | 기존 동작. 렌더 옵션 단계(8) |
| 숫자 폭 보정 | 반각·전각·공백·숫자 중 하나라도 2 이하이면 넷 다 5 로 올림 | 숫자 폭만 보고 올림 | 글꼴을 읽지 못한 예외 상황에서만 다릅니다 |
| 브라우저 클라이언트 | (해당 없음) | 접기 영역 없이 줄 번호 + 10 | `taide-remote-web` 은 기본 presentation 을 씁니다 |
| 접기 컨트롤 | 접기 영역에 마우스를 올리면 표시 | 폭만 예약 | 접기 단계(7) |

## 9. 메인 판단이 필요한 사항

1. 물결 밑줄을 본문 앞에 그린 점(5절 1번). 지시의 순서 표기("본문, 밑줄")와 다르지만 Monaco 의 실제 구조입니다. 본문 뒤로 옮기려면 `Layers::row` 에서 `range_overlays` 의 물결 부분만 `text` 뒤로 옮기면 됩니다.
2. `application.rs` 에 `folding = !largeFile` 을 넘긴 점(5절 10번). 빼면 앱의 본문 x 가 TS 보다 16 왼쪽이 되고, 접기 단계에서 다시 16 이동합니다.
3. `EditorAppearance.horizontal_padding` 이 더는 배치에 쓰이지 않습니다. 필드와 유효성 검사(음수·NaN 거절)는 그대로 뒀습니다. literal 13곳 중 2곳이 동결된 크레이트의 테스트라 지우지 않았습니다.
4. `editor_surface.rs` 의 `too_many_arguments` 3건은 남아 있습니다. 서명을 유지하라는 지시 때문입니다. 소비처를 `show_request` 로 옮기면 없앨 수 있습니다.
5. 설계의 자료형에서 뺀 필드들(5절 2·3번). 다음 소비 단계가 필요할 때 추가하는 쪽으로 판단했습니다.

## 10. 변경 파일

| 파일 | 구분 |
| --- | --- |
| `native/taide-native-editor/src/decoration.rs` | 신규, 범위 안 |
| `native/taide-native-editor/src/lib.rs` | 범위 안 |
| `native/taide-native-editor/tests/decoration.rs` | 신규 테스트 |
| `native/taide-native-ui/src/editor-gutter.rs` | 신규, 범위 안 |
| `native/taide-native-ui/src/editor-overlay.rs` | 신규, 범위 안 |
| `native/taide-native-ui/src/editor-paint.rs` | 범위 안 |
| `native/taide-native-ui/src/editor-geometry.rs` | 범위 안 |
| `native/taide-native-ui/src/editor-row-text.rs` | 범위 안 |
| `native/taide-native-ui/src/editor_surface.rs` | 범위 안 |
| `native/taide-native-ui/src/lib.rs` | 범위 안 |
| `native/taide-native-ui/tests/editor_surface.rs` | 범위 안 |
| `native/taide-native-app/src/application.rs` | 범위 안(서명 외 수정, 9절 2번) |
| `docs/quality-assurance/2026-10-06-native-batch6-decorations-anchors.md` | 이 문서 |

`Cargo.toml`·`Cargo.lock`, `taide-remote-web`, `taide-native-syntax`, `crates/`, `src/`, 설계 문서는 건드리지 않았습니다. 범위 밖 파일 수정은 없습니다.

## 11. 남은 위험과 실기 확인

남은 위험:

1. gutter 폭 변화로 화면 전체의 본문 위치가 바뀝니다. 글꼴 크기·줄 수(자릿수)·줄 번호 설정·대형 파일 여부의 조합은 테스트가 식으로만 확인했습니다.
2. 줄 수가 자릿수 경계(999 → 1000 등)를 넘으면 본문이 한 숫자 폭만큼 움직입니다. Monaco 와 같은 동작이지만 wrap 이 켜져 있으면 표시 줄이 다시 나뉩니다.
3. `EditorOutput.geometry` 가 그 프레임의 galley 를 들고 있습니다. 좌표 계산에만 쓰므로 atlas 재생성과는 무관하지만, 호출자가 프레임을 넘겨 보관하면 그동안 메모리가 유지됩니다.
4. 뒤처진 층은 프레임마다 복제해 옮깁니다. 공급자가 갱신하지 않은 채 항목이 많은 층(찾기 일치 수만 건)을 오래 두면 프레임당 비용이 생깁니다. 공급자가 `DecorationLayer::apply` 로 직접 따라가면 복제가 없습니다.
5. `intersecting` 은 앞쪽 항목을 선형으로 봅니다. 문서 끝 쪽을 볼 때 층 하나당 항목 수만큼 비교합니다. 측정하지 않았습니다.
6. 저널 상한(64번)을 한 프레임 사이에 넘기면 층이 그려지지 않습니다. 공급자가 새 층을 줄 때까지입니다.
7. 물결 밑줄이 많은 화면(수십 줄 전체에 밑줄)의 꺾은선 tessellation 비용은 측정하지 않았습니다. 구간당 도형은 하나입니다.
8. 삭제 삼각형은 윗줄 영역으로 4 만큼 넘어갑니다. 첫 표시 줄에서는 편집기 clip 에 잘립니다(Monaco 와 같음).

실기 확인이 필요한 것:

- [ ] 본문·줄 번호의 가로 위치가 TS 화면과 같은지(글꼴 13 기본, 줄 번호 켬·끔, 1000줄 이상 파일, 대형 파일)
- [ ] 줄 번호가 gutter 왼쪽에 붙어 잘리지 않는지, 본문과 줄 번호 사이 간격(26, 대형 파일 10)이 TS 와 같은지
- [ ] word wrap 을 켠 상태에서 wrap 위치가 TS 와 같은지(본문 폭이 달라졌습니다)
- [ ] 클릭·드래그 선택·IME 후보 창 위치가 글자와 맞는지
- [ ] 파일 열기·검색 결과 이동(reveal) 뒤 가로 스크롤 위치가 맞는지
- [ ] 브라우저 클라이언트의 편집기 화면(본문이 약 6 왼쪽으로 이동)
- [ ] (소비자가 연결된 뒤) 물결 밑줄의 모양·두께, Git 막대와 삼각형의 위치, 줄 배경과 선택 영역이 겹칠 때의 색

## 12. 테스트 부채

- [ ] 앱 프레임 수준의 gutter 확인. 재현 조건: 실제 `show_document` 프레임에서 일반 파일과 대형 파일의 본문 x 단언. 생략 이유: 앱 프레임을 GUI 없이 돌리는 장치가 없습니다. 표면의 `folding` 옵션은 UI 테스트가 다룹니다. 필요한 시점: 접기 단계(7)에서 옵션 배선을 다시 건드릴 때.
- [ ] 다중 선택이 있는 표시 줄의 도형 순서. `Layers::row` 가 선택과 캐럿을 나눠 그리게 바뀌었는데 다중 선택 화면의 순서를 고정하는 테스트는 원래 없었습니다. 남은 위험: 낮음(선택 rect 와 캐럿은 서로 가리지 않습니다). 필요한 시점: 다중 커서 입력 경로를 넣을 때.
- [ ] 비 ASCII 바꾸기 뒤 문자 중간에 남은 경계를 저널 경로로 재현하는 테스트. 지금은 표면의 보정(문자 시작으로 내림)만 직접 확인했습니다. 필요한 시점: 진단 단계에서 한글 식별자 rename 등으로 어긋남이 보고될 때.
- [ ] 대형 층(수만 항목)의 프레임 비용 측정. 필요한 시점: 찾기 단계(6).
- [ ] 저널 상한을 넘는 undo 연타 뒤 층 무효화의 화면 확인. 순수 테스트로는 `tracking` 이 `None` 인 것까지 확인했습니다.
