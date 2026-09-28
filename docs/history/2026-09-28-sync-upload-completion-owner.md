# sync upload의 원격 결과 이후 완료 소유

## 대상 파일

- `src-tauri/src/domain/sync/commands.rs`, `src-tauri/src/remote_gateway.rs`
- `src-tauri/tests/sync_actions_runtime.rs`

## 리포트

Native·원격 `sync_upload`를 TaskSupervisor의 취소되지 않는 완료 operation으로 감쌌습니다. 기존 runtime sync action은 단일 정책 출처로 유지합니다. 최초 gist 생성 또는 기존 gist 갱신 중 요청이 중단돼도 원격 응답 뒤 guard·live 설정 재검증·bookkeeping 저장·SyncStateChanged까지 정상 root가 기다립니다.

## 상세

1. command는 AppState clone·SecretStore Arc·AppHandle을 operation에 소유시키고, 원격 gateway는 같은 TaskSupervisor State를 넘깁니다. 공개 IPC 인수/응답과 생성 binding은 그대로입니다.
2. 최초 create는 기존처럼 guard를 원격 왕복 동안 보유해 중복 gist 생성을 배제합니다. update는 guard 밖 원격 왕복 뒤 재취득·live gist 재검증·설정 보존을 유지합니다. 요청 취소 뒤에도 이미 시작된 업로드는 완료되므로 원격 쓰기를 독립적으로 중단하지 않습니다.
3. Tokio [JoinHandle 문서](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)는 대기 핸들을 버려도 작업이 계속된다고 명시합니다. 자체 TaskSupervisor의 완료 lease를 worker에 보유해 detached 작업을 정상 종료에서 추적합니다.

## 검증과 잔여

완료 소유 부재 source fixture의 RED(exit 101) 뒤 sync 통합 21건이 통과했습니다. 자기 메모리 gist에서 create/update 각각 원격 응답을 멈추고 요청을 취소했습니다. 정상 root가 대기하다 재개 후 설정 저장과 이벤트 완료를 확인했습니다. 상세 명령은 [QA](../quality-assurance/2026-09-28-sync-upload-completion-owner.md)에 기록합니다.

실제 GitHub·키링·AppHandle·GUI/OS 종료는 실행하지 않았습니다. `sync_connect`의 discovery와 `sync_status`의 fetch는 로컬 변경 전 취소 가능 경계로 남고, `sync_disconnect`의 guard 이후에는 await가 없습니다. 전체 M6·M7·M8와 push는 별도입니다.
