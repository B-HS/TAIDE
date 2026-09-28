# 트리 prefetch worker 종료 소유

## 대상 파일

- `crates/taide-runtime/src/tree_actions.rs`
- `src-tauri/src/domain/tree/commands.rs`, `src-tauri/src/remote_gateway.rs`, `src-tauri/tests/tree_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`, 관련 QA

## 리포트

다섯 tree action이 공유하는 직접 `spawn_blocking` prefetch를 기존 TaskSupervisor의 결과형 worker로 이전했습니다. 각 action은 캐시 hit처럼 worker가 없는 경로도 포함해 처음부터 마지막 페이지 조립까지 operation lease를 보유합니다. 요청이 취소된 뒤 이미 시작한 prefetch worker의 실제 완료는 root가 추적합니다.

## 상세

1. `tree_rows`는 전역 mutation guard 없이 조회하고, miss의 프로젝트/entry 재확인 및 로컬 트리 삽입을 유지합니다. 수정 네 action은 기존처럼 prefetch 뒤 mutation/write lock을 취득합니다.
2. 닫힌 감독자에서는 cache hit·miss 모두 신규 입장을 거절합니다. Tauri는 기존 AppServices TaskSupervisor State만 주입하고 원격 gateway도 같은 State를 전달합니다. 공개 command 인자·응답, TreeStore 경로, perf span은 유지했습니다.
3. 메모리 감독자 경계의 기존 blocking 결과 worker 취소/root 검사 성공은 같은 API 구현이 불변이므로 재사용합니다. 이번 변경에서 새 tree gate fixture의 State 인수 부재 E0061 RED(exit 101)를 확인하고 수정했습니다. 자기 UUID 디렉터리의 기존 다섯 action 결과·cache/lock fixture가 통과했습니다.

## 검증

- `cargo test -p taide-runtime 닫힌_감독자는_트리_캐시_미스와_히트의_신규_입장을_거절한다 --lib`: 1건 통과.
- `cargo test -p taide --test tree_actions_runtime`: 5건 통과.
- `cargo clippy -p taide-runtime -p taide --lib --test tree_actions_runtime -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps --quiet`, `cargo test -p taide --lib typescript_바인딩을_생성한다`: exit 0. 실제 binding/manifest diff는 없습니다.
- `cargo fmt --all -- --check`, 새 history/QA와 갱신한 audit의 대상 Prettier check, `git diff --check`가 exit 0입니다. PROCESS/architecture 전체 포맷의 기존 baseline은 무관한 재작성 없이 유지했습니다. OS read stall·실제 앱/원격·search worker와 전체 M6 gate는 미완료입니다.
