# watcher 명시 중지 수단 QA

## 수행 결과

- [x] `cargo test -p taide-infra --lib watcher::tests::명시적_감시_중지는_콜백_자원_해제까지_기다린다 -- --exact`가 API 부재 E0599(exit 101)로 먼저 실패했고, 추가 후 1건 통과했습니다.
- [x] `cargo test -p taide-infra --lib watcher::tests` 26건이 통과했습니다. 자기 UUID 임시 감시 루트 외의 사용자 프로젝트는 사용하지 않았습니다.
- [x] `cargo clippy -p taide-infra --lib --tests -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-infra --no-deps --quiet`, `cargo fmt --all -- --check`, `git diff --check`가 exit 0입니다.

## 남은 gate

- [ ] project detach·복원·정상/직접 Exit의 watcher 실제 완료 소유와 잠금 순서, callback stall은 아직 검증하지 않았습니다.
- [ ] M6/M7/M8 전체와 push는 별도입니다.
