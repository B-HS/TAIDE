# native Problems 진단 열기 요청 순서

## 대상·관찰

`native/taide-native-app/src/problems.rs`, `problems-tests.rs`, 읽기 전용 actual caller `application.rs::problems_panel`입니다. 같은 프레임에 두 진단 행으로 실제 AX Click을 두 번째→첫 번째→두 번째 순서로 보냈지만 반환 위치가 `[1,2,2]`였습니다. 기대는 `[2,1,2]`입니다. 신규 headless 최소 재현은 compile6.67초/suite0.03초에 RED였습니다.

원본 `ProblemRow`의 실제 활성화 핸들러와 `ProblemsPanelContainer`는 사건마다 해당 위치의 preview open을 호출합니다. native는 각 행의 활성화 개수만 세고 UI row iteration 순서로 Open을 append해 사건 순서를 잃었습니다. Vec 도입만으로 횟수는 보존됐지만 순서는 보존되지 않았습니다.

## 해결

panel 시작의 미소비 사건 snapshot에서 활성화별 원래 인덱스를 보존합니다. 실제 소비 목록에 남은 사건만 받아 기존 Space 최초/반복/해제·Enter·AX Click과 Space owner 정책을 유지합니다. Primary 클릭은 실제 release 사건 인덱스로 배치합니다. `(사건 인덱스, Open)`을 정렬한 뒤 actual caller에 기존 Vec으로 반환하며 caller는 전달 순서를 그대로 유지합니다. 별도 debounce·진단 위치 재정렬·중복 제거는 하지 않습니다.

설치된 egui0.36.2 InputState의 in-order `events`와 Event PartialEq, Response clicked_by 소스를 확인했습니다. 이미 소비된 사건을 raw input에서 다시 실행하지 않으며 합성 AX Click은 실제 renderer가 생성한 node를 대상으로 보냈습니다.

## 검증·남은 경계

- 변경 영향을 받는 실제 입력7건 PASS,compile6.88초/suite0.08초입니다. 명령은 app `cargo test … --lib problems::tests -- --skip problems_viewport별_scroll`이며 좁은 순서 RED와 기존 헤더/행/필터/닫기 영향을 함께 확인했습니다.
- app lib/bin/tests strict exit0,17.68초입니다. 이후 추가한 혼합 입력 fixture만 별도 compile/PASS로 확인했으며 생산 코드 strict를 재실행하지 않았습니다.
- 신규 혼합 입력1건 PASS,compile6.93초/suite0.04초입니다. AX 첫 행→실제 pointer 두 번째 행 release→AX 첫 행은 `[1,2,1]`, 포커스된 두 번째 행 Enter→AX 첫 행→반복 Enter는 `[2,1,2]`이며 소비 후 키/AX가 남지 않았습니다.
- 변경하지 않은 두 viewport scroll·UI splitter·UI/terminal 글꼴 성공은 재사용합니다. 같은 성공을 반복 측정하지 않았습니다.
- 같은 프레임의 중간 Focus/Tab·여러 pointer target·collapse/filter/Close·status toggle까지 포함하는 전체 사건 적용 순서는 미완료입니다. 진단 Open 순서 성공을 전체 mixed-input parity로 확장하지 않습니다.

Rust2파일 exact rustfmt·tracked diff check exit0·각 untracked source의 no-index check는 빈 출력/exit1(새 파일 diff 존재)입니다. 보호 bundle·사용자 앱·OS·제품 TS·Git은 조작하지 않았으며 전체 M8은 계속 진행 중입니다.
