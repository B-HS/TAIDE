# Agent probe의 runtime 정책과 blocking worker 소유

## 대상 파일

- `crates/taide-runtime/src/agent_probe.rs`, `src/lib.rs`, `tests/agent_probe.rs`
- `src-tauri/src/domain/agent/commands.rs`, `hooks.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

PID 조회의 Unix/Windows producer와 Claude 버전 producer 3개를 기존 등록 TaskSupervisor로 감독합니다. Unix 이름 캐시·Windows lazy process-tree port·emitter 버전/실패/OnceLock 정책은 runtime에 두고 실제 `ps`/sysinfo/CLI 실행은 Native adapter에 유지합니다. caller 취소 또는 CLI timeout 뒤에도 시작한 worker의 완료·버려진 결과 회수까지 정상 root가 기다립니다. 전체 poll/hook application 또는 실제 CLI 강제 종료를 완료한 변경은 아닙니다.

## 상세

1. runtime의 private `run_probe`는 operation lease를 입장 시 취득하고 worker와 공유합니다. 같은 감독자의 blocking handle을 등록·await하며 oneshot 결과는 resource-before-lease 필드 순서로 소유합니다. caller future가 취소되거나 CLI deadline이 지나도 이미 시작한 worker owner가 남고, send 실패로 버려진 결과를 Drop한 뒤 owner를 반납합니다. 닫힌/queued 취소는 Forbidden, worker panic은 Internal로 변환하며 CLI는 기존처럼 DevTty로 fallback합니다.
2. Unix empty PID·전부 캐시된 PID는 worker/OS factory 없이 반환합니다. 기존 AgentStore의 unresolved 정렬/중복 제거와 None 캐시를 그대로 쓰며 worker 성공 뒤 이름을 저장하고 원래 세션 순서로 probe를 조립합니다. 요청 취소 뒤 worker가 늦게 돌아오더라도 캐시는 변경하지 않습니다. Windows는 같은 descendant 탐지 함수를 lazy port로 받으며 캐시를 추가하지 않습니다.
3. Native의 static OnceLock·3초 상수·`claude --version`·stdout lossy UTF-8·버전 지원 판정을 유지합니다. 종료 status를 새로 검사하지 않습니다. 성공/invalid/I/O/panic/timeout은 기존 판정과 캐시 정책을 따릅니다. 늦게 도착한 supported 답은 timeout으로 고정된 DevTty를 덮지 않습니다. 동시 최초 요청이 cache get 이후 각각 probe하는 기존 race는 직렬화하지 않습니다.
4. list·poll·hook install은 같은 `app.state::<TaskSupervisor>()`를 지역 binding으로 보유해 lazy closure에 빌려줍니다. Native 첫 compile의 E0515는 일시적인 State 값을 반환 future가 참조했기 때문에 발생했으며 지역 binding으로 수명을 고쳤습니다. 비IPC reconcile도 같은 등록 감독자를 전달합니다. 비IPC `detect_agents_for_pids_blocking`의 Rust helper에는 감독자 인수를 추가했지만 공개 IPC signature/문서는 바꾸지 않았습니다.
5. [Tokio spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)의 started 작업은 abort로 끝나지 않는 계약과 [timeout](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html)의 future 취소, [OnceLock](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)의 get/get_or_init 계약을 확인했습니다. CLI deadline은 사용자 요청의 응답 시간 제한이며 OS child kill/wait 완료와 같은 뜻이 아닙니다. 새 dependency/registry/timeout 성공 처리는 없습니다.

## 검증 기록

새 API 부재 E0432 RED(exit 101) 뒤 메모리 worker/cache/timeout/root 8건이 통과했습니다. source 1건은 rustfmt의 CLI 호출 줄바꿈으로 실패해 해당 emitter 구간의 공백만 정규화하고 실제 같은 실행 API를 계속 확인합니다. State 수명 수정 뒤 source 1건만 재확인했습니다. 기존 미감독 timeout 패턴의 root 누락 재현 1건을 추가·통과해 새 서로 다른 검사는 10건입니다. 영문 대문자 test-name 경고는 snake-case 이름으로 수정했고 Unix-only test import를 cfg로 제한했습니다. 성공한 기존 동작 검사는 반복 합산하지 않습니다.

| 명령                                                                                                                              | 실제 결과                                                       |
| --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `cargo test -p taide-runtime --test agent_probe`                                                                                  | 제품 부재 E0432 RED; 구현 뒤 8건 성공·source 1건 실패, exit 101 |
| `cargo test -p taide-runtime --test agent_probe native는_같은_등록_감독자와_lazy_os_port를_모든_probe_호출에_주입한다 -- --exact` | 영향 source 1건, exit 0                                         |
| `cargo test -p taide-runtime --test agent_probe 기존_미감독_timeout_패턴은_worker가_남아도_root를_해제한다 -- --exact`            | 기존 패턴 재현 1건, exit 0                                      |
| `cargo test -p taide --test agent_actions_runtime --test agent_hook_actions_runtime`                                              | 기존 agent 9·hook 10건, exit 0                                  |
| `cargo test -p taide --test domain_boundaries`                                                                                    | 경계 3건, exit 0                                                |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                      | exit 0                                                          |
| `cargo clippy -p taide-runtime --test agent_probe -- -D warnings`                                                                 | 최종 test import에서 exit 0                                     |
| `cargo clippy -p taide --lib --test agent_actions_runtime --test agent_hook_actions_runtime -- -D warnings`                       | State 수명 수정 뒤 exit 0                                       |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                 | exit 0                                                          |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                  | exit 0                                                          |

총 서로 다른 검사 32건입니다. Fixture는 가짜 PID 상수·메모리 AgentStore/OnceLock·자기 blocking 채널만 씁니다. 같은 실제 ExitDrain에 빈 LSP/terminal stores를 넣어 worker gate가 닫힌 동안 readiness false를 확인하고 gate를 해제한 뒤 completed/ready true·추적 0을 확인합니다. RAII Release는 실패 경로에서도 자기 worker 채널을 해제합니다. 기존 미감독 패턴 재현도 자기 worker handle을 보존해 마지막에 await합니다. 실제 사용자 PID 조회·`ps`/sysinfo/Claude/osascript·home/hooks/keyring·listener/AppHandle/앱은 실행하지 않았습니다. Windows port는 메모리로 검사했으며 Windows target 빌드·실기는 미검증입니다.

읽기 전용 Bun 대조에서 Native 미변경 함수 23개와 공개 command cfg 포함 signature 11개가 byte 동일입니다. Native 전체 제품 source는 정확한 import/worker/caller 치환 밖에서 byte 동일이며 문서·다른 함수·OS resolver·poll diff/prune 순서도 유지합니다. hooks 전체 파일은 같은 감독자 인수 1개를 제외해 byte 동일입니다. 대조기의 첫 실패는 cfg별 두 agent_cli_install 함수가 있는 것을 고유 이름으로 가정한 오류였으며 실제 동일 함수 구간으로 대응해 수정했습니다.

Binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. 등록·모델·Cargo·생성 입력이 불변이므로 이전 실제 생성 1/IPC 7/normal graph 551줄·Tauri 0개 성공을 재사용하고 이번 건수에 합산하지 않습니다. 신규 history는 docs ignore를 해제한 기존 Prettier로 검사하며 PROCESS/architecture의 기존 전체 포맷 baseline은 재포맷하지 않습니다.

## 남은 경계

이번 소유는 probe worker·반환 결과/Unix 캐시 처리까지입니다. agent_list의 후속 활동 조립·poll 전체 diff/event/prune·hook 파일 write/reconcile/server admission·callback·전체 root/direct Exit/강제 bounded 종료는 별도 미완료입니다. CLI/OS call이 영구적으로 멈추면 정상 root도 기다립니다. timeout 뒤 CLI를 kill/reap하지 않으며 이를 종료 완료로 숨기지 않습니다. F193/S0/A13/P0은 command entry 분류일 뿐 전체 M6 완료가 아닙니다. M6/M7/M8와 locale 경계 선택을 미완료로 유지하며 전체 M6 완료 전 push/UI를 실행하지 않습니다.
