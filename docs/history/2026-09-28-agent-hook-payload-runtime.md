# Agent hook payload application의 runtime 이전과 동기 callback owner

## 대상 파일

- `crates/taide-runtime/src/agent_actions.rs`, `tests/agent_hook_payload.rs`, `tests/agent_probe.rs`
- `src-tauri/src/domain/agent/hooks.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

decoded hook payload의 프로젝트 선택·override·활동/diff·이벤트 정책을 runtime으로 이전했습니다. Native의 기존 private signature는 같은 state/stores/EventSink/등록 TaskSupervisor를 전달하며 listener/인증/JSON decode/connection transport는 유지합니다. operation lease를 첫 policy gate 이전부터 마지막 동기 이벤트 callback 완료까지 보유합니다. 이 단위는 실제 HTTP transport나 모든 action의 종료를 완료했다는 뜻이 아닙니다.

## 상세

1. 감독자가 닫혔으면 override/diff/event를 건드리지 않습니다. 입장한 요청은 기존 managed-agent→event mapping→프로젝트 root snapshot→가장 긴 cwd match 순서를 유지합니다. HashMap의 같은 길이 tie/order와 lexical slash boundary를 새로 변경하지 않습니다. 실제 경로 canonicalize나 파일 접근은 없습니다.
2. 프로젝트가 정해지면 override를 먼저 저장합니다. 현재 agents가 비어 있어도 override는 남고 이벤트는 없습니다. 캐시에 다른 agent만 있으면 override만 갱신하며 그 agent의 diff/reason은 유지합니다. 같은 agent의 모든 세션은 활동을 변경하고 blocked_reason을 지운 뒤 diff를 저장하며 변경이 있을 때만 기존 AgentStateChanged wire를 발행합니다.
3. 동일 payload는 같은 활동/diff 이벤트를 반복하지 않습니다. payload 자체는 원래처럼 settings의 enabled 값을 다시 읽지 않으며 HTTP가 인증한 뒤 decode한 값만 전달하는 Native 순서를 유지합니다. 새로운 settings gate·shutdown 재검사·rollback·registry·dependency는 추가하지 않았습니다.
4. EventSink fixture는 callback에서 override와 diff가 이미 저장됐고 owner가 1개 남았으며 프로젝트 read guard는 해제된 것을 확인합니다. 같은 실제 ExitDrain은 막힌 synchronous callback 완료까지 ready를 false로 유지합니다. 자기 blocking fixture의 gate를 RAII Release로 해제하고 request handle을 실제 await한 뒤 root ready를 확인합니다. 제품에 blocking worker를 추가한 것이 아니라 synchronous callback의 operation 범위를 검사합니다. [Tokio JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)과 [spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)의 실제 완료/started 작업 취소 한계를 확인했습니다.
5. Native 기존 wrapper signature와 인증→JSON decode→payload 적용→200 응답, 403/400 실패 응답을 유지합니다. transport 함수에는 admission이나 새 동작을 추가하지 않았습니다. source 비교가 실제 listener/authentication 실기를 대신하지 않습니다.

## 검증 기록

새 API 부재 E0432 RED(exit 101) 뒤 새 memory/owner/root/source 검사 7건이 통과했습니다. 기존 probe 배선 source는 payload wrapper의 같은 감독자 binding을 포함하도록 실제 개수 3→4와 호출을 갱신했습니다. 해당 source 1건도 통과해 이번 실행의 서로 다른 검사는 8건입니다. runtime/Tauri clippy·최종 probe-target clippy·strict runtime rustdoc·fmt/diff는 exit 0이며 동일 입력의 성공은 반복하지 않습니다.

| 명령                                                                                                                              | 실제 결과                                                 |
| --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `cargo test -p taide-runtime --test agent_hook_payload`                                                                           | 최초 E0432 RED; 구현 뒤 새 memory/root/source 7건, exit 0 |
| `cargo test -p taide-runtime --test agent_probe native는_같은_등록_감독자와_lazy_os_port를_모든_probe_호출에_주입한다 -- --exact` | 기존 probe 배선 source 1건, exit 0                        |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                      | exit 0                                                    |
| `cargo clippy -p taide --lib -- -D warnings`                                                                                      | exit 0                                                    |
| `cargo clippy -p taide-runtime --test agent_probe -- -D warnings`                                                                 | exit 0                                                    |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                 | exit 0                                                    |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                  | exit 0                                                    |

Fixture는 생성하지 않은 자기 UUID 임시 AppPaths·메모리 프로젝트/stores·가짜 PID·자기 oneshot/채널/worker만 사용합니다. 사용자 home·파일·프로세스·CLI·listener·AppHandle·앱·keyring은 사용하지 않습니다. unknown agent/event·empty/unmatched/sibling cwd·닫힌 입장·nested longest root·같은 agent 여러 세션/다른 agent 보존·empty override·동일 payload·root callback 대기를 확인했습니다.

읽기 전용 Bun 대조에서 Native 잔여 함수 12개와 기존 runtime 전체 source는 byte 동일했습니다. Native 전체 source는 해당 wrapper/import 치환 밖에서 byte 동일하고 private wrapper signature도 byte 동일했습니다. 이전 payload body는 state/store binding 제거·EventSink named port·operation 추가 외 whitespace 정규화 뒤 동일합니다. 문자열은 정규화하지 않습니다. 실제 binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`와 manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. public commands·등록/생성 입력·Cargo·모델·bindings/manifest는 바뀌지 않았으므로 기존 실제 생성/IPC/normal graph 증거와 직전 hook reconcile/poll/probe/worker 검사 성공은 재사용하며 새 건수에 합산하지 않습니다.

## 남은 경계

실제 server admission/중복 시작/transport·인증/connection·전체 action/AppState 입장 선형화·프로젝트 snapshot drift·동기 callback 영구 stall·OS/GUI/Windows·CLI kill/reap·직접 Exit/강제 bounded 종료는 미완료입니다. 정상 root가 owner를 기다리는 것을 callback 강제 취소 완료로 해석하지 않습니다. F193/S0/A13/P0·미완료 M6/M7/M8와 전체 M6 완료 전 push/UI 금지를 유지합니다.
