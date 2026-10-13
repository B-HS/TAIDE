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
