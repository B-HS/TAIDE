# M8 원격 layout 실제 backend

## 대상과 구현

대상은 `native/taide-native-app/src/remote-layout.rs`, `remote-layout-tests.rs`, `lib.rs`입니다. 원본 layout19명령을 기존 runtime에 연결했습니다. 후속16 project/session23까지 합친 실제 domain adapter는88/177이며 남은89명령과 전체 앱 assembly는 미완료입니다. N1~N8은0/8·목표 active입니다.

- `extend_backend(Ports, remaining)`은 layout19명령만 처리하며 나머지 JSON/channel/raw를 필수 remaining에 전달합니다. 바깥 `with_policy`의 default-deny·owner·모드 제한을 유지합니다.
- 모든 인자·반환·dirty layout·revision·이벤트는 기존 `layout_actions`를 사용합니다. native UI의 추가 pinned/dirty 확인을 원본 remote 명령에 임의 추가하지 않았습니다.
- 닫기는 원본 close/commit 뒤 IDE reconcile→core terminal kill을 유지하고 필수 native registry 폐기를 연결합니다. `Ports::new(Arc<terminal_host::Hub>)`는 실제 Hub discard를 사용하며 제품 no-op/default는 없습니다.
- split/move/resize/focus·preview/view-state·pin·untitled/convert·reopen/path-change/shell-view를 같은 root service로 처리합니다. 새 package/dependency/lock/MSRV·root/Tauri/제품TS·보호 bundle·OS 설정 변경은 없습니다.

## 검증

공통 Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, manifest `native/taide-native-app/Cargo.toml`, `--locked --offline`, target `experiments/native-shell-spike/target`입니다. 사용자 파일/셸/앱은 사용하지 않았습니다.

- [x] `cargo test --lib remote_layout::tests:: -- --nocapture`: compile6.07초/suite0.01초, catalog/input/path/remaining1 PASS·fixture 기대값2 FAIL입니다. 원본 기본 layout은 Welcome+빈 Terminal이고 Welcome은 dedup합니다. 기본 layout이나 제품 dedup을 바꾸지 않고 fixture의 빈 layout 가정과 split용 탭을 정정했습니다. 테스트 이름의 대문자 IDE도 소문자로 정정했습니다.
- [x] 실패2개만 `--skip catalog_실제_arm과_원본_입력_파일_gate_remaining을_보존한다`로 재실행했습니다. compile3.15초/suite0.01초,2 PASS이며 성공1검사는 제외했습니다. 실제19명령·revision·이벤트·dirty·close/reopen의 id/view-state·IDE pending TabClosed·terminal callback 순서·atomic split·path no-op/외부 경로 거절을 확인했습니다.
- [x] 독립 실제 PTY 검사 `실제_native_hub_포트의_원격_terminal_닫기는_pty와_actor를_회수한다`를 추가했습니다. 첫 컴파일은 fixture가 TerminalSession의 존재하지 않는 `session_id` 필드를 사용해 실패했습니다. 실제 `id`로 정정한 뒤 이 신규 검사만 compile6.52초/suite0.02초,1 PASS입니다. 고유4검사가 모두 PASS이며 이전3개는 반복하지 않았습니다.
- [x] PTY 검사는 새 합성 프로젝트의 `/bin/cat`만 실행했습니다. actual Hub/TerminalStore와 `Ports::new`를 거쳐 원격 close 뒤 registry entry·core session 제거, 실제 completion/idle·supervisor tracked0을 확인했습니다. 사용자 shell/startup 설정과 실기 앱은 실행하지 않았습니다.
- [x] 후속16과 공유 `cargo clippy --lib --bin taide-native-app --tests -- -D warnings` exit0,12.93초입니다. authored5파일의 exact rustfmt check exit0입니다. 기존 Wry dependency17경고는 authored 검사와 별도이며 억제를 추가하지 않았습니다.

## 미완료

- [ ] 생산용 terminal/project/remote 전체 dispatcher와 앱 startup/Exit·Settings·assets/events를 조립합니다. 합성 callback을 제품 조립 완료로 세지 않습니다.
- [ ] 실제 UI/AX/pixel·성능/메모리·동시 mutation/aux window·원본 PTY remount 결정과 keybinding RED를 검증합니다. 전체 suite green·M8 완료를 주장하지 않으며 전체 완료 뒤만 commit/push합니다.

## 근거

원본 `src-tauri/src/remote_gateway.rs`의 layout19 arm, layout commands와 `lib.rs` close observer, root `layout_actions`, native Hub/TerminalStore를 대조했습니다. root 이벤트 publish 시점과 close callback 시점을 구분하고 callback은 commit 뒤 상태만 관찰했습니다.
