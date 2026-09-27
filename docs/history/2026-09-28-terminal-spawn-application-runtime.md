# 터미널 spawn application의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/terminal_actions.rs`
- `src-tauri/src/domain/terminal/commands.rs`, `tests/terminal_spawn_application_runtime.rs`, `tests/taide_terminal_store_extraction.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

남아 있던 터미널 spawn의 application 순서·입장/프로젝트 gate·metadata/삽입/이벤트를 runtime으로 이전했습니다. Toolkit env·UUID·raw channel 폐기·native session 생성은 명시적 port로 주입하며 같은 PTY worker 소유권을 사용합니다. 정상 root는 post-await publication caller의 완료도 기다립니다.

## 상세

1. env await→owned mutation guard→shutdown/project gate→inert initial channel 폐기→ID/output/metadata→같은 감독 spawn→삽입→TerminalSpawned 순서를 유지했습니다. 처음 받은 raw channel은 live subscriber로 등록하지 않으며 실제 출력은 기존 attach/replay 경로를 사용합니다.
2. `TerminalSpawnPorts`는 env future, initial sink 폐기, ID, native session factory 네 가지를 주입합니다. runtime에는 Tauri/UUID 의존을 추가하지 않았습니다. 기존 native output counter·scan dispatch·exit metadata/event는 같은 Tauri callback 조립에 남깁니다. scan 정책·UUID factory는 byte 동일합니다.
3. 실제 spawn은 기존 `run_terminal_spawn`·TerminalSpawnLease·owned guard/미반환 결과 RAII·강한 PTY 완료 소유자를 사용합니다. post-await 삽입과 이벤트 caller는 같은 TaskSupervisor의 operation으로 추적하고 caller의 guard 반납/operation Drop까지 정상 root가 기다립니다.
4. 실제 자기 `/bin/sh`와 ENV/BASH_ENV 빈 값·자기 UUID cwd를 사용해 삽입 뒤 guard 안에서 spawn event가 발행되는 것을 확인했습니다. publication을 메모리 gate로 차단한 동안 ExitDrain 준비가 열리지 않고, 반환 뒤 실제 child/worker 종료와 정상 root 완료를 확인했습니다. fixture 경로는 테스트가 정리했습니다. 사용자 profile/프로세스·시크릿·앱·네트워크는 실행/조회하지 않았습니다.
5. 공개 command 12개의 signature/Rustdoc는 byte 동일하고 같은 managed State·raw 등록/remote 인수·이벤트 payload를 유지합니다. native channel·observer·실제 env provider는 그대로이며 정적 command entry 하나를 P→F로 이전해 F178/S0/A13/P15입니다. 이는 전체 M6/동작 parity 완료 판정이 아닙니다.

## 검증 기록

새 action/port 부재 E0425/E0432 RED(exit 101)를 확인한 뒤 구현했습니다. fixture의 generic factory 필드에 필요한 Rust 경계 타입을 명시해 E0282를 해소했고, 기존 lib unit의 AppError import는 production에서 test module로 옮겨 E0433을 해소했습니다. 수정 후 관련 검사를 확인했습니다. 서로 다른 검사 18건이 통과했습니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide --test terminal_spawn_application_runtime --test taide_terminal_store_extraction` | action 6·기존 store/배선 3건, exit 0 |
| `cargo test -p taide --test terminal_spawn_application_runtime native_factory는_기존_출력_counter와_scan_exit_callback을_유지한다 -- --exact` | native callback source 1건, exit 0 |
| `cargo test -p taide-runtime --lib terminal_actions::tests` | 기존 spawn owner 6건, exit 0 |
| `cargo test -p taide --lib domain::terminal::commands::tests::출력_채널` | 실제 memory raw channel 2건, 최종 exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p taide --lib --test terminal_spawn_application_runtime --test taide_terminal_store_extraction -- -D warnings` | 최종 exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |

생성의 공개 signature/문서·등록 원천·wire 타입/함수 입력이 불변이므로 직전 fa2d4d9 단위의 실제 생성 1·IPC 7 성공을 재사용합니다. binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. 의존 파일도 불변이므로 runtime normal graph 543줄/Tauri 0개 성공을 재사용했습니다.

## 남은 경계

이 단위는 guard 이전의 env/입장 대기 전체를 operation으로 감독하거나 blocking read/callback·OS 실패·SIGHUP 무시/자손·직접 native Exit를 강제 bounded 종료하는 계약이 아닙니다. 기존 native callback의 실제 AppHandle 발행/사용자 프로필·GUI 실기는 실행하지 않았습니다. LSP spawn/stop/restart·project 4·agent 3·sync 5의 application과 나머지 nested worker/root·locale 보안 선택·M6/M7/M8는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
