# M8 원격 Rust 파일 소비자·공유 편집기·저장 수명

후속 최신 결과는 `2026-10-05-m8-shared-file-indent-and-save.md`입니다. actual per-file 명시 indent와 same native cleanup/save-preparation·prepared-config Chrome/Wasm을 연결했고 신규5건이 각1회 PASS입니다. 아래는 최초 file consumer 경계의 이력입니다. 자동감지/manual model/화면 tab-stop·전체 remote format/LSP/auto-save/mirror·App/canvas/제품 gates는 여전히 미완료입니다.

## 결과와 경계

FileViews/BrowserEditor의 실제 file_open/file_save 연결, 공용 EditorStore/NativeEditor 사용과 해당 응답 수명을 검증했습니다. 순수 신규3건·native admission 영향1건·새 Chrome/Wasm 연속1건이 각각 한 번 PASS입니다. 통과한 기존 shell/presentation/transport/App 검사는 재사용했습니다. 후속47 2/4(50%)·전체363/433(83.83%, 공수비 아님)·최종 N1~N8 0/8·전체 ETA 산정 보류·goal active이며 전체 완료 전 Git 작업은 하지 않았습니다. process/verify/save-docs를 main이 직접 적용했습니다.

이는 제품 전체 파일 UI나 M8 완료가 아닙니다. 원본 format-on-save/code actions/cleanup, auto-save, hot-exit mirror, LSP, tabs/dirty/Git 효과, conflict/read-only/error/loading 배너, per-file editorconfig, preview와 전체 App/canvas·폰트·제품 bundle/packaging·handoff/failed-close/GUI/성능/beta/cutover/Rust99%가 남습니다. save_prepared는 실제 formatter 처리 뒤 캡처할 원시 저장 경계이며 이 선행 처리들을 no-op으로 대체하거나 완료로 세지 않습니다. 시험용 HTML/Wasm은 제품 화면이나 fallback으로 넣지 않습니다.

## 대상 파일과 원본 대조

- `native/taide-native-editor/src/store.rs`: native open_file_with_draft의 absolute/ParentDir 검사는 유지하고 기존 문서 admission/draft 본문을 공유합니다. open_remote_file은 서버가 검증한 OpenedFile.path를 opaque identity로 등록하며 empty/NUL은 거절합니다. 브라우저 OS 경로로 canonicalize하지 않으며 호스트 FS 권한은 추가하지 않습니다. 같은 tier/lossy/read-only/byte quota·공유 document/view·undo·disk/save baseline을 사용합니다.
- `native/taide-remote-web/src/{files.rs,browser-editor.rs,lib.rs}`: FileViews의 typed DTO·view bindings·owned pending seq·오류와 FileEvent, 실제 BrowserWorkbench 한 연결을 사용하는 BrowserEditor, shared NativeEditor.show를 연결했습니다. BrowserEditor는 raw/mutable Client를 공개하지 않고 다른 consumer의 응답·채널·이벤트를 반환합니다. EditorLimits는 실제 caller가 제공하며 임의 제품 quota를 새로 정하지 않습니다.
- `tests/files.rs`, `tests/browser-probe/{src/file-probe.rs,src/lib.rs,files.html}`, `tools/m8-remote-rust-file-probe.ts`: 검증 전용 합성 fixture입니다. 실제 소비자/core/shared renderer를 실행하되 egui shape 생성 검사는 픽셀/canvas 실기 검사가 아닙니다.

원본 file.ipc/file.query/use-editor-file-persistence·ipc-sync-provider와 native remote-files·공용 core admission/save/observe를 읽고 대조했습니다. file_save의 정상 JSON은 객체나 파일 DTO가 아닌 unit/null입니다. fs:changed의 change.paths는 원본처럼 exact path membership이며 fs:rescan-required는 bound file 도메인을 갱신합니다. 원격 인가·root guard는 actual server에 남아 있으며 브라우저 경로 저장은 서버 파일 권한 우회가 아닙니다.

## 구현 계약

- [x] 같은 서버 path의 여러 ViewKey는 한 document/한 read를 공유합니다. 초기 loading에 빈 파일을 만들어 표시하지 않고 binary/malformed/remote/invoke 오류를 공개합니다. 오류 무한 자동 retry 없이 명시 retry와 recovery를 지원합니다.
- [x] invalidated in-flight read는 소유한 seq를 소비하되 옛 값을 적용하지 않고 최신 조회를 한 번 보냅니다. 기존 document의 재조회는 observe_file을 사용해 dirty 초안과 disk conflict 정책을 유지합니다. 저장 중에는 해당 document의 새 read를 보내지 않습니다.
- [x] 하나의 document에 저장 하나만 진행하며 캡처한 실제 SaveSnapshot을 정상 null 응답에서만 mark_saved합니다. 응답 전 추가 edit은 보존하고 saved baseline만 갱신합니다. file_save가 mtime을 반환하지 않으므로 확인 read 전 None이며 임의 시각을 만들지 않습니다. 성공 후 aliases를 무효화하고 observable SaveFinished를 제공합니다.
- [x] disconnect는 미완료 save마다 Closed 오류를 노출하고 baseline을 성공으로 바꾸지 않습니다. mutation을 재연결에 replay하지 않으며 새 seq로 read합니다. 마지막 clean view 해제는 document를 회수하고 dirty 초안/진행 중 save는 무단 폐기하지 않습니다. remount는 같은 retained document를 사용합니다.
- [x] shared renderer가 실제 view/core를 사용하며 changed/save_requested를 반환합니다. Edited/Loaded/SaveFinished 효과는 다음 제품 caller가 소비할 실제 이벤트이고 Git/mirror/LSP를 빈 포트로 완료 처리하지 않습니다.

## 실제 최소 검증

Cargo는 `/Users/hyunseokbyun/development/rust/cargo/bin/cargo`, CARGO_HOME은 `/Users/hyunseokbyun/development/rust/cargo`, target은 `/private/tmp/taide-m8-menu-build.j6Efnw`입니다. 일반 명령은 `--locked --offline --target-dir <target>`이며 아래 local lock 갱신 두 경계만 `--offline`을 사용했습니다.

- [x] `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test files`: 신규3 PASS, build1.45초/suite0.02초입니다. Windows 서버 경로·native 경로 거절 유지·shared two views/one document/실제 Text shape·마지막 회수, 중간 save/edit/late read, binary/readonly/path switch/disconnect/late ack/초안 보존을 확인했습니다.
- [x] production Wasm `cargo clippy --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib -- -D warnings`: 최초 filter_map_bool_then style 오류는 동일 predicate filter/map으로 수정했고 영향 static만0.20초 exit0입니다. 제품 동작 검사3건은 반복하지 않았습니다.
- [x] `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test store mirror_복원은_disk_baseline을_유지하고_용량_실패와_공유_문서를_원자적으로_보호한다 -- --exact`: 최초 locked 명령은 model의 local taide-remote-wire edge가 standalone lock에 없어 실행 전101이었습니다. offline 갱신의 diff를 확인했으며 이 local edge/package entry만 추가되고 UUID1.24.0 등 registry 버전은 불변입니다. 실제1 PASS, build5.24초/suite0.00초입니다.
- [x] 새 actual probe Wasm build1.84초·strict0.43초 exit0입니다. probe/browser manifest는 이미 graph에 존재하는 egui0.36.2/native-editor 직접 연결만 추가했습니다. 새 registry package/버전/MSRV/root lock 변경은 없습니다. 기존 wasm-bindgen0.2.129로 별도 `file-built`에 생성했으며 선행 결과를 덮어쓰지 않았습니다.
- [x] 새 tool strict TypeScript, Rust4파일 exactfmt·두 lib skip_children fmt, tool/HTML Prettier가 exit0입니다. 적용 규칙/단일 검증 원칙을 유지했습니다.

## Chrome/Wasm 연속 실측

명령은 `bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built`입니다. 이미 확인된 sandbox loopback EPERM을 반복하지 않고 승인된 격리 localhost/fresh headless Chrome 경계에서 실행했습니다. mock keychain·serviceWorkers block·downloads off, 합성 DTO만 사용했고 사용자 앱/프로필/OS/키체인/홈/파일은 접근하지 않았습니다.

최초0.47초 실패는 Wasm 초기화 전 HTML의 `{}`를 엄격 파싱한 시험 초기 대기 오류였습니다. 첫 ready 대기만 safeParse로 정정한 뒤 실패한 신규 연속 시나리오가 한 번 PASS했고 이전 통과 시나리오는 반복하지 않았습니다.

결과는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/file-result.json`입니다. seq1~21은 고유이고 boot file read1·views2/document1·실제 shared text `disk`, saved state는 `first later disk`/dirty=true/false conflict·saved baseline `first disk`, held old read가 적용되지 않고 확인 read만 Loaded를 추가했습니다. 같은 document 중복 save는 추가 wire 호출을 만들지 않습니다. disconnect된 save는 Closed1·성공 처리0·recovery1/read/초안 보존·mutation replay0입니다. dirty last-unbind는 document1을 보존하고 remount 후 정상 저장은 dirty=false, 마지막 clean unbind는 view/document0입니다. 전체 file_open5/file_save3·upgrades2·응답 누출0·page errors0입니다.

dispose 뒤 actual socket0·1.1초 추가 요청/연결0·wake22 불변·Drop 오류0을 확인했습니다. 결과 JSON의 disposed.connected=true는 dispose click 직후 다음10ms UI poll 전 캡처된 오래된 DOM 값입니다. 이를 실제 post-dispose 연결 상태 false의 실측으로 주장하지 않으며 socket0/quiet 검사의 성공을 구분합니다. 생산용 BrowserShell::dispose는 connected=false와 actual Client dispose를 직접 호출합니다. 이 표본을 정정하려고 같은 성공 시나리오 전체를 반복하지 않았습니다.

## 다음 기존 범위

shared surface caller에 원본 파일별 editorconfig·conflict/read-only/loading/error 배너와 전체 format/save/mirror/LSP 효과를 연결합니다. 실제 browser App/canvas/surfaces·전체 consumer·제품 자산과 최종 N1~N8 게이트는 계속 미완료로 유지합니다. CJK/VoiceOver 실기는 사용자 마지막 순서를 유지하고 보호 spike 앱을 수정·종료하지 않습니다. live Cargo/browser/서버 handle은 없습니다.
