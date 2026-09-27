# 원격 공개 action의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/remote_actions.rs`, `src/lib.rs`: 공개 action 5개
- `src-tauri/src/domain/remote/commands.rs`, `src-tauri/tests/remote_actions_runtime.rs`: 기존 adapter·문서 경로와 메모리 검사 8개
- `docs/architecture.md`, `docs/PROCESS.md`: 실제 배치와 잔여 서버 수명주기

## 리포트

status/link/password/revoke application 정책을 Tauri 미의존 runtime으로 이전했습니다. 기존 remote service/store·SecretStoreState를 사용하고 실제 HTTP/WS 서버·keyring·사용자 자격증명·파일·앱을 실행하지 않았습니다.

## 상세

1. 5개 action body는 State→참조와 AppHandle의 settings 접근→공유 AppState 참조의 명시 치환 외 차이가 없습니다. Tauri 공개 시그니처 5개·Rustdoc 27줄도 불변이며 비IPC bind/start/stop/toggle/password cache refresh 함수 5개 body는 원본과 같습니다.
2. status는 cache만 읽습니다. link는 running gate→일회 token 발급→settings host snapshot→기존 URL 형식 순서입니다. Tauri는 기존 조립에서 항상 등록하는 AppState를 runtime 호출 전에 참조로 꺼내며 실제 settings read는 token 뒤에 유지합니다. 등록 상태가 없는 잘못된 host 구성까지 새로 보장하는 API는 아닙니다.
3. password 설정은 trim/Unicode scalar 개수 확인→기존 salted hash→secret set 성공→cache true→sessions revoke/epoch 순서입니다. 해제는 secret delete 성공→cache false→revoke이며 실패하면 cache/session/epoch를 바꾸지 않습니다. hash/login/host/expiry 정책은 변경하지 않았습니다.
4. 메모리 SecretStore는 각 set/delete 호출 시 이전 cache·active session·epoch가 유지되는지 확인합니다. get은 panic fixture라 다섯 action이 keyring을 조회하지 않는 것도 확인합니다. password는 테스트용 생성 문자열이며 실제 사용자 자격증명은 없습니다.
5. link fixture는 자체 no-op Tokio handle로 store의 running 상태만 조립하고 끝에서 실제 완료를 await합니다. TCP listener·HTTP/WS·사용자 파일을 생성하지 않습니다. dependency 입력은 불변입니다.

## 검증 기록

새 runtime 모듈 부재 E0432(exit 101)로 RED를 확인한 뒤 이전했습니다. 초기 컴파일의 사용처가 사라진 import 2개는 실제 남은 참조를 확인해 정리했고 관련 clippy는 exit 0입니다. runtime 이동 때문에 service alias를 보통 빌드에서 쓰지 않지만 기존 공개 Rustdoc의 intra-doc link에 필요하므로 `#[cfg(doc)]`로 문서 생성에서만 가져옵니다. 검사기를 끄거나 공개 문구/생성 bindings를 바꾸지 않았습니다.

서로 다른 검사 19건이 통과했습니다.

| 명령                                                                                                                                                   | 실제 결과                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| `cargo test -p taide --test remote_actions_runtime --test taide_remote_service_extraction --test rust_native_phase0_contract --test domain_boundaries` | action 8·기존 service 1·IPC 7·도메인 3, exit 0                                      |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings`                                                                                           | exit 0                                                                              |
| `cargo clippy -p taide --lib --test remote_actions_runtime -- -D warnings`                                                                             | exit 0                                                                              |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`                                                                                      | exit 0                                                                              |
| `cargo doc -p taide --no-deps`                                                                                                                         | exit 0, 다른 불변 경계의 기존 문서 경고 10개; 전체 strict Tauri rustdoc 통과가 아님 |
| `cargo fmt --all -- --check`, `git diff --check`                                                                                                       | exit 0                                                                              |

Tauri 문서 경고는 Git의 StatusCache 1개와 LSP/project/sync/window private link 9개입니다. 해당 제품 파일은 이번 단위에서 HEAD와 불변입니다. remote 경고는 없고 생성 HTML의 `service::validate_and_trim_password`가 `../service/fn.validate_and_trim_password.html`의 실제 링크임을 확인했습니다. runtime strict와 이 선택적 문서 경로 확인을 전체 Tauri 문서 gate로 합치지 않습니다.

새 이력 MD만 docs ignore를 해제한 Prettier로 검사합니다. 비IPC 서버/TaskSupervisor·원래 remote store/service의 성공 결과와 dependency 입력이 같은 runtime Tauri 0개 graph를 재사용하며 이번 새 실행 수에 합산하지 않습니다. 실제 lifecycle body와 이벤트 검사 입력이 불변이어서 event 묶음은 재실행하지 않습니다.

공개 문서/시그니처·model/registry 생성 입력이 불변이므로 이전 Specta 생성 성공을 재사용합니다. 실제 bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`도 같습니다.

## 공식 근거와 남은 경계

[Rustdoc intra-doc link scope](https://doc.rust-lang.org/rustdoc/write-documentation/linking-to-items-by-name.html)와 [cfg(doc)](https://doc.rust-lang.org/rustdoc/advanced-features.html)의 문서 생성 전용 scope를 확인해 기존 서비스 링크를 보존했습니다.

IDE 이전 뒤 F134/S24/A13/P35에서 remote 5개(2 S + 3 P)를 반영하면 F139/S22/A13/P32입니다. 정적 entry 배치이며 실제 remote 접속/로그인/키링/shutdown 동등성 합격 수가 아닙니다.

비IPC refresh/bind/start/stop/toggle·HTTP/WS·session callback의 native-host application 분리와 root admission/drain, 나머지 M6 entry/nested worker/root, M7 전체/실기·M8 native UI와 Phase 0 gate는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
