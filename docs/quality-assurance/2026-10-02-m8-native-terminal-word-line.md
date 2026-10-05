# M8 native terminal 단어·줄 선택

## 대상·현재 경계

`native/taide-native-terminal/src/{lib,selection_bounds}.rs`, `tests/selection.rs`, `native/taide-native-app/src/terminal_surface.rs`입니다. 기본 선택·복사는 terminal-selection QA에 기록했고, 이번에는 기존 xterm의 기본 separator/whitespace·wrapped word/line과 pointer down/drag/Shift를 연결했습니다. 메인이 workflow·서브에이전트 없이 직접 수행했습니다. 상위 N4/M8·N1~N8 0/8은 유지하며 전체 M8 완료 뒤만 commit/push합니다.

## 원본·실제 계약

- 설치된 xterm `SelectionService._getWordAt/_selectWordAt/_selectToWordAt/_selectLineAt`, `Buffer.getWrappedRangeForLine`, `OptionsService`의 기본 wordSeparator와 기존 TS view의 옵션을 확인했습니다. ASCII 공백 선택은 연속 공백을 확장하고, 그 밖의 시작 셀은 이웃 separator 전까지 확장합니다. separator 자체를 클릭하면 그 셀과 양쪽의 연결된 이웃을 포함하는 원본 동작도 보존합니다. 이를 일반적인 다른 편집기의 단어 규칙으로 바꾸지 않았습니다.
- actual immutable Core/grid의 셀에서 Word/Line 범위를 계산합니다. wide spacer는 primary로 보정하고 NFD combined cell은 분리하지 않습니다. 연결된 WRAPLINE의 단어·줄은 위/아래 실제 history 경계까지 반복 탐색합니다. 재귀·전체 줄 문자열 복제·두 번째 parser·upstream Semantic bracket matcher는 추가하지 않았습니다. grid 밖 point와 실제 방문 상한 초과는 오류입니다. 결과는 두 Point의 SelectionRange이며 Core global selection/display_offset은 그대로입니다.
- egui pointer click count는 release에서만 확정하므로 완료된 두 클릭의 시각/위치를 독립 view에 보존하고, 다음 primary down에서 같은 egui InputOptions의 delay/distance로 Word/Line을 시작합니다. 임의로 새 double-click 타이밍 상수를 정하지 않습니다. 같은 frame의 실제 primary press 위치·modifier를 사용하며 word/line seed와 endpoint 단위를 보존해 누른 채 드래그·Shift 확장을 처리합니다. 일반/Alt column 선택은 기존 half-cell 경계입니다. 다른 button의 drag는 selection을 갱신하지 않습니다. Cmd+A는 gesture seed를 정리합니다.
- view는 같은 Session.snapshot 안의 Core를 빌리며 범위 계산 실패는 view.error에 기록합니다. UI/OS 입력 자체를 직접 조작하지 않았습니다. 종료 시 원래 TS `terminal-session.tsx`처럼 별도 종료/restart 화면을 표시하는 기존 native 정책은 유지합니다. 종료 화면을 과거 grid와 겹치도록 바꾸지 않았습니다.

## 실제 검증

기존 CARGO_HOME, 직렬 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 합성 grid와 headless egui만 사용했으며 보호 앱/bundle·사용자 파일·OS clipboard/settings/input method/VoiceOver는 조작하지 않았습니다.

1. [x] native `cargo test --manifest-path native/taide-native-terminal/Cargo.toml … --test selection -- --nocapture`: API 부재 E0432/E0599 RED 뒤 신규 1 PASS(compile 0.83초/suite 0.00초). separator를 클릭한 원본 확장·공백·wide/NFD/astral·wrapped word/line·미출력 tail·grid 밖 거절과 Core display offset/total rows 유지입니다.
2. [x] native `cargo test … --lib word_line -- --nocapture`: 신규 visit-budget 검사 1 PASS(compile 0.60초/suite 0.00초). 두 종류의 상한 초과를 부분 범위 성공으로 바꾸지 않습니다. 이 filter는 기존 성공 selection_text 검사를 반복하지 않았습니다.
3. [x] app `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib selection_pointer -- --nocapture`: 최종 1 PASS(compile 1.34초/suite 0.03초). 실제 headless pointer down/move/up의 두 번째 누름에서 Word, 세 번째 누름에서 Line, 각각의 drag와 wrapped line Shift 확장·일반 Shift·다른 View 선택 유지입니다. 최초 0.02초 실패는 최초 화면 등록과 첫 press를 같은 frame에 넣은 fixture 때문입니다. 설치된 egui Context source의 이전 pass widget rect hit-test를 확인하고 입력 없는 최초 화면을 먼저 그린 뒤 영향 검사만 재실행했습니다. 정상 renderer/검사기를 끄지 않았습니다.
4. [x] native `clippy … --lib --test selection -- -D warnings`: exit 0(0.50초). app `clippy … --lib -- -D warnings`: exit 0(2.67초). authored exact rustfmt·tracked `git diff --check` exit 0입니다. 기본 copy/PTy/headless/font/cursor·palette·Hub/query/owner의 같은 성공은 재사용했습니다. Wry 기존 dependency 경고 17개는 유지합니다.

## 전체 미완료 gate

선택 anchor의 실제 출력/trim/resize/alternate buffer/RIS 수명·auto-scroll·hidden focus/다중 window controlling view·링크 우선 double-click·context/search/link와 실제 OS click cadence·touch/비 Primary/많은 연속 click 동등성, 모든 VT wrap/erase·history 선택·줄/공백에 결합된 특수 Unicode의 원본 UTF-16 세부 동등성은 남습니다. 이번 bounds는 고정된 기존 xterm 기본 separator이며 새 custom separator 설정을 추가하지 않습니다. WRAPLINE과 xterm 다음 줄 isWrapped의 모든 변화 조합을 검증했다고 주장하지 않습니다. selection copy의 64KiB/visit 정책·전체 font/worker·aggregate/RSS/CPU/GPU·IME/AX·213 TS view/full cutover/배포도 아직 미완료입니다.

## 후속 anchor 경계의 확인된 source

이 checkpoint 당시 SelectionService의 onUserInput·buffer activate/reset·rowsChanged resize와 SelectionModel의 원래 start/end trim 정책을 확인했습니다. 이후 실제 Grid origin/trim·Term buffer/rows/input epoch, resize 내부 이동 보정·독립 view reconciliation·현재 Select All·원래 Word/Line 최소 길이/optional end를 연결한 결과는 `2026-10-02-m8-native-terminal-selection-lifetime.md`가 정본입니다. 후속 `_handleIncrementalClick` 대조에서 Shift down은 wrapped target 확장이 아닌 raw end임을 확인해 구현과 과거 fixture의 기대를 함께 정정했습니다. 후보 상태/미구현이라는 과거 설명을 현재 상태로 사용하지 않습니다. anchor 전체·컬럼 밖 start/half-open 끝점·auto-scroll/OS/full VT gate는 여전히 미완료입니다.
