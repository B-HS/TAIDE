# M8 native terminal overview ruler와 fork feature 조합

## 대상과 원본 근거

- native app의 `src/terminal_ruler.rs`, `src/terminal_surface.rs`, `src/lib.rs`와 native fork의 `src/term/native_commands.rs`입니다. 별도 `experiments/terminal-feature-gate`는 제품에서 사용하는 실제 Alacritty/vte path fork를 검사하며 registry Alacritty로 대체하지 않습니다.
- 설치된 xterm 6.0.0의 `OverviewRulerRenderer.ts`, `ColorZoneStore.ts`, `SortedList.ts`, `ThemeService.ts`, CSS와 FitAddon source가 기준입니다. 제품 `terminal-view.tsx`는 ruler 폭 14만 지정하고 top/bottom border를 켜지 않으며 `xterm-theme.ts`는 overview border 색을 설정하지 않습니다. 따라서 원본 기본 흰색 border를 사용합니다. 테마 전경색을 border에 재사용하지 않습니다.
- workflow·서브에이전트 없이 main이 구현했습니다. PROCESS의 N4-C OSC133 ruler/feature-off 항목 안에서 진행했습니다. 기존 제품 TS/root manifest/MSRV·보호 실기 bundle·OS 설정·실제 clipboard/browser는 변경하거나 조작하지 않았습니다.

## 구현 결과

- [x] CSS 14px ruler와 1 물리 pixel border, right zone의 floor/ceil 폭과 x 위치를 재현합니다. physical canvas 폭/높이는 JS Math.round로 계산하고 CSS 공간으로 다시 매핑합니다. fractional DPR의 여백도 임의로 정규화하지 않습니다.
- [x] 전체 primary buffer 행 수를 기준으로 6~12px clamp 뒤 DPR을 곱한 zone 높이, padding과 음수 y의 JS 반올림을 적용합니다. 같은 색의 첫 인접 zone에 행 범위를 합치고 전체 ruler clip을 유지합니다. alternate에서는 border와 zone 모두 숨깁니다. 명령이 없어도 primary border는 표시합니다.
- [x] 같은 Core의 history를 읽어 화면 밖 완료 명령도 표시합니다. gutter/cursor/preedit 뒤에 그려 원본 ruler의 상위 표시 순서를 유지하고 별도 widget이나 이벤트 소비를 만들지 않습니다. 살아 있는 primary 색 block이 없으면 fork decoration lookup은 행 map을 만들기 전에 반환합니다.
- [x] 원본 FitAddon처럼 scrollback 설정이 0이 아닐 때 열 계산에서 ruler 폭 14px를 예약합니다. history가 아직 비어 있거나 alternate라는 이유로 이 폭을 제거하지 않습니다. scrollback 0이면 예약하지 않습니다. 기존 native의 최소 3열/2행 admission은 이번 변경에서 유지합니다.
- [x] 실제 path fork의 native-retained off/no-serde, off/serde, on/serde를 격리 manifest에서 컴파일하고 기본 parser·Row clone·region swap 및 Row serde 왕복을 검사합니다. native off에서는 원본 Row의 4-word layout이 유지됩니다. on/serde에서는 runtime marker가 저장되지 않고 복원/clone에서 복제되지 않으며 원본 Row drop 때 폐기됩니다. 이미 성공한 제품 native on/no-serde 검사 결과는 재사용합니다.

## 실제 검증

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`으로 한 번에 하나씩 실행했습니다. 새 격리 feature-gate lock만 `generate-lockfile --offline`으로 처음 생성했습니다. root/native app lock을 갱신한 명령은 없습니다.

1. app `--lib ruler_`: geometry/실제 paint 신규 2 PASS, suite 0.30초·compile 1.96초입니다. DPR 1/2/1.25, 행 병합·잘못된 크기, 화면 밖 history·clip·색·alternate 복원·3J 뒤 border를 검사했습니다. actual egui shape의 결과이며 GPU/OS 최종 픽셀 검사가 아닙니다.
2. app `--lib ruler_fit`: 신규 1 PASS, suite 0.00초·compile 2.09초입니다. 640px/8px cell에서 scrollback 설정에 따라 78/80열과 같은 20행을 확인했습니다. 변경된 최소 크기 검사의 fixture 호출도 새 signature에 맞췄습니다.
3. app `--lib command_gutter`: 영향 1 PASS, suite 0.00초입니다. 같은 색 ruler Rect를 gutter로 잘못 세지 않도록 fixture의 기존 2px 폭 조건을 유지했습니다. native `--test commands osc133_decoration`: lookup 조기 반환 영향 1 PASS, suite 0.00초입니다.
4. app `--test terminal-host terminal_commands`: 크기/paint 영향 1 PASS, suite 0.41초·compile 6.41초입니다. 실제 합성 PTY/Views의 명령 이동·gutter·disabled·epoch 불변·continue 뒤 exit Some(0)·join/task 회수를 유지했습니다. 변경되지 않은 parser/Number/trim/clear/URL/menu 검사는 다시 실행하지 않았습니다.
5. feature-gate `--lib`, `--lib --features serde`, `--lib --features serde,native-retained`: 각각 1/2/2 PASS, suite 각 0.00초·compile 1.14/6.41/4.68초입니다. 서로 다른 feature 조합이며 동일 환경의 성공 검사를 반복한 것이 아닙니다.

app lib/terminal-host strict clippy exit 0(1.83초), native lib/commands strict clippy exit 0(0.73초), feature-gate lib/tests on/serde strict clippy exit 0(3.72초)입니다. 기존 Wry의 17 경고는 authored strict 성공과 구분하며 suppression을 추가하지 않았습니다. authored Rust 5파일 exact rustfmt check, QA 두 파일 Prettier check와 tracked diff check도 exit 0입니다. 코드·vendor/provenance·새 gate manifest/lock·QA의 명시된 untracked 10개 경로 no-index whitespace 출력은 모두 비어 있습니다(exit 1은 /dev/null과 내용 차이). 검사 뒤 live Cargo handle은 없습니다.

## 실패와 정정

- 최초 ruler geometry의 DPR 1 fixture가 right x를 9로 잘못 계산해 actual 636px ≠ expected 635px였습니다. 원본 floor((14-1)/3)=4, ceil((14-1)/3)=5, border 1의 합은 10입니다. 구현식은 그대로 두고 기대값만 정정했습니다. 동일 실패를 세 번 반복하지 않았습니다.
- feature-gate 최초 compile E0283은 generic `Processor::default()`의 Timeout 타입을 fixture에서 추론할 수 없는 문제였습니다. 설치된 기본 `StdSyncHandler`를 명시한 뒤 해당 조합만 재실행했습니다. 제품 parser 오류나 runtime 실패로 기록하지 않습니다. rustfmt로 바뀐 문맥을 사용한 patch 한 건은 원자적 apply 실패로 변경되지 않았으며 현재 파일을 다시 읽고 적용했습니다.

## 미완료와 다음 경계

- [ ] ruler의 실제 GPU rasterization·DOM clientHeight 정수화/zoom·작은 pane·원본 최소 2열/1행과 native admission·scrollbar drag/track 상호작용을 전체 terminal geometry gate에서 비교합니다. 현재 shape 검사를 최종 시각/입력 동등성으로 세지 않습니다.
- [ ] 동일 행에 서로 다른 색 decoration이 생길 때 xterm SortedList는 같은 flush batch 안에서 stable하지만 새 batch를 이전 같은 key 앞에 삽입합니다. native는 line stable sort를 사용하며 원본 async flush cadence를 아직 재현하지 않았습니다. 전체 overlap 동등성은 미완료입니다.
- [ ] original `pane-node-view.tsx`는 active terminal만 mount합니다. `terminal-view.tsx`는 매 mount마다 새 tracker를 replay 전에 설치하고 unmount 때 marker/decoration/latch를 폐기합니다. `TerminalSessionOutput::attach`는 같은 output lock 안에서 ESC[0m과 bounded ring front/back을 replay합니다. 반면 native Hub의 Core는 session 동안 유지되고 Views는 숨김 때 focus/link/menu만 정리합니다. remount 시 retained ring 밖의 오래된 block, C latch와 새 theme 적용이 달라질 수 있습니다. 단순히 보이는 block의 색만 바꾸거나 숨김 때 command 전체를 지우는 방식으로 이 차이를 덮지 않습니다. 단일 canonical parser/다중 view 계약과 replay/view 수명을 함께 연결하는 다음 구현이 필요합니다.
- [ ] history decoration lookup은 색 block이 있으면 전체 행 scan과 임시 map을 사용합니다. 무명령 조기 반환은 전체 50k 행 CPU/lock hold·aggregate/allocator/RSS 성능 통과가 아닙니다.
- [ ] feature-gate는 새 lock이 해석한 격리 dependency graph와 현재 Rust 1.98.1/macOS arm64의 좁은 검사입니다. upstream corpus·모든 serde 타입 roundtrip·원본 전체 feature-off runtime·선언 MSRV/Windows/Linux 통과로 확대하지 않습니다.

N4-C와 M8 상위 N1~N8은 0/8 완료입니다. 213 view 대응·full cutover·TS 제거·전체 IME/AX/성능/보안/배포가 남고 전체 완료 후에만 commit·일반 push합니다.

## 후속 확인: 재표시 정책과 단일 파싱 계약 충돌

`docs/acknowledge/2026-09-23-rust-native-transition-contract.md` §3.4는 PTY 출력을 하나의 Rust Core가 단 한 번 파싱하고 같은 parser 결과에서 화면/OSC 효과를 파생하도록 명시합니다. 원본 TS의 active-tab mount→새 xterm/tracker→raw ring replay 정책을 그대로 다시 구현하면 이미 파싱한 바이트의 두 번째 VT 파싱이 필요합니다. 새 theme/C latch/잘린 ring의 marker만 보이는 화면에서 추정해 바꾸는 것은 원본 replay 전체 재현도 아니고 근본 해결도 아닙니다.

후속 source를 다시 확인했습니다. native SharedTerminal은 같은 Core의 borrowed snapshot을 제공하며 Hub가 legacy raw ring도 보관합니다. 실제 파싱은 `spawn_with_output`의 live callback→`advance_with_delivery` 한 번이고 Views는 이를 snapshot으로 읽습니다. 재표시 때 공유 Core를 새로 만들거나 추가 parser를 두면 기존 단일 owner/다중 view와 부수 효과의 정확히 한 번 전달도 함께 변경해야 합니다.

사용자에게 한 묶음의 선택을 표시했습니다: A. 기존 단일 Core를 유지하고 재표시 시 화면·명령 이력 보존 차이를 명시(추천), B. 원본 재파싱을 재현하도록 단일 파싱 계약 변경. 아직 응답이 없으므로 어느 쪽도 새 합의로 확정하지 않았습니다. 구현·검사 성공을 추가로 주장하지 않으며 기존 ruler/feature-gate 성공은 재사용합니다. M8 목표는 active이고 commit/push·TS 제거는 수행하지 않았습니다.
