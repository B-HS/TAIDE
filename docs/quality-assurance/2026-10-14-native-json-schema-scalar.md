# JSON schema 공통·scalar 검증

## 대상 파일

- `native/taide-native-editor/src/json-schema-scalar.rs`, `json-value-text.rs`, `json-value.rs`, `json-symbols.rs`, `lib.rs`
- `native/taide-native-editor/tests/json-schema-scalar.rs`, `tests/fixtures/json-schema-scalar-oracle.mjs`, 생성한 `.rs`
- `native/taide-native-editor/json-language-license.txt`, `docs/PROCESS.md`, 기능 대응표

## 리포트

설치된 Monaco의 `vscode-json-languageservice/lib/esm/parser/jsonParser.js`에서 `JSONDocument.validate`와 공통·숫자·문자열 길이 검증을 대조했습니다. 이 단위는 `b3e11a`이며 전체 schema 검증기나 실제 앱 소비를 완료한 것이 아닙니다.

타입, enum, const, 폐기 경고, multipleOf, 배타·포함 최소/최대, 문자열 길이를 기존 AST·value arena에 연결합니다. enum/const의 일치 여부와 schema 값 ID를 보존해 후속 조합 검증이 재사용할 수 있게 합니다. 정수 판정은 AST의 원본 lexical `is_integer`를 사용합니다. 사용자 오류 메시지는 공통 오류에서만 적용하며 숫자/길이 오류의 원본 메시지를 덮어쓰지 않습니다.

진단은 byte·논리 UTF16 범위와 논리 UTF16 메시지, 선택적 코드, 심각도를 보존합니다. 폐기 경고는 기본 심각도와 무관하게 Warning이며 부모 property 전체를 가리킵니다. 같은 진단의 source를 임의로 추가하지 않습니다. 타입→enum→const→폐기→숫자/길이 순서를 확인했습니다. 조합 검증이 들어갈 원본의 타입과 enum 사이 단계는 아직 구현하지 않았습니다.

`json-value-text`는 기존 arena를 명시 스택으로 직렬화합니다. JSON 객체 키 순서, 유한 숫자의 기존 출력, 비유한 값의 null, 제어 문자와 단독 surrogate escape를 확인합니다. 키 문자열은 스택에 참조로 보관해 모든 키를 미리 escape·복제하지 않습니다. 순환은 거절하고 반복 공유 참조는 직렬화합니다. 깊이·출력·작업 예산을 적용하며 비교와 직렬화는 검증 단위의 남은 작업 예산을 공유합니다.

## 검증

1. `bun native/taide-native-editor/tests/fixtures/json-schema-scalar-oracle.mjs`
    - exit 0, 실제 원본 scalar 533사례·JSON.stringify 46사례를 생성했습니다.
    - 원본 `stringLength`는 high surrogate 다음 단위를 무조건 건너뜁니다. 단독 high surrogate 뒤의 정상 문자도 누락되는 8사례를 raw와 교정 결과로 나누어 보존했습니다. 교정은 실제 parser의 길이 호출 두 곳만 `Array.from(...).length`로 바꾸며 나머지 검증기·메시지·범위는 원본을 실행합니다.
2. `cargo check --manifest-path native/taide-native-editor/Cargo.toml --target-dir experiments/native-shell-spike/target --locked --offline --lib`
    - session 71457, 23.28초, exit 0입니다.
3. `cargo test --manifest-path native/taide-native-editor/Cargo.toml --target-dir experiments/native-shell-spike/target --locked --offline --lib --test json-schema-scalar --test json-value -- --nocapture`
    - 최초 session 9174는 검사 코드의 `json_parser::parse`에 collect_comments 인자가 빠진 세 컴파일 오류로 exit 101입니다. 성공으로 세지 않습니다.
    - 인자를 수정한 session 28103은 compile 15.51초, unit 1·scalar 5·기존 value 3의 서로 다른 9통과/0실패, exit 0입니다. 533/46 oracle 사례는 각각 해당 검사 내부의 비교 수이며 별도 테스트 수로 합산하지 않습니다.
    - enum/const 메타데이터, 논리 메시지, type/enum/const/폐기·숫자·문자열 순서와 property 범위, 자료/메시지/진단/누적 비교 작업 예산, 잘못된 필드 형식 거절, 8192단 깊이·출력 경계·공유/순환 참조를 확인했습니다.
    - 프로세스 출력 대기 중 기존 Cargo 핸들을 유지했습니다. 읽기 전용 ps는 실행 중인 PID와 CPU 0을 보여주었으나 sample 호출 시 대상이 이미 종료되어 호출 스택은 얻지 못했습니다. 시작 지연의 원인은 미확정이며 검사를 재시작하지 않았습니다.
4. `cargo check --manifest-path native/taide-native-editor/Cargo.toml --target-dir experiments/native-shell-spike/target --target wasm32-unknown-unknown --locked --offline --lib`
    - session 52727, `/private/tmp/taide-json-schema-scalar-wasm.log`, 31.53초, exit 0입니다.
5. 소유 Rust 5파일의 `rustfmt --edition 2024 --check`, 소유 diff의 `git diff --check`, 신규 oracle JS·이 문서의 기존 Prettier가 exit 0입니다.
    - 입력·환경이 같은 성공 검사는 반복하지 않았으며 App 전체 실행으로 확대하지 않았습니다.

## 제한과 남은 작업

- pattern/format, 배열/객체, allOf/anyOf/oneOf/not/조건, 평가 속성, draft/메타 schema, matching schema 선택과 실제 App 공급은 부모 `b3e11/b3e`에 남습니다. 이 API의 이름과 범위는 scalar 단계로 한정합니다.
- 잘못된 type 배열이나 truthy 비문자열 오류 메시지처럼 원본이 비문자열 diagnostic을 만들 수 있는 입력은 `InvalidBoundary`로 거절합니다. 스키마 자체의 오류 진단은 후속 메타 schema 단계의 책임입니다.
- 새 의존성·manifest·lock·동결 소스·공유 UI/App·실제 사용자 데이터·OS 설정을 변경하지 않았습니다. 기존 전체 App 준비 실패/OS 감시 RPC 지연·Core 성능 ignored 1·동결 Wasm tests 시간 API 불일치 7과 실기/성능/출시 게이트는 미완료입니다.
- 기능표는 303/588개(51.5%)·미완료285개를 유지합니다. 이 단위의 성공을 editor-57이나 전체 전환 완료로 확대하지 않으며 잔여 시간은 미산정입니다.
