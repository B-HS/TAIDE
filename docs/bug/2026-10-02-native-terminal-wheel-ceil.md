# Native terminal 작은 휠 입력의 과도한 행 이동

## 대상·재현

`native/taide-native-app/src/terminal_surface.rs`입니다. 실제 headless egui에서 1point MouseWheel을 보내면 기존 smooth_scroll_delta/cell height의 ceil로 offset 1이 됐습니다. 원본 Viewport의 fractional pixel·행 반올림 계약에서는 offset 0이며 실제 1≠0 RED입니다.

## 원인·수정

매 pass의 작은 또는 smoothed delta를 각각 한 행으로 올림하고 fractional 위치를 버렸습니다. 독립 WheelPosition의 pixel 위치·actual history 상한·원본 modern-wheel normalization과 Math.round를 연결합니다. raw event를 scoped 소비해 idle smooth pass만으로 native 행을 추가하지 않으며 alt/scrollback 0의 단일 방향키를 기존 live Core/writer로 전달합니다.

## 검증·잔여

point 1 PASS(0.01초)·정책 1 PASS(0.02초)·새 actual wheel PTY와 기존 surface/fixture 영향 2 PASS(0.39초)·관련 strict exit 0(3.06초)입니다. 정본은 `docs/quality-assurance/2026-10-02-m8-native-terminal-wheel.md`입니다. global smoothing tail의 다른 surface 소유권·legacy OS delta/DPR·전체 mouse/N4/M8는 아직 남습니다.
