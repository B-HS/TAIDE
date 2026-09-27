# Project build worker와 정상 root 소유권 보강

## 대상 파일

- `crates/taide-runtime/src/project_build.rs`, `src/lib.rs`, `tests/project_build.rs`
- `src-tauri/src/domain/project/commands.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

Capability attach와 watcher restore의 blocking build를 같은 등록 TaskSupervisor로 추적합니다. 기존 restore async waiter만 감독하던 경계에서 실제 nested worker가 정상 root의 완료 조건에 포함되지 않던 문제를 보강했습니다. ProjectBuild의 operation lease는 worker와 caller가 공유하며 결과 resource 회수·native guard/commit/event 스코프가 끝나기 전 정상 root가 완료되지 않습니다.

## 상세

1. 두 producer는 기존 순서의 build closure를 runtime run_project_build에 전달합니다. mutation guard 밖 build→guard 재취득→project/watchers 재검사→같은 등록/이벤트를 보존합니다. attach 오류는 기존 `project capability attach failed: <id>` Internal로 변환하고 watcher restore 오류는 기존 warning/continue입니다. 정상 경로의 capability 순서·watcher skip·file→git 등록·guard 해제 뒤 boot-gap correction 이벤트는 불변입니다.
2. runtime helper는 operation 입장→같은 감독자의 blocking worker 등록→실제 worker await→result channel 소비→ProjectBuild 반환입니다. worker가 operation clone을 보유하므로 요청/restore waiter 취소 뒤에도 시작한 worker와 보내지 못한 결과의 Drop이 추적됩니다. 이미 종료한 감독자는 work factory를 호출하지 않습니다.
3. ProjectBuild는 `value`를 lease보다 앞에 선언해 결과 Drop 완료 뒤 operation을 반납합니다. native는 value를 등록으로 옮기지만 남은 lease 필드가 scope 끝까지 살아 있습니다. 실제 non-Copy partial move 검사가 worker 완료·result 소비 뒤에도 post-await scope의 operation 1개가 root를 막는 것을 확인했습니다. native restore는 iteration의 handles/guard를 먼저 회수하고 남은 build lease를 반납합니다.
4. [Tokio spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)의 시작한 blocking 작업은 abort로 멈출 수 없다는 계약과 [oneshot Sender::send](https://docs.rs/tokio/latest/tokio/sync/oneshot/struct.Sender.html#method.send)의 receiver 부재 시 값 반환을 확인했습니다. queued 취소/종료 직전 입장 재검사는 기존 TaskSupervisor를 그대로 사용하며 새 registry·state·dependency는 추가하지 않았습니다.
5. 공개 command signature/Rustdoc 25개와 남은 native 함수 28개는 byte 동일입니다. 변경한 attach/restore producer 2개의 body는 명시한 감독 호출·owned result 치환만 다릅니다. capability registry·ProjectRestoreWatchers port·AppState·ExitDrain·TaskSupervisor 구현·runtime project action·Tauri 등록은 변경하지 않았습니다. 기존 source assertion도 실제 runtime helper의 blocking 등록을 확인하도록 옮겼으며 build/guard/commit/recheck 조건은 유지합니다.

## 검증 기록

새 API 부재 E0432 RED(exit 101)를 확인했습니다. 이전 패턴의 메모리 characterization은 감독 async waiter 안에서 미감독 nested blocking을 시작한 뒤 shutdown이 worker 미완료인데도 완료되는 것을 재현합니다. 보강 경로는 실제 ExitDrain이 같은 worker를 기다리고, work gate 해제 후에도 반환 resource의 Drop gate가 닫혀 있으면 readiness가 false인 것을 확인했습니다. 두 gate를 모두 해제해야 ready가 true이고 추적 수가 0입니다.

서로 다른 검사 44건의 성공을 사용하며 같은 입력의 성공과 영향 재검사는 중복 합산하지 않습니다.

| 명령                                                                                                                                                 | 실제 결과                                                     |
| ---------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `cargo test -p taide-runtime --test project_build`                                                                                                   | 최초 8건, exit 0; 이후 변경 없는 7건 성공 재사용              |
| `cargo test -p taide-runtime --test project_build worker_완료_뒤에도_post_await_commit과_event의_owner를_기다린다 -- --exact`                        | non-Copy partial move로 강화한 영향 1건, exit 0               |
| `cargo test -p taide-runtime --test project_build 기존_미감독_nested_worker_패턴은_작업이_미완료여도_root를_해제한다 -- --exact`                     | 추가한 기존 패턴 재현 1건, exit 0; 최종 서로 다른 새 검사 9건 |
| `cargo test -p taide --lib domain::project::commands::tests`                                                                                         | 기존 native 11건, exit 0                                      |
| `cargo test -p taide --lib tests::프로젝트_복원_워처는_조립부_포트로_build와_register를_분리한다 -- --exact`                                         | 기존 실제 조립 source 1건, exit 0                             |
| `cargo test -p taide --test project_lifecycle_actions_runtime --test project_boot_restore_runtime --test capability_symmetry`                        | lifecycle 13·boot 6·capability 4건, exit 0                    |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                         | exit 0                                                        |
| `cargo clippy -p taide-runtime --test project_build -- -D warnings`                                                                                  | 최종 test 변화 관련 target, exit 0                            |
| `cargo clippy -p taide --lib --test project_lifecycle_actions_runtime --test project_boot_restore_runtime --test capability_symmetry -- -D warnings` | exit 0                                                        |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                    | exit 0                                                        |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                     | exit 0                                                        |

새 fixture는 memory resource·자기 worker thread·channel gate만 사용하며 실제 watcher·사용자 home/파일·프로세스·AppHandle·앱·keyring·네트워크는 실행하지 않습니다. Release Drop은 assertion 실패 때도 자기 pause gate를 해제합니다. 이미 시작한 worker의 실제 종료와 resource의 동기 Drop을 별도로 관찰하며 abort 요청을 완료라고 보고하지 않습니다.

실제 binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`와 manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. 공개 생성 입력·등록·모델·Cargo 불변으로 기존 실제 binding 생성/IPC/normal graph 551줄/Tauri 0개 성공을 재사용하며 이번 검사 건수에 합산하지 않았습니다. 신규 history는 docs ignore를 해제한 Prettier로 검사합니다. 기존 PROCESS/architecture 전체 포맷 baseline은 무관하게 재포맷하지 않습니다.

## 남은 경계

정적 command 배치 F193/S0/A13/P0과 전체 M6 미완료 상태를 유지합니다. 실제 native capability/restore의 policy queue·registration callback은 아직 Tauri에 있고, 이번 변경은 worker와 미등록 결과/등록·발행 스코프의 정상 root 소유권 보강입니다. 이미 store에 등록된 watcher의 전체 종료·project action 전체의 guard 입장/outer 이벤트·다른 nonIPC callback/periodic flush·직접 Exit·OS stall/강제 bounded 종료·실제 사용자/GUI 검증을 완료했다고 주장하지 않습니다. 시작한 OS 작업이나 resource Drop이 영구적으로 멈추면 정상 root도 대기하며 이를 임의 timeout으로 성공 처리하지 않습니다. M6/M7/M8와 locale 보안 선택은 미완료이고 M6 전체 완료 전 push/UI는 수행하지 않습니다.
