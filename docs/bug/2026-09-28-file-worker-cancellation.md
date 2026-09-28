# 파일 blocking 작업 취소 뒤 guard 조기 해제

## 대상 파일

- `crates/taide-runtime/src/file_actions.rs`의 `file_save`, `file_copy`, `file_open`, `file_mirror_dirty`
- `src-tauri/src/domain/file/commands.rs`, `src-tauri/src/remote_gateway.rs`

## 리포트

기존 save/copy는 async caller가 mutation guard를 보유하고 `spawn_blocking` 결과를 기다렸습니다. 요청 future가 중단되면 이미 시작한 worker는 계속 실행되지만 guard는 먼저 해제됐습니다. copy가 완료돼도 await 뒤의 self-write 표시가 실행되지 않는 경로가 있었습니다. 네 blocking worker 모두 정상 root의 TaskSupervisor에 등록되지 않았습니다.

## 상세

- 두 mutation은 TaskSupervisor operation을 guard 대기 전에 시작하고 `OwnedMutexGuard`를 worker에 넘깁니다. worker 완료 전에는 요청 취소나 root 종료가 guard 소유를 잃지 않습니다.
- copy는 복사 성공과 self-write 표시를 같은 worker에서 순서대로 수행합니다. 실패 시 표시는 하지 않습니다.
- open과 dirty mirror도 같은 감독자의 결과형 blocking worker로 등록하며 open의 권한 확인→overlay callback 순서와 dirty mirror의 전역 mutation 비사용은 유지합니다.
- 실제 사용자 파일·앱은 실행하지 않았습니다. OS I/O가 끝나지 않는 작업의 강제 취소나 종료 시간 상한은 보장하지 않습니다.
