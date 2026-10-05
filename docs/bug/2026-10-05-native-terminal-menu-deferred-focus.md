# 같은 입력 묶음의 방향키가 포커스를 즉시 연쇄 이동

대상: `native/taide-native-app/src/terminal_surface.rs`의 navigate_menu입니다.

원본 actual Clear에서 같은 JS task ArrowDown 두 번은 각각 Clear 기준 후보 Split의 지연 focus를 예약합니다. native는 첫 key에서 focused를 바로 Split으로 바꾸고 두 번째 key를 그 기준으로 계산해 New terminal로 갑니다. 실제 source1회와 actual AX/owned PTY RED(4.44초/0.41초)로 구분했습니다.

item/search scheduled_focus를 실제 사건 당시 focused와 분리하고 루프 뒤 적용합니다. root Content의 동기 첫/끝 이동과 query의 동기 갱신·후보 없는 item 방향키·기존 window loss 취소는 유지합니다. 신규1 PASS(3.59초/0.39초)·탐색/검색/mixed Space 영향3 PASS(0.20초)·strict1.72초입니다.

정본은 context-menu QA의 같은 batch 방향키 절입니다. source 합성 키는 물리 task/frame 타이밍 증거가 아니며 전체 default action/close/topology/App graph는 미완료입니다.
