# 트리 application action 5개 runtime 분리

상태: 이 변경 단위 자동 검증 완료. M6 전체 및 M7·M8은 미완료입니다.

## 대상과 보존한 정책

`crates/taide-runtime/src/tree_actions.rs`가 tree_rows·tree_toggle·tree_collapse_all·tree_reveal·tree_refresh와 기존 helper·unit 4개를 소유합니다. `src-tauri/src/domain/tree/commands.rs`는 기존 command 인수·응답 타입과 TreeStore 재수출, collapse 공개 문서, TreeToggle·TreeReveal perf span을 유지합니다. 의존성·권한·서비스 정책·wire는 변경하지 않았습니다.

조회 hit는 read lock으로 기존 확장 상태의 페이지를 반환하고 아무것도 쓰지 않습니다. miss는 잠금 밖에서 root-loaded 트리를 만든 뒤 write lock에서 프로젝트가 여전히 열려 있는지 재확인합니다. 닫혔다면 캐시를 되살리지 않고 로컬 페이지를 반환합니다. entry().or_insert는 동시에 삽입된 기존 entry를 우선하므로 조회가 다른 수정의 확장 상태나 다른 프로젝트를 덮어쓰지 않습니다. 수정은 같은 write lock에서 entry를 인플레이스 변경합니다. 이전 private helper 문서의 전체-map 되쓰기·lost update·종료 후 재삽입 방지 설명은 이 문서에 보존합니다.

read plan은 현재 snapshot에 대한 힌트이며 정확성의 전제가 아닙니다. planned 디렉터리는 blocking pool에서 먼저 읽고 수정 action은 그 뒤 전역 mutation guard와 cache write lock을 취득합니다. 빠진 디렉터리와 실패한 prefetch는 원본 서비스의 동기 read fallback을 유지합니다. 따라서 모든 디스크 I/O가 잠금 밖이라는 보장은 새로 만들지 않습니다. Tauri spawn_blocking을 이미 의존하는 Tokio spawn_blocking으로 바꿨으며 JoinError 문자열 변환은 유지했습니다. 시작된 blocking 작업의 강제 취소 정책은 추가하지 않았습니다.

## 실제 검증

- 변경 전 `cargo test -p taide-tree --quiet` 정책 26건과 `cargo test -p taide --lib domain::tree::commands::tests --quiet` unit 4건이 통과했습니다. 새 tree_actions_runtime은 모듈 부재 E0432(exit 101)로 먼저 실패했습니다. 기존 unit 본문은 import·포맷 외에 불변임을 비교했고 service는 수정하지 않아 26건의 성공 증거를 재사용합니다.
- `cargo test -p taide --test tree_actions_runtime --test tree_store_runtime_boundary --test taide_tree_extraction --quiet`: 새 action 5건·기존 공개 경계 2건 통과, exit 0입니다. 페이지/캐시·toggle/collapse/reveal/refresh, 없는 프로젝트, 전역 mutation 대기 중 조회, 먼저 시작한 수정의 프로젝트 종료 후 cache 부활 방지, adapter 위임과 perf 경계를 확인합니다. 첫 Pending poll을 prefetch 완료 증거로 해석하지 않습니다.
- `cargo test -p taide-runtime --quiet`: 51건 통과, exit 0입니다. 기존 47건에 이전한 트리 unit 4건이 추가됐습니다. `cargo test -p taide --test rust_native_phase0_contract --test domain_boundaries --quiet`: IPC 7건·도메인 경계 3건 통과, exit 0입니다. 변경 후 관련 검사는 총 68건이며 변경 전 service 26건은 별도 근거입니다.
- `cargo clippy -p taide-runtime -p taide --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`, `cargo fmt --all -- --check`, `git diff --check`는 exit 0입니다. normal runtime dependency graph에서 taide-tree를 소비하며 Tauri 의존은 없습니다. runtime 엄격 rustdoc 성공을 Tauri 전체 엄격 rustdoc 성공으로 확대하지 않습니다.

bindings·manifest diff는 없고 SHA-256 `0710cb30ffff17322796e655f2fb8c41979bbf032e2b8dacc6fe06c6e99f20e7`도 불변입니다. 공개 타입·문서를 유지해 bindings 생성은 반복하지 않았고 IPC 검사가 실제 원천과 digest를 대조했습니다.

UUID 임시 디렉터리와 synthetic 파일만 사용했습니다. 사용자 파일·설정·시크릿·키링·앱 실행에는 접근하지 않았습니다. 전체 workspace·frontend·GUI 실기는 이번 변경에서 실행하지 않았습니다. 남은 M 전체 목표를 유지하며 승인된 일반 push는 M6 전체 완료 후 수행합니다.
