# Agent marker release의 종료 입장과 호출 수명 소유

## 대상 파일

- `crates/taide-runtime/src/agent_actions.rs`
- `src-tauri/src/domain/agent/commands.rs`, `src-tauri/src/remote_gateway.rs`, `src-tauri/tests/agent_actions_runtime.rs`
- `docs/PROCESS.md`, `docs/architecture.md`, `docs/history/2026-09-28-m6-agent-action-admission-audit.md`

## 리포트

공개 marker release는 같은 등록 TaskSupervisor operation을 mutation lock 대기 전부터 경로 검증·동기 삭제/NotFound 처리·tracking 해제와 반환까지 보유합니다. 닫힌 감독자와 `AppState` 종료 표시 뒤의 새 요청은 파일 및 tracking을 건드리지 않고 Forbidden으로 거절합니다. 이미 입장한 요청과 Exit cleanup의 삭제 경쟁은 기존 NotFound 성공 정책으로 멱등 처리합니다.

## 상세

1. 원래 경로는 `begin_mutation().await`→`validate_wait_marker_path`→`remove_file`/NotFound 성공→오류와 무관하게 `forget_wait_marker` 순서였습니다. 이 본문과 입력 검증/오류 정책은 그대로이며 앞에 등록 operation과 shutdown 표시 gate만 추가했습니다. 요청 task가 lock을 기다리는 동안 정상 `ExitDrain`은 operation 반납까지 ready가 되지 않습니다.
2. Tauri command는 이미 등록된 TaskSupervisor를 managed State로 받고 remote gateway도 같은 State를 추가 전달합니다. 외부 `marker` 인자, 반환 DTO, 기존 원격 허용 정책과 생성 bindings/manifest 입력은 바꾸지 않았습니다. `AppState::begin_shutdown`이 cleanup보다 먼저 호출되므로 표시 이후 새 요청을 차단합니다. 단, 표시와 이미 시작한 요청의 동시성은 완전한 선형화가 아닙니다.
3. 자기 UUID marker 파일/빈 디렉터리와 메모리 lock/oneshot만 사용했습니다. 종료 cleanup이 대기 요청보다 먼저 자기 파일을 삭제해도 뒤 요청은 NotFound 성공으로 끝나며 root는 실제 완료 뒤 ready가 됩니다. lock 대기 중 caller를 취소하면 operation을 반환하고 아직 삭제되지 않은 파일과 tracking을 보존합니다. 실제 사용자 파일/앱·원격 세션은 실행하지 않았습니다.

## 검증 기록

새 TaskSupervisor 인수 부재 E0061 compile RED(exit 101)를 먼저 확인했습니다. 구현 뒤 `cargo test -p taide --test agent_actions_runtime` 15건이 통과했습니다. 이 중 신규 marker 4건은 닫힌 supervisor, AppState 종료 표시, lock 대기/Exit cleanup/정상 root, caller 취소를 확인하며 기존 11건은 list/marker/queue/공개 위임 결과를 유지합니다. 생성 bindings SHA-256은 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, IPC manifest SHA-256은 `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`로 불변입니다.

`cargo test -p taide --test rust_native_phase0_contract` 7건과 `cargo test -p taide --lib typescript_바인딩을_생성한다 -- --exact tests::typescript_바인딩을_생성한다` 1건이 exit 0입니다. 실제 재생성 뒤 bindings 파일과 digest는 바뀌지 않았습니다. 신규/기존 action 15건을 포함한 서로 다른 검사 총 23건입니다. `cargo clippy -p taide-runtime --all-targets -- -D warnings`, `cargo clippy -p taide --lib --test agent_actions_runtime -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`, `cargo fmt --all -- --check`, `git diff --check`도 각각 exit 0입니다. 같은 코드 상태의 이미 성공한 검사는 반복하지 않았습니다. 실제 OS 파일 삭제 지연, 직접 native Exit·GUI·원격 실기는 검증되지 않았습니다.

## 남은 경계

동기 `remove_file`의 OS stall은 이미 시작한 작업을 abort해 해결할 수 없고, 직접 `RunEvent::Exit`는 정상 ExitDrain의 전체 대기와 다릅니다. hook 서버 cache/store 경쟁과 transport 실기, pending external open의 종료 소비 정책, CLI kill/reap·Windows/GUI·전체 M6/M7/M8은 미완료입니다. 전체 M6 완료 전 push/UI 착수는 보류합니다.
