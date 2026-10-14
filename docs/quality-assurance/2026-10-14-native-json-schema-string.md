# JSON schema 문자열 패턴·형식 검증

## 대상 파일

- Core `json-schema-scalar.rs`, `json-schema-string.rs`, `json-schema-formats.rs`, `lib.rs`, `json-language-license.txt`
- Syntax `schema-pattern.rs`, `lib.rs`, `tests/schema-string.rs`
- Core `tests/fixtures/json-schema-string-oracle.mjs`와 생성한 `.rs`, `docs/PROCESS.md`, 기능 대응표

## 리포트

설치된 `vscode-json-languageservice`의 `extendedRegExp`, `_validateStringNode`, format 정의를 기존 scalar 단계와 연결합니다. `validate_with_patterns`는 기존 타입/enum/const/폐기·문자열 길이를 실행한 뒤 pattern, format 순서로 진단을 추가합니다. 논리 UTF16 메시지와 byte/UTF16 범위, severity/code, `patternErrorMessage`→`errorMessage`→기본 메시지 우선순위를 보존합니다.

패턴은 앞의 `(?i)`만 처리하고 Unicode 컴파일을 먼저 시도하며 실패할 때 기존 방식으로 다시 컴파일합니다. 두 번 모두 실패하면 패턴 불일치 진단을 만들고, 자료/작업 용량 오류를 패턴 오류로 숨기지 않습니다. 기존 파일 연관 패턴의 compiler와 UCS2 동작은 바꾸지 않습니다.

원본의 색상·date-time/date/time·email/hostname/IPv4/IPv6 여덟 정규식과 메시지/대소문자 플래그는 oracle 생성기가 실제 설치 소스에서 추출합니다. URI 규칙은 원본의 비고정 끝 정규식에 따라 빈 문자열을 거절하고 uri의 첫 구분자 앞 scheme을 확인하며, uri-reference는 빈 문자열 이외의 상대 참조를 허용합니다. 원본이 수행하지 않는 URI 정규화나 추가 형식 판정을 넣지 않습니다.

## 검증

1. `bun native/taide-native-editor/tests/fixtures/json-schema-string-oracle.mjs`
    - exit 0, 실제 원본 format 8정의와 진단 비교 870사례를 생성했습니다. 정상/실패·Unicode/단독 surrogate·invalid pattern/Unicode fallback·메시지 override·공통 제약과의 순서·비문자열 무시를 포함합니다.
2. `cargo test --manifest-path native/taide-native-syntax/Cargo.toml --target-dir experiments/native-shell-spike/target --locked --offline --test schema-string --test schema-pattern -- --nocapture`
    - `/private/tmp/taide-json-schema-string-tests.log`, session 42774, compile 18.52초, 기존 association 2통과와 새 문자열 2통과/1실패, exit 101입니다.
    - 870사례의 원본 진단 비교와 예산 검사는 통과했습니다. 별도 backend 검사는 구형 모드에서도 `find_from_utf16`가 비BMP 쌍을 한 문자로 소비하는 차이를 재현했습니다. 컴파일 모드를 보존해 Unicode는 UTF16, 구형 모드는 UCS2 입력으로 실행하도록 수정했습니다.
    - 원본 실행기의 `new RegExp('^.$', 'u')`/구형 모드와 실제 `extendedRegExp('^\\a.$')`를 대조해 비BMP 입력에서 각각 true/false/false를 확인했습니다.
3. 수정 후 `--test schema-string`만 실행한 `/private/tmp/taide-json-schema-string-current.log`, session 68574는 compile 16.02초·3통과/0실패·exit 0입니다.
    - 긴 입력의 parser 거절 대신 파서가 수락한 4096단위 패턴을 작업 예산2048이 컴파일 전에 거절하도록 예산 검사를 강화했습니다. 이 검사만 실행한 `/private/tmp/taide-json-schema-string-budget.log`, session6852는 compile4.56초·1통과/2필터·exit0입니다. 기존 같은 예산 검사를 중복 합산하지 않습니다.
    - 현재 서로 다른 관련 성공은 기존 association2/문자열3의5건이며 870사례는 원본 비교 검사 내부의 사례 수입니다.
4. `cargo check --manifest-path native/taide-native-syntax/Cargo.toml --target-dir experiments/native-shell-spike/target --target wasm32-unknown-unknown --locked --offline --lib`
    - `/private/tmp/taide-json-schema-string-wasm.log`, session98462는 Core/Syntax lib·34.25초·exit0입니다.
5. 소유 Rust7파일의 format·소유 diff와 oracle JS/이 문서의 기존 Prettier가 통과했습니다. 최초 Core lib registry의 정렬 차이만 고쳤으며 전체 문서나 무관한 소스를 재포맷하지 않았습니다.

## 제한과 다음 작업

- 문자 수와 컴파일/호출의 누적 작업·메시지/진단 예산을 공유합니다. 기존 regress는 내부 backtracking step 제한 API를 제공하지 않으므로 정규식 자체의 성능/중단 게이트를 통과한 것으로 보고하지 않습니다. 전체 성능 게이트에 남깁니다.
- 배열/객체, 조합/조건/평가 속성, draft/메타 schema, matching schema와 실제 App 공급은 `b3e11/b3e`에 남습니다. 새 의존성·manifest·lock·동결 소스·공유 UI/App·실제 사용자 데이터·OS 설정 변경은 없습니다.
- 기능표는 누적303/588개(51.5%)·미완료285개이며 전체 전환/실기·성능·출시 게이트와 예상 시간 미산정을 유지합니다.
