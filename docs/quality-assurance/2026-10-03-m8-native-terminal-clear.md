# M8 native terminal 로컬 지우기

## 대상과 원본 계약

- 원본 `node_modules/@xterm/xterm/src/browser/CoreBrowserTerminal.ts`의 `clear()`는 history와 다른 화면 행을 버리고 현재 물리 cursor 행·column을 남깁니다. 첫 행이고 history가 없으면 no-op이며 PTY에 명령을 보내지 않습니다.
- `native/taide-native-terminal/vendor/alacritty-terminal/src/term/mod.rs`의 native-retained 전용 `native_clear_current_row`는 같은 grid의 current row를 첫 행에 보존하고 history/다른 행을 기본 Cell로 비웁니다. cursor template·column·pending wrap·parser/mode를 유지하고 buffer epoch를 갱신합니다. 행 끝 WRAPLINE은 제거합니다.
- `native/taide-native-terminal/src/{lib,session}.rs`와 `native/taide-native-app/src/terminal_host.rs`는 같은 Core/SharedTerminal/Session에서 실행하며 writer·user input epoch·output revision을 변경하지 않습니다. retained 검사 실패는 기존 Parser 실패/retire 경로로 전달합니다. Running 외에는 거절합니다.

## 검증

공통 명령 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 같은 성공 검사는 재실행하지 않았습니다.

- [x] terminal `cargo test --manifest-path native/taide-native-terminal/Cargo.toml --test clear`: 최초 compile은 TermDamage에 없는 helper를 호출해 실패했습니다. 기존 Term의 `mark_fully_damaged()`를 사용하도록 정정했습니다. 첫 runtime에서 Core 검사 1 PASS, Shared owner 검사 1 FAIL입니다. Core 검사는 main/alternate·history·NFD/wide·style·mode·partial UTF-8·no-op을 검증합니다.
- [x] 실패한 owner 검사만 `--test clear shared_owner -- --nocapture`로 재실행해 1 PASS(compile 1.17초, suite 0.00초)입니다. 최초 fixture가 synchronized bytes를 이미 표시된 셀로 오인했습니다. 실제 Processor는 sync 해제까지 bytes를 보류하므로 지우기 전후 보류 상태와 해제 후 `held` 표시를 검증하도록 기대값을 정정했습니다. production parser를 변경하지 않았습니다.
- [x] app `cargo test --manifest-path native/taide-native-app/Cargo.toml --test terminal-host clear_session -- --nocapture`: 최초 fixture는 초기 한 줄만 출력해 history가 없어서 history assertion이 실패했습니다(compile 11.00초, suite 0.35초). actual cursor가 다음 행에 있는 조건으로 정정한 관련 1 PASS(compile 2.10초, suite 0.15초)입니다. local clear→literal `continue\n` 수신→exit 0→dispatch/child/reader/flusher join→tracked task 0을 같은 실제 합성 PTY에서 확인했습니다. history 자체의 제거는 앞선 Core 검사에서 확인했습니다.
- [x] terminal `cargo clippy --manifest-path native/taide-native-terminal/Cargo.toml --lib --test clear -- -D warnings` exit 0(0.85초), app `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --lib --test terminal-host -- -D warnings` exit 0(2.13초)입니다. Wry의 inherited 17개 warning은 새 authored warning과 구분합니다.
- [x] authored lib/session/clear test/app host/host test의 `rustfmt --edition 2024 --check --config skip_children=true` exit 0입니다. vendor 전체 재포맷은 하지 않았습니다.

## 남은 경계

- [ ] 이 동작의 컨텍스트 메뉴·focus 복구·다른 view 재조정 연결은 다음 작업입니다. 이 helper 성공만으로 N4-C/M8 완료라고 하지 않습니다.
- [ ] OSC133 command metadata/marker·OSC8·full-wrap/pending-wrap 전체 조합·여러 viewport·동시 resize/출력의 전체 matrix는 실제 연결 후 검사합니다. 현재 검사만으로 전체 VT/RSS/aggregate 보장을 주장하지 않습니다.
- [ ] native feature가 꺼진 upstream compile과 실제 OS 메뉴·IME·VoiceOver는 실행하지 않았습니다. feature-off handler 변경은 없으며 전체 upstream corpus도 실행하지 않았습니다.

보호 실기 bundle·기존 TypeScript/root/MSRV·사용자 데이터·시스템 clipboard/입력기를 변경하지 않았습니다. M8 N1~N8는 0/8이며 전체 완료 뒤에만 commit/push합니다.
