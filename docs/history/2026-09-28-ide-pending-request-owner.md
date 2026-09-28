# IDE pending 요청 완료 소유

## 대상 파일

- `crates/taide-ide/src/store.rs`
- `src-tauri/src/domain/ide/server.rs`
- `docs/bug/2026-09-28-ide-connection-pending-owner.md`

## 리포트

IDE WebSocket 연결별 `JoinSet`이 `tools/call`을 취소하면 diff/save 응답 await 뒤의 timeout 정리 분기에 도달하지 않습니다. 기존 `IdeStore`는 연결별 pending 항목의 종료 소유자가 없어 서버 전체가 살아 있는 동안 responder를 계속 보유할 수 있었습니다.

## 상세

- 자기 메모리 store에서 취소된 diff/save 작업 2건이 자기 pending 항목을 남기는 실패를 확인했습니다. 다른 ID가 보존되어야 하는 조건도 같은 fixture에 넣었습니다.
- `IdeStore`에 요청별 `PendingRequestOwner`를 두고 diff/save 등록과 함께 반환합니다. owner Drop은 자기 ID만 회수하며 이미 응답·프로젝트/탭 해소·서버 전체 종료로 제거된 항목에서는 아무 일도 하지 않습니다.
- Tauri의 `tool_open_diff`·`tool_save_document`가 owner를 응답 대기 동안 보유합니다. 기존 timeout 결과, JSON-RPC 응답 형식, 이벤트 발행 순서는 변경하지 않았습니다.
- IDE 서버/연결의 실제 WebSocket 단절·재접속과 renderer 탭 상태는 실행하지 않았습니다. PTY SIGHUP 정책과 전체 M6/M7/M8 gate도 별도입니다.

## 검증

- RED: `cargo test --offline -p taide-ide --lib 취소된_요청 --quiet` — 2건 실패(exit 101), 각 pending 항목 잔류.
- GREEN: `cargo test --offline -p taide-ide --lib store::tests --quiet` — 19건 통과.
- `cargo test --offline -p taide --lib domain::ide::server::tests::존재하지_않는_경로의_open_file은_탭을_만들기_전에_거절된다 --quiet` — 1건 통과, 수정한 Tauri 연결부 컴파일 포함.
- `cargo clippy --offline -p taide-ide -p taide --lib --tests -- -D warnings`와 `RUSTDOCFLAGS='-D warnings' cargo doc --offline -p taide-ide --no-deps` — exit 0.
- `cargo fmt --all --check`와 `git diff --check` — exit 0.
- `node_modules/.bin/prettier --check --ignore-path /dev/null`로 history·QA·bug 대상 MD 3개 검사 — exit 0.
