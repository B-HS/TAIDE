# 일반 LSP wait 작업 수명 QA

## 대상 파일과 리포트

infra LspProcHandle/ChildWaitOwner, core LspStore, runtime ExitDrain과 Tauri spawn/종료 adapter를 확인합니다. 자기 생성 sh/sleep child·채널만 사용하며 실제 앱/서버·사용자 PTY·시크릿은 사용하지 않습니다.

## 상세 검증

- [x] 마지막 핸들 Drop 뒤 직접 child 생존 RED(exit 101)와 새 store API 부재 E0599 여섯 건을 재현했습니다. RED fixture도 자신의 Tokio child를 kill/wait한 뒤 실패합니다.
- [x] `cargo test --offline -p taide-infra --lib lsp_proc::tests --quiet`: 27건 통과했습니다. 기존 프레이밍/tail·reader와 Drop 종료 요청·대기 future 취소 후 재대기·미poll child owner 취소 경계를 확인했습니다.
- [x] `cargo test --offline -p taide-lsp --lib store::tests --quiet`: 3건 통과했습니다. 종료 뒤 factory 거절·factory 실패·제거된 세션의 작업 보유와 대기 Drop 뒤 callback 실제 완료를 확인했습니다.
- [x] `cargo test --offline -p taide-runtime --lib exit_drain::tests --quiet`: 3건 통과했습니다. child exited와 worker finished를 구분하고 종료 callback 완료 전에 ready/exit가 전달되지 않음을 확인했습니다.
- [x] `cargo test --offline -p taide --test taide_lsp_store_extraction --test taide_lsp_install_store_extraction --test taide_lsp_process_extraction --test task_supervisor --quiet`: 2·2·5·22건, 합계 31건 통과(exit 0)했습니다. 기존 공개 경로와 실제 adapter/store/root 배선을 확인합니다.
- [x] `cargo test --offline -p taide-infra --lib lsp_proc::tests::이미_종료로_표시된 --quiet`: 기존 27건 중 1건에 독립적인 closed PID capability/미설정 exited 플래그 조건을 추가한 뒤 통과했습니다. 동일 검사이므로 총수에 중복 집계하지 않습니다.
- [x] `cargo test --offline -p taide-lsp --lib process::tests --quiet`: 기존 종료 대기/재시작 5건 통과(exit 0)했습니다. 앞의 infra 27·core 8·runtime 3·Tauri 31은 서로 다른 69건입니다.
- [x] `cargo test --offline -p taide --test app_services_runtime --quiet`: 실제 공유 조립 2건 통과(exit 0), 변경 후 서로 다른 검사는 71건입니다. 잘못 선택한 `cargo test --offline -p taide-runtime --lib app_services::tests --quiet`는 exit 0이나 0건이므로 통과 건수·회귀 근거에서 제외했습니다.

## 정적 검사

- [x] `cargo clippy --offline -p taide-infra -p taide-lsp -p taide-runtime -p taide --all-targets -- -D warnings`: exit 0입니다.
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --offline -p taide-infra -p taide-lsp -p taide-runtime --no-deps --quiet`, `cargo fmt --all -- --check`: exit 0입니다.
- [x] 새 history/QA와 후속 링크를 반영한 종료/reader QA 네 문서의 대상 Prettier write/check·`git diff --check`: exit 0입니다. 큰 PROCESS/architecture의 무관한 재포맷은 하지 않았습니다.
- [x] dependency manifest·Cargo.lock·bindings·IPC manifest diff는 없습니다. 타입/계약 입력이 같아 기존 생성/IPC 성공 결과를 재사용합니다.

## 미완료 gate와 테스트 부채

- [ ] PTY reader/flusher/wait thread 소유와 callback 완료를 자기 fixture로 판정합니다. 현재 Drop의 unpause/kill 요청은 실제 join의 근거가 아닙니다.
- [ ] PTY kill_all의 세션 유지→Drop 미실행→pause gate 미해제와 Unix 복제 killer의 wait/PID 권한 비공유를 재현합니다. 실제 본문/portable-pty 0.9.0 공식 원천을 대조했으며 아직 fixture 통과나 수정 완료가 아닙니다. 회수된 실제 PID에 시그널을 보내지 않고 자기 gate/가짜 killer로 확인합니다.
  - 후속 [PTY pause QA](2026-09-27-pty-shutdown-pause-lifecycle.md)에서 명시 kill의 미해제와 늦은 재pause를 자기 gate/가짜 killer로 재현·수정했습니다. [PTY child wait QA](2026-09-27-pty-child-wait-lifecycle.md)에서는 WNOWAIT 관찰→시그널 권한 반납→실제 wait로 Unix PID gate를 보완했습니다. thread/root 종료·OS 오류 전체 회수는 미완료로 유지합니다.
- [ ] 정상 ExitRequested·직접 native Exit·종료 요청 실패와 메뉴 응답/실제 LSP 재시작은 사용자 실기로 확인합니다. synthetic 검사는 native 이벤트 전달을 증명하지 않습니다.
- [ ] runtime abort·wait/JoinError·non-yield callback의 회수/종료 계약을 판정합니다. 종료 완료 플래그만으로 모든 OS 자원 회수를 주장하지 않습니다. 이런 오류·callback 정책 변경·종료 지연이 관찰되면 별도 fixture를 실행합니다.
- [ ] Windows 및 그룹 이탈 자손·OS 강제 종료를 확인합니다. 현재 macOS 자기 child 검사와 best-effort Drop은 이 환경의 실제 회수를 증명하지 않습니다.
- [ ] M6 command body/port 전수 및 나머지 자원·M7/M8·Phase 0 gate는 별도 완료합니다.
