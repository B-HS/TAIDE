# M8 Rust 브라우저 종료 drain

## 대상 파일

- `native/taide-remote-web/src/close.rs`, `files.rs`, `browser-editor.rs`, `browser-application.rs`, `lib.rs`
- `native/taide-remote-web/tests/close.rs`, `tests/files.rs`
- `native/taide-remote-web/tests/browser-probe/src/application-probe.rs`, `application.html`
- `tools/m8-remote-rust-file-probe.ts`

## 리포트

BrowserApplication의 명시 close 요청이 실제 진행 중 file_save·대기/전송 중 디스크 선택·dirty ack·미러 write/cleanup ack를 모두 기다리고 Ready일 때만 동일 소유자를 dispose합니다. 실패는 연결과 초안을 유지한 Failed 상태이며 취소 후 명시 재시도로 진행합니다. 신규 portable 2건과 실제 Chrome/Wasm 연속 1건이 각 첫 실행 PASS입니다. 기존 pump/mirror 성공은 반복하지 않았습니다.

## 원본 대조와 범위

원본 native `NativeApplication::close`는 진행 중 저장/선택을 막고, close 실패 때 연결을 복원합니다. TypeScript `hot-exit-flush-provider.tsx`는 All/Window/Project scope와 제한 시간의 best-effort handshake이며 오류를 삼킨 후 완료를 알립니다. 현재 구현은 브라우저 Rust 소유자의 명시 dispose drain입니다. 서버 hot-exit handshake/timeout·모든 slot/project/tab close·OS 브라우저 탭 unload로 이 결과를 확대하지 않습니다. 새 dirty 확인 모달은 추가하지 않았습니다.

## 상세 계약

1. `FileViews::has_pending_operations`는 saves/choices/waiting_choice만 확인합니다. 일반 read나 cache refresh를 종료 필수 write로 세지 않습니다. 현재 pending format/code-action/auto-save/LSP의 full pipeline은 아직 연결되지 않았으므로 그 전 단계까지 모두 drain한다고 주장하지 않습니다.
2. portable `drain_state`는 파일 작업·dirty·mirror가 모두 Ready일 때만 Ready입니다. dirty/mirror 오류는 pending 파일이 있어도 Failed로 공개하고 완료로 바꾸지 않습니다.
3. request_close는 Open→Pending으로 전환하고 독립 pump를 예약합니다. Pending에서 현재 미러 의무를 force flush합니다. 파일 SaveFinished/ChoiceFinished 실패는 mandatory consumer가 사건을 가져가기 전에 관찰하고, consumer가 만든 후속 작업도 완료 판단 전에 다시 확인합니다.
4. Pending/Failed 중 외부 update는 Closing으로 거부하지만 readonly snapshot·socket 사건 소비는 유지합니다. 반복 close는 중복 제출/자동 재시도하지 않습니다. cancel_close는 Open으로 돌아갈 뿐 이미 서버로 보낸 작업을 취소하거나 rollback하지 않습니다. 명시 persistence retry는 기존 현재 초안 기반 dirty/mirror API이며 저장 mutation을 자동 재전송하지 않습니다.
5. Ready에서 borrow를 해제한 뒤 dispose를 한 번 수행합니다. 예약 timeout·미러 timer/blur listener·socket을 같은 소유자에서 해제하며 changed callback으로 Ready를 알립니다. 예약 실패는 SchedulerUnavailable입니다. 강제 dispose/Drop은 여전히 abort이며 정상 drain과 구분합니다.

## 검증

- [x] 신규 `tests/close.rs::종료는_진행_작업_dirty와_미러를_모두_기다리고_실패를_완료로_바꾸지_않는다` 첫 PASS입니다. all-ready·파일/dirty/mirror 각 pending·Closed dirty/mirror 오류의 실패 우선순위를 확인했습니다.
- [x] 신규 `tests/files.rs::종료_대기는_조회가_아닌_저장과_대기중_또는_전송중_선택을_소유한다` 첫 PASS입니다. 조회 비차단·actual save_sent/ack·waiting→sent choice/ack·disconnect의 pending 회수와 실패 사건을 확인했습니다. 함께 실행한 build1.00초/각 suite .00초이며 기존 files 11건은 filtered로 반복하지 않았습니다.
- [x] native lib/대상 close/files strict .40초, probe Wasm strict .46초 exit0입니다. 최신 probe build1.37초 및 공식 wasm-bindgen .2.129 binding 생성 뒤 `bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built application-close` 첫 실행 exit0입니다. 결과는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/application-close-result.json`입니다.
- [x] 최신 production Wasm owner 단독 strict .21초, 도구 TS strict와 Rust 8파일 exactfmt·TS/HTML Prettier check exit0입니다. apply_patch의 줄바꿈/백슬래시 context 불일치는 실제 문자열 길이를 확인한 뒤 수정했으며 거절된 patch 뒤 검증을 실행하거나 기능 실패로 세지 않았습니다. 최종 관련 문서/미추적 12파일 공백·끝 newline 문제0, PROCESS active363/433·live handle 없음입니다.

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 같은 경로의 `bin/cargo`, `--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw`입니다. portable 검사 명령은 `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test close --test files 종료 -- --nocapture`이고 strict는 native `--lib --test close --test files -- -D warnings`, probe `--target wasm32-unknown-unknown --lib -- -D warnings`입니다. 기존 의존성/lock/MSRV·제품 TS·보호 앱·OS·사용자 파일·Git은 변경하지 않았습니다. 새 합성localhost/headless Chrome은 mock keychain/service worker 차단/다운로드 금지의 정확한 승격 범위에서만 실행했습니다.

### 실제 runtime 관찰

1. interval/RAF를 throw로 막은 같은 App owner에서 실제 renderer Text 입력 후 dirty RPC를 실패시켰습니다. `Failed(Dirty(Remote(SYNTHETIC_DIRTY)))`/연결 true/socket1·초안 `typed disk`를 확인했습니다. 실패를 완료 처리하지 않았고 Closing update 거부도 확인했습니다. 이는 실패 경로를 의도적으로 주입해 기대대로 관찰한 PASS이며 구현 실패 RED가 아닙니다.
2. cancel→Open·미러 응답 release·명시 persistence retry 후 dirty/미러 Ready입니다. KeepMine의 fresh read를 보류한 진행 선택 중 close는 Pending/socket1이고 취소한 뒤 read release로 선택을 완료했습니다.
3. 다음 실제 입력 `typed typed disk`의 저장과 미러 쓰기를 각각 보류했습니다. Pending/file 작업 true/mirror Pending입니다. 미러 ack만 풀어도 file 작업 true/close Pending/socket1로 저장을 계속 기다렸습니다.
4. 저장 ack를 풀고 dirty=false ack는 보류했습니다. file 작업 false·본문 clean·mirror Ready이지만 close Pending/dirtyReady false/socket1입니다. dirty ack를 풀었을 때만 Ready/disposed true/socket0으로 전환했습니다. 실제 disk는 최신 본문이고 저장으로 mirror는 null입니다.
5. 전체 seq1~17 고유·연속, mirror write2/read1·file open3/save1·dirty3·upgrade1/recovery0·최종 pump26/change20입니다. 종료 후 blur와 1.1초 동안 snapshot/pump/change/요청/추가 연결이 늘지 않았고 마지막 Drop도 수행했습니다. page 오류/남은 응답0입니다. failures 배열은 시험 consumer가 저장·mirror 오류만 수집한 값0이며 dirty의 의도적 실패는 close 상태로 확인했습니다. 모든 오류 스트림이 0이었다고 주장하지 않습니다.

## 남은 실제 경계

- actual App canvas/모든 원본 surfaces/consumer의 close 진입점·화면 비활성화/실패 피드백을 이 소유자에 연결해야 합니다. server All/Window/Project handshake와 scoped view-state/LSP/untitled flush·slot/tab/project mutation은 별도 미완료입니다.
- 응답 없는 서버의 close deadline/사용자 취소 UI·receipt 전 단절의 불확실 완료·외부 writer/ViewDisk·원본 모든 persistence/save/format/code-action/auto-save/LSP는 현재 결과로 완료하지 않습니다.
- 실제 브라우저 탭 종료/unload는 async 완료를 보장할 수 없으므로 이 명시 drain을 OS unload 검증으로 대체하지 않습니다. 현재 timeout 정책을 임의로 force-success 처리하지 않습니다.
- 제품 Rust bundle·전체 App/성능/보안/beta/install/rollback/cutover/Rust99%/제품 TS 제거·N1~N8와 CJK/VoiceOver 사용자-last는 계속 남습니다. 필수 항목을 테스트 부채로 옮겨 완료 처리하지 않습니다.

## 참조

source 원본은 `native/taide-native-app/src/application.rs`, `src/app/providers/hot-exit-flush-provider.tsx`, `src/entities/editor/mirror-flush-registry.ts`입니다. [RefCell try_borrow_mut](https://doc.rust-lang.org/std/cell/struct.RefCell.html#method.try_borrow_mut)와 [Window timeout](https://docs.rs/web-sys/latest/web_sys/struct.Window.html#method.set_timeout_with_callback_and_timeout_and_arguments_0)의 기존 확인 근거를 재사용합니다.
