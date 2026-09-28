# IDE pending 요청 종료 검사

## 대상 파일

- `crates/taide-ide/src/store.rs`
- `src-tauri/src/domain/ide/server.rs`

## 리포트

연결 작업 취소가 pending diff/save responder를 남기는 결함을 요청 소유자 Drop으로 막습니다. 서버 전체 중지와 정상 renderer 응답은 독립된 해소 경로로 보존합니다.

## 확인 항목

- [x] 취소된 diff 작업만 제거되고 다른 diff ID는 남습니다.
- [x] 취소된 save 작업만 제거되고 다른 save ID는 남습니다.
- [x] 정상 diff/save 응답을 먼저 회수한 뒤 owner가 종료되어도 결과가 유지됩니다.
- [x] 서버 전체 종료가 먼저 회수한 diff/save는 owner 종료 뒤에도 종료 응답을 전달합니다.
- [x] 수정한 Tauri 연결부가 컴파일되고 기존 파일 경로 단위 검사 1건이 통과합니다.
- [ ] 실제 WebSocket 단절·재접속, renderer 탭 상태와 사용자 환경의 응답 지연을 실기로 확인합니다. 이 검사는 실제 앱이 필요한 M7 검증으로 유지합니다.

## 실행 결과

`cargo test --offline -p taide-ide --lib 취소된_요청 --quiet`의 수정 전 2건은 pending 잔류로 실패했습니다(exit 101). 수정 후 `cargo test --offline -p taide-ide --lib store::tests --quiet` 19건과 Tauri 연결부 대상 1건이 통과했습니다. IDE/Tauri lib·tests Clippy와 strict IDE rustdoc, Rust fmt/diff 및 대상 MD 3개 Prettier 검사는 exit 0입니다. 실제 WebSocket/GUI를 실행하지 않았으므로 연결 단절 실기까지 통과했다고 판정하지 않습니다.
