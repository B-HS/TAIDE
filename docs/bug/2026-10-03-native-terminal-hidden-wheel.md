# 일반 terminal wheel의 숨김 뒤 유실

## 대상·증상

`native/taide-native-app/src/terminal_surface.rs`의 일반 raw wheel 큐입니다. 실제 writer count=1·Outbox 64개 포화 뒤 wheel을 캡처하고 탭을 숨기면 child가 alt-buffer 방향키를 받지 못했습니다. actual PTY 검사 `hidden_wheel`의 최초 결과는 exit 101, `hidden wheel was not delivered`입니다.

## 원인·해결

raw hook은 직전 frame의 wheel target이 없으면 큐를 지웠고, `scroll_wheel`도 현재 frame이 아니면 큐를 지웠습니다. 전송 슬롯을 기다리는 이미 소유한 packet과 새 wheel hit 자격을 같은 수명으로 처리했습니다. background drain은 mouse만 처리했습니다.

새 hit의 target 신선도는 유지하고 기존 대기는 보존합니다. packet은 원래 geometry와 checked 전역 캡처 순번을 소유합니다. mouse/wheel 공통 drain은 처리/완료 frame과 keyboard event 경계에서 원래 순서로 Outbox에 승인합니다. 일반 scrollback의 로컬 offset은 슬롯 없이 처리하며 alt 방향키는 슬롯을 기다립니다. disabled/종료/취소 정리는 미승인 packet에 적용합니다.

실제 PTY 1 PASS(1.59초, 64 x→ESC OA·ESC OB→새 view z·epoch+67·exit 0/join/task 0), 신규 숨김/local/geometry/cancel unit 1 PASS(0.02초), 영향 mouse 2·wheel 정책 3·혼합 alt wheel 1 PASS와 app/fixture strict exit 0입니다. 정확한 명령·재현 시간·한계는 [hidden-wheel QA](../quality-assurance/2026-10-03-m8-native-terminal-hidden-wheel.md)에 기록했습니다.

raw 대기와 query/focus의 pre-Session 순서, mode/buffer ABA·전체 VT/GUI/OS/aggregate·213 view/cutover/TS 제거를 완료했다고 주장하지 않습니다.

후속에서 buffer 왕복 뒤 최종 mode가 같아도 stale wheel이 방향키 1개를 생성하는 실제 RED를 확인했습니다. 기존 buffer/rows epoch 정리에 일반 raw 큐도 연결해 alternate 왕복·RIS reset·rows resize의 미승인 packet을 지웁니다. 단순 숨김과 이미 Session에 들어간 encoded pending은 이 정리로 폐기하지 않습니다. `wheel_epoch`의 세 경계 1 PASS와 기존 숨김/local 1 PASS, strict exit 0이며 mouse tracking 자체의 mode ABA·columns-only resize·encoding 사이 변경은 남습니다. 후속 결과의 정본도 위 QA입니다.
