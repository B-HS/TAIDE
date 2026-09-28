# 트리 prefetch 종료 소유 QA

## 대상 파일과 리포트

`crates/taide-runtime/src/tree_actions.rs`의 prefetch worker와 다섯 action의 operation, Tauri/원격 배선을 확인합니다. 자기 UUID 디렉터리만 사용하고 실제 앱·사용자 프로젝트는 실행하지 않습니다.

## 수행 결과

- [x] 새 cache hit/miss 종료 admission 검사에서 TaskSupervisor 인수 부재 E0061(exit 101)를 먼저 확인했습니다. 수정 뒤 `cargo test -p taide-runtime 닫힌_감독자는_트리_캐시_미스와_히트의_신규_입장을_거절한다 --lib` 1건 통과했습니다.
- [x] `cargo test -p taide --test tree_actions_runtime` 5건이 통과했습니다. 페이지·캐시·확장/접기/새로고침, 없는 프로젝트, 조회/전역 mutation 경합, 닫힌 프로젝트 재확인을 포함합니다.
- [x] 동일한 `TaskSupervisor::run_blocking_result` 구현은 선행 [파일 worker QA](2026-09-28-file-worker-owner.md)의 요청 abort/root fixture가 통과했고 이번 변경에서 수정하지 않았습니다. 새 tree 코드의 실제 blocked OS read 취소는 별도로 실행하지 않았습니다.
- [x] `cargo clippy -p taide-runtime -p taide --lib --test tree_actions_runtime -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps --quiet`, 실제 TypeScript binding 생성 1건이 exit 0입니다. binding/manifest diff는 없습니다.
- [x] `cargo fmt --all -- --check`, 새 history/QA와 갱신한 audit의 대상 Prettier check, `git diff --check`가 exit 0입니다. PROCESS/architecture 전체 포맷의 기존 baseline은 무관한 재작성 없이 유지했습니다.

## 남은 gate

- [ ] 실제 OS read stall·native/remote 요청 취소와 GUI 실기는 미검증입니다.
- [ ] search의 직접 blocking worker 4개와 전체 M6-HK/M7/M8은 미완료입니다.
