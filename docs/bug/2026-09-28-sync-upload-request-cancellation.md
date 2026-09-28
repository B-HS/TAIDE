# sync upload 요청 취소 뒤 원격·로컬 불일치 경계

## 대상 파일

- `src-tauri/src/domain/sync/commands.rs`, `src-tauri/src/remote_gateway.rs`

## 리포트

기존 `sync_upload`는 caller future가 gist create/update와 후속 로컬 bookkeeping을 소유했습니다. 원격 요청 중 caller가 취소되면 서버에 원격 변경이 적용됐는지 확인할 수 없고, 성공 응답 이후에도 로컬 기록·이벤트 전에 중단될 수 있었습니다. 최초 create에서는 원격 gist만 만들어지고 로컬 id가 남지 않는 위험이 있습니다.

## 해결과 잔여

- Native·원격 entry의 전체 runtime action을 supervisor 완료 operation에 넣었습니다. 메모리 gist create/update 대기 중 요청 취소와 `stop_all` 뒤에도 정상 root가 기다리고, 응답 재개 뒤 설정·이벤트가 완료됨을 확인했습니다.
- 실제 HTTP 서버가 응답 없이 원격 쓰기만 성공하는 경우, 프로세스 강제 종료나 OS 네트워크 장애에서 원격·로컬 원자성은 보장하지 않습니다. 실제 GitHub·키링 실기도 남아 있습니다.
