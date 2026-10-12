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
