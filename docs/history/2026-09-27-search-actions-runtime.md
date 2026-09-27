# 검색 application action 4개 runtime 분리

상태: 이 변경 단위 자동 검증 완료. M6 전체 및 M7·M8은 미완료입니다.

## 대상과 변경

`crates/taide-runtime/src/search_actions.rs`가 search_run·search_cancel·search_replace·search_list_files와 UTF-8 경로 목록 helper를 소유합니다. `src-tauri/src/domain/search/commands.rs`는 기존 State·Channel·AppHandle 타입과 공개 인수/응답·SearchStore 재수출을 보존합니다. 사용하지 않는 AppHandle의 인수 이름만 _app으로 바뀌며 wire에는 포함되지 않습니다. Channel 전송의 기존 오류 무시 정책, perf span, debug 로그는 adapter에 남습니다.

run은 프로젝트 루트를 확인한 뒤 mutation guard에서 owner/session의 취소 identity를 시작하고 blocking 검색을 마친 뒤 같은 identity로 finish합니다. cancel도 기존 전역 mutation guard를 유지합니다. replace는 대상 목록을 blocking worker에서 확보하고 파일별로 같은 공유 AppState clone의 begin_mutation_blocking을 취득해 기존 replace_one_file을 실행합니다. Replaced 결과에만 self-write를 표시하고 NoMatch·Skipped와 skip report 상한은 기존처럼 집계합니다. 외부 explicit 경로는 기존 service의 root guard가 제외합니다. 해당 서비스 구현·패키지 버전·권한 정책은 변경하지 않았습니다.

runtime에는 이미 workspace에 있는 taide-search local path 의존만 추가했습니다. Cargo.lock의 runtime 의존 목록 한 줄을 동기화했습니다. [Tokio spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)의 Send·static·완료/취소 계약을 확인했으며 시작된 blocking 작업의 강제 취소 정책을 추가하지 않았습니다.

list는 기존 UTF-8 문자열 변환을 보존하고 비UTF-8 경로를 U+FFFD로 바꾸지 않고 제외합니다. Unix unit은 Tauri에서 runtime으로 옮겼습니다. APFS에서 잘못된 UTF-8 이름을 실제 생성하면 EILSEQ로 거절되므로 원본처럼 OsStr raw bytes를 사용한 synthetic fixture를 유지합니다. debug 목록 시간은 action 진입부터 프로젝트 조회·blocking walk 완료까지 측정하며 기존보다 프로젝트 조회도 포함합니다. perf slot과 IPC 결과에는 변화가 없습니다.

## 검증

- 변경 전 `cargo test -p taide-search --quiet`: 기존 정책 57건 통과. `cargo test -p taide --lib domain::search::commands::tests --quiet`: 기존 UTF-8 unit 1건 통과. service는 불변이므로 정책 성공 증거를 재사용합니다.
- 새 search_actions_runtime은 runtime module 부재 E0432(exit 101)로 먼저 실패했습니다. 첫 구현 실행은 3/4 통과, 치환 skip 경로만 /var와 /private/var 비교에서 실패했습니다. 기존 root guard가 반환하는 canonical path를 기대값으로 사용해 테스트만 수정했습니다. watcher self-write 검사도 추가한 최종 상태에서 4건이 통과했고 변경 파일만 from_app이 true, 불일치 파일·바이너리는 false임을 확인했습니다. 실패 뒤 수정한 해당 test target만 재실행했습니다.
- `cargo test -p taide-runtime --quiet`: 47건 통과. 기존 46건에 이전한 UTF-8 unit 1건이 추가됐고 Tauri lib의 같은 unit은 제거됐습니다.
- `cargo test -p taide --test search_actions_runtime --test search_runtime_boundary --test taide_search_extraction --quiet`: 새 action 4건·레지스트리 기존 경로 1건·서비스 기존 경로 1건 통과. `cargo test -p taide --test rust_native_phase0_contract --test domain_boundaries --quiet`: IPC 7건·도메인 경계 3건 통과. 변경 후 관련 검사 총 63건이며 변경 전 service 57건은 별도 재사용합니다. bindings 생성 1건은 별도입니다.
- `cargo clippy -p taide-runtime -p taide --all-targets -- -D warnings`, strict runtime rustdoc는 exit 0입니다. 테스트 기대값·self-write 검사 변경 뒤 `cargo clippy -p taide --test search_actions_runtime -- -D warnings`도 exit 0입니다. fmt·diff 검사와 normal dependency graph에 Tauri 부재를 확인했습니다. 이 검증을 전체 workspace·Tauri 전체 strict rustdoc의 성공으로 확대하지 않습니다.

bindings 생성 1건은 통과했고 실제 diff는 공개 search_list_files 문서의 Rust 경로 링크 한 줄뿐입니다. command/type/인수/응답은 불변입니다. 새 SHA-256 `0710cb30ffff17322796e655f2fb8c41979bbf032e2b8dacc6fe06c6e99f20e7`를 IPC manifest에 반영했습니다. bindings는 기존 formatter 제외, manifest는 기존 포맷을 유지해 digest 한 줄만 수정했습니다. 이 상태의 IPC 7건은 실제 source와 manifest/hash를 대조해 통과했습니다.

새 테스트는 UUID 임시 디렉터리와 synthetic 파일만 사용합니다. 사용자 파일·설정·시크릿·앱에는 접근하지 않았습니다. 전체 workspace·frontend·실제 다중 창 검색/취소 GUI는 이번 변경에서 실행하지 않았습니다. 남은 M 전체 목표를 유지하며 일반 push는 승인된 저장소·브랜치에 M6 전체 완료 후 수행합니다.
