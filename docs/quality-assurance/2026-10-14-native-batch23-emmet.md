# 배치 23 — Native Emmet

현재 상태: 원본 등록/후보/추출 경계를 대조하고 portable core 축약식 추출만 구현했습니다. 확장·필터/기본 snippets·후보/실제 앱 소비는 미완료이며 editor-80 missing/302완료·전체 목표 active를 유지합니다.

## 원본과 현재 경계

src/shared/lib/emmet-integration.ts는 emmet-monaco-es5.7.0의HTML(html/heex)/CSS(css/scss)/JSX(javascriptreact/typescriptreact)에tokenizer:standard로등록하며설정기본true의EmmetProvider에서dispose/re-register합니다. 설치패키지README/index.ts/abbreviationActions.ts/emmetHelper.ts와emmet2.4.11의dist실제extract/expand/config를읽었습니다. standard token은현재위치의Other만허용하고Comment/String/RegEx를거절하며라이브러리는embedded CSS를지원하지않습니다.

원본doComplete는markup lookAhead/stylesheet false와filter추출·open tag거절·유효축약식/noise·기본tag/snippet 추가후보·HTML/JSX/CSS 형식·문서/Emmet abbreviation detail·snippet tabstop/finalstop·range/filterText/sortText를조립합니다. native에는Settings.emmet_enabled와설정화면토글만있고확장/공급자는없었습니다. 추가의존성/JS런타임/manifest/lock/동결소스는사용하지않습니다.

## e3a 축약식 추출

native-editor의emmet.rs는여섯language ID의Syntax와borrowed Extraction/range를공개하고emmet-extraction.rs는원본extract의역방향curly/square/round·attribute/text 내부문자·실제HTML종료태그감지·앞쪽연산자제거·markup의자동닫힘lookAhead/style고정cursor를적용합니다. UTF-16원본위치를fixture에서UTF-8byte로변환해Unicode/surrogate를분리하지않습니다. 입력max_bytes와잘못된byte경계는None이며확장이나표준token/전체후보게이트라고확대하지않습니다. 적응한MIT알고리즘의출처/원문license는native/taide-native-editor/emmet-license.txt에보존합니다.

기존설치emmet@2.4.11의extract를실제로호출하는fixtures/emmet-extraction-oracle.mjs는66입력의모든문자경계/3syntax에서3096사례를고정합니다. raw Rust 데이터fixture를사용해core의새serde/JSON의존성을추가하지않았습니다. emmet-extraction-red.log는최초컴파일9.93초·3통과/0실패·0.00초·exit0입니다. 3096은oracle비교사례이며실행한검사수3과구분합니다. 두추가검사는정확한여섯언어/비지원ID와입력상한/잘못된cursor/Unicodebyte경계를확인합니다. Wasmcore compile은현재실행중이고이단계만으로Emmet 기능완료를선언하지않습니다.

emmet-extraction-wasm.log는새core의wasm32-unknown-unknown lib compile을직접확인했고의존성포함3분03초·exit0입니다. 기존remote-web이나Wasm테스트/시간API불일치해결로확대하지않습니다. 소유5Rustfmt·oracle MJS/QA포맷·manifest/lock/frozen변경0·디스크630GiB/66%이며fixture재생성의변경은포맷만이라같은3096입력의성공을재사용했습니다. e3a추출범위만닫고editor-80의확장/후보/앱소비및전체302/588(51.4%)·미완료286·잔여시간미산정을유지합니다.

소유추출core/검사/oracle/MIT출처/QA와PROCESS상단9경로를fdb39e3b로선별커밋했습니다. 사용자PROCESS하단2hunk·다른문서/결정/피드백/메모리를보존했습니다. 기능표baseline과editor-80의분리된추출기초근거를추가하되확장/후보/앱이없어missing/전체302완료는유지합니다.

## e3b 원본 출력의 초기 대조

설치emmet-monaco-es의실제doComplete에line/position/Range만주입해HTML/JSX각19축약식의주후보insertText를직접관찰했습니다. div는빈본문final field를포함하고span{hello}는추가final field없이다룹니다. a는href의첫field/본문finalfield, img는src/alt fields, input:checkbox는리터럴type을통해소모한fieldindex를보존해name2/id0으로출력합니다. class는JSX에서className으로바뀌며expression속성{value}는두syntax모두quotes없이보존합니다.

div>span+strong은inline자식이어서한줄이고ul>li*2와div>p^aside는tab들여쓰기·줄바꿈을사용합니다. (header+main)>p의p는그룹마지막main의child가아니라같은root의sibling입니다. repeat의$counter와여러빈본문field는마지막field만0으로바뀝니다. boolean hidden은두syntax모두hidden=hidden의quoted형태이며br/도현재원본output.selfClosingStyle:html때문에HTML형식입니다. 기억으로JSXselfclosing을추가하지않고원본출력에맞춥니다. 이관찰은Rust확장성공을뜻하지않으며e3b는구현전입니다.

## e3b1 markup 구조 파서

emmet-markup-parser.rs는원본TokenGroup/TokenElement의name/text/raw attributes·JSXmember/표현식/multiple·children/group·반복선언/닫힘을portable typed AST로구성합니다. 입력byte/노드/그룹·child·속성/text깊이예산은기존EditorError::Capacity이며CSS·잘못된구문은Refused입니다. 원본이허용하는미완성text/group/속성의구조는그대로구분합니다. 반복선언은Count/Implicit로보존하며반복의실제변환/기본snippet해결/필드/출력/후보와구분합니다. MIT출처에@emmetio/abbreviation2.3.3을추가했습니다.

markup-parser-red.log는1통과/1실패·compile10.04초·exit101입니다. 반복oracle가원본Repeater의반복횟수count대신현재반복index value(0)를읽던준비오류를실제토큰객체로확인해수정했습니다. core의속성내slash/dot와이름뒤${1}의원본name/text구분도수정했습니다. markup-parser-fixed.log는130사례·2통과·compile9.17초·exit0입니다. 기본markup snippet원문과추가quoted/expression을넣은markup-parser-defaults.log는225입력/450사례·2통과·compile4.40초·exit0입니다.

ASCII인쇄문자전체의name/class/attribute와분수형class를더한markup-parser-symbols.log는513입력/1026사례의1통과/1실패·compile4.42초·exit101입니다. native가거절해야할%,&,comma,semicolon,angle,question,at,backtick,pipe,tilde를이름에허용하던분기·숫자사이slash·미완성속성의다른closing bracket처리를재현했습니다. 원본getToken/isElementName/반복counter modifier와attribute별괄호깊이를대조해수정했습니다. symbols-fixed.log는소스생성의Unicodeescape표현오류6건으로컴파일실패/exit101이며검사성공으로세지않습니다. 이를Rustbrace escape로바로잡은symbols-final.log는1026사례대조/예산의2통과·compile8.98초·exit0입니다. 같은입력의예전성공/실패를중복합산하지않습니다.

markup-parser-wasm.log의현재core wasm32-unknown-unknown lib compile은4.82초·exit0입니다. 소유4Rustfmt와oracle MJS/QA포맷·manifest/lock/동결source변경0을확인했습니다. e3b1의raw구조범위만검증했으며기본snippet해결/반복counter변환/필드해결/HTML·JSX렌더·CSS/필터/후보/실제앱은미완료입니다. editor-80missing·302/588(51.4%)·미완료286·목표active·잔여시간미산정을유지합니다.

소유raw markup파서/core export/검사/원본oracle/출처/QA와PROCESS상단8경로를d565bcb9로선별커밋했습니다. 사용자PROCESS하단2hunk·다른문서/결정/피드백/메모리를보존했습니다. 기능표baseline과editor-80기초근거만갱신하며반복/스니펫/필드/렌더/앱이없어missing·전체302완료/목표active를유지합니다.

## e3b2 반복·필드/변수 변환

emmet-markup-conversion.rs는Repeat Count/Implicit·공유repeat guard·번호padding/base/reverse/상위repeat·group flatten/메타데이터·escaped text·fields/variables·속성quote/expression/boolean/implied/multiple와text-only children을typed ExpandedMarkupNode/Attribute/Value로변환합니다. maxRepeat를포함한node/byte/depth예산에서부분결과를반환하지않습니다. 원본의빈attribute set 존재를잃지않게raw has_attributes를추가했고형제/자식·변수없는implicit repeat의빈text삽입도보존합니다.

새변환oracle와기존파서가같은입력을사용하므로emmet-markup-inputs.mjs로공통화했습니다. 실제재생성된기존513입력/1026parser fixture는diff0입니다. conversion oracle는HTML/JSX·변수없는/세변수설정의528입력/2112사례이며maxRepeat7·0/implicit/2000반복·상위counter·fields/expression·text-only를포함합니다. conversion-check.log는초기core compile5.04초 exit0이며최종코드성공근거로전용하지않습니다. conversion-red.log는compile10.89초·1통과/1실패·0.01초·exit101이며coreparser target은fail-fast로실행되지않았습니다.

실패4사례는설치원본tokenVisitor.Bracket의group닫힘이괄호대신curly로작성된오탈자입니다. div[foo=(bar)]의원본AST값(bar}를관찰하고native는올바른(bar)를유지합니다. 원본oracle원문은보존하며해당정확한입력만기대오탈자를단언한뒤올바른닫힘과비교하고별도수정검사를추가했습니다. 2108사례는원본그대로일치하고4사례는의도한원본bug수정입니다. 원본버그를강제로복제하지않는프로젝트범위에맞춘차이를명시합니다.

conversion-final.log는compile9.93초·변환3/parser2의5통과·0실패·exit0입니다. 필드name으로복구한${1}의문자byte누락을추가로재현한field-budget-red.log는0통과/1실패·exit101로예산1에서Ok를보였습니다. name복구의추가marker/index 문자를charge하는field-budget-fixed.log는1통과·0.00초·compile9.45초·exit0입니다. 반환문자열이같은앞선5성공은재사용하며현재서로다른6검사입니다. coreWasm conversion-wasm.log는4.85초·exit0, 소유7Rustfmt·MJS/QA포맷·manifest/lock/frozen변경0·디스크630GiB/66%입니다.

기본snippet해결/implicit tag·HTML/JSX formatting/field offset·필터/lorem/CSS·후보/실제App소비는미완료입니다. e3b2변환만닫고editor-80missing/302완료·미완료286·전체목표active·잔여시간미산정을유지합니다.

소유typed변환/빈속성존재·예산수정/원본oracle/공통입력/QA와PROCESS상단10경로를1b409ced로선별커밋했습니다. 사용자PROCESS하단2hunk와다른문서/결정/피드백/메모리를보존했습니다. 기능표baseline/변환근거만갱신하고renderer/후보/App이없어editor-80missing/302완료·전체목표active를유지합니다.

## e3b3 기본 스니펫·태그/속성 준비

설치emmet2.4.11 resolveConfig의기본markup202 registry와5 variables·inline elements를Rust정적자료로고정했습니다. 재생성기emmet-markup-defaults.mjs는개발검증용이며native실행에JS런타임/새의존성을추가하지않습니다. 준비계층은재귀alias의definition stack 순환종료·기본/사용자속성/value/repeat 우선순위·마지막깊은노드에원본children연결·implicit tag/parent context·class중복값병합/다른속성override·label안의빈for/id회수를연결합니다. tag/byte/node/depth예산을적용하며기본dictionary/alias모두를실제원본prepared AST로대조합니다.

준비/변환oracle가같은canonical출력을쓰므로MJS/Rust 비교helper를fixtures/emmet-markup-canonical로공통화했습니다. 기존2112변환fixture재생성은diff0이며내용이같은기존성공을반복하지않습니다. preparation-red.log는919입력/1838사례·2검사통과·0.02초·compile9.50초·exit0입니다. 원본group닫힘2사례는이미기록한정확한입력의오탈자만바로잡으며나머지1836은원본그대로일치합니다. 기본202이름/원문·속성수정·label/input·부모/fieldsnippet·중복class/속성을포함합니다. 새검사이름AST대문자의snake case경고는이름을소문자로수정했고preparation-host.log의두검사target compile은4.97초·추가경고0·exit0입니다. 입력/환경/본문동작이같아2검사성공은재사용합니다.

preparation-wasm.log는현재core lib의wasm32 compile4.47초·exit0이며Wasm검사실행/기존시간API문제해결로확대하지않습니다. 소유7Rustfmt/MJS·QA포맷/manifest·lock·frozen변경0/디스크630GiB·66%입니다. e3b3준비2검사와변경없는이전파서/변환6검사는서로다른8성공으로재사용합니다. HTML/JSX출력/field offset·lorem/BEM/필터/CSS·후보/실제App은미완료라editor-80missing·전체302/588(51.4%)·미완료286·목표active·잔여시간미산정을유지합니다.

소유기본스니펫준비/생성자료/공통비교helper·검사/원본oracle/출처/QA와PROCESS상단14경로를31bb6ed8로선별커밋했습니다. 사용자PROCESS하단2hunk와다른문서/결정/피드백/메모리를보존했습니다. 기능표baseline과준비근거만갱신하며출력/후보/App이없어editor-80missing/전체302완료·목표active를유지합니다.

## e3b4 HTML/JSX 출력

준비된AST를inline/block·html skip/body force·tab/newline·문자열/placeholder·snippet field 위치의children·속성boolean/expression과JSX className/htmlFor/styleName/styles로출력합니다. 전역field offset과가장높은빈field의final0은typed field byte위치로관리하고최대byte/depth/index상한을거절합니다. CSS·lorem/BEM/필터와실제후보/App은현재범위에연결하지않았으므로e3b/e3의전체완료로확대하지않습니다.

설치emmet-monaco-es5.7.0의실제expandAbbreviation과원본doComplete의출력옵션을사용한oracle는920입력/1840사례입니다. 첫output-red.log는비교생성기의상대경로한단계초과→fixture없음으로compile실패/exit101이며0검사입니다. 상대경로를현재파일로수정한output-parity.log는compile9.09초·예산1통과/비교1실패·0.03초·exit101입니다. 차이는p{escaped field}의2사례로원본addFinalTabStop/escapeNonTabStopDollar가literal ${1}을탭위치0으로바꾸어본문문자를없애는오류였습니다. native는typed Text/Field를구분해literal문자를보존합니다. 원본oracle를그대로보관하고정확한input/원본잘못된output을단언한뒤올바른literal기대값과비교하며실제snippet_syntax/expansion에서텍스트/빈placeholder를검증합니다. 기존group닫힘2사례도이미기록한원본오탈자를수정하며나머지1836은원본그대로일치합니다.

output-fixed.log는4통과/0실패·0.05초·compile4.47초·exit0입니다. 출력비교/예산·CSS/field overflow/실제literal소비·성공한모든출력의현재native snippet문법소비를확인했습니다. 소유6Rustfmt와manifest/lock/frozen변경0을확인했으며변경core의전체all-targets/no-fail-fast/test-threads4는output-core-all.log에서실행중입니다. 종료전전체성공으로선언하지않습니다.

fixture literal생성은6생성기에서공통emmet-rust-literal.mjs를사용하도록정리했습니다. 문자열값을유지하면서codepoint/제어문자/quote/backslash를Rust표준literal로표기해code에그림문자를직접쓰지않습니다. 원본호출/추출3096/구조1026/변환2112/준비1838/출력1840의입력·기대값/예산은같고최종fixture컴파일을전체실행종료후확인합니다.

## e3b4 전체 결과·e3b5 필터

output-core-all.log의직접전체실행은92544 handle에서정상종료했습니다. --all-targets/--no-fail-fast/test-threads4·44대상/322통과/0실패/기존대형성능1ignored·compile43.64초·exit0입니다. 문서/편집/undo/mirror/완성/스니펫/들여쓰기/포맷/위치의기존target과당시Emmet15검사를포함하며앞선좁은성공을중복합산하지않습니다. 기본출력뒤의독립필터코드는이후추가했고변경없는기존target성공은재사용하며새/영향검사만확인했습니다.

필터는최대3개의알려진bem/c/t접미사를분리하고BEM기본block/cache·element/modifier/ancestor·중복제거/JS공백과닫힘주석의id/class/fieldoffset을연결했습니다. 원본getExpandOptions는t를인식해도출력변화를적용하지않으므로native도임의trim을추가하지않습니다. void/self-closing은원본처럼닫힘주석을추가하지않습니다. 원본3filter label의comma표현때문에oracle는primary의정확한detail을선택하며태그추가후보와구분합니다.

markup-filters-red.log는최종fixture/필터코드의compile10.90초·필터3(32stem/10suffix/2syntax의640사례)/기본출력4의7통과·0실패·0.01/0.07초·exit0입니다. field/comment/Unicode공백/대문자중복id/void/다중filter를포함합니다. 실제BEM변환전준비는성공하고변환후byte예산만넘는검사를보강한filters-budget.log는준비AST의속성이름길이를빠뜨린상한44의0통과/1실패·exit101입니다. 실제기본AST문자열길이49로상한을고정한filters-budget-final.log는해당1통과·compile4.36초·exit0이며앞선반복성공과더하지않습니다. 기존전체322와새필터3의서로다른325성공·0실패·성능1ignored입니다.

최종표기/현재library 및Emmet6검사target의output-final-host.log compile은12.94초·추가경고0·exit0, output-final-wasm.log는현재corelib4.86초·exit0입니다. 최종14Rustfmt/9MJS·QA포맷·manifest/lock/frozen변경0·디스크630GiB/66%를확인했습니다. 동일Unicode/문자열값의fixture표기만변경해성공을재사용하고새기능/기본출력은현재코드로다시확인했습니다. wasm검사실행/기존시간API불일치/전체App877·22준비실패/실기해결로확대하지않습니다.

HTML/JSX기본출력과필터의core구현은연결되어editor-80을missing에서unwired로반영합니다. lorem/기타추가경계/CSS·completion range/validation/추가후보·설정/실제App소비와전체게이트는남아있으며전체완료302/588(51.4%)·부분92/미연결111/미구현83·미완료286·목표active·잔여시간미산정을유지합니다.

출력·필터와공통Rust문자열fixture/MIT출처·검사·QA/PROCESS상단의26경로를5bf4acc9로선별커밋했습니다. 사용자PROCESS하단2hunk와다른문서/결정/피드백/메모리는보존했습니다. 기능표baseline을이커밋으로갱신하며editor-80unwired·전체302완료와미완료게이트를유지합니다.

## e3b6 — lorem

[공식 lorem 문서](https://docs.emmet.io/abbreviations/lorem-ipsum/)와설치emmet2.4.11의실제transform/latin·ru·sp 어휘·난수선택/쉼표/문장/반복을대조했습니다. 문서의class보존예시와달리현재설치원본은lorem처리때attributes를제거하므로실제프로젝트출력을기준으로했습니다. 대문자locale/알수없는locale는원본처럼latin이며범위상한은exclusive입니다. 첫반복만common문장을쓰고자체반복된자식은부모에맞는암시태그를사용합니다.

prepare_markup_with_random/expand_markup_with_random은u32 난수콜백을받아portable core에연결했습니다. 원본처럼추가난수draw의순서를유지하며기존비난수준비API는보존합니다. 실제앱연결은새난수API를선택해야하며아직없습니다. 기존앱의UUID 난수공급과같은주입경계를사용할수있고추가의존성/OS난수전역/JS런타임은코어에넣지않았습니다. 변환전체draw는byte예산으로제한하여반복되는난수에원본의sample루프가끝없이머무르는상태를거절합니다. 생성문자/단어수/깊이/기본변환예산초과는부분출력을반환하지않습니다.

새oracle는원본어휘를검증후Rust자료로생성하고37stem×3seed×2syntax×2filter의444사례를동일LCG 난수로실제원본확장합니다. seed는검사전용이며제품난수고정에사용하지않습니다. 기본30/0→1·단어수/범위/잘못된이름·ru/sp/알수없는언어·반복/중첩/child/속성제거·BEM/주석과native스니펫문법을비교합니다.

markup-lorem-first.log와markup-lorem-parity.log는검사호출의인자누락·잘못된모듈import로compile exit101/실행0건입니다. 실제snippet_syntax::ParseLimits/parse 계약으로수정한markup-lorem-parity-final.log는98864 handle에서compile4.44초·lorem3(444사례/비lorem 난수0/정체·생성예산)/0.02초·필터3/0.01초·출력4/0.05초·준비2/0.02초의12통과·0실패·exit0으로종료했습니다. 기존전체322+독립필터3+새lorem3의서로다른현재328성공/기존성능1ignored로재사용하며최신12를중복합산하지않습니다.

markup-lorem-host.log의현재lib/Emmet7target host check17.69초와markup-lorem-wasm.log의현재corelib wasm32 check4.55초는73717 handle에서exit0/추가경고0으로종료했습니다. 최종8Rustfmt·oracle MJS/QA포맷·어휘/444fixture의동일입력재생성·소유diff/manifest·lock·frozen변경0·디스크630GiB/66%를확인했습니다. e3b6 코어범위만완료이며CSS/후보/실제App/editor-80unwired·전체302/588(51.4%)·미완료286/목표active·기존App준비/Wasm시간API/성능/실기게이트는미완료입니다.

소유lorem core/생성어휘·난수API/검사/oracle/MIT출처·QA와PROCESS상단12경로를5ce6a2ba로선별커밋했습니다. 기능표baseline/lorem 근거만갱신하고editor-80unwired·전체302완료를유지합니다. 사용자PROCESS하단2hunk와다른문서/결정/피드백/메모리는보존했습니다.

## e3b7 — 문자열·필드 소비 경계

새boundaries oracle는현재원본helper의실제출력을보존합니다. 최초MJS의String.raw 안에필드보간문법이있어generator syntax exit1/생성0건이었고일반문자열로고쳐30입력/60사례를생성했습니다. boundaries-red.log는compile8.98초·0통과/1실패/exit101로속성name의field literal처리와기본값cost$5·escaped field/닫힘의실제차이를재현했습니다.

ExpandedMarkupAttribute.name_fields는변환때의진짜field token만보존합니다. 직렬화된name으로부터리터럴/field를다시추측하지않습니다. 원본name field의인덱스/일반value offset을유지하고최종0이이미있으면추가0으로덮어쓰지않습니다. literal출력은백슬래시와달러를snippet문법으로encode하고field default의일반달러/닫힘brace와중첩field의구조를분리했습니다. converter는field default의escaped brace를닫힘으로오인하지않습니다. 일반속성의double quote는HTML entity로,JSX styles의문자열key는single quote/backslash/줄바꿈을JS문법으로encode한뒤snippet encode합니다.

boundaries-parity.log는compile13.33초·새경계0통과/1실패와기존변환4/필터3/lorem3/출력4/준비2의16통과·exit101입니다. 수정된escape출력을원본의손실있는raw문자열과동일하게요구한초기assert는별도consumer비교로바꿨습니다. raw원본fixture는유지하고리터럴${1}→${0} 손실·백슬래시/달러·plain brace/escaped closing·깨진JSX key/속성quote의기대본문만corrected_text의명시적입력목록에기록했습니다. native snippet_syntax와snippet_expansion을실제로통과시켜출력표기차이가본문/placeholder에미치는결과를비교합니다. 슬롯이름1/value3/최종0과cost$5가추가슬롯이되지않음도직접단언합니다.

추가최종0 name/nested plain-brace 사례를포함한32입력/64사례의boundaries-final.log는43131 handle에서compile18.33초·새경계2/0.00초·변환4/0.01초·필터3/0.01초·lorem3/0.02초·출력3/1실패/0.05초·준비2/0.02초의17통과/1실패·exit101으로종료했습니다. 실패는기존1840출력oracle의title=\"a\"b\"를그대로요구한두syntax 사례입니다. 원본오류값을정확히단언한뒤native title=\"a&quot;b\" 기대값으로명시수정했습니다. boundaries-quoted-final.log는85334 handle에서해당1검사만재실행해compile4.40초·1통과/3filtered·0.03초·exit0으로종료했습니다. 현재변경직접18성공과이전전체322/필터3/lorem3/새경계2의서로다른330성공을재사용하고반복성공은더하지않습니다. 현재기본1840사례는1834원본raw일치와괄호/literal/속성quote의명시적6수정을포함합니다.

lorem 구현/기능표커밋5ce6a2ba/9ebfa0e4는일반push/local·remote0/0으로확인했습니다. boundaries-host.log의현재lib/Emmet8target compile17.88초와boundaries-wasm.log의현재corelib wasm32 compile4.73초는83847 handle에서exit0/추가경고0으로종료했습니다. 최종5Rustfmt·oracle MJS/QA포맷·소유diff/manifest·lock·frozen변경0·디스크630GiB/66%를확인했습니다. e3b1~7의HTML/JSX 코어확장만닫고e3/CSS·range/validation/추가후보/설정/실제App은미완료이며editor-80unwired·전체302/588(51.4%)·미완료286·목표active·잔여시간미산정을유지합니다.

소유typed name-field/출력escape·원본/consumer 검사·oracle와QA/PROCESS상단8경로를b55327d0로선별커밋했습니다. 기능표baseline/64경계근거를갱신하고editor-80unwired·전체302완료를유지합니다. 사용자PROCESS하단2hunk와다른문서/결정/피드백/메모리는보존했습니다.

## e3c1 — stylesheet AST

[공식 CSS 축약식 문서](https://docs.emmet.io/css-abbreviations/)와설치@emmetio/css-abbreviation2.1.8 README/type계약·현재emmet2.4.11의동일tokenizer/parser를대조했습니다. CSS/scss의숫자/raw/unit·RGBA/raw color·string quote·literal/custom property/field/variable·function arguments와property name/값fragment/important를portable typed AST로연결했습니다. byte/token/depth예산·finite 숫자·닫힘오류를검증하고JS런타임/새의존성은추가하지않았습니다.

원본순서의부호/단위없는값뒤dash구분·함수앞literal/number 병합·value모드·NBSP/ASCII whitespace·partial quote/open function의기존parser관용을유지합니다. 임의로정상CSS 선언parser로대체하지않았으며이AST는축약식해석전단계입니다. parse_stylesheet는명시적value모드와기존MarkupLimits의구조예산을받습니다. 실제공급자/설정/완성·속성keyword/fuzzy·기본값/단위/색상형식·CSS출력은없습니다.

설치253 stylesheet snippet의키/숫자추가/정의본문과수동경계의813입력×2모드/1626사례를원본으로생성했습니다. stylesheet-parser-first.log는16068 handle에서compile9.95초·원본AST/예산의2통과·0실패·0.01초·exit0으로종료했습니다. 동일기존330성공과새CSS2의서로다른현재332성공/기존대형성능1ignored로재사용하며1626을검사수로더하지않습니다. stylesheet-parser-host.log는현재lib/새target compile9.05초·stylesheet-parser-wasm.log는corelib wasm32 compile4.84초이며72737 handle에서모두exit0/추가경고0으로종료했습니다. 최종4Rustfmt·oracle MJS/QA포맷·소유diff/manifest·lock·frozen변경0·디스크629GiB/66%를확인했습니다. e3c1 AST만닫고editor-80unwired·전체302/588(51.4%)·미완료286/전체목표active·CSS해석/출력/후보/실제App/전체게이트는유지합니다.

소유CSS typed parser/export/검사/oracle·MIT출처와QA/PROCESS상단8경로를78bd5889로선별커밋했습니다. 기능표baseline/CSS AST근거만갱신하고editor-80unwired·전체302완료를유지합니다. 사용자PROCESS하단2hunk와다른문서/결정/피드백/메모리는보존했습니다.

e3c 출력의원본outputValue를대조하면서field.start===previous.end일때공백을생략하는의존성을확인했습니다. byte/UTF-16 절대위치대신StylesheetValue::Field.is_adjacent로동일fragment 안의직전값과붙어있는지보존합니다. 원본FunctionCall은end가없어뒤field와자동결합하지않으며같은동작을유지합니다. 기본canonical도이플래그를비교하도록확장하고숫자+field/구분자/함수내·함수뒤/연속field의다섯입력을추가해818입력/1636사례를생성했습니다. stylesheet-parser-adjacency.log는35395 handle에서compile9.20초·영향2검사/0.00초·exit0으로종료했고이전2성공을추가합산하지않습니다. stylesheet-adjacency-host.log/currentlib·CSS target compile8.54초와stylesheet-adjacency-wasm.log/corelib4.39초는14156 handle에서모두exit0/추가경고0으로종료했습니다. 최종3Rustfmt·oracle MJS/QA포맷·소유diff/manifest·lock/frozen변경0을확인했습니다. 초기AST자료와기능표는78bd5889/f80c0f85까지일반푸시했고local·remote0/0이며e3c1a만닫고전체CSS출력/App·editor-80unwired/302완료·목표active를유지합니다.

소유CSS field 인접성/core 검사/oracle와QA/PROCESS상단6경로를981da085로선별커밋했습니다. 기능표baseline/1636인접성근거만갱신하고editor-80unwired·전체302완료를유지합니다. 사용자PROCESS하단2hunk와다른문서/결정/피드백/메모리는보존했습니다.

981da085/dc3c4581은일반push/local·remote0/0으로확인했고작업트리에는사용자문서/피드백/메모리만남았습니다. 다음e3c2의원본createSnippet/nest/resolveNode·keyword/fuzzy/gradient/numeric와실제resolveConfig의선별옵션을확인했습니다. 기본253registry·unitAlias e/em,p/%,x/ex,r/rem·정수px/소수em·무단위8속성·global keyword auto/inherit/unset/none·fuzzy 최소0/strict false입니다. source동작에맞는registry/준비AST oracle/예산을먼저고정하며CSS출력·후보·실제App과기존전체게이트는미완료입니다.
