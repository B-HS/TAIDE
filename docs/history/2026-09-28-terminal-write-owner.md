# 터미널 write action·blocking writer 소유권

## 대상 파일

- `crates/taide-runtime/src/terminal_actions.rs`
- `src-tauri/src/domain/terminal/commands.rs`, `src-tauri/tests/terminal_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

pty_write 정책을 runtime으로 이전하고 blocking writer 자체를 기존 TaskSupervisor에 등록했습니다. 요청 future Drop/abort로 waiter가 사라져도 이미 시작한 write/flush가 실제 완료될 때까지 정상 root의 ExitDrain이 기다립니다.

## 상세

1. 기존 input observer→같은 writer Arc 취득→store lock 밖 writer lock→write_all→flush 순서를 유지합니다. Tauri는 AppHandle에서 이미 등록된 TaskSupervisor를 가져오고 native observer만 callback으로 주입합니다. IPC 인수·반환·공개 Rustdoc와 생성 입력은 그대로입니다.
2. worker 자체가 writer/data를 소유하며 별도 oneshot으로 결과를 전달합니다. 요청 waiter 취소는 시작한 worker의 강제 중단이 아닙니다. 정상 root는 기존 terminal shutdown 뒤 tasks.shutdown을 기다리므로 실제 writer 완료 전 ready를 올리지 않습니다. 종료 후 신규 admission과 시작 전 취소는 Forbidden으로 반환합니다.
3. IO 오류는 원래 AppError의 변환대로 Io이며 worker panic은 Internal입니다. write 실패 뒤 flush를 실행하지 않고 flush 실패도 반환합니다. 첫 테스트의 잘못된 Internal 기대값이 실제 Io로 실패해 테스트만 수정했습니다. 제품 오류 변환은 변경하지 않았습니다.
4. owner 테스트는 자기 메모리 writer와 제어된 channel만 사용합니다. 실제 OS pipe가 영구적으로 막혔거나 child/자손이 종료를 무시할 때의 bounded 종료는 보장하지 않습니다. 사용자 파일/프로세스·profile·앱·네트워크는 실행하지 않았습니다. action 회귀의 기존 자기 전용 PTY는 유지합니다.

## 검증 기록

새 helper 부재 E0432(exit 101)로 RED를 확인했습니다. writer owner 3·action 10·감독/Tauri 배선 22·IPC 7로 서로 다른 검사 42건이 통과했습니다. owner 3건에는 IO/flush/panic 네 가지 분기, 요청 task abort 뒤 실제 root 대기, 종료 후 writer 무실행을 포함합니다. 잘못된 IO 기대값 수정 뒤 owner target과 runtime clippy만 재실행했으며 중복 검사를 합산하지 않습니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide-runtime --lib terminal_actions::write_tests` | 최종 3건, exit 0 |
| `cargo test -p taide --test terminal_actions_runtime --test task_supervisor --test rust_native_phase0_contract` | 10·22·7건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | 최종 exit 0 |
| `cargo clippy -p taide --lib --test terminal_actions_runtime -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |

Tauri 공개 command 12개 시그니처·Rustdoc 68줄을 대조했고 변경 body는 pty_write 하나뿐입니다. bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변이며 이전 Specta 생성 성공을 재사용합니다. 의존 입력 불변으로 runtime의 Tauri 0개 graph도 재사용합니다. 테스트 기대값만 바뀐 뒤 공개 문서 검증을 반복하지 않았습니다.

## 공식 근거와 남은 경계

설치된 Tokio 1.53.1의 공식 `task/blocking.rs`와 [spawn_blocking](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html)의 시작한 blocking task는 abort할 수 없다는 계약을 적용했습니다. 기존 TaskSupervisor의 실제 is_finished 추적과 root drain을 사용하며 별도 강제 취소나 종료 timeout을 도입하지 않았습니다.

F159/S13/A13/P21에서 write 1개를 반영하면 F160/S13/A13/P20입니다. 정적 entry 배치 수이며 전체 shutdown·실기 동등성 합격 수가 아닙니다. spawn 전체 action/output callback·다른 nested worker/application/root·직접 Exit/OS 오류·M6/M7/M8는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
