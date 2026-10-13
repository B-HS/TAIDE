# 배치 23 — 내장 언어 서비스 원본 계약

현재단위는PROCESS b3/editor-57이며완료범위는원본과native 경계조사입니다. 서브에이전트/workflow 없이메인이직접읽었습니다. 고정줄b2 actual child3/기존UI32는통과했으나JSON/CSS/HTML/TS 원본내장공급까지있다고추론하지않습니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## 실제 프로젝트와 설치 코드

src/shared/lib/monaco/setup.ts는CSS/HTML/JSON/TS worker와editor worker를등록합니다. 프로젝트의추가내장옵션은configureBuiltinTypeScriptFallback이며JSON/CSS/HTML defaults의사용자옵션을설정하는코드는조회범위에서없었습니다. 타입스크립트diagnostics는semantic/suggestion off·syntax on입니다. compiler options/tsconfig·filesystem node_modules는내장worker에제공하지않고기본lib 기반조회/완성은유지합니다. typeScript/react 계열의외부서버세션은defaults 객체별전역refcount로suspend/restore합니다. React 언어ID는타입스크립트서비스suspension에는매핑되지만원본builtin worker 자체는typescript/javascript ID에만붙습니다. 현재원본setModeConfiguration이기존provider를역등록하지못하는제한은원본파일에명시돼있으며해당버그를Native로강제복제하지않습니다.

| 가족     | 설치기본/프로젝트 정책                                                                                                                    | 실제 기본 capability                                                                                      |
| -------- | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| JSON     | validate true·allowComments true·comments/trailingCommas error·schemas 빈목록·enableSchemaRequest false·schema request/validation warning | 문서/범위포맷·완성/resolve·hover·심볼·token/colors·folding·diagnostics·selection ranges                   |
| CSS/SCSS | validate true·원본lint severity·default data provider true·줄/선택자 사이새줄·collapse braces·기존줄보존                                  | 완성·hover·심볼·정의/참조/하이라이트·rename·colors·folding·diagnostics·selection·문서/범위포맷            |
| HTML     | 원본format/suggest·default data provider true                                                                                             | 완성·hover·심볼·links·하이라이트·rename·colors·folding·selection·HTML diagnostics/문서·범위포맷           |
| TS/JS    | allowNonTsExtensions·Latest·JS allowJs·프로젝트syntax-only 진단·서비스별전역suspension                                                    | 완성·hover·심볼·정의/참조/하이라이트·rename·syntax diagnostics·범위/입력포맷·signature·code actions·inlay |

근거는설치monaco-editor esm/vs/languages/features의register/각Worker·TypeScript mode와프로젝트builtin-typescript/builtin-typescript-mode입니다. JSON schema fetch 서비스구현은worker에존재하지만실제기본enableSchemaRequest는false이며외부네트워크조회가필요하다고가정하지않습니다. handlebars/razor/less의worker 구현존재를TAIDE 실제언어ID 자동활성화와동일하게보지않으며selector/파일언어는후속단위에서고정합니다.

## Native 현재 경계

native App lsp.rs는외부process/session·진단·완성·문서/선택/입력포맷·심볼/접기·정의/참조·rename·code action 등typed 소비자를이미갖습니다. crates/taide-lsp builtin_specs는vtsls/rust-analyzer 같은외부서버의manifest 정의와SDK 탐지이며프로세스/설치조건없는Monaco 내장fallback이아닙니다. 현재native manifests에서TS/JS parser/type engine·JSONC/schema engine·CSS language-service library는선언돼있지않습니다. serde_json은유효JSON Value용이고불완전편집문서·comment/recovery 위치·JSONC AST 계약의대체로간주하지않습니다. 외부server 연결만으로내장묶음을완료로올리지않습니다. 새의존성/JS UI·환경/사용자파일/동결은추가하지않았습니다.

## 다음 소유 단위

JSON worker가사용하는설치jsonc-parser의실제createScanner/parseJSONDocument부터portable core로옮깁니다. 다음b3a는JSON scanner이며소유파일은native/taide-native-editor/src/json-scanner.rs/lib.rs·실제설치scanner oracle/검사·Microsoft MIT 출처와관련QA/PROCESS/기능표입니다. 정상/불완전문자열·escape/Unicode·number/keyword·comment/trivia·원본UTF-16 offset/줄/열과native byte 경계·유한입력/token/byte 예산을검증합니다. escaped lone surrogate는논리UTF-16 code units를보존하고Rust String로검증을우회하지않습니다. 원본comment EOF의입력바깥offset 등오류는raw oracle를유지한채정확한수정기대값을별도기록합니다. 실제JSON AST/심볼/완성/진단/포맷/캐시·앱공급·CSS/HTML/TS 전체는b3에남아있습니다. scanner만으로editor-57을부분기능대체나전체완료로세지않습니다.

## b3a — JSON 원본 scanner

설치Monaco의esm/external/jsonc-parser/impl/scanner.js 444줄과actual createScanner API를읽어portable Rust scanner를구현했습니다. 정상/불완전string·Unicode escape/논리UTF-16 code units·invalidescape/control·number의부호/선행0/소수/불완전exponent·keyword/unknown·comment/공백/CRLF와EOF를다룹니다. 실제byte 범위는모든char boundary/입력안에있고UTF-16 범위와일치합니다. 값은Vec<u16>으로보존해escaped lone surrogate를Rust String 손실변환으로덮지않습니다. byte/token/전체value-unit 예산을검사하며ignored trivia도작업예산에포함하고capacity 초과는부분목록없이거절합니다. 값버퍼도남은예산+최대surrogate 두단위에서중단해출력상한을확인하기전전체값을할당하지않습니다. 새의존성/JS 런타임/manifest/lock/frozen 변경은없고Microsoft MIT 출처를포함합니다.

원본의문자열/숫자/분기경계 21문자 1~3조합과structured/Unicode/control corpus의9812입력·ignoreTrivia 두모드/19624사례를actual installed scanner로생성했습니다. raw 원본과명시적corrected 기대값을같이보관하고fixture를native 값으로교체하지않습니다. 최초generator의EOF overflow 가정은/* 끝만고려해unterminated CRLF의len+1에서도실패했습니다. 실제source 값을읽어해당두원인만정확히단언하고원본자료를생성했습니다.

parity.log는86067 handle에서compile10.12초·3통과/0.04초·exit0이며초기71수정기대값입니다. 원본comment value가앞문자한UTF-16 단위를포함하고unterminated /*/CRLF 범위가입력끝을넘는오류를수정했습니다. 마지막unterminated comment newline과illegal escaped newline 뒤line/column도actual 문서와다릅니다. 이추가교정의physical-position.log는67987 handle에서budget/range2통과·비교1실패/0.02초·exit101입니다. 검사에서source와corrected가다를때comment만허용하는예전assert가남아있어illegal escaped newline 차이를거절했습니다. 포맷문맥patch1회와공백이다른replace1회가적용되지않은상태에서실행한오류를보존하고원문을읽어명시적입력족assert로수정했습니다.

CRLF 중간offset의행계산을단순prefix split에맡기지않고설치vscode-languageserver-textdocument의actual TextDocument.positionAt을oracle 위치로사용합니다. native도CRLF는LF 뒤에서줄을증가시켜같은좌표를보존합니다. 현재77명시적수정·19547원본동일사례입니다. final.log는29231 handle에서현재3통과/0.05초·exit0으로종료했고처음3성공에반복합산하지않습니다. source range/논리values/errors·byte↔UTF-16·lone surrogate와trivia/전체/0/부분예산을직접검사했습니다.

host.log/current lib+scanner test9.05초와wasm.log/corelib4.97초는45647 handle에서exit0/추가경고0이며같은성공은커밋전에재실행하지않습니다. 소유3Rustfmt/1MJS·3.3MB raw/corrected source fixture·소유diff/라이선스·디스크618GiB/67%를확인했습니다. 기존Core343와신규3의346서로다른Core 성공/기존성능1ignored를재사용합니다. b3a scanner만닫고JSON AST/오류회복·심볼/포맷/조회/진단/완성·actual App/caching·CSS/HTML/TS 전체는부모b3에남습니다. editor-57missing/editor-16partial·전체303/588(51.5%)·미완료285·목표active/잔여시간미산정이며일반전체/OS/성능/전환게이트를유지합니다.

## b3b — JSON 구문 트리와 오류 복구

설치된 vscode-json-languageservice parser/jsonParser.js의 parse와 JSONDocument·AST 클래스, JSONC findNodeAtOffset, ErrorCode를 실제 코드로 확인했습니다. NativeEditor json-parser.rs는 기존 scanner를 사용하고 부모·자식 ID를 가진 평면 arena와 명시 작업 스택으로 파싱합니다. 배열/객체/속성/문자열·숫자·boolean/null, 콜론 위치, 누락된 키/콜론/값/쉼표·닫힘 복구, duplicate warning·주석 수집·동일 시작점 오류 억제를 보존합니다. `//` 중복 키 예외·prototype 이름·다음 줄의 문자열 키 복구도 원본과 비교했습니다. 원본 언어 서비스가 JSON.parse로 얻는 IEEE-754 값·음수 0/overflow/underflow와 소수점 표기에 따른 isInteger를 확인했습니다. 논리 문자열은 Vec<u16>으로 보존하며 외부 입력을 Rust String 단언이나 strict JSON Value로 대체하지 않았습니다.

bytes/UTF-16 두 좌표와 가장 안쪽 노드 조회의 좌우 끝 포함 계약을 유지합니다. 실제 byte 범위는 입력 안의 UTF-8 경계이며 콜론·부모/자식도 대조했습니다. 노드/깊이/진단/주석과 scanner의 전체 token/value/byte 예산은 부분 결과 없이 typed Capacity로 거절합니다. arena/명시 스택은 8,192단계 배열·4,096단계 객체의 생성·조회·폐기와 미완료 중첩 EOF/4,096공백에서 동작합니다. 같은 EOF 오류 범위는 한 번만 역탐색하며 재귀 호출과 재귀 소유 구조를 만들지 않았습니다. MIT 출처에 JSON language service를 추가했고 새 의존성/JS 런타임·manifest/lock/동결 변경은 없습니다.

실제 설치 parse의 12문자 1~3 조합·구조 입력의 모든 prefix/한 문자 삭제/쉼표 삽입·Unicode/escape/주석/원본 whitespace·15숫자 경계에서 3,168입력 × 주석 수집 두 모드 = 6,336 AST/회복/진단 사례를 생성했습니다. float는 문서의 문자 값이 아닌 실제 64bit 값으로 비교하며 long exponent/decimal 4,096자리도 포함합니다. 노드 조회는 actual JSONDocument.getNodeFromOffset의 1,118 좌우 경계 사례를 사용합니다. native 출력으로 golden을 생성하지 않습니다.

parity.log는3955 handle에서 compile10.52초·2통과/1실패·0.04초·exit101입니다. 54차이는 원본 unclosed comment의 입력 밖 EOF가 빈 오류 범위로 clamp되는 가족이었습니다. corrected.log는85059 handle에서 compile4.34초·같은54차이/2통과/1실패·exit101입니다. 첫 교정에서 EOF offset만 확인해 token length가1인 실제 overflow를 놓쳤습니다. actual scanner를 직접 읽은 값은 token offset=len·length1·position=len+1이며 이를 정확히 단언한 뒤 교정했습니다. eof.log는22937 handle에서 compile10.19초·2통과/1실패·0.04초·exit101이며 이전54차이는 없어지고 비BMP 문자 중간 EOF 진단4사례가 남았습니다. 원본 1 UTF-16 단위 backstep을 실제 scalar 전체로 교정하면서 같은 물리 시작점에 중복되는 후속 오류를 원본의 시작점 억제 계약으로 정리했습니다. 같은 가정의 세 번 실패로 숨기거나 제품 실패로 확대하지 않습니다.

raw/corrected fixture를 함께 유지합니다. 명시 교정은70사례이며 source AST 끝이 입력 len+1일 때만 clamp하고 scanner EOF의 offset=len/length1을 검사합니다. 원본 오류의 half-surrogate 범위는 전체 scalar로 넓혀 유효한 native 위치를 유지합니다. query.log는78038 handle에서 compile10.22초·4통과/0.04초·exit0이며 숫자 실패/EOF 범위 캐시·긴 숫자 추가 후 final.log는40694 handle에서 compile9.41초·현재4통과/0.05초·exit0입니다. 이전4건과 합산하지 않습니다. 로그는 /private/tmp/taide-json-parser-{parity,corrected,eof,query,final}.log입니다.

host.log/lib+json-parser test9.01초와 wasm.log/corelib4.95초는97927 handle에서 exit0·추가경고0으로 종료했습니다. 같은 성공은 커밋 전에 재실행하지 않습니다. 소유 Rust3 fmt와 MJS prettier는 통과했으며 다른 소스/사용자 문서는 선별 기록에서 제외합니다. shell의 unquoted crates/taide-browser* wildcard가 없어서 diff 경계 확인만 exit1이었으며 실제 파일 변경 실패나 format 실패가 아닙니다. 기존 Core346 + 신규4 = 서로 다른350성공이며 기존 성능1ignored·전체 App 준비 실패/remote Wasm Instant·OS/성능/전환 게이트는 유지합니다. b3b만 검증하고 JSON 심볼/접기/selection·포맷/진단 정책/완성/hover/colors와 실제 앱 공급·CSS/HTML/TS 전체는 부모b3에 남습니다. editor-57missing/editor-16partial·전체303/588(51.5%)·미완료285·목표active·잔여시간미산정입니다.
