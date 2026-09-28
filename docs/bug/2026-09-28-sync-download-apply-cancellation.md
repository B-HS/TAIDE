# sync download 적용 중 요청 취소 뒤 부분 완료 경계

## 대상 파일

- `crates/taide-runtime/src/sync_actions.rs`
- `src-tauri/src/domain/sync/commands.rs`, `src-tauri/src/remote_gateway.rs`

## 리포트

기존 `sync_download`는 원격 gist fetch 뒤 caller future가 mutation guard와 SettingsApplyPort를 소유했습니다. 설정 저장 뒤 observer를 await하는 동안 요청이 중단되면 guard가 먼저 풀리고 theme/locale 파일 적용과 SyncStateChanged가 생략될 수 있었습니다. root 종료도 이 로컬 적용 완료를 추적하지 않았습니다.

## 해결과 잔여

- fetch 이후 로컬 apply만 완료 보장 operation에서 실행합니다. 자기 fixture에서 callback 대기 중 요청 Drop 후에도 guard/root가 기다리고, 재개 뒤 설정·테마·언어·두 이벤트가 끝까지 적용됨을 확인했습니다.
- fetch 이전은 기존 취소 가능 경계입니다. 실제 네트워크 취소와 다른 sync action의 부분 적용/종료 소유는 별도 검증이 필요합니다.
