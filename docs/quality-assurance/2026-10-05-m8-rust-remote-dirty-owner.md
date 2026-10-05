# M8 실제 원격 dirty·선택 소유자

## 결과·범위

BrowserEditor가 actual FileEvent를 소비해 같은 문서의 모든 탭 dirty 전이를 전송하고, 실제 ChoiceRequested와 request_disk_choice를 dirty 응답 완료 이후의 새 파일 조회로 연결합니다. 직전에는 after_dirty_flush라는 caller 계약만 있었고 이번에는 제품 Rust 소유자가 그 flush를 수행합니다. 파일 사건은 take_file_events로 전달하며 take_dirty_failures/retry_dirty는 실패 관찰·명시 재시도입니다. 시험용 dirty 전송 caller를 제품으로 쓰지 않습니다.

새 pure3/선택 대기 actual UI1/새 Chrome-Wasm dirty-owner1와 신규 dirty-close1이 각각 PASS입니다. 후속으로 전송 전 Closed 미공개 RED를 수정했으며 그 실패1·새 실제 종료 경계만 검사하고 이전 성공한 browser 시나리오는 반복하지 않았습니다. 미러 복원·조건부 정리/persistence/LSP/auto-save·전체 App/canvas/제품 자산과 최종 게이트는 미완료입니다. 후속47 2/4(50%)·전체363/433(83.83%, 비가중)·최종0/8·전체 ETA 산정 보류·goal active·전체 완료 전 Git 없음·main 직접입니다.

## 대상·원본 계약

- `native/taide-remote-web/src/dirty.rs`: 탭별 desired/confirmed·seq별 in-flight·전이 병합·단일 탭의 동시 전송 제한·실패/Closed 공개를 구현했습니다. 처음 clean 상태는 전송하지 않습니다. 진행 중 새 값은 이전 ack 뒤 최신 전송으로 이어집니다. 해제된 탭의 늦은 응답은 소비하지만 소유자를 되살리지 않습니다. 오류 뒤에는 상태를 불확정으로 유지해 false→true 전송 중 다시 false가 된 경우에도 과거 false 확인값을 믿지 않습니다. Closed는 자동 재전송을 막고 retry는 과거 요청이 아니라 현재 desired를 보냅니다.
- `native/taide-remote-web/src/{files.rs,browser-editor.rs,lib.rs}`: Loaded/Edited/성공 SaveFinished/ChoiceFinished에서 실제 core snapshot을 읽어 모든 view의 TabId를 갱신합니다. renderer의 ChoiceRequested도 같은 소유자가 queue_choice→전체 dirty flush→fresh file_open→core choose_disk로 실행합니다. 대기 단계부터 입력·저장·해당 문서 일반 조회를 잠그며 failed dirty는 선택을 실패로 종료하고 조회를 보내지 않습니다. waiting request의 revision/view guard·unbind/Closed·dirty 보존을 유지합니다.
- 실제 프로덕션 상태/오류/응답은 한 Workbench/Client의 소유자 경로이며 별도 socket이나 JS fallback을 만들지 않았습니다. 직접 core store_mut으로 편집하는 외부 caller는 정상 renderer/cleanup의 Edited 경로와 구별해야 합니다. 최종 App은 해당 실제 사건을 쓰고 dirty 오류/재시도를 UI에 연결해야 합니다.
- `tests/dirty.rs`, `tests/files.rs`, test-only FileProbe/files.html/tool dirty-owner는 이 경계만 확인합니다. 기존 수동 dirty flush 모드는 역사적 primitive 검사용으로 남기고 새 모드는 그 모드를 호출하지 않습니다.

원본 `src/widgets/editor-pane/use-editor-file-persistence.ts`의 실제 dirty 전이/settle/mirror restore와 native `application.rs`의 submit(SetDirty)/flush_dirty/choose_disk를 읽었습니다. 실제 원격 명령은 `remote-layout.rs`의 layout_set_dirty이며 args tabId/dirty·unit null 응답입니다. egui/UI API는 기존 검증한 실제 shared renderer를 유지했고 새 의존성·버전·manifest/lock/MSRV·제품TS/vendor/OS/보호 앱/사용자 데이터는 변경하지 않았습니다. dirty 완료를 미러 flush 완료와 혼동하지 않습니다.

## 최소 검사

Cargo 환경은 이전 파일 QA와 같습니다. `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw`를 사용했고 Cargo는 직렬 실행했습니다.

- [x] `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test dirty`: 새2 PASS(build0.23초/suite0.00초)입니다. 첫 시도는 시험 코드가 temporary Vec의 slice를 빌려 E0716으로 실행 전 실패했으며 Vec 수명 바인딩으로 정정한 뒤 처음 실행됐습니다. 제품 실패나 성공 반복으로 세지 않습니다. 두 탭의 dirty/coalesce·이전 true ack 뒤 최신 false·unbind/늦은 응답·binary/remote error·Closed 미확정 상태/no replay·명시 retry의 현재 false를 확인했습니다.
- [x] `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test files 원격_choice_flush_대기는_실제_입력을_잠그고_조회_저장_해제와_closed를_소유한다 -- --exact`: 새1 첫 실행 PASS(build0.43초/suite0.02초)입니다. waiting 단계의 실제 Text 입력 비적용·save/read 제한·문서 유지·해제 후 NotFound·Closed 사건/dirty 보존입니다. 이전 성공한 파일 tests는 실행하지 않았습니다.
- [x] production Wasm lib clippy0.25초·native lib/dirty test clippy0.28초·새 probe Wasm build1.38초/clippy0.39초·tool strict TypeScript exit0입니다. 모두 strict는 `-D warnings`입니다. Rust6 exactfmt/lib skip_children와 tool/HTML Prettier·related whitespace 최종 검사를 기록했습니다. native App/배너 코드 변경은 없어 해당 성공을 재사용했습니다.
- [x] `bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built dirty-owner`: 새 실제 Wasm 시나리오 첫 실행 PASS입니다. approved synthetic localhost/fresh headless Chrome/mock keychain·SW block/downloads off이며 사용자 OS/키체인/보호 앱은 조작하지 않았습니다.

## 실제 브라우저 결과

`/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/dirty-file-result.json`이 정본입니다. 기존 continuous/prepared-config/disk-choice 결과 파일은 보존하고 해당 시나리오는 재실행하지 않았습니다. 최신 probe/binding은 같은 file-built에서 새 source로 생성했으므로 선행 결과의 source/artifact 경계와 구별합니다.

초기 문서1/view2·disk clean에서 실제 Text 입력으로 typed disk/dirty=true가 되고 실제 제품 소유자의 dirty 요청을 서버 fixture에서 지연했습니다. 디스크 변경 조회 뒤 owned ViewDisk를 요청한 동안 문서 내용·일반 조회 수2가 그대로이며 추가 입력도 비적용입니다. dirty null 응답을 풀자 fresh read→external1/clean·자동 dirty=false가 이어졌습니다.

다음 실제 입력의 dirty 전송1을 remote 오류로 실패시켰습니다. owned ViewDisk는 ChoiceFinished 실패를 공개하고 file_open을 보내지 않으며 draft/dirty를 유지했습니다. 명시 retry 후 owned KeepMine은 최신 디스크 baseline·draft/dirty 유지·conflict 해제를 적용합니다. 실제 save가 draft를 저장한 뒤 자동 dirty=false·확인 조회까지 완료했습니다. 시험용 수동 dirtyFlushes는0입니다.

실제 seq1~18 고유/read6/save1/dirty5(`[true,false,true,true,false]`, 오류/명시 retry 포함)입니다. 예상된 dirty failure1/선택 failure1만 공개되고 response 누출/pageerror는0입니다. dispose 후 connected=false/socket0·quiet1.1초 wake19/요청/추가 연결 불변·Drop을 확인했습니다. egui Text shape 확인이지 canvas 픽셀/전체 GUI 통과가 아닙니다.

## 남은 경계

### 후속: 전송 전 Closed와 실제 재연결

`dirty_owner는_전송전_closed도_한번만_공개하고_명시_retry까지_보류한다 -- --exact`는 새 RED(failure0/기대1, build0.98초/suite0.00초)였습니다. unsent desired를 freeze하면서 failure 목록에는 추가하지 않은 것이 원인입니다. next_updates도 Closed로 종료·공개해 같은 disconnect를 반복해도 중복 사건을 만들지 않도록 수정한 뒤 해당 실패1만 GREEN(build0.49초/suite0.00초)입니다. 실제 browser poll은 기존 renderer 사건을 socket 사건보다 먼저 소비하고, 같은 poll의 late 파일 사건도 disconnect 이후 자동 재전송하지 않도록 정리합니다. dispose 역시 pending renderer 사건부터 소비합니다. dirty_flush_state로 실제 ack 완료 여부를 공개합니다.

영향 Wasm strict0.26초는 종료 처리 변경 후이며 dirty_flush_state accessor 추가 전 결과입니다. 최신 source는 새 probe 최종 build0.77초·최종 tool strict TypeScript·실제 dirty-close runtime으로 검증했습니다. 중간 probe build1.39초는 이후 accessor/fixture가 바뀌어 최종 검증으로 세지 않습니다. 이전 pure2/waiting UI/browser dirty-owner 성공은 반복하지 않았습니다.

`bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built dirty-close` 새1이 첫 실행 PASS했습니다. 실제 renderer 편집 사건 직후 연결을 끊고 새 연결/recovery를 기다렸습니다. `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/dirty-close-result.json`의 recovery1/document1/view2·typed disk/dirty=true·Closed failure1·서버 dirty 수신0/save0·1.1초 자동 요청0입니다. 명시 retry 뒤 현재 dirty=true1회와 실제 dirtyFlushed=true를 확인한 후 dispose했습니다. 실제 서버 수신 seq16개가 고유하며 seq9는 서버 수신 목록에 없는 간격입니다. 이를 연속 seq라고 주장하거나 원인 로그 없이 특정 네트워크 원인으로 단정하지 않습니다. 마지막 socket0/connected=false·추가1.1초 wake18/요청/연결 불변·pageerror/응답누출0입니다.

기존 dirty-file-result.json 및 선행 성공 결과는 보존했고 같은 file-built의 최신 artifact만 다시 생성했습니다. 따라서 dirty-owner 첫 성공 자료는 종료 후속 전 source이고 dirty-close는 최종 종료 후속 source의 새 증거입니다. 실제 App/전체 mirror/GUI 완료가 아닙니다. bug 문서는 `docs/bug/2026-10-05-remote-dirty-closed-notification.md`입니다.

- [ ] mirror 조회/복원/epoch·지연 저장·compare-before-clear·프로젝트/루트·hot-exit/auto-save/LSP/저장 참여자와 실제 제품 UI 오류·명시 retry를 연결합니다.
- [ ] browser App/canvas/모든 surface·폰트/생산용 자산·전체 remount/종료와 최종 M8 게이트를 검증합니다. CJK·VoiceOver 실기는 사용자-last입니다.

이 소유자 경계가 완료돼도 후속47/최종 N1~N8은 아직 완료로 체크하지 않습니다. 검사 종료 후 live server/browser/Cargo handle은 없습니다.
