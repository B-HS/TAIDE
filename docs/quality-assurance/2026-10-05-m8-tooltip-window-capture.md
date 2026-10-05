# M8 window/document Tooltip 캡처와 Button Escape 포커스

## 대상·계약

대상은 native app의 `application.rs`, `terminal_surface.rs`, `keymap.rs`, `tooltips.rs`와 actual Button/Tooltip 테스트, pinned engine의 `context.rs`, `memory/mod.rs` 및 기존 Button source 도구입니다. 기존 후속46 안에서 main이 workflow·서브에이전트 없이 진행했고 PROCESS·verify·save-docs 스킬을 적용했습니다. 프로젝트에 없는 convention ai-process는 설치된 전문으로 대체했습니다.

원본 `use-keydown-capture.ts`의 window capture가 `use-global-keymap.ts`의 preventDefault/stopPropagation을 수행한 뒤 document Tooltip capture와 target 처리로 내려갑니다. [DOM dispatch](https://dom.spec.whatwg.org/#concept-event-dispatch), [WAI Button](https://www.w3.org/WAI/ARIA/apg/patterns/button/), 실제 pinned Focus/Context 코드를 확인했습니다. target 단계의 소비는 이미 실행된 document capture를 되돌리지 않습니다.

## 새 원본 측정

`bun tools/m8-button-default-key-source-measure.ts --escape-only`는 기존 성공 events build의 실제 shared Tooltip/HTML Button으로 Escape와 window capture 취소2사례만 측정했습니다. 이전 Enter/Space5사례를 반복하지 않았습니다. 임시 Chrome profile/mock Keychain·외부 요청 차단·독립 BrowserContext를 사용했고 종료했습니다. source window 취소는 preventDefault/stopPropagation을 호출하는 합성 capture listener이며 실제 전체 App shortcut handler/UI 변경까지 계측한 것은 아닙니다.

승격된 범위 한정 호출 exit0,0.580초입니다. [실제 JSON](assets/2026-10-05-button-escape-source.json)을 stdout 그대로 저장했습니다. 일반 Escape keydown/repeat/keyup에서 click0·Tooltip closed·Button focused=true입니다. window capture 취소에서는 click0·Tooltip instant-open·Button focused=true이며 keydown이 target listener에 도달하지 않았습니다. 새 build/CSS pause/픽셀 타이밍/OS 입력기 변경은 없습니다.

## 수정

- [x] 실제 App이 일반 Button의 window capture를 document Provider.begin_frame보다 먼저 실행합니다. 선행 경로는 현재 Enter/Space/Escape의 안정된 등록 Button batch이며 pointer/Touch/AX/window-focus/IME·그 외 key는 사건별 owner 통합이 남아 있습니다.
- [x] Provider는 시작 시 normalized survivor와 raw index의 대응을 저장하고, 선행 캡처에서 제거된 raw Escape를 다시 처리하지 않습니다. document capture가 소비한 raw index를 current normalized position으로 잘못 사용하는 것도 수정했습니다. 앞의 Enter가 제거돼도 Escape만 제거하고 뒤의 Space keyup은 보존합니다.
- [x] 기존 keymap raw index comparator를 `raw_event_index`로 추출해 공유했습니다. key의 physical/key/pressed/modifiers와 순서는 보존하며 egui가 정규화한 repeat만 기존 keymap처럼 비교에서 제외합니다. Tooltip의 후속 key survivor도 같은 comparator를 사용합니다. arbitrary duplicate-event identity 문제 전체를 해결했다고 주장하지 않습니다.
- [x] engine의 Focus.begin_pass가 UI/capture 전에 ordinary Button의 Escape focus를 해제하는 원인을 수정했습니다. 현재 실제 viewport의 previous Button 등록과 Focus ID를 연결해 ordinary Button만 유지하며 generic/custom/menu 정책은 보존합니다. 기존 public focus-lock API로 늦게 focus를 돌려놓거나 callback을 사후 취소하지 않습니다. 현재 viewport의 Focus를 직접 읽어 이전 부모 viewport의 같은 ID와 혼동하지 않습니다.

## 검증

Cargo는 기존 CARGO_HOME, manifest `native/taide-native-app/Cargo.toml`, `--locked --offline --target-dir experiments/native-shell-spike/target`로 직렬 실행했습니다.

- [x] `tooltip_document_capture는_button_window_escape_취소를_존중하고_target_소비와_구분한다`의 초기 window 취소 RED는 compile7.45초/suite0.03초입니다. 순서·normalized map만 수정한 뒤에도 compile8.08초/suite0.03초로 실패했고 engine의 선행 Escape focus 해제를 확인했습니다. 두 실패의 실제 관찰과 최종 focus 원인을 구분합니다.
- [x] 최종 lib `--no-run` compile11.48초 뒤 exact binary9는2신규/7영향 PASS,suite0.41초입니다. 신규는 window/unbound/target/prefix4조합과 첫 mount focus 다음 Escape에서 ordinary/generic2조합입니다. window 취소는 Tooltip을 유지하고 target 소비는 유지하지 않으며 앞선 전역 Enter를 제거한 뒤 Escape와 이웃 keyup을 정확히 구분합니다.
- [x] 영향7은 직전 ordinary Button 전역 default/capture gate, source ordered Focus/Escape, 완료 Button click 뒤 소비, later Modal Escape, actual Keybinding modal Tooltip, window raw decision 공유입니다. 이어 실제 Explorer 오류 input Escape1 PASS,suite0.03초와 Enter/Space 소비의 begin 전후1 PASS,suite0.01초입니다. 최종 신규2/영향9 성공이며 성공한 입력은 다시 실행하지 않았습니다.
- [x] engine/app strict `cargo clippy … -p egui -p taide-native-app --lib --tests -- -D warnings`에서 engine의 기존 lost-focus fixture4호출이 새 내부 인자를 누락해 E0061이 났습니다. 기존 generic 정책의 `None`을 명시하고 해당 정적 검사만 재실행해 exit0,21.04초입니다. 이미 성공한 app 테스트는 engine fixture 수정 뒤 반복하지 않았습니다. 기존 Wry dependency17경고는 숨기지 않았습니다.
- [x] 도구 TS는 기존 reference tsconfig로 `bunx --no-install tsc --noEmit` exit0입니다. Prettier write unchanged/check exit0, Rust8 exact fmt와 fixture 변경 Memory fmt exit0, `git diff --check` exit0입니다. 두 apply_patch context 불일치는 원자적으로 실패해 파일 변경이 없었고 실제 형식을 읽어 정정했습니다. nonexistent input/event 경로는 실제 input/ime_event·Memory/Context 코드로 확인했습니다.

vendor upstream 비교는 공통116 중106byte 동일/10변경·MIT/UPSTREAM 유지입니다. diff -qr exit1은 이 의도된 차이 보고이며 새 파일 수나 license 제거가 아닙니다. authored Rust99%에서 vendor는 제외합니다.

## 미완료

- [ ] 같은-pass pointer/AX/Tab/IME·중간 blur/refocus·동적 신규/삭제/disabled owner별 window→document→target/default action 순서입니다. 현재 마지막 Focus 하나로 전체 mixed batch를 라우팅하지 않습니다.
- [ ] 일반 Button의 Tab/arrow·generic/menu/select/checkbox의 전체 source graph, 여러 실제 default clicks 및 keymap action 후 중간 UI topology입니다.
- [ ] 실제 App/auxiliary/viewport/DPI/픽셀·CJK/VoiceOver와 M8 N1~N8 전체 gate입니다.

제품TS/root/Tauri/dependency/manifest/lock/MSRV/보호bundle·사용자 앱/OS 설정/clipboard/Keychain/Git은 불변입니다. 후속46/N1~N8 0/8·목표active이며 전체 완료 전 commit/push는 하지 않습니다. live command는 없습니다.
