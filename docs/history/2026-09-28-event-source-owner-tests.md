# 이벤트 source 검사의 runtime 소유 경로 갱신

## 대상 파일

- `src-tauri/tests/platform_event_sink.rs`
- `crates/taide-runtime/src/sync_actions.rs`, `terminal_actions.rs`, `agent_actions.rs`, `lsp_actions.rs`, `lsp_install_actions.rs`, `project_actions.rs`
- `src-tauri/src/domain/*/commands.rs`, `src-tauri/src/platform/event_sink.rs`

## 리포트

기존 source-scan 다섯 건은 application action 이전 전 Tauri body의 이벤트 marker를 찾고 있어 29건 중 5건이 실패했습니다. 현재 이벤트를 발행하는 runtime action 및 Tauri adapter를 읽고 검사의 원천 경로를 옮겼습니다. 이벤트 조건·상태 변경 순서·발행 개수의 기존 검사는 유지했습니다.

## 상세

1. sync 네 경로는 runtime의 live settings 갱신 또는 locale 적용 뒤 `SyncStateChanged`를 확인하고, Tauri command가 같은 action을 호출하는지도 검사합니다.
2. terminal은 Tauri의 cwd/command/exit callback과 runtime의 세션 insert→spawn event를 분리해 확인합니다. 이전 alias인 `exit_metadata` 대신 현재 callback의 `metadata`를 검사합니다.
3. project는 runtime open 세 경로·close·recent의 발행 개수와 attach/detach 선행 순서를 검사하고 Tauri 위임을 확인합니다. agent는 runtime poll/hook diff 뒤 이벤트와 Tauri 외부 열기 queue→이벤트를 확인합니다.
4. LSP는 Tauri 명령의 runtime action 위임과 runtime의 toolchain install 경로를 따라가며 설치 진행의 바이트 변환·EventSink 발행을 확인합니다.

## 검증

- 수리 전 전체 24/29건 통과, 첫 수리 뒤 terminal 변수명 한 곳이 남아 28/29건 통과, 최종 `cargo test -p taide --test platform_event_sink` 29/29건 통과했습니다.
- `cargo clippy -p taide --test platform_event_sink -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check`, 대상 MD Prettier check가 exit 0입니다.
- 테스트·문서만 변경했습니다. 실제 Tauri 앱/GUI 이벤트 전송은 실행하지 않았으며 전체 M6/M7/M8 gate는 미완료입니다.
