# PTY worker 완료 QA

## 대상 파일과 리포트

infra `PtyCompletionHandle`의 세 thread join·대기 취소/재대기·reader unwind stop과 기존 출력 계약을 확인합니다. 완료 handle 존재와 앱 root의 소유/드레인 완료를 구분합니다.

## 실제 결과

- [x] `cargo test --offline -p taide-infra --lib pty::tests::spawn은_reader_flusher --quiet`: 기존 unnamed spawn의 handle 폐기 source RED(exit 101), 세 handle을 완료 handle로 전달한 뒤 1건 GREEN(exit 0)입니다. 실제 프로세스 실패 재현이나 root 소유권 증거로 집계하지 않습니다.
- [x] `cargo test --offline -p taide-infra --lib pty::tests -- --skip drop은_일시정지된 --skip drop은_셸_통합`: 최종 상태 27건 통과(exit 0)입니다. 자기 `/bin/sh` 코드 7 뒤 callback을 붙잡고 대기 취소→세션 Drop→재대기 pending→callback 해제→세 worker 실제 join→반복 대기 성공을 확인했습니다.
- [x] 같은 27건 안에서 synthetic worker panic/오류에도 다른 held worker를 join한 뒤 오류를 반환하고, reader panic 때 flusher가 실제 종료되는 것을 확인했습니다. 기존 PID·pause·batch/flush·builder도 보존했습니다. 앞 source 단독 GREEN과 중복 합산하지 않습니다.
- [x] `cargo test --offline -p taide --test taide_terminal_runtime_extraction --quiet`: 기존 자기 전용 환경의 실제 출력→기록→스캔→callback/종료 코드 1건 통과(exit 0)입니다. 이번 단위는 infra 27·Tauri 1로 서로 다른 28건입니다.

## 정적 검사

- [x] `cargo clippy --offline -p taide-infra -p taide-terminal -p taide-runtime -p taide --all-targets -- -D warnings`: exit 0입니다.
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --offline -p taide-infra -p taide-terminal --no-deps --quiet`, `cargo fmt --all -- --check`, `git diff --check`: exit 0입니다.
- [x] 새 history/QA와 연결된 child wait/LSP wait QA 네 문서의 대상 Prettier write/check: exit 0입니다. dependency/IPC/bindings diff는 없고 같은 입력의 성공을 재사용합니다. 같은 owned_child helper 성공 3건도 변경이 없어 재사용하고 중복 실행하지 않습니다.

## 남은 gate와 실행 조건

- [ ] TerminalStore의 spawn/등록 admission과 실제 blocking spawn worker, 제거/교체 세션의 별도 root 소유 목록 및 정상 ExitDrain 배선을 구현·검증합니다. 마지막 완료 handle Drop까지 작업 소유권을 보장한 것은 아닙니다.
- [ ] thread/pipe 부분 시작 실패·OS waitid/wait 오류·join task의 runtime 취소/실패는 성공한 회수로 처리하지 않습니다. 현재 실패 cache는 성공 플래그를 올리지 않지만 정상 root의 오류 처리·회수 정책은 별도 구현입니다.
- [ ] SIGHUP 무시 child·pipe 보유 자손·blocking callback/Read의 bounded 종료는 미검증입니다. 새 API는 강제 abort하지 않으며 자기 fixture·OS 정책이 준비되면 확인합니다.
- [ ] 기존 infra Drop 2건·프로젝트 회수 fixture의 사용자 profile 격리, Windows/다른 Unix와 native ExitRequested/직접 Exit 및 실제 앱 재시작은 미실행입니다. 현재 성공으로 실기 결과를 대체하지 않습니다.
- [ ] M6-JP/JQ 전체·M6 전수 body/다른 자원·M7/M8·Phase 0은 미완료입니다. M6 전체 완료 전 push/UI는 수행하지 않습니다.
