# 정상 종료 드레인 QA

> 현재 판정: [M6 직접 Exit 결합 게이트](2026-09-29-m6-direct-exit-combined-gate.md) 완료. 아래 미완료 표기는 각 검사 당시의 잔여 범위이며 OS stall·미등록 자원·Windows process tree는 여전히 검증하지 않았습니다.

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

## 2026-09-28 직접 Exit 보강

- [x] `cargo test -p taide-runtime 직접_종료는_감독_작업_설치_ai_owner의_완료를_모두_기다린다 --lib`: 새 API 부재 E0599(exit 101)를 먼저 확인한 뒤 1건 통과했습니다. 각 owner의 완료 전 대기와 신규 입장 거절을 확인합니다.
- [x] `cargo test -p taide-runtime exit_drain::tests --lib`: 기존 종료와 첫 직접 fixture 8건 통과했습니다. 이후 추가한 LSP·PTY 직접 callback 검사는 `cargo test -p taide-runtime 직접_종료는_lsp와_pty_callback_반환까지_기다린다 --lib` 1건 통과로 확인했습니다.
- [x] `cargo test -p taide --lib 직접_exit은_전체_자원_drain을_사용한다`: 직접 `Exit` 분기의 같은 drain 및 네 State 인수 source contract 1건 통과했습니다.
- [x] `cargo clippy -p taide-runtime -p taide --lib --tests -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps --quiet`, `cargo fmt --all -- --check`, `git diff --check`: 최종 변경 범위에서 모두 exit 0입니다. 신규 acknowledge/history 및 이 QA 파일의 대상 Prettier check도 exit 0입니다. 공개 IPC/생성 입력을 변경하지 않아 실제 bindings/manifest diff가 없습니다.
- [ ] 실제 앱 `Exit` 이벤트 순서, 메인 스레드 callback 교착·OS I/O stall, 강제 OS 종료·등록되지 않은 nested 자원 회수는 합성 검사에서 증명하지 못했습니다. M6/M7 실기에서 판단합니다.

## 2026-09-28 격리 GUI 관찰

- [x] 권한 허용 debug 앱을 전용 identifier·빈 `ZDOTDIR`·별도 `CLAUDE_CONFIG_DIR`로 실행했습니다. 실제 창의 명령 팔레트 키보드 열기/닫기, 자기 임시 프로젝트 열기, `fixture.txt` 편집기 표시를 접근성 트리에서 확인했습니다. 사용자 설정·lockfile 내용은 읽지 않았습니다.
- [x] 프로젝트가 열린 상태에서 `⌘Q` 후 `App quit`, IDE listener 65299 부재, 격리 경로의 `65299.lock` 부재를 관찰했습니다. 기존 사용자 홈 `45059.lock`은 사용자 승인에 따라 앞서 휴지통으로 옮겼고 재생성되지 않았습니다. [실측 기록](../history/2026-09-28-m6-isolated-app-attempt.md)을 참조합니다.
- [ ] `RunEvent::ExitRequested`와 직접 `RunEvent::Exit`의 실제 이벤트 순서, 진행 중 watcher/PTY/LSP/원격 작업의 drain, 메인 스레드 교착·OS I/O stall을 계측합니다. 이번 GUI 관찰만으로 해당 직접 경로의 동작이나 M6 전체를 완료 처리하지 않습니다.

## 2026-09-29 실제 이벤트 계측

- [x] 계측한 debug 앱 세 실행에서 `ExitRequested → drain 완료 → ExitRequested → Exit → 직접 drain 완료` 순서를 확인했습니다. 세 실행 모두 앱 exit 0, 종료 실패·panic 로그 없음, 격리 IDE 포트·lockfile 제거를 확인했습니다. [실측 기록](../history/2026-09-29-m6-exit-event-trace.md)에 빌드·환경·범위를 분리했습니다.
- [x] 임시 프로젝트 watcher가 인덱싱한 상태에서 종료했고, 별도 실행에서는 PTY의 `/bin/sleep 120` PID 55613과 vtsls PID 55725가 각각 종료 직후 사라졌습니다. 이는 실제 자식 회수 관찰이며 모든 자원을 동시에 바쁘게 만든 검사는 아닙니다.
- [ ] 선행 `ExitRequested`가 없는 직접 `Exit`, 활성 원격 서버/WebSocket, 여러 자원의 동시 지연·OS I/O stall은 실앱 미검증입니다. 합성 직접 Exit 결과를 이 실측과 혼동하지 않습니다.

## 남은 gate

- [x] 계측한 `⌘Q`의 native `ExitRequested → drain 완료 → ExitRequested → Exit → 직접 drain 완료` 순서는 후속 격리 앱 세 실행에서 확인했습니다. 선행 요청 없는 직접 Exit·메뉴 callback과 모든 중첩 자원 동시 완료까지 일반화하지 않습니다.
- [x] 후속 [활성 원격 직접 Exit 실측](2026-09-29-m7-remote-direct-exit-verified.md)에서 선행 `ExitRequested` 없는 `applicationWillTerminate → 직접 종료 이벤트 수신 → 직접 종료 자원 대기 완료`, 활성 WebSocket·PTY 자식과 포트 정리를 한 번 확인했습니다. 이 결과는 아래의 LSP·watcher 동시 지연이나 OS stall 판정으로 확대하지 않습니다.
- [x] 후속 [일반 LSP wait QA](2026-09-27-lsp-process-wait-lifecycle.md)에서 정상 coordinator가 제거/교체된 세션의 wait·reader·exit callback 완료도 기다리도록 구현/검증했습니다. child exited만으로 ready를 세우지 않습니다.
- [x] 직접 Exit의 등록 자원 대기: 합성 감독 operation·설치 lease·AI owner 및 자기 `/bin/sh`의 LSP/PTY callback 완료 검사가 통과했습니다. Tauri source contract도 직접 `Exit`의 같은 drain 호출을 확인했습니다. 정상 종료와 직접 종료는 이제 같은 등록 자원 대기 함수를 사용합니다.
- [ ] 실제 native 직접 Exit·exit runtime 요청 실패, 감독되지 않은 nested blocking worker·PTY thread와 메인 이벤트 루프 교착 여부를 판정합니다. 등록되지 않은 자원 전체의 완료를 주장하지 않습니다.
- [x] 후속 [부모 선종료 QA](2026-09-27-lsp-install-parent-exit-lifecycle.md)에서 부모가 먼저 종료한 자기 그룹의 자손 생존을 재현/수정했습니다. 원래 exit code·reader 회수와 회수 뒤 그룹 재신호 금지를 확인합니다.
- [ ] 그룹을 벗어난 자손·Windows process tree·OS 강제 종료의 cleanup을 판정합니다. 모든 자손의 실제 wait/join과 bounded 종료를 증명한 것은 아닙니다.
- [ ] M6 전수 body 판정과 M7/M8·Phase 0 실기는 별도 완료해야 합니다.
