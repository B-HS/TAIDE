# M6 worker 이전 후 application callback 경계 대조

## 대상과 상태

- `crates/taide-runtime/src/app_services.rs`, `project_build.rs`, `project_actions.rs`, `settings_actions.rs`, `app_actions.rs`, `sync_actions.rs`
- `src-tauri/src/lib.rs`, `domain/project/commands.rs`, `domain/settings/commands.rs`
- `docs/history/2026-09-28-m6-command-body-census.md`는 `8e818b4` 기준의 역사적 분류입니다. 뒤따른 runtime 이전을 반영한 현시점 잔여 개수는 아직 재집계하지 않았습니다.

## 확인한 소유 경계

| 경계 | 실제 심볼 | 현재 판정 |
| --- | --- | --- |
| 공유 상태 조립 | `AppServices::new`, `lib.rs` setup의 `app.manage` | 같은 clone을 Tauri State와 AppServices에 등록합니다. AppHandle을 갖는 실제 이벤트·OS·toolkit port는 Tauri에 남습니다. |
| 프로젝트 blocking build | `project_build::run_project_build`, `attach_project_capabilities`, `restore_project_watchers` | build worker와 결과 자원은 TaskSupervisor operation이 추적합니다. 상위 open 요청의 상태 기록부터 attach·실패 rollback까지 전체 future가 독립 소유되는지는 별도입니다. |
| 설정 적용과 observer | `settings_actions::apply_and_broadcast`, `settings_update`, `settings_set_theme`, `settings::commands::reconcile_integrations` | 설정 저장·live state 적용 뒤 IDE→agent→remote observer를 await하고 이벤트를 냅니다. 현재 action은 caller future가 소유하며 종료의 TaskSupervisor 등록은 확인되지 않습니다. |
| 설정을 소비하는 다른 진입점 | `app_actions::app_file_write`, `apply_settings_file`, `sync_actions::sync_download`, `SettingsApplyPort` | 같은 apply callback을 mutation guard 안에서 호출합니다. 설정만 별도 task로 옮기면 guard 재진입과 중간 상태 순서가 달라질 수 있어 함께 검증해야 합니다. |
| 종료 대기 | `lib.rs`의 `RunEvent::ExitRequested`/`Exit`, `ExitDrain` | 등록된 supervisor·설치·LSP·AI·PTY owner를 기다립니다. 임의 command future나 main-thread callback의 강제 완료·OS I/O 상한은 포함하지 않습니다. |

## 검증 전 위험과 다음 fixture

1. `project_actions::project_open`은 상태 기록 뒤 `ProjectLifecyclePort::attach_project_capabilities`를 await하고 오류 반환 때 `project_close`로 rollback합니다. 요청 자체가 중단되면 rollback 분기를 실행하지 않을 수 있습니다. 자기 프로젝트와 대기 가능한 가짜 port로 기록 뒤 abort와 결과 자원 정리를 먼저 재현해야 하며, 이미 감독된 build worker의 완료를 상위 open 완료로 오인하지 않습니다.
2. `settings_actions::apply_and_broadcast`는 저장·state 반영 뒤 주입 observer를 await합니다. 이 사이 caller Drop은 뒤따른 observer·SettingsChanged를 생략할 수 있습니다. 자기 임시 설정 경로와 대기 가능한 callback으로 중간 상태·종료 대기 fixture를 먼저 작성해야 합니다. `settings_set_theme`의 추가 ThemeChanged와 app/sync의 guard 보유 경로도 포함해야 합니다.
3. composition의 `project_capabilities`, `settings_toggle_observers`, `pty_spawn_env_provider`, `pty_session_observers`, `layout_tab_closed_observers`, `remote_dispatch_port`는 실제 AppHandle·도메인 연결을 제공하므로 전체를 runtime으로 복사할 대상이 아닙니다. 그 안의 상태/순서 정책만 개별 action 계약과 대조합니다.

이 문서는 실제 앱·사용자 프로젝트·설정·프로세스에 접근하지 않은 정적 대조입니다. 두 요청 취소 위험은 재현 전 가설이며 수정·M6-HK 완료·현시점 command 재집계 성공을 주장하지 않습니다.
