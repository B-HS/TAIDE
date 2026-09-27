# Agent poll application의 runtime 이전과 전체 poll owner

## 대상 파일

- `crates/taide-runtime/src/agent_actions.rs`, `tests/agent_poll.rs`
- `src-tauri/src/domain/agent/commands.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

비IPC poll의 프로젝트 snapshot·foreground PID/probe·활동/diff·이벤트·signal/PID cache 정리를 runtime으로 이전했습니다. 같은 등록 TaskSupervisor의 operation lease를 첫 snapshot 이전부터 마지막 prune까지 보유합니다. Native는 기존 signature/setup tick을 유지하고 같은 state/stores/EventSink와 lazy OS port만 주입합니다. 시작한 probe worker의 소유는 직전 `agent_probe`와 결합하며 이 단위는 실제 GUI/OS 또는 모든 agent application 종료를 완료했다는 뜻이 아닙니다.

## 상세

1. 감독자가 닫혔으면 poll을 시작하지 않고 foreground/probe·활동/diff/event·prune도 건드리지 않습니다. 입장한 poll은 원래처럼 프로젝트 ID를 한 번 snapshot하며 read lock을 port 호출이나 await 중 보유하지 않습니다. HashMap iteration을 새로 정렬하거나 병렬화하지 않습니다. await 중 프로젝트가 제거되어도 snapshot ID를 처리하고 나중에 추가한 프로젝트는 다음 tick부터 처리하는 기존 정책을 유지합니다.
2. 각 프로젝트의 live PID는 probe await 전에 모읍니다. probe 오류는 해당 프로젝트의 diff/event를 건너뛰지만 그 PID는 이름 캐시 유지 대상에 포함됩니다. 실패한 프로젝트의 session은 valid-session에 추가하지 않으므로 마지막 prune에서 신호가 제거됩니다. 이전 diff cache는 유지합니다. 성공한 빈 probe는 이전 agent가 있었다면 empty 변경 이벤트를 발행합니다.
3. state helper가 만든 활동·blocked reason을 diff에 먼저 저장하고 변경이 있을 때만 같은 AgentStateChanged payload를 EventSink로 전달합니다. 같은 답의 다음 tick은 조용합니다. 모든 프로젝트가 끝나면 valid-session 신호→live PID 이름 순서로 정리하고 그 뒤 poll operation을 반납합니다. caller future가 probe 중 취소되면 원래처럼 이후 이벤트/prune을 수행하지 않습니다.
4. 정상 root는 단순 worker 완료가 아니라 poll의 post-await synchronous 이벤트 callback도 기다립니다. 실제 `agent_probe::probe_process_tree`와 결합한 메모리 fixture는 poll+probe lease+worker 추적 수 3을 확인하고 poll 취소 뒤 worker+공유 owner 2가 남는 것을 확인합니다. 정상 root는 그 worker gate를 해제해야 ready가 true가 됩니다. [Tokio JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)의 abort 요청/실제 완료 구분과 [spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)의 started 작업 취소 한계를 확인했습니다.
5. Native의 기존 setup ticker·Unix/Windows interval·같은 `agent-poll` 등록과 wrapper signature는 불변입니다. import 정리와 cfg(test)의 HashSet 위치만 조정했습니다. 새 registry/dependency·IPC·저장 파일·clock·타임아웃을 추가하지 않았습니다.

## 검증 기록

새 API 부재 E0432 RED(exit 101)를 확인하고 구현 뒤 새 poll 8건이 모두 통과했습니다. 실제 runtime probe와 결합한 취소/root 검사 1건을 추가하고 그 1건만 실행했습니다. 기존 성공은 재사용하며 서로 다른 새 검사 9건입니다. 영향받는 기존 native action 위임 source와 probe 배선 source 각 1건도 통과해 이번 실행의 서로 다른 검사는 11건입니다.

| 명령                                                                                                                              | 실제 결과                                            |
| --------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| `cargo test -p taide-runtime --test agent_poll`                                                                                   | 최초 E0432 RED; 구현 뒤 poll/source/root 8건, exit 0 |
| `cargo test -p taide-runtime --test agent_poll poll_취소_뒤에도_결합한_probe의_실제_worker가_root를_소유한다 -- --exact`          | 추가 결합 취소/root 1건, exit 0                      |
| `cargo test -p taide-runtime --test agent_probe native는_같은_등록_감독자와_lazy_os_port를_모든_probe_호출에_주입한다 -- --exact` | 기존 probe 배선 source 1건, exit 0                   |
| `cargo test -p taide --test agent_actions_runtime 세_command와_공유_helper는_runtime으로_위임한다 -- --exact`                     | 기존 action 위임 source 1건, exit 0                  |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                      | exit 0                                               |
| `cargo clippy -p taide-runtime --test agent_poll -- -D warnings`                                                                  | 최종 추가 검사에서 exit 0                            |
| `cargo clippy -p taide --lib --test agent_actions_runtime -- -D warnings`                                                         | exit 0                                               |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                 | exit 0                                               |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                  | exit 0                                               |

Fixture는 자기 UUID의 생성하지 않은 AppPaths·메모리 프로젝트/store·가짜 PID와 EventSink를 사용합니다. EventSink가 cache에 저장된 payload와 실제 받은 payload의 동일성을 검증합니다. 실패/empty/동일 tick·snapshot drift·signal/PID prune과 닫힌 입장을 확인합니다. 실제 공유 ExitDrain은 probe await 중 poll owner 및 probe 완료 뒤 막힌 이벤트 callback의 owner를 기다립니다. callback 검사는 자기 blocking thread의 Handle::block_on에서 poll을 실행하고 Release Drop으로 자기 채널을 해제하며 마지막에 request handle을 await합니다. 시작한 probe 결합 검사 역시 자기 채널만 사용합니다. 실제 사용자 프로세스·CLI/home/hooks/keyring/listener/AppHandle·앱 실행은 없습니다. 실제 Windows foreground/snapshot 실행과 target 빌드는 미검증입니다.

읽기 전용 Bun 대조에서 Native 전체 파일은 poll/import 치환 밖에서 byte 동일이며 기존 모든 public command cfg signature 11개도 byte 동일입니다. 기존 runtime 전체 source는 import 변경 밖에서 byte 동일입니다. 이전한 poll body는 foreground/probe/EventSink/참조의 named port 치환 밖에서 byte 동일입니다. setup·등록·모델·Cargo·생성 입력은 불변입니다.

실제 binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`와 manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. 영향 없는 기존 agent 동작 8/hook 10·probe 동작/worker/root 9·조립 감독·실제 생성/IPC/normal graph 성공은 기존 동일 입력의 증거를 재사용하고 이번 건수에 합산하지 않습니다. 신규 history만 docs ignore를 해제한 기존 Prettier로 검사하며 PROCESS/architecture의 전체 포맷 baseline은 무관하게 재포맷하지 않습니다.

## 남은 경계

이번 단위는 poll application의 감독자 입장과 끝까지의 owner입니다. AppState/모든 action의 전체 입장 선형화·agent_list 후속 조립·hook write/reconcile/server callback·실제 foreground/PID/GUI 실기·CLI timeout 뒤 kill/reap·OS stall/직접 Exit/강제 bounded 종료는 미완료입니다. 입장한 poll을 shutdown 뒤 새로 skip/recheck하는 정책은 추가하지 않았으며 시작한 worker가 영구 정지하면 정상 root도 기다립니다. F193/S0/A13/P0과 미완료 M6/M7/M8·locale 선택·M6 전체 완료 전 push/UI 금지를 유지합니다.
