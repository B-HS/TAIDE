# rust-native Phase 0 IPC 계약 fixture

`ipc-contract-manifest.json`은 `src-tauri/tests/rust_native_phase0_contract.rs`가 원천 코드와 양방향 비교하는 IPC 계약 기준선입니다. 스키마와 실측값은 `docs/acknowledge/2026-09-23-rust-native-phase0-contract-baseline.md` §5에 기록돼 있습니다.

`remote-wire-session-v1.json`은 `src-tauri/tests/rust_native_phase0_remote_fixture.rs`가 현재 remote 서비스·store·protocol API와 비교하는 비시크릿 기준선입니다. Host/Origin 허용·거부, 일회용 link/nonce, 세션 폐기, password 검증 결과와 JSON/binary frame을 고정합니다. 발급된 토큰·salt 자체는 fixture나 테스트 출력에 저장하지 않습니다. 실제 HTTP/WebSocket 연결·cookie·TTL 실기는 별도 gate입니다.

`ide-mcp-wire-v1.json`은 `src-tauri/tests/rust_native_phase0_ide_fixture.rs`가 현재 IDE protocol API의 initialize, tools/list, tools/call, 오류·알림 wire와 비교합니다. 현행 `2025-03-26` 버전의 [MCP lifecycle](https://modelcontextprotocol.io/specification/2025-03-26/basic/lifecycle)·[tools](https://modelcontextprotocol.io/specification/2025-03-26/server/tools) 형식을 기준으로 하며 실제 WebSocket 인증·요청 처리·사용자 파일 동작을 실행하지 않습니다. CLI marker 이름 fixture와 제거 대기 검사는 `crates/taide-cli/tests/fixtures/wait-marker-v1.txt`·`crates/taide-cli/src/main.rs`에 있습니다. 실제 앱의 marker 인수 전달·닫기 시 제거는 별도 gate입니다.

`persistence-v1.json`은 `src-tauri/tests/rust_native_phase0_persistence_fixture.rs`가 legacy settings/session/project/layout/hot-exit JSON을 실제 파서와 독립 임시 저장 경로에서 복원하는 기준선입니다. 파일 자체의 `schemaVersion`은 fixture 버전이고, 제품에서 별도 버전 필드가 없는 project·hot-exit 파일에 새 버전 필드를 강제하지 않습니다. v1 layout의 dirty 파일 탭, 과거 settings 키, session 기본 필드와 mirror의 누락된 disk baseline을 확인합니다. 사용자 저장 데이터나 실행 중 앱 경로는 읽지 않습니다.

- `ordering`은 `source-declaration-order`입니다. event는 `events.rs`, 나머지 목록은 각 등록·정책 원천의 선언 순서를 그대로 쓰며 `collect_events!`에는 등록 집합 일치만 요구합니다.
- `generatedBindings.sha256`은 `src/shared/api/bindings.ts` 전체 바이트의 SHA-256입니다.

## 갱신 절차

1. 원천(`src-tauri/src/lib.rs`, `events.rs`, `remote_gateway.rs`, `crates/taide-model/src/error.rs`)을 변경합니다.
2. 목록 항목을 해당 원천 선언 순서대로 manifest에 반영합니다.
3. `bindings.ts` digest를 다시 계산해 `generatedBindings.sha256`에 넣습니다.

```sh
shasum -a 256 src/shared/api/bindings.ts
```

4. `cargo test -p taide --test rust_native_phase0_contract`로 집합·순서·해시가 모두 맞는지 확인합니다.

manifest를 손으로 고치지 않고 생성 스크립트를 두지 않는 이유는, 생성기가 있으면 원천 파싱 규칙이 테스트와 생성기 두 곳으로 갈라지기 때문입니다. 검증 규칙의 단일 출처는 테스트 파일입니다.
