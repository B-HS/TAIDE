# 배치 13 — 편집기 괄호 일치 강조 (2026-10-09)

상태: 기본 near/enclosing query·원본 표시/테마·위젯 focus 공급과 실제 모델/표시 회귀·통합 게이트를 마쳤고 선별 Git 기록을 진행합니다. 서브에이전트·workflow 없이 메인이 직접 수행하며 모든 Cargo 명령은 직렬입니다. 전체 기능 완료율·잔여 시간은 최신 전수 대응표와 실행 시간 근거가 없어 미산정입니다.

## 실제 기준

- 실제 TS `src/features/editor/code-editor.tsx`는 matchBrackets 값을 변경하지 않습니다. Monaco 0.56.0 기본값은 always이며 커서가 괄호 위/끝 경계에 있으면 near를 우선하고 없으면 가장 안쪽 완성된 enclosing 짝을 찾습니다. 원본 BracketPairsTextModelPart의 matchBracket/findEnclosingBrackets와 BracketMatchingController를 직접 읽었습니다.
- AST near는 현재 위치를 포함하는 opening/closing 범위 중 실제 해당 괄호 범위의 시작 위치가 가장 오른쪽인 짝을 고릅니다. 같은 위치에서 여러 중첩 짝이 닿을 때 단순히 가장 안쪽 pair를 선택하는 것과 다릅니다. enclosing은 complete 짝의 전체 범위가 위치를 엄격하게 포함해야 합니다. 정렬/contains/토큰 경계는 실제 원본 기준값으로 검증합니다.
- 원본은 모델/위젯 focus가 없으면 강조를 없애고 non-empty 선택은 제외합니다. 여러 빈 선택을 처리하며 원본의 100개 상한은 렌더 비용 방어입니다. 20ms 요청 한도는 AST 분기에서는 적용되지 않습니다. 대형 파일의 AST/legacy 분기는 성능 구현 차이며 TS는 대형 파일에서 괄호 색상만 끄고 matchBrackets를 별도로 끄지 않습니다. 수치/결함 강제 재현 대신 정확한 결과와 기존 비용/대형 보호 경계를 확인합니다.
- 실제 TS `src/shared/lib/monaco/theme.ts`는 editorBracketMatch.border를 editor.bracketMatch에 연결합니다. background 기본값은 원본 editorColorRegistry의 #0064001a이며 foreground 기본값은 null입니다. 기존 토큰/괄호 색을 유지하고 원본 bracketMatching.css의 inside 1px 테두리와 배경을 기존 글자 좌표로 그립니다. 실제 TS에 없는 새 색 설정은 추가하지 않습니다.
- near 결과만 원본 overview ruler의 center lane/#A0A0A0 장식을 공급하고 enclosing은 공급하지 않습니다. native의 현재 Lane 장식은 다른 경계이므로 추측으로 overview를 완료 처리하지 않습니다. 진단/검색/SCM 등 실제 overview 공급을 묶는 후속 표시 배치에서 연결할 구분을 보존합니다.
- 기존 native BracketModel은 문서/revision/언어·Other 토큰과 변경 저널을 확인하며 configured/colorized 합집합의 짝·interval 조회를 공급합니다. active_pair는 엄격한 enclosing/안내선용이고 near의 가장 오른쪽 괄호 우선순위를 제공하지 않습니다. 기존 `bracket-display`의 Monaco 23언어/230표본·토큰/캐시 검사는 재사용하고 새 query 표본을 추가합니다. editor/UI에 구문·정규식 의존성을 추가하지 않습니다.

## 진행·보호 범위

현재 찾기 입력창·고정 줄·본문/다른 위젯의 실제 focus 소유 경계를 확인합니다. near/enclosing query·포커스/선택 수명·글자 좌표/clip·wrap/접기·테마·DPR·읽기 전용/대형을 원본 기준값과 메모리 egui로 검증한 뒤 변경 크레이트 전체 대상 --no-fail-fast를 한 번 직접 실행합니다. 보호 Trash 3건·ignored 성능/SDK 문서·실제 UI/OS 입력·IME/RTL/접근성·대형 실기·출시 부채를 통과로 집계하지 않습니다. 브라우저 전체와 모든 manifest/lockfile은 동결합니다.

원본 hasWidgetFocus는 본문 textarea뿐 아니라 DOM 위젯 전체의 focus입니다. 앱은 찾기 위젯을 본문보다 먼저 그리며 FindOutput.focused가 실제 find/replace/control ID 소유를 보고합니다. 이를 표시 문맥으로 공급하고 기존 본문의 문자/IME 소유 route를 바꾸지 않습니다. 고정 줄은 현재 InputState의 focus_ids와 SDK memory의 실제 소유자를 확인합니다. 외부 입력창/다른 pane은 이 표시 소유에 포함하지 않습니다.

원본 `common/viewModel/inlineDecorations.js`는 inlineClassName만 글자 장식에 넣고 고정 줄의 `stickyScrollWidget.js`는 그 inlineDecorations만 renderViewLine으로 전달합니다. 괄호 테두리/배경의 className은 본문 overlay이고 현재 foreground는 null이므로 고정 줄에 테두리/배경을 복제하지 않습니다. 추가하던 복제의 shape가 2개이고 원본대로라면 본문 닫는 괄호 1개여야 하는 실패를 재현한 뒤 복제를 제거했습니다. 고정 줄에 포커스가 있을 때 본문의 짝 강조는 유지됩니다.

## 현재 실행 근거

`docs/utils/2026-10-09-native-bracket-matching-reference.js`는 실제 BracketPairsTextModelPart 생성자/메서드를 실행해 23언어·483문서·모든 유효 Unicode 커서 5185위치의 near/enclosing을 저장했습니다. 원본 클래스/메서드는 수정하거나 재작성하지 않았으며 fixture와 대조한 syntax query 1건이 exit 0입니다(`/private/tmp/taide-batch13-model-before-20261009.log`, build 2.58초). 기존 token-filter 회귀에 새 query 검사를 추가해 문자열/주석/정규식·미완료 줄은 짝이 없음을 확인했고 exit 0입니다(`/private/tmp/taide-batch13-token-filter-20261009.log`). UI --tests 연결 컴파일은 exit 0·5.10초입니다(`/private/tmp/taide-batch13-render-check-20261009.log`).

메모리 UI 7건을 실행해 6 통과·고정 줄 복제 1 실패(exit 101, build 5.14초, `/private/tmp/taide-batch13-ui-matching-before-20261009.log`)를 확인했습니다. 실패 영향 1건은 수정 후 exit 0·build 1.78초입니다(`/private/tmp/taide-batch13-ui-sticky-after-20261009.log`). 서로 다른 7건의 성공 근거이며 전체 대상 게이트는 별도로 한 번 실행합니다. near 경계/enclosing·실제 글자/테두리/배경/clip·기존 색 보존·다중 선택/중복/상한·focus 해제/관련 위젯/외부 문자 입력·wrap/접기/탭/Unicode/스크롤·DPR 1/1.25/2/3/테마·읽기 전용/대형 tier·문자 편집/문서/뷰 수명·고정 줄 포커스와 본문만의 강조를 확인했습니다. 작은 합성 문서의 tier 검사이며 실제 대형 파일 성능 게이트를 대신하지 않습니다. 네 패키지 선별 Cargo fmt와 기준값 생성기 Prettier 확인은 exit 0입니다.

## 최종 통합 게이트

각 변경 크레이트에서 `cargo test --manifest-path native/<crate>/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --all-targets --no-fail-fast`를 하나씩 실행했습니다. UI는 `--features inspection`을 포함합니다. 앱은 최초 명령이 새 테마 검증의 잘못된 `egui` 경로 때문에 컴파일 단계에서 exit 101이었고 테스트는 시작되지 않았습니다(`/private/tmp/taide-batch13-app-full-20261009.log`). 기존 `eframe::egui` 경로로 수정한 뒤 전체 대상을 한 번 실행해 아래 결과를 얻었습니다. 앱 lib test에는 모든 builtin 테마의 실제 editor.bracketMatch border와 원본 기본 배경 검사가 포함됩니다.

| 대상 | 실제 결과 | 로그 |
| --- | --- | --- |
| editor 전체 26대상 | exit 0, 192 통과·0 실패·1 ignored, build 5.82초 | `/private/tmp/taide-batch13-editor-full-20261009.log` |
| syntax 전체 12대상 | exit 0, 159 통과·0 실패·3 ignored, build 4.05초. 기존 토큰/모델 캐시와 신규 near/enclosing 기준값 포함 | `/private/tmp/taide-batch13-syntax-full-20261009.log` |
| UI inspection 전체 19대상 | exit 0, 337 통과·0 실패, build 13.31초. 괄호 일치 화면 7건 포함 | `/private/tmp/taide-batch13-ui-full-20261009.log` |
| app 전체 67대상 | exit 0, 612 통과·0 실패·보호 Trash 3 filtered out, build 30.24초 | `/private/tmp/taide-batch13-app-full-after-20261009.log` |
| remote-web host | `cargo check --tests --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 4.27초 | `/private/tmp/taide-batch13-browser-host-20261009.log` |
| remote-web Wasm | `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --features canvas --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 2.28초 | `/private/tmp/taide-batch13-browser-wasm-20261009.log` |

총 124대상·1300건 통과·미해결 실패 0입니다. editor/syntax ignored 4건과 보호 Trash 3건을 성공으로 세지 않습니다. SDK는 변경하지 않았으므로 batch12의 단위 52·문서 167 통과·문서 1 ignored 근거를 재사용합니다. 앱은 기존 loopback·watcher·ImageIO 경계 때문에 승인된 샌드박스 밖에서 실행했고 `remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다`, `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다`, `실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다`를 명시적으로 제외했습니다.

네 패키지 Cargo fmt 확인·generator Prettier·이번 변경의 whitespace diff는 exit 0입니다. 기준 `5279c6e3` 대비 브라우저 전체와 모든 Cargo.toml/Cargo.lock diff는 exit 0입니다. 생성기는 보충 문자만 JSON escape로 기록하며 전/후 파싱 값이 모두 같음을 직접 확인했습니다. 원본 클래스/기준값·실제 문서/OS 설정·클립보드·Keychain·Trash·보호 앱 번들은 수정하지 않았고 캐시 정리는 실행하지 않았습니다. 최종 디스크 여유 664GiB·사용률 64%입니다. 실제 UI/OS 입력·IME/RTL/접근성·실제 대형/soak·출시 게이트는 미검증입니다.

## 후속 범위

기본 괄호 일치 본문 강조와 현재 찾기/고정 줄 focus 경계가 연결됐습니다. 실제 overview near-only 장식은 후속 공급 배치에서 연결하며 LSP/진단/검색/SCM·미니맵 장식·리거처·나머지 전체 전환을 완료로 표시하지 않습니다. 전체 기능 대응표를 현재 코드·앱 도달 경로·실행 검증에 맞춰 재감사하고 다음 구현 배치의 순서를 정합니다. 기존 미커밋 HANDOFF·architecture·합의/운영 문서와 PROCESS 하단 변경은 선별 Git 범위에서 제외합니다.
