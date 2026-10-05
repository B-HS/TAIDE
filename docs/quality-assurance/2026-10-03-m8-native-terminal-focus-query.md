# M8 native terminal 현재 focus query

## 대상·계약

`native/taide-native-terminal/src/{lib,session}.rs`, `tests/{session.rs,fixtures/session.rs}`, native Alacritty의 `src/term/mod.rs`·`UPSTREAM.md`입니다. 기존 focus lifecycle 다음으로 DECSET 1004가 현재 상태를 보고하는 경계를 연결했습니다.

설치된 `node_modules/@xterm/xterm/src/common/InputHandler.ts`의 1004 handler는 이미 켜져 있어도 `onRequestSendFocus`를 발생시킵니다. [xterm 6.0.0 CoreBrowserTerminal](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/src/browser/CoreBrowserTerminal.ts)의 `_reportFocus`는 현재 focus 여부에 따라 I/O를 보냅니다. 기존 native는 mode bit만 켜므로 요청 직후 보고가 없었습니다.

## 구현·검증

- [x] 새 `focus_query` 검사의 최초 활성화가 `[]`와 기대 `[ESC[O]` 불일치로 실패했습니다. native session test compile 0.73초 / suite 0.00초, exit 101입니다.
- [x] SharedTerminal은 `NativeInput::Focus`를 받으면 같은 core lock 안에서 실제 focus 관찰 상태를 갱신합니다. 이 상태는 PTY 전송 승인 여부와 다르므로 보고가 pending이거나 mode가 꺼져 있어도 현재 focus 관찰을 보존합니다. user input epoch는 올리지 않습니다.
- [x] native-retained feature의 DECSET 1004는 기존 Term의 `is_focused`에서 보고 문자열을 즉시 생성합니다. 기존 bounded PtyWrite effect에 넣으므로 별도 무상한 queue나 UI callback, 새로운 event/layout/dependency를 추가하지 않았습니다. feature 밖의 원본 동작은 유지합니다.
- [x] `cargo test --manifest-path native/taide-native-terminal/Cargo.toml … --test session focus_query`: 수정 뒤 1 PASS, compile 1.46초 / suite 0.01초입니다. 최초 Out, focus 뒤 반복 In 2회, 비활성 동안 blur 관찰, 다시 활성화 Out, 전송 pending이어도 현재 focus query In, epoch 불변을 확인했습니다. pending 보고 자체의 재시도를 검사한 것은 아닙니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml … --test terminal-host focus_surface`: 변경된 actual child fixture가 최초 Out을 포함한 Focus 11개+x를 정확히 수신했습니다. 1 PASS, compile 9.10초 / suite 0.34초, epoch+1·exit 0·close/join·task 0입니다. 숨김·logic-only blur·view 전환·headless auxiliary 제거도 같은 경로입니다.

공통 실행은 직렬 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 기존 startup/hidden mouse 성공은 해당 코드가 바뀌지 않아 재사용했습니다. 보호 bundle 빌드·실제 앱/OS 조작은 하지 않았습니다.

최종 native `clippy --lib --test session --bin native-session-fixture -- -D warnings` exit 0(1.62초), app `clippy --lib --test terminal-host -- -D warnings` exit 0(1.85초)입니다. 기존 Wry 17 warnings와 authored strict 성공을 구분합니다. 이번 query 및 직전 focus authored 7파일 exact rustfmt·추적 diff check exit 0이며, vendor 파일은 변경한 handler만 기존 형식으로 편집하고 전체 재포맷하지 않았습니다.

## 미완료 경계

- [x] 후속 UI Outbox 64개 포화 때 focus 보고 보존·재시도는 `2026-10-03-m8-native-terminal-focus-pressure.md`에서 구현·검증했습니다. query Dispatcher와의 shared writer 포화·전체 순서, focus 전용 byte 상한 초과 복구는 여전히 남습니다. query를 추가했다고 입력 전체 순서를 완료 처리하지 않습니다.
- [ ] 원래 focus 보고와 output query의 동시 순서, enable/disable 후 대기 보고, attach 직후 첫 UI 상태 갱신 시점, reset/다중 창/OS 전체 동등성은 별도 검증 대상입니다.
- [ ] feature 밖 fork compile·전체 VT/IME/AX·aggregate/RSS·213 TS view/full cutover/TS 제거와 M8 N1~N8은 미완료입니다. 기존 Term reset source가 `is_focused`를 재설정하지 않는 것은 확인했지만 실제 reset query 검사를 수행했다고 주장하지 않습니다.

앞선 기본 focus lifecycle 근거는 `2026-10-03-m8-native-terminal-focus-lifetime.md`입니다. M8 전체 완료 후 commit·push합니다.
