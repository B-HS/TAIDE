# Native terminal mouse mode의 xterm parity 차이

## 대상·관찰

native Alacritty fork의 `src/term/mod.rs`, `native/taide-native-terminal/src/{input,lib}.rs`, `tests/input.rs`입니다.

actual Core에서 `CSI ? 9 h` 뒤 left press가 기대한 6-byte binary report 대신 Ignore인 RED를 관찰했습니다. 기존 vte는 9/1016을 Unknown으로 전달하고 upstream Alacritty는 이를 무시합니다. source 비교에서는 원본 xterm이 tracking mode 중 어떤 reset도 protocol NONE으로 만들고 1005/1015를 무시하지만, Alacritty는 해당 bit만 제거하거나 UTF8 encoding을 설정하는 차이도 확인했습니다. 첫 assertion에서 멈춘 RED를 나머지 assertion의 실행 실패로 부풀리지 않습니다.

## 해결·판정

기존 native-retained opt-in의 typed Unknown private-mode 경계와 남은 u32 TermMode bit에 9/1016 상태를 추가했습니다. native tracking reset·encoding set/reset·무시된 UTF8/URXVT·mode query를 설치된 xterm과 일치시켰습니다. pure encoder의 default/SGR/pixel와 X10 modifier 제거·이벤트 제한은 actual Core의 live mode에서 도출합니다. 별도 parser·새 dependency·기대값 우회는 없습니다.

최종 actual input 3 PASS·실제 child PTY binary `0xff`/live SGR 1 PASS·native/app strict exit 0입니다. 수치·명령·off-feature compile 제약·설치된 xterm의 pixel convention과 남은 raw pointer/GUI gate는 `docs/quality-assurance/2026-10-02-m8-native-terminal-mouse-encoding.md`가 정본입니다. 전체 mouse UI/N4/M8 완료가 아닙니다.
