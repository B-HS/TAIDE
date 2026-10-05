# native keybindings modal·행 배치 차이

## 대상 파일

`native/taide-native-app/src/keybinding-editor.rs`

## 조건부 reset/unbind role identity (2026-10-05 후속2)

override 제거 뒤 남아야 할 Unbind의 ID6230519078993315154가 이전8730125973026454617와 달라지는 RED를 재현했습니다. conditional Reset의 삽입/제거가 auto ID 순번을 바꿔 실제 역할과 focus/cache 소유권을 혼동하게 합니다. row/viewport/ResolveConflict·Capture·Confirm·Change·Reset·Unbind 역할의 stable scope로 고정하고 현재 catalog/filter/locale/control shape로 Tab 순서를 계산합니다. 새 역할은 실제 widget을 만들기 전 next_auto_id로 focus를 준비합니다.

남은 Unbind ID/owner 유지·제거된 Reset container 회수·새 Reset 동일 frame Tab의 신규2와 row geometry/기존 modal·Tooltip/capture 영향11 PASS·native lib/tests strict19.77초입니다. 이전 baseline 새-reset PASS는 auto ID 재사용도 통과할 수 있어 버그 부재 근거가 아니었습니다. source 코드/공식 API·직전 DOM 제거 자료 재사용·실제 범위와 raw mixed topology/disabled/상위 modal/전체 graph 미완료는 keybinding-focus-scroll QA 후속2 절이 정본입니다.

## 동적 행 제거·목록 축소 후 포커스 (2026-10-05)

포커스된 row를 현재 query에서 제외하면 native focus None RED이며 원본 Radix FocusScope는 Dialog container로 회수했습니다. 이전-frame Tab 목록을 current visible row IDs로 pruning하고 비-Tab 대상 container를 등록했습니다. viewport·row ID의 stable UI scope로 남은 row의 identity를 보존합니다.

남은 마지막 row를 단일 검색으로 축소할 때 current rect y-127~-103/clip start336.5의 별도 RED도 확인했습니다. ScrollArea가 이전 offset으로 그린 뒤 끝에서 clamp하는 것이 원인입니다. 실제 ScrollAreaOutput.id의 State와 displayed IDs 변경을 비교해 offset이 바뀐 경우 같은 run_ui의 표준 request_discard 재배치를 사용합니다. 대기 frame·무조건 offset0은 추가하지 않았습니다.

신규2/영향8 PASS·native lib/tests strict17.73초입니다. source 공유 Dialog의 합성 자식 제거2사례를 단일 측정한 범위·0tests/권한 실패·계측 한계·재사용 성공은 keybinding-focus-scroll QA의 후속 절이 정본입니다. 조건부 controls/disabled/혼합/전체 current-pass graph와 App/auxiliary/실기/픽셀은 남았습니다.

## Tab·Shift+Tab의 offscreen focus 후속 (2026-10-05)

focused control에 scroll 노출이 없어 actual y667~691이 clip655 밖에 남았습니다. focused/clip 경계로 즉시 스크롤을 요청하고 현재 pass 좌표를 저장해 검증하도록 수정했습니다. 이전 진단은 Context.end_pass buffer swap 뒤 read_response가 이전 rect를 우선 읽는 fixture 오류도 포함했으므로 예전 수정 두 번이 무효였다는 판정을 철회합니다.

새 역방향은 다음 pass에 예약하는 egui Previous focus 때문에 actual y1101~1125가 아직 clip 밖인 별개 RED였습니다. 실제 enabled response의 순서 ID를 pass마다 갱신해 ordinary Tab/Shift+Tab을 직접 배정하고 popup/capture/IME/disabled gate와 close 회수를 유지했습니다. 정방향/역방향·offscreen capture/blur 고유3·영향5 PASS·native lib/tests strict17.50초입니다. 원자료·compile/fixture 실패·성공 재사용은 keybinding-focus-scroll QA의2026-10-05 절이 정본입니다. 여러 raw 입력/동적 목록/전체 App·auxiliary·AX·픽셀은 미완료입니다.

## 리포트

native modal의 고정 검은 scrim·기본 popup 그림자·앞선 close focus와 row의 단일12px 간격이 원본 Dialog/KeybindingRow와 달랐습니다. 실제 raw/render 검사에서 scrim·Tab 순환·icon36px 대28px·header y318.5 대314.0 차이를 재현했습니다.

## 원인과 수정

1. 원본 CSS의 app.shadow50%·0/8/24 그림자와 egui Frame stroke를 포함한 외곽 크기를 적용하지 않았습니다. theme 값/전용 Shadow·padding+stroke 합산을 연결했습니다.
2. close를 content 전에 그려 focus 순서가 원본과 달랐고 egui Previous focus가 다음 frame에 적용됐습니다. close interaction을 마지막에 생성하고 query/close의 양끝 Tab을 명시 순환합니다. popup/capture/IME/disabled는 별도 보호합니다.
3. row 네 열/두 하위 그룹을 한 horizontal 간격으로 계산했으며 source/button도 monospace로 측정했습니다. 실제 글꼴별 폭·열12/binding6/control4를 적용하고 고정 icon24px에서 기본 padding을 제거했습니다. header의 혼합 add_sized/direct label도 같은15px cell로 맞췄습니다.
4. theme 최종색 검사기는 egui Area fade 중간 프레임을 최종색으로 판단했습니다. 설치된 source를 확인해 결정적 time으로 settled frame을 읽었으며 제품 animation을 제거하지 않았습니다.

최종 검사와 정확한 미완료 범위는 `docs/quality-assurance/2026-10-03-m8-native-keybinding-layout.md`가 정본입니다. 전체 픽셀·focus trap·toast·M8 완료로 계산하지 않습니다.
