# 공용 native Button의 Space keydown 오활성화

## 대상·관찰

기존 native egui `Context.get_response`가 모든 click Sense의 Space keydown을 fake primary click으로 만들었습니다. 실제 원본 HTML Button은 Space press/repeat click0, release click1이며 취소/blur click0입니다. 실제 native Button 검사2 RED로 keydown·cancel-up 시 premature click을 확인했습니다.

## 해결·검증

일반 Button 전용 current-pass 결과와 viewport arm으로 Space release 활성화를 연결했습니다. generic row/menu 정책은 유지하고 unseen/disabled/blur/consumed key는 회수합니다. Tooltip은 실제 완료된 Button 기본 click 뒤 event 소비를 action 전 취소와 구별합니다.

원본 DOM5 1회 측정·native 신규6/영향8 성공·engine/app strict23.30초입니다. 정확한 실행/실패 정정·source/범위는 `../quality-assurance/2026-10-05-m8-native-button-default-keys.md`가 정본입니다. Problems fixture의 finish 누락과 원본 Presence에 맞지 않는 mixed 재진입 기대를 바로잡았고 제품 동작으로 숨기지 않았습니다.

늦은 App capture의 arm 취소·동일-pass 여러 default actions·동적 owner/IME/viewport/메뉴 전체 계약과 App GUI는 미완료입니다. M8 전체 완료로 주장하지 않습니다.

## 후속: 일반 Button 전역 캡처

기존 App post-draw window router는 유효한 Enter override가 전역 액션을 실행해도 이미 발생한 Button click을 막지 못했습니다. 알려진 ordinary Button의 Enter/Space batch에 production router를 shell draw 전 적용해 same-pass와 split-pass의 default action을 막았습니다. engine은 viewport/직전 Button 등록을 읽기 전용으로 노출합니다. 신규2/영향3의 최종 관련 성공·strict21.19초와 잘못된 fixture/이전 binary 재사용 조건은 `../quality-assurance/2026-10-05-m8-button-window-capture.md`가 정본입니다.

혼합 pointer/AX/Tab/Escape/IME와 window→document→target 전체 순서, 동적 topology·여러 action 수·GUI는 여전히 미완료입니다. 늦은 event 소비를 end_pass에서 지우거나 이미 실행한 callback을 사후 취소하는 방식은 사용하지 않았습니다.

## 후속: Escape의 선행 focus 해제

window/document 순서만 수정해도 같은 RED가 남았습니다. egui Focus.begin_pass가 App UI보다 먼저 raw Escape로 Button focus를 해제해 window capture의 Button 판정 자체가 사라졌습니다. 실제 원본2사례에서 Escape·window 취소 모두 Button focus를 유지함을 확인하고 현재 viewport의 previous ordinary Button에만 focus 유지 정책을 적용했습니다. normalized survivor/raw index 대응도 수정해 선행 Enter 소비 후 이웃 Space keyup을 잘못 제거하지 않습니다. 신규2/영향9 PASS·strict21.04초이며 `../quality-assurance/2026-10-05-m8-tooltip-window-capture.md`가 정본입니다. mixed focus/Tab/IME·전체 owner graph/GUI는 미완료입니다.
