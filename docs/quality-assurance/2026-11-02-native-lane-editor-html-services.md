# lane editor 묶음 4/4 (html-services) — editor-57 E5 HTML 구조·접기·선택 확장·포맷

> 작성일: 2026-11-03 · worktree `TAIDE-lanes/editor`(브랜치 `lane/editor`) · target `TAIDE-lanes/targets/editor`
> 기준: `docs/research/2026-10-28-lane-editor-preview-spec.md`(이하 사양) 2.4·2.5·2.7·2.8 E5, 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(결정 4: HTML 순수 Rust 이식, 새 의존성 없음)

## 대상 ID와 결과

| ID | 결과 | 요약 |
| --- | --- | --- |
| editor-57 E5 | partial 유지 | HTML scanner·parser·문서 심볼·접기·선택 확장·포맷(beautify-html, `<style>`은 E4 beautify-css 재사용) 이식과 실제 App 공급 완료. 완성·hover(웹 자료 표 생성 필요)·하이라이트·링크·rename은 남음. E6(TS/JS)은 결정 4 허락 대기로 보류 |
| editor-16 | partial 유지 | HTML 고정 줄: 내장 문서 심볼(Monaco 고정 줄 모델 1순위 outline) 공급, 실제 App에서 스크롤 시 고정 줄 표시 확인. CSS 문서 심볼(파서 뒤)·TS/JS 남음 |
| editor-32 | partial 유지 | HTML 붙여넣기 포맷 실제 App 확인(원본 HTML은 onType 공급자 없음). TS/JS 남음 |
| editor-49 | partial 유지 | HTML 문서·선택·저장 포맷 실제 App 확인. TS/JS(E6) 남음 |

사양 1.3의 정정 제안 확인: HTML은 진단·색 공급자를 등록하지 않으므로(`MON/languages/features/html/htmlMode.js`의 `registerProviders`) 남은 범위에 넣지 않았습니다.

## 시작 상태

- 작업 트리에 앞 묶음(display-commands·json-services·css-services)의 미커밋 변경이 있어 그대로 두었습니다. 이번 범위(html)의 기존 변경은 없었습니다.

## 원본 근거

- scanner: `MEXT/vscode-html-languageservice/lib/esm/parser/htmlScanner.js` 전체(406줄). 요소 이름 `^[_:\w][_:\w-.\d]*`(ASCII), 속성 이름 `^[^\s"'></=\x00-\x0F\x7F\x80-\x9F]*`, 따옴표 없는 값 `^[^\s"'`=<>]+`와 `<a b=x/>` 되돌리기, `type` 값이 `text/x-handlebars-template`·`text/html`이면 script 안을 HTML로 읽음, script 내용의 `<!--`/`<script`/`</script` 3단계 상태, style은 `</style` 까지.
- parser: `parser/htmlParser.js`(pseudo close tag 켬, 끝 태그로 조상까지 닫기, void 요소 자동 닫힘, `findNodeAt`). void 목록은 `languageFacts/dataManager.js`의 `getVoidElements`(기본 자료 `webCustomData.js`의 `void: true` 14개).
- 심볼: `services/htmlSymbolsProvider.js`. Monaco HTML 워커는 평면 `findDocumentSymbols`(SymbolInformation, `containerName` = 부모 이름)를 쓰고(`MON/languages/features/html/htmlWorker.js:76-84`), 어댑터가 children 없는 DocumentSymbol로 바꿉니다(`MON/languages/features/common/lspLanguageFeatures.js:403-428`). 이름은 태그 + `#id` + `.class...`(따옴표 제거, `\s+` 분리, 빈 조각 유지), 없으면 `?`, 종류 Field, 범위=선택 범위=요소 전체.
- 접기: `services/htmlFolding.js`(태그 스택, void 요소는 시작 태그 닫힘에서 처리, 끝 줄 −1, 같은 시작 줄 중복 제외, `/^\s*#(region\b)|(endregion\b)/`, 여러 줄 주석). Monaco가 넘기는 context에 `rangeLimit`이 없어(`MON/editor/contrib/folding/browser/syntaxRangeProvider.js`) `limitRanges`는 실행되지 않습니다.
- 선택 확장: `services/htmlSelectionRange.js`(부모 태그의 내용·전체 범위, 시작 태그 안·내용·끝 태그 이름, void/self-close, 속성 이름·값·따옴표 안 값, 짝 안 맞는 태그는 요소 범위 하나, 인접 중복 제거, 범위가 없으면 커서 위치).
- 포맷: `services/htmlFormatter.js`(범위 줄 공백 확장, 태그 안 시작이면 원문 반환, 시작 줄 들여쓰기 수준 유지, `trimLeft`), `beautify/beautify-html.js`(Output·OutputLine 줄 감싸기, Tokenizer, Beautifier 전체), Monaco 옵션 `MON/languages/features/html/register.js:31-44`와 워커 `html/htmlWorker.js:37-45`(기본값과 tabSize·insertSpaces 병합). `<script>`의 JS beautifier는 무변경 mock(`beautify/beautify.js`), `<style>`은 beautify-css.

## 구현

### ED (portable core, wasm32 확인)

- `html-scanner.rs`(`pub mod html_scanner`): `Scanner::new(text, offset, state, emit_pseudo_close_tags)`, 원본 TokenType 22종·ScannerState 11종. byte offset 기준이며 구분자는 모두 ASCII라 원본 UTF-16 offset과 같은 경계입니다. `\s`는 JS 공백 집합(`json_parser::is_ecma_whitespace`). 재귀 `internalScan()`은 반복으로 바꿨습니다. 오류 메시지(l10n)는 HTML 진단 공급자가 없어 옮기지 않았습니다.
- `html-parser.rs`: 노드 arena(0번 = 문서 루트), `attribute(name)`(마지막 값, 값 없는 속성은 `Some(None)`), `find_node_at`, `VOID_ELEMENTS`, 토큰·노드 상한.
- `html-symbols.rs`: 전위 순서 평면 심볼(`name`·`container`·`bytes`), 명시 스택(깊은 문서에서 재귀 없음), 이름 byte 상한.
- `html-folding.rs`: 원본 순서·규칙 그대로, 결과를 시작 줄로 안정 정렬(Monaco `sanitizeAndMerge`와 같은 순서라 공통 `SyntaxFolds::new`의 공급자별 상한이 앞쪽 범위를 남김), 깊이·범위·토큰 상한.
- `html-selection.rs`: 위치별 안쪽→바깥 byte 범위 사슬(JSON 선택 확장과 같은 반환 형태), 위치·범위·토큰 상한.
- `html-format.rs`: Monaco 고정 옵션(`wrap_line_length` 120, `wrap_attributes` auto, templating none, `indent_handlebars` false, `extra_liners` head/body//html, `content_unformatted` pre, `max_preserve_newlines` 기본 32786, 탭이면 `indent_size` 1→4)에서 도달하는 경로 전체를 이식했습니다. OutputLine의 UTF-16 글자 수·wrap point·정렬(alignment)·음수 들여쓰기 원시 줄, Tokenizer(속성·값·raw content·주석/CDATA/조건부 주석/처리 지시·`<!-- beautify ignore:start -->` 지시문), TagStack과 선택적 끝 태그(`p`·`li`·`dd/dt`·`tr`·`td/th`·`option` 등), custom beautifier(css·javascript mock·html 재귀·`test/null`), 주석/CDATA로 감싼 script 내용 분해. CSS는 `css_format::beautify`를 `pub(crate)`로 열어 재사용했습니다(1줄).
- 원본 옵션 문자열 결함: `register.js`의 `unformatted` 기본값이 `'default": "a, abbr, …'`로 시작해 첫 항목이 `default": "a`가 됩니다. 태그 검사 문자열은 공백을 포함할 수 없어 이 항목은 어떤 태그와도 일치하지 않으므로, native 목록에서 그 항목을 뺀 것과 결과가 같습니다(원본과 똑같이 `<a>`는 unformatted가 아님).
- force·aligned·preserve 속성 줄 감싸기, handlebars·angular control flow, templating 분기는 Monaco 옵션으로 도달하지 않아 옮기지 않았습니다.

### APP

- `editor-html.rs`(새 모듈, `lib.rs` 편집기 모듈 목록에 1줄): `is_html`(언어 `html`·Normal·AppFile 아님), `models`(심볼 + 접기), `selection_ranges`, `is_format_eligible`(+readonly 아님), `format`(문서/범위, onType은 공용 범위 처리에서 거절).
- 구조: 기존 내장 구조 상태(`editor-json.rs`, 감독 worker·generation·닫힘/취소)가 html 문서도 받습니다. `provides_symbols`에 html 추가, `build`에 html 분기 1개. 심볼은 원본처럼 평면(`parent: None`, `container_label` = 부모 이름)으로 `DocumentSymbols::from_symbols`에 넣어 고정 줄 모델과 `@` 팔레트가 그대로 씁니다.
- 선택 확장: `editor-selection-ranges.rs`의 내장 경로에 `is_builtin`(json 또는 html)과 `builtin_ranges`(html이면 `editor_html::selection_ranges`, 아니면 기존 `json_ranges`)를 추가하고 호출 2곳만 바꿨습니다. 기존 worker 이름·slot(`native-json-selection`)은 최소 변경 원칙으로 그대로 둡니다.
- 포맷: `editor-css.rs`의 범위 병합·작업 예산 로직을 `format_ranges`(공급자 클로저를 받음)로 꺼내 CSS·HTML이 함께 씁니다(동작 변화 없음, CSS `[A]` 재통과로 확인). `editor-formatting.rs`의 `is_native_eligible`·`native_build`에 html 1줄씩.
- 새 앱 상태·Host 명령·사용자 문구 없음(locale 키 추가 없음, `domains::editor::State` 신설 불필요).
- 실제 앱 경로: `main.rs` → `NativeApplication` → `background_tick` → `domains/editor.rs::reconcile_builtin_json` → `editor_json::State::tick` → `build` → `editor_html::models` → `editor_symbols.install_builtin`·`editor_folding.install_builtin` → 고정 줄 / 스마트 선택 키 → `editor_selection_ranges::State` → `builtin_ranges` → `editor_html::selection_ranges` / 팔레트·붙여넣기·저장 → `editor_formatting::State` → `native_build` → `editor_html::format` → `apply_edits`.

### 원본 결함 비재현(근거)

- JS 객체 키 순서(정수 모양 속성 이름이 앞으로 감)·`__proto__` 속성: native는 삽입 순서 목록. 심볼 이름은 `id`·`class` 조회만 하므로 결과 차이 없음.
- scanner가 진전 없을 때 1 UTF-16 단위 전진(서로게이트 쌍 분리): native는 1글자 전진.
- 태그 없는 노드에 속성이 붙을 때 `"undefined#id"` 이름: scanner 상태상 도달 불가, native는 빈 태그로 처리.

## 검사

| 표기 | 검사 | 결과 |
| --- | --- | --- |
| `[C]` | `taide-native-editor --test html-services` 7건: scanner token 열(속성·값 없는 속성·주석·doctype·script 안 `<!--<script></script>-->`·`type="text/html"` script·대문자 `</STYLE>`·`<a href=http://x/>` 되돌리기·태그 이름 없음), parser 암시적 닫힘·void·self-close·노드 상한, 심볼 이름(`div#m.a.b`, `i#..x.`, `?`)과 컨테이너·byte 범위, 접기(요소·region·void 여러 줄·여러 줄 주석·같은 시작 줄 중복 제외·`#regionx`/`myendregion`·깊이 상한), 선택 확장(따옴표 안 값→값→속성→시작 태그 안→요소, 내용, 끝 태그 이름, void, 루트 커서, 위치 상한), 포맷(block/inline/unformatted span·extra liner와 head/body·pre 원문·void 뒤 주석 같은 줄·120열 단어 줄바꿈·탭·`li` 선택적 끝 태그·style CSS·script mock·주석 감싼 script·알 수 없는 type), 범위(줄 공백 확장·시작 들여쓰기·태그 안 원문 반환·출력 상한) | 7 통과 |
| `[A]` | `application::json_tests::html_tests::actual_app의_html은_요소_심볼로_고정줄과_접기를_공급한다`: 실제 `.html` 파일 → 내장 접기 (0,60), 심볼 61개(`div#main.box.wide`, 자식 컨테이너), sticky 모델, 고정 줄 켬 + 20줄 스크롤 뒤 `sticky-line` 0 응답 | 통과 |
| `[A]` | `…html_tests::actual_app의_html은_스마트선택으로_속성값에서_속성까지_확장하고_되돌린다`: `"foo"` 안 커서에서 ⌃⇧⌘→ 반복으로 `class="foo"`까지 확장(내장 공급 없이는 도달 불가), ⌃⇧⌘← 같은 횟수로 커서 복귀 | 통과 |
| `[A]` | `…html_tests::actual_app의_html은_외부서버없이_문서_선택_붙여넣기_저장_포맷을_공급한다`: 팔레트 formatDocument·formatSelection 결과=core 결과·undo 원문, format-on-paste 결과·undo 두 번, format-on-save 디스크·문서=core 결과·undo | 통과 |

실패 우선 확인: 구조·포맷 `[A]` 2건은 작성 뒤 `editor_html`의 언어 ID를 임시로 다른 값으로 바꿔 실행해 2건 실패(포맷 명령 `is_enabled: false`, "built-in html structure" 대기 초과)를 확인하고 되돌렸습니다. 스마트 선택 `[A]`는 선택 확장 연결 전에 실행해 "html smart select" 대기 초과 실패를 확인한 뒤 연결했습니다. crate 검사 기대값은 원본 코드를 따라 손으로 계산했습니다(첫 실행부터 기대값 수정 없이 통과; 타입 비교 컴파일 오류 1회만 고침). oracle(bun으로 원본 실행)은 셸 규칙상 bun을 쓸 수 없어 만들지 못했습니다.

## 검증 명령과 결과

모든 cargo 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/editor`, manifest는 lane worktree 아래 절대 경로입니다.

| 명령 | 결과 |
| --- | --- |
| editor `cargo check --quiet --lib` | 성공, 경고 없음 |
| editor `cargo test --test html-services` | 첫 실행 컴파일 실패(검사 코드가 `PartialEq` 없는 `Edit`의 `Result` 비교) → `.err()` 비교로 고친 뒤 6 통과, 선택 확장 추가 뒤 7 통과 |
| app `cargo test --lib -- html_tests --test-threads=2`(언어 ID 임시 변경) | 첫 실행 컴파일 실패(`Arc<[FoldRegion]>`와 배열 비교) → `[..]`로 고친 뒤 2 실패(의도한 실패) |
| app `cargo test --lib -- html_tests --test-threads=2`(되돌린 뒤) | 2 통과 |
| app `cargo test --lib -- html_tests::actual_app의_html은_스마트선택 --test-threads=2`(연결 전) | 1 실패(의도한 실패, 대기 초과) |
| app `cargo test --lib -- html_tests selection_ranges --test-threads=2`(연결 뒤) | 17 통과(HTML `[A]` 3 + 기존 선택 확장 14) |
| editor·app `cargo fmt -- --check` | 이번 묶음의 새/수정 파일만 차이 → `cargo fmt` 적용(2회), `git status`로 다른 파일 변화 없음 확인, 재검사 통과 |
| editor `cargo check --quiet --all-targets` | 성공, 기존 `DocumentSymbol::deprecated` 경고만 |
| editor `cargo check --quiet --lib --target wasm32-unknown-unknown` | 성공, 경고 없음(선택 확장 추가 뒤 재확인) |
| editor `cargo test --test html-services --test css-services --test default-colors --test json-folding --test json-format --test folding` | 6개 대상 합 42 통과(출력 순서 5·2·21·7·3·4) |
| app `cargo check --quiet --all-targets` | 성공, 기존 경고만(vendor wry·`is_applying`·`zen.rs` mut·`fixture_with_emmet_peek`) |
| app `cargo test --lib -- application::json_tests editor_json editor_formatting editor_folding editor_symbols lsp_editor_formatting selection_ranges --test-threads=2` | 47 통과(JSON·CSS·HTML 실제 App, 포맷·접기·심볼·선택 확장 단위 검사 포함. 위 개별 통과와 합산하지 않음) |
| remote-web `cargo check --quiet --target wasm32-unknown-unknown --lib` | 성공, 기존 dead_code 경고만. remote-web 소스·Cargo 무변경 |

실행하지 않은 것: 필터 없는 앱 전체 테스트(메인 병합 때), UI crate 검사(UI 무변경), GUI 실기.

## 남은 범위(다음 작업자가 그대로 이어서)

1. HTML 완성·hover: `services/htmlCompletion.js`·`htmlHover.js`는 태그·속성·값 설명 자료(`languageFacts/data/webCustomData.js` 6220줄)를 쓰므로 사양대로 bun 스크립트로 Rust 표(`ED/web-data/html-*.rs`)를 생성해야 합니다. 이번 셸 규칙에서는 bun을 쓸 수 없어 하지 않았습니다. 완성 트리거 `. : < " = /`, 자료 없는 닫는 태그 완성(`</` → 열린 태그)도 같은 파일입니다.
2. HTML 하이라이트(`htmlHighlighting.js`, 시작/끝 태그 이름 쌍)·rename(`htmlRename.js`)·링크(`htmlLinks.js`, 사양 1.4 링크 UI와 공유): core는 `html_parser::find_node_at`·scanner로 바로 이식 가능하지만, 앱에 내장 하이라이트·rename·링크 공급 경로가 없어(JSON에도 없음) 공급 구조를 새로 만들어야 합니다.
3. `<style>` 안 CSS는 HTML 옵션의 `wrap_line_length` 120이 beautify-css에도 넘어가 원본은 긴 CSS 줄을 감쌉니다. E4의 `css_format`은 Monaco CSS 옵션(감싸기 0)만 이식해 이 감싸기가 없습니다. 120열을 넘는 `<style>` 줄에서만 차이가 납니다.
4. oracle 부채: `html-format`·`html-folding`·`html-selection` 원본 대조 fixture(bun). 위험: 손 계산 사례 밖의 beautify 분기(속성 감싸기 정렬 경계, 여러 단계 선택적 끝 태그 조합, `<!-- beautify ignore -->` 구간, `text/html` script 재귀).
5. HTML 전용 `[A]`의 readonly·Large 제외·untitled·peek·외부 HTML 공급자 공존 사례: 구현은 JSON·CSS와 같은 공통 경로(`is_format_eligible`의 readonly, `is_html`의 Normal 등급, 공급자 비어 있을 때만 내장 포맷)이지만 HTML 전용 실제 App 사례는 파일 3건뿐입니다.
6. E6(TS/JS): 결정 4(임베디드 JS 엔진 허락) 대기. editor-16·32·49·57은 E6까지 partial입니다.

## 바꾼 파일

- `native/taide-native-editor/src/html-scanner.rs`·`html-parser.rs`·`html-symbols.rs`·`html-folding.rs`·`html-selection.rs`·`html-format.rs`(신규), `src/lib.rs`(모듈 6개, 정렬 위치), `src/css-format.rs`(`beautify` 가시성 1줄), `tests/html-services.rs`(신규)
- `native/taide-native-app/src/editor-html.rs`(신규), `src/lib.rs`(모듈 1줄), `src/editor-css.rs`(`format_ranges` 추출), `src/editor-json.rs`(심볼 판정·build 분기), `src/editor-formatting.rs`(html 1줄씩), `src/editor-selection-ranges.rs`(`is_builtin`·`builtin_ranges`, 호출 2곳), `src/application-json-tests.rs`(하위 모듈 선언 2줄), `src/application-html-tests.rs`(신규)
- 문서: 이 파일, `2026-10-09-native-function-matrix.md`(editor-16·32·49·57 행)

## 리뷰 수정 (2026-11-03)

- editor-49 기능표 비고에 "남은 범위 3번" 차이를 한 줄 넣었습니다. `<style>` 안 CSS 줄이 120열을 넘으면 원본은 감싸지만 native는 감싸지 않습니다. 첫 작업 요약의 "원본처럼 감싸지 않음"은 원본과 같다는 뜻으로 읽힐 수 있어 바로잡습니다. 원본과 다른 동작이며, 이 문서 3번의 서술이 맞습니다.
- `APP/editor-selection-ranges.rs`: 이 lane이 바꾼 줄의 지역 변수 `native_json`이 이제 HTML에도 쓰여 `native_builtin`으로 고쳤습니다. 같은 지역 변수 3곳이고 동작은 같습니다(`editor-formatting.rs`의 `builtin` 개명과 맞춤). `JsonSlot`·`active_json`·`MAX_JSON_ACTIVE`는 이 lane 전부터 있던 이름이라 최소 변경 원칙에 따라 그대로 두었습니다. HTML 선택 확장도 이 JSON worker 동시 실행 상한을 함께 씁니다.
- 검증: app `cargo test --lib -- application::json_tests editor_selection_ranges --test-threads=2` 19 통과(JSON·CSS·HTML 앱 검사 포함), app `cargo check --quiet --all-targets` 성공(기존 경고만), app `cargo fmt -- --check` 통과.
- 나머지 리뷰 항목은 `2026-11-02-native-lane-editor-display-commands.md`의 "리뷰 수정" 절에 있습니다. 병합 충돌 관리 반박, 동결 미러 두 명령 실행, untracked 새 파일 목록이 그 절에 있습니다.
