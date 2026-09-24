# LSP 공유 세션의 부분 root 종료가 메시지 구독을 끊던 문제 (수정)

## 대상 파일

- `src-tauri/src/domain/lsp/commands.rs`의 `lsp_stop`·`release_owner_root`
- `src/shared/api/bindings.ts`와 `src-tauri/tests/fixtures/rust-native/ipc-contract-manifest.json`의 명령 문서 계약

## 증상과 원인

같은 창의 LSP 세션이 여러 root 또는 한 root의 여러 참조를 공유할 때, `lsp_stop`이 일부만 해제해도 해당 owner의 메시지 구독을 먼저 제거했습니다. root 목록이 남아 프로세스는 계속 실행되지만, 이후 서버 메시지는 창에 전달되지 않을 수 있었습니다. 일반 프론트엔드 dispose는 공유 그룹의 모든 root를 정리하지만, 부분 해제는 `lsp_stop(sessionId, root, owner)` 명령의 유효한 경로입니다.

## 수정

root 참조 수를 먼저 줄이고 남은 root가 있을 때는 구독을 유지합니다. 마지막 root를 해제하거나 root 없이 전체 종료할 때만 구독을 제거합니다. 기존 IPC 시그니처, workspace-folder 제거 알림, 스토어에서 세션을 제거한 뒤 프로세스를 종료하는 순서는 유지합니다.

## 검증

- 수정 전 Tauri LSP 명령 24건 중 기존 22건은 통과했고, 새 부분 해제 회귀 2건만 owner 구독 유지 단언에서 실패했습니다(exit 101).
- 수정 후 서로 다른 root·동일 root 참조 수·root 없는 전체 종료를 포함한 Tauri LSP 명령 25건이 통과했습니다. 부분 해제 뒤 남은 구독이 실제 테스트 채널로 서버 메시지를 전달하는지도 확인했습니다.
- Phase 0 IPC 계약 7건, `cargo fmt --all --check`, `cargo clippy -p taide --lib -- -D warnings`가 통과했습니다. 생성 바인딩은 `lsp_stop`의 설명 주석만 바뀌었고 manifest SHA-256은 `2bb2a35885f128ea9d13d7464c358fccee469f9f82c5d3c066b79190401ecd38`로 동기화했습니다.
- 실제 언어 서버와 다중창 GUI에서 부분 root 종료·메시지 수신을 확인하는 실기는 미실행입니다. 이는 M7 실기 gate에 남깁니다.
