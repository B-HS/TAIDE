# Native 배치 4 단계 1 — 표시 계층 골격 (2026-10-06)

상태: K1~K6 구현과 검증 계약(V-E, V-U, V-UL, V-A, V-AL, V-W, V-F, 앱 통합 테스트 2종)이 전부 exit 0 입니다. 화면과 입력 결과는 리팩터링 전과 같습니다(특성화 테스트 4건과 기존 13건이 기대값 수정 없이 통과). 메인 판단이 필요한 사항 3건은 7절에, 설계 문서와 달라진 점은 3절에 있습니다. GUI 실행은 하지 않았습니다.

경로는 저장소 루트(`/Users/hyunseokbyun/development/TAIDE`) 기준입니다. `MONACO` 는 `node_modules/monaco-editor/esm/vs`, `EPAINT` 는 `/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/epaint-0.36.2/src`, `EGUI` 는 `native/taide-native-app/vendor/egui-input/src`, `ES` 는 `native/taide-native-ui/src/editor_surface.rs`, 설계 문서는 `docs/research/2026-10-06-native-editor-display-layer-design.md` 입니다.

## 1. 기준 확인

이 단계는 구조 변경이므로 TS 화면 동작을 새로 옮기지 않았습니다. 구조와 자료형의 근거로 직접 읽은 소스는 아래와 같습니다.

| 대상 | 확인한 사실 | 근거 |
| --- | --- | --- |
| 뷰 파트 구성 순서 | 현재 줄 강조 → 선택 → 들여쓰기 가이드 → 장식 → 공백 표식, 여백 overlay, content widget, 캐럿 순으로 등록 | `MONACO/editor/browser/view.js:114-167` |
| identity 줄 투영 | wrap·숨김이 없을 때 뷰 줄 수 = 모델 줄 수, 뷰 줄 내용 = 모델 줄 내용 | `MONACO/editor/common/viewModel/viewModelLines.js:838-930` (`ViewModelLinesFromModelAsIs`) |
| 세로 배치 | 줄의 세로 위치 = 앞 줄 높이 합 + 앞 whitespace 합, 세로 위치의 줄은 범위 밖에서 첫 줄·마지막 줄로 고정 | `MONACO/editor/common/viewLayout/linesLayout.js:336-340, 401-412, 469-497` |
| 캐럿 스타일 값 | `line` 은 `cursorWidth` 0 일 때 2px, `line-thin` 은 1px | `MONACO/editor/browser/viewParts/viewCursors/viewCursor.js:126-145` |
| 캐럿·공백 옵션 값 집합 | cursorBlinking `blink·smooth·phase·expand·solid`, cursorStyle `line·block·underline·line-thin·block-outline·underline-thin`, renderWhitespace `none·boundary·selection·trailing·all` | `MONACO/editor/common/config/editorOptions.js:366-373, 397, 410-416, 3156, 3165, 3331` |
| TS·Rust 설정 값 집합 | renderWhitespace `none·boundary·selection·all`, cursorStyle `line·block·underline`, cursorBlinking `blink·smooth·phase·expand·solid` | `crates/taide-model/src/settings.rs:15-49` |
| `layout_no_wrap` | `LayoutJob::simple(text, font, color, f32::INFINITY)` 를 만들어 `layout_job` 으로 넘김 | `EGUI/painter.rs:503-510`, `EPAINT/text/fonts.rs:931-939`, `EPAINT/text/text_layout_types.rs:126-142` |
| wheel 즉시 반영 조건 | `TouchPhase::Start` 뒤의 `Move` 는 평활화 없이 그 프레임에 반영 | `EGUI/input_state/wheel_state.rs:84-159` (특성화 테스트의 스크롤 입력 구성 근거) |

## 2. 항목별 변경

### K1. 특성화 테스트 (리팩터링 전 고정)

- `native/taide-native-ui/tests/editor_surface.rs`
    - 도우미 `painted`(도형 한 개를 종류·rect·채움색·텍스트·위치·크기·색·job 형태·clip 으로 적음), `view_line`(스크롤·선택·조합 문자열), `snapshot`(한 프레임의 도형 목록 + IME 출력 + 뷰 상태), `assert_frame`, `scrolled_document`.
    - `평문_화면은_배경_현재_줄_캐럿_본문_줄_번호를_고정된_순서와_좌표로_그린다`: 탭·한글·한자·결합 문자·4바이트 문자·CRLF·빈 줄이 든 6줄 문서(`PLAIN_DOCUMENT`)의 이벤트 없는 프레임. 기대값 `PLAIN_FRAME` 17줄.
    - `포인터_드래그로_만든_여러_줄_선택은_줄별_선택_영역을_고정된_좌표로_그린다`: 클릭 뒤 같은 위치에서 눌러 두 줄 아래로 드래그. 기대값 `SELECTION_FRAME` 19줄(선택 14..50, 줄 끝을 넘는 선택의 본문 오른쪽 끝 확장, 빈 줄 선택 포함).
    - `스크롤_뒤_ime_조합은_preedit와_후보_창_좌표를_스크롤된_캐럿에_맞춘다`: wheel(-24, -130) → 긴 줄 클릭 → End(가로 캐럿 따라가기) → IME preedit. 기대값 `SCROLLED_COMPOSITION_TRACE` 35줄(단계별 뷰 상태 3줄 + 마지막 프레임의 도형·스크롤바 slider·IME rect).
    - `reveal은_긴_줄의_끝_열을_고정된_가로_세로_스크롤로_맞춘다`: K4 대상인 `reveal` 의 가로·세로 스크롤 결과를 `REVEALED_VIEW` 로 고정. 지시의 3종에 더해 추가했습니다. 기존 reveal 테스트는 세로만 정확히, 가로는 `> 0` 으로만 단언하기 때문입니다.
    - 본문 줄과 줄 번호, preedit 의 `Shape::Text` 는 `galley.job` 이 `LayoutJob::simple(text, monospace(14), 색, INFINITY)` 와 같은지 비교해 `job simple <색>` 으로 기록합니다. K3 의 "현재와 같은 LayoutJob" 조건이 이 값으로 고정됩니다.
- 기대값은 리팩터링 전 코드에서 실행한 출력을 그대로 옮겼고, 리팩터링 전 상태에서 17건(기존 13 + 특성화 4) 통과를 확인한 뒤 리팩터링을 시작했습니다(5절 1행). 이후 이 4건의 기대값과 도우미는 수정하지 않았습니다.

### K2. 표시 모델 (`taide-native-editor`)

- 신규 `native/taide-native-editor/src/display-map.rs`
    - `RowSegment { line, bytes, is_continuation, indent_columns, ends_folded }`(설계 4.2 의 자료형 그대로).
    - `DisplayMap::identity(line_count, revision)`, `revision()`, `row_count()`, `segment(document, row)`(줄 끝 문자를 뺀 바이트 범위는 기존 `editing::line_content_range` 재사용, 범위 밖 row 는 마지막 줄로 고정), `row_of_byte(document, byte)`(바이트는 문서 길이로 고정한 뒤 `byte_to_line`).
- 신규 `native/taide-native-editor/src/display-layout.rs`
    - `VerticalLayout::new(line_height, row_count)`, `row_top`, `row_bottom`, `row_center`, `row_at`, `content_height`, `visible_rows(top, height, overscan)`.
    - `row_at` 은 Monaco `getLineNumberAtOrAfterVerticalOffset` 과 같이 음수는 첫 줄, 끝을 넘으면 마지막 줄입니다.
    - `visible_rows` 는 기존 식 `first = floor(top / lh)`, `end = min(first + ceil(height / lh) + overscan, row_count)`, `first.min(end)..end` 를 그대로 옮긴 것입니다.
- `native/taide-native-editor/src/lib.rs`: `display_layout`, `display_map` 모듈 연결(`#[path]`).
- 테스트: 신규 `native/taide-native-editor/tests/display-layout.rs`(3건), `display-map.rs`(2건).
- 표면 적용(ES `show_presented`, `reveal`): `줄 수 * line_height` → `layout.content_height()`, 캐럿 줄 위·아래 → `layout.row_top`·`row_bottom`, 첫 줄·끝 줄 계산 → `layout.visible_rows`, 줄 y → `layout.row_top`, 포인터 y 의 줄 → `layout.row_at`, `byte_to_line` → `display.row_of_byte`, `line_content_range` 직접 호출 → `display.segment`.

### K3. 표시 줄 텍스트 (`editor-row-text.rs`)

- 신규 `native/taide-native-ui/src/editor-row-text.rs`
    - `RowSection { chars, foreground }`, `RowText { text, sections, model_bytes }`.
    - `RowText::plain(text, foreground)`: 텍스트는 문서 줄 조각 그대로, 구간은 전체를 덮는 하나, `model_bytes` 는 문자별 바이트 오프셋과 끝 위치 한 개.
    - `layout_job(font)`: 구간마다 `TextFormat::simple(font, foreground)`, `wrap.max_width = INFINITY`, `break_on_newline = true`. 평문 한 구간이면 `LayoutJob::simple(text, font, foreground, INFINITY)` 와 같습니다(테스트로 단언).
    - `model_byte(display_char)`(표시 문자 → 바이트, 끝을 넘으면 끝 바이트), `display_char(model_byte)`(바이트 → 그 바이트가 속한 표시 문자. 기존 `rope.byte_to_char(byte) - start_char` 와 같은 내림 의미).
- 탭 확장·주입 텍스트·토큰 구간은 넣지 않았습니다.
- 테스트(`tests/editor_surface.rs`): `평문_표시_줄_텍스트는_문서_조각과_같고_wrap_없는_단일_구간_layout_job을_만든다`, `표시_줄_구간은_문자_범위를_layout_job의_바이트_구간과_전경색으로_옮긴다`.

### K4. 좌표 함수 통합 (`editor-geometry.rs`)

- 신규 `native/taide-native-ui/src/editor-geometry.rs`
    - `Row { index, segment, text, origin, galley }` 와 `Row::layout`(표시 줄 조각 → `RowText` → `painter.layout_job`), `Row::caret`(galley 좌표의 캐럿 rect), `Row::caret_rect`(화면 좌표), `Row::byte_at`(포인터 위치 → 문서 바이트).
    - `gutter_width(painter, line_count, appearance)`(ES 에서 이동, 식 동일), `scroll_x_revealing(scroll_x, caret, text_width)`(캐럿이 보이도록 하는 가로 스크롤. `show_presented` 와 `reveal` 이 함께 사용), 상수 `CURSOR_STROKE`·`LINE_NUMBER_MIN_DIGITS`·`PADDING_SIDES`(ES 에서 이동).
    - `EditorGeometry`(K6).
- ES `reveal`: 직접 하던 galley 구성·gutter 폭·가로 스크롤 계산을 `Row::layout`, `row.caret`, `gutter_width`, `scroll_x_revealing`, `VerticalLayout::row_center`·`content_height` 로 바꿨습니다. 세로 중앙 식 `(줄 + 0.5) * line_height` 는 `row_center` 가 같은 연산 순서로 계산합니다.
- `reveal` 의 가로 값은 `scroll_x_revealing(..).max(0.0)` 입니다. 기존 코드는 오른쪽으로 넘칠 때만 `.max(0.0)` 을 적용했으나, 저장된 스크롤은 `EditorStore::set_view_state` 가 음수를 거절하고(`native/taide-native-editor/src/store.rs:1030`) 캐럿의 왼쪽 좌표는 0 이상이므로 두 식의 결과는 같습니다.

### K5. 페인트 층 (`editor-paint.rs`)

- 신규 `native/taide-native-ui/src/editor-paint.rs`
    - `Layers { painter, text_painter, rect, text_rect, gutter, appearance }` 와 층 함수 `background`, `current_line`, `selection`, `caret`, `text`, `line_number`, `composition`, `scrollbar`.
    - `Layers::row(row, carets)` 가 한 표시 줄의 층을 기존 순서(현재 줄 강조 → 선택별 [선택 영역, 캐럿] → 본문 → 줄 번호)로 부릅니다. `Carets { document, display, selections, primary_row, focused }` 는 선택·캐럿 판정 입력입니다.
    - 상수 `SCROLLBAR_IDLE_OPACITY`·`SCROLLBAR_ENGAGED_OPACITY`(ES 에서 이동).
- ES 의 그리기 순서는 그대로입니다: 배경 → 줄마다 `Layers::row` → preedit → 스크롤바 slider. 도형 목록의 순서가 특성화 테스트로 고정돼 있습니다.

### K6. 공개 API

- ES
    - `RenderWhitespace { None(기본), Boundary, Selection, All }`, `CursorStyle { LineThin(기본), Line, Block, Underline }`, `CursorBlinking { Solid(기본), Blink, Smooth, Phase, Expand }`.
    - `EditorDisplayOptions`: 설계 4.9 의 14개 필드(`word_wrap`, `render_whitespace`, `rulers`, `cursor_style`, `cursor_blinking`, `smooth_caret`, `scroll_beyond_last_line`, `smooth_scrolling`, `sticky_scroll`, `minimap`, `folding`, `bracket_pair_colorization`, `bracket_pair_guides`, `bold_family`). `Default` 는 전부 꺼짐·1px 선 캐럿·깜빡임 없음·공백 표식 없음으로 현재 native 동작입니다.
    - `EditorPresentation { options }`.
    - `NativeEditor::show_presented(ui, store, view, request_focus, keymap, route, presentation)`: 기존 `show_with_input_route` 본문이 여기로 옮겨졌습니다. `show_with_input_route` 는 `&EditorPresentation::default()` 로 위임하고, `show` → `show_with_keymap` → `show_with_input_route` 사슬은 그대로입니다.
    - `EditorOutput.geometry: EditorGeometry { rect, content_rect, gutter_rect, line_height, visible_rows, scroll }`.
- `native/taide-native-ui/src/lib.rs`: `editor_geometry`(pub), `editor_paint`(비공개), `editor_row_text`(pub) 모듈 연결.
- `EditorAppearance`, `NativeEditor { appearance }`, `show`·`show_with_keymap`·`show_with_input_route`·`reveal` 의 서명은 바꾸지 않았고 `taide-native-app`·`taide-remote-web` 은 수정하지 않았습니다.
- 테스트: `기본_presentation의_show_presented는_기존_show와_같은_화면과_좌표_정보를_돌려준다`(도형 목록이 `PLAIN_FRAME` 과 같고 geometry 값이 gutter 폭·뷰포트·스크롤과 일치).
- 이 단계에서 표면은 `presentation` 의 값을 읽지 않습니다. 각 옵션은 그 동작을 구현하는 단계에서 연결해야 합니다(3절 4항, 7절).

## 3. 설계 문서와 달라진 점

| # | 설계 | 구현 | 이유 |
| --- | --- | --- | --- |
| 1 | `DisplayMap::build`, `WrapSettings`, `rows_of_line`, `row_of_byte(.., prefer_next_row)` | `identity`·`revision`·`row_count`·`segment`·`row_of_byte(document, byte)` 만 | 단계 1 내용은 identity 뿐입니다. `build` 는 `line-breaks.rs`(단계 4)와 숨김 줄(단계 7)이 있어야 하고, affinity 는 뷰 상태에 값이 생기는 단계 4 에서 인자와 함께 넣어야 호출부가 의미 있는 값을 넘길 수 있습니다. |
| 2 | `VerticalLayout::new(line_height, row_count, zones, padding_bottom)`, `ViewZone`, `zone_top` | `new(line_height, row_count)`, zone 없음 | 단계 1 내용이 "zone 없음"입니다. zone 은 단계 14, `padding_bottom`(scrollBeyondLastLine)은 단계 8 에서 Monaco 식을 대조한 뒤 넣어야 합니다. |
| 3 | `VerticalLayout` 에 `row_bottom`·`row_center`·`visible_rows` 없음 | 추가 | 기존 부동소수 연산 순서를 그대로 유지하기 위해서입니다. `reveal` 의 `(줄 + 0.5) * lh` 를 `row_top + lh / 2` 로 바꾸면 큰 줄 번호와 2진 분수가 아닌 줄 높이에서 1 ulp 차이가 날 수 있습니다. 보이는 줄 범위도 `row_at` 두 번으로 구하면 기존 `rendered_lines` 와 끝 값이 달라집니다(예: 높이 200, 줄 높이 20 에서 기존 11줄, `row_at` 방식 12줄). |
| 4 | `EditorPresentation<'a>` 에 `tokens`, `token_styles`, `layers`, `zones`, `hidden_lines` | `options` 만, 수명 인자 없음 | `LineTokens`(3b)·`TokenStyleTable`(3c)·`DecorationLayer`(5)·`ViewZone`(14) 자료형과 숨김 줄 투영(7)이 아직 없습니다. 자료형이 생기는 단계에서 필드와 수명 인자를 추가하면 됩니다. `..Default::default()` 로 만드는 호출부는 영향이 없습니다. |
| 5 | `RowSection` 7개 필드, `RowText` 가 스타일 run 병합 | `chars`·`foreground` 만 | 설계 4.5 가 단계 1 의 job 을 "단일 섹션"으로 정했습니다. 밑줄·취소선은 `Stroke` 폭, 굵기는 굵은 face 패밀리(단계 2)가 정해져야 `TextFormat` 으로 옮길 수 있어 3d 에서 토큰과 함께 넣는 것이 맞습니다. |
| 6 | `model_bytes[i]` 는 표시 문자 i 의 바이트 | 끝 위치 한 개를 더 가짐(길이 = 문자 수 + 1) | 줄 끝 캐럿(문자 인덱스 = 문자 수)을 같은 표로 환산하기 위해서입니다. |
| 7 | `CursorStyle` 기본값이 "1px 선 캐럿" | `LineThin`(기본)과 `Line` 을 분리 | Monaco 의 `line` 은 2px 이고 1px 는 `line-thin` 입니다(`viewCursor.js:126-145`). 기본값을 `Line` 으로 두면 단계 8 에서 `Line` 의 의미가 1px 와 2px 로 갈립니다. TS 설정 값 집합(`line·block·underline`)은 그대로 포함합니다. |
| 8 | `EditorGeometry::caret_rect`·`range_rects`·`top_right_anchor` | 필드 6개만 | 설계 7절이 좌표 질의를 단계 5 의 목표로 두었습니다. 표면 내부는 `Row::caret_rect` 를 씁니다. |
| 9 | `editing.rs` 의 시각 열 함수 공개 범위 확대 | 변경 없음 | identity 투영은 시각 열을 쓰지 않습니다(`indent_columns` 는 항상 0). 단계 4 에서 필요해질 때 넓히면 됩니다. |
| 10 | 페인트 "층별 함수" | 층 함수는 나누되 줄 단위 `Layers::row` 가 현재 순서로 호출 | 층을 문서 전체에 대해 차례로 그리면 도형 순서가 바뀝니다(줄 단위 → 층 단위). 설계 7절의 "순서는 지금과 같게"와 4.6 의 `paint_row` 형태를 따랐습니다. Monaco 순서(캐럿이 본문 뒤)로의 변경은 화면이 바뀌는 변경이라 단계 8 대상입니다. |

## 4. 설계 문서 서술·줄 번호 대조

- 설계 문서의 기준 시점은 HEAD `09ced3a4` 이고 착수 시점은 `d525c282` 입니다. ES, `editing.rs`, `tests/editor_surface.rs` 에 대해 설계가 적은 줄 번호(ES 255-307, 340-780, 417-419, 502-518, 515, 593, 621-693, 727-771, 773, 1029-1049, 1051-1057, `editing.rs` 311·322·330, `tests/editor_surface.rs:692`)는 착수 시점 파일과 일치했습니다.
- 2.6 의 `layout_no_wrap` 서술(`EGUI/painter.rs:503-510`, `EPAINT/text/fonts.rs:931-942`)은 소스와 일치했고, 특성화 테스트가 `job simple` 로 실제 동작을 확인했습니다.
- 4.9 의 "1px 선 캐럿이 기본"은 Monaco 값 이름과 어긋납니다(3절 7항).
- 설계에 없던 관찰: 빈 줄도 `Shape::Text`(크기 0×16)를 냅니다. 특성화 기대값에 포함돼 있습니다.
- 이 단계 뒤 ES 의 줄 번호는 설계 문서와 달라졌습니다. 후속 단계는 함수 이름으로 찾아야 합니다(`show_presented`, `reveal`, `editor-geometry.rs` 의 `Row`·`gutter_width`, `editor-paint.rs` 의 `Layers`).

## 5. 검증

공통 접미사는 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` 입니다.

| 순서 | 명령 | 결과 |
| --- | --- | --- |
| 1 (리팩터링 전) | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` | exit 0, 17 passed(기존 13 + 특성화 4), 0.04s |
| 2 V-E | `cargo test --manifest-path native/taide-native-editor/Cargo.toml` | exit 0, 48 passed(기준 43 + 신규 5), 대상별 0.00s |
| 3 V-U | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` | exit 0, 20 passed(기존 13 무수정 + 특성화 4 + 신규 3), 0.05s |
| 4 V-UL | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib` | exit 0, 116 passed(기준 116), 0.52s |
| 5 V-A | `cargo check --manifest-path native/taide-native-app/Cargo.toml` | exit 0, 5.65s. 경고는 vendored `wry` 17건뿐(기존) |
| 6 V-AL | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` | exit 0, 347 passed(기준 347), 14.73s |
| 7 V-W | `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | exit 0, 2.31s, 경고 없음 |
| 8 앱 통합 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test paste-shortcuts --test save` | exit 0, paste-shortcuts 2 passed(0.03s), save 1 passed(0.05s) |
| 9 V-F | `cargo fmt --manifest-path native/taide-native-editor/Cargo.toml -- --check` | exit 0 |
| 10 V-F | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | exit 0 |

계약 밖에서 추가로 실행한 것:

| 명령 | 결과 |
| --- | --- |
| `cargo clippy --manifest-path native/taide-native-editor/Cargo.toml --all-targets` | exit 0, 경고 0 |
| `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml` (리팩터링 전) | exit 0, lib 경고 8 |
| `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` (리팩터링 후) | exit 0, lib 경고 9. 늘어난 1건은 `show_presented` 의 `too_many_arguments (8/7)`(7절 1항). 테스트 파일 경고 0 |
| `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml --all-targets` | exit 101. `tests/snippet-session.rs:621, 625` 의 기존 `clippy::reversed_empty_ranges`(deny) 때문이며 이 단계가 건드리지 않은 파일입니다 |

실행하지 않은 것: `taide-native-app` 의 `tests/terminal-host.rs`, `taide-remote-web` 의 테스트와 wasm32 대상 빌드, GUI 실행.

## 6. 실패했다가 고친 내역

- 특성화 기대값 수집: 기대값을 빈 목록으로 두고 실행해 실제 출력을 얻었습니다(의도한 실패, 3회). 첫 실행은 `assert_eq!` 의 좌우 출력이 겹쳐 잘렸기 때문에 `assert_frame` 을 `assert!` + 여러 줄 출력으로 바꾸고 테스트별로 다시 실행했습니다. 기대값을 채운 뒤 리팩터링 전 코드에서 17건 통과를 확인했습니다.
- `cargo fmt -- --check` 가 새로 쓴 코드 4곳(`display-layout.rs` 의 `visible_rows`, `tests/display-layout.rs` 의 호출 2곳, ES `reveal` 의 `scroll.y` 대입, `tests/editor_surface.rs` 끝의 빈 줄)을 지적해 손으로 맞췄습니다. 그 뒤 V-E 와 V-U 를 다시 실행했습니다(5절 2·3행이 최종 결과).
- 리팩터링 뒤의 테스트 실패는 없었습니다.

## 7. 남은 위험과 메인 판단이 필요한 사항

1. `show_presented` 는 설계 4.9 의 서명대로 인자가 8개(`&self` 포함)라서 clippy `too_many_arguments (8/7)` 경고가 1건 늘었습니다. 같은 크레이트에 같은 경고가 이미 5건 있고(`settings-code-view.rs:108, 566, 1250`, `settings-view.rs:714`, `snippet-editor.rs:1768`) 검증 계약에 clippy 는 없습니다. 경고를 없애려면 서명을 바꿔야 합니다(예: `editor.presented(&presentation).show(..)` 형태). 설계 서명 유지와 경고 제거 중 어느 쪽을 택할지 결정이 필요합니다.
2. `EditorDisplayOptions` 의 14개 필드는 자료형만 있고 표면이 읽지 않습니다(`show_presented` 의 인자 이름이 `_presentation` 입니다). 호출자가 기본값이 아닌 값을 넘겨도 화면은 바뀌지 않습니다. 현재 호출부는 전부 기본값이므로 동작 차이는 없지만, 옵션을 연결하는 단계는 인자 이름을 되돌리고 해당 필드의 테스트를 함께 넣어야 합니다.
3. `CursorStyle::LineThin` 을 기본값으로 둔 것(3절 7항)은 설계에 없는 값입니다. 단계 8 에서 APP 이 TS 설정 `line` 을 `CursorStyle::Line`(2px)으로 넘기면 캐럿 폭이 지금의 1px 에서 2px 로 바뀝니다. 그 변화가 의도인지 단계 8 에서 확인해야 합니다.

그 밖의 주의 사항:

- `EditorOutput.rendered_lines` 는 지금 `geometry.visible_rows` 와 같은 값(표시 줄 범위)입니다. identity 에서는 문서 줄 범위와 같지만 wrap·접기가 들어오면 달라지므로 단계 4 에서 문서 줄 범위로 다시 정의해야 합니다.
- `Row` 는 `segment.bytes.start` 를 줄 시작으로 씁니다. 이어지는 표시 줄(wrap)에서는 줄 시작과 조각 시작이 달라지므로 단계 4 에서 `RowText::plain` 과 `Row::caret`·`byte_at` 의 기준을 함께 고쳐야 합니다.
- 매 프레임 보이는 줄마다 `RowText`(텍스트 복사 1회 + `model_bytes`)를 만들고 job 용으로 텍스트를 한 번 더 복사합니다. 일반 줄에서는 무시할 수준이지만 수 MB 한 줄 문서에서는 기존보다 프레임당 비용이 늘어납니다. 설계 4.5 의 "앞 10000자까지만 표시"가 들어오면 해소됩니다. 측정은 하지 않았습니다.
- 특성화 기대값은 번들 monospace 글꼴(Hack)의 폭과 줄 높이 16 에 묶여 있습니다. 단계 2(글꼴 체인·세로 정렬)는 화면이 바뀌는 단계이므로 Monaco·TS 근거와 함께 이 기대값을 갱신해야 합니다.
- 다중 선택이 있는 프레임의 도형 순서는 코드상 그대로지만 특성화 테스트는 단일 선택만 다룹니다.
- 뷰포트 높이가 0 이하인 경우의 보이는 줄 범위는 기존 식을 그대로 옮겼고 테스트는 `VerticalLayout::visible_rows` 의 문서 끝 경계(빈 범위)까지만 다룹니다.

## 8. 실기 확인이 필요한 것

- [ ] 평문 파일을 열어 줄 번호·현재 줄 강조·캐럿·선택 영역이 이전 빌드와 같은 위치에 그려지는지
- [ ] 긴 줄에서 End·타이핑으로 가로 스크롤이 캐럿을 따라가는지, 세로·가로 스크롤바 드래그와 track 클릭이 이전과 같은지
- [ ] 빠른 열기·검색 결과 이동(`reveal`)이 대상 줄을 세로 중앙에, 캐럿을 가로로 보이게 두는지
- [ ] 한글 IME 조합 중 preedit 밑줄과 후보 창 위치가 스크롤된 상태에서도 캐럿에 붙는지
- [ ] 수만 줄 파일에서 스크롤이 이전과 같은 체감 속도인지
- [ ] 브라우저 클라이언트(`taide-remote-web`)의 편집기 화면이 이전과 같은지
