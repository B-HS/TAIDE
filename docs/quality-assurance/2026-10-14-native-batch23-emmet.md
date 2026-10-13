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
