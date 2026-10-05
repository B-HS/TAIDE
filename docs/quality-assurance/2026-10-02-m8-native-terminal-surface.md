# M8 native terminal 기본 surface 연결

## 대상·상태

`native/taide-native-app/src/{terminal_surface,application,presentation,terminal_dispatch,terminal_writer}.rs`, `tests/{terminal-host,terminal-dispatch}.rs`, `native/taide-native-terminal/src/lib.rs` 및 Alacritty vendor의 `event.rs`, `term/mod.rs`입니다. terminal placeholder를 측정된 attach/resize caller와 같은 Session/Core의 borrowed grid 렌더·기본 입력에 연결했습니다. 기본 연결만 검증했으며 N4-B/N4/M8 전체 완료가 아닙니다. N1~N8은 0/8이며 전체 완료 뒤만 commit/push합니다.

## 연결한 경계

- 화면의 실제 font glyph width/row height와 clip rect로 크기를 계산합니다. 3열·2행 미만, 비유한·u16 범위 밖 크기는 attach/resize하지 않습니다. 빈/복원 session ID는 기존 Tabs의 measured attach를 요청하고, pending tab은 중복 요청을 보내지 않습니다. reply·tab 제거·submission 실패에서 pending을 정리하며 late closed-tab reply가 geometry를 다시 생성하지 않습니다. 실제 Session이 있으면 새 Core나 raw-output replay를 만들지 않습니다.
- focused view만 resize를 요청합니다. 같은 Session에서 한 resize reply를 기다리는 동안 추가 resize는 보류하며, 다음 화면에서 최신 크기와 Core를 다시 비교합니다. query geometry는 첫 attach의 Arc/Mutex port를 유지하며 활성 view의 값으로 갱신합니다. 여러 창의 단일 controlling-view 정책·실제 child 내부 geometry 확인은 아직 미완료입니다.
- terminal 전용 theme의 16 ANSI·기본색·cursor·selection, 6단계 색상 cube·grayscale·dim과 Core 동적 색상을 읽습니다. background를 먼저 그린 뒤 glyph를 그려 wide glyph가 뒤 셀 배경에 덮이지 않게 합니다. wide spacer/hidden·결합 문자·inverse·기본 underline/strike·cursor를 투영합니다. per-pane/tab scroll/selection은 Core의 display offset을 변경하지 않습니다.
- 실제 Core input encoder와 기존 bounded writer에 기본 navigation/F1~F12/Ctrl·Enter/Shift+Enter·text/paste·focus·IME preedit/commit을 연결합니다. preedit는 view에만 보관하고 확정만 전송합니다. writer receipt를 view별 최대 64개 유지하고 try_wait로 완료/실패를 관찰하며 pending 때만 repaint를 예약합니다. preedit/input/선택 text는 64KiB 경계를 적용하고 closed view는 receipt 소유를 해제합니다. started IO의 소유는 기존 root supervisor에 유지됩니다. drag 선택·Shift PageUp/Down·SelectAll과 egui Copy 경로를 연결했으나 clipboard OS 실기는 하지 않았습니다.
- 동적 색상 query는 parser가 query를 생성하는 바로 그 시점의 override를 typed ColorQuery에 담습니다. 이후 같은 batch의 reset이나 다음 output 때문에 앞선 query가 다른 색을 읽지 않습니다. Dispatcher는 override가 있으면 그대로 쓰고, 없으면 UI palette port를 사용합니다. Core mutex를 writer await에 유지하거나 매 query에서 palette 사본을 생성하지 않습니다. inline 필드와 실제 payload는 기존 typed retained graph에 포함됩니다.

Rust `fn`/enum·필요한 mutation과 egui/protocol 수치 변환은 기존 native Rust 예외 범위를 유지합니다. 새 dependency/package/lock/MSRV·기존 TS/Tauri 실행 경로·보호 실기 bundle과 OS 설정을 변경하지 않았습니다. API는 고정 egui 0.36.2의 installed source와 [공식 Painter 문서](https://docs.rs/egui/0.36.2/egui/struct.Painter.html), 기존 NativeEditor의 focus/IME 경계를 확인했습니다.

## 실제 검증

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`로 직렬 실행했습니다. 보호 `.app` bundle은 빌드·교체·조작하지 않았습니다.

1. [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib terminal_surface::tests -- --nocapture`: 1 PASS(compile 4.86초/suite 0.02초). palette 경계·잘못된 measured 크기 거절, 실제 Core의 동적 ANSI/wide/NFD와 선택 text·headless paint의 galley text/color, 독립 view scroll에서 Core display offset 0을 확인했습니다. galley text가 실제 OS 픽셀의 CJK glyph 품질을 증명하는 것은 아닙니다.
2. [x] `cargo test … --test terminal-host headless_surface -- --nocapture`: 1 PASS(compile 8.86초/suite 0.37초). 두 UI frame에서 attach command 1개만 생성, 실제 Tabs/Hub/PTY spawn, Core의 ready glyph를 headless 화면에 투영, preedit→commit+Enter가 실제 child의 정확한 continue 명령으로 이어짐·Exited(0)·close/join/tracked 0을 확인했습니다. UI event는 합성 입력이며 OS IME 실기가 아닙니다. 기본 화면 연결을 원본 UI full parity로 주장하지 않습니다.
3. [x] `cargo test … --test terminal-dispatch color_query -- --nocapture`: 1 PASS(compile 1.10초/suite 0.00초). 실제 parser의 set→query→reset→query 단일 batch에서 Core의 최종 override가 None이어도 첫 writer reply는 `aaaa/bbbb/cccc`, 두 번째는 palette `0101/0202/0303`임을 확인했습니다. 실제 Dispatcher/Frame/writer receipt를 사용하며 sink만 합성입니다. 첫 compile은 잘못된 `TaskSupervisor::drain` 이름으로 실패했고 실제 `shutdown`으로 바로잡은 뒤 해당 검사만 실행했습니다.
4. [x] app `cargo clippy … --lib --test terminal-host --test terminal-dispatch -- -D warnings`: exit 0(2.51초). native terminal `cargo clippy … --lib -- -D warnings`: exit 0(0.57초). 최초 app lib strict의 nested Copy 조건 collapsible_if 1건은 검사기 억제 없이 같은 short-circuit 조건으로 수정했습니다. app dependency Wry의 기존 17 warnings는 유지됩니다. 최초 app check exit 0(2.77초), authored 파일 exact rustfmt와 tracked diff check exit 0입니다. 이전 live sync/resize/publication·tab/host/env·writer/queue/Core 성공은 재사용하고 같은 성공 runtime을 반복하지 않았습니다.

## 다음 구현·미완료 gate

- [x] synchronized update의 기본 unrelated repaint 화면 보존은 후속 실제 검사에서 수정 전부터 1 PASS(0.01초)였습니다. vte는 sync 중 bytes를 보관하고 flush 때 Handler/grid를 바꾸므로 위 mutable grid 추정을 정정합니다. 별도 cache를 추가하지 않았으며 startup QA의 source·검사 경계가 정본입니다. actor deadline 성공을 이 화면 검사로 대체하거나 실제 픽셀 gate까지 완료 주장하지 않습니다.
- [x] 시작 전 입력 버퍼·attach 실패 retry·종료/실패 기본 표시와 restart를 후속 startup QA에 구현/검증했습니다. UTF-16 pending 1 PASS(0.00초)·확장된 실제 headless/PTy 연속 1 PASS(0.40초)입니다. hidden view focus/view identity/window control·project Hub·agent/remote/IDE/CLI 및 일반 bell/reset-title 등은 아직 미완료입니다.
- [ ] 설정 font family·CJK shaping·default cursor style/blink와 runtime 설정 변경, bold/italic font·double/dotted/dashed/undercurl, 정확한 선택/wrap/trim·종료 후 copy, search/link/context menu·마우스/keypad/Kitty·IME 주변 문자 삭제·접근성·실제 다중 창을 구현/검증합니다.
- [ ] per-view projection/galleys/input receipt의 전체 retained·합산/peak/RSS·장기 history 및 paint CPU·실제 GPU/OS pixel/IME/AX/성능/제품 채택·TS 제거·패키징 gate는 남습니다. logical quota를 process 전체 메모리 상한으로 주장하지 않습니다. 실기 앱은 유지하고 사용자 담당 시스템 설정 검사는 마지막 순서로 둡니다.
