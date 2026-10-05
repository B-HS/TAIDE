# native Problems 버튼 Space 실행 시점

## 대상·재현

`native/taide-native-app/src/problems.rs`, `problems-tests.rs`입니다. 원본 `src/shared/ui/icon-button.tsx`는 HTML button이며 Problems 상태바·severity 필터·Close에 쓰입니다. 기존 egui의 `Response::clicked`는 Space keydown을 가짜 클릭으로 처리해 Space 누름과 반복마다 활성화했습니다.

실제 상태바에 포커스를 주고 최초 Space를 보내면 패널이 이미 열리는 RED를 재현했습니다(compile8.35초/suite0.02초). 원본 button과 [공식 W3C APG 입력 예시](https://github.com/w3c/aria-practices/blob/main/content/patterns/button/examples/js/button.js)를 대조해 Space 해제/Enter 누름 경계를 구분했습니다.

## 해결·검증

같은 활성화 판정을 세 버튼에 재사용했습니다. 최초 Space는 pending만 만들고 반복은 무시하며 같은 버튼의 Space 해제에 실행합니다. Enter 반복·실제 Primary 클릭·AX Click은 유지합니다. 포커스 이탈은 pending을 취소합니다. 현재 pending은 소유자 안의 viewport별 response ID로 격리하며 소유자 폐기 시 회수합니다.

신규 실제 입력1 PASS(compile6.44초/suite0.03초), 변경 영향을 받는 기존 포인터 입력1 PASS(compile0.22초/suite0.03초)·키보드/스크롤 입력1 PASS(compile0.22초/suite0.06초), 당시 app strict exit0(15.04초)입니다.

## 프레임 batch 후속

같은 프레임 두 Enter가 한 번의 토글로 축약돼 닫힌 패널을 여는 RED를 재현했습니다. boolean 대신 사건별 활성화 횟수를 유지합니다. 짝수 토글도 실제 닫기/재열기를 수행해 로컬 필터 초기화를 보존하며 여러 Space 누름/해제와 repeat 무시도 검사합니다. 신규 batch2건 PASS(compile10.52초/suite0.04초), scroll 소유권 변경 뒤 관련6건 PASS(compile11.22초/suite0.09초)이며 최종 app lib/bin/tests strict exit0(17.21초)입니다. 전체 혼합 사건 순서/AX/GUI는 미완료이며 native-problems QA가 정본입니다.
