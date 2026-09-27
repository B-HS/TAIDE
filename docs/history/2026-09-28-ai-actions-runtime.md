# AI 공개 action의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/ai_actions.rs`, `Cargo.toml`, `src/lib.rs`: AI 8개 action·기존 입력 상한 helper/검사와 로컬 taide-ai 연결
- `src-tauri/src/domain/ai/commands.rs`, `src-tauri/tests/ai_actions_runtime.rs`: 기존 IPC adapter와 메모리 port characterization
- `Cargo.lock`, `docs/architecture.md`, `docs/PROCESS.md`: 기존 패키지 의존 관계와 실제 상태 기록

## 리포트

M6 command body 전수 대조의 제품 기준선 `8e818b4`에서 AI 5개 command는 설정·요청·취소·응답 조립을 Tauri 안에 보유했고, 나머지 3개도 service/store 얕은 위임이었습니다. 등록 목록이 있다는 사실과 공통 action이 완성됐다는 판정은 구분합니다. 이번 단위는 8개를 같은 runtime facade로 이전하며 제품 동작·요청 수명주기 정책을 새로 바꾸지 않습니다.

## 상세

1. `ai_token_status`, `ai_set_token`, `ai_clear_token`, `ai_list_models`, `ai_inline_complete`, `ai_inline_edit`, `ai_commit_message`, `ai_request_cancel`과 private byte-limit helper를 그대로 이전했습니다. 이동 전/후 9개 함수 body는 문자열 대조로 모두 일치합니다. Tauri 공개 시그니처 8개와 공개 Rustdoc 8줄도 같습니다.
2. prefix 32 KiB·suffix 16 KiB·selection 100 KiB·instruction 4 KiB·diff 64 KiB·recent commits 8 KiB는 UTF-8 byte 길이 기준입니다. 초과 입력은 begin/provider 전에 거절하고 잘라 보내지 않습니다. 이는 token budget이 아니며 이번에 새 상한을 정한 것도 아닙니다.
3. edit/commit의 provider/model 해석은 begin보다 먼저이고, 기존 settings snapshot·사용자 경로 기반 prompt 선택·비편향 `tokio::select!`·결과/오류를 반환하기 전 identity finish·원래 request ID 응답을 유지합니다. cancel은 같은 owner/request ID만 제거하고 이전 token의 늦은 finish는 새 항목을 지우지 않습니다.
4. runtime은 이미 추출된 로컬 `taide-ai`를 직접 사용합니다. lockfile의 기존 runtime 의존 목록 한 줄만 추가됐고 패키지 버전·외부 의존성·unsafe·IPC wire 변경은 없습니다. 일반 의존성 graph의 Tauri 패키지는 0개입니다.
5. 신규 fixture는 메모리 SecretStore와 생성하지 않은 UUID 임시 경로만 씁니다. OMLX URL 미설정 오류에서 provider HTTP 전에 반환하는 경로와 토큰 삭제/빈 OMLX token을 검사합니다. 실제 provider·키링·credential·사용자 prompt 파일·앱/GUI는 사용하지 않았습니다. token status의 loopback URL은 상태 판정에만 쓰고 HTTP를 호출하지 않습니다.

## 검증 기록

이전 Tauri byte-limit 검사 6건은 이동 전에 통과했습니다. 새 runtime 모듈이 없을 때 integration은 unresolved import E0432(exit 101)로 실패했고, 이전 뒤 동작 7건은 통과했습니다. adapter source 검사 1건은 공개 Rustdoc의 `request_store.begin()` 설명을 실행 코드로 오인해 실패했습니다. 문서 줄을 제외한 실제 source 검사로 고친 뒤 신규 8건이 모두 통과했습니다. 기존 동작 검사를 삭제하거나 완화하지 않았습니다.

최종 관련 동작 검사는 서로 다른 33건이 통과했습니다. 이동 전 동일 byte-limit 6건을 다시 합산하지 않습니다.

| 명령                                                                                                                                                                       | 실제 결과                                                                |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `cargo test -p taide-runtime --lib ai_`                                                                                                                                    | action helper 6·request store 6, 총 12건 통과, exit 0                    |
| `cargo test -p taide --test ai_actions_runtime --test ai_request_runtime_boundary --test app_services_runtime --test rust_native_phase0_contract --test domain_boundaries` | 신규 8·기존 타입 1·공유 상태 2·IPC 7·도메인 경계 3, 총 21건 통과, exit 0 |
| `cargo tree -p taide-runtime --edges normal --prefix none`                                                                                                                 | 일반 graph 518줄, taide-ai 연결 확인·Tauri 패키지 0, exit 0              |
| `cargo fmt --all -- --check`                                                                                                                                               | exit 0                                                                   |
| `git diff --check`                                                                                                                                                         | exit 0                                                                   |

`cargo clippy -p taide-runtime -p taide --all-targets -- -D warnings`와 `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`도 exit 0입니다. 새 이력 문서는 docs ignore를 해제한 Prettier write/check를 통과했습니다. PROCESS/architecture의 무관한 기존 내용을 재포맷하지 않았습니다.

bindings SHA-256은 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest는 `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`로 이전 결과와 같습니다. 공개 시그니처/문서와 생성 입력이 같아 이전 실제 Specta 생성 성공을 재사용했고 이번에 생성기를 다시 실행하지 않았습니다.

## 남은 경계

이전 baseline의 F 57·S 35·A 13·P 101은 과거 조사 시점의 분류로 보존합니다. 이번 8개 action 이전만 반영하면 F 65·S 32·A 13·P 96입니다. 이는 entry 배치의 정적 진척이며 runtime 자원 종료·전체 동등성의 합격 수가 아닙니다.

request future를 중간에 Drop하면 기존 수동 finish 전에 항목이 남을 수 있는 경계는 이번에 고치거나 통과로 처리하지 않았습니다. 동시 service/cancel readiness, 진행 중 HTTP의 Drop·앱 shutdown/admission/실제 종료·실제 credential/provider 응답은 별도 gate입니다. M6 callback/nested worker/root 전체, M7 workspace·frontend·사용자 실기, M8 native UI·Phase 0의 남은 조건과 M6 전체 완료 뒤 일반 push 조건은 유지합니다.
