# M6 agent action 입장과 정상 종료 대기 조사

## 대상 파일

- `crates/taide-runtime/src/agent_actions.rs`, `agent_probe.rs`, `task_supervisor.rs`, `state.rs`, `app_services.rs`
- `src-tauri/src/domain/agent/commands.rs`, `src-tauri/src/remote_gateway.rs`, `src-tauri/src/lib.rs`

## 리포트

Hook reconcile·payload·공개 hook action·server start의 최근 소유권 이전과 별개로, `agent_list`와 `agent_release_marker`의 전체 호출 수명은 등록 TaskSupervisor의 입장/정상 종료 대기에 포함되지 않습니다. 이는 실제 프로세스 잔존을 재현한 결과가 아니라 코드 경계의 정적 관찰입니다. M6-HK는 계속 미완료입니다.

## 실제 배선과 미완료 경계

1. `AppServices::new`의 한 TaskSupervisor를 Tauri State에도 clone으로 등록하고, `agent_list`는 그 State를 PID probe callback에 전달합니다. `agent_probe::run_probe`는 blocking worker와 owner를 감독하므로 실행한 probe worker의 완료는 root가 기다립니다. 그러나 `agent_actions::agent_list` 자체에는 `begin_operation`이 없으며 프로젝트 gate→foreground PID→probe await→detected 조립→응답 전체를 소유하지 않습니다. PID가 없거나 cache hit이면 probe worker도 만들지 않습니다. 따라서 probe owner 완료를 공개 list action 완료로 대신할 수 없습니다.
2. `agent_release_marker`는 `AppState::begin_mutation`으로 직렬화하지만 이 lock은 `TaskSupervisor` 입장/완료 추적이 아닙니다. 검증 뒤 동기 `remove_file`과 marker set 정리 사이의 호출 수명을 root가 기다린다는 근거가 없습니다. Exit 경로는 `cleanup_all_wait_markers`를 별도로 실행하므로 순서 경쟁도 synthetic fixture로 확인해야 합니다. 실제 사용자 marker·파일은 읽거나 제거하지 않았습니다.
3. `agent_pending_external_opens`는 AgentStore queue를 동기로 drain하고 remote gateway에서는 기존 SharedSingletonStateRace 정책으로 거부됩니다. 목록/marker와 달리 비동기 대기·blocking worker가 없어 같은 수명 위험이라고 단정하지 않습니다. shutdown 이후 queue 소비 허용 여부는 별도 정책 결정이 필요합니다.
4. Tauri `ExitRequested`/`Exit`는 AppState·설치/AI·terminal/LSP shutdown 뒤 marker cleanup과 hook server stop을 호출합니다. 정상 `ExitRequested`는 `ExitDrain`으로 등록 감독자·설치/프로세스/terminal 완료를 기다리지만 직접 `Exit`는 감독자 `stop_all`과 설치 idle 대기만 수행합니다. 두 경로를 동등한 drain으로 기록하지 않습니다.

## 후속 합격 조건

- list의 empty/cache/실제 synthetic probe 지연·요청 취소·정상 root를 분리해 전체 application owner의 필요성을 실패 검사로 확인합니다. 동일 store/supervisor를 Native와 remote 호출에 전달하고 기존 project gate/조회/오류/응답 순서와 IPC를 유지해야 합니다.
- marker는 자기 임시 경로만 사용해 mutation lock 대기·삭제 구간과 종료 cleanup 경합을 재현합니다. 이미 시작한 동기 파일 작업을 abort로 완료 처리하지 않습니다.
- 실제 앱·사용자 파일/프로세스·원격 서버·GUI 실기, 직접 Exit 및 OS stall은 별도 gate입니다. 이 문서는 정적 조사이며 구현·동작 검증 통과가 아닙니다.
