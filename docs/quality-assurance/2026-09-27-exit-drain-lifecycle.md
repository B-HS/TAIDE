# 정상 종료 드레인 QA

## 대상 파일과 리포트

대상은 TaskSupervisor 실제 task 완료, LspInstallStore 마지막 lease, ExitDrain 소유권과 Tauri 종료 adapter입니다. 실제 앱/설치기·사용자 파일/프로세스·시크릿을 사용하지 않으며 직접 만든 채널/worker·UUID process-group child만 사용합니다.

## 상세 검증

- [x] `cargo test -p taide-runtime --lib task_supervisor::tests --quiet`: 선행 재현은 abort 직후 아직 살아 있는 작업을 0개로 보고해 exit 101이었습니다. 수정 뒤 3건 통과(exit 0)로 실제 async Drop/새 등록 거절·시작한 blocking 작업/대기 Drop 뒤 재대기를 확인했습니다.
- [x] `cargo test -p taide-lsp --lib install::tests --quiet`: 14건 통과(exit 0), 새 wait API 부재 E0599 RED 뒤 마지막 lease/복수 대기자/이미 빈 저장소 검사 2건을 포함합니다.
- [x] `cargo test -p taide-runtime --lib lsp_install_ --quiet`: 26건 통과(exit 0). core/supervisor 변경 뒤 기존 HTTP/파일 I/O/추출·child/reader lifecycle을 확인하며 store shutdown child fixture는 실제 supervisor shutdown→wait_for_idle도 소비합니다.
- [x] `cargo test -p taide-runtime --lib exit_drain::tests --quiet` 2건, Tauri `--test task_supervisor --test taide_lsp_install_store_extraction` 24건 통과(exit 0). main-reply 응답을 막지 않는 coordinator·대기 중 재진입/한번만 exit callback·owner Drop·실제 adapter 순서를 확인합니다.
- [x] 자기 group 자손 단일 검사 1건 통과(exit 0). 실행 당시 필터 `lsp_install_toolchain::tests::종료_드레인은_TERM`이며 그 뒤 snake_case 요구에 따라 `TERM`→`term` 이름만 수정했습니다. TERM을 보낸 뒤 살아 있는 자기 child가 group 취소 뒤 사라짐을 관찰했습니다. 앞의 성공 본문은 재사용하며 이번 서로 다른 검사는 runtime 32·core 14·Tauri 24로 70건입니다.

## 정적 검사

- [x] 이름 수정 뒤 `cargo clippy -p taide-lsp -p taide-runtime -p taide --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-lsp -p taide-runtime --no-deps --quiet`, `cargo fmt --all -- --check`, `git diff --check`: 모두 exit 0입니다. 새 history/QA와 갱신한 toolchain QA 3개 MD의 대상 Prettier write/check도 exit 0입니다.
- [x] 추가 fixture의 owned JoinHandle 이동 E0509와 이름 non_snake_case 경고는 mutable borrow와 소문자 이름으로 해결했습니다. 경고 억제/불필요한 derive는 추가하지 않았습니다. 이는 실제 lifecycle 실패와 구분합니다.
- [x] IPC DTO·command signature·등록/이벤트 payload·bindings/manifest/lockfile은 변경하지 않았고 bindings 해시는 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`입니다. 기존 bindings 생성/IPC baseline·infra archive 입력은 같아 성공을 재사용합니다.

## 남은 gate

- [ ] 실제 native ExitRequested/직접 Exit·메뉴 callback의 이벤트 순서와 앱 종료 실기는 사용자 실행이 필요합니다. fixture가 native 이벤트 전달까지 증명하지는 않습니다.
- [x] 후속 [일반 LSP wait QA](2026-09-27-lsp-process-wait-lifecycle.md)에서 정상 coordinator가 제거/교체된 세션의 wait·reader·exit callback 완료도 기다리도록 구현/검증했습니다. child exited만으로 ready를 세우지 않습니다.
- [ ] 직접 Exit/exit runtime 요청 실패의 비설치 작업, 감독되지 않은 nested blocking worker·PTY thread를 판정합니다. 정상 감독자 snapshot이나 LSP 대기가 이 자원 전체를 포함한다고 주장하지 않습니다.
- [x] 후속 [부모 선종료 QA](2026-09-27-lsp-install-parent-exit-lifecycle.md)에서 부모가 먼저 종료한 자기 그룹의 자손 생존을 재현/수정했습니다. 원래 exit code·reader 회수와 회수 뒤 그룹 재신호 금지를 확인합니다.
- [ ] 그룹을 벗어난 자손·Windows process tree·OS 강제 종료의 cleanup을 판정합니다. 모든 자손의 실제 wait/join과 bounded 종료를 증명한 것은 아닙니다.
- [ ] M6 전수 body 판정과 M7/M8·Phase 0 실기는 별도 완료해야 합니다.
