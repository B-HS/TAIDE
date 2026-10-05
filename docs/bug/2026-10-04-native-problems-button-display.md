# native Problems 버튼 표시 차이

## 대상·근거

native/taide-native-app/src/problems.rs·problems-tests.rs입니다. 원본 status-bar.tsx·problem-severity-filter.tsx·problems-panel.tsx와 global.css, 설치된 Tailwind preflight와 Radix Popper, pinned egui Popup/RectAlign 소스를 대조했습니다.

## 관찰·해결

버튼 배경 모서리가0px이고 status 높이는20px였습니다. 원본 rounded-sm은4px이고 status의11px·unitless line-height1.5는16.5px입니다. CountButton에 높이·tooltip align을 명시하고 status/filter/Close 배경을4px로 바꿨습니다. 필터와 Close의20px는 유지했습니다.

모서리 RED는 compile6.63초/suite0.03초입니다. 수정 뒤 실제 status tooltip은 버튼110~126.5px 아래145.5px에 나타났습니다(compile6.30초/suite0.03초). centered top 후보가 왼쪽 가로 경계를 넘자 egui 기본 대안 순서가 bottom-start를 선택했습니다. 같은 방향 start/end를 우선하고 그 다음 반대 방향을 시도하도록 수정했습니다. Radix의 정확한 shift/compositor 전체를 구현했다고 주장하지 않습니다.

## 실제 검증·잔여

`CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --lib problems_버튼은_원본 --locked --offline --target-dir experiments/native-shell-spike/target` 신규1 PASS(compile5.32초/suite0.06초)입니다. 실제 RectShape·AX bounds·지연 hover tooltip 텍스트 위치를 status/filter/Close3곳에서 확인했습니다. app lib/tests strict exit0(15.17초)·Rust2fmt이며 성공 검사를 재실행하지 않았습니다. 이전 입력/viewport/font/bin strict는 재사용합니다.

## 비활성 hover·tooltip 기본 표시 추가 수정

비활성 필터 hover 배경에 원본 opacity-60이 빠져 alpha255/기대153 RED를 확인했습니다(compile10.26초/suite0.04초). CountButton의 배경·icon/text에 같은 opacity를 전달하고 status/선택 상태는1을 유지했습니다. 실제 AX Click off/on·hover·text·선택 복귀 신규1 PASS(compile4.56초/suite0.04초)입니다. CSS의 offscreen group 합성 픽셀 전체를 확인한 검사는 아닙니다.

tooltip의 기본 popup 색상도 제품 theme와 달라 RED(compile7.50초/suite0.03초)였습니다. resolved tooltip.background/border·radius6px·border1px·가로12px/세로6px padding·12px UI font/line-height18px·app.foreground를 공통3곳에 연결했습니다. 실제 dark/light theme/shape/galley/치수 신규1 PASS(7.59초/0.04초), tooltip 치수 변경의3곳 방향 영향1 PASS(0.23초/0.03초), app lib/tests strict exit0(16.09초)입니다. 정확한 명령과 앞선 성공 재사용은 native-problems QA에 기록했습니다.

원본 DOM 실측·tooltip 실제 AppProviders400ms(egui 기본500ms)/focus/skip-delay·arrow/animation/text-balance·정확한 collision shift·inactive opacity 그룹 합성·focus ring·전체 font/tnum/CJK/GUI와 M8은 미완료입니다. 제품TS·manifest/lock·보호 bundle·사용자 앱·OS·Git은 불변입니다.
