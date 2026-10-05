# 숨긴 native 터미널의 mouse release 유실

## 증상·원인

writer/Outbox가 찬 뒤 터미널 view를 숨기면, `terminal_surface.rs` raw hook이 직전 frame target이 없다는 이유로 mouse packet과 held 버튼을 지웠습니다. Outbox 재시도는 이미 인코딩된 입력만 처리했으므로 아직 승인되지 않은 release가 PTY에 도달하지 않았습니다.

## 해결

새 press/wheel hit와 이미 시작한 document 범위 capture를 구분했습니다. 캡처 packet의 geometry·순번을 유지하고, 완료 frame 또는 해당 keyboard event 이전의 packet만 global drain에서 Outbox에 제출합니다. 다른 view의 새 위치로 이전 이벤트 좌표를 재해석하지 않습니다. disabled/retire/앱 취소는 명시적으로 정리합니다.

actual count=1 PTY의 64개 포화→숨김→release 검사에서 최초 3초 timeout RED, 구현 뒤 1 PASS입니다. 다른 view의 같은 frame release/Text 순서까지 확장한 최종 검사는 1 PASS(1.66초)이며 정확한 바이트·epoch+68·exit 0·join/task 0을 확인했습니다. 자세한 명령·영향 검사·남은 focus/OS gate는 `docs/quality-assurance/2026-10-03-m8-native-terminal-hidden-input.md`에 기록했습니다.
