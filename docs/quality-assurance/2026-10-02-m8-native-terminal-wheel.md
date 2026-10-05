# M8 native terminal wheel 기본 경계

> 2026-10-02 정정·후속: user-input epoch는 viewport 위치만 재동기화하며 wheel partial은 유지하도록 원본 CoreMouseService에 맞췄습니다. 기존 0 기대값은 입력 전 fractional 값 보존으로 정정했고 wheel_policy 1 PASS(0.02초)입니다. raw-wheel owner 뒤 mouse adapter에서 실제 5회 point→input epoch→6번째 wheel byte와 X10 fallback을 확인했습니다. 정본은 `2026-10-02-m8-native-terminal-mouse-adapter.md`이며 아래 본문은 당시 checkpoint 기록입니다.

## 대상·상태

`native/taide-native-app/src/terminal_surface.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/src/lib.rs`, `tests/fixtures/session.rs`입니다. 메인이 workflow·서브에이전트 없이 직접 진행했습니다. 실제 raw event/독립 scalar view·live Core·합성 PTY의 기본 연결 checkpoint이며 전체 wheel/mouse·N4/M8 완료가 아닙니다. 보호 실기 bundle·사용자 데이터/clipboard·OS 설정·TS/root/MSRV는 유지하고 전체 M8 완료 뒤만 commit/push합니다.

## 원본·구현

1. 설치된 xterm Viewport는 소수 pixel scrollTop을 유지하고 `Math.round(scrollTop / cell.height)`로 행을 결정합니다. 기존 native의 smooth_scroll_delta/cell.height를 매 pass ceil하던 코드는 실제 1point에서도 한 행을 이동했습니다. 독립 WheelPosition이 offset/history/cell height와 fractional pixel 위치를 유지하며 위/아래 실제 history까지 제한합니다. 외부 Page/drag/출력으로 정수 offset/history/height가 달라지면 실제 위치에서 다시 시작합니다. buffer/input/rows epoch는 Wheel을 취소합니다.
2. `vs/base/browser/mouseEvent.ts`의 modern WheelEvent와 `scrollableElement.ts`의 50px 감도·pixel delta/40·line delta·Alt×5·방향별 pixel ceil/floor를 대조했습니다. 원본 현대 경로는 Page도 pixel 분기에 두므로 여기서도 동일하게 처리합니다. legacy WebKit wheelDelta/120·Chrome 이전 DPR·Firefox/비macOS 차이와 실제 OS 장치의 변환 parity는 미완료입니다. 이 현대 source mapping을 모든 플랫폼의 정확한 wheel 감도로 주장하지 않습니다.
3. normal buffer의 설정된 scrollback capacity가 0보다 크면 현재 history가 비어도 방향키를 보내지 않습니다. 실제 Core `has_scrollback`은 live ALT_SCREEN과 기존 설정 capacity를 확인합니다. alt/설정 0에서는 CoreMouseService.consumeWheelEvent처럼 point를 cell height로 나누고 작은 point에 0.3·Alt/Ctrl×5·fractional remainder를 적용합니다. Shift는 0, Line/Page는 해당 source 분기이며 결과가 여러 행이어도 raw wheel event 하나에 방향키 하나입니다. 기존 live encoder가 APP_CURSOR에 맞춰 CSI 또는 SS3를 만들고 기존 session/writer로 보냅니다.
4. hovered·enabled인 surface만 vertical-dominant raw MouseWheel을 소비합니다. disabled/outside·수평/수평 우세·다른 event를 보존합니다. MOUSE_MODE에서는 이벤트를 보존하며 가짜 viewport scroll/대체 화면 방향키를 보내지 않습니다. 실제 mouse encoder 연결은 아직 남습니다. 결과 Key 벡터는 남은 64 receipt 상한을 검사하며 초과/비유한 geometry는 명시적 실패입니다. 초과 batch는 writer로 보내지 않으며 소비된 앞부분과 남은 event의 완전 replay는 보장하지 않습니다.
5. InputState.wheel은 egui 비공개 필드입니다. E0616 compile 실패 뒤 해당 접근을 제거했습니다. 공개 raw events와 smooth_scroll_delta.y만 처리하며 egui fork/unsafe/검사 억제/새 dependency는 추가하지 않았습니다. native 자체는 raw event가 없는 idle pass에서 새 행을 만들지 않습니다. egui 내부 global smoothing tail이 pointer 이동 뒤 다른 ScrollArea에 전달되는 전체 소유권은 아직 검사/해결하지 않았으며 후속 wheel owner/mouse gate입니다.
6. 테스트 fixture만 승인된 자기 child PTY에 `stty -icanon`을 적용하고 정확한 6byte `ESC OA ESC OB`를 읽은 뒤 별도 완료 title·exit 0을 보냅니다. 테스트 opt-in env key가 없으면 기존 canonical continue/live-sync/final-query 경로를 유지합니다. 사용자 terminal/입력기/VoiceOver나 보호 앱을 조작하지 않았습니다.

## 실제 검증

기존 CARGO_HOME·직렬 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. OS GUI가 아니라 actual headless egui·actual Core와 합성 실제 child PTY만 사용했습니다.

| app target/filter | 실제 결과 | 범위 |
| --- | --- | --- |
| `--lib wheel는 -- --nocapture` | offset 1≠0 실제 RED·비공개 필드 compile 실패 수정 뒤 1 PASS, compile 3.77초/suite 0.01초 | 작은 point가 매 pass 한 행이 되지 않음 |
| `--lib wheel_policy -- --nocapture` | 1 PASS, compile 2.74초/suite 0.02초 | 실제 5 event fractional 위치·idle 유지·point/line/page/Alt/Ctrl·상하 상한·disabled/outside/수평 보존·설정 0/빈 history/alt·partial/Shift·live CSI/SS3·mouse 비처리·64 receipt 거절·input epoch·NaN 거절 |
| `--test terminal-host headless_ -- --nocapture` | 신규 actual wheel 1 + 변경 surface/fixture의 기존 startup/IME/restart 영향 1 PASS, compile 10.30초/suite 0.39초 | 실제 surface→기존 writer→alt/app-cursor PTY에서 정확히 한 Up/Down·완료 title/exit 0·join/TaskSupervisor 0 |
| `clippy ... --lib --test terminal-host --bin native-terminal-queue-fixture -- -D warnings` | exit 0, 3.06초 | authored app/host test/fixture 정적 검사, 기존 Wry 17 warnings와 구분 |
| authored 4파일 `rustfmt --edition 2024 --check`, tracked `git diff --check`, 명시된 untracked 파일 trailing blank 검사 | fmt/diff exit 0·공백 불일치 없음 | native/app 작성 파일·이 checkpoint 문서, vendor 전체 formatting 아님 |

고유 신규 3건과 실제 변경된 surface/fixture의 기존 영향 1건입니다. 같은 정상 구현의 point/policy 성공은 후속 테스트 fixture 추가 뒤 재사용했습니다. 기존 PTY/Hub/font/selection 전체 성공을 통합만을 위해 다시 실행하지 않았습니다. native Core의 새 읽기 메서드는 app graph에서 compile되었으며 이번에 별도 native clippy 성공을 주장하지 않습니다.

## 잔여 gate

- [x] actual raw/headless 기본 누적·상한·scope와 실제 synthetic alt PTY의 단일 방향키/회수입니다.
- [ ] global egui smoothing tail의 capture/다른 surface 소유권·숨은/다중 창·mouse tracking입니다. pointer가 다른 스크롤 영역으로 이동하는 synthetic frame을 먼저 재현하고 actual raw-input adapter 경계에서 연결해야 합니다.
- [ ] 실제 OS wheelDelta/line/page/DPR·장치/키 조합·TUI·GPU/AX·전역 retained/peak/RSS/CPU·전체 terminal/N4/M8·제품 cutover/TS 제거입니다. 현대 source mapping과 논리 상한만으로 이 gate를 통과시키지 않습니다.
