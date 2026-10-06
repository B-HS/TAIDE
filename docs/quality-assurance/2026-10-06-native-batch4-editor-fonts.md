# Native 배치 4 단계 2 — 편집기 글꼴 체인과 줄 상자 정렬 (2026-10-06)

상태: N1~N4 구현과 검증 계약(V-E, V-U, V-UL, V-A, V-AL, V-W, V-F)이 전부 exit 0 입니다. 화면이 바뀌는 단계입니다(본문·줄 번호 글리프의 세로 위치, 홀수 글꼴 크기의 줄 높이, 편집기 글꼴). 설계 문서·지시와 다르게 구현한 점 6건은 3절에, 메인 판단이 필요한 사항은 7절에 있습니다. GUI 실행은 하지 않았습니다.

경로는 저장소 루트(`/Users/hyunseokbyun/development/TAIDE`) 기준입니다. `MONACO` 는 `node_modules/monaco-editor/esm/vs`, `EPAINT` 는 `/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/epaint-0.36.2/src`, `EGUI` 는 `native/taide-native-app/vendor/egui-input/src`, `FONTDB` 는 `/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/fontdb-0.24.0/src/lib.rs`, `ES` 는 `native/taide-native-ui/src/editor_surface.rs`, 설계 문서는 `docs/research/2026-10-06-native-editor-display-layer-design.md` 입니다.

## 1. 기준 확인

직접 읽어 확인한 소스입니다.

| 대상 | 확인한 사실 | 근거 |
| --- | --- | --- |
| TS 글꼴 스택 | 사용자 글꼴(있을 때) → `ui-monospace` → `SFMono-Regular` → `Menlo` → `"Apple SD Gothic Neo"` → `monospace` | `src/shared/lib/font-stack.ts:1-8` |
| TS 가 넘기는 글꼴 옵션 | `fontFamily`, `fontSize`, `fontLigatures` 뿐. `lineHeight`·`letterSpacing`·`fontWeight` 는 넘기지 않음 | `src/shared/lib/code-editor-settings.ts:43-65`, `src/features/editor/code-editor.tsx:183-184, 335-341` |
| 설정 모델의 글꼴 필드 | `editor_font_size`, `editor_font_family`, `terminal_font_family`, `ui_font_family`, `editor_font_ligatures`. 줄 높이·자간 필드 없음 | `crates/taide-model/src/settings.rs:88-89, 103-107, 141` |
| Monaco 줄 높이 | `lineHeight === 0` 이면 `GOLDEN_LINE_HEIGHT_RATIO * fontSize`(macOS 1.5), 그 뒤 `Math.round`, 8 미만이면 8 | `MONACO/editor/common/config/fontInfo.js:12, 16, 22-33` |
| Monaco 글꼴 기본값 | `fontWeight: 'normal'`, `lineHeight: 0`, `letterSpacing: 0` | `MONACO/editor/common/config/fontInfo.js:164-170` |
| 본문 줄 상자 | 줄 div 에 `top`, `height: lineHeight`, `line-height: lineHeight` 를 함께 지정. CSS 는 글꼴 content area 위아래에 같은 half-leading 을 둠 | `MONACO/editor/browser/viewParts/viewLines/viewLine.js:134-139, 165-170`, `MONACO/editor/browser/config/domFontInfo.js:7-26` |
| 여백(줄 번호) 줄 상자 | 여백 overlay 도 같은 글꼴 정보와 `height`·`line-height` 를 받음 | `MONACO/editor/browser/view/viewOverlays.js:135-137, 184, 188` |
| 캐럿 | `cursorHeight` 기본값 0 이면 줄 높이 전체. `top = 줄 위쪽 + (lineHeight - 캐럿 높이) / 2` 이므로 줄 상자 전체를 덮음 | `MONACO/editor/browser/viewParts/viewCursors/viewCursor.js:121-125, 151`, `MONACO/editor/common/config/editorOptions.js:3176` |
| macOS textarea(IME 기준 상자) | 주 캐럿 줄의 위쪽에 `height: lineHeight` 로 배치. 조합 중에도 `height: lineHeight` | `MONACO/editor/browser/controller/editContext/textArea/textAreaEditContext.js:576-598, 613, 620-632` |
| egui 글꼴 교체 | `Context::set_fonts` 는 다음 pass 에서 정의 전체를 교체("This will overwrite the existing fonts") | `EGUI/context.rs:652-656, 2247-2268` |
| 등록되지 않은 패밀리 | `FontFamily::Name` 이 정의에 없으면 panic(`is not bound to any fonts`) | `EPAINT/text/fonts.rs:1021-1025` |
| egui 줄 높이와 세로 배치 | 줄 높이 = `ascent - descent + line_gap`(체인의 첫 face). 글리프는 줄 위쪽 + ascent 에 놓이고 가운데 정렬 없음 | `EPAINT/text/font.rs:563-587, 691-703`, `EPAINT/text/fonts.rs:865-875`, `EPAINT/text/text_layout.rs:446-449, 953-994` |
| galley hit-test 의 세로 여유 | 첫 줄 위 5pt, 마지막 줄 아래 5pt 를 벗어나면 x 와 무관하게 galley 시작·끝을 돌려줌 | `EPAINT/text/text_layout_types.rs:1181-1194` |
| 가변 축 지정 | `FontData.tweak.coords` 에 `(tag, 값)` 을 넣으면 그 face 의 기본 위치가 됨. 축 목록은 `FontData::variation_axes` | `EPAINT/text/fonts.rs:153-173`, `EPAINT/text/text_layout_types.rs:418-440`, `EPAINT/text/font.rs:574-578` |
| fontdb 질의 | 패밀리 → stretch → style → weight 순의 CSS 방식 매칭. 일반 `monospace` 의 기본 패밀리는 `Courier New` | `FONTDB:190, 663-679, 1212-1300` |
| 기존 터미널 체인 | 사용자 글꼴 → `ui_monospace` → `SFMono-Regular`·`Menlo`·`Apple SD Gothic Neo` → 일반 monospace → egui 기본 monospace. 같은 정의에 UI 글꼴을 이어 붙임 | `native/taide-native-app/src/terminal_fonts.rs`(`prepare_requested`), `native/taide-native-app/src/ui-fonts.rs:23-105` |

## 2. 항목별 변경

### N1. 편집기 글꼴 체인

- 신규 `native/taide-native-app/src/editor-fonts.rs`
    - `family()`: 편집기 패밀리 `FontFamily::Name("taide-editor")`.
    - `Families { terminal, editor }` 와 `Families::new(settings)`: `terminal_font_family`·`editor_font_family` 를 기존 `terminal_fonts::requested` 로 검증한 요청 쌍. 글꼴 로더의 키입니다.
    - `prepare(database, definitions, remaining, requested, shared)`: 편집기 체인을 같은 `FontDefinitions` 에 등록하고 경고 목록을 돌려줍니다. 선택 순서는 사용자 글꼴(`named`) → `ui_monospace` → `FONT_FALLBACKS`(`SFMono-Regular`, `Menlo`, `Apple SD Gothic Neo`) → `Family::Monospace` → egui 기본 monospace 체인이며 TS 스택과 같습니다.
    - `Registry::face(id, weight, name)`: face 하나를 등록합니다. 이미 등록된 face(터미널 체인이 적재한 정적 face 포함)는 바이트를 다시 읽지 않고 이름만 공유합니다. 새로 읽을 때는 기존 `terminal_fonts::face_data` 의 검증과 공유 바이트 예산(`remaining`)을 그대로 씁니다.
- `native/taide-native-app/src/terminal_fonts.rs`
    - 로더 키를 `Requested` 에서 `Families` 로 바꿨습니다(`Pending.families`, `Loader.attempted`, `Loader::new`, `Loader::update`, `load`, `prepare_requested`). 비동기 적재·취소·오래된 응답 폐기 구조는 그대로입니다.
    - `prepare_requested` 가 터미널 체인에서 적재한 `(face id, 글꼴 이름)` 목록을 모아 `ui_fonts::prepare` 다음에 `editor_fonts::prepare` 를 부릅니다.
    - 공개 범위: `FONT_FALLBACKS`, `ui_monospace`, `prepare_requested` 를 `pub(crate)` 로 넓혔습니다.
    - 기존 테스트는 설정 줄만 바꿨습니다(`families(..)` 도우미로 편집기 요청 `Default` 를 붙임). 단언은 그대로입니다.
- `native/taide-native-app/src/application.rs`
    - 시작 시 `Families::new(&settings)` 로 한 번 적재하고(`terminal_fonts::load`), `background_tick` 이 매 프레임 `Families::new` 결과를 `Loader::update` 에 넘깁니다. `editorFontFamily` 가 바뀌면 키가 달라져 다시 적재되고 완료 시 `context.set_fonts` 로 적용됩니다.
- `native/taide-native-app/src/presentation-refresh.rs`
    - `Appearances::new` 가 `editor.font.family = editor_fonts::family()` 를 지정합니다(터미널의 `terminal.font.family = terminal_fonts::family()` 와 같은 자리, 같은 방식).
- `native/taide-native-app/src/lib.rs`: `editor_fonts` 모듈 연결(`#[path]`).

### N2. 굵은 face

- `native/taide-native-ui/src/font-families.rs`: `EDITOR_FAMILY = "taide-editor"`, `EDITOR_BOLD_FAMILY = "taide-editor-bold"`.
- `editor-fonts.rs` 의 `prepare`
    - 일반 체인의 각 자리마다 `bold_face` 로 같은 패밀리·stretch·style 의 굵기 700 face 를 찾아 `taide-editor-bold` 패밀리에 같은 순서로 등록합니다(CSS 가 굵은 글씨에 같은 font-family 목록을 굵기 700 으로 푸는 방식).
    - 가변 글꼴(`wght` 축 보유)은 `tweak.coords` 에 `wght` 를 넣습니다. 일반 패밀리는 400, 굵은 패밀리는 700 이며 굵기마다 별도 항목입니다(`ui-fonts.rs:64-74` 와 같은 방식).
    - 정적 글꼴에 굵은 face 가 따로 있으면 그 face 를 새 항목으로 등록하고, 없으면 일반 face 의 이름을 굵은 패밀리가 공유합니다.
    - 예산 순서는 일반 체인 전체가 먼저, 굵은 face 가 나중입니다. 굵은 face 가 거절되면 경고를 남기고 그 자리에 일반 face 를 넣어 글리프 범위(한글 등)를 유지합니다.
- 이번 단계에서 굵은 패밀리를 그리는 코드는 없습니다. `EditorDisplayOptions.bold_family` 는 그대로 `None` 입니다.

### N3. 줄 상자 정렬

- `native/taide-native-ui/src/editor-geometry.rs`
    - `half_leading(painter, appearance)`: `round((line_height - 글꼴 줄 높이) / 2)`. 글꼴 줄 높이는 `fonts.row_height(&appearance.font)` 입니다(설계의 식).
    - `Row.half_leading` 필드와 `Row::text_origin()`(줄 상자 원점 + half-leading). `Row::layout` 이 half-leading 을 인자로 받습니다.
    - `Row::caret_rect`: galley 기준 캐럿 rect 를 `text_origin` 으로 옮깁니다(글리프 상자를 따라감).
    - `Row::byte_at`: 포인터의 x 만 씁니다(y 는 galley 중앙으로 고정). 표시 줄은 `VerticalLayout::row_at` 이 이미 골랐기 때문입니다.
- `native/taide-native-ui/src/editor-paint.rs`
    - `Layers::text`·`line_number`: `row.text_origin()` 에 그립니다.
    - `Layers::caret`: 줄 상자 위쪽(`row.origin.y`)부터 `line_height` 까지 명시적으로 그립니다(결과는 이전과 같은 줄 상자 전체).
    - 현재 줄 강조와 선택 영역은 줄 상자 기준 그대로입니다.
- `ES`: `show_presented` 가 프레임당 한 번 `half_leading` 을 구해 `Row::layout` 에 넘기고, `reveal` 도 같은 함수를 씁니다.
- 결과: 글꼴 14, 줄 높이 20, 번들 Hack(줄 높이 16)에서 본문·줄 번호·preedit 글리프와 IME `cursor_rect` 가 2pt 내려갑니다. 캐럿 선·선택·현재 줄 강조는 그대로입니다.

### N4. 글꼴 관련 설정 필드 반영 확인

| 필드(설계 3.1) | TS·Monaco 기준 | native 상태 |
| --- | --- | --- |
| editorFontSize | `fontSize` | 반영돼 있음(`editor_appearance`, 매 프레임 `update_editor_font_size`). 줄 높이 계산만 아래와 같이 고침 |
| editorFontFamily | `fontFamily`(스택) | N1 에서 연결 |
| 줄 높이 | TS 미지정 → Monaco `round(1.5 * fontSize)`, 최소 8 | `font_size * 1.5` 를 반올림 없이 쓰고 있었음 → 고침 |
| 자간 | TS 미지정 → Monaco 0 | egui 기본 `extra_letter_spacing` 0. 변경 없음 |
| 굵기 | TS 미지정 → Monaco `normal` | 일반 체인은 굵기 400 질의, 가변 글꼴은 `wght` 400 |
| editorFontLigatures | `fontLigatures` | 비목표(D4). 건드리지 않음 |

- `editorLineHeight`·`editorLetterSpacing` 설정 필드는 TS 에도 Rust 설정 모델에도 없습니다(1절).
- `native/taide-native-ui/src/presentation.rs`: `editor_line_height(font_size)` = `round(font_size * 1.5).max(8)` 를 추가하고 `editor_appearance` 와 `update_editor_font_size` 가 함께 씁니다. 기본 글꼴 크기 13 의 줄 높이가 19.5 에서 Monaco 와 같은 20 이 됩니다. 짝수 크기는 값이 같습니다.

## 3. 설계 문서·지시와 달라진 점

| # | 설계·지시 | 구현 | 이유 |
| --- | --- | --- | --- |
| 1 | `presentation.rs` 의 `editor_appearance` 가 편집기 패밀리를 지정 | `editor_appearance` 는 `FontId::monospace` 그대로. 앱의 `Appearances::new`(`presentation-refresh.rs`)가 패밀리를 지정 | `editor_appearance` 는 동결된 `taide-remote-web` 도 부릅니다(`native/taide-remote-web/src/presentation.rs:92`, `browser-editor.rs:1107, 1194`). 그 클라이언트는 `taide-editor` 패밀리를 등록하지 않고, epaint 는 등록되지 않은 패밀리에서 panic 합니다(`EPAINT/text/fonts.rs:1021-1025`). |
| 2 | `terminal_fonts.rs` 는 공개 범위만 변경 | 로더 키를 `Families` 로 바꾸고 `prepare_requested` 에 편집기 등록 호출과 적재 face 목록을 추가 | `Context::set_fonts` 는 정의 전체를 교체하므로(`EGUI/context.rs:2247-2268`) 편집기 전용 로더를 따로 두면 터미널 로더가 끝날 때마다 편집기 패밀리가 사라지고, 사라진 프레임에 1항의 panic 이 납니다. 한 파이프라인이 터미널·UI·편집기를 함께 만들고 로더 키가 두 설정을 모두 담아야 합니다. `ui-fonts.rs` 가 이미 같은 방식으로 이 파이프라인에 붙어 있습니다. |
| 3 | 수정 범위에 `presentation-refresh.rs` 없음 | 3줄 + 테스트 단언 1줄 수정 | 1항의 지정 지점입니다. `application.rs` 에서 하면 `Appearances::new` 호출부 3곳(시작, 테마 갱신, 테마 미리보기)에 같은 줄을 반복해야 합니다. |
| 4 | 설계 3.1 의 줄 높이는 `1.5 * fontSize`, 최소 8 | `Math.round` 를 포함해 구현 | 설계 표가 `fontInfo.js:30` 의 반올림을 빠뜨렸습니다. 직접 읽어 확인했습니다. |
| 5 | 수정 범위에 `taide-remote-web` 없음 | `native/taide-remote-web/tests/presentation.rs:86-89` 의 기대식에 `.round()` 추가 | 4항으로 공유 함수의 값이 바뀌어 이 테스트가 실패했습니다(기본 13 에서 20.0 대 19.5). 기대값을 Monaco 식으로 맞췄습니다. 소스·의존성은 건드리지 않았습니다. |
| 6 | 세로 오프셋만 변경 | `Row::byte_at` 이 y 를 쓰지 않도록 함께 변경 | galley 를 줄 가운데로 옮기면 half-leading 이 5pt 를 넘는 큰 글꼴에서 줄 위쪽 가장자리 클릭이 `cursor_from_pos` 의 5pt 여유를 벗어나 줄 시작으로 튑니다. 아래쪽 가장자리는 옮기기 전에도 줄 높이와 글꼴 줄 높이의 차가 5pt 를 넘으면 줄 끝으로 튀었습니다(글꼴 48 로 구현 전에 재현, 5절). |

그 밖에 설계와 같지만 선택이 있었던 점입니다.

- 설계 식의 "글꼴 줄 높이"로 `fonts.row_height`(ascent − descent + line gap)를 썼습니다. CSS 의 content area 는 line gap 을 포함하지 않으므로 hhea line gap 이 0 이 아닌 글꼴에서는 글리프가 `line gap / 2` 만큼 위에 놓입니다. epaint 공개 API 는 ascent·descent 를 따로 주지 않습니다.
- IME `cursor_rect` 는 galley 를 따라 half-leading 만큼 내려갑니다(글리프 상자). Monaco 의 textarea 는 줄 상자 전체입니다(1절). 줄 상자로 바꾸는 것은 WebKit 이 textarea 캐럿 rect 를 어떻게 보고하는지 확인할 수 없어 이번에 하지 않았습니다(7절 3항).
- `font-families.rs` 에는 이름 상수만 두고 `medium(ui)` 같은 가용성 확인 함수는 추가하지 않았습니다. 이번 단계에 호출부가 없습니다.
- 편집기 체인의 face 선택은 `editor-fonts.rs` 에 따로 있고 터미널의 선택 코드(`prepare_requested` 앞부분)와 내용이 겹칩니다. `terminal_fonts.rs` 변경을 줄이기 위해 합치지 않았습니다.

## 4. 설계 문서 서술·줄 번호 대조

- 설계 2.1 의 `presentation.rs:18, 210-211`(monospace 고정, 줄 높이 `font_size * 1.5`)은 착수 시점 파일과 일치했습니다.
- 설계 2.6 의 "현재 표면은 galley 를 줄 위쪽 가장자리에 그림(ES 680)"은 단계 1 뒤 `editor-paint.rs` 의 `Layers::text` 로 옮겨져 있었습니다. 서술 내용은 맞습니다.
- 설계 7절 단계 2 의 `ui-fonts.rs:64-74`, `terminal_fonts.rs` 로더 서술은 실제 파일과 일치했습니다.
- 설계 3.1 의 줄 높이 줄은 반올림(`fontInfo.js:30`)이 빠져 있습니다(3절 4항).
- 설계 4.8 은 브라우저 클라이언트가 "지금과 같은 화면"이라고 했지만, 표면과 `editor_appearance` 를 공유하므로 이 단계의 세로 정렬과 줄 높이 반올림은 브라우저 클라이언트에도 적용됩니다. 편집기 글꼴 체인은 적용되지 않습니다(패밀리가 monospace 그대로).
- 설계 7절 단계 2 는 `editor_appearance` 가 패밀리를 바꾼다고 했으나 그대로 하면 브라우저 클라이언트가 panic 합니다(3절 1항).

## 5. 검증

공통 접미사는 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` 입니다.

구현 전(실패 확인):

| 명령 | 결과 |
| --- | --- |
| `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` | exit 101, 16 passed / 7 failed. 신규 3건(세로 가운데 `0.0 != 2.0`, 줄 높이 `19.5 != 20.0`, 큰 글꼴 클릭)과 기대값을 고친 특성화 4건 |
| 같은 명령 `-- 큰_글꼴에서` | exit 101. 줄 아래쪽 가장자리 클릭이 줄 끝 `(13, 13)`, 기대 `(10, 10)` |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- editor_fonts` | exit 101, 1 passed / 4 failed(`prepare` 가 비어 있는 상태) |

구현 후(최종 작업 트리):

| 기호 | 명령 | 결과 |
| --- | --- | --- |
| V-E | `cargo test --manifest-path native/taide-native-editor/Cargo.toml` | exit 0, 48 passed, 대상별 0.00s |
| V-U | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` | exit 0, 23 passed(단계 1 의 20 + 신규 3), 0.05s |
| V-UL | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib` | exit 0, 116 passed, 0.51s |
| V-A | `cargo check --manifest-path native/taide-native-app/Cargo.toml` | exit 0. 경고는 vendored `wry` 17건뿐(기존) |
| V-AL | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` | exit 0, 352 passed(기준 347 + 신규 5), 14.50s |
| V-W | `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | exit 0, 1.23s, 경고 없음 |
| V-F | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | exit 0 |
| V-F | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | exit 0(처음 exit 1, 6절) |
| V-F | `cargo fmt --manifest-path native/taide-remote-web/Cargo.toml -- --check` | exit 0 |

계약 밖에서 추가로 실행한 것:

| 명령 | 결과 |
| --- | --- |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- editor_fonts` | exit 0, 5 passed, 3.77s |
| `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test presentation` (기대식 수정 전) | exit 101, 1 failed(`20.0 != 19.5`) |
| `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test presentation --test files` (수정 후) | exit 0, presentation 3 passed, files 12 passed |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --test paste-shortcuts --test save` | exit 0, 2 passed / 1 passed |
| `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` | exit 0, lib 경고 9(단계 1 과 같은 9건), 테스트 파일 경고 0 |
| `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --lib` | exit 0, 앱 크레이트 경고 0 |
| `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --lib --tests` | exit 0, 경고 2(`src/terminal-input-tests.rs:267`, `tests/explorer.rs:1840`. 이 단계가 건드리지 않은 파일) |

실행하지 않은 것: `taide-native-app` 의 `tests/terminal-host.rs`, `taide-remote-web` 의 나머지 테스트와 wasm32 대상 빌드, GUI 실행, 실제 시스템 글꼴을 읽는 측정.

신규 테스트:

- `native/taide-native-app/src/editor-fonts.rs`(합성 글꼴 데이터베이스. egui 번들 Hack 바이트에 뒤 패딩을 달리 붙여 face 를 구별하고, 가변 글꼴은 `fvar` 표를 합성)
    - `editor_fonts는_사용자_글꼴과_ts_fallback_순서로_체인을_만들고_굵은_face를_별도_패밀리에_등록한다`: 일반 체인 순서 `[사용자, SF Mono, Menlo, Apple SD Gothic Neo, 일반 monospace]`, 굵은 체인 `[사용자 Bold, SF Mono, Menlo Bold, Apple SD Gothic Neo, 일반 monospace]`, 굵은 face 가 없는 자리의 이름 공유, 예산 정확히 소진, 기본 Monospace·Proportional 불변, 실제 렌더.
    - `editor_fonts는_없는_글꼴과_예산_부족을_경고하고_남은_체인으로_대체한다`: 없는 글꼴·잘못된 이름·미지정, 굵은 face 만 거절되는 예산, 예산 0.
    - `editor_fonts는_가변_글꼴의_굵기를_wght_축으로_고정해_굵기별_face로_등록한다`: `wght` 400·700 항목.
    - `editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`: 전체 파이프라인(`prepare_requested`)에서 이름 공유와 항목 수.
    - `editor_fonts는_편집기_글꼴_설정이_바뀌면_같은_로더에_다시_적재를_요청한다`: `Families::new` 와 로더의 재적재 시도.
- `native/taide-native-ui/tests/editor_surface.rs`
    - `본문과_줄_번호의_글리프는_줄_상자의_세로_가운데에_놓이고_캐럿은_줄_상자_전체를_덮는다`: 줄 높이 20·27·12 에서 글리프 위쪽 = 줄 위쪽 + `round((줄 높이 - 글꼴 줄 높이) / 2)`, 위·아래 여백 차 1 이하, 캐럿 선은 줄 상자 전체.
    - `큰_글꼴에서_줄_상자의_위아래_가장자리_클릭은_클릭한_열에_캐럿을_둔다`: 글꼴 48 에서 줄 상자 위·아래 1pt 안쪽 클릭.
    - `편집기_줄_높이는_글꼴_크기의_1_5배를_정수로_반올림하고_최소값을_지킨다`: 13→20, 14→21, 15→23, 12→18, 4→8.
- `presentation-refresh.rs` 의 기존 테스트에 편집기 패밀리 단언 1줄 추가.

고친 기존 기대값(Monaco 근거: `viewLine.js:134-139`, `viewOverlays.js:135-137` 의 줄 상자와 `viewCursor.js:121-125, 151` 의 캐럿):

- `PLAIN_FRAME`, `SELECTION_FRAME`, `SCROLLED_COMPOSITION_TRACE` 의 본문·줄 번호·preedit(배경 rect, 글자, 밑줄) y 좌표와 IME `cursor` 의 y 범위를 +2.00 했습니다. 캐럿 선, 선택 rect, 현재 줄 rect, 스크롤바, 스크롤·선택 상태 줄은 그대로입니다.

## 6. 실패했다가 고친 내역

- `tests/editor_surface.rs` 의 `measure_with` 가 `FnMut` 클로저 안에서 `FontId` 를 옮겨 컴파일 오류(E0507)가 났습니다. `font.clone()` 으로 고쳤습니다.
- 큰 글꼴 클릭 테스트가 처음에는 `(0, 0)` 으로 실패했습니다. 클릭 직전 프레임이 편집기를 그리지 않는 폭 측정 프레임이라 egui 가 직전 pass 의 위젯 rect 로 하는 hit-test 에 편집기가 없었기 때문입니다. 측정을 먼저 하고 편집기 프레임 뒤에 클릭하도록 순서를 고쳤고, 그 뒤 의도한 원인(아래쪽 가장자리 클릭이 줄 끝으로 감)으로 실패하는 것을 확인했습니다.
- `cargo fmt -- --check` 가 `editor-fonts.rs` 4곳을 지적해 손으로 맞췄습니다. 그 뒤 V-AL·V-A 를 실행했습니다.
- `taide-remote-web` 의 `tests/presentation.rs` 가 줄 높이 반올림으로 실패했습니다(3절 5항).
- 구현을 고쳐 다시 돌린 경우는 없었습니다. 각 구현은 첫 실행에서 통과했습니다.

## 7. 남은 위험과 메인 판단이 필요한 사항

1. `terminal_fonts.rs` 를 공개 범위 이상으로 고쳤고(3절 2항) `presentation-refresh.rs`·`taide-remote-web/tests/presentation.rs` 는 지정 범위 밖입니다(3절 3·5항). 되돌려야 한다면 대안은 줄 높이 반올림을 빼는 것(5항)과 패밀리 지정을 `application.rs` 3곳으로 옮기는 것(3항)입니다. 2항은 대안이 없습니다.
2. 동결된 브라우저 클라이언트의 화면이 바뀝니다. 글리프가 줄 가운데로 내려가고, 홀수 글꼴 크기의 줄 높이가 정수가 됩니다. 표면과 `editor_appearance` 를 공유하는 구조에서 피할 수 없습니다.
3. IME `cursor_rect` 는 글리프 상자입니다. Monaco 의 textarea 는 줄 상자 전체이므로 후보 창이 TS 보다 half-leading 만큼(14/20 에서 2pt) 위에 뜰 수 있습니다. 설계 4.7 의 IME 단계에서 줄 상자로 바꿀지 정해야 합니다.
4. 굵은 face 가 없는 정적 글꼴은 굵은 패밀리에도 일반 face 가 들어갑니다. 브라우저는 합성 굵기를 그리지만 epaint 에는 없습니다. 구문 강조 단계에서 굵은 토큰이 굵게 보이지 않는 글꼴이 생깁니다.
5. 메모리와 시작 시간. `face_data` 는 face 하나에 글꼴 파일 전체를 읽습니다(컬렉션 파일이면 컬렉션 전체). 일반 체인은 터미널과 공유해 추가 바이트가 없지만(사용자 글꼴이 다를 때만 추가), 굵은 체인은 별도 굵은 face 가 있는 자리마다 그 파일을 한 번 더 읽습니다. 총 예산(128MB)을 넘으면 굵은 face 부터 거절되고 일반 face 로 대체됩니다. 실제 시스템 글꼴의 파일 크기와 적재 시간은 저장소 밖이라 측정하지 않았습니다.
6. 터미널 체인은 가변 글꼴에 `wght` 를 지정하지 않습니다(기존 동작). 편집기는 400 을 지정하므로 기본 위치가 400 이 아닌 가변 글꼴은 터미널과 편집기의 굵기가 다르게 보일 수 있습니다. 터미널 글꼴은 비목표라 건드리지 않았습니다.
7. `Appearances::new` 가 만든 편집기 외형은 `taide-editor` 패밀리가 등록된 egui context 에서만 그릴 수 있습니다(터미널 외형과 같은 전제). 앱은 첫 프레임 전에 `set_fonts` 를 하고 이후 적재도 항상 이 패밀리를 포함합니다. 글꼴 정의 없이 이 외형으로 편집기를 그리는 테스트를 새로 쓰면 panic 합니다.
8. half-leading 의 반올림 규칙과 line gap 처리(3절)는 WebKit 의 실제 배치와 1pt 안에서 다를 수 있습니다. 한글 등 폴백 face 의 글리프는 epaint 가 주 글꼴 줄 안에서 가운데로 맞추므로(`EPAINT/text/text_layout.rs:982-984`) 라틴 글자와 기준선이 다를 수 있습니다.
9. macOS 외 줄 높이 비율 1.35, 리거처, 컬러 이모지는 지시대로 건드리지 않았습니다.

## 8. 실기 확인이 필요한 것

- [ ] 한글이 든 파일에서 한글이 대체 글리프가 아니라 Apple SD Gothic Neo 로 보이는지, 라틴 글자와 기준선이 어색하지 않은지
- [ ] 설정에서 편집기 글꼴을 바꾸면 다시 시작하지 않아도 편집기에 적용되는지, 없는 글꼴 이름을 넣으면 폴백으로 보이는지
- [ ] 편집기 글꼴과 터미널 글꼴을 서로 다르게 지정했을 때 각각 유지되는지
- [ ] 본문·줄 번호 글리프가 줄 상자의 세로 가운데에 있고 TS 화면과 같은 위치인지(글꼴 크기 12·13·14·16)
- [ ] 글꼴 크기 13 에서 줄 간격이 20 으로 고르고 긴 파일의 스크롤 높이가 TS 와 같은지
- [ ] 글꼴 크기를 크게(24 이상) 했을 때 줄의 위·아래 가장자리 클릭이 클릭한 열에 캐럿을 두는지
- [ ] 한글·일본어 IME 조합 중 preedit 가 본문 글자와 같은 높이에 있고 후보 창이 글자를 가리지 않는지
- [ ] 시작 시간과 메모리가 이전 빌드와 비교해 눈에 띄게 늘지 않는지, 로그에 `native editor ... font` 경고가 없는지
- [ ] 굵은 글씨(구문 강조 단계 뒤): 정적 글꼴의 굵은 face, 가변 글꼴의 `wght` 700, 한글 굵은 글씨
- [ ] 브라우저 클라이언트(`taide-remote-web`)의 편집기 화면이 바뀐 세로 정렬로 정상 표시되는지
