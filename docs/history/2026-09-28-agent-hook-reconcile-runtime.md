# 비IPC agent hook reconcile의 runtime 이전과 application owner

## 대상 파일

- `crates/taide-runtime/src/agent_hook_reconcile.rs`, `src/lib.rs`, `tests/agent_hook_reconcile.rs`, `tests/agent_probe.rs`
- `src-tauri/src/domain/agent/hooks.rs`, `commands.rs`, `src-tauri/tests/agent_hook_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

toggle/reconcile/uninstall과 프로젝트·사용자 hook 파일 정책을 runtime으로 이전했습니다. 같은 등록 TaskSupervisor의 단일 operation을 마지막 파일 적용 또는 disable cleanup 뒤 server stop까지 보유합니다. Native는 기존 공개 signature·root cleanup 재수출과 setup/observer/capability 순서를 유지하고 같은 state·lazy home/emitter/server/stop port만 주입합니다. listener/인증/connection transport와 hook payload application은 이번 단위에 이전하지 않았습니다.

## 상세

1. 동일 toggle은 바로 반환합니다. reconcile의 disabled 설정 및 닫힌 감독자의 입장 거절은 lazy home/emitter/server/stop을 실행하지 않습니다. 기존 정책에 종료 입장/owner만 더하고 shutdown 뒤 재검사·새 rollback·파일 worker·registry·dependency는 추가하지 않았습니다.
2. reconcile은 home→프로젝트 root/JSON snapshot→TAIDE 설치 여부 filter→lazy emitter→프로젝트 rewrite→사용자 인밴드 rewrite→server await→HTTP rewrite 순서를 유지합니다. 프로젝트 read lock은 emitter await 중 보유하지 않습니다. 설치된 프로젝트가 없으면 emitter factory를 실행하지 않습니다. 기존 owned-file helper의 invalid JSON·권한·비소유 파일·사용자 row 보존과 멱등 skip·로그·읽기 실패 skip을 유지합니다.
3. 프로젝트 JSON은 원래처럼 emitter await 전에 읽은 snapshot으로 씁니다. 그 사이 외부 파일 변경을 덮을 수 있는 기존 위험을 fixture로 확인했으며 구조 이전과 섞어 변경하지 않았습니다. server 실패/취소는 이미 완료한 프로젝트·인밴드 rewrite를 되돌리지 않고 아직 쓰지 않은 HTTP 파일은 보존합니다. disable은 프로젝트 cleanup→home 해석→사용자 cleanup→server stop 순서이며 stop callback에서도 owner가 남아 있습니다.
4. 같은 실제 ExitDrain은 emitter 또는 server await에 남은 reconcile operation을 기다립니다. 자기 caller를 abort하고 실제 완료를 await한 뒤에만 root ready가 true가 됩니다. 이 fixture는 실제 CLI/listener를 실행하지 않으므로 CLI worker kill/reap 또는 server 전체 admission을 증명하지 않습니다. 직전 probe worker 소유 증거는 입력이 같아 재사용합니다. [Tokio JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)의 abort 요청과 실제 완료 구분을 유지합니다.
5. Native의 공개 toggle Rustdoc와 세 wrapper signature 및 `remove_taide_hooks_from_roots` 경로를 유지합니다. 이전 private 정책 설명은 이 문서로 옮겼으며 새 공개 runtime API에는 영어 Rustdoc를 둡니다. 사용처가 사라진 Native 내부 파일 helper 재수출 7개는 제거하고 Native 기존 cfg(test)는 Core helper를 직접 import합니다. 공개 command의 입력/반환/문서/제품 body는 변경하지 않았습니다.

## 검증 기록

새 API 부재 E0432 RED(exit 101)를 확인했습니다. 구현 뒤 기존 owned-file 15건과 새 integration 8건이 통과했고, server await 취소/root 검사 1건을 추가해 그 검사만 실행했습니다. disabled/closed 검사에 public uninstall 입장 거절을 보강하고 해당 검사 1건만 재실행했으며 추가 건수로 합산하지 않습니다. 영향받는 기존 source 각 1건과 감독/조립 22건을 포함해 이번 실행의 서로 다른 검사는 48건입니다.

| 명령                                                                                                                                              | 실제 결과                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `cargo test -p taide-runtime --test agent_hook_reconcile`                                                                                         | 최초 E0432 RED; 구현 뒤 새 integration 8건, exit 0                                        |
| `cargo test -p taide-runtime --test agent_hook_reconcile --lib agent_hook_reconcile`                                                              | 이전 owned-file unit 15건, exit 0; integration은 filter로 0건이며 새 검사에 합산하지 않음 |
| `cargo test -p taide-runtime --test agent_hook_reconcile server_await_취소_전_root는_owner를_기다리고_이미_쓴_인밴드_파일은_유지한다 -- --exact`  | 추가 server await/root 1건, exit 0                                                        |
| `cargo test -p taide-runtime --test agent_hook_reconcile disabled_동일_toggle_닫힌_감독자는_lazy_port와_파일을_건드리지_않는다 -- --exact`        | 강화한 기존 검사 1건, exit 0; 재실행 중복 제외                                            |
| `cargo test -p taide --test agent_hook_actions_runtime 세_command의_native_port는_기존_provider를_주입하고_file_url_helper는_공유한다 -- --exact` | 기존 hook 배선 source 1건, exit 0                                                         |
| `cargo test -p taide-runtime --test agent_probe native는_같은_등록_감독자와_lazy_os_port를_모든_probe_호출에_주입한다 -- --exact`                 | 기존 probe 배선 source 1건, exit 0                                                        |
| `cargo test -p taide --test task_supervisor`                                                                                                      | 감독/조립 22건, exit 0                                                                    |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                      | 초기 test 이름의 대문자 HTTP 경고를 snake-case로 수정; 최종 exit 0                        |
| `cargo clippy -p taide --lib --test agent_hook_actions_runtime -- -D warnings`                                                                    | 초기 미사용 내부 재수출 7개를 제거; 수정 후 exit 0                                        |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                 | exit 0                                                                                    |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                  | exit 0                                                                                    |

Fixture는 자기 UUID 임시 경로와 메모리 AppState/프로젝트/store, 가짜 home/server/CLI 문자열만 사용합니다. 실제 사용자 home·파일·프로세스·CLI·AppHandle·앱·listener·인증·keyring은 실행하거나 읽지 않았습니다. 정상 root fixture는 자기 oneshot·caller handle만 취소하고 실제 완료를 await합니다. Native transport/payload는 source 보존 확인이며 transport 실기가 아닙니다.

읽기 전용 Bun 대조에서 Native 서버/transport/payload 함수 10개는 byte 동일하고 공개 wrapper signature 3개도 byte 동일했습니다. 이전 정책 body 9개는 정확한 named port/helper 치환 뒤 동일하며, 생성한 기존 owned-file 검사 15개와 실제 이동 결과를 대조했습니다. 이 비교는 Rustfmt 공백·선택적 trailing comma만 정규화하며 문자열 내용은 보존합니다. Native commands 전체 파일은 미사용 내부 재수출 제거와 cfg(test) import 치환 밖에서 byte 동일합니다. 대조 스크립트의 generic 함수명 탐색 및 Rustfmt trailing comma 차이는 검사의 추출/정규화를 수정했으며 제품 동작을 바꾸지 않았습니다.

실제 binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`와 manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. Cargo·모델·생성 입력·등록을 바꾸지 않았으므로 영향 없는 기존 실제 생성/IPC/normal graph·agent action/poll/probe/worker 성공을 재사용하며 이번 건수에 합산하지 않습니다. 신규 history만 기존 Prettier로 검사하고 PROCESS/architecture의 무관한 전체 포맷 baseline은 재포맷하지 않습니다.

## 남은 경계

stale JSON snapshot의 외부 수정 덮어쓰기·부분 파일 적용·동기 파일 I/O stall은 기존 위험으로 남습니다. server admission/중복 시작/transport/인증/connection·hook payload application·모든 action/AppState의 전체 입장 선형화·실제 foreground/Windows/GUI·CLI timeout 뒤 kill/reap·OS stall/직접 Exit/강제 bounded 종료는 미완료입니다. 정상 root owner 대기를 전체 취소/강제 회수 완료로 해석하지 않습니다. F193/S0/A13/P0, 미완료 M6/M7/M8·locale 결정과 전체 M6 완료 전 push/UI 금지를 유지합니다.
