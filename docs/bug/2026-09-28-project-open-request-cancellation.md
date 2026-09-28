# 프로젝트 열기 요청 취소 뒤 attach 미완료 경계

## 대상 파일

- `src-tauri/src/domain/project/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/remote_gateway.rs`

## 리포트

기존 Native `project_open`·`project_open_in_slot`·`project_group_open`은 caller future 안에서 session/live state 기록 뒤 capability attach를 기다렸습니다. 이 사이 요청 future가 취소되면 attach와 실패 rollback·완료 이벤트가 생략돼 저장된 상태와 실제 capability 소유가 어긋날 수 있고, 정상 root 종료도 남은 작업을 추적하지 않았습니다.

## 해결과 잔여

- 세 entry의 전체 runtime action을 supervisor의 취소되지 않는 operation으로 등록했습니다. 자기 fixture에서 attach 대기 중 요청 취소와 `stop_all` 뒤에도 root가 기다리고, 성공 시 이벤트·실패 시 rollback까지 완료됨을 확인했습니다.
- 실제 watcher·AppHandle·메뉴/원격 호출자의 중단, OS 종료 및 다른 project action의 요청 소유는 별도 검증이 필요합니다.
