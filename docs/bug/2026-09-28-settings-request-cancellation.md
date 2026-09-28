# 직접 설정 요청 취소 뒤 observer·이벤트 누락 경계

## 대상 파일

- `crates/taide-runtime/src/settings_actions.rs`, `task_supervisor.rs`
- `src-tauri/src/domain/settings/commands.rs`, `src-tauri/src/remote_gateway.rs`

## 리포트

설정 action은 파일 저장과 live state 갱신 뒤 외부 통합 observer를 await하고 마지막에 이벤트를 발행합니다. 기존 직접 명령 future는 이 전체 순서를 caller가 소유했으므로 저장 뒤 caller가 중단되면 observer와 이벤트 후반이 실행되지 않을 수 있었고 정상 종료가 이 command를 추적하지 않았습니다.

## 해결과 잔여

- 직접 두 명령은 입장한 전체 action을 취소되지 않는 TaskSupervisor operation으로 실행합니다. 메모리 fixture는 observer가 정지한 뒤 요청이 중단되어도 root가 기다리고, 재개 뒤 이벤트가 발행됨을 확인했습니다.
- 이미 시작한 작업의 callback이나 OS I/O를 강제로 중단하지 않습니다. `SettingsApplyPort`를 공유하는 app/sync 경로의 취소·guard 소유는 별도로 남깁니다.
