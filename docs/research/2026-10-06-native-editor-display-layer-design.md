# Native 편집기 표시 계층 재설계와 구문 강조 엔진 설계 (2026-10-06)

상태: 읽기 전용 조사와 설계입니다. 코드·설정은 바꾸지 않았고 빌드·테스트·실행도 하지 않았습니다. 이 문서는 배치 4 의 직렬 작업자가 이 문서만 읽고 착수할 수 있도록 작성했습니다.

## 0. 읽는 법

- 기준 시점: HEAD `09ced3a4`, 작업 트리 변경 없음(`git status --short` 출력 없음). 줄 번호는 이 시점 값이므로 착수 전에 실제 파일로 다시 확인하십시오.
- 경로는 저장소 루트(`/Users/hyunseokbyun/development/TAIDE`) 기준 상대 경로입니다. 약어는 아래와 같습니다.

| 약어 | 경로 |
| --- | --- |
| ES | `native/taide-native-ui/src/editor_surface.rs` |
| APP | `native/taide-native-app/src/application.rs` |
| STORE | `native/taide-native-editor/src/store.rs` |
| EGUI | `native/taide-native-app/vendor/egui-input/src` (vendored egui 0.36.2) |
| EPAINT | `/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/epaint-0.36.2/src` (lockfile 이 고정한 epaint 0.36.2 설치본. epaint 는 vendored 가 아니라 registry 소스입니다) |
| MONACO | `node_modules/monaco-editor/esm/vs` (0.56.0) |
| SHIKI-MONACO | `node_modules/@shikijs/monaco/dist/index.mjs` (4.4.3) |
| SHIKI-PRIM | `node_modules/@shikijs/primitive/dist/index.mjs` |

- 코드에 대한 서술은 직접 읽은 파일을 근거로 달았습니다. 외부 라이브러리 수치는 2026-10-06 에 crates.io API·docs.rs·공식 저장소에서 조회한 값이고 출처는 10절에 있습니다. 확인하지 못한 것은 9절에 모았습니다.
- 원칙: 동작 기준은 TS 소스와 TS 가 쓰던 라이브러리(Monaco 0.56.0, Shiki 4.4.3)의 실제 동작입니다. 새 기능·새 디자인은 넣지 않습니다.

## 1. 결론 요약

1. 표시 계층은 Monaco 의 3층 구조(뷰 모델 줄 투영 → 세로 레이아웃 → 뷰 파트)를 그대로 따릅니다. egui 에 의존하지 않는 표시 모델(줄 매핑·세로 배치·장식·토큰·접기·찾기)은 `taide-native-editor` 에, egui 로 그리는 표면은 `taide-native-ui` 에, 토큰화 엔진은 새 크레이트 `native/taide-native-syntax` 에 둡니다(4.1).
2. 기존 공개 서명(`EditorAppearance`, `NativeEditor { appearance }`, `show`·`show_with_keymap`·`show_with_input_route`)은 바꾸지 않습니다. 구조체 literal 소비처가 13곳이고 동결된 `taide-remote-web` 도 포함되기 때문입니다. 새 입력은 `EditorPresentation` 인자를 받는 새 진입 함수로 넣고 기존 함수는 기본값으로 위임합니다(4.9).
3. 텍스트는 표시 줄마다 `LayoutJob` 하나로 만들고, galley 캐시는 앱이 따로 두지 않고 epaint 의 `GalleyCache` 에 맡깁니다. 폰트 atlas 가 80% 를 넘으면 epaint 가 `Fonts` 를 통째로 다시 만들면서 galley 캐시를 버리므로(EPAINT `text/fonts.rs:728-743`) 프레임을 넘겨 `Arc<Galley>` 를 보관하면 UV 가 무효가 됩니다(4.5).
4. egui 0.36.2 의 확인된 한계: 탭은 탭 정지가 아니라 고정 폭, 굵기 스타일 없음, 물결 밑줄 없음, 리거처를 끌 방법 없음, 컬러 이모지 없음, bidi 없음(2.6). 탭·굵기·물결 밑줄은 표시 계층이 직접 처리하고, 리거처·컬러 이모지·bidi 는 사용자 결정 또는 한계로 남깁니다(8절).
5. 구문 강조 엔진: syntect 5.3.0 은 `.sublime-syntax` 만 읽으며 VS Code `tmLanguage` JSON·플러그인·VSIX 문법을 그대로 읽지 못합니다. 변환 브리지 `syntect-tmlanguage` 0.1.0 은 `begin`/`while`, `\G`, 선택자 기반 injection 을 지원하지 못한다고 스스로 밝힙니다. 추천안은 vscode-textmate 이식 엔진 `ferriki-textmate` 0.12.0(+ 순수 Rust Oniguruma 이식 `ferroni` 1.8.1)이며, 최초 공개가 2026-09-28 인 신생 크레이트이므로 내부 trait 뒤에 격리하고 적합성 게이트를 통과한 경우에만 채택합니다. 사용자 결정 "syntect 계열"과 다르므로 확인이 필요합니다(5절, 8절 D1).
6. 토큰 색은 TS 와 같은 3단계(vscode-textmate 테마 매칭 → `shikiToMonaco` 의 색·글꼴 역조회 → Monaco 토큰 테마 trie)로 결정해야 TS 화면과 같아집니다. 엔진의 색을 그대로 쓰면 전경색 없는 규칙의 글꼴 스타일 등에서 TS 와 어긋납니다(6절).
7. 현재 native 는 `install_syntax` 호출부가 없어서 plaintext 가 아닌 파일의 "저장 시 후행 공백 제거"가 한 줄도 적용되지 않습니다(`native/taide-native-editor/src/save_cleanup.rs:91-112`). 구문 강조 단계에서 표준 토큰 종류를 공급하면 함께 풀립니다(2.4, 5.6).
8. 구현 순서: 골격 리팩터링(동작 불변) → 편집기 글꼴 체인 → 구문 강조 → word wrap → 장식·편집 저널·앵커 기반 → 찾기/바꾸기 → 접기 → 나머지(7절).

## 2. 현재 구조

### 2.1 호출 체인

- `native/taide-native-app/src/main.rs` → APP `NativeApplication`(`editor: NativeEditor` 필드 APP 158, 생성 APP 271-272, 테마·설정 갱신 시 `self.editor.appearance = appearances.editor` APP 616·3474, 글꼴 크기 APP 3544) → `eframe::App::ui`(APP 3762) → `AppSurfaces::tab_content`(APP 4953) → `show_document`(APP 5428-5620).
- `show_document` 의 순서: 충돌·읽기 전용 배너(APP 5437-5467) → `attach_view`(APP 5473-5475) → reveal 큐 소비와 `NativeEditor::reveal`(APP 5483-5493) → editorconfig 를 반영한 들여쓰기 해석과 `with_indent`(APP 5500-5509) → 커맨드 팔레트 등에서 온 문서 편집 적용 `command_dispatch::apply_document_edits`(APP 5511-5519, `native/taide-native-app/src/command-dispatch.rs:71-91`) → `show_with_input_route`(APP 5527-5564) → 키맵 대상 등록·포커스·dirty·오류 처리(APP 5565-5614).
- 키 라우팅: 표면은 이벤트마다 호출자 `keymap` 콜백을 먼저 부르고(ES 417-419) 처리되지 않은 것만 `input` 으로 넘깁니다(ES 420-431). APP 은 이 콜백에서 `terminal_views.route_keymap` 으로 전역 키맵을 실행합니다(APP 5533-5561). 오버레이 위젯의 키 가로채기(완성 목록 탐색 등)는 이 콜백으로 구현할 수 있어 새 장치가 필요 없습니다.
- 테마 → 외형: `Appearances::new(theme: &ResolvedTheme, settings)`(`native/taide-native-app/src/presentation-refresh.rs:173-191`)가 `taide_native_ui::presentation::editor_appearance`(`native/taide-native-ui/src/presentation.rs:204-226`)를 부릅니다. 편집기 글꼴은 `FontId::monospace` 고정이고 줄 높이는 `font_size * 1.5` 입니다(`presentation.rs:18, 210-211`).

### 2.2 `editor_surface.rs` 의 한 프레임

`show_with_input_route`(ES 340-780) 한 함수가 입력·스크롤·레이아웃·hit-test·페인트·IME 를 모두 처리합니다.

| 순서 | 내용 | 줄 |
| --- | --- | --- |
| 1 | 외형 검증, id·rect·`interact`, 포커스 요청 | ES 349-363 |
| 2 | 뷰·문서 스냅샷, egui temp data 의 `InputState`, `InputContext` 구성 | ES 364-385 |
| 3 | 입력 소유 경로 판정, 포커스 잠금 필터, 이벤트 루프(keymap 콜백 → `input`), 남은 이벤트 복원 | ES 386-434 |
| 4 | 포커스 상실 시 조합 취소, 복사 내용 반출 | ES 435-443 |
| 5 | 재스냅샷, 세로 스크롤 상한, 문서·선택 변경 시 캐럿 따라가기 | ES 444-465 |
| 6 | 휠, 세로 스크롤바 드래그 | ES 466-494 |
| 7 | gutter 폭, 본문 rect, 보이는 줄 범위 계산, 줄마다 `to_string` + `layout_no_wrap` | ES 495-519 |
| 8 | 최대 줄 폭 추적, 가로 스크롤 범위·캐럿 reveal, 가로 스크롤바 | ES 520-576 |
| 9 | 뷰 상태 저장, 포인터 hit-test 로 선택 갱신(항상 단일 선택) | ES 577-620 |
| 10 | 배경 → 줄별(현재 줄 강조 → 선택 → 캐럿 → 텍스트 → 줄 번호) | ES 621-693 |
| 11 | IME preedit 덮어 그리기와 `IMEOutput` | ES 694-726 |
| 12 | 스크롤바 fade·페인트, 상태 저장, `EditorOutput` | ES 727-780 |

- 줄 ↔ 화면 좌표는 `줄 번호 * line_height - scroll.y` 한 식뿐입니다(ES 515). 표시 줄 개념이 없어 wrap·fold·줄 사이 블록을 넣을 자리가 없습니다.
- 좌표 변환은 줄마다 만든 galley 의 `cursor_from_pos`(ES 593)와 `pos_from_cursor`(ES 1051-1057)에 의존합니다. 표시 텍스트와 문서 텍스트가 같다는 가정입니다(탭 확장·주입 텍스트 없음).
- `reveal`(ES 255-307)은 gutter 폭·galley·스크롤 계산을 `show_with_input_route` 와 따로 한 번 더 구현합니다. 표시 계층이 생기면 한 곳으로 합쳐야 합니다.
- 접근성 노드는 만들지 않습니다(`widget_info`·`accesskit_node_builder` 호출 없음). IME 후보 창 위치는 주 캐럿이 있는 줄의 캐럿 rect 를 그대로 넘깁니다(ES 713-725).

### 2.3 문서·뷰 모델

- 문서: `DocumentSnapshot { id, key, revision, rope, metadata, dirty }`(`native/taide-native-editor/src/document.rs:108-116`). `revision` 은 적용된 트랜잭션마다 1 증가하고(STORE 1169-1172, 1215) undo·redo 도 1 증가합니다(STORE 1271-1274, 1300). ropey `Rope` 는 복제 비용이 낮아 스냅샷을 스레드로 넘길 수 있습니다.
- 뷰: `ViewState { selection, scroll, folds, composition, edit_run, goal_columns }`(`view.rs:137-148`). 선택은 바이트 오프셋 쌍입니다(`view.rs:18-22`).
- `folds: Vec<Range<usize>>` 는 바이트 범위이고 편집 시 선택과 같은 규칙으로 옮겨지며(STORE 1230-1247) undo·redo 에서는 전부 지워집니다(STORE 1297). 접기를 만들거나 그리는 코드는 없습니다.
- undo 이력은 편집 목록이 아니라 전후 rope 를 보관합니다(`HistoryEntry { before, after, .. }` STORE 56-63). 따라서 undo·redo 때는 범위를 옮길 편집 정보가 없습니다. 4.3 의 편집 저널 설계가 이 제약을 다룹니다.
- 시각 열 계산(탭 정지, 전각 2열)은 `editing.rs` 의 비공개 함수 `next_visible_column`(311)·`visible_column`(322)·`offset_at_visible_column`(330)에 이미 있습니다. 표시 모델이 같은 크레이트에 있으면 공개 범위만 넓혀 재사용할 수 있습니다.

### 2.4 `syntax.rs` 와 `install_syntax`

- `native/taide-native-editor/src/syntax.rs`(40줄)는 색 정보가 아닙니다. `TokenKind { Other, Comment, String, Regex }` 와 줄별 `Token { start_byte, kind }`, `SyntaxSnapshot { revision, language_id, lines }`, `token_at(line, byte)` 뿐입니다. Monaco 의 `StandardTokenType` 에 대응합니다.
- `EditorStore::install_syntax`(STORE 232-281)는 revision·언어 일치와 줄·바이트 순서를 검증해 문서에 저장하고, `syntax`(STORE 218-230)는 revision 이 맞을 때만 돌려줍니다. 편집과 undo·redo 마다 지워집니다(STORE 1217, 1302).
- 유일한 소비자는 저장 정리입니다. `trim_trailing_whitespace` 는 plaintext 가 아니면 후행 공백 시작 위치의 토큰 종류가 `Other`·`Comment` 일 때만 지웁니다(`save_cleanup.rs:105-112`). `install_syntax` 의 비테스트 호출부가 없으므로 지금은 plaintext 외 파일에서 후행 공백 제거가 전혀 일어나지 않습니다.
- 기준 동작: Monaco 는 토큰이 정확하지 않은 줄은 건너뛰고, 정확한 줄은 `String`·`RegEx` 안의 후행 공백만 보존합니다(`MONACO/editor/common/commands/trimTrailingWhitespaceCommand.js:79-90`). 강제 토큰화는 하지 않습니다. `install_syntax` 는 일부 줄만 담은 스냅샷도 받으므로(줄 번호 오름차순만 요구, STORE 250-257) 같은 의미를 그대로 표현할 수 있습니다.

### 2.5 소비처와 서명 제약

- `EditorAppearance { .. }` literal: `native/taide-native-ui/src/presentation.rs:209`, `native/taide-native-ui/tests/editor_surface.rs:692`, `native/taide-remote-web/tests/files.rs:77, 879`, `native/taide-native-app/src/{save.rs:185, keymap-tests.rs:433, 594, shell_keymap.rs:953, presentation.rs:265, zen.rs:276}`, `native/taide-native-app/tests/{terminal-host.rs:1318, 3787, paste-shortcuts.rs:160}`. 합계 13곳입니다.
- `NativeEditor { appearance }` literal 도 같은 파일들과 `native/taide-remote-web/src/browser-editor.rs:1106, 1197`, APP 271 에 있습니다.
- `EditorOutput` 은 ES 773 에서만 만들어지고 소비처는 필드를 읽기만 합니다. 필드 추가는 안전합니다.
- `taide-remote-web` 은 `taide-native-ui`(`default-features = false`)와 `taide-native-editor` 를 그대로 컴파일합니다(`native/taide-remote-web/Cargo.toml`). 두 크레이트에 넣는 코드는 wasm32 에서 컴파일돼야 하고 C 의존성을 끌어오면 안 됩니다.

### 2.6 egui 0.36.2 텍스트 계층의 확인된 사실

| 항목 | 사실 | 근거 |
| --- | --- | --- |
| 셰이핑 | `harfrust` 0.12.0 으로 run 단위 셰이핑. run 은 글꼴 face 단위이며 섹션 경계를 넘지 않음 | EPAINT `text/text_layout.rs:429-520, 1357-1393` |
| 리거처 | `harfrust::ShapeOptions::new()` 기본값으로 호출. 기본값은 feature 목록이 비어 있어 글꼴의 기본 feature(liga·calt 포함)가 적용됨. `TextFormat` 에 feature 필드가 없어 끌 수 없음 | EPAINT `text/text_layout.rs:1400-1434`, `harfrust-0.12.0/src/hb/face.rs:259-271`, EPAINT `text/text_layout_types.rs:478-523` |
| 탭 | 탭 정지가 아니라 `tab_size * 공백 폭` 고정 전진. `tab_size` 는 글꼴별 `FontTweak` 값(기본 4.0) | EPAINT `text/text_layout.rs:262-270`, `text/fonts.rs:263, 277` |
| 글꼴 스타일 | `TextFormat` 은 `italics`(쿼드 상단을 높이의 25% 만큼 기울이는 합성), `underline`·`strikethrough`(직선 `Stroke`), `background`, `coords`(가변 글꼴 축)만 제공. 굵기 플래그와 물결·점선 밑줄 없음 | EPAINT `text/text_layout_types.rs:478-523`, `text/text_layout.rs:1079-1095, 1173-1199, 1208-1250` |
| 세로 배치 | 줄 상자 안에서 글리프를 가운데로 맞추지 않음. 현재 표면은 galley 를 줄 위쪽 가장자리에 그림(ES 680). Monaco 는 CSS line-height 로 가운데 정렬 | EPAINT `text/text_layout.rs:953-994` |
| 이모지 | 글리프는 윤곽선만 흰색으로 래스터화해 정점 색으로 칠함. 컬러 글리프(COLR·sbix) 경로 없음 | EPAINT `text/font.rs:220-314` |
| bidi | 미지원. 소스에 `TODO(emilk): heed bidi characters` 와 "RTL/bidi 지원이 추가되면" 주석 | EPAINT `text/font.rs:829`, `text/text_layout.rs:1363-1366` |
| 다중 문자 클러스터 | 클러스터의 글리프 수가 문자 수보다 적으면 폭 0 인 연속 글리프를 넣어 `glyphs.len() == 문자 수` 를 유지. 커서 변환이 문자 인덱스 기준으로 성립 | EPAINT `text/text_layout.rs:224-229, 396-425` |
| galley 캐시 | `LayoutJob` 해시 + pixels_per_point 로 조회. 그 프레임에 쓰이지 않은 항목은 `begin_pass` 에서 삭제 | EPAINT `text/fonts.rs:1061-1160, 1285-1291` |
| atlas 재생성 | atlas 사용률이 0.8 을 넘거나 `TextOptions` 가 바뀌면 `Fonts` 와 galley 캐시를 새로 만듦 | EPAINT `text/fonts.rs:728-743` |
| `layout_no_wrap` | `LayoutJob::simple(text, font, color, f32::INFINITY)` 와 같음 | EGUI `painter.rs:503-510`, EPAINT `text/fonts.rs:931-942`, `text/text_layout_types.rs:126-142` |
| 기본 monospace | `Hack` → `Ubuntu-Light` → `NotoEmoji-Regular` → `emoji-icon-font`. 앱은 터미널(`taide-terminal`)·UI 패밀리만 시스템 글꼴로 확장하고 `FontFamily::Monospace` 는 건드리지 않음 | EPAINT `text/fonts.rs:501-553`, `native/taide-native-app/src/{terminal_fonts.rs:11-19, 203-239, ui-fonts.rs:23-105}` |
| 접근성 | `Context::accesskit_node_builder` 는 공개, `register_accesskit_parent` 는 `pub(crate)`. 텍스트 위젯용 `update_accesskit_for_text_widget` 과 `Galley::concat` 은 공개 | EGUI `context.rs:4210, 4218`, `text_selection/accesskit_text.rs:30-199`, `text_selection/mod.rs:3`, EPAINT `text/text_layout_types.rs:1065` |
| IME | `PlatformOutput.ime = IMEOutput { purpose, rect, cursor_rect, should_interrupt_composition }` | EGUI `data/output.rs:100-113` |

## 3. TS 기준 기능 목록

### 3.1 설정 필드와 표시 계층 요구

설정 화면 Editor 절은 33개 필드입니다(`src/widgets/settings-view/settings-editor-section.tsx:64-268`). `editorMinimap` 은 설정 화면이 아니라 편집기 액션 `taide.toggleMinimap` 으로 바꿉니다(`src/features/editor/code-editor.tsx:100, 246-252`). 기본값은 `src/shared/lib/code-editor-settings.ts:43-65` 와 `crates/taide-model/src/settings.rs:84-334` 가 정본입니다.

| # | 필드 | 기본값 | Monaco 옵션 | 표시 계층 요구 |
| --- | --- | --- | --- | --- |
| 1 | editorFontSize | 13 | fontSize | 글꼴·줄 높이 변경 시 레이아웃 전체 무효화(기존 처리 유지) |
| 2 | editorFontFamily | 없음 | fontFamily | 편집기 전용 글꼴 체인과 굵은 face. TS 스택은 `src/shared/lib/font-stack.ts`(사용자 글꼴, ui-monospace, SFMono-Regular, Menlo, Apple SD Gothic Neo, monospace) |
| 3 | formatOnSave | false | (저장 경로) | 없음 |
| 4 | organizeImportsOnSave | false | (저장 경로) | 없음 |
| 5 | fixAllOnSave | false | (저장 경로) | 없음 |
| 6 | trimTrailingWhitespaceOnSave | false | (액션) | 표준 토큰 종류 공급(2.4) |
| 7 | insertFinalNewlineOnSave | false | (액션) | 없음 |
| 8 | editorConfigEnabled | false | 모델 옵션 | 탭 폭 표시가 문서별로 달라짐 |
| 9 | editorCodeLensEnabled | true | codeLens provider | 줄 사이 블록(view zone)과 클릭 영역 |
| 10 | autoSaveDelayMs | 0 | (저장 경로) | 없음 |
| 11 | editorWordWrap | false | wordWrap on/off | 표시 줄 매핑, 가로 스크롤 제거, 표시 줄 기준 이동 |
| 12 | editorLineNumbers | true | lineNumbers | gutter 구성 |
| 13 | editorTabSize | 4 | tabSize | 탭 정지 표시, wrap 열 계산 |
| 14 | editorInsertSpaces | true | insertSpaces | 없음(편집 동작) |
| 15 | editorDetectIndentation | true | detectIndentation | 감지 결과가 탭 폭 표시에 반영 |
| 16 | editorRenderWhitespace | selection | renderWhitespace | 공백·탭 표식 오버레이(none·boundary·selection·all) |
| 17 | editorBracketPairColorization | true | bracketPairColorization | 괄호 토큰 색 덮어쓰기. 대형 파일에서는 꺼짐(`code-editor.tsx:271`) |
| 18 | editorBracketPairGuides | false | guides.bracketPairs | 세로·가로 가이드 선 |
| 19 | editorRulers | 빈 목록 | rulers | 열 위치 세로선 |
| 20 | editorFontLigatures | false | fontLigatures | liga·calt 켜고 끄기(2.6 한계) |
| 21 | editorCursorStyle | line | cursorStyle | line·block·underline 캐럿 |
| 22 | editorCursorBlinking | blink | cursorBlinking | blink·smooth·phase·expand·solid 애니메이션 |
| 23 | editorCursorSmoothCaretAnimation | false | cursorSmoothCaretAnimation | 캐럿 위치 보간 |
| 24 | editorScrollBeyondLastLine | true | scrollBeyondLastLine | 세로 스크롤 범위 |
| 25 | editorSmoothScrolling | false | smoothScrolling | 스크롤 위치 보간 |
| 26 | editorStickyScrollEnabled | true | stickyScroll.enabled | 상단 고정 줄 위젯 |
| 27 | editorSemanticHighlighting | true | semantic tokens provider | 토큰 색 덮어쓰기 층 |
| 28 | editorFormatOnType | false | formatOnType | 없음 |
| 29 | editorFormatOnPaste | false | formatOnPaste | 없음 |
| 30 | editorSuggestPreview | false | suggest.preview | 주입 텍스트(ghost text) |
| 31 | emmetEnabled | true | (provider) | 완성 목록 위젯 |
| 32 | editorDiffHideUnchangedRegions | false | diff hideUnchangedRegions | diff 표면의 숨김 영역과 줄 사이 블록 |
| 33 | editorDiffShowMoves | false | diff experimental.showMoves | diff 표면의 이동 표시 |
| 추가 | editorMinimap | true | minimap.enabled | minimap. 대형 파일에서는 꺼짐(`code-editor.tsx:275`) |

TS 가 설정하지 않아 Monaco 기본값이 기준인 표시 옵션입니다. 파일을 적지 않은 줄 번호는 `MONACO/editor/common/config/editorOptions.js` 이고, 다른 파일은 `MONACO/editor/` 아래 경로를 함께 적었습니다.

| 옵션 | 기본값 | 줄 |
| --- | --- | --- |
| lineHeight | 0 → macOS `1.5 * fontSize`, 그 외 `1.35 * fontSize`, 최소 8 | `common/config/fontInfo.js:12, 16, 23` |
| lineDecorationsWidth | 10 (접기 컨트롤이 있으면 +16) | 1481, 1220-1225 |
| lineNumbersMinChars | TS 가 3 으로 지정(`code-editor.tsx:186`) | 3244 |
| glyphMargin | TS 가 false 로 지정(`code-editor.tsx:185`) | 3233 |
| wrappingIndent / wrappingStrategy | same / simple | 2899, 1319 |
| wordWrapBreakAfterCharacters / BeforeCharacters | 문자 목록 | 3450-3455 |
| cursorWidth | 0 → line 스타일에서 2px | 3175, `browser/viewParts/viewCursors/viewCursor.js:135` |
| renderLineHighlight | line | 3320 |
| roundedSelection | true | 3342 |
| matchBrackets | always | 3247 |
| occurrencesHighlight / selectionHighlight | singleFile / true | 3281, 3353 |
| guides.indentation / highlightActiveIndentation | true / true | 2428-2432 |
| showFoldingControls / foldingStrategy / foldingHighlight | mouseover / auto / true | 3357, 3213, 3220 |
| stickyScroll | maxLineCount 5, defaultModel outlineModel | 1382 |
| minimap | side right, size proportional, showSlider mouseover, renderCharacters true, maxColumn 120, scale 1 | 1518-1529 |
| scrollbar | 세로 14, 가로 12, useShadows true | 1988-1998 |
| find | seedSearchStringFromSelection always, addExtraSpaceOnTop true, loop true, cursorMoveOnType true | 462-471 |
| stopRenderingLineAfter | 10000 | 3379 |
| renderControlCharacters / renderFinalNewline | true / on(Linux 는 dimmed) | 3318, 3319 |
| overviewRulerLanes / overviewRulerBorder | 3 / true | 3295, 3294 |
| links / colorDecorators | true / true | 3246, 3140 |

### 3.2 제품이 쓰는 Monaco 기능과 표시 계층 요구

| 기능 | TS 근거 | 표시 계층에 요구하는 것 |
| --- | --- | --- |
| 구문 강조 | `src/shared/lib/shiki/shiki-monaco.ts`, `lang-map.ts:65-97` | 줄별 색·글꼴 스타일 run, 줄 끝 상태 캐시, 테마 변경 시 재계산 |
| semantic token | `src/shared/lib/theme-convert/semantic-token-map.ts:32-96`, `build-shiki-theme.ts:91-99` | 구문 토큰 위에 덮는 run 층 |
| word wrap | `code-editor.tsx:284` | 문서 줄 → 여러 표시 줄, 이어지는 줄 들여쓰기(same), 가로 스크롤 없음 |
| 접기 | `code-editor.tsx:271`, `src/shared/lib/monaco/monaco-actions.ts:747-827` | 숨김 줄 범위, gutter 접기 컨트롤, 접힌 줄 끝 표시, 접기 명령 20여 개 |
| minimap | `code-editor.tsx:246-252, 275` | 전체 표시 줄의 축소 렌더, 뷰포트 슬라이더, 장식 표식 |
| sticky scroll | `code-editor.tsx:279` | 흐름 밖의 임의 문서 줄을 상단에 고정해 그리기, 클릭 이동 |
| 줄 번호·gutter | `code-editor.tsx:185-186, 285` | 줄 번호(이어지는 표시 줄에는 없음), 장식 lane, 접기 lane |
| 진단 밑줄 | `src/shared/lib/lsp/adapters/diagnostics.ts` (`setModelMarkers`) | 물결 밑줄, 심각도 색, hover 메시지 앵커, overview 표식 |
| Git gutter | `src/widgets/editor-pane/use-editor-git-gutter-and-conflicts.ts:224`, `src/shared/styles/global.css:371-408` | 줄 장식 lane 의 3px 막대(added·modified)와 삼각형(deleted), 클릭 |
| conflict 표시 | 같은 파일 211-219, `global.css:416-446` | 줄 전체 배경, lane 의 클릭 가능한 막대 |
| blame | `src/widgets/editor-pane/use-editor-blame.ts:128-134`, `global.css:410-414` | 줄 끝 주입 텍스트(기울임, 불투명도 0.8, `editorBlame.foreground`) |
| inlay hint | `src/shared/lib/lsp/adapters/inlay-hints.ts` | 줄 안 주입 텍스트(커서가 들어가지 않음), 패딩·배경 |
| code lens | `src/shared/lib/lsp/adapters/code-lens.ts` | 줄 위 블록과 그 안의 클릭 가능한 항목 |
| 완성·hover·signature | `src/shared/lib/lsp/adapters/{completion,hover,signature-help}.ts` | 문서 위치 앵커, 위·아래 선호 배치, 스크롤 추종 |
| 찾기 위젯 | Monaco 내장(`monaco-actions.ts:66-72, 409-463, 609`) | 우상단 오버레이, 일치 배경, 현재 일치 강조, overview 표식, 상단 여백 블록 |
| 다중 커서 | Monaco 내장(`monaco-actions.ts:103-111, 306-318`) | 여러 캐럿·선택 렌더(이미 반복 렌더 중, ES 637-671), 입력 경로는 명령 계층 |
| bracket pair colorization·가이드 | `code-editor.tsx:271, 294` | 괄호 토큰 색, 가이드 선. 언어 구성 필요 |
| whitespace 렌더 | `code-editor.tsx:289` | 공백 점·탭 화살표 오버레이 |
| rulers | `code-editor.tsx:298` | 열 기준 세로선 |
| cursor 스타일·blink | `code-editor.tsx:291-292, 296` | 캐럿 모양·깜빡임·부드러운 이동 |
| ligature | `code-editor.tsx:290` | OpenType feature 제어 |
| AI 인라인 편집 | `src/features/editor/ai-inline-edit.ts:131-137, 212-213, 230-233` | 문서 위치 content widget(위 우선, 아래 차선), 범위 배경, 줄 뒤 블록(`heightInPx = lineHeight * lineCount`) |
| AI 인라인 완성 | `src/shared/lib/ai/inline-completion.ts` | 주입 텍스트(ghost text, 여러 줄 가능) |
| diff 편집기 | `src/features/git/diff-view.tsx:46-81`, `src/widgets/claude-diff-pane/claude-diff-pane.tsx:58` | 표면 2개의 스크롤 동기, 정렬용 줄 사이 블록, 줄·문자 배경, 숨김 영역 |

### 3.3 TS 가 실제로 만드는 장식 사양

| 장식 | 종류 | 값 | 근거 |
| --- | --- | --- | --- |
| Git added·modified | 줄 장식 lane | 줄 전체, lane 왼쪽 3px 막대, 색 `editorGutter.addedBackground`·`modifiedBackground` | `use-editor-git-gutter-and-conflicts.ts:224`, `global.css:371-392` |
| Git deleted | 줄 장식 lane | 줄 위쪽 경계에 8px 높이 삼각형(가로 6px), 색 `editorGutter.deletedBackground` | `global.css:394-408` |
| conflict current·incoming | 줄 전체 배경 | `diff.insertedLineBackground`, `diff.removedLineBackground` | `use-editor-git-gutter-and-conflicts.ts:211-215`, `global.css:416-422` |
| conflict action | 줄 장식 lane, 클릭 | 3px 막대 `git.conflicted`, 포인터 커서 | 같은 파일 219, `global.css:424-442` |
| blame | 줄 끝 주입 텍스트 | 기울임, 불투명도 0.8, 색 `editorBlame.foreground` | `use-editor-blame.ts:128-134`, `global.css:410-414` |
| AI 편집 삭제 대상 | 범위 배경 | `diff.removedLineBackground` | `ai-inline-edit.ts:17, 212-213` |
| AI 편집 미리보기 | 줄 뒤 블록 | `afterLineNumber = 대상 끝 줄`, 높이 `lineHeight * 줄 수` | `ai-inline-edit.ts:230-233` |
| AI 편집 입력 | content widget | 위 우선, 아래 차선 | `ai-inline-edit.ts:131-137` |

## 4. 표시 계층 설계

### 4.1 구성과 소유 경계

| 층 | 위치 | 내용 | wasm 공유 |
| --- | --- | --- | --- |
| 문서·뷰 | `native/taide-native-editor/src/{document,view,store,editing}.rs` | 기존 | 공유 |
| 표시 모델 | `native/taide-native-editor/src/` 신규 `display-map.rs`, `line-breaks.rs`, `display-layout.rs`, `decoration.rs`, `line-tokens.rs`, `change-journal.rs`, `folding.rs`, `find.rs`, `language-configuration.rs` | 줄 투영, wrap 계산, 세로 배치, 장식·토큰 자료형, 편집 저널, 접기·찾기 모델. egui 비의존 | 공유 |
| 표면 | ES 와 `native/taide-native-ui/src/` 신규 `editor-row-text.rs`, `editor-geometry.rs`, `editor-paint.rs`, `editor-gutter.rs`, `editor-find-widget.rs`, `editor-sticky-scroll.rs`, `editor-minimap.rs`, `editor-overlay.rs` | LayoutJob 구성, 좌표, 페인트 층, 위젯 | 공유 |
| 토큰화 엔진 | 신규 크레이트 `native/taide-native-syntax` | 문법 집합, 토큰 테마, 토큰화 worker | app 전용 |
| 배선 | APP 와 `native/taide-native-app/src/` 신규 `editor-syntax.rs`, `editor-fonts.rs`, `editor-decorations.rs` | 설정·테마·LSP·Git·플러그인 연결 | app 전용 |

kebab-case 파일은 각 `lib.rs` 에서 `#[path]` 로 연결합니다(기존 관례, `native/taide-native-ui/src/lib.rs`, `native/taide-native-editor/src/lib.rs`).

- 결정: 표시 모델을 `taide-native-editor` 에 둡니다. 근거는 egui 없이 `native/taide-native-editor/tests` 에서 단위 테스트할 수 있고, `editing.rs` 의 시각 열 함수와 `view.rs` 의 `map_offset` 을 같은 크레이트에서 재사용할 수 있다는 점입니다.
- 기각: 표시 모델을 `taide-native-ui` 에 두는 안. 테스트마다 `egui::Context` 가 필요해지고 편집 함수(표시 줄 기준 세로 이동, Home·End)가 표시 모델을 읽을 수 없게 됩니다.
- 기각: 토큰화 엔진을 `taide-native-ui` 또는 `taide-native-editor` 의존성으로 넣는 안. 동결된 `taide-remote-web` 의 의존 그래프와 lockfile 이 바뀌고 wasm 컴파일 가능성을 별도로 보장해야 합니다.
- 기각: 루트 workspace `crates/` 에 두는 안. Tauri 앱(`src-tauri`)의 그래프와 lockfile 까지 바뀝니다(`Cargo.toml` workspace members).

### 4.2 좌표계와 표시 줄 매핑

좌표는 네 단계입니다.

| 좌표 | 단위 | 소유 |
| --- | --- | --- |
| 문서 위치 | 바이트 오프셋(기존 `Selection`), 문서 줄 번호 | `document.rs`, `view.rs` |
| 표시 위치 | 표시 줄 번호 + 줄 안 바이트 오프셋 | `DisplayMap` |
| 표시 텍스트 위치 | 표시 줄 텍스트의 문자 인덱스 | `RowText` |
| 화면 | 포인트(px) | `VerticalLayout`(y), galley(x) |

`DisplayMap`(순수 모델, Monaco `common/viewModel/viewModelLines.js`·`modelLineProjection.js` 대응):

```rust
pub struct WrapSettings {
    pub wrap_column: u32,
    pub tab_size: u32,
}

pub struct RowSegment {
    pub line: usize,
    pub bytes: std::ops::Range<usize>,
    pub is_continuation: bool,
    pub indent_columns: u32,
    pub ends_folded: bool,
}

impl DisplayMap {
    pub fn identity(line_count: usize, revision: u64) -> Self;
    pub fn build(document: &DocumentSnapshot, hidden_lines: &[std::ops::Range<usize>], wrap: Option<WrapSettings>) -> Self;
    pub fn row_count(&self) -> usize;
    pub fn segment(&self, document: &DocumentSnapshot, row: usize) -> RowSegment;
    pub fn rows_of_line(&self, line: usize) -> Option<std::ops::Range<usize>>;
    pub fn row_of_byte(&self, document: &DocumentSnapshot, byte: usize, prefer_next_row: bool) -> usize;
}
```

- `DisplayMap` 의 필드는 비공개입니다.
- wrap 이 꺼져 있고 숨김 줄이 없으면 `identity` 를 써서 줄별 저장 없이 `표시 줄 == 문서 줄` 로 동작합니다. 1단계(동작 불변)와 대형 파일의 기본 경로입니다.
- 문서 줄별 표시 줄 수의 누적합은 Fenwick 트리로 유지합니다(Monaco 는 `ConstantTimePrefixSumComputer`). 줄 → 표시 줄, 표시 줄 → 줄 모두 O(log n) 입니다.
- 숨김(접기) 먼저, wrap 나중 순서로 투영합니다. 숨겨진 문서 줄은 표시 줄 수 0 입니다. 숨김 범위의 첫 줄 바로 앞 줄이 머리 줄이며 `ends_folded` 가 참입니다.
- wrap 줄 나눔은 `line-breaks.rs` 에 Monaco `common/viewModel/monospaceLineBreaksComputer.js`(472줄)를 옮깁니다. 열 단위 계산이며 입력은 wrap 열, 탭 크기, break-before·after 문자 집합(`editorOptions.js:3450-3455`), wrappingIndent same 입니다. wrap 열은 `max(1, floor((contentWidth - 14 - 2) / typicalHalfwidthCharacterWidth))` 입니다(`editorOptions.js:1278-1285`).
- 표시 줄 → 문서 바이트 hit-test: y 로 `VerticalLayout::row_at` → `DisplayMap::segment` → 그 줄 galley 의 `cursor_from_pos` → `RowText` 로 문서 바이트 환산. wrap 경계에서는 같은 바이트가 앞 줄 끝과 뒷 줄 시작 두 곳에 놓일 수 있으므로 Monaco 처럼 affinity(앞·뒤 선호)를 뷰 상태에 둡니다. egui 의 `CCursor.prefer_next_row` 와 같은 개념입니다(EPAINT `text/text_layout_types.rs:1211-1215`).

`VerticalLayout`(순수 모델, Monaco `common/viewLayout/linesLayout.js` 대응):

```rust
pub struct ViewZone {
    pub id: u64,
    pub after_row: Option<usize>,
    pub height: f32,
}

impl VerticalLayout {
    pub fn new(line_height: f32, row_count: usize, zones: &[ViewZone], padding_bottom: f32) -> Self;
    pub fn row_top(&self, row: usize) -> f32;
    pub fn row_at(&self, y: f32) -> usize;
    pub fn zone_top(&self, id: u64) -> Option<f32>;
    pub fn content_height(&self) -> f32;
}
```

- 줄 높이는 균일하고, 줄 사이 블록(view zone)만 높이를 더합니다. `after_row: None` 은 첫 줄 위입니다(찾기 위젯의 `addExtraSpaceOnTop` 블록).
- `scrollBeyondLastLine` 은 `padding_bottom = 뷰포트 높이 - line_height` 로 표현합니다(Monaco `viewLayout.js` 의 스크롤 높이 계산을 구현 단계에서 대조).

대안과 기각:

- egui 의 wrap(`LayoutJob.wrap.max_width`)에 맡기는 안은 기각합니다. 줄 나눔 후보가 Monaco 와 다르고(EPAINT `text/text_layout.rs:1254-1279` 의 공백·CJK·대시 후보), 이어지는 줄 들여쓰기(same)를 표현할 수 없으며, 문서 줄 전체를 한 galley 로 만들어야 해서 표시 줄 수를 알려면 전 문서를 레이아웃해야 합니다.
- 줄 높이를 줄마다 다르게 두는 안(Monaco 0.56 의 `lineHeights.js`)은 TS 가 줄별 높이 장식을 쓰지 않으므로 넣지 않습니다.
- 접기를 표시 줄 삭제가 아니라 높이 0 으로 표현하는 안은 hit-test 와 누적합이 복잡해져 기각합니다.

### 4.3 장식 모델

자료형은 `taide-native-editor/src/decoration.rs` 에 둡니다. Monaco `IModelDecorationOptions` 중 3.2·3.3 이 실제로 쓰는 것만 담습니다.

```rust
pub enum UnderlineKind { Straight, Squiggly, Dotted }

pub struct InlineStyle {
    pub foreground: Option<[u8; 4]>,
    pub background: Option<[u8; 4]>,
    pub italic: bool,
    pub bold: bool,
    pub underline: Option<(UnderlineKind, [u8; 4])>,
    pub strikethrough: bool,
    pub opacity: Option<f32>,
}

pub enum LaneMark { Bar, DeletedTriangle }

pub struct InjectedText {
    pub text: String,
    pub style: InlineStyle,
    pub cursor_stops: bool,
}

pub struct OverviewMark {
    pub color: [u8; 4],
    pub lane: u8,
}

pub enum DecorationKind {
    Inline(InlineStyle),
    LineBackground([u8; 4]),
    Lane { mark: LaneMark, color: [u8; 4], is_clickable: bool },
    Before(InjectedText),
    After(InjectedText),
}

pub struct Decoration {
    pub bytes: std::ops::Range<usize>,
    pub kind: DecorationKind,
    pub overview: Option<OverviewMark>,
    pub hover: Option<u64>,
}

pub struct DecorationLayer {
    pub owner: u32,
    pub revision: u64,
    pub z_order: u8,
    pub items: Vec<Decoration>,
}
```

- 색은 egui 형식이 아니라 RGBA 바이트로 둡니다. 순수 크레이트가 egui 에 의존하지 않게 하기 위함입니다.
- 소유자(owner)별로 한 층입니다: 진단, Git hunk, conflict, blame, 찾기 일치, 선택 일치·document highlight, 괄호 매칭, inlay hint, ghost text, 스니펫 자리표시자, AI 편집. 층은 `items` 를 시작 바이트 오름차순으로 유지하고 표면은 보이는 바이트 범위를 이진 탐색합니다.
- 구문·semantic 토큰은 장식이 아니라 별도 자료(`line-tokens.rs`)입니다. Monaco 도 토큰과 장식을 분리합니다. 줄당 `Vec<u32>`(시작 바이트, 스타일 id) 쌍으로 저장합니다.
- 그리는 순서는 Monaco 뷰 파트 순서를 따릅니다(`MONACO/editor/browser/view.js:114-167`): 본문 배경 → 현재 줄 강조 → 선택 → 들여쓰기 가이드 → 장식(줄 배경·인라인 배경) → 공백 표식 → 텍스트 → 밑줄 → gutter(현재 줄 여백 강조 → 줄 장식 lane → 줄 번호) → content widget → 캐럿 → rulers → minimap → 스크롤 그림자 → overview ruler → overlay widget.

revision 태그와 무효화:

- 층은 계산 기준 문서 revision 을 답니다. 표면은 `층.revision == 문서.revision` 인 층만 그대로 그리고, 다르면 편집 저널로 범위를 옮겨서 그립니다. 옮길 수 없으면(저널 범위 밖) 그 층을 그리지 않습니다. Monaco 는 장식 범위를 모델 안에서 편집에 따라 옮기므로 공급자가 갱신하기 전에도 밑줄이 글자를 따라갑니다.
- 편집 저널(`change-journal.rs`): `EditorStore` 가 문서별로 최근 N 개 변경을 보관합니다. 항목은 `{ revision_before, revision_after, spans }` 이고 span 은 `{ start_byte, old_end_byte, new_end_byte }` 입니다. `apply_transaction` 은 병합된 편집(STORE 1158-1162 의 `edits`)에서 바로 만듭니다.
- undo·redo 는 편집 목록이 없으므로(2.3) 전후 rope 의 공통 접두·접미 바이트를 비교해 span 하나를 만듭니다. 선형 비교지만 undo 한 번당 한 번이고 chunk 단위 비교로 충분합니다. 이 방식이면 undo·redo 에서 접기를 지우는 현재 동작(STORE 1297)도 Monaco 처럼 유지로 바꿀 수 있습니다.
- 저널 소비자: 장식 범위 이동, 토큰 무효화 시작 줄, `DisplayMap` 증분 갱신, minimap 무효화.

대안과 기각:

- 공급자가 매 프레임 장식을 다시 계산하는 안은 기각합니다. LSP 진단·Git diff 는 비동기라 편집 직후 프레임에 범위가 어긋납니다.
- 장식 범위를 `ViewState.folds` 처럼 스토어가 직접 보관하고 옮기는 안은 undo·redo 경로 때문에 결국 저널이 필요하므로 저널 하나로 통일합니다.
- undo 이력에 편집 목록을 추가 저장하는 안은 `HistoryEntry` 병합 로직(STORE 1198-1213)을 고쳐야 해서 범위가 큽니다. 접두·접미 비교로 같은 효과를 얻습니다.

### 4.4 오버레이 위젯 앵커

Monaco 는 위젯을 두 종류로 나눕니다. native 도 같은 구분을 씁니다.

| 종류 | Monaco | native |
| --- | --- | --- |
| content widget | 문서 위치에 고정, 스크롤 추종, 위·아래 선호 | 완성 목록, hover, signature help, rename 입력, AI 편집 입력, peek |
| overlay widget | 편집기 모서리에 고정 | 찾기 위젯(우상단), sticky scroll(상단) |

- 표면은 위젯을 직접 그리지 않고 좌표만 제공합니다. `EditorOutput` 에 `geometry: EditorGeometry` 필드를 추가합니다.

```rust
pub struct EditorGeometry {
    pub rect: egui::Rect,
    pub content_rect: egui::Rect,
    pub gutter_rect: egui::Rect,
    pub line_height: f32,
    pub visible_rows: std::ops::Range<usize>,
    pub scroll: egui::Vec2,
}

impl EditorGeometry {
    pub fn caret_rect(&self, byte: usize) -> Option<egui::Rect>;
    pub fn range_rects(&self, bytes: std::ops::Range<usize>) -> Vec<egui::Rect>;
    pub fn top_right_anchor(&self) -> egui::Pos2;
}
```

- `caret_rect` 는 보이는 표시 줄에 있는 위치만 `Some` 을 돌려줍니다(숨김·화면 밖은 `None`). 구현은 그 프레임에 만든 줄 정보(galley, `RowText`)를 `EditorGeometry` 안에 `Arc` 로 들고 있는 방식입니다. galley 를 프레임 안에서만 쓰므로 2.6 의 atlas 재생성 문제와 무관합니다.
- 배치기는 `editor-overlay.rs` 에 둡니다. 입력은 앵커 rect, 위젯 크기, 선호 순서(위·아래), 경계 rect 이고 출력은 위치입니다. Monaco `browser/viewParts/contentWidgets/contentWidgets.js` 의 ABOVE·BELOW 판정을 옮깁니다. 기존 `native/taide-native-ui/src/tooltip-placement.rs` 와 같은 순수 함수 형태로 만듭니다.
- 그리기는 호출자가 `egui::Area::new(id).order(Order::Foreground).fixed_pos(..)` 로 합니다(EGUI `containers/area.rs:133-358`). 기존 팔레트·툴팁이 같은 방식입니다.
- 키 입력: 편집기가 포커스를 가진 채 위젯이 키를 가로채는 경우(완성 목록의 화살표·Enter·Escape)는 기존 `keymap` 콜백에서 먼저 처리합니다(ES 417-419). 위젯이 포커스를 갖는 경우(찾기 입력)는 egui 포커스가 옮겨가므로 기존 입력 소유 경로(ES 386-412)가 그대로 동작합니다.
- 찾기 위젯은 표면 안에서 그립니다. 위치가 편집기 내부 레이아웃(세로 스크롤바 14, minimap 폭)에 묶여 있고(`MONACO/editor/contrib/find/browser/findWidget.js:589-602`) 상단 블록 높이 33 을 세로 배치에 넣어야 하기 때문입니다(`findWidget.js:67, 139-146`).

대안과 기각: 표면이 위젯 내용까지 클로저로 받아 그리는 안은 LSP·AI 자료형이 `taide-native-ui` 로 새어 들어오므로 기각합니다.

### 4.5 텍스트 레이아웃 전략

`RowText`(표시 줄 하나의 표시 텍스트와 매핑, `editor-row-text.rs`):

```rust
pub struct RowText {
    pub text: String,
    pub sections: Vec<RowSection>,
    pub model_bytes: Vec<u32>,
}

pub struct RowSection {
    pub chars: std::ops::Range<usize>,
    pub foreground: egui::Color32,
    pub background: egui::Color32,
    pub italic: bool,
    pub bold: bool,
    pub strikethrough: bool,
    pub straight_underline: Option<egui::Color32>,
}
```

- 구성 순서: 문서 줄 조각 → 탭을 탭 정지까지 공백으로 확장 → 주입 텍스트·조합 문자열 삽입 → 스타일 run 병합(구문 토큰 → semantic 토큰 → 인라인 장식) → `LayoutJob`.
- `model_bytes[i]` 는 표시 문자 i 가 대응하는 문서 바이트(줄 시작 기준)입니다. 탭이 만든 공백들은 탭의 바이트를, 주입 텍스트는 삽입 지점의 바이트를 가리킵니다. hit-test 는 `cursor_from_pos` 의 문자 인덱스를 이 표로 환산하고, 탭 안쪽은 가까운 쪽 경계로, 주입 텍스트 안쪽은 삽입 지점으로 붙입니다. 반대 방향은 이진 탐색입니다.
- `LayoutJob` 은 표시 줄당 하나이며 wrap 없음(`max_width = INFINITY`)입니다. 1단계에서는 `LayoutJob::simple(text, font, foreground, f32::INFINITY)` 와 정확히 같은 job(단일 섹션, `break_on_newline = true`)을 만들어 현재 galley 와 같은 결과를 유지합니다(2.6).
- 굵기는 `FontId.family` 를 굵은 face 패밀리로 바꿔 표현합니다. 물결·점선 밑줄, 불투명도, 공백 표식은 galley 가 아니라 표면이 직접 그립니다.

캐시:

- 결정: 앱 수준 galley 캐시를 두지 않습니다. 매 프레임 보이는 표시 줄(화면 높이 / 줄 높이 + overscan)만 `RowText` 와 `LayoutJob` 을 만들고 `painter.layout_job(job)` 을 부릅니다. epaint 가 job 해시로 galley 를 재사용하고, 그 프레임에 쓰인 항목만 유지합니다(EPAINT `text/fonts.rs:1101-1157, 1285-1291`). 지금 구현도 같은 방식입니다(ES 502-518).
- 무효화 키는 job 내용 자체입니다. 문서 revision, 폭(wrap 조각), 테마(색), 글꼴이 바뀌면 job 이 달라져 자동으로 새로 레이아웃됩니다. 별도 무효화 코드가 필요 없습니다.
- `RowText` 구성 비용을 줄여야 한다는 측정 결과가 나오면 `(문서 id, revision, 표시 줄, 토큰 세대, 장식 세대, 폭)` 키의 `RowText` 캐시만 추가합니다. galley 는 보관하지 않습니다.
- 기각: `Arc<Galley>` 를 프레임을 넘겨 보관하는 안. atlas 재생성(EPAINT `text/fonts.rs:728-743`)을 외부에서 감지할 공개 신호가 없어 깨진 UV 를 그릴 수 있습니다.

긴 줄과 대형 파일:

- wrap 이 꺼진 상태에서 문서 줄의 표시 대상은 앞 10000자까지입니다(Monaco `stopRenderingLineAfter` 10000, `editorOptions.js:3379`). rope 를 문자 범위로 잘라 그 부분만 `String` 으로 만듭니다. 지금은 줄 전체를 매 프레임 `to_string` 합니다(ES 503-506).
- 토큰화는 UTF-16 길이 20000 이상인 줄을 건너뜁니다(SHIKI-MONACO 93, 100-106).
- 세로 가상화는 기존대로 보이는 표시 줄만 만듭니다(ES 498-500). `DisplayMap` 이 identity 일 때는 줄별 자료가 없어 30만 줄 문서도 추가 메모리가 없습니다.
- wrap 을 켜면 전 문서 줄 나눔을 계산해야 스크롤 높이가 정해집니다. Monaco 도 전 줄을 계산합니다. 순수 문자 순회라 비용은 낮을 것으로 보지만 측정이 필요합니다(9절). 한 프레임을 넘기면 계산을 프레임에 나누고 미계산 줄을 표시 줄 1개로 추정하는 방안을 4단계의 대비책으로 둡니다.

리거처·CJK·이모지·bidi:

- 리거처: epaint 는 글꼴 기본 feature 를 항상 적용합니다. 기본 monospace 체인과 TS 폴백 스택(SF Mono, Menlo)에는 프로그래밍 리거처가 없다고 알려져 있으나 이번에 확인하지는 않았습니다. 사용자가 리거처 글꼴을 고르면 `editorFontLigatures = false`(기본값)를 지킬 수 없습니다. Monaco 는 off 일 때 `"liga" off, "calt" off` 를 겁니다(`editorOptions.js:578-579`). 8절 D4 입니다.
- CJK: 글리프는 문자별로 face 를 고르고(EPAINT `text/font.rs:729-733`) 폭은 해당 face 의 advance 입니다. 정확히 2칸이 아닐 수 있으나 캐럿·선택은 galley 좌표를 쓰므로 어긋나지 않습니다. wrap 열 계산은 Monaco 와 같이 전각을 2열로 가정합니다. 현재 `FontFamily::Monospace` 에 CJK 글꼴이 없어 한글이 대체 글리프로 나올 수 있으므로 2단계에서 편집기 글꼴 체인을 먼저 만듭니다.
- 이모지: 단색 윤곽선으로만 나옵니다. WebKit 의 컬러 이모지와 다릅니다. 8절 D5 입니다.
- bidi: RTL 문자열이 논리 순서로 그려집니다. Monaco 는 브라우저가 처리합니다. 8절 D5 입니다.

### 4.6 minimap, sticky scroll, 스크롤바 장식

- 공통: 줄 그리기를 `paint_row(painter, origin, row_text, layers)` 형태의 함수로 분리해 본문, sticky scroll, (추후) peek 이 같은 함수를 씁니다. 1단계 리팩터링에서 이 형태로 뽑습니다.
- sticky scroll: 상단에 최대 5줄(`editorOptions.js:1382`)의 문서 줄을 스크롤과 무관한 y 에 그립니다. 배경은 `editor.widgetBackground`, 테두리는 `editor.widgetBorder` 입니다(`src/shared/lib/monaco/theme.ts:194-196`). 어떤 줄을 고정할지는 Monaco `contrib/stickyScroll/browser/stickyScrollProvider.js` 의 모델(outline → folding → indentation 순)을 따르며 document symbol 이 없을 때는 접기 영역을 씁니다. 따라서 접기 단계 뒤에 옵니다.
- minimap: 표시 줄 전체를 축소한 이미지를 CPU 에서 `egui::ColorImage` 로 만들어 텍스처로 올리고, 그 위에 슬라이더와 장식 표식을 그립니다. 재생성 조건은 문서 revision, 토큰 세대, 테마, minimap 높이, 시작 표시 줄입니다. 문자 렌더는 Monaco 의 사전 제작 문자 시트(`browser/viewParts/minimap/minimapPreBaked.js`, `minimapCharRenderer.js`)를 옮깁니다. 폭은 `editorOptions.js:1165-1175` 의 식, maxColumn 120, 오른쪽 배치입니다. 토큰이 아직 없는 줄은 기본 전경색입니다.
- 스크롤바 장식(overview ruler): 세로 스크롤바 영역(폭 14)을 3 lane 으로 나눠 장식의 `overview` 표식을 `표시 줄 / 전체 표시 줄` 비율 위치에 칠합니다(`browser/viewParts/overviewRuler/decorationsOverviewRuler.js`). 캐럿 위치 표식도 포함합니다(`hideCursorInOverviewRuler` false).
- 스크롤 그림자: `scroll.y > 0` 일 때 상단에 `app.shadow` 색 그림자(`theme.ts:178`, `browser/viewParts/scrollDecoration/scrollDecoration.js`).
- 기각: minimap 을 본문과 같은 galley 를 축소해 그리는 안. 수천 줄의 galley 를 매 프레임 만들게 됩니다.

### 4.7 접근성과 IME

- 접근성: 편집기 응답 id 에 `Role::MultilineTextInput` 을 지정하고, 보이는 표시 줄의 galley 를 `Galley::concat` 으로 합쳐 `egui::text_selection::accesskit_text::update_accesskit_for_text_widget` 에 넘깁니다. 이 함수가 줄마다 `TextRun` 노드(값, 문자 길이·위치·폭, 단어 시작)와 선택 범위를 만듭니다(EGUI `text_selection/accesskit_text.rs:30-199`). 자식 노드 등록 함수가 `pub(crate)` 라서 직접 노드 트리를 만들 수는 없습니다(EGUI `context.rs:4218`). `ctx.accesskit_node_builder(id, ..)` 가 `None` 이면 AccessKit 이 꺼진 것이므로 concat 을 생략합니다.
- 한계: 화면 밖 줄은 노드가 없습니다. Monaco 도 보이는 영역 중심의 접근성 버퍼를 씁니다. `Galley::concat` 이 줄별 job 이 다른 galley 묶음에 대해 `layout_from_cursor` 를 올바르게 계산하는지는 구현 단계에서 검증합니다(9절).
- 기각: vendored egui 를 고쳐 `register_accesskit_parent` 를 공개하는 안. vendored 변경 파일이 이미 10개이고(`native/taide-native-app/vendor/egui-input/UPSTREAM.md`) 공개 API 로 가능한 경로가 있습니다.
- IME 후보 창: `cursor_rect` 는 표시 줄 좌표에서 구합니다(wrap·fold·주입 텍스트 반영). `rect` 는 편집기 전체가 아니라 본문 rect 로 좁히는 것이 맞는지 구현 단계에서 TS 동작과 대조합니다. 지금은 편집기 rect 전체를 넘깁니다(ES 721).
- 조합 문자열: 지금은 캐럿 위치에 덮어 그려 뒤 글자를 가립니다(ES 699-712). Monaco 는 조합 중에도 모델에 글자를 넣어 뒤 글자가 밀립니다. `RowText` 구성 때 조합 문자열을 `replace` 범위 자리에 넣고 밑줄 섹션으로 표시하면 같은 모습이 됩니다. 동작이 바뀌므로 1단계가 아니라 주입 텍스트 단계에서 처리합니다.

### 4.8 `taide-remote-web` 과의 공유 경계

- 공유되는 것: `taide-native-editor` 의 표시 모델 전부, `taide-native-ui` 의 표면 전부. 새 의존성이 없으므로 wasm 컴파일에 영향이 없어야 합니다.
- 공유되지 않는 것: `taide-native-syntax`(엔진과 문법 자산), APP 의 배선.
- 브라우저 클라이언트는 `NativeEditor::show` 를 그대로 부르므로(`native/taide-remote-web/src/browser-editor.rs:1149-1151`) 기본 `EditorPresentation` 으로 동작합니다. 즉 구문 강조·wrap·접기 없이 지금과 같은 화면입니다. 동결 결정과 일치합니다.
- `EditorPresentation::default()` 는 TS 기본값이 아니라 현재 native 동작(전부 꺼짐)이어야 합니다. 그래야 1단계가 동작 불변이고 브라우저 클라이언트가 바뀌지 않습니다.
- 각 단계의 검증에 `cargo check --manifest-path native/taide-remote-web/Cargo.toml` 을 포함합니다(배치 1 의 V5 와 같음, `docs/quality-assurance/2026-10-06-native-batch1-editor-input.md` 4절).

### 4.9 공개 API 변경 방식

```rust
#[derive(Default)]
pub struct EditorDisplayOptions {
    pub word_wrap: bool,
    pub render_whitespace: RenderWhitespace,
    pub rulers: Vec<u32>,
    pub cursor_style: CursorStyle,
    pub cursor_blinking: CursorBlinking,
    pub smooth_caret: bool,
    pub scroll_beyond_last_line: bool,
    pub smooth_scrolling: bool,
    pub sticky_scroll: bool,
    pub minimap: bool,
    pub folding: bool,
    pub bracket_pair_colorization: bool,
    pub bracket_pair_guides: bool,
    pub bold_family: Option<egui::FontFamily>,
}

#[derive(Default)]
pub struct EditorPresentation<'a> {
    pub options: EditorDisplayOptions,
    pub tokens: Option<&'a LineTokens>,
    pub token_styles: Option<&'a TokenStyleTable>,
    pub layers: &'a [&'a DecorationLayer],
    pub zones: &'a [ViewZone],
    pub hidden_lines: &'a [std::ops::Range<usize>],
}

impl NativeEditor {
    pub fn show_presented(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request_focus: bool,
        keymap: impl FnMut(&Ui, &Event, bool) -> bool,
        route: impl FnOnce(&Response) -> Option<KeyboardInputRoute>,
        presentation: &EditorPresentation<'_>,
    ) -> Result<EditorOutput, EditorError>;
}
```

- `show`, `show_with_keymap`, `show_with_input_route` 는 서명을 유지하고 `show_presented(.., &EditorPresentation::default())` 로 위임합니다. 13곳의 literal 과 기존 테스트가 수정 없이 컴파일됩니다.
- 각 enum 의 `Default` 는 현재 동작입니다(공백 표식 없음, 1px 선 캐럿, 깜빡임 없음). APP 이 `Settings` 에서 TS 기본값을 반영한 `EditorDisplayOptions` 를 만듭니다.
- 표면이 프레임 사이에 유지해야 하는 표시 상태(`DisplayMap`, wrap 폭, 찾기 위젯 상태, 캐럿 깜빡임 시작 시각)는 기존 `InputState` 처럼 egui temp data 에 뷰 id 로 둡니다(ES 370-372, 772).
- 기각: `EditorAppearance` 에 필드를 추가하는 안. 13곳을 고쳐야 하고 그중 2곳이 동결된 크레이트의 테스트입니다. 배치 1 도 같은 이유로 피했습니다(`2026-10-06-native-batch1-editor-input.md` 7절).

## 5. 구문 강조 엔진

### 5.1 TS 기준 사실

- 엔진: `@shikijs/core` + `@shikijs/engine-javascript`(Oniguruma 패턴을 JS 정규식으로 변환) + `@shikijs/langs` + `@shikijs/monaco`, 전부 4.4.3. 내부 토크나이저는 `@shikijs/vscode-textmate` 10.0.2(Shiki 의 vscode-textmate 포크)입니다(`package.json:36-39`, `node_modules/@shikijs/vscode-textmate/package.json`).
- 언어 31개: rust, typescript, typescriptreact, javascript, javascriptreact, json, jsonc, markdown, toml, yaml, html, css, scss, python, go, shellscript, java, ruby, erb, dart, swift, scala, elixir, heex, haskell, c, cpp, kotlin, lua, zig, hcl(`src/shared/lib/shiki/lang-map.ts:3-35`). `typescriptreact` 는 tsx, `javascriptreact` 는 jsx 문법의 이름만 바꿔 쓰고 `heex` 는 html 문법을 씁니다(`lang-map.ts:68, 70, 89`). 실제 문법은 30종입니다.
- 언어 id 는 확장자 표에서 정해지며 31개 외에는 플러그인 overlay 또는 `plaintext` 입니다(`crates/taide-infra/src/language.rs:30-77, 101`).
- 문법 출처: `@shikijs/langs/dist/<id>.mjs` 가 문법 JSON 과 임베드 언어 import 를 담습니다. 예: html 은 javascript·css, cpp 는 cpp-macro·regexp·glsl, ruby 는 html·haml·xml·sql·graphql·css·cpp·c·javascript·shellscript·lua·yaml 을 끌어옵니다. 30종의 의존 합집합은 37개 모듈, 2433KB 입니다(`docs/research/shiki.md:132`). markdown 은 `embeddedLangsLazy` 라서 해당 언어가 이미 로드된 경우에만 코드 펜스가 강조됩니다(SHIKI-PRIM 272-296).
- 로딩: 시작 시 json·jsonc·markdown 만 로드하고 나머지는 그 언어의 모델이 처음 생길 때 로드합니다(`lang-map.ts:56`, `shiki-monaco.ts:43, 187-196, 206-217`).
- 플러그인·VSIX 문법: 플러그인 매니페스트의 `grammar` 는 JSON 파일만 허용합니다. Rust 검증이 `serde_json` 으로 파싱하고 `scopeName` 을 요구합니다(`crates/taide-plugin/src/service.rs:199-216`). VSIX 가져오기는 문법 파일 내용을 `grammars/<languageId>.tmLanguage.json` 으로 저장하므로(`crates/taide-vsix/src/service.rs:554-555`) plist 문법은 같은 검증에서 `GrammarInvalid` 가 됩니다. 즉 TS 도 plist 문법을 지원하지 않습니다. 본문은 `plugin_read_grammar` 로 읽고(`service.rs:218-247`), 프런트가 `name = 언어 id`, `embeddedLangs = embeddedLanguages` 로 등록하며 로드되지 않은 임베드 이름은 버립니다(`src/entities/plugin/plugin-grammar.ts:10-50`, `shiki-monaco.ts:120-142`). 언어 id·scopeName 충돌은 먼저 온 플러그인이 이깁니다(`service.rs:95-130`).
- 토큰 한도: 줄 길이 20000 이상은 토큰화하지 않고 상태를 그대로 넘기며, 줄당 시간 한도는 500ms 입니다(SHIKI-MONACO 93, 100-108). 20MB 초과 또는 30만 줄 초과 문서는 토큰화하지 않습니다(`MONACO/editor/common/model/textModel.js:120-121, 202-203`, `code-editor.tsx:182`).
- JS 엔진 호환: 공식 표 기준 238개 중 237개 지원, 불일치 0, TAIDE 31개는 전부 지원 범위입니다(`docs/research/shiki.md:59-60`). 따라서 Oniguruma 의미가 기준입니다.

### 5.2 후보 비교

조회일은 2026-10-06 입니다.

| 후보 | 최신 버전(공개일) | 라이선스 | MSRV | VS Code tmLanguage JSON 을 변환 없이 읽는가 | 줄 단위 증분·상태 | 정규식 엔진 | C 의존 | 유지 상태 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| syntect | 5.3.0 (2025-09-27) | MIT | crates.io 에 미기재 | 아니오. `.sublime-syntax` 만 | `ParseState` 복제·비교 가능 | onig 또는 fancy-regex 선택 | onig 선택 시 있음 | 누적 3061만 다운로드, 활발 |
| syntect + syntect-tmlanguage | 0.1.0 (2026-08-23) | MIT 또는 Apache-2.0 | 미확인 | JSON 을 sublime-syntax 로 번역. `begin`/`while`, `\G`, 선택자 injection 미지원 | syntect 와 같음 | onig 필수 | 있음. wasm 불가라고 명시 | 다운로드 73 |
| ferriki-textmate (+ ferroni) | 0.12.0 (2026-10-05), ferroni 1.8.1 (2026-10-01) | MIT 또는 Apache-2.0, ferroni 는 BSD-2-Clause | 1.94 | 예. `parse_raw_grammar` 가 JSON·plist 처리 | 예. `tokenize_line`·`tokenize_line2`, `StateStack` | ferroni(Oniguruma 의 순수 Rust 이식, vscode-oniguruma 형태 스캐너 포함) | 없음(ferroni 의 `cc` 는 선택적 build 의존) | 최초 공개 2026-09-28, 8일간 13회 배포, beta 표기, 다운로드 610 |
| syntaxmate | 0.2.1 (2026-09-30) | MIT | 1.88 | 예(문자열 JSON 로딩) | 예. `TokenizerState`, scope stack | 자체 구현(외부 정규식 크레이트 의존 없음) | 없음 | 최초 공개 2026-08-02, 다운로드 801 |
| giallo | 0.5.2 (2026-08-06) | EUPL-1.2 | 미기재 | 예(Shiki 문법 모음 사용) | 문서 단위 API 만 확인 | onig 포크(`onig-regset`) | 있음 | Zola 가 사용, 다운로드 2.2만 |
| zalo | 0.3.18 (2026-06-27) | EUPL-1.2 | 미기재 | giallo 와 같은 설명 | 미확인 | 미확인 | 미확인 | 다운로드 1247 |
| tree-sitter + tree-sitter-highlight | 0.27.0 (2026-08-30) | MIT | 1.90 | 아니오. 언어별 파서 크레이트와 highlight query 필요 | 증분 파싱 | 해당 없음 | 언어별 C 파서 | 누적 4334만 다운로드, 활발 |
| 자체 이식 + onig | onig 6.5.3 (2026-04-27) | MIT | 1.70 | 직접 구현 | 직접 구현 | Oniguruma C | 있음 | onig 는 활발 |
| 자체 이식 + fancy-regex | 0.19.2 (2026-09-13) | MIT | 1.66 | 직접 구현 | 직접 구현 | fancy-regex(Oniguruma 와의 문법 차이는 미확인) | 없음 | 활발 |

기준별 평가:

| 기준 | syntect 계열 | ferriki-textmate | syntaxmate | giallo | tree-sitter |
| --- | --- | --- | --- | --- | --- |
| 플러그인·VSIX 문법 재사용 | 불가(브리지는 기능 결손) | 가능 | 가능 | 가능 | 불가 |
| Shiki 와 같은 scope | 아니오(Sublime 패키지는 scope 체계가 다름. 번들 언어 범위는 미확인) | vscode-textmate 이식이라 같은 구조. 실제 일치는 게이트에서 검증 | 미확인 | "VSCode 와 같은 출력"을 표방 | capture 이름이라 다름 |
| 테마 tokenColors 재사용 | scope 가 달라 색이 달라짐 | `RawTheme` 이 tokenColors 구조. `tokenize_line2` 가 색 인덱스·글꼴 스타일 반환 | 자체 `Theme` | 가능 | 31개 syntax 토큰으로 근사 매핑만 가능 |
| 표준 토큰 종류 | 없음 | `StandardTokenType` 제공 | 미확인 | 미확인 | 없음 |
| Wasm | fancy-regex 선택 시 가능 | 의존성은 순수 Rust. wasm32 빌드는 미확인 | 미확인 | onig 때문에 어려움 | 가능하나 복잡 |
| 성능 | README: 62KB XML 34ms, fancy-regex 는 onig 의 절반 속도 | 공급자 주장: Shiki JS 엔진 대비 3.5배, ferroni 가 C Oniguruma 대비 2.7~5.7배. 독립 검증 없음 | 수치 없음 | 수치 없음 | 증분 파싱이 강점 |
| 라이선스 정책 적합 | 적합 | 적합 | 적합 | EUPL 은 copyleft. `docs/acknowledge/2026-09-05-license-mit-decision.md` 3절에 따라 도입 전 사용자 확인 대상 | 적합(언어별 파서는 개별 확인) |
| 31개 문법 확보 | 불가 | `@shikijs/langs` 4.4.3 의 JSON 그대로 | 같음 | 같음 | 언어별 크레이트를 새로 선정(미확인) |

### 5.3 사용자 결정과의 관계

- 결정 문구는 "TextMate 문법 호환 엔진(syntect 계열)"이고 근거는 "기존 Shiki 와 VSIX 문법이 TextMate 형식이라 재사용할 수 있습니다"입니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md:12`).
- 조사 결과 syntect 자체는 그 근거를 충족하지 못합니다. syntect 는 Sublime Text 문법 정의만 파싱하고(docs.rs `SyntaxDefinition`), 유일하게 찾은 변환 브리지는 VS Code markdown·yaml 문법이 쓰는 `begin`/`while` 과 injection 을 지원하지 못한다고 밝힙니다.
- 결정의 의도(TextMate 문법 재사용)를 충족하는 것은 vscode-textmate 호환 엔진입니다. 8절 D1 로 확인을 요청합니다.

### 5.4 추천안

추천: `ferriki-textmate` 0.12.0 + `ferroni` 1.8.1 을 `taide-native-syntax` 안의 내부 trait 뒤에 두고, 아래 게이트를 통과한 경우에만 채택합니다.

근거:

1. TS 파이프라인과 같은 계층의 API 를 가집니다. `tokenize_line2(line, 이전 StateStack, 시간 한도)` 가 색 인덱스·글꼴 스타일·표준 토큰 종류를 담은 32비트 메타데이터를 돌려주고 `SyncRegistry::get_color_map` 이 색 표를 줍니다. SHIKI-MONACO 가 쓰는 호출과 1:1 로 대응합니다(SHIKI-MONACO 107-116).
2. Shiki 4.4.3 과 vscode-textmate 9.3.2 를 수정 없이 미러링해 CI 에서 대조한다고 밝힙니다. TAIDE 가 쓰는 Shiki 버전과 같습니다.
3. 순수 Rust 입니다. 새로 내려받을 크레이트는 `ferriki-textmate` 와 `ferroni` 두 개뿐입니다. 나머지 전이 의존성(serde, serde_json, aho-corasick, bitflags 2, memchr, smallvec)은 이미 `native/taide-native-app/Cargo.lock` 에 있습니다.
4. 라이선스가 MIT 또는 Apache-2.0, BSD-2-Clause 로 기존 정책에 맞고 MSRV 1.94 가 native 크레이트의 1.95 이하입니다.

위험:

- 신생 크레이트입니다. `ferriki-textmate` 는 최초 공개 후 8일, GitHub 별 1개, 문서화율 21.84%, beta 표기입니다. API 가 바뀔 수 있고 유지 주체가 한 곳입니다.
- 성능·적합성 수치는 공급자 주장이며 이 저장소의 문법으로 검증한 적이 없습니다.
- `SyncRegistry` 는 `Send` 가 아니고 `grammar_for_scope_name` 이 `Rc<Grammar>` 를 돌려줍니다. 토큰화 스레드가 레지스트리를 소유해야 합니다(5.6).
- 토큰 오프셋 단위(UTF-16 인지 UTF-8 바이트인지)를 확인하지 못했습니다. ferroni 가 "UTF-16 offsets included" 스캐너를 제공한다고 하므로 UTF-16 일 가능성이 있습니다.

완화:

- 버전을 `=0.12.0`, `=1.8.1` 로 고정합니다(native 크레이트의 기존 관례).
- 엔진 형식을 `taide-native-syntax` 밖으로 내보내지 않습니다. 밖에서는 `LineTokens`, `TokenStyleTable`, 줄 상태 불투명 핸들만 봅니다.

게이트(3a 단계에서 수행, 하나라도 실패하면 채택하지 않음):

1. `--locked --offline` 운영 체계에서 내려받기와 lockfile 갱신 후 `cargo check` 통과.
2. 31개 언어 각 표본 파일에 대해 TS 스택이 만든 기준 자료(줄별 토큰 시작 위치와 최종 전경색·글꼴 스타일)와 일치. 기준 자료는 `@shikijs/core` + `createJavaScriptRegexEngine` + SHIKI-MONACO 의 `textmateThemeToMonacoTheme` + Monaco `TokenTheme`(`MONACO/editor/common/languages/supports/tokenization.js`)로 DOM 없이 만들 수 있습니다. 생성 스크립트는 `docs/utils/` 에 둡니다.
3. 성능: 가장 큰 문법(cpp 778KB, `lang-map.ts:46-47`)의 첫 토큰화 지연과 1만 줄 문서의 전체 토큰화 시간을 측정해 기록.
4. 줄당 시간 한도와 20000자 한도 동작 확인.

게이트 실패 시 차선: (1) `syntaxmate` 0.2.1 을 같은 게이트로 평가, (2) vscode-textmate 를 직접 이식하고 정규식은 `onig` 6.5.3(C 의존 수용) 사용. giallo 는 EUPL-1.2 와 문서 단위 API 때문에, tree-sitter 는 플러그인·VSIX 문법과 테마 tokenColors 를 재사용할 수 없어 TS 동작을 재현하지 못하므로 차선에 넣지 않습니다.

### 5.5 도입 시 새 의존성

| 크레이트 | 확인한 최신 버전 | 라이선스 | 왜 필요한가 | 들어가는 곳 |
| --- | --- | --- | --- | --- |
| ferriki-textmate | 0.12.0 | MIT 또는 Apache-2.0 | vscode-textmate 호환 문법 해석과 테마 매칭(`tokenize_line2`, `StateStack`, `SyncRegistry`) | `native/taide-native-syntax/Cargo.toml` |
| ferroni | 1.8.1 (ferriki-textmate 가 `^1.8.0` 으로 요구) | BSD-2-Clause | Oniguruma 문법 정규식 엔진. TextMate 문법의 정규식을 변환 없이 실행 | 전이 의존성 |

- lockfile 변경 대상: 신규 `native/taide-native-syntax/Cargo.lock`, `native/taide-native-app/Cargo.lock`. `taide-native-ui`·`taide-native-editor`·`taide-remote-web` 의 lockfile 은 바뀌지 않아야 합니다.
- `serde_json`·`serde`·`taide-model`·`taide-native-editor` 는 기존 의존성입니다. 문법 JSON 을 압축해 넣기로 하면 `flate2`(APP 이 이미 `=1.1.9` 사용)를 새 크레이트에 추가합니다. 바이너리 증가분을 측정한 뒤 정합니다.
- `THIRD_PARTY_LICENSES.md` 의 "Rust crates" 절에 두 크레이트를 추가해야 합니다.

### 5.6 토큰화 파이프라인

`taide-native-syntax` 의 공개 형태:

```rust
pub struct LineState(EngineState);

pub struct TokenizedLine {
    pub spans: Vec<u32>,
    pub kinds: Vec<taide_native_editor::syntax::Token>,
    pub end_state: LineState,
    pub is_stopped_early: bool,
}

pub trait GrammarTokenizer {
    fn tokenize_line(&mut self, language_id: &str, line: &str, previous: Option<&LineState>) -> Option<TokenizedLine>;
    fn is_same_state(&self, left: &LineState, right: &LineState) -> bool;
}
```

- `LineState` 는 엔진 상태(ferriki 의 `Arc<StateStack>`)를 감싼 불투명 핸들이고 `EngineState` 는 크레이트 밖으로 공개하지 않습니다.
- `spans` 는 (줄 안 UTF-8 바이트 시작, 스타일 id) 쌍입니다. 스타일 id 는 6절의 `TokenStyleTable` 인덱스입니다.
- worker: 레지스트리가 `Send` 가 아니므로 전용 스레드가 레지스트리·문법·토큰 테마를 소유합니다. 요청은 `{ 문서 id, revision, 언어 id, rope 스냅샷, 무효화 시작 줄, 보이는 줄 범위 }`, 응답은 `{ 문서 id, revision, 줄 범위, 줄별 TokenizedLine }` 입니다. 응답이 오면 `ctx.request_repaint()` 로 다시 그립니다. 스레드 수명은 APP 의 기존 종료 정리 흐름(LSP·PTY worker 와 같은 방식)에 등록합니다.
- 조율자(`native/taide-native-app/src/editor-syntax.rs`): 문서별 토큰 저장소(줄별 spans, 줄 끝 상태)를 메인 스레드에 두고 표면에 `&LineTokens` 로 빌려줍니다. 문서 revision 이 바뀌면 편집 저널에서 첫 변경 줄을 구해 그 줄부터 무효화하고, 저널로 줄 이동(삽입·삭제 줄 수)을 반영합니다.
- 증분: 변경 줄부터 다시 토큰화하다가 줄 끝 상태가 이전에 저장된 같은 줄의 상태와 같아지면 멈춥니다(vscode-textmate `StateStack` 비교, ferriki 의 `StateStack::equals`). Monaco 의 방식입니다(`MONACO/editor/common/model/textModelTokens.js`).
- 우선순위: 보이는 줄 범위를 먼저 처리하고 나머지는 뒤에서 이어갑니다. 아직 토큰이 없는 줄은 기본 전경색으로 그립니다. Monaco 도 첫 페인트 뒤에 배경 토큰화가 따라옵니다. Monaco 의 뷰포트 추정 토큰화(`textModelTokens.js:104-120` 의 `tokenizeHeuristically`)는 첫 구현에서 생략하고 차이를 QA 문서에 남깁니다.
- 한도: 줄의 UTF-16 길이가 20000 이상이면 토큰화하지 않고 상태를 그대로 넘깁니다. 줄당 500ms 한도. 문서가 20MB 초과 또는 30만 줄 초과면 토큰화하지 않습니다(5.1).
- 문법 로딩: TS 처럼 요청된 언어 집합(`json`, `jsonc`, `markdown` + 열린 문서의 언어 + 플러그인 문법이 임베드한 TAIDE 언어)을 유지하고, 집합이 커지면 레지스트리를 다시 만들어 영향받는 문서를 재토큰화합니다. markdown 코드 펜스가 "그 언어를 연 뒤에만" 강조되는 TS 동작이 그대로 나옵니다(`shiki-monaco.ts:43, 136-142, 187-196`).
- 플러그인 문법: `taide_runtime::plugin_actions::{plugin_list, plugin_read_grammar}` 를 씁니다(`native/taide-native-app/src/remote-plugins.rs:30-40` 이 같은 함수를 호출). 등록 규칙은 `plugin-grammar.ts:10-30` 과 `shiki-monaco.ts:120-127` 을 옮깁니다. 플러그인 재로드 시 레지스트리를 다시 만듭니다(`src/entities/plugin/plugin.query.ts:21-22`). 실패하면 강조 없이 평문으로 남깁니다(`shiki-monaco.ts:161-171`).
- 저장 정리 공급: 현재 revision 에서 위에서부터 연속으로 정확히 토큰화된 줄까지를 `SyntaxSnapshot` 으로 만들어 `install_syntax` 에 넣습니다. 그 아래 줄은 스냅샷에 넣지 않아 "정확하지 않은 줄은 건너뜀" 동작이 됩니다(2.4). 표준 토큰 종류는 6.2 의 방식으로 정합니다.
- 테마 변경: `tokenize_line2` 결과는 테마에 묶여 있으므로 전 문서를 재토큰화합니다. TS 도 테마 적용 때 토큰 공급자를 다시 붙여 재토큰화하며 150ms 로 debounce 합니다(`shiki-monaco.ts:25, 255-291`). 같은 debounce 를 둡니다.

### 5.7 문법 확보와 라이선스 고지

- 출처는 지금 TS 가 쓰는 것과 같은 `@shikijs/langs` 4.4.3 입니다. `dist/<id>.mjs` 안의 `JSON.parse("...")` 문자열을 꺼내 `native/taide-native-syntax/grammars/<id>.tmLanguage.json` 으로 저장합니다. 추출 스크립트는 `docs/utils/` 에 두고 30종과 임베드 합집합(37개 모듈)을 대상으로 합니다.
- 문법 파일은 수정하지 않습니다. hcl 이 MPL-2.0 이라 파일을 고치면 공개 의무가 생깁니다(`THIRD_PARTY_LICENSES.md:330-341`). JSON 재직렬화가 "수정"에 해당하는지 애매하므로 추출한 문자열을 바이트 그대로 저장합니다.
- 고지: `THIRD_PARTY_LICENSES.md` 228-380 줄에 30종 표와 회색 지대 4종(elixir, toml, yaml, erb)의 사용자 승인 기록이 있습니다. native 바이너리에 같은 파일을 포함한다는 문장을 추가하고, 표에 없는 임베드 모듈(37 - 30 = 7개)의 출처와 라이선스를 추가해야 합니다. 이 7개의 목록과 라이선스는 이번에 확인하지 못했습니다(9절). GPL 문법(ada, gnuplot, nginx, org, racket)이 합집합에 들어 있지 않은지도 추출 단계에서 확인합니다.
- Monaco 소스를 옮기는 부분(줄 나눔 계산, 토큰 테마 trie, 들여쓰기 접기, minimap 문자 시트)은 MIT 고지가 필요합니다. `native/taide-native-editor/LICENSE-MONACO-SNIPPET` 의 선례를 따릅니다.

## 6. 테마 연결

### 6.1 TS 가 색을 정하는 방식

편집기 UI 색(배경·전경·선택·캐럿·줄 번호 등)은 `ResolvedTheme.colors` 의 TAIDE 토큰을 Monaco 색 id 로 옮깁니다(`src/shared/lib/monaco/theme.ts:8-33, 174-198, 248-253`). native 는 이미 같은 TAIDE 토큰을 직접 읽습니다(`native/taide-native-ui/src/presentation.rs:213-218`).

토큰 색은 3단계를 거칩니다.

1. Shiki 테마 조립(`src/shared/lib/shiki/build-shiki-theme.ts:109-125`): `tokenColors = raw ++ overlay ++ semantic`.
    - raw: `resolved.tokenColors` 가 있으면 그대로, 없으면 `fallbackFromSyntax`(31개 syntax 토큰 → scope 후보, 먼저 나온 토큰이 scope 를 소유. `build-shiki-theme.ts:40-64`, `src/shared/lib/theme-convert/ui-token-vocabulary.ts:107-173`).
    - overlay: `syntaxOverrides` 에 있는 토큰만 같은 규칙으로 덧붙임.
    - semantic: `taideSemantic.<토큰>` scope 규칙(`build-shiki-theme.ts:91-99`, `semantic-token-map.ts:111-113`).
    - Shiki 가 규칙 맨 앞에 전역 설정 `{ foreground: editor.foreground, background: editor.background }` 를 넣고, `#` 로 시작하지 않는 색은 자리표시 색으로 바꿉니다(SHIKI-PRIM 113-195).
2. vscode-textmate 가 scope stack 을 이 규칙에 매칭해 색 인덱스와 글꼴 스타일 비트를 메타데이터에 넣습니다(`tokenizeLine2`).
3. SHIKI-MONACO 가 메타데이터를 Monaco 용 scope 문자열로 되돌립니다.
    - 테마 규칙을 scope 하나당 Monaco 규칙 하나로 펼칩니다. 전경·배경·글꼴 스타일이 모두 없는 규칙은 버립니다(SHIKI-MONACO 31-57).
    - `키 = 정규화한 전경색` 또는 `전경색|글꼴 스타일` 로, 그 키를 가진 첫 규칙의 scope 를 기억합니다. 전경색이 없는 규칙은 표에 들어가지 않습니다(SHIKI-MONACO 74-80).
    - 토큰마다 `colorMap[색 인덱스]` 와 글꼴 스타일로 키를 만들어 scope 를 찾고, 없으면 빈 문자열을 넘깁니다(SHIKI-MONACO 109-121).
    - Monaco 가 그 scope 를 토큰 테마 trie 에 매칭해 최종 전경색·글꼴 스타일을 정하고, scope 문자열에 `comment`·`string`·`regex`·`regexp` 단어가 있으면 표준 토큰 종류를 정합니다(`MONACO/editor/standalone/browser/standaloneLanguages.js:166-174`, `MONACO/editor/common/languages/supports/tokenization.js:166-183`).

이 구조에서 나오는 TS 고유 동작:

- 배경색 규칙은 화면에 반영되지 않습니다.
- 전경색 없이 글꼴 스타일만 가진 규칙은 반영되지 않습니다(키가 없어 빈 scope 가 됨).
- `#` 형식이 아닌 전경색은 자리표시 색이 돼서 키가 맞지 않으므로 기본 전경색이 됩니다.
- 표준 토큰 종류는 문법의 scope 가 아니라 "같은 색을 가진 첫 규칙의 scope 이름"으로 정해집니다. 문자열과 같은 색을 쓰는 다른 규칙이 먼저 있으면 문자열 안의 후행 공백이 저장 시 지워질 수 있습니다.

### 6.2 native 설계

`taide-native-syntax/src/token-theme.rs` 가 `ResolvedTheme` 하나에서 두 가지를 만듭니다.

1. 엔진용 테마: 6.1 의 1번을 Rust 로 옮겨 `Vec<RawThemeSetting>` 을 만들고 `SyncRegistry::set_theme` 에 줍니다. 입력 자료형은 이미 Rust 에 있습니다(`crates/taide-model/src/theme.rs:29-52, 95-114` 의 `SyntaxStyle`, `TokenColorRule`, `ResolvedTheme.token_colors`·`syntax_overrides`). scope 후보 표와 semantic 매핑은 TS 상수를 그대로 옮깁니다.
2. `TokenStyleTable`: `(색 인덱스, 글꼴 스타일 비트) → { 전경색, italic, bold, underline, strikethrough, 표준 토큰 종류 }` 표입니다. 6.1 의 3번을 그대로 계산합니다. 색 표는 `get_color_map()`, 역조회 표는 규칙 순서대로, 최종 스타일은 Monaco `TokenTheme` trie(`tokenization.js`)를 옮긴 함수로 구합니다. 조합 수가 작아 테마 변경 때 한 번 전부 계산합니다.

- 표면은 토큰의 스타일 id 로 `TokenStyleTable` 을 조회해 `RowSection` 을 채웁니다. 색 결정 로직은 표면에 없습니다.
- semantic 토큰은 LSP legend 의 type 이름을 `SEMANTIC_TOKEN_TYPE_MAP`(`semantic-token-map.ts:32-96`)으로 31개 syntax 토큰에 옮기고 `resolved.syntax[토큰]` 의 색을 씁니다. 표에 없는 type 은 버립니다. 구문 토큰 run 위에 덮습니다.
- 테마 편집기의 실시간 미리보기는 색이 초당 수십 번 바뀝니다. TS 와 같이 150ms debounce 를 두고(`shiki-monaco.ts:25`) UI 색은 즉시, 토큰 색은 debounce 뒤에 갱신합니다.
- 장식·위젯 색은 `ResolvedTheme.colors` 의 TAIDE 토큰에서 직접 읽습니다. 필요한 토큰은 이미 어휘에 있습니다(`ui-token-vocabulary.ts:51-73`): `editor.inactiveSelection`, `editor.lineNumberActive`, `editor.indentGuide`, `editor.whitespace`, `editor.bracketMatch`, `editor.findMatch`, `editor.findMatchHighlight`, `editor.hoverBackground`, `editor.widgetBackground`, `editor.widgetBorder`, `editorGutter.*`, `editorBlame.*`, `diff.*`, `scrollbar.thumb`, `scrollbar.thumbHover`. 진단 밑줄 색은 `statusIndicator.error`·`warning`·`info` 입니다(`theme.ts:186-188`).
- Monaco 가 테마에서 받지 못해 자체 기본값을 쓰는 색(괄호 쌍 색 6종, 선택 일치 강조 등)은 `shikiToMonaco` 가 `inherit: false` 로 등록하므로 `vs`·`vs-dark` 기본값입니다(`docs/research/shiki.md:83-86`). 해당 기능 단계에서 Monaco 소스의 기본값을 확인해 옮깁니다.

대안과 기각: 엔진이 준 색 인덱스를 색 표로 바로 쓰는 안은 6.1 의 TS 고유 동작(글꼴 스타일만 가진 규칙, 표준 토큰 종류)이 달라져 기각합니다. TS 보다 정확해지는 방향이지만 기준은 TS 동작입니다.

## 7. 구현 계획 (배치 4, 직렬)

공통 검증 계약은 배치 1 과 같습니다(`docs/quality-assurance/2026-10-06-native-batch1-editor-input.md` 4절). 공통 접미사는 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` 입니다.

| 기호 | 명령 |
| --- | --- |
| V-E | `cargo test --manifest-path native/taide-native-editor/Cargo.toml` |
| V-U | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface` (새 테스트 대상이 생기면 함께) |
| V-A | `cargo check --manifest-path native/taide-native-app/Cargo.toml` |
| V-W | `cargo check --manifest-path native/taide-remote-web/Cargo.toml` |
| V-F | 변경한 크레이트의 `cargo fmt -- --check` 와 `cargo clippy` |

테스트 도구: UI 테스트는 `Context::run_ui(RawInput, ..)` 로 한 프레임을 돌려 `output.shapes` 를 검사합니다(`native/taide-native-ui/tests/editor_surface.rs:45-83`). `Shape::Text` 의 `galley.job.sections[i].format.color` 를 읽으면 토큰 색을 단언할 수 있습니다. 순수 모델은 `native/taide-native-editor/tests/` 에 파일을 추가합니다.

### 단계 0. 결정 확인과 의존성 반입

- 목표: 8절 D1~D3 확정. 승인되면 네트워크로 두 크레이트를 내려받고 lockfile 을 갱신.
- 수정 파일: 없음(결정 기록은 `docs/acknowledge`).
- 선행 조건: 없음. 단계 1·2 는 이 결정과 무관하게 진행할 수 있습니다.

### 단계 1. 표시 계층 골격 (동작 불변)

- 목표: `show_with_input_route` 의 단일 함수를 표시 모델·줄 텍스트·좌표·페인트 층으로 나누되 화면과 입력 결과를 한 픽셀도 바꾸지 않습니다.
- 수정 파일: ES, `native/taide-native-ui/src/lib.rs`, 신규 `editor-row-text.rs`·`editor-geometry.rs`·`editor-paint.rs`, `native/taide-native-editor/src/lib.rs`, 신규 `display-map.rs`·`display-layout.rs`, `native/taide-native-editor/src/editing.rs`(시각 열 함수 공개 범위만).
- 내용:
    1. `DisplayMap::identity` 와 `VerticalLayout`(zone 없음)을 넣고 `줄 * line_height` 계산을 이 둘로 바꿉니다.
    2. `RowText` 를 넣되 탭 확장·주입 없이 문서 텍스트와 같게 만들고 `LayoutJob::simple(.., f32::INFINITY)` 와 같은 job 을 씁니다.
    3. `reveal`(ES 255-307)이 같은 좌표 함수를 쓰게 합칩니다.
    4. 페인트를 층별 함수로 나누고 순서는 지금과 같게 둡니다(ES 621-693, 727-771).
    5. `EditorPresentation`·`EditorDisplayOptions`·`show_presented`·`EditorOutput.geometry` 를 추가하고 기존 세 함수는 위임합니다.
- 선행 조건: 없음.
- 검증: 리팩터링 전에 특성화 테스트를 먼저 추가합니다. 고정 문서와 이벤트 3종(평문, 여러 줄 선택, 스크롤 후 IME 조합)에 대해 `output.shapes` 의 종류·rect·텍스트·위치 목록을 기대값으로 고정하고, 리팩터링 전 커밋에서 통과하는 것을 확인한 뒤 진행합니다. 이후 V-E, V-U(기존 13건 무수정 통과), V-A, V-W, V-F.
- TS 근거: 해당 없음(구조 변경). 층 구성은 `MONACO/editor/browser/view.js:114-167`.
- 위험: 스크롤 clamp 와 캐럿 따라가기의 계산 순서가 바뀌면 한 프레임 어긋남이 생깁니다. 기존 테스트 `커서_이동과_큰_선택_삭제는_스크롤을_현재_문서로_복원한다`, `scrollbar는_가로_scroll을_...` 가 이 순서를 고정합니다. `taide-native-app` 의 편집기 사용 테스트(`save.rs`, `keymap-tests.rs`, `shell_keymap.rs`, `zen.rs`, `tests/terminal-host.rs`, `tests/paste-shortcuts.rs`)는 V-A 로 컴파일만 확인되므로, 가능하면 해당 테스트도 실행합니다.

### 단계 2. 편집기 글꼴 체인과 줄 상자 정렬

- 목표: `editorFontFamily` 적용, CJK 폴백, 굵은 face 패밀리, 글리프의 줄 상자 가운데 정렬.
- 수정 파일: 신규 `native/taide-native-app/src/editor-fonts.rs`(`terminal_fonts.rs` 의 로더 구조 재사용), APP(글꼴 정의 적용 지점), `native/taide-native-ui/src/presentation.rs`(`editor_appearance` 의 `FontId` 패밀리), `native/taide-native-ui/src/font-families.rs`(편집기 패밀리 이름), ES(세로 오프셋).
- 내용: 체인은 TS 스택과 같게 사용자 글꼴 → 시스템 monospace → SFMono-Regular → Menlo → Apple SD Gothic Neo → 기본 monospace(`src/shared/lib/font-stack.ts`). 굵은 face 는 `ui-fonts.rs:64-74` 처럼 가변 글꼴이면 `wght` 축, 아니면 굵은 face 를 별도 패밀리로 등록. 줄 상자 정렬은 `y += round((line_height - 글꼴 줄 높이) / 2)`.
- 선행 조건: 단계 1.
- 검증: 글꼴 선택 로직 단위 테스트(`terminal_fonts.rs` 테스트 형태), V-U 에 세로 오프셋 단언 추가, V-A, V-W, 실기 확인(한글·굵은 글씨).
- TS 근거: `code-editor.tsx:183, 335-337`, `font-stack.ts`, `MONACO/editor/common/config/fontInfo.js:12, 23`.
- 위험: 글꼴 바이트 예산과 비동기 로딩(터미널 로더가 쓰는 예산·취소 구조를 그대로 따름). 세로 오프셋은 화면이 바뀌는 변경이므로 기존 UI 테스트의 y 좌표 단언을 함께 고칩니다. macOS 외 줄 높이 비율 1.35 는 플랫폼 범위가 정해진 뒤 반영합니다.

### 단계 3. 구문 강조

3a. 엔진 게이트

- 목표: 5.4 의 게이트 4개 수행과 결과 기록.
- 수정 파일: 신규 크레이트 `native/taide-native-syntax`(Cargo.toml, 최소 래퍼, 적합성 테스트), `docs/utils/` 의 기준 자료 생성·문법 추출 스크립트, `docs/quality-assurance` 결과 문서.
- 선행 조건: 단계 0.
- 검증: 31개 언어 표본의 기준 자료 일치 테스트, 성능 측정값 기록.
- 위험: 불일치가 나오면 원인이 엔진인지 JS 정규식 엔진 쪽 차이인지 구분해야 합니다. 기준은 TS 출력입니다.

3b. 편집 저널과 토큰 저장소

- 목표: `change-journal.rs`, `line-tokens.rs`, STORE 의 저널 기록(`apply_transaction`, `restore_history`).
- 수정 파일: `native/taide-native-editor/src/{lib.rs, store.rs}`, 신규 두 파일, `native/taide-native-editor/tests/` 신규 테스트.
- 선행 조건: 단계 1.
- 검증: V-E. 편집·undo·redo 뒤 저널 span 이 실제 변경 범위를 덮는지, 줄 이동 반영 후 토큰 줄 번호가 맞는지 단위 테스트.
- 위험: STORE 는 저장·충돌·hot exit 가 의존하는 핵심입니다. 저널은 읽기 전용 부가 기록으로만 넣고 기존 동작을 바꾸지 않습니다.

3c. 토큰 테마와 worker

- 목표: `token-theme.rs`(6.2), 문법 집합, `GrammarTokenizer` 구현, worker 스레드, APP 조율자.
- 수정 파일: `native/taide-native-syntax/src/*`, `native/taide-native-syntax/grammars/*`, `native/taide-native-app/Cargo.toml`·`Cargo.lock`, 신규 `native/taide-native-app/src/editor-syntax.rs`, APP, `THIRD_PARTY_LICENSES.md`.
- 선행 조건: 3a 통과, 3b.
- 검증: 번들 테마 전체(`crates/taide-theme/resources/themes/*.json`)에 대해 `TokenStyleTable` 이 기준 자료와 같은지, 증분 재토큰화의 조기 종료, 한도(20000자, 500ms, 20MB, 30만 줄) 단위 테스트. V-A.
- TS 근거: 5.1, 6.1 전부.
- 위험: worker 수명과 종료 순서. 문서가 닫히거나 revision 이 지난 응답은 버립니다.

3d. 표면 연결과 저장 정리 공급

- 목표: `RowText` 가 토큰 run 으로 섹션을 만들고, 조율자가 `install_syntax` 를 호출.
- 수정 파일: `native/taide-native-ui/src/editor-row-text.rs`, ES, APP(`show_document` 에서 `EditorPresentation` 구성), `native/taide-native-app/src/editor-syntax.rs`.
- 선행 조건: 3c, 단계 2(굵은 face).
- 검증: V-U 에 "토큰이 주어지면 섹션 색이 표와 같다" 테스트, 저장 정리 통합 테스트(문자열 안 후행 공백 보존, 주석 뒤 제거, 미토큰 줄 건너뜀), V-A, V-W, 실기 확인.
- TS 근거: `trimTrailingWhitespaceCommand.js:79-90`.
- 위험: 저장 정리가 지금까지 사실상 꺼져 있었으므로(2.4) 이 단계부터 사용자 파일이 실제로 바뀝니다. `trimTrailingWhitespaceOnSave` 가 켜진 경우에만 해당하지만 변경 사실을 QA 문서에 명시합니다.

3e. 플러그인 문법

- 목표: 플러그인·VSIX 문법 로딩과 재로드.
- 수정 파일: `native/taide-native-app/src/editor-syntax.rs`, `native/taide-native-syntax/src/*`.
- 선행 조건: 3c.
- 검증: 합성 플러그인(JSON 문법, 임베드 언어, 충돌)으로 통합 테스트.
- TS 근거: `plugin-grammar.ts`, `shiki-monaco.ts:120-171, 302-309`, `register-plugin-languages.ts:23-37`.

### 단계 4. word wrap

- 목표: `editorWordWrap`. 표시 줄 매핑, 탭 정지 표시, 표시 줄 기준 이동.
- 수정 파일: 신규 `native/taide-native-editor/src/line-breaks.rs`, `display-map.rs`, `editing.rs`(세로 이동·Home·End 가 표시 줄을 기준으로 하도록 `Motion` 확장), `view.rs`(affinity), `editor-row-text.rs`(탭 확장과 `model_bytes`), ES(gutter 는 문서 줄의 첫 표시 줄에만 번호, 가로 스크롤 비활성), APP(옵션 전달).
- 선행 조건: 단계 1. 단계 3 과는 독립이지만 순서상 뒤.
- 검증: `monospaceLineBreaksComputer.js` 의 동작을 고정하는 순수 테스트(공백·CJK·탭·break 문자·들여쓰기), `DisplayMap` 왕복(바이트 → 표시 줄 → 바이트) 속성 테스트, V-U 에 wrap 상태의 클릭·선택·캐럿 위치·PageDown 테스트, 대형 파일 wrap 계산 시간 측정.
- TS 근거: `code-editor.tsx:284`, `editorOptions.js:1241-1285, 2899, 3450-3455`, `MONACO/editor/common/viewModel/{monospaceLineBreaksComputer.js, viewModelLines.js, modelLineProjection.js}`, 커서의 뷰 줄 이동은 `MONACO/editor/common/cursor/cursorMoveOperations.js`.
- 위험: 탭 정지 확장은 wrap 이 꺼진 상태의 화면도 바꿉니다(탭 크기가 4 가 아니거나 탭이 열 중간에 있을 때). 기존 테스트의 폭 단언(`measure`)을 확인합니다. goal column 은 표시 줄 기준으로 다시 정의해야 합니다(배치 1 의 E6 과 충돌 주의).

### 단계 5. 장식·앵커 기반

- 목표: `decoration.rs`, 표면의 층 렌더(줄 배경, 인라인 배경·전경, 직선·물결 밑줄, lane 표식), `EditorGeometry` 의 좌표 질의, `editor-overlay.rs` 배치기.
- 수정 파일: 신규 `native/taide-native-editor/src/decoration.rs`, `native/taide-native-ui/src/{editor-paint.rs, editor-gutter.rs, editor-geometry.rs, editor-overlay.rs}`, ES.
- 선행 조건: 단계 1, 3b(저널).
- 검증: 장식 범위의 저널 이동 순수 테스트, V-U 에 합성 층을 넣어 rect·색 단언, 배치기 순수 테스트.
- TS 근거: 3.3, `MONACO/editor/browser/view.js:114-167`, `contentWidgets.js`.
- 위험: gutter 폭이 Monaco 식(줄 번호 폭 + 줄 장식 10 + 접기 16)으로 바뀌면 본문 x 가 달라집니다(`editorOptions.js:1220-1239`). 지금은 `줄 번호 폭 + 패딩 8 * 2` 입니다(ES 1029-1049). 화면이 바뀌는 변경이므로 이 단계에서 명시적으로 처리합니다.

### 단계 6. 찾기/바꾸기 위젯

- 목표: ⌘F·바꾸기, 일치 강조, 다음·이전, 옵션(대소문자·단어·정규식·선택 영역), 바꾸기·모두 바꾸기.
- 수정 파일: 신규 `native/taide-native-editor/src/find.rs`, 신규 `native/taide-native-ui/src/editor-find-widget.rs`, ES, `native/taide-native-ui/src/command-registry.rs`(`monaco.actions.find` 등 실행 연결), `native/taide-native-app/src/{shell_keymap.rs, command-dispatch.rs}`(`find` 액션).
- 선행 조건: 단계 5.
- 검증: 찾기 모델 순수 테스트(한도 19999, 순환, 선택 시드, 바꾸기 패턴), V-U 에 위젯 열기·입력·Enter·Escape·상단 블록 높이 33 테스트, 키맵 테스트(`command-dispatch.rs:189` 의 `find` 미지원 단언 갱신).
- TS 근거: `monaco-actions.ts:66-72, 409-463, 609`, `MONACO/editor/contrib/find/browser/{findController.js, findModel.js, findWidget.js, findDecorations.js, replacePattern.js}`, 상수 `findModel.js:68-69`, `findWidget.js:62-67`, 옵션 기본값 `editorOptions.js:462-471`, 키 기본값 `native/taide-native-ui/src/keymap-defaults.json:33`.
- 위험: 정규식 엔진. Monaco 는 JS 정규식입니다. APP 은 `regex` 1.13.1 을 이미 쓰지만(`native/taide-native-app/Cargo.toml`) 찾기 모델을 `taide-native-editor` 에 두려면 그 크레이트에 정규식 의존성이 새로 필요하고 wasm 공유 경계에 걸립니다. 찾기 모델의 정규식 부분은 trait 로 빼서 APP 이 구현을 넣는 방식을 권합니다. JS 정규식과 `regex` 의 문법 차이(후방 참조·전후방 탐색 없음)는 이 단계에서 사용자 결정이 필요할 수 있습니다(8절 D6).

### 단계 7. 접기

- 목표: 들여쓰기 기반 접기 영역, gutter 컨트롤, 숨김 줄, 접기 명령, 접힌 줄 표시.
- 수정 파일: 신규 `native/taide-native-editor/src/folding.rs`, `display-map.rs`(숨김 줄), STORE(undo·redo 에서 접기 유지), `editor-gutter.rs`, ES, `command-registry.rs`(접기 액션 연결), APP.
- 선행 조건: 단계 4(`DisplayMap` 이 숨김과 wrap 을 함께 다룸), 단계 5(gutter).
- 내용: `ViewState.folds` 의 의미를 "숨겨진 첫 줄의 시작 바이트부터 숨겨진 마지막 줄의 내용 끝 바이트까지"로 고정합니다. 편집 뒤에는 줄 경계로 정규화하고 비면 버립니다. 캐럿이 숨김 범위에 들어가거나 숨김 범위가 편집되면 펼칩니다.
- 검증: `indentRangeProvider.js` 동작 순수 테스트, 숨김과 wrap 이 함께 있을 때의 `DisplayMap` 테스트, V-U 에 gutter 클릭·명령·캐럿 진입 테스트.
- TS 근거: `code-editor.tsx:271`(대형 파일에서 꺼짐), `monaco-actions.ts:747-827`, `MONACO/editor/contrib/folding/browser/{folding.js, foldingModel.js, foldingRanges.js, hiddenRangeModel.js, indentRangeProvider.js, foldingDecorations.js}`, 한도 5000(`indentRangeProvider.js:8`), 옵션 `editorOptions.js:3213-3224, 3357`.
- 위험: 접기 표식(marker)과 offSide 규칙은 언어 구성에 있습니다(`indentRangeProvider.js:20-22`). native 에는 언어 구성 표가 없으므로(배치 1 문서 7절) 이 단계는 표식 없는 들여쓰기 접기까지만 하고 표식·LSP folding range 는 단계 9·LSP 배치로 넘깁니다. 넘긴 차이는 QA 문서에 남깁니다.

### 단계 8 이후 (나머지)

| 단계 | 목표 | 주요 파일 | 선행 | TS·Monaco 근거 | 위험·메모 |
| --- | --- | --- | --- | --- | --- |
| 8. 렌더 옵션 | 캐럿 스타일·깜빡임·부드러운 이동, 선택 모서리·줄바꿈 폭, 현재 줄 번호 색, 공백 표식, rulers, 들여쓰기 가이드, scrollBeyondLastLine, 스크롤 그림자, smoothScrolling, 스크롤바 색 토큰 | `editor-paint.rs`, ES, APP | 1, 5 | `MONACO/editor/browser/viewParts/` 아래 `viewCursors/viewCursor.js:135`, `viewCursors/viewCursors.js`, `selections/selections.js:50, 204`, `whitespace/whitespace.js`, `indentGuides/indentGuides.js`, `rulers/rulers.js`, `scrollDecoration/scrollDecoration.js`, 그리고 `src/shared/lib/monaco/theme.ts:175-178` | 깜빡임은 `request_repaint_after` 로 구동. 기본값이 켜져 있는 항목이 많아 화면 변화가 큼 |
| 9. 언어 구성과 괄호 | 언어별 괄호·주석·접기 표식 표, 괄호 매칭 강조, 괄호 쌍 색·가이드 | 신규 `language-configuration.rs`, `editor-paint.rs` | 3, 5, 7 | `MONACO/basic-languages/<언어>/<언어>.js` 의 `conf`, `MONACO/editor/common/model/bracketPairsTextModelPart/*`, `standaloneLanguages.js:174`(모든 토큰에 balanced brackets 비트) | Monaco 에 등록되지 않은 9개 언어 id(`docs/acknowledge/2026-08-12-w7-textmate-contract.md:33-35`)의 기본 구성은 미확인 |
| 10. 진단·overview | 물결 밑줄, hover 앵커, overview ruler, 다음·이전 문제 이동 | 신규 `native/taide-native-app/src/editor-decorations.rs`, `editor-paint.rs` | 5 | `src/shared/lib/lsp/adapters/diagnostics.ts`, `decorationsOverviewRuler.js` | 진단 저장소는 이미 있음(`native/taide-native-app/src/diagnostics.rs:42-78`) |
| 11. sticky scroll | 상단 고정 줄 | 신규 `editor-sticky-scroll.rs` | 7 | `contrib/stickyScroll/browser/*`, `editorOptions.js:1382` | document symbol 이 없으면 접기 모델로 대체 |
| 12. minimap | 축소 렌더, 슬라이더, 표식, `taide.toggleMinimap` | 신규 `editor-minimap.rs` | 3, 5 | `browser/viewParts/minimap/*`, `editorOptions.js:1058-1190, 1518-1529`, `code-editor.tsx:246-252` | 텍스처 갱신 비용. 문자 시트 이식의 고지 |
| 13. 주입 텍스트 | blame, inlay hint, ghost text, 조합 문자열의 줄 안 표시 | `editor-row-text.rs`, `decoration.rs` | 5 | 3.3, `MONACO/editor/common/viewLayout/lineDecorations.js` | 캐럿이 주입 텍스트에 들어가지 않게 하는 규칙. IME 표시가 바뀜 |
| 14. 줄 사이 블록·content widget | code lens, AI 편집 미리보기·입력 | `display-layout.rs`, `editor-overlay.rs`, APP | 5 | `ai-inline-edit.ts`, `MONACO/editor/browser/viewParts/viewZones/viewZones.js` | 블록 높이 변경 시 스크롤 보정 |
| 15. 접근성 | AccessKit 노드 | ES | 1 | 4.7 | `Galley::concat` 검증 필요 |
| 16. semantic token | LSP 토큰 run 덮어쓰기 | `line-tokens.rs`, APP | 3 | `semantic-token-map.ts`, `src/shared/lib/lsp/adapters/semantic-tokens.ts` | LSP 기능 요청 경로가 선행(감사 문서 7장) |
| 17. diff 편집기 | 표면 2개, 스크롤 동기, 정렬 블록 | 별도 설계 필요 | 5, 14 | `src/features/git/diff-view.tsx:46-81` | 이 문서 범위 밖. 별도 배치 |

다중 커서의 입력 경로(추가 캐럿 만들기, 열 선택)는 편집기 명령 계층의 일이며 표시 계층은 이미 여러 선택을 그립니다(ES 637-671).

## 8. 사용자 결정이 필요한 사항

1. D1 — 구문 강조 엔진을 "syntect 계열"에서 vscode-textmate 이식 엔진으로 바꿀지. **A안(추천): `ferriki-textmate` 0.12.0 + `ferroni` 1.8.1 을 게이트 통과 조건으로 채택. 플러그인·VSIX 문법과 테마 tokenColors 를 그대로 재사용할 수 있는 유일하게 확인된 허용 라이선스 후보** / B안: 게이트부터 `syntaxmate` 0.2.1 로 수행 / C안: vscode-textmate 를 직접 이식하고 `onig`(C 의존) 사용 / D안: syntect 를 유지하고 Sublime 문법으로 대체(색과 지원 언어가 TS 와 달라지고 플러그인 문법 기능을 잃음).
2. D2 — 새 의존성 반입(네트워크 fetch 와 `native/taide-native-app/Cargo.lock` 갱신) 승인. **A안(추천): D1 의 두 크레이트만 버전 고정으로 반입** / B안: 보류.
3. D3 — 문법 JSON 을 native 바이너리에 포함하는 것과 라이선스 고지 범위. **A안(추천): TS 와 같은 `@shikijs/langs` 4.4.3 의 30종과 임베드 모듈을 무수정으로 포함하고 2026-08-12 의 회색 지대 4종 승인을 native 바이너리에도 적용, 임베드 7개 모듈의 라이선스를 추출 단계에서 확인해 문제가 있으면 다시 보고** / B안: 회색 지대 4종을 native 에서는 제외.
4. D4 — 리거처 끄기. epaint 0.36.2 는 글꼴의 liga·calt 를 끌 수 없습니다. **A안(추천): 한계로 기록하고 진행. 기본 글꼴 체인에서는 영향이 없을 것으로 보이며 리거처 글꼴을 고른 경우에만 항상 켜짐** / B안: epaint 를 vendoring 해 feature 전달을 추가(새 vendored 크레이트와 유지 부담) / C안: `editorFontLigatures` 설정을 native 에서 "항상 켜짐"으로 표시.
5. D5 — 컬러 이모지와 bidi 는 egui 0.36.2 에서 표현할 수 없습니다. **A안(추천): 한계로 기록하고 진행** / B안: epaint vendoring 으로 해결을 시도(범위가 큼).
6. D6 — 찾기 위젯의 정규식 방언. Monaco 는 JS 정규식입니다. **A안(추천): 6단계 착수 시 후보(기존 `regex`, `fancy-regex`, ECMAScript 호환 엔진)를 공식 문서로 비교해 다시 보고** / B안: 기존 `regex` 로 진행하고 차이를 문서화.

원칙에 따라 결정 없이 적용한 것: 토큰 색의 3단계 결정과 TS 고유 동작(6.1), markdown 코드 펜스의 지연 강조(5.6), 브라우저 클라이언트는 평문 유지(4.8).

## 9. 확인하지 못한 사항

- 빌드·테스트·실행을 하지 않았습니다. 설계의 타당성은 소스 읽기에 근거합니다.
- `ferriki-textmate`: 이 저장소 문법에 대한 실제 적합성, 성능, 메모리, 토큰 오프셋 단위, `time_limit_millis` 의 정확한 동작, wasm32 빌드 가능 여부, 임베드·injection 처리가 Shiki 레지스트리와 같은지. 공급자 README 와 docs.rs 의 서술만 확인했습니다. Rust 툴체인 1.98.1 에서의 빌드도 실행하지 않았습니다.
- `ferroni`: 성능 수치와 "모든 upstream UTF-8 테스트 통과"는 공급자 주장입니다.
- `syntaxmate`: 자체 정규식 엔진의 Oniguruma 호환 범위, plist 지원, 적합성 테스트 유무.
- `giallo`·`zalo`: 줄 단위 재개 API 유무, zalo 의 정규식 엔진.
- syntect 의 번들 문법이 31개 언어를 어디까지 덮는지, scope 이름이 VS Code 문법과 얼마나 다른지. 본문에서는 "VS Code 문법을 읽지 못한다"는 확인된 사실만 판단 근거로 썼습니다.
- tree-sitter: 31개 언어 각각의 파서 크레이트 존재 여부·라이선스·유지 상태.
- `fancy-regex` 와 Oniguruma 의 문법 차이.
- 임베드 문법 합집합 37개 모듈 중 `THIRD_PARTY_LICENSES.md` 표에 없는 7개의 이름과 라이선스. cpp 와 ruby 모듈의 import 만 확인했습니다(cpp-macro, regexp, glsl, haml, xml, sql, graphql 이 보였으나 전체 합집합을 계산하지는 않았습니다).
- egui 기본 monospace 체인(Hack, Ubuntu-Light)의 한글 글리프 범위. 글꼴 파일을 검사하지 않았습니다.
- 리거처가 실제 화면에 나타나는지. 소스상 기본 feature 가 적용된다는 것만 확인했습니다. SF Mono·Menlo·Hack 에 프로그래밍 리거처가 없다는 것도 확인하지 않았습니다.
- 전 문서 wrap 계산 비용과 minimap 텍스처 갱신 비용.
- `Galley::concat` 으로 합친 galley 가 `update_accesskit_for_text_widget` 과 올바르게 동작하는지.
- Monaco 에 등록되지 않은 9개 언어 id 의 기본 언어 구성(괄호·주석)과 `inherit: false` 테마에서의 괄호 쌍 색 기본값.
- Monaco `viewLayout.js` 의 `scrollBeyondLastLine` 스크롤 높이 식과 찾기 위젯 상단 블록의 정확한 스크롤 보정. 해당 단계에서 소스를 읽어야 합니다.
- Monaco 의 조합 입력 표시(조합 중 모델 갱신)는 기억에 근거한 서술이며 이번에 소스로 확인하지 않았습니다. 13단계에서 `MONACO/editor/browser/controller/editContext` 를 확인하십시오.
- macOS 외 플랫폼의 지원 범위가 미결이라 줄 높이 비율 1.35 적용 여부를 정하지 못했습니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md` 미결 절).
- `native/taide-native-app` 의 편집기 사용 테스트 6개 파일이 이 리팩터링에서 실제로 통과하는지는 구현 단계에서 실행해야 합니다.

## 10. 출처

외부 조회(2026-10-06):

- syntect: https://crates.io/api/v1/crates/syntect , https://crates.io/api/v1/crates/syntect/5.3.0/dependencies , https://raw.githubusercontent.com/trishume/syntect/master/Readme.md , https://raw.githubusercontent.com/trishume/syntect/master/Cargo.toml , https://docs.rs/syntect/5.3.0/syntect/parsing/syntax_definition/struct.SyntaxDefinition.html , context7 `/trishume/syntect`
- syntect-tmlanguage: https://crates.io/api/v1/crates?q=tmLanguage , https://raw.githubusercontent.com/orofarne/syntect-tmlanguage/main/README.md
- ferriki-textmate: https://crates.io/api/v1/crates/ferriki-textmate , https://crates.io/api/v1/crates/ferriki-textmate/0.12.0/dependencies , https://docs.rs/ferriki-textmate/0.12.0/ferriki_textmate/ (Grammar, SyncRegistry, StateStack, TokenizeLineResult2, EncodedTokenAttributes, RawTheme, parse_raw_grammar, all.html) , https://raw.githubusercontent.com/sebastian-software/ferriki/main/README.md , https://github.com/sebastian-software/ferriki
- ferroni: https://crates.io/api/v1/crates/ferroni , https://crates.io/api/v1/crates/ferroni/1.8.1/dependencies , https://raw.githubusercontent.com/sebastian-software/ferroni/main/README.md
- syntaxmate: https://crates.io/api/v1/crates/syntaxmate , https://docs.rs/syntaxmate/latest/syntaxmate/ , https://raw.githubusercontent.com/phongndo/syntaxmate/main/README.md , https://raw.githubusercontent.com/phongndo/syntaxmate/main/Cargo.toml
- giallo: https://crates.io/api/v1/crates/giallo , https://raw.githubusercontent.com/getzola/giallo/master/README.md , https://raw.githubusercontent.com/getzola/giallo/master/Cargo.toml , https://docs.rs/giallo/latest/giallo/
- zalo: https://crates.io/api/v1/crates/zalo
- neco-syntax-textmate: https://crates.io/api/v1/crates/neco-syntax-textmate
- tree-sitter: https://crates.io/api/v1/crates/tree-sitter , https://crates.io/api/v1/crates/tree-sitter-highlight
- onig: https://crates.io/api/v1/crates/onig
- fancy-regex: https://crates.io/api/v1/crates/fancy-regex
- 크레이트 검색: https://crates.io/api/v1/crates?q=textmate&per_page=40&sort=downloads

저장소 안의 근거 문서:

- `docs/quality-assurance/2026-10-06-native-audit-editor.md`, `2026-10-06-native-batch1-editor-input.md`
- `docs/acknowledge/2026-10-06-native-transition-decisions.md`, `2026-08-12-w7-textmate-contract.md`, `2026-09-05-license-mit-decision.md`
- `docs/research/shiki.md`, `docs/theme-system.md`, `THIRD_PARTY_LICENSES.md`
- `native/taide-native-app/vendor/egui-input/UPSTREAM.md`
