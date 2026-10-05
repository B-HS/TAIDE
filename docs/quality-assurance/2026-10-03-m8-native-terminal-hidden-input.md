# M8 native 숨김 터미널의 캡처 입력

## 대상·근거

`native/taide-native-app/src/terminal_surface.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`입니다. N4-B 입력 수명 중 Outbox 승인 전의 raw mouse release 유실을 수정했습니다.

설치된 `node_modules/@xterm/xterm/src/browser/{CoreBrowserTerminal.ts,services/MouseService.ts}`와 [xterm 6.0.0 CoreBrowserTerminal](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/src/browser/CoreBrowserTerminal.ts)의 `bindMouse`를 확인했습니다. 원본은 press 뒤 document 범위의 mouseup/drag를 연결하며 blur 처리와 mouseup listener 해제를 분리합니다. native에서도 단순 숨김만으로 버튼 캡처를 잊어서는 안 됩니다. 실제 OS의 숨긴 DOM 좌표와 native 마지막 유효 geometry의 픽셀 동등성까지 검증한 것은 아닙니다.

## 재현·수정

- [x] actual writer count=1에서 63개 문자와 press를 넣어 64개 Outbox를 채우고, view를 숨긴 다음 release를 보냈습니다. 기존 코드는 stale frame을 이유로 held/packet을 지우므로 actual child가 release를 받지 못해 3초 timeout으로 실패했습니다. 실패 뒤에도 child close/join·supervisor 회수를 수행했습니다.
- [x] raw mouse packet에 캡처 순번과 마지막 유효 rect·cell·좌표 변환을 보유합니다. 새 press/wheel hit는 직전 유효 target에서만 허용하지만, 이미 held인 버튼의 release/drag 캡처는 숨김 뒤에도 유지합니다. 인코딩 전 mode 확인·기존 64개 raw 상한·release 슬롯 예약은 유지합니다.
- [x] UI에서 처리한 frame 또는 완료한 frame의 대기 packet을 background에서 캡처 순번대로 Outbox에 넣습니다. 새 입력을 처리하는 화면도 먼저 완료된 대기를 처리하며, 현재 raw event 앞의 다른 view mouse packet도 먼저 처리합니다. 논리-only replay의 미래 keyboard event보다 mouse를 먼저 보내지 않도록 처리 frame/event 경계를 확인합니다.
- [x] 다른 view가 focus를 받았다고 아직 처리하지 않은 mouse를 즉시 보내지 않습니다. 해당 keyboard의 raw 순서 또는 frame 완료 시 처리하고, 남은 raw queue도 repaint를 요청합니다. 일반 wheel의 기존 scroll/key 경로는 별도로 유지합니다.
- [x] 비활성/종료 target은 기존 정책대로 미승인 캡처·held를 정리합니다. 앱 입력 취소는 raw mouse/wheel과 Outbox pending을 함께 회수합니다. 이미 승인된 writer 전송을 되돌린다고 주장하지 않습니다.

## 실제 검사

공통 Cargo 환경은 기존 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 직렬 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다.

| 대상·검사 | 실제 결과 |
| --- | --- |
| app `--test terminal-host hidden_mouse` 최초 재현 | exit 101, compile 3.65초 / suite 3.31초. `hidden release was not delivered` |
| 같은 검사, 숨김 처리 연결 후 | 1 PASS, compile 7.92초 / suite 1.51초. 65개 입력·원래 좌표·actual PTY 완료 확인 |
| 같은 검사, 다른 view의 입력 순서까지 확장 후 | 최종 1 PASS, compile 7.88초 / suite 1.66초. 첫 64개 포화→숨김 release에 이어 두 번째 press→숨김→다른 위치의 새 view에서 release/Text를 같은 frame에 처리했습니다. child가 정확한 바이트 순서·좌표를 확인했고 epoch+68·exit 0·join/task 0을 확인했습니다. |
| app `--lib mouse_` | 영향받은 adapter/queue 2 PASS, compile 3.40초 / suite 0.06초 |
| app `--test terminal-host mouse_surface` | 변경된 global dispatch의 기존 혼합 입력 1 PASS, compile 2.73초 / suite 0.49초 |
| app `--test terminal-host headless_wheel` | 변경된 wheel dispatch의 1 PASS, compile 2.80초 / suite 0.35초 |
| app `--lib mouse_disabled` | 신규 1 PASS, compile 1.94초 / suite 0.01초. 비활성 target 등록 시 raw packet·held·forced 정리 확인 |
| native terminal `clippy --bin native-session-fixture -- -D warnings` | exit 0, 0.23초 |
| app `clippy --lib --test terminal-host -- -D warnings` | 최종 exit 0, 1.23초 |

앞선 순서 배선 직후 app check exit 0(2.66초), 비활성 등록 guard 전 strict exit 0(2.42초)입니다. 기존 Wry 17 warnings와 authored 검사를 구분합니다. 동일 상태의 성공을 반복하지 않고, 다른 view 순서 배선·비활성 guard처럼 변경된 위험만 후속 검사했습니다.

authored 3파일 exact rustfmt와 추적 diff 검사는 exit 0입니다. 잘못된 apply_patch context 1회는 파일을 바꾸지 않았고 실제 문맥을 확인해 수정했습니다. 보호 bundle 재빌드·실기 앱 조작은 하지 않았습니다.

## 남은 경계

- [x] 후속 숨김/복원·WindowFocused 전환·preedit 취소의 기본 수명은 `2026-10-03-m8-native-terminal-focus-lifetime.md`에서 실제 PTY로 확인했습니다. focus 포화 재시도·모드 활성화 query·실제 OS 및 일반 captured-wheel의 hidden 수명은 여전히 별도 구현/검증 대상입니다. mouse release 검사만으로 focus 전체를 완료 처리하지 않습니다.
- [ ] 동일 event 일부 소비의 모호한 raw 정렬·UI 주입·mode ABA와 encoding 사이 변경·여러 view의 동시 다중 버튼·forced selection·rows/buffer epoch의 모든 전환은 남습니다.
- [ ] 실제 OS/보조 창·닫힌 viewport·DPR/transform·원본 DOM 숨김 좌표와 픽셀 동등성, 모든 context/search/link/project Hub·전체 VT/IME/AX·aggregate/RSS/GUI/성능은 미완료입니다.

후속 일반 captured-wheel의 숨김/포화 대기 보존과 mouse/wheel 공통 순번 drain은 `2026-10-03-m8-native-terminal-hidden-wheel.md`에서 확인했습니다. 현재 함수명은 `flush_captured_pointer`이며, 위 mouse 검사 때의 구현과 후속 확장을 구분합니다. 모든 raw/query/focus의 전체 순서·buffer/mode ABA·OS/aggregate 경계는 여전히 미완료입니다.

보호 실기 bundle·기존 앱·사용자 데이터/clipboard·OS 설정·TS/root/MSRV를 변경하지 않았습니다. M8 N1~N8 전체는 0/8이며 전체 완료 후 commit·push합니다.
