# M6 runtime blocking worker 종료 소유 경계

## 대상 파일

- `crates/taide-runtime/src/file_actions.rs`, `search_actions.rs`, `tree_actions.rs`, `search_store.rs`, `task_supervisor.rs`, `state.rs`
- `src-tauri/src/domain/file/commands.rs`, `domain/search/commands.rs`, `domain/tree/commands.rs`, `remote_gateway.rs`
- `docs/PROCESS.md`의 M6-HK와 file worker 후속 항목

## 리포트

조사 당시 Native `src-tauri/src` 제품 코드에서 직접 async/blocking spawn 검색 결과는 테스트 fixture 범위였지만, runtime application에는 caller future만 await하는 직접 `spawn_blocking` 9개가 있었습니다. caller가 중단되어도 이미 시작한 blocking worker는 계속 실행될 수 있으므로 정상 root의 `TaskSupervisor.shutdown`은 이를 추적하지 못했습니다. 이는 command body가 runtime으로 이전됐는지와 별개인 자원 소유 경계입니다.

## 상세

1. `file_actions.rs`: `file_open`, `file_save`, `file_copy`, `file_mirror_dirty`의 worker 4개가 미등록입니다. save/copy의 mutation guard는 caller가 보유해 요청 Drop 시 worker보다 먼저 풀릴 수 있습니다. copy의 self-write 표시는 await 뒤 caller에서만 실행돼 실제 복사가 끝나도 취소된 요청에는 남지 않을 수 있습니다. Tauri file command를 거쳐 `remote_gateway.rs`에서도 같은 경로를 호출합니다.
2. `search_actions.rs`: `search_run`, replace target scan, replace per-file worker, `search_list_files`의 worker 4개가 미등록입니다. replace의 guard는 per-file worker 안에 있어 해당 파일 처리 중에는 유지되지만 root가 worker를 추적하지 않습니다. run은 caller 중단 시 `SearchStore::finish`를 건너뛰며 현재 token/세션 항목을 남길 수 있습니다.
3. `tree_actions.rs`: `prefetch_listings`의 read-only worker 1개가 미등록이고 다섯 공개 tree action이 공유합니다. caller 중단 뒤 결과는 버려져도 파일 시스템 조회는 계속될 수 있습니다.
4. `TaskSupervisor`에는 실제 완료까지 추적하는 `run_blocking_result`와 작업 전체를 소유하는 operation lease가 이미 있습니다. `AppState`에는 blocking worker로 이동 가능한 `begin_owned_mutation`이 있습니다. 별도 의존성 추가나 새로운 전역 상태 없이 이 경계를 사용할 수 있습니다.

## 남은 판정

- file의 guard/worker owner 4개는 [후속 수리](2026-09-28-file-worker-ownership.md)로 완료했습니다. 현재 직접 `spawn_blocking`은 search 4·tree 1개이며 독립 작업으로 검증합니다. 추가 대상은 외부 요청 중단·정상 root·OS I/O stall을 구분합니다.
- 실제 사용자 파일·프로세스·앱을 실행하거나 읽지 않았습니다. 이 정적 조사만으로 M6-HK 전체나 M6 완료를 주장하지 않습니다.
