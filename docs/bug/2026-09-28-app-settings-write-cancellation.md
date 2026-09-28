# 앱 설정 쓰기 요청 취소 뒤 guard 조기 해제 경계

## 대상 파일

- `crates/taide-runtime/src/app_actions.rs`
- `src-tauri/src/domain/app/commands.rs`, `src-tauri/src/remote_gateway.rs`

## 리포트

기존 두 앱 쓰기 entry는 caller future에서 mutation guard를 잡고 SettingsApplyPort를 await했습니다. 설정 저장 뒤 observer가 멈춘 동안 caller가 취소되면 guard가 먼저 풀리고 뒤따른 observer·이벤트가 생략될 수 있었으며 정상 종료도 이 작업을 추적하지 않았습니다.

## 해결과 잔여

- Native·원격 command가 전체 runtime action을 취소되지 않는 등록 operation으로 실행합니다. 자기 메모리/UUID 설정 fixture에서 요청 Drop 뒤에도 guard와 root 대기가 유지되고 observer 재개 뒤 이벤트가 발행됨을 확인했습니다.
- 이미 시작한 OS I/O나 callback을 강제 중단하지 않습니다. `sync_download`의 네트워크 후 로컬 적용은 별도 요청 취소/guard 설계가 필요합니다.
