# M8 Rust 원격 미러 쓰기 소유자

## 대상 파일

- `native/taide-remote-web/src/mirror-writes.rs`, `mirror-runtime.rs`, `browser-editor.rs`, `mirrors.rs`, `lib.rs`, `Cargo.toml`
- `native/taide-remote-web/tests/mirror-writes.rs`, `tests/mirrors.rs`
- `native/taide-remote-web/tests/browser-probe/src/file-probe.rs`, `tests/browser-probe/files.html`
- `tools/m8-remote-rust-file-probe.ts`

## 리포트

실제 BrowserEditor가 동일 Workbench 연결로 지연 쓰기·응답·epoch·receipt 정리·해제 flush를 소유합니다. dirty와 쓰기 의무를 구별하며 복원/기존 공유 모델 채택은 다시 쓰지 않습니다. 새 고유 검증은 순수 쓰기 상태 3건, cache 1건, 실제 Chrome/Wasm 연속 1건으로 총 5건 PASS입니다. 기존 서버 receipt 및 조회/복원 성공은 재사용했습니다. 이 결과는 전체 browser App/canvas나 제품 번들 완료가 아닙니다.

## 상세 계약

1. 실제 Edited만 500ms 지연을 예약합니다. 편집마다 본문을 복제하지 않고 전송 시 현재 core snapshot을 읽습니다. 같은 project/path는 한 쓰기만 in-flight이며 추가 편집은 합칩니다. 원본 `applyMirrorRestore`와 `adoptUnobservedModelEdit`는 pendingMirror를 세우지 않고 `handleChange`만 세웁니다.
2. 성공한 저장/ViewDisk는 epoch를 전환합니다. 늦은 쓰기가 아직 동일한 현재 초안이고 쓰기 의무가 남으면 현재 epoch로 다시 씁니다. 그 외에는 서버의 정확한 receipt로 조건부 정리합니다. 실패는 공개하고 Closed 뒤 자동 쓰기 재전송을 하지 않습니다. 명시 retry는 현재 초안을 읽습니다.
3. write receipt는 path/content/ID/쓰기 직후 baseline 상태를 검증합니다. 조회 전 cache를 만들지 않고 기존 cache만 patch합니다. 쓰기/저장 이전 pending 조회는 무효화합니다. file 응답마다 사건을 즉시 소비해 같은 poll의 다음 mirror 응답보다 저장 epoch를 먼저 반영합니다. 실제 runtime에서 동일 poll 배치 순서까지 별도 계측했다고 주장하지 않습니다.
4. Window Performance 기반 단조 시각·가장 이른 deadline의 단일 owned timeout·blur listener를 사용합니다. callback은 owner를 wake할 뿐 직접 RPC하지 않습니다. 해제 시 해당 파일의 쓰기 의무를 강제 flush하고 다른 공유 view가 있어도 빠뜨리지 않습니다. Drop/dispose는 timer/listener를 해제합니다.
5. `flush_mirrors`는 전송을 요청하며 완료를 기다리는 함수가 아닙니다. 실제 close owner는 `mirror_flush_status` Ready까지 기다려야 합니다. dirty 문서 유지와 pending 쓰기를 구분하며 임의 경로/클라이언트 시각으로 정리하지 않습니다.

## 검증과 결과

- [x] `tests/mirror-writes.rs` 신규 3건 첫 실행 PASS(build .28초/suite .00초): 499/500ms, coalesce, single in-flight, 저장 epoch/receipt 정리, 동일 초안 재쓰기, 해제 flush, retarget, 오류/Closed/no replay, 명시 retry, malformed 응답을 확인했습니다. 첫 compile의 존재하지 않는 EditorLimits::default는 실제 구조체의 bounded fixture로 정정했으며 실행 전 compile 실패를 기능 RED로 세지 않습니다.
- [x] `tests/mirrors.rs` 신규 `미러_쓰기와_저장은_미조회_cache를_만들지_않고_대기중_옛_목록을_무효화한다` 1건 PASS(build .73초/suite .00초): never-seed, 기존 목록 patch/remove, 옛 pending 조회 폐기를 확인했습니다. 이전 조회 검사 4건은 반복하지 않았습니다.
- [x] 최신 source의 probe Wasm build 5.34초 및 공식 wasm-bindgen .2.129 binding 생성 뒤 `bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built mirror-write` 첫 연속 실행 exit0입니다. 결과는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/mirror-write-file-result.json`이며 test-only입니다.
- [x] 최신 production Wasm strict .22초, native lib/대상 mirrors·mirror-writes test strict .12초, probe Wasm strict .52초 각각 exit0입니다. 최초 collapsible_if와 시험 assert의 cloned_ref_to_slice_refs 4곳을 수정한 뒤 영향 검사만 재실행했습니다. 성공한 동작 검사는 반복하지 않았습니다.
- [x] 도구 TS strict, Rust 8파일 exact rustfmt, TS/HTML Prettier check exit0입니다. 기존 web-sys에 Performance feature만 추가했으며 버전/lock/MSRV·제품 TS·보호 앱·OS 설정·사용자 파일을 변경하지 않았습니다.

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 같은 경로의 `bin/cargo`, `--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw`를 사용했습니다. production/probe Wasm 검사는 `--target wasm32-unknown-unknown --lib -- -D warnings`, native는 `--lib --test mirrors --test mirror-writes -- -D warnings`입니다. 테스트는 해당 두 integration target의 신규 검사만 실행했습니다. localhost/Chrome은 정확한 합성 범위 승격으로 실행했고 사용자 앱/시스템 설정은 사용하지 않았습니다.

### 실제 Chrome/Wasm 관찰값

1. 같은 문서 1개/view 2개에서 실제 egui Text 입력 후 probe poll을 멈췄습니다. timer 전 wake11/armed true, 1.1초 뒤 wake12/armed false/Pending입니다. owner poll이 없으므로 서버 쓰기는 0이고, poll 재개 뒤 첫 쓰기가 발생했습니다. timer 자체 wake 1회를 확인했으며 interval 기반 지연을 timer 검증으로 대신하지 않았습니다.
2. 첫 쓰기 응답을 보류하고 저장 null ack로 clean 처리한 뒤 늦은 receipt를 풀었습니다. 정확한 조건부 정리 1회, cache 빈 목록, Ready를 확인했습니다. 다음 실제 입력/명시 flush는 두 번째 쓰기이며 쓰기 의무 없는 반복 flush는 추가 전송 0입니다.
3. 다음 입력 뒤 첫 view 해제에서 공유 view가 남아도 세 번째 쓰기를 flush했습니다. 마지막 해제 뒤 document1/view0·dirty 초안 유지·Ready이고 재바인딩/공유 모델 채택은 자동 재쓰기 0입니다.
4. 네 번째 초안의 미전송 의무 중 연결을 끊었습니다. recovery1/Closed 실패1/타이머 해제·초안 유지이며 자동 쓰기는 여전히 3회입니다. 명시 retry만 현재 네 번째 초안으로 write4를 보냈습니다. seq1~32는 고유·연속이고 mirror write4/clear1/read4, file open3/save1, dirty4입니다.
5. dispose 뒤 connected false/socket active0/timer false/wake35·응답 누출0/page 오류0/저장 실패0/복원 실패0입니다. dispose 이후 합성 blur와 추가 1.1초 동안 wake/요청/추가 연결이 늘지 않았습니다. 기존 file-result/dirty/mirror-restore 결과는 이전 source이며 새 쓰기 검증 결과로 재분류하지 않습니다.

## 남은 실제 작업

- 전체 App의 독립 event pump·background blur/close registry·flush 완료 대기 및 제품 canvas/surfaces/모든 consumer를 연결해야 합니다. timer wake만으로 실제 hidden App/OS hot exit를 검증했다고 하지 않습니다.
- ViewDisk의 목록 기반 expected CAS는 이전 MirrorEntry 모드이며 별도 writer의 동일 Entry 충돌에 receipt ID 보장을 주장하지 않습니다. unknown cache의 지연 clear와 새 입력 취소는 구현했지만 외부 writer/복구 선택 전체 UI는 아직 미완료입니다.
- 응답 receipt를 받기 전 연결이 끊겨 서버 쓰기 완료 여부가 불명확한 경우는 Closed/no replay로 공개합니다. cold crash·불확실한 cleanup 프로토콜의 전체 close 보장은 아직 입증하지 않았습니다.
- sourceMissing SaveAs UI·query GC/rescan throttle·auto-save/LSP/save 전체 pipeline·실제 Rust public bundle와 N1~N8/성능/보안/beta/install/rollback/cutover/Rust99%·제품 TS 제거는 남습니다. 필수 미완료를 테스트 부채로 옮겨 전체 완료 처리하지 않습니다.

## 공식 참조

기존 generated web-sys .3.106 bindings와 [Window timeout](https://docs.rs/web-sys/latest/web_sys/struct.Window.html#method.set_timeout_with_callback_and_timeout_and_arguments_0), [Performance now](https://docs.rs/web-sys/latest/web_sys/struct.Performance.html#method.now)를 대조했습니다. source 원본은 `src/widgets/editor-pane/use-editor-file-persistence.ts`, `src/shared/constants/mirror.ts`입니다. process/verify/save-docs에 따라 성공 근거를 재사용하고 상위 PROCESS gate를 열어 둡니다.
