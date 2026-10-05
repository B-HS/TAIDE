# M8 native terminal 시작 입력·복구·sync 화면

## 대상·상태

`native/taide-native-app/src/{terminal_surface,terminal_tabs,host,application}.rs`, `tests/terminal-host.rs`입니다. 원본 `src/widgets/terminal-pane/{terminal-session.tsx,pending-terminal-input.ts}`의 최초 spawn 입력·실패/종료 화면·재시작을 연결했습니다. 같은 parser와 writer를 유지하며 별도 화면 cache/parser를 추가하지 않았습니다. 상위 N4-B/N4/M8는 미완료이며 전체 완료 뒤만 commit/push합니다.

## sync 가정의 실제 정정

앞선 surface QA는 mutable Core grid가 sync 중 새 출력으로 바뀔 수 있다고 추정했습니다. 신규 재현 검사는 수정 전에도 통과했습니다. 실제 vte `Processor::advance`는 sync deadline이 있을 때 `advance_sync`로 bytes만 보관하고, `stop_sync_internal`에서 buffer를 Handler에 반영합니다. old→BSU/new→새 UI frame→flush/new 연속 검사에서 중간 frame은 old를 유지합니다. 따라서 별도 표시 cache를 만들 근거가 없으며 이전 추정을 정정합니다. 버그 수정의 RED→PASS로 주장하지 않습니다. buffer 상한·ESU·deadline/종료 flush의 기존 성공은 해당 QA로 재사용합니다. 실제 OS 픽셀/GPU·resize/IME 전체 동등성은 이 headless 검사의 범위가 아닙니다.

## 입력·복구 소유

- spawn이 진행 중이고 focused view일 때만 pending 입력을 보관합니다. settled/starting 입력이 같은 event 분류와 native encoder를 사용합니다. 초기 mode의 encoded UTF-8를 원본 JS와 같은 UTF-16 4,096단위 tail로 보관하며 초과하면 앞을 버립니다. 잘린 surrogate는 flush에서 UTF-8 replacement로 표현합니다. preedit는 전송하지 않으며 paste CRLF·Shift Enter도 같은 encoder를 사용합니다. 기존 64KiB 단일 입력 admission은 유지합니다.
- typed attach reply가 수락되면 해당 탭의 view pending을 즉시 기존 Session writer에 제출합니다. 다시 visible frame이 올 때까지 기다리지 않으며 pending을 먼저 비워 중복 제출하지 않습니다. attach 실패와 host submission 실패에서 pending/preedit를 지웁니다. closed tab의 late reply는 기존 pending guard에서 무시합니다. 입력 receipt의 완료/오류 소유는 해당 view와 root supervisor 경계에 남습니다.
- 원본 locale의 `terminal.restart`, `terminal.processExited`와 실제 exit code를 사용해 semantic button을 표시합니다. 최초 attach 실패·Core 실패·종료 상태에서 복구 요청이 가능합니다. restore/live tab의 정상 paint는 유지하며 종료/실패 화면은 original TerminalSession처럼 grid 화면을 대체합니다. 색상/폰트 등 전체 status 시각 동등성은 아직 gate입니다.
- `RestartTerminal` typed host command는 같은 Tabs의 serial start lock 안에서 대상 탭·project/session 소유를 확인합니다. 정상 Running 세션은 거절하며, 종료/실패한 기존 Hub owner를 close/join한 뒤 같은 attach_inner의 spawn/owned settlement를 사용합니다. 실패 owner의 close 오류는 실제 실패 latch와 is_finished가 함께 확인될 때만 복구 대상으로 취급합니다. 같은 tab의 이전 ID를 가진 layout을 settlement에서 재확인하며 새 session ID를 저장합니다. restart click은 같은 tab의 이전 view 입력/선택을 초기화하고, 새 spawn 동안 새 입력은 pending에 보관합니다.

## 실제 검증

Cargo는 기존 `CARGO_HOME`, locked/offline·공유 target으로 실행했습니다. 모두 합성 fixture/상태·headless egui이며 사용자 설정/clipboard·실기 앱·보호 bundle을 조작하지 않았습니다.

1. [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib sync_중_새_ui_frame -- --nocapture`: 1 PASS(compile 2.64초/suite 0.01초). 기존 production paint를 바꾸지 않은 상태에서 old/new sync 화면 보존·flush를 확인했습니다.
2. [x] `cargo test … --lib pending_input -- --nocapture`: 1 PASS(compile 2.92초/suite 0.00초). 4,096 UTF-16 tail·잘린 surrogate replacement, preedit와 조합 중 Enter 미전송·commit/paste/Shift Enter 순서를 확인했습니다.
3. [x] 변경 영향을 덮는 기존 headless 실제 PTY 검사를 한 연속 흐름으로 확장했습니다. `cargo test … --test terminal-host headless_surface -- --nocapture`: 1 PASS(compile 9.28초/suite 0.40초). 잘못된 합성 shell 설정의 attach 실패·pending 폐기, 실제 실패 화면 Restart click의 AttachTerminal 생성·정상 retry, spawn 중 con 입력의 reply 시점 제출→확정 tinue+Enter→정확한 child continue/Exited(0), locale 종료 표시·RestartTerminal click 생성, 실제 Tabs restart의 새 ID/이전 Arc owner 및 Hub 회수, 정상 Running restart 거절·새 PTY close/join/tracked 0을 확인했습니다. 이전 0.37초 검사는 이 입력/복구 구현 변경의 영향을 받으므로 확장된 검사만 1회 실행했습니다. restart command의 실제 HostBridge 왕복은 이번 검사에서 직접 실행하지 않았으며 실제 Tabs 호출과 host compile 근거를 구분합니다.
4. [x] 최종 `cargo clippy … --lib --test terminal-host --test terminal-dispatch -- -D warnings`: exit 0(1.89초). exact authored rustfmt·git diff --check exit 0입니다. production check exit 0(1.71초)와 중간 strict exit 0(1.73초)이며 dependency Wry의 기존 17 warnings는 유지됩니다. 기존 palette/query/borrowed paint/live/Hub/queue/writer runtime은 반복하지 않았습니다.

## 남은 구현

후속 기본 font/cursor 설정은 `2026-10-02-m8-native-terminal-settings.md`에 구현·신규 증거를 기록했습니다. 아래 목록은 이 시작/복구 checkpoint 당시의 잔여이며 전체 font fidelity·worker/OS/aggregate 및 나머지 상위 gate는 계속 미완료입니다.

설정 font family/default cursor style/blink·색/장식·선택/wrap/trim/copy와 search/link/context menu, hidden view focus/여러 창의 controlling view·마우스/keypad/Kitty/IME 주변 문자 삭제·AX, project close Hub 정리와 agent/remote/IDE/CLI·aggregate memory/CPU/RSS·실제 pixels/GPU/OS IME/배포/제품 채택·TS 제거가 남습니다. 시작/재시작의 취소·중복 external request·project/tab 소유 변경 전체 경계는 기본 연속 검사로 완료 주장하지 않습니다. per-view UTF-16 길이와 writer quota는 process 전체 메모리 상한이 아닙니다. 신규 package/lock/MSRV·기존 Tauri 경로는 유지합니다.
