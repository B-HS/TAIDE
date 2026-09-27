# LSP lifecycle application의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/lsp_actions.rs`
- `src-tauri/src/domain/lsp/commands.rs`, `tests/lsp_lifecycle_actions_runtime.rs`, `tests/task_supervisor.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

LSP spawn·stop·restart의 application 정책을 runtime으로 이전했습니다. 같은 process registry를 한 번만 사용하며, 정상 root가 입장한 action의 guard 반납·publication 완료도 기다리도록 기존 TaskSupervisor operation을 사용합니다. Channel·UUID·실제 native process/callback은 Tauri adapter에 남깁니다.

## 상세

1. spawn의 guard→project→bundled server→owner별 reuse→root 참조/channel 교체→workspace 추가 알림 또는 ID/channel/entry 등록→epoch→process 생성→slot/Running 순서를 유지했습니다. 기존 notification write 오류는 그대로 생략하며 생성 오류는 먼저 등록한 entry를 제거하고 이벤트를 발행하지 않습니다.
2. stop은 일부 root/중복 참조/알 수 없는 root에서 기존 subscriber·세션을 유지하고 실제 root가 빠질 때만 removed notification을 보냅니다. 최종 root 또는 rootless stop은 owner를 제거하고 guard 안에서 entry를 unlink한 뒤 guard 밖에서 같은 protocol shutdown/Stopped를 실행합니다. rootless stop이 기존 root 목록 자체를 비우지 않는 정책도 유지합니다.
3. restart는 같은 entry/ID를 유지하고 비가드 shutdown 뒤 guard를 재취득합니다. 그 사이 entry가 삭제되면 NotFound로 반환하며 새 process factory를 실행하지 않습니다. 기존 epoch 증가·manual restart의 generation 유지·Stopped/Starting/Running 순서를 보존하고 factory 실패에서 Starting을 유지하는 기존 정책을 바꾸지 않았습니다.
4. LspActionContext는 기존 state/store/supervisor를 빌리며 LspSpawnPorts는 lazy sink·ID·bare process factory를 주입합니다. runtime의 LspStore.spawn_process가 application process를 단일 admission으로 강하게 소유합니다. Tauri의 기존 등록 wrapper는 자동 재시작만 소비하고 새 bare factory에는 registry 잠금이 없어 중첩 취득하지 않습니다. 기존 helper 경로는 runtime 재수출로 유지해 공개 Rustdoc 링크와 실제 raw channel unit이 같은 정책을 소비합니다.
5. 각 action은 기존 논리 gate 뒤에 같은 TaskSupervisor operation을 등록합니다. 닫힌 supervisor는 기존 LSP Forbidden 문구로 새 action을 거절합니다. project/unknown server와 missing session의 오류 우선순위는 유지합니다. guard는 operation의 첫 필드로 lease보다 먼저 Drop되며 stop/restart의 비가드 구간도 operation은 계속 보유합니다. 이는 supervisor 종료 admission 보강이며 제품 프로토콜 정책의 변경은 아닙니다.

## 검증 기록

새 API 부재 E0432/E0425 RED(exit 101)를 확인했습니다. generic fixture의 String 경계 추론·borrow 오류, process handle의 Debug 요구는 실제 타입에 맞게 수정했습니다. AppError 표시 접두사 2개의 기대값은 기존 `path not found`·`operation failed`로 맞췄고 제품 오류는 변경하지 않았습니다. Copy status의 불필요한 clone은 역참조로 수정했습니다.

서로 다른 검사 64건의 성공 근거를 사용합니다. 동일 구현의 성공 결과는 재사용하며 재실행 건수를 중복 합산하지 않습니다.

| 명령                                                                                                                                                                                                            | 실제 결과                                                          |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `cargo test -p taide --test lsp_lifecycle_actions_runtime`                                                                                                                                                      | 새 lifecycle/source 12건, exit 0                                   |
| `cargo test -p taide --test lsp_lifecycle_actions_runtime stop_요청_abort로_삭제된_session의_process도_정상_root가_드레인한다 -- --exact`                                                                       | 추가 stop abort/root 1건, exit 0                                   |
| `cargo test -p taide --test lsp_lifecycle_actions_runtime process`                                                                                                                                              | fixture cleanup/Copy 보완 뒤 영향 4건, exit 0                      |
| `cargo test -p taide --test lsp_actions_runtime --test taide_lsp_store_extraction --test taide_lsp_session_roots_extraction --test task_supervisor`                                                             | 기존 action 13·store 2·root 2·감독/배선 22건, exit 0               |
| `cargo test -p taide --lib domain::lsp::commands::tests`                                                                                                                                                        | 실제 memory raw channel·reuse/root·마스킹/URI 등 기존 12건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                                                                                    | exit 0                                                             |
| `cargo clippy -p taide --lib --test lsp_lifecycle_actions_runtime --test lsp_actions_runtime --test taide_lsp_store_extraction --test taide_lsp_session_roots_extraction --test task_supervisor -- -D warnings` | Copy 수정 뒤 최종 exit 0                                           |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                                                                               | exit 0                                                             |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                                                                                | exit 0                                                             |

자기 UUID cwd의 직접 `/bin/cat`은 framed protocol을 echo하는 통제된 fixture이며 language server가 아닙니다. workspace 추가/해제 payload를 실제 writer/reader로 대조하고 같은 ID의 process 교체·epoch·status 순서를 확인했습니다. 차단한 Running publication의 action/guard 때문에 정상 ExitDrain이 준비되지 않고 반환 뒤 실제 process 종료가 완료되는 것도 확인했습니다. stop caller abort로 unlink된 process는 strong registry가 유지해 정상 root에서 드레인합니다. fixture Drop은 자기 store 종료를 요청한 뒤 자기 UUID 디렉터리만 정리합니다. 사용자 파일·프로세스·프로필·시크릿·설치기·앱·네트워크에는 접근하지 않았습니다.

공개 LSP command 11개의 signature/Rustdoc와 native 정책 7개는 byte 동일하며 실제 process callback은 공백 제외 동일합니다. 등록/remote/wire·기존 8개 action·의존은 변경하지 않았습니다. binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`·manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. 같은 생성 입력/의존의 직전 실제 binding 생성 1·IPC 7·runtime normal graph 543줄/Tauri 0개 성공을 재사용하며 이번 실행 수에 더하지 않습니다.

## 남은 경계

신규 history는 `bunx --no-install prettier --ignore-path /dev/null`로 실제 포맷·검사합니다. `.prettierignore`가 docs 전체를 제외하므로 일반 호출의 exit 0은 문서 포맷 합격으로 해석하지 않습니다. PROCESS와 architecture의 전체 check는 exit 1이며 `git show HEAD:<해당 경로> | bunx --no-install prettier --ignore-path /dev/null --stdin-filepath <해당 경로> --check`도 각각 exit 1입니다. 변경 전 baseline과 같은 실패이므로 이 단위에 무관한 전체 문서 재포맷은 포함하지 않습니다.

3개 command entry를 P→F로 옮겨 정적 배치는 F181/S0/A13/P12입니다. application entry의 잔여는 project 4·agent 3·sync 5이며 이것은 전체 M6 완료 판정이 아닙니다. 초기 guard admission 대기는 operation으로 추적하지 않습니다. native 생성은 기존 동기 호출이고 callback/OS write를 강제로 중단하거나 종료를 bounded하게 보장하지 않습니다. protocol write 중 cancellation의 완전성·OS 실패/부분 시작·실제 AppHandle callback/자동 recovery·GUI·직접 native Exit·나머지 nested worker/root gate는 미검증입니다. caller abort 뒤 post-await Stopped/cache/event를 복구한다고 주장하지 않으며 실제 process 회수는 정상 root에서 확인한 범위입니다. locale 보안 선택·M6/M7/M8는 미완료이고 M6 전체 완료 뒤 일반 push 조건을 유지합니다.
