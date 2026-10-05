# M8 native 터미널 마우스 adapter

## 대상·범위

`native/taide-native-app/src/{terminal_surface,application}.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`, eframe vendor의 `src/{epi.rs,native/epi_integration.rs}`입니다. 기존 live mouse encoder를 실제 egui 입력과 PTY에 연결했습니다. N4-B의 기본 adapter checkpoint이며 전체 N4/M8는 미완료입니다.

원본은 설치된 xterm 6.0.0의 `CoreBrowserTerminal`, `SelectionService`, `CoreMouseService`, `CoreTerminal`과 egui/eframe 0.36.2 실제 source입니다. 공식 [App hook/logic 계약](https://docs.rs/eframe/0.36.2/eframe/trait.App.html)과 [Context API](https://docs.rs/egui/0.36.2/egui/struct.Context.html#method.run_logic)도 확인했습니다.

## 구현과 실제 오류

- [x] viewport/pane/tab의 직전 enabled·clipped/global layer target 하나만 raw mouse를 받습니다. 이벤트 시점 pointer/modifier·세 버튼 상태를 유지하고 press 후 바깥 release/drag를 같은 view에 전달합니다. orphan release와 다른 widget에서 시작한 drag는 거절합니다. PointerGone은 버튼이 눌린 동안 capture를 유지합니다.
- [x] live mode·grid·cell/pixel 좌표를 다시 검사하고 canvas 밖의 capture 좌표를 제한합니다. 같은 cell의 motion은 한 번만 보내며 pixel mode는 pixel 차이를 구분합니다. X10 press-only와 normal wheel fallback을 분리했습니다. tracking wheel은 같은 mouse queue에 들어갑니다.
- [x] raw pointer를 보존하는 경로에서 logic 재호출 시 queue 2→4 RED를 확인했습니다. eframe이 기존 pending prefix 길이를 넘기는 hook을 추가했고 새 입력만 복사합니다. 두 UI pass에서도 한 번만 drain합니다. 기존 hook 사용자는 default 위임으로 유지합니다.
- [x] 64칸 queue에서 release가 사라지는 RED를 확인했습니다. 누른 버튼별 release 자리를 예약하고 초과 새 이벤트에는 오류를 남깁니다. receipt 여유만 처리하고 같은 active view의 미전송 packet을 다음 frame으로 유지합니다. 실제 62개 receipt가 남은 상황에서 입력 2개를 처리하고 다음 frame에 나머지 62개를 처리했습니다.
- [x] user-input epoch에서 wheel fraction까지 0으로 지우던 정책을 원본에 맞춰 수정했습니다. 해당 epoch는 viewport 위치만 재동기화하며 CoreMouseService partial을 유지합니다. 이전 wheel 검사의 0 기대값은 source와 달랐으므로 입력 전 fractional 값 보존 assertion으로 고쳤습니다. 실제 5회 작은 point→user-input epoch→6번째 point가 정확히 한 wheel byte를 내는 별도 adapter 경로도 확인했습니다.
- [x] forced selection은 macOS의 기존 TS/default 정책(false), 그 밖의 Shift 정책을 구현했습니다. 강제 선택된 press는 터미널 보고에서 제외하며 primary drag는 기존 local selection으로 연결합니다. macOS 밖 실기·forced drag/keyboard selection 전체 검증은 남습니다.

초기 adapter 작성 중 RawInput에 modifiers 필드가 없다는 E0609는 설치된 Event/InputState 계약에 맞춰 수정했습니다. 재현 테스트의 run_logic must-use 경고는 반환값을 명시적으로 받아 해결했습니다. suppression·새 dependency·unsafe는 추가하지 않았습니다.

## 검사 증거

모든 Cargo 명령은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`로 직렬 실행했습니다.

| 검사 | 결과 | 확인 범위 |
| --- | --- | --- |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib mouse_adapter -- --nocapture` | replay RED 수정 후 1 PASS, compile 2.75초 / suite 0.04초 | 실제 raw 순서·클릭/해제·outside capture·cell/pixel·orphan/external drag·PointerGone·logic replay·두 UI pass·modifier·partial·X10 fallback |
| `… --lib wheel_policy -- --nocapture` | source에 맞는 partial 보존 수정 후 1 PASS, compile 1.43초 / suite 0.02초 | 기존 wheel 정책에 대한 변경 영향 |
| `… --test terminal-host mouse_surface -- --nocapture` | 1 PASS, compile 9.53초 / suite 0.35초 | 기존 binary 0xff 단계 뒤 실제 raw hook→Views.show→writer→child PTY의 SGR middle modifier press/release·wheel, logic replay·두 UI pass, exit 0·join·TaskSupervisor 0 |
| `… --lib mouse_queue -- --nocapture` | release RED 수정 후 1 PASS, compile 1.92초 / suite 0.01초 | 64 packet 상한·release 예약·실제 writer receipt·연속 frame 2+62 처리·task 회수 |
| `cargo clippy … --lib --test terminal-host --bin native-terminal-queue-fixture -- -D warnings` | exit 0, 2.92초 | 최종 authored app/host/fixture, eframe 소비자 compile |

고유 신규 unit은 adapter와 queue 2건입니다. host는 기존 mouse_writer 검사를 실제 surface 경로로 확장해 mouse_surface로 이름을 바꿨습니다. fixture의 SGR literal은 primary+modifier 28에서 middle+modifier 29로 바뀌었으며 좌표 3;2와 wheel 64를 그대로 확인합니다. 이전 기본 adapter의 다른 입력 버전 PASS를 별도 신규 검사로 합산하지 않습니다. 마지막 queue 수정은 정상 소량 입력 경로를 유지하며 이미 통과한 adapter/PTY 결과를 재사용하고 새 포화 분기만 검사했습니다. 기존 Wry 17 warnings는 authored strict 결과와 구분합니다.

## 후속: 같은 frame의 문자·마우스·휠 순서

actual host에 `a → press → b → release → wheel → c`를 넣어 exit 1≠0 RED를 확인했습니다. mouse/wheel 선처리 후 keyboard를 처리하던 원인을 고쳤습니다. 각 packet의 완료 frame·필터링 후 raw 위치를 저장하고 남은 egui event 앞까지만 pointer queue를 처리합니다. 원본에 없는 UI 주입 event는 native raw 뒤에 둡니다. keyboard 문자열의 별도 raw 복제나 새 dependency는 추가하지 않았습니다.

최종 mixed mouse host 1 PASS(compile 7.46초/suite 0.36초), 별도 alt wheel의 Text·IME commit·Tab interleave host 1 PASS(3.00초/0.34초), 변경된 normal wheel owner/lifetime/policy 3 PASS(3.48초/0.05초)입니다. actual child literal·exit 0·join·TaskSupervisor 0을 확인했습니다. normal wheel도 receipt 여유를 기다리는 packet을 다음 active frame으로 유지합니다. 최종 app/host/fixture strict exit 0(2.50초)·authored 4파일 exact fmt exit 0입니다. 버전이 바뀐 실제 입력 순서를 확인한 것이며 위 초기 PASS와 별도 고유 신규 테스트로 합산하지 않습니다. 재현 정본은 `docs/bug/2026-10-02-native-terminal-input-order.md`입니다.

## 다음 기존 N4-B 경계

- [x] 같은 frame의 일반 raw keyboard/IME·mouse/wheel interleave와 두 UI pass/logic replay에서 실제 PTY byte 순서를 확인했습니다.
- [ ] shared writer 자체가 포화되었을 때 typed retry·release 전송 수명과 승인 전 input epoch/agent activity 갱신입니다. 동일 값의 event 일부만 다른 widget이 소비한 모호한 raw 정렬·원본 순서를 바꾸는 UI 주입·focus 전환 중 입력은 전체 입력 gate에 남습니다.
- [ ] mode ABA·resize/alt/reset의 완전한 fraction/held 정리, focus loss 뒤 release·다중 창/mount/overlay, forced selection과 keyboard Select All/Copy·right-click/context입니다.
- [ ] 실제 OS/TUI/DPR·CJK IME/AX·aggregate/peak·full VT/keypad/Kitty·context/search/link/project Hub·213 TS view와 제품 cutover/TS 제거·N1~N8 전체입니다.

보호 실기 bundle·사용자 파일/clipboard·OS 입력기/VoiceOver·TS/root/MSRV는 유지했습니다. 전체 M8 완료 전 commit/push하지 않았습니다.
