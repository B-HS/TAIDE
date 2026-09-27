# 주기 layout flush의 runtime 이전과 worker 감독

## 대상 파일

- `crates/taide-runtime/src/layout_actions.rs`, `tests/periodic_layout_flush.rs`
- `src-tauri/src/lib.rs`의 주기 flush setup과 cfg(test) source assertion
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

주기 flush loop를 runtime으로 이전하고 실제 저장 blocking worker를 같은 등록 TaskSupervisor로 추적·await합니다. 기존에는 감독된 async ticker가 직접 생성한 nested worker를 정상 root가 추적하지 못했습니다. 이제 ticker 취소 뒤에도 이미 시작한 저장 worker의 완료를 정상 root가 기다립니다. 같은 state·기존 2초 interval·worker await를 유지하며 창/exit의 동기 flush는 변경하지 않습니다.

## 상세

1. native는 등록 AppState·TaskSupervisor를 clone해 runtime loop future에 전달하고 기존 `layout-flush` 이름으로 같은 감독자에 등록합니다. AppHandle은 loop/worker에 전달하지 않습니다. runtime은 기존 Tokio interval→tick await→state clone→등록 blocking worker→worker await 순서입니다. 감독자가 닫혔으면 worker 입장을 거절하고 loop를 끝냅니다.
2. [Tokio interval](https://docs.rs/tokio/latest/tokio/time/fn.interval.html)의 즉시 첫 tick·기본 Burst·nonzero period 계약을 확인했습니다. Native의 interval은 원래 상수 2,000ms를 전달하며 clock/missed-tick 설정을 새로 변경하지 않았습니다. [spawn_blocking](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html)의 이미 시작한 작업은 abort로 완료되지 않는 계약에 따라 실제 worker handle을 추적합니다.
3. 기존 flush_dirty_layouts는 dirty drain→snapshot→동기 저장이며 body를 그대로 유지합니다. 없는 layout·저장 실패의 로그/생략과 drain된 dirty를 자동 복구하지 않는 기존 정책도 유지합니다. 새 timeout·트랜잭션·자동 retry를 추가하지 않았습니다. worker 결과는 unit이며 별도 post-await 이벤트/resource를 생산하지 않아 새 operation/registry는 필요하지 않습니다.
4. 기존 runtime 제품 함수 28개는 byte 동일입니다. Native 제품 본문은 해당 setup producer 외에는 byte 동일하며 등록/공개 계약·동기 window/exit flush·domain/layout/service의 상수/재수출·TaskSupervisor/ExitDrain·Cargo/모델은 불변입니다. source assertion은 실제 runtime tick body의 blocking 등록과 정확한 worker await를 확인합니다.

## 검증 기록

새 API 부재 E0432 RED(exit 101)를 확인했습니다. 실제 저장/root/중복/닫힌 입장 4건은 통과했고 첫 source 검사는 rustfmt가 spawn 호출을 줄바꿈한 문자열 모양 때문에 실패했습니다. loop future를 변수로 분리해 기존 `.spawn("layout-flush", ...)` 등록 모양을 보존하고 영향 source 1건만 재확인했습니다. 제품 정책이나 검사 조건을 완화하지 않았습니다. 최종 서로 다른 검사 32건이 통과했고 같은 입력의 성공은 재사용하며 중복 합산하지 않습니다.

| 명령                                                                                                                                    | 실제 결과                                                         |
| --------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `cargo test -p taide-runtime --test periodic_layout_flush`                                                                              | 실제 자기 파일/lock/root 4건 성공·source 모양 1건 실패; exit 101  |
| `cargo test -p taide-runtime --test periodic_layout_flush native는_같은_state_supervisor_interval로_runtime_loop를_등록한다 -- --exact` | 수정 뒤 영향 source 1건, exit 0; 기존 4건과 새 서로 다른 검사 5건 |
| `cargo test -p taide-runtime --lib layout_actions::tests`                                                                               | 기존 flush 4건, exit 0                                            |
| `cargo test -p taide --lib tests::주기적_레이아웃_flush_는_blocking_스레드에서_실행된다 -- --exact`                                     | 기존 실제 tick source 1건, exit 0                                 |
| `cargo test -p taide --test task_supervisor`                                                                                            | 기존 감독/배선 22건, exit 0                                       |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                            | exit 0                                                            |
| `cargo clippy -p taide --lib --test task_supervisor -- -D warnings`                                                                     | 최종 native producer에서 exit 0                                   |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                       | exit 0                                                            |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                        | exit 0                                                            |

Fixture는 자기 UUID 데이터 디렉터리와 실제 공유 AppState·Core layout 저장만 사용합니다. 자기 controller thread가 layouts write lock을 보유하고 worker가 dirty를 drain한 뒤 snapshot read에서 기다리게 합니다. 같은 실제 ExitDrain이 ticker 취소 뒤에도 readiness false·파일 미생성을 유지하고, lock 해제 후 실제 layout 파일 저장을 확인해야 ready가 true가 됩니다. 여러 tick이 지난 동안 재삽입한 dirty가 소비되지 않고 추적 수가 loop+worker 2개로 유지돼 같은 loop의 저장 중복을 막는 것도 확인했습니다. Release Drop은 자기 lock gate를 해제하고 controller thread를 join하며 fixture Drop은 자기 UUID 디렉터리만 정리합니다. 실제 사용자 home/파일·AppHandle·앱·프로세스·keyring·네트워크는 사용하지 않았습니다.

실제 binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`와 manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. 논리 생성 입력·등록·모델·Cargo가 불변이므로 같은 실제 생성/IPC/normal graph 551줄/Tauri 0개 성공을 재사용하며 이번 검사 건수에 합산하지 않습니다. 신규 history는 docs ignore를 해제한 Prettier로 검사하고 기존 PROCESS/architecture 전체 포맷 baseline은 무관하게 재포맷하지 않습니다.

## 남은 경계

이번 단위는 주기 loop와 실제 worker의 정상 종료 추적입니다. Window/ExitRequested/Exit의 동기 flush·dirty 저장 실패의 부분 정책·다른 callback/application·project의 전체 guard 입장/outer 이벤트·등록된 watcher 전체 회수·실제 OS stall/직접 Exit/강제 bounded 종료·사용자 GUI 검증은 미완료입니다. 시작한 fsync/lock이 영구적으로 멈추면 정상 root도 기다리며 이를 임의 timeout으로 성공 처리하지 않습니다. F193/S0/A13/P0과 전체 M6/M7/M8·locale 선택 미완료, M6 전체 완료 전 push/UI 금지를 유지합니다.
