# Agent hook server의 runtime 정책 이전과 시작 owner

## 대상 파일

- `crates/taide-runtime/src/agent_hook_server.rs`, `src/lib.rs`, `tests/agent_hook_server.rs`, `tests/agent_probe.rs`
- `src-tauri/src/domain/agent/hooks.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

Hook server의 cached 응답, uncached bind부터 accept 등록·shutdown 확인·store 게시까지, stop의 저장소 정리 뒤 handle abort를 runtime으로 이전했습니다. Native의 공개 시작/중지 signature와 실제 loopback TcpListener·UUID 토큰·accept/connection·인증/요청 파싱은 유지합니다. uncached start가 같은 등록 TaskSupervisor operation을 바인딩 이전부터 마지막 저장 결과까지 보유해 정상 root가 미완료 bind를 놓치지 않습니다. 이 이전은 실제 listener/인증 실기나 AppState·store의 완전한 선형화를 의미하지 않습니다.

## 상세

1. 기존 cached info가 있으면 새 입장이나 bind/start/shutdown port 없이 그대로 반환합니다. 정보가 없고 감독자가 닫혔으면 bind 전에 기존 supervisor-stopped 오류로 거절합니다. 입장한 요청의 bind 오류는 그대로 전달하고 accept 등록 실패는 같은 stopped 오류로 반환합니다. binding은 소비되며 등록되지 않은 listener가 남지 않습니다.
2. Native port는 원래처럼 `127.0.0.1:0`에 bind한 뒤 실제 local port와 새 UUID 토큰을 만든 다음 같은 감독자의 `agent-hooks-accept` 작업을 등록합니다. accept loop와 `agent-hooks-connection` 등록·`handle_connection`은 본문을 유지합니다. `is_shutting_down()` 검사는 accept 등록 다음이고 true면 후보를 abort한 뒤 원래 오류로 끝납니다.
3. store.set_server의 기존 정책은 동시 bind 때 첫 서버를 반환하고 뒤늦은 accept handle을 abort합니다. stop은 store.take_server로 저장 정보를 지우며 프로젝트 override도 함께 지운 뒤 반환된 핸들을 abort합니다. cached fast path는 종료 이후에도 원래처럼 저장 정보가 남았다면 캐시를 반환합니다. AppState shutdown check 직후 store 게시와 stop의 동시 실행 사이에 남은 경쟁은 이번 구조 이전에서 새로 잠그지 않았습니다.
4. synthetic bind await를 기다리는 caller를 취소하면 정상 ExitDrain은 그 caller의 실제 완료/operation Drop 전까지 ready가 되지 않습니다. synthetic binding의 Drop, 등록 실패와 shutdown true의 store 불변, 중복 후보 취소·stop override 정리를 메모리로 확인했습니다. 제품 worker를 새로 만들거나 실제 소켓을 연 테스트는 아닙니다. [Tokio JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)의 abort 요청과 실제 완료 구분을 유지합니다.

## 검증 기록

새 API 부재 E0432 RED(exit 101) 뒤 compile 단계에서 `HooksServerInfo`에 Debug가 없어 테스트의 `unwrap_err()`가 실패했습니다. 제품 타입을 불필요하게 바꾸지 않고 `.err().expect(...)`로 검사했습니다. 이후 메모리 동작 7건이 통과했고 source 검사 1건은 Rustfmt 줄바꿈을 놓쳐 `.begin_operation` 확인으로 수정한 뒤 해당 1건만 재실행해 통과했습니다. 영향받는 probe source 1건과 Native 감독 회귀 22건도 통과했습니다. 서로 다른 검사 총 31건입니다.

| 명령                                                                                                                                                             | 실제 결과                                       |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| `cargo test -p taide-runtime --test agent_hook_server`                                                                                                           | 최초 E0432 RED; 구현 뒤 7/8, source 줄바꿈 실패 |
| `cargo test -p taide-runtime --test agent_hook_server native는_loopback_bind_accept_connection을_유지하고_runtime에_같은_store_supervisor를_전달한다 -- --exact` | 수정한 source 1건, exit 0                       |
| `cargo test -p taide-runtime --test agent_probe native는_같은_등록_감독자와_lazy_os_port를_모든_probe_호출에_주입한다 -- --exact`                                | 1/1, exit 0                                     |
| `cargo test -p taide --test task_supervisor`                                                                                                                     | 22/22, exit 0                                   |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                                     | exit 0                                          |
| `cargo clippy -p taide --lib --test task_supervisor -- -D warnings`                                                                                              | exit 0                                          |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                                | exit 0                                          |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                                 | exit 0                                          |

테스트는 등록 TaskSupervisor·AgentHooksStore·자기 synthetic binding/handle/oneshot/ExitDrain만 사용합니다. 실제 user home·파일·프로세스·CLI·listener·AppHandle·앱·keyring은 사용하지 않습니다. Root fixture는 자기 caller를 abort하고 실제 join을 await한 뒤 ready를 확인합니다. 중복·stop fixture도 자기 등록 handle을 감독자 shutdown으로 완료시키며 다른 프로세스나 전역 자원을 종료하지 않습니다.

읽기 전용 대조에서 Native 공개 시작/중지 signature 2개와 서버 함수 뒤 나머지 제품 함수는 byte 동일했습니다. accept/connection loop는 공백을 제외해 동일하고 실제 bind·local_addr·UUID 토큰 경로도 Native에 남습니다. runtime lib.rs는 새 모듈 export 한 줄만 다릅니다. 모델/Cargo/등록·생성 입력·공개 IPC 및 실제 binding/manifest digest는 불변입니다. 기존 공개 hook action/reconcile/payload/poll/probe/worker·Phase 0 계약 성공은 영향 없는 부분만 재사용하고 이번 검사의 건수에 합산하지 않습니다.

## 남은 경계

cached 정보가 실제 accept 생존성을 보장하지 않는 기존 정책과 shutdown check→store 게시 사이의 경쟁, 직접 server start와 AppState의 전체 입장 선형화, 실제 연결 인증/timeout/HTTP parsing·listen/socket 실기, CLI kill/reap·OS stall·직접 Exit/강제 bounded 종료는 미완료입니다. 운영환경 사용자 파일/GUI·Windows도 확인하지 않았습니다. F193/S0/A13/P0와 전체 M6/M7/M8·전체 M6 완료 전 push/UI 금지를 유지합니다.
