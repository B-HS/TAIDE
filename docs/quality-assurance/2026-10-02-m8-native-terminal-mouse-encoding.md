# M8 native terminal live mouse encoding checkpoint

> 2026-10-02 후속: 기본 raw mouse adapter·X10 wheel fallback·logic replay·release queue와 actual Views→PTY 연결을 `2026-10-02-m8-native-terminal-mouse-adapter.md`에 기록했습니다. 아래 내용은 encoder 완료 시점의 증거이며, 후속 UI 구현 여부는 adapter QA가 정본입니다. 전체 mouse/N4/M8는 미완료입니다.

## 대상과 완료 범위

대상은 `native/taide-native-terminal/src/{input,lib}.rs`, `tests/input.rs`, `tests/fixtures/session.rs`, `native/taide-native-app/tests/terminal-host.rs`, native Alacritty fork의 `src/term/mod.rs`·`UPSTREAM.md`입니다.

이번 완료는 actual Core의 live mode→typed mouse encoding→기존 bounded writer→합성 child PTY 입력 경계입니다. egui pointer adapter·tracking 소유권/선택·GUI mouse 동작은 아직 연결하지 않았습니다. 앞선 raw-wheel owner checkpoint는 재사용하며 N4/M8 전체 완료가 아닙니다.

## 근거와 실제 실패

공식 [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.pdf)와 설치된 xterm 6.0.0의 `src/common/services/CoreMouseService.ts`, `src/common/InputHandler.ts`, `src/browser/{CoreBrowserTerminal.ts,services/MouseService.ts,services/SelectionService.ts}`를 대조했습니다. 제품은 현재 설치된 xterm의 동작을 재현하는 것이 기준입니다.

- [x] actual Core에서 DECSET 9 후 left press가 `Write([27,91,77,32,33,33])` 대신 `Ignore`인 RED를 확인했습니다. compiler/test suppression이나 기대값 변경 없이 native 전용 X10 mode 처리를 추가했습니다.
- [x] source 대조에서 기존 Alacritty의 특정 tracking bit만 해제하는 동작, 1005가 SGR를 끄는 동작과 설치된 xterm의 차이를 확인했습니다. xterm은 tracking 9/1000/1002/1003 중 어떤 reset도 protocol을 NONE으로 만들고, 1005/1015 set/reset은 encoding을 변경하지 않습니다. 후속 actual Core 연속 검사는 이를 포함합니다. 첫 RED assertion 이후 실행되지 않은 assertion을 별도 관찰 RED로 합산하지 않습니다.
- [x] xterm의 1016 pixel encoding도 연결했습니다. 설치된 `MouseService`는 local pixel을 floor한 0-based x/y로 반환하며 SGR_PIXELS는 col/row와 달리 +1하지 않습니다. 원본의 이 동작을 재현하고 표준의 모든 emulator와 같은 pixel convention이라고 주장하지 않습니다.

## 구현 계약

- [x] `MouseButton`은 left/middle/right, `MouseAction`은 press/release·선택적 button의 move·vertical wheel up/down으로 유효 조합을 제한합니다. mouse modifier는 Shift/Alt/Ctrl만 보고하고 Command는 mouse code에 포함하지 않습니다. motion debounce·physical held state는 향후 view adapter의 책임입니다.
- [x] Core는 실제 grid의 columns/rows로 0-based cell 좌표를 재검사하고 밖의 좌표는 Ignore합니다. pure encoder는 grid를 보유하지 않습니다. pixel 입력은 typed optional u32 pair이며 pixel mode에서 없으면 InvalidCoordinates로 거절합니다. 실제 renderer canvas 범위 검증은 아직 연결하지 않았습니다.
- [x] default encoding은 실제 binary Vec로 CSI M·code+32·1-based cell+32를 생성합니다. 223을 넘는 1-based 좌표는 원본처럼 Ignore하며 UTF-8 String으로 바꾸지 않습니다. SGR/SGR_PIXELS는 press/move/wheel의 M과 실제 release의 m을 구분하고 caller byte capacity를 확인합니다. 출력 format의 숫자는 bounded 정수입니다.
- [x] X10은 press만 보고하고 modifier를 제거합니다. VT200은 motion을 무시하고, Drag는 button 없는 move를 무시하며 Any는 이를 보고합니다. mouse-mode NONE은 전송하지 않습니다.
- [x] native-retained 전용 TermMode의 남은 u32 bit에 X10·SGR pixel 상태를 추가했습니다. 9/1016은 기존 vte의 Unknown private-mode typed 경계에서 처리하며 별도 parser/복제 상태·vte 변경·새 dependency를 추가하지 않았습니다. 1006/1016 encoding set/reset은 상호 배타적이며 native 1005/1015 query는 permanently reset(4)을 반환합니다.
- [x] 기존 pure encoder source를 사용하는 registry Alacritty spike도 compile됩니다. 그 legacy spike의 API는 native-only X10/pixel mode 상태를 지원한다고 주장하지 않습니다. native Core는 같은 live TermMode에서 확장 MouseMode를 도출하며 renderer의 복제 mode를 신뢰하지 않습니다.
- [x] 합성 fixture에 opt-in `TAIDE_NATIVE_FIXTURE_MOUSE`를 추가했습니다. child 자신의 PTY에만 noncanonical·8-bit 보존을 설정하고 독립 literal byte와 비교합니다. default encoding 수신 후 child가 live SGR mode와 별도 ready title을 출력합니다. wheel opt-in과 동시에 켜면 거절하며 opt-in 없는 canonical fixture 경로는 유지합니다. 사용자 shell/profile·OS 설정을 바꾸지 않습니다.

## 최소 검사와 실제 결과

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`이며 직렬 실행했습니다.

1. [x] 첫 typed basic encoder 추가 뒤 `--test input`: 신규 encoder와 기존 keyboard/paste/focus/retire 2 PASS, compile 1.16초·suite 0.00초입니다. 초기 native strict exit 0(0.55초)였습니다.
2. [x] X10 actual RED는 `--test input mouse_protocol`, compile 0.45초·suite 0.00초·exit 101입니다. mode/pixel source를 실제 수정한 후 최종 `cargo test --manifest-path native/taide-native-terminal/Cargo.toml … --test input -- --nocapture`: 3 PASS, compile 1.38초·suite 0.00초입니다. 고유 신규 encoder/protocol 2건과 변경 영향 keyboard 1건입니다. 버전별 실행을 고유 검사 수에 중복 합산하지 않습니다.
3. [x] 최종 native `cargo clippy … --lib --test input -- -D warnings`: exit 0(0.83초)입니다.
4. [x] 공유 source의 실제 소비자 `cargo check --manifest-path experiments/terminal-core-spike/Cargo.toml … --lib --test input-contract --test pty-input`: exit 0(1.40초)입니다. 기존 같은 keyboard runtime/PTY 성공 body는 반복하지 않았습니다.
5. [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml … --test terminal-host mouse_writer -- --nocapture`: 신규 actual PTY 1 PASS, compile 9.75초·suite 0.34초입니다. default의 마지막 1-byte column인 222의 press/release에서 child가 `0xff` 좌표를 그대로 받았습니다. 같은 session의 live SGR 전환·modifier press/release·wheel literal 수신·exit 0·Dispatcher/child/reader/flusher/writer join과 TaskSupervisor 0을 확인했습니다. native GUI event 입력은 이 검사에 포함되지 않습니다.

추가 static은 최종 app lib/host test/fixture의 `cargo clippy … --lib --test terminal-host --bin native-terminal-queue-fixture -- -D warnings`: exit 0(2.93초)입니다. Wry 의존의 기존 17 warnings는 authored strict와 구분합니다. authored 5파일의 `rustfmt --edition 2024 --config skip_children=true --check`와 추적 `git diff --check`는 exit 0이며 같은 명시된 untracked authored 파일의 trailing whitespace는 일치 없음입니다. vendor 전체 rustfmt는 하지 않았습니다.

dependency package를 `-p alacritty_terminal --no-default-features`로 선택한 별도 off-feature check 시도는 Cargo가 “cannot specify features for packages outside of workspace”로 exit 101을 반환했습니다. compile/통과로 계산하지 않습니다. 별도 vendor off-feature compile gate는 남으며, source의 비-native 분기는 조건부로 보존했습니다. 새 lock/manifest/harness로 검사를 우회하지 않았습니다.

## 남은 기존 N4-B 연결

- [ ] egui raw pointer와 mouse wheel의 event-time 좌표/순서·viewport/pane/tab 소유권·bounded queue/receipt·active capture·release·move debounce·mode/epoch 전환·disabled/hidden/close/focus 정리를 연결합니다. 현재 terminal surface는 mouse mode에서 pointer를 보고하지 않고 일반 wheel도 억제하는 미완료 경로입니다.
- [ ] X10은 wheel을 보고하지 않으므로 원본처럼 normal viewport 또는 alt 방향키 fallback 경로로 가야 합니다. 기존 surface의 blanket MOUSE_MODE wheel guard를 adapter 연결 단계에서 분리해야 합니다. 이번 core encoder 성공으로 이 UI 분기가 해결됐다고 계산하지 않습니다.
- [ ] forced selection은 source 플랫폼 정책을 재현합니다. 현재 TS는 macOptionClickForcesSelection을 설정하지 않으며 설치 기본값은 false이고, macOS 밖은 Shift입니다. mouse 모드의 keyboard Select All/Copy와 primary forced-selection drag·right-click/context는 제품 UI 연결 뒤 검사합니다.
- [ ] 실제 TUI/OS/DPR·픽셀 clamp·auxiliary window/IME/AX·large event/aggregate·full VT·keypad/Kitty·213 TS view·제품 cutover/TS 제거·M8 N1~N8 전체는 미완료입니다.

보호 bundle·TS/root manifest/lock·MSRV·사용자 데이터/clipboard·OS 입력기/VoiceOver는 이 mouse checkpoint에서 변경하거나 실행하지 않았습니다. 새 dependency·unsafe·suppression·코드 주석을 추가하지 않았습니다. Rust fn/enum/mutation은 기존 native Rust 예외를 따릅니다. 전체 M8 완료 전 commit/push하지 않았으며 현재 Cargo live handle은 없습니다.
