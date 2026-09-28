# watcher 정상 root 완료 소유 QA

## 대상 파일

- `crates/taide-infra/src/watcher.rs`
- `crates/taide-runtime/src/watcher_stop.rs`, `crates/taide-runtime/src/state.rs`, `crates/taide-runtime/src/exit_drain.rs`
- `src-tauri/src/domain/file/capability.rs`, `src-tauri/src/domain/git/watch.rs`, `src-tauri/src/lib.rs`

## 수행 결과

- [x] `cargo test -p taide-infra --lib watcher::tests` 27건 통과: 예약된 Drop 이후에도 callback 자원을 소유하고 stop 실행 뒤 해제함을 확인했습니다.
- [x] `cargo test -p taide-runtime --lib watcher_stop::tests` 1건과 `cargo test -p taide-runtime --lib state::tests::종료는_live_watcher를_폐기하고_중지_완료를_남긴다` 1건 통과: 대기 취소·재대기와 실제 watcher의 live 맵 폐기 순서를 확인했습니다.
- [x] `cargo test -p taide-runtime --lib exit_drain::tests` 11건 통과: 정상·직접 종료가 지연된 중지 작업을 기다리는 신규 2건과 기존 자원 종료 회귀를 포함합니다.
- [x] `cargo test -p taide --test project_lifecycle_actions_runtime` 15건 통과: 열기/닫기·rollback·복원 관련 기존 action 계약을 유지했습니다.
- [x] `cargo test -p taide --lib watcher_종료_소유는_두_builder와_두_exit_경로에_연결된다` 1건 통과: file/Git 빌더와 정상·직접 Exit에 tracker가 연결됐습니다.
- [x] `cargo clippy -p taide-infra -p taide-runtime -p taide --lib --tests -- -D warnings`와 `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-infra -p taide-runtime --no-deps --quiet`가 exit 0입니다.
- [x] `cargo fmt --all -- --check`, `git diff --check`, 신규 history·bug·QA 문서의 Prettier 검사가 exit 0입니다.

## 남은 gate

- [ ] 실제 macOS 앱의 project close·부팅 복원·정상/직접 Exit에서 callback 교착과 OS 오류를 확인합니다.
- [ ] M6 전체와 M7/M8·원격 push는 별도입니다.
