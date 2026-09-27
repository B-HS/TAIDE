# LSP stdout/stderr reader 회수

상태: reader 회수 단위 구현·자동 검증을 완료했습니다. M6 전체·M7·M8은 미완료입니다.

## 대상 파일과 리포트

`crates/taide-infra/src/lsp_proc.rs`의 stdout/stderr handle을 private ReaderTask가 보유하고 부모 wait worker에 넘깁니다. 정상 child exit 뒤 두 reader를 함께 드레인하고, 기존 500ms 대기 한도를 넘으면 handle을 abort한 뒤 실제 완료를 await합니다. owner Drop은 abort를 요청합니다. 기존 stderr timeout에 handle을 값으로 넘겨 detach하던 경로와 stdout handle 미보유를 제거했습니다. helper는 실제 두 stream에서 사용하며 새 의존성과 공개 API 변경은 없습니다.

기존 child wait/kill select·exited flag 위치·동기 PID kill과 이미 종료된 PID guard·stderr tail 상한·message framing·masking adapter는 유지했습니다. stdout callback의 이후 입력이 exit callback 뒤로 남지 않도록 두 stream 회수 뒤 exit를 보고합니다. 드레인은 `tokio::join!`으로 함께 수행하며 두 500ms 대기를 순차로 더하지 않습니다. 이 시간은 EOF 대기 한도이고 non-yield callback의 강제 중단이나 전체 OS 종료의 절대 상한이 아닙니다.

`docs/architecture.md`에 실제 소유권·잔여 gate를 반영하고, LSP Drop 전체 보장을 이미 완료한 사실처럼 표현하던 문장은 요구 규약과 현재 미완료 구현을 구분했습니다. `docs/quality-assurance/2026-09-27-lsp-reader-lifecycle.md`에 부모 Drop의 비동기 abort와 실제 OS pipe·앱 종료 미검증 조건을 기록했습니다.

## 검증과 상세

1. 변경 전 `cargo test -p taide-infra --lib lsp_proc --quiet` 18건이 통과했습니다. 새 5건은 ReaderTask 부재 E0433(exit 101)로 먼저 실패했습니다. 구현 후 같은 관련 묶음은 23건 통과했습니다. EOF 지연·owner를 poll 전에 Drop·부모가 드레인 중 취소되는 조건은 capture 해제 handshake와 bounded 검사로 확인했습니다.
2. stdout 메시지/exit callback의 실제 연결 위험을 덮도록 synthetic sh 자식 검사 1건을 추가했습니다. `cargo test -p taide-infra --lib 'lsp_proc::tests::stdout_프레임과_stderr_tail은_자식_종료_콜백_전에_드레인된다' -- --exact`는 1건 통과했습니다. 정상 프레임·stderr tail·exit code·exited flag를 확인했고 기존 성공 23건을 재사용해 변경 후 서로 다른 검사 합계는 24건입니다. 각 명령 exit 0입니다.
3. `cargo clippy -p taide-infra -p taide-lsp --all-targets -- -D warnings`는 exit 0입니다. 추가 synthetic unit 뒤에는 변경된 infra test target의 `cargo clippy -p taide-infra --tests -- -D warnings`만 실행해 exit 0을 확인했습니다. `cargo fmt --all -- --check`, `git diff --check`와 새 결과 문서의 Prettier도 exit 0입니다.
4. 공개 API·IPC·Tauri adapter·dependency 목록·bindings/manifest diff가 없습니다. bindings SHA-256은 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`로 불변입니다. 공개 문서/시그니처를 바꾸지 않았으므로 bindings 생성·IPC 재실행·전체 workspace/frontend와 strict rustdoc을 추가하지 않았습니다. 동일한 Phase 0 계약 입력의 앞선 성공 결과를 재사용하며 이 단위 검사로 전체 앱 동등성을 주장하지 않습니다.

Tokio 1.53.1의 [timeout](https://docs.rs/tokio/1.53.1/tokio/time/fn.timeout.html)과 [join](https://docs.rs/tokio/1.53.1/tokio/macro.join.html) 공식 문서를 확인했습니다. 같은 버전 JoinHandle 웹 문서는 접근 오류가 있어 설치된 Tokio 공식 원천 `src/runtime/task/join.rs`에서 mutable await의 cancel safety, Drop detach, abort와 완료 시 destructor 회수 계약을 직접 확인했습니다. Drop 취소 요청은 join 완료와 구분했습니다.

기존 unit은 이 검사가 직접 생성한 sh/sleep fixture만 사용했고 추가 자식은 고정 프레임과 tail을 쓰고 종료했습니다. 실제 앱·외부 LSP/설치기·사용자 프로세스·시크릿/키링은 실행하거나 조사하지 않았습니다. LSP 설치 store의 shutdown gate/전체 취소·child/reader 소유권, infra wait worker 자체의 감독/Drop, PTY와 잔여 application 정책은 남아 있습니다. 일반 push는 기존 명시 승인대로 M6 전체 완료 뒤 GitHub B-HS/TAIDE의 to_rust_native에 수행합니다.
