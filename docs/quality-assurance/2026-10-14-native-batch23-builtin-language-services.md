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

## b3c1 — JSON 선택 확장

설치 jsonSelectionRanges.js 전체와 JSONDocument.getNodeFromOffset·JSONC findNodeAtOffset을 확인했습니다. json-selection.rs는 기존 AST·scanner를 사용해 실제 문자 경계의 cursor마다 안쪽 내용·전체 노드·속성/배열 항목 뒤 첫 유효 쉼표·부모 순서의 평면 범위 체인을 반환합니다. 주석/공백 사이의 쉼표도 원본과 일치하며 배열/속성/문자열의 중복된 단계는 원본 계약대로 유지합니다. 노드가 없으면 cursor의 빈 범위를 반환합니다. 입력은 byte cursor로 검증하고 UTF-16 unit→byte 색인으로 원본 unit 기반 안쪽 범위를 계산합니다. 불완전 문자열에서 half-surrogate로 끝나는 원본 범위는 실제 scalar 전체로 넓히며 원문의 논리 UTF-16 값은 바꾸지 않습니다. byte 밖/문자 중간 cursor는 typed InvalidBoundary로 거절합니다.

실제 설치 getSelectionRanges를 정상/불완전 문자열·escape/lone surrogate·number/boolean/null·객체/배열·중첩·주석/CRLF/쉼표의164입력 모든 실제 cursor 위치3,303사례에서 호출했습니다. 원본 raw와 명시 교정 기대값을 모두 fixture에 기록했습니다. 최초 parity.log는24744 handle에서 compile10.00초·2통과/1실패·0.06초·exit101입니다. 9차이는 scanner에서 이미 고친 입력 바깥 EOF 길이1이 원본 AST의 안쪽 범위에도 영향을 주는 가족이며 제품 경계 검사는 통과했습니다. 실제 scanner의 EOF offset=len/length1과 source AST end=len+1을 단언한 뒤 source AST 범위만 len으로 교정하고 actual getSelectionRanges를 다시 호출했습니다. 이를 native 출력으로 대체하지 않았습니다. 문자 중간 범위4건과 EOF 영향9건의 명시 교정은13사례입니다.

corrected.log는79780 handle에서 compile4.28초·현재3통과/0.06초·exit0입니다. 원본 범위/쉼표/부모 순서, 다중 cursor의 독립 순서/같은 cursor 반복·모든 범위의 포함/UTF-8/UTF-16, 노드 없음/빈 입력·8,192단계 배열의16,385범위와 전체 positions/ranges/parser 예산을 검사했습니다. capacity 오류는 부분 목록을 반환하지 않으며 재귀 호출/소유 구조를 사용하지 않습니다. 이전3건과 반복 합산하지 않습니다. 소유 Rust3 fmt/MJS prettier·소유diff/manifest·lock·동결 변경0을 확인했습니다. host.log/lib+json-selection test13.68초와 wasm.log/corelib4.89초는31716 handle에서 exit0·추가경고0으로 종료했으며 동일 성공은 재실행하지 않습니다. 기존Core350과 신규3의 서로 다른353성공/성능1ignored이며 b3c1만 닫고 심볼/접기는 부모b3c·실제 앱 공급과 나머지 JSON/CSS/HTML/TS는 b3에 미완료로 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정이며 전체 App 준비 실패/remote Wasm/OS·성능·전환·최종33 게이트는 유지합니다.

## b3c2 — JSON 문서 심볼

설치 JSONDocumentSymbols의 findDocumentSymbols/findDocumentSymbols2·getKeyLabel/getDetail/getName과 실제 JSONC getNodeValue를 확인했습니다. json-symbols.rs는 기존 AST의 평면 arena와 명시 BFS queue를 사용합니다. nested 객체 키/배열 인덱스·부모/자식/selection·detail과 flat 객체 키/점으로 연결한 container 이름·배열 통과, 실제 SymbolKind·원본 결과 제한/초과 callback을 보존합니다. source resultLimit 0은 무제한이고 일반 모드는 실제 추가 결과가 있을 때만 초과로 표시합니다. keybindings 예외는 원본처럼 제한에 도달한 마지막 결과에서도 callback을 표시합니다. 자원 상한은 별도 max_symbols/전체 text units와 parser 예산으로 두고 부분 결과 없이 Capacity로 거절합니다. flat container의 작업 queue 복제도 전체 text 예산에 포함합니다.

name/detail/container는 Vec<u16>으로 escaped lone surrogate를 보존합니다. 키의 newline→원본 줄바꿈 표식·공백뿐인 키의 따옴표·빈 배열/객체 detail·원본 IEEE-754 String 표시의 음수0/NaN/infinity·소수/지수 경계가 실제 oracle과 일치합니다. keybindings 예외는 정확한 default URI 또는 Unicode lowercase 이후 /user/keybindings.json 접미사이며 query string과 default URI 대문자 차이를 그대로 구분합니다. 일반 문서의 키 이름에 영향을 주지 않습니다.

원본 getName은 getNodeValue를 반환하므로 잘못된 key 값에서 name이 문자열 아닌 boolean/number/array/null-prototype object가 됩니다. 첫 generator에서 null-prototype object의 String 변환이 TypeError: No default value로 실패했습니다. jsonc getNodeValue의 Object.create(null)을 실제로 확인했습니다. 설치 monaco.d.ts의 DocumentSymbol.name은 string입니다. typed native 이름은 falsy 원본의 <empty>를 유지하고 primitive/array join·객체 안전 표시를 문자열로 변환합니다. raw source 이름의 타입/값과 교정 문자열을 함께 기록하며 이 오류를 native UI로 강제 복제하지 않습니다. actual provider 두 API의 결과를 먼저 얻고 source canonical은 객체를 JSON.stringify로 보존합니다. native 결과로 기대값을 생성하지 않습니다.

78입력·nested/flat 두 모드·URI6개·resultLimit None/0/1/2/4/8의5,616사례를 actual 설치 provider로 생성했습니다. 객체/배열·중첩/불완전/주석·키 이름/중복/논리Unicode·숫자20경계·keybindings와 URI의 Kelvin 대소문자를 포함합니다. 명시 타입 교정558사례이며 native 표시는 유효한 문자열입니다. 모든 source 범위/selection/부모의 원문 포함·UTF-8/UTF-16을 검사하며 논리 d800/dfff 값을 별도로 단언합니다. 4,096깊이의 실제 생성/폐기와 전체 심볼/표시값/parser 예산·원본 결과 제한을 확인했습니다.

first.log/lib은12870 handle에서5.04초·exit0입니다. 첫 generator 실패 뒤 원본 fixture가 없는 상태에서도 이어진 parity.log Cargo는49247 handle에서 컴파일 exit101·실행 검사0건으로 종료했습니다. 이 실패를 성공에 포함하지 않고 이후 생성/format 앞에 set -e를 적용했습니다. Prettier 실제 줄과 다른 patch2회는 적용되지 않았으며 파일을 읽고 정확히 수정했습니다. canonical.log는67073 handle에서 compile4.49초·3통과/0.03초·exit0의URI4개/3,744사례입니다. Unicode lowercase URI 두 사례를 추가한 uri.log는65449 handle에서 compile9.00초·현재3통과/0.05초·exit0의5,616사례입니다. 이전3건과 중복 합산하지 않습니다.

소유 Rust3 fmt/MJS prettier·소유diff와 manifest/lock/동결 변경0을 확인했습니다. host.log/lib+json-symbols test8.71초와 wasm.log/corelib4.85초는67928 handle에서 exit0·추가경고0으로 종료했으며 동일 성공을 재실행하지 않습니다. 기존Core353+신규3의356서로 다른 성공/기존성능1ignored를 유지합니다. 문서 심볼의 portable core만 닫고 실제 App 공급·접기/나머지 JSON 서비스·CSS/HTML/TS 및 전체App 준비/remoteWasm·OS/성능/전환/최종33 게이트는 미완료로 남깁니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3c3 — JSON 접기

설치 jsonFolding.js 전체와 현재 native SyntaxFoldRange/FoldRegion·문서별 SyntaxFolds 수명을 실제 코드로 대조했습니다. json-folding.rs는 기존 scanner의 실제 byte 경계를 원본 CR/LF/CRLF 줄 시작으로 변환해 객체/배열 닫힘 이전 줄·블록 comment 전체 줄·region·종류 불일치·동일 시작 억제를 구현합니다. 닫히지 않은 comment는 원본처럼 다음 물리 줄부터 scanner를 다시 시작하고 EOF/반복 comment와 줄 안의 문자열/중괄호를 구분합니다. 기존 parser의 ECMA 공백 helper는 구현 그대로 crate 내부에만 노출해 region의 공백/BOM/NBSP·ASCII word boundary를 적용했습니다. 공유 타입/manifest/lock/동결은 바꾸지 않았습니다.

원본 regex /^\/\/\s*#(region\b)|(endregion\b)/의 두 번째 대안은 comment 안 임의 endregion 단어를 종료 지시자로 취급합니다. 또한 제한 계산이 level<30만 집계해 깊은 영역의 결과 제한에서 정당한 범위를 누락합니다. 실제 설치 source를 읽고 두 정확한 marker가 한 번씩만 존재함을 단언했습니다. golden의 raw는 수정 없는 actual getFoldingRanges를 호출합니다. 교정은 동일 실제 source의 해당 regex를 양쪽 모두 // 공백 #로 묶고 count guard를 모든 비음수 깊이로 고친 private temp 모듈을 호출합니다. 상대 import는 실제 파일 URI로만 재배치했으며 native 구현으로 기대값을 만들지 않습니다. 이 test-only source의 runtime/새 의존성을 제품에 반입하지 않았습니다.

native 결과 제한은 원본의 순서·얕은 깊이 우선·같은 깊이의 기존 순서를 유지하고 초과 callback 상태를 함께 반환합니다. 자원 상한은 max_depth/ranges와 scanner token/value/input byte·재탐색 전체 work byte에 별도로 적용합니다. suffix scan에서 이미 소비한 token/value 예산을 되돌리지 않으며 반복 미완료 comment의 누적 작업을 검증합니다. 명시 stack/평면 범위를 사용해 깊은 생성/조회/폐기에도 재귀 소유 구조를 만들지 않습니다.

99입력·원본 rangeLimit None/0/1/2/4/8/32의693사례에서 객체/배열·혼합/불일치·주석/불완전 재탐색·region/중첩/잘못된 종료·Unicode whitespace/word boundary·세 newline 방식과 깊이1/2/4/29/30/31/34를 대조했습니다. raw 교정41사례는 두 원본 오류에만 해당합니다. 4,096깊이와 원문 줄 범위/닫힌 줄 제외·제한과 전체 작업/값/token/byte/range/depth 예산을 별도로 검증했습니다.

first.log/lib은62531 handle에서4.90초·exit0입니다. parity.log는58124 handle에서 compile10.27초·2통과/비교1실패·0.01초·exit101입니다. 실패는 raw/교정이 다른 입력에서 region 또는 깊은 객체만 허용하는 검사 assert가 깊은 배열을 빠뜨렸기 때문입니다. 실제 출력 불일치로 단정하지 않고 해당 원본 guard 가족도 정확히 허용한 뒤 corrected.log를2522 handle에서 compile8.39초·현재3통과/0.02초·exit0으로 확인했습니다. 이후 숫자29 비교를 같은 의미의 source depth limit30 상수로만 정리했으며 입력/검사 의미가 같은 성공은 반복 실행하지 않습니다.

소유Rust3/MJS format과 helper의1줄 가시성 diff·MIT·manifest/lock/동결 변경0·디스크618GiB/67%를 확인했습니다. host.log/lib+json-folding test4.22초와 wasm.log/corelib4.85초는8830 handle에서 exit0·추가경고0으로 종료했습니다. helper를 포함한 소유Rust4 fmt가 통과했으며 동일 성공은 재실행하지 않습니다. 기존Core356+신규3의359서로 다른 성공/성능1ignored를 유지합니다. 선택 확장·심볼·접기의 portable 구조 서비스 b3c는 구현·원본/경계/예산 검증을 마쳤지만 실제 App 공급과 JSON 진단 정책/포맷/조회/완성/colors·CSS/HTML/TS·전체 App 준비/remoteWasm·OS/성능/전환/최종33 게이트는 부모b3/전체 목표에 남습니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3d — JSON 포맷

실제 JSONWorker.format·language service utils/format.js wrapper와 jsonc-parser impl/format.js 전체를 확인했습니다. json-format.rs는 기존 scanner/IndentOptions/typed Edit를 사용해 문서/선택 범위의 줄 확장·초기 들여쓰기·0 tabSize의4 fallback·공백/탭·원문 첫 EOL 우선·keepLines·최종 줄·주석/불완전 입력의 국소 편집과 원본 편집 순서를 구현합니다. wrapper는 EOL을LF로 제공하고 원문에CR/LF가 있으면 원본 포맷터가 해당 첫 EOL을 선택합니다. raw 포맷터의 fallback EOL도 실제 API대로 처리합니다. 새로운 JSON stringify 대체나 전체 문서 재작성으로 축소하지 않습니다.

출력은 newlines/indent/spaces 반복 패턴으로 표현해 범위/오류/이미 같은 텍스트를 먼저 판정합니다. 동일 구간은 출력 문자열을 할당하지 않습니다. 실제 편집을 만드는 경우에만 전체 output bytes/편집 수를 checked 연산과 제한으로 검사하고 부분 결과 없이 Capacity로 거절합니다. 폭/입력 byte·전체 scanner token/value와 실제 scalar 범위도 검증하며 음수 indentation의 원본 반복0 계약을 보존합니다. 새 의존성/JS 런타임·manifest/lock/동결/공유 타입은 바꾸지 않았습니다. Microsoft MIT 출처를 포함합니다.

48입력·indent6개·keepLines/finalNewline 두 값·raw/actual service wrapper 두 경로·문서/전체 범위/빈 cursor/뒤 범위4개에서9,216사례를 실제 설치 코드로 생성했습니다. 정상/불완전·Unicode/escape·주석/CR/LF/CRLF·선행/빈 줄/탭·잘못된 구조·fallback U+2028도 원본 동작 대조에 포함합니다. raw/교정 결과와 실제 편집 적용 결과를 함께 fixture에 저장했습니다. source EOF가 입력 밖 len+1인 block comment/EOF 13/17 종류일 때만 단언한 scanner wrapper로 getTokenOffset/Length를 입력에 clamp하고 실제 원본 formatter를 다시 호출했습니다. native 출력으로 기대값을 만들지 않습니다. 명시 교정은60사례이며 service의 실제 TextDocument 위치도 직접 비교합니다.

first.log/lib은13311 handle에서4.88초·exit0입니다. parity.log는15230 handle에서 compile10.10초·3통과/0.06초·exit0입니다. 모든 편집의 원문 범위/UTF-8 경계·UTF-16/순서/실제 적용 결과, 범위 밖/문자 중간과 전체 출력/편집/폭/token·깊이1,024의 출력 예산 실패 및 무편집0 budget을 확인했습니다. 동일 성공은 재실행하지 않습니다.

실제 기존 formatting::apply_edits 소비를 readonly/writable 두 문서의 real EditorStore/DocumentId/ViewId·같은 문서 두 뷰와 검사했습니다. consumer.log는57970 handle에서 compile4.43초·1실패/0.00초·exit101이며 undo 후 dirty=false 기대가 틀렸습니다. native store의 requires_save=true·기존 store 검사와 TS code-editor.onDidChangeModelContent→use-editor-file-persistence.handleChange/setDirty(true)를 실제로 확인했습니다. 원문 복원 뒤에도 dirty=true를 보존하는 현재 원본/저장 계약을 기대값에 반영했습니다. 제품 저장 정책은 수정하지 않았습니다. consumer-final.log는80354 handle에서 compile4.49초·실제소비1통과/0.00초·3선별제외·exit0입니다. 적용된 전체 결과/비BMP 역방향 선택·mirror 단어 선택·단일 undo/추가undo 없음·기존 dirty·readonly 내용/revision/clean·stale 요청 거절을 확인했습니다. 기존3+신규소비1의서로 다른4성공이며 반복 합산하지 않습니다.

소유Rust3/MJS format·소유diff와 manifest/lock/동결 변경0을 확인했습니다. host.log/lib+json-format test9.14초와 wasm.log/corelib5.09초는14409 handle에서 exit0·추가경고0으로 종료했습니다. 같은 성공은 재실행하지 않습니다. 기존Core359+신규4의363서로 다른 성공/성능1ignored입니다. portable 포맷과 기존 Core 편집 소비만 닫고 실제 App 내장 공급/진단 정책·schema/조회/완성/colors·CSS/HTML/TS·전체 App 준비/remoteWasm·OS/성능/전환/최종33 게이트는 부모b3/전체 목표에 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3e1 — JSON 구문 진단 정책

실제 JSONValidation.configure/doValidation·toDiagnosticSeverity와 JSONDocument.parse·원본 메시지/코드/source 및 현재 native Marker/Message/MarkerSet을 확인했습니다. json-validation.rs는 기존 AST의 구문 진단/주석을 기존 Marker로 연결합니다. 전체 validate off·allowComments·문서 comments/trailingCommas의 None/error/warning/ignore·schema 허용 우선순위·duplicate warning·syntax 먼저/주석 나중 순서와 같은 위치/같은 메시지 중복 억제를 보존합니다. source는 구문에 실제 language 문자열, 주석 금지에는 None이며 코드/원본 영어 메시지를 함께 보존합니다. parser Error.message만 추가했으며 파싱/회복의 기존 구현을 바꾸지 않았습니다.

schema_allows_comments/trailing_commas는 실제 schema 해결 결과가 제공해야 하는 허용 정책이며 여기서 schema 선택/해결/의미검증을 대체하지 않습니다. 설치된 configuration.js는 schemas=[] 설정과 별개로 draft meta schema를 번들합니다. 원본 JSONSchemaService/JSONDocument.validate 전체와 App 공급은 부모b3e/전체b3에서 미완료로 유지하며 default schemas=[]를 이유로 생략하지 않습니다. 새 의존성/JS 런타임·manifest/lock/동결·공유 타입 변경은 없습니다.

36입력·validate2/allowComments2·comments4/trailingCommas4·schema 정책4의9,216사례를 실제 JSONValidation 클래스에서 실행했습니다. schema service 포트는 원본 DI 경계에서 없음/허용 옵션만 가진 resolved schema를 반환하며 실제 parser와 실제 validation 구현은 그대로 사용합니다. 정상/불완전/숫자·escape/control·키/중복·Unicode와 CRLF/주석/쉼표를 다룹니다. raw는 수정 없는 실제 parser, 교정은 actual scanner의 block comment/EOF len+1만 clamp하고 실제 parser _errorAtRange의 half-surrogate 범위를 scalar 전체로 맞추는 원본 모듈을 사용합니다. 해당 marker가 source에 정확히 한 번씩 있는지 확인했습니다. 같은 normalized 시작점의 원본 오류 억제도 보존하며384명시교정 사례를 raw와 같이 기록합니다. native 결과로 기대값을 만들지 않습니다.

scanner source 범위 교정 helper는 format/validation 두 실제 사용에서 공유했습니다. format oracle의 구현은 이 helper 호출로만 바뀌었고 전체9,216기대값/실제 적용 fixture를 재생성·rustfmt한 뒤 기존 파일과 cmp가 byte-for-byte exit0입니다. 기존 포맷 검사의 같은 성공을 다시 실행하지 않았습니다. 제품 source와 oracle 수정 파일의 역할을 구분합니다.

first.log/lib은16204 handle에서4.84초·exit0입니다. parity.log는38618 handle에서 compile10.75초·현재3통과/0.03초·exit0입니다. 실제 영어 메시지/숫자 code/심각도/source/UTF-16 범위/순서와 옵션 조합, 실제 readonly EditorStore snapshot에서 비BMP 키·duplicate warning2/trailing1/comment1의 MarkerSet 수명/범위·원문/revision/clean 보존을 확인했습니다. 전체 output 진단·message bytes·parser 예산은 부분 결과 없이 거절하고 validation off/ignore/no diagnostic의0 budget은 불필요한 출력을 만들지 않습니다.

소유Rust4/JS3 format·format fixture 동일성·소유diff/라이선스·manifest/lock/동결 변경0을 확인했습니다. host.log/lib+json-validation test4.69초와 wasm.log/corelib4.93초는89241 handle에서 exit0·추가경고0으로 종료했으며 동일 성공은 재실행하지 않습니다. 기존Core363+신규3의366서로 다른 성공/성능1ignored입니다. b3e1 구문 정책만 닫고 부모b3e의 schema 전체·실제 App/조회/완성/colors·CSS/HTML/TS와 전체 App 준비/remoteWasm·OS/성능/전환/최종33 게이트는 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3e2 — JSON schema 값·graph 기반

실제 JSONSchemaService의 register/선택/캐시/참조/anchor/known schema 순회·getSection과 현재 JSONDocument/getNodeValue/equals/stringLength를 읽었습니다. schema 선택/의미검증의 숫자·배열·객체/키·논리 문자열과 참조 해결의 공유 node ID를 위해 json-value.rs의 평면 value arena를 구현했습니다. AST property/key wrapper를 값에서 제거하고 배열/객체/primitive만 유지하며, 누락된 속성 값은 원본처럼 생략합니다. 중복 키는 마지막 값/첫 키 위치를 유지하고 JS Object.keys의 canonical array index0..4294967294 우선·오름차순/나머지 삽입 순서를 보존합니다. prototype 이름과 escaped lone surrogate도 자료로만 유지하며 Rust String 손실 변환을 쓰지 않습니다. arena 값은 외부에서 읽기만 가능하고 루트/멤버/항목은 실제 생성 node ID로 조회합니다.

문자열/숫자/boolean/null·NaN/infinity/음수0·공유 객체 동일성과 객체의 키 순서 무관/배열 순서 유지 비교는 실제 equals와 일치합니다. 명시 pair stack과 이미 방문한 pair 집합을 사용하고 대기 pair도 전체 비교 작업 예산에 포함합니다. AST→value 변환과 비교/폐기는 재귀 소유 구조를 만들지 않습니다. 노드/전체 key·value units·선행 AST node/진단/주석/깊이 예산을 checked 연산으로 거절하고 객체의 중복 위치와 비교 키 lookup은 borrowed BTreeMap으로 처리합니다. 무제한 새 할당/선형 중복키 반복이나 prototype 수정을 만들지 않았습니다.

원본 stringLength는 high surrogate 뒤의 다음 code unit이 low인지 확인하지 않고 건너뜁니다. 실제 getNodeValue/stringLength의 raw 결과를 유지하면서 Unicode scalar 길이는 actual JS Array.from의 iterator 결과로 별도 기대값을 생성했습니다. native는 valid high+low일 때만 두 단위를 한 scalar로 계산하며 단독 high 뒤의 일반 문자·다른 high를 보존합니다. 명시 교정4사례이고 논리 값 자체는 변경하지 않습니다. 원본12문자1~3조합과 structured prefix/불완전/주석/escape·숫자/키 순서에서2,348값 사례, 실제 equals의24×24+동일 객체24=600비교 사례를 생성했습니다. native 값으로 fixture를 만들지 않습니다.

first.log는19542 handle의 lib 컴파일에서 E0282/members[position] 타입 추론 실패·exit101입니다. BTreeMap의 borrowed key/usize 위치를 명시한 inference.log는41244 handle에서5.06초·exit0입니다. parity.log는36574 handle에서 compile10.36초·3통과/0.02초·exit0이며, 선행 AST의 자체 node/depth 예산도 확인하는 변경 뒤 budget.log는27866 handle에서 compile9.15초·현재3통과/0.02초·exit0입니다. 이전3건과 중복 합산하지 않습니다. 원본 값/논리문자·JS 키 순서/중복·deep equality/동일 객체의NaN 구분과8,192중첩의 생성/별도 arena 비교/폐기·조회/노드·문자·pair·AST 예산·무값0 budget을 확인했습니다.

source JSONSchemaService는 실제 URI normalize/relative path·연관 glob·외부 및 local pointer/anchor·참조 cache/의존 무효화와 draft meta schema를 사용하므로 값 변환만으로 해당 기능을 닫지 않습니다. NativeCore find는 기존 FindPatternCompiler 포트, 실제 App은 이미 taide_native_syntax::MonacoFindPatternCompiler를 쓰는 것을 확인했습니다. 이후 schema pattern/glob은 이 기존 능력과 실제 Source API/flags를 대조하며 새 의존성을 임의로 추가하지 않습니다. raw schema load는 JSONC parse와 LanguageService AST getNodeValue의 불완전 숫자/prototype 의미가 다름도 실제 source로 확인했으며 loader 단위에서 별도 보존/교정합니다.

소유Rust3/MJS format·diff/라이선스와 manifest/lock/동결 변경0을 확인했습니다. host.log/lib+json-value test8.74초와 wasm.log/corelib4.77초는26072 handle에서 exit0·추가경고0으로 종료했고 같은 성공은 재실행하지 않습니다. 기존Core366+신규3의369서로 다른 성공/성능1ignored입니다. b3e2 값 기반만 닫고 부모b3e의 URI/schema 선택/참조/anchor/의미검증·실제 App/조회/완성/colors·CSS/HTML/TS와 전체 App 준비/remoteWasm·OS/성능/전환/최종33 게이트는 미완료로 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3e3 — local JSON schema 참조·anchor

실제 JSONSchemaService.resolveSchemaContent/traverseNodes 전체와 JSON pointer/merge/anchor·메타 feature 진단을 대조했습니다. json-schema-refs.rs는 기존 value arena의 실제 node ID를 사용해 known single/map/array/items schema만 순회하고 $ref를 제거·반복/순환을 정리합니다. $id/id를 제외한 section 속성이 target sibling을 덮는 원본 merge 우선순위·JS 키 순서·공유 하위 object/array·순환 graph·anchor의 $id/id/#/fallback·중복 anchor의 원본 순서·지연 anchor index와 unsupported feature 순서를 보존합니다. schema 값을 외부에 가변으로 노출하지 않고 같은 crate의 resolver만 다루며 기존 키 정렬은 동일한 helper로 추출해 두 실제 사용에서 재사용했습니다.

[RFC 6901](https://www.rfc-editor.org/info/rfc6901/)의 pointer 문법/평가/URI fragment 표기를 확인했습니다. percent UTF-8은 한 번만 해제하고 ~1/~0의 의미와 배열의 선행0 금지·원문 Unicode 이름/빈 키/NUL을 구분합니다. 원본은 false section을 없는 것으로 취급하거나 배열 length·상속 property를 pointer로 읽고, anchor의 percent 인코딩을 해제하지 못하며 잘못된 ~ escape를 허용합니다. section.hasOwnProperty가 자료에 가려지면 예외가 발생하고 non-string $ref도 전체 Promise를 거절합니다. raw 원본 결과/예외를 보존한 뒤 실제 source의 정확한 marker9곳만 고친 모듈로 교정 기대값을 얻었습니다. false schema는 target의 not:{}로 보존하고 잘못된 참조는 bounded 진단으로 반환합니다. native 결과로 fixture를 생성하지 않습니다. RFC 코드나 예제를 복사하지 않았습니다.

104 actual resolver 사례에서 local pointer/anchor·참조 chain/동일참조/순환·공유/metadata·known single10/map6/array5/items 순회·중복/unsupported·draft03 거절/draft07/빈 draft를 확인했습니다. 20종 이름의 ~ / %·결합/비결합 Unicode·비BMP·공백/#/숫자/예약 이름/NUL과 percent anchor도 포함합니다. graph canonical은 object/array의 실제 공유 identity만 ref로 기록하고 primitive는 값으로 비교합니다. 명시 교정21사례이며 raw 예외도 그대로 기록했습니다.

외부 $ref는 source split 계약의 실제 target node ID·URI와 선택적 논리 fragment로 분리하고 기존 sibling을 보존합니다. 이것은 URI normalize/load/cache/외부 해결의 완료가 아니며 후속 서비스가 소비할 대기 항목입니다. 이 단계에서 네트워크/파일 요청·JS 런타임·새 의존성·manifest/lock/동결을 추가하지 않았습니다.

전체 작업은 방문/대기 node·pointer segment/lookup·merge의 target/source member 처리에 예산을 적용합니다. working ref/anchor/복사 key/draft·message·merge 횟수·value node 제한을 검사하고 부분 결과 없이 typed Capacity로 거절합니다. pointer의 Capacity를 미해결 참조로 삼키지 않으며 반복참조 set/방문 set·평면 graph로 종료를 보장합니다. false schema 변환의 새 node도 같은 node 예산에 포함합니다. error suffix는 URI를 포함한 총 메시지 크기를 먼저 검사한 뒤 할당합니다.

first.log는72555 handle에서 lib E0106/string helper의 lifetime 누락·exit101입니다. 실제 format과 다른 patch1회는 적용되지 않았으며 lifetime과 bounded child 수집을 고친 lifetime.log는63981 handle에서4.87초·exit0입니다. parity.log는61938 handle에서 compile10.68초·2통과/비교1실패·0.01초·exit101입니다. 차이는 빈 $schema를 None이 아닌 Some(empty)로 둔1사례였으며 원본 truthy 조건에 맞춰 수정했습니다. draft.log는64971 handle에서 compile9.24초·3통과/0.01초·exit0의65사례입니다. Unicode/escape 이름 보강 뒤 unicode.log는19035 handle에서 compile4.45초·104사례/3통과/0.01초·exit0입니다. URI 메시지 사전 예산/false 새 node와 pointer Capacity 전파를 추가한 final.log는6710 handle에서 compile8.92초·현재3통과/0.01초·exit0입니다. 이전3건과 반복 합산하지 않습니다. 깊이1,024·merge/탐색/문자/메시지/노드/무추가작업 예산과 실제 외부 대기 항목의 소유를 확인했습니다.

소유Rust4/MJS format·MIT/소유diff와 manifest/lock/동결 변경0을 확인했습니다. json-value의 기존3성공은 같은 키 정렬/읽기 API를 재사용하며 현재 변경은 helper 추출과 crate 가시성뿐입니다. 직렬 host.log/lib+json-schema-refs test15.78초와 wasm.log/corelib5.01초의 Finished 로그를 확인했습니다. 재개 시 handle19160은 이미 종료되어 재조회할 수 없었으므로 회수하지 못한 tool exit값은 별도로 단정하지 않습니다. 두 compiler의 완료 로그와 현재3검사 exit0을 검증 근거로 재사용하며 기존Core369+신규3의372서로 다른 성공/성능1ignored입니다. local 참조·anchor만 닫고 URI normalize/선택/glob·외부 요청/참조/cache/의존 무효화·의미검증·실제 App/나머지 JSON·CSS/HTML/TS 및 전체 App 준비/remoteWasm·OS/성능/전환/최종33 게이트는 부모b3e/전체 목표에 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3e4 — JSON schema URI 정규화

설치 JSONSchemaService의 normalizeId/normalizeResourceForMatching과 실제 번들 vscode-uri parse/toString(true)를 확인했습니다. Microsoft의 [vscode-uri 원본](https://github.com/microsoft/vscode-uri/blob/main/src/uri.ts)도 대조했습니다. 목표는 schema ID/연관 resource의 실제 정규화 계약이며 일반 URL 보안 검증·상대 context resolution·파일/네트워크 요청 기능으로 확대하지 않습니다.

json-schema-uri.rs는 논리 UTF-16 입력/출력을 그대로 유지합니다. 생략된 scheme의 file fallback·file/http/https path 보정·authority/userinfo/host lowercase/port·drive lowercase·query/fragment와 최소 percent escaping을 portable core로 옮겼습니다. schema ID의 비어 있는 trailing #를 제거하고 resource matching에서는 query/fragment를 제거합니다. 원본 parse가 거절하는 scheme/path는 원문을 유지합니다. 단독 surrogate·비BMP·Unicode/Greek/Turkish 호스트 변환과 인코딩된 #/?·원문 역슬래시·dot segment를 확인했습니다. Unicode 정규화나 상대 path 해석을 이 단계에서 추가하지 않습니다.

원본 percentDecode는 %와 두 ASCII alphanumeric의 연속 run을 잡고 decodeURIComponent 실패 시 첫 triple을 그대로 둔 뒤 재귀 호출합니다. 앞쪽 정상 byte도 뒤의 오류 때문에 보존되는 실제 동작과 마지막 유효 UTF-8 suffix를 유지하면서 Rust는 반복 UTF-8 경계 검사로 처리합니다. malformed percent의 깊이에 비례한 호출 스택/2차 문자열 재복사를 만들지 않습니다. 입력 단위 상한으로 중간 자료를 제한하고 출력 단위/전체 작업 예산은 checked 연산으로 부분 결과 없이 Capacity를 반환합니다. 이 정규화 결과 자체는 IO 인가나 허용된 URL 검증을 대신하지 않습니다.

oracle은 실제 schema source의 helper 두 marker를 확인하고 상대 import 경로만 actual file URL로 바꿔 helper를 export했습니다. 실제 vscode-uri와 schema adapter의 결과도 대조합니다. source 기능 교정0이며 native 값으로 기대값을 생성하지 않습니다. 최초 5,461사례에서 7,100으로 보강했으며 scheme/path/query/fragment 조합, percent14종의 3중 조합, logical surrogate·인코딩 userinfo/host/port·개행 경계를 포함합니다. fixture는 큰 UTF-16 배열 literal의 컴파일 비용을 피하고 동일 논리 units를 hex 문자열로 기록합니다. oracle의 Set/배열 누적 mutation은 수천 비교 자료 생성 비용에 한정하며 제품 상태를 변경하지 않습니다.

first.log는82509 handle에서 fixture의 &[u16] 필드에 배열 값 대신 참조 표기를 빠뜨린 E0308 16,383개·exit101입니다. 이는 제품 정규화 오류가 아닌 생성 자료 표현 오류입니다. 같은 기대값을 hex 표현으로 수정한 fixture.log는37920 handle에서 compile4.28초·5,461사례/3통과·0.05초·exit0입니다. authority/개행 보강 후 authority.log는50744 handle에서 compile4.28초·현재7,100사례/3통과·0.06초·exit0입니다. 이전3건과 반복 합산하지 않습니다. 별도 검사에서32,768회 malformed percent 뒤의 유효 suffix와 입력/출력/작업 상한·정확한 limit 수락·빈 입력을 확인했습니다.

직렬 host.log/lib+json-schema-uri test15.27초와 wasm.log/corelib5.10초는27976 handle에서 exit0·새 경고0입니다. 소유Rust4/MJS format·MIT/소유diff를 확인했습니다. lib.rs의 원본 module 순서를 유지하기 위해 reorder_modules=false 검사만 적용했고 manifest/lock/동결 변경0입니다. 기존Core372+신규3의375서로 다른 성공/성능1ignored이며 실제 App 전체 준비실패/remoteWasm·OS/성능/전환/최종33 게이트는 유지합니다. URI normalize만 닫고 relative context/연관 glob·schema 선택/외부 요청·참조/cache/의존 무효화·의미검증·실제 App와 나머지 JSON/CSS/HTML/TS는 부모b3e/전체 목표에 미완료로 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3e5 — 상대 schema context와 문서 속성 선택

실제 JSONWorker 생성자의 workspaceContext와 resolvePath/joinPath/normalizePath, JSONSchemaService.getSchemaFromProperty/getSchemaURIsForResource, JSONValidation.getLanguageStatus를 확인했습니다. resource의 원문 마지막 slash까지를 base로 삼고 absolute path는 교체·relative는 이어 붙이며 빈/dot segment 제거·부모 segment 회수·마지막 slash 보존을 구현했습니다. URI.parse된 base path와 raw relative path를 구분하고 URI.with/toString의 기본 encoding을 적용합니다. 이는 기본 URL 상대 해석과 다르며 fragment처럼 보이는 raw relative 문자도 원본처럼 경로 문자로 encoding합니다.

기존 URI 파싱/format을 두 실제 사용을 위해 Parts·명시 formatter로 공유했습니다. normalizeId/resource의 최소 encoding 결과는 유지하고 context의 기본 encoding은 UTF-8 percent·허용 unreserved/path slash·authority bracket/colon·userinfo password와 raw port suffix를 원본에 맞춥니다. lone surrogate 때문에 원본 URI encoding이 throw하는 경계는 손실 문자로 바꾸지 않고 InvalidBoundary로 반환합니다. 입력 두 문자열의 합·경로 segment 처리·middle/final 출력·전체 작업 예산을 checked 연산으로 확인하고 재귀 normalize나 raw 문자열의 반복 복사를 만들지 않습니다. URI 정규화는 IO 허용/프로젝트 인가를 대신하지 않습니다.

schema_property는 실제 기존 JSON AST의 root object에서 첫 $schema string을 선택합니다. 누락/비문자열을 건너뛰고 duplicate의 첫 유효 문자열·불완전/댓글 문서·nested 제외·빈 문자열의 실제 context URI·원본 ASCII has-scheme 조건을 보존합니다. 새 JSON.parse/last-key value 자료로 첫 속성을 바꾸지 않습니다. 선행 AST의 잘못된 node ID는 typed InvalidBoundary로 거절하고, 실제 빈 object/no-schema는 불필요한0-output 작업을 만들지 않습니다. 이 helper와 resolve_relative_path는 후속 선택/외부 참조 서비스가 소비하는 portable 경계이며 실제 App 연결이나 fileMatch association 완료를 뜻하지 않습니다.

oracle은 설치 JSONWorker 소스의 path helper3 marker를 검증·actual 상대 import 경로만 바꾸고 원본 resolvePath를 export했습니다. 실제 JSONWorker를 schemaRequest=false로 생성한 뒤 실제 parser/getLanguageStatus에서 속성 선택 기대값을 얻습니다. actual bundled meta schema는 worker에 그대로 있으며 요청/파일/네트워크 IO를 하지 않습니다. native 결과나 손으로 쓴 resolver로 기대값을 생성하지 않았습니다. actual source context384·property768=1,152사례/교정0이며 URI/authority/drive/query/fragment/rawpercent·Unicode/단독surrogate/invalidscheme·부모/중복/불완전/비문자열을 대조합니다. fixture 생성의 Set/누적 배열 mutation은 원본 비교 자료 생성 비용에 한정합니다.

first.log/lib은28772 handle에서5.15초·exit0입니다. parity.log는64774 handle에서 compile11.10초·context3통과/0.01초와 기존URI3통과/0.06초·exit0입니다. 새 검사 이름의 대문자 URI 때문에 snake-case 경고1을 발견해 이름을 수정했습니다. 잘못된 AST root 거절과 실제 경계 단언을 보강한 boundary.log는1812 handle에서 compile13.92초·현재context3통과/0.01초·exit0/새경고0입니다. URI의 같은7,100사례·deep percent/예산 성공은 관련 경로 구현이 바뀌지 않아 재사용하며 반복 합산하지 않습니다. 깊이8,192 parent path·전체 입력/출력/작업 예산·empty/no-output·잘못된 root를 확인했습니다.

직렬 host.log/lib+context/URI test11.15초와 wasm.log/corelib5.15초는12015 handle에서 exit0·새경고0입니다. 소유Rust3/MJS format·MIT/소유diff와 manifest/lock/동결 변경0을 확인했습니다. 기존Core375+신규3의378서로 다른 성공/성능1ignored입니다. context/명시 속성만 닫고 연관 glob·전체 선택/외부 요청·참조/cache/의존 무효화·의미검증·실제 App/나머지 JSON·CSS/HTML/TS와 전체 App 준비/remoteWasm·OS/성능/전환/최종33 게이트는 부모b3e/전체 목표에 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3e6 — JSON schema 파일 연관 패턴

실제 FilePatternAssociation·createRegex 전체를 읽고 등록/contribution의 ID normalize·folder resource normalize·마지막 match가 결정하는 include/exclude와 invalid 패턴의 전체 해제 계약을 대조했습니다. json-schema-associations.rs는 원본 prefix **/·선행 !/slash·empty·extended/group/class/escape·globstar segment 조건·순서/폴더 경계와 normalized schema URI를 portable Core로 구현합니다. 여러 연관의 dedupe/registry/cache나 문서 schema 선택 전체를 이 단위에서 닫지 않습니다.

Core의 Pattern/PatternCompiler 포트는 실제 소유한 논리 UTF-16 자료만 사용합니다. Syntax의 JsonSchemaPatternCompiler는 기존 regress0.12/utf16 feature를 재사용하고 새 의존성/manifest/lock을 만들지 않습니다. 실제 source glob은 Unicode flag 없는 legacy RegExp입니다. regress의 find_from_utf16은 flag와 별도로 surrogate pair를 합치는 것을 실제 구현에서 확인했으므로 find_from_ucs2·code-unit iterator/empty flags로 원본을 보존합니다. ?/??·단독/짝 surrogate·원문 비BMP pattern을 실제 엔진에서 확인했고 기존 FindPatternCompiler의 u flag나 손실 String 변환으로 대체하지 않았습니다.

실제 schema source의 FilePatternAssociation/URI helper3 marker를 확인하고 import만 remap·class/helper를 export한 oracle로 7,728연관 사례를 생성했습니다. glob 소스의 RegExp 생성 marker1곳을 반환 source 노출로 바꿔 44변환 기대값을 기록하고 원본 createRegex의 실제 컴파일 가능 여부도 보존했습니다. 교정0이며 native로 기대값을 만들지 않습니다. 폴더7종·resource24종·pattern 묶음46종의 ordered include/exclude·invalid whole reset·대소문자/URI query/fragment·group/star/class/escape·문자 경계를 확인했습니다. source fixture 생성의 누적 배열/Set은 원본 비교 자료 생성 비용에 한정합니다.

입력 pattern 수/전체units·합산 regex출력/작업·URI 수/합산units·정규화 입력/출력·매칭 대상과 pattern 수의 외부 처리 예산은 typed Capacity로 거절합니다. invalid pattern만 전체 해제하고 Capacity는 삼키지 않습니다. Core 예산 검사의 인라인 matcher는 해당 메모리/순서 계약만 검증하며, 실제 match/compile 계약은 별도 Syntax consumer에서 검사합니다. regex 엔진 내부의 backtracking step/큰 tier 성능은 외부 work 계산으로 완료했다고 주장하지 않습니다. 실제 앱의 사용자 pattern 공급에 연결할 때 반복 wildcard/backreference·큰 문서와 nesting/cancellation 성능을 확인해야 하며 전체 성능 게이트에 미완료로 남깁니다.

first.log/Syntax lib은77210 handle에서1분10초·exit0입니다. 직렬 core.log는62683 handle에서 compile10.68초·Core2통과/0.00초·새 검사 이름 snake-case 경고1입니다. 대문자 URI를 소문자로 바꾸고 같은 64값을 명명 상수로 옮겼으며 같은 검사 입력/동작을 재사용합니다. format context가 달랐던 patch1회는 적용되지 않아 실제 파일을 읽고 정확한 위치로 적용했습니다. syntax.log는 같은62683 handle에서 compile27.17초·실제source 연관/legacy compile2통과/0.21초·exit0입니다. 기존Core378+신규2의380서로 다른 성공/성능1ignored와 신규 Syntax2를 구분하며 반복 합산하지 않습니다.

host-core.log/lib+association test9.92초·host-syntax.log/lib+schema-pattern test4.85초와 wasm.log/Syntax lib+Core dependency2분45초는66600 handle에서 exit0·새경고0입니다. 중간 ps 조회는 sandbox에서 operation not permitted였으나 같은 live handle/변경된 build 로그로 계속 관찰해 정상 종료를 확인했습니다. 관찰 timeout 때문에 재시작하지 않았습니다. 소유Rust7/MJS format·MIT/소유diff·manifest/lock 변경0을 확인했습니다. file association만 닫고 실제 registry/캐시/외부 요청·참조/semantic/App·나머지 JSON/CSS/HTML/TS 및 전체 App 준비/remoteWasm·OS/성능/전환/최종33 게이트는 부모b3e/전체 목표에 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3e7 — schema 등록·선택·캐시 수명

실제 JSONSchemaService/SchemaHandle의 contribution/외부 등록·같은 ID 교체/콘텐츠 없는 기존 handle 재사용·등록 순서/필터·clear 복원과 explicit/fileMatch/combined 선택을 대조했습니다. Service는 URI/AST/association/value 기반을 실제 typed handle에 연결하고, source의 생성 URI schemaservice://combinedSchema/와 encodeURIComponent·allOf/$ref 순서를 보존합니다. 등록은 staged quota 검사 후 적용하며 같은 ID 교체에서는 사용하지 않는 handle을 회수합니다. contribution이 외부 덮어쓰기 뒤에도 원래 handle/cache로 복원되는 계약을 유지합니다. combined 선택의 실제 내부 handle을 정규화된 다른 URI로 놓치지 않고 재사용합니다.

unresolved/resolved 캐시는 실제 Arc 자료를 재사용하고 같은 요청은 Start/Pending ticket으로 공유합니다. ticket은 서비스마다 실제 Arc 소유·handle·generation·phase를 검증하므로 다른 서비스/교체/변경 뒤 응답이나 이미 완료한 요청을 받지 않습니다. source의 load/resolve promise 단계를 typed 상태로 분리했고, load가 완료된 유효 object/boolean schema만 resolved 캐시에 게시합니다. 미해결 external 항목은 완료 cache로 게시하지 않습니다. 알려진 의존 URI 변경은 transitive closure에서 관련 unresolved/resolved/dependency/대기 상태를 회수하고 단일 resource 캐시는 항상 만료합니다. Core 의존 검사에 넣은 metadata는 callback 수명 포트의 소유 fixture이며 실제 외부 참조 해결 완료 근거로 확대하지 않습니다.

resolve_arena는 actual cached arena를 직접 사용하도록 기존 local resolver를 두 실제 사용에서 공유합니다. 선행 value node/text 예산을 재확인하고 schemaDraft는 실제 URI normalize로 보존합니다. URI는 실제 encodeURIComponent의 추가 punctuation 계약과 scheme 조회를 제공하며 기존 URI/context/refs 9검사 성공을 재사용합니다. 최대 handle/등록/연관·전체 schema values/text·URI/합성 값/작업 예산을 체크하고 quota 실패 시 기존 registry/cache를 보존합니다. registered URI alias를 누락한 quota 실패를 실제 재현한 뒤 추가 저장 비용을 검사하도록 수정했습니다.

oracle은 실제 schema source와 실제 JSONWorker의 constructor workspaceContext·원본 parser를 사용합니다. 6scenario/61event에서 contribution/외부 교체·restore·등록 ID 순서/내부 scheme 제외·중복 association dedupe·folder·명시 root schema 우선·실제 allOf factory·소유 request callback의 load/resolve 재사용/변경을 대조합니다. combined은 actual createCombinedSchema/getUnresolvedSchema factory 계약을 대조하며 전체 외부 resolve를 실행했다고 쓰지 않습니다. 기존 getSchemaForResource 전체·raw loader/HTTP/file 및 실제 App 소비는 미완료입니다. 실제 fixture request callback은 소유 JSON 문자열만 반환하고 외부 IO를 하지 않습니다. generator의 누적 배열·계수 mutation은 이 원본 대응 자료 생성에 한정합니다.

원본의 config.schema truthy와 SchemaHandle constructor truthy는 유효 false schema를 잃고 request callback으로 대체합니다. raw 결과를 보존하고 actual source의 정확한 marker2곳을 undefined 여부로 교정한 기대값을 사용했습니다. 원본 교정6event이며 native로 기대값을 생성하지 않습니다. Native 등록/게시 경계는 object/boolean만 받아 잘못된 schema 모양을 거절합니다.

first.log/lib은73798 handle에서4.91초·exit0입니다. core.log는36699 handle에서 compile13.03초·service3통과/0.00초 및 기존context3/0.01·refs3/0.01·URI3/0.06의12통과·exit0입니다. syntax.log는89076 handle에서 compile15.24초·actual lifecycle/명시 우선2통과/0.00초·exit0입니다. quota-repro.log는27568 handle에서 compile8.15초·새 strict alias 단언1실패/0.00초·exit101이며 실제 Ok(())/예상Capacity로 누락을 확인했습니다. alias 비용과 로딩/잘못된 모양 게시 거절을 수정한 직렬 final-core.log는42421 handle에서 compile9.56초·현재Core3통과/0.00초, final-syntax.log는compile23.64초·현재Syntax2통과/0.00초·exit0입니다. 이전 성공/회귀를 반복 합산하지 않습니다. 기존Core380+신규3의383서로 다른 성공/성능1ignored와 JSON currentCore40/Syntax4를 구분합니다.

contribution-repro.log는60168 handle에서 compile4.52초·상한0의 contribution 등록1실패/0.00초·exit101입니다. clear_external이 복원할 등록 수/URI alias도 등록 시 예약해야 하는 별도 경로를 실제 Ok(())/예상Capacity로 확인했습니다. 현재 등록 수와 미래 복원 수의 큰 값 및 아직 등록 목록에 없는 contribution URI 저장분을 함께 검사하도록 수정했습니다. restored-core.log는38104 handle에서 compile18.84초·현재Core3통과/0.00초, restored-syntax.log는compile23.71초·현재Syntax2통과/0.00초·exit0입니다. 이후 restored-host-core.log18.59초·restored-host-syntax.log13.46초·restored-wasm.log9.27초는18418 handle에서 exit0·새경고0입니다. 앞선 host/Wasm 결과는 이전 변경 상태의 이력으로 보존하고 최신 근거는 이 restored 로그입니다. 같은3/2성공을 중복 합산하지 않습니다.

소유Rust8/MJS format·MIT/diff와 manifest/lock/동결 변경0을 확인했습니다. host-core.log/lib+service/URI/context/refs test11.27초·host-syntax.log/lib+schema-service test13.82초와 wasm.log/Syntax lib+Core dependency10.20초는99664 handle에서 exit0·새경고0입니다. registry/선택/캐시 수명 단위만 닫고 actual raw schema load/외부 graph/HTTP/file·semantic·조회/완성/실제 App·CSS/HTML/TS와 전체 App 준비/remoteWasm·OS/regex 성능/전환/최종33 게이트는 부모b3e/전체 목표에 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3e8 — schema 응답 decode와 JSONC 값 복구

실제 loadSchema의 no-request/content/error·json-schema.org HTTPS·BOM trim·첫 JSONC parse offset와 URI fsPath 표시를 확인했습니다. 문서용 AST는 unquoted key/숫자/진단 복구 계약이 달라 대신 사용하지 않습니다. 기존 scanner와 value arena·키 정렬을 재사용한 명시 stack JSONC 파서는 실제 parse visitor의 객체/배열/last key·누락 콜론·쉼표·unknown/trivia 복구·숫자 Number/NaN→0·첫 token offset을 보존합니다. 반환 graph는 평면 node ID이며 깊은 생성/폐기에 재귀 소유 구조를 만들지 않습니다.

decode는 요청 서비스 없음/빈 본문/실패 문자열·Error: 분리/끝 period·BOM 경고 뒤 parse 오류 순서와 URI 표시를 bounded UTF-16 메시지로 반환합니다. http://json-schema.org/ 요청 URI는 원본대로 https로 바꾸고 no-request 오류에서는 원래 URI를 유지합니다. 플랫폼 선택은 Core 전역 OS 대신 인자로 전달하며 drive/UNC·percent·Unicode·비file URI 표시를 actual vscode-uri POSIX/Windows18사례로 확인합니다. Windows 기대값은 실제 번들 URI source의 플랫폼 selector1곳만 바꾼 소유 copy에서 얻었고 OS 설정/process.platform을 변경하지 않았습니다.

원본 raw JSONC 객체는 **proto** 대입으로 자료가 prototype으로 빠집니다. raw 결과를 보존하고 실제 parse의 객체 생성 marker1곳을 null-prototype으로 바꾼 기대값과 대조합니다. 2,316 raw 사례·명시교정57입니다. 객체/boolean 이외 root는 [JSON Schema core](https://json-schema.org/draft/2020-12/json-schema-core)의 schema 자료형 조건에 맞춰 오류와 빈 object로 돌려 후속 resolver 예외를 막습니다. actual loader source의 반환 marker1곳에만 이 root 확인을 추가한 기대값을 사용했고 276load 사례·교정114입니다. parse 오류가 이미 있으면 추가 shape 오류를 중복하지 않습니다. native로 기대값을 생성하지 않습니다.

원본 oracle은 실제 schema class/JSONC parse/URI를 실행합니다. 정상/불완전 prefix·key 순서/중복·예약 이름·비BMP/단독 surrogate escape·Number bits/negative0/infinity·comment/trailing comma와 parse offset을 포함합니다. generator에서 display 배열을 URIS/units 초기화보다 앞에 둔1회는 ReferenceError로 실패했습니다. 실제 선언 순서로 옮긴 뒤 set-e로 전체 재생성을 확인했으며 실패 때 남은 이전 fixture를 최종 근거로 쓰지 않았습니다. 누적 배열/Set mutation은 이 원본 비교 생성 비용에 한정합니다.

첫 lib first.log는74386 handle에서4.82초·exit0입니다. parity.log는88068 handle에서 compile10.82초·loader3통과/0.01초와 기존URI3통과/0.06초·exit0입니다. 새 검사 이름 대문자 JSONC의 snake-case 경고1을 소문자로 수정했습니다. 실제 cache publish_response 경계를 추가한 cache.log는56921 handle에서 compile9.51초·현재4통과/0.01초·exit0/새경고0입니다. 실제 Start ticket→decode→Ready Arc/오류/동일 문서 재사용·변경 뒤 이전 load/resolve 응답 거절·fresh false schema 게시를 확인했습니다. 단순 local resolve에서 BOM 등 load errors가 자동 전파되었다고 주장하지 않으며 전체 graph resolver의 unresolved error 전파는 후속 단위입니다.

입력 byte/token/value units·논리 key/string 값·value 수·깊이/전체 작업·URI·메시지 수/합산 units는 Capacity로 거절하고 부분 cache 게시를 하지 않습니다. 8,192중첩·각 quota/0-error budget·no-request의 정확한 URI limit을 확인합니다. 소유 callback은 문자열만 반환하며 실제 파일/HTTP/네트워크 요청을 하지 않습니다. publish_response는 Service의 실제 owner/generation ticket을 확인하고 decode 결과를 기존 cache에 게시하는 제품 경로입니다.

소유Rust5/MJS format·MIT/소유diff와 manifest/lock/동결 변경0을 확인했습니다. Core host.log/lib+json-schema-load test8.92초와 wasm.log/lib5.10초는47165 handle에서 exit0·새경고0입니다. 응답 decode 단위만 닫습니다. 기존Core383+신규4의387서로 다른 성공/성능1ignored·JSON currentCore44/Syntax4와 기존URI3 회귀를 구분합니다. 실제 HTTP/file adapter·외부 graph/semantic·조회/완성/실제 App·CSS/HTML/TS와 전체 App 준비/remoteWasm·OS/regex 성능/전환/최종33 게이트는 부모b3e/전체 목표에 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3e9 — 외부 schema graph·로딩 오류·원자적 캐시 게시

실제 resolveRefs/resolveExternalLink의 Promise continuation·공유 unresolved 객체·순회/local 문맥·URI 정규화·의존 집합과 로딩 오류/anchor/unsupported 순서를 대조했습니다. json-schema-graph.rs는 실제 Service의 root resolve/load ticket으로 시작하는 재개 가능한 작업입니다. Start/Pending/Ready를 재사용하고 새 요청만 Fetch로 내보내며, 회신 전에는 같은 작업을 Waiting으로 보존합니다. 실제 HTTP/file 전송을 대신 구현한 것으로 쓰지 않습니다. 등록된 schema만으로 끝나는 작업은 요청 없이 실제 cache까지 연결됩니다.

각 문서의 값 arena는 한 번만 전체 예산 검사 후 평면 node ID로 import합니다. 원래 공유/순환 object·array identity를 유지하고 local pointer/anchor는 참조를 가져온 parent document의 root/URI/anchor cache에서 해결합니다. 본문/root 로딩 오류는 먼저 보존하고 외부 오류는 원본처럼 첫 메시지와 fragment를 붙여 전달합니다. draft03의 원본 조기 거절과 unsupported 순서도 유지합니다. source의 ID/root object/boolean·known schema fields 계약을 사용하며 임의 JavaScript 실행·새 의존성을 넣지 않았습니다.

실제 Source는 root A가 B→C→B 간접 순환 바깥에 있을 때 같은 target의 외부 참조를 반복해 끝나지 않습니다. oracle의 raw source subclass는512 traverse를 넘으면 oracle-cycle-budget으로 종료시켜 실제 무한 처리 경로를 기록합니다. 이것을 source 정상 출력으로 쓰지 않습니다. 기존 local 참조 교정9곳에 더해 actual 외부 참조 guard2 marker만 수정한 모듈에서 expected를 생성했습니다. 같은 대상/URI/fragment 반복은 다시 merge하지 않고 의존 관계는 남기며 native도 동일하게 처리합니다. actual21사례·raw/교정 차이4이며 native로 기대값을 생성하지 않습니다. Source dependency suffix는 진단용 원본 metadata로 보존하고 native graph/오류/경고/draft를 직접 비교합니다. native의 실제 direct/transitive 의존은 별도의 캐시 만료/원자성 검사로 확인합니다.

Service의 load watch는 실제 owner/handle/generation을, resolve watch는 추가 sequence를 확인합니다. 해결 취소는 공유 load를 만료시키지 않으면서 이전 resolve observer를 거절합니다. 모든 문서 snapshot을 재개/게시 전에 재확인하고 문서 변경/교체/취소/Capacity에서는 root 작업을 회수합니다. 아직 외부로 내보내지 않은 소유 요청만 취소하므로 다른 작업의 이미 발송된 공유 load를 잘못 회수하지 않습니다. 완료된 모든 문서의 direct dependency와 root 결과는 일괄 quota 검증 뒤 원자적으로 게시합니다. 최종 게시 실패가 Root dependency 일부만 남기는 것을 실제 cache 검사에서 막습니다.

local resolver의 실제 traverse/merge/pointer/anchor·work/text/message 예산을 두 구현이 공유하도록 crate 범위로 추출했습니다. 실제 원본 module 교정과 JSONWorker workspaceContext helper는 각각 두 generator가 사용하도록 공유했고 기존 local104/서비스61event fixture는 재생성·rustfmt 뒤 cmp가 byte-for-byte exit0입니다. 추출 시 통계 출력의 replacements 범위를 놓친 ReferenceError1회와 format context가 다른 patch1회는 수정/재적용했으며 최종 기대값은 동일합니다. generator Set/Map/배열 mutation은 이 원본 graph/참조 대응 자료 생성 비용에 한정합니다.

first.log/lib은90797 handle에서4.54초·engine.log는77465 handle에서4.79초·exit0입니다. parity.log는52064 handle에서 compile13.34초·graph3/0.00초·기존load4/0.01·refs3/0.01·service3/0.00의13통과·exit0입니다. retired-repro.log는70615 handle에서1실패/0.00초·exit101이며 external 덮어쓰기→미발송 이전 요청 거절→contribution 복원 뒤 Pending이 남는 실제 실패입니다. 취소만 소유/generation/phase를 기준으로 숨겨진 이전 handle도 회수하도록 고쳤고 게시/읽기는 여전히 현재 active handle만 허용합니다.

수정 후 lifecycle.log는43129 handle에서 compile10.62초·현재graph5/0.00초·load4/0.01·service3/0.00의12통과·exit0입니다. consumer.log는 같은 handle에서 compile27.25초·실제 패턴/합성 선택→외부 graph→동일 Arc cache/의존 변경까지 포함한Syntax3통과/0.00초·exit0입니다. graph에는 값/문자/메시지·문서/요청/URI/작업 상한, 변경/이전 observer·미발송 복원, 실제 direct/transitive 의존 및 최종 quota 실패의 원자성 검사가 포함됩니다. 기존Core387+신규graph5의392서로 다른 성공/성능1ignored·current JSONCore49/Syntax5이며 기존 성공과 새 Syntax1을 중복 합산하지 않습니다.

host-core.log/lib+graph/refs/service/load test11.31초·host-syntax.log/lib+schema-service test13.94초·wasm.log/Syntax lib+Core dependency9.45초는17319 handle에서 exit0·새경고0입니다. 소유Rust7/MJS5 format·MIT/소유diff와 manifest/lock/동결 변경0을 확인했습니다. 외부 graph/캐시 단위만 닫고 실제 HTTP/file adapter·schema semantic/조회/완성/실제 App·CSS/HTML/TS와 전체 App 준비/remoteWasm·OS/regex 성능/전환/최종33 게이트는 부모b3e/전체 목표에 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3e10 — 실제 schema 전송·프로젝트 수명과 접근 경계

App editor-schema-transport.rs는 기존 AppServices/TaskSupervisor와 taide-infra HTTP Api client·root_guard, 기존 preview_web_file::Anchor를 재사용합니다. manifest/lock/새 의존성은 수정하지 않습니다. 실제 graph Fetch의 원래 canonical URI와 load ticket·실제 referrer를 보존하고 요청 URL의 json-schema.org HTTPS 교정은 기존 request_url을 사용합니다. enableSchemaRequest=false는 전송 없이 원본 Unavailable 응답을 decode합니다. 설정 활성화와 전체 App provider 연결은 후속 b3e/b3에 남습니다.

설치 JSONWorker의 defaultSchemaRequestService는 HTTP status를 검사하지 않고 response.text()를 읽으므로 Native도 404 등의 본문을 decode합니다. 임의 status를 네트워크 실패로 바꾸지 않습니다. 기존 reqwest 0.12.28 async_impl/response.rs의 content_length/chunk와 태스크 API 실제 소스를 읽었고, 길이 선언과 실제 수신 chunk 모두를 상한 검사합니다. UTF-8 오류는 fetch text처럼 replacement로 처리합니다. 요청 시간 제한과 활성/응답 대기 수 상한을 적용하고 완료된 payload도 Loading 소유자가 회수할 때까지 slot에 포함합니다. callback 결과는 owner/generation ticket을 검증한 뒤 한 번만 게시하고 decode Capacity·취소·종료/프로젝트 root 변경에서는 load를 회수합니다. Loading drop은 네트워크 작업을 취소하며 서비스 ticket 회수는 명시 cancel과 provider 소유자의 책임입니다.

파일 읽기는 실제 프로젝트 root와 원본 resource/referrer를 확인하고 경로 정규화·root 내부 여부를 검사합니다. Unix의 기존 Anchor는 directory descriptor/openat·NOFOLLOW/NONBLOCK으로 경로 교체·symlink/FIFO와 비정규 파일을 차단합니다. 이 기존 파일 경계를 재사용하며 해당 파일은 수정하지 않았습니다. non-Unix는 Anchor가 구현되지 않은 플랫폼 파일 요청을 명시 거절하며 완료로 확대하지 않습니다. 원격 HTTP schema에서 file 참조로 권한을 넓히지 않습니다. 실제 내부 combinedSchema handle에서 프로젝트 파일로 나가는 참조는 원래 resource의 프로젝트 권한을 사용합니다.

최초 tests.log는4735 handle·compile50.68초·5통과/1실패·검사0.04초·exit101입니다. root 밖 파일은 차단됐으나 localized Forbidden을 일반 오류로 변환하는 실패를 재현했고 AppError.kind로 원래 거절 분류를 보존했습니다. lifecycle.log는75349 handle·compile21.60초·8통과/0.03초·exit0이며, final.log는53659 handle·compile29.62초·10통과/0.03초·exit0입니다. 뒤의 10건은 기존 성공을 포함하므로 8/10을 합산하지 않습니다.

cache-repro.log는51035 handle·compile24.49초·0통과/1실패·0.04초·exit101입니다. 같은 Service에서 실제 로컬 schema를 먼저 읽어 둔 뒤 원격 schema가 해당 file URI를 참조하면 새 Fetch가 없어 전송 경계만으로는 차단되지 않았습니다. Core graph에 참조별 접근 검사를 추가하고 App Scope.advance가 캐시 import 전에도 확인하도록 수정했습니다. 기본 portable advance의 원본 계약은 허용 정책으로 유지하고 접근 검사 작업/URI·메시지도 기존 예산을 사용합니다. 이 수정의 최종 검사 결과는 아래에 기록합니다.

guard.log는89511 handle·compile34.09초·10통과/0.03초·exit0이며 실제 먼저 읽힌 로컬 cache의 원격 재사용을 차단했습니다. Scope.begin은 root의 Ready cache를 돌려주기 전에도 resource 권한을 검사하고 실제 combined handle ID는 원래 문자열로 Core에 전달합니다. owned.log는84807 handle·compile21.52초·10통과/0.03초·exit0입니다. 최종 retention.log는42520 handle·compile21.59초·현재10통과/0.03초·797filtered·exit0이며, 취소 직후 아직 실행 중인 task나 queued blocking 파일 worker의 slot도 실제 종료까지 보유하도록 Arc 소유를 연결했습니다. 같은 current-thread에서 시작 직후 취소→즉시 다음 요청 Capacity→실제 drain 뒤 재요청 성공을 확인했고, 단순 handle 개수만 세어 대기 작업을 초과시키지 않습니다. 앱 전체 807건을 실행했다는 뜻이 아닙니다.

core.log는70208 handle·compile26.02초·실제 oracle/공유·순환/예산/캐시원자성의 기존5통과/0.00초·exit0입니다. 변경되지 않은 기존 Core392의 성공은 재사용하고 신규 Core 건수로 합산하지 않습니다. 실제 현재 JSONCore49/Syntax5에 새 App10을 별도로 표시합니다. 모든 검사는 소유 임시 fixture와 자체 loopback 연결이며 사용자 데이터/외부 네트워크·OS 입력/클립보드·설정·키/시크릿·동결/manifest/lock 변경은 없습니다.

syntax.log는28351 handle·compile20.29초·실제 association/합성 선택/graph/cache 소비자3통과/0.00초·exit0입니다. 현재 변경 범위는 Core5/Syntax3/App10의 서로 다른18건이며, 이전 동일 검사의 성공을 중복 합산하지 않습니다. 전체 App 준비 실패·보호3제외/기존 remote Wasm Instant/OS·성능/최종 전환 게이트는 해소하지 않았습니다.

wasm.log는94142 handle·Syntax lib/Core dependency9.47초·exit0입니다. 소유 Rust4 format·소유 diff·manifest/lock/동결 변경0을 확인했습니다. App에는 vendor wry17경고와 큰 lib test binary의 compact unwind linker1경고가 출력됐으며 검사 exit0과 구분합니다. 이 단위는 전송 모듈과 실제 Core 소비를 닫으며 UI 프레임의 자동 builtin provider·기본 설정 연결/선행 semantic·조회/완성·CSS/HTML/TS는 부모b3e/b3에 남습니다. 기존 전체 App 준비 실패/remote Wasm·플랫폼·regex/큰tier 성능·최종 전환 게이트도 유지합니다. 전체303/588(51.5%)·미완료285·editor-57missing/editor-16partial·목표active·잔여시간미산정입니다.

## b3f — JSON 구조 공급자와 실제 NativeApplication

App editor-json.rs는 실제 DocumentId/DocumentSnapshot·파일 URI 또는 실제 TabId의 untitled URI를 사용해 기존 JSON 심볼·접기를 최대2개 blocking 작업으로 공급합니다. LSP owner/ProjectId를 생성하거나 서버 응답으로 위장하지 않습니다. 편집·언어/키·크기 등급·뷰 닫힘에서 이전 receiver/cancel owner를 회수하고 실제 worker가 끝날 때까지 slot을 유지합니다. 취소된 worker의 종료도 repaint로 대기 작업을 깨웁니다. Normal JSON의 실제 붙은 뷰만 처리하고 원본 register.js의 json 언어 선택을 그대로 사용합니다. jsonc/앱 설정 파일·다른 언어까지 임의 활성화하지 않습니다.

Core DocumentSymbols::from_symbols는 원래 byte 범위/selection/부모 순서·UTF-8 경계를 검사하고 기존 sticky 생성을 공유합니다. Native 모델을 LSP DTO/가짜 URI에 왕복하지 않습니다. 작업의 parser/scanner/깊이·노드/심볼/전체 문자·접기/재탐색 예산은 기존 Normal2MiB 기준과 명시 상수로 제한합니다. 최종 container 경로의 누적 표시 bytes도 제한해 깊은 경로의 제곱 크기 확장을 차단합니다. Unicode UTF-16 원본 label은 화면 String의 replacement 표시로 변환하고 원본 Core의 논리 값/좌표는 변경하지 않습니다.

editor-symbols는 별도 내장 슬롯과 실제 publish generation을 갖고 외부의 비어 있지 않은 팔레트를 우선하며 빈 외부 결과에는 내장 모델을 사용합니다. 팔레트 source가 바뀌면 이전 generation 위치 이동을 거절합니다. sticky는 기존 범위 점수와 내장 점수를 함께 비교하고 점수를 publish 때 계산해 매 프레임 전체 심볼을 다시 순회하지 않습니다. editor-folding은 외부/내장 범위를 publish 때 합성하고 같은 시작 줄은 외부 우선, 다른 native 범위는 유지합니다. 설치 languageFeatureRegistry의 score/builtin/time 정렬과 JSON register·원본 workers/Core oracle 근거를 읽었습니다. 내장 회수는 외부 캐시를 삭제하지 않습니다.

실제 앱 프레임은 LSP bridge 존재와 관계없이 구조 작업을 처리합니다. 기존 심볼 팔레트/outline/breadcrumb 위치·sticky와 본문 접기에 공급하고 peek의 별도 실제 snapshot에도 동일한 모델을 전달합니다. 설치 referencesWidget은 embedded editor에 minimap/scroll 옵션만 덮어쓰고 stickyScrollController는 현재 enabled 설정을 사용하므로 peek도 부모 sticky 설정을 상속하도록 기존 강제 false를 제거했습니다. 현재 peek snapshot의 모델을 전달해 본문 모델을 잘못 재사용하지 않습니다. shutdown/직접 on_exit에서 작업을 취소하고 기존 TaskSupervisor drain에 포함합니다. 읽기 전용 Normal은 구조를 제공하며 Large 이상은 작업/대기 표시를 중단합니다. 플랫폼/성능/전체 언어 서비스 완료를 뜻하지 않습니다.

first.log/16584는 검사 트랜잭션의 실제 필드명3오류·exit101·실행0건입니다. 실제 Transaction/Edit 계약을 읽고 수정했습니다. state.log/73171은 compile37.87초·2통과/1실패·0.03초·exit101이며 배열 원소0/1도 심볼인 원본 계약을 누락한 기대값입니다. current.log/91989는 compile22.81초·2통과/1실패·0.03초·exit101이며 기존 원본 접기가 닫는 줄을 제외하는 기대값을 잘못 적었습니다. 이 두 건은 제품 오류로 확대하지 않습니다. native.log/26533은 compile22.29초·영향1통과/0.00초·exit0이며 나머지 내용/닫힘·깊은 container/UTF-8 경계2건은 입력과 제품 경로가 바뀌지 않은 성공을 재사용합니다.

app.log/9699는 compile0.26초·실제 앱1통과/28.48초·exit0입니다. LSP bridge를 실제 disconnect/await한 뒤 JSON 파일을 로딩하고 Native 심볼 위치/sticky·접기 후 본문 숨김·텍스트 편집 후 새 generation/위치·디스크 원문 보존과 정상 종료 tasks0을 확인했습니다. coexist.log/42180은 compile22.55초·소유 typed provider 상태1통과/0.01초·exit0이며 외부 팔레트 우선/Native sticky·접기 합성/같은 줄 외부 우선·이전 팔레트 generation 거절/내장만 회수했을 때 외부 유지입니다. 이 상태 검사의 provider 식별자는 테스트 프로토콜 입력이며 제품 native runtime은 LSP owner를 만들지 않습니다.

peek.log/36573은 compile22.33초·0통과/1실패·37.64초·exit101입니다. fixture가 열린 대상의 언어를 retarget_file로 변경해 참조 추적 journal 밖 revision을 만들고 peek를 종료시킨 뒤 포커스 대상을 기대했습니다. retarget은 언어 변경 전용 포트가 아니므로 제품을 우회해 수정하지 않습니다. 실제 대상 파일을 읽은 JSON OpenedFile 모델을 peek 전에 준비하고 기존 실제 Rust child의 정의 응답으로 해당 문서를 열도록 fixture를 수정했습니다. 이후 최종 실제 앱/회귀 결과를 아래에 기록합니다.

app-current.log/28465는compile18.75초·본문/실제 child peek2통과/47.13초·exit0입니다. 본문은 언어 변경/읽기전용 Normal 재분석/Large 회수까지 확인했습니다. peek는 실제 파일을 읽은 JSON 문서 모델을 먼저 준비한 후 Rust child의 실제 정의 응답으로 열었습니다. 임의 좌표나 가짜 공급자/프로젝트로 앱 결과를 만들지 않습니다. 들여쓰기 없는 JSON의 F9 재지정 접기는 해당 peek만 변경하고 본문 revision/원문을 보존하며 닫힘 뒤 내장 결과와 tasks를 회수합니다. 최초 retarget fixture의 실패를 이 성공으로 지우지 않습니다.

설치 referencesWidget의 embedded editor는 부모 sticky 설정을 상속하고 stickyScrollController는 enabled 값을 사용합니다. Native의 peek 강제 false를 제거하고 현재 peek의 올바른 모델을 전달했습니다. peek-sticky.log/94671은compile21.04초·실제 앱1통과/38.53초·exit0입니다. 40개 속성의 들여쓰기 없는 대상에서 scroll10줄 뒤 실제 sticky-line 응답·현재 peek 모델/키 접기·본문 보존·닫힘/tasks0을 확인했습니다. 기존 본문/미리보기 성공을 새 검사 건수로 중복 합산하지 않습니다.

심볼 회귀 symbol-regression.log는직접exit0·compile0.25초·기존5통과/0.00초입니다. Core core.log/87391은compile10.25초·기존 document-symbols3통과/0.00초·exit0입니다. Core 검사 fixture의 deprecated2경고를 성공과 구분하며 신규 Core 검사를 추가한 것으로 합산하지 않습니다. 현재 새 상태4/실제 앱2와 기존 상태5/Core3의 서로 다른14건을 관련 근거로 사용합니다.

분석 Capacity가 terminal인데 팔레트가 계속 pending이 될 수 있는 경로는 현재 snapshot의 별도 실패 상태로 종료합니다. 빈 성공 모델을 만들어 오류를 감추지 않고 실제 에러를 기존 보고 경로로 전달합니다. 외부 공급자의 결과/진행은 유지하고 새 revision의 성공 게시가 실패 상태를 회수합니다. 실제 앱의 깊은 container 용량 실패→로딩 종료→Undo→정상 재분석 검사를 추가했으며 결과는 아래에 기록합니다.

recovery.log/37695는compile26.47초·0통과/1실패·38.99초·exit101입니다. 용량 실패의 terminal 상태와 로딩 종료는 확인했지만 기존 report가 status 대신 toast를 사용하는데 status를 기대해 실패했습니다. 실제 기존 report/Toasts 계약을 읽고 localized error toast를 검사하도록 기대값을 수정했습니다. 제품 오류 보고를 다른 경로로 우회하지 않았으며 Undo/정상 재분석의 최종 결과는 별도로 확인합니다.

recovered.log/65162는compile14.91초·실제 본문 앱1통과/38.15초·exit0입니다. 깊은 container 표시 용량 초과의 pending 종료/실제 localized toast·Undo/정상 내장 결과 복원·언어 변경/읽기전용 Normal/Large 게이팅·원문·정상 종료tasks0을 확인했습니다. 앱2는 이 현재 본문1과 앞의 실제 peek-sticky1이며 과거 동일 본문/peek 성공을 합산하지 않습니다. 새로운 failed snapshot은 빈 성공 모델 없이 terminal 상태를 표시하고 오래된 위치 이동·새 revision의 복구를 구분합니다.

wasm.log/32462는Syntax lib/Core dependency9.43초·exit0입니다. 소유 Rust11 format·소유 diff·manifest/lock/동결 변경0을 확인했습니다. 새 App 구조6건과 기존 상태5/Core3의 관련14건이며 기존 전송 App10/JSONCore49/Syntax5와 전체 Core392의 성공을 재사용할 뿐 새 전체 실행으로 합산하지 않습니다. 전체 App 준비 실패/보호3제외·remote Wasm tests의 시간 API·실제 OS/성능/신뢰성/출시·전환 게이트도 유지합니다.

실제 전역 팔레트와 outline는 기존에 파일 탭/경로만 선택하므로 untitled의 내장 모델 성공만으로 실제 untitled 탐색을 완료로 보지 않습니다. 본문/peek 단위를 b3f1로 닫고 부모 b3f 및 실제 untitled 탐색 b3f2는 PROCESS에 미완료로 남깁니다. JSON의 구조 공급이 실제 앱에 연결됐으므로 editor-57은 missing에서partial로 갱신하며 나머지 JSON 진단/조회/완성/포맷·다른 언어 전체를 완료로 올리지 않습니다. 전체303/588(51.5%)·부분93/미연결110/미구현82·미완료285·최종33·목표active·잔여시간미산정입니다.

## b3f2 — 실제 untitled 심볼 탐색

SymbolIndex는 실제 DocumentId를 전달하며 파일 경로 유무와 문서 존재를 분리합니다. 전역 팔레트는 현재 파일/untitled 모델을 선택하고 실제 심볼 문맥에서만 @ 목록/대기/빈 결과를 표시합니다. 기존 파일 위치 이동은 기존 deferred reveal을 유지합니다. Untitled는 실제 ViewKey/현재 snapshot·generation과 byte 선택을 검증해 해당 뷰의 caret/스크롤을 이동하고 숨겨진 접기를 풉니다. 경로 URI/ProjectId·LSP owner를 만들지 않으며 readonly 탐색에서 Rope/revision을 변경하지 않습니다. 닫힌 뷰·내용/언어 변경·잘못된 UTF-8 byte와 IME 중의 이동은 거절합니다.

Outline은 파일 전용 path 대신 기존 DocumentKey의 File/Untitled variant로 문서 변경을 구분합니다. Native producer의 실제 문서 집합으로 pending을 판단해 지원하지 않는 untitled 언어를 무한 로딩하지 않습니다. 키/문서가 바뀌면 트리 선택/접힘 캐시도 회수하며 기존 파일의 공급/키/가상화는 유지합니다. 새로운 공통 저장소·의존성·manifest/lock·동결 소스는 변경하지 않았습니다.

outline.log/72008은compile32.77초·기존3통과/0.04초·exit0입니다. palette.log/96393은compile16.05초·기존28+신규untitled1의29통과/0.29초·exit0이며 파일 경로/프로젝트 없는 실제 Core document ID에서 loading/목록/검색/세대 선택·닫힘의 구분을 확인합니다. 동일 입력의 성공을 새 전체 UI 실행으로 합산하지 않습니다.

app.log/41197은compile21.06초·0통과/1실패·37.51초·exit101입니다. 실제 NewUntitled 명령의 탭 ID에 JSON 문서 모델을 준비해 정식 OpenUntitled로 연결했지만 팔레트 목록이 빈 상태를 먼저 관찰했습니다. 기존 UI harness처럼 두 번의 초기 표시를 거치고 query/문서 ID를 실패 근거에 포함하도록 fixture를 보강했습니다. 제품 원인인지 준비 조건인지 최종 결과 전에는 단정하지 않습니다.

app-current.log/7096은compile19.44초·실제 앱1통과/38.34초·exit0입니다. 실제 NewUntitled로 생성한 TabId에 공개 모델 포트로 JSON 내용을 준비하고 정식 OpenUntitled가 같은 모델을 소비하게 했습니다. 실제 전역 팔레트의 @value 입력/목록/Enter와 outline의 실제 focus/ArrowDown/Enter가 비BMP 문자 뒤의 올바른 byte로 현재 뷰를 이동하며 문서 key·원문/revision과 종료tasks0을 보존합니다. 언어 선택 UI나 새 untitled 언어 자동 감지의 구현·검증으로 확대하지 않습니다. 고정 두 프레임 대기 대신 실제 palette inspection의 has_input_focus를 준비 조건으로 바꾼 최종 검사는 별도로 기록합니다. 최초 빈 목록의 원인은 당시 추가 진단이 없어 단정하지 않습니다.

reveal.log는compile0.26초·직접1통과/0.00초·exit0입니다. 실제 EditorStore의 untitled/mirror와 readonly 모델에서 해당 ViewId만 이동하고 원문/revision을 보존하며 잘못된 UTF-8 시작 byte·내용 변경/닫힌 뷰를 거절합니다. outline-current.log는compile0.20초·4통과/0.03초·exit0이며 기존3건과 untitled DocumentKey 변경 시 선택/접힘 회수1건입니다. regression.log는compile0.20초·기존 심볼5통과/0.00초·exit0입니다. 관련 서로 다른 검사는 UI29/실제App1/reveal1/outline4/기존심볼5의40건이며 앞선 동일 outline3·앱 성공은 중복 합산하지 않습니다.

설치 Monaco의 gotoSymbolQuickAccess.js는 symbol.selectionRange를 Range.collapseToStart로 접은 뒤 editorNavigationQuickAccess.js의 setSelection/revealRangeInCenter로 전달합니다. Untitled 직접 이동도 검증된 selection.start에 caret을 두고 기존 native reveal 정책으로 표시하며 실제 접기를 풉니다. 전체 픽셀/스크롤 애니메이션 일치로 확대하지 않습니다.

wasm.log/73404는UI lib의 --target wasm32-unknown-unknown --no-default-features --locked --offline 검사·7분07초·exit0입니다. 이후 시작한 최종 App 검사는 같은 build directory의 Cargo lock에서 대기했고 먼저 시작한 Wasm이 종료된 뒤 컴파일했습니다. 두 실행 핸들을 유지했으며 대기를 실패/종료로 처리하거나 Cargo를 재시작하지 않았습니다. 이후 Cargo는 종료를 관찰한 뒤 직렬 실행합니다. 기존 vendor wry17경고와 App 검사 링크의 compact unwind1경고는 성공과 구분합니다.

app-focus.log/4803은lock 대기를 포함한compile6분08초·실제앱1통과/43.41초·exit0입니다. 고정 프레임 수 대신 실제 has_input_focus가 준비된 뒤 @value를 입력했고 현재 untitled 팔레트/outline 이동·원문/revision·정상 종료tasks0을 확인했습니다. 기존 app-current의 같은1건을 중복 합산하지 않습니다. 소유Rust9 format·소유diff·manifest/lock/동결 변경0의 근거와 함께 b3f2 및 부모 b3f의 구조 공급 범위만 닫습니다. 전체 Core392/JSONCore49/Syntax5 및 b3f1 실제 peek의 변경 없는 성공은 재사용하며 새 전체 검사로 세지 않습니다. 이전 전체App877통과/22준비실패/보호3제외/exit101·Core성능ignored1·remote Wasm tests의 시간 API 불일치7·실제 OS/접근성/큰tier 성능/신뢰성/출시·전환 게이트는 유지합니다. 전체303/588(51.5%)·부분93/미연결110/미구현82·미완료285·배치23잔여18·최종33·목표active·잔여시간미산정입니다.

## b3g — JSON 구문 진단의 실제 소비

원본 setup.ts는 JSON defaults를 덮어쓰지 않습니다. 설치 features/json/register.js의 validate=true/allowComments=true/comments=error/trailingCommas=error·schemas 빈 목록/enableSchemaRequest=false를 확인했습니다. JSONWorker.doValidation은 이 설정을 language service에 넘기고 common/lspLanguageFeatures.js는 모델 생성 시 즉시, 내용 변경 시500ms 후 다시 검증하며 언어 변경/모델 폐기 시 해당 marker를 회수합니다. schema semantic과 schema의 주석/쉼표 허용 확장은 기존 b3e에 남기며 구문 진단만으로 전체 JSON 검증 완료를 주장하지 않습니다.

editor-json-validation.rs는 기존 Core syntax_diagnostics를 별도 supervised blocking 작업으로 실행합니다. 실제 Normal JSON의 붙은 뷰/DocumentId·키/revision/언어/tier로 결과 수명을 제한하고 변경 지연500ms·활성 worker1개·기존 Normal byte 상한과 scanner/parser/깊이/노드/진단·메시지 예산을 명시합니다. 취소한 실제 worker가 종료될 때까지 slot을 보유하며 종료가 다음 작업을 깨웁니다. 구조 공급의 표시 용량 실패와 독립된 작업이므로 심볼 실패가 구문 진단을 성공으로 위장하거나 막지 않습니다.

진단 Store에는 native DocumentId 슬롯을 별도로 추가했습니다. 외부 Owner/bindings의 생성·사용 없이 Core byte marker를 검증하고 기존 표시·탐색 MarkerSet 및 문제 UI의 Diagnostic 표현에 연결합니다. DTO 변환은 UI의 메시지/심각도/UTF-16 좌표 소비용이며 서버 메시지를 송수신하거나 가짜 LSP owner를 만들지 않습니다. 파일/untitled DocumentKey·실제 탭 제목을 보존하고 외부 공급의 회수와 독립적으로 native 진단/개수/표시 캐시를 회수합니다. 문제 목록도 typed key를 사용하며 파일 행의 기존 안정 ID는 유지합니다.

validation-first.log/83521은검사 작성의5컴파일 오류·exit101·실행0건입니다. 부모 모듈에서 제거한 PathBuf import의 기존 검사 의존·TaskSupervisor의 실제 new(runtime) 계약·문제 Open의 typed key로 바뀐 기존 단언3곳을 대조해 수정했습니다. 제품 실행 실패나 통과 건수로 합산하지 않으며 최종 관련 검사와 실제 앱 소비의 결과는 아래에 기록합니다.

validation-current.log/64001은compile35.40초·5통과/1실패·45.49초·exit101입니다. 새 문서 작업2/별도 native 저장소1과 기존 JSON 본문 앱1이 관련 성공이며 필터에 함께 잡힌 remote JSON 왕복1은 이 기능 건수에서 제외합니다. 기존 JSON peek는 마지막 Escape 뒤 구조 문서 회수를 기다리다10초 제한에 도달했습니다. validation-peek-diagnostic.log/55255는compile18.76초·0통과/1실패·40.29초·exit101이며 추가 호출 위치 관찰로 마지막 닫힘 대기임을 확인했습니다. 당시 native/validation 문서가 각각1개·supervised tasks9개 남아 있었으며 분석 자체의 실패로 확대하지 않습니다.

Peek의 presentation은 본문 진단을 그대로 복사하지 않고 현재 peek snapshot의 별도 native/외부 진단을 다시 조회하도록 연결했습니다. validation-app.log/97604는compile21.86초·실제 앱2통과/46.75초·exit0입니다. 본문 앱은 실제 untitled의 주석/끝 쉼표 오류2건·현재 byte의 diagnostic-hover 응답·현재 문서 문제 widget/이동·Undo 후 오류 회수와 원문/tasks0을 확인합니다. Peek 닫힘은 한 번의 추가 UI 표시 후 즉시 세션 닫힘과 기존 문서 회수 조건을 확인했습니다. 추가 표시가 없는 기존 순서도 다시 확인하며 앞선 시간 초과의 정확한 원인은 아직 단정하지 않습니다.

validation-diagnostics.log/91370은compile18.02초·7통과/0.03초·exit0입니다. 저장소6건(기존4+native untitled/같은 파일 외부 공존2)과 기존 raw 수명1건으로, native 슬롯은 실제 키/UTF-16 좌표·표시·탐색/개수를 제공하고 외부 회수와 독립이며 잘못된 native byte 게시가 이전 유효 결과를 손상하지 않습니다. 제품은 native에 Owner를 만들지 않고 외부 mock Owner는 별도 테스트 프로토콜 입력입니다. UI/이동과 마지막 실제 앱의 결과는 아래에 이어 기록합니다.

validation-problems.log/30186은compile26.44초·24통과/0.11초·exit0입니다. 기존 편집기 문제 탐색5와 목록19(기존18+새 untitled1)에서 파일 행의 안정 ID/키/접기/필터/사건 순서/툴팁/viewport 수명과 native 실제 DocumentKey/제목·행 클릭의 document/revision/UTF-16 위치 반환을 확인했습니다.

validation-app-current.log/1769는실제앱1통과/1실패·35.60초·exit101입니다. 본문은 추가한 실제 문제 목록의 untitled 행 클릭→기존 현재 뷰의 쉼표 byte 이동·문제 widget·Undo 회수까지 통과했습니다. 한 번의 추가 UI 표시를 제거한 peek는 원래 마지막 닫힘 대기에서 다시 시간 초과했으므로 앞의2통과를 최종 닫힘 성공으로 재사용하지 않습니다. Escape 입력 직전의 실제 preview 포커스와 직후 세션 폐기를 직접 검사하도록 관찰을 추가했으며 이 경계가 해결되기 전 전체 b3g를 닫지 않습니다.

validation-peek-focus.log/80945는compile20.70초·실제 peek1통과/33.25초·exit0입니다. 추가 UI 프레임 없이 Escape 직전 입력 포커스가 실제 preview ViewId에 속하고 입력 직후 owner 세션이 폐기된다는 직접 단언·기존 JSON 구조/접기·문서 회수/tasks0이 통과했습니다. 제품 코드는 앞의 실패 때와 같고 검사 관찰만 추가했으므로 간헐적 마지막 회수 시간 초과의 원인을 해결했다고 단정하지 않습니다. 이전 실패/성공 로그와 재현 순서는 그대로 보존하며 peek의 실제 비어 있지 않은 진단 표시·hover 및 다른 문서/프로젝트의 untitled 문제 이동과 함께 b3g2의 잔여 검증으로 유지합니다. 성공한 본문·현재 소유 포커스의 좁은 peek 실행을 전체 닫힘 신뢰성 게이트로 확대하지 않습니다.

validation-app-body.log/25261은compile18.44초·현재 본문 앱1통과/33.94초·exit0입니다. 실제 파일 언어 변경 후 진단 작업 회수·읽기 전용 Normal의 진단 문서 자격·Large 회수·실제 untitled의 오류2/행 클릭·UTF-16→현재 byte 이동/문제 widget/Undo 회수·두 native 작업 집합의 종료 회수/tasks0을 확인했습니다. 같은 본문 성공과 앞의 좁은 peek 성공을 중복 합산하지 않습니다.

validation-removal-before.log/93319는compile15.53초·0통과/1실패·0.03초·exit101입니다. Store.remove_document 직후 늦은 native 결과를 게시하면 오류 수가0 대신1로 되살아나는 실제 신규 경계 오류를 재현했습니다. 제거 시 builtin_documents의 게시 자격도 회수하도록 수정하며 외부 Owner/bindings나 다른 native 문서의 진단은 변경하지 않습니다. 수정 후 같은 저장소/원본 raw 경계를 검사하며 최종 결과는 아래에 기록합니다.

validation-removal-after.log/8844는compile14.09초·관련 진단7통과/0.03초·exit0입니다. 제거 뒤 늦은 게시를 거절하고 현재 다른 공급자/문서와 유효한 byte/원본 데이터를 보존합니다. 문서 작업2·진단7·목록/탐색24·현재 본문 앱1·입력 소유를 직접 확인한 peek1의 서로 다른35건을 성공 근거로 사용하며 앞의 동일 성공·관련 없는 remote1은 합산하지 않습니다. 간헐 peek 회수 실패의 재현 원인이 미확정인 사실은 별도 미완료로 유지합니다.

소유Rust13 format·소유diff·manifest/lock/동결 변경0을 확인했습니다. 변경은 host App 내부이며 UI/Core의 공유 공개 계약과 Wasm 소스는 바뀌지 않아 b3f2의 Wasm UI lib·기존 Core392/JSONCore49/Syntax5 성공을 재사용합니다. 새 전체 App/Core/UI/Wasm 실행으로 세지 않으며 기존 vendor wry17/compact unwind1경고, 전체App877통과/22준비실패/보호3제외/exit101·Core성능ignored1·remote Wasm tests의 시간 API 불일치7·OS/접근성/대형 성능/신뢰성/출시·전환 게이트를 유지합니다. b3g1만 닫고 b3g2/부모 b3g·schema semantic/나머지 JSON/CSS/HTML/TS는 미완료입니다. 전체303/588(51.5%)·부분93/미연결110/미구현82·미완료285·배치23잔여18·최종33·목표active·잔여시간미산정입니다.

## b3g2a — Peek 오류·호버와 사라진 고정 줄의 포커스

peek-message.log/68190은compile22.92초·0통과/1실패·36.87초·exit101입니다. 실제 JSON peek에 준비한 주석 오류의 marker 응답은 존재하지만 최초 tooltip 표시에서 메시지를 기대해 실패했습니다. 기존 problems tooltip 검사처럼 준비/표시 시간을 구분하고 실제 shape의 문구를 관찰합니다. peek-message-current.log/33192는compile22.07초·0통과/1실패·35.82초·exit101이며 실제 오류 메시지/본문 분리는 통과한 뒤 Escape 직전 포커스가 None으로 사라진 사실을 확인했습니다. 시간 초과만 반복하지 않고 입력 소유 단계에서 원인을 분리했습니다.

기존 App 검사는 editor_keymap_targets HashMap의 첫 preview 항목을 선택했습니다. 실제 Provider는 본문 response와 sticky/문제/위치 popup 등의 focus_ids를 함께 등록하므로 이 항목은 안정된 본문을 뜻하지 않습니다. NativeEditor는 입력 전 sticky.prepare의 무효화에는 본문 포커스를 회수하지만 그 뒤 maintain_folds/최종 배치에서 고정 줄이 없어지는 경로에는 회수가 없었습니다.

sticky-focus-before.log/55659는compile14.50초·0통과/1실패·0.02초·exit101입니다. 실제 고정 줄 ID에 포커스를 둔 뒤 외부 FoldAll 명령을 소비하면 헤더가 사라지고 본문 대신 None이 되는 동일한 수명 오류를 재현했습니다. 최종 sticky paint 뒤 현재 포커스가 이전 고정 줄 소유이고 새 표시 목록에서 사라졌을 때만 해당 본문 response로 회수합니다. 다른 입력창의 현재 포커스는 바꾸지 않으며 기존 입력 사건 순서를 유지합니다.

sticky-focus-after.log/11602는compile10.77초·기존17+신규1의18통과/0.06초·exit0입니다. 고정 줄 소실/본문 회수와 기존 클릭/키/다른 입력창·닫기/읽기 전용·설정·행/문서 변경 수명이 통과했습니다. App은 임의 HashMap 항목 대신 실제 고정 줄 ID를 선택해 접기 후 본문 포커스·Escape 즉시 세션 폐기·뷰/작업 회수까지 확인하며 일반/읽기 전용 JSON 두 사례의 결과는 아래에 기록합니다.

native-problem-before.log/71768은compile19.42초·0통과/1실패·0.03초·exit101입니다. 실제 Core untitled 문서/byte marker만 남긴 뒤 파일의 NextInFiles를 실행하면 현재 파일 전용 Provider가 요청을 만들지 않는 미연결 경계를 재현했습니다. 이 신규 검사와 File/Untitled 요청·프로젝트/뷰 소유 연결은 b3g2b에 진행 중이며 포커스 수정이나 전체 JSON 소비 완료로 합산하지 않습니다.

sticky-focus-app.log/40864는compile29.44초·실제 App1통과/44.10초·exit0입니다. 한 검사 안에서 편집 가능/Normal readonly 두 실제 앱을 생성해 주석 오류의 현재 peek marker·실제 tooltip 문구·본문 Rust 진단과의 분리·고정 줄 ID의 실제 입력 소유→F9/FoldAll→해당 본문 focus→Escape 즉시 세션 폐기·native 문서/작업0을 확인했습니다. HashMap의 임의 focus 항목을 쓰거나 접힌 뒤 강제로 포커스를 다시 지정하지 않습니다. 이전 실패와 같은 일반 peek를 별도 신규 건수로 합산하지 않습니다.

sticky-focus-wasm.log/2260은변경한 UI lib의 --target wasm32-unknown-unknown --no-default-features --locked --offline 검사4.85초·exit0입니다. 소유Rust3 format/소유diff와 manifest/lock/동결 변경0을 확인했습니다. UI의 공유 공개 계약은 바꾸지 않고 native-host의 사라진 고정 줄 focus만 회수합니다. 기존 foreign 입력창/원문·readonly 탐색과 Web/Wasm 경계를 유지하며 전체 App/Core/UI/플랫폼·성능·출시·전환 검증으로 확대하지 않습니다. b3g2a만 닫고 b3g2b/부모 b3g2·b3g/전체 JSON 서비스는 미완료입니다. 전체303/588(51.5%)·부분93/미연결110/미구현82·미완료285·배치23잔여18·최종33·목표active·잔여시간미산정입니다.

## b3g2b — 실제 untitled 문제의 문서·프로젝트 이동

파일 전용 요청을 DocumentKey로 바꾸고 실제 Untitled TabId의 프로젝트·pane을 찾아 기존 layout_activate_tab/project_activate를 호출합니다. 응답은 현재 source 뷰·문서/토큰·창 소유와 목적 layout/key/활성 탭을 검증하고 추적된 marker byte로 실제 EditorStore 뷰를 이동합니다. 문제 행도 현재 문서 key/revision을 검사한 뒤 실제 탭이 속한 프로젝트를 활성화합니다. File용 기존 공통 응답 검사는 유지합니다.

native-problem-state.log/83756은 기존 파일 응답 타입을 사용하는 다른 소비자 9곳의 컴파일 오류로 종료했습니다. 파일 API를 유지하고 새 문서 응답 경계를 분리한 state-current.log/8348은6통과/exit0입니다. native-problem-scope.log의 현재6통과/exit0은 Main/Auxiliary source 소유, 다른 프로젝트 Main 허용·다른 창/slot 거절, 비BMP 좌표와 대기 중 편집의 추적, 기존 파일/토큰/문서 수명을 포함합니다. 같은6건은 중복 합산하지 않습니다.

실제 App 확장의 첫 native-problem-app.log/15229는 fixture의 borrowed TabId/임시 snapshot 참조 컴파일 오류3건으로 종료했습니다. 소유 ID를 복제한 app-current.log/5312는 새 프로젝트 focus가 layout 등록보다 먼저 보이는 조건에서 실패했습니다. app-ready.log/49723·app-inspect.log/27551·project-stage.log/32619는 실제 OpenProject의 layout 등록 대기가 각각 실패했습니다. inspection에서는 프로젝트2개/layout1개와 작업8을 관찰했고 stage 로그에서는 watcher 준비가 실패 후에 끝났습니다.

동일 바이너리 sample-run.log/46369는exit101이며 watch-sample.txt에서 build_files→watcher::start_watch→notify FsEventWatcher::run→채널 수신 대기, 별도 FSEvents 스레드의 FSEventStreamStart→register_with_server→f2d_register_rpc→mach_msg 대기를 확인했습니다. 운영체제 감시 등록 지연은 해결했다고 보고하지 않습니다. 진단용 projects.rs 출력은 모두 제거했습니다. 문제 이동 검사는 기존 첫 프로젝트와 동일하게 소유 실제 경로/프로젝트/session/layout이 있는 복원 세션을 시작 조건으로 사용하며, 실제 NewUntitled·문서 작업·F8/ShiftF8·문제 행·프로젝트 활성화 경로는 유지합니다. 이 검사를 프로젝트 열기 검증으로 확대하지 않습니다. 직접 taide_project 서비스 접근 시도는 현재 App의 직접 의존성이 없어 two-project.log/55748에서 컴파일 오류1건으로 종료했고 의존성은 추가하지 않았습니다.

restored-projects.log/90388은compile27.11초·실제App1통과/33.54초·exit0입니다. 두 소유 프로젝트의 실제 NewUntitled/JSON 모델·진단 작업을 사용하며 F8로 다른 프로젝트의 비BMP 뒤 주석 오류로 이동하고 목적 본문 focus·현재 문제 widget/document를 확인합니다. 강제 재포커스 없이 ShiftF8로 원 프로젝트에 복귀하고 실제 문제 행 클릭으로 다시 목적 프로젝트/탭/caret을 활성화합니다. 목적 원문/revision과 원 문서 undo/오류 회수·문서 닫힘·정상 종료tasks0을 확인합니다. 마지막 문제 행 뒤 포커스 단언을 추가한 검사는 아래에 별도 기록하며 이전 성공을 새 검사로 중복 합산하지 않습니다.

row-focus.log/11059는compile18.03초·0통과/1실패·35.81초·exit101입니다. 마지막 문제 행의 목적 프로젝트/탭/caret은 맞지만 실제 egui focus가 이전 프로젝트 ViewId2이고 목적은ViewId3인 오류를 재현했습니다. 문제 행 처리에서 목적이 표시되기 전에 self.focused를 목적 pane/tab으로 설정해 show_document의 실제 새 본문 포커스 요청을 막고 있었습니다. 이 조기 설정 한 줄을 제거하고 기존 show_document가 목적 본문을 표시할 때 포커스를 갱신하게 합니다. 같은 문서 문제 행의 기존 직접 focus 요청은 유지합니다.

row-focus-fixed.log/92421는compile18.37초·0통과/1실패·35.58초·exit101이며 같은 시점의 포커스가None이었습니다. wait의 준비 조건은 controller의 새 프로젝트와 이미 변경된 caret만 확인해 비동기 새 snapshot이 아직 UI에 그려지기 전에 끝날 수 있었습니다. 실제 목적 본문의 has_focus를 준비 조건에 추가해 전환 결과의 실제 입력 소유까지 검사합니다. 고정 프레임 수·추가 시간 제한·강제 포커스 지정은 사용하지 않습니다.

row-rendered.log/27595는compile19.40초·실제App1통과/35.56초·exit0입니다. 목적 본문이 실제 focus를 가진 상태의 F8/ShiftF8·문제 행 왕복과 문서 key/위치·본문 분리·비BMP·원문/revision·undo·닫힘·tasks0을 확인했습니다. 현재 관련 검사는 상태6/실제App1의 서로 다른7건이며 변경 없는 g1·peek의 성공을 재사용하고 같은 앱 반복 성공은 합산하지 않습니다. 소유Rust5 format/소유diff가 통과했고 새 의존성/manifest/lock/동결·공유 UI/Core 소스 변경은 없습니다. b3g2b·b3g2·구문 소비 b3g만 닫으며 JSON schema 의미 검증·언어 선택 UI·포맷/선택 확장/조회/완성·CSS/HTML/TS 및 전체 App 준비/성능/OS/출시 게이트는 미완료입니다. 전체303/588(51.5%)·미완료285·editor-57partial/editor-16partial·목표active·잔여시간미산정입니다.
