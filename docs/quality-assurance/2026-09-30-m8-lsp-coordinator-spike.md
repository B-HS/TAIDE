# M8 Rust LSP coordinator 상태기계 실험

## 대상과 현재 판정

`experiments/lsp-coordinator-spike`는 독립 Cargo workspace입니다. 초기 구현은 실험에만 있었으며, 아래 제품 이전 단계에서 pure coordinator·capability·sync를 `taide-lsp::native`로 옮겼고 실험은 제품 구현을 직접 검사합니다. 기존 제품 LSP client·Tauri adapter의 실행 경로와 사용자의 language server·프로젝트·설정은 바꾸지 않습니다. 기존 source에서 Rust는 process lifecycle·framing을, `src/shared/lib/lsp/client.ts`와 initialization/session registry는 handshake·pending·document mirror를 소유하는 것을 확인했습니다. 결정적 상태기계 proof와 infra transport를 사용하는 실제 합성 Rust 서버의 crash/restart 연결 검사가 통과했습니다. 제품 LSP 전체 구현·실제 language server 동등성 완료는 아닙니다.

초기 실험은 제품에 이미 존재하는 `serde_json` 1.0.151·Tokio 1.53.1·`taide-infra`를 재사용했습니다. 후속 표준 DTO 경계에는 아래에 기록한 MIT `lsp-types` 0.97.0을 추가했습니다. 제품·실험 MSRV 선언은 1.89로 유지하며 실제 검사는 설치된 rustc 1.98.1에서 수행했습니다. Rust 1.89 자체 실행 검사는 아직 수행하지 않았습니다.

공식 계약: [initialize](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/general/initialize.md), [didOpen](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/textDocument/didOpen.md), [didChange](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/textDocument/didChange.md). initialize 응답 전 일반 요청을 금지하고 generation별 handshake를 한 번으로 제한합니다. initialized 뒤 열린 mirror를 replay하며, 동일 URI의 close/open도 균형을 유지합니다.

## 구현 경계

- 제품 `LspCoordinator`는 handshake phase·generation·request ID·pending deadline·capabilities·문서 mirror·revision/version·문서 incarnation을 소유합니다. 실험의 `CoordinatorProbe`는 호환 재수출 이름일 뿐 별도 구현이 아닙니다.
- mock 시간은 주입된 monotonic millisecond 값입니다. 실제 대기를 이용해 timeout을 흉내 내지 않습니다.
- 일반 요청은 Running에서만 만들고 generation·revision·incarnation이 바뀐 응답을 채택하지 않습니다. restart·cancel·timeout은 pending completion과 필요한 wire message를 반환합니다.
- replay batch를 쓴 뒤 호출하는 `finish_replay`는 그 사이 발생한 edit·close/open의 delta를 먼저 반환합니다. 아직 delta가 있으면 Replaying을 유지하며, 최신 mirror까지 write acknowledgement를 받은 뒤에만 Running으로 바뀝니다. 실제 process 검사는 각 batch의 `write_message` 완료 뒤 acknowledgement를 호출하고, 서버의 다음 hover 응답으로 최신 mirror·version·메시지 순서를 확인합니다. 이는 OS pipe write 완료이며 서버의 별도 replay acknowledgement 프로토콜은 아닙니다.
- pending 256개·open mirror 256개·mirror text 합계 64MiB의 실험 상한이 있습니다. replay snapshot은 별도 mirror 복사본을 가지므로 이를 전체 메모리 상한이라고 주장하지 않습니다. 이 값은 제품의 대형 파일 정책 확정이 아닙니다.

## 실행 결과

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --offline --test coordinator
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --test coordinator replay_write_중
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --test coordinator revision_cancel
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --test coordinator generation마다
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --all-targets -- -D warnings
```

초기 API 부재 E0432 red 뒤 첫 3건이 통과했습니다. replay 중 edit·close/open 검사를 추가했을 때 기존 단순 acknowledgement API의 E0608/E0599 red를 확인했고, replay delta와 incarnation을 구현한 뒤 새 검사 1건이 통과했습니다. 해당 변경이 영향을 주는 revision/cancel과 generation/replay 검사만 각각 1회 재검증해 통과했습니다. strict clippy는 exit 0이며 총 4개 계약의 실제 통과 증거를 확보했습니다.

1. generation마다 initialize 한 번, handshake·replay write 전 일반 request 차단, restart 시 이전 pending 실패와 오래된 process 응답 제외, Unicode mirror replay.
2. 문서 revision 변경 뒤 stale 응답 제외, cancel의 late response 제외, deadline 직전/직후 처리와 pending 제거, 동일 URI·동일 revision으로 재오픈한 다른 incarnation의 이전 응답 제외.
3. initialize error·timeout·미지원 position encoding에서 Degraded로 전환하고 pending을 종결합니다. timeout 때 initialize 응답 전 cancellation notification을 보내지 않습니다.
4. replay write 중 edit와 close/open을 최신 delta로 전달하고 추가 write가 남으면 Running으로 넘어가지 않습니다.

## 실제 framed process 연결 검사

대상: `tests/real-process.rs`, `src/bin/mock-server.rs`, `crates/taide-infra/src/lsp_proc.rs`의 기존 `spawn` framing·owned process API입니다. 처음 연결할 때는 제품 source가 변경되지 않았고, 아래 bounded API 단계에서 내부 builder를 공유하도록 바꾼 뒤 기존 검사를 다시 한 번 통과했습니다. Cargo가 빌드한 정확한 합성 실행 파일을 직접 실행하며 shell·실제 project file·사용자 설정은 사용하지 않습니다. GUI 실기용 앱에도 접근하지 않습니다.

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test real-process
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
```

서버 실행 파일 부재의 compile-time red 뒤 구현했고 실제 연결 검사 1건이 0.32초에 통과했습니다. strict clippy exit 0입니다. 기존 상태기계 4건은 source를 변경하지 않아 성공 증거를 재사용했습니다. 별도 실험 간 기존 build cache를 재사용했으며 제품 root target·Cargo.lock에는 변경이 없습니다.

1. 첫 owned child에 initialize → initialized → Unicode·NFD·supplementary·CRLF mirror open → full change → hover를 보냅니다. 합성 서버는 순서·initialize 한 번·open/close 균형·증가 version을 실제 stdin에서 검사합니다.
2. 첫 hover에서 합성 서버가 exit 7과 고정 stderr를 반환합니다. 실제 child exit·stdout/stderr reader 종료·wait worker join을 관찰한 뒤에만 새 process를 만듭니다. 이전 pending은 Restarted로 종결됩니다.
3. 새 generation은 마지막 mirror를 replay하고, replay 중 추가 edit를 delta로 전송한 뒤에만 Running이 됩니다. 다음 hover가 최신 원문·version 3·initializeCount 1·메시지 순서를 실제 서버 mirror에서 반환합니다. 이어서 close/open의 새 mirror·version 1도 확인합니다.
4. 최초 검사는 harness가 직접 shutdown 응답 null 뒤 exit를 보내고 정상 exit 0·reader/wait join을 확인했습니다. 이후 아래 종료 상태기계를 연결해 coordinator가 didClose·shutdown·exit를 만들고 실제 회수 확인 뒤 Stopped로 전환하는 검사로 확장했습니다. [shutdown](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_includes/messages/3.17/shutdown.md)·[exit](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_includes/messages/3.17/exit.md)·[initialized](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_includes/messages/3.17/initialized.md) 공식 순서를 사용합니다.

서버 fixture는 header 4KiB·body 1MiB·32개 method·8개 mirror로 제한하며, test reply queue는 8개입니다. 이는 합성 fixture 상한이지 기존 infra의 전체 incoming framing·제품 backpressure 보안 통과가 아닙니다. watchdog은 단계별 3초이며 요청 timeout 상태기계는 주입된 결정적 시간을 유지합니다. crash/restart에 필요한 두 process를 한 검사에서 사용했으며 같은 성공 검사를 반복하지 않았습니다.

## 문서 동기화 capability

대상: `src/synchronization.rs`, coordinator의 open/change/close/replay·새 `saved` 경계, `tests/synchronization.rs`입니다. [Microsoft protocol 정의](https://raw.githubusercontent.com/microsoft/vscode-languageserver-node/main/protocol/src/common/protocol.ts)와 [공식 client의 legacy number 정규화](https://raw.githubusercontent.com/microsoft/vscode-languageserver-node/main/client/src/common/client.ts), [Position](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/position.md)·[Range](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/range.md) 계약을 참조했습니다.

- capability 누락·numeric None·빈 options는 문서 알림을 보내지 않습니다. options의 openClose/change/save·includeText 광고값을 따릅니다. legacy Full/Incremental number는 Microsoft client처럼 openClose와 text 미포함 save로 정규화합니다.
- Full은 원문 전체를 보냅니다. Incremental은 이전 mirror 전체의 UTF-16 range를 명시한 replacement 한 건을 보냅니다. 최소 diff·transaction edit 목록 전송·큰 파일 성능 통과는 아니며 후속 DocumentStore 연결에서 개선합니다.
- range 끝은 CJK·NFD·supplementary와 LF·CRLF·CR을 처리합니다. replay 중 여러 차례 수정되면 각 delta의 이전 snapshot을 기준으로 계산합니다.
- malformed sync/options는 initialize pending을 제거하고 Degraded로 전환합니다. feature capability guards는 아래 단계에서 구현했으며 dynamic registration·willSave/willSaveWaitUntil은 아직 구현하지 않았습니다.

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test synchronization
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test coordinator --test real-process
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test coordinator revision_cancel
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test real-process
```

새 save API 부재 E0599 red 뒤 동기화 검사 3건이 통과했습니다. coordinator source 변경의 영향 검사에서 기존 4건 중 3건은 통과했고, revision/cancel fixture 한 건이 sync 광고 없이 didChange를 기대해 실패했습니다. fixture에 Full sync를 명시하고 기대 assertion은 그대로 유지했으며 해당 한 건만 재실행해 통과했습니다. 초기 통합 명령은 그 실패에서 멈춰 real-process를 실행하지 않았습니다. 이후 변경된 동기화 source로 실제 process 검사 1건을 한 번 실행해 0.34초에 통과했습니다. strict clippy exit 0입니다. 현재 상태기계 4건·실제 process 1건·동기화 3건, 총 8개 계약의 통과 증거를 확보했습니다.

## 기능 capability·종료 상태기계

대상: `src/capabilities.rs`, coordinator의 request/pending·stop/finish_stop, `tests/capabilities.rs`, `tests/stopping.rs`, 기존 `tests/real-process.rs` 종료 부분입니다. 제품 TS client의 `FEATURE_CAPABILITY_CHECKS` 27개를 실제 source와 대조했습니다. bool/options·options-only·prepare/resolve·semantic full/delta를 구분하며, 알려진 미지원 feature는 request ID·pending을 만들기 전에 거절합니다. null·false·문자열·숫자·배열을 지원 광고로 오인하지 않습니다. 옵션 내부의 모든 필드까지 검증한 것은 아니며 별도 native adapter DTO 작업이 남습니다. 알려지지 않은 extension request는 기존 TS 정책처럼 허용합니다.

- restart는 이전 capability를 버립니다. 새 handshake/replay 전에는 supports가 false이며 새 server 광고를 다시 사용합니다. dynamic registration은 아직 구현하지 않았습니다.
- request 종류를 Initialize/Feature/Shutdown으로 구분했습니다. Running의 stop은 feature pending을 Stopped 오류로 종결하고 cancel → 광고된 didClose → shutdown 순서로 보냅니다. shutdown을 보낸 뒤에는 exit 이외의 알림을 추가하지 않습니다.
- Initializing·Replaying·Degraded의 stop은 일반 요청이나 미전송 mirror의 didClose를 추정해 보내지 않고 exit만 반환합니다. 실제 process가 없던 Detected와 이미 Stopping/Stopped인 상태에는 새 stop을 거절합니다.
- shutdown null 응답·서버 error·비정상 result·timeout에서 pending을 정리하고 exit를 반환합니다. shutdown timeout에 cancel을 보내지 않습니다. 모든 경우 회수 acknowledgement 전에는 Stopping을 유지합니다. 종료 중 open/change/close/save/request/restart를 막습니다.
- `finish_stop(generation)`은 현재 generation·Stopping·pending 없음이 확인돼야 Stopped로 전환하고 mirror를 정리합니다. 이 pure API가 자체적으로 OS 회수를 증명하지는 않습니다. 실제 process harness는 child exit와 stdout/stderr/wait worker join 뒤에만 호출합니다. 같은 소유 계약을 제품 async supervisor에 강제하는 작업은 남아 있습니다.

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test capabilities
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test stopping
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test coordinator --test synchronization --test real-process
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
```

각 신규 API 부재 E0599 red를 확인한 뒤 capability 3건·종료 3건이 통과했습니다. source 변경 영향을 받는 상태기계 4건·동기화 3건·실제 process 1건도 한 번 실행해 모두 통과했으며 실제 process는 0.68초였습니다. 기존 hover fixture 두 곳은 새 guard와 일치하도록 hoverProvider를 명시했고 assertion은 유지했습니다. strict clippy exit 0입니다. 이 단계까지 총 14개 계약의 실제 통과 증거를 확보했습니다. 정상 종료의 실제 process 회수에 이어 아래 단계에서 bounded EOF·전송 실패와 exit 무시의 kill escalation을 검사했습니다. write cancellation의 안전성은 아직 남았습니다.

## opt-in bounded framing·실제 실패 회수

대상: 제품 `crates/taide-infra/src/{lsp_frame,lsp_proc}.rs`, `crates/taide-infra/tests/lsp-frames.rs`, 실험 `tests/bounded-process.rs`, 합성 서버의 통제된 실패 모드입니다. [Microsoft의 LSP base framing 설명](https://microsoft.github.io/sqltoolssdk/guide/jsonrpc_protocol)의 ASCII 헤더·CRLF 구분자·바이트 단위 Content-Length·UTF-8 본문 계약을 확인했습니다. 새 의존성 없이 기존 allocator·Tokio·owned child wait task를 사용했습니다.

- `FrameLimits`는 caller가 header/body budget을 선택하는 fallible 생성자입니다. 0·범위 overflow를 거절합니다. 합성 검사의 128B header·1024B body는 제품의 큰 파일 정책이 아닙니다. 기존 `spawn`은 legacy buffer와 동작을 유지하고 새로운 `spawn_bounded`를 선택한 caller에만 엄격한 framing을 적용합니다.
- header budget은 구분자를 포함합니다. ASCII·필수 Content-Length 한 개·숫자·범위·charset을 검사하고 body 상한을 넘으면 본문을 할당하기 전에 실패합니다. 완전한 frame의 UTF-8 검증 뒤 원문을 consumer에 전달합니다. partial header/body EOF·invalid UTF-8·consumer 오류는 구분하며 실패 상태는 sticky입니다. Content-Type의 모든 MIME parameter 검증과 JSON DTO 검증을 대체하지 않습니다.
- receive buffer의 보관 길이·capacity는 선택된 header+body 범위로 검사했습니다. body는 전달할 때 복사하지 않고 이동합니다. consumer에 넘어간 메시지, allocator rounding, process 전체 RSS까지 한 상한으로 보장했다고 주장하지 않습니다. 합성 reply queue의 프레임 수는 제한되지만 제품의 pending·URI·metadata·전송 큐 전체 byte budget은 별도 작업입니다.
- bounded reader의 프로토콜·read·consumer 실패는 최초 원인을 기록하고 해당 child를 소유한 wait task에 kill 신호를 보냅니다. PID를 외부에서 새로 찾아 종료하지 않습니다. child exit 뒤 stdout/stderr drain 또는 제한 시간 뒤 abort·join을 마쳐야 exit callback과 wait completion이 끝납니다. oversized outgoing은 frame 생성 전에 거절하며 valid 요청은 이후 정상 전송됩니다.
- exit 무시 서버는 shutdown 응답 뒤 exit를 실제 수신했다는 합성 알림을 반환하고 살아 있습니다. fixture grace 20ms 동안 wait는 완료되지 않았고, 해당 owned child를 kill한 뒤 실제 child exit·reader/wait join을 확인한 경우에만 coordinator를 Stopped로 전환했습니다. 이 grace 값은 제품 정책 확정이 아니며 제품 async supervisor 연결도 아직 아닙니다.

```sh
cargo test -p taide-infra --locked --offline --target-dir experiments/terminal-core-spike/target --test lsp-frames
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test bounded-process
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test real-process
cargo test -p taide-infra --locked --offline --target-dir experiments/terminal-core-spike/target --lib lsp_proc::tests
cargo clippy -p taide-infra --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
```

API 부재 E0432와 E0425/E0599 red 뒤 새 framing 3건·bounded process 3건이 각각 통과했습니다. bounded process는 0.35초이며 정상 종료 1건, oversized body·큐 포화·잘린 EOF를 묶은 원인별 회수 1건, exit 무시 1건입니다. 공유 builder 변경으로 영향받은 기존 actual process 1건은 0.32초·legacy infra 27건은 0.51초에 통과했고 두 strict clippy도 exit 0입니다. 변경되지 않은 coordinator·sync·capability·stop의 순수 검사 13건은 이전 성공을 재사용했습니다. 실험의 총 17건과 새 infra framing 3건의 증거를 확보했으며 legacy 회귀 27건은 따로 집계합니다.

## cancellation-safe writer·실제 process ownership

대상: 제품 `crates/taide-infra/src/lsp_writer.rs`, `lsp_proc.rs`의 새 `spawn_owned`, `crates/taide-infra/tests/lsp-writer.rs`, 실험 `tests/real-process.rs`입니다. Tokio 1.53.1의 local 공식 source `io/util/async_write_ext.rs`와 `sync/{mpsc/bounded,semaphore}.rs`에서 `write_all`의 cancellation 비안전성, channel reservation 반환, owned byte permit을 확인했습니다. 새 dependency는 없습니다.

- `QueuedWriter`는 검증된 caller 선택형 `WriterLimits`의 큐 수·byte budget을 사용합니다. frame header/body 길이 확인 → 큐 슬롯·byte permit 확보 → fallible frame allocation → enqueue 순서입니다. byte budget에는 queued와 in-flight의 전체 wire frame이 포함되며 한 프레임이 끝날 때까지 permit을 보유합니다. 짧은 header 임시 문자열·metadata·allocator overhead·caller의 입력 문자열·OS pipe까지 포함한 전체 RSS 상한은 아닙니다.
- 접수된 프레임의 write/flush는 독립 worker가 소유하고 caller는 receipt만 기다립니다. ack를 기다리는 future를 취소해도 worker의 `write_all`을 취소하거나 같은 프레임을 처음부터 재전송하지 않습니다. 다음 프레임은 이전 프레임 뒤에만 전송합니다. 미접수·포화·크기 초과는 명시적 오류이며 자동 drop·재시도는 하지 않습니다.
- write 실패는 해당 receipt를 WriteFailed로 종결하고 나머지 queued receipt를 Closed로 해제한 뒤 failure callback을 한 번 부릅니다. 제품 `spawn_owned` callback은 최초 TransportFailure::WriteFailed를 기록하고 소유 child의 wait task에 kill을 알립니다. manager가 닫을 때는 신규 전송을 닫고 pending write/receipt를 회수하며 이를 별도 write failure로 보고하지 않습니다. manager의 종료·Drop은 transport를 영구 폐기하므로 중간 프레임을 정상 session에서 재사용하지 않습니다.
- `spawn_owned`만 bounded reader와 독립 writer를 함께 사용합니다. 기존 제품 `spawn`과 `spawn_bounded`는 직접 stdin 쓰기 정책을 유지합니다. child exit 후 stdout/stderr와 writer를 모두 join해야 exit callback과 wait completion이 끝납니다. 실제 process 검사를 owned 경로로 바꿨고 crash 7→전체 join→새 generation replay→정상 exit 0→전체 join→Stopped assertion은 유지했습니다.

```sh
cargo test -p taide-infra --locked --offline --target-dir experiments/terminal-core-spike/target --test lsp-writer
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test real-process --test bounded-process
cargo test -p taide-infra --locked --offline --target-dir experiments/terminal-core-spike/target --lib lsp_proc::tests
cargo clippy -p taide-infra --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
```

신규 API 부재 E0432 red 뒤 writer 계약 3건이 0.00초에 통과했습니다. 실제 child 대신 bounded Tokio duplex stream에서 이미 일부 바이트가 쓰인 것을 관찰한 뒤 ack future를 취소하며 나머지 프레임과 다음 Unicode 프레임의 정확한 wire bytes를 확인했습니다. 포화 검사도 첫 바이트가 실제 도착한 뒤 수행해 in-flight를 시간 추정으로 대체하지 않았습니다. broken pipe에서는 first receipt WriteFailed·queued receipt Closed·failure count 1·join 뒤 retained bytes 0을 확인했습니다. source 변경이 없는 이 세 검사는 통합 때 재사용했습니다.

writer 통합으로 영향을 받는 actual owned process 1건은 0.01초, bounded process 3건은 0.33초, legacy infra 27건은 0.51초에 한 번씩 통과했습니다. 두 strict clippy exit 0입니다. 기존 framing 3건과 순수 coordinator 13건은 영향이 없어 재실행하지 않았습니다. 총 실험 17건·신규 infra framing/writer 6건·legacy 회귀 27건의 성공 증거를 보유합니다. 실제 server의 stdout EOF만 닫은 hang·제품 async supervisor·모든 feature DTO·대형 파일 및 전체 memory/performance gate는 이 검사로 완료되지 않습니다.

## 제품 coordinator 이전·disconnect 경계

대상: `crates/taide-lsp/src/native.rs`, `native/{capabilities,synchronization}.rs`, 실험 `src/lib.rs`, `Cargo.toml`, `tests/transport-lifecycle.rs`입니다. 기존 `taide-lsp`는 이미 infra·Tokio·serde_json에 의존하므로 제품 dependency와 root Cargo.lock·MSRV는 유지했습니다. 실험에는 해당 제품 crate의 path dependency만 추가했고 기존 두 구현 파일을 제거했습니다. `CoordinatorProbe`는 제품 `LspCoordinator`의 재수출이며 구현이 두 벌 남지 않습니다.

- 이전한 3개 source는 타입 이름과 module 상대 참조만 바꿨습니다. 새로운 disconnect API를 제외한 원래 coordinator와 이전본의 비공백 source 일치도 확인했습니다. 기존 27개 feature guard와 sync·pending·replay·stop 동작은 그대로 사용합니다.
- `request_deadline`은 현재 pending의 가장 이른 실제 deadline 또는 None을 반환해 async owner가 주기 polling 대신 다음 요청 경계까지 기다릴 수 있게 합니다. 이 accessor 자체가 runtime timer 구현을 완료한 것은 아닙니다.
- `process_disconnected(generation)`은 오래된 process 신호를 거절하고 현재 pending을 한 번 종결하며 capability·sync·replay snapshot을 비웁니다. 살아 있는 문서 mirror는 보존하므로 Degraded 상태에서 편집한 최신 Unicode·version이 다음 generation의 handshake/replay에 사용됩니다. 아직 죽은 pipe에 cancel·exit 알림을 보내지 않습니다.
- 종료 중 disconnect는 shutdown pending을 Stopped 오류로 정리하되 phase를 Stopping으로 유지합니다. 실제 child·reader/writer/wait join이 확인된 뒤 async owner가 `finish_stop`을 호출해야 합니다. pure API의 허용 여부를 OS 회수 증거라고 주장하지 않습니다.
- 기존 `LspStore`가 process admission을 닫고 superseded/removed process도 retain해 shutdown·wait_for_idle을 소유하며 runtime `TaskSupervisor`가 operation lease를 관리함을 확인했습니다. 새 병렬 lifecycle 소유자를 추가하지 않고 이 경계를 재사용하는 단일 native async session 연결은 다음 작업입니다. Tauri·TS caller는 아직 교체하지 않았습니다.

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test coordinator --test synchronization --test capabilities --test stopping --test real-process
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test transport-lifecycle
cargo clippy -p taide-lsp --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
```

제품 module 부재 E0432 red 뒤 제품을 직접 검사하는 기존 pure 13건·actual owned process 1건이 통과했습니다. actual process는 0.35초였습니다. disconnect API 부재 E0599 red 뒤 초기 variant가 Phase에 들어간 compile 오류를 발견해 Failure로 옮겼고, 신규 disconnect/deadline 2건은 0.00초에 통과했습니다. 두 strict clippy exit 0입니다. 새 method/variant를 쓰지 않는 이전 검사는 source가 같아 재실행하지 않았습니다. 실험 총 19건·infra framing/writer 6건·legacy 회귀 27건의 통과 증거를 유지합니다.

## 제품 비동기 session·store 소유권

대상: `crates/taide-lsp/src/native/session.rs`, infra의 cancel-safe receipt 재대기·native `submit_message`, 실험 `tests/native-session.rs`와 합성 서버의 ignore-hover/initialize 모드입니다. 기존 Tokio 1.53.1 local source에서 watch·oneshot의 cancel-safe 재대기와 channel 동작을 확인했습니다. dependency·root Cargo.lock·MSRV는 유지합니다.

- `SessionRunner::prepare`는 client와 실행할 runner를 반환하며 자체적으로 또 다른 runtime/supervisor를 만들지 않습니다. runner의 단일 event loop가 coordinator·wire queue·process slot·pending reply·deadline·cancellation을 소유합니다. native host는 이 future를 기존 TaskSupervisor에 등록해야 하며 실제 제품 host 연결은 아직 남았습니다.
- process는 기존 `LspStore::spawn_process` admission gate로 등록합니다. runner의 process guard는 작업 취소·Drop에서도 해당 child의 종료를 요청하고 store는 기존 process worker의 회수를 계속 소유합니다. restart는 이전 child의 writer/reader/wait join·is_exited 확인 뒤에만 generation을 증가시키고 새 child를 등록합니다. 새 handshake·replay write ack 전 Running을 게시하지 않습니다.
- actor는 `submit_message` receipt를 보유하고 cancel-safe `wait_ref`를 다른 사건과 함께 기다립니다. wire 쓰기로 actor 전체를 막지 않습니다. 마지막 write ack 뒤 replay delta를 다시 계산하며, command·incoming/outgoing frame 수와 byte budget을 caller가 선택합니다. notification consumer가 보유한 payload도 같은 incoming permit을 유지하고 이 budget은 generation 간 공유합니다. JSON Value·command/mirror URI·metadata·allocator overhead·OS pipe까지 포함한 전체 RSS 상한은 아닙니다.
- request waiter Drop은 bounded command 큐에 취소 메시지를 무조건 밀어 넣는 대신 atomic flag·Notify를 사용합니다. actor는 해당 pending을 취소하고 wire의 `$/cancelRequest`를 순서대로 보냅니다. 실제 request/initialize deadline과 write watchdog·exit grace·stop deadline 중 가장 빠른 시각까지 기다리며 idle polling을 하지 않습니다. initialize timeout의 TimedOut 원인은 child 회수 뒤에도 유지합니다.
- 이 단계에서는 정상 notification을 generation-tagged bounded 단일 subscription으로 전달했습니다. ready인 incoming 메시지를 exit 신호보다 먼저 처리합니다. 당시 미지원이던 server request와 generic notification은 아래의 typed 경계로 대체했고, 실제 native consumer·dynamic registration은 계속 남았습니다.
- stdout EOF·process exit·write 오류·강제 stop은 회수 뒤 pending을 정리합니다. Stopped는 child join과 is_exited가 확인된 경우에만 게시하며 stop reply보다 상태를 먼저 게시합니다. 회수 작업의 실패가 확인되면 Stopped 성공을 만들지 않고 session을 종료해 client가 Degraded/TransportClosed를 관찰합니다. host 강제 abort 시 store의 idle join은 별도로 필요합니다.

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test native-session
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test native-session native_owner는_stdout
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test transport-lifecycle
cargo test -p taide-infra --locked --offline --target-dir experiments/terminal-core-spike/target --test lsp-writer
cargo clippy -p taide-lsp -p taide-infra --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
```

session module 부재 E0432 red 뒤 정상/crash 연결 2건이 통과했고 notification subscription 부재 E0599 red 뒤 추가 경계를 구현했습니다. 최종 source에서 native session 6건이 0.83초에 통과했습니다. 실제 request deadline 500ms와 exit grace 20ms는 fixture 정책이며 제품 기본값 확정이 아닙니다. request cancellation은 watch의 실제 pending 등록·해제를 관찰하고, 정상 request·stop·crash/replay·ignore exit·worker abort·client drop·initialize timeout을 실제 process로 검사했습니다. 각 store의 shutdown·wait_for_idle과 runner join도 확인했습니다.

stdout만 닫는 추가 1건은 Unix에서 0.00초에 통과했습니다. 이 사례만 고정 `/bin/sh`의 `exec 1>&-; exec /bin/sleep 30`으로 같은 직접 child PID의 stdout을 닫고 살아 있게 만듭니다. 사용자 입력·profile·실제 프로젝트는 쓰지 않으며 별도 descendant를 만들지 않습니다. agent나 GUI의 stdout을 닫지 않고 unsafe FD ownership 재구성도 하지 않습니다. native actor가 EOF를 관찰해 해당 child를 회수하고 Degraded/TransportClosed·pending 0·pid 없음·후속 Stop/runner/store join을 확인했습니다.

disconnect 원인 보존 변경 영향 검사 2건과 receipt 재대기 변경 영향 검사 3건이 통과했고 양 strict clippy도 exit 0입니다. 그대로인 순수 기능 matrix·legacy process 경로는 기존 성공을 재사용했습니다. 실험 총 26건·infra framing/writer 6건·legacy 회귀 27건의 통과 증거를 유지하지만 native UI·전체 LSP 기능 동등성 완료는 아닙니다.

## 서버 요청·타입별 알림 경계

대상: `crates/taide-lsp/src/native/protocol.rs`, `native/session.rs`, 제품·실험 Cargo lock, 합성 서버와 `tests/{protocol,server-messages}.rs`입니다. 기존 TS의 per-session applyEdit·semanticTokens/codeLens refresh와 process-wide configuration fallback을 대조했습니다. 표준 DTO를 수동 재정의하지 않기 위해 [lsp-types 0.97.0의 MIT manifest](https://raw.githubusercontent.com/gluon-lang/lsp-types/master/Cargo.toml)를 확인해 재사용하며, 기존 serde/serde_json/serde_repr/bitflags를 공유합니다. root lock에는 lsp-types와 fluent-uri 2 package만 추가됐고 실험 lock에는 이미 root에 있던 serde_repr도 들어갑니다. 두 새 package는 manifest에 MSRV를 명시하지 않습니다. 1.98.1에서 실제 build·검사가 통과한 사실을 1.89 실행 검증으로 대체하지 않습니다.

- [LSP 3.17 base protocol](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/specification.md)에 따라 서버 request ID는 signed 32-bit integer/string으로 검증합니다. request/notification/response 혼합, result/error 중복, 잘못된 params와 error code를 구분합니다. unknown request는 MethodNotFound, known invalid params는 InvalidParams로 응답하고 notification에는 응답하지 않습니다. 외부 detail·원본 경로는 오류 메시지에 붙이지 않습니다. 당시 남은 pure coordinator의 outgoing ID 범위와 feature 응답 DTO는 아래 별도 단계에서 보완했으며 전체 native 기능 동등성 완료는 아닙니다.
- configuration·applyEdit·workDoneProgress/create·여섯 refresh·workspaceFolders·showMessageRequest를 typed host port로 전달합니다. 한 subscription과 incoming budget을 공유하고 request가 pending이거나 consumer가 보유한 동안 permit을 유지합니다. pending 수·등록된 work-done token 수는 caller의 incoming frame 수 상한을 따릅니다. URI 문자열·token·파싱된 DTO 및 allocator overhead까지 포괄하는 전체 메모리 budget은 아직 아닙니다.
- reply ticket은 생성한 session의 private identity·generation·단조 serial에 묶입니다. 다른 session의 같은 request ID, 이미 응답한 ticket, timeout 후 ticket, restart 이전 ticket은 wire 전송 전에 거절합니다. host reply는 해당 request의 결과 종류·configuration 항목 수·showMessage action membership을 검사합니다. applyEdit 실패 이유는 `edit rejected`로 제한합니다. 이 port가 파일 편집을 수행하거나 allowed roots를 승인하는 것은 아닙니다. native DocumentStore의 live root/version guard·host handler를 구현해야 합니다.
- server 요청 기한에는 generic RequestFailed 응답을 보내고 pending을 해제합니다. stop은 남은 server pending을 취소하며 reap/restart는 pending과 token을 폐기합니다. incoming cancel은 해당 server pending에 RequestCancelled를 보냅니다. 실제 cancellation·queue 포화·consumer drop 분기 전체는 이번 추가 검사 범위가 아니며 native 활성화 전 lifecycle gate에 포함합니다.
- diagnostics는 URI·version·range·severity·code 등을 타입 검증하고 signed integer/uinteger range와 optional non-null 규칙을 보완합니다. 전체 원본 envelope도 notice에 유지하므로 `data:null`과 unknown diagnostic metadata를 잃지 않습니다. empty clear batch도 parser 계약으로 검사했습니다. 실제 진단 store의 replace/version/stale 정책은 아직 구현하지 않았습니다.
- progress는 payload의 `kind`만 보고 추측하지 않습니다. 이 단계에서는 host가 create 요청을 승인한 session token만 work-done DTO와 percentage로 검사하며 end에 token을 해제했습니다. 아래 client 진행 수명 단계에서 initialize·feature 요청의 token도 단일 registry로 연결했습니다. 미등록 token의 partial 값은 object/array/scalar/null을 그대로 보존하므로 같은 `kind`가 있어도 임의 partial 결과를 거절하지 않습니다. native UI 소비자는 다음 전체 adapter 단계입니다. log/showMessage도 typed notice이며 자동으로 외부 로그에 출력하지 않습니다.

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test protocol --test server-messages --test native-session
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test protocol --test server-messages
cargo clippy -p taide-lsp --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
```

먼저 valid configuration server request가 전달되지 않아 3초 timeout으로 실패하는 red를 재현했습니다. 새 검사 파일의 괄호 오류를 수정한 뒤 신규 parser 3건(0.00초)·실제 server 왕복/timeout·다른 session/restart 거절 2건(0.51초)·영향 native session 7건(0.85초)이 통과했습니다. 이후 progress를 session token으로 구분하는 관련 source 변경 뒤 신규 5건만 한 번 재실행해 3건 0.00초·2건 0.96초로 통과했고, native session 7건과 이전 unchanged 검사 결과는 재사용했습니다. 최종 양 strict clippy exit 0입니다. 실험 총 31개 검사의 성공 증거를 유지하며 이 수치는 전체 기능 parity나 M8 완료 수가 아닙니다.

## 동적 feature 등록·문서 selector

대상: `native/{registration,selector,protocol,session}.rs`, coordinator, 합성 서버의 dynamic-registration 모드, `tests/{registration,dynamic-session}.rs`와 제품 session의 duplicate ID unit입니다. 기존 TS는 synchronization.dynamicRegistration을 false로 광고하고 register/unregister fallback은 null만 반환했습니다. 새 native 경계는 실제 등록 상태를 소유하며 기존 TS 실행 경로를 바꾸지 않습니다.

- [register](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_includes/messages/3.17/registerCapability.md)·[unregister](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_includes/messages/3.17/unregisterCapability.md)의 method별 client opt-in을 검사합니다. 지원하지 않은 method·광고하지 않은 dynamic capability는 성공 처리하지 않습니다. base feature 등록이 해당 prepare/resolve/semantic full/delta 세부 guard에도 적용되며 workspace executeCommand는 등록된 command 이름만 허용합니다. 이미 static으로 지원한 기능은 별도 static 광고가 유지됩니다.
- batch의 ID 중복·method·옵션·selector·상한을 모두 확인한 뒤 한 번에 반영합니다. 실패한 batch는 이전 record·revision을 바꾸지 않습니다. unregister는 정확한 ID/method 쌍과 표준 3.17의 `unregisterations` 필드를 사용합니다. 등록 수 256·serialized registration 총량 1MiB·selector filter 총수 256·패턴 4KiB·중첩 32단계·개별 compiled regex 64KiB를 제한합니다. 이 수치는 DTO/regex allocator를 포함한 전체 RSS 상한은 아닙니다.
- [DocumentFilter](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/documentFilter.md)의 language/scheme/pattern을 AND로, 여러 filter를 OR로 판정합니다. empty selector는 아무 문서도 매칭하지 않고 null selector는 이 session에 제공된 mirror 집합을 사용합니다. URI를 파싱하고 percent-decoded UTF-8 path에 pattern을 적용하며 file 권한을 승인하거나 경로를 열지 않습니다. 문서와 다른 params URI는 전송 전에 거절합니다.
- `*`, `?`, `**`, brace OR, character class/range/negation을 기존 regex 1.13.1로 컴파일합니다. regex는 Unicode 문자 기준·path separator 구분·anchor·compiled size 상한을 명시합니다. 기존 globset source의 byte regex와 escaped UTF-8 literal은 Unicode `?`/class 의미와 달라 사용하지 않았습니다. regex는 이미 root lock에 있으며 direct 의존성만 추가했고 새로운 root package는 없습니다. 해당 manifest의 MSRV 1.65를 확인했고 제품 1.89 선언은 유지합니다.
- native session이 register/unregister 요청을 coordinator에 적용한 뒤 null 결과 또는 generic 오류를 FIFO wire queue에 넣습니다. host로 넘긴 기존 요청과 충돌하는 server ID는 자동 제어 응답보다 먼저 거절합니다. 등록 count/revision을 watch snapshot에 게시해 향후 native provider가 변경을 관찰할 수 있게 합니다. restart/disconnect는 registry를 폐기하고 이전 generation의 제어 요청을 거절합니다. 실제 native provider의 등록·폐기·pending result 소비자는 아직 없습니다.
- lsp-types 0.97.0의 ExecuteCommandRegistrationOptions는 `commands`를 직접 정의하면서 같은 required field가 있는 ExecuteCommandOptions를 flatten하여 정상 JSON을 거절했습니다. [공식 계약](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/workspace/executeCommand.md)은 추가 field 없는 확장이므로 wire shape가 같은 표준 ExecuteCommandOptions로 검증합니다. 중복 DTO를 새로 쓰거나 upstream dependency source를 변경하지 않았습니다.

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --offline --target-dir experiments/terminal-core-spike/target --test registration --test capabilities --test protocol
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test registration dynamic_옵션
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test dynamic-session --test native-session --test server-messages
cargo test -p taide-lsp --locked --offline --target-dir experiments/terminal-core-spike/target --lib duplicate_server_id
```

URI wrapper에 없는 decode method 호출 E0599를 source의 `as_estr().decode().into_string()` API로 바로잡았습니다. selector·atomic batch 2건과 영향받은 기존 capability/protocol 6건이 통과했고 옵션 검사는 위의 upstream duplicate field 문제로 MalformedResponse 실패를 재현했습니다. 공식 shape에 맞춰 변경하고 검사 fixture의 두 번째 finish_replay 중복 호출도 제거한 뒤 실패한 옵션 검사만 한 번 재실행하여 0.00초에 통과했습니다. 실제 dynamic session 1건은 0.32초, 영향받은 native session 7건·server-message 2건은 각각 0.51초에 통과했습니다. 마지막 duplicate ID 선행 거절 변경은 별도 process 없는 actor unit 1건으로 0.00초에 확인했고 최종 양 strict clippy exit 0입니다. 실험 총 35건과 별도 제품 unit 1건의 성공 증거를 유지합니다. 그대로인 성공 검사는 다시 실행하지 않았습니다.

정적인 초기 광고의 registration ID·documentSelector를 unregister 가능한 record로 반영하는 작업, synchronization·configuration/watcher·file-operation 동적 기능, 실제 native provider·feature DTO 전체 matrix는 아직 미완료입니다. 이번 helper의 true capability 조회는 사용 가능한 등록이 존재한다는 뜻이며 모든 문서에 적용된다는 뜻이 아닙니다. 실제 요청은 mirror selector를 별도로 검사합니다. 임의 malformed payload·queue 포화·장기 register/unregister 반복의 전체 lifecycle/성능 검사는 native 활성화 전 gate로 유지합니다.

## 기능별 Params/Reply·원형 보존

대상: `native/feature.rs`, `native/session.rs`, `tests/feature.rs`, 실제 native/dynamic fixture와 mock server의 malformed-hover 모드입니다. 기존 capability table의 27개 기능을 같은 단일 method table로 검사하며 Params 27개와 Result 26개는 `lsp-types::request::Request`의 연관 타입에서 유도합니다. `FeatureRequest::Reply`는 일반 기능에서는 원본 Result이고 semantic delta 하나만 아래의 표준 보완 타입입니다. `request_typed`와 `TypedReply`는 같은 계약을 사용하므로 표준 SemanticTokensFullDeltaRequest marker도 보완된 결과를 받습니다. 별도 우회 marker나 수정한 vendor는 없습니다.

- native actor는 malformed Params를 pending 등록·wire 전송 전에 거절합니다. 정상 envelope지만 feature의 malformed Result인 경우 해당 pending만 MalformedResponse로 끝내고 session과 child는 유지합니다. 실제 후속 요청·stop·runner/store join을 확인했습니다. extension method의 object/array Params와 임의 Result는 이 표준 feature DTO로 오해하지 않습니다.
- typed projection에서 알려진 URI의 absolute scheme·position의 uinteger 상한·range 순서·folding line 및 optional character·semantic full/delta 숫자 경계를 검사합니다. WorkspaceEdit의 changes URI key도 검사합니다. data·arguments·experimental과 executeCommand의 임의 Result는 좌표로 재해석하지 않으며 원본 JSON metadata·data:null을 유지합니다. formatter의 flattened 사용자 속성도 표준 좌표 필드로 오해하지 않습니다.
- [공식 semantic token 계약](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/language/semanticTokens.md)은 delta를 숫자 배열의 임의 부분 변경으로 정의하며 숫자 하나를 교체하는 예시를 제공합니다. upstream 0.97.0은 SemanticTokensEdit.data를 Vec<SemanticToken>으로 역직렬화해 길이가 5의 배수가 아닌 합법적인 `[1]`을 거절했습니다. 최초 3건 중 2건은 통과하고 이 사례만 실패했습니다. 이 Result만 표준의 start/deleteCount/data uinteger 배열로 보완하며 full Result와 Params는 기존 타입을 재사용합니다. 기존 serde 1의 derive를 direct 의존성으로 추가했고 양 lock은 같은 기존 package를 재사용합니다. 새 package·MSRV 선언 변경은 없습니다.
- lsp-types 0.97.0의 CompletionList는 3.17의 itemDefaults를 표현하지 않습니다. [공식 completion 계약](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/language/completion.md)에 따라 raw itemDefaults의 known editRange·commitCharacters·format/mode를 별도 검증하고 원형을 보존합니다. 실제 native completion adapter에서 default를 소비하는 구현은 아직 없으므로 클라이언트의 itemDefaults 기능을 완료했다고 주장하거나 광고하지 않습니다.

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test feature
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test feature position_range_uri
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test native-session --test dynamic-session
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --test native-session 잘못된_feature
cargo clippy -p taide-lsp --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
```

실패한 delta 경계만 수정 뒤 1건 0.00초에 통과했습니다. 보완 DTO의 derive 사용에 필요한 기존 serde의 direct 의존성 누락으로 E0432가 발생한 것을 바로잡았습니다. 영향을 받는 실제 native session 8건 0.51초·dynamic session 1건 0.36초가 통과했습니다. 표준 marker의 native Reply 연관 타입을 정리한 최종 변경 뒤 feature 3건 0.00초·실제 malformed 요청 격리 1건 0.44초가 통과했고 양 strict clippy exit 0입니다. 하나의 cargo 호출에 붙인 이름 filter가 feature target에도 적용돼 0건으로 출력된 결과는 검증 근거에서 제외하고 feature target을 별도 실행했습니다. unchanged server-message·capability·등록·transport 성공 검사는 재사용합니다. 총 실험 39건과 별도 제품 unit 1건의 성공 증거이며 전체 LSP parity 수치가 아닙니다.

알려진 타입/좌표의 wire validation은 실제 DocumentStore의 UTF-16 offset 변환·clamping·live revision 및 allowed root 확인, WorkspaceEdit 원자적 적용, semantic delta의 이전 결과 적용·legend 검사·native 화면 표시를 대체하지 않습니다. 모든 feature의 세부 semantic invariant·client work-done 수명·signature tuple UTF-16 label 검사와 전체 RSS/성능도 선행 gate로 남습니다. typed/raw/projection을 함께 만드는 추가 할당은 장기 memory 및 latency 검증에 포함해야 합니다.

## 기존 host 감독자·root 종료 연결

대상: `crates/taide-runtime/src/native_lsp_actions.rs`, module export와 `tests/native_lsp.rs`입니다. 제품 runtime은 이미 taide-lsp·infra에 의존하므로 dependency·lock·MSRV 변경은 없습니다. 기존 TaskSupervisor·ExitDrain·LspStore의 코드와 종료 순서를 읽고 재사용했습니다.

- `spawn_session`은 먼저 child를 만들지 않는 SessionRunner::prepare로 옵션과 채널을 준비한 뒤 기존 supervisor의 `spawn_transient`에 runner future를 등록합니다. admission이 닫혔으면 future를 실행하지 않고 TransportClosed로 실패합니다. malformed 옵션도 Capacity로 반환하며 추적 작업이 생기지 않습니다. 별도 Tokio runtime·detached process registry·신규 작업 감독자는 없습니다.
- 실제 fixture는 Unix `/bin/sleep`의 직접 child 하나만 실행해 초기화 응답을 주지 않으며 watcher/profile/GUI/사용자 파일에 접근하지 않습니다. watch에서 Initializing과 실제 PID를 관찰한 후 기존 wait_for_direct_exit을 호출했습니다. 기존 경로가 store/process admission을 닫고 child termination을 요청하며 supervisor의 runner abort와 store worker completion을 모두 기다립니다. 이후 tracked_count 0·client Degraded/TransportClosed·새 session admission 거절을 확인했습니다. actor가 강제 취소됐으므로 이를 graceful LSP shutdown→exit 또는 Stopped snapshot 증거로 표시하지 않습니다.

```sh
cargo test -p taide-runtime --locked --offline --target-dir experiments/terminal-core-spike/target --test native_lsp
cargo clippy -p taide-runtime --locked --offline --target-dir experiments/terminal-core-spike/target --all-targets -- -D warnings
```

host module 부재 E0432 red 뒤 구현한 최소 2건이 0.02초에 통과했고 runtime strict clippy는 4.13초·exit 0입니다. 기존 native session·feature DTO·infra 검사는 변경이 없어 성공 결과를 재사용합니다. 이 2건은 별도 runtime 검사이며 기존 실험 39건에 더해 같은 실험 검사를 반복했다고 계산하지 않습니다. 실제 native composition root가 이 action을 호출하는 연결, Running 서버의 host graceful 종료와 다중 session·command/provider 수명은 다음 단계입니다.

## Client work-done·partial token 수명

대상: 제품 `native/progress.rs`, `native/session.rs`, 실험의 `tests/client-progress.rs`와 합성 서버 `--client-progress` mode입니다. 제품·실험 dependency와 lockfile은 이 변경에서 바꾸지 않았습니다. [LSP 3.17 work-done 계약](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/types/workDoneProgress.md)은 client token의 유효 기간을 요청 응답까지로 정하며 진행 취소를 해당 요청 취소에 연결합니다. initialize의 같은 token도 처리합니다.

단일 bounded registry가 server-created work-done, client request-owned work-done, partial-result token을 구별합니다. 수 상한은 기존 caller `incoming_capacity`를 그대로 사용하며 partial token도 포함합니다. token은 원본 `lsp_types::NumberOrString`으로 검증하고 null·boolean·i32 범위 밖 수, 동일 요청 내 두 token의 충돌, 세션 내 다른 request/server token의 중복을 wire 전송 전에 거절합니다. request admission 실패는 token을 예약하지 않습니다.

Client의 end 알림은 typed work-done으로 전달하지만 요청 응답 전까지 token을 예약해 새 요청과의 재사용 경합을 막습니다. 응답·개별 malformed reply·취소·timeout의 coordinator completion이 모든 해당 request token을 정리합니다. initialize completion도 feature waiter 유무와 무관하게 정리합니다. Server의 end는 그 등록만 해제하며 reap/restart는 registry 전체를 폐기합니다. initialize progress를 새 generation에 다시 등록하고 `SessionSnapshot.progress_tokens`로 예약 수를 게시합니다. 부분 결과와 미등록/끝난 token의 raw 값은 `kind`만으로 work-done으로 추측하지 않습니다.

```sh
cargo test -p taide-lsp --lib native::progress --locked --offline --target-dir experiments/terminal-core-spike/target
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test client-progress --locked --offline --target-dir experiments/terminal-core-spike/target
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test server-messages --locked --offline --target-dir experiments/terminal-core-spike/target
cargo clippy -p taide-lsp --lib --locked --offline --target-dir experiments/terminal-core-spike/target -- -D warnings
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --bin mock-server --test client-progress --locked --offline --target-dir experiments/terminal-core-spike/target -- -D warnings
```

Snapshot field 부재 E0609 red를 먼저 확인했습니다. Registry unit 1건은 0.00초, 실제 client 진행 왕복 1건은 최종 fixture에서 0.65초, 변경 영향을 받는 기존 server 요청 2건은 0.65초에 통과했습니다. 양쪽 대상 strict clippy exit 0이며 같은 성공 검사는 반복하지 않습니다. 신규 mode 외의 initialize 광고는 기존 fixture와 같은 형태를 유지합니다.

처음 신규 test에만 100ms request deadline을 사용해 초기화 대기가 두 번 실패했습니다. 두 번째에는 watch snapshot으로 Degraded/TimedOut·pending/token 0·pid 없음을 확인했습니다. 합성 서버 직접 framed 입력은 begin/report·응답·뒤늦은 알림을 출력했고 입력 EOF로 exit 1이었으며 성공 테스트 증거로 세지 않습니다. 기존 native/server 실험의 500ms request deadline과 일치시킨 최종 test는 통과했습니다. 제품 timeout을 늘린 것이 아니며, 100ms 초과의 세부 원인이나 제품 초기화 성능을 규명한 것으로 주장하지 않습니다. 제품 활성화 전 성능 gate에서 child 시작·첫 write·read·initialize response 경계를 측정해야 합니다.

- [x] Unit: malformed token·동일/서로 다른 요청 충돌·server token 충돌·상한의 무변경 실패, end 뒤 요청 완료까지 예약 유지, 다른 ID의 completion 무효, server end 해제, clear와 arbitrary partial object 보존을 확인했습니다.
- [x] 실제 mode: initialize 응답 전 begin/report를 typed로 전달하고 응답 후 같은 token의 `title:false` 원형을 partial로 보존했습니다. definition 요청의 begin·partial·report·end·응답 뒤 알림, 중복 요청 거절, waiter 취소의 실제 cancel wire·뒤늦은 두 알림과 token 0, 500ms timeout의 cancel·뒤늦은 알림·token 0, restart의 새 generation initialize 재등록, stop·runner/store join을 확인했습니다.
- [x] 위 최초 실제 검사에서는 restart와 stop 직전에 client progress 예약이 이미 0이었습니다. 아래 추가 검사는 활성 두 요청·네 token을 가진 실제 session의 restart/stop 경계를 확인합니다. 실제 native GUI/host의 graceful 종료·crash/provider 수명은 별도 composition 연결 gate로 유지합니다.
- [ ] native UI monitor 생성/종료·사용자 cancel, server-created progress의 cancel command·capability 광고와 host 정책, 엄격한 begin/report/end 순서·percentage 표시 정책, typed partial result 소비와 전체 metadata/RSS budget·성능은 남습니다. 과거 work-done 알림의 payload kind 검증은 전체 native 진행 UI 완료가 아닙니다.

## 활성 두 요청의 실제 session restart·stop

대상은 `tests/client-progress.rs`의 `활성_두_요청과_네_진행_token의_실제_session_restart_stop은_owner와_child를_남기지_않는다`와 합성 서버의 held request map입니다. 제품 LSP·runtime source, 제품/실험 dependency·lockfile은 이번 추가 검사에서 바꾸지 않았습니다. 두 테스트가 같은 prepare 설정을 사용하도록 helper를 추출했으며 기존 성공 테스트의 입력과 설정은 유지합니다.

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test client-progress token --locked --offline --target-dir experiments/terminal-core-spike/target
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --bin mock-server --test client-progress --locked --offline --target-dir experiments/terminal-core-spike/target -- -D warnings
rustfmt --edition 2021 --check experiments/lsp-coordinator-spike/src/bin/mock-server.rs experiments/lsp-coordinator-spike/tests/client-progress.rs
git diff --check
```

최초 한글 filter 실행은 `0 tests / 2 filtered out`이어서 성공 증거에서 제외했습니다. 실제 신규 검사만 고르는 ASCII `token` filter로 수정했습니다. 신규 검사는 기존 fixture가 마지막 held 요청 하나만 보관해 첫 cancel에서 연결을 닫는 실패를 재현했습니다. 합성 서버를 numeric request ID별 map으로 변경하고, 같은 실패 검사만 한 번 재실행해 `1 passed / 1 filtered out`, 0.44초를 확인했습니다. 기존 정상/취소/timeout 검사, registry unit과 제품 검사는 반복하지 않았습니다. 합성 서버의 method/frame 상한은 유지하며 다른 fixture mode는 바꾸지 않습니다.

- [x] 동일 session에서 두 definition 요청과 네 work-done/partial token을 실제로 등록했습니다. pending 2·token 4 상태에서 restart하고 두 waiter의 Restarted, 새 generation·child PID, initialize token의 typed begin/report·응답 뒤 partial, pending/token 0을 확인했습니다.
- [x] 다음 generation에서 같은 token들을 다른 두 request owner에 다시 등록했습니다. pending 2·token 4 상태에서 stop하고 두 waiter의 Stopped, 각 numeric ID의 실제 cancel에 대한 네 late partial 알림, 최종 Stopped·pending/token 0·pid 없음, runner 완료와 store idle join을 확인했습니다.
- [x] 대상 strict clippy는 exit 0(0.46초)이며 대상 Rust format·diff 검사도 exit 0입니다. 제품 source가 바뀌지 않아 제품의 기존 검사 결과를 재사용합니다.

검사는 직접 `SessionRunner`에 붙인 실제 합성 process입니다. root `TaskSupervisor`, 사용자 project/provider, native GUI의 진행 monitor를 연결한 종료 증거는 아닙니다. macOS의 이 실행에서 관찰한 서로 다른 PID는 프로세스 identity 전체 계약을 대체하지 않으며, 실제 child ownership과 회수는 기존 owned process·store join 경계를 사용합니다. 서로 다른 request owner와 generation을 PID만으로 판정하지 않습니다. 실제 server crash·지속적인 신규 요청·backpressure 중 진행 UI는 native composition/soak gate에 남습니다.

## 내부 요청 identity와 표준 wire ID 분리

대상: 제품 `native.rs`와 `native/session.rs`의 요청 생성·응답 매칭·취소와 unit 검사입니다. 제품/실험 dependency·lockfile과 합성 서버는 이번 변경에서 수정하지 않았습니다. [LSP 3.17 base protocol](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/specification.md)의 integer는 signed 32-bit 범위이며 RequestMessage와 CancelParams의 ID는 integer 또는 string입니다. 내부 단조 u64를 그대로 숫자로 전송하면 이 범위를 벗어나므로 internal identity와 wire 표현을 분리했습니다.

- 기존 0부터 i32::MAX까지의 요청은 같은 numeric ID를 전송합니다. 이후는 `taide-native-request:`와 내부 decimal ID를 합친 고유 string을 원본 `lsp_types::NumberOrString`으로 생성합니다. 내부 pending/completion/progress owner는 계속 u64를 사용하고 restart에서 counter를 되감거나 순환시키지 않습니다. initialize·일반 요청·shutdown과 명시적 cancel·timeout·stop의 취소 알림이 모두 같은 변환을 사용합니다.
- 응답은 같은 원본 DTO로 검증한 뒤 internal ID로 변환합니다. 정확한 wire type과 표기가 다시 일치할 때만 소유권을 매칭합니다. 음수·알 수 없는 문자열·숫자처럼 보이는 문자열·prefix 뒤의 leading zero는 다른 pending을 소비하지 않으며 i32 밖 numeric ID는 malformed입니다. 기존 generation guard는 ID decoding보다 먼저 적용됩니다.
- actor의 request admission과 initialize 진행 등록도 같은 decode를 사용합니다. 별도 수동 integer/string DTO나 새 dependency는 없습니다. 테스트의 prepare helper는 기존 duplicate-server-ID unit과 같은 options를 재사용하며 실제 child를 시작하지 않습니다.

```sh
cargo test -p taide-lsp --lib wire_id --locked --offline --target-dir experiments/terminal-core-spike/target
cargo test -p taide-lsp --lib wire_id_actor --locked --offline --target-dir experiments/terminal-core-spike/target
cargo clippy -p taide-lsp --lib --tests --locked --offline --target-dir experiments/terminal-core-spike/target -- -D warnings
rustfmt --edition 2021 --check --config skip_children=true crates/taide-lsp/src/native.rs crates/taide-lsp/src/native/session.rs
git diff --check
```

최초 coordinator 경계 검사는 기존의 `2147483648` numeric ID를 관찰해 실패했습니다.

수정 뒤 coordinator 검사 `1 passed / 69 filtered out`, 0.00초를 확인했습니다. 마지막 합법적 numeric initialize ID, 첫 string 요청, malformed numeric·미등록/다른 표기의 응답이 pending을 유지하는 경계, string cancel과 late reply, timeout, restart/stale generation, 활성 요청 stop·string shutdown 응답·exit·회수 acknowledgement를 한 검사에 포함합니다.

추가 actor 검사 `1 passed / 70 filtered out`, 0.00초를 확인했습니다. high-ID initialize 진행 등록/정리, typed hover의 string ID와 pending/token 1, raw experimental metadata 보존·응답 뒤 pending/token 0, 후속 string cancel과 Cancelled waiter·pending/token 0을 직접 actor port에서 검사합니다. 이는 실제 transport 또는 언어 서버 실행 증거가 아닙니다.

대상 strict clippy는 exit 0(0.49초), 대상 Rust format과 diff 검사도 exit 0입니다. 성공한 신규 검사는 반복하지 않았습니다. 일반 numeric ID의 wire 형태는 유지되며 해당 경로의 이전 실제 process 성공 증거를 재사용합니다. 이번 제품 source 변경이 실제 서버의 string ID 연동이나 전체 session matrix를 통과했다는 주장은 하지 않습니다.

실제 언어 서버별 string ID 왕복·전체 native feature adapter·RSS/latency는 남습니다. null response ID의 host 오류 정책, typed error의 전체 계약과 u64 counter exhaustion의 host 처리는 이번 수정 범위 밖이며 제품 활성화 전 경계 검토 대상입니다. 이 단위 검사들은 canonical DocumentStore·GUI composition 또는 native LSP parity 완료 판정을 대체하지 않습니다.

## Running runtime host의 명시적 stop·root direct Exit

대상은 `experiments/lsp-coordinator-spike/tests/runtime-host.rs`, 격리 manifest/lock과 제품 `native/session.rs::SessionClient::snapshot`입니다. 격리 dev dependency로 기존 taide-runtime을 path 재사용하며 새로운 product crate·감독자·GUI host·제품 dependency 또는 root lock 변경은 없습니다. runtime이 필요로 하는 기존 Git/remote 등의 dependency도 격리 lock graph에 포함되므로 단순 LSP-only graph와 같다고 주장하지 않습니다.

처음 offline 해석은 기존 제품 lock의 libssh2-sys 0.3.2가 yanked라 새 graph에서 거절돼 exit 101로 끝났고 테스트를 실행하지 못했습니다. [Cargo resolver](https://doc.rust-lang.org/cargo/reference/resolver.html#yanked-versions)의 기존 lock 재사용 계약을 확인하고 root lock의 동일 package/version/source/checksum/dependency record만 격리 lock에 옮겼습니다. 이후 Cargo가 나머지 runtime 경로의 49개 record를 해석했습니다. libssh2의 기존 버전 위험을 없앴다는 주장이 아니며 제품 버전 변경·alternate source·dependency patch·검사 억제는 하지 않았습니다. 격리 lock은 기존 실험의 다른 transitive 버전을 유지하므로 제품 lock 전체와 동일한 빌드라고 표시하지 않습니다.

한 Tokio runtime의 같은 TaskSupervisor·LspStore에 서로 다른 두 실제 child를 등록했습니다. 각 native session의 Running·실제 initialize 진행·문서 open·held definition 요청과 work-done/partial 두 token을 확인한 뒤 다음 두 경계를 연속 검사합니다.

1. 첫 session을 명시적으로 stop합니다. pending waiter는 Stopped, late progress 알림은 별도 notice로 전달되고 stop 완료 뒤 phase Stopped·pending/token 0·pid 없음입니다. 기존 stop 경로의 shutdown reply→exit→owned worker completion을 사용하며 root 종료가 자동으로 모든 session을 graceful stop한다는 증거로 확대하지 않습니다.
2. 두 번째 session이 Running·pending 1인 상태에서 기존 ExitDrain::wait_for_direct_exit을 호출합니다. store admission 종료·child kill·runner abort·store의 reader/writer/wait completion을 기다린 뒤 waiter TransportClosed·작업 추적 0·Degraded/TransportClosed와 신규 admission 거절을 확인합니다. 이는 강제 종료이며 protocol graceful 또는 Stopped로 표기하지 않습니다.

최초 owner 완료 검사는 1건 0.36초에 통과했지만 폐기된 pending 수 assertion을 포함하지 않았습니다. snapshot의 해당 assertion을 보완하니 root 회수 뒤 실제 pending 객체는 해제됐어도 마지막 watch snapshot에 pending 1이 남아 exit 101(0.17초)로 실패했습니다. SessionClient::snapshot이 closed watch를 Degraded로 표시할 때 pending·server_pending·progress_tokens·registrations도 0으로 정정했습니다. generation과 마지막 관찰 PID 등 진단 정보는 보존하며, 이 값들은 OS child의 join 완료 표시가 아닙니다. subscribe의 raw watch 소비자는 기존 계약대로 채널 종료를 직접 처리해야 하고 이번 accessor 수정을 raw watch의 새로운 publication이라고 설명하지 않습니다.

```sh
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test runtime-host --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
cargo test -p taide-lsp --lib closed_owner_snapshot --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test runtime-host --target-dir experiments/terminal-core-spike/target --locked --offline -- -D warnings
cargo clippy -p taide-lsp --lib --tests --target-dir experiments/terminal-core-spike/target --locked --offline -- -D warnings
```

수정 후 runtime host 1건이 0.33초, 종료된 owner의 네 count/generation·failure snapshot unit 1건이 0.00초에 통과했습니다. 대상 strict clippy는 각각 0.55초와 0.38초·exit 0입니다. 초기 불완전 assertion 통과·추가 assertion 실패·수정 뒤 최종 통과를 같은 입력의 반복 성공으로 합산하지 않습니다. 이전 initialize host·일반 session·server/feature/process 성공 검사는 재실행하지 않았습니다.

이 검사는 합성 framed server와 제품 runtime action의 실제 OS child 연결입니다. native 창 close callback·GUI composition root·실제 language server·root의 자동 graceful 정책·모든 provider와 root drain의 동시 부하·전체 메모리/성능 완료가 아닙니다. 사용자 실기 앱·입력기·VoiceOver·사용자 project/profile/설정은 조작하지 않았고 CPU fallback 방향 질문은 반복하지 않았습니다.

## Payload 소유·송신 직렬화 budget 선행

대상은 제품 `native/session.rs::enqueue`, private BoundedJsonPayload와 해당 단일 unit입니다. 기존 SessionOptions·frame/outgoing 상한·writer 정책·제품 dependency·lockfile·MSRV·GUI 후보는 바꾸지 않았습니다.

송신 직렬화 보완 직후·아래 command admission 추가 전의 source 대조 결과는 다음과 같습니다. 상한이 있는 개별 계층을 합쳐서 전체 RSS 보장으로 설명하지 않습니다.

| 소유 계층                          | 현재 상한/유지 기간                                                         | 남은 경계                                                                                                    |
| ---------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| infra wire writer                  | caller WriterLimits의 queued+inflight frame/byte permit                     | 이미 만들어진 caller JSON Value·payload·allocator/OS pipe는 별도                                             |
| actor outgoing                     | caller outgoing count/byte와 frame body; queued payload를 writer로 이동     | 이번 bounded serialization으로 복사 전에 logical byte 상한 적용                                              |
| incoming/notice/server pending     | caller frame/count/byte permit을 공유하고 consumer 보유 동안 유지           | 원형 JSON·typed projection·metadata 복사와 object overhead는 byte permit의 정확한 RSS가 아님                 |
| command envelope                   | caller command 개수 상한, owned Value/String/config·reply                   | byte admission이 아직 없고 queued command는 actor validation 전 payload를 보유                               |
| coordinator mirror/replay          | 열린 문서 256·현 mirror text 합계 64MiB, replay는 별도 snapshot 복사        | URI/language metadata는 text 합산에서 빠지며 key·pending URI·replay·outgoing Value 복사를 별도로 제한해야 함 |
| initialize/capability/registration | initialize는 runner에 보존·restart에서 복사, registry는 256·serialized 1MiB | prepare 이전/초기 metadata·static capability와 parsed allocation 전체 budget은 미완료                        |

기존 enqueue는 serde_json::to_string으로 완전한 String을 만든 뒤 body/aggregate byte 상한을 검사했습니다. 큰 caller Value를 이미 보유하는 문제와 별개로, 거절할 oversized 직렬화 문자열을 먼저 추가 할당하는 경계였습니다. 또한 batch의 앞 프레임을 queue에 반영한 뒤 뒤 프레임을 거절했습니다. 신규 unit은 small+oversized batch 거절 뒤 outgoing_bytes의 관찰값 60/기대값 0으로 실패했습니다(exit 101, 0.00초).

[serde_json to_writer](https://docs.rs/serde_json/1.0.151/serde_json/fn.to_writer.html), [Rust Write](https://doc.rust-lang.org/std/io/trait.Write.html), [Vec try_reserve_exact](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.try_reserve_exact)의 계약을 확인했습니다. private writer는 매 write의 checked length를 body·남은 outgoing byte budget 이하로 확인한 뒤 fallible reservation과 copy를 수행합니다. 일반 Value의 UTF-8 JSON과 escaping은 기존 serde_json을 재사용하고 content를 자체 구현하지 않습니다. geometric reservation의 요청 용량도 limit에 제한하며 allocator 실제 rounding·object overhead를 process RSS 상한이라고 주장하지 않습니다. 오류의 사용자 payload를 로그에 남기지 않습니다.

batch는 local frame 목록으로 직렬화하고 전체 성공·queue reservation 뒤에만 queue와 byte counter에 한 번 반영합니다. 실패 시 local buffer를 폐기해 기존 queue 내용·counter를 유지합니다. String::from_utf8은 Vec 소유권을 이동하며 JSON 원문을 다시 복사하지 않습니다. 입력 Value·coordinator Outcome·mirror snapshot의 기존 할당은 이 sink가 소급 제한하지 못합니다. allocation failure의 fallible 분기는 구현했지만 실제 OS OOM 주입 검사를 수행하지 않았습니다.

```sh
cargo test -p taide-lsp --lib outgoing_payload_budget --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
cargo clippy -p taide-lsp --lib --tests --target-dir experiments/terminal-core-spike/target --locked --offline -- -D warnings
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test runtime-host --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
```

수정 후 신규 unit 1건 0.00초 통과: batch 거절의 원자성, 한글·newline/quote escaping의 기존 JSON과 byte 일치·roundtrip, body/aggregate의 정확한 경계 허용·한 byte 부족 거절, 이미 있는 queue/counter 유지, writer limit 뒤 추가 write의 무변경 실패를 확인했습니다. strict clippy exit 0(0.54초)입니다. 모든 initialize·feature·cancel·shutdown→exit serialization이 같은 경로를 사용하므로 영향받는 runtime host 실제 두 child 검사만 수정된 source에서 한 번 실행해 1건 0.37초 통과했습니다. 이전 snapshot unit·나머지 일반/session/infra/parser 성공은 그대로 재사용했습니다.

이 단계 당시 command byte admission, URI/language metadata·전체 mirror/replay budget, initialize/capability DTO의 retained allocation, notice clone·생존 consumer의 전체 quota와 RSS/latency가 남았습니다. command admission의 후속 결과는 아래에 분리합니다. 이 완료는 송신 직렬화 경계뿐이며 native LSP memory gate 또는 M8 전체 완료가 아닙니다. 새 상한 정책·제품 GUI 채택으로 이 누락을 숨기지 않습니다.

## Command payload admission

대상은 `native/budget.rs`, `native/session.rs`와 기존 native SessionOptions의 Rust caller fixture입니다. 새 `command_bytes`는 native opt-in caller가 선택하는 값이며 fixture는 기존 QUEUE_BYTES 4MiB를 재사용합니다. 제품 기본값·기존 Tauri API·GUI 후보·dependency·lockfile·MSRV를 바꾸지 않았습니다. 0·Semaphore 한도 초과·u32 표현 범위 초과는 prepare에서 거절합니다.

기존 command queue는 개수만 제한해 actor validation 이전에 String·Value·config·reply를 보유했습니다. send는 command의 quota weight를 검사하고 nonblocking byte permit을 얻은 뒤 기존 count-limited queue에 넣습니다. String과 PathBuf는 capacity, Vec은 capacity와 원소 슬롯, JSON과 추가 property map은 동적 값·key capacity 및 구조 비용을 합산합니다. JSON depth 128은 이 admission의 지역 정책이며 LSP 표준의 필수 한도나 serde parser와 정확히 같은 수용 경계라는 주장은 하지 않습니다. u32 변환·checked add/multiply와 한도 초과는 Capacity로 반환합니다.

설치된 Tokio 1.53.1 공식 source의 Semaphore::try_acquire_many_owned와 OwnedSemaphorePermit::drop을 확인했습니다. Envelope가 permit을 소유하므로 waiter 취소만으로 아직 queued payload의 예산을 반환하지 않습니다. actor가 처리하는 동안에는 lexical permit을 유지하고 envelope/handling scope가 끝나면 반환합니다. queue full·closed 실패에서 버려지는 envelope도 반환합니다. Stop은 payload가 없어 byte quota 0을 사용하되 기존 queue count 제한은 유지합니다. 따라서 byte 포화에서도 Stop admission이 가능하지만 count 포화 때 무조건 종료 요청을 삽입하는 별도 우선순위 채널은 아닙니다.

추가 shape 검사의 최초 컴파일은 MessageActionItem.properties 필드 누락으로 E0063 실패했습니다. 설치된 lsp-types 0.97.0 DTO를 확인해 fixture를 완성한 뒤, ShowMessage의 추가 속성을 기존 제목-only 계산이 누락하는 assertion 실패를 재현했습니다(exit 101, 0.00초). properties map 슬롯·key·String/JSON Value를 합산하도록 수정했습니다. 추가 속성 자체를 삭제하거나 지원 capability를 끄는 우회는 하지 않았습니다.

```sh
cargo test -p taide-lsp --lib command_byte_budget --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
cargo test -p taide-lsp --lib payload_shape --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test runtime-host --test server-messages --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test server-messages server_request를_response로 --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
cargo clippy -p taide-lsp --lib --tests --target-dir experiments/terminal-core-spike/target --locked --offline -- -D warnings
cargo clippy --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --tests --target-dir experiments/terminal-core-spike/target --locked --offline -- -D warnings
cargo check -p taide-runtime --test native_lsp --target-dir experiments/terminal-core-spike/target --locked --offline
```

command unit 1건은 0.00초에 byte 포화·취소된 waiter와 실제 envelope 폐기 구분·짧지만 큰 String/Vec capacity·Stop byte 면제/count 포화·closed·정수 overflow를 통과했습니다. shape unit 1건은 수정 뒤 0.00초에 configuration/applyEdit/workspaceFolders/showMessage의 exact/one-byte-under weight, 추가 property의 보유 capacity, payload 없는 reply, JSON 깊이 경계를 통과했습니다. 최초 command unit의 E0382는 test oneshot sender 이동 뒤 envelope를 다시 사용하는 fixture 오류였으며 destructuring으로 수정했습니다. 제품 동작 재현 실패와 컴파일 오류를 구분합니다.

새 admission이 영향을 주는 actual runtime host 1건 0.36초와 typed server 왕복·ticket/restart 2건 0.51초가 통과했습니다. properties 합산 수정 뒤에는 영향받는 typed server 왕복 1건만 다시 검사해 0.36초에 통과했습니다. strict LSP clippy는 최종 source에서 exit 0(0.88초), spike 모든 test target clippy exit 0(6.09초), runtime native_lsp caller check exit 0(1.25초)입니다. 기존 성공한 command unit·runtime host·나머지 session/parser 전체 검사는 반복하지 않았습니다.

이 quota는 queued/handling command의 계산된 weight이며 정확한 allocator RSS 상한이 아닙니다. String/Vec의 실제 보유 capacity를 반영하지만 HashMap/BTree node·allocator rounding·Envelope/Arc/oneshot 메타데이터는 정확한 전체 할당량으로 모델링하지 않습니다. opaque Uri는 공개 문자열 길이만 반영하며 underlying capacity는 접근할 수 없습니다. serde Value 숫자의 고정 크기는 현재 workspace serde 기능을 따릅니다. caller가 send 전에 만든 할당·pending method/URI·coordinator mirror·replay·initialize/capability·consumer clone은 이 command permit 범위 밖입니다. command 처리 후 persistent mirror로 이전된 텍스트를 이 budget이 계속 보유한다고 주장하지 않습니다.

정확한 per-allocation peak/RSS·입력 생성 전 quota·전체 metadata owner budget·성능은 남으며 native UI 또는 LSP 전체 memory gate 완료가 아닙니다. Stop count 포화의 제품 UX·별도 priority 정책은 기존 정책을 보존한 상태로 후속 host 연결에서 결정해야 합니다. 사용자 실기 앱·시스템 설정은 조작하지 않았습니다.

## 초기화·문서 metadata 소유 수명 확인

실제 `native.rs`와 `native/session.rs`에서 다음 경계를 확인했습니다. 기존 한도를 전체 메모리 보장으로 해석하지 않습니다.

| 소유자                          | 수명과 복사 경계                                                             | 미완료 정책                                                |
| ------------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------- |
| SessionRunner.initialize        | prepare에서 보존하고 run/restart마다 coordinator 요청으로 clone              | prepare 이전 할당·runner의 persistent payload quota        |
| coordinator.client_capabilities | begin에서 initialize capabilities를 clone해 dynamic opt-in 판정에 유지       | capabilities retained allocation·opaque metadata quota     |
| coordinator.capabilities        | initialize reply의 capabilities를 clone하고 disconnect/restart/stop에서 폐기 | incoming frame 크기와 별개인 구조 비용·제품 제한           |
| coordinator.documents           | URI key와 DocumentMirror를 소유하고 text len 합계 64MiB·256개 제한           | String capacity·URI/language·key·pending URI의 전체 합산   |
| coordinator.replayed_documents  | initialize/delta write 중 기존 내용을 비교하는 사본                          | 진행 중 peak·outgoing Value/String와 원본의 동시 소유 비용 |

Running 진입 뒤 불필요한 replay 사본을 보유하던 결함을 재현해 수정했습니다. delta가 있을 때 사본을 유지하고 delta가 없을 때만 폐기합니다. 소유 수명 unit 1건·기존 편집/close-open replay 1건은 각각 0.00초, 실제 crash/restart/latest replay 1건은 진단 보강 뒤 0.14초 통과했습니다. 최초 실제 검사의 initial Running 3초 timeout 원인은 미확정이며 assertion 진단만 보강하고 기존 시간 정책을 유지했습니다. [버그·검증 기록](../bug/2026-09-30-native-lsp-replay-snapshot-lifetime.md)에 실패와 최종 증거를 구분합니다.

새 persistent quota 또는 GUI/editor dependency를 임의로 확정하지 않았습니다. prepare 이전 allocation, 전체 retained metadata·진행 중 replay/wire peak·RSS/latency·provider 소비와 canonical DocumentStore 연결은 미완료입니다. 최종 제품 quota는 native composition root의 동시 문서·서버·consumer 수와 실제 memory gate에서 정해야 하며 개별 headless 경계 성공으로 대체하지 않습니다.

## 다음 단계와 미검증

- [x] 기존 `taide-infra::lsp_proc`의 실제 framing·owned process와 합성 Rust mock server를 연결해 initialize → initialized → didOpen → change/hover → crash → reap → 새 generation replay를 확인했습니다.
- [x] 결정적 None/Full/Incremental sync/options·openClose/save/includeText·UTF-16 replacement range·replay delta·malformed options 검사 3건이 통과했습니다.
- [x] 기존 27개 feature의 static capability guard·restart 재광고 검사 3건, 종료 pending 정책·정상/오류/timeout·회수 acknowledgement 검사 3건과 실제 정상 shutdown→exit→join→Stopped 연결이 통과했습니다.
- [x] caller 선택형 bounded header/body·allocation 전 거절·UTF-8·truncated EOF·consumer 오류와 owned child 회수, exit 무시의 grace→kill→join을 확인했습니다. 기존 `spawn`의 정책과 사용처는 유지합니다.
- [x] caller receipt 취소와 wire write를 분리했고 queued+inflight byte budget·queue rejection·broken pipe·pending 해제·writer join, 실제 owned process의 crash/restart/stop을 확인했습니다.
- [x] pure coordinator를 제품 crate로 옮겨 구현 중복을 제거했고 next deadline·current generation disconnect·pending 정리·mirror 보존·종료의 회수 ack 구분 2건을 확인했습니다.
- [x] 단일 native async session을 기존 process store에 연결해 실제 handshake·request·crash/replay·취소/timeout·EOF/exit 무시·client/worker 종료·join 7건을 확인했습니다.
- [x] 기본 server request와 typed diagnostics/log/progress 알림·session ticket·deadline·reply guard의 신규 5건과 영향받은 native session 7건을 확인했습니다. host 효과·전체 method matrix 완료는 아닙니다.
- [x] native의 기본 dynamic feature opt-in·atomic batch·Unicode selector·generation 경계를 pure 3건·실제 process 1건으로 확인했고 duplicate server ID 선행 거절 actor unit 1건을 확인했습니다.
- [x] 기존 feature 27개의 Params/Reply 계약과 원본 metadata 보존을 검사하고 malformed 요청·응답의 개별 실패 및 후속 실제 typed 왕복을 확인했습니다. native UI 기능 소비와 DocumentStore coordinate 검증은 남았습니다.
- [x] native runner의 기존 runtime supervisor 등록 action과 admission/초기화 중 실제 child root 회수 2건을 확인했습니다. 후속 두 Running session의 명시적 stop·활성 root 강제 회수·추적/admission·종료 snapshot을 실제 host 검사 1건과 unit 1건으로 확인했으며 GUI composition root·자동 graceful 정책 연결은 남았습니다.
- [x] Client initialize/feature의 work-done와 partial token 소유권·충돌·상한·completion cleanup을 unit 1건과 실제 mode 1건으로 확인했습니다. 후속 활성 host 종료의 제한된 합성 경계는 위 검사로 확인했으며 native 진행 UI·전체 root 동시 부하는 남습니다.
- [x] internal u64 요청 identity와 integer/string wire 표현을 분리하고 i32 상한의 coordinator·actor 검사 2건을 확인했습니다. 실제 언어 서버의 string ID 연동은 미검증입니다.
- [x] 기존 frame/outgoing 한도 안에서 JSON을 fallible bounded sink로 직렬화하고 batch 실패의 원자성을 확인했습니다. 신규 unit 1건과 영향 actual host 1건이 통과했으며 command/metadata·전체 RSS budget은 남았습니다.
- [x] native caller 선택형 command byte admission을 envelope 소유권에 연결했습니다. 신규 unit 2건과 영향 actual host/server 3건이 통과했고 추가 속성 계산 수정 뒤 영향 server 왕복 1건만 재검사했습니다. 전체 metadata/persistent owner·RSS budget은 남습니다.
- [x] initialize·capabilities·document key/text·replay의 실제 persistent 소유를 대조하고 Running 뒤 replay 사본 보유 결함을 재현·수정했습니다. 소유 unit·기존 replay 편집 경계·actual crash/restart가 통과했으며 첫 실제 timeout의 미확정 원인은 QA 부채로 분리했습니다.
- [ ] dynamic registration, 나머지 server request/notification, 모든 기능 adapter와 UTF-16 coordinate를 전체 matrix로 포팅합니다. Incremental은 현재 전체 range replacement proof이며 제품의 효율적인 edit 전송·실제 Incremental 서버 동등성은 미완료입니다.
- [ ] native host의 실제 composition root·전체 server request/notification adapter, command/payload/URI/metadata 및 전체 receive memory 정책·모든 failure listener 해제와 성능을 검증합니다.
- [ ] canonical DocumentStore·native editor와 연결하고 syntax-only fallback·degraded UX·실제 서버 crash soak·성능을 확인합니다.

결정적 reply fixture와 실제 framed mock server 각각의 통과는 제품 LSP 기능 동등성의 대체 증거가 아닙니다. TS client·Monaco worker는 제거하지 않았으며 M8 완료 상태도 아닙니다.
