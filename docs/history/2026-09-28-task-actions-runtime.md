# task action과 scan worker의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/task_actions.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`
- `src-tauri/src/domain/task/commands.rs`, `src/remote_gateway.rs`
- `src-tauri/tests/task_actions_runtime.rs`, `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

프로젝트 task 탐지 action을 runtime으로 이전하고 실제 blocking scan worker를 기존 TaskSupervisor에 등록했습니다. 요청 waiter가 사라져도 정상 root는 이미 시작한 worker 완료를 기다립니다.

## 상세

1. project_root gate를 worker 입장보다 먼저 수행하고 기존 taide-task service의 결과·순서·cwd·command 문자열을 유지합니다. 탐지한 command를 실행하지 않습니다.
2. Tauri의 내부 Rust signature에 managed TaskSupervisor를 추가했고 원격 호출도 같은 등록 상태를 전달합니다. 기존 AppServices/Tauri 상태 등록은 변경하지 않았습니다. 실제 Specta 생성 결과는 이전 파일과 동일해 wire 인수·응답은 불변입니다.
3. panic은 기존 JoinError의 Internal 변환을 유지합니다. 종료 뒤 worker 입장과 취소된 worker는 Forbidden입니다. 이미 시작한 blocking 작업은 abort로 종료됐다고 간주하지 않으며 실제 완료까지 감독합니다.
4. 자기 UUID package.json/bun.lock/Makefile/Cargo.toml과 메모리 worker만 사용했습니다. 요청 abort 뒤 ExitDrain이 대기하고 release 뒤 완료되는 실제 경로를 검사했습니다. 사용자 파일·task/script/process·앱·네트워크는 실행하지 않았습니다.

## 검증 기록

새 모듈 부재 E0432(exit 101)로 RED를 확인했고 최종 서로 다른 검사 16건이 통과했습니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide-runtime --lib task_actions::tests` | owner 3건, exit 0 |
| `cargo test -p taide --test task_actions_runtime --test taide_task_extraction` | action 4·추출 1건, exit 0 |
| `cargo test -p taide --lib tests::typescript_바인딩을_생성한다` | 실제 생성 1건, exit 0 |
| `cargo test -p taide --test rust_native_phase0_contract` | IPC 7건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p taide --lib --test task_actions_runtime -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |
| `cargo tree -p taide-runtime --edges normal --prefix none` | 543줄, Tauri 패키지 0개 |

생성 bindings digest는 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest는 `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`로 불변입니다. 코드 변경 없는 성공 결과를 재사용했습니다.

## 남은 경계

task 1개의 S→F로 F170/S3/A13/P20입니다. 정적 entry 배치 수이며 자원 회수·보안·실기 전체 합격 수가 아닙니다. locale 보안 선택·LSP/project/agent/sync/terminal application·nested worker/root·직접 Exit/OS 오류·M6/M7/M8는 미완료입니다. 이미 시작한 scan의 강제 중단이나 bounded 종료는 검증하지 않았고 M6 전체 완료 뒤 일반 push 조건을 유지합니다.
