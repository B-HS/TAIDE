# M6 현시점 조립·종료 경계 재대조

## 기준

`docs/history/2026-09-28-m6-command-body-census.md`의 F57/S35/A13/P101은 `8e818b4` 당시 상태입니다. 후속 action 이전 이력은 project lifecycle까지 F193/S0/A13/P0으로 기록됐고, 이번 프로젝트 열기·sync 업로드 변경은 action 배치가 아니라 요청 완료 소유를 보강했습니다. 따라서 F193을 현재 command entry의 정적 배치로만 취급하며 M6 완료나 실제 기능 동등성 증거로 쓰지 않습니다.

## 실제 소유·adapter 대조

| 경계          | 실제 파일·심볼                                                                                                                                                                        | 현시점 판정                                                                                                                                                                                                                                                                      |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 공유 조립     | `crates/taide-runtime/src/app_services.rs::AppServices::new`, `src-tauri/src/lib.rs` setup                                                                                            | AppServices 21개 필드와 Tauri State clone을 조립합니다. EventSink·PlatformServices의 실제 AppHandle 구현은 Tauri adapter에 남습니다.                                                                                                                                             |
| callback 조립 | `src-tauri/src/lib.rs`: `project_capabilities`, `settings_toggle_observers`, `pty_spawn_env_provider`, `pty_session_observers`, `layout_tab_closed_observers`, `remote_dispatch_port` | 실제 watcher/IDE/agent/remote/Channel/OS 처리는 host adapter입니다. 정책 action은 runtime에 있지만 callback의 동기 stall·요청 중단·종료 소유는 종류별로 판정해야 합니다.                                                                                                         |
| 작업 등록     | `crates/taide-runtime/src/task_supervisor.rs`: `run_nonabortable_result`, `spawn_blocking_transient_handle`, `shutdown`; `src-tauri/src/lib.rs` setup                                 | 일반 등록 task와 완료 operation은 정상 root가 실제 종료까지 기다립니다. 시작한 blocking 작업을 abort로 종료했다고 보지 않습니다. 제품의 직접 spawn 검색은 remote/IDE 테스트 fixture와 감독자 내부를 제외하면 TaskSupervisor 사용 경로였고, IDE 연결의 하위 작업은 JoinSet입니다. |
| 종료 조립     | `crates/taide-runtime/src/exit_drain.rs::wait_for_owned_resources`·`wait_for_direct_exit`, `src-tauri/src/lib.rs` RunEvent                                                            | 정상 ExitRequested는 supervisor·설치·일반 LSP·AI·PTY idle을 기다리고, 직접 Exit도 같은 자원을 동기 대기합니다. 등록되지 않은 OS callback이나 영구 stall의 bounded 완료는 보장하지 않습니다.                                                                                      |
| watcher       | `src-tauri/src/domain/project/commands.rs::attach_project_capabilities`·`restore_project_watchers`, `crates/taide-infra/src/watcher.rs::WatcherHandle`                                | build worker와 상위 project open 요청은 감독됩니다. live watcher는 AppState map의 handle Drop으로 멈추는 계약입니다. 실제 debouncer callback/thread의 중지 완료와 직접 Exit 순서는 synthetic/실앱에서 아직 확인하지 않았습니다.                                                  |

## 남은 M6 판정 순서

1. `M6-JD~JG`의 설치 child/reader·그룹 자손·OS wait 오류와 `M6-JP/JQ`의 PTY 부분 시작·wait 오류·SIGHUP 무시/자손·Windows 분기를 자기 자원 fixture로 더 검증합니다. 기존 성공은 각 항목의 부분 진척이며 전체 체크는 아닙니다.
2. live watcher Drop 뒤 callback/thread 완료를 확인하고, AppState shutdown→새 action/adapter 입장·설정 observer/remote/IDE/agent callback과 ExitDrain 사이의 경쟁을 경로별로 판정합니다. 이미 검증한 project open·settings/app/sync·검색·blocking 작업의 요청 취소 fixture는 재사용합니다.
3. 실제 macOS 앱에서 다중 창·프로젝트 복원/닫기·PTY/LSP/설치·원격/IDE·메뉴·직접 Exit를 사용자 소유 데이터가 아닌 전용 fixture로 실행합니다. OS stall/Windows는 성공으로 가정하지 않고 별도 증거 또는 명시적 미검증으로 남깁니다. M7 전체 회귀와 M8 native UI gate는 M6 증거에 합산하지 않습니다.

이번 조사는 읽기 전용 원천 대조이며 실제 watcher·GUI·키링·GitHub·사용자 프로세스는 실행하지 않았습니다. 현재 작업 트리의 제품 동작을 바꾸지 않으며, 체크된 개별 단위가 있더라도 M6-HK와 M6 전체는 미완료입니다.
