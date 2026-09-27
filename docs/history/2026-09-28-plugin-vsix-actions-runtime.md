# 플러그인·VSIX 공개 action의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/plugin_actions.rs`, `vsix_actions.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`
- `src-tauri/src/domain/{plugin,vsix}/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/tests/plugin_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

플러그인 5개·VSIX 2개 공개 action과 root의 VSIX commit/cache 정책을 Tauri 미의존 runtime으로 이전했습니다. 실제 AppHandle commit port는 같은 조립부에 남으며 기존 정책·공개 IPC는 유지했습니다.

## 상세

1. 명시적인 State 참조·서비스 alias·Tauri blocking→Tokio 치환과 포맷 외 7개 action body 불일치가 없습니다. 공개 시그니처 7개·Rustdoc 21줄은 불변이고 root commit body도 같은 정책입니다. 인접 project restore 선언은 변경하지 않았습니다.
2. 플러그인 install은 directory/archive stage 완료 뒤 mutation guard→commit→목록 재로드→cache 교체→ID 조회 순서입니다. VSIX import도 stage 뒤 guard 안에서 같은 commit port를 호출합니다. reload/uninstall/grammar/read-through 및 함수별 오류 정책은 유지했습니다.
3. 테스트는 자기 UUID 아래 manifest/grammar/ZIP/VSIX만 만들고 정리합니다. 원본 fixture 보존·중복/잘못된 uninstall·stage 실패가 guard보다 먼저 반환함·VSIX callback에서 guard가 잡혀 있음과 cache/임시 stage의 실제 상태를 확인했습니다. 사용자 파일·플러그인·실제 앱·네트워크는 실행하지 않았습니다.
4. 의존 변경은 이미 존재하는 local taide-vsix crate 연결과 lock edge 한 줄씩입니다. normal graph 532줄의 Tauri 패키지는 0개입니다. 신규 외부 의존성은 없습니다.

## 검증 기록

새 모듈 부재 E0432(exit 101)로 RED를 확인했습니다. 초안에서 root helper의 텍스트 범위를 잘못 잡아 인접 함수 선언이 누락됐고 fmt가 unexpected closing delimiter를 검출했습니다. 선언을 즉시 원복한 뒤 실제 diff에서 인접 함수 불변을 확인했습니다. 이 초안 컴파일 실패에서는 테스트가 실행되지 않았습니다.

root helper 이전으로 AppError/AppErrorKind import가 미사용이 되어 Tauri clippy가 exit 101이었으며, 해당 import만 제거하고 같은 clippy를 한 번 재실행해 exit 0을 확인했습니다. lint 예외나 정책 변경은 없습니다.

서로 다른 검사 22건이 통과했습니다.

| 명령                                                                                                                                                                        | 실제 결과                                       |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| `cargo test -p taide --test plugin_actions_runtime --test taide_plugin_extraction --test taide_vsix_extraction --test rust_native_phase0_contract --test domain_boundaries` | action 9·plugin 1·VSIX 1·IPC 7·도메인 3, exit 0 |
| `cargo test -p taide --lib tests::파일_git_ide_vsix는_조립부의_플러그인_포트를_사용한다`                                                                                    | root 포트 1, exit 0                             |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                                                | exit 0                                          |
| `cargo clippy -p taide --lib --test plugin_actions_runtime -- -D warnings`                                                                                                  | import 제거 뒤 exit 0                           |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                                           | exit 0                                          |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                                            | exit 0                                          |

새 이력 MD만 docs ignore를 해제한 Prettier로 검사합니다. 공개 시그니처/문서·model/registry 생성 입력이 불변이므로 이전 Specta 생성 성공은 재사용합니다. bindings digest는 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest는 `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`로 불변입니다. 이전 입력이 같은 검사를 이번 실행 수에 중복 합산하지 않습니다.

## 공식 근거와 남은 경계

기존 ZIP 2.4.2의 [ZipWriter](https://docs.rs/zip/2.4.2/zip/write/struct.ZipWriter.html)에서 start_file/write/finish의 계약을 확인했고 fixture도 finish 성공을 확인합니다. 기존 [Tokio spawn_blocking](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html)의 이미 시작한 작업을 abort할 수 없다는 경계를 유지합니다.

remote 이전 뒤 F139/S22/A13/P32에서 plugin/VSIX 7개(2 S + 5 P)를 반영하면 F146/S20/A13/P27입니다. 정적 entry 배치 수이며 전체 lifecycle 동등성 합격 수가 아닙니다.

stage의 raw PathBuf와 요청 Drop·취소·시작한 blocking worker·임시 stage 회수, 남은 application/비IPC callback/root admission/drain, M6 전체·M7 전체/실기·M8 native UI/Phase 0 gate는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
