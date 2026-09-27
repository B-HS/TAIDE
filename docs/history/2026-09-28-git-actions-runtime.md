# Git 공개 action의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/git_actions.rs`, `src/lib.rs`: 공개 action 41개·repo root·무효화/이벤트 helper와 synthetic 검사 4개
- `src-tauri/src/domain/git/commands.rs`, `src-tauri/tests/git_actions_runtime.rs`: 원래 IPC adapter·구독 경계와 신규 검사 5개
- `src-tauri/tests/platform_event_sink.rs`, `docs/architecture.md`, `docs/PROCESS.md`: 실제 이벤트 배선과 상태 기록

## 리포트

기존 41개 Git command는 extracted service/store를 사용하지만 repo root·status cache·mutation/repo lock·blocking 실행·이벤트 조립을 Tauri 안에 보유했습니다. 같은 application action을 native host에서 소비할 수 있도록 runtime으로 옮깁니다. Git service/store 정책·실제 사용자 저장소·remote·credential/hook·앱/GUI는 변경하거나 실행하지 않았습니다.

## 상세

1. 공개 action 41개와 repo root/cache/이벤트 helper를 이전했습니다. 모든 action body를 State/AppHandle→참조/port·Tauri blocking→Tokio 경로의 명시 치환 뒤 비교했고 포맷 외 불일치는 0입니다. Tauri 공개 시그니처 41개·공개 Rustdoc 136줄도 같습니다. 기존 model/service 타입·인수·반환·조건 분기는 유지합니다.
2. `git_status`는 perf span→최초 구독 callback→repo root→fresh/stale cache→blocking 계산→identity/generation finish 순서입니다. 실제 세 이벤트 구독과 OnceLock 1회 등록은 Tauri adapter가 유지합니다. diff의 plugin overlay callback은 루트 해석 뒤에만 호출합니다. 기존 perf shim은 같은 taide-infra 재수출입니다.
3. status/refs helper는 주입한 같은 GitStore의 cache를 먼저 무효화한 뒤 EventSink에 발행합니다. refs 변경도 branch/ahead/behind를 바꾸므로 status cache를 무효화합니다. commit/checkout/init 등의 status→refs, branch create의 refs→선택적 status, revert의 conflict 조건 등 각 함수의 기존 이벤트 순서는 그대로입니다.
4. mutation guard의 기존 취득 지점·await 수명과 push/fetch의 같은 repo 락, pull의 별도 mutation 정책은 유지합니다. 시작한 blocking worker를 감독자로 새로 감싸거나 요청 Drop 소유권을 바꾼 단위가 아닙니다. AppHandle 기반 구독/plugin 취득과 native 이벤트 발행 adapter만 Tauri에 남깁니다.
5. 기존 workspace taide-git·infra/model/runtime 의존만 사용하며 Cargo.toml/Cargo.lock은 불변입니다. 신규 fixture는 미개방 프로젝트·메모리 cache/sink·생성하지 않은 UUID 임시 경로만 소비합니다. 미개방 프로젝트의 push/pull/fetch 등은 service 전에 NotFound로 반환하므로 Git child/네트워크/사용자 저장소를 실행하지 않습니다.

## 검증 기록

신규 integration은 runtime module 부재 E0432(exit 101)로 먼저 실패했습니다. 초안 fixture의 존재하지 않는 DiffMode 이름 E0599는 실제 `WorkdirVsIndex`로 고쳐 E0432만 재확인했습니다. 이전 뒤 신규 5건·runtime helper 4건·기존 Tauri adapter 2건이 통과했습니다.

이벤트 묶음의 layout source 검사 1건은 Git 변경 전 HEAD에서도 같은 실패 조건이었습니다. root/layout 제품 파일과 실패한 검사 함수가 HEAD와 불변이고, HEAD root에 이전 직접 publish/finish 문자열이 없음을 대조했습니다. 이미 runtime으로 이전된 창 이동/보조 탭 복귀를 TauriEventSink로 위임하는 실제 경로와 runtime의 finish/publish를 검사하도록 고쳤습니다. 제품 layout 코드를 되돌리거나 무효화·발행 순서 검사를 삭제하지 않았습니다.

최종 결과는 서로 다른 50건 통과입니다. 이벤트 묶음의 최초 28건과 수정 뒤 같은 29건을 중복 합산하지 않습니다.

| 명령                                                                                                                                    | 실제 결과                                                                                       |
| --------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `cargo test -p taide-runtime --lib git_actions::tests`                                                                                  | helper 4건 통과, exit 0                                                                         |
| `cargo test -p taide --lib domain::git::commands::tests`                                                                                | 실제 구독/source·IPC 표면 2건 통과, exit 0                                                      |
| `cargo test -p taide --test git_actions_runtime --test platform_event_sink --test rust_native_phase0_contract --test domain_boundaries` | Git 5·도메인 3건은 통과; 기존 layout source 검사로 전체 exit 101, 뒤 IPC target은 실행되지 않음 |
| `cargo test -p taide --test platform_event_sink --test rust_native_phase0_contract`                                                     | 수정 뒤 이벤트 29·IPC 7건 통과, exit 0                                                          |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                            | exit 0                                                                                          |
| `cargo clippy -p taide --lib --test git_actions_runtime --test platform_event_sink -- -D warnings`                                      | production lib·변경 integration, exit 0                                                         |
| `cargo clippy -p taide --test platform_event_sink -- -D warnings`                                                                       | layout 검사 수정 뒤 해당 target만 재검사, exit 0                                                |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                       | exit 0                                                                                          |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                        | exit 0                                                                                          |

새 이력 문서는 docs ignore를 해제한 Prettier write/check로 검사합니다. 불변 GitStore 21건의 이전 성공은 재사용하며 이번 변경 후 새 실행 수로 합산하지 않습니다. Cargo.toml/lock 입력이 같아 직전 runtime normal graph의 Tauri 0개 결과도 재사용합니다. 공개 시그니처/문서·registry·bindings/manifest 생성 입력도 불변이므로 이전 실제 Specta 생성 성공을 재사용합니다. bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다.

## 공식 근거와 남은 경계

설치된 Tauri 2.11.5의 `async_runtime.rs`는 Tokio JoinHandle을 감싸고, `error.rs`의 JoinError는 transparent Display입니다. 기존 `.to_string()`→Internal 변환은 Tokio join 오류로 옮겨도 같은 경로입니다. [Tokio 1.53.1 spawn_blocking](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html)의 시작한 blocking 작업은 abort로 종료되지 않는 계약도 확인했습니다. 새 runtime action은 기존 runtime action들과 같이 Tokio context에서 실행합니다.

AI 이전 뒤 정적 entry 분류 F65/S32/A13/P96에서 Git 41개를 반영하면 F106/S32/A13/P55입니다. 이는 facade 배치 진척이며 실제 Git subprocess/libgit2 성공·취소/오류·hook·remote·cross-platform/GUI 동등성의 통과 수가 아닙니다.

요청 future Drop 때 시작한 blocking worker와 mutation/repo guard가 같은 수명을 보유하는지, 전체 shutdown admission/drain, 구독 callback 수명, 나머지 application entry와 M6 nested resource/root·M7 workspace/frontend/실기·M8 native UI·Phase 0는 계속 별도 gate입니다. M6 전체 완료 뒤 일반 push 조건은 유지합니다.
