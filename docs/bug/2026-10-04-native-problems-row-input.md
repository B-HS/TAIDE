# native Problems 행 입력 가로채기

## 대상·관찰

`native/taide-native-app/src/problems.rs`, `problems-tests.rs`입니다. error 필터 뒤 warning 표시 텍스트 중심 `[717,65]`를 실제 포인터로 눌렀지만 Open이 없었습니다. 행 interact rect `[0,55]`~`[800,75]`는 해당 위치를 포함했지만 interact pointer는 NaN이었습니다. source/position Label은 selectable 기본값을 상속했고 RTL child는 텍스트를 `[800,65]`, `[814.2,65]`까지 밀었습니다. 원본은 행 전체 button과 내부 select-none입니다.

## 해결·결과

내부 interactive Label을 제거하고 행 response 하나만 유지했습니다. 메시지/source/위치/경로는 clip rect 내 단일행 LayoutJob/Painter로 표시하고 오른쪽 필드 뒤 실제 남은 폭으로 자릅니다. 검사기 해제나 hit target 확대는 쓰지 않았습니다.

첫 실패 compile7.25초/suite0.05초·trace compile6.54초/suite0.05초 뒤 제품 수정은 고유1 PASS(compile7.54초/suite0.05초)입니다. 실제 row click의 `/synthetic/b.rs` line3/column8 요청·접기까지 확인했습니다. exact glyph·전체 분할/키보드/GUI는 별도이며 native-problems QA를 참조합니다.

## 반복 Space 입력 후속

새 실제 키보드 검사에서 접힌 그룹에 반복 Space를 보내면1행이1001행으로 다시 펼쳐졌습니다(compile12.70초/suite0.05초). 원본 활성화 핸들러는 반복 Space를 무시하지만 egui의 `key_pressed`는 반복 입력도 포함합니다.

행 포커스에서 반복 Space 단독 활성화만 거절하고 Enter·실제 Primary 클릭·AccessKit Click은 유지했습니다. Space/Enter 소비로 아래 위젯에 같은 키가 전파되지 않게 합니다. 실제 그룹 접기/반복 무시/Enter 펼치기·진단 Enter 위치·스크롤/닫기/재열기·빈 필터와 AX Focus/Enter를 포함한 신규 검사1 PASS(compile5.38초/suite0.09초), 관련 strict exit0(17.46초)입니다. 중간 scroll ID·필터 focus fixture 실패와 정정은 native-problems QA에 구분합니다.

## 프레임 batch 후속

같은 프레임의 최초 Space+반복에서 최초 사건까지 무시해 실제2행/기대1행인 RED를 재현했습니다. 사건마다 최초/반복/해제를 구분해 활성화 횟수를 보존하며 여러 Enter도 각각 처리합니다. 제품 수정 뒤 신규 batch2건 PASS(compile10.52초/suite0.04초)입니다.

진단 요청의 Option 집계는 source 검토에서 확인해 Vec으로 바꿨고 actual Application caller도 각 요청을 HostCommand로 전달합니다. 실제 renderer는 두 Enter와 두 AX Click 각각 두1-based 위치 요청을 확인했습니다. 최종 관련3건 PASS(compile6.96초/suite0.09초)·app lib/bin/tests strict exit0(17.21초)이며 전체 혼합 사건 순서/AX/GUI는 미완료입니다.
