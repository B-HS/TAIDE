# LSP 설치 admission·download·extraction 소유권

## 대상 파일

- `crates/taide-lsp/src/install.rs`, `crates/taide-lsp/Cargo.toml`
- `crates/taide-runtime/src/lsp_install_actions.rs`, `crates/taide-runtime/src/lib.rs`, `crates/taide-runtime/Cargo.toml`
- `src-tauri/src/domain/lsp/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/tests/taide_lsp_install_store_extraction.rs`
- `docs/architecture.md`, `docs/PROCESS.md`, `docs/quality-assurance/2026-09-27-lsp-install-download-lifecycle.md`

## 리포트

요청 guard가 사라져도 이미 실행 중인 extraction이 같은 서버의 슬롯과 UUID 임시 경로를 보유하도록 worker lease를 분리했습니다. store shutdown은 admission을 닫고 기존 요청에 취소를 알리며 슬롯을 조기 해제하지 않습니다. download 정책은 UI 비의존 runtime으로 이전하고 실제 이벤트 전송은 EventSink/Tauri adapter에 유지했습니다.

취소와 atomic 적용은 같은 mutex gate에서 직렬화합니다. 취소가 앞서면 적용 callback과 Done을 실행하지 않습니다. 적용이 먼저 성공하면 늦은 취소는 이미 설치된 결과를 되돌리지 않습니다. callback은 같은 store를 재진입하지 않으며 실제 소비자는 기존 atomic_install뿐입니다. 기존 atomic_install의 기존 버전 제거·rename 정책을 그대로 소비하며 새로운 rollback 보장을 주장하지 않습니다.

## 상세

1. 기존 store 4건과 infra 설치 16건을 선행 확인했습니다. 새 store 5건은 lease/shutdown API 부재 E0599·E0609(exit 101), 새 runtime blocking 4건은 helper 부재 E0425(exit 101)로 먼저 실패했습니다. runtime RED에는 아직 정의하지 않은 구현 import AppError 부재 E0433도 포함됐습니다.
2. owned guard의 Drop은 취소 요청이며 마지막 lease Drop만 active entry를 제거합니다. store는 control을 보유하고 lease는 store를 보유해 Arc 순환을 만들지 않습니다. shutdown은 control snapshot을 만든 뒤 store lock을 놓고 취소 gate를 취득합니다.
3. TaskSupervisor가 extraction worker 자체를 추적합니다. 요청 future Drop은 시작한 blocking 작업을 abort하거나 join 완료한 것으로 취급하지 않습니다. worker가 lease와 임시 경로를 보유하고 종료 후 cancellation을 다시 확인해 적용을 차단합니다. 대형 archive의 동기 decode/파일 작업은 기존처럼 blocking pool에서 실행하며 삭제한 private 주석의 근거를 이 문서에 보존합니다.
4. Notify 취소와 다운로드의 select는 header/body가 오지 않아도 취소를 관찰합니다. 설정·플랫폼·공개 checksum, 기존 archive 5종, checksum mismatch 오류와 Failed 메시지, 정상 진행 payload의 byte→f64와 atomic 경로는 유지합니다. 새 package/version/lockfile 변경 없이 기존 Tokio sync 및 테스트용 net/io-util 기능만 명시했습니다.
5. HTTP fixture 첫 실행은 fixture가 manifest ID 대신 binary 이름 rust-analyzer를 조회해 3건 실패했습니다. 실제 manifest의 rustAnalyzer로 교정한 뒤 정상 download·checksum mismatch·정지 응답 취소가 통과했습니다. 설치기나 실제 언어 서버는 실행하지 않았습니다.

## 검증

최종 검증의 명령과 실제 결과는 연결된 QA 문서에 기록합니다. hidden Tauri State 인수 변경 뒤 실제 TypeScript 생성 검사 1건이 통과했으며 bindings SHA-256은 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`로 manifest와 동일합니다. bindings를 수동 수정하지 않았습니다.

## 미완료 경계

이 절은 이 slice 시점의 미완료 경계를 보존합니다. 후속 HTTP 파일 I/O 보완과 실제 재현 결과는 [파일 I/O 소유권 이력](2026-09-27-lsp-install-file-io-ownership.md)과 해당 QA 문서에 기록합니다.

HTTP 내부 Tokio 파일 create/write 작업 중 future Drop은 이번 정지 응답 fixture로 증명하지 않았습니다. 내부 blocking I/O가 요청 future보다 늦게 끝나는 경우 임시 경로 cleanup과 슬롯 수명을 추가로 재현·보완해야 합니다. toolchain child/process-group/reader EOF 지연·wait worker/PTY의 전체 소유권, 앱 종료 시 실제 자원 drain, M6 전체·M7·M8도 미완료입니다. root의 store shutdown은 취소 통지/신규 등록 거절이며 자식 프로세스를 동기 kill하거나 모든 worker를 join한 결과가 아닙니다. JD~JG는 전체 설치 lifecycle이 끝날 때까지 미완료로 유지합니다.

## 공식 근거

사용 중인 Tokio 1.53.1의 공식 원천 `src/runtime/task/join.rs`에서 JoinHandle Drop의 detach·await 뒤 destructor 완료·시작한 blocking 작업의 abort 불가 계약을 확인했습니다. 같은 버전 `src/sync/notify.rs`의 notified 문서는 생성된 future가 미poll 상태여도 notify_waiters를 관찰한다고 명시합니다. `src/fs/file.rs`의 create→OpenOptions::open 경로도 읽어 HTTP 내부 I/O를 extraction 소유권과 동일하게 완료 처리하지 않았습니다.
