# IDE 연결 취소와 pending 응답 소유

## 대상 파일

- `src-tauri/src/domain/ide/server.rs`
- `crates/taide-ide/src/store.rs`
- `src-tauri/src/domain/ide/commands.rs`

## 리포트

`handle_connection`은 각 `tools/call`을 `JoinSet`에 넣고 연결 종료 때 `connection_tasks.shutdown().await`를 호출합니다. [Tokio JoinSet 계약](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html#method.shutdown)에 따라 이는 하위 작업을 abort한 뒤 종료를 기다립니다. `tool_open_diff`·`tool_save_document`는 각각 pending 항목을 저장한 뒤 응답 receiver를 await하지만, 항목 제거는 정상 timeout/실패 분기에서만 실행합니다. 이 시점에 하위 작업이 abort되면 해당 제거 분기에 도달하지 못합니다.

## 수정 전 경계

`IdeStore::take_shutdown_state`는 서버 전체 종료 시 모든 pending diff/save를 drain합니다. 수정 전에는 연결만 끊기고 IDE 서버가 계속 실행 중인 구간의 pending 잔류를 해소하지 않았습니다. 이후 renderer가 같은 request ID에 응답하거나 서버가 종료될 때까지 저장소가 responder를 소유할 수 있었습니다. 이 경로는 코드·공식 API 계약으로 확인했으며, 실제 WebSocket 단절이나 응답 지연 fixture로 재현·검증하지는 않았습니다.

## 검증 계획과 잔여

메모리 pending store와 취소된 작업 fixture에서 diff/save 잔류를 RED로 확인하고 요청 소유자의 Drop에서 해당 ID만 회수합니다. 서버 종료의 전체 drain·정상 응답/timeout·공개 JSON-RPC wire를 유지하며, 실제 WebSocket 단절과 renderer 확인은 별도 실기 gate에 남깁니다.

## 수정 결과

`docs/PROCESS.md`의 M6-PF~PH 범위에서 자기 메모리 store의 취소 작업 2건이 각각 pending diff/save 잔류로 실패했습니다(exit 101). `IdeStore::insert_pending_diff_owned`·`insert_pending_save_owned`가 `PendingRequestOwner`를 반환하고, `tool_open_diff`·`tool_save_document`가 응답 대기 동안 이를 보유합니다. 연결 작업 취소로 owner가 Drop되면 자기 request ID만 제거합니다. 정상 응답이나 서버 전체 종료가 이미 회수한 항목은 제거하지 않습니다. 실제 WebSocket 단절과 렌더러 탭 상태는 아직 실기로 확인하지 않았습니다.
