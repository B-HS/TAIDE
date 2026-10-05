# M8 native terminal 선택 수명

## 대상·상태

`native/taide-native-terminal/src/{lib,session,selection_text}.rs`, `tests/selection.rs`, vendor Alacritty의 `grid/{mod,resize}.rs`·`term/mod.rs`·`selection.rs`, `native/taide-native-app/src/terminal_surface.rs`입니다. 메인이 workflow·서브에이전트 없이 직접 진행했습니다. 기본 출력/trim·입력·buffer/RIS/rows resize·word 최소 길이·Select All·컬럼/half-open copy와 highlight 분리 코드 경계의 결과이며 전체 선택/N4/M8 완료가 아닙니다. 기본 anchor checkpoint는 완료했고 다음은 drag auto-scroll입니다. N1~N8 0/8과 전체 M8 완료 뒤만 commit/push를 유지합니다.

## 실제 source와 구현

1. 설치된 xterm SelectionService는 ordinary output마다 선택을 지우지 않습니다. onUserInput·buffer activate/reset·rowsChanged resize에서 정리합니다. SelectionModel.handleTrim은 원래 start/end row를 줄이고 원래 end가 trim 밖이면 지우며 start만 밖이면 row를 0으로 제한합니다. 실제 코드는 start column을 유지합니다. end가 없는 Word/Line seed는 start row만 제한하고 selectionStartLength를 유지합니다. 반대 방향 선택에서도 정렬된 최종 end가 아니라 원래 end를 사용합니다.
2. native Grid의 실제 top scroll은 history 증가/포화 양쪽에서 origin을 증가시킵니다. history purge/shrink와 column reflow의 실제 오래된 행 제거는 trimmed를 증가시킵니다. top이 0이 아닌 부분 scroll/reverse index는 절대 buffer 좌표를 바꾸지 않는 xterm 정책입니다. resize 내부 scroll은 최종 history 변화와 실제 trim으로 origin을 다시 계산해 일반 출력으로 이중 계산하지 않습니다. Term의 buffer/rows epoch는 같은 batch의 alt 진입→복귀·rows 변경→복원·RIS도 감지합니다. checked overflow는 실패 상태이며 Core admission 시 retire합니다. 단일 scalar stamp이고 새 grid/문자열 캐시/두 번째 parser는 없습니다.
3. 독립 view가 이전 stamp를 보존하고 같은 snapshot 안에서 paint·copy·pointer/Select All 전에 조정합니다. 바닥 offset 0은 바닥에 남고 scrollback offset은 실제 origin 변화만큼 조정한 뒤 남은 history로 제한합니다. 원래 Simple/Block anchor를 정렬해서 content tracking으로 바꾸지 않습니다. SharedTerminal의 실제 user Write 인코딩은 input epoch를 변경해 같은 Core를 보는 다른 view도 다음 snapshot에서 정리합니다. Focus 보고·로컬 Select All/Page 동작·거절된 입력은 input epoch를 변경하지 않습니다. 이 경계는 인코딩 성공 시점이며 OS write 완료 시점이나 모든 다른 창의 즉시 repaint 증거가 아닙니다.
4. Select All은 고정된 과거 범위를 이동하는 대신 현재 버퍼 전체 범위를 유지합니다. Word/Line은 원래 start·최소 셀 길이·optional half-open end를 독립 seed에 보존하고 xterm finalSelectionStart/End 정책을 따릅니다. 가로 reflow 뒤 seed 길이를 현재 range 폭으로 줄이지 않습니다. Shift down은 원본 incremental raw end이며 drag에서만 Word/Line 끝점을 확장합니다. LINE drag는 원본 pointer row의 시작/끝이고 별도의 wrapped target 범위 합집합이 아닙니다. 산술/행 변환은 checked, seed는 inline scalar이며 복사는 기존 64KiB/visit 예산을 유지합니다.
5. 기존 TS view는 scrollOnEraseInDisplay를 설정하지 않으며 설치된 xterm 기본값은 false입니다. native-retained 전용 CSI 2J는 visible rows만 지워 history를 늘리지 않습니다. CSI 3J는 history를 실제 purge합니다. feature 밖의 기존 upstream 2J 분기는 유지합니다. 전체 erase/VT 동등성 검사 완료를 뜻하지 않습니다.
6. 후속 끝점 대조에서 inclusive range만으로 copy하면 비wrapped 다음 줄 col 0에 포함된 trailing newline이 사라지는 실제 RED를 확인했습니다. read-only native_anchors의 원래 Point/Side에서 checked half-open Slice를 만들고 copy와 실제 highlight range를 별도로 유도합니다. 현재 폭 밖의 start를 마지막 실제 셀로 붙이지 않고 해당 첫 줄의 빈 문자열·줄바꿈을 보존합니다. Word seed도 원래 column을 저장합니다. 실제 grid 밖의 word 최소 길이 tail은 원본 Buffer.translateBufferLineToString처럼 빈 줄이며 방문/byte 예산을 그대로 적용합니다. highlight는 실제 셀에만 제한합니다. 원본 Selection::to_range를 전체 fork에서 바꾸거나 새 parser/문자열 cache를 만들지 않습니다.

## 검증 환경

기존 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, Cargo 직렬 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 합성 Core·같은 SharedTerminal·headless egui만 사용했습니다. 보호 실기 bundle·사용자 파일/clipboard·OS 입력기/VoiceOver/settings는 조작하지 않았습니다. dependency/root/MSRV/TS 제거·Git 변경은 없습니다.

## 신규 runtime 결과

| 명령의 target/filter | 실제 결과 | 덮는 경계 |
| --- | --- | --- |
| app `--lib selection_lifetime -- --nocapture` | 실제 RED `secon`≠`first` 뒤 1 PASS, compile 4.78초/suite 0.03초 | history 증가/포화·완전 trim·독립 scrollback 위치 |
| native `--test selection selection_motion -- --nocapture` | fixture 끝점 교정 뒤 1 PASS, compile 0.36초/suite 0.03초 | 일부 trim/원래 역방향 end·2J/3J·부분 scroll·alt/rows ABA/RIS·resize scalar 경계 |
| app `--lib selection_owner -- --nocapture` | 1 PASS, compile 2.56초/suite 0.00초 | Focus Write/local/reject 유지·user Write 후 두 독립 view 선택/offset 정리 |
| app `--lib selection_all -- --nocapture` | 1 PASS, compile 7.36초/suite 0.01초 | 새 출력/trim/3J의 현재 전체 선택·RIS 정리 |
| native `--test selection selection_columns -- --nocapture` | 1 PASS, compile 1.14초/suite 0.00초 | 실제 reflow/head trim 뒤 원래 절대 좌표에 해당하는 `ghijkl` 복사 |
| app `--lib selection_seed -- --nocapture` | 실제 RED `ghijkl`≠`ghijklmnopqrstuvwx\nline2` 뒤 1 PASS, compile 3.92초/suite 0.00초 | 원래 24셀 word seed의 가로 resize/trim 뒤 최소 길이 |
| app `--lib selection_model -- --nocapture` | 1 PASS, compile 1.85초/suite 0.00초 | 원래 정/역방향·같은 줄 최소 끝점·wrapped 길이·end 유무에 따른 trim 정리 |
| native `--test selection selection_slice -- --nocapture` | 실제 RED `cdef`≠`cdef\n` 뒤 1 PASS, compile 1.19초/suite 0.00초 | trailing newline·폭 밖 원래 열의 첫 줄 빈 문자열/줄바꿈 |
| native `--test selection selection_render -- --nocapture` | 1 PASS, compile 0.76초/suite 0.00초 | 폭 밖 시작 열의 실제 highlight·column 빈 행·실제 grid 밖 tail·copy byte 거절 |

고유 신규 9건입니다. Word/Line 모델 변경의 영향 pointer 검사는 그 시점에 한 번 1 PASS(compile 1.61초/suite 0.03초)입니다. 후속 half-open copy/range 구현이 실제로 바뀐 뒤에는 app `--lib selection_ -- --nocapture`에서 영향 7건이 한 번 PASS(compile 2.91초/suite 0.03초)했고 native `--lib selection_text -- --nocapture`의 byte/visit·공백/erase/Unicode 영향 1건도 한 번 PASS(compile 0.69초/suite 0.00초)했습니다. 같은 production 상태에서 통합만을 위한 반복이 아닙니다. Shift down의 과거 fixture는 wrapped 전체 target까지 기대했지만 원본 incremental endpoint는 클릭한 행의 col 0이므로 앞 12셀까지만 기대하도록 수정했습니다. 원본 `_handleIncrementalClick/_handleMouseMove` 근거와 구현을 함께 변경했고 검사기를 끄지 않았습니다.

최초 컴파일은 Grid의 generic `clear_history`에서 constrained impl의 native_record_trim을 호출해 E0599가 발생했습니다. scalar helper를 기존 `impl<T> Grid<T>`로 옮겼습니다. selection_motion의 첫 기대 문자열 실패는 fixture end를 Line(0)으로 만들어 놓고 다음 Line(1)의 `fou`까지 기대한 오류입니다. 실제 보고값은 `cond\nthi`였고 endpoint fixture를 Line(1)으로 수정한 뒤 해당 검사만 한 번 실행했습니다. 이를 production trim 오류 RED로 세지 않습니다. CSI 2J 자체는 수정 뒤 검사이며 수정 전 runtime RED를 주장하지 않습니다.

## 최소 정적 검사

- [x] native `cargo clippy … --lib --test selection -- -D warnings`: half-open 변경 뒤 최종 exit 0(0.69초). 이전 0.28초 성공은 그 시점의 근거로만 보존합니다.
- [x] app `cargo clippy … --lib -- -D warnings`: half-open 변경 뒤 최종 exit 0(1.41초). 이전 1.81초 성공은 최종 변경의 검사로 대신하지 않습니다. library scope이며 전체 integration targets의 재검사는 아닙니다.
- [x] 저작한 5개 Rust 파일의 exact `rustfmt --check --edition 2024`와 tracked `git diff --check` exit 0. vendor 전체 포맷은 하지 않았습니다. 기존 Wry dependency 경고 17개는 그대로이며 새 라이브러리 strict 검사 통과와 구분합니다.
- [x] 기본 copy/visit 검사는 위 실제 변경 영향만 검사했고 실제 PTY host/font/cursor/live/Hub/query의 같은 성공은 재사용했습니다. 현재 변경이 없는 성공 검사를 통합을 이유로 반복하지 않았습니다.

## 남은 선택·제품 gate

- [x] 폭 밖 start와 half-open newline의 기본 copy/highlight 분리 — 실제 RED와 신규 slice/render·변경 copy 영향 결과는 위에 기록했습니다. 모든 block/Unicode/VT 끝점 조합의 완전 검증이라고 주장하지 않습니다.
- [ ] drag auto-scroll·wheel 중 선택·링크 우선 double click/context/search/link·hidden/다중 window controlling view와 즉시 repaint·프로젝트 Hub 수명·mouse/keypad/Kitty/전체 VT/TUI·특수 UTF-16 Unicode/erase 조합은 미완료입니다.
- [ ] OS clipboard·실제 창/GPU/IME/AX와 aggregate/peak/RSS/CPU·213 TS view/full product cutover·TS 제거·배포는 미완료입니다. 기본 상한을 전체 xterm/제품 동등성으로 계산하지 않습니다.

현재 Cargo live handle은 없습니다. 다음은 원본 50ms drag scroll·최대 15행/50px 정책과 같은 buffer row/column 끝점을 독립 view의 repaint timer/scrollback에 연결하는 것입니다.
