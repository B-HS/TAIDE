# PTY callback join CI 시간 제한 부채

## 재현 조건과 관찰값

`main` CI run `36549889549`의 macOS Rust job에서 `crates/taide-infra/src/pty.rs`의 `부분_시작_오류는_child_회수_뒤에도_held_output_callback의_실제_join을_기다린다`가 마지막 `tokio::time::timeout`의 3초 제한을 넘겨 `Elapsed(())`로 실패했습니다. 같은 Rust 파일·의존성을 사용한 직전 CI run `36549227506`과 `v0.3.0` Release run `36550614766`의 Rust 테스트는 성공했습니다. 제품 코드의 변경 없이 관찰된 간헐 실패이며, 정확한 스케줄 지연 원인은 확정하지 않았습니다.

## 생략 이유와 위험

사용자가 동일 검사의 반복 실행을 피하고 한 번 통과한 결과를 재사용하도록 지시했습니다. 이번 릴리스에서 시간 제한만 늘려 실패를 숨기거나 전체 Rust suite를 별도로 다시 돌리지 않았습니다. 고부하 runner에서 다시 실패해 릴리스 작업을 중단시킬 수 있는 위험은 남습니다. 제품의 PTY 자원 join이 실제로 지연되는 경우와 테스트 스케줄링 지연은 아직 구분되지 않았습니다.

## 실행 시점

M8의 PTY 작업을 시작하기 전에 이 테스트의 callback 해제부터 worker join까지 단계별 완료 신호를 분리해 관찰하고, 외부 부하에 영향을 받지 않는 결정적 동기화로 바꿉니다. 변경된 테스트와 PTY 자원 회수 경계만 한 번 검증합니다.

## M8 source 확인 (2026-09-30)

`PtySpawnOwner::drop`은 pause·flush signal 중지, writer/master 해제, child 회수 뒤 시작된 flusher/reader를 순서대로 join합니다. 실패한 테스트는 callback 시작과 child 회수를 확인한 뒤 callback release channel을 drop하고 전체 blocking worker 완료만 3초로 기다립니다. callback 반환·개별 flusher/reader 완료 신호는 없어 실패 로그만으로 어느 join이 지연됐는지 구분할 수 없습니다.

실제 kernel read/EOF 지연 또는 OS·blocking worker 스케줄 지연은 아직 재현·확정하지 않았습니다. 해당 source 확인만으로 제품 종료 결함이나 단순 테스트 부하라고 결론 내리지 않습니다. 다음 검증에서는 종료 신호를 분리하고, callback join 계약의 결정적 fixture와 실제 PTY 종료 검사를 구분합니다. 이번 단계에서 PTY 코드·timeout 값은 변경하지 않았습니다.

## 종료 경계 보강 결과

테스트 전용 `WaitRecordingChild`와 worker factory에 child 회수·callback 반환·개별 worker 완료 신호를 연결했습니다. 60ms timeout으로 pending을 추정하던 부분은 callback return channel이 아직 비어 있고 blocking worker가 끝나지 않았다는 직접 관찰로 교체했습니다. child 회수 대기의 busy `yield_now` loop도 oneshot signal로 바꿨습니다.

callback release 후 callback 반환·reader/flusher 완료·최종 join은 하나의 동일한 절대 deadline을 사용합니다. 전체 제한은 기존 3초 그대로이며 단계별로 3초를 새로 부여하지 않습니다. 단계별 오류 메시지로 다음 CI 실패 위치를 구분할 수 있습니다. 실제 OS PTY reader와 `/bin/sh` child는 유지해 kernel 종료 경계를 mock으로 우회하지 않았습니다.

```sh
cargo test -p taide-infra --locked --offline --lib held_output_callback
cargo clippy -p taide-infra --locked --offline --all-targets -- -D warnings
rustfmt --edition 2021 --check crates/taide-infra/src/pty.rs
```

실제 대상 테스트 1 passed / 0 failed, 실행 0.01초이며 strict clippy·대상 fmt는 exit 0입니다. 첫 잘못된 selector 실행은 검증으로 계산하지 않으며, 해당 빌드의 테스트 fixture 필드 누락 E0063도 수정했습니다. 제품 PTY 구현은 바꾸지 않았고 성공 검사는 반복하지 않습니다.

이 결과는 종료 계약과 새 관찰 경계의 통과입니다. 과거 CI 지연의 정확한 원인은 여전히 미확정이며, 재현 없이 “해결”로 닫지 않습니다. 이후 제품 PTY 변경 시 이 새 경계의 실패 위치를 먼저 확인합니다.
