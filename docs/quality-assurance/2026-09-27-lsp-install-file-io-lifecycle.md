# LSP 설치 파일 I/O lifecycle QA

## 대상 파일과 리포트

대상은 infra lsp_install/http 공개 API와 runtime lsp_install_actions입니다. HTTP 요청 취소와 내부 파일 worker 종료를 구분하고 실제 설치 경로가 감독된 file I/O port를 사용하도록 보완합니다. 테스트는 직접 생성한 UUID 디렉터리·localhost·전용 blocking pool만 사용합니다.

## 상세 검증

- [x] `cargo test -p taide-runtime --lib lsp_install_actions::tests::파일_생성_대기_중_요청_drop은_슬롯을_유지하고_늦은_임시_파일을_남기지_않는다 --quiet`: 수정 전 슬롯 재사용+늦은 파일 1개로 exit 101, 수정 뒤 동일 검사 exit 0. 파일 생성 대기를 실제 다운로드 경로에서 관찰했습니다.
- [x] `cargo test -p taide-runtime --lib lsp_install_actions::tests --quiet`: 마지막 owner Drop 순서 보완과 cleanup witness 뒤 14건 통과(exit 0). 새 5건+기존 9건이며 write/flush 큐 2개 경우·열린 파일 owner·파일 생성 오류·시작 전 취소 cleanup을 포함합니다.
- [x] `cargo test -p taide-infra --lib lsp_install::tests --quiet`: 16건 통과(exit 0). archive/path/checksum/버전/atomic 기존 경계를 확인했습니다. 이번 slice의 서로 다른 실행 검사는 30건입니다.
- [x] `cargo clippy -p taide-infra -p taide-runtime -p taide --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-infra -p taide-runtime --no-deps`: exit 0. owner 구조체와 추가 cleanup unit 뒤 runtime tests clippy의 최종 결과는 별도로 확인합니다.
- [x] 마지막 owner/cleanup 뒤 `cargo clippy -p taide-runtime --tests -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check`: exit 0. 변경 history/QA 4개 파일의 대상 Prettier write/check 모두 exit 0이며 재포맷 변경은 없습니다. 다운로드 본문은 파일 I/O 주입 치환 뒤 기존 본문과 동일하게 대조했습니다.

## 재사용·생략 근거

store 구현·Tauri command/종료 adapter·IPC DTO/등록·EventSink byte 변환·archive helper와 manifest/bindings/lockfile은 변경하지 않았습니다. 이전 slice의 store·공유 조립/도메인/이벤트/감독·IPC와 실제 bindings 생성 결과는 해당 입력이 동일해 재사용합니다. bindings SHA-256은 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`로 그대로입니다. native Rust의 IO port 추가를 공개 IPC 변경으로 간주해 생성기를 반복 실행하지 않았습니다. Tauri consumer의 Send/type 경계는 이번 clippy 컴파일에서 확인합니다.

## 남은 전체 gate

- [ ] toolchain child/reader의 실제 kill/reap/drain·command Drop·EOF 지연을 synthetic child/pipe로 구현/검증합니다. 실제 설치기나 사용자 프로세스는 사용하지 않습니다.
- [ ] root shutdown의 실제 자원 drain과 std::process::exit 전 종료, LSP wait worker 자체/PTY thread·전체 M6 command body·M7/M8 gate는 별도로 완료해야 합니다.
- [ ] Windows/OS 강제 종료·실제 앱 실행/재시작·GUI/performance/data 실기는 이 fixture로 확인하지 않았습니다. 앱 실행·재시작은 사용자 몫입니다. 기존 cleanup 제거 오류 무시와 atomic 설치 정책 이상의 보장을 주장하지 않습니다.
