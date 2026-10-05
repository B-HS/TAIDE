# M8 Rust 브라우저 App 사건 pump

## 대상 파일

- `native/taide-remote-web/src/browser-application.rs`, `src/lib.rs`
- `native/taide-remote-web/tests/browser-probe/src/application-probe.rs`, `src/lib.rs`, `application.html`
- `tools/m8-remote-rust-file-probe.ts`

## 리포트

실제 Rust BrowserApplication이 BrowserEditor의 socket·미러 timeout·blur wake를 소유하고 단일 예약 timeout으로 사건을 소비합니다. probe의 interval/animation frame이 없어도 통신·미러 쓰기가 진행됩니다. 사건 소비 callback과 화면 변경 callback은 필수이며 no-op 제품 surface를 만들지 않았습니다. 이 소유자는 실제 canvas/App 전체 consumer 및 close registry의 완성품이 아닙니다.

## 상세

1. 생성 시 동일 BrowserEditor를 소유하고 최초 pump를 예약합니다. 동시 wake는 이미 예약한 timeout으로 합치며 wake callback은 Weak만 캡처합니다. callback은 owner 생존/폐기를 확인하고 실제 `editor.poll()`의 남은 사건을 필수 consumer로 전달합니다. 별도 무상한 사건 큐·새 연결·setInterval·RAF를 추가하지 않습니다.
2. `read`는 읽기만 하며 pump하지 않습니다. `update`는 실제 편집/renderer 작업 뒤 한 pump를 예약합니다. 콜백 재진입은 `try_borrow`/`try_borrow_mut`의 Busy 오류로 반환하며 panic하지 않습니다. read/update/consumer 안에서 dispose 요청이 발생하면 borrow 해제 뒤 teardown을 마칩니다.
3. 단순 화면 update의 후속 pump가 다시 화면 update를 영구 예약하지 않도록 wake에만 변경 알림을 세웁니다. 현재 pump 내부에 새 wake가 발생하면 다음 pump의 알림을 보존하도록 시작 시 현재 알림을 소비합니다. 이 후자는 source 검토 후 수정이며 독립 실패 재현을 했다고 주장하지 않습니다.
4. timeout 예약 실패는 `scheduler_failed`로 공개하고 `retry_pump`는 pump만 다시 요청합니다. 이미 수행한 update를 실패로 되돌려 사업 동작을 중복 재실행시키지 않습니다. Drop/dispose는 예약 timeout과 동일 Editor의 socket·미러 timer/listener를 해제합니다. 실행 중 callback은 임시 Rc가 Inner를 보유하므로 callback 저장소를 자기 호출 중 해제하지 않습니다.
5. 임의 강제 dispose는 graceful close가 아닙니다. 아래 마지막 pending 편집은 의도적으로 abort한 것이며 미러 영속화를 입증하는 close 검사가 아닙니다. 실제 close는 mirror/dirty와 진행 중 저장·선택까지 settle을 기다리는 별도 연결이 남습니다.

## 검증

- [x] 초기 production Wasm strict .18초 exit0입니다. 새 fixture 첫 compile의 SaveFailed 없는 variant/DocumentSnapshot.text 없는 필드/replace_selections 반환 bool 혼동 3곳은 실제 FileEvent::SaveFinished·rope·반환 타입으로 수정했습니다. 그 compile 실패를 기능 RED로 세지 않습니다. 수정 뒤 probe strict .26초 exit0입니다.
- [x] 새 `application-pump` runtime 첫 실행은 첫 미러 쓰기 기대1/관찰0으로 10초 제한 실패했습니다. fixture가 core에 직접 replace_selections만 실행해 FileViews::show의 Edited 사건을 만들지 않은 원인이었습니다. 실제 shared renderer `show_file`에 egui Text 입력을 주도록 수정하고 실패한 새 경계만 재실행했습니다. 기존 mirror-write 및 이전 성공은 반복하지 않았습니다.
- [x] 수정된 production/probe strict .37초·최신 probe build1.06초 및 binding 생성 exit0 뒤 `bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built application-pump` exit0입니다. 결과는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/application-pump-result.json`입니다. 초기 build1.64초는 실패 fixture artifact이며 성공 결과와 구분합니다.
- [x] 최신 production Wasm strict .21초·Rust 4파일 exactfmt·TS strict/도구 및 HTML Prettier check exit0입니다. 명령 환경은 아래와 같으며 이전 코드 동작 성공은 재사용했습니다.

### 실제 runtime 관찰

1. 합성 페이지에서 `window.setInterval`과 `window.requestAnimationFrame`은 호출하면 throw하도록 고정했습니다. 테스트의 snapshot 요청은 읽기만 하고 app poll을 호출하지 않습니다. 초기 loaded1/text disk/doc 준비·socket 연결·프로젝트 snapshot을 확인했습니다.
2. 실제 scope bind 뒤 mirror 조회1·실제 Text 입력 뒤 timer가 자동 write1을 보냈습니다. `typed disk`/dirty true/Ready·timer false·pump12/change10·schedulerFailed false입니다. nested update는 Busy true이며 page panic/error는 없습니다.
3. 다음 실제 입력과 합성 blur 뒤 write2, `typed typed disk`/Ready·timer false·pump14/change12를 확인했습니다. 1.1초 idle 동안 전체 snapshot이 동일해 영구 repaint/pump loop가 없었습니다. blur 후 500ms보다 먼저 전송됐다는 시간 임계값까지 따로 계측하지 않았습니다.
4. 세 번째 실제 입력 후 강제 dispose는 pump15/change12에서 정지했습니다. socket active0이고 dispose 뒤 blur·1.1초 동안 pump/변경/요청/추가 연결이 늘지 않았습니다. write2/read1/upgrade1, seq1~12 고유·연속, 남은 응답/오류0입니다. 다음 Drop도 수행했습니다. graceful flush 완료로 이 abort를 재분류하지 않습니다.

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 같은 경로의 `bin/cargo`, `--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --target wasm32-unknown-unknown`이며 strict는 `--lib -- -D warnings`입니다. 공식 wasm-bindgen .2.129 CLI와 기존 test-only file-built를 사용했습니다. localhost/headless Chrome은 합성 범위 승격·mock keychain/service worker 차단/다운로드 금지이며 사용자 앱·OS·키/환경 파일·제품 TS·의존성/lock/MSRV를 변경하지 않았습니다. 현재 live handle은 없습니다.

## 남은 실제 경계

- 실제 App canvas·원본 모든 ShellSurfaces/consumer가 이 소유자의 read/update와 사건 callback을 사용하도록 연결해야 합니다. 현재 probe를 제품 화면이나 제품 자산으로 쓰지 않습니다.
- close registry·mirror/dirty/진행 저장·선택의 drain, 실패 뒤 취소/현재 상태 명시 retry, receipt 이전 단절의 불확실 완료는 미완료입니다. Drop abort와 정상 hot exit를 구분합니다.
- 시스템의 실제 background timeout 지연·실제 OS 창/GPU/IME/VoiceOver는 이 합성 입력으로 검증하지 않았습니다. CJK/VoiceOver는 기존 사용자-last 계약을 유지합니다.
- 전체 persistence/auto-save/LSP·query GC/sourceMissing UI·제품 Rust bundle/성능/보안/beta/install/rollback/cutover/Rust99%/제품 TS 제거·N1~N8는 계속 필수 작업입니다. 상위 체크박스는 열어 둡니다.

## 공식 참조

[RefCell try_borrow_mut](https://doc.rust-lang.org/std/cell/struct.RefCell.html#method.try_borrow_mut), [Window setTimeout](https://docs.rs/web-sys/latest/web_sys/struct.Window.html#method.set_timeout_with_callback_and_timeout_and_arguments_0)와 기존 BrowserClient/MirrorRuntime 및 installed generated bindings를 대조했습니다. process/verify/save-docs에 따라 정확한 경계만 완료 기록하고 같은 성공을 반복하지 않았습니다.
