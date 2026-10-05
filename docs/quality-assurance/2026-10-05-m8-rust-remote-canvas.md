# M8 Rust 브라우저 canvas host

## 대상 파일

- `native/taide-remote-web/src/canvas.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`
- `native/taide-remote-web/tests/browser-probe/src/canvas-probe.rs`, `src/lib.rs`, `canvas.html`, `Cargo.toml`, `Cargo.lock`
- `tools/m8-remote-rust-file-probe.ts`

## 리포트

실제 eframe WebRunner의 canvas·GPU·DOM 입력·repaint와 동일 BrowserApplication을 연결했습니다. mandatory CanvasContents가 사건 소비/원본 화면/종료 화면/raw input을 제공해야 하며 기본 no-op 제품 화면을 제공하지 않습니다. 신규 실제 canvas 입력·미러·정상 종료 한 건이 첫 PASS이고, 캡처의 합성 동일 색상 문제는 구분되는 테마로 별도 읽기 렌더 한 건만 확인했습니다. 앞선 성공 검사는 재실행하지 않았습니다.

## 상세

1. 실제 eframe App::ui는 같은 BrowserApplication update에서 provider를 호출합니다. close/cancel action은 editor borrow 밖에서 수행해 재진입 Busy를 만들지 않습니다. Pending/Failed는 show_close만 호출하며 편집 raw input은 Open일 때만 전달합니다. 실제 사건 처리는 RAF가 아닌 앞선 독립 pump 소유자를 유지합니다.
2. changed callback은 Weak runner/Weak application slot을 사용합니다. Ready에서 실제 WebRunner::destroy를 호출해 DOM listener/ResizeObserver/예약 RAF/GPU runner를 회수합니다. 아직 Ready가 아니면 실제 egui context에 repaint를 요청합니다. 강제 abort/Drop은 Application dispose이며 graceful drain으로 세지 않습니다.
   강제 abort 뒤 `close_state()`는 None이며 Ready를 합성하지 않습니다. 시험 UI는 이를 Disposed로 표시합니다. 정상 Ready는 실제 Application 상태의 Some(Ready)로만 제공됩니다.
3. async 시작 전에 임시 host를 구성해 start 실패/취소에서도 Drop이 부분 생성된 runner를 회수합니다. egui memory persistence는 꺼져 있고 eframe persistence feature를 활성화하지 않았습니다. OS 설정·홈·사용자 profile·clipboard·보호 앱은 변경하지 않았습니다.
4. canvas는 optional feature입니다. 기존 native와 같은 vendored eframe 0.36.2/egui input 패치를 사용하고 기존 버전을 변경하지 않습니다. standalone browser/probe lock 각각에 eframe 전이 package 229개가 추가됐습니다. Tokio/platform lock 항목이 있더라도 실제 Wasm normal dependency graph에는 taide-runtime/taide-infra/terminal/Tokio가 없습니다. root/native lock·MSRV 1.95는 이 변경에서 유지했습니다.
5. 테스트용 CanvasProbe는 합성 단일 파일만 렌더합니다. 원본 ShellSurfaces/전체 CanvasContents 제품 구현·진입점·빌드 자산·패키징을 대신하지 않습니다. GPU backend 선택을 강제로 바꾸거나 JS renderer fallback을 추가하지 않았습니다. 현재 증거는 headless Chrome의 기본 toolkit renderer이며 특정 WebGPU/WebGL backend를 실측했다고 주장하지 않습니다.

## 확인한 공식 근거

- [eframe App 계약](https://docs.rs/eframe/latest/eframe/trait.App.html)과 정확한 vendored `eframe/src/web/web_runner.rs`의 start/destroy/자원 소유자를 대조했습니다.
- [wasm-bindgen async export](https://rustwasm.github.io/docs/wasm-bindgen/reference/js-promises-and-rust-futures.html)의 futures edge를 따릅니다. 기존 전이 버전 0.4.79를 canvas probe의 직접 optional dependency로 연결했습니다.
- [Playwright locator screenshot](https://playwright.dev/docs/api/class-locator#locator-screenshot)을 사용해 실제 canvas만 캡처했습니다.

## 검증

- [x] 최초 production canvas Wasm check 19.36초 exit0, Weak/action 변경 후 임시 host 이름 충돌 E0308을 정정한 production strict .36초 exit0입니다. 최초 probe async macro E0433의 직접 futures edge 누락을 수정한 probe strict 4.23초 exit0입니다. 이 둘은 컴파일 실패이며 runtime RED로 세지 않습니다.
- [x] `cargo build --manifest-path native/taide-remote-web/tests/browser-probe/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --target wasm32-unknown-unknown --features canvas` 23.19초 exit0, 공식 wasm-bindgen 0.2.129 binding 생성 exit0입니다. Cargo는 기존 지정 CARGO_HOME/toolchain을 사용하고 직렬 실행했습니다.
- [x] `bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built canvas` 첫 실행 PASS입니다. 실제 disk→canvas DOM 입력 `typed disk`/Text1→dirty→500ms 미러 write1→close Ready/socket0입니다. seq1~11 고유·연속, frame4→12/마지막12·pump9→18→19, 종료 후 blur/1.1초 추가 frame·pump·요청·연결0, failures/panic/page error0입니다. 결과는 `canvas-result.json`입니다.
- [x] 첫 `canvas.png`는 글자·배경 동일 `#102030`인 합성 fixture로 단색이었습니다. 제품 코드 변경 없이 `canvas-visual`의 구분되는 합성 theme만 별도로 실행했습니다. 입력/미러/정상 close 성공을 반복하지 않았습니다. 새 결과 seq9/frame4/pump10/Drop/socket0/quiet 요청0/오류0, `canvas-visual.png`에서 실제 line number 1/본문 disk/caret를 직접 확인했습니다. 결과는 `canvas-visual-result.json`입니다.
- [x] 최종 도구 strict TS exit0, 대상 TS/HTML Prettier 적용 exit0, 실제 Wasm normal `cargo tree --locked --offline --features canvas --target wasm32-unknown-unknown --edges normal` exit0입니다. 실제 GPU/DOM 실행 위험을 static만으로 완료 처리하지 않았습니다.
- [x] abort 상태 API 정정 후 production 포함 probe strict .43초/영향 build1.39초·binding 생성 exit0입니다. 새 `canvas-abort`만 첫 PASS(seq9/frame5/pump11·Open→Disposed/socket0·blur/quiet1.1초 snapshot/요청 불변·오류0)이고 `canvas-abort-result.json`이 증거입니다. 정상 close/입력/렌더 성공은 다시 실행하지 않았으며 그 이전 결과는 당시 source의 독립 증거로 유지합니다.

결과/이미지는 모두 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/`의 test-only 산출물입니다. 현재 bindings는 canvas feature 빌드이며 이전 close/pump/mirror 결과는 각각 당시 source의 독립 증거입니다. 같은 generated 파일 경로라는 이유로 예전 실행이 새 canvas를 검증했다고 쓰지 않습니다.

## 미완료

- 원본 전체 CanvasContents/ShellSurfaces·font/preview/terminal/settings/locale/keymap/각 consumer의 제품 브라우저 연결과 Rust public assets 빌드·패키징은 미완료입니다.
- 모든 close 진입점·server All/Window/Project handshake·응답 없는 deadline·format/auto-save/LSP/untitled/view-state flush·외부 writer·receipt 전 단절 복구는 기존 pending에 유지합니다.
- CJK IME/VoiceOver 사용자-last, 실제 GUI/성능/security/beta/install/rollback/cutover/Rust99/제품 TS 제거·N1~N8은 미완료입니다. 이 host/probe 성공으로 상위 gate를 체크하지 않습니다.

현재 canvas 경계 3/3(100%), 후속47 2/4(50%), M8 active363/433(83.83%, 공수비 아님), N1~N8 0/8입니다. 전체 ETA는 남은 제품 UI/패키징/최종 gate 공수가 미확정이라 산정 보류합니다. goal active·main 직접·전체완료 전 Git 없음·실행 중 handle 없음입니다.
