# M8 native terminal 포커스·조합 입력 수명

## 대상·근거

`native/taide-native-app/src/{terminal_surface,application}.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`입니다. N4-B의 숨김·복원·창 포커스 변경과 preedit 수명을 연결했습니다. 전체 focus protocol·N4·M8 완료는 아닙니다.

설치된 egui 0.36.2의 `context.rs::run_logic`는 새 `raw.focused`와 viewport 목록을 반영하지만 UI events와 `InputState.focused`는 해석하지 않습니다. logic-only blur는 `raw.focused`로 확인합니다. `ImeEvent::Enabled/Disabled`는 deprecated이며 실제 egui-winit에서 사용하지 않으므로 구현·검사에서 제거했습니다. 빈 Preedit와 Commit, 실제 포커스 상실을 조합 종료 경계로 사용합니다.

[xterm 6.0.0 CoreBrowserTerminal](https://raw.githubusercontent.com/xtermjs/xterm.js/6.0.0/src/browser/CoreBrowserTerminal.ts)의 focus/blur 보고와 설치된 `InputHandler.ts`의 DECSET 1004 경계를 대조했습니다. 원본 focus는 스크롤 이동을 요청하지 않습니다. native의 일반 입력 제출이 focus 보고에도 offset을 0으로 만들던 부분을 분리했습니다. 원본의 1004 활성화 때 현재 focus를 즉시 보고하는 동작은 이번 구현에 포함되지 않았습니다.

## 구현

- [x] 각 View에 focus widget id·마지막 표시 frame을 기록합니다. 완료 frame에서 보이지 않는 View, 다른 widget으로 포커스가 이동한 View, window blur의 preedit를 해제하고 FocusOut을 보냅니다. 반복 logic tick은 중복 보고하지 않습니다.
- [x] 새 View에 FocusIn을 보내기 전에 기존 View의 FocusOut을 처리합니다. 같은 Session의 두 View도 이 순서를 유지합니다. NativeApplication은 shell 표시 뒤 및 닫기 dialog 처리 뒤에 완료 frame 정리를 호출합니다.
- [x] 시작 중인 View도 표시 frame을 기록하여 Session attach 전 preedit가 매 frame 잘못 지워지지 않게 했습니다. mouse 처리 frame과 focus 표시 frame은 별개입니다.
- [x] raw viewport 목록에서 제거된 View는 FocusOut·preedit 정리 후 회수합니다. raw mouse drain이 이미 닫힌 viewport의 egui frame counter를 조회하지 않도록 현재 viewport와 저장된 캡처 frame을 구분합니다.
- [x] Focus 보고로 scroll offset을 0으로 바꾸지 않습니다. 이 조건의 코드·strict 검사는 확인했지만 실제 스크롤 위치를 비교하는 GUI 검사는 하지 않았습니다.

## 실제 검사

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 직렬 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. app manifest는 `native/taide-native-app/Cargo.toml`입니다.

| 검사 | 실제 결과 |
| --- | --- |
| `--test terminal-host focus_surface` 최초 재현 | exit 101, compile 5.13초 / suite 3.47초. 숨김·logic blur의 report가 빠져 child 완료를 기다리다 실패했습니다. |
| 같은 검사, 기본 연결 후 | 1 PASS, compile 7.96초 / suite 0.36초. 당시 deprecated IME Disabled 경고가 있어 최종 strict 근거로 사용하지 않았습니다. |
| deprecated 분기 제거·startup frame 기록·완료 frame 배선 후 | 1 PASS, compile 6.41초 / suite 0.40초. 실제 preedit·raw focus만 사용했습니다. |
| 닫힌 viewport 조회 분리 후 `--test terminal-host headless_surface` | 1 PASS, compile 2.79초 / suite 0.46초. 중단된 session 91737 결과를 회수했으며 재실행하지 않았습니다. |
| 공유 수명 변경의 `--test terminal-host hidden_mouse` | 1 PASS, compile 0.22초 / suite 1.44초. 포화·숨김 release·다른 View의 입력 순서를 확인했습니다. |
| auxiliary viewport 표시·제거까지 확장한 `--test terminal-host focus_surface` | 최종 1 PASS, compile 4.96초 / suite 0.36초. child가 Focus 10개와 문자 x의 정확한 순서를 확인했습니다. epoch는 x 한 번만 증가했고 exit 0·close/join·task 0입니다. |

실제 합성 PTY와 headless egui 검사입니다. OS 창·실제 CJK 입력기·VoiceOver·사용자 clipboard는 조작하지 않았습니다. 초기 실패에서도 child close/join 후 assertion하도록 했습니다. 같은 성공 검사는 반복하지 않고, 구현 및 검사 범위가 바뀐 경우만 관련 검사를 수행했습니다.

최종 app `clippy --lib --test terminal-host -- -D warnings` exit 0(0.67초), native terminal `clippy --bin native-session-fixture -- -D warnings` exit 0(0.23초)입니다. 기존 Wry 17 warnings와 authored strict 검사를 구분합니다. authored 4파일 exact rustfmt·추적 diff 검사는 exit 0입니다. 이전 기본 strict 1.40초/0.23초 이후 auxiliary 검사 코드가 바뀌어 해당 target을 최종 검사했습니다.

## 남은 경계

- [x] 후속 64개 Outbox 포화의 bounded focus 보존·재시도·새 입력 추월 방지는 `2026-10-03-m8-native-terminal-focus-pressure.md`에서 구현·검증했습니다. 전용 byte 상한 초과·모든 View/query/keyboard 순서까지 입력 큐 수명 전체 완료로 계산하지 않습니다.
- [x] 후속 DECSET 1004 활성화·반복 활성화 때 현재 focus 보고는 `2026-10-03-m8-native-terminal-focus-query.md`에서 구현·검증했습니다. 활성화/해제와 대기 보고의 순서, 정확한 attach 전후 조합 보존은 여전히 추가 구현/검증 대상입니다.
- [ ] 실제 OS 보조 창·숨김/최소화·DPR/transform·모든 modal/context 진입점·scroll 픽셀과 전체 IME/AX는 미완료입니다. headless auxiliary 검사는 실제 창 생성/종료를 입증하지 않습니다.
- [ ] 일반 captured-wheel 숨김·동일 event 일부 소비의 모호한 순서·UI 주입·mode ABA/forced selection·context/search/link/project Hub·전체 VT/aggregate/GUI/성능과 213 TS view/full cutover/TS 제거가 남습니다.

보호 실기 bundle·기존 TS/root/MSRV·사용자 데이터/OS 설정은 유지했습니다. M8 N1~N8 전체는 0/8이며 전체 완료 후 commit·push합니다.
