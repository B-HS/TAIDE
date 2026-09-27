# 메뉴 blocking 작업 감독

상태: 메뉴 listener의 worker/결과 waiter 감독과 이 단위 자동 검증 완료. M6 전체·M7·M8은 미완료입니다.

## 대상 파일과 감독 계약

`crates/taide-runtime/src/task_supervisor.rs`에 spawn_blocking_transient_handle을 추가했습니다. 기존 Tokio handle로 FnOnce worker 자체를 실행하고 호출별 Blocking key·AbortHandle·cleanup을 등록합니다. 동일 이름의 연속 event도 각각 실행하며 완료·queued 취소·panic에서 해당 key를 회수합니다. 종료 뒤 신규 등록을 거절하고 worker 진입에서 종료 상태를 확인합니다. 기존 async named/transient 등록·중복 거절·JoinHandle 반환 계약은 유지합니다. 외부/로컬 dependency·버전 추가는 없습니다.

stop_all은 모든 작업에 abort를 요청하되 기존 async 항목만 즉시 회수하고 blocking 항목은 실제 cleanup까지 유지합니다. 이미 진입한 blocking 작업이 중단된 것처럼 tracked_count를 0으로 만들지 않습니다. worker가 종료 gate를 통과한 뒤 실행을 마칠 때까지 감독 목록에 남으며, 작업 본문에는 감독자 mutex를 유지하지 않습니다. 여러 번 stop_all해도 같은 worker를 강제로 종료한다고 해석하지 않습니다.

이 경계는 현재 Cargo.lock과 같은 Tokio 1.53.1의 [Handle::spawn_blocking](https://docs.rs/tokio/1.53.1/tokio/runtime/struct.Handle.html#method.spawn_blocking)과 [AbortHandle::abort](https://docs.rs/tokio/1.53.1/tokio/task/struct.AbortHandle.html#method.abort) 계약에 맞춥니다. 시작한 blocking 작업은 abort할 수 없고 queued 작업은 시작을 막을 수 있습니다. [JoinHandle](https://docs.rs/tokio/1.53.1/tokio/task/struct.JoinHandle.html) Drop만으로 작업이 종료되지 않으므로 결과 waiter의 취소와 worker 실제 완료를 구분합니다. 이 API는 시작한 OS 호출의 강제 취소·bounded drain·main-thread menu callback 취소를 보장하지 않습니다.

`src-tauri/src/lib.rs`의 recent/settings 두 listener는 공통 schedule_menu_refresh를 소비합니다. listener는 디스크 I/O를 하지 않고 worker를 등록한 뒤 감독한 async waiter가 JoinHandle 결과를 await/로그합니다. 종료 중 waiter 등록이 거절되거나 기존 waiter가 취소돼도 worker 자체의 추적은 유지합니다. project list/activated는 각 event마다 최근 메뉴를 갱신하고 settings payload 파싱·같은 language 무갱신·drawn mutex 해제 뒤 전체 메뉴 갱신 순서는 불변입니다. 실제 메뉴 갱신/OS main-thread 전송은 기존 window adapter에 유지합니다. 기존 메뉴 실패는 원래처럼 adapter가 로그하며 원래 event를 실패로 바꾸지 않습니다.

## 검증 근거와 범위

- 새 `crates/taide-runtime/tests/task_supervisor_blocking.rs`는 API 부재 E0599(exit 101)로 먼저 실패했습니다. `cargo test -p taide-runtime --test task_supervisor_blocking --quiet` 변경 후 6건 통과, exit 0입니다. handshake로 실행/대기를 분리해 이름별 반복·완료/재등록, async waiter 취소 뒤 running worker 추적, 단일 blocking pool의 queued 취소·capture 해제, 종료 후 등록 거절, panic JoinError/회수와 실제 listener source 계약을 확인합니다. sleep 기반 경합 추정은 사용하지 않습니다.
- `cargo test -p taide --test task_supervisor --test rust_native_phase0_contract --test domain_boundaries --quiet`: 기존 감독 22건·IPC 7건·도메인 3건 통과, exit 0입니다. 변경 후 서로 다른 검사 총 38건이며 기존 서버 fixture도 synthetic state/task만 사용합니다.
- runtime/Tauri all-target clippy·strict runtime rustdoc·fmt/diff는 exit 0입니다. Cargo.toml/Cargo.lock·bindings/manifest diff는 없고 bindings SHA-256은 f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a로 불변입니다. public IPC 문서·시그니처가 불변이므로 bindings 생성과 불변 dependency graph 검사는 반복하지 않습니다. 직전 normal runtime graph의 Tauri 미의존 결과를 재사용합니다.

실제 앱·OS 메뉴·사용자 history/설정·외부 설치기/LSP/PTY process는 실행하지 않았습니다. 전체 workspace/frontend/GUI, 창 이동/탭 복귀 application 경계와 LSP 설치/reader·PTY 수명 소유권, 전체 command body 판정은 미완료입니다. 일반 push는 기존 승인대로 M6 전체 완료 후 수행합니다.
