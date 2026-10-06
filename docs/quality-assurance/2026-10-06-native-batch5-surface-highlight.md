# Native 배치 5 단계 3/3 — 편집기 표면의 구문 강조 연결과 저장 정리 공급 (2026-10-07)

## 0. 먼저 알아야 할 것 — 사용자 파일이 실제로 바뀝니다

이 단계부터 `trimTrailingWhitespaceOnSave`(또는 `.editorconfig` 의 `trim_trailing_whitespace = true`)가 켜진 상태에서 plaintext 가 아닌 파일을 저장하면 후행 공백이 실제로 지워집니다. 지금까지 native 는 `install_syntax` 호출부가 없어 plaintext 외 파일에서는 한 줄도 지우지 않았습니다.

| 경우 | 저장 시 동작 |
| --- | --- |
| 설정이 꺼져 있음(기본값) | 변화 없음. 토큰 스냅샷도 만들지 않습니다 |
| 번들 31개 언어, 정확히 토큰화된 줄 | 후행 공백 시작 위치의 토큰이 문자열·정규식이면 남기고, 그 외(코드·주석)는 지웁니다 |
| 같은 문서에서 아직 다시 토큰화되지 않은 줄(첫 무효 줄부터 아래) | 그 저장에서는 건너뜁니다 |
| 토큰화 한도(20MB 또는 30만 줄)를 넘어 토큰화하지 않는 문서 | 모든 줄의 후행 공백을 지웁니다(문자열 안 포함). Monaco 가 그렇게 동작합니다. 3절 H3 과 8절 2번을 확인해 주십시오 |
| 번들 밖 언어(플러그인 문법 등), 토큰 테마가 한 번도 적용되지 못한 상태 | 이전과 같이 지우지 않습니다. TS 와 다릅니다(7절) |

상태: H1~H4 구현 완료. 검증 계약 V1, V2, V3, V5, V6 과 V4 의 `cargo check`·통합 테스트는 exit 0 입니다. V4 의 `cargo test --lib` 는 exit 101 입니다(365건 중 355 통과, 10 실패). 실패 10건은 앞 단계와 같은 것으로, 미리 빌드된 예제 실행 파일 `target/debug/examples/native-lsp-mock` 이 없어서 납니다(9절 1번). 이번 변경이 닿는 코드를 쓰지 않습니다. GUI 는 실행하지 않았습니다.

기준 시점은 HEAD `cffd0a09` 이고 앞 두 단계의 미커밋 변경 위에서 작업했습니다. 경로는 저장소 루트 기준입니다. `MONACO` 는 `node_modules/monaco-editor/esm/vs`, `EPAINT` 는 `/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/epaint-0.36.2/src`, `ES` 는 `native/taide-native-ui/src/editor_surface.rs`, `APP` 은 `native/taide-native-app/src/application.rs` 입니다.

## 1. 한눈에 보는 결과

| 항목 | 결과 |
| --- | --- |
| H1 표면 색칠 | `RowText::highlight` 가 토큰 run 을 표시 문자 구간으로 바꾸고 `styled_layout_job` 이 색·굵기·기울임·밑줄·취소선을 `LayoutJob` 섹션으로 만듭니다. 신규 UI 테스트 7건 |
| H2 조율자 연결 | `show_document` 가 입력 처리 뒤에 조율자에서 현재 revision 의 토큰을 빌려 그립니다. 보이는 줄 범위를 조율자에 알립니다. 신규 단위 테스트 4건 |
| H3 저장 정리 공급 | 저장 직전에 정확한 줄까지의 `SyntaxSnapshot` 을 `install_syntax` 에 넣습니다. 신규 통합 테스트 3건, 편집기 크레이트 테스트 2건 |
| H4 설정 반영 | 편집기 굵은 글꼴 패밀리를 표시 옵션으로 연결했습니다. 구문 강조를 켜고 끄는 설정은 TS 에 없습니다. 신규 단위 테스트 1건 |
| 화면 변화 | 번들 언어 문서가 처음으로 색이 입혀집니다. 평문 문서와 브라우저 클라이언트는 그대로입니다 |

## 2. 기준 확인

| 대상 | 확인한 사실 | 근거 |
| --- | --- | --- |
| TS 저장 정리 | 저장 때 Monaco 내장 액션 `editor.action.trimTrailingWhitespace` 를 실행합니다. 자동 저장이면 `reason: 'auto-save'` 를 넘깁니다 | `src/shared/lib/monaco/on-save-cleanup.ts:12, 61-69`, `src/shared/lib/monaco/monaco-actions.ts:23`, `src/widgets/editor-pane/use-editor-file-persistence.ts:397-399` |
| 문자열·정규식 보존 여부 | 액션이 `files.trimTrailingWhitespaceInRegexAndStrings` 설정을 읽습니다. standalone Monaco 에는 이 설정이 없어 값이 없고(거짓), 따라서 토큰을 봅니다 | `MONACO/editor/contrib/linesOperations/browser/linesOperations.js:397-400` |
| 줄별 규칙 | 토큰이 정확하지 않은 줄은 건너뜁니다(강제 토큰화 없음). 정확한 줄은 `findTokenIndexAtOffset(fromColumn)` 위치의 표준 토큰 종류가 String(2)·RegEx(3) 이면 건너뛰고, 아니면 `max(커서 열, fromColumn)` 부터 줄 끝까지 지웁니다 | `MONACO/editor/common/commands/trimTrailingWhitespaceCommand.js:31-96` (토큰 판정 79-90) |
| "정확한 줄"의 뜻 | 첫 무효 줄보다 앞인 줄입니다 | `MONACO/editor/common/model/textModelTokens.js:86-89` |
| 토크나이저가 없을 때 | 문서가 토큰화 한도를 넘거나 그 언어의 토큰 공급자가 없으면 토크나이저를 만들지 않고, 이때 `hasAccurateTokensForLine` 은 항상 참입니다. 토큰은 기본값(종류 Other)이므로 모든 줄을 지웁니다 | `MONACO/editor/common/model/tokens/tokenizerSyntaxTokenBackend.js:70-94, 225-230` |
| native 판정 위치 | `(후행 공백 시작 바이트 + 1)` 을 줄 길이로 자른 위치의 토큰 종류가 Other·Comment 일 때만 지웁니다. Monaco 의 `fromColumn`(1부터 세는 열을 0부터 세는 오프셋으로 쓰는 것)과 같은 위치입니다. 후행 공백은 ASCII 라서 UTF-8 과 UTF-16 의 차이가 없습니다 | `native/taide-native-editor/src/save_cleanup.rs:91-112` |
| 토큰 글꼴 스타일의 CSS | `.mtki { font-style: italic }`, `.mtkb { font-weight: bold }`, `.mtku { text-decoration: underline; text-underline-position: under }`, `.mtks { text-decoration: line-through }`, 둘 다면 `underline line-through` | `MONACO/editor/common/languages/supports/tokenization.js:279-283` |
| 이어지는 줄의 들여쓰기 | 들여쓰기 부분은 토큰 종류 없이 그립니다("The faux indent part of the line should have no token type"). 토큰은 표시 줄 범위로 잘라 들여쓰기만큼 밀어 놓습니다 | `MONACO/editor/common/viewLayout/viewLineRenderer.js:341-352`, `MONACO/editor/common/viewModel/modelLineProjection.js:120-126` |
| egui 의 글꼴 스타일 | `TextFormat` 에 `italics`(쿼드 위쪽을 높이의 25% 만큼 민 합성 기울임), `underline`·`strikethrough`(직선 `Stroke`)가 있습니다. 밑줄은 글리프 논리 상자의 아래쪽, 취소선은 세로 가운데에 그립니다. 굵기 플래그는 없습니다 | `EPAINT/text/text_layout_types.rs:478-523`, `EPAINT/text/text_layout.rs:1079-1095, 1173-1199, 1208-1250` |
| 섹션 불변식 | 섹션은 본문 전체를 빈틈·겹침 없이 덮어야 합니다. 셰이핑은 섹션 단위입니다 | `EPAINT/text/text_layout_types.rs:53-60, 340-344` |
| 등록되지 않은 패밀리 | 글꼴 정의에 없는 `FontFamily::Name` 으로 그리면 epaint 가 panic 합니다. 같은 크레이트에 가용성을 먼저 확인하는 선례가 있습니다 | 배치 4 QA(`2026-10-06-native-batch4-editor-fonts.md` 1절), `native/taide-native-ui/src/font-families.rs:16-23` |
| 굵은 패밀리 | 앱의 글꼴 파이프라인이 `taide-editor` 와 `taide-editor-bold` 를 항상 함께 등록합니다 | `native/taide-native-app/src/editor-fonts.rs:193-199` |
| 구문 강조 관련 설정 | 설정 모델에 구문 강조를 켜고 끄는 필드가 없습니다. 관련 필드는 `editor_font_ligatures`, `editor_semantic_highlighting` 뿐이고 둘 다 이 단계 범위 밖입니다. TS 는 대형 파일에서 접기·괄호 색·minimap 만 끕니다 | `crates/taide-model/src/settings.rs:141, 328`, `src/features/editor/code-editor.tsx:182, 271, 275` |

## 3. 항목별 변경 내용

### H1. 표면 색칠 (`native/taide-native-ui`)

| 파일 | 내용 |
| --- | --- |
| `src/editor-row-text.rs` | `RowFontStyle`(기울임·굵기·밑줄·취소선), `RowTokens`(문서 줄의 span, 스타일 표, 표시 줄이 시작하는 줄 안 바이트), `RowText.font_styles`, `RowText::highlight`, `RowText::styled_layout_job`. `layout_job` 은 `styled_layout_job(font, None)` 으로 위임합니다 |
| `src/editor_surface.rs` | `EditorTokens { revision, lines, styles }`, `NativeEditor::show_tokenized`, `NativeEditor::reveal_tokenized`, 비공개 `EditorTokens::describes`·`registered_family`. `show_presented`·`reveal_presented` 는 토큰 없이 위임합니다 |
| `src/editor-geometry.rs` | `RowLayout` 에 `tokens`, `bold_family` 필드. `RowLayout::row` 가 토큰이 있으면 `highlight` 를 부르고 `styled_layout_job` 으로 galley 를 만듭니다(범위 밖 파일, 4절 5번) |
| `tests/editor_surface.rs` | 신규 7건과 도우미. 기존 33건의 본문은 건드리지 않았습니다(바뀐 기존 줄은 `use` 4줄뿐) |

`RowText::highlight` 의 규칙:

- 표시 줄이 덮는 줄 안 바이트 범위와 겹치는 토큰만 봅니다. 시작 토큰은 이진 탐색으로 찾습니다.
- 토큰 시작 바이트는 `RowText::display_char` 로 표시 문자가 됩니다. 탭은 펼친 공백 전체가 탭이 속한 토큰의 구간이 되고, 전각·보조 평면 문자는 문자 하나입니다. 바이트가 문자 중간이면 그 문자의 시작으로 내립니다.
- 이어지는 줄의 들여쓰기 공백은 스타일 표의 기본 스타일입니다(Monaco 의 faux indent).
- 첫 토큰 앞의 빈 구간과 토큰이 없는 줄은 스타일 표의 기본 스타일입니다. Monaco 가 토큰화되지 않은 줄을 기본 토큰(`mtk1`)으로 그리는 것과 같습니다.
- 색과 글꼴 스타일이 같은 이웃 구간은 하나로 합칩니다(표준 토큰 종류만 다른 토큰 포함).
- 빈 줄은 기본 스타일의 빈 구간 하나입니다.

`styled_layout_job` 의 규칙:

- 굵은 구간은 `FontId::new(같은 크기, 굵은 패밀리)` 입니다. 굵은 패밀리가 없으면 일반 글꼴 그대로입니다.
- 기울임은 `TextFormat.italics`, 밑줄·취소선은 폭 1pt·구간 전경색의 `Stroke` 입니다.
- 글꼴 스타일이 없는 단일 구간은 `LayoutJob::simple(text, font, color, INFINITY)` 와 같은 job 입니다. 토큰이 없는 화면이 한 픽셀도 바뀌지 않는 근거입니다.
- 구간 경계의 바이트 위치를 앞에서부터 이어 가며 구합니다(이전에는 구간마다 줄 처음부터 다시 셌습니다).

표면:

- `show_tokenized` 는 그 프레임의 입력 이벤트를 문서에 반영한 뒤에 호출자의 토큰 공급 함수를 부릅니다. 받은 토큰은 `revision` 과 줄 수가 그리려는 문서와 같을 때만 씁니다. 다르면 평문으로 그립니다.
- `registered_family` 가 굵은 패밀리의 등록 여부를 프레임마다 확인하고, 없으면 굵은 구간도 일반 글꼴로 그립니다.
- 선택·캐럿·현재 줄 강조·IME 좌표는 galley 좌표를 그대로 쓰므로 코드 변경이 없습니다. 굵은 글꼴의 폭이 다르면 galley 가 달라지고 좌표도 그에 맞게 따라갑니다. `reveal_tokenized` 도 같은 줄 구성을 써서 가로 스크롤을 굵은 글꼴 폭에 맞춥니다.
- `editor-paint.rs`, `presentation.rs`(UI 크레이트)는 바꿀 필요가 없어 건드리지 않았습니다.

### H2. 조율자 연결 (`native/taide-native-app`)

| 파일 | 내용 |
| --- | --- |
| `src/editor-syntax.rs` | `EditorSyntax::tokens`, `show_lines`, `supply_save_cleanup`, 비공개 `catch_up`·`follow_document`. `TrackedDocument.shown_lines`. `EditorSyntax` 와 `connect`·`tick`·`disconnect` 를 `pub` 으로 넓혔습니다 |
| `src/application.rs` | `AppSurfaces.editor_syntax` 필드와 생성. `show_document` 가 `reveal_tokenized`·`show_tokenized` 를 부르고 `show_lines` 로 그린 줄 범위를 알립니다. 저장 경로에 `supply_save_cleanup` 호출 1곳 |
| `src/lib.rs` | `mod editor_syntax` → `pub mod editor_syntax`(범위 밖 파일, 4절 6번) |
| `src/editor-syntax-tests.rs` | 신규 4건 |

- `tokens(store, document)`: 그 문서 하나를 스토어의 현재 revision 까지 따라간 뒤(`catch_up`) 토큰과 스타일 표를 빌려줍니다. 편집이 있었으면 저널로 토큰을 옮기고 재토큰화 작업을 바로 시작합니다. 다음 프레임의 `tick` 을 기다리지 않으므로, 편집 뒤에 더 그릴 프레임이 없어도 worker 가 깨워 다시 그리게 됩니다.
- 낡은 토큰이 그려지지 않는 경로:

| 상황 | 동작 |
| --- | --- |
| 편집·undo·redo(같은 프레임 포함) | 저널로 줄과 줄 안 토큰을 옮긴 토큰을 그 revision 으로 빌려줍니다. 저널이 끊기면 전체를 비웁니다 |
| 문서 전환 | 토큰은 문서 id 별입니다. 처음 보인 문서는 `tick` 없이도 추적을 시작하고, 응답 전에는 기본 스타일로 그립니다 |
| 문서 닫기 | 스토어에 없는 문서는 `None` 이고 그 자리에서 추적에서 뺍니다 |
| 테마 변경 | 새 설정의 응답을 받는 순간 스타일 표 교체와 토큰 비우기가 한 번에 일어납니다(앞 단계 `TokenPipeline`). 그 전까지는 이전 표와 이전 토큰이 짝을 이룹니다 |
| 언어 변경 | 토큰을 비우고 다시 받습니다. 번들 밖 언어가 되면 `None` 입니다 |
| 토큰화 한도 초과 문서 | 토큰 저장소가 0줄이라 표면의 줄 수 검사에서 걸러져 평문으로 그립니다 |

- `show_lines(document, lines)`: 표면이 그린 문서 줄 범위(`EditorOutput.rendered_lines`)를 받습니다. 같은 문서를 여러 pane 이 보이면 범위를 합쳐 두었다가 다음 `tick` 에서 한 번 `TokenPipeline::set_visible_lines` 로 넘깁니다. 프레임마다 pane 수만큼 worker 에 메시지를 보내지 않기 위함입니다.
- 응답이 온 프레임: worker 가 응답마다 `request_repaint` 를 부르고(앞 단계), 그 프레임의 `background_tick` 이 응답을 반영한 뒤 `show_document` 가 그립니다.

### H3. 저장 정리 공급

| 파일 | 내용 |
| --- | --- |
| `native/taide-native-editor/src/syntax.rs` | `SyntaxSnapshot::from_accurate_lines(revision, language_id, &LineTokens, &TokenStyleTable)`, `SyntaxSnapshot::without_tokenizer(revision, language_id, line_count)` |
| `native/taide-native-app/src/editor-syntax.rs` | `EditorSyntax::supply_save_cleanup(store, document, flags)` |
| `native/taide-native-app/src/application.rs` | 저장 경로에서 `crate::save::prepare` 직전에 호출 |
| `native/taide-native-editor/tests/save_cleanup.rs` | 신규 2건 |
| `native/taide-native-app/tests/save-syntax.rs` (신규) | 통합 테스트 3건 |

- `from_accurate_lines`: 0번 줄부터 `LineTokens::has_accurate_tokens` 가 참인 동안의 줄만 담습니다. 줄마다 span 의 스타일 id 를 스타일 표의 표준 토큰 종류로 바꾸고, 종류가 바뀌는 위치만 남깁니다. 첫 무효 줄부터는 넣지 않으므로 `save_cleanup` 이 그 줄들을 건너뜁니다.
- `supply_save_cleanup`: 설정(`.editorconfig` 값이 있으면 그 값)이 꺼져 있으면 아무것도 하지 않습니다. 켜져 있으면 도착한 worker 응답을 먼저 반영하고, 그 문서를 현재 revision 까지 따라간 뒤 스냅샷을 만들어 `install_syntax` 에 넣습니다. `install_syntax` 가 거절하면 경고 로그만 남기고 저장은 계속됩니다(그 저장에서는 지우지 않음).
- 토큰화 한도를 넘는 문서: Monaco 는 이런 문서에 토크나이저를 두지 않고 모든 줄을 정확한 기본 토큰으로 봅니다(2절). 그래서 `without_tokenizer` 로 모든 줄을 Other 한 토큰으로 담아 넣습니다. 한도 판정은 `TokenPipeline` 이 문서를 처음 볼 때 내린 결정을 그대로 씁니다(토큰 저장소가 0줄인 문서. `native/taide-native-syntax/tests/token-pipeline/pipeline.rs:344-345` 가 고정하는 계약).
- 표준 토큰 종류는 앞 단계의 스타일 표에 이미 들어 있습니다(문법 scope 가 아니라 "같은 색·글꼴 스타일을 가진 첫 테마 규칙의 scope 이름"으로 정해지는 TS 고유 동작 포함). 이 단계는 그 값을 그대로 씁니다.

### H4. 설정 반영

| 필드 | 연결 |
| --- | --- |
| editorFontFamily(굵은 face) | `native/taide-native-app/src/presentation-refresh.rs` 의 `editor_presentation(settings)` 가 `EditorDisplayOptions.bold_family = taide-editor-bold` 를 지정합니다. 편집기 패밀리를 지정하는 것과 같은 파일입니다 |
| editorFontSize | 굵은 구간도 같은 크기의 `FontId` 를 씁니다 |
| editorWordWrap, editorTabSize, editorconfig 들여쓰기 | 토큰 경계가 표시 줄 분할·탭 확장·이어지는 줄 들여쓰기와 결합합니다(H1) |
| trimTrailingWhitespaceOnSave, editorconfig `trim_trailing_whitespace` | H3 |
| 테마 `tokenColors`·`syntax`·`syntaxOverrides`·`editor.foreground`, 테마 미리보기 | 앞 단계가 `tick` 에서 연결했습니다. 이 단계에서 화면에 나타납니다 |
| editorSemanticHighlighting, editorBracketPairColorization, editorFontLigatures | 범위 밖. 건드리지 않았습니다 |

UI 크레이트의 `presentation::editor_presentation` 은 그대로입니다(굵은 패밀리 `None`). 기존 테스트가 기본 설정에서 `EditorPresentation::default()` 와 같다고 고정하고 있고, 브라우저 클라이언트는 굵은 패밀리를 등록하지 않기 때문입니다.

## 4. 설계 문서·지시와 달라진 점

| 번호 | 설계·지시 | 실제 | 이유 |
| --- | --- | --- | --- |
| 1 | 설계 4.9, 작업 항목 H2: 토큰과 스타일 표를 `EditorPresentation<'a>` 에 담아 넘김 | `EditorPresentation` 은 그대로 두고 새 진입 함수 `show_tokenized`·`reveal_tokenized` 가 `EditorTokens<'a>` 를 따로 받습니다 | 기존 테스트가 `EditorPresentation { options: .. }` literal 을 쓰고 `fn wrapping() -> EditorPresentation` 처럼 수명 없이 돌려줍니다(`tests/editor_surface.rs` 의 `wrapping`). 필드나 수명 인자를 더하면 기존 33건이 수정 없이 컴파일되지 않습니다. 설계 4.9 의 "새 입력은 새 진입 함수로, 기존 함수는 위임" 방식은 지켰습니다 |
| 2 | 설계에는 토큰을 값으로 넘김 | `show_tokenized` 는 토큰을 입력 처리 뒤에 부르는 함수로 받습니다 | 표면은 한 함수 안에서 입력을 반영하고 바로 그립니다. 프레임 시작에 빌린 토큰은 그 프레임에 편집이 있으면 한 revision 뒤처져, 줄을 넣거나 지울 때 아래 줄 전체가 한 프레임 동안 이웃 줄의 색으로 그려집니다. Monaco 는 편집 때 토큰을 동기로 옮기므로(`MONACO/editor/common/model/tokens/tokenizerSyntaxTokenBackend.js:163-179`) 이런 프레임이 없습니다. 표면에서 토큰 저장소를 복제해 저널을 적용하는 대안은 편집 프레임마다 줄 수만큼 복제가 생겨 기각했습니다 |
| 3 | 설계 4.5: `RowSection` 에 `italic`·`bold`·`strikethrough`·`straight_underline` 필드 | `RowSection { chars, foreground }` 는 그대로, 글꼴 스타일은 `RowText.font_styles`(구간과 같은 순서의 병렬 목록) | 기존 테스트가 `RowSection { chars, foreground }` literal 을 두 필드로 씁니다. `font_styles` 가 구간보다 짧으면 남은 구간은 평문입니다(기존 테스트가 `row.sections` 만 바꿔 넣는 경우) |
| 4 | 설계 5.6: 조율자가 `install_syntax` 호출(시점 미정) | 저장 직전에만 만듭니다. 설정이 꺼져 있으면 만들지 않습니다 | 토큰이 바뀔 때마다 문서 전체의 스냅샷을 다시 만들 이유가 없습니다. 기본값이 꺼짐이므로 대부분의 저장에서 비용이 0 입니다 |
| 5 | 수정 범위에 `editor-geometry.rs` 없음 | `RowLayout` 필드 2개와 `row()` 9줄 | 표시 줄의 `RowText` 와 galley 를 만드는 곳이 이 파일입니다. 로직은 `editor-row-text.rs` 에 두고 여기서는 호출만 합니다 |
| 6 | 수정 범위에 `lib.rs` 없음 | `pub mod editor_syntax` | `tests/` 의 통합 테스트가 실제 조율자를 쓰려면 모듈이 공개여야 합니다. 이 크레이트에서 통합 테스트가 쓰는 모듈은 모두 `pub mod` 입니다 |
| 7 | 작업 항목 H3: 정확한 줄까지만 스냅샷에 넣음 | 토큰화 한도를 넘는 문서는 모든 줄을 Other 로 넣습니다 | Monaco 근거(2절). 지시보다 넓은 동작이라 8절 2번에 결정 사항으로 올렸습니다 |
| 8 | 설계 2.4: `install_syntax` 는 STORE 232-281 | 지금은 `native/taide-native-editor/src/store.rs:264-313` | 앞 단계가 `changes_since` 를 앞에 넣어 줄 번호가 밀렸습니다. 서술 내용은 맞습니다 |
| 9 | 설계 7절 3d: UI 테스트는 "토큰이 주어지면 섹션 색이 표와 같다" | 색에 더해 글꼴 패밀리·기울임·밑줄·취소선, 탭·wrap·CJK·emoji 경계, 평문 화면 불변, 입력 뒤 요청, reveal 까지 | 검증 계약의 요구 |
| 10 | (설계에 없음) | 굵은 패밀리 등록 여부를 표면이 확인 | 등록되지 않은 패밀리는 epaint 에서 panic 합니다. 테스트와 브라우저 클라이언트처럼 패밀리가 없는 context 에서도 안전해야 합니다 |

설계 2.4 의 `save_cleanup.rs:105-112`, `trimTrailingWhitespaceCommand.js:79-90` 은 실제 파일과 일치했습니다.

## 5. 실행한 명령과 실제 결과

cargo 명령은 모두 `--locked --offline --target-dir experiments/native-shell-spike/target` 을 붙였습니다(`cargo fmt` 제외). 경로는 저장소 루트 기준으로 줄였습니다.

| 순서 | 명령 | 결과 |
| --- | --- | --- |
| 1 | `git status --short`, `git diff --stat` (시작) | 앞 두 단계의 미커밋 변경만 있음. 이 단계 범위 파일 중 변경은 앞 단계가 넣은 `application.rs` 뿐 |
| 2 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` (구현 전) | exit 101. `EditorTokens`·`RowTokens`·`RowFontStyle`·`show_tokenized`·`reveal_tokenized`·`highlight`·`font_styles`·`styled_layout_job` 이 없어 컴파일 오류 13건(H1 의 실패하는 테스트) |
| 3 | V1 `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --test editor_surface` (구현 뒤) | exit 0. lib 116, editor_surface 40(기존 33 + 신규 7), 0.09초 |
| 4 | V6 `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | exit 1(신규 테스트 코드 3곳) → 손으로 맞춘 뒤 exit 0 |
| 5 | V2 `cargo test --manifest-path native/taide-native-editor/Cargo.toml` | exit 0. 97 통과, 1 ignored(앞 단계 95 + 신규 2). `save_cleanup` 7건 |
| 6 | V6 `cargo fmt --manifest-path native/taide-native-editor/Cargo.toml -- --check` | exit 0 |
| 7 | V4 `cargo check --manifest-path native/taide-native-app/Cargo.toml` | exit 0. 경고는 기존 `vendor/wry-preview` 17건뿐 |
| 8 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib --example native-lsp-mock` (1회차) | exit 101. 신규 테스트의 타입 추론 오류(E0282·E0283) 3건(6절 2번) |
| 9 | 같은 명령 (수정 뒤) | exit 101. 테스트는 실행됐으나 출력이 잘려 요약을 보지 못했고, 예제가 `examples/native_lsp_mock-<hash>` 로만 만들어져 테스트가 찾는 `examples/native-lsp-mock` 은 생기지 않았습니다(9절 1번) |
| 10 | `cargo test --quiet … --lib -- --quiet` | exit 101. `Option 'quiet' given more than once`(명령 실수. 테스트는 돌지 않음) |
| 11 | V4 `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib` | exit 101. 365건 중 355 통과, 10 실패, 3.61초. 실패는 `lsp::diagnostics_tests` 8건, `lsp_process::tests::production_port…` 1건, `remote_lsp::tests::실제_합성프로세스…` 1건이고 전부 `mock.is_file()` 또는 "example is required" 단언입니다. 신규 5건(`editor_syntax::tests` 4, `presentation_refresh::tests` 1)은 통과 |
| 12 | V4 `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --test save-syntax --test save --test paste-shortcuts` | exit 0. paste-shortcuts 2, save 1, save-syntax 3(3.99초) |
| 13 | V6 `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | exit 1(`tests/save-syntax.rs` 1곳) → 손으로 맞춘 뒤 exit 0 |
| 14 | V3 `cargo test --quiet --manifest-path native/taide-native-syntax/Cargo.toml` | exit 0. lib 35, engine-gate 8 통과·3 ignored(17.66초), token-pipeline 18(2.74초), token-theme 2 |
| 15 | V5 `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | exit 0 |
| 16 | `cargo clippy --manifest-path native/taide-native-editor/Cargo.toml --all-targets` | exit 0. 경고 0 |
| 17 | `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` | exit 0. lib 경고 11(기존 9 + 신규 2: `reveal_tokenized` 8/7, `show_tokenized` 9/7 의 `too_many_arguments`). 테스트 파일 경고 0 |
| 18 | `cargo clippy --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --test save-syntax` | exit 0. 앱 크레이트 경고 0 |
| 19 | V1 재실행(서식만 고친 테스트 파일) | exit 0. lib 116, editor_surface 40 |
| 20 | `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --test save-syntax` 재실행(서식만 고친 파일) | exit 0. 3 통과(3.98초) |
| 21 | `git status --short`, `git diff --numstat` (끝) | 이 단계가 바꾼 파일은 10절 목록과 같음. `Cargo.toml`·`Cargo.lock` 은 건드리지 않았습니다(앱 lockfile 의 +35 는 앞 단계의 것) |

11번은 `--quiet` 를 붙여 실행했습니다. 같은 테스트 집합이고 출력 형식만 다릅니다(8·9번에서 출력이 잘려 결과를 읽을 수 없었습니다).

## 6. 신규 테스트와 실패했다가 고친 내역

신규 테스트:

- `native/taide-native-ui/tests/editor_surface.rs`
    - `토큰_구간의_색과_글꼴_스타일은_스타일_표와_같고_토큰_없는_줄은_표의_기본_스타일이다`: 키워드(굵게)·문자열(기울임+밑줄)·주석(취소선) 구간의 색·패밀리·`italics`·`underline`·`strikethrough`. 굵은 패밀리가 없거나 등록되지 않았으면 일반 패밀리, 등록됐으면 굵은 패밀리. 스타일이 같은 이웃 토큰의 병합. 토큰 없는 줄과 빈 줄
    - `토큰은_본문_글자의_구간만_바꾸고_선택_캐럿_현재_줄_ime_좌표는_평문_화면과_같다`: 드래그 선택과 IME 조합 프레임에서 모든 도형의 종류·좌표·글자 위치·크기와 IME 캐럿이 토큰이 없을 때와 같음
    - `탭_wrap_cjk_emoji가_섞인_줄에서_토큰_경계는_문서_바이트에_맞는_표시_문자에_놓인다`: 탭·한글·emoji·보조 평면 문자가 든 줄을 wrap 한 화면과 펼친 화면. 표시 줄 경계를 넘는 토큰, 이어지는 줄 들여쓰기
    - `토큰이_없거나_현재_문서와_맞지_않으면_기존_평문_화면과_같다`: 토큰 없음, revision 불일치, 줄 수 불일치가 모두 기존 특성화 기대값 `PLAIN_FRAME` 과 같음
    - `토큰은_그_프레임의_입력을_반영한_문서에_대해_요청된다`
    - `reveal은_토큰의_굵은_글꼴로_그린_줄의_끝에_가로_스크롤을_맞춘다`: 폭이 다른 글꼴을 굵은 패밀리로 등록해 확인. 토큰이 없으면 기존 기대값 `REVEALED_VIEW`
    - `줄_토큰은_탭과_들여쓰기를_건너_표시_문자_구간이_되고_어긋난_경계는_문자_시작으로_내린다`: `RowText::highlight`·`styled_layout_job` 단위
- `native/taide-native-editor/tests/save_cleanup.rs`
    - `토큰_저장소의_정확한_줄만_저장_정리_스냅샷이_되고_그_아래_줄은_지우지_않는다`
    - `토크나이저가_없는_문서의_스냅샷은_모든_줄을_일반_토큰으로_두어_후행_공백을_지우게_한다`
- `native/taide-native-app/src/editor-syntax-tests.rs`
    - `화면에_빌려주는_토큰은_편집_직후에도_현재_revision의_줄에_맞고_재토큰화를_바로_시작한다`
    - `테마가_다시_적용되거나_언어가_바뀐_문서는_이전_토큰을_새_스타일_표로_빌려주지_않는다`
    - `토큰은_번들_밖_언어와_닫힌_문서에는_없고_처음_보인_문서는_tick_없이_추적을_시작한다`
    - `화면이_알린_보이는_줄은_다음_tick까지_합쳐_두었다가_한_번_전달한다`
- `native/taide-native-app/src/presentation-refresh.rs`: `native_editor_presentation은_설정의_word_wrap과_편집기_굵은_글꼴_패밀리를_전달한다`
- `native/taide-native-app/tests/save-syntax.rs`(실제 엔진·worker·조율자·`save::prepare`)
    - `저장_정리는_문자열_안_후행_공백을_남기고_주석_뒤는_지우며_설정이_꺼지면_그대로_둔다`
    - `저장_정리는_아직_다시_토큰화되지_않은_줄을_건너뛰고_토큰화가_끝난_뒤에_지운다`
    - `토큰화_한도를_넘는_문서의_저장_정리는_모든_줄을_일반_토큰으로_본다`

실패했다가 고친 내역:

1. H1 의 UI 테스트는 구현 전에 컴파일 오류로 실패하는 것을 확인했습니다(5절 2번).
2. 신규 단위 테스트의 `vec![Vec::new(); n]` 이 타입을 추론하지 못해 컴파일 오류가 났습니다. 기대값에 타입을 적었습니다.
3. `cargo fmt -- --check` 가 신규 테스트 코드 4곳(UI 3, 앱 1)을 지적해 손으로 맞췄습니다.
4. 명령 실수 2건: `--example native-lsp-mock` 은 필요한 실행 파일을 만들지 못했고(9절 1번), `--quiet` 를 두 번 줬습니다.

구현이 테스트 기대값과 어긋나서 고친 것은 없습니다. 테스트 기대값을 구현에 맞춰 바꾼 것도 없습니다. H1·H3 구현은 첫 실행에서 통과했습니다.

## 7. TS 와 다르게 남는 점

| 항목 | TS | native | 영향 |
| --- | --- | --- | --- |
| 기울임 | 글꼴에 italic face 가 있으면 그 face, 없으면 브라우저의 합성 기울임 | 항상 egui 의 합성 기울임(일반 face 의 쿼드를 25% 기울임). italic face 패밀리는 등록돼 있지 않습니다 | SF Mono·Menlo 처럼 italic face 가 있는 글꼴에서 글자 모양이 다릅니다 |
| 굵기 | face 가 없으면 브라우저가 합성 굵기를 그림 | 굵은 face 가 없는 자리는 일반 face(배치 4 QA 7절 4번) | 굵은 토큰이 굵게 보이지 않는 글꼴이 있습니다 |
| 밑줄 두께 | 글꼴이 정한 두께(크기에 비례) | 1pt 고정 | 큰 글꼴에서 TS 보다 가늘 수 있습니다 |
| 취소선 위치 | 글꼴이 정한 위치(x-height 부근) | 글꼴 줄 상자의 세로 가운데 | 1~2pt 차이가 날 수 있습니다 |
| 첫 화면의 토큰 | 보이는 줄을 그리기 전에 동기 토큰화 | worker 응답이 온 프레임부터. 그 전에는 기본 스타일 | 파일을 열 때 한두 프레임 색 없는 화면이 보일 수 있습니다(비목표: 뷰포트 추정 토큰화) |
| 테마 전환 직후 | 기본 전경색은 바로 새 테마 | 새 스타일 표가 올 때까지 이전 토큰 색과 이전 기본 전경색(새 배경 위) | 몇 프레임. 실기 확인 대상 |
| 평문·한도 초과 문서의 글자색 | 기본 토큰색(`editor.foreground` 의 RGB, 알파 버림) | `editor.foreground` 그대로(알파 유지). 기존 동작 | `editor.foreground` 에 알파가 있는 테마에서만 다릅니다 |
| 저장 정리, 번들 밖 언어 | 플러그인 문법이 있으면 토큰대로, 토큰 공급자가 없으면 모든 줄을 지움 | 지우지 않음(이전과 같음) | 플러그인 문법 단계(3e) 전까지의 차이 |
| 저장 정리, 토큰 테마가 적용된 적 없는 상태(거절된 테마로 시작 등) | 토큰 공급자가 없어 모든 줄을 지움 | 지우지 않음 | TS 의 고장 상태를 옮기지 않았습니다. 문자열 안 공백을 지우는 방향이라 보수적으로 뒀습니다 |
| 저장 정리, 편집 직후 저장 | 보이는 줄은 다음 렌더에서 동기 토큰화되므로 건너뛰는 창이 짧음 | worker 응답 전이면 편집 줄부터 아래를 그 저장에서 건너뜀 | 응답은 보통 수 ms 안에 옵니다. 저장 때 도착한 응답은 먼저 반영합니다 |

앞 단계 QA 의 차이(세션 중 테마 전환, 줄당 500ms 한도, 숨은 탭의 배경 토큰화)는 그대로입니다.

## 8. 메인 판단이 필요한 사항

1. clippy `too_many_arguments` 경고가 2건 늘었습니다(`reveal_tokenized` 8/7, `show_tokenized` 9/7). 배치 4 에서 `show_presented`(8/7)에 대해 "설계 서명 유지와 경고 제거 중 어느 쪽"으로 남겨 둔 결정과 같은 문제이고, 같은 방식(진입 함수 추가)으로 맞췄습니다. 경고를 없애려면 `editor.presented(&presentation, tokens).show(..)` 같은 형태로 세 함수를 함께 바꿔야 합니다. clippy 는 검증 계약에 없습니다.
2. 토큰화 한도를 넘는 문서에서 모든 줄의 후행 공백을 지우는 동작(3절 H3). Monaco 소스로 확인한 TS 동작이지만 작업 지시의 문구("정확히 토큰화된 줄까지")보다 넓습니다. 빼려면 `supply_save_cleanup` 의 `is_exempt_from_tokenization` 분기와 `SyntaxSnapshot::without_tokenizer`, 테스트 2건을 지우면 됩니다. 20MB 이상 파일은 읽기 전용으로 열리므로(`crates/taide-file/src/service.rs:99`) 실제로 해당하는 것은 30만 줄을 넘는 20MB 미만 파일입니다.
3. 범위 밖 파일 수정 2건(`editor-geometry.rs`, `lib.rs`)과 `EditorPresentation` 에 토큰을 담지 않은 점(4절 1·5·6번).
4. `EditorSyntax` 와 메서드 6개가 `pub` 이 됐습니다(통합 테스트용). 크레이트 밖 소비자는 테스트뿐입니다.

## 9. 미결 사항

1. V4 `cargo test --lib` 의 실패 10건. 앞 단계 QA 9절 1번과 같습니다. `cargo test --lib --example native-lsp-mock` 을 시도했으나 cargo 가 예제를 해시가 붙은 이름(`examples/native_lsp_mock-07aa297d3e6020b5`)으로만 만들고 `examples/native-lsp-mock` 은 만들지 않았습니다. 그 이름은 `cargo build --example` 이나 필터 없는 `cargo test` 가 만드는데 둘 다 이 단계에서 허용되지 않습니다. 메인이 배치 끝에 전체 테스트를 실행하면 다시 판정됩니다. 시도로 생긴 실행 파일(약 1MB)이 target 에 남아 있습니다.
2. 구문 강조의 실제 화면은 GUI 없이 확인할 수 없습니다(11절).
3. 8절의 결정 사항.

## 10. 변경 파일

| 파일 | 구분 |
| --- | --- |
| `native/taide-native-ui/src/editor-row-text.rs` | 범위 안 |
| `native/taide-native-ui/src/editor_surface.rs` | 범위 안 |
| `native/taide-native-ui/src/editor-geometry.rs` | 범위 밖(4절 5번) |
| `native/taide-native-ui/tests/editor_surface.rs` | 범위 안 |
| `native/taide-native-editor/src/syntax.rs` | 범위 안 |
| `native/taide-native-editor/tests/save_cleanup.rs` | 관련 테스트 |
| `native/taide-native-app/src/editor-syntax.rs` | 범위 안 |
| `native/taide-native-app/src/editor-syntax-tests.rs` | 관련 테스트 |
| `native/taide-native-app/src/application.rs` | 범위 안 |
| `native/taide-native-app/src/presentation-refresh.rs` | 범위 안 |
| `native/taide-native-app/src/lib.rs` | 범위 밖(4절 6번) |
| `native/taide-native-app/tests/save-syntax.rs` | 신규 통합 테스트 |
| `docs/quality-assurance/2026-10-06-native-batch5-surface-highlight.md` | 이 문서 |

`Cargo.toml`·`Cargo.lock`, `taide-remote-web`, `taide-native-syntax`, `crates/`, `src/` 는 건드리지 않았습니다.

## 11. 남은 위험과 실기 확인

남은 위험:

1. 굵은 face 의 세로 메트릭(ascent·줄 높이)이 일반 face 와 다르면 굵은 토큰이 있는 줄만 글자 세로 위치가 달라질 수 있습니다. epaint 는 한 줄 안의 서로 다른 글꼴을 줄 상자 아래쪽 기준으로 맞춥니다. 같은 패밀리의 일반·굵은 face 는 보통 메트릭이 같습니다.
2. 굵은 face 의 글자 폭이 일반 face 와 다르면 그 줄의 글자가 열에서 어긋나 보입니다. 캐럿·선택·클릭은 galley 좌표를 쓰므로 어긋나지 않지만 wrap 열 계산은 일반 글꼴 폭 기준입니다.
3. 저널 상한(한 프레임에 64번 넘는 편집)을 넘으면 그 문서의 토큰을 전부 비우고 다시 받습니다. 그동안 기본 스타일로 보입니다.
4. worker 가 죽으면 강조가 멈추고(앞 단계 QA 8절 2번) 저장 정리도 마지막으로 받은 토큰까지만 씁니다.
5. 저장 때 만든 `SyntaxSnapshot` 은 다음 편집까지 문서에 남습니다. 30만 줄을 넘는 문서에서는 줄마다 항목 하나라 십수 MB 이상이 될 수 있습니다(측정하지 않았습니다).
6. 큰 문서를 처음 여는 동안 worker 응답마다 다시 그립니다(앞 단계 QA 8절 3번). 이제 실제로 색이 채워지는 과정이 보입니다.
7. 같은 문서를 연 pane 이 여럿이면 우선 토큰화 범위는 보이는 범위들의 합(맨 위 줄부터 맨 아래 줄까지)입니다.
8. 앱 크레이트의 단위 테스트 파일은 clippy 로 보지 않았습니다(`--tests` 는 통합 테스트 대상 전체를 검사해야 해서 생략).

실기 확인이 필요한 것:

- [ ] 번들 언어 파일(rust, typescript, json, markdown 등)을 열면 TS 화면과 같은 색으로 보이는지, 테마의 굵은·기울임·밑줄 토큰이 그렇게 보이는지
- [ ] 파일을 열 때와 탭을 바꿀 때 색 없는 화면이 눈에 띄게 오래 보이지 않는지
- [ ] 입력·줄 추가·줄 삭제·붙여넣기·undo 중에 아래 줄의 색이 깜빡이거나 한 줄씩 밀려 보이지 않는지
- [ ] 문자열·블록 주석을 열고 닫을 때 아래 줄이 곧 다시 칠해지는지
- [ ] 테마 전환과 테마 편집기 미리보기에서 토큰 색이 새 테마로 바뀌는지, 전환 순간의 색이 거슬리지 않는지
- [ ] word wrap 을 켠 상태, 탭 들여쓰기 파일, 한글·emoji 가 든 줄에서 색 경계가 글자와 맞는지
- [ ] 굵은 토큰이 있는 줄의 글자 높이·열이 다른 줄과 어긋나지 않는지(사용자 글꼴, 기본 글꼴, 한글)
- [ ] 선택 영역·현재 줄 강조·IME 조합(한글)과 토큰 색이 함께 정상인지
- [ ] 수만 줄 파일을 열고 중간으로 스크롤했을 때 보이는 범위가 먼저 칠해지는지, 그동안 입력이 끊기지 않는지
- [ ] `trimTrailingWhitespaceOnSave` 를 켜고 저장했을 때: 코드·주석 뒤 공백은 지워지고 여러 줄 문자열 안의 줄 끝 공백은 남는지, 자동 저장에서 캐럿 왼쪽 공백이 남는지, undo 로 되돌아가는지
- [ ] 설정이 꺼진 상태에서 저장 내용이 바뀌지 않는지
- [ ] 브라우저 클라이언트(`taide-remote-web`)의 편집기가 이전과 같은 평문 화면인지

## 12. 테스트 부채

- [ ] `show_document` 수준의 화면 테스트. 재현 조건: 실제 `NativeApplication` 프레임에서 편집 직후 프레임의 토큰 색을 단언. 생략 이유: 앱 프레임을 GUI 없이 돌리는 장치가 없습니다. 표면(`show_tokenized`)과 조율자(`tokens`)는 각각 테스트했습니다. 필요한 시점: 편집 중 색 밀림이 실기에서 보고될 때.
- [ ] 한도 초과 문서의 실제 정리 실행(30만 줄의 삭제 편집을 한 transaction 으로 적용). 스냅샷 내용만 확인했습니다. 남은 위험: 그런 저장에 걸리는 시간. 필요한 시점: 대형 파일 저장이 느리다는 보고가 있을 때.
- [ ] 보이는 줄 우선의 효과를 조율자 수준에서 관찰하는 테스트. worker 수준 테스트는 앞 단계에 있고, 조율자는 범위를 합쳐 전달하는 것까지만 확인했습니다(`TokenPipeline` 에 조회 함수가 없습니다).
- [ ] 메트릭이 다른 굵은 face(다른 패밀리가 섞인 체인)에서의 세로 정렬. 합성 글꼴이 필요합니다. 필요한 시점: 11절 실기 확인에서 어긋남이 보일 때.
- [ ] 정규식 literal 안의 후행 공백. 엔진이 정규식 토큰을 내는 실제 문서로는 확인하지 않았습니다(종류 판정 자체는 앞 단계의 기준 대조와 편집기 크레이트의 기존 테스트가 다룹니다).
