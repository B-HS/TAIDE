# M8 공통 keymap 결정기와 native terminal 연결

## 대상·원본 근거

- `native/taide-native-app/src/keymap.rs`, `keymap-defaults.json`, `lib.rs`, `terminal_surface.rs`, `tests/terminal-host.rs`가 대상입니다. PROCESS N4-C의 공통 keymap 결정 경로 안에서 workflow·서브에이전트 없이 main이 직접 구현했습니다.
- 원본 `src/shared/lib/keymap/keymap.ts`의 APP_KEYMAP 41개를 Bun으로 실제 export해 같은 순서·id·key·mods·when·chord·descriptionKey의 JSON 카탈로그로 저장했습니다. 제품 TS를 수정하거나 Rust 실행 때 TS를 읽도록 연결하지 않았습니다.
- 원본 `keymap-dispatch.ts`, `keymap-chord-store.ts`, `keymap-context.ts`, `command-binding-dispatch.ts`, `src/shared/hooks/use-global-keymap.ts`의 결정/부수 효과와 기존 `Settings.keymap_overrides`를 기준으로 합니다. handler 없는 단일 dispatch는 삼키지 않지만 chord 단계 2는 handler 유무와 무관하게 삼키는 차이를 별도 Decision으로 유지합니다.
- 설치된 egui/egui-winit 0.36.2의 Context.run_ui, RawInput.take, Key.name/symbol_or_name, on_keyboard_input source를 확인했습니다. run_ui 재처리 pass에는 RawInput.take가 이벤트를 다시 전달하지 않습니다. egui-winit의 일반 문자 입력은 Key 뒤 Text로 전달되며 Cmd/Ctrl 조합의 Text는 생성하지 않습니다.

## 좁은 구현 완료

- [x] 41개 기본 카탈로그 순서와 정확한 네 modifier 비교, clean logical 키와 DOM physical code fallback, terminal/editor scope를 연결했습니다. 플랫폼 mod는 macOS Meta/그 외 Ctrl로 해석합니다. 현재 카탈로그에서 사용하는 when은 닫힌 predicate로 구현하며 임의 ContextKeyExpr parser를 완료했다고 주장하지 않습니다.
- [x] actionId별 첫 유효 override, `keybindings.open` alias, chord의 완전 재지정/제거, 첫 단계 변경 시 원본 chord when 제거를 연결했습니다. 이 변경 비교는 raw mods 배열 길이·membership과 lowercased key를 사용하며 매칭용 canonical key로 대신하지 않습니다. 잘못된 JSON/배열은 기본 카탈로그로 돌아갑니다.
- [x] prefix를 공유하는 모든 후보, repeat/단독 modifier/IME 비명령 입력의 대기 보존, 두 번째 단계의 matched/no-match 삼킴, 5초 만료, editor chord 유예를 구현했습니다. 시간은 결정기 인수 Instant로 주입하므로 실시간 5초를 기다리지 않고 4999/5000ms 경계를 검사합니다. Cmd/Ctrl mid-IME는 원본처럼 결정기에 들어갑니다.
- [x] native terminal의 직접 Mod+Up/Down 비교를 같은 resolver의 이전/다음 command action으로 교체했습니다. Settings 변경을 캐시하며 Views 내부 viewport별 대기 상태를 유지하고 닫힌 viewport/shutdown에서 회수합니다. 실제 focused/running/enabled terminal에서만 실행합니다. 전체 앱의 공통 capture 경로로 확장한 상태는 아닙니다.
- [x] 소비한 bare Key 바로 다음의 같은 raw frame/index와 같은 egui logical 키 Text는 PTY로 보내지 않습니다. IME Commit/Paste나 다른 event를 임의로 지우지 않습니다. 합성 Key J/Text j, chord L/Text l와 no-match Q/Text q에 대해 실제 PTY epoch 불변을 확인했습니다. 이 좁은 연결을 raw Unicode/dead-key provenance 전체 해결로 확대하지 않습니다.

## 실행 증거와 실패 정정

Cargo는 기존 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`으로 직렬 실행했습니다. manifest는 `native/taide-native-app/Cargo.toml`입니다.

1. `cargo test ... --lib keymap_`: 변경된 결정기 3 PASS, 새 egui adapter 1 FAIL입니다. compile 2.51초·suite 0.00초입니다. 카탈로그/첫 override/충돌 우선순위/legacy/기본 복원, chord 후보·IME·repeat·scope·timeout·editor 유예, physical/정확한 modifier 검사는 통과했습니다.
2. adapter 실패는 egui Key::Minus.symbol_or_name가 ASCII `-`가 아니라 표시용 `−`를 반환하는 실제 경계 차이였습니다. 구현에서 Minus/Quote를 각각 DOM `-`/`'`로 매핑하고 `--lib keymap_egui_adapter`만 다시 실행해 1 PASS(compile 1.35초·suite 0.00초)했습니다. 이미 성공한 결정기 3건은 재실행하지 않았습니다.
3. `cargo test ... --test terminal-host terminal_commands`: 실제 합성 PTY/40개 OSC133 명령·Core·egui Views 신규 영향 1 PASS(compile 9.87초·suite 0.60초)입니다. 기본 Up/Down·gutter·disabled에 custom bare J, 공통 prefix의 sibling L, unmatched Q를 추가했습니다. 실제 2-pass run_ui에서도 단일 이동이고 input epoch 불변입니다. 마지막 continue 뒤 exit Some(0), dispatch join, Hub close, fixture task 회수를 확인했습니다.
4. `cargo clippy ... --lib --test terminal-host -- -D warnings`: exit 0, 2.62초입니다. inherited Wry 17 경고는 authored strict 통과와 구분합니다. suppression·새 의존성·unsafe를 추가하지 않았습니다.

이전 ruler/Core/parser/feature-gate/URL/file 링크의 성공은 해당 QA로 재사용합니다. 같은 상태의 성공 검사를 세 번 반복하지 않습니다. 보호 실기 bundle·OS clipboard/browser/input method/VoiceOver·사용자 프로젝트·제품 TS/root manifest/MSRV는 변경하거나 조작하지 않았습니다.

원본 APP_KEYMAP export와 native JSON을 Bun의 실제 JSON.stringify로 대조해 41개 전체의 순서·속성이 정확히 같음을 확인했습니다(exit 0). authored Rust 4파일 exact rustfmt check, JSON/이 QA Prettier check, tracked diff check도 exit 0입니다. keymap/JSON/lib/surface/terminal-host/이 QA의 명시된 untracked 6개 no-index whitespace 검사 출력은 모두 비어 있습니다(exit 1은 /dev/null과 내용 차이). 검사 뒤 live Cargo handle은 없습니다.

## 미완료 경계

- [ ] 공통 capture/handler registry와 41개 실제 action, command palette/quick open/설정 편집 UI, editor binding·Monaco command 대응, chord 표시/no-match 알림, shell slot 및 auxiliary window scope를 연결합니다. 현재 결정기의 action id 반환을 전체 앱 action 실행으로 계산하지 않습니다. terminal에서 다른 단일 action이 먼저 매칭되면 terminal jump로 우회하지 않지만 그 action의 실제 앱 UI는 아직 미연결입니다.
- [ ] egui-winit은 원본 논리 Unicode를 Key enum으로 축약하거나 physical fallback합니다. bare Unicode/dead-key/여러 codepoint의 실제 Key→Text provenance, semantic Copy/Cut/Paste가 raw Key보다 먼저 변환되는 조합, non-macOS Super modifier와 IME/platform 순서를 전체 raw input gate에서 보존해야 합니다. 현재 인접·동일 logical Text 규칙과 순수 physical 모델 검사는 이 경계를 해결했다는 증거가 아닙니다.
- [ ] 후속 window-keymap QA에서 viewport/frame/raw index의 공통 Decision과 terminal/editor/global의 7개 기존 shell action 연결을 구현·검사했습니다. 이 문서의 이전 terminal-only 증거와 구분합니다. 전체 capture ordering·다른 palette/webview/input으로의 focus·settings 변경 중 pending/owner·aux/전체 41개 handler는 계속 미완료이며 전체 다중 창 수명 통과로 주장하지 않습니다.
- [ ] override JSON/cache의 바이트·retained aggregate·parse lock/CPU budget과 초대형 호환성은 전체 성능/보안 gate에서 구현·측정합니다. malformed entry에 대해서는 Rust 경계의 key string/mods array 검증을 적용하며 원본의 잘못된 shape가 낼 수 있는 런타임 오류를 복제하지 않습니다.
- [ ] 재표시/replay와 단일 Core 계약의 A/B 질문은 응답 대기입니다. 결정하지 않았으며 추가 VT parser/Core reset을 구현하지 않았습니다. 전체 M8 N1~N8 0/8, TS 제거·cutover·IME/AX/GUI·성능/배포는 미완료이고 전체 완료 후에만 commit/push합니다.
