# 직접 설정 action의 요청 취소 후 완료 소유

## 대상 파일

- `crates/taide-runtime/src/task_supervisor.rs`
- `src-tauri/src/domain/settings/commands.rs`, `src-tauri/src/remote_gateway.rs`
- `src-tauri/tests/settings_actions_runtime.rs`, `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

직접 `settings_update`와 `settings_set_theme`는 공통 TaskSupervisor에 operation을 먼저 등록하고, caller가 기다리는 별도 async 작업에서 기존 runtime action을 끝까지 실행합니다. 요청 future가 중단되어도 작업 핸들 Drop은 작업 자체를 취소하지 않고, 앱 종료의 `stop_all`도 이 operation을 abort하지 않습니다. 정상 root는 observer와 이벤트 완료를 기다립니다.

## 상세

1. `run_nonabortable_result`는 입장 전에 닫힌 감독자를 거절하고, 입장한 작업의 실제 future에 operation lease를 둡니다. 정상 결과와 오류는 caller에게 전달하며 요청만 사라지면 결과는 버리고 작업은 계속됩니다.
2. Native·원격 명령은 동일 AppState clone과 AppHandle을 작업에 넘깁니다. `settings_actions`의 mutation guard, 저장→상태→IDE/agent/remote observer→SettingsChanged와 테마의 ThemeChanged 순서는 수정하지 않았습니다. 공개 IPC 인수와 생성 TypeScript binding은 불변입니다.
3. `SettingsApplyPort`를 통하는 `app_file_write`·`apply_settings_file`·`sync_download`는 caller가 mutation guard를 보유합니다. 이 세 경로에 이번 완료 소유 wrapper를 그대로 적용하지 않았으며 독립된 취소/guard 설계가 필요합니다.

## 검증과 한계

새 API 부재 E0599를 선행 확인한 뒤 자기 설정 fixture에서 저장 후 멈춘 observer 사이 caller abort·root 대기·재개 후 이벤트 완료를 확인했습니다. 감독자 11건·설정 integration 6건, 설정 이벤트 source 2건, 실제 binding 생성과 관련 정적 검사가 통과했습니다. 전체 `platform_event_sink`의 다른 도메인 source-scan 실패 5건은 [QA](../quality-assurance/2026-09-28-settings-direct-action-owner.md)에 별도 기록했습니다.

실제 GUI·OS observer 정지·직접 Exit 메인 스레드 교착과 전체 M6-HK는 검증되지 않았습니다. 완료까지 기다리는 정책은 시간 상한을 제공하지 않습니다.
