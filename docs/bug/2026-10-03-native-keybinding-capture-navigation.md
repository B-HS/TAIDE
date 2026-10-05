# Native keybindings의 popup Escape·캡처 focus navigation

## 대상 파일

`native/taide-native-app/src/keybinding-editor.rs`, `application.rs`.

## 리포트

popup이 열려도 modal이 Escape를 먼저 input.events에서 제거했습니다. 캡처 중 Tab/화살표는 egui frame 시작의 focus direction으로 이미 처리되어 이후 Key event 제거만으로 navigation을 막지 못했습니다. 최소 raw 검사에서 popup Escape 미보존과 Tab chord save0건의 두 RED를 확인했습니다.

## 수정·검증

popup open 상태에서는 modal이 key events를 소비하지 않습니다. capture에서는 current focus direction을 취소하고 capture button의 EventFilter를 설정합니다. underlying pointer blur는 허용하고 실제 focus 소유권을 확인한 뒤 적용합니다. App raw window blur는 viewport의 pending/editor chord를 초기화합니다.

변경 raw 검사2 PASS(0.24초), 후속 icon UI 변경 영향1 PASS(0.56초), 최종 app lib/bin/test strict exit0(11.37초)입니다. 일반 query focus trap·OS blur·full App/aux/IME/AX·같은 frame/multi-pass gate는 `keybinding-icons-and-input` QA에 남습니다.
