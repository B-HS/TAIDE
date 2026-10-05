# M8 공용 native Button 기본 키

## 대상·근거

native app의 기존 egui path patch `vendor/egui-input/src/{context.rs,pass_state.rs,widgets/button.rs,containers/menu.rs}`, app `src/{button-key-tests.rs,lib.rs,tooltips.rs,tooltip-key-tests.rs,problems-tests.rs}`, source 계측 도구/tsconfig입니다. main이 workflow·서브에이전트 없이 직접 수행했습니다. PROCESS·verify·save-docs 스킬과 설치된 ai-process/desktop/common/comments 기준을 유지하며 제품 TS view를 수정하지 않았습니다.

기존 `Context.get_response`는 클릭 가능한 Sense의 Space/Enter keydown을 모두 fake primary click으로 처리했습니다. 실제 HTML 버튼과 custom row/menu의 기본 동작은 같은 경계가 아닙니다. [WAI Button 패턴](https://www.w3.org/WAI/ARIA/apg/patterns/button/)과 [Playwright Keyboard down/up/repeat](https://playwright.dev/docs/api/class-keyboard#keyboard-down), 실제 pinned Button/AtomLayout/Context/InputState와 원본 Tooltip을 확인했습니다. Button docs.rs 두 URL은 조회 오류여서 성공 공식 웹 문서로 세지 않고 설치된 public API doc/구현을 사용했습니다.

## 실제 원본 측정

기존 성공 build `/private/tmp/taide-tooltip-events.lDAI8X/built`의 실제 shared Tooltip/HTML Button을 그대로 사용했습니다. 새 build·의존성은 추가하지 않았습니다. `tools/m8-button-default-key-source-measure.ts`는 keydown/repeat/keyup·click·blur를 버튼 attribute에 기록하고 cancel 사례에서만 preventDefault를 호출합니다. 각 사례는 독립 BrowserContext이며 외부 요청 차단·service worker 차단·임시 Chrome profile/mock Keychain을 사용하고 전부 닫았습니다. CSS animation pause나 타이밍/픽셀 측정은 없습니다.

초기 sandbox Chrome 실행은 SIGABRT/EPERM으로 실패했습니다. 합성 fixture만 범위로 승격한 재실행은 exit0,0.7928초입니다. 성공 측정은 한 번이며 raw [source JSON](assets/2026-10-05-button-default-keys-source.json)을 그대로 저장했습니다.

| 사례 | keydown 누적 click | repeat 누적 click | keyup 누적 click |
| --- | --- | --- | --- |
| Space | 0 | 0 | 1 |
| keydown 취소 | 0 | 0 | 0 |
| keyup 취소 | 0 | 0 | 0 |
| 다른 버튼으로 blur | 0 | 0 | 0 |
| Enter | 1 | 2 | 2 |

Space keyup의 실제 click 뒤 Tooltip state는 closed이고, 취소 두 경우는 instant-open입니다. blur도 Tooltip은 닫지만 첫 버튼 click은 없습니다. 실제 DOM5를 native 모든 화면/모든 OS의 실기 증거로 확대하지 않습니다.

## 구현

Button만 등록하는 current-pass keyboard 결과 cache와 viewport별 Space arm을 추가했습니다. 결과는 실제 Button Response 생성과 paint/style 조회에서 공유하며, pointer-down 상태로 키를 위조하지 않습니다. 일반 Button은 Space 최초 press에 arm하고 repeat에서 click하지 않으며 짝 있는 release에 click합니다. normalized input에서 소비된 press/release, disabled·focus 이탈·window blur에서는 활성화하지 않고 unseen Button owner는 end_pass에서 제거합니다. same-pass AX Focus는 이전 focus에서 사건 순서대로 배정하므로 UI draw 순서와 최종 focus에 클릭이 몰리지 않습니다.

generic click Sense는 기존 press 정책입니다. MenuButton/SubMenuButton은 `space_on_release(false)`를 사용하고 UiKind::Menu 내부도 기존 정책을 유지합니다. MenuBar의 UiKind::Menu 분류를 포함한 정책이며 모든 Radix menu topology를 새로 측정했다는 뜻은 아닙니다. ComboBox는 Button이 아니라 자체 click Sense라 이번 일반 Button 변경에 포함하지 않습니다. Problems의 명시적 Press/Release activation vector도 유지합니다.

Tooltip은 실제 Button keyboard activation을 current-pass typed 읽기로 받습니다. 따라서 실제 click이 이미 끝난 뒤 소비자가 input을 제거해도 정상 close되고, 기본 action 전에 취소된 입력과 구별합니다. wrapper의 활성 child focus alias도 같은 경계를 사용합니다. generic Trigger는 직전 raw owner/generation·normalized survivor 모델을 유지합니다. 새로운 Context.data 순환 소유권·dependency/MSRV·unsafe/OS/IPC 변경은 없습니다.

## 검증·실패 정정

공통 Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`이며 직렬 실행했습니다.

- [x] `cargo test … --lib button_key_tests`: 최초2 RED compile7.34초/suite0.02초입니다. 일반 Button Space keydown click과 cancel-up 시 press click을 재현했습니다. 엔진 수정 뒤2 PASS compile20.05초/suite0.03초입니다. Menu/typed read 후속에서 같은 ordinary Button 분기의 성공은 재사용하며 반복하지 않았습니다.
- [x] 후속 `cargo test … --lib --no-run` compile10.79초 뒤 실제 lib binary의 `--exact` 여러 필터 batch11은10 PASS·1 FAIL,suite0.66초입니다. 신규 Enter/repeat/AX/generic/menu1·unmount same-ID1·actual Button/alias Tooltip completed-click1과 영향7은 PASS입니다. 실패1은 오래된 Problems Tooltip fixture입니다.
- [x] Problems fixture가 App owner의 begin/finish를 호출하지 않아 최초 Focus의 Tooltip role이 없었습니다. production처럼 begin/finish를 추가한 뒤 최초 Focus·Escape·blur는 통과했지만 mixed 최종 warning open 기대가 실패했습니다(compile5.97초/suite0.04초). 기존 원본 focus JSON의 빠른 재진입 open→self-close와 설치된 Content listener를 대조해 own closing Presence 중 재진입 기대를 closed로 정정했습니다. 제품 계약을 완화하지 않았으며 같은 source 성공을 재측정하지 않았습니다.
- [x] 추가 actual Button AX Focus8조합과 정정한 Problems 영향1은 compile7.51초 뒤 exact binary2 PASS,suite0.04초입니다. source code상 첫 Enter는 이전 Button, Focus 뒤 Enter는 다음 Button, Space press 중 Focus 이탈은 release를 새 Button에 오배정하지 않습니다. actual native draw 정/역 순서를 확인하되 App action의 동적 UI 변경까지 완료로 세지 않습니다.
- [x] 전체 고유 신규6·영향8의 성공 근거입니다. 영향8은 Problems header release/repeat·custom row batch·header batch·Tooltip focus, disabled span·Theme icon, actual Keybinding modal actions·Tooltip입니다. batch 중 성공한7은 마지막 실패 정정 뒤 반복하지 않았습니다.
- [x] `cargo clippy … -p egui -p taide-native-app --lib --tests -- -D warnings`: exit0,23.30초입니다. Wry 기존17 dependency 경고는 숨기지 않았습니다. 전체 bin/root workspace 실행 gate가 아닙니다.
- [x] 계측 도구 `bunx --no-install tsc --noEmit -p experiments/native-tooltip-reference/tsconfig.json`: exit0,0.643초입니다. 기존 도구/tsconfig Prettier 적용은 unchanged였습니다. 정확한 Rust8 `rustfmt --check`와 `git diff --check`는 exit0입니다. source JSON과 문서는 최종 검사를 아래 PROCESS 상태에 기록합니다.

`diff -qr`의 exit1은 의도된 vendor 차이 보고입니다. 공통116 중106 동일/10 변경·추가 MIT이며 새 UPSTREAM provenance를 남겼습니다. native source registry .cargo_vcs_info의 commit/license를 확인했고 Cargo graph/lock/MSRV는 변경하지 않았습니다. authored Rust99%에서 vendor를 제외합니다.

없는 atom_layout.rs/menu.rs/tools tsconfig 및 미매칭 셸 glob 조회는 실제 atomics/atom_layout.rs·containers/menu.rs·reference tsconfig와 디렉터리 rg로 정정했습니다. 조회 실패를 제품 RED나 성공으로 세지 않습니다.

## 다음 경계

- [ ] App capture/default action의 전체 우선순위입니다. 후속 `2026-10-05-m8-button-window-capture.md`에서 알려진 일반 Button의 Enter/Space same-pass/split-pass를 draw 전 실제 router로 수정했습니다. 혼합 focus/Tab/Escape/IME·동적 owner와 window→document→target 전체 순서는 남아 있어 부모 전체 완료로 세지 않습니다. 이미 완료된 click 뒤 처리된 입력과 아직 기본 action을 막아야 할 취소는 구별해야 합니다.
- [ ] 같은 pass 여러 Space/Enter default clicks의 전체 action 개수, 실제 mouse/AX/focus·새/삭제/disabled owner의 중간 topology, menu/item/select/checkbox의 전체 source 계약입니다. 현재 Response.clicked의 bool 결과를 원본 여러 click 전부의 대체로 주장하지 않습니다.
- [ ] 중간 window blur/re-focus·IME·여러 실제 viewport·전체 App/auxiliary/AX/pixel/OS gate입니다. 이번 Space 상태는 viewport별 보관하지만 모든 viewport 실기를 새로 검증하지 않았습니다.

후속46/N1~N8은 미완료이고 목표 active입니다. 제품 TS·root/Tauri·dependency/manifest/lock/MSRV·보호 bundle·사용자 앱/OS 설정·IME/VoiceOver·clipboard/Keychain·Git은 이번 작업에서 불변입니다. 전체 M8 완료 전 commit/push는 하지 않으며 live command는 없습니다.
