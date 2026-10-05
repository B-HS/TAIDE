# Native 터미널 마우스 재수집·해제 유실

## 대상

`native/taide-native-app/src/terminal_surface.rs`, `src/application.rs`, `experiments/native-shell-spike/vendor/eframe/src/{epi.rs,native/epi_integration.rs}`입니다.

## 관찰·원인

1. actual `Context::run_logic` 뒤 같은 raw pointer press/release를 hook에 다시 전달하면 패킷 개수가 2 대신 4였습니다. eframe의 logic-only 경로가 필터링된 입력을 pending으로 저장하고 다음 호출 앞에 붙이기 때문입니다. wheel은 첫 hook에서 제거되지만 focus/drag에 필요한 pointer는 원본에 남아 중복 복사됐습니다.
2. press·64회 move·release를 연속 입력한 실제 raw queue에서 마지막 패킷이 Release가 아니었습니다. 고정 64칸을 move가 모두 사용해 버튼 해제를 거절했습니다.
3. 기존 `process_mouse`는 남은 receipt 수를 넘으면 앞에서 모은 입력까지 Err로 버렸고, 다음 frame raw hook도 남은 packet을 지웠습니다. 이 분기는 source에서 확인했고 아래 연속 frame 검사로 수정 결과를 확인했습니다.

## 수정

eframe이 pending event 개수를 전달하고 Views는 그 prefix에서 pointer/modifier 상태만 복원합니다. 새 이벤트만 패킷으로 수집합니다. 기존 hook의 기본 동작을 유지하며 무제한 입력 복사·문자열 hash·event 값 추측을 추가하지 않았습니다.

64칸 안에서 현재 누른 버튼마다 release 한 칸을 예약합니다. 새 press/move/wheel을 수용할 수 없으면 명시적 오류로 거절하고 이전 버튼 상태를 복구합니다. 유효한 동일 view의 미전송 패킷은 다음 frame에 유지하고 남은 receipt 수만큼만 처리합니다. hidden/stale target·mode/epoch/disabled 정리는 계속 적용합니다.

## 결과

replay RED(exit 101, 0.01초) 뒤 adapter 1 PASS(0.04초), 실제 Views→PTY 1 PASS(0.35초)입니다. 별도 release RED(exit 101, 0.01초) 뒤 queue 1 PASS(0.01초)에서 64칸·최종 release·첫 2개/다음 62개 순서·writer/task 회수를 확인했습니다. 후속 일반 raw mouse/keyboard interleave 수정은 `2026-10-02-native-terminal-input-order.md`에 기록했습니다. 실제 writer의 별도 포화 시 재시도·동일 event 일부 소비의 모호한 정렬·전체 N4/M8는 미완료입니다.
