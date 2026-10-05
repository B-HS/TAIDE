# M8 native terminal drag auto-scroll 기본 경계

## 대상·상태

`native/taide-native-app/src/terminal_surface.rs`의 독립 View·Drag·select_pointer·drag_tick·drag_endpoint와 실제 headless egui 검사입니다. 메인이 workflow·서브에이전트 없이 직접 진행했습니다. 기본 코드 checkpoint이며 실제 OS pointer/다중 창·전체 선택/N4/M8 완료가 아닙니다. 기존 Core/PTy/Hub/font 성공은 재사용하며 N1~N8 0/8·전체 M8 완료 뒤만 commit/push를 유지합니다.

## 원본·구현

1. 설치된 xterm `src/browser/services/SelectionService.ts`의 `_getMouseEventScrollAmount`, `_handleMouseMove`, `_dragScroll`, `_handleMouseUp`, clearSelection을 대조했습니다. 50ms 주기·실제 canvas의 위/아래 50px·최대 15행과 JavaScript Math.round의 음수 반올림을 보존합니다. canvas 높이는 실제 Core rows×cell height입니다.
2. View에 inline scalar timer·지난 시각·행 수·end 유무를 보존합니다. 한 pass에 한 bounded step만 수행하고 누락 주기는 건너뜁니다. 역행/비유한 시각·다음 주기 산술 실패는 취소합니다. 실제 history까지 제한하고 같은 Core display_offset이나 다른 view를 변경하지 않습니다. thread/task·추가 Core/문자열 cache는 없습니다.
3. 위쪽 tick은 viewport 첫 행의 col 0, 아래쪽 tick은 원본처럼 viewport 다음 행 또는 buffer 마지막 행의 col columns입니다. Block은 실제 half-open x를 보존합니다. Word/Line은 정렬된 최종 범위 대신 seed의 raw end를 사용합니다. mousemove는 원래 행을 유지하며 열만 강제하고 timer만 viewport 행을 적용합니다. Select All 최종 범위는 현재 전체 buffer를 유지합니다.
4. mouseup만으로 끝점을 다시 계산하지 않습니다. 실제 PointerMoved와 release가 같은 frame에 있으면 그 이동을 처리한 뒤 timer를 취소합니다. disabled·기존 clear_selection·buffer/rows/input epoch·view 교체/재시작 경계를 재사용합니다. 숨은 view에는 실행되는 별도 callback이 없지만 실제 hidden/multi-window ownership 전체 검증은 남습니다.
5. egui 0.36.2 `Response::drag_stopped_by`는 내부에서 Context::input을 호출합니다. Ui::input closure 안의 재호출은 실제 10초 RwLock 실패였으며 closure 밖에서 응답 상태를 읽어 중첩을 제거했습니다. 검사기/잠금을 끄지 않았습니다.

## 실제 검증

Cargo는 기존 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 직렬 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 합성 실제 Core·headless egui RawInput/down/move/idle/release만 사용했습니다.

| target/filter | 결과 | 범위 |
| --- | --- | --- |
| app `--lib selection_autoscroll -- --nocapture` | offset 0≠8 실제 RED 뒤 1 PASS, compile 3.46초/suite 0.01초 | 50ms 전후 실제 위쪽 timer·최대 속도/history 8행 상한·row6 copy·Core offset 0 |
| app `--lib selection_drag_policy -- --nocapture` | release end Line0≠Line1 RED, 중첩 잠금 실패 수정 뒤 최종 1 PASS, compile 1.44초/suite 0.01초 | 실제 아래쪽 timer·release 범위 유지/정지·Block/Line raw seed/All 직접 경계·disabled·실제 alt ABA |
| app `--lib selection_pointer -- --nocapture` | 수정한 release의 기존 영향 1 PASS, compile 0.22초/suite 0.01초 | down/Word/Line/drag/Shift·독립 view·paint |
| app `clippy ... --lib -- -D warnings` | exit 0, 1.88초 | authored app 엄격 검사, inherited Wry 17 warnings와 구분 |
| authored `rustfmt --edition 2024 --check .../terminal_surface.rs`, `git diff --check` | exit 0 | 정확한 작성 파일·추적 변경 공백 |

고유 신규 검사는 2건입니다. 위쪽 timer 정상 경로는 이후 release/seed/All/clock 방어 수정으로 입력·정상 분기가 바뀌지 않아 최초 성공을 재사용했습니다. release/seed 계약 추가는 실제 production/test 입력 변경 뒤에만 관련 검사 1회씩 수행했습니다. 같은 상태 성공을 통합 때문에 다시 실행하지 않았습니다. untracked native 파일은 `git diff --check` 대상이 아니므로 별도 authored rustfmt 결과로 구분합니다.

## 잔여 gate

- [x] 기본 actual headless down/move/timer·위/아래 끝점·history 상한·release/disabled/alt 정리입니다.
- [ ] 실제 OS 창 밖 pointer/capture·multi-window/hidden focus·실제 resize/클립 지연·GUI repaint/GPU/AX입니다. 현재 headless 결과를 OS 성공으로 주장하지 않습니다.
- [ ] wheel/mouse tracking·link/context/search·전체 Block/Word/Unicode/VT·원본 TUI corpus·전체 retained/peak/RSS/CPU·N4/M8입니다. 후속 해당 코드 연결 또는 제품 cutover 전에 실행합니다.

보호 실기 bundle·기존 TS/root/MSRV·사용자 데이터/clipboard·입력기/VoiceOver는 조작하지 않았으며 dependency/새 package/unsafe/검사 억제/Git 변경은 없습니다.
