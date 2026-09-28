# sync download의 fetch 이후 로컬 적용 소유

## 대상 파일

- `crates/taide-runtime/src/sync_actions.rs`
- `src-tauri/src/domain/sync/commands.rs`, `src-tauri/src/remote_gateway.rs`, `src-tauri/src/lib.rs`
- `src-tauri/tests/sync_actions_runtime.rs`, `src-tauri/tests/platform_event_sink.rs`

## 리포트

`sync_download`의 원격 fetch와 로컬 apply를 runtime의 `prepare_sync_download`·`apply_sync_download`로 분리했습니다. Native·원격 명령은 fetch가 끝난 뒤에만 TaskSupervisor의 취소되지 않는 operation으로 apply를 시작합니다. fetch 중 요청 취소는 기존처럼 가능하고, apply가 입장한 뒤 요청이 사라져도 설정 callback·테마/언어 파일 적용·SyncStateChanged까지 정상 root가 기다립니다.

## 상세

1. prepare는 기존 token/gist snapshot·lazy client fetch를 소유하고, opaque `PreparedSyncDownload`에 적용 결정에 필요한 값을 보관합니다. fetch 자체는 감독 operation에 등록하지 않습니다.
2. apply는 기존 guard 재취득 뒤 gist 변경→다른 sync 완료→conflict→payload parse/schema→SettingsApplyPort→theme/locale→SyncStateChanged 순서를 그대로 수행합니다. 기존 runtime `sync_download`는 같은 두 함수를 순서대로 호출해 기존 직접 소비자 계약을 유지합니다.
3. Tauri adapter는 fetch 뒤 AppState clone·SettingsApplyPort 함수 포인터·AppHandle을 완료 보장 작업에 넘기고 원격 gateway도 동일 TaskSupervisor State를 전달합니다. 공개 IPC와 생성 binding은 불변입니다.

## 검증과 잔여

Native split source 기대 실패(exit 101) 뒤 기존 sync 경계/오류 통합과 새 post-fetch 취소 fixture 등 19건, 조립부 source 1건, 이벤트 source 29건, 실제 binding 생성 1건이 통과했습니다. 관련 Clippy·strict runtime rustdoc·fmt/diff도 통과했습니다. 상세 결과는 [QA](../quality-assurance/2026-09-28-sync-download-apply-owner.md)에 기록합니다.

실제 GitHub/키링·앱은 실행하지 않았습니다. fetch 자체를 abort하는 실행 fixture와 `sync_connect`·`sync_upload`·`sync_disconnect`의 전체 요청 소유, OS/main-thread callback의 종료 시간 상한 및 M6 전체는 미완료입니다.
