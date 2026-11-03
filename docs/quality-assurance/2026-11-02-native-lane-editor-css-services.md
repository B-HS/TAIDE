# lane editor 묶음 3/4 (css-services) — editor-57 E4 CSS/SCSS 접기·포맷과 기본 색 공급자 core

> 작성일: 2026-11-02 · worktree `TAIDE-lanes/editor`(브랜치 `lane/editor`) · target `TAIDE-lanes/targets/editor`
> 기준: `docs/research/2026-10-28-lane-editor-preview-spec.md`(이하 사양) 2.4·2.5·2.7·2.8 E4, 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(결정 4: CSS/SCSS 순수 Rust 이식, 새 의존성 없음)

## 대상 ID와 결과

| ID | 결과 | 요약 |
| --- | --- | --- |
| editor-57 E4 | partial 유지 | CSS/SCSS scanner·접기·포맷 이식과 앱 공급, Monaco 기본 색 공급자 계산·색 형식 core 완료. CSS 파서 이후 서비스(lint·완성·hover·탐색·선택 확장·CSS/JSON 색)와 색 장식·선택기 UI·앱 연결은 남음 |
| editor-49 | partial 유지 | CSS/SCSS 문서·선택·저장 포맷 실제 App 확인. HTML(E5)·TS(E6) 남음 |
| editor-32 | partial 유지 | CSS/SCSS 붙여넣기 포맷 실제 App 확인(원본 CSS는 onType 공급자 없음). HTML·TS 남음 |
| editor-16 | partial 유지 | CSS/SCSS 내장 접기 공급자 연결(Monaco 고정 줄 모델의 2순위 재료). 1순위인 CSS 문서 심볼은 파서 뒤 |

## 시작 상태

- 작업 트리에 앞 묶음(display-commands·json-services)의 미커밋 변경이 있어 그대로 두었습니다. 이번 범위의 기존 변경은 없었습니다.

## 원본 근거

- scanner: `MEXT/vscode-css-languageservice/lib/esm/parser/cssScanner.js` 전체(599줄), `scssScanner.js`(변수·`#{`·`==`/`!=`/`<=`/`>=`·`...`·`//` 주석).
- 접기: `services/cssFolding.js`(중괄호·`#{`, `/* #region */`·SCSS `// #region`, 여러 줄 주석, 정렬·교차 제거). Monaco는 접기 공급자에 `rangeLimit` 없는 빈 context를 넘깁니다(`MON/editor/contrib/folding/browser/syntaxRangeProvider.js:9, 49`) → 결과 수 제한 없음, 이후 공통 정리는 기존 `SyntaxFolds::new`.
- 포맷: `services/cssFormatter.js`(범위 줄 확장·규칙 안 `{\n` 감싸기·시작 들여쓰기), `beautify/beautify-css.js` 1059~1566줄(Beautifier)과 Output/InputScanner/Directives/Options. Monaco 옵션 `MON/languages/features/css/register.js:60-67`(selector 줄바꿈·규칙 사이 빈 줄·collapse·`maxPreserveNewLines` undefined → 32786), 어댑터 `MON/languages/features/common/lspLanguageFeatures.js:538-543`(tabSize·insertSpaces만 전달 → 끝 줄바꿈 없음), 워커 `css/cssWorker.js:158-165`.
- 여러 범위: `MON/editor/contrib/format/browser/format.js:100-170`(겹치거나 닿는 선택 병합, 결과 편집이 교차하면 두 범위를 합쳐 재계산).
- 색: `MON/editor/common/languages/defaultDocumentColorsComputer.js`(줄 단위 정규식 — 워커 `findMatches`는 줄마다 `matchAll`: `textModelSync.impl.js:135-149`), `MON/editor/contrib/colorPicker/browser/defaultDocumentColorProvider.js`(표현 rgb·hsl·hex), `color.js:13-60`(다른 공급자가 있으면 기본 공급자 제외), `MON/base/common/color.js`(RGBA/HSLA 정규화, `formatRGB`·`formatHSL`·`formatHexA(compact)`·`parseHex`).

## 구현

### ED (portable core, wasm32 확인)

- `css-scanner.rs`(`pub mod css_scanner`): `Scanner::new(source, Dialect::{Css, Scss})`, `scan`·`scan_unquoted_string`·`try_scan_unicode`, 원본 token 종류(SCSS 7종 포함)와 escape 해석 text. byte 기준이며 원본의 UTF-16 비교는 모두 ASCII라 같은 결과입니다(비ASCII는 원본처럼 식별자 문자). `Dialect::from_language_id`는 TAIDE 언어 ID `css`·`scss`만(사양 2.8: less 없음).
- `css-folding.rs`: 원본 순서 그대로, 정규식 `#region\b\s*(.*?)\s*\*\/`의 줄 경계 규칙(표지 뒤 공백은 줄바꿈 포함 허용, 내용은 한 줄, 뒤 공백 후 `*/`)을 손으로 구현. 깊이·범위·token 상한.
- `css-format.rs`: Output(줄·들여쓰기 -1/0 규칙·`ensure_empty_line_above`·`trim`·`get_code`), InputScanner(읽기·`peekUntilAfter`·`lookBack`), ignore 지시문, Beautifier 분기 전체(collapse 경로만: Monaco 설정이 collapse 고정), cssFormatter 범위 처리. 줄 감싸기(wrap)는 `wrap_line_length` 0이라 원본에서도 동작하지 않아 옮기지 않았습니다. 출력 byte 상한.
- `color.rs`: `DocumentColor`, `Rgba`/`Hsla`(clamp·`|0`·3자리 반올림), 변환, `format_rgb`·`format_hsl`·`format_hex`·`parse_hex`, `default_presentations`.
- `default-colors.rs`: 줄 단위로 `\b(rgb|rgba|hsl|hsla)(…)`·줄 시작/따옴표·공백 뒤 `#hex{3,4,6,8}\b`를 찾고 매개변수 검증 정규식(0~255 무선행0, hsl 색상 0~360+소수, 백분율, alpha 형식 — rgba는 `1.0` 거절·hsla는 허용, `/` 구분)을 손으로 구현. 일치했지만 검증 실패면 원본처럼 그 구간을 건너뜁니다.

### APP

- `editor-css.rs`(새 모듈, `lib.rs` 편집기 모듈 목록에 1줄): `dialect`(css/scss·Normal·AppFile 아님), `folds`, `is_format_eligible`(+readonly 아님), `format`(문서/범위, 범위는 원본처럼 정렬·병합 후 결과 교차 시 합쳐 재계산, 작업 byte 예산, onType 거절).
- 접기: 기존 내장 구조 상태(`editor-json.rs`의 `State`, 감독 worker·generation·닫힘/취소 그대로)가 css/scss 문서도 받아 `build`에서 CSS 접기로 분기합니다. `Models.symbols`를 `Option`으로 바꾸고 `symbol_documents()`(JSON만)를 추가해, CSS 문서가 심볼 "대기 중"으로 남지 않게 했습니다(`domains/editor.rs::reconcile_builtin_json`). 모듈 이름은 최소 변경 원칙으로 유지했습니다.
- 포맷: `editor-formatting.rs`의 내장 경로를 JSON 전용에서 내장 공용으로 일반화(`is_native_eligible`·`native_build`, `native_json`→`native`, worker 이름 `native-builtin-formatting`). 외부 공급자 우선·내장 fallback 순서는 그대로입니다. 호출처: `domains/editor.rs` 저장 경로 3곳·요청 2곳, `application.rs` 명령 활성 조건 1줄(공용 파일, 편집기 명령 문장의 함수 이름만 교체).
- 새 앱 상태 없음(기존 editor 상태 재사용) → `domains::editor::State` 신설 불필요. 사용자 문구 추가 없음 → locale 키 없음.
- 실제 앱 경로: `main.rs` → `NativeApplication` → `background_tick` → `reconcile_builtin_json` → `editor_json::State::tick` → `editor_css::folds` → `editor_folding.install_builtin` / 팔레트·붙여넣기·저장 → `editor_formatting::State::begin*`/`begin_native_save` → `tick_native(_saves)` → `editor_css::format` → `apply_edits`.

### 원본 결함 비재현(근거)

- beautify-css `in`/객체 조회의 prototype 키: `@constructor ` 등이 중첩 at-rule로, `10constructor`가 함수 token 종류로 잡힘 → native는 표의 키만.
- `eatString`이 끝의 `\`에서 `null`을 문자열로 붙임(`result += this._input.next()`) → native는 붙이지 않음.
- scanner `_escape`가 입력 끝의 `\`에서 위치를 길이+1로 넘김 → native는 끝에서 멈춤(token 끝이 1 짧음).
- `String.fromCharCode`의 lone surrogate → U+FFFD.
- Monaco 선택 병합이 포함된 범위의 끝을 줄이는 경우(`fromPositions(start, range.end)`) → native는 큰 끝을 유지.

## 검사

| 표기 | 검사 | 결과 |
| --- | --- | --- |
| `[C]` | `taide-native-editor --test css-services` 5건: CSS token 종류·escape·문자열/BadString·단위(켈빈 기호 `K` 소문자화 포함)·CSS의 `//`, unicode range, SCSS token 13종, CSS 접기(중괄호 끝 줄 −1·region·여러 줄 주석·줄바꿈 낀 region 표지·닫히지 않은 region), SCSS `//` region과 교차 범위 제거·`#{}`, 깊이/개수 상한, 포맷(선택자 줄바꿈·`url( x )` 원문·규칙 사이 빈 줄·`@media` 중첩, SCSS 변수·빈 줄 보존·주석 위 빈 줄 삽입, 탭, 기준 들여쓰기·빈 규칙, ignore 지시문), 범위 포맷(줄 공백 확장·규칙 안 감싸기·tabSize별 시작 들여쓰기·`@media` 안 이중 들여쓰기), 경계/출력 상한 | 5 통과 |
| `[C]` | `taide-native-editor --test default-colors` 2건: hex 3/4/6/8·줄 시작/따옴표/공백 경계·단어 경계, rgb/rgba(`/`)·hsl/hsla, 거절(256·선행0·361·`1.0` rgba·빈 값·대문자·`xrgb`), CRLF 줄, `360.5` 허용, 개수 상한, 표현 rgb/hsl/hex·`toFixed(2)` 반올림(0.125→0.13)·hex alpha·gray HSL·clamp | 2 통과 |
| `[A]` | `application::json_tests::css_tests::actual_app의_css는_외부서버없이_접기와_문서_선택_붙여넣기_저장_포맷을_공급한다`: 실제 `.css` 파일(외부 rust-analyzer mock만 연결 → CSS 공급자 없음) → 내장 접기 (0,2)·region(4,6), CSS 심볼 미설치, 팔레트 formatDocument/formatSelection 결과=core 결과·undo 원문, format-on-paste 결과·undo 두 번, format-on-save 디스크·문서=core 결과·undo | 통과 |
| `[A]` | `…css_tests::actual_app의_scss는_한줄주석_region과_중첩규칙_접기를_공급하고_문서포맷을_적용한다`: `.scss` 파일 `// #region` (0,4)·중첩 규칙 (1,2), 문서 포맷 | 통과 |

실패 우선 확인: 두 `[A]`는 연결 전에 실행해 "built-in css folds" 대기 시간 초과로 2건 실패를 확인한 뒤 연결했습니다. crate 검사는 구현 뒤 작성했습니다(원본을 읽고 손으로 계산한 기대값). oracle(bun으로 원본 실행)은 이번 셸 규칙에서 bun을 쓸 수 없어 만들지 못했습니다.

## 검증 명령과 결과

모든 cargo 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/editor`, manifest는 lane worktree 아래 절대 경로입니다.

| 명령 | 결과 |
| --- | --- |
| editor `cargo check --quiet --lib` | 성공, 경고 없음 |
| editor `cargo test --test css-services` | 첫 실행 컴파일 실패(검사 코드가 `PartialEq` 없는 `Edit`의 `Result`를 비교) → `.err()` 비교로 고친 뒤 5 통과 |
| app `cargo test --lib -- css_tests --test-threads=2`(연결 전) | 첫 실행 컴파일 실패(`save_formatting_options`가 `&mut` 필요) → 수정 뒤 2 실패(의도한 실패, 접기 대기 초과) |
| app `cargo check --quiet --all-targets` | 첫 실행 실패(`editor-formatting-tests.rs`의 `active_json`·`MAX_JSON_ACTIVE` 참조 2곳 누락) → 이름 반영 뒤 성공, 기존 경고만(vendor wry·`is_applying`·`zen.rs` mut·`fixture_with_emmet_peek`) |
| app `cargo test --lib -- css_tests --test-threads=2`(연결 뒤) | 2 통과 |
| editor `cargo test --test default-colors --test css-services` | 2·5 통과 |
| editor·app `cargo fmt -- --check` | 이번 묶음의 새 파일만 차이 → `cargo fmt` 적용, `git status`로 다른 파일 변화 없음 확인, 재검사 통과 |
| editor `cargo check --quiet --all-targets` | 성공, 기존 `DocumentSymbol::deprecated` 경고만 |
| editor `cargo check --quiet --lib --target wasm32-unknown-unknown` | 성공, 경고 없음 |
| editor `cargo test --test css-services --test default-colors --test json-folding --test json-format --test folding` | 5개 대상 합 35 통과(출력 순서 5·2·21·3·4) |
| app `cargo test --lib -- application::json_tests editor_json editor_formatting editor_folding editor_symbols lsp_editor_formatting --test-threads=2` | 30 통과(위 CSS `[A]` 2건 포함, 따로 합산하지 않음) |
| remote-web `cargo check --quiet --target wasm32-unknown-unknown --lib` | 성공, 기존 dead_code 경고만. remote-web 소스·Cargo 무변경 |

실행하지 않은 것: 필터 없는 앱 전체 테스트(메인 병합 때), UI crate 검사(UI 무변경), GUI 실기.

절차 이탈 1건: 연결 전 실패 확인 실행 한 번에 출력 정리용 파이프(`2>&1 | grep | tail`)를 썼습니다. 이후 명령은 규칙대로 단일 명령만 썼습니다.

## 남은 범위(다음 작업자가 그대로 이어서)

1. CSS 파서(`cssParser.js` 1794줄·`scssParser.js` 809줄·`cssNodes.js`)와 그 위 서비스: 문서 심볼(editor-16 1순위 고정 줄, `editor_json::build`의 CSS 분기에서 `Models.symbols: Some`으로 공급하면 기존 경로가 그대로 씀), 하이라이트·정의·참조·rename(`cssNavigation.js`·`scssNavigation.js`), 선택 확장(`cssSelectionRange.js`), CSS 색(`findDocumentColors`·`getColorPresentations`: rgb/hsl/hwb/hex 등), lint(`lint.js`·`lintRules.js`, `css/register.js:37-59` 기본 심각도), hover(`cssHover.js`·`selectorPrinting.js`), 완성(`cssCompletion.js`·`scssCompletion.js`, 트리거 `/ - :`). 완성·hover 자료(`data/webCustomData.js`)는 사양대로 bun 스크립트 생성이 필요해 bun 실행이 허용된 작업자가 해야 합니다.
2. 색 장식·색 선택기 UI와 앱 연결: core(`color.rs`·`default-colors.rs`)만 완료. 남은 것: (a) 앱 문서별 색 계산 상태(Normal 등급, 공급자 규칙 — CSS/SCSS/JSON 문서는 내장 색 공급자가 있으므로 기본 공급자 제외, 그 외 모든 언어는 기본 공급자), (b) ED 표시 맵의 주입 항목(색 견본, `colorDecoratorsLimit` 500개)과 UI 행 배치·그리기(`UI/editor-color-decorators.rs` 신설, 기존 inlay hint 주입 경로 `editor_surface.rs`의 `inlay_hints`·`DisplayMap::set_inlay_hints` 참고), (c) 견본 클릭/hover(`colorDecoratorsActivatedOn` 기본 clickAndHover) 선택기(표현 순환·원래 색 복원·SV/색상/불투명도, `UI/theme-color-picker.rs` 재사용)와 표현 편집 적용, (d) editor-34 `showOrFocusStandaloneColorPicker` 실행, (e) JSON 색(schema `format: color`). 결정 2 A의 새 행 번호(editor-88~)는 E1이 Unicode 강조·링크 감지에 배정하기로 한 뒤 아직 행이 없어, 번호 충돌을 피하려 이번에 색 장식 행을 만들지 않았습니다.
3. oracle 부채: `css-format`·`css-folding`·`default-colors` 원본 대조 fixture(bun). 위험: 손 계산 사례 밖의 beautify 분기(`@` less 혼합, `insideScssMap`, `grid-template-areas` 줄 유지, `url(` 안 따옴표)와 정규식 엣지.
4. CSS 전용 `[A]`의 readonly·Large 제외·untitled·peek·외부 CSS 공급자 공존 사례: 구현은 JSON과 같은 공통 경로(`is_format_eligible`의 readonly, `dialect`의 Normal 등급, 공급자 비어 있을 때만 내장)이지만 CSS 전용 실제 App 사례는 파일 2건뿐입니다.

## 바꾼 파일

- `native/taide-native-editor/src/css-scanner.rs`·`css-folding.rs`·`css-format.rs`·`color.rs`·`default-colors.rs`(신규), `src/lib.rs`(모듈 5개, 정렬 위치), `tests/css-services.rs`·`tests/default-colors.rs`(신규)
- `native/taide-native-app/src/editor-css.rs`(신규), `src/lib.rs`(모듈 1줄), `src/editor-formatting.rs`, `src/editor-formatting-tests.rs`(이름 반영 5줄), `src/editor-json.rs`, `src/editor-json-tests.rs`(`Option` 반영), `src/domains/editor.rs`, `src/application.rs`(명령 활성 조건 1줄), `src/application-json-tests.rs`(하위 모듈 선언 2줄), `src/application-css-tests.rs`(신규)
- 문서: 이 파일, `2026-10-09-native-function-matrix.md`(editor-16·32·49·57 행)
