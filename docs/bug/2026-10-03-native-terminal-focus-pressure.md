# 일반 입력 큐 포화에서 focus 보고 유실

## 대상·원인

`terminal_surface.rs::Outbox::submit`이 모든 전송을 같은 64개 admission으로 처리했습니다. `change_focus`는 UI 상태를 먼저 바꾸므로 보고가 거절되면 숨김/복원 다음 tick에서 같은 변화가 다시 발생하지 않았습니다. actual writer count=1·Outbox 64개를 채운 후 Out/In 보고를 기다리는 신규 검사로 유실을 재현했습니다.

## 수정

일반 입력 예산과 별도로 하나의 bounded focus batch를 보유하고, 포화 동안 발생한 보고를 encoding 시점의 바이트 순서대로 붙입니다. 이전 일반 입력 뒤에 drain하며 batch가 남아 있는 동안 새 일반 입력을 받지 않습니다. 공유 Session 소유권·writer payload/retained capacity를 검사하고 다른 세션/사용자 입력은 합치지 않습니다. Session의 clear/cancel은 batch도 회수합니다.

## 결과·한계

actual PTY의 63개 x·포화 중 Out/In/Out/In·이후 z/Out 순서와 epoch·종료/회수, control token 상한·identity 원자적 거절이 통과했습니다. 상세 명령과 시점은 `docs/quality-assurance/2026-10-03-m8-native-terminal-focus-pressure.md`가 정본입니다.

focus 전용 byte 상한 자체를 넘는 새 보고는 명시적으로 거절합니다. output query와 UI queue 사이의 포화·전체 순서 문제, 실제 OS/GUI·모든 종료 조합은 아직 미완료입니다.
