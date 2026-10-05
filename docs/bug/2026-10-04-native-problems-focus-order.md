# native Problems 같은 프레임 AX Focus 전환

## 대상·관찰

`native/taide-native-app/src/problems.rs`, `problems-tests.rs`입니다. 실제 렌더된 두 진단 행으로 AX Focus 두 번째 행→Enter 눌림/해제→AX Focus 첫 번째 행→Enter 눌림/해제를 보내면 위치 `[1,1]`이 반환됐습니다. 기대는 `[2,1]`입니다. 신규 검사 RED는 compile6.78초/suite0.04초입니다.

원본 role-button 핸들러는 각 사건의 target에서 실행합니다. 기존 native 처리는 프레임 최종 `Response::has_focus`만 참조해 중간 Focus를 잃었고, 모든 같은 Enter를 한 위젯에서 소비했습니다. 단순 Event 값 비교는 같은 형태의 서로 다른 Enter 사건을 구별할 수 없습니다.

## 해결·근거

panel 안의 header/Close/행은 같은 `ActivationInputs` 사건 snapshot과 원래 소비 인덱스를 공유합니다. 명시적 root AX Focus가 있을 때 공식 Memory의 `had_focus_last_frame`으로 시작하고 각 Focus의 target을 따라 키를 배정합니다. 포커스 이탈 시 armed Space를 취소하며 Focus가 없는 기존 경로는 현재 response focus를 유지합니다. disabled/window-unfocused·다른 tree/data가 있는 Focus는 활성화 조건을 완화하지 않습니다.

소비는 Event 값의 전체 삭제가 아니라 원래 사건 인덱스별로 적용합니다. 이미 소비한 인덱스를 건너뛴 snapshot과 현재 미소비 events를 순서대로 대조하므로 서로 같은 Enter 눌림/해제가 다른 target으로 전달돼도 다음 target의 사건을 없애지 않습니다. pinned egui0.36.2 Memory::begin_pass/interested_in_focus/had_focus_last_frame·InputState/Response 소스를 확인했습니다. native 전용 OS 접근성 호출이나 앱 조작은 하지 않았습니다.

## 실제 검증·남은 범위

app `cargo test … --lib problems::tests -- --skip problems_viewport별_scroll`은 제품 수정 후9 PASS,compile11.13초/suite0.09초입니다. 신규 Focus/Enter 위치·최종 실제 Memory focus·키 소비와 헤더 Space/반복/기존 pointer/AX·필터/닫기 영향을 확인했습니다. 당시 app lib/bin/tests strict exit0,17.50초입니다. 이후 상태 변경 순서 수정의 영향 성공은 native-problems QA가 정본입니다.

AX Focus와 pointer/Tab·키가 함께 있는 전체 순서·상태바/여러 slot/viewport·실제 VoiceOver는 미완료입니다. 명시적 AX Focus 성공을 전체 focus parity로 확대하지 않습니다. source2파일 exact rustfmt이며 보호 bundle·OS/IME·제품 TS·manifest/lock·Git은 변경하지 않았습니다.
