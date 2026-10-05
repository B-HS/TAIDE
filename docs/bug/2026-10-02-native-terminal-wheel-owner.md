# Native terminal wheel의 global smoothing tail·multi-pass 소유권

## 대상

`native/taide-native-app/src/terminal_surface.rs`, `application.rs`, `tests/terminal-host.rs`입니다. 실행 증거와 남은 경계는 `docs/quality-assurance/2026-10-02-m8-native-terminal-wheel-owner.md`에 있습니다.

## 관찰과 원인

terminal wheel을 UI에서 처리하고 공개 smooth delta를 지워도 egui의 내부 smoothing 큐는 이미 raw wheel을 받았습니다. 다음 frame에 pointer를 다른 ScrollArea로 옮기기만 해도 해당 offset이 100에서 91.47196으로 움직였습니다. UI 단계의 공개 delta 초기화만으로 event ownership을 보장할 수 없습니다.

actual 두 UI pass의 추가 재현에서는 pass 번호로 등록한 managed-route가 두 번째 pass에 false로 바뀌었습니다. `request_discard`는 같은 프레임을 다시 그리므로 pass와 완료 frame generation은 다릅니다.

## 해결

eframe raw hook에서 event-time pointer·직전 완료 frame의 clipped/global target·layer·viewport·유일 owner를 확인하고 원본 wheel을 bounded view queue로 분리합니다. managed UI는 이 packet만 처리하며 다른 surface의 unclaimed wheel을 가져가지 않습니다. 같은 frame의 logic 재호출과 UI 다중 pass는 완료 frame 번호를 공유하고, 다음 frame의 stale packet은 정리합니다. 비공개 egui state·검사기 억제·vendor 변경은 사용하지 않았습니다.

## 실제 판정

최종 관련 wheel 3 PASS·actual PTY 영향 2 PASS·authored strict/fmt/diff exit 0입니다. smoothing-tail와 multi-pass assertion을 유지했습니다. mouse protocol·actual auxiliary window/OS·전체 M8 판정은 별도 미완료입니다.
