# M6 IPC·이벤트 현행 계약 재검사

## 대상 파일

- `src-tauri/tests/rust_native_phase0_contract.rs`, `src-tauri/tests/platform_event_sink.rs`, `src-tauri/tests/app_services_runtime.rs`
- `src-tauri/src/domain/ide/server.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/platform/event_sink.rs`
- `docs/PROCESS.md`

## 리포트

현행 command·event·raw channel의 등록 목록과 생성 bindings, 실제 `collect_commands!` 출력·dispatch 테이블, AppServices 공유·주입과 EventSink 발행 배선을 재검사했습니다. IDE pending 응답 수리 뒤 `platform_event_sink`의 소스 검사가 예전 `insert_pending_diff/save` 이름을 찾으며 29건 중 1건 실패했습니다. 제품은 이미 `insert_pending_diff_owned`·`insert_pending_save_owned`를 사용하므로 검사 문자열 두 곳만 갱신했습니다. 제품 코드·공개 IPC·bindings는 변경하지 않았습니다.

## 상세

Phase 0 검사는 Specta command 203개, raw channel 3개, event 30개의 manifest·등록·bindings 대응과 bindings 전체 바이트 해시를 검사합니다. 이것은 정적 wire 기준선이며 실제 앱에서 모든 command의 동작과 모든 event payload가 렌더러에 전달되는 실기를 증명하지 않습니다. `platform_event_sink`의 29건은 합성 port와 소스 순서를 검사하며 IDE diff/save는 pending owner 등록이 event 발행보다 앞선다는 현행 경계를 확인합니다.

## 검증

- `cargo test --offline -p taide --test rust_native_phase0_contract --quiet`: 7/7 통과.
- `cargo test --offline -p taide --lib collect_commands_매크로_출력과_dispatch_테이블은_커맨드_이름_집합이_일치한다 --quiet`: 1/1 통과.
- `cargo test --offline -p taide --test app_services_runtime --quiet`: 2/2 통과.
- `cargo test --offline -p taide --test platform_event_sink --quiet`: 수정 전 28/29 통과·IDE 소스 검사 1건 실패(exit 101), 수정 후 29/29 통과.
- `cargo clippy --offline -p taide --test platform_event_sink -- -D warnings`: exit 0.
- `cargo fmt --all --check`와 `git diff --check`: exit 0.

실제 native GUI·원격/IDE WebSocket·PTY/LSP OS 수명주기·Windows는 실행하지 않았습니다. M6-JP/JQ와 M6 전체·M7/M8 gate는 열린 상태입니다.
