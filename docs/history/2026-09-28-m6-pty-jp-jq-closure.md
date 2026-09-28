# M6 PTY JP/JQ의 macOS 정상 root 범위 판정

## 대상 파일

- `crates/taide-infra/src/pty.rs`, `crates/taide-terminal/src/store.rs`
- `crates/taide-runtime/src/terminal_actions.rs`, `crates/taide-runtime/src/exit_drain.rs`
- `src-tauri/src/domain/terminal/commands.rs`, `src-tauri/src/lib.rs`
- `docs/PROCESS.md`, `docs/quality-assurance/2026-09-27-terminal-spawn-root-lifecycle.md`

## 리포트

JP/JQ 원문의 구현·검증 범위를 현재 코드와 누적 검증에 다시 대조했습니다. 세 PTY worker의 완료 핸들, 부분 시작 owner, 등록된 spawn lease와 blocking worker, 제거·교체·미반환 세션의 완료 목록, 정상 ExitDrain은 구현·자기 fixture 검증을 마쳤습니다. JQ의 Tauri 배선·공유 상태·IPC·출력 순서도 관련 검사에 포함됩니다. 따라서 이 두 하위 항목만 완료로 표시합니다. 실제 앱의 native Exit·GUI와 M6 전체 완료는 이 판정에 포함하지 않습니다.

## 상세

| JP/JQ 계약                           | 확인한 근거                                                                                                                                                                                    |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| reader/flusher/wait 완료와 부분 시작 | `PtyCompletionHandle::wait_for_completion`, `PtySpawnOwner`와 infra PTY 33건. 시작한 worker를 abort로 완료 처리하지 않고 실패 시 다른 worker도 join합니다.                                     |
| spawn 입장과 제거·교체 소유          | `TerminalSpawnLease::spawn`, `TerminalStore::owned`/`wait_for_idle` 및 core 4·runtime spawn 6건. 요청 Drop 뒤에도 등록 worker와 mutation guard를 추적합니다.                                   |
| 정상 root와 Tauri 조립               | `ExitDrain::begin`/`wait_for_owned_resources`의 runtime 5건, 현행 Tauri spawn 7건·terminal action 10건. callback panic에는 ready를 올리지 않고 기존 이벤트·출력 스캔·replay 경로를 유지합니다. |
| 공개 wire와 종료 한계                | Phase 0 계약 7건·생성 bindings 불변 증거를 재사용했습니다. 자기 HUP 무시 PTY는 100ms 동안 완료 대기가 유지되고 fixture 정리 뒤 join됐습니다. 제품은 승인된 강제 종료 없는 대기를 유지합니다.   |

기존 [전환 계약](../acknowledge/2026-09-23-rust-native-transition-contract.md)은 macOS Apple Silicon을 첫 완료 대상으로 하고 Windows·Linux 조건부 코드의 삭제를 금지합니다. 현재 설치된 Rust target도 `aarch64-apple-darwin`뿐이며 CI Rust job은 macOS입니다. Windows/다른 Unix 실행 결과를 macOS의 JP/JQ 성공으로 주장하지 않고 별도 플랫폼 부채로 남깁니다.

## 검증과 남은 경계

이 판정에서 제품 코드와 검증 입력은 변경하지 않았습니다. 관련 infra 33건, core 4건, runtime spawn 6건·ExitDrain 5건, Tauri spawn 7건·terminal action 10건, IPC 7건의 기존 통과 결과와 신규 HUP 무시 fixture 1건을 파일·입력 상태에 맞춰 재사용했습니다. 이전 all-target Clippy·strict rustdoc·Rust fmt/IPC 성공 및 신규 infra test-target Clippy 성공을 중복 실행하지 않았습니다.

실제 커널 waitid/wait 실패, 그룹 이탈/pipe 보유 자손, 비반환 callback/Read, 직접 native Exit의 메인 스레드 교착 가능성은 합성 검사로 성공을 주장하지 않습니다. QA에 재현 조건·생략 이유·위험·재검 시점을 남깁니다. 실제 앱은 전용 identifier/데이터·키링 격리와 실행 주체 결정 후 M6/M7에서 확인합니다. M6 전체·M7/M8은 미완료이며 승인된 일반 push는 M6 완료 후입니다.
