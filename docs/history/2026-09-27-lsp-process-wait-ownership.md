# 일반 LSP wait 작업 소유와 정상 종료 드레인

## 대상 파일

- `crates/taide-infra/src/lsp_proc.rs`: child wait·reader·종료 callback 완료 핸들과 kill/reap gate
- `crates/taide-lsp/src/store.rs`: 세션과 독립적인 프로세스 소유 목록·입장/종료 gate
- `crates/taide-runtime/src/exit_drain.rs`: 정상 종료의 LSP 작업 완료 대기
- `src-tauri/src/domain/lsp/commands.rs`, `src-tauri/src/lib.rs`와 관련 extraction 검사
- `docs/architecture.md`, `docs/PROCESS.md`, 연결된 QA

## 리포트

기존 infra는 wait worker의 JoinHandle을 버렸고 마지막 LspProcHandle Drop이 child 종료를 요청하지 않았습니다. 자기 생성 sleep child 최소 재현은 Drop 뒤에도 살아 있어 exit 101로 실패했습니다. store의 프로세스 입장·종료·대기 API 부재는 E0599 여섯 건으로 재현했습니다. 핸들에서 실제 wait worker를 소유하고, 세션 제거·재시작 뒤에도 store가 완료까지 프로세스를 보유하도록 구현했습니다.

## 상세

1. `kill`과 child wait future의 각 poll은 같은 mutex gate를 사용합니다. 정상 회수 poll이 끝나기 전에 숫자 PID 시그널 권한을 닫고 실제 성공한 wait에만 exited 플래그를 세웁니다. Pending 동안 mutex를 보유하지 않으며 기존 SIGCHLD 기반 idle 대기와 프레이밍·tail 정책은 보존합니다.
2. ChildWaitOwner는 spawn 전에 만들어져 미poll worker 취소에서도 child field Drop 전에 PID 권한을 닫습니다. command의 kill_on_drop은 조기 pipe 오류·runtime 취소의 보조 종료 요청입니다. Tokio orphan 회수는 best effort이므로 이 fallback을 실제 완료 대기로 표현하지 않습니다.
3. LspProcHandle Drop은 동기 kill을 요청합니다. `wait_for_completion`은 mutex 안의 mutable JoinHandle을 await해 대기 future Drop 뒤에도 핸들을 잃지 않습니다. 성공한 worker 경로의 완료는 child wait, 두 reader의 회수, 종료 callback 반환까지 포함합니다. JoinError·wait 오류·runtime 강제 종료에서 동일한 OS 회수를 보장하지 않습니다.
4. LspStore는 공유 내부 상태에 세션 맵과 독립적인 강한 프로세스 목록을 둡니다. `spawn_process`는 종료 gate 안에서 동기 factory 실행·소유 등록을 직렬화하고 종료 뒤 factory 호출을 거절합니다. factory는 이 프로세스 gate에 재진입하면 안 됩니다. 완료 핸들은 다음 spawn/idle 대기에서 정리합니다. `shutdown` 후 `wait_for_idle`을 사용해야 신규 입장과 snapshot 대기의 경계가 닫힙니다.
5. Tauri의 기존 spawn/restart factory는 같은 store gate를 소비합니다. 정상 ExitDrain은 감독 task·설치 lease 뒤 일반 LSP worker 완료도 기다린 뒤 ready/exit callback을 호출합니다. 직접 native Exit fallback은 여전히 설치 lease만 동기 대기하며, 메뉴 main-thread 응답을 기다리는 전체 감독자를 native callback 안에서 동기 join하지 않습니다.

## 공식 근거와 검증

[Rust poll_fn](https://doc.rust-lang.org/std/future/fn.poll_fn.html), [Tokio 1.53.1 Mutex](https://docs.rs/tokio/1.53.1/tokio/sync/struct.Mutex.html)와 설치된 Tokio 1.53.1 공식 원천 `process/mod.rs`의 Child.wait 취소 안전성·kill_on_drop 계약, `process/unix/reap.rs`의 SIGCHLD 등록→try_wait/Drop orphan 경계를 대조했습니다. 새로운 의존성·unsafe·IPC DTO/서명·bindings 변경은 없습니다.

실제 명령·결과와 미완료 경계는 [일반 LSP wait QA](../quality-assurance/2026-09-27-lsp-process-wait-lifecycle.md)에 기록합니다. 실제 앱·LSP 서버·사용자 프로세스·키링/시크릿은 사용하지 않았습니다.

## 미완료 경계

PTY의 세 thread는 현재 JoinHandle을 보관하지 않으며 Drop의 unpause/kill 요청만으로 실제 thread·callback 완료를 증명할 수 없습니다. 정상 LSP 대기는 직접 Exit/runtime 종료 요청 실패·non-yield callback·wait/JoinError·OS 강제 종료의 bounded 회수를 뜻하지 않습니다. M6 전수 body/port 및 나머지 자원·M7/M8·Phase 0 실기는 별도 gate이며 전체 M6 완료 전 push/UI 실행은 보류합니다.
