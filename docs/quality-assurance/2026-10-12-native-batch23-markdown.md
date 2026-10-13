# 배치 23 — Markdown 미리보기 분할

원본 `src/widgets/editor-pane/editor-pane.tsx`의 languageId 게이팅·Columns2 토글·stable editor 패널·양쪽 최소 20%/최초 50% 분할과 `use-editor-file-persistence.ts`의 200ms 최신 본문 갱신, `use-editor-markdown-preview.ts`의 pane 내 상태·cleanup을 확인했습니다. `src/features/editor/markdown-preview.tsx`는 독립 세로/가로 스크롤과 Markdown/GFM 및 DOMPurify로 정제한 HTML을 표시합니다. 편집기는 토글 때 재생성되지 않으며 저장 전 본문을 미리보기에 사용합니다.

기존 native에는 앱 토글/분할 소비자가 없었습니다. LSP 문서용 `editor-markup.rs`의 CommonMark/GFM parser·rich renderer와 프로젝트/경로/이미지 용량을 검증하는 기존 이미지 cache, 검증된 링크 열기 포트를 재사용합니다. 설치된 pulldown-cmark 0.13.4의 Parser/Options 문서와 egui 0.36.2의 UiBuilder ID·Label·WidgetInfo/포인터/ScrollArea 공식 소스를 확인했습니다. 새 dependency/feature/manifest/lock/engine/frozen source는 추가하지 않습니다.

`editor-markdown.rs`는 viewport/pane별 토글/폭과 현재 문서/revision/대기 snapshot을 소유합니다. 언어가 Markdown일 때 기존 locale 메시지의 접근성 레이블/선택 상태를 가진 Columns2 형태 버튼을 표시합니다. 편집기 child의 ID는 기존 parent UI ID를 유지하고 미리보기는 별도 UI/스크롤 영역에 그립니다. 동일 문서 편집은 quiet period 후 최신 snapshot만 소비하고 문서 교체는 즉시 표시하며 비 Markdown 전환·토글 해제·비활성 소스/닫힘·pane 제거 때 이전 본문/이미지 요청을 정리합니다. readonly와 기존 편집/찾기/구문/LSP/메뉴·view state 포트는 그대로 소비합니다.

첫 컴파일은 resizerThickness의 u32/f32 인자와 새 검사의 존재하지 않는 SelectionSet::single/누락 folds 인자 3개로 exit101이었습니다. 실제 타입/API를 사용하도록 작성한 코드를 수정했습니다. 첫 UI 검사는 egui TexturesDelta를 소비하지 않은 검사 준비 오류 0통과/1실패·0.05초·exit101이었으며 소유 출력의 texture delta를 clear했습니다. 이를 제품 실패나 성공으로 집계하지 않습니다.

`/private/tmp/taide-batch23-markdown-integration.log`는 실제 App의 Markdown 파일 열기→버튼 클릭→분할/제목→동일 ViewId/입력 ID/선택 유지→실제 본문 입력→최신 미리보기→디스크 원문 보존/정상 종료 task0, UI 언어/폭/상태/삭제와 기존 plain/code parser 검사의 서로 다른 3통과·74.84초·exit0입니다. 컴파일은 19.15초였습니다. owned process 조회 당시 실행 중이었으며 뒤의 sample 요청 시 이미 종료되어 exit255였고 스택/원인 결론은 없습니다. 기한/OS/캐시/검사 프로세스 재시작은 하지 않았습니다.

이후 원본과 같은 아이콘/접근성 선택 상태, editor 본문 색상, 현재 탭과 모델 수명에 따른 source/image 정리, 실제 undo·다른 Markdown 전환/닫힘 검증을 보강했습니다. lifetime.log는 UI/기존 parser 2통과와 App 1실패·43.54초·exit101입니다. 긴 Text 입력은 기존 native의 여러 입력 undo 묶음을 생성하여 한 번의 undo가 마지막 단어만 제거했으므로 전체 교체를 한 번에 되돌린다는 검사 가정이 틀렸습니다. owned egui Paste 이벤트로 실제 전체 선택 교체/undo 경계를 검사하며 실제 OS 클립보드는 접근하지 않습니다.

lifetime-fixed.log는 UI 두 검사와 기존 parser의 3통과·App 1실패·41.24초·exit101입니다. 실제 버튼/20% 최소폭 드래그/키보드 조절과 편집/Paste/undo·다른 Markdown 즉시 전환/첫 닫힘은 통과했습니다. 마지막 수정 이력이 있는 문서를 raw Host discard:true로 호출하여 source closed 준비가 실패했습니다. 코드에서 확인한 기존 release_tab은 UI에서 승인한 revision 없이 discard할 때 revision0만 허용하므로 revision2 문서를 폐기하도록 작성한 fixture가 계약을 어겼습니다.

app-close-fixed.log는 undo 후 dirty false라는 검사 가정이 44.81초·0통과/1실패·exit101로 실패했습니다. 기존 EditorStore::restore_history는 본문을 되돌린 뒤에도 requires_save=true로 두며 이 dirty 계약은 이번 구현에서 변경하지 않았습니다. 수정 이력이 있는 문서는 기존 App request_tab_close와 주입한 CloseChoice::Discard를 통해 현재 revision의 approved_discard를 생성하고 원래 닫힘/회수 경계로 확인합니다. raw Host 폐기 보호를 우회하거나 테스트용으로 본문/dirty/승인 revision을 강제로 수정하지 않습니다. App 실패 한 건만 재검사하며 UI 두 검사/기존 parser의 같은 입력 성공은 재사용합니다.

app-close-dialog.log는 최종 actual App1통과·41.79초·exit0(컴파일9.07초)입니다. 실제 Markdown 토글·동일 ViewId/입력 ID/선택·본문 입력/Paste/undo/최신 본문·다른 Markdown 즉시 전환/닫힘과 기존 앱 닫힘 절차의 주입 선택→현재 revision 승인→source/model 회수·원문/정상 종료 task0이 통과했습니다. 수정하지 않은 UI2/기존 parser1의 성공을 재사용한 서로 다른4건이며 준비/작성 실패와 이전 중복 App 성공은 합산하지 않습니다. readonly-regression.log의 기존 lossy/readonly 새 뷰 위치 복원·선택만 이동·본문/소유 보존이1통과·36.53초·exit0(컴파일0.26초)입니다. 현재 서로 다른 실제 App2/UI2/기존 parser1의5건이 통과했습니다. Markdown readonly/mirror·추가 뷰/독립 스크롤과 전체App 게이트의 완료로 확대하지 않습니다. 소유6Rust fmt와 관련 diff는 exit0입니다.

editor-72 전체는 아직 완료가 아닙니다. 기존 parser가 생략하는 raw HTML, 상대 URL/이미지·프리뷰 이미지 max-width, 원본 표/인용/코드 블록 표시와 추가 readonly/mirror/독립 스크롤/실제 divider 검증을 이어갑니다. 해당 기능은 partial이며 전체App 준비 실패·실제 OS/다중 창/접근성/성능/신뢰성/출시 게이트를 유지합니다. 현재 기능 완료 302/588(51.4%)·미완료286·배치23 잔여19/최종33이며 전체 전환율/신뢰할 잔여 시간은 미산정입니다.

제품/검사/QA와 PROCESS 상단 두 소유 hunk의8경로를0c2b26e3으로 선별 커밋했습니다. author/최종 메시지에 AI 트레일러가 없고 사용자 문서/PROCESS 하단 두 hunk를 제외했습니다. 기능표599행/413근거 경로·302완료/90부분/110미연결/86미구현·588대상과 원래302 ID 배정의 누락/중복/미확인ID0·잔여19/52/23/64/32/39/51/6·최종33을 실제 JSON으로 확인했습니다.

## HTML 구조·원본 경로/이미지와 오류 복구 (2026-10-12)

이전 골 턴은 분할 코드/검사/문서의 권위 상태를 변경하고 선별 커밋·일반 푸시한 progress입니다. 현재 editor-72의 전체 요구사항을 유지하고 남아 있던 HTML/상대 경로·이미지 폭을 구현했습니다. 기존 pulldown-cmark/dom_query/RichDocument와 이미지/링크 포트를 재사용하며 새 dependency/manifest/lock/engine/frozen 변경은 없습니다.

새 editor-markdown-parse.rs는 CommonMark/GFM의 HTML 출력을 기존 HTML preview 문서 예산 안에서 구성하고 editor-markdown-html.rs는 문서 요소를 native 구조로 변환합니다. HTML 제목/strong/em/del/code·인용/중첩 순서·체크 목록/표/이미지·링크와 파일의 canonical URI 기준 상대 경로를 보존합니다. 실행 요소·위험 URI·hidden 요소를 native 구조에서 제외하고 일반 text/속성은 HTML escaping과 기존 DOM 파서로 처리합니다. 이미지 fetch/open file는 새 권한을 만들지 않고 기존 프로젝트/경로/용량/취소 경계로 소비합니다. LSP parser는 동작을 바꾸지 않고 GFM autolinks만 같은 구현을 재사용합니다.

HTML 출력/할당의 기존64MiB와 DOM 중첩 예산을 초과하면 부분 출력이나 panic 대신 typed localized 오류를 반환합니다. 미리보기의 이전 image 요청/출력은 정리하며 편집기 본문을 유지합니다. editor.markdownPreviewError를 기존 en/ko/ja와 required key registry에 추가해 구현 오류 문구가 사용자 화면에 나오지 않게 했습니다. 다음 편집/undo의 정상 내용은 원래 debounce/문서 수명으로 복구합니다. 이미지 캐시의 기존 show는 폭 제한 없이 유지하고 Markdown만 available width에 맞춰 종횡비를 보존합니다.

html-red.log는 기존 parser에서 HTML 제목 부재를0통과/1실패·0.04초·exit101(컴파일20.15초)로 재현했습니다. html-green.log는 새 DOM 루트 html 요소를 인라인으로 처리해 구조가 평탄화되는 제품 구현 실패0/1·0.03초·exit101(컴파일40.43초)이며 fragment 루트를 블록 컨테이너로 처리해 수정했습니다. html-fixed.log는 기본 HTML/경로·GFM/표/코드/체크 목록·위험 URI/hidden/escaping·중첩/출력 상한4통과·0.01초·exit0(컴파일21.22초)입니다. 이후 localized 오류/검사 이름/문구 상태의 final-parser.log4통과를 최종 근거로 사용하며 같은 성공을 중복 합산하지 않습니다.

html-app.log1통과·31.84초·exit0(컴파일25.19초)은 실제 임시 MD/PNG의 HTML 제목·relative PNG 로딩·실제 image rect의 폭/비율과 기존 입력/Paste/undo·즉시 문서 전환/닫힘/원문/task0입니다. app-error-recovery.log1통과·30.14초·exit0(컴파일39.13초)에 깊은 HTML의 localized 오류·본문 보존·undo 복구를 추가했습니다. 원문 영역도 같은 제목을 포함하므로 마지막 app-preview-surface.log에서는 오른쪽 preview rect/clip 안의 정확한 galley text만 검사하여 실제 HTML 제목·오류/복구를 입증했습니다. 최종1통과·30.16초·exit0(컴파일14.62초)이며 앞선 App 중복 성공은 합산하지 않습니다. 실제 OS/보호 앱/클립보드는 접근하지 않았습니다.

image-regression.log는 기존 도움말의 data/base64/percent SVG·width/height·root 격리/중복·HTTP pixel/닫힌 대기 취소·GIF/전체 texture 예산·project/task 회수의4통과·0.21초·exit0입니다. locale.log는 키 집합/required key/보간·세 언어·외부 파일/사용자 팩/캐시/경로 보호의 전체20통과·0.04초·exit0입니다. 현재 App1/parser4/images4/locale20의 서로 다른29건이며 이전 UI/readonly와 해당 시점의 실패/중복은 이 수에 더하지 않습니다. 소유10Rust fmt와 locale fmt/diff는 exit0입니다.

원본 DOMPurify의 설치된 tags.ts에는 details/summary·style·SVG·MathML·audio/video가 허용되어 있습니다. 현재 native 변환기의 일반 HTML 텍스트/구조 지원만으로 이를 완료로 볼 수 없습니다. CSS·SVG/MathML/미디어·details/anchor·음수 ordered start/표 병합 등 추가 raw HTML 의미와 원본 paragraph/heading/quote/table/code/scroll 표시·readonly/mirror/추가 뷰를 d6c/d6b의 미완료 범위로 유지합니다. HTML/media의 제한된 Rust 소유 WebView 경계와 플랫폼/구조·cutover 미결은 기존 합의를 보존하며 임의로 제거·확정하지 않습니다. editor-72 partial·302/588(51.4%)·미완료286·배치23 잔여19/최종33·전체App/실기/성능/출시와 전체 목표는 유지합니다. 전체전환율/신뢰할 잔여 시간은 미산정입니다.

HTML/이미지/오류 코드·카탈로그/검사/QA와 소유 PROCESS의16경로를40de51de로 선별 커밋했습니다. 최종 메시지/author에 AI 트레일러가 없고 사용자 문서와 PROCESS 하단 두 hunk를 제외했습니다. 현재 기능표599행/416근거 경로·302완료/90부분/110미연결/86미구현·588대상과 원래302 ID 배정의 누락/중복/미확인ID0·잔여19/52/23/64/32/39/51/6·최종33을 실제 JSON으로 확인했습니다.

## 원본 표시와 앱 테마/글꼴 갱신 (2026-10-12)

이전 골 턴은 기본 HTML/경로/이미지·오류 복구와29건 검증·선별 Git으로 권위 상태를 바꾼 progress입니다. 현재 원본 markdown-preview.tsx와 설치된 Tailwind theme.css의 rem 크기/line-height/leading-relaxed, global.css의 font·원본 CSS spacing/색상을 대조했습니다. 프로젝트/환경/스택을 바꾸지 않고 현재 egui 소스로 Frame/ScrollArea/Label/WidgetInfo와 MouseWheel의 TouchPhase::Move를 확인했습니다.

기존 일반 호버 renderer는 편집기 글꼴의 배율로 제목을 그려 원본 Markdown의 독립 제목/본문 크기와 달랐습니다. display-red.log는 편집기20 크기의 제목을 원본24/20/18로 그리는 실제 UI 실패0/1·0.02초·exit101(컴파일36.75초)입니다. preview 전용 editor-markup-preview.rs와 show_preview를 추가하고 inline renderer만 재사용했습니다. 일반 hover show의 배율/여백/코드/표 계약은 유지하며 preview의 고정 본문/제목·leading-relaxed·문단/제목/규칙 여백·인용 left border/opacity·셀 border/padding·인라인 코드 rounded padding·줄바꿈 없는 pre의 독립 horizontal ScrollArea를 연결했습니다. Markdown 바깥 margin은 원본 px6/py4입니다.

별도의 editor_markdown_colors는 editor background/foreground와 app.border/app.accent/appSidebar.itemHover를 사용합니다. 이전 hover/widgetBorder/terminal.linkForeground/panel 색상을 그대로 사용하는 차이를 제거하고 Appearances/초기 생성/설정·theme reload/preview theme 두 갱신 소비에 연결했습니다. 실제 Markdown 미리보기 font는 편집기 font size/mono 설정과 분리하며 코드는 기본 Monospace를 사용합니다.

첫 green.log의 제목1통과·0.02초·exit0 이후 surface.log/surface-fixed.log는 새 검사 작성에서 MouseWheel.phase 누락/없는 Moved variant 때문에 각각 컴파일 exit101·0검사였습니다. 설치된 Event/TouchPhase 소스의 필수 Move를 사용했습니다. surface-phase.log는 일반 문서 UI13/preview2/theme3의18통과입니다. 인라인 코드 padding을 공유 leaf에 추가한 뒤 display-inline.log의 UI13/preview2가 통과했고 theme3의 변하지 않은 성공은 재사용합니다. 일반 hover/서명·IME/readonly/mirror·provider/focus/포인터·이미지 링크/크기 조절을13건에서 확인했습니다. display-code-link.log는 실제 둥근 인라인 코드 배경/링크 버튼의 pointer press/release→검증된 target 전달1통과·0.01초·exit0(컴파일4.45초)입니다. 현재 UI13/preview3/theme3/App1의 서로 다른20건이며 앞선2건/반복 성공/작성 실패는 중복 합산하지 않습니다.

실제 display-app.log는 임시 settings의 editor font30/기존 vscode-light-modern theme 전환을 기존 presentation request/reply로 소비하고 실제 오른쪽 preview heading24·현재 foreground·palette 갱신을 확인했습니다. 기존 Markdown/HTML/PNG 폭·비율·입력/Paste/undo/오류·복구/문서 전환/승인 revision 닫힘·원문/task0도1통과·31.94초·exit0(컴파일32.10초)입니다. 실제 사용자 설정/OS/보호 앱/클립보드는 접근하지 않았습니다. 소유9Rust fmt·diff와 manifest/lock 변경0을 확인했습니다.

full HTML의 CSS/SVG/MathML/media/details/anchor·음수 목록/표 병합, 표의 모든 cell 경계/크기·추가 readonly/mirror/독립 scroll/뷰·실제 OS/pixel/접근성/성능/출시는 계속 미완료입니다. 현재 native 기본 표시를 이 전체 범위의 완료로 확대하지 않습니다. editor-72 partial·302/588(51.4%)·미완료286·배치23 잔여19/최종33·전체 목표를 유지하고 전체전환율/신뢰할 잔여 시간은 미산정입니다.

표시/테마 코드·검사/QA와 소유 PROCESS의11경로를c0f4fd83으로 선별 커밋했습니다. author/최종 메시지에 AI 트레일러가 없고 사용자 문서/PROCESS 하단 두 hunk를 제외했습니다. 기능표599행/419근거 경로·302완료/90부분/110미연결/86미구현·588대상과 원래302 ID 배정의 누락/중복/미확인ID0·잔여19/52/23/64/32/39/51/6·최종33을 실제 JSON으로 대조했습니다.

## 표 경계와 signed 순서 목록 (2026-10-12)

이전 골 턴은 원본 표시/테마 소비와20건 검증·선별 Git으로 권위 상태를 변경한 progress입니다. 현재 native 표의 같은 행에서 서로 다른 내용 높이에 따라 cell bottom이48/88로 갈라지는 제품 실패를 table-red.log0/1·0.01초·exit101로 확인했습니다. 기존 Grid의 본문/행 배치는 유지하되 모든 cell response 수집 후 행/열 경계를 정규화하고 공유 edge 좌표를 한 번만 그려 collapsed border와 일관된 높이/폭을 연결했습니다. 독립 색상/opacity에서 중복 경계의 누적을 피하며 본문·클릭/선택을 바꾸지 않습니다. warmup 후 서로 다른 폭/줄바꿈3행2열에서 모든 수평/수직 segment가 같은 행/열 좌표를 가지고 각 공유 edge가 한 번만 존재하는 실제 shape 검사를 수행했습니다.

ordered-red.log0/1·0.03초·exit101은 ol start의 음수가 u32 파서에서1로 바뀌는 실제 실패입니다. [WHATWG ol start](https://html.spec.whatwg.org/multipage/grouping-content.html#the-ol-element)와 [정수 파싱 규칙](https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#rules-for-parsing-integers)을 확인했습니다. ASCII whitespace/부호/첫 digit sequence와 실패 기본값을 처리하며 HTML/Markdown list의 start를 공통 RichDocument에서i64로 나타냅니다. LSP의 기존 양수 start는 범위 검증 변환을 거쳐 같은 값을 유지합니다. 두 UI consumer가 공통 list_ordinal을 사용하여 음수→0→양수와 큰 번호/인덱스 overflow를 안전하게 처리합니다. 역순/reversed·li value/type·span/CSS 등의 전체 HTML 의미는 이번 기본 signed start 구현의 완료로 대체하지 않습니다.

표 수정 뒤 grid-fixed.log는 기존 preview/인라인 code/link/quote/table/pre scroll과 새 표 경계4통과·0.05초·exit0(컴파일8.93초)입니다. signed 변경 뒤 ordered-ui.log는 preview5/일반 hover13의18통과·preview0.06초/hover0.21초·exit0입니다. ordered-fixed.log는 기본 HTML/GFM/보안/상한과 signed start5통과·0.02초·exit0이고 이후 whitespace/sign/digit tail/실패 사례를 더한 final-parser.log5통과·0.01초·exit0를 현재 근거로 사용합니다. 같은 성공/재검사를 중복 집계하지 않습니다.

ordered-app.log는 실제 임시 Markdown의 signed -1/0 목록과 다중 높이 HTML 표·현재 theme/editor font→오른쪽 preview 표시, 원문·HTML/PNG/폭·비율·입력/Paste/undo/오류/복구/문서 전환/승인 revision 닫힘/task0의1통과·32.19초·exit0(컴파일16.20초)입니다. 원본 문자열이나 모델만으로 화면을 대신하지 않고 preview rect/clip의 정확한 galley label을 확인했습니다. core-docs.log는 공통 호버/서명·UTF16/원문 범위·종류/trigger의7통과·0.00초·exit0이고 lsp-parser.log는 기존 plain/코드/강조/목록·GFM link/이미지/표·비신뢰 HTML/명령/상대 URL 제외의8통과·0.00초·exit0입니다. UI18/App1/parser5/Core7/LSP parser8의 서로 다른39건이며 일반색상/이미지 캐시·locale의 변경 없는 이전 성공은 추가 합산하지 않습니다.

공통 start 타입 변경의 동결 host tests 포함 check에서 이전 표시 변경의 editor_markdown_colors가 native-host 모듈을 무조건 참조하는 E0433 두 오류·exit101을 발견했습니다. 기존 editor_documentation_colors와 같은 cfg native-host 경계를 helper/해당 native 검사에 적용했습니다. 동결 feature 강제 활성화나 module 경계 완화로 우회하지 않으며 host-fixed.log는 tests 포함22.39초·exit0이며 frozen-wasm.log는 canvas feature의 wasm32-unknown-unknown lib1분27초·exit0입니다. Wasm tests의 기존 std/web Instant 불일치 부채는 실행하거나 완료로 바꾸지 않았습니다. portable-theme.log는 UI no-default-features의 기존2통과·0.00초·exit0(컴파일40.06초)입니다. 현재 UI18/App1/parser5/Core7/LSP parser8/portable2의 서로 다른41건과 frozen host/Wasm compile이 통과했습니다. 최초 E0433/단위 실패와 변경 없는 성공/중복을 따로 보존합니다. 소유10Rust fmt/diff·manifest/lock 변경0을 확인했습니다. 실제 OS/보호 앱/사용자 설정/클립보드는 접근하지 않았습니다. 표의 모든 data/viewport/이미지·cell span/column sizing·HTML reversed/value/type/CSS/SVG/MathML/media/details/anchor·추가 readonly/mirror/독립 scroll/뷰와 전체App/실기/접근성/성능/출시는 미완료입니다. editor-72 partial·302/588(51.4%)·미완료286·배치23 잔여19/최종33·전체 목표를 유지하고 전체전환율/신뢰할 잔여 시간은 미산정입니다.

표/목록/feature 경계 코드·검사/QA와 소유 PROCESS의12경로를497d5d43으로 선별 커밋했습니다. author/최종 메시지에 AI 트레일러가 없고 사용자 문서/PROCESS 하단 두 hunk를 제외했습니다. 기능표599행/419근거 경로·302완료/90부분/110미연결/86미구현·588대상과 원래302 ID 배정의 누락/중복/미확인ID0·잔여19/52/23/64/32/39/51/6·최종33을 실제 JSON으로 확인했습니다.

## 읽기 전용·동일 문서 두 뷰·독립 스크롤과 창 상태 회수 (2026-10-12)

원본 MarkdownPreview의 독립 overflow 영역과 pane별 토글 수명, 실제 Host OpenFileTab/OpenFileToSide 및 설치된 egui ScrollAreaOutput/RawInput.viewports 계약을 대조했습니다. App 검사는 소유 임시 Markdown을 실제 Host로 두 영역에 열어 같은 DocumentId와 서로 다른 ViewId를 만들었습니다. 실제 버튼·키·wheel/Paste/undo와 승인 revision 닫힘 경로를 사용하며, preview scroll 관찰은 cfg(test)에서 실제 ScrollAreaOutput의 offset/inner_rect를 저장합니다. 입력 동작을 대체하거나 제품 대기 시간/검사를 늘리지 않았습니다.

두 뷰의 독립 토글/선택, preview wheel이 두 본문/다른 preview를 움직이지 않음, 본문 wheel이 두 preview/다른 본문을 움직이지 않음, mirror Paste/문서 변경과 undo의 공유를 확인했습니다. 먼저 그려진 뷰는 다음 프레임에 새 revision을 관찰하므로 그 뒤 quiet period를 만료시킵니다. 한 뷰를 닫아도 같은 DocumentId의 남은 뷰가 유지되고 최종 닫힘/디스크 원문/작업0도 확인합니다. 별도 invalid UTF-8 Markdown은 실제 파일 admission의 lossy/read_only로 열고 preview 표시/선택 이동/독립 scroll을 허용하면서 Paste/Text 입력·revision/dirty/메타데이터/원문 bytes를 보존합니다.

검사 작성의 views-initial.log는 DocumentSnapshot의 실제 metadata/dirty 필드 대신 존재하지 않는 직접 필드를 사용해 E0609 6오류·exit101·0검사였습니다. views-app.log는 공유 ScrollPosition의 clone 누락 E0507·exit101·0검사이며 실제 타입 정의로 수정했습니다. views-runtime.log는 readonly/기존 App2통과·두 뷰1실패·47.63초·exit101(컴파일25.65초)입니다. 제품 실패가 아니라 다음 프레임 이전에 pending snapshot을 가정한 검사 순서 오류이며 다음 관찰 프레임을 추가한 views-observed.log는 두 뷰1통과·32.30초·exit0(컴파일14.08초)입니다. 이 성공과 반복 결과는 중복 합산하지 않습니다.

viewport-red.log는 같은 pane을 유지한 채 egui child viewport가 사라져도 State.entries가 남는 제품 실패0/1·0.03초·exit101(컴파일17.55초)입니다. 기존 reconcile은 pane 존재만 확인했습니다. 현재 raw.viewports에 없는 소유자를 먼저 회수하고 기존 이미지 retain_documents를 이어 사용하여 살아 있는 preview의 자산만 유지합니다. App 호출도 현재 Context를 전달합니다. viewport-fixed.log는 기존 최소폭 drag/keyboard·언어/폭/상태와 새 viewport 회수의3통과·0.03초·exit0(컴파일21.08초)입니다. root의 독립 상태를 유지하며 사라진 child의 상태를 회수합니다. 이 검사는 실제 egui viewport 목록/프레임을 사용한 상태 검사이며 물리 OS 창의 생성/종료 검증과 구분합니다.

views-final-app.log는 회수 수정 뒤 readonly/두 뷰/기존 토글·theme·HTML/PNG·최신 입력/undo·오류 복구/문서 교체/닫힘의 실제 App3통과·46.49초·exit0(컴파일20.39초)입니다. 두 뷰의 실제 선택 분리·닫힘을 추가해 최신 입력으로 확인했습니다. 현재 관련 상태3/App3의 서로 다른6통과이며 이전 성공/작성 실패를 추가 합산하지 않습니다. 소유4Rust fmt·diff는 통과했습니다. 실제 사용자 설정/OS/보호 앱/클립보드/시크릿은 접근하지 않았고 의존성/manifest/lock/Core/engine/frozen source를 변경하지 않았습니다.

views-production-check.log는 inspection 없는 일반 App lib의 cfg(test) 밖 경계를 cargo check --locked --offline로 확인해2분37초·exit0입니다. 공통 공개 계약/동결 소스 변경이 없어 이전 frozen host/Wasm compile 성공은 재사용하며 관련 없는 전체 검사를 반복하지 않았습니다.

d6b의 기능 수명 범위는 완료했습니다. 표 data/viewport/이미지/span·HTML reversed/value/type/CSS/SVG/MathML/media/details/anchor와 실제 OS 창/전체App 준비·접근성/성능/신뢰성/출시·전환/제거는 계속 미완료입니다. editor-72 partial·302/588(51.4%)·미완료286·배치23 잔여19/최종33이며 전체 전환율/신뢰할 잔여 시간은 미산정입니다. 전체 목표를 유지하고 이번 범위의 성공을 전체 기능 완료로 올리지 않습니다.

## 역순 목록과 항목별 번호 (2026-10-12)

창 상태 회수/뷰 수명과 검증 기록은4939d891/5697284b로 선별 커밋·일반 푸시했습니다. 사용자 문서와 PROCESS 하단 두 hunk는 제외했으며 현재 기능표599행/419근거·302완료/90부분/110미연결/86미구현·588대상, 미완료286의 배정 누락/중복/미확인ID0·배치23–30 잔여19/52/23/64/32/39/51/6·최종33을 확인했습니다.

[WHATWG ol](https://html.spec.whatwg.org/multipage/grouping-content.html#the-ol-element)과 [li 번호](https://html.spec.whatwg.org/multipage/grouping-content.html#the-li-element), 설치된 DOMPurify attrs의 reversed/value 허용을 대조했습니다. reversed-red.log는 역순 목록이 항목 수3 대신1에서 시작하는 제품 실패0/1·0.03초·exit101(컴파일15.97초)입니다. HTML 파서에 공통 정수 prefix 처리를 재사용하고 reversed의 기본 시작/감소 방향·명시 start·li value와 후속 번호·hidden 항목 제외·중첩/비순서 목록의 분리 범위를 연결했습니다. 기존 i64 번호 경계를 유지하며 범위를 넘는 값/전체 CSS list ownership/type 의미는 전체 HTML의 잔여 범위입니다.

공통 RichDocument.ListItem의 optional ordinal로 항목 번호를 표현합니다. 일반 LSP 파서는 None을 유지하며 기존 start/index 번호를 사용하고 HTML의 explicit ordinal은 일반 표시/preview 양쪽에서 소비합니다. 기존 LSP/구문 코드 캐시 fixture와 공개 타입 소비자를 실제 컴파일로 확인하며 새 의존성이나 동결 기능 변경은 추가하지 않았습니다. hidden을 제외한 직접 li 소유 범위와 중첩 분리의 native 구조이며 전체 CSS의 display:list-item/ownership 지원까지 완료한 것으로 확대하지 않습니다.

reversed-parser.log는 기존 기본 HTML/GFM/보안/상한·signed prefix와 새 reversed/boolean presence·start/value reset·음수/0·무효값·nested/hidden/비순서의7통과·0.02초·exit0(컴파일33.26초)입니다. reversed-ui.log는 일반 도움말13/preview6의19통과·0.20/0.07초·exit0(컴파일15.02초)이며 실제 shape의3/-4/-5를 두 renderer에서 확인했습니다. 기존 최소폭/표/코드/링크·IME/focus/readonly/mirror/provider/이미지 소비도 유지합니다.

reversed-app.log는 실제 임시 Markdown의 역순/value 번호3/-4/-5와 기존 signed -1/0, theme/preview 영역·HTML/PNG·원문/입력/undo/오류 복구/전환/닫힘/작업0의1통과·24.34초·exit0(컴파일0.27초)입니다. 원본 문자열이 아니라 오른쪽 preview의 실제 galley label을 단언했습니다. reversed-lsp.log는 기존 plain/code/GFM/목록/표/안전한 링크/이미지/비신뢰 HTML 제외의8통과·0.00초·exit0입니다. 현재 parser7/UI19/App1/LSP8의 서로 다른35통과이며 이전 readonly/두 뷰의 입력이 같은 성공은 재사용하고 중복 집계하지 않습니다. 소유9Rust fmt/diff도 통과했습니다.

공통 모델 변경의 frozen host는 reversed-frozen-host.log의 tests/inspection 포함 check26.52초·exit0, 실제 Wasm canvas lib는 reversed-frozen-wasm.log의 wasm32-unknown-unknown check12.65초·exit0으로 확인했습니다. 기존 전체App 준비 실패/성능 ignored/Wasm 시간 API 검사 부채·실제 OS/접근성/성능/신뢰성/출시·cutover/제거는 그대로 남겨두며 기능 강제 활성화/검사 생략/OS 변경으로 우회하지 않습니다. 의존성/manifest/lock/frozen source와 사용자 문서/PROCESS 하단 두 hunk를 보존했습니다. 표 data/viewport/이미지/span·list type/CSS ownership/범위 밖 번호·전체 CSS/SVG/MathML/media/details/anchor는 미완료이고 editor-72 partial·302/588(51.4%)·배치23 잔여19/최종33·전체 목표/예상 시간 미산정을 유지합니다.

## 표의 변경 프레임·자동 너비·이미지 배치 (2026-10-13)

이전 골 턴은 readonly/두 뷰/viewport 회수와 역순/value 번호를4939d891/5697284b/aa5f8b8d/4e801f0c로 구현·검증·일반 푸시한 progress입니다. 현재 요청도 전체 마이그레이션과 workflow/서브에이전트 사용하지 않음을 유지합니다. 원본 CSS table:auto/border-collapse 및 [CSS 2.2 자동 표 배치](https://www.w3.org/TR/CSS22/tables.html#auto-table-layout), 설치 egui Grid/UiBuilder/폰트 galley 계약을 읽고 최초/내용·행 수/viewport/이미지의 변경 프레임을 확인했습니다.

table-first.log는 기존 경계 검사에서 warmup3을 실제 첫 프레임1로 줄인1통과·0.01초·exit0(컴파일4.44초)입니다. table-change-red.log는 내용/행 수/좁힘·넓힘에서 두 열의 경계가3개 대신4개로 갈라지는 실패0/1·0.02초·exit101입니다. 변경 입력으로 새 Grid ID를 만드는 첫 수정의 change-fixed.log도 같은4/3 실패0/1·0.03초·exit101(컴파일16.98초)이고 coordinates.log는 x좌표0/44.84375/45.1875/88.09375·y0/88/436을 확인했습니다. Frame의 실제 글자 영역과 Grid가 배정한 셀 영역이 달랐으므로 cache key만 바꾸는 접근을 폐기했습니다.

표의 상태 ID를 고정하고 입력 fingerprint 하나를 저장합니다. 본문/스타일/정렬·화면 폭/scale·폰트 너비/이미지 준비·표시 크기가 바뀌면 기존 Grid 계약의 보이지 않는 너비/높이 측정으로 현재 배치를 준비한 뒤 표시합니다. 측정/표시 child의 explicit ID와 서로 다른 내부 pass ID로 표시 위젯의 선택·링크 ID를 유지하고 입력은 보이지 않는 측정에서 비활성입니다. 셀 경계는 글자 Frame이 아니라 실제 배정한 cursor 영역을 사용하며 공유 edge를 한 번만 그립니다. 새 내용마다 Grid 상태를 쌓거나 타이머/프레임 수를 늘리는 방식으로 우회하지 않습니다.

table-layout.log는 작성한 metrics closure가 mutable Ui를 move한 E0507/E0382 두 오류·exit101·0검사입니다. 즉시 수집하는 borrow로 수정한 layout-fixed.log는 내용/행 수/viewport·모든 경계와 실제 galley가 셀 안에 배치되는1통과·0.02초·exit0(컴파일11.73초)입니다. surface.log의 일반 도움말13/preview7 성공과 image1 실패는 bitmap 비율에 f32::EPSILON을 강제한 검사 정밀도 오류였습니다. 픽셀 정렬된 Response rect의1point 기준으로 확인한 image-pixels.log는 이미지 준비 None→작은 이미지→높은 이미지→작은 이미지, 경계/비율·행 높이 복원·변경 뒤 동일 상태 개수/실제 pointer press/release 링크1통과·0.01초·exit0입니다. 새 검사 이름의 대문자 ID warning도 snake case로 수정했습니다.

너비 비교의 table-width-red.log는560point 화면에서 한 줄로 표시할 문장을18줄로 접는 실제 실패0/1·0.01초·exit101(컴파일4.47초)입니다. 원본은 내용의 자연 너비를 사용하고 공간이 부족할 때 줄을 나눕니다. 공유 폰트 job 생성은 그대로 재사용하고 preview의 glyph/단어·공백/한중일 break와 이미지·inline code/cell padding에서 최소/자연 너비를 계산해 열에 배분합니다. 최대 너비가 들어가면 그대로 사용하고, 들어가지 않을 때는 최소 너비를 보존하며 축소합니다. 최소보다 좁으면 기존 양방향 ScrollArea가 가로 넘침을 처리합니다. 전체 CSS의 line-break/white-space/column/style/셀 병합과 UA 간 모든 배치를 구현한 것으로 확대하지 않습니다.

table-auto-width.log는 기존 표시/인용/코드/링크/표/목록과 새 변경 프레임/이미지/자연 너비의9통과·0.11초·exit0(컴파일10.01초)입니다. 뒤의 table-language.log는 공백 없는 긴 단어의1줄·가로 넘침과 한중일 문장의 좁은 영역 줄바꿈을 실제 galley/경계로 확인한1통과·0.04초·exit0입니다. 현재 preview10건이며 default egui 폰트의 구조/배치 검사로 실제 OS 글꼴·픽셀 게이트와 구분합니다. 이전 성공은 중복 합산하지 않습니다.

Provider의 기본 image_size는 None이고 실제 Markdown은 기존 Cache의 준비된 texture/승인된 project/source와 동일 displayed_size를 읽습니다. 조회는 로딩·playback·입력을 변경하지 않습니다. table-cache.log의4통과·0.21초·exit0은 자연/폭만/높이만/두 크기의 read-only 계산과 다른 project 거절, 기존 PNG 실제 픽셀·root/중복/회수·HTTP/취소·data URI/SVG/애니메이션/용량을 포함합니다. table-hover.log는 공유 font job 추출 뒤 기존 일반 도움말13통과·0.20초·exit0입니다. 기존 표시/입력/IME·readonly/mirror/provider/focus·링크/이미지/크기 조절을 유지합니다.

table-app.log는 소유 임시 PNG를 실제 HTML 표 셀과 본문에 함께 표시한 App1통과·34.06초·exit0(컴파일53.63초)입니다. 미리보기 geometry와 실제 이미지 response 두 개·폭/비율, 기존 theme/font·signed/reversed/value 번호·HTML·최신 입력/Paste/undo/오류 복구/문서 전환/승인 revision 닫힘/원문/task0을 확인했습니다. cfg(test)의 image rect 기록도 보이는 실제 표시만 포함해 측정을 관찰 결과에 섞지 않습니다. 현재 preview10/일반 도움말13/cache4/App1의 서로 다른28통과·소유8Rust fmt/diff입니다.

공개 Hash/Provider 기본 method 경계의 table-frozen-host.log는 tests/inspection 포함 check27.54초·exit0, table-frozen-wasm.log의 실제 Wasm canvas lib는13.65초·exit0입니다. 새 의존성/manifest/lock/frozen source·실제 사용자 설정/OS/클립보드/보호 M8 앱을 변경하지 않았으며 사용자 문서/PROCESS 하단 두 hunk를 보존합니다. 셀 병합·caption/thead/tfoot/열 스타일/전체 CSS·list type·SVG/MathML/media/details/anchor·CLI 이미지의 별도 소유 경계 대조와 실제 OS/전체App 준비/접근성/성능/신뢰성/출시·전환·제거는 미완료입니다. editor-72 partial·302/588(51.4%)·미완료286·배치23 잔여19/최종33·전체 목표/예상 시간 미산정을 유지합니다.

## 표 셀 병합·행 그룹·제목과 오류 위치 (2026-10-13)

이전 골 턴은 표의 변경 배치/자동 너비를22798293/5c21f4ac로 구현·검증·일반 푸시한 progress입니다. 전체 목표/workflow·서브에이전트 사용하지 않음을 유지합니다. [WHATWG 표 모델](https://html.spec.whatwg.org/multipage/tables.html#processing-model)과 [셀 속성](https://html.spec.whatwg.org/multipage/tables.html#attributes-common-to-td-and-th-elements), 원본 sanitized HTML과 기존 rich document 소비자를 대조했습니다. span-red.log는 colspan2/rowspan2 표의 실제3열을2열로 처리하는 실패0/1·0.03초·exit101(컴파일16.46초)입니다.

Block.Table의 optional TableLayout과 TableCell은 실제 시작 열·두 병합 범위·셀 정렬·caption/행·열 수를 보존합니다. 기존 LSP 파서는 None을 유지하고 기존 header/rows와 동일 표시 계약을 사용합니다. HTML은 머리/본문/꼬리 그룹, rowspan0의 남은 그룹·양수 span의 암시된 행과 다음 그룹 분리, colspan0/무효값/양수 prefix와1000/65534 상한·큰 정수 clamp를 처리합니다. 표 메타데이터/track 표현 예산과 checked 크기 연산의 오류는 전체 blocks/ListItem 재귀 경계에서 기존 localized preview 오류로 전달하며 부분 출력/panic으로 대체하지 않습니다.

editor-markdown-table.rs는 실제 DOM의 셀 위치를 계산하고 editor-markup-table.rs는 현재 내용의 최소/자연 너비를 공유해 병합 열의 너비·행 높이를 배분합니다. 병합 셀을 한 번 표시하고 공유 경계 구간을 합쳐 내부 가짜 경계/중복 stroke를 없앱니다. span65534의 모든 빈 슬롯마다 선을 그리는 방식 대신 실제 셀 perimeter만 처리합니다. 기본 th/명시 align과 셀 수직 가운데·caption 가운데/여백을 연결합니다. geometry Arc는 소유 표 ID별 하나를 교체하며 입력·scale/폰트·이미지 준비/표시 크기 변경에 따라 갱신합니다. 렌더링/측정 ID를 구분하고 실제 표시 링크 ID는 유지합니다.

span-parser-initial.log는 기존/새 기본 파서8통과·0.03초·exit0(컴파일45.95초)입니다. 이후 group/제목/정렬·rowspan0/상한/암시된 행·다음 그룹/예산 overflow 사례를 추가한 span-parser-final.log는 parser10/상태3/표 예산1의14통과·0.05초·exit0(컴파일24.62초)입니다. UI 기존 생성자의6개를8개로 잘못 단언한 보조 스크립트와 포맷 이후 patch context 불일치는 파일 확인 후 수정했고 검사 통과로 집계하지 않습니다.

span-geometry.log는 최초/좁힘/넓힘·colspan/rowspan의7개 실제 perimeter, 없는 내부 경계·한 번만 있는 공유 경계·제목/수평 정렬/수직 가운데·셀 안의 실제 galley와 변경 뒤 같은 상태 개수1통과·0.01초·exit0(컴파일9.85초)입니다. span-ui-final.log는 preview12/일반 도움말13의25통과·0.15/0.20초·exit0(컴파일5.34초)입니다. caption만 있는 빈 표와 실제 pointer 링크, 기존 제목/코드/표/목록/링크·IME/focus/readonly/mirror/provider/이미지 소비를 확인합니다. 기존 이미지 준비/높이/링크/캐시 검사에 metadata와 legacy 양쪽 입력을 추가한 span-image-ui.log1통과·0.03초·exit0의 최신 성공을 사용하고 중복 합산하지 않습니다.

span-app.log는 실제 PNG 병합 표·caption/thead/tfoot 표시와 wheel 뒤 오류 문구가 첫 프레임에 보이지 않는 제품 실패0/1·33.59초·exit101(컴파일14.02초)입니다. 새 오류가 표시될 때 미리보기 scroll만1회0으로 초기화하고 이후 스크롤/본문/독립 뷰는 유지합니다. 같은 앱의 source 수정/undo/오류 복구·원문/PNG·실제 미리보기 제목/머리/꼬리·승인 revision 닫힘·작업0과 readonly/두 뷰 회귀를 span-app-fixed.log의3통과·49.33초·exit0(컴파일24.01초)으로 확인했습니다. 본문 대신 문자열을 검사하지 않고 오른쪽 preview의 실제 galley label을 확인합니다.

span-cache.log는 caption만 참조하는 소유 PNG의 동일 승인 texture 유지와 기존 실제 픽셀/root/중복/회수·HTTP/취소·data/SVG/애니메이션/용량의4통과·0.21초·exit0입니다. caption의 이미지 수집/자동 링크도 기존 보안 경계를 사용합니다. span-lsp.log는 기존 plain/code/GFM/목록/표/안전한 링크/이미지/비신뢰 HTML 제외의8통과·0.00초·exit0입니다. 현재 parser10/상태3/예산1/UI25/App3/cache4/LSP8의 서로 다른54통과·소유15Rust fmt/diff입니다. 작성 오류/앞선 성공/입력이 같은 반복 결과는 추가 합산하지 않습니다.

공통 모델의 span-frozen-host.log는 tests/inspection 포함 check27.77초·exit0, span-frozen-wasm.log의 실제 Wasm canvas lib는13.60초·exit0입니다. 의존성/manifest/lock/frozen source·실제 사용자 설정/OS/클립보드/보호 M8 앱과 사용자 문서/PROCESS 하단 두 hunk를 보존합니다. 전체 CSS/열·행 스타일/임의 블록 cell 내용·접근성 표 의미·list type/추가 HTML/SVG/MathML/media/details/anchor·CLI 소유 경계·실기/성능/전체App 준비/신뢰성/출시/전환/제거는 남습니다. editor-72 partial·302/588(51.4%)·미완료286·배치23 잔여19/최종33·전체 목표/잔여시간 미산정을 유지합니다.

## 목록 type의 원본 CSS 우선순위와 항목별 표시 (2026-10-13)

원본 markdown-preview.tsx의 [&_ol]:list-decimal·[&_ul]:list-disc·[&_li]:ml-5를 설치된 Tailwind로 생성하고, 설치된 DOMPurify와 별도 headless Chrome의 소유 HTML에 적용했습니다. ol/ul의 type은 원본 CSS에 의해 decimal/disc로 표시되며 li의 type은 직접 지정되어 a/A/i/I/1·disc/circle/square/none으로 남습니다. WHATWG [목록의 type 매핑](https://html.spec.whatwg.org/multipage/rendering.html#lists), W3C [표현 속성과 CSS 우선순위](https://www.w3.org/TR/css-cascade-4/#preshint), [표준 counter 형식](https://www.w3.org/TR/css-counter-styles-3/#predefined-counters)과 대조했습니다. 부모 type까지 새 알파벳 번호로 바꾸지 않습니다.

/private/tmp/taide-markdown-list-browser.ts와 list-browser-final.log는 정화 뒤 공백·BOM/NBSP 제거/NEL 보존·대소문자·무효값·중첩 목록을 포함한21사례 통과·exit0입니다. 실제 소유 HTML 이미지의 ol start27/typeA→AA, 다음 I→XXVIII 및 ul value27의 AA/XXVIII, 0/4000 fallback·circle/square/none을 확인했습니다. 원본 여백에서 긴 marker가 화면 왼쪽으로 잘리는 현상은 새 native 요구사항으로 강제 재현하지 않습니다. 번들 Chromium 부재·DOMPurify trim의 최초 기대값 오류·포맷 후 문자열 치환 누락은 작성/환경 실패이며 제품 성공에 합산하지 않습니다. 설치나 사용자 브라우저/OS 설정 변경은 하지 않았습니다.

list-red.log는 항목 수/기한을 보존한 실제 App에서 AA/C marker 미표시의0통과/1실패·33.22초·exit101(컴파일11.52초)입니다. 공통 ListItem의 optional marker_style과 marker, ListMarkerStyle.format으로 두 표시 소비자를 연결합니다. HTML 파서는 정화기의 type 공백 규칙·부모 CSS와 직접 li style을 구분하고 unordered value를 포함한 항목 counter를 유지합니다. 알파벳의 양수 자리수·로마1..3999와 decimal fallback·i64 경계·도형/none을 처리하며 일반 LSP는 marker_style None으로 기존 기본 표시를 보존합니다. 기존 checkbox 경로는 변경하지 않습니다. 전체 CSS counter 소유·display/list-style-position·마커 정렬과 임의 CSS 표현까지 완료한 것으로 확대하지 않습니다.

list-parser.log는 parser11/상태3/표 예산1의15통과·0.05초·exit0(컴파일46.86초), list-core.log는 공통 문서8통과·0.00초·exit0(컴파일9.67초), list-ui.log는 실제 번호/도형/생략·표/이미지/링크/현재 geometry의preview13통과·0.06초·exit0(컴파일27.57초)입니다. 같은 UI 실행의 일반 도움말0건은 inspection 누락으로 제외하고 list-hover.log의 feature inspection13통과·0.04초·exit0(컴파일16.56초)를 사용합니다. 최초 recursive fmt에서 이전 소유 표의 포맷 차이를 발견해 같은 미리보기 파일의 포맷을 맞췄으며 list-fmt-final.log의 소유10Rust fmt와 diff check가 통과했습니다. 포맷만 바뀐 동일 기능 성공은 재실행하지 않습니다.

공백 처리 수정 뒤 영향받은 type 검사만 list-parser-final.log의1통과·0.00초·exit0(컴파일0.28초)으로 확인했고 이전14성공과 중복 집계하지 않습니다. list-app.log는 실제 preview AA/C·음수/역순/value·HTML 표/PNG/테마/입력/undo/오류 복구·원문/승인 revision 닫힘과 readonly/두 뷰의3통과·48.64초·exit0(컴파일20.73초)입니다. list-lsp.log는 기존 plain/code/GFM/목록/표/링크/이미지/안전 경계8통과·0.00초·exit0(컴파일0.22초)입니다. 현재 parser15/Core8/UI26/App3/LSP8의 서로 다른60건이며 원본 브라우저21사례와 분리합니다.

공통 계약의 list-frozen-host.log는 tests/inspection 포함 check27.74초·exit0, list-frozen-wasm.log는 wasm32-unknown-unknown canvas lib check13.64초·exit0입니다. 소유10Rust fmt/diff·의존성/manifest/lock/frozen source 변경0·536GiB/71%·사용자 문서/PROCESS 하단 두 hunk를 보존했습니다. CSS counter 소유·마커 정렬·전체 CSS/표 행·열 스타일/임의 block cell·접근성·i64 밖 번호·SVG/MathML/media/details/anchor/CLI 경계와 실제 OS·전체App877통과/22준비실패/보호3제외/exit101·성능 ignored1/Wasm 시간 API 검사7·실기/신뢰성/출시/전환/제거는 미완료입니다. editor-72 partial·302/588(51.4%)·잔여286·배치23 잔여19/최종33·전체 목표와 예상 시간 미산정을 유지합니다.

## 표 셀의 flow 블록 구조 (2026-10-13)

[WHATWG td의 flow 내용](https://html.spec.whatwg.org/multipage/tables.html#the-td-element)과 원본 MarkdownPreview의 p/heading/list/pre/table CSS 및 설치된 DOMPurify 경계를 대조했습니다. block-cell-red.log는 실제 소유 Markdown 앱에서 first paragraph가 한 문자열로 평탄화되어 독립 galley로 표시되지 않는0통과/1실패·34.14초·exit101(컴파일19.47초)입니다. TableCell.blocks의 optional Vec<Block>과 Block/ListItem/ListMarkerStyle의 Hash를 추가해 단순 inline 셀과 기존 LSP None을 보존하고 문단·제목·목록·pre/code·인용·중첩 표를 기존 Block 경로에 연결했습니다. 복합 셀의 rows inline은 비워 중복 표시/이미지 demand를 만들지 않습니다. th의 inline 굵기 전달·상대 이미지·URL 필터/중첩 autolink와 기존 depth/출력·span 모델 예산을 유지합니다.

preview의 공통 inline 최소/자연 너비를 블록의 실제 글꼴/들여쓰기·번호/코드/중첩 표 제약에 연결하고 모든 셀/제목·span의 현재 높이를 기존 병합 geometry로 측정합니다. layout/blocks Hash·실제 폰트 metrics·재귀 이미지 크기로 입력 fingerprint를 만들며 블록 추가/회수의 높이를 첫 프레임에 갱신합니다. UiBuilder.invisible은 설치된 egui-input/src/ui_builder.rs의 문서/구현에서 disabled도 설정하므로 측정 pass의 입력은 차단되고 실제 셀의 widget ID/clip은 유지됩니다. Cache collect와 autolinks apply가 중첩 Quote/List/Table의 셀 body를 순회하도록 연결했습니다. 일반 문서 code-cache/legacy renderer·의존성/manifest/lock/동결 소스는 변경하지 않았습니다.

block-cell-ui-initial.log는 기존 preview13/일반 도움말13의26통과·0.05/0.04초·exit0(컴파일17.59초)입니다. 복합 구조/병합/좁은·넓은 viewport/독립 문단·제목/번호/코드/중첩 표·state 수와 기존 이미지 준비/높이/실제 링크 클릭을 legacy/HTML/인용 블록3모드로 확장한 block-cell-ui.log는 preview14통과·0.06초·exit0(컴파일4.73초)입니다. 블록 추가/회수의 같은 너비·변하지 않은 자연 너비에서 현재 높이 갱신은 block-cell-ui-final.log의 해당1통과·0.03초·exit0(컴파일4.75초)이며 중복 합산하지 않습니다.

설치된 egui-input/src/widgets/checkbox.rs·widget_style.rs·atomics/atom_layout.rs에서 체크박스의 실제 최소 너비는 icon_width와 interact_size.y 중 큰 값이며 외부 frame은0임을 확인했습니다. 경계 검사만으로는 줄바꿈 차이를 잡지 못해 block-cell-checkbox-red.log의1성공을 전체 충실도 증거로 확대하지 않았습니다. 자연 너비의 한 줄 표시를 단언한 block-cell-checkbox-wrap-red.log는 실제7줄/기대1줄의0/1·0.03초·exit101(컴파일12.55초)입니다. 블록 최소/자연 너비에 실제 widget 최소값을 적용한 block-cell-checkbox-fixed.log는1통과·0.03초·exit0(컴파일9.30초)이며 이전 변경 없는13 preview/13 도움말 성공과 재사용합니다.

block-cell-parser.log는 parser12/상태3/표 예산1의16통과·0.04초·exit0(컴파일45.43초)입니다. 블록 순서/코드 개행·언어/inline-only 경계·th 굵기·상대 이미지·명시 링크와 중첩 autolink를 확인했습니다. block-cell-cache.log는 caption/Quote/중첩 표의 같은 소유 PNG를 같은 texture1개로 유지하고 기존 경로/프로젝트·data/SVG/HTTP·취소/예산을 포함한4통과·0.20초·exit0입니다. block-cell-lsp.log는 기존 plain/code/GFM/표/목록/안전 경계8통과·0.00초·exit0입니다. block-cell-app.log는 실제 preview의 독립 문단/목록/코드/중첩 표·인용 PNG1개80x40·내용/PNG/디스크 보존·닫힘/작업0과 기존 토글/입력/undo/오류/readonly/두 뷰의4통과·60.56초·exit0입니다. 새로운 checkbox 너비 연산은 해당 검사로 덮고 분기를 사용하지 않는 이전 App/파서/캐시/LSP의 성공을 재사용합니다. 현재 parser16/UI27/cache4/LSP8/App4의 서로 다른59건입니다.

block-cell-frozen-host.log는 공통 계약의 tests/inspection 포함 check38.25초·exit0, block-cell-frozen-wasm.log는 canvas lib check13.57초·exit0입니다. checkbox 내부 너비 수정 뒤 native UI 검사와 block-cell-frozen-wasm-final.log의0.12초·exit0을 확인했으며 공개 타입/동결 소비 계약이 같은 host 성공은 재사용합니다. 소유11Rust fmt/diff와534GiB/71%·사용자 문서/PROCESS 하단 두 hunk·실제 OS/설정/클립보드/보호 앱 경계를 유지했습니다. 암시 인라인/문단 여백 collapse/글꼴 상속·caption의 복합 block·행/열/임의 CSS·counter 소유/접근성·SVG/MathML/media/details/anchor/CLI 경계·전체App877통과/22준비실패/보호3제외/exit101·성능 ignored1/Wasm 시간 API 검사7·실기/신뢰성/출시/전환/제거는 미완료입니다. editor-72 partial·302/588(51.4%)·미완료286·배치23 잔여19/최종33·전체 목표/예상 시간 미산정을 유지합니다.

## 암시 인라인·여백 병합·글꼴 상속 (2026-10-13)

원본 MarkdownPreview의 설치된 Tailwind와 DOMPurify를 /private/tmp/taide-markdown-flow-browser.ts의 소유 HTML/별도 headless Chrome에 적용했습니다. flow-browser-standard.log는 원본과 같은 CSS1Compat에서 plain li20px/p22.75px·p sibling8px·heading 앞16px·quote/list의 자식 여백 외부 병합·셀 p sibling8px·th의 code/list700 굵기14조건 통과·exit0입니다. 최초 doctype 없는 결과는 표의 폰트가 quirks mode의16px이므로 현재 근거로 쓰지 않습니다. [CSS 여백 병합](https://www.w3.org/TR/CSS22/box.html#collapsing-margins)과 [익명 블록](https://www.w3.org/TR/CSS22/visuren.html#anonymous-block-level)을 확인했습니다.

flow-margin-red.log는 실제 galley의 문단 사이16px/원본8px0통과/1실패·0.01초·exit101(컴파일2분14초)입니다. HTML flush를 공통 Block.Inline로 구분하고 Code/List에 기존 Style의 상속 값을 연결했습니다. LSP 파서는 Paragraph/Code/List에 기존 의미와 default Style을 유지합니다. 새 editor-markup-flow.rs가 지원 블록의 앞/뒤·빈 내용/quote/list 경계의 양수 여백을 합치며, preview의 인라인20px/문단22.75px과 부모·자식 여백을 중복 배치하지 않습니다. 인용 border는 실제 자식 내용만 둘러싸고 일반 li의 간격을 불필요한 p 여백으로 늘리지 않습니다. NBSP는 ASCII whitespace와 구분해 원본의 유효한 암시 줄로 남깁니다. 기존 이미지/링크/code 수집과 두 표시의 exhaustive 소비를 연결했습니다.

TextWeight의 normal/600/700을 표시와 너비 측정에 함께 사용합니다. 미리보기 inline/pre 코드의 고정폭 가족은 편집기 설정과 구분하고, h1..h3의 코드600·th/strong 코드700·list marker700과 일반 UI bold를 대응 별칭으로 선택합니다. ui-fonts가 sans/mono 역할과400/500/600/700에 따라 같은 공유 byte budget 안에서 실제 face/variation을 준비하며 font-data 이름에 역할을 넣어 같은 weight의 데이터 충돌을 방지합니다. 기존 LSP code 가족은 그대로 유지하고 미등록 bold 별칭은 정상 고정폭 가족으로 대체해 renderer panic을 막습니다. [Apple의 고정폭 system font API](<https://developer.apple.com/documentation/appkit/nsfont/monospacedsystemfont(ofsize:weight:)>)와 설치된 objc2 호출 계약을 확인했습니다.

flow-ui-initial.log는 preview15/일반 도움말13의28통과·0.06/0.04초·exit0입니다. 역할별 코드/600/700을 구분한 flow-ui-fonts.log도28통과·0.06/0.04초·exit0(컴파일12.81초)입니다. 새 검사 closure의 label.into() 타입 추론 E0282는 명시적인 &str 경계로 수정했고 제품 검사 성공에 합산하지 않습니다. flow-ui-expanded-fixed.log는 preview17통과·0.06초·exit0(컴파일4.57초)이며 인라인20px/문단8px·quote border/바깥 여백·편집기의 미등록 글꼴과 독립된 inline/strong/heading/pre 코드/marker 가족을 실제 shape로 확인했습니다. 일반 도움말13의 변경 없는 성공을 재사용한 현재 UI30건이며 이전 성공/동일 검사를 중복 집계하지 않습니다.

flow-parser-initial.log는 parser13/상태3/표 예산1의17 통과·0.03초·exit0(컴파일4분31초)입니다. 상속/Inline 긍정 단언을 반영한 flow-parser-final.log의17통과·0.05초·exit0(컴파일34.17초)를 현재 근거로 사용합니다. NBSP 수정 영향1건은 flow-inline-final.log의1통과·0.00초·exit0(컴파일14.78초)로 확인하고 이전16성공은 재사용합니다. 포맷 뒤 patch context 불일치와 잘못된 rg 옵션/읽기 경로·샌드박스 ps 거절은 작성/환경 관찰이며 검사 성공으로 집계하지 않습니다. 실행 handle의 관찰 대기를 재시작하지 않았습니다.

flow-font-loader.log는 실제 weight/shared budget1통과·실제 OS face1실패·24.36초·exit101입니다. macOS가 선택한 system mono face의 URL이 없는 실제 입력에서 기존 필수 URL 가정을 재현했습니다. URL이 있으면 기존 경로/face 검증을 유지하고 없으면 같은 mono 범주의 catalog face를 head/maxp/hhea 실제 테이블과 대조합니다. 기존 face_data의 byte budget/안전 읽기·variation 축 검증을 거쳐 실제 데이터만 선택합니다. flow-font-loader-fixed.log는 영향1통과·37.78초·exit0(컴파일18.09초)이고 정상/shared budget·거절1건의 변경 없는 성공은 재사용합니다. OS가 선택한 sans/mono와 각 weight의 데이터가 서로 다르며 실제 renderer 별칭에 해당 데이터를 유지하는 현재 fonts2건입니다. UIFont 지원을 근사 face로 바꿔 실패를 없애지 않았습니다.

flow-lsp.log는 기존 plain/code/GFM/표/목록/안전 경계8통과·0.00초·exit0, flow-cache.log는 상대/caption/중첩 이미지·root/프로젝트/texture·cancel/budget4통과·0.19초·exit0입니다. flow-app.log는 토글/안정된 본문/입력/undo/오류/readonly/두 뷰/scroll과 복합 셀의 독립 문단/목록/코드/중첩 표·PNG/원문/디스크/닫힘/task0의4통과·52.81초·exit0입니다. 현재 parser17/UI30/fonts2/LSP8/cache4/App4의 서로 다른65통과와 원본 브라우저14조건을 분리합니다.

flow-frozen-host.log는 tests/inspection check1분42초·exit0, flow-frozen-wasm.log는 canvas lib check1분44초·exit0입니다. 소유16Rust fmt/diff·532GiB/71%·의존성/manifest/lock/frozen source 변경0·사용자 문서/PROCESS 하단 두 hunk와 실제 설정/OS 입력/클립보드/보호 앱 경계를 유지합니다. 상대 bolder/임의 font-weight·모든 CSS display/clearance/negative margin·복합 caption/행·열/임의 CSS·counter 소유/접근성·SVG/MathML/media/details/anchor/CLI 경계·전체App877통과/22준비실패/보호3제외/exit101·성능 ignored1/Wasm 시간 API 검사7·실기/신뢰성/출시/전환/제거는 미완료입니다. editor-72 partial·302/588(51.4%)·잔여286·배치23 잔여19/최종33·전체 목표/예상 시간 미산정을 유지합니다.

## 표 제목 블록과 줄별 텍스트 정렬 (2026-10-13)

원본 MarkdownPreview/Tailwind·DOMPurify를 별도 headless Chrome의 standards 문서에서 대조했습니다. /private/tmp/taide-caption-browser.log는 CSS1Compat·제목/목록의 중앙 정렬·문단 간8px·표 위 배치·셀 없는 제목·sanitized 블록 보존·텍스트/링크/이미지 중심의10조건 통과·exit0입니다. 최초8/10은 Chrome의 실제 -webkit-center 값을 문자열 center만으로 비교한 검사 오류이며 원본 CSS 의미를 바꾸지 않고 허용된 중앙 값으로 확인했습니다. [CSS 표 제목 배치](https://www.w3.org/TR/CSS22/tables.html#caption-position)와 설치된 egui Label/LayoutJob/Galley의 배치 계약을 대조했습니다.

TableLayout의 optional caption_blocks로 기존 Inline 제목과 복합 flow를 구분합니다. 파서는 문단·목록·코드·인용 구조를 보존하며 같은 재귀 autolink/이미지 준비·측정·Hash/현재 프레임 geometry를 소비합니다. inline-only 제목과 기존 LSP 계약은 그대로 유지합니다. 최초 App 컴파일은 caption map의 오류 타입 추론 E0282/E0283·exit101·실행 검사0건이며 AppResult 경계를 명시해 수정했습니다. 실제 파서/분할 상태/표 예산18통과·0.03초·exit0(컴파일36.37초)가 caption-parser.log에 남습니다. 새로운 셀 없는 제목의 flow·상대 이미지/링크·hidden을 포함합니다.

caption-alignment-red.log는 짧은 제목의 줄 중심16.375/표105point의 실제 실패0/1·0.03초·exit101(컴파일2분28초)입니다. 블록 영역은 표의 사용 가능한 폭을 유지하고 각 텍스트 줄은 부모 정렬에 맞게 배치합니다. editor-markup-aligned.rs는 같은 inline_font를 사용하면서 강조/취소선/링크·명시 개행·선택을 하나의 galley에 보존합니다. 이미지/inline code 프레임은 기존 표시와 클릭을 유지하며 자연 폭의 그룹으로 정렬합니다.

caption-ui.log는 일반 도움말13/미리보기18성공·링크1실패·0.04/0.06초·exit101(컴파일15.71초)입니다. caption-link-diagnostic.log의0/1·0.01초·exit101은 같은 글꼴의 LayoutJob section 병합을 span과 일대일로 간주한 문제였습니다. 문자별 링크 범위를 별도로 보존하고 실제 glyph 위치에서만 열도록 수정했습니다. 임시 진단 출력을 제거한 caption-ui-fixed.log의관련3통과·0.03초·exit0(컴파일9.31초)은 좁고 넓은 첫 프레임의 문단/강조 줄별 중심·한국어/개행 뒤 링크·링크 밖 prefix/tail 클릭 거절·복합 셀 경계를 확인합니다. 영향 없는 성공29건과 재사용한 서로 다른 UI32건이며 중복 성공은 더하지 않습니다. 기존 이미지 검사는 복합 제목의 준비 None→작은 크기→큰 높이→복원을 추가해 같은 프레임의 표 이동/중심/링크·캐시 개수 보존까지 확인합니다.

caption-cache.log는 이미지4통과·1.03초, caption-app.log는 실제 App4통과·65.31초, caption-lsp.log는 기존 LSP parser8통과·0.00초·모두 exit0입니다. 제목과 셀의 같은 PNG를 두 곳80x40으로 표시하고 원문/이미지 bytes·디스크·입력/undo/readonly/두 뷰/scroll·승인 revision 닫힘/작업0을 유지했습니다. 현재 parser18/UI32/cache4/App4/LSP8의 서로 다른66건입니다. 링크 범위를 글자마다 할당하지 않고 span의 문자 범위로 보존한 최종 caption-range-final.log도 관련3통과·0.03초·exit0(컴파일9.07초)이며 나머지 영향 없는 성공은 재사용합니다. caption-frozen-host.log의 tests/inspection check2분09초와 caption-frozen-wasm.log의 canvas lib check2분03초·exit0 뒤, 마지막 변경 상태의 host-final0.09초/Wasm-final0.10초·exit0을 확인했습니다. 소유12Rust fmt/diff·manifest/lock/frozen source 변경0·666GiB/64%를 확인했습니다. PROCESS 포맷 경고는 사용자 하단 문서의 빈 줄1곳이며 그 내용을 보존하고 소유 상단과 QA의 포맷만 확인합니다. 목록 marker와 혼합 이미지·코드의 모든 줄별 정렬/임의 CSS·행/열·접근성 및 전체 App 준비 실패/성능/OS/출시 게이트는 미완료입니다. d6c3h는 열어 두며 editor-72 partial·302/588(51.4%)·미완료286·배치23 잔여19/최종33·전체 완료 시간 미산정을 유지합니다.

## 바깥 목록 마커·제목 코드·블록 이미지 (2026-10-13)

원본 MarkdownPreview의 설치 Tailwind preflight와 marked의 작업 목록을 별도 headless Chrome의 standards 문서로 대조했습니다. [CSS 목록의 바깥 마커](https://www.w3.org/TR/CSS22/generate.html#lists)와 설치 egui의 UiBuilder/Label/ScrollArea 배치 계약을 확인했습니다. /private/tmp/taide-list-caption-browser-final.log는12조건 통과·exit0입니다. 목록 본문 왼쪽20px·중첩40px, 제목 안 본문의 중심이 caption 중심보다 각각10/20px 오른쪽, 원본 pre의 각 줄 중앙 정렬, disabled checkbox13x13과 바깥 마커, img display:block·실제40x20 SVG와 앞뒤 텍스트 전체65.5px를 확인했습니다. 잘못 인코딩한 첫 SVG는 현재 근거에서 제외했으며 최종은 실제 이미지 크기를 단언합니다.

outside-list-red.log는 본문이 원본의20px 대신 marker 폭만큼 더 밀리는 실제0/1·0.02초·exit101(컴파일4.58초)입니다. preview의 목록 마커를 별도 child에 배치하고 본문 자연/최소 너비에서 바깥 마커를 제외했습니다. 숫자 폭/형식/none과 무관한20px 본문 영역에 부모 정렬을 유지하며 문단/제목/코드의 실제 font job 기준선으로 마커 위치를 계산합니다. 기본 작업 목록은 바깥 bullet을 유지하면서13px disabled checkbox와 내부 텍스트 그룹을 표시합니다. pre Label은 부모 정렬을 명시해 각 줄을 정렬하며 일반 호버는 기존 표시를 유지합니다.

block-image-red.log는 앞 텍스트의 아래쪽이 이미지 위쪽보다 뒤에 놓인 실제0/1·0.01초·exit101(컴파일4.82초)입니다. preview의 Inline 배열을 이미지 경계에서 나눠 텍스트·이미지·뒤 텍스트를 독립된 블록으로 표시합니다. 공백뿐인 조각은 기존 flow 판정을 재사용하고 이미지 준비/alt·크기·링크는 기존 Provider를 소비합니다. 최소/자연 너비도 서로 더하지 않고 각 블록의 최대값으로 계산하며 이전 무효한 인라인 이미지 너비 분기를 제거했습니다. 인라인 코드 frame·GFM/일반 LSP 계약은 변경하지 않았습니다.

최초 목록 수정은20통과·0.07초·exit0(컴파일10.29초)이며 outside-image-ui.log의preview21/일반 호버13·0.07/0.04초·exit0(컴파일10.30초)가 원본 이미지 분리와 기존 변경 프레임/이미지 준비·높이/링크·캐시 개수/표/서명/IME/readonly를 확인합니다. outside-list-baseline.log는 paragraph/h1/pre와 마커 기준선1통과·0.03초·exit0(컴파일9.49초), outside-list-task.log는 checkbox13px/그룹 중앙과 바깥 bullet1통과·0.04초·exit0(컴파일4.59초)입니다. 최종 outside-image-ui-final.log의preview21통과·0.07초·exit0(컴파일4.54초)은 실측65.5px 전체 높이까지 확인합니다. 같은 성공/새 단언을 중복 집계하지 않은 현재 UI34건입니다.

outside-image-app.log는 실제 App의 제목9883/작업 목록/여러 줄 코드·두 PNG 표시와 입력/undo/오류/읽기 전용/두 뷰·원문/이미지/디스크 보존·닫힘/작업0의4통과·52.62초·exit0(컴파일24.05초)입니다. 새 제목 코드의 source escape는 실제 개행이며 double escape가 아님을 파일로 확인했습니다. 서둘러 넓게 적용한 테스트 setup/포맷 patch context와 escape 관찰 오류는 수정 전 작성 단계이며 제품 검사 성공에 합산하지 않습니다. 이전 parser18/cache4/LSP8의 입력/경계는 변경 없어 재사용합니다. 새 UI34/App4와 재사용30의 서로 다른68건, 소유4Rust fmt/diff·frozen host tests/inspection0.08초/Wasm canvas lib0.09초·exit0·663GiB/64%입니다. 새 의존성/manifest/lock/frozen source와 사용자 문서/PROCESS 하단 두 hunk·실제 OS/설정/클립보드/보호 앱은 변경하지 않았습니다.

혼합 inline code의 모든 줄 정렬·raw checkbox 위치/다중 줄 흐름·첫 블록 table/임의 CSS baseline·list-style-position/이미지 display override·counter 소유/행·열/접근성·추가 HTML/CLI와 이전 전체App 준비22실패/성능/OS/출시·전환·제거는 미완료입니다. 부모 d6c3h·editor-72 partial·302/588(51.4%)·미완료286·배치23 잔여19/최종33·전체 완료 시간 미산정을 유지합니다.
