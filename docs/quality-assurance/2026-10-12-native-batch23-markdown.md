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
