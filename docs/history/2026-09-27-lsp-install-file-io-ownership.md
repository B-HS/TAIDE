# LSP 설치 download 파일 I/O 소유권 보완

## 대상 파일

- `crates/taide-infra/src/lsp_install.rs`, `crates/taide-infra/src/http.rs`
- `crates/taide-runtime/src/lsp_install_actions.rs`
- `docs/architecture.md`, `docs/PROCESS.md`, download 소유권 이력/QA 및 `docs/quality-assurance/2026-09-27-lsp-install-file-io-lifecycle.md`

## 리포트

blocking pool을 테스트 전용 1개 worker로 제한하고 파일 생성이 대기 중인 상태에서 실제 설치 요청을 Drop했습니다. 수정 전에는 서버 슬롯이 재사용됐고 pool을 풀자 이미 cleanup된 경로에 임시 파일 1개가 늦게 생성됐습니다. 실제 실패 exit 101 뒤 파일 생성·쓰기·flush가 실제 worker와 열린 파일의 lease를 보유하도록 수정해 같은 재현 검사가 통과했습니다.

## 상세

1. Tokio 1.53.1 공식 원천에서 File::create→OpenOptions::std_open→asyncify→spawn_blocking 경로를 확인했습니다. 요청 future가 사라져도 내부 작업의 JoinHandle Drop은 작업을 종료하지 않습니다. 외부 artifact guard만으로는 늦은 파일 생성과 슬롯 재진입을 막지 못합니다.
2. infra의 DownloadFileIo로 stream/hash/throttle과 실제 파일 I/O를 나눕니다. 기존 공개 download_to_file은 Tokio 구현을 그대로 사용하며 소유권 안전을 일반 보장하지 않습니다. 실제 설치 runtime은 ownership-aware 구현을 주입합니다. 기존 API/오류·archive 추출/검증·atomic 적용과 byte/progress 정책을 유지합니다.
3. runtime의 create/write/flush는 기존 TaskSupervisor blocking 등록을 소비합니다. write는 네트워크 chunk 하나만 복사하며 전체 archive를 메모리에 모으지 않습니다. 열린 파일은 std 파일→artifact owner→lease 순서로 Drop하며 worker가 파일 Arc를 보유하므로 부모 Drop이 실제 열린 파일을 앞질러 슬롯을 해제하지 않습니다.
4. InstallBlockingWork는 work와 lease를 선언 순서대로 보유하며 whole owner를 worker body에서 이동합니다. queued worker가 시작 전 취소돼도 work capture의 cleanup을 먼저 완료하고 마지막 worker lease를 해제합니다. 독립 closure capture의 unspecified Drop 순서에 의존하지 않습니다. 시작한 blocking 작업을 abort했다고 주장하지 않습니다.
5. 새 5개 unit은 queued create 취소·write/flush future Drop·열린 파일 수명·파일 생성 오류·queued work cleanup 순서를 확인합니다. 기존 정상 binary/checksum mismatch/정지 header/body/추출 취소와 함께 runtime 14건이 통과했습니다. 정확한 최종 명령/결과는 연결된 QA 문서에 기록합니다.

## 공식 근거

[Rust 공식 async trait 문서](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/)의 public future Send 경계에 따라 trait의 반환 future에 Send를 명시했습니다. 기존 Rust 최소 버전 1.89 안에서 구현하며 새 macro/dependency는 추가하지 않습니다. [Rust Reference의 Drop 순서](https://doc.rust-lang.org/reference/destructors.html)는 struct 필드의 선언 순서와 closure capture의 미지정 순서를 구분합니다. 이 근거는 열린 파일과 queued work의 owner 배치에 적용했습니다.

## 미완료 경계

toolchain child/process-group/reader EOF·wait worker/PTY 및 root shutdown의 실제 drain은 아직 남아 있습니다. 이 slice는 std::process::exit 전에 모든 worker를 join하는 root 정책이나 실제 앱/OS 강제 종료 뒤 RAII cleanup을 구현/증명하지 않습니다. 부모 요청과 HTTP 파일 작업의 소유권 보완을 전체 M6·M7/M8 완료로 바꾸지 않습니다. 전체 설치 JD~JG도 미완료로 유지합니다.
