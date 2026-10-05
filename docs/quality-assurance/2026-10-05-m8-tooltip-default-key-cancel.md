# M8 Tooltip 기본 키 취소·선행 Focus

## 대상·계약

`native/taide-native-app/src/tooltips.rs`, `tooltip-key-tests.rs`, `tooltips-tests.rs`입니다. 사용자 요청대로 workflow·서브에이전트 없이 main이 직접 구현했습니다. PROCESS·verify·save-docs 스킬을 적용하며 존재하지 않는 `docs/convention/ai-process.md` 대신 설치된 `/Users/hyunseokbyun/.codex/llm-rules/ai-process.md`를 기준으로 삼습니다.

설치된 Radix Tooltip Trigger는 Enter/Space 자체의 key handler가 아니라 `composeEventHandlers(props.onClick, context.onClose)`로 닫습니다. 기본 버튼 클릭을 취소한 키는 close 근거가 아닙니다. [Event.preventDefault](https://developer.mozilla.org/en-US/docs/Web/API/Event/preventDefault)와 [egui InputState.consume_key](https://docs.rs/egui/0.36.2/egui/struct.InputState.html#method.consume_key), 실제 pinned InputState를 확인했습니다. consume_key는 pressed 사건을 normalized events에서 제거하지만 raw.events에는 남깁니다. InputState의 repeat 정규화는 raw와 normalized 복사 전에 적용됩니다. 원본 DOM 새 계측은 하지 않았습니다.

## 재현·수정

begin_frame 전후에 Enter/Space를 소비한 실제 Response에서 clicked=false인데 Tooltip이 닫히고, 취소된 Space press의 다음 release도 닫는 RED를 확인했습니다. raw 선행 replay가 소비 여부와 무관하게 즉시 close/arm한 것이 원인입니다.

등록된 owner에 Enter/release close 후보를 기록하고 Trigger 처리 시점의 남은 normalized key만 반영합니다. Space arm은 pass/index를 보존해 같은 pass의 취소된 press와 취소된 release를 배제합니다. 각 후보의 owner/open generation을 확인해 나중에 열린 다른 Tooltip을 닫지 않습니다. 새 Trigger의 fallback도 같은 소비 여부를 확인합니다. 기존 pointer/AX/Escape 선행 capture는 유지합니다. Trigger마다 Text/다른 Event 전체를 다시 clone하지 않고 Key 사건만 수집합니다.

Enter→AX Focus2에서 원래 owner1을 다시 여는 별도 RED도 확인했습니다. 선행 AX replay의 focus와 widget draw 시점의 memory focus가 달랐습니다. 이미 등록된 AX Focus를 replay한 pass는 해당 owner의 순서대로 계산된 focus를 사용해 UI 그리기 순서가 선행 배정을 덮지 않게 했습니다. 등록 전/동적 graph와 이후 handler의 새로운 memory focus까지 해결한 것으로 주장하지 않습니다.

기존 400ms/skip 검사는 내장 egui Tooltip을 직접 그리는 fixture여서 공용 dismissal registry를 거치지 않았습니다. 렌더 후 등록만 추가한 첫 정정도 실패했습니다. 진단 snapshot에서 등록 layer는 `.with(1)`, widget.dismissal_id는 `.with(0)`임을 확인했습니다. `next_tooltip_id`의 카운터가 내장 렌더 뒤 증가한 것이 원인입니다. 제품 Provider와 동일하게 Content 렌더 전에 `track_layer`를 호출하도록 fixture를 정정했습니다. 진단 출력은 제거했고 제품 Escape 계약을 약화하지 않았습니다.

## 실행 결과

공통 Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. Cargo는 직렬 실행했고 성공 결과는 같은 위험 범위에서 재사용합니다.

- [x] `cargo test … --lib tooltips::key_tests::tooltip_소비된_enter와_space는_begin_frame_전후에도_click으로_닫지_않는다 -- --exact`: RED compile7.20초/suite0.02초, 수정 뒤 1 PASS compile8.15초/suite0.02초입니다. begin 전후×Enter/Space press/Space release의6조합과 취소된 press의 후속 release를 확인했습니다.
- [x] 추가2검사는 `cargo test … --lib --no-run` compile7.14초 뒤 실제 생성된 lib test binary에 여러 이름과 `--exact`를 전달했습니다. Cargo에 테스트 이름 두 개를 직접 전달한 최초 호출은 CLI exit1이며 테스트 실행/실패로 세지 않습니다.
- [x] 최초 신규2/영향9 batch는9 PASS·2 FAIL, suite0.06초입니다. 새 Trigger/same-pass pair 검사1과 pointer/AX 순서·Space·viewport·controlled2·Modal Escape·닫힘 Presence 성공은 해당 경계의 근거입니다. 실패2는 새 AX Focus 순서1과 기존 fixture의 Escape1입니다.
- [x] AX focus와 fixture 등록을 수정한 뒤 compile6.33초, 영향 범위12검사는11 PASS·1 FAIL, suite0.06초입니다. 신규 AX 검사1은 Enter/Space×Focus 전후×UI draw 순서의8조합을 확인합니다. 추가된 transit/unpaired pointer/touch/content down/pointer ref 검사4도 PASS입니다. 변경 없는 controlled/Modal 성공은 재실행하지 않았습니다.
- [x] 남은 fixture ID 오류를 진단한 단일 실행은 compile7.57초/suite0.02초 FAIL입니다. 관찰값 `.with(1)`/`.with(0)`을 근거로 등록 시점을 바로잡은 뒤 실패1만 재실행해 1 PASS compile7.45초/suite0.01초입니다. 성공한 나머지를 전체 재실행하지 않았습니다.
- [x] `cargo clippy … -p taide-native-app --lib --tests -- -D warnings`: exit0,19.43초입니다. Wry의 기존17 dependency 경고를 숨기지 않았습니다. 최종 Key-only snapshot 수집 변경은 이 정적 검사로 확인하며 이벤트 순서/내용 보존에 관한 기존 성공을 재사용합니다.
- [x] 대상 Rust3의 정확한 rustfmt를 적용하고 `git diff --check` exit0을 확인했습니다. 진단 eprintln/임시 변수는 남기지 않습니다.

이번 범위의 고유 신규3과 영향13은 성공 근거가 있으나 위 실행마다의 실패/수정 상태와 범위를 구별합니다. 기존 검사 성공을 새 제품 전체 gate로 승격하지 않습니다.

## disabled IconButton span 후속

대상에 `icon-tooltip-tests.rs`를 추가했습니다. 원본 `src/shared/ui/icon-button.tsx`는 disabled span만 tabindex=0이고 자식 Button은 disabled입니다. 설치된 Radix의 onClick에는 Enter/Space를 span 클릭으로 바꾸는 handler가 없습니다. pinned `Sense::focusable_noninteractive`도 click을 감지하지 않습니다.

실제 `wrap_button`·disabled Button·AX Focus·Provider renderer에서 Enter/Space 뒤 click=false인데 open/설명이 사라지는 RED를 확인했습니다. 공용 Widget에 기본 키 활성화 가능 여부를 저장해, click sense가 있거나 활성 child focus alias가 있을 때만 default key close를 후보/적용/fallback에 허용합니다. 단순 focusable span은 열림을 유지하고 활성 child alias는 원래대로 닫습니다. pointer/AX click 계약을 blanket keyboard 정책으로 바꾸지 않았습니다.

- [x] `tooltips::icon_tests::disabled_icon_span의_enter_space는_click_없이_tooltip을_닫지_않는다 --exact`: disabled/enabled×Enter/Space 실제 wrapper의4조합입니다. RED compile7.56초/suite0.04초, 수정 뒤 1 PASS compile7.95초/suite0.04초입니다.
- [x] 실제 Theme icon·dark/light·disabled focus/AX/hover/button child의 기존 `icon_wrapper_tooltip은_theme_버튼자식과_disabled_span_focus를_보존한다 --exact`: 1 PASS,suite0.08초입니다. 앞 절의 clickable Trigger 성공은 true 분기와 동일하며 반복하지 않습니다.

최종 `cargo clippy … -p taide-native-app --lib --tests -- -D warnings`는 exit0,18.05초이며 대상 Rust4 exact rustfmt `--check`도 exit0입니다. 이 후속은 앞 절 strict19.43초 뒤에 생산 코드가 달라진 독립 변경이므로 해당 정적 검사를 한 번 실행했습니다. 총 신규4·영향14의 성공 근거가 있으며 전체 M8 검증으로 세지 않습니다.

## 남은 경계

- [ ] disabled/enabled가 같은 raw batch 중 전환되는 current topology와 새 child focus alias의 중간 수명은 전체 graph에서 검증해야 합니다. 위 고정4조합으로 전환 전체를 통과했다고 주장하지 않습니다.
- [ ] 제품 버튼의 기본 Space action은 pinned egui에서 keydown fake click입니다. Tooltip의 paired release와 별개이며 Problems의 전용 release 정책을 일반 버튼에 무단 확장하지 않았습니다.
- [ ] Tooltip 처리 뒤 App keymap가 늦게 소비한 키, 임의로 재작성한 Event와 동일한 반복 Event 중 일부만 제거한 경우, raw 중간/new/disabled/modal topology·Tab·전체 source graph·GUI/DPI는 미완료입니다. 현재 surviving subsequence는 source-index 계약의 전체 대체물이 아닙니다.

후속46/N1~N8은 미완료이며 목표 active입니다. 이번 작업은 제품 TS/vendor/dependency/manifest/lock/MSRV/보호 bundle·사용자 앱·OS·IME/VoiceOver·clipboard/Keychain·Git을 변경하지 않았습니다. 전체 M8 완료 전 commit/push는 하지 않으며 종료 시 live command는 없습니다.
