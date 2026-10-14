# JSON schema 구조·조합 평가

## 대상 파일

- Core `json-schema-evaluation.rs`, `json-schema-structure.rs`, `json-schema-scalar.rs`, `json-schema-string.rs`, `json-value.rs`, `lib.rs`, JSON MIT 고지
- Core `tests/fixtures/json-schema-evaluation-oracle.mjs`와 생성한 `.rs`, Syntax `tests/schema-evaluation.rs`
- PROCESS와 기능 대응표

## 리포트

설치 `vscode-json-languageservice`의 배열/객체 검증, allOf/anyOf/oneOf/not/조건 분기, ValidationResult의 비교·병합과 평가된 속성 집합을 기존 값 arena·scalar·pattern 단계에 연결합니다. `type`→조합→enum/const/폐기→문자열/숫자→구조 순서를 유지합니다. 공통 scalar 검증을 두 단계로 나누고 검증 요청 전체의 남은 작업 예산을 공유합니다.

평가 프레임과 후속 연산을 명시 스택에 보관합니다. 같은 문서 노드/스키마 쌍의 진행 없는 재진입은 거절하며, 실제 자식 문서로 진행하는 재귀 참조는 허용합니다. 진단·매칭·평가 집합·메시지·작업·프레임 상한을 적용합니다. 대기 중인 진단도 생성 즉시 예산에 포함하며 빈 결과로 병합할 때는 벡터 소유권을 옮겨 깊은 문서에서 반복 복사를 줄입니다.

AST의 key/value 대응을 유지해 propertyNames가 기존 키 문자열과 부모 property 범위를 사용하게 합니다. 기존 `Arena::from_document`의 값·키 순서와 공개 반환형은 유지하고 평가기용 매핑에서만 key 값도 포함합니다. 중복 키는 속성 순서와 마지막 값/키 위치를 구분하고, 없는 값은 required 판정에 사용하지 않습니다.

배열의 prefix/items/additional·contains·unevaluated·범위·중복, 객체의 required/properties/patternProperties/additional/unevaluated·크기·세 가지 의존성·propertyNames를 처리합니다. draft 식별자와 명시 draft에 따라 tuple 배열과 2020 prefixItems 정책을 구분합니다. 분기 선택은 오류 수가 아니라 원본의 오류 유무·enum/primary/value/property 점수를 사용하고, 동점 enum 목록과 매칭·평가 집합을 병합합니다.

## 검증 기록

1. `bun native/taide-native-editor/tests/fixtures/json-schema-evaluation-oracle.mjs`
    - 최초 시도는 이미 export된 ValidationResult의 중복 export로 실패했습니다. 임시 원본 모듈에 비공개 평가 진입점만 export하도록 수정한 뒤 exit0, 원본690사례·문자열 길이 교정2사례를 생성했습니다.
    - 진단의 byte/UTF16·코드/심각도/논리 메시지/순서, ValidationResult 점수·enum metadata·평가 집합, 실제 객체 스키마의 매칭 순서를 비교합니다. boolean 변환이 만드는 임시 객체는 실제 schema 객체 매칭 목록에서 제외합니다.
2. Core lib 기본 check는 session61830·7.53초·exit0입니다. 사용하지 않는 Kind/Node import 경고를 확인해 제거했으며 이후 검사는 현재 코드로 수행합니다.
3. `cargo test --manifest-path native/taide-native-editor/Cargo.toml --target-dir experiments/native-shell-spike/target --locked --offline --lib --test json-value --test json-schema-scalar -- --nocapture`
    - `/private/tmp/taide-json-schema-evaluation-core.log`, session43776는 compile1분25초·unit1/scalar5/value3의9통과·exit0입니다. 기존 Arena/공통/문자열 길이·직렬화·예산과 공개 API가 보존됩니다.
4. `cargo test --manifest-path native/taide-native-syntax/Cargo.toml --target-dir experiments/native-shell-spike/target --locked --offline --test schema-evaluation --test schema-string -- --nocapture`
    - `/private/tmp/taide-json-schema-evaluation-tests.log`, session8351는 compile44.16초·새 평가기5/문자열3의8통과·exit0입니다. 원본690사례의 진단/점수/평가 집합/실제 객체 매칭과 2048단 문서·schema, 용량 예산, 명시 draft 우선순위·불완전/중복 입력, 실제 resolver의 자식 진행 재귀·공유/비진행 순환 거절을 확인했습니다. 문자열870사례도 현재 공유 helper에서 통과했습니다.
    - 현재 서로 다른 관련 성공은 Core9/Syntax8의17건입니다. oracle 사례 수나 이전 같은 검사의 성공을 중복 합산하지 않습니다.
5. 소유 Rust7파일의 format·소유 diff와 oracle JS/이 문서의 기존 Prettier는 통과했습니다.
6. `cargo check --manifest-path native/taide-native-syntax/Cargo.toml --target-dir experiments/native-shell-spike/target --target wasm32-unknown-unknown --locked --offline --lib`
    - `/private/tmp/taide-json-schema-evaluation-wasm.log`, session13256는 Core/Syntax lib·57.32초·exit0입니다. 현재 App 전체나 동결 Wasm tests의 성공으로 확대하지 않습니다.

## 남은 범위

- 메타 schema와 실제 App 진단/설정 소비는 부모 `b3e11/b3e`에 유지합니다. matching collector의 focus/exclude와 boolean 임시 스키마 표현은 조회/완성 소비와 함께 검증할 경계이며 현재 실제 객체 매칭 검사로 전체 collector 완료를 선언하지 않습니다.
- 기존 regress 내부 backtracking의 성능/중단 게이트, 전체 App 준비/OS 감시 RPC 지연·Core ignored 성능 검사·동결 Wasm tests 시간 API 불일치와 실기/접근성/출시/전환은 미완료입니다.
- 새 의존성·manifest·lock·동결·공유 UI/App·실제 사용자 데이터·OS 설정 변경은 없습니다. 전체303/588개(51.5%)·미완료285개와 잔여 시간 미산정을 유지합니다.
