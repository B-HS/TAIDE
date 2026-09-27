# LSP 조회·루트 탐지·설치 취소의 runtime 이전

## 대상 파일

- `crates/taide-runtime/src/lsp_actions.rs`, `src/lib.rs`
- `src-tauri/src/domain/lsp/commands.rs`, `src-tauri/tests/lsp_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

LSP 세션 조회·루트 탐지·설치 취소 3개를 동기 runtime action으로 이전했습니다. 원래 await 없는 본문을 옮겼으며 Tauri 공개 async signature와 기존 service/store/manifest 정책을 유지합니다.

## 상세

1. 전체 공개 command 11개의 signature와 문서 입력이 byte 동일하고 OS PATH server 탐지 본문도 byte 동일합니다. 원격 호출·의존성·상태 등록을 변경하지 않았습니다.
2. 세션 조회는 프로젝트별 기존 lifecycle snapshot을 반환합니다. 루트 탐지는 bundled server 확인 뒤 동일 service를 호출하고 unknown server는 같은 InvalidArgument입니다. 설치 취소는 멱등이며 worker lease가 살아 있으면 슬롯을 해제하지 않습니다.
3. 자기 UUID의 go.mod marker와 메모리 store만 사용했습니다. 사용자 프로젝트·PATH·LSP 프로세스·앱·네트워크는 실행하지 않았습니다.

## 검증 기록

모듈 부재 E0432(exit 101)로 RED를 확인했습니다. 첫 action 검사의 오류 문자열 기대값은 기존 Display의 `invalid argument:` 접두사를 누락해 실패했습니다. 기대값만 수정했고 제품 코드는 불변입니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide --test lsp_actions_runtime --test taide_lsp_store_extraction --test taide_lsp_service_extraction --test taide_lsp_install_store_extraction` | 최종 action 5·store 2·service 1·install store 2건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p taide --lib --test lsp_actions_runtime -- -D warnings` | 제품/초기 fixture exit 0 |
| `cargo clippy -p taide --test lsp_actions_runtime -- -D warnings` | 기대 문자열 수정 후 최종 fixture exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |

공개 생성 입력 불변이므로 task 단위의 실제 Specta 생성 1·IPC 7 성공과 bindings/manifest digest 불변을 재사용합니다. 의존 입력이 같으므로 normal graph 543줄의 Tauri 0개 결과도 재사용합니다. 별도 실행한 검사 10건과 재사용한 검사를 중복 합산하지 않습니다.

## 남은 경계

F170/S3/A13/P20에서 3개 S→F로 F173/S0/A13/P20입니다. 정적 entry 배치 수이며 전체 application·자원 회수·보안·실기 합격 수가 아닙니다. locale 보안 선택과 나머지 application/nested worker/root·M6/M7/M8는 미완료이며 M6 전체 완료 뒤 일반 push 조건을 유지합니다.
