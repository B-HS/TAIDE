# AI 요청 owner Drop과 정상 root 종료 대기

## 대상 파일

- `crates/taide-runtime/src/ai_request_store.rs`, `exit_drain.rs`
- `src-tauri/src/lib.rs`, `src-tauri/tests/{ai_actions_runtime,task_supervisor}.rs`
- `docs/architecture.md`, `docs/PROCESS.md`, `docs/quality-assurance/2026-09-28-ai-request-owner-drain.md`

## 리포트

request future가 finish 전에 drop되면 registry에 남던 key를 재현하고 identity 기반 RAII로 정리합니다. registry 제거와 실제 owner 완료를 구분하며 정상 ExitDrain은 같은 AppServices의 AI owner가 회수되기 전에 ready/exit callback을 실행하지 않습니다.

## 상세

1. token이 store/key/identity를 소유하고 registry/live 목록은 identity Arc만 보유하므로 순환 소유가 없습니다. token Drop은 자기 identity인 key만 제거하고 sender 폐기 뒤 live-owner를 해제하며 모든 idle waiter를 깨웁니다. sender 폐기/취소 알림은 mutex 밖에서 실행합니다.
2. 취소와 수동 finish는 기존 key를 제거해 재시작을 허용하지만 이전 token의 실제 수명은 별도 목록에서 유지합니다. 늦은 finish/Drop이 새 token의 key를 지우지 않습니다.
3. shutdown은 같은 mutex에서 admission을 닫고 등록 sender를 꺼낸 뒤 취소를 보냅니다. 대기는 live owner가 비었는지 확인하기 전에 Notified를 만들어 알림 누락을 피합니다. 대기 future를 drop해도 목록/종료 상태는 유지돼 다시 기다릴 수 있습니다.
4. 기존 AI action/공개 command/provider/응답은 바꾸지 않았습니다. 정상 Tauri ExitRequested에서 등록된 AiRequestStore를 clone해 ExitDrain::new에 한 번 주입합니다. root shutdown과 coordinator shutdown 모두 idempotent이며 실제 owner 회수 뒤에만 기존 readiness를 엽니다.
5. 직접 Exit는 AI 취소/입장 차단만 요청하며 전체 owner drain 성공을 주장하지 않습니다. 로컬 future 정리는 외부 provider의 원격 처리 중단 보장이 아닙니다. 실제 앱·keyring·사용자 자격증명·provider 네트워크는 실행하지 않았습니다. 기존 coordinator 검사는 자기 전용 worker/child만 사용합니다.

## 검증 기록

`cargo test -p taide-runtime ai_request_store::tests::request_future_drop은_finish_없이도_같은_key의_재시작을_허용한다`가 registry 잔류 assertion으로 exit 101이었습니다. 초안의 ExitDrain struct update는 Drop 타입 필드 이동 E0509로 컴파일 실패했고, 명시 초기화로 해결했습니다. 검사 억제나 우회는 없습니다.

| 명령                                                                                                                                                                     | 실제 결과                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------- |
| `cargo test -p taide-runtime --lib ai_request_store::tests`                                                                                                              | 최종 12건, exit 0                                             |
| `cargo test -p taide-runtime --lib exit_drain::tests`                                                                                                                    | 최종 7건, exit 0                                              |
| `cargo test -p taide --test ai_actions_runtime --test ai_request_runtime_boundary --test app_services_runtime --test task_supervisor --test rust_native_phase0_contract` | action 9·동일 타입 1·AppServices 2·감독/배선 22·IPC 7, exit 0 |
| `cargo test -p taide --test ai_actions_runtime`                                                                                                                          | 최종 Drop 순서 보완 뒤 9건, exit 0                            |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                                             | 최종 exit 0                                                   |
| `cargo clippy -p taide --lib --test ai_actions_runtime --test ai_request_runtime_boundary --test app_services_runtime --test task_supervisor -- -D warnings`             | 최종 exit 0                                                   |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                                        | exit 0, 뒤의 내부 Drop 순서/테스트 추가는 문서 API 불변       |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                                         | exit 0                                                        |

초기 store 11/root 7 성공 뒤 sender 폐기보다 먼저 idle을 보고하지 않도록 Drop 순서를 보완하고 관련 store/root를 다시 실행했습니다. 최종 store 12에는 poll된 task의 abort 완료 검사를 추가했습니다. 최초 필터 명령의 integration 0건은 합격 수에 합산하지 않습니다. 전체 workspace·실제 provider/앱 종료 gate를 대신한 결과가 아닙니다.

서로 다른 검사 60건(store 12·root 7·action 9·동일 타입 1·AppServices 2·감독/배선 22·IPC 7)이 통과했습니다. 같은 검사의 재실행과 integration 0건은 중복 합산하지 않습니다.

공개 IPC 생성 입력·manifest·dependency/lock은 불변입니다. bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`를 확인했습니다. 같은 Specta 생성 성공과 Tauri 0개 graph를 재사용합니다. 정적 entry 배치 F149/S19/A13/P25를 바꾸는 단위는 아닙니다.

## 공식 근거와 남은 경계

[Rust Drop](https://doc.rust-lang.org/std/ops/trait.Drop.html)의 scope/owned field destructor 계약을 확인했습니다. Tokio 1.53.1 [Notify](https://docs.rs/tokio/1.53.1/tokio/sync/struct.Notify.html)의 웹 조회가 실패해 설치된 공식 `src/sync/notify.rs`의 notified/notify_waiters 문서를 직접 확인했습니다. notify_waiters 알림은 Notified 생성 이후라면 첫 poll 전에도 전달된다는 계약을 사용합니다.

정상 AI token owner 회수는 보완됐으나 나머지 application/비IPC callback·nested blocking/임시 stage·직접 Exit/OS 오류·M6 전체·M7 실기·M8/Phase 0 native UI gate는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
