# M8 native terminal 선택·복사

## 대상·현재 경계

`native/taide-native-terminal/src/{lib,selection_text}.rs`, native Alacritty `term/{cell,mod}.rs`, `native/taide-native-app/src/terminal_surface.rs`, `tests/terminal-host.rs`입니다. 메인이 workflow·서브에이전트 없이 직접 구현했습니다. 이번 범위는 기본 Simple/Block 선택·복사입니다. 단어/줄·anchor 수명·전체 N4/M8, N1~N8 0/8은 유지합니다. M8 전체 완료 전 commit/push하지 않습니다.

## 실제 코드 계약

- 설치된 xterm `browser/services/SelectionService.ts`의 selectionText, `common/buffer/BufferLine.ts`의 getTrimmedLength/translateToString, `browser/input/Mouse.ts`의 셀 반쪽 끝점을 확인했습니다. 원본 TS view의 Alt column selection과 wide trailing spacer 경계도 같은 설치 source로 대조했습니다. [egui copy_text](https://docs.rs/egui/0.36.2/egui/struct.Context.html#method.copy_text)와 설치된 Selection API를 확인했습니다. Alacritty의 해당 docs.rs 페이지 조회는 접근 오류였고 실제 vendor source를 사용했습니다.
- 실제 출력된 공백은 NATIVE_CONTENT bit로 미출력 blank와 구분합니다. Cell layout·별도 allocator/parser는 추가하지 않습니다. native feature 밖의 기존 Cell 동작은 유지합니다. 실제 write·wide cleanup·reset·reflow occupied length와 연결합니다. 이 feature가 켜진 native 경로의 공백 reflow 동작은 의도적으로 달라집니다.
- actual immutable Core/grid의 SelectionRange를 빌려 추출합니다. Simple/Block만 허용하며 upstream Semantic/Lines의 무상한 검색을 임의 호출하지 않습니다. 일반 선택은 WRAPLINE에서 연결하고 column 선택은 각 줄을 분리합니다. 실제 공백과 결합 문자는 보존하고 TAB/NBSP는 공백으로 처리합니다. 추출 UTF-8 byte cap·실제 String capacity·cell/zero-width 방문 cap을 검사합니다. 한도를 넘으면 오류로 끝나며 Core를 폐기하거나 부분 문자열로 성공하지 않습니다.
- view는 독립 Selection을 소유하며 Core의 global selection·display_offset을 변경하지 않습니다. half-cell·끝 exclusive 경계·wide spacer 보정과 release endpoint를 반영하고 Alt는 Block입니다. 선택 배경과 wide 두 셀을 함께 그립니다. Copy는 실제 Session.snapshot 안의 동일 Core를 사용하고 성공은 egui platform CopyText 출력, 오류는 view.error로 표시합니다. 복사 bytes는 기존 64KiB입니다. 이것은 xterm의 무제한 copy 전체 동등성은 아닙니다.

## 실제 검사

Cargo는 기존 CARGO_HOME, 직렬 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 합성 grid/font와 실제 자기 fixture PTY/headless 출력만 사용했습니다. 보호 실기 앱·bundle·OS clipboard/설정·사용자 파일은 조작하지 않았습니다.

1. [x] native `cargo test --manifest-path native/taide-native-terminal/Cargo.toml … --lib selection_text -- --nocapture`: 신규 1 PASS(compile 1.94초/suite 0.00초). 실제 출력 공백·wrap·reverse/column·resize reflow·erase·wide overwrite·NFD/NBSP·정확한 UTF-8 byte cap·visit cap·empty/unsupported 선택·retained admission입니다.
2. [x] app `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib selection_copy -- --nocapture`: 실제 RED `abc         `≠`abc   ` 뒤 1 PASS(compile 4.22초/suite 0.00초). 출력/blank/TAB/NBSP와 half-cell·wide boundary·right edge·column 선택·byte 거절입니다.
3. [x] app `cargo test … --lib borrowed_grid -- --nocapture`: 변경 영향 1 PASS(compile 0.26초/suite 0.01초). 독립 선택 배경·wide/NFD·palette와 Core display_offset 유지입니다. 실제 OS 픽셀 검사와 여러 view lifecycle 전체 증거는 아닙니다.
4. [x] app `cargo test … --test terminal-host headless_surface -- --nocapture`: 신규 CopyText assertion을 포함한 실제 fixture PTY/headless 1 PASS(compile 2.52초/suite 0.16초). Cmd+A→Copy의 Unicode 문자열·미출력 padding 제외를 egui 출력에서 확인했습니다. 초기 `starts_with` 가정 실패(0.30초)는 pending 입력을 먼저 쓰는 기존 fixture에 맞게 포함 조건으로 수정했습니다. 기존 attach 실패/retry/IME/restart/join을 같은 단일 연속 검사에서 유지했습니다. OS clipboard를 쓰지 않았습니다.
5. [x] native `clippy … --lib -- -D warnings`: exit 0(1.46초). app `clippy … --lib --test terminal-host -- -D warnings`: exit 0(2.08초). 선택 배경의 nested if 한 건을 같은 조건의 let-chain으로 정리한 뒤 영향 정적 검사만 재실행했습니다. 이 조건 구조만의 변경에 성공 runtime을 반복하지 않습니다. authored exact rustfmt·tracked `git diff --check` exit 0이며 Wry dependency의 기존 경고 17개는 유지합니다.

## 미완료 gate

이후 기본 Word/Line·pointer down/drag/Shift 연결과 실제 결과는 `docs/quality-assurance/2026-10-02-m8-native-terminal-word-line.md`가 최신 정본입니다. 아래는 이 기본 copy checkpoint 당시의 잔여이며 후속 문서의 완료 범위만 반영합니다.

복사 오류의 실제 UI repaint/OS clipboard, 마우스 drag 모든 event 순서·Shift 확장·단어/줄·trim/reflow/출력 scroll과 선택 anchor 수명·auto-scroll·scrollback 끝점·wrap erase 동등성은 남습니다. WRAPLINE은 현재 Alacritty의 이전 줄 flag이며 xterm의 다음 줄 isWrapped와 모든 VT erase/scroll 조합에서 같다고 주장하지 않습니다. Windows newline fixture·전체 upstream corpus와 다양한 앱/TUI도 실행하지 않았습니다. 64KiB copy/방문 상한의 제품 정책·aggregate/RSS/CPU·검색/link/context·hidden focus/다중 view·mouse/keypad/Kitty·IME/AX·project Hub·전체 폰트 fidelity/worker와 213 TS view/full cutover는 아직 미완료입니다.
