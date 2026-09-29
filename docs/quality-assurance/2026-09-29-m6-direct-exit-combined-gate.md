# M6 직접 Exit 결합 자원 게이트

## 대상과 판정 기준

`crates/taide-runtime/src/exit_drain.rs`의 직접 종료가 감독 작업과 등록된 watcher·LSP·PTY 자원의 마지막 완료까지 기다리는지 판정했습니다. 실제 앱의 원격 WebSocket·PTY 직접 종료와 Tauri `Exit` 배선 검증은 기존 성공 결과를 재사용했습니다. 사용자 프로젝트·시크릿은 사용하지 않았습니다.

## 이번 결합 검사

- `cargo test -p taide-runtime --lib 직접_종료는_원격_감독_작업과_watcher_lsp_pty의_완료를_모두_기다린다 --offline --quiet`: 1건 통과, 175건 필터 제외, exit 0. 빌드에 약 7분이 걸렸고 테스트 본문은 0.26초였습니다.
- 한 직접 `ExitDrain` 호출에서 원격 writer를 나타내는 `TaskSupervisor` operation, 중지 중인 watcher callback, 실제 `/bin/sh` LSP 프로세스 callback, 실제 PTY callback을 동시에 붙잡았습니다. 종료 중 신규 작업 등록은 거부됐고, 네 소유자를 순서대로 해제할 때마다 남은 자원이 있으면 직접 종료가 계속 대기했습니다. 마지막 PTY callback 반환 뒤 종료가 성공하고 감독 작업 수는 0, LSP·PTY 완료 핸들은 finished였습니다.
- `cargo clippy -p taide-runtime --lib --tests --offline -- -D warnings`: exit 0. `cargo fmt --all -- --check`와 `git diff --check`: exit 0. 변경은 runtime 테스트와 문서뿐이며 제품 IPC·wire·종료 정책은 그대로입니다.

## 연결한 기존 실측

- [격리 앱 실측](2026-09-29-m7-remote-direct-exit-verified.md): 활성 원격 WebSocket·PTY가 있는 release 앱에서 선행 `ExitRequested` 없는 `applicationWillTerminate → 직접 종료 이벤트 수신 → 직접 종료 자원 대기 완료`, 앱 exit 0과 자식·포트 정리를 확인했습니다. 재시작 뒤 원격 서버도 껐습니다.
- [M6 이벤트 실측](../history/2026-09-29-m6-exit-event-trace.md): 정상 요청 경로와 직접 `Exit` 경로의 이벤트 순서, 별도 실행의 watcher 인덱싱·PTY·LSP 자식 정리를 확인했습니다.
- [종료 lifecycle QA](2026-09-27-exit-drain-lifecycle.md): Tauri 직접 `Exit`가 같은 `ExitDrain`과 실제 State 인수를 쓰는 source contract, 설치·AI owner·감독 작업 및 LSP·PTY callback 대기 검사를 확인했습니다. 현행 결합 검사는 이들 성공을 재실행하지 않고 동시 대기만 추가합니다.
- [M7 자동 게이트](2026-09-28-m7-automated-gate.md)의 Rust workspace 전체 테스트·Clippy·fmt, 프론트엔드 test·typecheck·build와 [Phase 0 계약](2026-09-23-rust-native-parity-plan.md)의 IPC·event·raw channel fixture 결과를 재사용했습니다.

## 결론과 한계

등록 자원에 대한 직접 Exit adapter 게이트는 실앱 원격·PTY 동시 종료, Tauri 배선, 네 자원 동시 대기 검사를 합쳐 통과로 판정합니다. 한 앱 실행에서 원격 WebSocket·watcher·LSP·PTY 네 자원을 모두 바쁘게 만든 실측이라고 주장하지 않습니다. 원격 writer는 결합 검사에서 실제 WebSocket이 아닌 감독 operation으로 표현됐습니다. 미등록 worker, OS I/O stall·강제 종료, Windows process tree는 이 게이트의 보장 범위 밖이며 별도 QA 부채입니다.

## 후속 QA 부채

실제 앱에서 네 자원을 모두 동시에 지연시키고 직접 종료하는 조건은 원격 접근 허용·LSP 서버·watcher callback·PTY 자식을 한 격리 실행에 안정적으로 준비해야 해 이번 단일 검증 범위에서 생략했습니다. 자원별 등록 경로가 실제 앱 배선과 다르면 합성 대기가 놓칠 위험이 있으므로 native UI로 종료 adapter를 바꾸기 전 같은 조건을 재현해 PID·포트·callback 완료를 확인합니다. OS I/O가 영구히 멈추거나 강제 종료된 경우와 Windows process tree는 현재 등록 자원 대기 보장 밖이며 해당 플랫폼 종료 정책을 설계·검증할 때 별도 판정합니다.
