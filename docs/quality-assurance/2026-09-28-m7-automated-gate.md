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
- [ ] 저장 데이터·IPC fixture와 실제 앱 GUI·직접 Exit·사용자 회귀 확인
- [ ] M6와 Phase 0 선행 조건을 충족한 뒤 M7 전체 완료 판정

remote fixture는 Host/Origin 허용·거부, 일회용 link/nonce·세션 폐기, password 검증 결과와 JSON/binary frame을 실제 API 결과와 비교했습니다. `cargo test --offline -p taide --test rust_native_phase0_remote_fixture --quiet`는 3건 통과, exit 0이고 `cargo clippy --offline -p taide --test rust_native_phase0_remote_fixture -- -D warnings`도 exit 0입니다. 실제 HTTP/WebSocket handshake·cookie·TTL·전송 큐는 실행하지 않았으므로 Phase 0의 remote fixture 전체와 M7-C는 아직 미완료입니다.
