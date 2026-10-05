# 숨김·logic-only blur에서 터미널 FocusOut 누락

## 대상·재현

`native/taide-native-app/src/terminal_surface.rs`의 focus 처리가 해당 View의 `show` 호출 안에만 있었습니다. 다른 editor로 전환해 표시되지 않거나 egui가 logic-only tick을 수행하면 기존 focus/preedit가 남았습니다. 실제 합성 PTY의 focus lifecycle 검사에서 필요한 보고가 오지 않아 3초 timeout으로 실패했습니다.

## 원인·수정

포커스의 수명을 렌더 함수 호출과 동일시한 것이 원인입니다. View의 widget id·표시 frame과 egui의 현재 widget/raw window focus·live viewport 목록으로 수명을 확인합니다. 표시 완료 및 background tick에서 FocusOut·preedit 해제를 처리하고, View 전환은 이전 Out을 새 In보다 먼저 보냅니다. Session 시작 중 표시도 기록하며 닫힌 viewport는 egui counter를 다시 조회하지 않습니다.

focus 보고가 일반 사용자 입력 제출의 scroll-to-bottom 동작을 공유하던 부분도 분리했습니다. egui에서 더 이상 쓰지 않는 Disabled IME 이벤트에 의존하지 않습니다.

## 증거·잔여

실제 PTY의 10개 focus 보고·문자 x 순서, 숨김/복원·반복 logic blur·같은 Session의 View 변경·headless auxiliary 제거가 통과했습니다. x만 input epoch를 증가시켰고 child exit 0·join/task 0을 확인했습니다. 상세 명령·실패/성공 시점은 `docs/quality-assurance/2026-10-03-m8-native-terminal-focus-lifetime.md`가 정본입니다.

Outbox 포화 때 focus report 재시도와 DECSET 1004 현재 상태 query, 실제 OS/GUI/전체 IME는 아직 미완료입니다. 이 기록은 전체 focus 입력 동등성의 완료 선언이 아닙니다.
