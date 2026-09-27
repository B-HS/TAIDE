# Git worker·guard의 취소 및 정상 root 소유권

## 대상 파일

- `crates/taide-runtime/src/git_actions.rs`, `src/task_supervisor.rs`, `src/lib.rs`
- `src-tauri/src/domain/git/commands.rs`, `src/remote_gateway.rs`, `tests/git_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`, `docs/bug/2026-09-28-git-worker-guard-cancellation.md`

## 리포트

Git 41개 action의 blocking worker와 guard를 공유 owner로 감독했습니다. 요청이 취소돼도 이미 시작한 worker의 guard는 실제 완료까지 유지하며, 정상 root는 worker 이후 cache/event 처리의 마지막 owner도 기다립니다.

## 상세

1. 기존 action은 caller의 borrowed guard와 raw blocking JoinHandle을 분리해 보유했습니다. 메모리 차단 worker의 동일 primitive 패턴에서 caller abort 직후 mutation guard 재취득이 성공해, guard 유지 기대와 다른 RED(exit 101)를 확인했습니다. 실제 저장소 Git/훅을 차단한 재현은 아닙니다.
2. async에서 동일 mutex의 owned guard를 취득하고 caller와 worker가 하나의 `GitOperation`을 공유합니다. 마지막 owner의 guard가 먼저 Drop되고 공유 `TaskOperationLease`가 반납됩니다. 감독자는 operation ID만 보관하고 lease는 감독자를 강하게 소유하므로 순환 소유가 없습니다.
3. `stop_all`은 admission을 닫고 등록 task의 취소를 요청하지만 operation을 완료로 삭제하지 않습니다. `shutdown`은 실제 task 완료와 마지막 operation 반납을 함께 기다립니다. 이미 시작한 blocking 작업은 abort 요청만으로 완료 처리하지 않습니다.
4. global mutation guard 23개, push/fetch의 repo guard 2개, guard 없는 조회 16개의 기존 정책을 유지했습니다. push/fetch는 global guard를 새로 취득하지 않고 다른 repo·일반 mutation과 기존처럼 독립적입니다. pull은 기존 global guard만 사용합니다.
5. 41개 body의 ownership 교체·공백·formatter closure 차이만 제거해 비교한 결과, gate/cache/service 인수/반환 오류/이벤트 조건·순서는 모두 동일했습니다. status의 perf→구독→root→fresh cache와 diff의 root→overlay callback을 보존했습니다. Tauri는 `GitActionContext`로 기존 등록 감독자를 주입하고 원격도 같은 adapter를 사용합니다. 내부 Rust AppHandle 인수 15개를 추가했지만 실제 생성 wire payload는 불변입니다.

## 검증 기록

서로 다른 검사 54건이 통과했습니다. 초기 Git unit 검사에서 guard 반환값 미소비 경고 3개를 확인해 명시적 Drop으로 수정하고 최종 9건과 strict clippy를 확인했습니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide-runtime --lib git_actions::tests` | 최종 owner/cache 9건, exit 0 |
| `cargo test -p taide-runtime --lib task_supervisor::tests` | operation/기존 감독자 5건, exit 0 |
| `cargo test -p taide-runtime --lib exit_drain::tests::종료_드레인은_메인_응답을_막지_않고_worker와_lease_종료_뒤_한번만_종료를_요청한다 -- --exact` | 기존 root 회귀 1건, exit 0 |
| `cargo test -p taide --test git_actions_runtime --test task_supervisor --test taide_git_service_extraction` | action 6·기존 감독/배선 22·서비스 재수출 1건, exit 0 |
| `cargo test -p taide --lib domain::git::commands::tests` | 구독/기존 wire 2건, exit 0 |
| `cargo test -p taide --lib tests::typescript_바인딩을_생성한다 -- --exact` | 실제 생성 1건, exit 0 |
| `cargo test -p taide --test rust_native_phase0_contract` | IPC 7건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p taide --lib --test git_actions_runtime -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |

binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 실제 생성 뒤에도 불변입니다. 의존 파일 불변으로 runtime normal graph 543줄/Tauri 0개 성공을 재사용했습니다. 메모리 차단 worker·자기 UUID의 존재하지 않는 경로만 사용했으며 사용자 파일·Git/훅·자격증명·네트워크·사용자 프로세스·앱은 실행하지 않았습니다.

## 남은 경계

요청이 취소되면 기존 post-await cache/event는 실행하지 않는 정책과 동기 cold repo discover를 유지합니다. 이 검사는 실제 Git/훅/OS stall·서비스가 시작한 subprocess/pipe reader의 모든 실패·강제 bounded 종료·직접 Exit를 검증한 결과가 아닙니다. guard 입장 대기 중인 모든 IPC caller를 operation으로 등록한 것도 아닙니다. 정상 root는 등록된 worker/operation을 기다리며 완료되지 않는 외부 작업의 종료 시간을 보장하지 않습니다.

정적 entry 배치는 F177/S0/A13/P16으로 불변입니다. 나머지 application/nested worker/root·locale 보안 선택·M6/M7/M8는 미완료이며 M6 전체 완료 뒤 일반 push 조건을 유지합니다.
