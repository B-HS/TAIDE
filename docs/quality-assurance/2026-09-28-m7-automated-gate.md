# M7 자동 검증 진행 기록

## 대상 파일과 리포트

현재 `to_rust_native`의 Rust workspace, `src-tauri/src/lib.rs`의 IDE 저장 경계 검사, 프런트엔드 테스트·타입·빌드와 M7 전체 회귀 gate를 대상으로 합니다. 이 문서는 자동 검사와 실제 앱·데이터·사용자 회귀를 구분하며 M7 완료 판정이 아닙니다.

## 확인한 결과

- `bun test`: 294개 파일의 2,949건 통과, 실패 0건, exit 0입니다.
- `bun run typecheck`: exit 0입니다.
- `cargo clippy --offline --workspace --all-targets -- -D warnings`: 전체 대상 검사 exit 0입니다.
- 수정 뒤 `cargo fmt --all -- --check`, `git diff --check`, 신규 QA 문서 Prettier는 각각 exit 0입니다.
- 직전 동일한 제품 코드로 실행한 `CARGO_NET_OFFLINE=true bun run tauri build --debug --bundles app --no-sign --config '{"identifier":"net.gumyo.taide.m6bundle.0ee5f27e64f94daa82e4789556697b3c"}'`는 sidecar·Vite·Rust macOS 번들 생성까지 exit 0이었습니다. 이번 실행은 빌드를 반복하지 않았고 서명·공증·앱 실기로 계산하지 않습니다.

## Rust 전체 테스트의 첫 실패와 수정

첫 `cargo test --offline --workspace --quiet`는 샌드박스에서 Tauri lib 190건 중 183건 통과·7건 실패 후 exit 101로 끝났습니다. 프로세스 조회 1건은 자기 PID를 찾지 못했고, 테스트용 로컬 소켓 5건은 `Operation not permitted`로 bind가 실패했습니다. 나머지 1건은 `src-tauri/src/lib.rs::ide_diff_저장은_조립부의_파일_저장_경로를_사용한다`가 runtime으로 이전된 저장 본문을 여전히 Tauri 파일에서 찾는 낡은 source assertion이었습니다.

검사는 Tauri command의 `ide_actions::ide_resolve_diff` 위임과 `crates/taide-runtime/src/ide_actions.rs`의 저장 port 호출을 각각 확인하도록 수정했습니다. 해당 대상만 재실행한 `cargo test --offline -p taide --lib ide_diff_저장은_조립부의_파일_저장_경로를_사용한다 --quiet`는 1건 통과, exit 0입니다.

첫 권한 허용 `cargo test --offline --workspace --quiet`는 Tauri lib 190건과 여러 통합 바이너리를 통과한 뒤 `task_supervisor` 22건 중 1건에서 exit 101로 멈췄습니다. 이 source 검사는 이전의 `LspInstallStore` 직접 `block_on` 문자열을 찾고 있었습니다. 현재 앱은 `ExitDrain::wait_for_direct_exit`에 같은 store를 전달하고 runtime에서 `installs.wait_for_idle().await`를 실행하므로 검사 지점을 두 경계로 갱신했습니다. `cargo test --offline -p taide --test task_supervisor 앱_조립은_장기_작업과_자동_시작을_등록하고_종료시_취소한다 --quiet`는 1건 통과, exit 0입니다.

두 번째 권한 허용 전체 실행은 위 `task_supervisor` 22건까지 통과했지만 `taide-runtime --test task_supervisor_blocking` 6건 중 1건에서 exit 101로 멈췄습니다. `stop_all()` 직후 `tracked_count()`가 1이어야 한다는 테스트 가정과 달리 관찰값은 2였습니다. 현재 구현은 async observer의 abort를 요청할 뿐 즉시 완료로 집계하지 않으며, 같은 crate의 기존 단위 검사도 그 경계를 고정합니다. 요청 직후의 정확한 수량 주장만 제거하고 observer 취소 완료 뒤 blocking worker 1개가 남는 검사는 유지했습니다. `cargo test --offline -p taide-runtime --test task_supervisor_blocking --quiet`는 수정 뒤 6건 통과, exit 0입니다. 세 번째 권한 허용 `cargo test --offline --workspace --quiet`는 전체 타깃 종료까지 exit 0입니다.

## 남은 gate

- [x] 권한 허용 Rust workspace 전체 테스트의 최종 exit 확인
- [x] Rust workspace Clippy의 최종 exit 확인
- [x] `remote-wire-session-v1.json`의 서비스·store·protocol API 기준 3건, 해당 test target Clippy, Rust fmt와 JSON Prettier 확인
- [x] `ide-mcp-wire-v1.json` protocol API 3건·CLI marker bin 18건과 각 대상 Clippy 확인
- [x] `persistence-v1.json`의 실제 저장/복원 3건, 해당 test target Clippy와 JSON Prettier 확인
- [ ] 저장 데이터·IPC fixture와 실제 앱 GUI·직접 Exit·사용자 회귀 확인
- [ ] M6와 Phase 0 선행 조건을 충족한 뒤 M7 전체 완료 판정

remote fixture는 Host/Origin 허용·거부, 일회용 link/nonce·세션 폐기, password 검증 결과와 JSON/binary frame을 실제 API 결과와 비교했습니다. `cargo test --offline -p taide --test rust_native_phase0_remote_fixture --quiet`는 3건 통과, exit 0이고 `cargo clippy --offline -p taide --test rust_native_phase0_remote_fixture -- -D warnings`도 exit 0입니다. 실제 HTTP/WebSocket handshake·cookie·TTL·전송 큐는 실행하지 않았으므로 Phase 0의 remote fixture 전체와 M7-C는 아직 미완료입니다.

`src-tauri/src/domain/remote/ws.rs`의 채널 sink는 JSON과 binary를 같은 큐에 넣고 Drop에서 channel-end를 보냅니다. `cargo test --offline -p taide --lib 수신자가_살아있는_채널은_json_binary_end를_순서대로_전송한다 --quiet`는 순서·index 대상 1건 통과, exit 0입니다. `cargo clippy --offline -p taide --lib -- -D warnings`, `cargo fmt --all -- --check`, 대상 MD Prettier와 `git diff --check`도 exit 0입니다. 이는 실제 HTTP/WebSocket upgrade나 writer 송신 검사가 아닙니다. 현재 writer 큐는 `tokio::sync::mpsc::unbounded_channel`이므로 느린 수신자에 대한 메모리 backpressure 상한이 없고, dispatch semaphore 128개는 동시 실행 수만 제한합니다. [Tokio 공식 문서](https://docs.rs/tokio/latest/tokio/sync/mpsc/fn.unbounded_channel.html)도 수신자가 뒤처지면 메시지가 임의로 버퍼링된다고 명시합니다. 따라서 큐 경계·실제 socket 수명은 별도 검증/결정 전까지 미완료입니다.

IDE/MCP fixture는 현행 `2025-03-26`의 initialize·tools/list·tools/call 및 오류·알림 wire를 protocol API와 비교했습니다. `cargo test --offline -p taide --test rust_native_phase0_ide_fixture --quiet` 3건과 해당 대상 Clippy가 exit 0입니다. CLI는 고유 UUID 임시 디렉터리에서 marker 생성·즉시 timeout·제거 뒤 완료를 확인했으며 `cargo test --offline -p taide-cli --bin taide-cli --quiet` 18건과 해당 all-target Clippy가 exit 0입니다. 실제 IDE WebSocket 인증·tool handler, 앱의 CLI marker 인수 전달·닫기 시 제거는 실행하지 않아 Phase 0의 IDE/CLI 항목은 미완료입니다.

Persistence fixture는 legacy settings 키를 메모리 파서와 실제 `settings.json` 로드에 적용하고, 고유 UUID 경로의 `session.json`·`project.json`·v1 `layout.json`을 복원했습니다. dirty 파일 탭과 활성 프로젝트가 유지되며 layout 버전은 v2가 됩니다. legacy hot-exit mirror는 `disk_modified_ms` 필드가 없어도 목록에 unsaved content를 유지합니다. `cargo test --offline -p taide --test rust_native_phase0_persistence_fixture --quiet` 3건과 해당 대상 Clippy는 exit 0입니다. 사용자 실제 데이터·GUI 복원은 실행하지 않았습니다.

기능 inventory는 editor·LSP·terminal·preview·shell의 구현과 기존 자동 검사 파일을 연결했습니다. 모든 항목의 실기 근거와 preview 형식별 자동 검사 공백을 미완료로 표시했으므로 기능 baseline 통과로 계산하지 않습니다. 자세한 목록은 [Phase 0 기능 inventory](2026-09-28-rust-native-function-inventory.md)에 있습니다.
