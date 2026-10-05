# Native 터미널의 문자·마우스 입력 순서

## 대상·재현

`native/taide-native-app/src/terminal_surface.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`입니다.

실제 Views.show에서 `a → middle press → b → middle release → wheel → c`를 보냈습니다. child는 이 순서의 독립 literal byte를 읽도록 했으나 exit 0 대신 exit 1로 종료했습니다. 관련 검사는 compile 5.13초·suite 0.39초·exit 101입니다. UI가 mouse/wheel batch를 모두 보낸 뒤 keyboard events를 처리한 것이 원인입니다.

## 수정

captured mouse/wheel에 완료 frame과 필터링된 raw stream 내 위치를 저장합니다. raw에서 제거되는 wheel도 다음 retained event 직전의 위치를 유지합니다. UI는 남아 있는 egui event를 원래 raw 순서에 대응시키고 각 event 앞까지만 pointer queue를 처리한 뒤 keyboard/IME를 전달합니다. keyboard가 없는 나머지 pointer는 마지막에 처리합니다. normal wheel의 receipt 대기 packet도 유효한 다음 frame으로 이어집니다.

keyboard 문자열을 별도 raw queue에 복제하지 않고 이미 남아 있는 Event를 이동합니다. 이미 소비된 egui shortcut은 재실행하지 않습니다. 원본에 없는 UI 주입 event는 native raw 뒤에 처리합니다. 동일 값의 event 일부만 다른 widget이 소비한 모호한 정렬과 원본을 재배열하는 UI 주입은 아직 전체 gate입니다.

## 결과

- [x] 실제 mixed mouse host 1 PASS: compile 7.46초·suite 0.36초, 독립 literal `a / SGR29 press / b / SGR29 release / SGR64 wheel / c` 수신·exit 0·join·TaskSupervisor 0입니다.
- [x] 별도 alt wheel host의 입력을 `Text a / Up / IME commit b / Down / Tab`으로 확장해 1 PASS: compile 3.00초·suite 0.34초, 실제 `a ESC OA b ESC OB TAB` 수신·exit 0·join입니다.
- [x] 변경된 normal wheel의 owner/lifetime/policy 3 PASS: compile 3.48초·suite 0.05초입니다. 같은 mouse mode/encoding/queue 정상 성공은 재사용했습니다.
- [x] 최종 app/host/fixture strict exit 0(2.50초), authored 4파일 exact fmt exit 0입니다. Wry의 기존 17 warnings와 구분합니다.

shared writer 자체 포화의 재시도와 input epoch/agent activity 승인 시점, focus/forced-selection·다중 창·전체 N4/M8는 미완료입니다. 보호 앱·사용자 데이터/OS 설정은 조작하지 않았습니다.
