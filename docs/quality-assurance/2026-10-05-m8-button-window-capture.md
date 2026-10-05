# M8 일반 Button 전역 키맵 캡처 순서

## 대상·범위

`native/taide-native-app/src/{application.rs,terminal_surface.rs,button-key-tests.rs}`와 native 한정 pinned egui `vendor/egui-input/src/context.rs`입니다. main이 workflow·서브에이전트 없이 기존 후속46 안에서 직접 수행했습니다. PROCESS·verify·save-docs 스킬을 사용하며 프로젝트에 없는 `docs/convention/ai-process.md` 대신 설치된 ai-process/desktop 전문을 사용했습니다.

원본 `src/shared/hooks/use-keydown-capture.ts`는 window keydown capture이고 `use-global-keymap.ts`의 실제 dispatch는 preventDefault/stopPropagation 뒤 handler를 호출합니다. [DOM dispatch](https://dom.spec.whatwg.org/#concept-event-dispatch)의 capture 순서와 취소된 activation 경계, [HTML activation](https://html.spec.whatwg.org/multipage/interaction.html#activation-behavior)를 확인했습니다. 원본 기본 키 DOM5 자료와 기존 build 성공을 재사용했고 새 브라우저 계측은 하지 않았습니다.

기존 NativeApplication은 shell/Button을 그린 뒤 전역 라우팅을 했습니다. Button의 Enter click이 이미 발생하거나 Space가 이미 armed된 뒤 press를 소비하므로, 전역 액션과 Button 액션이 중복될 수 있었습니다.

## 구현

- [x] `Context::focused_button_has_default_keys`는 현재 viewport의 실제 focused ID와 직전 pass의 일반 Button 등록을 읽습니다. 이전 등록 없이 임의의 generic click Sense를 Button으로 간주하지 않습니다. engine 동작을 변경하거나 Context.data에 Context/Response를 보관하지 않습니다.
- [x] 실제 `Views::capture_button_keymap`을 NativeApplication의 shell draw 전에 연결했습니다. 기존 busy/종료/확인창/키캡처/OS focus gate를 유지합니다. known ordinary Button의 Enter/Space batch에서만 non-editor/non-terminal scope로 전역 라우팅을 먼저 적용합니다. 취소된 press가 Button에 도착하지 않아 같은 pass default click과 다음 pass Space release가 실행되지 않습니다.
- [x] 기존 늦은 window 루프를 `route_window_keys`로 추출해 같은 구현을 두 위치에서 공유합니다. raw index·normalized remaining 순서·action 지원/focused-shell gate·오류의 event 제거/최종 오류·keyup 및 인접 Text 처리 정책은 유지합니다. 실제 editor/terminal 내부 라우팅과 modal 재바인딩 캡처는 변경하지 않았습니다.
- [x] pointer/Touch/AX/window-focus/IME 또는 Enter·Space 이외 key가 섞인 batch는 선행 경로에 넣지 않습니다. 하나의 최종 focus로 여러 사건의 scope를 추측하지 않습니다. 이 분기는 기존 처리에 남으며 전체 source 순서가 완성됐다는 뜻이 아닙니다.

## 검증과 정정

Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, manifest `native/taide-native-app/Cargo.toml`, `--locked --offline --target-dir experiments/native-shell-spike/target`이며 직렬 실행했습니다.

- [x] 테스트는 실제 Context/Button과 production window router를 사용합니다. 최초 override를 object로 작성한 두 실행(compile7.23/12.37초)과 array의 `id`로 작성한 실행(7.04초)은 실제 parser 계약인 array/actionId와 달랐으므로 취소 재현 근거에서 제외합니다. 같은 Enter 실패 문구만으로 제품 RED라고 세지 않습니다.
- [x] 올바른 array/actionId 뒤 기존 post-draw adapter는 Enter canceled press에서 RED(compile2.84초/suite0.02초)입니다. action 개수 선행 assertion을 추가한 기존순서 RED(7.23초/0.02초)로 전역 액션도 실제 실행됐음을 확인했습니다. 후자는 baseline 재현의 추가 실행이며 성공 검증이나 별도 제품 버그로 세지 않습니다.
- [x] 선행 capture adapter로 바꾼 `app_window_keymap은_button_default_click보다_먼저_소비한다`는 Enter/Space×same-pass/split-pass4조합 PASS입니다. Space press를 소비한 뒤 override를 제거하고 release해도 클릭하지 않으며 전역 액션은 한 번입니다.
- [x] `app_button_capture는_다른_focus_사건과_generic_target을_선점하지_않는다`는 generic/unbound/pointer/AX/blur/Tab/Escape/IME8조합 PASS입니다. 이벤트 순서와 액션이 선행 경로에서 변하지 않습니다. deprecated ImeEvent::Enabled fixture를 수정하면서 Preedit의 tuple 추측이 E0533을 냈고 실제 struct variant `text/active_range_chars`로 정정했습니다. 최종 exact 검사 compile4.14초/suite0.06초,1 PASS입니다.
- [x] 첫 성공 build는 compile7.68초였습니다. 이후 IME fixture compile 실패를 확인한 호출에서 실행한 existing lib binary batch5는2신규/3영향 PASS,suite0.05초입니다. 이 binary는 현재 Preedit fixture를 포함하지 않은 이전 성공 build이므로 최종 전체 동일 상태 batch로 주장하지 않습니다. 변경 없는 첫 신규1과 영향3(window raw cache·actual editor 기본 chord·editor override/IME/cache)의 성공은 재사용하고, 변경한 IME fixture를 포함한 신규1만 위 exact 명령으로 재검증했습니다. 신규2/영향3의 최종 관련 성공 근거입니다.
- [x] 최종 `cargo clippy … -p egui -p taide-native-app --lib --tests -- -D warnings` exit0,21.19초입니다. 기존 Wry dependency17경고를 숨기지 않았습니다. Rust4 exact fmt check exit0입니다. 전체 bin/root/GUI 검사가 아닙니다.

## 남은 경계

- [ ] 전역 window capture→document Tooltip/Modal capture→현재 target/default action의 전체 순서입니다. Escape 등은 이번 선행 경로 밖이며 Tooltip raw replay보다 먼저 차단돼야 할 사건도 남습니다.
    - 후속 `2026-10-05-m8-tooltip-window-capture.md`에서 App 순서·normalized survivor/raw index·engine Button focus를 수정했습니다. ordinary Enter/Space/Escape는 성공했고 mixed focus/Tab/IME·동적 owner의 전체 순서는 남습니다. 위 최초 Enter/Space-only와 Escape guard8조합 기록은 해당 당시 상태이며 최신 guard는 Escape를 선행 처리합니다.
- [ ] pointer/Touch/AX/Tab/IME·중간 blur/refocus 및 여러 default actions에서 사건마다 실제 owner와 중간 UI topology를 사용한 라우팅입니다. 현재 mixed batch를 건너뛴 것은 source parity의 완료나 허용된 영구 차이가 아닙니다.
- [ ] 새/삭제/disabled control·generic row/menu/select/checkbox·새 viewport/actual App GUI입니다. 이전-pass Button 등록 API는 최초 mount의 임의 focus를 일반 Button으로 판정하지 않습니다.

vendor 공통116 중106동일/10변경과 MIT/upstream 고지는 유지되며 authored99%에서 제외합니다. 제품TS/root/Tauri/dependency/manifest/lock/MSRV/보호bundle/OS 설정/clipboard/Keychain/Git은 불변입니다. N1~N8 0/8·후속46 미완료·목표active이며 전체 M8 완료 전 commit/push는 하지 않습니다.
