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
