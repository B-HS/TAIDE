# native Problems 빈 필터의 스크롤 수명

## 대상·재현

`native/taide-native-app/src/problems.rs`, `problems-tests.rs`입니다. 원본 ProblemsPanel은 필터 결과가 비면 scrolling element를 제거합니다. native는 빈 목록에서도 같은 persisted scroll state를 유지했습니다.

실제1000개 진단을 wheel로 스크롤한 뒤 error 필터를 off/on하면 offset이1000px/기대0인 RED였습니다(compile6.53초/suite0.04초). 직접 scroll state를 주입하지 않았습니다.

## 해결·검증

빈 목록에서 viewport 소유자를 폐기하고 nonempty 복귀 때 새 mount를 생성합니다. guard Drop은 실제 egui scroll State를 제거하며 닫기/슬롯 회수도 같은 수명을 따릅니다. pruning은 input lock 밖에서 수행하고 동일 Context/ID guard는 매 프레임 교체하지 않습니다.

실제 pointer off/on 및 한 프레임 두 Enter off/on에서 새 ID·offset0·이전 State 제거·첫 그룹 복귀와 닫기 회수를 확인했습니다. 관련 Problems6건 PASS(compile11.22초/suite0.09초)·당시 app lib/bin/tests strict exit0(17.21초)입니다.

두 실제 egui viewport의 독립1000px/500px wheel·child 제거/복귀 offset0·슬롯 reconcile 양 State 회수 신규1 PASS(compile7.29초/suite0.03초)·추가 fixture의 app lib/tests strict exit0(1.99초)입니다. 기존 app lib/bin 성공은 재사용하며 전체 viewport DPI/glyph/분할 cache나 OS 창 검증은 아닙니다.
