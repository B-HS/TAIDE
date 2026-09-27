# Agent hook application의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/agent_hook_actions.rs`, `src/lib.rs`
- `crates/taide-agent/src/hook_files.rs`, `src/service.rs`, `src/lib.rs`
- `src-tauri/src/domain/agent/commands.rs`, `hooks.rs`, `tests/agent_hook_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

Agent hook status/install/uninstall 3개의 application 정책을 runtime으로 이전했습니다. 공통 파일 정책과 loopback URL 조립은 기존 의존성을 가진 taide-agent에 두며 Tauri의 비IPC reconcile도 같은 helper를 재수출로 소비합니다. 실제 home 조회·Claude emitter probe·CLI availability·서버 시작은 lazy host port로 유지합니다. 공개 IPC·기존 scope/shape·파일 소유·오류·권한 정책은 변경하지 않았습니다.

## 상세

1. scope 해석이 settings/project/native port보다 먼저 실행됩니다. install만 기존 agent_hooks_enabled gate를 확인하며 status/uninstall에는 새 gate나 mutation guard를 추가하지 않았습니다. project install은 emitter 완료 뒤 최신 JSON을 읽는 기존 순서를 유지합니다.
2. project/user JSON의 missing은 빈 문서이고 invalid JSON·다른 IO 오류는 그대로 전파합니다. 기존 사용자 row를 보존해 같은 service로 inject/remove하며 JSON atomic write는 기존 mode 또는 신규 0600을 유지합니다. OwnedFile은 비소유 파일을 설치로 덮거나 해제로 삭제하지 않으며 기존 preserving-mode atomic write를 사용합니다.
3. user in-band/OwnedFile은 project·emitter·CLI·서버 port를 호출하지 않습니다. HTTP shape는 home/path·기존 JSON 읽기·CLI gate·서버 시작·동일 URL/command/events/timeout 조립·write 순서를 유지합니다. CLI 부재와 서버 오류에서 기존 파일 bytes는 변경하지 않습니다.
4. AgentHookInstallPorts는 기존 callback을 늦게 호출하며 상태·감독기·managed State·의존성을 추가하지 않습니다. Tauri helper 경로와 hook server의 URL helper 경로는 재수출로 유지합니다. 공통 persist facade는 기존 taide-infra 함수의 재수출이므로 같은 atomic writer를 소비합니다.

## 검증 기록

새 runtime 모듈/API 부재 E0432 RED(exit 101)를 확인한 뒤 구현했습니다. 마지막 user install helper 구조 보존 수정 이후 새 10건이 다시 통과했으며 관련 runtime clippy를 다시 확인했습니다. 같은 입력의 기존 성공은 재사용하며 재실행 건수를 합산하지 않습니다.

서로 다른 기능 검사 27건이 통과했습니다.

| 명령                                                                                                                                                                                                                                                                    | 실제 결과                                                   |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `cargo test -p taide --test agent_hook_actions_runtime`                                                                                                                                                                                                                 | 새 application/file/source 10건, 최종 exit 0                |
| `cargo test -p taide --test agent_actions_runtime --test taide_agent_service_extraction --test domain_boundaries`                                                                                                                                                       | 기존 action 9·service 1·boundary 3건, exit 0                |
| `cargo test -p taide --lib domain::agent::commands::tests -- --skip domain::agent::commands::tests::프로세스_조회는_한_번의_ps_호출로_살아있는_pid를_해석한다 --skip domain::agent::commands::tests::에이전트가_아닌_pid도_이름_캐시에_남겨_다음_틱에_다시_묻지_않는다` | empty PID·JSON/mode 기존 4건, exit 0; 실제 ps 조회 2건 제외 |
| `cargo clippy -p taide-agent -p taide-runtime --all-targets -- -D warnings`                                                                                                                                                                                             | exit 0; 이후 변경 없는 agent 결과 재사용                    |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                                                                                                                                            | 최종 helper 수정 뒤 exit 0                                  |
| `cargo clippy -p taide --lib --test agent_hook_actions_runtime --test agent_actions_runtime --test taide_agent_service_extraction --test domain_boundaries -- -D warnings`                                                                                              | exit 0                                                      |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-agent -p taide-runtime --no-deps`                                                                                                                                                                                        | exit 0                                                      |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                                                                                                                                        | exit 0                                                      |

자기 UUID 임시 디렉터리의 가짜 home/project만 사용했습니다. fixture는 port panic으로 불필요한 native 호출 부재를 확인하고 실제 JSON/owned-file 읽기·쓰기로 사용자 row 보존·멱등·비소유 파일 보호·existing 0640/0644와 신규 0600을 대조했습니다. HTTP server info는 합성 값이며 listener·실제 credential·CLI·osascript·HTTP·사용자 home·앱·프로세스에는 접근하지 않았습니다. fixture Drop은 자기 UUID 디렉터리만 정리합니다.

공개 agent 명령 9종의 cfg 포함 signature/Rustdoc 11개와 남은 native 함수 24개의 body는 byte 동일합니다. 이동한 파일 helper 11개는 persist namespace 교체를 제외하고 공백 정규화 동일하며 URL body는 byte 동일합니다. 비IPC hooks 전체는 URL 재수출/import 정리를 제외하고 공백 정규화 동일합니다. Cargo·등록·remote/wire·bindings/manifest diff는 없습니다. 실제 binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`·manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`도 불변입니다. 같은 생성 입력/의존의 직전 실제 생성 1·IPC 7·runtime normal graph 543줄/Tauri 0개 성공을 재사용하며 이번 실행 건수에 더하지 않습니다.

## 남은 경계

신규 history는 `.prettierignore`의 docs 제외를 해제하는 `bunx --no-install prettier --ignore-path /dev/null`로 실제 포맷·검사합니다. 일반 docs check의 exit 0을 합격으로 해석하지 않습니다. PROCESS/architecture 전체 포맷 실패는 직전 단위에서 HEAD에도 같은 exit 1로 확인한 baseline이며 무관한 전체 문서 재포맷은 포함하지 않습니다.

3개 command entry를 P→F로 옮겨 정적 배치는 F184/S0/A13/P9입니다. 잔여 application entry는 project 4·sync 5이며 전체 M6 완료 판정이 아닙니다. 기존 동기 파일 IO·scope/shape 정책을 보존한 단위이고 supervisor admission·요청 Drop·probe/nested blocking worker의 완전 회수·실제 AppHandle 서버/콜백·비IPC reconcile lifecycle·직접 Exit·강제 bounded 종료·사용자 GUI 실기는 검증하지 않았습니다. 별도 locale 보안 결정과 M6/M7/M8는 미완료이며 M6 전체 완료 뒤 일반 push 조건을 유지합니다.
