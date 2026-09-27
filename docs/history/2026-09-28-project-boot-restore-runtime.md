# Project 부팅 복원 정책의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/project_actions.rs`
- `src-tauri/src/domain/project/commands.rs`, `src/lib.rs`의 cfg(test) source assertion
- `src-tauri/tests/project_boot_restore_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

비IPC restore_state와 projects_pending_watcher_restore 2개 정책을 runtime으로 이전했습니다. 기존 native wrapper·동기 setup 순서·같은 공유 state와 서비스·저장 계약을 유지합니다. 실제 watcher queue/build/register·AppHandle callback·감독 조립은 변경하지 않았습니다. 공개 IPC·dependency·등록은 불변입니다.

## 상세

1. restore_state는 session 복원→project별 layout 로드→legacy chrome 승격→live layout 반영→layout lock 해제→dirty 표시/필요한 session 저장→live session/projects 반영→warning 병합→settings 로드 순서입니다. session 읽기 오류는 기존 live session/projects를 유지하고 warning을 반환하지만 settings 복원은 계속합니다.
2. chrome 승격은 live layout의 legacy flag를 지우고 dirty를 표시합니다. layout 파일을 즉시 재저장하지 않으며 session 저장 실패에도 live state를 반영하는 기존 부분 실패 정책을 보존합니다. 새 트랜잭션이나 rollback 보장을 추가하지 않습니다.
3. watcher 대상은 active→session 순서→map에만 남은 drift 순서이며 missing root는 제외합니다. map의 여러 drift 사이 순서를 새로 정렬하지 않습니다. setup은 기존 native 호출로 같은 state를 AppServices 조립 전에 복원하고 watcher snapshot을 얻습니다.
4. 이전 body 2개·남은 native body 28개·기존 runtime body 30개는 byte 동일합니다. source assertion은 실제 runtime policy를 읽도록 위치만 바꾸고 native 위임도 확인합니다. 실제 watcher의 동기 build/register port·blocking worker·event correction과 guard 범위는 그대로 남습니다.

## 검증 기록

새 runtime API 부재 E0425 7건으로 RED(exit 101)를 확인했습니다. 같은 실행의 fixture AppPaths Clone 부재 E0599는 제품 타입을 바꾸지 않고 같은 data_dir로 새 AppPaths를 만들어 수정했습니다. 검사 억제·의존성 추가·실제 앱 실행은 하지 않았습니다. 이번 단위의 서로 다른 검사 26건이 통과했으며 같은 입력의 재실행은 합산하지 않습니다.

| 명령                                                                                                         | 실제 결과                                                 |
| ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| `cargo test -p taide --test project_boot_restore_runtime`                                                    | 새 자기 파일/오류/선택/source 6건, exit 0                 |
| `cargo test -p taide --lib domain::project::commands::tests`                                                 | 기존 선택·group·source 등 11건, exit 0                    |
| `cargo test -p taide --lib tests::프로젝트_복원_워처는_조립부_포트로_build와_register를_분리한다 -- --exact` | 실제 조립 source 1건, exit 0                              |
| `cargo test -p taide --test session_restore`                                                                 | 기존 자기 파일 session/layout/slot/group 복원 8건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                 | exit 0                                                    |
| `cargo clippy -p taide --lib --test project_boot_restore_runtime --test session_restore -- -D warnings`      | exit 0                                                    |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                            | exit 0                                                    |
| `cargo fmt --all -- --check`, `git diff --check`                                                             | exit 0                                                    |

Fixture는 자기 UUID 디렉터리의 실제 session/project/layout/settings 파일만 사용합니다. 자기 session.json 경로를 디렉터리로 바꿔 read 오류를 재현하고 그때도 settings가 복원됨을 확인했습니다. missing root·drift 선택·빈 복원의 디렉터리 미생성·layout 원본 파일 불변과 dirty/session 승격을 검사했습니다. Drop은 자기 UUID 디렉터리만 정리하며 사용자 home·파일·실제 watcher·프로세스·keyring·네트워크·AppHandle·앱은 사용하지 않았습니다.

공개 생성 입력·등록·모델·Cargo는 변경하지 않았습니다. binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`와 manifest digest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 실제 대조에서 불변입니다. 같은 입력의 실제 생성/IPC/normal graph 성공은 재사용하고 이번 검사 건수에 합산하지 않습니다. 신규 history는 docs ignore를 해제한 Prettier로 검사하며 기존 PROCESS/architecture 전체 포맷 baseline을 무관하게 재포맷하지 않습니다.

## 남은 경계

비IPC 부팅 복원 정책 2개 이전은 command 분류 F193/S0/A13/P0이나 전체 M6 완료 상태를 바꾸지 않습니다. 실제 capability/watch restore의 nested blocking worker와 callback·flush·취소/guard admission·normal root 회수·OS 오류/직접 Exit·강제 bounded 종료·사용자 실기는 미완료입니다. chrome 승격 저장 실패를 별도로 주입하는 테스트 seam은 새로 추가하지 않았으며 byte 동일한 기존 정책을 보존합니다. locale 보안 선택과 M6/M7/M8는 미완료이고 M6 전체 완료 전 push/UI 금지를 유지합니다.
