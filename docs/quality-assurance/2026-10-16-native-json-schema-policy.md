# JSON schema 정책과 매칭 계약

## 대상 파일

- Core `json-schema-evaluation.rs`, `json-schema-structure.rs`, `json-schema-policy.rs`(신규), `json-validation.rs`, `lib.rs`
- Core `tests/fixtures/json-schema-matching-oracle.mjs`·`json-schema-policy-oracle.mjs`와 생성한 `.rs`
- Syntax `tests/schema-matching.rs`·`tests/schema-policy.rs`
- PROCESS와 기능 대응표

## 리포트

설치된 `vscode-json-languageservice`의 `SchemaCollector`/`getMatchingSchemas`, `asSchema`, `JSONValidation.doValidation`, `JSONSchemaService.resolveSchemaContent` 정책을 기존 평가기·해결 결과·구문 진단에 연결했습니다.

- 매칭 수집기: `Selection { focus_byte, exclude_node }`를 받는 `matching_schemas`를 추가했습니다. 원본처럼 포함되지 않은 노드는 평가하지 않습니다. 조합 분기(`allOf`/`anyOf`/`oneOf`/`not`/`if`/`then`/`else`)의 하위 수집기는 focus를 풀고 exclude만 유지합니다. `contains`/`propertyNames`는 수집하지 않습니다. 루트가 거짓 스키마이면 빈 목록입니다.
- boolean 임시 표현: `asSchema`가 만드는 `{}`/`{"not":{}}`를 `SchemaView::Empty`/`Reject`로 구분합니다. `Reject`는 원본 순서대로 반전된 빈 스키마 다음에 자기 자신을 기록합니다. 원본이 boolean을 변환하지 않고 넘기는 루트 `true`와 `unevaluatedItems: true`는 `Source`로 남깁니다.
- 루트 거짓 스키마의 `validate`는 원본 `if (this.root && schema)` 계약대로 진단 없이 끝납니다.
- 문서 진단 정책(`json_schema_policy::diagnostics`)은 원본 순서를 따릅니다.
    1. 해결 오류가 있으면 첫 오류만 768로 기록합니다.
    2. 오류가 없고 schemaValidation이 꺼져 있지 않으면 meta-schema 미지원 경고(769)와 의미 진단을 기록합니다.
    3. 그다음 trailing comma와 기타 구문 진단을 기록합니다.
    4. 마지막으로 주석 진단을 기록합니다.

  스키마 문제의 위치는 첫 속성이 `$schema`일 때 그 값(값이 없으면 속성), 아닐 때 루트 첫 글자입니다. schemaRequest/schemaValidation의 `ignore`·기본 Warning·문서 `schemaDraft` 우선을 반영합니다. 스키마의 `allowComments`/`allowTrailingCommas`/구형 `allowsTrailingCommas`는 `allOf`를 원본 깊이 우선 순서로 찾으며, 명시 스택·방문 집합·작업 상한을 둡니다. 모든 진단은 시작 위치와 메시지 기준으로 중복을 제거합니다.
- 구문 진단 수집기를 공용 `Collector`/`collect_syntax`로 분리했습니다. `syntax_diagnostics`의 공개 API와 메시지 예산 계산은 바꾸지 않았습니다.

## 검증 기록

1. `bun native/taide-native-editor/tests/fixtures/json-schema-matching-oracle.mjs`: 1428사례를 생성했습니다(문서6 × 스키마14 × 노드별 focus/exclude 위치·조합). 원본 결과에는 boolean 임시 스키마와 루트/unevaluated의 raw boolean이 포함됩니다.
2. `bun native/taide-native-editor/tests/fixtures/json-schema-policy-oracle.mjs`: 실제 `getLanguageService().configure()`/`doValidation()` 경로에서 889사례를 생성했습니다. 이 가운데 768은 90행, 769는 36행입니다.
    - 첫 실행에서는 `$ref` 사례가 1건 불일치했습니다. 원본 resolver가 등록된 스키마 객체의 `$ref`를 제자리에서 지워 이후 기록 문자열이 `{}`가 된 fixture 오류였습니다. 매 서비스에 새 사본을 넘기도록 고쳤습니다.
    - 등록 스키마 없이 `$schema` 속성으로 원격 로드를 선택하는 21사례는 선택·로드 경로의 몫이므로 제외했습니다(이전 11건 불일치의 원인).
3. `cargo test --manifest-path native/taide-native-syntax/Cargo.toml --target-dir experiments/native-shell-spike/target --locked --offline --test schema-matching --test schema-policy`: 4통과, exit 0입니다.
    - 매칭 1428사례와 범위 밖 exclude 거절, 매칭 상한, 2048단 문서의 focus 경로를 확인했습니다.
    - 정책 889사례와 4096단 allOf의 허용 탐색·작업 상한, 순환 allOf, 진단 상한을 확인했습니다.
    - 첫 실행의 상한 검사 1실패는 `items: true`를 원본처럼 수집하지 않는다는 점을 놓친 검사 가정이었습니다. `items: {}`로 고쳤습니다.
4. 회귀 확인
    - Core lib·`json-validation`·`json-value`·`json-schema-scalar`는 1/3/3/5통과입니다.
    - Syntax `schema-evaluation`·`schema-string`·`schema-service`·`schema-pattern`은 5/3/3/2통과입니다.
5. 다음 검사는 exit 0입니다.
    - `cargo check --manifest-path native/taide-native-syntax/Cargo.toml --target-dir experiments/native-shell-spike/target --target wasm32-unknown-unknown --locked --offline --lib`
    - 소유 Rust 6파일 rustfmt
    - oracle 2파일 Prettier

## 남은 범위

- 실제 App이 JSON 문서에 해결된 스키마를 선택·로드하고 이 정책 진단을 공급하는 작업은 부모 `b3e11`에 남습니다. 조회·완성의 matching 소비와 regex backtracking 성능 게이트도 남습니다.
- 새 의존성·manifest·lock 변경은 없습니다. 동결 remote-web도 바꾸지 않았습니다.
