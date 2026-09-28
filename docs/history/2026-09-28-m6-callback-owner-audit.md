# M6 callback·감독 경계 재대조

## 대상 파일

- `crates/taide-runtime/src/app_services.rs`, `exit_drain.rs`, `task_supervisor.rs`, `terminal_actions.rs`, `project_actions.rs`
- `src-tauri/src/lib.rs`, `domain/terminal/commands.rs`, `domain/remote/ws.rs`, `domain/ide/server.rs`
- `crates/taide-ide/src/store.rs`, `crates/taide-terminal/src/store.rs`

## 리포트

`AppServices::new`의 상태·포트 21개, Tauri 조립의 여섯 callback 묶음과 `TaskSupervisor`·`ExitDrain`을 현재 코드에 다시 대조했습니다. 정적 command 분류 F193/S0/A13/P0은 완료 판정이 아니라 배치 결과입니다. 제품의 직접 `tokio::spawn`·`tauri::async_runtime::spawn`·`spawn_blocking` 검색 결과는 감독자 내부를 제외하면 테스트 fixture에 있었고, IDE 연결의 하위 RPC 작업은 연결별 `JoinSet`이 보유합니다. 이 조사는 제품/실앱을 실행하지 않았으며 종료가 OS stall이나 모든 외부 자손까지 완료된다는 증거가 아닙니다.

## 상세 경계

| 조립 경계                                                       | 현재 소유·남길 위치                                                                                                                                                                       | 별도 완료 조건                                                                            |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `lib.rs::project_capabilities`                                  | 프로젝트 action은 runtime, 실제 watcher·PTY·lockfile 등 capability는 Tauri adapter입니다. watcher stop은 AppState tracker와 정상/직접 ExitDrain이 기다립니다.                             | 실제 OS watcher callback과 프로젝트 복원·닫기 GUI 회귀는 M6/M7 gate입니다.                |
| `lib.rs::settings_toggle_observers`                             | IDE→agent→remote observer 순서는 Tauri 조립에 남습니다. 설정·app·sync의 저장 후 callback은 등록된 완료 operation이 담당합니다.                                                            | 실제 IDE/agent/remote 시작·중지와 OS/네트워크 stall은 별도입니다.                         |
| `lib.rs::pty_spawn_env_provider`·`pty_session_observers`        | IDE readiness env await는 spawn 입장 이전이며 이후 shutdown 상태를 재검사합니다. 출력/input observer는 PTY reader 또는 write 경로에서 실행하고, 세션 worker는 TerminalStore가 추적합니다. | SIGHUP 무시 자손, OS/Windows wait 오류와 callback thread 종료는 M6-JP/JQ·실기 gate입니다. |
| `lib.rs::layout_tab_closed_observers`                           | 탭 닫힘의 IDE pending 해소와 PTY kill/retire는 Tauri 동기 adapter입니다. 제거된 세션 완료도 TerminalStore의 root 대기 대상입니다.                                                         | 실제 PTY 프로세스/GUI 탭 닫힘은 M6-JP/JQ·M7에서 확인합니다.                               |
| `lib.rs::remote_dispatch_port`                                  | Tauri gateway가 실제 AppHandle/채널을 제공하고 WebSocket 요청은 TaskSupervisor가 추적합니다. 세션 무효화 전에 받은 permit 대기 요청은 기존 계약상 이후에도 실행될 수 있습니다.            | 실제 원격 소켓·세션 무효화와 OS/네트워크 stall은 사용자 실기·별도 보안 판정 대상입니다.   |
| `lib.rs::ide_layout_actions`와 `ide::server::handle_connection` | 실제 탭/파일·WebSocket은 Tauri adapter입니다. 연결별 `JoinSet` 취소 시 diff/save 요청 owner가 자기 pending ID를 회수하고 서버 전체 중지는 별도 drain합니다.                               | 실제 WebSocket 단절·재접속과 renderer 탭 상태는 M7 실기입니다.                            |

`ExitDrain::wait_for_owned_resources`는 감독 task·watcher stop·설치 lease·일반 LSP·AI 요청·PTY 완료 목록을 정상/직접 Exit에서 기다립니다. 따라서 설치 JD~JG의 OS 오류/그룹 밖 자손·Windows, PTY JP/JQ의 SIGHUP 정책/OS 오류, 직접 native Exit의 실제 앱 동작은 여전히 M6 완료 조건입니다. M7의 workspace/frontend/GUI 전체 회귀와 M8의 native UI gate는 이 정적 대조에 포함하지 않습니다.

## 검증

- `rg -n 'tokio::spawn|tauri::async_runtime::spawn|spawn_blocking\(' src-tauri/src crates/taide-runtime/src --glob '*.rs'` 결과의 감독자 외 직접 호출 위치를 코드 범위에 대조했습니다. 테스트 fixture 호출과 감독자 구현을 제품 미등록 작업으로 집계하지 않았습니다.
- `src-tauri/src/lib.rs`의 callback 조립·등록, runtime의 terminal spawn 입장·ExitDrain 대기와 Tauri의 remote/IDE 연결 작업을 읽기 전용으로 확인했습니다.
- Rust/프론트엔드/실앱 검사는 이 문서 작업에서 실행하지 않았습니다. 제품 코드와 공개 IPC는 변경하지 않았습니다.
