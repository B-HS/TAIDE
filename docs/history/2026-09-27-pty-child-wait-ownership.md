# PTY child wait의 시그널 권한 반납

## 대상 파일

- `crates/taide-infra/src/pty.rs`: child wait owner·공유 killer 권한·자기 PTY/가짜 killer 회귀
- `crates/taide-infra/src/owned_child.rs`: 단독 소유 direct child의 blocking WNOWAIT 관찰
- `src-tauri/tests/taide_terminal_runtime_extraction.rs`: 자기 `/bin/sh`의 profile 환경 차단
- `docs/architecture.md`, `docs/PROCESS.md`, PTY 종료 QA

## 리포트

기존 Unix 복제 killer는 숫자 PID를 보유하지만 child wait는 별도 thread에서 PID를 회수했습니다. 종료 callback 뒤에도 세션의 kill/Drop이 그 숫자 PID에 SIGHUP을 다시 보낼 수 있었습니다. 자기 PTY child를 코드 7로 종료하고 OS 시그널 대신 호출 여부만 기록하는 가짜 killer를 설치한 fixture에서 회수 뒤 killer 호출을 재현했습니다(exit 101).

## 상세

1. native Unix PTY는 `std::process::Child`를 반환합니다. wait owner가 이를 단독으로 소유하고 `waitid(P_PID, WEXITED | WNOWAIT)`로 종료를 관찰합니다. 살아 있는 동안 mutex를 보유하거나 polling하지 않으며 PID/exit status를 회수하지 않습니다.
2. 종료 관찰 뒤 세션과 같은 killer mutex를 잠가 권한을 반납한 다음 portable child wait로 실제 회수합니다. kill이 먼저 잠그면 아직 회수되지 않은 child만 대상으로 하며, 권한 반납이 먼저 잠그면 늦은 kill/Drop은 시그널 없이 성공합니다. helper의 PID 범위·siginfo 일치·EINTR 재시도 검증을 재사용합니다.
3. child owner는 pipe 복제보다 먼저 생성합니다. owner Drop은 남아 있는 권한으로 기존 종료 요청을 한 뒤 권한을 반납하고 child handle을 놓습니다. Drop의 종료 요청을 실제 wait나 thread join으로 해석하지 않습니다. 이 배치의 종료 관찰 오류 경로는 권한을 닫고 오류를 반환했으나 소유 child의 실제 wait를 시도하지 않았습니다. 2026-09-28 후속 수정은 [관찰 오류 child wait](2026-09-28-pty-observation-error-wait.md)에 따로 기록합니다.
4. Windows는 복제 OS handle의 기존 wait/termination 정책을 유지하고 wait 뒤 권한을 반납합니다. 현재 macOS 결과로 Windows/다른 Unix 실행 검증을 대체하지 않습니다. Unix SIGHUP 정책·출력 batching·scan/replay·IPC·의존성은 변경하지 않습니다.
5. 새 실제 PTY fixture와 기존 Tauri 출력 fixture에는 `ENV`/`BASH_ENV`를 빈 값으로 명시해 사용자 profile 실행을 차단합니다. 실제 앱·사용자 프로세스·시크릿은 사용하지 않으며 회수된 숫자 PID에 실제 시그널을 보내는 재현은 하지 않습니다.

## 공식 근거와 검증

[Rust Child](https://doc.rust-lang.org/std/process/struct.Child.html)의 wait/try_wait 계약, 설치된 portable-pty 0.9.0의 native Unix `spawn_command`/`ProcessSignaller`와 downcast-rs 1.2.1의 `as_any_mut`를 대조했습니다. 직접 `downcast_mut` 호출은 `Child + Send + Sync`에 메서드가 없어 컴파일 실패(E0599, exit 101)했고 실제 trait의 `as_mut().as_any_mut()` 경로로 수정했습니다. unsafe cast나 새 의존성은 추가하지 않았습니다. [Apple XNU waitid 원천](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_exit.c)의 WNOWAIT/no-reap 및 blocking 대기도 확인했습니다.

명령과 실제 결과는 [PTY child wait QA](../quality-assurance/2026-09-27-pty-child-wait-lifecycle.md)에 기록합니다. 같은 helper 변경 상태의 성공 결과는 재사용하며 중복 실행은 합산하지 않습니다.

## 미완료 경계

세 reader/flusher/wait thread의 완료 핸들, blocking spawn 입장과 제거/교체 세션 소유 목록, 정상 root drain은 M6-JP 잔여 작업입니다. SIGHUP 무시 child·pipe 보유 자손·OS wait 오류·callback panic·직접 native Exit도 별도 gate입니다. pause 깨우기와 안전한 시그널 권한 반납만으로 OS Read 취소나 전체 자원 회수를 완료 처리하지 않습니다. M6 전체·M7/M8·Phase 0은 미완료이며 승인된 일반 push는 M6 전체 완료 후에만 수행합니다.
