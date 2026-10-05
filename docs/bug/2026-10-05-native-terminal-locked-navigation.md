# Native terminal 잠긴 탐색키 입력 유실

현재 상태: 입력 시작 owner가 실제 등록한 focus lock 안의 Tab/방향키와 AX Focus를 섞은 기본 경로를 수정했습니다. 모든 혼합 입력·M8 완료는 아닙니다.

## 대상

- `native/taide-native-app/vendor/egui-input/src/{context.rs,memory/mod.rs}`
- `native/taide-native-app/src/terminal_surface.rs`
- `native/taide-native-app/tests/terminal-host.rs`
- `native/taide-native-terminal/tests/fixtures/session.rs`

## 증상과 원인

동일 session의 실제 두 terminal view에 IME commit `con`, Tab, ArrowLeft, AX Focus, Text `tinue\n`을 넣었을 때 기대 wire를 받지 못했습니다. 신규 actual PTY 테스트는 compile6.38초/suite3.35초/exit101이며 reverse=false에서 `terminal input was not delivered`로 실패했습니다. 이 실패를 child exit1이나 실제 OS AX 실패라고 기록하지 않습니다.

local route는 잠긴 키도 미확정 focus 이동으로 취급했고, 전송 경계는 모든 탐색키를 일괄 barrier로 처리했습니다. 원본 `terminal-view.tsx`의 custom handler는 Shift+Enter만 별도 처리하며 기본 xterm 데이터는 onData로 전달합니다. 설치된 egui0.36.2의 EventFilter/Focus begin_pass 계약에서 lock은 Tab/방향키를 widget 내부 입력으로 유지합니다.

## 수정과 검증

Memory가 초기 실제 focus filter를 저장하고 Context가 초기 owner·직전 pass의 enabled/focusable/interactive target·pressed 탐색키를 확인합니다. 같은 판정을 window capture, local route, ordered stage에 연결했습니다. 새 target이나 실제 포커스 이동은 임의로 확정하지 않습니다.

실제 PTY 정/역 wire 신규1건 PASS(compile15.28초/suite0.40초), 등록/상한 predicate 신규1건 PASS(15.23초/0.02초), 선행 keymap scope 신규1건 PASS(4.94초/0.02초)입니다. 선행 scope fixture의 첫 실패는 generic click을 ordinary Button으로 간주하고 연속 Enter press를 repeat가 아니라고 기대한 오류였습니다. 실제 Button ID와 key release로 fixture만 수정했습니다. 최종 engine/app lib/tests strict22.55초 exit0입니다. exact 명령과 성공 재사용 범위는 `docs/quality-assurance/2026-10-05-m8-keymap-event-owner.md`에 기록했습니다.

pointer/Touch/wheel·새 AX owner의 잠금·실제 Tab focus 이동·current-pass 동적/disabled/modal/viewport 및 전체 source/GUI 검증은 남습니다. 보호 앱·OS 설정·제품TS·root/Tauri·의존성/manifest/lock/MSRV·Git은 바꾸지 않았습니다.
