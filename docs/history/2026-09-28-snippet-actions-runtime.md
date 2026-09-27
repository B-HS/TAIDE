# 스니펫 공개 action의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/snippet_actions.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`
- `src-tauri/src/domain/snippet/commands.rs`, `src-tauri/tests/snippet_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

snippet list/save/delete 3개를 runtime으로 이전했습니다. 기존 await 없는 service 호출을 동기 action으로 제공하며 Tauri async 공개 시그니처는 유지합니다. 기존 local taide-snippet crate와 lock edge 각 한 줄만 연결했습니다.

## 상세

1. 원본 action body와 이전 body는 포맷 외 불일치 0이며 공개 시그니처 3개·문서 입력은 불변입니다. mutation guard·이벤트·spawn·추가 입력 스키마를 도입하지 않았습니다.
2. filename의 경로 구분자/상위 경로/Windows prefix/확장자 검증, JSON 스키마 검증→atomic 원문 저장과 오류를 같은 service에 위임합니다. tolerant scan의 잘못된 JSON/확장자 skip과 파일명 정렬·missing delete의 NotFound도 유지합니다.
3. 자기 UUID fixture로 원문 공백/줄바꿈 보존·배열 prefix/body·unknown field 관용·정렬·삭제·잘못된 경로/JSON의 무변경을 확인했습니다. 직접 생성한 fixture만 정리하며 사용자 snippet/파일·앱·네트워크는 사용하지 않았습니다. 기존 service 구현/12개 unit은 불변이며 이번 실행 수에 합산하지 않습니다.

## 검증 기록

새 runtime 모듈 부재 E0432(exit 101)로 RED를 확인했습니다. 새 action 4·기존 추출 1·IPC 7·도메인 3, 서로 다른 검사 15건이 통과했습니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide --test snippet_actions_runtime --test taide_snippet_extraction --test rust_native_phase0_contract --test domain_boundaries` | 4·1·7·3건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p taide --lib --test snippet_actions_runtime -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo tree -p taide-runtime --edges normal --prefix none` | 537줄, Tauri 패키지 0개, exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |

공개 생성 입력 불변으로 이전 Specta 생성 성공을 재사용합니다. bindings digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`도 불변입니다.

## 남은 경계

F160/S13/A13/P20에서 S 3개를 반영하면 F163/S10/A13/P20입니다. 정적 entry 배치 수이며 전체 자원 회수·실기 동등성 합격 수가 아닙니다. 나머지 appearance/task/LSP·project/agent/sync/terminal application과 nested worker/root·직접 Exit/OS 오류·M6/M7/M8는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
