# PTY 관찰 오류 child wait QA

## 확인한 결과

- [x] 자기 child wrapper의 종료 관찰 `Unsupported`에서 소유 child `wait` 누락을 신규 테스트 exit 101로 재현했습니다. 실패 뒤 fixture가 같은 child를 회수했습니다.
- [x] `cargo test -p taide-infra --lib pty::tests -- --skip drop은_일시정지된 --skip drop은_셸_통합`: 33건 통과(exit 0). 신규 실제 child wait, 기존 권한 반납/늦은 신호 차단·partial spawn/thread join·pause/배치 계약을 확인했습니다.
- [x] `cargo clippy -p taide-infra -p taide-terminal -p taide-runtime --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check`: 각각 exit 0입니다. 공개 signature·의존성·bindings 입력은 바뀌지 않아 기존 성공을 재사용합니다.

## 남은 gate

- [ ] 실제 커널 `waitid`/`wait` 오류와 SIGHUP 무시 child·pipe 보유/그룹 이탈 자손·non-yield callback/Read의 bounded 종료는 검증하지 않았습니다. `wait` 시도와 성공한 회수는 구분합니다.
- [ ] 위험한 기존 user profile Drop fixture 두 건, Windows/다른 Unix, 실제 앱·직접 native Exit, 전체 M6/M7/M8은 미실행입니다. 자기 fixture 외 프로세스·profile은 건드리지 않았습니다.
