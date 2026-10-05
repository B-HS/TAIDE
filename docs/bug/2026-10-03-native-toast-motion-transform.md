# Native toast의 원본 motion·signed transform

## 대상 파일

`native/taide-native-app/src/{toast.rs,toast-motion.rs}`

## 리포트

이전 native는200ms 선형 opacity와 순간 배치, 접힌 card의 양수 축소를 사용했습니다. 원본 Sonner2.0.7의 CSS transition/ease·음수 scale·제거 offset/수명과 달랐습니다. source-level 상태를 연결했으며 실제 전체 픽셀 동등성은 미완료입니다.

## 상세

- property별 target/start/current/reversal shortening을 유지해 프레임마다 전환을 재시작하지 않습니다. 원본400/500/200/100ms와200ms 제거를 구분합니다.
- 음수 scale을 일반 egui Shape transform에 직접 전달하면 음수 radius/stroke/Rect가 됩니다. 소유 shape의 mesh vertex와 normalized hit Rect를 변환하며 양수는 일반 Shape를 유지합니다.
- graphics_mut 안에서 content_rect를 다시 조회한 첫 구현은10.02초 뒤 lock panic으로 재현됐습니다. Context의 잠금 중 다른 Context 조회를 하지 않고 viewport rect를 밖에서 가져오도록 수정했습니다. vendor/검사기 억제는 없습니다.
- source 수식과 fixture의 반대 종료 방향/첫 프레임 즉시 표시/이전-pass response/Duration 정밀도를 구분했습니다. 실패·수정·실제 명령 결과는 `docs/quality-assurance/2026-10-03-m8-native-toast-motion.md`에 있습니다.

swipe/hover/shadow/reduced motion·source mounted timing·full App/실기/성능과 별개 keybindings Tab RED는 이 수정의 해결 근거가 아닙니다.
