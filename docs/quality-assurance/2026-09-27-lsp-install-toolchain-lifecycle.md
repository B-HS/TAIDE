# LSP toolchain 설치 lifecycle QA

## 대상 파일과 리포트

대상은 core install gate, runtime toolchain action/공유 helper, Tauri 설치 adapter와 이벤트 검사입니다. 자기 생성 sh/sleep child·UUID marker·메모리 pipe만 사용하며 실제 npm/go 등의 설치기, 앱, 사용자 파일/프로세스와 시크릿에는 접근하지 않습니다.

## 상세 검증

- [x] `cargo test -p taide-runtime --lib lsp_install_toolchain::tests --quiet`: child 취소·요청 Drop·store shutdown·정상 reap/output·EOF 지연·등록 거절·실패 마스킹·PGID 경계 10건 통과(exit 0). UUID marker의 자기 child PID가 취소 후 존재하지 않고 감독 작업이 0개인지 확인했습니다.
- [x] `cargo test -p taide-runtime --lib lsp_install_ --quiet`, `cargo test -p taide-lsp --lib install::tests --quiet`: 연결 변경 뒤 runtime 24건·core 12건 통과(exit 0). 앞의 toolchain 10건은 runtime 24건에 포함되므로 중복 합산하지 않습니다.
- [x] Tauri `--test platform_event_sink lsp` 2건·`--test taide_lsp_install_store_extraction lsp` 1건 및 이름 필터에 빠진 `설치_adapter` 1건, `--lib domain::lsp::commands::tests` 12건 통과(exit 0). 옮긴 3개 unit은 runtime에서 검증하며 adapter 검사·이벤트 payload 검사는 제거하지 않았습니다.
- [x] `cargo test -p taide-runtime --lib lsp_install_toolchain::tests::실패_child --quiet`, `cargo test -p taide-runtime --lib lsp_install_toolchain::tests::reader_owner_drop --quiet`: 각각 1건 통과(exit 0). 앞의 runtime 24건에 추가한 서로 다른 2건이며 runtime 설치 26·core 12·Tauri 16으로 이번 slice의 서로 다른 검사는 54건입니다. 같은 입력의 앞선 성공은 재사용했습니다.
- [x] `cargo clippy -p taide-lsp -p taide-runtime --all-targets -- -D warnings`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, 추가 검사 뒤 `cargo clippy -p taide-runtime --tests -- -D warnings`: 모두 exit 0입니다. `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-lsp -p taide-runtime --no-deps --quiet`도 exit 0입니다. 최종 `cargo fmt --all -- --check`, `git diff --check`와 새 history/QA·연결된 기존 QA 3개 MD의 대상 Prettier write/check는 exit 0입니다.

## 재사용·생략 근거

공개 IPC DTO·등록/command signature·EventSink payload와 manifest/생성 bindings/lockfile은 변경하지 않았습니다. bindings SHA-256은 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`로 동일합니다. 이전 bindings 생성·IPC baseline 성공은 입력이 같아 재사용하며 생성기를 반복 실행하지 않습니다. 이전 infra archive 16건과 TaskSupervisor 22건도 해당 구현/입력이 같아 재사용합니다. 현재 runtime/core와 변경한 Tauri consumer는 이번 좁은 검사와 clippy로 확인합니다.

추가 검사 파일을 작성하는 동안 선행 fmt check가 미포맷 줄을 관찰해 exit 1이었습니다. 추가 파일의 cargo fmt 뒤 최종 check는 exit 0이며 이를 제품 동작 실패로 보고하지 않습니다. 실제 설치/앱 GUI 실행은 하지 않습니다.

## 남은 전체 gate와 테스트 부채

- [x] 후속 [종료 QA](2026-09-27-exit-drain-lifecycle.md)에서 정상 ExitRequested의 owned coordinator·감독 실제 완료/마지막 설치 lease 대기와 직접 Exit의 설치 backstop을 구현/검증했습니다. 실제 native 이벤트 실기·비설치/nested worker gate는 별도이며 store shutdown만을 전체 join으로 해석하지 않습니다.
- [x] 후속 자기 group fixture에서 TERM 무시 자손은 TERM 뒤 살아 있고 store/supervisor 종료 뒤 사라짐을 확인했습니다. 모든 가능한 자손/OS 종료 효과를 증명한 것은 아닙니다.
- [x] 후속 [부모 선종료 QA](2026-09-27-lsp-install-parent-exit-lifecycle.md)에서 자손 생존을 재현하고 회수 없는 부모 관찰→그룹 KILL→부모 wait로 수정했습니다. 회수 플래그로 늦은 취소의 PID 재사용을 막습니다. 모든 자손/OS의 bounded 종료까지 증명한 것은 아닙니다.
- [ ] Windows process tree·OS 강제 종료·실제 앱/설치기 실기는 실행 환경/사용자 실기가 필요한 gate입니다. child wait 실패나 비정상 OS 종료에서도 cleanup이 항상 성공한다고 주장하지 않습니다.
- [x] 일반 LSP wait worker와 reader의 소유·완료는 후속 M6-JI~JK에서 자기 fixture로 검증했습니다. 이는 설치기 OS 오류나 PTY 완료 증거가 아닙니다.
- [ ] PTY의 SIGHUP 무시/OS·Windows 분기, 전체 M6·M7/M8와 Phase 0 실기 gate는 별도 완료해야 합니다.
