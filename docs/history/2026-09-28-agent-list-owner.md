# Agent list의 공개 호출 전체 정상 종료 소유권

## 대상 파일

- `crates/taide-runtime/src/agent_actions.rs`
- `src-tauri/src/domain/agent/commands.rs`, `src-tauri/tests/agent_actions_runtime.rs`
- `docs/PROCESS.md`, `docs/architecture.md`, `docs/history/2026-09-28-m6-agent-action-admission-audit.md`

## 리포트

공개 `agent_list`의 프로젝트 확인 이후 PID 조회→probe callback await→detected 상태 조립→결과 반환까지 AppServices와 같은 등록 TaskSupervisor operation으로 소유합니다. 기존 프로젝트 미존재 오류는 종료 거절보다 앞서며, 종료 뒤 유효한 프로젝트 요청은 lazy PID/probe port를 호출하지 않습니다. Native와 원격 gateway의 공개 입력·출력은 변경하지 않았습니다.

## 상세

1. 이전 구현은 probe worker가 실제로 시작된 경우 그 worker만 감독했습니다. PID가 없거나 probe cache가 채워진 경우에는 worker가 없고, callback await 이후의 공개 list application도 별도 owner가 없었습니다. 새 operation은 project gate 직후 입장해 최종 결과 또는 오류까지 유지됩니다. 닫힌 입장에서는 기존 agent runtime shutdown Forbidden 오류를 사용합니다.
2. Native의 기존 `TaskSupervisor` State를 probe callback뿐 아니라 runtime list에도 전달합니다. remote gateway는 같은 Native command를 직접 호출하므로 인자/응답 분기, 권한 정책과 wire는 그대로입니다. probe 자체의 blocking worker/결과 회수 및 project gate·PID·activity/override 순서는 변경하지 않았습니다.
3. synthetic pending callback 중 정상 ExitDrain은 ready가 되지 않고, 요청 task의 실제 취소/JoinHandle 완료 뒤 ready가 됩니다. 이 검사는 synthetic 메모리·oneshot·같은 TaskSupervisor만 사용하며 실제 사용자 PID·CLI·앱/원격 서버·파일은 실행하지 않습니다. [Tokio JoinHandle](https://docs.rs/tokio/1.53.1/tokio/task/struct.JoinHandle.html)의 abort 요청과 실제 완료를 구분합니다.

## 검증 기록

새 인수 부재 E0061을 포함한 compile RED(exit 101)를 먼저 확인했습니다. 구현 뒤 `cargo test -p taide --test agent_actions_runtime` 11건이 통과했습니다. 신규 2건은 종료 뒤 PID/probe 무호출과 pending callback의 정상 root 대기/요청 취소를 검사합니다. 기존 9건은 project gate·조회 순서·probe 오류·marker/queue 및 runtime 위임을 유지합니다. public Native command signature·등록·모델/Cargo·생성 입력은 변경하지 않았으며 실제 bindings/manifest SHA-256은 각각 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`로 불변입니다.

`cargo clippy -p taide-runtime --all-targets -- -D warnings`, `cargo clippy -p taide --lib --test agent_actions_runtime -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`, `cargo fmt --all -- --check`, `git diff --check`가 각각 exit 0입니다. 같은 코드 상태의 이미 성공한 11건은 반복하지 않았습니다. 실제 PID probe/OS stall·원격/GUI·직접 Exit 실기는 이 검사로 대체하지 않습니다.

## 남은 경계

`agent_release_marker`의 동기 파일 삭제와 종료 cleanup 경쟁, `agent_pending_external_opens`의 종료 뒤 queue 소비 정책, hook 서버 cache/store 경쟁·transport 실기, CLI kill/reap·OS stall·직접 Exit 및 전체 M6/M7/M8은 미완료입니다. 전체 M6 완료 전 push와 native UI 착수는 보류합니다.
