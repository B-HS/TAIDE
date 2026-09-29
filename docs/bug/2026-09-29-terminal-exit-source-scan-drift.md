# 직접 종료 변경 후 terminal source-scan 계약 누락

## 대상 파일

- `src-tauri/tests/taide_terminal_store_extraction.rs`
- `src-tauri/src/lib.rs`

## 관찰과 원인

`main` CI #85 Rust job은 포맷·Clippy를 통과한 뒤 `spawn_adapter는_감독_worker와_동일한_guard를_등록까지_유지한다`에서 실패했습니다. `src-tauri/tests/taide_terminal_store_extraction.rs:43`의 `split_once("move || handle.exit(exit_code)")`가 `None`이었습니다. M7 직접 종료 경로를 추가하면서 `ExitRequested` 콜백이 여러 줄 블록으로 바뀌었지만 source-scan 테스트의 문자열 경계를 갱신하지 않았습니다. 제품 동작 실패가 아니라 검사 기준 문자열의 불일치입니다.

## 수정과 검증

검사의 범위를 `ExitRequested` drain 시작부터 다음 `RunEvent::Exit` 분기 전까지로 잡고, 그 안에 `TerminalStore` 전달과 `handle.exit(exit_code)`가 모두 있는지 확인하도록 바꿨습니다. `cargo fmt --all --check`·`git diff --check`는 exit 0이며, Rust 테스트 최종 판정은 수정 커밋의 `main` CI 결과로 합니다. 로컬 전체 `taide` 대상 테스트는 이전 시도에서 약 16분 컴파일 후 중단돼 통과 근거로 쓰지 않습니다.
