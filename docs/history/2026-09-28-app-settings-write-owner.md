# 앱 설정 쓰기의 요청 취소 후 완료 소유

## 대상 파일

- `src-tauri/src/domain/app/commands.rs`, `src-tauri/src/remote_gateway.rs`, `src-tauri/src/lib.rs`
- `src-tauri/tests/app_actions_runtime.rs`, `crates/taide-runtime/src/app_actions.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

`app_file_write`와 원격의 parsed 설정 전용 `apply_settings_file`은 기존 AppState mutation guard를 보유한 채 SettingsApplyPort의 저장·observer·이벤트를 기다립니다. 두 entry의 전체 runtime action을 기존 TaskSupervisor의 취소되지 않는 operation에서 실행해 요청 future가 중단돼도 guard와 callback 완료를 정상 root가 기다리도록 했습니다.

## 상세

1. Tauri command는 공유 AppState clone과 SettingsApplyPort 함수 포인터를 작업에 전달합니다. 원격 gateway도 같은 TaskSupervisor State를 넘기며 설정 target의 gated field strip은 작업 입장 전에 기존 순서로 유지합니다.
2. runtime `app_actions`의 설정 JSON parse, prompt 검증/저장, parsed 설정 apply 및 mutation guard 정책은 수정하지 않았습니다. `SettingsApplyPort`의 AppHandle callback은 adapter에 남습니다. 공개 IPC 인자·TypeScript binding은 불변입니다.
3. `sync_download`는 fetch 뒤 공유 callback을 호출하지만 네트워크와 설정/테마/언어 적용의 별도 원자성이 있어 이번 단위에 포함하지 않았습니다.

## 검증과 한계

새 wrapper source 기대 0/2 RED 뒤 Tauri 패키지 앱 action 7건, 조립부 1건, 실제 binding 생성 1건 및 관련 Clippy·fmt/diff가 통과했습니다. 자기 설정 파일의 observer 대기 fixture는 두 경로 모두 요청 취소 후 guard/root 대기와 마지막 이벤트를 확인했습니다. 조립부 source 검사의 오래된 sync 저장 위치 기대는 실제 runtime owner로 수정해 재검사했습니다.

실제 앱·사용자 설정·원격 세션은 실행하지 않았습니다. OS/main-thread callback 정지의 시간 상한, `sync_download`, 전체 M6-HK는 미완료입니다.
