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
