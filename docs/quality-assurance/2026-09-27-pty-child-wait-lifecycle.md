# PTY child wait 시그널 권한 QA

## 대상 파일과 리포트

infra PTY wait owner·공유 killer, owned_child WNOWAIT helper와 Tauri 출력 fixture를 확인합니다. 실제 앱 대신 자기 `/bin/sh`/PTY·가짜 killer·자기 thread와 채널만 사용합니다.

## 실제 결과

- [x] `cargo test --offline -p taide-infra --lib pty::tests::child_wait_회수_뒤의_kill --quiet`: 수정 전 child 코드 7 뒤 가짜 killer가 호출되어 RED(exit 101)입니다. 실제 회수된 PID에 시그널을 보내지 않았습니다. 첫 구현은 downcast 호출 E0599(exit 101)로 테스트에 도달하지 않았으며 trait 경로 수정 뒤 1건 GREEN(exit 0)입니다.
- [x] `cargo test --offline -p taide-infra --lib owned_child::tests --quiet`: 3건 통과(exit 0)입니다. 새 blocking 관찰이 살아 있는 child에서 대기하고 실제 wait 전 여전히 waitable이며 종료 코드 7을 보존합니다. 기존 비차단 관찰·그룹 종료도 보존했습니다. helper 변경 상태가 같아 이 성공을 재사용합니다.
- [x] `cargo test --offline -p taide-infra --lib pty::tests -- --skip drop은_일시정지된 --skip drop은_셸_통합`: 23건 통과(exit 0)입니다. late kill 차단·관찰 대기 중 살아 있는 자기 PTY kill·owner Drop 권한 반납·관찰 오류 반납·기존 pause/batching/flush/builder를 함께 확인했습니다. 앞선 단독 GREEN과 중복 합산하지 않습니다.
- [x] `cargo test --offline -p taide --test taide_terminal_runtime_extraction --quiet`: 자기 전용 profile 환경으로 기존 실제 출력→기록→스캔→callback/종료 코드 보존 1건 통과(exit 0)입니다. 이번 단위의 서로 다른 성공 검사는 infra 26·Tauri 1로 27건입니다.

## 정적 검사

- [x] `cargo clippy --offline -p taide-infra -p taide-terminal -p taide-runtime -p taide --all-targets -- -D warnings`: exit 0입니다.
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --offline -p taide-infra -p taide-terminal --no-deps --quiet`, `cargo fmt --all -- --check`, `git diff --check`: exit 0입니다.
- [x] 새 history/QA와 연결된 PTY pause/LSP wait QA 네 문서의 대상 Prettier write/check: exit 0입니다. dependency manifest·Cargo.lock·bindings/IPC manifest diff는 없으며 같은 타입/계약 입력의 기존 성공을 재사용합니다. 큰 PROCESS/architecture 및 누적 이력은 무관하게 재포맷하지 않습니다.

## 남은 gate와 실행 조건

- [x] 후속 [PTY worker 완료 QA](2026-09-27-pty-worker-completion-lifecycle.md)와 [터미널 spawn/root QA](2026-09-27-terminal-spawn-root-lifecycle.md)에서 세 worker 실제 join·callback 반환·대기 취소/재대기·spawn/등록 입장과 제거/교체/미반환 세션의 정상 root drain을 확인했습니다. [부분 시작 QA](2026-09-27-pty-partial-startup-lifecycle.md)는 초기 owner Drop의 자기 child wait와 부분 thread 오류 회수를 확인합니다. kill 요청·callback 진입·handle Drop만을 join으로 해석하지 않습니다.
- [ ] SIGHUP 무시·pipe 보유 자손과 실제 waitid/wait OS 오류·callback panic의 회수는 자기 fixture/OS 정책으로 확인해야 합니다. 이 배치의 unsupported 가짜 child 검사에는 실제 child가 없었습니다. 후속 [관찰 오류 child wait QA](2026-09-28-pty-observation-error-wait.md)는 실제 자기 child wrapper의 unsupported 분기에서 wait 시도를 확인했지만 커널 waitid 오류 주입이나 bounded 종료를 증명하지 않습니다.
- [ ] 기존 infra 실제 Drop 2건은 자기 profile/환경 구성을 갖춘 뒤 확인합니다. 프로젝트 회수 fixture는 후속 root 단위에서 ENV/BASH_ENV를 비우고 admission/actual idle 대기에 연결했습니다. zsh의 사용자 profile을 실행하거나 회수된 PID에 실제 시그널을 보내는 검사는 하지 않습니다.
- [ ] Windows·다른 Unix 및 실제 native ExitRequested/직접 Exit/앱 재시작은 미실행입니다. 플랫폼/실기 실행 권한과 fixture가 준비되었을 때 별도 검증하며 현재 성공으로 대체하지 않습니다.
- [ ] M6-JP/JQ 전체와 M6 전수 body·다른 자원, M7/M8·Phase 0은 미완료입니다. 일반 push는 M6 전체 완료 후입니다.
