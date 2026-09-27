# LSP 설치 download·extraction lifecycle QA

## 대상 파일과 리포트

대상은 `crates/taide-lsp/src/install.rs`, `crates/taide-runtime/src/lsp_install_actions.rs`, Tauri LSP command/종료 adapter와 store 경계 검사입니다. 요청 수명과 실제 extraction worker 수명을 분리합니다. 설치 child·reader·HTTP 내부 파일 I/O를 포함한 전체 lifecycle gate는 아직 완료하지 않았습니다.

## 상세 검증

- [x] `cargo test -p taide-lsp --lib install::tests --quiet`: 기존 4건+새 5건, 총 9건 통과(exit 0). 요청 Drop 후 retained lease·shutdown 공유/재진입 거절·취소 선행 적용 거절·완료 뒤 늦은 취소·취소 전/후 알림을 확인했습니다.
- [x] `cargo test -p taide-runtime --lib lsp_install_actions::tests --quiet`: 새 7건 통과(exit 0). 실제 blocking worker 시작/부모 abort·명시 취소 뒤 완료·감독자 등록 거절·패닉/작업 오류, localhost 정상 binary·checksum mismatch·정지 header/body 취소를 확인했습니다. 부모 abort 뒤 임시 파일은 worker 완료까지 보존되고 이후 제거됩니다.
- [x] `cargo test -p taide-runtime --lib lsp_install_actions::tests::설치_설정과_플랫폼_checksum이_없으면_네트워크와_worker를_시작하지_않는다 --quiet`와 `cargo test -p taide-runtime --lib lsp_install_actions::tests::추출_직전_취소는_기존_binary를_보존하고_done을_발행하지_않는다 --quiet`: 추가 2건 통과(exit 0). runtime 합계는 서로 다른 9건입니다.
- [x] `cargo test -p taide --lib typescript_바인딩을_생성한다 -- --exact tests::typescript_바인딩을_생성한다`: 1건 통과(exit 0). `shasum -a 256 src/shared/api/bindings.ts`가 기존 manifest 해시와 일치하며 generated diff가 없습니다.
- [x] `cargo test -p taide --test taide_lsp_install_store_extraction --test app_services_runtime --test domain_boundaries --test rust_native_phase0_contract --quiet`: 각각 2·2·3·7건, 총 14건 통과(exit 0). 새 source 배선 검사는 root shutdown의 admission과 runtime/supervisor 주입을 확인하며 실제 프로세스 종료를 증명하지 않습니다.
- [x] `cargo test -p taide --test platform_event_sink lsp_ --quiet`, `cargo test -p taide --test task_supervisor --quiet`, `cargo test -p taide --lib domain::lsp::commands::tests --quiet`: 각각 2·22·15건 통과(exit 0). 변경 후 서로 다른 검사는 위 항목을 포함해 72건이며 선행 infra 설치 16건은 별도 재사용 근거입니다.
- [x] `cargo clippy -p taide-lsp -p taide-runtime -p taide --all-targets -- -D warnings`, 추가 2개 unit 뒤 `cargo clippy -p taide-runtime --tests -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-lsp -p taide-runtime --no-deps`: 모두 exit 0.
- [x] `cargo fmt --all -- --check`, `git diff --check`, 새 history/QA 두 파일의 대상 Prettier 검사: exit 0. 기존 PROCESS/architecture 전체 포맷이나 전체 workspace/frontend/실기 검사는 수행하지 않았습니다.

## 전체 gate에 남는 필수 항목

- [ ] 내부 파일 I/O future Drop: Tokio file create/write의 blocking 작업이 진행 중인 시점에 command를 취소해 재현합니다. 이번 HTTP fixture는 header/body 수신 정지 경계만 검증했으므로 임시 경로 cleanup 이후 파일 생성·write 지속 가능성과 슬롯 조기 해제를 아직 판정하지 않습니다. M6-JD/JE의 필수 후속 검사입니다.
- [ ] toolchain child/reader: command future Drop·앱 shutdown·EOF 지연 때 직접 생성한 child/pipe로 실제 kill/reap/drain과 슬롯 유지·재진입 거절을 검증하고 소유권을 구현합니다. 실제 설치기·사용자 프로세스는 사용하지 않습니다.
- [ ] root shutdown의 실제 자원 drain: admission을 닫고 token을 알린 것과 프로세스 종료/worker join 완료를 구분합니다. std::process::exit 전 자원 회수의 전체 gate를 별도로 확인합니다.
- [ ] LSP wait worker 자체/PTY thread·전체 M6 command body 및 M7 workspace/frontend/사용자 실기·M8 native gate는 미완료입니다. 앱 실행·재시작·실기는 사용자 몫이며 이번에 수행하지 않았습니다.

모든 파일·TCP 자원은 테스트가 만든 UUID 임시 경로와 localhost listener만 사용했습니다. 실제 server URL·설치기·앱·키링·사용자 설정/프로세스는 실행하거나 변경하지 않았습니다. cleanup의 파일 제거 오류는 기존처럼 무시하며 OS 강제 종료 뒤 RAII 실행을 보장했다고 주장하지 않습니다.
