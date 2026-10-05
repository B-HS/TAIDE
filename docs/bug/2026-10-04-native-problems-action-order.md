# native Problems 필터·접기·닫기 사건 적용 순서

## 대상·관찰

`native/taide-native-app/src/problems.rs`, `problems-tests.rs`입니다. 실제 AX 진단 Click 뒤 error 필터 Click을 보냈지만 진단 Open이0개였습니다. 기대는 먼저 실행된 진단 Open1개입니다. 신규 최소 재현 RED는 compile6.94초/suite0.04초입니다.

기존 처리는 화면상의 header를 먼저 렌더하면서 즉시 필터를 변경하고, 나중에 행을 순회했습니다. 행 입력이 먼저 도착해도 필터가 먼저 적용돼 행이 사라졌습니다. 반대로 접기·닫기는 뒤늦게 확정해 먼저 사라졌어야 할 행의 입력이 실행되는 차이도 있었습니다.

## 해결

실제 response로 수집한 `Filter/Collapse/Open/Close`를 사건 인덱스 순서로 적용합니다. Filter는 원본처럼 severity를 토글하고 각 단계의 실제 groups/rows/viewport 수명을 갱신합니다. Collapse는 남아 있는 그룹만 토글합니다. Open은 해당 단계에서 필터로 제거되거나 접힌 진단을 거절하며, 원래 store의 Diagnostic Arc identity를 확인합니다. Close는 panel owner를 회수하고 뒤의 동작을 중단합니다. 앞서 정상 발행한 Open은 그대로 보존합니다.

paint 중 UI row/header 순서로 상태를 변경하지 않습니다. 적용 후 filter/collapse/close 상태가 바뀌면 명시적으로 repaint를 예약해 다음 렌더가 실제 새 상태/viewport를 표시합니다. debounce·동작 병합·임의 순서나 외부 dispatch 재발행으로 대체하지 않았습니다. status toggle·여러 slot/새로 나타난 행까지 포함하는 전체 수명은 별도 미완료입니다.

## 검증·실패 구분

- 실제 렌더된 node에 진단 먼저/제어 먼저를 각각 보내는6조합을 신규1검사로 확인했습니다. Filter·Collapse·Close마다 먼저 Open이면1개, 먼저 제어로 행이 사라지면0개이며 최종 active/rows/panel owner를 확인합니다.
- 영향 검사10건 실행의 결과는9 PASS/1 FAIL,compile6.80초/suite0.10초입니다. 신규6조합과 Focus/동일-key 소비·기존 header/filter/scroll lifecycle9건은 통과했습니다. 실패는 기존 fixture가 필터 정착 프레임의 새 warning 텍스처2개를 clear하지 않고 다음 output으로 대입해 발생한 TexturesDelta Drop 오류입니다.
- headless fixture의 해당 output에 명시적 `textures_delta.clear()`를 적용하고 실패한1건만 `cargo test … --lib problems::tests::problems_입력은_필터`로 재실행했습니다. 1 PASS,compile5.37초/suite0.06초입니다. 상태 변경 뒤 repaint 예약도 이때 추가했고 이미 성공한 입력9건은 재실행하지 않았습니다.
- 최종 app lib/bin/tests strict exit0,15.42초입니다. Rust2파일 exact rustfmt·문서 포맷과 whitespace 검사도 확인했습니다. 기존 glyph/font·UI splitter/두 viewport 성공은 재사용합니다.

실제 GUI/OS·Tab/여러 pointer·동적 행 재등장·상태바/여러 slot 전체 입력/AX와 전체 M8은 미완료입니다. 보호 bundle·OS·제품 TS·manifest/lock·Git은 변경하지 않았습니다.

후속 정정: 위 Diagnostic Arc identity 검사는 필터 변경으로 같은 index에 새 진단이 배정되는 원본 node 계약을 보존하지 못했습니다. 현재 구현은 사건 적용 시점의 path/index로 실제 진단을 조회하며, 재배정/사라진 행의 RED·3조합 성공은 [row-retarget bug](2026-10-04-native-problems-row-retarget.md)가 정본입니다. 앞선 순서/Close 거절 정책은 유지합니다.
