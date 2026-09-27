# 나머지 테마·언어 공개 action의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/theme_actions.rs`, `src/locale_actions.rs`
- `src-tauri/src/domain/theme/commands.rs`, `src/domain/locale/commands.rs`
- `src-tauri/tests/appearance_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`, `docs/bug/2026-09-28-locale-path-boundary.md`

## 리포트

현재 selector를 제외한 theme 4·locale 2개 공개 action을 기존 runtime 모듈로 이전했습니다. 기존 await 없는 service 호출은 동기 action으로 제공하고 공개 Tauri async signature와 current selector는 유지합니다.

## 상세

1. 이전 body 6개는 포맷 외 불일치 0이며 전체 공개 command 8개 시그니처·문서 입력은 불변입니다. 기존 current selector·service·의존성·guard/event/설정 쓰기 정책을 변경하지 않았습니다.
2. theme은 builtin 보호·식별자 검증·같은 save/load/delete와 오류를 유지합니다. locale은 builtin/user pack·missing/malformed 오류·current selector의 별도 fallback을 유지합니다. 직접 locale_get의 missing은 current selector의 fallback과 달리 NotFound입니다.
3. 자기 UUID fixture와 builtin만 사용했습니다. 새 테스트는 저장/조회/목록/삭제·검증 실패의 파일 무변경·언어 pack 오류와 기존 경로 이탈 동작을 확인합니다. 사용자 appearance 파일·OS 설정·앱·네트워크는 실행하지 않았습니다.
4. 기존 locale load/exists의 검증 없는 path join을 발견했고 자기 fixture의 locales 디렉터리 밖 pack이 `../fixture-outside`로 조회됨을 확인했습니다. 이 동작은 보안 합격이 아니며 별도 버그 문서에 원천·재현·위험/미실행을 기록했습니다. 사용자에게 경로 검증 추가와 기존 동작 유지 중 방향을 질문했고 답변 전에는 service 정책을 변경하지 않습니다.

## 검증 기록

새 action 부재 E0425(exit 101)로 RED를 확인했습니다. 경로 재현 fixture를 테마 테스트에 잘못 배치한 첫 실행은 NotFound로 실패했습니다. 해당 block을 올바른 언어 pack fixture로 옮긴 뒤 영향받은 appearance target과 아직 실행되지 않은 관련 target을 검사했습니다. runtime 제품 코드는 변경하지 않았습니다.

최종 appearance 9·theme/locale 추출 각 1·IPC 7·도메인 3, 서로 다른 검사 21건이 통과했습니다. 같은 검사를 두 번 합산하지 않습니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide --test appearance_actions_runtime --test taide_theme_extraction --test taide_locale_extraction --test rust_native_phase0_contract --test domain_boundaries` | 최종 9·1·1·7·3건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p taide --lib --test appearance_actions_runtime -- -D warnings` | exit 0 |
| `cargo clippy -p taide --test appearance_actions_runtime -- -D warnings` | fixture 배치 수정 후 exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |

공개 생성 입력 불변으로 이전 Specta 생성 성공을 재사용합니다. bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`도 불변입니다. 의존 입력이 snippet 단위와 같으므로 runtime normal graph 537줄의 Tauri 0개 결과를 재사용합니다. fixture만 바뀐 뒤 runtime 문서를 반복 검사하지 않았습니다.

## 남은 경계

F163/S10/A13/P20에서 S 6개를 반영하면 F169/S4/A13/P20입니다. 정적 entry 배치 수이며 보안/자원 회수·실기 동등성 합격 수가 아닙니다. locale 경로 검증의 사용자 결정과 보안 수정·task/LSP/project/agent/sync/terminal application·nested worker/root·직접 Exit/OS 오류·M6/M7/M8는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
