# native toast reduced-motion 누락과 첫 Area sizing

## 대상 파일

`native/taide-native-app/src/{toast.rs,toast-motion.rs,toast-swipe.rs,motion-preference.rs,application.rs}`

## 리포트

native toast가 OS reduced-motion을 읽지 않아 원본의 transition/animation:none을 재현하지 못했습니다. 모델 정책을 연결한 뒤 첫 renderer에서는 egui Area의 자동 sizing pass 때문에 즉시 보여야 할 텍스트가 invisible이었습니다.

## 원인과 수정

1. baseline에서100ms opacity가0.4085106이었습니다. reduced-motion target 전환을 duration0으로 적용하고 child·hover·shadow·height까지 연결했습니다. 정책2 PASS(0.00초)입니다.
2. 원본 swipe-out의 animation:none은 마지막 이동량을 유지합니다. 일반 exit/fade를 강제로 적용하지 않고 기존200ms 삭제를 유지했습니다.
3. 첫 renderer에서 opacity 모델이 아니라 Area의 invisible sizing이 원인이었습니다. 공개 sizing-pass 판정과 bounded request_discard를 reduced-motion에만 적용했습니다. 실패1회 뒤 renderer1 PASS(0.02초)이며 일반 animation 경로는 변경하지 않았습니다.
4. macOS 읽기 전용 getter·workspace observer repaint·Drop을 연결했습니다. private notification center의 두 창별 수명1 PASS(0.28초)입니다. removeObserver token의 E0308/E0282 compile 오류는 typed AsRef<AnyObject>로 해결했으며 unsafe cast/검사 억제를 사용하지 않았습니다.

실제 명령·의존성·남은 OS/browser/다른 플랫폼 위험은 [reduced-motion QA](../quality-assurance/2026-10-03-m8-native-toast-reduced-motion.md)에 기록합니다. 실제 사용자 설정이나 보호 앱은 변경하지 않았고 M8 전체 완료가 아닙니다.
