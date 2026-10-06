# Native 배치 5 단계 1/3 — 구문 강조 엔진 게이트와 문법 자산 (2026-10-06)

상태: 게이트 4개 통과. 검증 계약 V1~V5 가 모두 exit 0 입니다. 미결 사항은 8절에 있습니다.

기준 시점은 HEAD `cffd0a09` 이고, 시작할 때 작업 트리 변경은 `docs/PROCESS.md` 하나였습니다(이 단계가 만든 변경이 아닙니다).

## 1. 게이트 판정

| 게이트 (설계 5.4) | 판정 | 근거 |
| --- | --- | --- |
| 1. 오프라인 빌드 | 통과 | `cargo generate-lockfile --offline` 후 `cargo check --locked --offline` exit 0. lockfile 에 `ferriki-textmate 0.12.0`, `ferroni 1.8.1` 고정, `cc` 없음 |
| 2. 적합성 | 통과 | 표본 32개(31개 언어 + markdown 지연 임베드 시나리오) × 테마 2개. 3068줄, span 19955개에서 불일치 0 |
| 3. 성능 기록 | 통과(기록) | 6절. 가장 큰 문법 cpp 의 첫 줄 30ms, 1만 줄 문서 cold 608ms |
| 4. 한도 동작 | 통과 | 20000 UTF-16 단위 이상 줄은 토큰화하지 않고 상태를 넘김. 줄당 시간 한도 초과 시 중간에서 멈추고 `is_stopped_early` 를 돌려줌 |

적합성은 세 층으로 비교했고 모두 불일치 0 입니다.

- span 층: 토큰 시작 위치(UTF-16)와 (색 인덱스, 글꼴 스타일 비트). 게이트 기준보다 엄격한 층입니다.
- 최종 층: `shikiToMonaco` 의 scope 역조회와 Monaco `TokenTheme` 을 거친 최종 전경색·글꼴 스타일·표준 토큰 종류. 설계 5.4 게이트 2번의 기준입니다.
- 종류 층: `TokenizedLine.kinds` 와 Monaco 표준 토큰 종류.

한계는 8절에 적었습니다. 표본은 합성한 11~87줄짜리(덧붙인 3줄 제외)이고 테마는 47개 중 2개입니다.

## 2. 항목별 변경 내용

### G1. 엔진 API 확인

코드 변경 없음. 결과는 4절에 있습니다.

### G2. 크레이트 골격 (`native/taide-native-syntax`)

| 파일 | 내용 |
| --- | --- |
| `Cargo.toml` | edition 2024, rust-version 1.95, `publish = false`, 자체 `[workspace]`. 의존성 `ferriki-textmate = "=0.12.0"`, `serde`(derive), `serde_json`, `taide-native-editor`(path) |
| `Cargo.lock` | 52개 패키지. `ferroni` 는 전이 의존성으로 1.8.1 고정 |
| `src/lib.rs` | 모듈 연결과 공개 항목 재공개 |
| `src/tokenizer.rs` | `LineState`, `TokenizedLine`, `GrammarTokenizer`, `SyntaxError`, 스타일 id 함수(`style_id`, `style_color_index`, `style_font_bits`), 글꼴 스타일 비트 상수 |
| `src/textmate-tokenizer.rs` | 엔진을 쓰는 유일한 파일. `TextmateTokenizer`(`new`, `try_tokenize_line`, `color_map`, `style_color`, `monaco_scope`, `token_kind`), `GrammarSet`, `LanguageGrammar`, `TokenizerLimits`, 비공개 `EngineState` |
| `src/style-scopes.rs` | `shikiToMonaco` 의 색·글꼴 스타일 → scope 역조회 표(`StyleScopes`)와 Monaco `toStandardTokenType`(`standard_token_kind`) 이식 |
| `src/theme-settings.rs` | 엔진에 넣는 테마 설정 입력형 `ThemeSetting`, `ThemeStyle` |
| `src/utf16-offsets.rs` | UTF-16 위치 → UTF-8 바이트 위치 변환(`Utf16ByteCursor`), `utf16_len` |
| `src/bundled-grammars.rs` | 문법 37개를 `include_str!` 로 포함. `bundled_language_ids`, `bundled_grammar_set` |
| `src/*-tests.rs` | 단위 테스트 13개 |

공개 형태는 설계 5.6 과 같습니다.

```rust
pub struct LineState(EngineState);
pub struct TokenizedLine { pub spans: Vec<u32>, pub kinds: Vec<Token>, pub end_state: LineState, pub is_stopped_early: bool }
pub trait GrammarTokenizer {
    fn tokenize_line(&mut self, language_id: &str, line: &str, previous: Option<&LineState>) -> Option<TokenizedLine>;
    fn is_same_state(&self, left: &LineState, right: &LineState) -> bool;
}
```

- 엔진 형식(`StateStack`, `Grammar`, `SyncRegistry`, `RawTheme`)은 `textmate-tokenizer.rs` 밖으로 나가지 않습니다.
- `spans` 는 (줄 안 UTF-8 바이트 시작, 스타일 id) 쌍을 이어 붙인 배열입니다. 스타일 id 는 `색 인덱스 × 16 + 글꼴 스타일 비트` 이고, 3c 의 `TokenStyleTable` 을 `색 표 길이 × 16` 칸짜리 표로 만들면 그대로 인덱스가 됩니다.
- `kinds` 는 종류가 바뀌는 위치만 담습니다. 첫 항목은 항상 바이트 0 이고 시작 위치는 오름차순이어서 `EditorStore::install_syntax` 의 검증(`native/taide-native-editor/src/store.rs:263-276`)을 그대로 통과하는 형태입니다.
- `try_tokenize_line` 은 실패 사유(`SyntaxError`)를 돌려주고, trait 의 `tokenize_line` 은 그 결과를 `Option` 으로 줄입니다.

### G3. 문법 추출

- 스크립트: `docs/utils/2026-10-06-extract-shiki-grammars.ts`
- 산출물: `native/taide-native-syntax/grammars/<id>.tmLanguage.json` 37개, `grammars/manifest.json`
- 고지: `THIRD_PARTY_LICENSES.md`, `native/taide-native-syntax/{NOTICE.md, LICENSE-SHIKI, LICENSE-MONACO-SNIPPET}`

동작은 다음과 같습니다.

1. `src/shared/lib/shiki/lang-map.ts` 의 loader 줄에서 TAIDE 언어 id → `@shikijs/langs/<id>` 대응을 읽고, `TAIDE_LANGUAGE_IDS` 31개와 개수·이름이 맞지 않으면 실패합니다.
2. 각 모듈 `node_modules/@shikijs/langs/dist/<id>.mjs` 의 정적 import 를 따라 합집합을 구합니다.
3. 모듈 안 `Object.freeze(JSON.parse("..."))` 의 문자열 literal 값을 그대로 파일에 씁니다. 파싱 후 다시 직렬화하지 않습니다. 쓴 파일을 다시 읽어 같은지 확인합니다.
4. TS loader(`loadTaideGrammar`)가 돌려주는 등록 객체와 추출한 문법이 이름을 빼고 같은지 31개 언어 전부에서 대조합니다.
5. GPL-3.0 문법(`ada`, `gnuplot`, `nginx`, `org`, `racket`)이 합집합에 있으면 실패합니다.

결과는 37개, 2,188,558바이트입니다. 표는 5절에 있습니다.

### G4. 기준 자료 생성

- 스크립트: `docs/utils/2026-10-06-generate-syntax-reference.ts`
- 표본: `native/taide-native-syntax/tests/fixtures/samples/*.sample` 31개와 `manifest.json`
- 산출물: `tests/fixtures/reference/{one-dark-pro,github-light}/{theme.json, <표본 id>.json}` (테마마다 33개 파일)

TS 가 실제로 쓰는 코드를 그대로 불러 씁니다.

| 단계 | 사용한 실제 코드 |
| --- | --- |
| 테마 조립 | `src/shared/lib/shiki/build-shiki-theme.ts` 의 `buildShikiTheme` |
| 문법 로딩 | `src/shared/lib/shiki/lang-map.ts` 의 `loadTaideGrammars`, `loadTaideGrammar`. 코어 3종(`json`, `jsonc`, `markdown`)으로 만든 뒤 요청 언어를 `loadLanguage` 로 추가 (`shiki-monaco.ts:144-153, 187-196` 과 같은 순서) |
| 엔진 | `createHighlighterCore` + `createJavaScriptRegexEngine()` (`shiki-monaco.ts:2, 148-152`) |
| 토큰 공급자 | `@shikijs/monaco` 의 `shikiToMonaco` 를 DOM 없는 가짜 `monaco` 객체에 연결해 `defineTheme` 인자와 `setTokensProvider` 가 받은 공급자를 그대로 받음 |
| 최종 스타일 | `monaco-editor/esm/vs/editor/common/languages/supports/tokenization.js` 의 `TokenTheme` |

스크립트가 직접 옮긴 부분은 DOM 에 묶여 import 할 수 없는 두 군데뿐입니다.

- `StandaloneTheme.tokenTheme` 의 규칙 조립(`standaloneThemeService.js:117-148`): `inherit: false` 이므로 `editor.foreground`·`editor.background` 로 만든 기본 규칙 뒤에 테마 규칙을 붙입니다.
- `TokenizationSupportAdapter._toBinaryTokens`(`standaloneLanguages.js:166-198`): 메타데이터가 같은 연속 토큰을 합칩니다.

테마 해석은 `crates/taide-theme/src/service.rs:831-936` 의 `resolve_theme(theme, None)` 과 같은 규칙(팔레트 참조 치환, `syntaxOverrides` 빈 배열)으로 번들 테마 JSON 에서 만듭니다. 두 테마 모두 `palette` 가 비어 있고 `extends` 가 없습니다.

`theme.json` 에는 엔진에 넣을 테마 설정(Shiki `normalizeTheme` 을 거친 `settings`), vscode-textmate 색 표, Monaco 색 표, 기본 최종 메타데이터, 그리고 (색 인덱스, 글꼴 스타일) 조합 전체에 대한 scope 역조회 결과가 들어 있습니다. 역조회 결과는 실제 `shikiToMonaco` 공급자에 모든 조합을 담은 합성 토큰을 넣어 얻었습니다(탐침용 하이라이터의 `json` 문법 객체에서 `tokenizeLine2` 만 바꿔 끼웁니다. 표본 토큰화에는 다른 하이라이터를 씁니다).

표본 문서는 저자가 쓴 줄 뒤에 세 줄을 덧붙여 만듭니다. 길이 19999 인 줄(토큰화됨), 길이 20000 인 줄(토큰화되지 않음), 꼬리 줄입니다. 긴 줄은 `manifest.json` 의 `prefix + "x" 반복 + suffix` 규칙으로 스크립트와 Rust 테스트가 각각 만들고, 줄별 UTF-16 길이를 기준 자료와 대조해 같은 문서인지 확인합니다.

표본 구성:

- 31개 언어 각각: 문자열(이스케이프·보간·여러 줄), 주석(줄·블록·중첩·문서), 정규식 literal(있는 언어), 숫자 literal, 중첩 구조, 한글·한자·emoji·결합 문자 줄.
- 임베드: html 의 `<style>`·`<script>`, erb 의 Ruby, heex(html 문법), elixir 의 `~H`, markdown 코드 펜스, ruby heredoc 12종(SQL, HTML, GraphQL, XML, HAML, JS, CSS, SH, LUA, YAML, CPP, C), cpp raw string(glsl, sql, regex).
- RTL 문자: rust 표본의 문자열 한 줄.
- markdown 은 시나리오가 둘입니다. `markdown` 은 코어 3종만 실린 상태(json 펜스만 강조), `markdown-with-languages` 는 typescript·rust·shellscript 가 추가로 실린 상태입니다.

### G5. 적합성 테스트

- `tests/engine-gate.rs`, `tests/engine-gate/{reference.rs, conformance.rs}`
- `표본_전체의_토큰_경계와_최종_스타일이_ts_기준_자료와_일치한다`: 1절의 세 층 비교.
- `요청한_언어_집합이_ts와_같은_문법_집합을_싣는다`: `bundled_grammar_set` 이 고른 문법의 scopeName 집합이 TS 가 실은 집합과 같은지.
- `엔진의_색_표와_스타일별_scope_역조회와_토큰_종류가_ts_기준과_같다`: 엔진 색 표, 그리고 `색 표 길이 × 16` 개 조합 전체의 scope 문자열과 토큰 종류.

### G6. 성능과 한도

- `tests/engine-gate/limits.rs`: 한도 동작 테스트 5개(일반 테스트).
- `tests/engine-gate/performance.rs`: 측정 전용 테스트 3개(`#[ignore]`, 시간 단언 없음).

### G7. 판정

1절과 같습니다.

## 3. 근거 경로

TS:

- `src/shared/lib/shiki/lang-map.ts:3-35`(언어 31개), `:56`(코어 3종), `:65-97`(문법 모듈 대응과 이름 바꾸기)
- `src/shared/lib/shiki/shiki-monaco.ts:91-101`(테마 적용과 공급자 재부착), `:144-153`(하이라이터 생성), `:187-196`(언어 추가)
- `src/shared/lib/shiki/build-shiki-theme.ts:109-125`

Shiki 4.4.3:

- `node_modules/@shikijs/monaco/dist/index.mjs:21-27`(`normalizeColor`), `:31-57`(`textmateThemeToMonacoTheme`), `:69-82`(`setTheme` 과 역조회 표), `:93`(한도 기본값), `:99-127`(`tokenize`), `:130-154`(글꼴 스타일 정규화와 키)
- `node_modules/@shikijs/primitive/dist/index.mjs:113-196`(`normalizeTheme`), `:273-296`(`loadLanguage` 와 문법 설정), `:364-372`(`getInjections`), `:413-424`(`setTheme`)
- `node_modules/@shikijs/engine-javascript/dist/engine-compile.mjs`
- `node_modules/@shikijs/vscode-textmate/dist/index.js:2438-2526`(`tokenizeLine2`, `_tokenize`), `:3026-3101`(`SyncRegistry`), `:3104-3206`(`Registry`)

Monaco 0.56.0:

- `node_modules/monaco-editor/esm/vs/editor/common/languages/supports/tokenization.js:20-61, 65-104, 134-165, 166-183, 221-272`
- `node_modules/monaco-editor/esm/vs/editor/standalone/browser/standaloneLanguages.js:166-198`
- `node_modules/monaco-editor/esm/vs/editor/standalone/browser/standaloneThemeService.js:117-148`

엔진(`ferriki-textmate-0.12.0/src`): `grammar.rs:27-35, 82-104, 265-285, 315-361`, `tokenize_string.rs:81-90`, `state_stack.rs:66-75, 288-310`, `encoded_token_attributes.rs:7-19`, `registry.rs:25-37, 47-58, 65-83, 95-117`, `theme.rs:17-52, 432-516, 665-719`, `line_output.rs:138, 175-231`, `rule_factory.rs:161-173, 399-441`, `parse_raw_grammar.rs:44-54`, `raw_grammar.rs:28-53`.

## 4. 엔진 API 확인 결과 (G1)

| 항목 | 확인한 사실 |
| --- | --- |
| 문법 로딩 | `parse_raw_grammar(content, file_path)` 는 경로가 `.json` 으로 끝날 때만 JSON 으로 읽고 그 외에는 plist 로 읽습니다. `RawGrammar` 는 모르는 키(`displayName`, `embeddedLangs` 등)를 무시합니다. `SyncRegistry::add_grammar(raw, injection_scope_names)` 로 등록합니다 |
| 문법 컴파일 | `SyncRegistry::grammar_for_scope_name(scope, GrammarConfiguration)` 이 `Rc<Grammar>` 를 돌려주고 scope 별로 캐시합니다. 규칙 트리는 이때 컴파일하고 정규식 scanner 는 처음 쓰일 때 컴파일합니다. `add_grammar`·`set_injections`·`set_theme` 은 컴파일된 문법 캐시를 비웁니다 |
| 문법 설정 | Shiki 와 같게 `initial_language_id = 1`, `balanced_bracket_selectors = ["*"]`, 그 외 기본값으로 호출합니다(`@shikijs/primitive` 277-282). 이 값은 메타데이터의 balanced-bracket 비트에 들어가 토큰 병합 경계에 영향을 줍니다 |
| 테마 | `SyncRegistry::new(Some(RawTheme), None)`. `RawTheme`·`RawThemeSetting`·`RawThemeStyle` 은 `#[non_exhaustive]` 라서 `Default` 와 `with_*` 로 만듭니다. scope 없는 설정이 기본 전경·배경이 됩니다. 색 유효성(`#` + 3·4·6·8자리 16진수)과 글꼴 스타일 분해는 vscode-textmate 와 같습니다 |
| 색 표 | `SyncRegistry::get_color_map()`. 인덱스 0 은 빈 문자열, 나머지는 대문자 `#RRGGBB`. TS 의 색 표와 순서까지 같았습니다 |
| `tokenize_line2` 입력 | `(&str 줄, Option<Arc<StateStack>> 이전 상태, u64 시간 한도 ms)`. 줄 끝 문자는 엔진이 `\n` 을 붙입니다 |
| `tokenize_line2` 출력 | `tokens: Vec<u32>` 가 (시작 위치, 메타데이터) 쌍. `rule_stack`, `stopped_early`, `fonts`(쓰지 않음) |
| 오프셋 단위 | UTF-16 code unit 입니다. `grammar.rs:90-99` 의 문서 주석에 있고, 한글·emoji 줄에서 TS 기준 위치와 일치했습니다. 래퍼가 UTF-8 바이트로 바꿉니다 |
| 메타데이터 비트 | 언어 id 0-7, 표준 토큰 종류 8-9, balanced bracket 10, 글꼴 스타일 11-14, 전경색 인덱스 15-23, 배경색 인덱스 24-31. vscode-textmate·Monaco 와 같은 배치입니다 |
| 시간 한도 | 0 은 무제한입니다. 줄 처리 루프의 매 반복에서 경과 시간이 한도보다 크면 그때까지의 토큰과 스택을 돌려주고 `stopped_early = true` 가 됩니다. vscode-textmate 와 같은 의미입니다 |
| 상태 비교 | `StateStack::equals`: 포인터가 같거나, 깊이·규칙 id·end 규칙 문자열이 스택 전체에서 같고 content name scope 목록이 같으면 같습니다. vscode-textmate `StateStackImpl.equals` 와 같습니다 |
| 초기 상태 | `StateStack::null()` 또는 `None`. 둘 다 첫 줄로 처리합니다 |
| 임베드 문법 | `include: "source.x"` 와 `include: "source.x#rule"` 은 레지스트리에 등록된 raw 문법을 scope 이름으로 찾습니다. 없으면 그 include 를 건너뜁니다. TS 처럼 "실린 문법 집합"만 맞추면 결과가 같습니다 |
| injection | 문법 자신의 `injections` 와, `add_grammar` 의 두 번째 인자로 준 scope 목록 중 `injectionSelector` 가 있는 문법을 합쳐 우선순위(L, 일반, R)로 정렬합니다. 번들 37개에는 Shiki 의 `injectTo` 를 쓰는 문법이 없어 래퍼는 빈 목록을 넘깁니다 |
| 스레드 | `SyncRegistry` 가 `Rc<Grammar>` 를 쥐므로 `TextmateTokenizer` 는 `Send` 가 아닙니다. 설계 5.6 대로 토큰화 스레드가 소유해야 합니다 |
| 빌드 | `ferriki-textmate` 는 build 스크립트가 없습니다. `ferroni` 의 `cc` 는 꺼져 있는 `ffi` feature 에서만 쓰여 lockfile 에 들어오지 않았습니다 |

엔진이 Shiki 와 다르게 동작하는 것으로 확인한 점은 하나입니다. 엔진은 vscode-textmate 9.3.2 를 옮긴 것이고 Shiki 는 포크(`@shikijs/vscode-textmate` 10.0.2)를 씁니다. 엔진에는 "줄에 RTL 문자가 있으면 메타데이터가 같은 연속 토큰을 합치지 않는다"(`line_output.rs:138`)와 글꼴 패밀리·크기 속성(`fonts`)이 있고 포크에는 없습니다. 래퍼가 스타일 id 가 같은 연속 span 을 합치므로 결과에 차이가 없고, RTL 줄이 든 rust 표본에서 일치를 확인했습니다.

## 5. 추출한 문법 37개

`@shikijs/langs` 4.4.3. 패키지 라이선스는 MIT 입니다. 문법별 상류 라이선스는 `THIRD_PARTY_LICENSES.md` 의 "Bundled TextMate Grammars" 에 있습니다.

| 문법 id | scopeName | 바이트 | 끌어오는 모듈 | TAIDE 언어 id |
| --- | --- | ---: | --- | --- |
| c | source.c | 69561 | | c |
| cpp | source.cpp | 476145 | cpp-macro regexp glsl | cpp |
| cpp-macro | source.cpp.embedded.macro | 270396 | regexp glsl | (임베드) |
| css | source.css | 48071 | | css |
| dart | source.dart | 8207 | | dart |
| elixir | source.elixir | 15583 | html | elixir |
| erb | text.html.erb | 1974 | html ruby | erb |
| glsl | source.glsl | 3490 | c | (임베드) |
| go | source.go | 44514 | | go |
| graphql | source.graphql | 17390 | javascript typescript jsx tsx | (임베드) |
| haml | text.haml | 7812 | javascript css | (임베드) |
| haskell | source.haskell | 39465 | | haskell |
| hcl | source.hcl | 9714 | | hcl |
| html | text.html.basic | 56408 | javascript css | html, heex |
| java | source.java | 26339 | | java |
| javascript | source.js | 159352 | | javascript |
| json | source.json | 2705 | | json |
| jsonc | source.json.comments | 2990 | | jsonc |
| jsx | source.js.jsx | 162317 | | javascriptreact |
| kotlin | source.kotlin | 8409 | | kotlin |
| lua | source.lua | 14841 | c | lua |
| markdown | text.html.markdown | 56874 | (지연 임베드 57개) | markdown |
| python | source.python | 68156 | | python |
| regexp | source.regexp.python | 7736 | | (임베드) |
| ruby | source.ruby | 44537 | html haml xml sql graphql css cpp c javascript shellscript lua yaml | ruby |
| rust | source.rust | 14706 | | rust |
| scala | source.scala | 26958 | | scala |
| scss | source.css.scss | 26449 | css | scss |
| shellscript | source.shell | 39955 | | shellscript |
| sql | source.sql | 22888 | | (임베드) |
| swift | source.swift | 84244 | | swift |
| toml | source.toml | 6126 | | toml |
| tsx | source.tsx | 160061 | | typescriptreact |
| typescript | source.ts | 163900 | | typescript |
| xml | text.xml | 5235 | java | (임베드) |
| yaml | source.yaml | 9997 | | yaml |
| zig | source.zig | 5053 | | zig |

- 기존 고지 표(30종)에 없던 임베드 모듈은 7개입니다: `cpp-macro`, `regexp`, `glsl`, `sql`, `graphql`, `haml`, `xml`.
- GPL-3.0 문법 5종은 합집합에 없습니다(스크립트가 검사).
- `injectTo` 를 쓰는 문법은 없습니다.
- 문법 JSON 에는 상류 출처·라이선스 정보가 없습니다(최상위 키는 `displayName`, `name`, `scopeName`, `fileTypes`, `embeddedLangs` 류뿐). 임베드 7종의 출처·라이선스는 네트워크 없이 확인할 방법이 없었습니다(8절 1번).

`THIRD_PARTY_LICENSES.md` 변경: 서두 문장, native 바이너리가 같은 파일을 포함한다는 문단, "Embedded grammar modules — 7" 표와 검증 대기 주석, 추출 스크립트의 GPL 검사 문단, "Native syntax highlighting engine" 절(두 크레이트의 저작권 고지와 BSD-2-Clause 전문, 옮긴 Shiki·Monaco 소스 고지), "Rust crates" 요약 한 줄.

## 6. 측정값

측정 환경은 macOS arm64 입니다. Rust 는 release 프로필, TS 스택은 bun 1.4.2(JavaScriptCore)에서 측정했습니다. 시간 단언은 없습니다.

### 6.1 Rust 엔진 (V3)

문법 적재와 첫 토큰화:

| 표본 | 적재(JSON 파싱·등록) | 첫 줄 | 표본 전체 첫 통과 | 두 번째 통과 |
| --- | ---: | ---: | ---: | ---: |
| cpp (63줄) | 3.7ms | 30.1ms | 239ms (최장 줄 29.6ms) | 3.0ms |
| typescript (63줄) | 1.1ms | 8.5ms | 50.6ms | 0.9ms |
| ruby (87줄) | 6.8ms | 40.3ms | 53.1ms (최장 줄 16.5ms) | 0.8ms |
| erb (29줄) | 6.1ms | 41.4ms | 21.9ms | 0.4ms |
| markdown (56줄) | 0.6ms | 1.7ms | 1.7ms | 0.1ms |
| json (11줄) | 0.6ms | 0.3ms | 0.1ms | 0.06ms |

1만 줄 문서(표본 줄을 반복해 만든 문서, 기본 한도 적용):

| 표본 | 바이트 | cold | warm | 가장 느린 줄(cold) | 시간 한도에 걸린 줄 |
| --- | ---: | ---: | ---: | ---: | ---: |
| cpp | 355,918 | 608ms | 315ms | 45.7ms | 0 |
| typescript | 266,014 | 159ms | 96ms | 9.1ms | 0 |
| rust | 243,673 | 41ms | 38ms | 0.6ms | 0 |
| python | 266,590 | 79ms | 68ms | 3.9ms | 0 |
| markdown | 132,750 | 23ms | 20ms | 1.6ms | 0 |

한도 근처 긴 줄(19999 UTF-16 단위, 대부분 문자열 한 개): 표본 32개 전부 0.04~0.62ms 였고 기본 한도 500ms 에 걸린 줄은 없습니다. 가장 느린 것은 cpp 0.61ms, typescript 계열 0.55~0.62ms, markdown 0.55ms, go 0.42ms 입니다. 길이 20000 인 줄은 약 7µs(UTF-16 길이를 세는 시간)에 건너뜁니다.

debug 프로필에서도 적합성 테스트 전체(3068줄, 긴 줄 64개 포함)가 18초에 끝납니다.

### 6.2 TS 스택 (기준 자료 생성 중 측정)

기준 자료 스크립트가 시간 한도 없이 토큰화하면서 500ms 를 넘긴 줄을 기록했습니다. TS 앱에서는 이 줄들이 500ms 한도에 걸려 줄 중간에서 토큰화가 멈춥니다.

| 표본 | 19999자 줄 | 그 밖에 500ms 를 넘긴 줄 |
| --- | ---: | --- |
| typescript, typescriptreact, javascript, javascriptreact | 14.6~15.9초 | 첫 줄 520~577ms (실행에 따라 미발생) |
| cpp | 12.1~12.6초 | |
| swift | 2.6초 | |
| rust | 1.44~1.51초 | |
| yaml | 1.37~1.42초 | |
| c | 0.93~0.95초 | |
| hcl, zig | 0.69~0.71초 | |
| dart | 0.57~0.63초 | |
| html | | `<script>` 시작 줄 543~552ms (실행에 따라 미발생) |
| heex | | `<script>` 줄 690~699ms (실행에 따라 미발생) |
| erb | | `<script>` 시작 줄 544~548ms (실행에 따라 미발생) |
| ruby | | JS heredoc 시작 줄 544~558ms |
| markdown-with-languages | | ts 펜스 시작 줄 519~524ms (실행에 따라 미발생) |

19999자 줄이 아닌 줄들은 모두 그 문서에서 JavaScript·TypeScript 계열 문법이 처음 쓰이는 줄입니다.

같은 줄에서 Rust 엔진은 1ms 를 넘지 않습니다.

### 6.3 한도 동작 (G6, 일반 테스트)

| 테스트 | 확인한 동작 | TS 근거 |
| --- | --- | --- |
| `한도_길이_이상인_줄은_토큰화하지_않고_앞_줄의_상태를_그대로_넘긴다` | 길이 20000 줄은 span `[0, 스타일 없음]`, 종류 `Other`, 끝 상태는 받은 상태 그대로. 다음 줄이 블록 주석 안에서 이어짐 | `@shikijs/monaco` `index.mjs:100-106` |
| `한도는_바이트_수가_아니라_utf16_길이로_잰다` | 한글 19999자(59997바이트)는 토큰화, emoji 10000자(20000단위)는 건너뜀 | 같은 곳(`line.length`) |
| `첫_줄이_한도를_넘으면_다음_줄은_문서_처음과_같은_상태에서_토큰화된다` | 초기 상태가 그대로 넘어감 | `index.mjs:96-98, 100-106` |
| `줄당_시간_한도를_넘기면_줄_중간에서_멈추고_멈춘_사실을_알린다` | 한도 1ms 에서 `is_stopped_early = true`, span 은 줄 앞부분만. 돌려준 상태로 다음 줄 토큰화 가능 | `index.mjs:107-108`, `@shikijs/vscode-textmate` `tokenizeLine2` |
| `싣지_않은_언어는_토큰화하지_않는다` | trait 의 `tokenize_line` 이 `None` | |

## 7. 설계 문서와 달라진 점

| 설계 서술 | 실제 | 처리 |
| --- | --- | --- |
| 5.4 위험: 토큰 오프셋 단위 미확인 | UTF-16 | 래퍼가 UTF-8 바이트로 변환 |
| 5.4 게이트 2: TS 스택 그대로 기준 자료 생성 | TS 기본값(줄당 500ms)으로는 19999자 줄과 일부 첫 줄이 한도에 걸려 결과가 실행 시간에 따라 달라짐 | 기준 자료는 `shikiToMonaco` 의 `tokenizeTimeLimit: 0` 으로 만들고, 500ms 를 넘긴 줄은 6.2 에 기록. 한도 동작은 별도 테스트(6.3) |
| 5.1: 합집합 37개 모듈, 2433KB | 문법 JSON 텍스트 합은 2,188,558바이트. 2433KB 는 `.mjs` 파일 크기 기준 | 표 5절 |
| 5.6: `spans` 는 (바이트 시작, 스타일 id) 쌍 | 스타일 id 가 같은 연속 토큰을 래퍼가 합쳐서 돌려줌. vscode-textmate 는 표준 토큰 종류만 달라도 토큰을 나누므로 엔진 원본보다 span 수가 적음 | 화면 결과 동일. Shiki 포크와 엔진의 RTL 병합 차이도 이 병합으로 사라짐 |
| 5.6: `kinds` 는 6.2 의 `TokenStyleTable` 로 정함 | 토큰 종류는 trie 없이 scope 역조회 표와 `toStandardTokenType` 만으로 정해지므로 이 단계에서 구현하고 기준 자료와 대조함 | `src/style-scopes.rs`. 최종 전경색·글꼴 스타일을 정하는 Monaco 토큰 테마 trie 는 비목표라 구현하지 않았고, 테스트는 기준 자료의 표를 씀 |
| 5.6: 문법 로딩은 요청 언어 집합 유지 | 이 단계는 `bundled_grammar_set(요청 언어)` 로 한 번에 구성하는 것만 제공 | 집합 변경 시 재구성은 3c |
| 6.1: `#` 형식이 아닌 전경색은 키가 맞지 않아 기본 전경색이 됨 | 확인하지 못함. 소스로는 자리표시 색 `#0000000N` 이 테마 규칙과 토큰 양쪽에 같은 값으로 남아 키가 맞고, Monaco `ColorMap` 이 앞 6자리만 써서 `#000000` 이 될 가능성이 있음(`@shikijs/primitive` 155-175, `@shikijs/monaco` 21-27, `tokenization.js` 105-129) | 번들 테마에 해당 색이 없어 기준 자료로 확인하지 못함. 3c 에서 합성 테마로 확인 필요 |

수정 범위 밖 파일은 수정하지 않았습니다.

## 8. 미결 사항과 남은 위험

1. 임베드 모듈 7종의 상류 출처와 라이선스를 검증하지 못했습니다. 네트워크 접근이 금지돼 있고 저장소·`node_modules` 에 `tm-grammars` 표가 없습니다. `THIRD_PARTY_LICENSES.md` 에는 출처·라이선스를 적되 "verification pending" 주석을 달았고 `glsl` 은 "not determined" 로 뒀습니다. `glsl` 의 상류(`polym0rph/GLSL.tmbundle`)에 LICENSE 파일이 없다면 기존 회색 지대 4종과 같은 성격이 되어 사용자 승인이 필요합니다. 7종 모두 GPL-3.0 5종에는 해당하지 않습니다.
2. TS 의 테마 전환 동작이 의도와 다릅니다. 테마 이름이 항상 `taide` 라서 `highlighter.setTheme('taide')` 가 두 번째부터 vscode-textmate 레지스트리에 새 테마를 넣지 않습니다(`@shikijs/primitive` `index.mjs:413-419` 의 `_lastTheme !== name`). 기준 자료 스크립트로 확인한 결과, 한 하이라이터에서 one-dark-pro → github-light 로 바꾸면 색 표가 이전 테마 그대로이고 rust 표본 63줄 중 43줄의 최종 토큰이 새 하이라이터 결과와 달랐습니다. 하이라이터가 다시 만들어지는 시점(플러그인 재로드, 앱 재시작)까지 유지됩니다. native 가 이 동작을 재현할지, 새 테마로 다시 토큰화할지 3c 전에 결정이 필요합니다.
3. TS 의 시간 한도 결과는 재현할 수 없습니다. TS 에서는 TS·JS 계열 파일의 첫 줄과 수천 자 이상의 긴 줄이 500ms 한도에 걸려 부분만 강조되는데(6.2), 이는 기기 속도에 따라 달라집니다. Rust 엔진은 같은 줄을 1ms 안에 끝내므로 native 에서는 그 줄들이 끝까지 강조됩니다.
4. 표본 범위의 한계입니다. 언어당 11~87줄의 합성 표본이고 테마는 47개 중 2개입니다. 실제 대형 파일, 플러그인·VSIX 문법, `injectTo`, plist 문법은 다루지 않았습니다.
5. 엔진의 `RawGrammar.injections` 는 `BTreeMap` 이라 키 순서가 사전순이고, vscode-textmate 는 JSON 에 적힌 순서입니다. 우선순위가 같은 injection 이 같은 위치에서 겹치면 결과가 달라질 수 있습니다. 표본에서는 차이가 없었습니다.
6. 저장소 루트의 `bun run format:check`(`prettier --check .`)와 `bun run lint`(`eslint .`)는 `native/` 를 제외하지 않습니다. 표본은 확장자를 `.sample` 로 둬서 대상이 아니지만, `grammars/*.json`(수정 금지)과 `tests/fixtures/**/*.json` 은 prettier 대상입니다. 이 검사가 강제된다면 `.prettierignore` 에 `native/taide-native-syntax/grammars` 와 `native/taide-native-syntax/tests/fixtures` 를 추가해야 합니다. 범위 밖 파일이고 이 검사를 실행할 수 없어 수정하지 않았습니다. 이후 리뷰 차단 항목으로 올라와 12절에서 두 줄을 추가했습니다.
7. 문법 37개를 압축 없이 `include_str!` 하므로 바이너리가 약 2.19MB 늘어납니다. 압축 여부는 설계 5.5 대로 앱에 연결하는 3c 에서 측정 후 정합니다. 앱 바이너리 증가분은 이 단계에서 측정하지 않았습니다.
8. `ferriki-textmate` 는 beta 표기의 신생 크레이트입니다. 버전은 `=0.12.0` 으로 고정했습니다.
9. wasm32 빌드는 확인하지 않았습니다. `taide-remote-web` 은 이 크레이트에 의존하지 않습니다.

실기 확인이 필요한 항목은 이 단계에 없습니다(UI 연결 없음).

## 9. 테스트 부채

- [ ] 번들 테마 47개 전체의 scope 역조회 표 대조. 재현: 기준 자료 스크립트의 `REFERENCE_THEME_IDS` 에 테마를 추가해 `theme.json` 만 생성. 생략 이유: 이 단계 요구는 다크 1·라이트 1 이상. 필요한 시점: 3c 의 `TokenStyleTable`.
- [ ] `#` 형식이 아닌 색, 전경색 없는 규칙, 배경만 있는 규칙을 가진 합성 테마의 기준 자료. 필요한 시점: 3c.
- [ ] 플러그인 문법과 `injectTo`. 필요한 시점: 3e.
- [ ] 우선순위가 같은 injection 이 겹치는 문법. 재현 조건: 한 문법의 `injections` 에 같은 우선순위 선택자 둘이 같은 위치에서 매칭. 필요한 시점: 플러그인 문법에서 강조가 TS 와 다르다는 보고가 있을 때.
- [ ] 실제 대형 소스 파일(수천 줄)의 TS 대조. 필요한 시점: 3d 실기 확인.

## 10. 실행한 명령과 결과

경로는 저장소 루트 기준으로 줄였습니다. cargo 명령에는 모두 `--manifest-path native/taide-native-syntax/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target` 을 붙였습니다(`cargo fmt` 와 `cargo generate-lockfile` 제외).

| 명령 | 결과 |
| --- | --- |
| `git status --short` (시작) | ` M docs/PROCESS.md` 뿐 |
| `bun docs/utils/2026-10-06-extract-shiki-grammars.ts` | exit 0. 문법 37개, 2,188,558바이트. 스크립트 정리 뒤 한 번 더 실행해 같은 결과 |
| `cargo generate-lockfile --manifest-path native/taide-native-syntax/Cargo.toml --offline` | exit 0. 52개 패키지 |
| `bun docs/utils/2026-10-06-generate-syntax-reference.ts` (1회차) | exit 1. rust 표본 60번째 줄이 500ms 한도에 걸림(11절) |
| 같은 명령 (수정 뒤 3회: 최초, 표본 보강 뒤, 스크립트 정리 뒤) | exit 0. 테마 2개 × 표본 32개 |
| V1 `cargo check` | exit 0 |
| `cargo test --lib` | 13 passed |
| `cargo clippy --all-targets` | exit 0. 처음 경고 3건, 수정 뒤 경고 0 |
| `cargo fmt` 후 V4 `cargo fmt -- --check` | exit 0 |
| V2 `cargo test` | exit 0. lib 13 passed, engine-gate 8 passed·3 ignored(18.06초), doc-test 0 |
| V3 `cargo test --release -- --ignored --nocapture --test-threads 1` | exit 0. 3 passed(4.10초). 값은 6.1 |
| `cargo test --release --test engine-gate` | 8 passed, 3 ignored(1.99초) |
| `cargo test --release --test engine-gate 표본_전체 -- --nocapture` | 1 passed. 합계 3068줄, span 19955, 불일치 0 |
| V5 `git status --short` | `M THIRD_PARTY_LICENSES.md`, `M docs/PROCESS.md`(기존), `?? docs/utils/2026-10-06-extract-shiki-grammars.ts`, `?? docs/utils/2026-10-06-generate-syntax-reference.ts`, `?? native/taide-native-syntax/`. `native/taide-native-app`·`taide-native-ui`·`taide-native-editor`·`taide-remote-web`·`taide-native-terminal`·`taide-native-retained`·`crates`·`src` 는 변경 없음 |

V2 는 기준 자료를 마지막으로 다시 만든 뒤에 실행한 결과입니다. V3 는 그 직전에 실행했고, 이후 Rust 소스와 표본은 바뀌지 않았습니다.

## 11. 실패했다가 고친 내역

1. 기준 자료 스크립트 첫 실행이 실패했습니다. 원인은 TS 스택이 rust 표본의 19999자 줄에서 500ms 한도를 넘겨 `stoppedEarly` 가 된 것입니다. 한도에 걸린 결과는 실행 시간에 따라 달라져 기준이 될 수 없으므로, 기준 자료는 한도 없이 만들고 500ms 를 넘긴 줄을 기록하도록 바꿨습니다(7절). 표본이나 기준 값을 엔진에 맞춰 고친 것은 아닙니다.
2. clippy `chunks_exact_to_as_chunks` 경고 3건(`textmate-tokenizer.rs` 1건, `conformance.rs` 2건)을 `as_chunks` 로 바꿔 없앴습니다.

엔진 출력과 기준 자료가 어긋나서 고친 것은 없습니다. 첫 실행부터 불일치 0 이었고, ruby heredoc 임베드 10종과 cpp raw string 3종을 표본에 추가한 뒤에도 0 이었습니다.

## 12. 리뷰 후속

배치 5 구현 단계가 모두 끝난 뒤의 직렬 수정 단계에서 작성했습니다. 차단 항목은 1건입니다.

### 12.1 항목 1: 새 크레이트의 JSON 자산이 prettier 검사 대상에 들어옴 — 수정(전제 일부는 반증)

판정은 둘로 나뉩니다.

- 옳은 부분(수정함): 새 자산은 prettier 대상이고 prettier 형식이 아니며, 문법 파일은 형식을 맞춰 고칠 수 없습니다.
- 성립하지 않는 부분(반증): "지금 통과하는 검사를 배치 5 가 깨뜨린다"는 전제입니다. HEAD 에 이미 들어 있는 `native/` 파일이 prettier 형식이 아니어서, 이 브랜치의 `bun run format:check` 는 배치 5 이전부터 실패할 것으로 판단합니다. 아래 근거는 파일을 읽어 확인한 정적 근거이고 prettier 를 실행해 확인한 것은 아닙니다.

#### 지적이 옳다는 근거

| 확인한 사실 | 위치 |
| --- | --- |
| `format:check` 는 `prettier --check .`, `format` 은 `prettier --write .` | `package.json:14-15` |
| CI 의 frontend job 이 `bun run format:check` 를 실행. 트리거는 `pull_request` 와 dev·main push | `.github/workflows/ci.yml:5-8, 40-41` |
| 수정 전 `.prettierignore` 는 10줄이고 `native/` 를 제외하지 않음 | `.prettierignore:1-10` |
| prettier 3.9.6 CLI 의 ignore 파일 기본값은 작업 디렉터리의 `.gitignore`·`.prettierignore` 뿐. 디렉터리 확장은 `dot: true` | `node_modules/prettier/internal/legacy-cli.mjs:952-962, 1730` |
| 설정은 `tabWidth: 4`, `printWidth: 150` | `prettier.config.js:2-3` |
| 문법 파일은 줄바꿈이 없는 한 줄. `wc -l` 0, `rust` 14,706바이트, `hcl` 9,714바이트 | `native/taide-native-syntax/grammars/*.tmLanguage.json` |
| 기준 자료는 쉼표 뒤 공백 없는 배열(`["json","jsonc","markdown","rust"]`)을 쓰고 150자를 넘는 줄이 23개 | `tests/fixtures/reference/one-dark-pro/rust.json:5-7` |
| 표본 manifest 는 150자를 넘는 줄이 21개 | `tests/fixtures/samples/manifest.json` |

제외 말고 다른 방법이 없는 이유는 두 가지입니다. 문법 파일은 추출한 문자열을 그대로 저장해야 하고(`hcl` 은 MPL-2.0), prettier 는 하위 디렉터리의 ignore 파일을 읽지 않습니다. 그리고 CI 와 별개로, 제외하지 않은 상태에서 누군가 `bun run format` 을 실행하면 문법 파일 37개가 실제로 다시 쓰입니다.

#### 수정

`.prettierignore` 끝에 두 줄을 추가했습니다. 기존 항목(`src-tauri/target` 등)과 같은 표기입니다.

```
native/taide-native-syntax/grammars
native/taide-native-syntax/tests/fixtures
```

`.prettierignore` 는 이 단계의 수정 범위 밖 파일입니다. 범위 안에서는 해결할 수 없어(위 이유) 두 줄만 추가했습니다. 그 밖의 범위 밖 파일은 수정하지 않았습니다.

실패하는 테스트로 재현하지는 않았습니다. 이 항목을 재현하는 검사는 `prettier --check` 인데 이 단계의 허용 명령이 아닙니다. Rust 테스트가 저장소 루트의 `.prettierignore` 를 읽게 하는 것은 이 저장소에 없는 방식이라 만들지 않았습니다.

#### 전제가 성립하지 않는다는 근거

아래 파일은 모두 커밋 `40057316` 에서 추가돼 HEAD(`cffd0a09`)에 들어 있고, `.prettierignore`·`.gitignore` 어느 쪽에도 걸리지 않습니다.

| 파일 | 상태 |
| --- | --- |
| `native/taide-native-app/tests/fixtures/keybinding-search.json` | 한 줄, 146,642바이트 |
| `native/taide-native-app/tests/fixtures/keybinding-catalog.json` | 한 줄, 494,251바이트 |
| `native/taide-native-app/vendor/zip-retained/.cargo_vcs_info.json` | 2칸 들여쓰기(`tabWidth: 4` 와 다름) |
| `native/taide-native-app/vendor/wry-preview/renovate.json` | 2칸 들여쓰기 |

이 밖에도 `native/` 아래에는 상류에서 가져온 vendor 문서와 예제(`vendor/**/README.md`, `CHANGELOG.md`, `*.html`, `script.js`)가 prettier 대상으로 남아 있습니다. 이 파일들은 형식을 하나씩 확인하지 않았습니다.

따라서 이번 두 줄을 추가한 뒤에도 `bun run format:check` 는 위 기존 파일 때문에 실패할 것으로 예상합니다. 리뷰어가 제안한 "커밋 전 `bun run format:check` 1회 실행"이 실패하더라도, 출력에 `native/taide-native-syntax/` 경로와 `THIRD_PARTY_LICENSES.md` 가 없으면 배치 5 몫은 해소된 것입니다.

기존 파일의 처리(`native` 전체 제외, vendor·fixture 만 제외, 형식 맞추기 중 선택)는 배치 5 범위 밖이고 결정이 필요해 손대지 않았습니다.

### 12.2 `THIRD_PARTY_LICENSES.md` 와 `NOTICE.md`

두 문서는 prettier 대상으로 남습니다. prettier 를 실행하지 못해 정적으로만 점검했고, 고칠 곳을 찾지 못해 수정하지 않았습니다.

| 점검 | 결과 |
| --- | --- |
| 새 표(`THIRD_PARTY_LICENSES.md:315-323`)의 열 정렬 | 모든 행의 구분자 위치가 같고, 각 열 너비가 가장 긴 셀과 같음. 같은 파일의 기존 표(261-292, 654-660)와 같은 형식 |
| 목록 기호, 강조, 구분선, 코드 블록 | `-`, `_..._`·`**...**`, `---`, 언어 없는 fence. 같은 파일의 기존 절(35-225, 337-366)과 같은 표기 |
| `*`·`+` 목록, 탭 들여쓰기, `#` 뒤 공백 누락, 줄 끝 공백 | 두 파일 모두 0건(`rg`) |
| 숫자로 시작하는 이어지는 줄(`4.4.3 (`, `0.56.0 (`) | 숫자 뒤가 공백이 아니라 순서 목록으로 해석되지 않음 |
| 150자를 넘는 줄(`NOTICE.md`) | 0건 |

### 12.3 실행한 명령과 결과

| 명령 | 결과 |
| --- | --- |
| `git status --short` (시작) | 배치 5 앞 단계들의 변경만 있음. `.prettierignore` 없음 |
| `grep -n -E "format\|prettier" package.json .github/workflows/ci.yml` | `package.json:14-15, 25, 92`, `ci.yml:41` |
| `wc -l -c` (문법 2개, 문법 manifest, 기존 테마 JSON) | `rust` 0줄 14,706바이트, `hcl` 0줄 9,714바이트, `manifest.json` 761줄, `one-dark-pro.json` 2125줄 |
| `rg -c "^.{151,}$"` (기준 자료·manifest·`NOTICE.md`) | `samples/manifest.json` 21, `reference/one-dark-pro/rust.json` 23, 나머지 0 |
| `wc -l -c` (기존 native JSON 9개) | 위 표의 값 |
| `git log --oneline` (기존 native JSON·vendor 파일) | `40057316` |
| `sed -n 940,975p node_modules/prettier/internal/legacy-cli.mjs` | `ignorePath` 기본값 `.gitignore`, `.prettierignore` |
| `rg -n "[ \t]+$"`, `rg -n -e '^[*+] ' ...` (두 문서) | 0건 |
| `git diff -- .prettierignore` (수정 뒤) | 두 줄 추가만 있음 |
| V5 `git status --short` (수정 뒤) | 시작 시점과 비교해 ` M .prettierignore` 한 줄만 늘어남. `native/` 아래 Cargo.toml·Cargo.lock·소스는 이 단계에서 바뀌지 않음 |

실행하지 않은 검사는 다음과 같습니다.

- `bun run format:check`: 이 단계의 허용 명령이 아니어서 실행하지 않았습니다. 12.1 과 12.2 의 prettier 관련 판단은 모두 정적 근거입니다.
- V1~V4: 이 단계에서 Rust 소스, Cargo 파일, 문법, 기준 자료를 바꾸지 않아 다시 실행하지 않았습니다. 게이트 판정(불일치 0)은 10절의 결과 그대로입니다.

### 12.4 남은 일

- [ ] 메인이 `bun run format:check` 를 1회 실행해 출력에 `native/taide-native-syntax/` 경로, `THIRD_PARTY_LICENSES.md` 가 없는지 확인. 있으면 그 파일만 고칩니다.
- [ ] 기존 `native/` 파일의 prettier 불일치 처리 방향 결정(12.1). 필요한 시점: 이 브랜치를 PR 로 올리거나 dev·main 에 push 하기 전.
