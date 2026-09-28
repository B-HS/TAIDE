# IDE 연결 취소와 pending 응답 소유

## 대상 파일

- `src-tauri/src/domain/ide/server.rs`
- `crates/taide-ide/src/store.rs`
- `src-tauri/src/domain/ide/commands.rs`

## 리포트

`handle_connection`은 각 `tools/call`을 `JoinSet`에 넣고 연결 종료 때 `connection_tasks.shutdown().await`를 호출합니다. [Tokio JoinSet 계약](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html#method.shutdown)에 따라 이는 하위 작업을 abort한 뒤 종료를 기다립니다. `tool_open_diff`·`tool_save_document`는 각각 pending 항목을 저장한 뒤 응답 receiver를 await하지만, 항목 제거는 정상 timeout/실패 분기에서만 실행합니다. 이 시점에 하위 작업이 abort되면 해당 제거 분기에 도달하지 못합니다.

## 현재 경계

`IdeStore::take_shutdown_state`는 서버 전체 종료 시 모든 pending diff/save를 drain합니다. 이는 연결만 끊기고 IDE 서버가 계속 실행 중인 구간의 pending 잔류를 해소하지 않습니다. 이후 renderer가 같은 request ID에 응답하거나 서버가 종료될 때까지 저장소가 responder를 소유할 수 있습니다. 이 경로는 코드·공식 API 계약으로 확인했으며, 실제 WebSocket 단절이나 응답 지연 fixture로 재현·검증한 결과는 아직 없습니다.

## 다음 검증

사용자가 M6 체크리스트 확장을 승인하면, 메모리 pending store와 취소된 연결 작업 fixture로 diff/save 각각의 잔류를 RED로 확인하고 요청 소유자의 Drop에서 해당 ID만 회수하도록 수정합니다. 서버 종료의 전체 drain·정상 응답/timeout·공개 JSON-RPC wire는 유지합니다.
