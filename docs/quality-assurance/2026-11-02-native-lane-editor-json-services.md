# lane editor 묶음 2/4 (json-services) — editor-57 E3 JSON 완성 연결, hover core

> 작성일: 2026-11-02 · worktree `TAIDE-lanes/editor`(브랜치 `lane/editor`) · target `TAIDE-lanes/targets/editor`
> 기준: `docs/research/2026-10-28-lane-editor-preview-spec.md`(이하 사양) 2.8 E3, 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`

## 대상 ID와 결과

| ID | 결과 | 요약 |
| --- | --- | --- |
| editor-57 E3 | partial 유지 | `doComplete` core·앱 공급·triggerSuggest 재요청 완료, `doHover` core 완료(앱 소비 남음). 비고의 "언어 선택 UI"는 원본에 없어 대응표 비고에서 뺌(사양 1.3, `rg changeLanguage\|languageMode\|selectLanguage` 결과 내부 `setModelLanguage`뿐) |

## 시작 상태

- 작업 트리에 앞 시도의 미커밋 `ED/json-completion.rs`(lib.rs 미등록)와 `json-scanner.rs`(지연 토큰 반복자 `tokens`), `json-schema-document.rs`(`document_schema`: `configure` 뒤 `getSchemaForResource`·`getRegisteredSchemaIds`), `json-value-text.rs`(`quoted` crate 공개)가 있어 원본과 줄 단위로 대조한 뒤 이어서 썼습니다.

## 원본 근거

- `MEXT/vscode-json-languageservice/lib/esm/services/jsonCompletion.js` 전체(954줄), `jsonHover.js`(113줄), `utils/json.js`(`stringifyObject`).
- Monaco 연결: `MON/languages/features/json/jsonWorker.js`는 `ClientCapabilities.LATEST`(markdown 문서, labelDetails 지원)로 서비스를 만들고, `MON/languages/features/common/lspLanguageFeatures.js:104-164` `CompletionAdapter`는 label·insertText·sortText·filterText·documentation·detail·command·range·kind만 옮깁니다(labelDetails 버림, resolve 공급자 없음). TAIDE TS는 JSON `setDiagnosticsOptions`를 부르지 않아 schema 요청 서비스·설정 schema가 없습니다(`rg jsonDefaults src` 없음).
- Monaco `SuggestController`는 수락한 항목의 `command`가 `editor.action.triggerSuggest`이면 `trigger({ auto: true, retrigger: true })`로 목록을 다시 엽니다.

## 구현

- `ED/json-completion.rs`(`pub mod json_completion`): `complete(input, byte, Environment, Limits)`. 원본과 같은 순서로 주석 안·여는 중괄호 앞 제외, 덮어쓰기 범위(scalar 노드 전체 / 단어+앞 따옴표), 기존 속성 제외, schema 속성(`properties`·`propertyNames` enum/const, `deprecationMessage`·`doNotSuggest` 제외, `suggestSortText`, 값 스니펫 규칙과 `$1`로 끝날 때 triggerSuggest 명령), schema 없는 속성(비슷한 객체·최상위 `$schema`·입력 단어+incomplete), 값(enum/const/default/examples/defaultSnippets와 배열 깊이 5, `uniqueItems` 중복 제외, tuple `items` 위치, patternProperties·additionalProperties, boolean/null, `$schema` 값의 http/https 등록 id와 draft `#`), 채움(`{}`·`[]`). 라벨 60자 축약·개행 기호, 중복 라벨 문서 보충도 원본대로입니다.
    - 앞 시도 대비 수정: `#[allow(clippy::too_many_arguments)]` 대신 `Environment` 구조체, `byte > len` 경계 거절, 속성 schema가 배열일 때도 원본처럼 키만 제안(JS `typeof [] === 'object'`).
    - 원본 기본값의 "Default value"는 `labelDetails`로만 붙고 Monaco 어댑터가 버리므로 화면에 없습니다 → native도 붙이지 않음.
    - `doResolve`는 Monaco에 resolve 공급자가 없어 호출되지 않음 → 구현하지 않음(사양 문구의 resolve는 실제로 쓰이지 않음).
- `ED/json-hover.rs`(`pub mod json_hover`): `hover(input, byte, Environment, Limits) -> Option<Hover>`. 괄호 안 위치 거절(UTF-16 기준 `offset > start+1 && offset < end-1`), 키 위면 값 노드로, 범위는 원래 노드, title/markdownDescription/description의 `toMarkdown`(UTF-16 단위 비중첩 단일 개행 → 문단, markdown 기호 escape), enum 설명(뒤 schema가 덮어쓰는 원본 순서 그대로), `` `값`: 설명 ``(백틱 포함 시 `` `` ``). 원본이 내용이 빈 문자열 hover를 돌려줄 때 Monaco는 빈 markdown을 그리지 않으므로 `None`.
    - 원본 결함 비재현: enum 설명이 남은 채 다음 schema에서 값 위치를 못 찾으면 원본은 `toMarkdownCodeBlock(undefined)`로 예외가 나 hover 전체가 실패합니다 → native는 앞서 찾은 값만 씁니다. 문자열이 아닌 title/description(원본 `replace` 예외)도 없는 것으로 봅니다.
- 앱 공급(`APP/editor-completion-json.rs`, `editor_completion` 하위 모듈): `language_id == "json"`·Normal 등급·AppFile 아님에서 요청마다 감독 worker(`native-json-completion`)로 계산하고 `Candidates`(snippet, `TextEdit` 범위, command 유지)로 바꿔 전용 `ProviderIdentity`의 group(ordinal 최대)으로 넣습니다. LSP group과 같은 모델에서 병합되어 외부 공급자와 공존합니다(사양 2.8 공존 정책). schema 설정·contributions·pattern 엔진은 진단(`editor-json-validation.rs`)과 같은 값을 `pub(crate)`로 재사용합니다.
    - `editor-completion.rs`: `Entry.json`(대기 중 결과), `pending`에 포함. `editor-completion-supply.rs`: 공급 때 이전 JSON group 제거 후 spawn, `poll_supply`에서 같은 token·`describes`일 때만 group 반영. `editor-completion-cache.rs`: JSON 대기 중 모델 생성 보류, 입력 진행 시 대기 결과 폐기(다음 공급에서 재계산), JSON group이 incomplete면 오른쪽 입력에서 모델 재시작, incomplete 재질의 집합은 LSP 공급자로 한정.
    - 수락 뒤 재요청: `provider.rs` `accept`가 후보 `command`를 `Command::from_action`으로 보고 Trigger면 `retriggers`에 기록합니다. UI가 수락 직후 `close`로 대기 명령을 지우므로(`State::close`가 `commands` 제거) 명령 큐 대신 별도 집합에 두고 `take_commands`가 Trigger를 내며 `automatic_triggers`로 표시, `request`가 그 표시를 자동 요청(`automatic`)으로 씁니다(원본 `auto: true`: 결과가 비면 목록을 띄우지 않음). LSP 항목의 triggerSuggest에도 같은 경로가 적용됩니다(원본 SuggestController 동작). 다른 command id 실행은 이번 범위 밖입니다.
- 실제 앱 경로: `main.rs` → `NativeApplication` → `domains/editor.rs::dispatch_completion` → `editor_completion::State::supply` → `json::spawn` → `poll_supply`(`domains/editor.rs` frame) → 완성 위젯 → `provider.accept`.

## 검사

| 표기 | 검사 | 결과 |
| --- | --- | --- |
| `[C]` | `taide-native-editor --test json-services` 5건: schema 없는 `{}`·입력 단어(incomplete)·비슷한 배열 객체·`$schema` 값(draft-04/07 `#`, 시험 contribution), schema 속성 6개(문서 markdown/plain, `"$1"`/`$1`+명령/`${1:3}`/`[$1]`/`{$1}`, doNotSuggest 제외), enum 설명·boolean·uniqueItems·`[]` 채움, 주석 안/여는 괄호 앞/잘못된 경계, hover 제목·설명·enum 설명·CRLF 비중첩 문단·escape·괄호 안/schema 없음/값 없음 | 5 통과 |
| `[A]` | `application::json_tests::actual_app의_json_완성은_내장_meta_schema_속성을_공급하고_수락과_undo를_연결한다`: 실제 파일 `{"$schema": "…draft-07…", "tit"}`에서 Trigger → JSON group 후보 `title`(`"title": "$1"`, filter `"title"`, PROPERTY, 범위=키 문자열), `readOnly` `${1:false}`, 기존 `$schema` 제외 → Tab 수락 결과 `"title": ""`·커서 따옴표 안·디스크 불변·undo 원문, 종료 tasks 0 | 통과 |
| consumer | `editor_completion::consumer_tests::native_completion_수락은_trigger_suggest_명령이_있을때만_자동요청으로_목록을_다시연다`: 명령 없는 수락은 재요청 없음, triggerSuggest 명령 수락은 다음 frame에 `automatic` 요청 | 통과(첫 두 시도 실패: 수락 직후 UI `close`가 큐 명령을 지움 → 별도 집합으로 근본 수정) |

oracle(bun으로 원본 실행해 `.rs` 생성)은 이번 셸 규칙에서 bun을 실행할 수 없어 쓰지 못했습니다. 기대값은 위 원본 줄을 읽고 손으로 계산했습니다(테스트 부채: 아래 남은 범위).

## 검증 명령과 결과

모든 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/editor`, manifest는 lane worktree 아래 절대 경로입니다.

| 명령 | 결과 |
| --- | --- |
| editor `cargo check --quiet --lib`(모듈 등록 직후) | 성공, 경고 없음 |
| editor `cargo test --test json-services`(fmt 전) | 5 통과 |
| app `cargo test --lib -- trigger_suggest_명령이 json_완성은_내장_meta_schema --test-threads=2` | JSON `[A]` 통과, consumer 실패 1 → 수정 뒤 단독 재실행 통과 |
| editor `cargo check --quiet --all-targets` | 성공, 기존 `DocumentSymbol::deprecated` 경고만 |
| editor `cargo test --test json-services --test completion --test json-scanner --test json-symbols --test json-validation` | 24 통과(5·10·3·3·3) |
| editor `cargo test --lib` | 1 통과 |
| app `cargo test --lib -- editor_completion application::json_tests application::emmet_tests application::ai_completion_tests editor_json --test-threads=2` | 110 통과(새 검사 2건 포함, 위 개별 실행과 합산하지 않음) |
| app `cargo check --quiet --all-targets` | 성공, 기존 경고만(vendor wry·`is_applying`·`zen.rs` mut·`fixture_with_emmet_peek`) |
| remote-web `cargo check --quiet --target wasm32-unknown-unknown --lib` | 성공, 기존 dead_code 경고만. remote-web 소스·Cargo 무변경 |
| editor·app `cargo fmt -- --check` | 통과(이번 묶음 파일에 rustfmt 적용 뒤, 다른 파일 변경 없음을 `git status`로 확인) |

실행하지 않은 것: 필터 없는 앱 전체 테스트(메인 병합 때), UI crate 검사(이번 묶음은 UI 무변경), GUI 실기.

## 남은 범위(다음 작업자가 그대로 이어서)

- JSON hover 앱 소비: `APP/editor-documentation.rs`의 `Provider::available`·`version`·`request`가 `lsp.documentation_providers`만 봐서 LSP가 없으면 hover 요청 자체가 없습니다. 설계: `editor-completion-json.rs`처럼 JSON 전용 `ProviderIdentity`와 worker를 두고 `Entry`에 내장 대기 결과를 추가, `complete`를 LSP 완료∧내장 완료로 계산, `domains/editor.rs` frame에서 poll해 `HoverGroup`(ordinal은 LSP 뒤, 원본 등록 순서상 나중에 등록된 LSP가 앞)을 넣습니다. core `json_hover::hover`의 `contents`는 markdown 문자열 1개이고 `range`는 byte입니다.
- 외부 LSP 공존·readonly·Large 제외·닫힘 취소의 JSON 전용 `[A]`: 구현은 공통 경로(`begin`의 readonly 거절, `applies`의 Normal 등급, `Request::is_cancelled`)를 쓰지만 JSON 전용 실제 App 검사는 1건(파일)뿐입니다. untitled·peek도 미검사.
- oracle 부채: `json-completion-oracle.mjs`/`json-hover-oracle.mjs`(bun) 생성. 조건: bun 실행이 허용된 작업자. 위험: 손 계산 기대값 5건 밖의 분기(patternProperties 정규식, tuple items, defaultSnippets bodyText/배열 깊이, propertyNames enum 설명)는 원본 대조가 없습니다.
- JSON 색(`findDocumentColors`)은 E4 색 UI와 함께.
- 원본과 다른 작은 점: JSON group이 incomplete(schema 없음+따옴표 밖 단어)일 때 원본은 그 공급자만 다시 묻지만 native는 모델을 재시작해 LSP도 다시 묻습니다(보이는 결과는 같음, 요청 수만 많음).

## 바꾼 파일

- `native/taide-native-editor/src/json-completion.rs`(앞 시도 신규 + 수정), `src/json-hover.rs`(신규), `src/lib.rs`(모듈 두 줄, 정렬 위치), `src/json-scanner.rs`·`src/json-schema-document.rs`·`src/json-value-text.rs`(앞 시도 변경 유지, rustfmt), `tests/json-services.rs`(신규)
- `native/taide-native-app/src/editor-completion-json.rs`(신규), `src/editor-completion.rs`, `src/editor-completion-supply.rs`, `src/editor-completion-cache.rs`, `src/editor-completion-provider.rs`, `src/editor-json-validation.rs`(상수·함수 3개 `pub(crate)`), `src/application-json-tests.rs`, `src/editor-completion-consumer-tests.rs`
- 문서: 이 파일, `2026-10-09-native-function-matrix.md`(editor-57 행)

공용 파일(`application.rs`·`host.rs`·`domains/editor.rs`·locale)은 바꾸지 않았습니다. 새 앱 상태는 기존 `editor_completion::State` 안에만 두었고(`domains/editor.rs`의 영역 `State` 신설 불필요), 사용자 문구 추가가 없어 locale 키도 없습니다.
