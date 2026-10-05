# Native terminal 메뉴의 Shift+F10 가로채기·우클릭 prefix 손실

현재 상태: 원본과 다른 Shift+F10 메뉴 가로채기를 제거했고, 기본 우클릭 전 문자와 이후 차단을 실제 PTY에서 확인했습니다. contextmenu의 전체 OS 시점·focus report/return·다중 닫힘·전체 M8 완료는 아닙니다.

## 대상과 원인

`native/taide-native-app/src/terminal_surface.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`입니다.

원본 `TerminalView`는 screenReaderMode를 지정하지 않아 xterm6.0.0 기본 false를 사용합니다. custom key handler는 Shift+Enter만 처리합니다. xterm `Keyboard.ts`의 F10/modifier 계산은 Shift+F10을 `ESC[21;2~`로 생성하고 `_keyDown`은 데이터를 전송한 뒤 기본 동작을 취소합니다. Radix trigger는 contextmenu를 처리하며 별도 Shift+F10 가로채기가 없습니다. native가 정확한 non-repeat 키를 메뉴 열기로 처리하고 keyboard anchor를 저장한 기존 구현은 이 원본 계약과 달랐습니다. 기존 QA의 해당 성공은 source parity 근거에서 철회하며 당시 실행 이력은 보존합니다.

실제 valid layout/owned PTY의 `con → Shift+F10 → tinue\n`은 메뉴가 열리고 wire가 완료되지 않는 RED였습니다(compile4.92초/suite3.35초/exit101). 가로채기와 새로 미사용이 된 anchor를 제거해 원본 바이트를 그대로 전송합니다. 신규1건은 compile13.71초/suite0.34초/exit0입니다.

실제 `continue\n → secondary down/up → blocked`도 메뉴 열림 상태를 전체 frame에 적용해 앞선 문자를 잃었습니다(compile4.98초/suite3.40초/exit101). 기본 단일 click의 raw release index를 global response rect로 식별하고, owned prefix를 먼저 처리한 뒤 현재 survivor로 메뉴를 표시합니다. menu copy snapshot도 prefix admission 뒤 Core/selection을 읽습니다. 이후 terminal 입력은 차단하고 이미 열린 메뉴의 기존 input mask는 유지합니다. normalized/raw cursor는 owner/메뉴 guard 전에 진행합니다. 새 검사1건은 compile8.66초/suite0.44초/exit0이며 exact `continue\n`, 메뉴 열림, user input epoch 정확히1회, owned child/session/task 정리를 확인합니다. read_exact가 suffix를 놓칠 수 있어 epoch로 suffix 미입장을 함께 검사합니다.

## 영향과 한계

기존 메뉴2건은 실제 secondary press/release로 열도록 정정했습니다. 첫 sizing pass의 invisible 결과를 표시 완료로 취급했던 action fixture 실패는 다음 visible pass에서 검사하도록 수정했습니다. action 의미·copy/paste/split/new/kill·source epoch 구현을 이 실패에 맞춰 바꾸지 않았습니다. 실제 명령/시간/strict/성공 재사용은 native-terminal-context-menu QA의2026-10-05 후속 절이 정본입니다.

당시 native popup은 egui secondary release를 열림 boundary로 사용했습니다. 아래 눌림 후속으로 기본 열림 시점을 수정했으며, macOS/browser의 press→contextmenu→global release, focus-report/메뉴 focus·return 및 복수 열림/닫힘을 native가 모두 재현했다고 주장하지 않습니다. pointer transform 전체·현재 disabled/modal/viewport·window capture의 메뉴 owner·Touch/long-press/IME·실제 GUI/source 전체 parity는 기존 M8 parent gate에 유지합니다. 이 범위를 기본 prefix 검사로 대신하지 않습니다.

제품TS·engine/vendor·root/Tauri·새 의존성·manifest/lock/MSRV·보호 앱·사용자 데이터/OS/clipboard/Keychain·Git은 변경하지 않았습니다. 전체 M8 완료 전에 commit/push하지 않습니다.

## 실제 원본 측정 후 눌림 경계 수정

actual TerminalView/TerminalContextMenu를 격리 macOS Chrome에서 plain/SGR 각1회 측정했습니다. 우클릭 down에서 이미 메뉴가 열리고 SGR press→terminal blur/Focus1004 loss→global release 순서입니다. Escape 후 실제 메뉴 detach/onRestoreFocus로 gain과 입력이 복귀합니다. source 측정/API·격리 조건·정확한 bytes는 context-menu QA 마지막 절이 정본입니다.

native prefix fixture에서 release를 제외하자 실제 child는 prefix를 읽었지만 메뉴가 열리지 않았습니다. raw events의 첫 secondary press·engine hit/focus 대상·global rect/좌표로 열고 실제 위치를 유지하도록 수정했습니다. 처리되지 않아야 할 뒤 문자는 admission epoch1로 계속 확인합니다. private API compile 오류를 공개 builder로 정정했고, input lock 내 Context 재조회로 생긴 10초 deadlock을 events 복사 후 lock 밖 조회로 수정했습니다. 강화1/영향2 PASS·app strict24.40초입니다. 실제 메뉴 owner/focus 보고/복귀 및 다른 플랫폼·전체 graph는 미완료입니다.
