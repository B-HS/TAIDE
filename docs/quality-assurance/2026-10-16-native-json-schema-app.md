# JSON schema 의미 검증의 실제 App 공급

## 대상 파일

- Core:
    - 신규 `json-schema-document.rs`, `resources/json-schema-draft-04.json`, `resources/json-schema-draft-07.json`
    - 변경 `json-schema-policy.rs`, `lib.rs`
- App: `editor-json-validation.rs`와 해당 테스트
- 추출 스크립트: `docs/utils/2026-10-16-extract-json-schema-contributions.ts`
- 원본 대조: `tests/fixtures/json-schema-document-oracle.mjs`와 생성한 `.rs`, Syntax `tests/schema-document.rs`

## 원본 동작

TS 앱은 Monaco JSON 기본값을 바꾸지 않습니다. `src/`에 `jsonDefaults`·`setDiagnosticsOptions` 호출이 없습니다. 설치 Monaco `register.js`의 기본값은 다음과 같습니다.

- `validate: true`, `allowComments: true`, `schemas: []`
- `enableSchemaRequest: false`
- `schemaRequest`·`schemaValidation`: `"warning"`
- `comments`·`trailingCommas`: `"error"`

`jsonWorker.js`는 이 값을 언어 설정과 문서 설정으로 함께 넘깁니다. 요청 서비스는 없습니다(`schemaRequestService: undefined`). 언어 서비스는 `configuration.js`의 `schemaContributions`(draft-04/draft-07 메타 schema)를 등록합니다. 그 결과는 다음과 같습니다.

- 문서의 `$schema`가 두 메타 schema 중 하나이면 문서를 그 메타 schema로 의미 검증합니다.
- 다른 `$schema`(원격·상대 경로·2020-12 등)는 `Unable to load schema from '…'. No schema request service available` 경고(768)를 `$schema` 값 위치에 냅니다. `$schema`가 첫 속성이 아니면 루트 첫 글자에 냅니다.
- `$schema`가 없거나 문자열이 아니면 schema 없이 구문·주석 진단만 냅니다.

## 구현

- 추출 스크립트가 설치 원본의 두 메타 schema를 리소스로 저장합니다. 이 리소스는 JSON 언어 서비스 MIT 고지(`json-language-license.txt`)의 범위입니다. `json_schema_document::contributions`가 원본 등록 순서대로 읽고, App은 프로세스당 한 번만 parse합니다(`OnceLock`).
- `json_schema_document::diagnostics`는 다음을 묶습니다.
    1. 문서 parse
    2. `Service` 생성과 contribution 등록
    3. `select(resource, document)`
    4. graph `Job` 진행
    5. 정책 진단
    - 4단계의 모든 외부 load에는 원본처럼 `Response::Unavailable`을 게시합니다.
    - graph 단계 상한과 취소 확인이 있습니다.
- 정책 진단에 이미 parse한 문서를 받는 `document_diagnostics`를 추가해 parse를 한 번만 합니다.
- App의 `editor-json-validation` 작업이 구문 진단 대신 이 함수를 호출합니다. 기존 500ms 지연·최대 1개 worker·Normal 등급만·AppFile 제외·취소/닫힘/오래된 결과 처리는 그대로입니다.
    - resource URI는 기존 구조 공급과 같습니다. 파일은 `file://` URL, untitled는 `untitled:{tab}`입니다.
    - 상한은 기존 대형 파일 byte 상한을 기준으로 정했습니다.
- 새 네트워크·파일 요청은 만들지 않습니다. b3e10 전송 adapter는 원본의 `enableSchemaRequest: false` 때문에 이 경로에서 쓰지 않습니다.

## 검증 기록

1. `bun native/taide-native-editor/tests/fixtures/json-schema-document-oracle.mjs`
    - 최초 실행은 OS 임시 디렉터리(`/var/folders`)에 쓴 worker 모듈을 Bun이 찾지 못해 실패했습니다. 기존 oracle처럼 `/private/tmp`를 쓰도록 바꾼 뒤 57사례를 생성했습니다.
    - 실제 `JSONWorker`와 `register.js`의 기본값을 사용했습니다. URI 3종(파일·한글 퍼센트 경로·untitled)과 입력 19종을 조합했습니다.
2. `cargo test --manifest-path native/taide-native-syntax/Cargo.toml --target-dir experiments/native-shell-spike/target --locked --offline --test schema-document`
    - 2통과입니다. 57사례가 최초 실행부터 원본과 일치했습니다.
    - 취소, 단계 상한, 비활성 설정도 확인했습니다.
    - 정책 refactor 뒤 `schema-policy`·`schema-matching`도 다시 통과했습니다(각 2).
3. App `--lib editor_json_validation`은 3통과입니다. 새 검사는 다음을 확인합니다.
    - 한글·공백 경로의 readonly 파일에서 메타 schema enum 경고가 나옵니다.
    - untitled 상대 `$schema`에서 768 경고가 trailing comma보다 먼저 나옵니다.
    - 원문이 보존됩니다.
4. App `--lib json`은 20통과입니다. 실제 App의 JSON 진단·문제 목록·peek를 포함합니다.
5. App lib 전체(`--no-fail-fast`, 보호 Trash 3건 skip)는 807통과·12실패입니다(15분 40초).
    - 실패 12건만 다시 실행했습니다. LSP 2건은 통과했고 10건은 같은 실패를 재현했습니다.
    - 재현한 10건은 다음과 같습니다.
        - 프로젝트 열기·원격 열기·원격 dispatch 시간 초과
        - 실제 App 종료의 tasks=2
        - 글꼴 face 공유
    - 이 변경을 stash한 상태에서도 projects hooks·editor_fonts 2건이 같은 방식으로 실패했습니다. JSON 변경과 무관한 기존 환경 부채로 판단합니다(FSEvents 등록 지연·시스템 글꼴 상태). 원인은 확정하지 않았고 배치 31에 둡니다.
6. Syntax lib Wasm check와 동결 remote-web Wasm lib check가 exit 0입니다.
7. 소유 Rust 6파일 rustfmt와 JS·TS 2파일 Prettier를 통과했습니다.

## 남은 범위

- JSON hover·completion의 schema 소비, 언어 선택 UI, CSS/HTML/TS 내장 서비스는 editor-57에 남습니다.
- 대형 문서 regex backtracking 성능 게이트도 남습니다.
- 새 의존성·manifest·lock 변경은 없습니다.
