# 프로젝트 공개 action의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/project_actions.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`: action 21개와 snapshot helper 3개, 기존 로컬 project crate 연결
- `src-tauri/src/domain/project/commands.rs`, `src-tauri/tests/project_actions_runtime.rs`, `platform_event_sink.rs`: IPC adapter와 실제 상태·이벤트 경계 검사
- `docs/architecture.md`, `docs/PROCESS.md`: 현재 배치와 미완료 gate

## 리포트

프로젝트 25개 command 중 capability lifecycle인 open/open_in_slot/close/group_open 4개를 제외한 21개의 기존 application body를 Tauri 미의존 runtime으로 이전했습니다. 조회·활성화/정렬/display·그룹 CRUD·shell slot/chrome 정책은 native host도 공유할 수 있습니다. capability attach/detach·복원·flush·사용자 데이터·실제 앱/GUI는 변경하거나 실행하지 않았습니다.

## 상세

1. 21개 body는 State/AppHandle에서 참조/EventSink로 명시 치환한 결과와 포맷 외 차이가 없습니다. 전체 25개 Tauri 공개 시그니처·공개 Rustdoc 144줄도 불변입니다. 남긴 4개 lifecycle body는 원본과 같습니다.
2. snapshot helper 3개는 session read lock을 해제한 뒤 같은 payload를 발행합니다. Tauri lifecycle은 실제로 사용하는 list/slot helper에 위임하며 이동 뒤 사용처가 없는 groups wrapper는 제거했습니다.
3. activate/reorder/display는 기존 mutation guard를 발행까지 유지하고, 그룹 7개 변경·slot focus/resize/close·chrome은 기존 명시 drop 뒤 발행합니다. forget_recent의 scope 종료→list→그룹이 실제 바뀐 경우만 groups→removed/skipped 결과 순서도 유지합니다. 모든 함수를 일괄적으로 같은 guard 수명으로 바꾸지 않았습니다.
4. 새 fixture는 자기 UUID 임시 경로와 메모리 state/sink만 사용합니다. sink는 live snapshot·session read lock 해제·mutation guard 취득 가능 여부와 발행 시점의 디스크 session 일치를 확인합니다. 성공 activation/display와 두 slot focus/resize/close는 프로젝트 root나 capability를 실제로 만들지 않습니다.
5. 기존 workspace `taide-project` 의존 한 줄과 lock graph edge만 추가했습니다. project/model/service 정책·persisted schema·IPC registry/remote allow-deny·bindings는 불변입니다.

## 검증 기록

신규 integration은 runtime module 부재 E0432(exit 101)로 먼저 실패했습니다. adapter 초안의 multiline 인수 파싱 4곳은 Rust fmt 구문 오류로 발견해 고쳤습니다. 테스트 target 이름 오기는 실제 Cargo target 목록으로 바로잡았으며 이를 제품 실패나 검증 성공으로 계산하지 않습니다.

이전 뒤 첫 integration 7건과 이벤트 29·IPC 7·도메인 3·project service 추출 1건이 통과했습니다. 이후 성공 activation/display·slot 경로와 sink의 발행 시점 저장 검사를 보강한 최종 integration 9건이 통과했습니다. 이전 7건을 별도로 합산하지 않습니다.

기존 Tauri project 검사 11건 중 10건은 통과했고 clear-recent 결과 이벤트 source 검사 1건이 이전된 body를 찾지 못해 실패했습니다. adapter가 runtime으로 위임하는지와 runtime이 skipped_with_drafts를 그대로 발행하는지 검사하도록 바꿨으며 해당 1건을 재실행해 통과했습니다. 별도의 조건부 groups source 검사도 adapter/runtime 경로를 함께 검사합니다. 제품 이벤트 계약을 약화하거나 삭제하지 않았습니다.

최종 서로 다른 검사 60건이 통과했습니다.

| 명령                                                                                                                                                                                | 실제 결과                                                         |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `cargo test -p taide --test project_actions_runtime --test platform_event_sink --test taide_project_service_extraction --test rust_native_phase0_contract --test domain_boundaries` | 최초 action 7·event 29·추출 1·IPC 7·도메인 3 통과, exit 0         |
| `cargo test -p taide --test project_actions_runtime --lib domain::project::commands::tests`                                                                                         | 최초 10 통과·source 1 실패; 뒤 integration target은 실행되지 않음 |
| `cargo test -p taide --lib domain::project::commands::tests::project_forget_recent_은_초안_때문에_건너뛴_수를_이벤트로_알린다`                                                      | 수정한 해당 1건 통과, exit 0                                      |
| `cargo test -p taide --test project_actions_runtime`                                                                                                                                | 최종 action 9건 통과, exit 0                                      |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                                                        | exit 0                                                            |
| `cargo clippy -p taide --lib --test project_actions_runtime --test platform_event_sink --test taide_project_service_extraction -- -D warnings`                                      | exit 0                                                            |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                                                   | exit 0                                                            |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                                                    | exit 0                                                            |

새 이력 MD만 docs ignore를 해제한 Prettier로 검사합니다. runtime normal graph 523줄에 Tauri 패키지는 0개이고 기존 로컬 project edge를 확인했습니다. 공개 IPC 시그니처/문서와 model/registry가 불변이므로 이전 Specta 생성 성공을 재사용합니다. 실제 bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`도 같습니다.

## 남은 경계

Git 이전 뒤 F106/S32/A13/P55에 이번 21개(F 이전 5 S + 16 P)를 반영하면 F127/S27/A13/P39입니다. 이는 정적 command entry 배치이며 동작/자원 gate 합격 수가 아닙니다. 8e818b4에 고정한 과거 전수 조사 수치는 재작성하지 않습니다.

프로젝트 lifecycle 4개·capability build/commit/rollback·restore watcher·nested blocking과 root admission/drain·나머지 application entry, M6 전체 종료, M7 workspace/frontend/실기, M8 native UI와 Phase 0 동등성 gate는 미완료입니다. 실제 프로젝트 열기·사용자 파일 정리·앱 실행은 이번 검증 범위가 아닙니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
