# 공개 agent hook action의 정상 종료 소유권

## 대상 파일

- `crates/taide-runtime/src/agent_hook_actions.rs`, `tests/agent_hook_action_owner.rs`
- `src-tauri/src/domain/agent/commands.rs`, `src-tauri/src/remote_gateway.rs`, `src-tauri/tests/agent_hook_actions_runtime.rs`
- `docs/PROCESS.md`, `docs/architecture.md`

## 리포트

공개 status/install/uninstall 3개 action을 동일한 등록 TaskSupervisor에 입장시키고 첫 정책 gate부터 최종 status 반환 또는 오류까지 operation을 보유합니다. 특히 install의 emitter/server await가 끝나고 이어지는 파일 쓰기도 정상 root 종료 전에 끝납니다. Tauri 명령과 원격 gateway 모두 같은 감독자를 전달하지만 공개 frontend/원격 인자·응답과 기존 file/CLI/server 정책은 유지합니다. 실제 server bind/accept/store의 독립 admission이나 HTTP transport 완료를 이 owner로 주장하지 않습니다.

## 상세

1. 세 runtime action의 기존 본문은 entry owner 이외의 정책을 바꾸지 않았습니다. unknown agent→settings/project/home/shape·emitter/CLI/server·파일 read/write와 기존 오류·멱등·권한 보존 순서는 그대로입니다. 닫힌 감독자에는 새 Internal 오류를 반환하며 파일 및 lazy port를 건드리지 않습니다. 입장한 action은 shutdown 뒤 재검사 없이 원래 정책대로 완료/취소됩니다.
2. 프로젝트 install의 emitter await에서 caller를 취소하면 아직 읽거나 쓰지 않은 JSON은 그대로이고 operation은 실제 caller 종료 후 반납됩니다. synthetic HTTP server await 중에는 정상 ExitDrain ready가 false이고, fake 답을 받은 action이 기존 command hook JSON을 쓴 뒤 최종 status를 반환해야 ready가 true가 됩니다. callback/파일 I/O가 영구 정지하면 정상 종료도 기다립니다. 실제 listener나 CLI는 시작하지 않았습니다.
3. Native status/uninstall은 Tauri-managed `State<TaskSupervisor>`를 추가 주입하며 install은 이미 잡은 같은 State를 넘깁니다. 원격 gateway의 status/uninstall 직접 호출도 `app.state()`를 한 번 더 전달합니다. 요청에서 파싱하는 것은 계속 projectId·agentName뿐이고 User-scope install의 기존 remote denial은 건드리지 않았습니다. [Tauri의 managed state 설명](https://v2.tauri.app/develop/calling-rust/#accessing-managed-state)은 `State<T>`가 등록된 값에서 주입됨을 명시합니다. 실제 setup은 `services.tasks.clone()`을 관리 상태에 등록합니다.
4. 읽기 전용 대조에서 runtime 기존 공개 본문 3개는 입장 추가 외 정규화 뒤 동일하고 나머지 runtime source는 byte 동일합니다. Native commands는 이 세 함수 밖에서 byte 동일하며 remote gateway diff는 status/uninstall에 State를 전달하는 두 호출로 한정됩니다. 공개 TS bindings·IPC manifest 파일의 해시는 이전과 동일합니다. 이 결과는 generated bindings의 재생성/실기 검사를 대신하지 않습니다.

## 검증 기록

새 owner 인수 부재 E0061 RED(exit 101)를 확인하고 새 검사 4건을 작성했습니다. 최초 3건이 통과하고 source 검사는 Rustfmt 줄바꿈을 놓쳐 실패했습니다. 실제 `.begin_operation` 호출을 확인하도록 source 검사를 고친 뒤 해당 1건만 재실행해 통과했습니다. 기존 hook action 10건과 Phase 0 계약 7건도 통과했습니다. 서로 다른 검사는 21건이며 성공한 동일 입력을 반복하지 않았습니다. Native 컴파일에서 원격 gateway의 직접 호출에 새 State 인수가 빠진 E0277/E0061을 확인하고 해당 두 호출만 수정한 뒤 기존 검사에서 통과했습니다.

| 명령                                                                                                                                          | 실제 결과                                       |
| --------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| `cargo test -p taide-runtime --test agent_hook_action_owner`                                                                                  | 최초 E0061 RED; 구현 뒤 3/4, source 줄바꿈 실패 |
| `cargo test -p taide-runtime --test agent_hook_action_owner native는_공개_입력_반환을_유지하고_같은_감독자를_세_action에_주입한다 -- --exact` | 수정한 source 1건, exit 0                       |
| `cargo test -p taide --test agent_hook_actions_runtime`                                                                                       | 기존 파일/port/명령 배선 10건, exit 0           |
| `cargo test -p taide --test rust_native_phase0_contract`                                                                                      | command/event/raw/remote/해시 계약 7건, exit 0  |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                  | exit 0                                          |
| `cargo clippy -p taide --lib --test agent_hook_actions_runtime -- -D warnings`                                                                | exit 0                                          |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                             | exit 0                                          |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                              | exit 0                                          |

자기 UUID AppPaths·project/home만 test가 읽고 쓰며 fixture Drop이 자기 경로만 지웁니다. synthetic server/CLI 문자열·oneshot과 실제 ExitDrain/TaskSupervisor를 사용합니다. 실제 user home, CLI 실행, PID, listener, AppHandle, 앱, keyring은 사용하지 않습니다. 원격 gateway는 컴파일/정적 diff로만 확인했으며 live 인증/원격 세션 실기는 미검증입니다.

실제 binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`와 manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. 모델/Cargo/등록과 공개 입력/반환 DTO는 변경하지 않았습니다. Native 내부 State 인수를 추가했으므로 generated bindings의 실제 재생성 여부는 별도 gate로 남깁니다. 영향 없는 기존 hook reconcile/payload/poll/probe/worker 및 normal graph 성공은 재사용하고 이번 건수에 합산하지 않습니다. 신규 history만 기존 Prettier로 검사하고 PROCESS/architecture의 무관한 baseline 포맷은 다시 쓰지 않습니다.

## 남은 경계

실제 hook server의 캐시 조회→bind→accept task 등록→shutdown 재검사→store.set_server, stop 시 store 정리/핸들 abort, 실제 연결 인증/timeout/요청 파싱은 Tauri에 남아 있습니다. caller와 독립적으로 직접 server를 시작할 때의 입장/소유와 shutdown 사이 cache/store 경쟁은 이번 단위로 해결되지 않았습니다. 동기 파일 I/O/CLI child/OS stall, 실제 GUI/Windows/원격 세션, 직접 Exit/강제 bounded 종료도 미완료입니다. F193/S0/A13/P0·전체 M6/M7/M8와 전체 M6 완료 전 push/UI 금지를 유지합니다.
