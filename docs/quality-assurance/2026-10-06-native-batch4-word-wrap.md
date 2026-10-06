# Native 배치 4 단계 3 — word wrap (2026-10-06)

상태: W1~W6 구현과 검증 계약(V-E, V-U, V-UL, V-A, V-AL, V-W, V-F)이 전부 exit 0 입니다. 화면이 바뀌는 단계입니다(탭이 열 중간에 있을 때의 폭, wrap 을 켠 화면 전체). 설계 문서·지시와 다르게 구현한 점 14건은 3절에, 메인 판단이 필요한 사항은 7절에 있습니다. GUI 실행은 하지 않았습니다.

경로는 저장소 루트(`/Users/hyunseokbyun/development/TAIDE`) 기준입니다. `MONACO` 는 `node_modules/monaco-editor/esm/vs`, `EPAINT` 는 `/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/epaint-0.36.2/src`, `ROPEY` 는 같은 registry 의 `ropey-1.6.1/src`, `ES` 는 `native/taide-native-ui/src/editor_surface.rs`, 설계 문서는 `docs/research/2026-10-06-native-editor-display-layer-design.md` 입니다.

## 1. 기준 확인

직접 읽어 확인한 소스입니다.

| 대상 | 확인한 사실 | 근거 |
| --- | --- | --- |
| TS 가 넘기는 wrap 옵션 | `wordWrap: wordWrap ? 'on' : 'off'` 하나뿐. `wordWrapColumn`·`wrappingIndent`·`wrappingStrategy`·`wordBreak` 는 넘기지 않음. 언어별 예외 없음 | `src/features/editor/code-editor.tsx:282-301`, `src/shared/lib/code-editor-settings.ts:47`, `src` 전체에서 `wordWrap\|wrappingIndent\|wrappingStrategy\|wordWrapColumn` 검색 결과 |
| 설정 기본값 | `editorWordWrap` 기본 false(TS `?? false`, Rust `editor_word_wrap: false`) | `code-editor-settings.ts:47`, `crates/taide-model/src/settings.rs:127, 547` |
| Monaco 기본값 | wrappingIndent `Same`, wrappingStrategy `simple`, wordBreak `normal`, wordWrapColumn 80(`on` 에서는 쓰이지 않음), wrapOnEscapedLineFeeds false | `MONACO/editor/common/config/editorOptions.js:2899, 1319, 3415, 3456, 3467` |
| wrap 열 | `on` 이면 `max(1, floor((contentWidth - verticalScrollbarWidth - 2) / typicalHalfwidthCharacterWidth))` | `editorOptions.js:1248-1250, 1276-1285` |
| 줄바꿈 문자 집합 | break-before·break-after 목록 | `editorOptions.js:3450-3455` |
| 줄 나눔 알고리즘 | 첫 문자 뒤부터 순회하며 후보(`canBreak`)를 기억하고, 열이 넘치면 후보 또는 넘친 문자 앞에서 나눔. 후보가 이어지는 줄 폭(`wrappedLineBreakColumn`)을 넘으면 버림 | `MONACO/editor/common/viewModel/monospaceLineBreaksComputer.js:300-400` |
| 문자 분류 | 256 이상은 가나(3040-30FF)·한자(3400-4DBF, 4E00-9FFF) 범위를 목록보다 먼저 `BREAK_IDEOGRAPHIC` 으로 분류. 한글은 분류 없음 | `monospaceLineBreaksComputer.js:48-75` |
| 문자 열 폭 | 탭은 `tabSize - (visibleColumn % tabSize)`, 전각은 `columnsForFullWidthChar`, 32 미만 제어 문자도 `columnsForFullWidthChar`, surrogate pair 는 분류 없는 2열 한 단위 | `monospaceLineBreaksComputer.js:340-348, 355-364, 401-413` |
| 전각 열 폭 | `typicalFullwidthCharacterWidth / typicalHalfwidthCharacterWidth`. 반각 기준 문자 `n`, 전각 기준 문자 U+FF4D. 폭이 2 이하로 읽히면 5 로 대체 | `monospaceLineBreaksComputer.js:26`, `MONACO/editor/browser/config/fontMeasurements.js:76-96, 110-111, 167-168` |
| 전각 범위 | 2E80-D7AF, F900-FAFF, FF01-FF5E, FFE0-FFE6 | `MONACO/base/common/strings.js:497-541` |
| 금칙 | 공백 앞에서는 나누지 않음. break-after 연속의 끝, break-before 연속의 시작, 한자·가나 앞뒤(닫는 문자 앞·여는 문자 뒤 제외) | `monospaceLineBreaksComputer.js:440-446` |
| 이어지는 줄 들여쓰기 | 선행 공백·탭의 열 폭(Indent 는 탭 1개, DeepIndent 는 2개 추가). `들여쓰기 + 전각 열 폭 > wrap 열` 이면 0 | `monospaceLineBreaksComputer.js:447-470` |
| 뷰 줄 내용과 열 | 이어지는 뷰 줄 = 들여쓰기만큼의 공백 + 조각. 최소 열 = 들여쓰기 + 1. 시작 visible column = 앞 break 의 visible column | `MONACO/editor/common/viewModel/modelLineProjection.js:49-71, 76-83, 117-133`, `MONACO/editor/common/modelLineProjectionData.js:55-76` |
| 위치 → 뷰 줄 | affinity 없음·Right 는 break 오프셋을 뒷 줄 시작으로, Left 는 앞 줄 끝으로 | `modelLineProjectionData.js:116-153`, `MONACO/editor/common/viewModel/viewModelLines.js:602` |
| 캐럿의 뷰 위치 유지 | 뷰 상태로 옮긴 캐럿은 모델 위치가 같으면 뷰 줄을 유지(`validateViewPosition`) | `MONACO/editor/common/cursor/oneCursor.js:84-108` |
| 화살표·Page 이동 단위 | `cursorUp/Down/PageUp/PageDown` 은 `Unit.WrappedLine`. page 크기 `max(1, floor(height / lineHeight) - 2)` | `MONACO/editor/browser/coreCommands.js:571-686`, `MONACO/editor/common/cursorCommon.js:51` |
| 세로 이동 | 뷰 줄 내용의 visible column + leftover 로 목표 열을 구하고 뷰 줄의 최소·최대 열로 고정. 첫·마지막 위치에서는 leftover 0 | `MONACO/editor/common/cursor/cursorMoveOperations.js:132-174`, `cursorCommon.js:142-153` |
| 좌우 이동 | 왼쪽은 Left, 오른쪽은 Right affinity 로 정규화한 뒤 한 글자 이동. 오른쪽 이동이 wrap 경계에 닿으면 윗 줄 끝에 머묾 | `cursorMoveOperations.js:54-74, 116-131`, `modelLineProjectionData.js:154-175` |
| Home | 이어지는 뷰 줄에서 뷰 줄의 첫 비공백이 아니면 뷰 줄 기준(첫 비공백, 없으면 최소 열), 그 외에는 모델 줄 기준 | `MONACO/editor/common/cursor/cursorMoveCommands.js:52-71`, `cursorMoveOperations.js:261-274` |
| End | 뷰 줄 끝이거나 모델 줄의 마지막 뷰 줄이면 모델 줄 끝, 아니면 뷰 줄 끝. `cursorEnd` 의 sticky 는 false | `cursorMoveCommands.js:80-99`, `coreCommands.js:847-850` |
| 탭 렌더 | 탭은 `tabSize - (visibleColumn % tabSize)` 개의 공백(NBSP). 전각은 2열. visible column 은 `startVisibleColumn` 에서 시작하고 들여쓰기 공백은 세지 않음 | `MONACO/editor/common/viewLayout/viewLineRenderer.js:764, 845-852, 880-883, 905-907` |
| 탭 안쪽 hit-test | 가까운 쪽 경계, 같으면 왼쪽 | `viewLineRenderer.js:154-203` |
| 줄 번호 | 뷰 줄의 1열이 모델 1열이 아니면 번호 없음 | `MONACO/editor/browser/viewParts/lineNumbers/lineNumbers.js:83-87` |
| 선택의 줄 끝 폭 | 다음 뷰 줄이 다른 모델 줄일 때만 줄 끝 폭을 더함(wrap 경계에서는 더하지 않음) | `MONACO/editor/browser/viewParts/viewLines/viewLines.js:353-360` |
| wrap 일 때 가로 내용 폭 | `maxLineWidth` 그대로(끝 여백·스크롤바 폭 없음) | `MONACO/editor/common/viewLayout/viewLayout.js:207-229` |
| ropey 줄 끝 | `cr_lines` 기능으로 LF·CRLF·CR 을 줄 끝으로 봄. chunk 는 CRLF 를 가르지 않고 역방향 순회 가능 | `native/taide-native-editor/Cargo.toml`, `ROPEY/iter.rs:1265-1273, 1510-1536` |
| egui glyph 폭 | galley 의 `Glyph.advance_width` 가 셰이핑된 전진 폭. 글리프 x 는 물리 픽셀 정수 좌표(`physical_x`)에서 만든 값이라 전진 폭의 합과 다를 수 있음(테스트에서 공백 4개 뒤 글리프 x 33.0, 같은 문자열의 끝 33.72 로 관찰) | `EPAINT/text/text_layout.rs:196-211`, `EPAINT/text/text_layout_types.rs:886-895` |

## 2. 항목별 변경

### W1. 줄바꿈 계산 (`taide-native-editor`)

- 신규 `native/taide-native-editor/src/line-breaks.rs`
    - `WrappingIndent { None, Same, Indent, DeepIndent }`, `WrapSettings { wrap_column, tab_size, full_width_columns, wrapping_indent }`, `LineBreakData { break_offsets, break_offsets_visible_column, wrapped_text_indent_length }`. `break_offsets` 는 줄 안 UTF-8 바이트 오프셋이고 마지막 값은 줄 길이입니다(Monaco 는 UTF-16 오프셋).
    - `create_line_breaks(settings, text)`: Monaco `createLineBreaks` 이식. 나눌 필요가 없으면 `None`.
    - `WrappingCharacterClassifier`(비공개): ASCII 256칸 표 + 나머지 `HashMap`. break-before 를 먼저, break-after 를 나중에 넣고(겹치면 after 우선), 256 이상은 가나·한자 범위를 목록보다 먼저 봅니다. 문자열 두 개는 `editorOptions.js:3452, 3455` 와 바이트 단위로 같은지 `rg -F` 로 대조했습니다.
    - `is_full_width_character`, `tab_columns(visible_column, tab_size)`(공개. 표면의 탭 확장이 같은 식을 씀), `character_columns`, `can_break`, `wrapped_text_indent_length`.
    - `fits_by_byte_length`: 줄의 바이트 수 + 탭마다 `tab_size - 1` 이 wrap 열 이하이고 제어 문자가 없으면 순회 없이 `None` 을 돌려줍니다. 전각 열 폭이 3(전각 문자의 UTF-8 바이트 수) 이하일 때만 적용합니다. 그 조건에서 모든 문자의 열 폭이 바이트 수 이하이므로 결과는 전체 순회와 같습니다. Monaco 에는 없는 최적화입니다.
- `native/taide-native-editor/src/lib.rs`: `line_breaks` 모듈 연결(`#[path]`).
- 이식하지 않은 것: `createLineBreaksFromPreviousLineBreaks`(3절 6항), `wordBreak: keepAll`, `wrapOnEscapedLineFeeds`, 주입 텍스트. TS 가 설정하지 않고 Monaco 기본값이 꺼짐입니다.
- 테스트: 신규 `native/taide-native-editor/tests/line-breaks.rs` 9건. 기대값은 Monaco 소스의 규칙을 손으로 따라가 구했습니다(강제 나눔, break-after·before, 공백, 탭 정지, 전각·한자·한글, 측정된 전각 폭 1.5, 금칙, 가나 범위 우선, surrogate pair, 제어 문자, wrappingIndent 4종과 들여쓰기 0 복귀).

### W2. 표시 줄 매핑 (`taide-native-editor`)

- `native/taide-native-editor/src/display-map.rs`
    - `DisplayMap::build(document, wrap)`: wrap 이 `None` 이면 identity 와 같은 값. `Some` 이면 문서 줄마다 줄바꿈 자료(`Vec<Option<Box<LineBreakData>>>`)와 문서 줄의 첫 표시 줄 누적합(`first_rows`)을 만듭니다.
    - `refresh(document)`: revision 이 다르면 갱신합니다. wrap 상태에서는 보관한 rope 와 새 rope 의 공통 앞·뒤 바이트(chunk 단위 비교)로 바뀐 문서 줄 범위를 구해 그 줄만 다시 계산하고 누적합은 그 줄부터 다시 셉니다. 편집과 undo·redo 가 같은 경로입니다.
    - `wrap_settings()`, `rows_of_line(line)`, `row_start_column(row)`(이어지는 줄의 시작 visible column), `row_of_head(document, byte, at_row_end)`(wrap 경계 바이트를 윗 줄 끝으로 볼지 선택).
    - `segment`, `row_of_byte` 는 서명 그대로 wrap 을 반영합니다. `row_of_byte` 는 wrap 경계 바이트를 뒷 줄로 돌려줍니다(Monaco 의 affinity 없음).
    - `DisplayMap` 은 `f64` 를 담게 되어 `Eq` derive 를 뺐습니다(`PartialEq` 유지).
- 무효화
    - 편집·undo: `refresh`(증분).
    - 폭·글꼴·탭 크기·줄 번호 표시 변경: 표면이 매 프레임 `WrapSettings` 를 다시 계산해 보관한 값과 다르면 전체를 다시 만듭니다(ES `Projection::map`).
- 보관 위치: `ViewState.display: Option<Arc<DisplayMap>>`, `EditorStore::take_display`·`set_display`(3절 4항). 표면은 프레임 시작에 꺼내고 끝에 되돌립니다.
- 테스트(`tests/display-map.rs` 신규 5건 + `#[ignore]` 측정 1건)
    - 정확한 조각·들여쓰기·시작 열·경계 바이트, 7개 wrap 열에서의 덮음·왕복·줄 폭 불변식, 문자·CRLF 중간을 가르는 편집 8종의 갱신, 무작위 편집·undo·redo 400회 × 2열의 갱신 = 전체 계산.
    - 측정: `큰_문서의_wrap_전체_계산과_편집_뒤_갱신에_걸린_시간을_기록한다`(5만 줄, 신규 5건에 포함), `수십만_줄_문서의_...`(30만 줄, `#[ignore]`). 갱신 결과가 전체 계산과 같은지만 단언하고 시간 단언은 없습니다.

측정값(합성 문서: 10줄마다 약 230자 줄, 나머지는 탭으로 시작하는 약 45자 줄, wrap 열 100, 이 작업 장비(macOS)에서 최종 작업 트리로 1회 실행):

| 문서 | 빌드 | 전체 계산 | 한 글자 편집 뒤 갱신 |
| --- | --- | --- | --- |
| 50,001줄, 3.26MB, 표시 줄 60,001 | release | 6.5ms(다시 계산 6.0ms) | 0.12ms |
| 50,001줄 | debug | 96.4ms | 0.75ms |
| 300,001줄, 20.0MB, 표시 줄 360,001 | release | 31.0ms(다시 계산 28.2ms) | 0.64ms |
| 300,001줄 | debug | 554ms | 4.2ms |

- 5만 줄은 release 에서 한 프레임(16.7ms) 안입니다. 30만 줄 전체 계산은 약 2프레임이며 파일을 wrap 상태로 처음 그릴 때와 폭·글꼴·탭 크기가 바뀐 프레임에만 일어납니다. 편집은 증분이라 1ms 미만입니다. 설계 4.5 의 프레임 분할 대비책은 넣지 않았습니다(7절 1항).

### W3. 탭 정지 표시 (`taide-native-ui`)

- `native/taide-native-ui/src/editor-row-text.rs`
    - `RowColumns { tab_size, start_column, indent_columns }`, `RowText::expanded(source, columns, foreground)`. 들여쓰기 공백을 앞에 두고, 탭은 `floor(tab_size - visible % tab_size)` 개의 공백으로 바꿉니다. visible column 은 `start_column` 에서 시작하고 전각·surrogate pair 는 2열입니다(`viewLineRenderer.js` 의 계산).
    - `model_bytes`: 들여쓰기 공백은 0, 탭이 만든 공백은 탭의 바이트. `indent_chars` 필드 추가.
    - `model_byte(display_char)`: 들여쓰기 안쪽은 조각 시작, 탭 안쪽은 가까운 쪽 경계(같으면 왼쪽).
    - `display_char(model_byte)`: 그 바이트로 시작하는 첫 표시 문자(탭이면 첫 공백, 조각 시작이면 들여쓰기 뒤). 문자 중간 바이트는 그 문자(기존 내림 의미). 공백이 0개인 탭의 바이트는 앞 문자 뒤입니다.
    - `RowText::plain` 은 호출부가 없어져 제거했습니다.
- wrap 이 꺼진 화면의 변화: 탭이 열 중간에 있으면 다음 탭 정지까지만 넓어집니다(이전에는 항상 공백 4개 폭). 탭 크기가 4 가 아닌 파일도 그 크기를 따릅니다. 열 0 의 탭(탭 크기 4)은 픽셀 위치가 그대로입니다(`PLAIN_FRAME` 의 폭 143.28, 선택 x 83.28 불변).

### W4. 표시 줄 기준 이동 (`taide-native-editor`)

- `native/taide-native-editor/src/view.rs`: `WrapAffinities { revision, heads_at_row_end }`, `ViewState.wrap_affinities`, `ViewState::head_at_row_end(selection, revision)`. revision 이 다르면 무시됩니다.
- `native/taide-native-editor/src/store.rs`: `set_wrap_affinities`(선택 수 검증), `set_view_state` 가 선택이 바뀌면 `wrap_affinities` 도 지움, `attach_view` 초기값.
- `native/taide-native-editor/src/editing.rs`
    - `move_selection_displayed(store, view, motion, extend, display)` 추가. `move_selection` 은 서명 그대로이고 identity 표시 맵으로 같은 구현(`move_selection_across`)을 탑니다. revision 이 문서와 다른 표시 맵은 쓰지 않고 identity 로 처리합니다.
    - `Motion::Vertical { lines, .. }` 의 `lines` 는 표시 줄 수입니다. `vertical_target` 은 표시 줄 내용(들여쓰기 공백 + 조각)의 visible column 으로 계산하고 목표 열을 이어지는 줄의 들여쓰기 뒤로 고정합니다. goal column(leftover)은 표시 줄 기준입니다.
    - `row_start_target`(Home), `row_end_target`(End), 오른쪽 이동의 윗 줄 끝 머묾, 선택 접기(Left·Right)의 선호 유지.
    - 이동 뒤 `wrap_affinities` 를 다시 씁니다(윗 줄 끝에 머문 캐럿이 없으면 `None`).
- wrap 이 꺼진 상태: identity 맵에서는 표시 줄 = 문서 줄, 들여쓰기 0 이라 계산이 이전 식과 같습니다. 기존 `tests/editing.rs` 11건이 수정 없이 통과하고, identity 맵을 넘긴 이동과 맵 없는 이동이 같은 선택·goal column 을 내는 테스트를 추가했습니다.
- 테스트: 신규 `native/taide-native-editor/tests/display-motion.rs` 9건.

### W5. 표면 (`taide-native-ui`)

- `native/taide-native-ui/src/editor-geometry.rs`
    - `RowLayout { painter, document, display, appearance, half_leading, tab_size }` 와 `RowLayout::row(index, origin)`(기존 `Row::layout` 대체. 인자가 8개가 되어 묶었습니다). `Row.wraps`(같은 문서 줄의 다음 표시 줄이 있음).
    - `wrap_settings(painter, appearance, content_width, vertical_scrollbar_width, tab_size)`: 1절의 wrap 열 식과 전각 열 폭. 반각·전각 기준 문자의 폭은 한 글자 galley 의 `advance_width` 입니다.
- `native/taide-native-ui/src/editor-paint.rs`
    - `Carets { selections, head_rows, primary_row, focused }`: 캐럿은 선호를 반영한 표시 줄에만 그립니다.
    - 줄 번호는 `!segment.is_continuation` 인 표시 줄에만.
    - 선택: wrap 경계에서는 오른쪽 끝을 본문 끝으로 늘리지 않고, 경계에서 시작하는 선택은 윗 줄에 그리지 않습니다. 실제 줄 끝의 처리(본문 오른쪽 끝까지)는 그대로입니다.
- ES
    - `Projection { painter, appearance, width, wrap_tab_size, cached }` 와 `Projection::map(document)`. `InputContext` 가 들고 있어 키 이벤트마다 그 시점 문서의 표시 맵으로 `move_selection_displayed` 를 부릅니다.
    - `show_presented`: 표시 맵에서 세로 배치·보이는 줄·캐럿 줄(선호 반영)을 구합니다. wrap 이면 가로 내용 폭 = 가장 넓은 표시 줄(끝 여백·스크롤바 폭 없음)이라 monospace 에서는 가로 스크롤 범위가 0 입니다. wrap 열이 바뀌면 가장 넓은 줄 기록을 지웁니다(`InputState.widest_wrap_column`).
    - 클릭·드래그: 클릭한 표시 줄에서 바이트를 구하고, 그 바이트의 기본 표시 줄이 클릭한 줄과 다르면(줄 끝 너머 클릭) 윗 줄 끝 선호를 저장합니다.
    - Escape 로 선택을 접을 때 선호를 유지합니다(`cursorMoveCommands.js:180-187` 의 뷰 위치 유지).
    - `EditorOutput.rendered_lines` 는 보이는 표시 줄이 속한 문서 줄 범위입니다(단계 1 문서 7절의 인계 사항). `geometry.visible_rows` 는 표시 줄 범위 그대로입니다.
    - `reveal_presented(ui, store, view, line, column, presentation)` 추가. `reveal` 은 서명 그대로 기본값으로 위임합니다. 세로 중앙은 대상 바이트의 표시 줄 기준입니다.
- IME 조합·스크롤바·caret 따라가기는 표시 줄 번호만 바뀌고 식은 그대로입니다.
- 테스트(`native/taide-native-ui/tests/editor_surface.rs`, 신규 8건): 표시 줄 나눔·줄 번호·가로 스크롤 0·wrap 열 식, 클릭·줄 끝 너머 클릭·좌우 이동·드래그 선택 rect, End·Home·PageDown·PageUp·세로 이동과 스크롤 범위, reveal·IME 좌표, 편집·undo·폭·글꼴 변경·wrap 끄기, 이어지는 줄 들여쓰기와 탭·들여쓰기 안 클릭, 탭 확장 단위 테스트 1건, 옵션 전달 1건.

### W6. 옵션 전달

- `native/taide-native-ui/src/presentation.rs`: `editor_presentation(settings)` = `EditorDisplayOptions { word_wrap: settings.editor_word_wrap, ..Default::default() }`.
- `native/taide-native-app/src/application.rs`(`show_document`): 문서 스냅샷·들여쓰기 해석·`with_indent` 를 reveal 앞으로 옮기고, `editor.reveal_presented(.., &editor_presentation)` 과 `editor.show_presented(.., &editor_presentation)` 을 부릅니다. 순서를 옮긴 이유는 reveal 과 그리기가 같은 탭 크기(editorconfig 반영)로 wrap 을 계산해야 하기 때문입니다. reveal 은 문서를 바꾸지 않으므로 스냅샷 내용은 같습니다.
- wrap 열·wrappingIndent 는 TS 가 넘기지 않으므로 표면이 Monaco 기본 동작(`on` = 뷰포트 폭, `Same`)을 씁니다. 언어별 예외는 TS 에 없습니다(1절).
- `taide-remote-web` 은 `show` 를 그대로 부르므로 wrap 이 꺼진 상태입니다.

## 3. 설계 문서·지시와 달라진 점

| # | 설계·지시 | 구현 | 이유 |
| --- | --- | --- | --- |
| 1 | wrap 열 계산은 전각을 2열로 가정(설계 4.5) | 전각 열 폭을 글꼴에서 측정(`full_width_columns: f64`) | Monaco 는 `typicalFullwidthCharacterWidth / typicalHalfwidthCharacterWidth` 를 씁니다(`monospaceLineBreaksComputer.js:26`). 2 로 고정하면 한글·한자 줄이 TS 보다 일찍 나뉩니다. 탭 렌더와 캐럿 열 계산의 전각 2열은 Monaco 도 고정값이라 그대로 2 입니다. |
| 2 | `WrapSettings { wrap_column, tab_size }` | `full_width_columns`, `wrapping_indent` 추가 | 1항과 `computeWrappedTextIndentLength` 의 입력입니다. |
| 3 | 누적합은 Fenwick 트리 | 누적합 배열(`first_rows`)을 바뀐 줄부터 다시 셈 | 줄 수가 바뀌는 편집은 줄별 자료의 `Vec::splice` 가 이미 O(n) 이고 Fenwick 도 다시 만들어야 합니다. 조회는 줄 → 표시 줄 O(1), 표시 줄 → 줄 O(log n) 입니다. 30만 줄 문서의 갱신 전체가 0.64ms 입니다. Monaco 의 `ConstantTimePrefixSumComputer` 도 같은 방식입니다. |
| 4 | 표시 상태는 egui temp data 에 뷰 id 로 보관(설계 4.9) | `ViewState.display`(`Arc<DisplayMap>`) | temp data 는 뷰가 닫혀도 지워지지 않습니다. wrap 표시 맵은 증분 갱신을 위해 rope 사본과 줄별 자료를 들고 있어 닫힌 파일마다 메모리가 남습니다. 뷰 상태에 두면 `detach_view` 에서 함께 사라집니다. `Arc` 는 `ViewState` 복제가 줄별 자료를 복사하지 않게 하기 위한 것이고, 표면은 프레임 동안 꺼내 쓰므로 복사가 일어나지 않습니다. |
| 5 | `row_of_byte(document, byte, prefer_next_row)` | `row_of_byte(document, byte)` 유지 + `row_of_head(document, byte, at_row_end)` | 단계 1 테스트가 2인자 서명을 씁니다. 기본값(뒷 줄)이 Monaco 의 affinity 없음과 같습니다. |
| 6 | `monospaceLineBreaksComputer.js` 이식 | `createLineBreaks` 만 이식 | `createLineBreaksFromPreviousLineBreaks` 는 wrap 열만 바뀔 때 이전 결과를 재활용하는 경로입니다. 대부분 같은 결과지만 "break 뒤 남는 폭이 탭 크기 이하이면 그 후보를 버림"(`:226-243`) 같은 추가 규칙이 있어 창 크기를 바꾼 직후의 나눔이 새로 연 화면과 다를 수 있습니다. native 는 항상 새로 연 화면의 결과입니다. |
| 7 | `RowSegment` 에 시작 열 없음 | `DisplayMap::row_start_column(row)` 별도 제공 | 단계 1 테스트가 `RowSegment` 를 5개 필드 literal 로 만듭니다. |
| 8 | `rows_of_line -> Option<Range>` | `Range` | 숨김 줄(접기)이 아직 없어 항상 값이 있습니다. |
| 9 | `build(document, hidden_lines, wrap)` | `build(document, wrap)` | 접기는 단계 7 입니다. |
| 10 | "Motion 을 확장" | `Motion` 은 그대로, `move_selection_displayed` 가 표시 맵을 인자로 받음 | 좌우 이동·선택 접기도 표시 맵이 필요해(윗 줄 끝 선호) 변형별 필드로는 표현이 중복됩니다. `Motion: Copy` 와 기존 호출부·테스트가 그대로입니다. |
| 11 | W1·W2 는 실패하는 테스트부터 | W1 은 지켰고 W2 는 구현 초안을 먼저 쓴 뒤 테스트를 썼습니다 | 순서를 지키지 못했습니다. W2 테스트의 첫 실행 결과는 6절에 있습니다. |
| 12 | 수정 범위에 `reveal` 서명 변경 없음 | `reveal_presented` 추가, `application.rs` 의 호출 순서 변경 | wrap 상태에서 reveal 이 표시 줄을 모르면 세로 중앙이 어긋납니다. 기존 `reveal` 서명과 동작은 그대로입니다. |
| 13 | 가로 스크롤을 끔 | 가로 내용 폭을 가장 넓은 표시 줄로 둠 | Monaco 의 식입니다(`viewLayout.js:213-222`). monospace 에서는 범위가 0 이고, 글리프 폭이 열 계산보다 넓은 줄(대체 글리프 등)만 스크롤됩니다. |
| 14 | Home 의 "문서 줄의 첫 표시 줄" 판정 | 표시 줄이 이어지는 줄인지로 판정 | Monaco 는 `뷰 열 === 모델 열` 로 판정합니다(`cursorMoveCommands.js:55`). 이어지는 줄의 시작 오프셋이 들여쓰기 열 수와 우연히 같으면 첫 줄로 오인하는 식이라 변수 이름(`isFirstLineOfWrappedLine`)의 의미를 따랐습니다. |

그 밖에 설계와 같지만 선택이 있었던 점입니다.

- 줄 번호 3개 이상 자릿수 변화(1000줄 등)로 gutter 폭이 바뀌면 wrap 열이 바뀌어 전체를 다시 계산합니다.
- 표시 맵의 탭 크기는 표면의 `indent_options(document).tab_size` 입니다(캐럿 열 계산과 같은 값, 7절 3항).
- 캐럿의 윗 줄 끝 선호는 head 에만 둡니다. Monaco 는 선택 시작점도 뷰 위치를 갖지만 그리기와 이동에 차이가 없습니다.
- 줄바꿈 계산의 제어 문자 폭은 Monaco 대로 전각 폭이지만, 제어 문자를 control picture 로 바꿔 그리는 것(`renderControlCharacters`)은 하지 않았습니다.

## 4. 설계 문서 서술·줄 번호 대조

- 설계 7절 단계 4 의 근거 줄(`code-editor.tsx:284`, `editorOptions.js:1241-1285, 2899, 3450-3455`)은 실제 파일과 일치했습니다.
- 설계 4.2 의 wrap 열 식은 `editorOptions.js:1278` 과 같습니다. 단, 식의 14 는 고정값이 아니라 `verticalScrollbarWidth` 입니다(TS 미지정 → 14).
- 설계 4.5 의 "wrap 열 계산은 Monaco 와 같이 전각을 2열로 가정"은 소스와 다릅니다(3절 1항).
- 설계 4.2 의 "누적합은 Fenwick(Monaco 는 ConstantTimePrefixSumComputer)" 서술은 맞지만 구현은 3절 3항으로 정했습니다.
- 설계 2.3 의 `editing.rs` 시각 열 함수 줄 번호(311·322·330)는 일치했습니다. 공개 범위는 넓히지 않았습니다(같은 파일 안에서 씀).
- 설계 4.9 의 `EditorPresentation<'a>` 는 단계 1 에서 수명 인자 없는 `{ options }` 로 구현돼 있었습니다(단계 1 문서 3절 4항). 그대로 썼습니다.
- 설계 2.6 의 "탭은 `tab_size * 공백 폭` 고정 전진"은 `EPAINT/text/text_layout.rs:262-270` 과 일치했고, 탭을 공백으로 바꾼 뒤 열 0 탭의 폭이 같은 것으로 확인됐습니다.

## 5. 검증

공통 접미사는 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` 입니다.

구현 전(실패 확인):

| 명령 | 결과 |
| --- | --- |
| `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test line-breaks` | exit 101, E0432(`line_breaks` 모듈 없음) |
| `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test display-motion` | exit 101, E0432(`move_selection_displayed` 없음) |
| `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface -- 설정의_word_wrap` | exit 101, E0432(`editor_presentation` 없음) |

구현 후(최종 작업 트리):

| 기호 | 명령 | 결과 |
| --- | --- | --- |
| V-E | `cargo test --manifest-path native/taide-native-editor/Cargo.toml` | exit 0, 71 passed / 1 ignored(기준 48 + line-breaks 9 + display-map 5 + display-motion 9, ignored 는 30만 줄 측정). display-map 0.24s, 나머지 0.00s |
| V-U | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` | exit 0, 31 passed(기준 23 + 신규 8), 0.05s |
| V-UL | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib` | exit 0, 116 passed, 0.51s |
| V-A | `cargo check --manifest-path native/taide-native-app/Cargo.toml` | exit 0. 경고는 vendored `wry` 17건뿐(기존) |
| V-AL | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` | exit 0, 352 passed(단계 2 와 같음), 16.56s |
| V-W | `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | exit 0, 4.01s, 경고 없음 |
| V-F | `cargo fmt --manifest-path native/taide-native-editor/Cargo.toml -- --check` | exit 0(처음 exit 1, 6절) |
| V-F | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | exit 0(처음 exit 1, 6절) |
| V-F | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | exit 0 |

계약 밖에서 추가로 실행한 것:

| 명령 | 결과 |
| --- | --- |
| `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test display-map --release -- --nocapture --include-ignored 문서의_wrap` | exit 0, 2 passed. 2절 W2 표의 release 값 |
| 같은 명령에서 `--release` 제외 | exit 0, 2 passed. 2절 W2 표의 debug 값 |
| `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test presentation --test files` | exit 0, presentation 3 passed, files 12 passed |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --test paste-shortcuts --test save` | exit 0, 2 passed / 1 passed |
| `cargo clippy --manifest-path native/taide-native-editor/Cargo.toml --all-targets` | exit 0, 경고 0 |
| `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` | exit 0, lib 경고 9(단계 1·2 와 같은 9건), 테스트 파일 경고 0 |
| `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --lib` | exit 0, 앱 크레이트 경고 0 |

실행하지 않은 것: `taide-native-app` 의 `tests/terminal-host.rs`, `taide-remote-web` 의 나머지 테스트와 wasm32 대상 빌드, GUI 실행, 실제 시스템 글꼴에서의 전각 폭 측정.

고친 기존 기대값(Monaco 근거: `viewLineRenderer.js:845-852` 의 탭 → 탭 정지까지의 공백):

- `PLAIN_FRAME`, `SELECTION_FRAME` 의 둘째 줄 galley 텍스트 `"\tlet 값 = ..."` → `"    let 값 = ..."`. 위치·크기·선택 rect·IME 좌표는 그대로입니다.
- `평문_표시_줄_텍스트는_문서_조각과_같고_...` → `표시_줄_텍스트는_탭을_탭_정지까지의_공백으로_펼치고_단일_구간_layout_job을_만든다`: `RowText::plain` 이 없어져 `RowText::expanded` 로 바꾸고, 표시 텍스트·표시 문자 ↔ 바이트 기대값을 탭 확장에 맞췄습니다. 끝을 넘는 인덱스 → 끝 바이트, 문자 중간 바이트 → 그 문자 단언은 유지했습니다.
- `표시_줄_구간은_문자_범위를_...`: 기준 문자열을 표시 텍스트로 바꿨습니다(단언 내용 동일).

## 6. 실패했다가 고친 내역

- W1: 구현 뒤 첫 실행에서 9건 모두 통과했습니다.
- W2: 구현 초안 뒤 첫 실행에서 `wrap_표시_줄은_문서_줄을_빈틈없이_덮고_...` 1건이 실패했습니다. 테스트가 "경계 바이트의 윗 줄 선호" 기대값을 뒷 줄로 잘못 적은 것이어서 테스트 식을 고쳤습니다(구현 변경 없음).
- W4: 구현 뒤 첫 실행에서 8건 모두 통과했습니다.
- W3: 탭 확장 뒤 특성화 3건이 galley 텍스트 차이 한 줄로 실패했고 5절의 근거로 기대값을 고쳤습니다.
- W5: 신규 6건 중 `wrap_이어지는_줄은_들여쓰기를_채우고_...` 1건이 실패했습니다(캐럿 x 74.28 대 75.0). 테스트가 기준 x 를 "공백 4개 문자열의 끝"으로 측정했는데, epaint 는 문자열 끝은 전진 폭 합, 글리프 위치는 픽셀 내림으로 돌려줍니다. 뒤에 글자가 있는 문자열에서 측정하도록 고쳤습니다(구현 변경 없음).
- 표시 맵 보관 위치: 처음에는 설계대로 egui temp data 에 두었다가 닫힌 뷰의 메모리가 남는 문제를 확인하고 `ViewState` 로 옮겼습니다(3절 4항). 옮긴 뒤 V-E, V-U 를 다시 실행했습니다.
- `cargo fmt -- --check`: `taide-native-editor` 11곳, `taide-native-ui` 17곳을 손으로 맞췄습니다. 테스트 이름 6개는 rustfmt 가 100열을 넘는 서명을 `()` 뒤에서 꺾으려 해 이름을 줄였습니다.
- 줄바꿈 빠른 경로: 처음에는 탭이 있는 줄을 제외했다가 측정(5만 줄 release 11.4ms) 뒤 탭 상한을 넣었습니다. 그 뒤 측정은 실행에 따라 6.5~8.2ms 였습니다.

## 7. 남은 위험과 메인 판단이 필요한 사항

1. 프레임 분할 대비책 미구현. 전체 계산은 release 기준 5만 줄 6.5ms, 30만 줄 31ms 입니다. 30만 줄 문서는 wrap 상태로 처음 그릴 때와 창 폭을 끄는 동안 프레임마다 약 30ms 가 듭니다(편집은 1ms 미만). 설계 4.5 의 대비책(미계산 줄을 1줄로 추정하고 프레임에 나눠 계산)은 보이는 줄의 즉시 계산, 위쪽 줄이 계산될 때의 스크롤 고정, 이동 명령의 추정 줄 처리가 함께 필요해 이번에 넣지 않았습니다. debug 빌드는 5만 줄 96ms 입니다.
2. 편집기 폭이 매우 좁으면(gutter + 16px 이하) wrap 열이 1 이 되어 문자마다 표시 줄이 생깁니다. Monaco 도 같은 식이지만 수 MB 문서에서는 표시 줄이 수백만 개가 되어 계산과 메모리가 급증합니다. 패널을 끝까지 줄이는 동작에서 실기 확인이 필요합니다.
3. 탭 크기 전달의 기존 한계. `NativeEditor::with_indent` 는 탭 들여쓰기(`insertSpaces=false`)일 때 탭 크기를 `"\t"` 로만 넘겨 `indent_options` 가 editorconfig 또는 4 로 되돌립니다. 설정의 `editorTabSize` 가 4 가 아니고 editorconfig 가 없으면 탭 표시 폭과 wrap 계산이 4 기준입니다. 이전에도 표시는 항상 4 였으므로 퇴행은 아니지만 TS 와 다릅니다. `EditorAppearance` 서명을 건드려야 해 이번 범위에서 고치지 않았습니다.
4. 3절 1항(전각 열 폭 측정)과 4항(표시 맵을 `ViewState` 에 둠)은 설계와 다른 결정입니다. 4항은 `taide-native-editor` 의 `ViewState` 가 표면의 폭에 따라 달라지는 값을 들게 합니다.
5. 창 크기 변경 직후의 나눔이 Monaco 와 다를 수 있습니다(3절 6항). TS 화면에서 창 폭을 바꾼 뒤와 새로 연 뒤를 비교해야 차이가 보입니다.
6. 동결된 브라우저 클라이언트의 화면이 바뀝니다. 표면을 공유하므로 열 중간 탭의 폭이 탭 정지 기준이 됩니다. wrap 은 꺼진 상태 그대로입니다.
7. 긴 줄. wrap 이 꺼진 상태에서 줄 전체를 매 프레임 탭 확장·레이아웃합니다(이전에도 줄 전체를 레이아웃). 설계 4.5 의 "앞 10000자까지만 표시"(`stopRenderingLineAfter`)는 이번 항목에 없어 넣지 않았습니다. `RowText::display_char` 는 일치하는 바이트가 없을 때만 문자 순회를 합니다.
8. 전각 기준 문자(U+FF4D)가 글꼴 체인에 없으면 대체 글리프의 폭이 전각 폭이 됩니다. 그 경우에도 화면의 대체 글리프 폭과 일치하지만 TS(브라우저 폴백 글꼴)와는 나눔 위치가 달라집니다.
9. 표시 맵은 뷰가 붙어 있는 동안 유지됩니다. 보이지 않는 탭의 뷰도 마지막으로 그린 시점의 줄별 자료와 rope 사본을 들고 있습니다(5만 줄 문서에서 수백 KB~수 MB).
10. `show_presented`·`reveal_presented` 가 오류로 중단되면 그 프레임에 꺼낸 표시 맵이 버려져 다음 프레임에 전체를 다시 계산합니다. 동작에는 영향이 없습니다.
11. 다중 커서의 표시 줄 선호는 head 별로 저장되지만 UI 테스트는 단일 커서만 다룹니다(모델 테스트에 2개 커서 1건).
12. `EditorOutput.rendered_lines` 의 의미가 표시 줄 범위에서 문서 줄 범위로 바뀌었습니다. 저장소 안의 소비처는 테스트뿐이고 wrap 이 꺼진 상태의 값은 같습니다.

## 8. 실기 확인이 필요한 것

- [ ] 설정에서 자동 줄바꿈을 켜면 긴 줄이 편집기 폭에서 나뉘고, 이어지는 줄이 원래 들여쓰기에 맞춰 시작하는지(TS 화면과 나눔 위치 비교)
- [ ] 줄 번호가 문서 줄의 첫 표시 줄에만 보이고 가로 스크롤바가 나타나지 않는지
- [ ] 한글·한자·일본어가 섞인 긴 줄의 나눔 위치가 TS 와 같은지(전각 열 폭 측정, 금칙)
- [ ] 위·아래 화살표, PageUp·PageDown 이 표시 줄 단위로 움직이고 열을 유지하는지
- [ ] Home·End 를 한 번 누르면 표시 줄의 시작·끝, 다시 누르면 문서 줄의 첫 글자·끝으로 가는지
- [ ] 표시 줄 끝 너머를 클릭하면 캐럿이 그 줄 끝에 보이고, 오른쪽 화살표로 다음 줄의 둘째 글자로 가는지
- [ ] 여러 표시 줄에 걸친 드래그 선택의 모양(wrap 경계에서 오른쪽 끝까지 칠해지지 않음)
- [ ] 탭이 열 중간에 있는 줄(`a\tb`)과 탭 크기 2·8 파일의 탭 폭이 TS 와 같은지
- [ ] 창 폭을 끌어 바꿀 때와 글꼴 크기를 바꿀 때 나눔이 바로 다시 계산되는지, 큰 파일에서 끊김이 없는지
- [ ] 수만 줄 파일에서 wrap 을 켠 채 입력·삭제·undo 가 지연 없이 되는지
- [ ] wrap 상태에서 빠른 열기·검색 결과 이동(`reveal`)이 대상 위치를 세로 중앙에 두는지
- [ ] wrap 상태의 한글 IME 조합 위치와 후보 창 위치
- [ ] 편집기 패널을 아주 좁게 줄였을 때 멈춤이 없는지(7절 2항)
- [ ] 브라우저 클라이언트(`taide-remote-web`)의 탭 표시가 정상인지

## 9. 리뷰 후속 (2026-10-06)

상태: 리뷰 차단 항목 2건을 세 단계가 모두 들어간 작업 트리에서 다시 확인했고 둘 다 유효했습니다. 실패하는 UI 테스트로 재현한 뒤 고쳤습니다. 항목 2 의 `reveal_presented` 부분만 반증입니다. 수정 파일은 `native/taide-native-ui/src/editor-paint.rs`, `native/taide-native-ui/src/editor_surface.rs`, `native/taide-native-ui/tests/editor_surface.rs` 이고 `taide-native-editor`·`taide-native-app` 은 건드리지 않았습니다.

### 9.1 항목 1 — wrap 상태의 현재 줄 강조 (수정)

- 지적 확인: `Layers::row` 가 `row.index == carets.primary_row` 일 때만 현재 줄 배경을 그려 wrap 상태에서 캐럿이 놓인 표시 줄 하나만 칠했습니다.
- Monaco 근거
    - `MONACO/editor/browser/viewParts/currentLineHighlight/currentLineHighlight.js:22, 64`: `_wordWrap = layoutInfo.isViewportWrapping`.
    - `:109-124`: wrap 이면 커서 뷰 줄의 모델 줄을 구해 그 모델 줄의 첫 뷰 줄부터 마지막 뷰 줄까지 `.current-line` 을 채웁니다. `:125-133`: 커서의 뷰 줄만 `.current-line-exact` 로 덮습니다.
    - `:186-198`: 배경색은 `.current-line` 에 걸립니다. `:199-209`: 테두리(`.current-line-exact`)는 배경색이 없거나 투명하거나 테마가 `editor.lineHighlightBorder` 를 정의할 때만 생깁니다.
    - TS 는 `editor.lineHighlightBackground` 만 넘깁니다(`src/shared/lib/monaco/theme.ts:11`). `MONACO/editor/standalone/common/themes.js` 에는 `lineHighlight` 정의가 없습니다(검색 결과 없음). 따라서 TS 화면은 테두리 없이 모델 줄의 모든 뷰 줄에 배경만 칠합니다.
- 수정: `Carets` 에 `primary_line`(주 캐럿 표시 줄의 문서 줄, `display.segment(&document, primary_row).line`)을 추가하고 `Layers::row` 의 조건을 `row.segment.line == carets.primary_line` 으로 바꿨습니다. `primary_row` 는 캐럿·IME 좌표용으로 그대로 씁니다. wrap 이 꺼진 상태는 문서 줄당 표시 줄이 1개라 결과가 같습니다(`PLAIN_FRAME`, `SELECTION_FRAME` 무변경 통과).
- 테스트: `wrap_현재_줄_강조는_캐럿이_놓인_문서_줄의_모든_표시_줄을_칠한다`. 3개 표시 줄로 나뉘는 줄의 둘째 표시 줄을 클릭해 캐럿이 둘째 줄에 그려지는 것과 현재 줄 색 rect 가 세 표시 줄에 있는 것, `tail` 줄을 클릭하면 그 한 줄만 칠해지는 것을 단언합니다. 단언은 rect 의 세로 범위만 봅니다(가로 범위는 9.5 의 1항 때문에 고정하지 않았습니다). 수정 전 실행에서는 rect 전체를 비교하는 형태였고 그 뒤 세로 범위 비교로 좁혔습니다.

### 9.2 항목 2 — wrap 설정이 바뀔 때 첫 보이는 위치 유지 (수정, reveal 은 반증)

- 지적 확인: `Projection::map` 은 wrap 설정이 다르면 표시 맵을 버리고 새로 만들 뿐이었고 `show_presented` 는 `scroll.y` 를 상한으로만 고정했습니다.
- Monaco 근거
    - `MONACO/editor/common/viewModel/viewModelImpl.js:172-181`: `_captureStableViewport` 는 뷰포트 시작이 유효하고 `scrollTop > 0` 일 때 뷰포트 첫 뷰 줄의 최소 열을 모델 위치로 바꿔 `startLineDelta` 와 함께 잡습니다.
    - `:182-210`: 설정 변경 처리의 처음에 잡고 `setWrappingSettings`·`viewLayout.onConfigurationChanged` 뒤에 `recoverViewportStart` 를 부릅니다. `:1183-1196`: 모델 위치를 새 뷰 줄로 바꿔 `그 뷰 줄의 top + startLineDelta` 로 스크롤합니다.
    - 뷰포트 시작은 렌더 때 갱신됩니다(`MONACO/editor/browser/view.js:477` → `viewModelImpl.js:614-616, 1068-1077`, `startLineDelta = scrollTop - 첫 뷰 줄 top`). 첫 뷰 줄은 `scrollTop` 이 걸친 줄입니다(`MONACO/editor/common/viewLayout/linesLayout.js:511`).
    - 뷰포트 시작은 `scrollTop` 이 바뀌면(`viewModelImpl.js:67-73`), 그리고 내용이 바뀌면(`:329-333`, 줄 수 갱신에 따른 설정 변경보다 먼저) 무효가 되고 다음 렌더에서 다시 유효해집니다.
    - 모델 위치 → 뷰 줄은 affinity 없음이라 break 오프셋을 뒷 줄로 봅니다(1절의 `modelLineProjectionData.js:116-153`). native 의 `row_of_byte` 와 같습니다.
- 수정(ES)
    - `RenderedViewport { scroll_top, line_height }` 를 `InputState` 에 두고 `show_presented` 끝에서 그 프레임의 최종 `scroll.y` 와 줄 높이로 기록합니다(Monaco 의 렌더 시 `ViewportStart.update`).
    - `Projection` 에 `rendered_viewport`(프레임 시작의 `scroll.y` 가 기록과 같을 때만 전달)와 `stable_scroll_top` 을 추가했습니다. `Projection::map_with_stable_scroll_top` 은 wrap 설정이 달라 표시 맵을 버릴 때, 버리는 맵의 revision 이 문서와 같고 기록의 `scroll_top > 0` 이면 이전 맵·이전 줄 높이로 첫 보이는 표시 줄을 구해 `(그 표시 줄의 시작 바이트, scroll_top - 줄 top)` 을 잡고, 새 맵을 만든 뒤 `row_top(row_of_byte(바이트)) + delta` 를 `stable_scroll_top` 에 둡니다. `Projection::map` 은 같은 함수의 표시 맵만 돌려줍니다(키 이벤트 경로와 reveal 이 씀).
    - `show_presented` 는 `stable_scroll_top` 이 있으면 그 값을 `scroll.y` 로 쓰고 기존대로 상한에 고정합니다. 그 뒤의 캐럿 따라가기·휠·스크롤바 처리는 그대로입니다.
- 리뷰 제안과 다르게 한 점
    1. 하위 줄 번호가 아니라 바이트로 복원합니다. Monaco 가 모델 위치(줄, 열)를 새 매핑의 뷰 줄로 바꾸기 때문입니다. 폭이 바뀌면 같은 하위 줄 번호는 다른 글자를 가리킵니다.
    2. 이전 프레임의 줄 높이를 기록합니다. 글꼴 크기가 바뀌면 줄 높이도 바뀌어, 새 줄 높이로는 이전 맵에서의 첫 보이는 표시 줄을 구할 수 없습니다. delta 는 Monaco 처럼 이전 픽셀 값 그대로 더합니다.
    3. 유효 조건을 Monaco 의 뷰포트 시작 무효화에 맞췄습니다. 같은 프레임에 문서가 바뀌었거나(편집으로 줄 번호 자릿수가 늘어 wrap 열이 바뀌는 경우 포함) 마지막 렌더 뒤 `scroll.y` 가 밖에서 바뀌었으면 복원하지 않습니다.
    4. `reveal_presented` 에는 적용하지 않았습니다(반증). reveal 은 `scroll.y` 를 대상 표시 줄의 세로 중앙으로 덮어써서(ES `reveal_presented` 의 `scroll.y = (layout.row_center(index) - ...)`) 이전 뷰포트를 유지할 대상이 없습니다. reveal 이 새 wrap 설정으로 맵을 만들어 두면 같은 프레임의 `show_presented` 는 맵을 다시 만들지 않으므로 reveal 이 정한 스크롤이 유지됩니다. reveal 의 `Projection` 은 `rendered_viewport: None` 입니다.
- 테스트: `wrap_설정이_바뀌어도_맨_위에_보이던_문서_위치는_맨_위에_남는다`. 30줄(줄당 200자) 문서를 wrap 이 꺼진 채 13번째 줄 + 7px 로 스크롤한 뒤 (1) wrap 켜기, (2) 그 줄의 둘째 표시 줄로 스크롤 후 폭 800 → 400, (3) 글꼴 14 → 28(줄 높이 20 → 42), (4) wrap 끄기와 글꼴 복귀를 차례로 적용하고, 매번 `scroll.y` 가 "기준 바이트가 속한 새 표시 줄의 top + 7" 이고 첫 렌더 문서 줄이 13번째 줄인지 단언합니다.

### 9.3 검증

공통 접미사는 5절과 같습니다.

| 시점 | 명령 | 결과 |
| --- | --- | --- |
| 수정 전 | V-U `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` | exit 101, 31 passed / 2 failed. `wrap_현재_줄_강조는_...`: 현재 줄 rect 가 `[0,20]-[800,40]` 1개(기대 3개). `wrap_설정이_바뀌어도_...`: wrap 을 켠 뒤 `scroll.y` 247.0(기대 727.0) |
| 수정 후 | V-U 같은 명령 | exit 0, 33 passed(31 + 신규 2), 0.06s |
| 수정 후 | V-UL `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib` | exit 0, 116 passed, 0.53s |
| 수정 후 | V-A `cargo check --manifest-path native/taide-native-app/Cargo.toml` | exit 0. 경고는 vendored `wry` 17건뿐(기존) |
| 수정 후 | V-AL `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` | exit 0, 352 passed, 15.43s |
| 수정 후 | V-W `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | exit 0, 경고 없음 |
| 수정 후 | V-F `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | 처음 exit 1(ES 의 대입문 한 곳 줄 나눔), 고친 뒤 exit 0. 테스트 파일을 마지막으로 고친 뒤에도 exit 0 |
| 계약 밖 | `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` | exit 0, lib 경고 9(5절과 같은 9건), 테스트 파일 경고 0 |

실행하지 않은 것: V-E 와 `taide-native-editor`·`taide-native-app` 의 V-F(두 크레이트를 수정하지 않아 5절 결과를 그대로 씁니다), 5만 줄 wrap 계산 시간 측정(표시 맵 계산 코드를 바꾸지 않아 2절 W2 의 값이 그대로입니다), GUI 실행.

고친 기존 테스트 코드: 도우미 `selected(shapes)` 를 `filled(shapes, fill)` 로 일반화하고 호출부 1곳을 `filled(.., Color32::BLUE)` 로 바꿨습니다. 기대값은 바꾸지 않았습니다.

### 9.4 테스트 부채

| 재현 조건 | 생략 이유 | 남은 위험 | 실행이 필요해지는 시점 |
| --- | --- | --- | --- |
| wrap 설정이 바뀌는 프레임에 편집 이벤트가 함께 들어옴 | 편집이 캐럿 따라가기로 스크롤을 다시 정해 복원 여부가 화면에서 구분되지 않음 | 복원을 건너뛰는 조건(버리는 맵의 revision ≠ 문서 revision)이 깨져도 테스트가 잡지 못함 | 복원 조건을 고치거나 프레임 안 입력 순서를 바꿀 때 |
| 렌더 없이 `scroll.y` 를 밖에서 바꾼 직후 wrap 설정이 바뀜 | 운영 코드에서 표면 밖의 스크롤 변경은 `reveal_presented` 뿐이고 reveal 은 맵을 직접 다시 만듦 | 다른 스크롤 복원 경로가 생기면 "기록과 다르면 복원하지 않음" 동작이 의도와 다를 수 있음 | 세션 복원 등 표면 밖에서 스크롤을 쓰는 기능을 추가할 때 |

### 9.5 범위 밖에서 확인한 Monaco 와의 차이 (수정하지 않음)

1. 현재 줄 강조의 조건과 범위. Monaco 기본값 `renderLineHighlight: 'line'`(`MONACO/editor/common/config/editorOptions.js:3320`, TS 미지정)에서는 선택이 하나라도 비어 있지 않으면 본문 강조를 그리지 않고(`currentLineHighlight.js:150-154`), 줄 번호 영역은 칠하지 않으며(`:174-185` 의 margin 은 `current-line-margin` 클래스가 있을 때만 배경이 걸림, `:192`), 모든 커서의 줄을 칠합니다(`:36-47`). native 는 선택이 있어도 그리고(`SELECTION_FRAME` 의 `rect 0.00,60.00..800.00,80.00`), gutter 를 포함한 전체 폭을 칠하며, 주 커서 줄만 칠합니다. 이번 배치 이전부터의 동작입니다.
2. wrap 설정이 그대로이고 줄 높이만 바뀌는 경우(wrap 이 꺼진 상태의 글꼴 크기 변경)에는 첫 보이는 줄 유지가 없습니다. Monaco 는 모든 설정 변경에서 같은 복원을 합니다(`viewModelImpl.js:182-210`). 이번 수정은 표시 맵이 다시 만들어지는 경우만 다룹니다.
3. 창 폭을 끄는 동안에는 매 프레임 기준 바이트가 "그 바이트를 담은 표시 줄의 시작"으로 다시 잡혀 같은 문서 줄 안에서 위로 조금씩 옮겨 갈 수 있습니다. Monaco 도 뷰 줄의 최소 열을 기준으로 잡아 같습니다(`viewModelImpl.js:176-177`).

### 9.6 실기 확인이 필요한 것

- [ ] wrap 상태에서 여러 표시 줄로 나뉜 줄의 가운데 표시 줄에 캐럿을 두면 그 문서 줄의 모든 표시 줄이 현재 줄 색으로 칠해지는지(TS 화면과 비교)
- [ ] 문서 중간으로 스크롤한 뒤 자동 줄바꿈을 켜고 끌 때 맨 위에 보이던 줄이 그대로 맨 위에 있는지
- [ ] wrap 상태에서 창 폭을 끌어 바꾸는 동안과 글꼴 크기를 바꾼 직후 맨 위 내용이 밀리지 않는지, 스크롤바가 잠깐 나타나는 정도가 TS 와 비슷한지
