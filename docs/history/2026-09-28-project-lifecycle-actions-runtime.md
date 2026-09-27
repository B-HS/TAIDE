# Project lifecycle application의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/project_actions.rs`
- `src-tauri/src/domain/project/commands.rs`, `tests/project_lifecycle_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

Project open/open-in-slot/close/group-open 4개의 application 정책과 group queue/member를 runtime으로 이전했습니다. ProjectLifecyclePort는 같은 native capability detect/attach·flush·detach를 주입하며 공유 AppState와 EventSink를 유지합니다. 공개 command 25개의 signature/Rustdoc와 기존 perf span·서비스·등록·저장/이벤트·오류 우선순위는 보존합니다. dependency·State·registry는 추가하지 않았습니다.

## 상세

1. open은 guard 아래 Core service의 canonical root 검증/detect·session 저장·live state 반영을 수행하고 guard 밖에서 attach를 기다립니다. 이미 열린 프로젝트는 detect/attach 없이 기존 활성화/slot 이벤트만 발행합니다. fresh attach 실패는 같은 runtime close로 되돌리고 rollback 실패가 발생해도 원래 attach 오류를 반환하는 기존 정책을 유지합니다.
2. open-in-slot은 정확히 하나의 path/projectId 선택·기존 project 해석·target/duplicate 검증을 fresh open보다 먼저 수행합니다. slot 배치/저장/state 반영 뒤 guard 밖 attach와 기존 이벤트 순서를 유지합니다. rollback은 새 project/slot을 닫고 기존 focus를 복원합니다.
3. close는 최초 NotFound 검사→guard 밖 flush→guard 아래 membership 재검사→저장/state 반영→detach→Closed/Activated/slots/list 순서입니다. 두 close가 flush에서 겹친 뒤 두 번째 close는 성공하되 detach·이벤트를 다시 실행하지 않습니다. 저장 실패는 live state·detach·이벤트를 바꾸지 않으며 기존 부분 저장 정책을 새 트랜잭션으로 바꾸지 않았습니다.
4. group은 기존 plan·record root 해석·skipped/warning·첫 실제 성공으로 활성화 이월·멤버별 shutdown 검사를 유지합니다. background 멤버는 기존 focus/slot을 변경하지 않습니다. 실제 native attach의 blocking build→guard→membership→commit, flush의 window/ticket/timeout handshake, restore/watch callback은 그대로 남깁니다.
5. Lifecycle trait는 기존 DownloadFileIo와 같은 `impl Future + Send` 계약이며 native adapter는 async method입니다. 앞선 sync 단위에서 확인한 [Rust Reference](https://doc.rust-lang.org/reference/types/impl-trait.html#return-position-impl-trait-in-traits-and-trait-implementations)의 같은 계약을 재사용합니다. 독립 module이나 신규 task/admission을 추가하지 않았습니다.

## 검증 기록

새 API 부재 E0432/E0425 RED(exit 101)를 확인한 뒤 구현했습니다. 새 검사 이름의 `EventSink`를 snake-case `event_sink`로 정리하고 이전 위치에 남은 private test-doc 3개를 제거했습니다. 검사 억제나 제품 계약 변경은 하지 않았습니다.

서로 다른 검사 42건의 성공을 사용하며 같은 입력의 성공과 영향 재검사를 중복 합산하지 않습니다.

| 명령                                                                                                                                                                                                                                                      | 실제 결과                                                   |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `cargo test -p taide --test project_lifecycle_actions_runtime`                                                                                                                                                                                            | 새 memory/file/race/source 13건, exit 0                     |
| `cargo test -p taide --test project_lifecycle_actions_runtime native_네_command는_같은_capability_flush_detach_adapter와_event_sink를_주입한다 -- --exact`                                                                                                | 이름/doc 정리 뒤 영향 source 1건, exit 0; 중복 합산 제외    |
| `cargo test -p taide --lib domain::project::commands::tests`                                                                                                                                                                                              | 기존 native source/restore/group/forget 11건, exit 0        |
| `cargo test -p taide --test project_actions_runtime --test taide_project_service_extraction --test taide_project_policies_extraction --test capability_symmetry --test domain_boundaries`                                                                 | action 9·service 1·정책 1·capability 4·boundary 3건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                                                                                                                              | exit 0                                                      |
| `cargo clippy -p taide --lib --test project_lifecycle_actions_runtime --test project_actions_runtime --test taide_project_service_extraction --test taide_project_policies_extraction --test capability_symmetry --test domain_boundaries -- -D warnings` | exit 0                                                      |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                                                                                                                         | exit 0                                                      |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                                                                                                                          | exit 0                                                      |

Fixture는 자기 UUID AppPaths/root와 실제 Core service·저장 파일·메모리 port만 사용했습니다. attach/flush를 실제 pending future로 만들어 guard 위치와 동시 close를 관찰했습니다. 자기 session.json 경로를 디렉터리로 바꿔 저장 실패와 attach rollback 실패를 재현했습니다. 실제 watcher·사용자 home/프로젝트/프로세스·window flush·AppHandle·앱·keyring·네트워크는 사용하지 않았습니다. Drop은 자기 UUID 디렉터리만 정리합니다.

공개 command signature/Rustdoc 25개·남은 native body 26개·기존 runtime body 24개는 byte 동일합니다. 이전 정책 body 6개는 named host-port 치환을 제외한 공백 정규화 동일입니다. 기존 source assertion 3개는 같은 조건을 실제 runtime body에서 검사하며 native 위임도 확인합니다. 등록/서비스/모델/Cargo/생성 입력은 불변이고 실제 binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`·manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`도 동일합니다. 변경 없는 실제 binding 생성 1·IPC 7·runtime normal graph 551줄/Tauri 0개는 직전 sync 성공을 재사용하며 이번 검사 건수에 합산하지 않았습니다.

## 남은 경계

신규 history는 docs ignore를 해제한 Prettier로 실제 검사합니다. PROCESS/architecture 전체 포맷 실패는 앞선 단위에서 변경 전 HEAD에도 동일한 exit 1로 확인한 baseline이며 무관한 전체 재포맷을 하지 않습니다.

4개 command entry를 P→F로 옮겨 정적 배치는 F193/S0/A13/P0입니다. 모든 command entry의 application 이전을 전체 M6 완료로 처리하지 않습니다. native capability worker·restore/watch callback·periodic/window flush 등 non-IPC application 정책과 cancellation·guard admission·supervisor/root 회수·OS 오류·직접 Exit/강제 bounded 종료는 별도 미완료입니다. 실제 NativeProjectLifecycle 실기는 검사하지 않았고 새로운 rollback/트랜잭션 보장도 추가하지 않았습니다. locale 보안 결정과 M6/M7/M8는 미완료이며 전체 M6 완료 후 일반 push 조건을 유지합니다.
