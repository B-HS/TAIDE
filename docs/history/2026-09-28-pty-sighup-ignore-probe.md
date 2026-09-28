# PTY SIGHUP 무시 자식의 완료 대기 실측

## 대상 파일

- `crates/taide-infra/src/pty.rs`의 Unix `ProcessSignaller` 소비 경로와 `PtyCompletionHandle`
- `docs/PROCESS.md`, `docs/quality-assurance/2026-09-27-terminal-spawn-root-lifecycle.md`

## 리포트

설치된 `portable-pty 0.9.0`의 Unix 복제 killer는 자식 PID에 SIGHUP만 보냅니다. 사용자 profile을 끈 자기 `/bin/sh` PTY에서 HUP을 무시하도록 설정한 뒤 출력으로 준비를 확인하고 `PtySession::kill`을 호출했습니다. 완료 핸들의 대기는 100ms 관찰 동안 끝나지 않았습니다. 이 검사는 현행 제품 종료 정책의 실제 한계를 확인한 것이며 새 유예 시간이나 제품 SIGKILL을 추가하지 않았습니다.

## 상세

테스트는 shell의 준비 문자열을 확인한 다음 HUP을 보냅니다. 완료 대기 future가 관찰 시간에 끝나지 않은 뒤 fixture 정리에 한해서 미회수 자기 자식의 신호 권한 mutex를 보유한 채 SIGKILL을 보내고 reader·flusher·wait worker의 완료 join을 확인합니다. 따라서 테스트가 앱의 자원 대기를 방치하거나 재사용된 PID에 신호를 보내지 않습니다. 100ms는 검사의 관찰 구간일 뿐 제품 동작 상한이 아닙니다.

자손이 PTY fd를 보유하거나 그룹을 이탈한 경우, 실제 커널 waitid/wait 오류, callback·OS read의 장기 정지, Windows/다른 Unix와 직접 native Exit는 이 한 건으로 증명하지 않습니다. 제품에서 PTY 전용 강제 종료를 도입할지 여부는 사용자 결정 대기 중입니다.

## 검증

- `cargo test --offline -p taide-infra --lib pty::tests::sighup을_무시하는_자기_pty는_종료_요청만으로_완료되지_않는다 --quiet`: 최종 fixture 1/1 통과. 초기 callback 관찰형 1/1 성공 뒤 root가 쓰는 완료 핸들 대기로 좁혀 다시 검사했습니다. 두 성공을 별도 범위의 2건으로 합산하지 않습니다.
- `cargo clippy --offline -p taide-infra --tests -- -D warnings`: exit 0. `--lib` 성공은 test body를 검사하지 않으므로 별도 합산하지 않습니다.
- `cargo fmt --all --check`, `git diff --check`: exit 0.

실제 앱·사용자 프로세스·사용자 profile·시크릿은 사용하지 않았습니다. M6-JP/JQ 및 전체 M6~M8은 미완료이고 승인된 일반 push는 M6 완료 후입니다.
