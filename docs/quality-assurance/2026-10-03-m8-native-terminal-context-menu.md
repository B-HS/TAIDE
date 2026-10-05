# M8 native terminal 컨텍스트 메뉴

## 2026-10-05 owner 탐색 뒤 선택·닫힘 이후 입력

대상은 actual `src/terminal_surface.rs`·`tests/terminal-host.rs`·기존 measurement tool입니다. PROCESS·verify·save-docs의 위험 비례 검사/분류 기록으로 main이 직접 수행했습니다. engine/vendor·제품 TS·의존성/lock/MSRV·보호 bundle/cache·OS/사용자 데이터·Git은 변경하지 않았습니다.

source Content owner의 End는 focusFirst로 동기 Kill focus를 만들고 다음 Enter는 Kill을 선택합니다. 실제 기존 build에서 같은 JS task `End→Enter→t`를 1회 측정했습니다. action:kill1회/메뉴 closed 요청 뒤에도 Presence가 DOM을 잠시 유지해 후행 t는 터미널 data로 전송되지 않습니다. menu detach/restore-focus/실제 textarea focus 및 Focus1004 gain 뒤 trusted tinue만 전송됩니다. Kill callback은 합성 기록이므로 실제 원본 terminal 종료의 증거는 아닙니다. 키는 trusted=false인 합성 task이고 물리 OS frame/픽셀 timing 동등의 증거도 아닙니다.

native actual owner의 End→Enter는 버튼 그리기 뒤 탐색만 갱신해 선택/닫힘이 없었습니다. actual AX/owned PTY RED(compile4.51초/suite0.38초/exit101)로 재현했습니다. navigate가 raw 사건 당시 현재 enabled item ID의 Enter/Space 선택을 반환하고 실제 root/child Response ID→기존 MenuAction으로 매핑하도록 연결했습니다. root의 동기 owner 이동·item/search의 지연 이동·default 버튼/typed host command는 유지하며 fake Response나 이전 frame ID 예측은 사용하지 않습니다. 메뉴 활성 batch는 기존 terminal admission이 차단하므로 닫힘 뒤 후행 문자는 같은 batch에서 PTY로 새지 않습니다.

fixture 초기 E0277(MenuOperation Debug 없음)·E0433(HostCommand 미import)은 작성 오류이며 matches!/실제 host::HostCommand 경로로 고쳤습니다. 축약된 초기 출력에서 E0433을 놓쳐 관련 compile을 한 번 더 실패했고 제품 RED로 계산하지 않습니다. 중간 controller 경로 추정은 실제 파일 조회로 정정했으며 그 경로로 Cargo를 실행하지 않았습니다. 첫 수정 뒤 실제 선택/닫힘 assertion은 통과했지만 fixture가 이미 닫힌 메뉴에 Escape를 추가로 보내 PTY 종료1을 만들었습니다(12.42초/0.42초). source에는 이 Escape가 없으므로 owner 선택 branch만 빈 frame으로 바꾸고 원래 Escape 시나리오는 유지했습니다. 예상 wire는 기존 fixture `MENU_FOCUS_INPUT`의 con/loss/gain/tinue입니다. 이를 제품 문자 누출 bug로 주장하지 않습니다.

```sh
bun tools/m8-terminal-menu-source-measure.ts /private/tmp/taide-terminal-menu.Ppxh7N /private/tmp/taide-tooltip-source.J9DufD/built/assets/index-DEfYI-e9.css --reuse-build --owner-select-close
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu는_owner_end_enter_선택과_같은_batch_닫힘_뒤_문자_차단을_보존한다 -- --exact
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/terminal_host-3226916b98261227 terminal_menu_actions는_enter_space_keydown의_disabled_copy_select_clear_paste를_연결한다 terminal_menu는_실제_keyboard_split_new_kill을_host_worker에_전달한다 terminal_menu는_같은_batch_첫_검색문자_뒤_space의_leaf_선택을_억제한다 --exact
```

- [x] source1 exit0/0.78초·`menu-owner-select-close.json`입니다. 기존 동일 CSS SHA256/Chrome154.0.8037.95·격리 origin/mock Keychain·context/browser finally 회수이며 build/이전 source scenario는 반복하지 않았습니다.
- [x] 신규1 PASS(compile4.35초/suite0.19초/exit0): 실제 current owner End/Enter의 menu closed/terminal focus·정확한 Kill1개/project/pane/tab/session/title·후행 t 차단·다음 frame tinue의 exact wire/child exit0·assert 전 hub/session/fixture/tasks 회수입니다. 이 신규 typed command는 worker에 보내지 않았고 실제 worker의 Kill 성공은 영향 검사의 기존 경로와 구분합니다.
- [x] 영향3 PASS(suite0.36초/exit0): root 실제 copy/select/clear/paste Enter/Space·actual bounded host child4방향/new/kill·mixed 검색 Space입니다. 기존 single-event 탐색/지연 탐색/typeahead/engine·pointer/SGR 성공은 불변 경로에서 재사용했습니다.
- [x] 앞 절과 같은 lib/terminal-host strict3.93초·Rust2 fmt/diff exit0·source tool TS0.63초·Prettier33ms unchanged입니다. inherited Wry17 warning은 authored 실패가 아닙니다. 성공을 다시 실행하지 않습니다.

이번 단계4/4(100%)·M8 checkbox361/428(84.35%, 비가중 부모/자식·공수비 아님)·최종 N1~N8 0/8·전체 ETA 공수 미확정입니다. scalar 선택의 복수 action/repeat·owner→Split opener·current topology/AX/pointer/window/viewport·동적 locale/disabled/discard·Presence의 실제 native 시간/전체 App/GUI·Rust99%/beta/cutover는 미완료입니다. 다음은 기존 production App 조립의 assets/terminal effects/caller 선행 경계를 확인하며, 남은 메뉴 graph는 부모 체크리스트에서 유지합니다. 목표active·전체완료 전 Git 없음·live 검사 없음입니다.

## 2026-10-05 같은 batch 방향키의 지연 focus

대상은 실제 `src/terminal_surface.rs`·`tests/terminal-host.rs`와 기존 measurement tool입니다. 앞 단계와 같은 main 직접/PROCESS·verify·save-docs 계약이며 engine/vendor·제품 TS·의존성/lock/MSRV·보호 bundle/cache·OS/사용자 데이터·Git을 변경하지 않았습니다.

installed Radix RovingFocusGroupItem은 keydown 당시 currentTarget으로 후보를 계산한 뒤 setTimeout으로 focusFirst를 예약합니다. 실제 Content owner의 첫/끝 이동은 동기 focusFirst라 item과 구분해야 합니다. 실제 source 기존 build의 Clear에서 같은 JS task ArrowDown keydown/up 두 번은 나중 Split으로 끝나며 menu 유지/clear action 없음입니다. 각각 Clear에서 같은 Split을 예약하므로 두 칸 이동하지 않습니다. searchRef 갱신과 focus 예약도 서로 다른 시점입니다.

```sh
bun tools/m8-terminal-menu-source-measure.ts /private/tmp/taide-terminal-menu.Ppxh7N /private/tmp/taide-tooltip-source.J9DufD/built/assets/index-DEfYI-e9.css --reuse-build --mixed-navigation
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu는_같은_batch_방향키의_지연_focus를_즉시_연쇄하지_않는다 -- --exact
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/terminal_host-3226916b98261227 terminal_menu는_root_child의_비순환_탐색_disabled_건너뛰기_tab과_modifier를_보존한다 terminal_menu는_실제_label_검색_수명_반복문자와_space_억제를_보존한다 terminal_menu는_같은_batch_첫_검색문자_뒤_space의_leaf_선택을_억제한다 --exact
```

- [x] source1 exit0/0.61초·`menu-mixed-navigation.json`입니다. CSS/hash/Chrome/격리 origin/mock Keychain은 바로 아래 검색 절과 같고 원자료 파일은 별개입니다. 합성 key trusted=false이며 물리 이벤트 task/frame 대응을 실측한 것은 아닙니다. 이번 scenario만 실행했고 앞선 source 성공은 반복하지 않았습니다. source 도구 patch1회는 formatter의 현재 줄과 달라 적용되지 않았고 파일을 읽어 정정했습니다.
- [x] actual native Clear→Down 두 번 RED(compile4.44초/suite0.41초/exit101): AX focus가 Split 아닌 두 번째 항목에 있습니다. owned PTY/hub/fixture를 회수한 뒤 assertion했습니다. item/search 이동이 루프 중 focused를 즉시 변경하던 원인입니다.
- [x] item/search의 scheduled_focus를 실제 focused와 분리해 신규1 PASS(compile3.59초/suite0.39초/exit0)입니다. root owner 이동은 동기 유지하고 후보 없는 끝의 item 방향키는 이전 예약을 덮지 않습니다. window loss의 이전 취소 정책은 유지하며 실제 query는 기존 루프에서 동기 갱신합니다. fake Response/추가 OS timer/engine API는 없습니다.
- [x] 변경 영향3 PASS(suite0.20초/exit0): 기존 root/child22탐색·root/child19검색·바로 앞 mixed Space입니다. strict는 앞 절과 같은 lib/terminal-host 명령1.72초, TS0.67초, Rust2 fmt/diff exit0·Prettier30ms unchanged입니다. 최종 test 소스는 check_mixed_search→check_mixed_input 변수명만 실행 binary 이후 바뀌었고 strict/fmt로 확인했습니다. 의미가 같은 성공은 재사용하고 전체 재빌드/같은 검사를 반복하지 않습니다.

단계4/4(100%)·M8 checkbox356/423(84.16%, 비가중 부모/자식 집계·공수비 아님)·최종 N1~N8 0/8·전체 ETA 산정 보류입니다. source task 경계와 native 실제 OS frame 경계의 모든 대응/다중 timer의 동적 disabled·focus loss·AX/pointer·default action/Enter·닫힘 이후 input·full App/GUI/beta/cutover/Rust99%는 미완료입니다. 이 검사는 같은 raw 묶음의 roving 지연 경계를 구현한 좁은 증거이며 전체 mixed 메뉴 완료가 아닙니다. 다음은 기존 close 뒤 input/current topology·owner navigation 뒤 기본 선택 경계입니다. 목표active·전체완료 전 Git 없음·live 검사 없음입니다.

## 2026-10-05 같은 batch의 첫 검색 문자와 Space

대상은 `native/taide-native-app/src/terminal_surface.rs`, `tests/terminal-host.rs`, `tools/m8-terminal-menu-source-measure.ts`입니다. PROCESS·verify·save-docs 스킬에 따라 main이 직접 수행했습니다. 기존 engine/vendor·제품 TS·의존성/lock/MSRV·보호 bundle·OS/사용자 데이터/clipboard/Keychain·Git은 변경하지 않았습니다.

installed Radix MenuContentImpl의 searchRef는 keydown에서 동기 갱신되고 focus는 setTimeout 뒤 이동합니다. MenuItem은 searchRef가 비어 있지 않으면 Space 선택을 실행하지 않습니다. 기존 actual source build/CSS를 재사용해 focused Clear에서 같은 JS task의 `s` keydown/up→Space keydown/up을 1회 기록했습니다. `menu-mixed-search.json`은 menu 유지·action:clear 없음·나중 activeText=Split을 보여 줍니다. 키는 합성 trusted=false이고 실제 OS 이벤트/물리 시간 동등의 증거는 아닙니다. 열기/초기 문자만 trusted CDP이며 임시 headless Chrome/mock Keychain·동일 synthetic origin 라우팅을 사용하고 context/browser를 finally 회수했습니다. 기존 menu.json은 덮어쓰지 않았습니다.

native actual UI/AX Focus Clear·owned PTY에 같은 raw Key/Text batch를 주면 메뉴가 사라지는 RED가 재현됐습니다(compile4.81초/suite0.35초/exit101, missing actual item tab.split). 이전 prepare는 프레임 시작 query만 검사했고 실제 query는 버튼 뒤 navigate에서 갱신되어 Space가 먼저 Clear를 선택했습니다. fixture/hub/session은 assertion 전에 회수했습니다.

prepare가 살아 있는 raw Key/Text를 사건 순서대로 스캔해 default button 전에 Space 억제 index를 정하도록 수정했습니다. modifier/UTF-16/소비된 Key·window loss·사건별 focus 요청을 고려하고 실제 query/focus 갱신은 기존 navigate에 남깁니다. query를 두 번 append하거나 source timer를 OS 타이머로 바꾸지 않습니다. 기존 fixture의 검색 검사 flag는 일반 검색/이번 mixed 두 경계를 분리한 tuple이며 신규 검사는 이전19검색 전체를 반복하지 않습니다.

```sh
bun tools/m8-terminal-menu-source-measure.ts /private/tmp/taide-terminal-menu.Ppxh7N /private/tmp/taide-tooltip-source.J9DufD/built/assets/index-DEfYI-e9.css --reuse-build --mixed-search
bunx tsc --noEmit -p experiments/native-terminal-reference/tsconfig.json
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu는_같은_batch_첫_검색문자_뒤_space의_leaf_선택을_억제한다 -- --exact
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/terminal_host-3226916b98261227 terminal_menu는_실제_label_검색_수명_반복문자와_space_억제를_보존한다 terminal_menu_actions는_enter_space_keydown의_disabled_copy_select_clear_paste를_연결한다 --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --lib --test terminal-host -- -D warnings
/Users/hyunseokbyun/development/rust/cargo/bin/rustfmt --edition 2024 --check --config skip_children=true native/taide-native-app/src/terminal_surface.rs native/taide-native-app/tests/terminal-host.rs
```

- [x] source1 exit0/0.74초, Chrome154.0.8037.95/darwin·CSS SHA256 `b66166f52e29a2ffd85e4eaf679cc6ba3bc72b6580ae57f5ef8485edff53a177`입니다. 기존 sandbox Chrome 권한 실패 근거로 범위 한정 승격 실행했고 build/기존 plain·SGR 측정은 반복하지 않았습니다.
- [x] 신규 native1 PASS(compile11.51초/suite0.38초/exit0): Clear focus의 첫 검색→Space 뒤 메뉴 유지/Split focus·Escape/terminal restore·실제 plain Focus1004 wire/continue/owned child 회수입니다.
- [x] 이번 변경 영향2 PASS(suite0.34초/exit0): root/child 검색·취소 Key/Space와 검색 없는 root Enter/Space 선택입니다. 기존 host worker·pointer/기본 탐색/engine/SGR 성공은 불변 경로에서 재사용했습니다.
- [x] lib/terminal-host strict2.01초·Rust2 fmt·TS0.79초·Prettier30ms unchanged·diff exit0입니다. inherited Wry17 warning은 authored 실패가 아닙니다. 동일 성공은 재실행하지 않습니다.

현재 단계4/4(100%)·M8 checkbox351/418(83.97%, 비가중 부모/자식 집계·공수비 아님)·최종 N1~N8 0/8입니다. 전체 ETA는 실기/beta/cutover/packaging 공수가 미확정이라 산정 보류입니다. raw 여러 navigation/Enter/default action·close 뒤 입력/current topology·partial disabled/locale/discard·전체 App/GUI/CJK/VoiceOver/213 views/Rust99%는 미완료이고 목표active·전체완료 전 Git 없음입니다. 이번 성공은 source 지연 focus의 모든 mixed 동등성을 뜻하지 않습니다.

## 2026-10-05 actual leaf Enter/Space·host worker

대상 변경은 `native/taide-native-app/tests/terminal-host.rs`와 문서뿐입니다. main이 workflow 없이 직접 수행했고 PROCESS·verify·save-docs 스킬을 적용했습니다. 이번 단계에서 제품 Rust/TS·engine/vendor·의존성/manifest/lock/MSRV·OS/clipboard/Keychain/사용자 데이터·보호 bundle·Git을 바꾸지 않았습니다. 이전 typeahead/탐색과 전체 N1~N8 완료는 구분하며 목표active·전체완료 전 Git 없음입니다.

### 실제 구현 근거·이전 추정 철회

installed Radix MenuItem은 enabled/실제 target에서 Enter/Space keydown에 click하고 기본 선택이 취소되지 않으면 root를 닫습니다. SubTrigger/search 중 Space는 앞선 구현 계약대로 처리됩니다. 이전 HANDOFF에 ordinary Button Space keyup과의 차이를 제품 수정 대상으로 적었지만 실제 menu button의 전체 경로를 확인한 결과 철회합니다.

고정 patched egui `widgets/button.rs`는 UI stack의 `UiKind::Menu`를 검사해 `space_on_release && !menu_item` 때만 ordinary `register_button_keys`를 사용합니다. 메뉴는 `context.rs`의 현재 enabled/focused widget key_pressed Space/Enter 경로를 사용하므로 이미 keydown click입니다. [Radix ContextMenu 공식 문서](https://www.radix-ui.com/primitives/docs/components/context-menu)·직전 source 자료는 재사용했으며 새 DOM/OS 계측은 없습니다. 이미 맞는 제품 경로에 별도 keyboard 버튼이나 fake Response를 추가하지 않았습니다.

### 실제 UI와 bounded host 검증

기존 pointer action fixture를 선택 방식만 분리해 실제 AX Focus→pressed-only keydown→다음 keyup으로 구동합니다. Space와 Enter의 서로 다른2입력을 각각 같은 연속 copy(disabled)→selectAll→copy→clear→paste 시나리오에서 확인했습니다. 기존 결과의 copy CJK/astral/NFD 내용·current row clear·typed paste target/actual PTY continue/exit0·terminal 복귀도 유지합니다. menu keydown 직후 disabled Copy만 open이고 나머지는 즉시 closed인지 별도 기록합니다.

기존 atomic terminal menu fixture의 keyboard branch에서는 root Split에 실제 Focus·Right로 child 진입·실제 방향 Focus·pressed-only Enter/Space로 selection한 typed `HostCommand::TerminalMenu`를 실제 `HostBridge` bounded queue에 넣습니다. synthetic clipboard reader/writer는 호출되면 실패하고 environment도 합성 port이며 OS clipboard를 만들지 않습니다. 실제 tracked worker의 dispatch 종료 repaint Notify를 각 command 완료 신호로 기다리며 단순 시간 sleep이나 임의 poll count로 성공을 추정하지 않습니다.

네 방향 split마다 실제 queued direction·project/pane/tab/session·title, root/child 닫힘/terminal 복귀, atomic revision+1·새 tab·cwd/root guard를 확인합니다. new는 same-pane 활성 새 terminal/cwd None, 원래 source 재활성 뒤 kill은 실제 worker의 Closed reply·원래 tab/child 회수와 다른 tab 보존입니다. inactive/stale identity와 outside cwd의 기존 검사는 같은 fixture에서 유지합니다. worker를 disconnect/join하고 owned terminal/tasks를 회수한 뒤 기록한6개 close/target·4개 direction assertion을 확인합니다. source pane를 직접 그린 UI/typed host 검증이며 전체 App layout/queue composition을 입증한 것은 아닙니다.

### 실패·실행 명령과 결과

root 신규는 첫 실행 PASS였으므로 제품 RED나 제품 수정을 주장하지 않습니다. 새 host fixture의 E0004는 directional array를 DropEdge 전체 enum match로 다시 써 Center를 빠뜨린 작성 오류로, loop를 실제4방향/label/key tuple로 고쳤습니다. E0502는 frame의 response_id mutable capture를 나중 선택에서도 사용하면서 초기 assertion과 겹친 오류로, 실제 ID를 Cell에 저장했습니다. 이에 따른 unused_mut도 제거했습니다.

첫 host runtime 실패(compile4.78초/suite0.41초/exit101)는 `commands.is_empty()` assertion입니다. 초기 fixture의 UI는 실제 ResizeTerminal을 먼저 큐에 넣는데 기존 direct host 검사는 그 queue를 쓰지 않아 잔존했습니다. 구현 source의 `self.resizing.insert`를 확인하고 메뉴 선택 전 setup queue가 ResizeTerminal만 포함하는지 검사한 뒤 그 setup 항목만 비웁니다. 이후 선택 queue는 여전히 정확히1명령인지 검사하므로 중복/다른 메뉴 action을 숨기지 않습니다. 실제 host actor에 Resize를 보내는 전체 App geometry 경로는 이번 범위가 아닙니다. 이 실패를 제품 duplicate menu bug라고 주장하지 않습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu_actions는_enter_space_keydown의_disabled_copy_select_clear_paste를_연결한다 -- --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu는_실제_keyboard_split_new_kill을_host_worker에_전달한다 -- --exact
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/terminal_host-3226916b98261227 terminal_menu_actions는_실제_click으로_disabled_copy_select_clear_paste와_focus를_연결한다 terminal_menu는_실제_secondary_popup과_원자적_split_new_kill의_소유권을_보존한다 --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host -- -D warnings
/Users/hyunseokbyun/development/rust/cargo/bin/rustfmt --edition 2024 --check --config skip_children=true native/taide-native-app/tests/terminal-host.rs
```

- [x] root keydown 신규1 PASS(compile4.56초/suite0.64초/exit0): Space/Enter2입력의5연속 action, disabled 무선택/각 keydown 즉시 닫힘·copy/select/clear/paste/terminal restore·owned PTY exit0입니다. 이후 host fixture만 변경했으므로 이 성공은 재사용했습니다.
- [x] UI→실제 host worker 신규1 PASS(compile4.24초/suite0.31초/exit0): split4/Space new/Enter kill의6선택·정확한 direction/target/title·닫힘/복귀·atomic/current cwd·stale/inactive guard·Closed reply·child/worker/tasks 회수입니다. 실제 물리 OS 키/전체 App geometry나 모든 modifier/source timing의 증거는 아닙니다.
- [x] fixture 공통화 영향2 PASS(suite0.14초/exit0): 기존 실제 pointer copy/select/clear/paste와 direct atomic split/new/kill입니다. source/제품 코드가 바뀌지 않아 typeahead·22navigation·root/child AX·plain/SGR/F10/prefix·engine/app 이전 성공은 재사용하고 다시 실행하지 않았습니다.
- [x] 해당 test target strict0.89초/exit0·authored Rust1 exactfmt/diff exit0입니다. 제품 lib/tests 전체 검사 대신 test-only 변경 위험에 맞춰 범위를 줄였습니다. inherited Wry17 warning은 authored 실패가 아닙니다. 문서 뒤는 diff만 확인합니다.

### 남은 범위와 진행률

leaf 기본 선택/host 단계4/4(100%) 완료 시 M8 checkbox346/413(83.78%, 비가중 부모/자식 집계)입니다. 최종 N1~N8는0/8이며 공수 비율/전체 완료를 뜻하지 않습니다.23:25:11→23:32:46 UTC는 이번 단계 일부 관측 구간이고 전체 ETA 근거가 아닙니다. 마지막 검사 예상1분 이내는 직전4.24초/.31초 실행에 한정했으며 영향2/strict도 그 범위에서 종료됐습니다. 현재 단계 ETA는 완료, 전체 ETA는 beta/cutover/실기/packaging 공수 미확정으로 산정 보류입니다.

다음은 기존 메뉴의 raw mixed default action/Space/AX/탐색 순서·같은 batch close 뒤 입력과 current topology입니다. source setTimeout·repeat keydown/multi-action 개수·modifier·focus loss/disabled/partial 축·dynamic resize/locale·강제 discard/재열기 search·전체 App geometry·pointer/Touch/IME·OS/WKWebView/다른 플랫폼·bounds/시각/CJK/VoiceOver/full cutover/Rust99%·213 views와 N1~N8는 계속 미완료입니다. source leaf 기본 성공을 이 전체 graph 완료로 바꾸지 않습니다.

## 2026-10-05 actual label typeahead·검색 중 Space

대상은 actual `native/taide-native-app/src/terminal_surface.rs`·`tests/terminal-host.rs`입니다. main이 workflow 없이 직접 진행했고 PROCESS·verify·save-docs 스킬을 적용했습니다. engine/vendor·제품TS·의존성/manifest/lock/MSRV·보호 bundle/기존 cache·OS 설정/clipboard/Keychain/사용자 데이터·Git은 이번 단계에서 불변입니다. full M8 N1~N8 0/8이며 목표는 active입니다.

### 실제 원본 계약과 구현

원본 TS ContextMenu와 installed Radix `MenuContentImpl`·`getNextMatch`·`MenuItemImpl`·`MenuSubTrigger`를 다시 대조했습니다. 기존 [공식 ContextMenu](https://www.radix-ui.com/primitives/docs/components/context-menu) 조회와 source DOM 자료는 재사용했고 새 브라우저/OS 측정은 없습니다.

- 실제 locale label의 textContent.trim을 검색하며 ctrl/alt/meta를 무시하고 Shift 문자 자체는 허용합니다. key.length는 UTF-16 한 단위 기준입니다. 원본 문자와 같은 문자의 반복을 검사한 뒤 case-insensitive prefix를 매칭하며 현재 label의 첫 위치부터 순환합니다. 단일 문자 검색은 현재 label과 같은 값을 모두 제외하고, 다중 문자 검색의 첫 match가 현재 label이면 이동하지 않습니다.
- root와 child는 독립된1초 검색 수명을 가집니다. child unmount는 child 검색을 폐기하고 root unmount는 전체 snapshot을 폐기합니다. 검색이 살아 있을 때 Space는 item/SubTrigger 선택 keydown을 실행하지 않고 검색어에 포함됩니다. source 검색/roving의 focus는 setTimeout이므로 같은 raw batch 여러 key의 timing 동등까지 완료한 것은 아닙니다.
- 고정 egui-winit0.36.2 `src/lib.rs`는 printable pressed Key 다음 Text를 생성하며 ctrl/command text를 제외하고 Ime Commit은 별도 event입니다. native는 바로 앞에서 살아 있는 Key와 UTF-16 한 단위 Text만 연결하고 ctrl/alt/command/mac_cmd를 제외합니다. normalized에서 이미 소비된 Key의 Text는 검색하지 않으며 메뉴가 직접 소비한 Space raw index만 예외로 인정합니다. Ime Commit/Paste/standalone Text와 astral Text는 character keydown 검색으로 처리하지 않습니다.
- 실제 root/child enabled Response ID와 actual label을 pass-local로 수집합니다. owner별 `MenuSearch`가 query/deadline·frame 시작 snapshot·이번 pass 억제 Space index를 보관합니다. 같은 frame discard pass에서 검색어를 중복 append하지 않도록 시작 상태를 복원합니다. 만료/repaint는 기존 egui time/request_repaint_after를 사용하고 typed clock을 외부 OS 시간으로 바꾸지 않습니다. fake AX tree나 신규 engine API·의존성·검사 억제는 없습니다.

### 실패와 실제 결과

첫 fixture 실행(compile4.54초/suite0.40초/exit101)은 plain en 메시지 맵을 LocalePack으로 읽어 `missing field version`이었습니다. 실제 shape로 정정했고 locale 파싱을 PTY 생성 전으로 옮겼습니다. 첫 실패 실행의 PTY 정리를 별도로 관측한 증거는 없으므로 성공 cleanup 근거로 사용하지 않습니다. 이후 actual 제품 RED(compile4.39초/suite0.33초/exit101)는 첫 `c` 뒤 disabled Copy를 건너뛰어 Clear로 가지 않고 owner에 남은 것입니다. 제품에 검색 경로가 없던 원인입니다.

구현의 최초 compile E0277/exit101은 matched `&Id` 비교의 추가 dereference 오류였습니다. actual source `getNextMatch`의 현재 label 시작 순서에 맞춰 Clear 다음 uppercase S의 기대도 SelectAll이 아닌 Split으로 정정했습니다. 이 기대 오류는 runtime 실패로 주장하지 않습니다. unpatched rustfmt 뒤 줄 모양을 대상으로 한 fixture patch1회는 적용되지 않았고 실제 줄을 읽어 수정했습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu는_실제_label_검색_수명_반복문자와_space_억제를_보존한다 -- --exact
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/terminal_host-3226916b98261227 terminal_menu는_root_child의_비순환_탐색_disabled_건너뛰기_tab과_modifier를_보존한다 terminal_menu는_enter_space로_split을_열고_첫_항목에_포커스를_준다 terminal_menu_actions는_실제_click으로_disabled_copy_select_clear_paste와_focus를_연결한다 --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --lib --tests -- -D warnings
/Users/hyunseokbyun/development/rust/cargo/bin/rustfmt --edition 2024 --check --config skip_children=true native/taide-native-app/src/terminal_surface.rs native/taide-native-app/tests/terminal-host.rs
```

- [x] 기본 신규1 PASS(compile12.04초/suite0.51초/exit0)입니다. 실제 en 메시지·root Copy disabled/Clear·반복 문자·Shift 대문자/순환 시작 위치·1초 초기화·ctrl/alt/astral/IME/Paste/standalone Text 무검색·Split/Paste 검색 중 Space 무선택, child 독립 search/repeated s4항목 순환·Split Up 다중 prefix를 actual AX focus로 확인했습니다. sleep 없이 raw clock을1.1초씩 전진시킨 결정적 검사이며 실제 OS 시간/입력기가 아닙니다.
- [x] 소비된 Key 기본 경계 추가 뒤 관련1 PASS(compile8.86초/suite0.53초/exit0)입니다. actual UI caller가 normalized key만 먼저 지웠을 때 남은 Text가 검색에 사용되지 않는지 추가 확인했고 이번 pass 메뉴의 own Space 억제 index만 검색에 인정합니다. 이 변경은 입력/코드가 바뀐 관련 검사1회이며 동일 상태 성공을 반복한 것이 아닙니다. 최종19 focus snapshots(root14/child5) 뒤 Left·Escape/terminal restore·plain Focus1004 exact wire·child exit0/owned 회수를 확인하고 asserts 전에 fixture를 정리합니다.
- [x] 영향3 PASS(suite0.19초/exit0)입니다. 같은 최종 binary로 basic root/child22탐색·submenu Enter/Space first-enabled·실제 copy/select/clear/paste/focus를 확인했습니다. all-disabled/SGR/F10/prefix/atomic host/App scope 및 engine 성공은 관련 불변 입력 branch에서 재사용합니다.
- [x] app lib/tests strict4.19초/exit0·authored2 exactfmt·diff exit0입니다. inherited Wry17 warning은 authored 실패가 아닙니다. 문서 변경 뒤는 diff만 확인하며 새 빌드/검색 성공을 반복하지 않습니다.
- [x] fixture 잔류 확인의 sandbox `pgrep -fl native-terminal-queue-fixture`는 sysmond service/목록 접근 실패 exit3였습니다. 같은 정확한 executable 이름의 승인된 읽기 전용 조회는 exit1·출력 없음으로 현재 해당 fixture가 없음을 확인했습니다. 프로세스를 종료하거나 다른 목록/OS 설정을 변경하지 않았습니다. 첫 실패 시점 cleanup을 직접 관측한 증거를 소급 주장하지 않습니다.

### 계속 남은 경계

- [ ] 실제 leaf의 Enter/Space keydown default action과 host 왕복, raw 같은 batch 여러 문자/Space/AX/탐색→activation·current focus/modal/topology·동적 enabled/크기·partial disabled/typeahead입니다. 기본 Space 억제 성공이 leaf 기본 선택 timing 완료 증거가 아닙니다. next priority는 actual default action/host입니다.
- [ ] same-frame discard 검색 snapshot은 구현 근거가 있지만 독립 강제 discard assertion은 아직 없습니다. root/child 재진입/unmount 검색 폐기는 코드로 연결했으나 모든 재열기/locale 동적 교체·Unicode lowercase/full keyboard backend/IME 실기/OS/WKWebView/다른 플랫폼·bounds/시각/VoiceOver 전체 완료는 아닙니다. 실제 CJK/VoiceOver 검사는 사용자-last 조건을 유지합니다.
- [ ] 지속 입력의 검색어는 source처럼 deadline을 갱신하는 query입니다. 새 임의 cap을 도입하지 않았으며 전체 resource/beta/cutover/packaging/Rust99%·213 views와 N1~N8은 미완료입니다.

typeahead4단계 완료 시 checklist341/408(83.58%)이며 비가중 부모/자식 집계이지 공수 비율이 아닙니다. 최종 N1~N8는0/8입니다. 현재 turn에서 관측한23:11:32→23:20:16 UTC는 구현·fixture 실패·검증을 포함한 일부 구간이며 전체 ETA 근거는 아닙니다. 검사 실행 예상1분 이내라는 업데이트는 직전12.04초 빌드/0.51초 suite와 관련 범위에 한정하며 진단/문서·전체 공수 예측이 아닙니다. typeahead ETA는 완료, 전체 ETA는 공수 미확정으로 산정 보류입니다.

## 2026-10-05 actual root/child 기본 탐색

대상은 `native/taide-native-app/src/terminal_surface.rs`·`tests/terminal-host.rs`입니다. main이 workflow 없이 직접 수행했고 PROCESS·verify·save-docs 스킬과 설치된 ai-process/desktop/security 전문을 사용했습니다. 프로젝트 `docs/convention/ai-process.md`는 없으므로 설치된 전문을 대체 근거로 유지합니다. 이번 변경에는 engine/vendor·제품TS·의존성/manifest/lock/MSRV·보호 bundle·OS 설정/clipboard/Keychain·사용자 데이터·Git 변경이 없습니다.

### source 계약과 변경

기존 [Radix ContextMenu 공식 문서](https://www.radix-ui.com/primitives/docs/components/context-menu) 조회를 재사용하고 실제 설치된 `@radix-ui/react-menu/dist/index.mjs`·`@radix-ui/react-roving-focus/dist/index.mjs` 및 원본 shared ContextMenu wrapper를 대조했습니다. wrapper는 loop를 변경하지 않으며 기본값은 false입니다. enabled item 순서의 Up/Down·Home/End/PageUp/PageDown, 비순환 끝 경계와 모든 Tab의 preventDefault를 재현합니다. root owner의 first/last handler에는 modifier gate가 없지만 item roving은 modifier 입력을 무시합니다. typeahead는 별도의1초 search 수명이므로 이번 기본 탐색 완료에 포함하지 않습니다. 새 DOM/OS 실측은 하지 않았습니다.

실제 렌더된 root/child Response의 ID·enabled를 pass-local 목록으로 수집하고 같은 `navigate_menu` 함수로 처리합니다. 이전 viewport input-start owner·raw index의 AX/declared focus 요청과 살아 있는 normalized event를 사용하며 fake AX node나 이전 Response를 저장하지 않습니다. Tab과 적용된 탐색 keydown만 소비하고 generic cardinal 이동을 정리합니다. Left/Right의 기본 submenu 동작은 기존 opener/close 처리가 우선합니다. 실제 focused owner/item에 egui `EventFilter`의 tab/horizontal_arrows/vertical_arrows를 설정해 다음 입력에서 generic Tab 예약이 먼저 생기지 않게 합니다. installed0.36.2 `memory/mod.rs`·`data/input/event_filter.rs`의 구현으로 대조했으며 새 engine API는 없습니다.

### 실제 실패·검증

기본 탐색 초안을 먼저 작성했으므로 최초 구현 전 RED를 주장하지 않습니다. 신규 actual UI/owned PTY 검사의 첫 실행은 compile12.55초/suite0.49초/exit101입니다. root 연속 입력 중 index9 Shift+Tab 뒤 실제 AX focus가 SelectAll 대신 다른 항목으로 이동했습니다. egui의 `interested_in_focus`가 그리기 중 `id_next_frame`에 이전 위젯을 예약하고 후반 `move_focus(None)`·`request_focus`만으로는 예약이 취소되지 않는 원인을 확인했습니다. actual focused 메뉴의 navigation filter를 유지한 뒤 관련 신규 검사만1회 재실행했습니다. 잘못된 QA 제목을 대상으로 한 문서 patch1건은 적용되지 않았고 실제 제목을 읽은 뒤 수정했습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu는_root_child의_비순환_탐색_disabled_건너뛰기_tab과_modifier를_보존한다 -- --exact
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/terminal_host-3226916b98261227 terminal_menu는_enter_space로_split을_열고_첫_항목에_포커스를_준다 terminal_menu는_모든_split이_disabled여도_submenu_owner와_복귀를_보존한다 terminal_menu_actions는_실제_click으로_disabled_copy_select_clear_paste와_focus를_연결한다 --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --lib --tests -- -D warnings
/Users/hyunseokbyun/development/rust/cargo/bin/rustfmt --edition 2024 --check --config skip_children=true native/taide-native-app/src/terminal_surface.rs native/taide-native-app/tests/terminal-host.rs
```

- [x] 신규1 PASS(compile4.10초/suite0.48초/exit0)입니다. 실제 root12/child10의 서로 다른 연속 key 입력으로 disabled Copy skip·Up/Down 끝 비순환·Home/End/Page·item Shift+Down 무이동·Tab/Shift+Tab 유지·non-trigger Right 유지를 actual AX tree focus로 확인했습니다. UI/owned PTY fixture이며 OS keyboard 실측은 아닙니다.
- [x] 같은 신규 시나리오의 submenu first-enabled/관계·Left trigger 복귀·Escape terminal 복귀·plain Focus1004 exact wire·child exit0/owned 회수를 확인했습니다. snapshots의 assert 전에 fixture/child를 회수합니다. 각 key 뒤 blank pass를 둔 검사이지 같은 raw batch 전체 default action 완료가 아닙니다.
- [x] 변경 영향3 PASS(suite0.15초/exit0): Enter/Space opener, 모든 방향 disabled owner, 실제 copy/select/clear/paste/focus입니다. 신규 성공을 재실행하지 않고 같은 compiled binary의 관련 범위만 확인했습니다.
- [x] app lib/tests strict3.64초/exit0·authored2 exactfmt/exit0입니다. inherited Wry17 경고는 authored 실패가 아닙니다. engine 변경이 없어 이전 engine strict, F10/prefix/SGR·host atomic·App scope 등 불변 성공을 재사용합니다.

### 남은 경계와 진행률 표시

기본 탐색 기록 뒤 다음 기존 typeahead를4단계로 분해했습니다. 현재1/4(25%)는 installed Radix source와 고정 egui-winit의 Key/Text·Ime Commit 및 실제 en locale 대조만 완료한 상태입니다. 구현·runtime PASS는 아직 없습니다. M8 체크리스트 현재337/408(82.60%)이며 아래 기본 탐색 완료 당시와 분모가 다릅니다. typeahead ETA 역시 구현/검증 전이라 산정 보류입니다.

root modifier first/last·raw 안의 여러 navigation→Enter 등의 default action timing·첫/동적 focus→Tab의 같은 batch 예약·typeahead Space 억제/IME/Text 경계·disabled 축/동적 크기·leaf actual host 왕복·App/current topology·bounds/시각/OS/WKWebView/VoiceOver는 미완료입니다. source의 roving/typeahead focus는 setTimeout이므로 단순히 native 같은 batch를 순차 즉시 focus로 처리했다고 source 동등으로 단정하지 않습니다.

진행률은 PROCESS M8 절의 checked/전체 checkbox 비가중 집계이며 부모/자식 중복을 포함하므로 구현 공수 비율이 아닙니다. 새 checklist가 추가되면 분모도 늘어납니다. 이번 기본 탐색은4단계 중4완료, 최종 M8 N1~N8는0/8로 별도 지표입니다. 2026-10-04 23:00:57→23:04:30 UTC는 이번 재개 중 관측 구간이며 이전 초안 작성이나 전체 M8 공수를 포함한 시간이 아닙니다. 전체 M8 ETA는 실기/beta/cutover/packaging 공수가 미확정이므로 산정 보류입니다. 짧은 compile/suite 시간을 전체 M8 시간으로 외삽하지 않습니다. 이후 갱신에도 작업 비율·전체 checklist 비율·근거 있는 ETA 또는 산정 보류를 표시합니다.

## 2026-10-05 실제 submenu 관계·기본 키보드 열기와 복귀

대상은 actual `terminal_surface.rs`·`tests/terminal-host.rs`, native-only egui patch의 `containers/menu.rs`입니다. main 직접 수행이며 현재 요청 workflow 미사용을 유지합니다. 앞선 실제 root AX 수정/성공은 이 아래 이력으로 보존합니다.

### 원본과 구현

공식 [Radix ContextMenu](https://www.radix-ui.com/primitives/docs/components/context-menu)와 설치된 `@radix-ui/react-menu/dist/index.mjs`의 SubTrigger/Content를 읽었습니다. ltr SubTrigger는 Enter/Space/ArrowRight로 열고 keyboard 진입 때 첫 enabled item을 포커스합니다. ArrowLeft는 child만 닫고 trigger에 복귀하고 Escape는 root를 닫습니다. source의 onKeyDown에는 modifier gate가 없고 로컬 구현도 입력에 실제 담긴 modifier로 해당 키를 소비합니다. source `Root`·document에 direction override가 없는 현재 ltr 경계를 사용하며 임의 RTL UI를 추가하지 않습니다. root/Home/End/Tab/roving/typeahead·다중 raw 사건은 이 기본 경계의 완료 근거가 아닙니다. 새 source DOM 측정은 하지 않았고 이전 plain/SGR 자료를 재사용합니다.

- 기존 SubMenu는 click/hover만 열므로 pass별 `open_override: Option<bool>`를 추가했습니다. None은 기존 state/heuristic/dismissal이며 실제 App의 keyboard open/close 때만 Some을 줍니다. 현재 메뉴 상태·Popup stack·pointer dismiss 코드는 재사용하며 native-only framework public API 영어 doc만 추가했습니다. 다른 submenu를 강제로 바꾸거나 별도 popup/cache로 우회하지 않습니다.
- 실제 Split의 stable auto ID·입력 시작 focus와 현재 normalized key를 사용합니다. opener key를 먼저 소비하고 generic cardinal 이동을 None으로 정리해 그 키가 child action까지 중복 전달되지 않도록 합니다. first sizing pass의 disabled 실제 위젯을 건너뛰고 visible pass의 첫 enabled 방향에만 pending focus를 적용합니다. 네 방향 모두 unavailable이면 실제 focusable submenu owner가 focus를 받습니다.
- actual child menu owner를 실제 Ui에 생성하고 실제 방향 Button들을 accessibility parent 아래에 그립니다. Split의 expanded/controls와 child의 labelled_by가 같은 실제 IDs를 사용합니다. 방향 목록4개의 실제 widget ID는 이 menu snapshot 수명에만 보관하고 닫힌 child/pending은 회수합니다. ArrowLeft는 actual item 또는 empty child owner에서 Split로 복귀하며 root는 유지합니다.

### 실패와 검증

첫 fixture(compile5.14초/suite0.45초/exit101)는 root-only assertion을 submenu 복귀 뒤에도 적용해 focused MenuItem을 Menu로 기대한 작성 오류였습니다. 기존 root 검사는 그대로 두고 새 submenu branch에서만 해당 assertion을 분리했습니다. 제품 RED는 compile3.03초/suite0.19초/exit101에서 Right 뒤 Split expanded가 None으로, 원본 Some(true) 계약을 만족하지 못한 것입니다. 기존 engine source의 click/hover-only와 누락된 actual child 관계를 함께 수정했습니다.

Enter 추가 검사 RED는 compile4.77초/suite0.17초/exit101의 actual focus != 첫 방향 item입니다. 메뉴는 열렸지만 keyboard entry focus가 없던 원인이라 Enter/Space도 opener로 소비하고 같은 actual pending focus에 연결했습니다. Space는 이 실패 실행에서 아직 실행되지 않았으며 아래 수정 후 검사에서 처음 실행됐습니다. 세 검사는 같은 시나리오3회 반복이 아니라 Right/Left, Enter/Space, all-disabled라는 다른 입력/위험입니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu는_실제_split_ax와_방향키_열기_복귀의_소유권을_보존한다 -- --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu는_enter_space로_split을_열고_첫_항목에_포커스를_준다 -- --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu는_모든_split이_disabled여도_submenu_owner와_복귀를_보존한다 -- --exact
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/terminal_host-3226916b98261227 terminal_menu는_실제_ax_focus의_메뉴항목_자식과_닫힌_트리_회수를_보존한다 terminal_menu_actions는_실제_click으로_disabled_copy_select_clear_paste와_focus를_연결한다 --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p egui -p taide-native-app --lib --tests -- -D warnings
```

- [x] actual Right/Left·AX 신규1 PASS(12.40초/0.40초/exit0): actual Split AX Focus→Right→child actual Menu role/labelled_by·4 MenuItem/모두 enabled·controls/expanded·첫 Left 항목 focus→ArrowLeft→Split focus/child node 회수/expandedfalse/controls없음입니다. 다음 Escape/terminal restore와 plain Focus1004 exact wire·owned child exit0/회수도 확인합니다.
- [x] actual Enter/Space 신규1의 서로 다른2입력 PASS(7.85초/0.51초/exit0): 같은 actual child 관계·첫 enabled focus·Left/terminal 복귀입니다. Right 성공은 재사용하고 다시 실행하지 않았습니다.
- [x] actual 작은 창 신규1 PASS(7.58초/0.32초/exit0): viewport200×220에서 네 방향이 모두 disabled로 표시되고 실제 submenu owner focus→Left/trigger 복귀·닫힌 child AX 회수입니다. all-disabled fallback 및 child owner 복귀 조건의 구현 후1회 실행입니다. 원본 first-enabled 조건/코드 검토로 구현했으며 별도 native RED를 주장하지 않습니다.
- [x] None child의 남은 pending flag를 지우는 수명 정리 뒤 no-run3.69초/exit0, 실제 root AX와 copy/select/clear/paste/action 영향2 PASS(0.42초/exit0), engine/app lib/tests strict3.86초/exit0·authored2 exactfmt/vendor Menu parse-only/diff exit0입니다. 신규3의 입력은 child가 열린 때의 branch이고 정리는 None 때 false 보장이므로 해당 성공을 재사용합니다. inherited Wry17 warning은 authored 실패가 아닙니다. 문서 갱신 뒤에는 diff만 검사합니다.
- [x] 기존 F10/prefix/입장epoch·plain/SGR wire·host atomic split/new/kill·App generic scope/후행 AX·root AX/엔진 이전 성공은 변경 없는 branch에서 재사용합니다. 새 의존성/manifest/lock/MSRV/unsafe/OS/IPC API·제품TS 변경은 없고 existing vendor Menu 파일만 수정했습니다. MIT/Apache2·native-only patch·vendor99% 제외와 기존10변경 파일 범위를 유지합니다. 보호 bundle/기존 cache·OS/clipboard/Keychain/사용자 데이터·Git은 불변입니다.

### 계속 미완료

- [ ] root/child의 전체 roving·Home/End/Page/Tab/typeahead, disabled 축 하나만 남은 첫 enabled 순서·정확한 크기 경계/동적 resize·leaf Enter/Space/pointer/AX 선택의 전체 실제 host 왕복입니다.
- [ ] 같은 raw batch AX/키/여러 open-close/닫힌 뒤 input·App window→document/후행 AX/current topology·primary outside/hover grace/Touch/IME/전부 disabled의 혼합·전체 bounds/시각/locale/테마입니다.
- [ ] 실제 OS/WKWebView/다른 플랫폼/VoiceOver·213 views/full cutover/Rust99% 및 M8 N1~N8는 미완료입니다. 후속46/N1~N8 0/8·목표active·전체완료 전 commit/push 없음·live 검사 없음입니다. 다음은 root/child navigation과 동일-frame mixed 소유권입니다.

## 2026-10-05 실제 AX 구조·secondary release focus

이 절은 root AX 단계의 이력입니다. 여기의 submenu 미완료 표기는 당시 범위이며 기본 submenu 관계/키보드 후속은 위 실제 submenu 절이 최신 정본입니다. 전체 graph/실기 미완료는 유지합니다.

대상은 `native/taide-native-app/src/terminal_surface.rs`, `tests/terminal-host.rs`, native-only egui path patch `src/context.rs`입니다. 현재 요청의 workflow 미사용 선택으로 main이 직접 수행했습니다. PROCESS·verify·save-docs 스킬을 적용했고 없는 프로젝트 `docs/convention/ai-process.md` 대신 설치된 전문을 사용했습니다.

### 재현과 수정

- 최초 fixture의 `Role::Separator`는 설치된 AccessKit0.24.1에 없어 E0599/exit101입니다. 실제 Splitter로 정정했으며 Windows adapter의 UIA Separator/"separator", AT-SPI Separator 매핑을 확인했습니다. macOS adapter는 NSAccessibilitySplitterRole이며 VoiceOver 실기 확인은 아닙니다.
- 제품 RED는 compile4.59초/suite0.32초/exit101의 actual tree focus `Window != Menu`입니다. 진단1회(4.56초/0.16초/exit101)에서 visible 빈 pass의 Memory focus는 유지됐고 secondary release 뒤 Window였음을 확인했습니다. 실제 owner widget 본문 밖 Frame 경계 release를 기본 Clicks 외부 클릭으로 처리하는 원인입니다. focused ID·직전 viewport menu owner·press 없는 secondary-only release만 surrender에서 제외합니다. 다른 버튼 release/press/일반 widget·Presses/Never는 기존 정책입니다.
- 실제 menu owner를 먼저 생성하고 actual 내용을 `UiBuilder::accessibility_parent(menu_owner)` child Ui에서 그립니다. 가짜 descendant나 TreeUpdate 재작성은 없습니다. 실제 Button bounds/actions/disabled를 유지하며 반복 helper로 MenuItem을 부여하고 실제 Separator Response에 Splitter/Horizontal을 부여합니다.
- 첫 수정 뒤 Menu focus와 앞4개 자식은 통과했지만 `tab.split` 이름이 없다는 RED(11.58초/0.33초/exit101)였습니다. SubMenuButton의 장식 화살표도 이름에 들어가는 원인이므로 실제 Split node label을 원본 locale 이름으로 지정하고 HasPopup::Menu를 부여했습니다.

공식 [UiBuilder accessibility_parent](https://docs.rs/egui/latest/egui/struct.UiBuilder.html#method.accessibility_parent)를 조회하고 고정0.36.2 installed `ui.rs`·`context.rs`·`response.rs`·`containers/menu.rs`의 부모 등록/노드 생성·장식 이름 구현으로 대조했습니다. 온라인 latest는 고정 버전 증거로 사용하지 않습니다. 고정 AccessKit 역할/adapter는 installed source가 정본입니다. 원본 TS ContextMenu7항목/3구분자와 직전 source DOM 자료를 재사용했고 새 브라우저/OS 측정은 없습니다.

### 실행 결과

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --test terminal-host terminal_menu는_실제_ax_focus의_메뉴항목_자식과_닫힌_트리_회수를_보존한다 -- --exact
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/terminal_host-3226916b98261227 terminal_menu는_실제_focus_owner와_press_loss_global_release_restore_wire를_보존한다 terminal_menu는_실제_secondary_popup과_원자적_split_new_kill의_소유권을_보존한다 terminal_menu_actions는_실제_click으로_disabled_copy_select_clear_paste와_focus를_연결한다 --exact
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p egui -p taide-native-app --lib --tests -- -D warnings
```

- [x] actual AX 신규1 PASS(compile4.07초/suite0.43초/exit0): actual visible menu/secondary release의 focus Menu·Focus action, children DFS7개 이름/MenuItem/Click action·Copy만 disabled·Split HasPopup Menu·구분자3개, Escape/복귀 뒤 Menu node 없음과 plain Focus1004 exact wire/child exit0/owned 회수입니다. assertions 전 owned child/session/hub/fixture를 정리합니다. Horizontal은 실제 builder 구현 근거이며 orientation assertion/OS mapping 실기는 아닙니다.
- [x] 마지막 guard를 다른 release 전체 제외/focused ID로 한정한 뒤 no-run compile10.49초/exit0, 같은 binary의 영향3 suite0.42초/exit0입니다. 실제 plain/SGR wire·원자적 menu host·실제 action/copy/select/clear/paste/focus를 확인했습니다. engine release와 child Ui 구조 변경의 영향이며 신규 AX 성공의 secondary-only branch는 불변이라 재사용합니다.
- [x] engine/app lib/tests strict3.98초/exit0, authored Rust2 exactfmt·vendor Context parse-only·diff exit0입니다. inherited Wry17 경고는 authored 실패와 구분합니다. upstream diff는 기존10변경 목록/메타데이터 차이뿐이며 diff exit1은 예상 결과입니다. vendor 재포맷/검사 억제·새 파일/의존성/OS/unsafe API는 없습니다. MIT/Apache2·native-only patch·vendor authored99% 제외를 유지합니다.
- [x] F10/prefix·입장 epoch·App generic scope/후행 AX·primary/middle·prepared/pass와 원본 성공 자료는 재사용합니다. 문서 갱신에는 diff만 수행합니다. 기존 cache/bundle을 유지하고 다음 Cargo도 위 새 target을 사용합니다.

### 남은 범위

- [ ] 실제 submenu Menu root/항목 부모·controls/expanded·focus/navigation/typeahead·전체 bounds/disabled/current topology입니다. 방향 항목 MenuItem helper는 구현했지만 새 submenu runtime 검사는 아닙니다.
- [ ] 여러 press/open-close·동일 raw frame Escape/닫힌 뒤 Text·primary outside/후행 AX의 전체 혼합·disabled/modal/viewport·Touch/IME·물리 OS/WKWebView/다른 플랫폼/VoiceOver입니다.
- [ ] 213views/full cutover와 전체 N1~N8/Rust99%는 미완료입니다. M8 0/8·목표active·완료 전 Git 없음이며 보호 bundle/OS 설정/clipboard/Keychain/사용자 데이터/제품TS/의존성/manifest/lock/MSRV는 이번 수정에서 불변입니다.

## 대상과 구현

최신 실제 AX 구조 후속: 포커스된 actual 메뉴의 실제 항목 자식·MenuItem 역할·구분자·Split HasPopup/이름과 닫힌 tree 회수를 확인했습니다. secondary release 뒤 focus가 Window로 떨어지는 RED도 수정했습니다. 위 실제 AX 구조 절이 정본이며 이전 미완료 표기는 당시 이력입니다. 전체 submenu/keyboard·bounds/관계·mixed/current topology/실기는 계속 미완료입니다.

최신 focus owner 후속: 기본 press→menu keyboard owner/Focus1004 loss→global release→Escape/terminal restore/gain은 actual PTY plain/SGR에서 확인했습니다. engine/App window capture는 같은 전·후 owner cache를 사용하고 메뉴의 generic global scope를 보존합니다. 마지막 focus owner 절이 최신이며 앞 절의 해당 미완료 표기는 당시 이력입니다. 메뉴의 actual AX tree/parent/items·동일-frame 닫힘/연속 사건·전체 그래프/실기는 아직 미완료입니다.

최신2026-10-05 눌림 후속: 아래 기본 click/release 설명은 당시 구현 이력입니다. 현재는 raw hit/focus 대상의 첫 secondary press index와 실제 global 좌표로 메뉴를 열고 그 이전 owned 입력만 처리합니다. 실제 plain/SGR 원본 DOM 측정과 press-only PTY 검사는 마지막 절이 정본이며 native Focus1004/actual menu owner/return은 여전히 미완료입니다.

- 원본은 `src/features/terminal/terminal-context-menu.tsx`, `src/widgets/terminal-pane/{terminal-pane,terminal-session}.tsx`, `terminal-split-availability.ts`입니다. copy/paste/selectAll/clear, 네 방향 split, same-pane new terminal, kill의 순서와 locale key를 유지합니다. 새 terminal title은 `terminal.title`이며 new는 cwd를 지정하지 않습니다. SearchAddon은 load/dispose만 있으므로 새 검색 UI를 추가하지 않습니다.
- `native/taide-native-app/src/terminal_surface.rs`: 같은 Core의 selection/clear와 실제 egui popup·Escape/선택 후 terminal focus 복구를 연결했습니다. Shift+F10은 원본 xterm 키로 전송하며 예전 가로채기/keyboard anchor는2026-10-05에 제거했습니다. 기본 단일 secondary click에서는 raw 열림 boundary 앞의 owned 입력을 먼저 처리하고 이후 메뉴 입력을 분리합니다. copy snapshot도 prefix admission 뒤 읽습니다. 네 방향을 숨기지 않고 크기 미달 축을 비활성으로 표시하며 disabled click은 메뉴를 닫지 않습니다. 숨김/viewport 제거/세션 교체/취소 시 메뉴와 paste lifetime을 폐기합니다. 전체 메뉴 focus/OS contextmenu 시점은 아래 미완료 범위입니다.
- `src/host.rs`/`src/application.rs`: bounded HostCommand queue와 tracked blocking worker의 typed clipboard read/reply를 추가했습니다. reader/writer는 테스트에서 주입 가능하며 OS clipboard는 명시적 메뉴 선택 전까지 만들거나 읽지 않습니다. 빈/비텍스트 clipboard는 빈 paste 의도로 전달합니다. read 결과 String capacity는 64KiB 이하만 reply로 허용합니다. 기존 NativeInput::Paste 정규화/bracketed/Outbox/Receipt 경로를 사용합니다.
- `src/terminal_tabs.rs`: 메뉴의 project/pane/tab/session과 active identity를 owned mutation 안에서 재검사합니다. split은 새 terminal을 원자적으로 open-in-split, new는 같은 pane, kill은 기존 close/history/회수 경로입니다. live/persisted/tab cwd 우선순위와 기존 root guard를 사용하며 선택된 후보가 밖이면 cwd=None으로 root fallback합니다. stale project/pane/session·inactive target·pinned kill을 거절합니다. operation lease는 layout commit부터 child 회수까지 유지합니다.
- paste target은 viewport/pane/tab/session과 Weak mount lifetime입니다. read 전 죽은 lifetime을 거절하고 reply 시 같은 view lifetime·실제 active layout/session·Running을 재검사합니다. 다른 탭으로 fallback하지 않습니다. 같은 view의 연속 요청은 last-request 방식으로 버리지 않습니다.

공식 API 근거는 [arboard 3.6.1 Clipboard](https://docs.rs/arboard/3.6.1/arboard/struct.Clipboard.html)와 고정 egui 0.36.2 registry의 `response.rs`/`containers/{popup,area}.rs`, arboard registry source입니다. egui 온라인 페이지 조회는 실패해 고정 source의 문서·실제 구현을 대조했습니다. 새 dependency·unsafe·suppression은 없습니다.

## 검증과 정정

아래2026-10-03 keyboard popup/anchor 성공은 당시 실행 이력입니다. 원본 xterm과 다른 Shift+F10 가로채기임을2026-10-05 source 검토·actual PTY에서 확인했으므로 해당 부분을 현재 parity 완료 근거로 사용하지 않습니다. 현재 메뉴 입력 경계의 증거는 이 문서 마지막 후속 절입니다.

공통 Cargo 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 아래는 서로 다른 위험을 덮는 좁은 검사이며 같은 성공을 세 번 반복한 결과가 아닙니다.

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test terminal-host clipboard_paste -- --nocapture`: 1 PASS(compile 2.14초, suite 0.15초). 합성 reader의 empty/oversize·active identity·hidden lifetime 거절·stale read 전 port 호출 없음·새 유효 대상의 literal continue 수신·exit 0/join/task 0을 한 PTY에서 확인했습니다. 초기 compile의 fixture Tab `view_state` 누락과 첫 runtime의 empty paste epoch 기대 0≠실제1을 정정했습니다. 빈 paste도 사용자 의도 epoch를 증가시키는 기존 입력 동작을 변경하지 않았습니다.
- [x] `--test terminal-host terminal_menu -- --nocapture` 당시 단일 해당 검사: 1 PASS(compile 6.13초, suite 0.37초). keyboard popup의 7개 최상위 표시·Escape focus 복구/user epoch 불변·네 방향 fresh terminal/원자적 revision·승인 cwd·외부 cwd root fallback·same-pane new cwd=None·inactive/session/pane/project stale target 거절/layout 불변·실제 kill/reap/task 0을 확인했습니다. 최초 runtime의 `missing menu item terminal.copy`는 egui 첫 sizing pass가 invisible임을 fixture에서 누락한 결과였습니다. source 대조에서 다음 프레임의 keyboard anchor 누락도 확인해 저장하도록 수정했습니다. sizing fixture와 production anchor 변경을 동시에 적용했으므로 처음 실패의 원인을 anchor 하나로 단정하지 않습니다. 다음 `terminal_menu_actions` 추가 뒤 넓은 filter를 다시 돌리지 않았습니다.
- [x] `--test terminal-host terminal_menu_actions -- --nocapture`: 1 PASS(compile 5.05초, suite 0.53초). 실제 egui primary press/release로 disabled copy 유지/미전송→selectAll→copy의 CJK/astral/NFD 내용→local clear/current row→paste target command→같은 actual PTY continue→exit 0/join/task 0을 연속 확인하고 매 close 후 같은 terminal focus를 확인했습니다. 처음 두 번째 open의 `missing terminal.selectAll`은 fixture가 F10 key-up을 누락해 고정 egui가 repeat=true로 판정한 결과입니다. 실제 눌림/해제 쌍으로 정정하고 production non-repeat 정책은 유지했습니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --lib --test terminal-host -- -D warnings` 최종 exit 0(1.40초), authored terminal/app 9파일 exact `rustfmt --edition 2024 --check --config skip_children=true`와 `git diff --check` exit 0입니다. Wry inherited 17 warning은 authored 실패와 구분합니다. 새 reply를 잘못된 web bridge match에 넣은 초기 compile E0308은 실제 HostBridge match로 이동한 뒤 check exit 0(1.79초)이며 메뉴 기본 check도 exit 0입니다.
- [x] 앞선 local clear Core/SharedTerminal/actual PTY·static 결과는 [terminal-clear QA](./2026-10-03-m8-native-terminal-clear.md), hidden input/wheel·writer/query/focus 성공은 각 기존 QA로 재사용합니다. 이 변경으로 기존 OS 검사·보호 bundle 빌드/실행을 반복하지 않았습니다.

## 아직 미완료

disabled parent UI에서 남아 있던 popup을 닫고 잘못된 target의 popup도 즉시 폐기하는 guard를 source 검토 뒤 추가했습니다. 이 조건만 추가한 마지막 lib strict 검사 exit 0(2.01초)·해당 surface exact fmt/diff exit 0이며 정상 enabled 입력의 앞선 runtime 성공은 재사용합니다. 실제 modal 입력 검증을 수행했다는 의미는 아닙니다.

- [ ] 네 방향 submenu의 실제 pointer/keyboard 선택·크기 경계·disabled 색상, right-click mouse-report mode·다중 창/aux viewport·locale 3종/theme/pixel·modal 입력은 전체 menu/GUI gate에서 검사합니다. 현재 headless top-level 클릭과 직접 host operation 성공을 모든 submenu/OS 성공으로 바꾸지 않습니다.
- [ ] OS clipboard 반환 전 allocation/peak/RSS·여러 HostBridge/eframe clipboard 동시 접근·전체 aggregate 상한, blocking OS API 취소는 현재 logical reply/admission 상한과 다릅니다. 최종 clipboard/성능 gate에서 검사합니다. reader 오류 내용은 일반 메시지만 반환하며 clipboard 본문을 로그하지 않습니다.
- [ ] 특정 live cwd 변경/동시 root 교체·kill 중 forced failure·동일 view의 연속 async paste 전체 matrix와 이미 제출된 명령 후 닫힘의 제품 UX는 전체 수명 gate에 남습니다. 현재 stale source 검사는 layout/session identity를 덮습니다.
- [ ] URL/OSC8/file-link/project Hub·OSC133 command block·전체 VT/IME/AX/GUI·pre-Session/query raw ordering·213 view/full cutover/TS 제거·M8 상위 N1~N8는 미완료입니다.

보호 실기 bundle·기존 TypeScript/root/MSRV·사용자 데이터·실제 OS clipboard/입력기를 유지했습니다. M8 전체 완료 뒤에만 commit/push합니다.

## 2026-10-05 원본 키 정정·기본 우클릭 prefix

대상은 `src/terminal_surface.rs`·`tests/terminal-host.rs`, 실제 queue fixture `native/taide-native-terminal/tests/fixtures/session.rs`입니다. 원본 `TerminalView`는 custom Shift+Enter만 가로채고 screenReaderMode는 xterm6.0.0 기본 false입니다. 설치된 `common/input/Keyboard.ts`의 F10(121)/Shift modifier1 계산, `CoreBrowserTerminal._keyDown`의 triggerDataEvent와 cancel(event,true), Radix ContextMenu의 contextmenu handler와 MenuContent focus를 읽었습니다. 새 브라우저/OS 측정은 아닙니다. 원본에 없는 native Shift+F10 가로채기와 keyboard anchor를 제거했습니다.

- [x] 신규 `terminal_menu는_원본_shift_f10을_가로채지_않고_앞뒤_문자를_전송한다`: 실제 valid layout/owned PTY에서 원본 `con ESC[21;2~ tinue\n` exact wire·메뉴 미열림·child exit0·session/task join입니다. 최초 compile4.92초/suite3.35초/exit101에서 native 메뉴가 열렸고 실제 wire 완료 timeout이었습니다. 제거 후 compile13.71초/suite0.34초/exit0입니다.
- [x] 기존 ownership 메뉴 검사를 `terminal_menu는_실제_secondary_popup과_원자적_split_new_kill의_소유권을_보존한다`로 정정했습니다. actual Response rect 중심의 secondary press/release로 열고 기존7표시·Escape/epoch·host split/new/kill/root/identity·owned 회수를 유지합니다. actions 검사와 같은 최신 binary에서1회 실행했고 이 검사는 PASS입니다. joint suite0.05초/exit101은 actions만 실패했기 때문이며 두 검사가 모두 실패했다는 뜻이 아닙니다.
- [x] 기존 `terminal_menu_actions는_실제_click으로_disabled_copy_select_clear_paste와_focus를_연결한다`도 secondary press/release로 열도록 정정했습니다. 초기 `missing menu action terminal.copy`는 첫 release/open sizing pass의 invisible 결과를 검사한 fixture 오류입니다. 별도 다음 visible pass에서만 확인해 compile4.26초/suite0.26초/exit0입니다. 제품 action/epoch/clipboard 정책을 이 실패에 맞춰 바꾸지 않았습니다.
- [x] 신규 `terminal_menu는_우클릭_이전_문자를_잃지_않고_이후_문자를_차단한다`: 실제 `continue\n → secondary press/release → blocked`입니다. 최초 compile4.98초/suite3.40초/exit101에서 prefix wire 완료 timeout이었습니다. 기본 raw release boundary와 normalized/raw index로 prefix를 먼저 처리하고 나서 메뉴를 현재 survivor로 표시합니다. 수정 뒤 compile8.66초/suite0.44초/exit0입니다. exact prefix·메뉴 열림·정확히1회 input epoch로 suffix 미입장·child exit0/task0/owned 정리를 확인했습니다. `read_exact`만으로 suffix 차단을 주장하지 않습니다.
- [x] F10 제거 뒤 중간 app strict19.95초 성공, prefix/메뉴 표시 순서 변경 뒤 최종 `--lib --tests -- -D warnings` strict19.94초 exit0입니다. 변경 Rust3 exactfmt·diff exit0입니다. 제품TS·engine/vendor·root/Tauri·새 의존성/manifest/lock/MSRV·보호 앱·사용자 데이터/OS/clipboard/Keychain·Git은 불변입니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test terminal-host terminal_menu는_원본_shift_f10을_가로채지_않고_앞뒤_문자를_전송한다 -- --nocapture
experiments/native-shell-spike/target/debug/deps/terminal_host-3226916b98261227 --exact terminal_menu는_실제_secondary_popup과_원자적_split_new_kill의_소유권을_보존한다 terminal_menu_actions는_실제_click으로_disabled_copy_select_clear_paste와_focus를_연결한다 --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test terminal-host terminal_menu_actions는_실제_click으로_disabled_copy_select_clear_paste와_focus를_연결한다 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test terminal-host terminal_menu는_우클릭_이전_문자를_잃지_않고_이후_문자를_차단한다 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target -p taide-native-app --lib --tests -- -D warnings
```

성공 재사용: 메뉴 표시를 뒤로 이동한 최종 구현에서도 F10 branch에는 메뉴 열림이 없고 같은3개 입력/bytes를 처리합니다. helper 추출·mode1의 bounded read buffer 변경은 동일 data/환경이며 해당 성공을 재실행하지 않았습니다. prefix 검사는 helper의 epoch1을 실제 확인했습니다. 직후 F10에 불필요하게 추가한 미실행 epoch3 assert/상수는 제거했으며 pointer의 실행한 epoch1은 그대로입니다. 이 test-only 정리 뒤 같은 runtime/strict도 재실행하지 않습니다. 기존2 메뉴 fixture에는 prefix Text가 없고 이미 열린 메뉴 입력은 같은 mask/현재 survivor/action/selection 상태를 읽으므로 앞선 실제 secondary 영향2 성공을 재사용합니다. engine strict·primary/middle·기본 AX/prepared/pass/locked/passive wire의 해당 사건 raw index/상대 순서도 불변 입력에서 재사용하며 전체 GUI 검사를 대체하지 않습니다.

- [ ] 전체 menu 사건 graph는 계속 미완료입니다. egui secondary release가 아닌 실제 OS/browser contextmenu 시점·press→contextmenu→global release·Focus1004 loss/gain·메뉴 actual AX focus/return·복수 open/close와 닫힌 뒤 입력·새 window capture menu scope·current disabled/modal/viewport·Touch700ms/IME/selection 전체를 검사해야 합니다. 기본 prefix가 누름과 해제 사이에 입력이 들어오는 모든 원본 순서를 덮지 않습니다. 해당 gate는 PROCESS의 terminal menu parent와 후속46/N1~N8 0/8에 남기며 전체 완료 전 commit/push하지 않습니다.

## 2026-10-05 실제 원본 메뉴 사건·눌림 열기

대상은 `experiments/native-terminal-reference/{index.html,tsconfig.json,app/menu.tsx}`, `tools/m8-terminal-menu-source-measure.ts`, native `src/terminal_surface.rs`와 `tests/terminal-host.rs`입니다. 제품 TS를 수정하거나 xterm/Radix를 모방하지 않고 실제 TerminalView/TerminalContextMenu·기존 addons/React Compiler/i18next를 구동합니다. onRestoreFocus는 실제 attachRef.focus, synthetic output은 attachRef.write, clipboard 동작은 기록만 하며 OS clipboard를 만들거나 읽지 않습니다. 공식 API는 [Playwright Mouse](https://playwright.dev/docs/api/class-mouse), [xterm Terminal](https://xtermjs.org/docs/api/terminal/classes/terminal/)와 설치 source/고정 egui popup source를 확인했습니다.

- [x] `bunx tsc --noEmit -p experiments/native-terminal-reference/tsconfig.json` exit0입니다. 최초 splitAvailability에 원본 타입에 없는 up/down을 넣은 fixture TS2353은 실제 SplitEdge의 top/bottom으로 수정했습니다. 제품 타입을 완화하지 않았습니다.
- [x] build1회: `bun tools/m8-terminal-menu-source-measure.ts /private/tmp/taide-terminal-menu.Ppxh7N /private/tmp/taide-tooltip-source.J9DufD/built/assets/index-DEfYI-e9.css --build-only`, Vite381ms/exit0입니다. configFile/envDir/publicDir=false이며 기존 stylesheet SHA256 `b66166f52e29a2ffd85e4eaf679cc6ba3bc72b6580ae57f5ef8485edff53a177`을 재사용합니다. 큰 synthetic bundle 경고는 테스트 번들 경고이지 제품 성능 합격이 아닙니다.
- [x] 같은 빌드의 `--reuse-build` 원본 측정1회/exit0입니다. Chrome154.0.8037.95·darwin/headless·임시 profile·mock Keychain·service workers 차단·exact invalid origin의 로컬 assets 외 request abort입니다. 기존 sandbox EPERM 근거로 scoped Chrome 실행만 escalation했고 빌드를 다시 하지 않았습니다. 원본 plain와 SGR1000/1006을 각1회 기록한 결과는 `/private/tmp/taide-terminal-menu.Ppxh7N/menu.json`입니다. 문서에 압축한 관찰값이 정본이며 임시 파일은 제품 입력이 아닙니다.
- [x] trusted CDP pointerdown/mousedown/contextmenu는 모두 secondary down 호출 안에 발생했습니다. 메뉴는 down snapshot에서 이미 존재하고 activeRole=menu입니다. 그 전 owned `con` 뒤, SGR에서는 `ESC[<2;30;9M`→메뉴 open→textarea blur→`ESC[O`→menu focus입니다. up snapshot은 메뉴를 유지하고 document mouseup을 통해 `ESC[<2;30;9m`을 전송합니다. plain에는 mouse report가 없고 같은 blur가 있습니다. Escape keydown은 메뉴를 닫으며 source CSS Presence의 실제 detach 뒤 onRestoreFocus→textarea focus→`ESC[I`→`tinue`로 복구됩니다. Focus1004 활성화 시 source의 초기 `ESC[O`도 그대로 기록했습니다. 이 자료는 macOS Chrome의 trusted CDP/DOM 경계이지 실제 물리 입력/WKWebView/모든 OS 실측이 아닙니다.
- [x] 기존 prefix 검사를 release 없는 `continue\n → secondary down → blocked`로 강화했습니다. 제품 RED는 compile7.09초/suite0.31초/exit101이며 menu_opened=false였습니다. raw event별 engine hit/focus target·global response rect와 실제 press 좌표를 확인하고, 첫 열림 이전 owned prefix만 처리합니다. Popup의 기본 release 재열기를 open_memory로 덮고 실제 pointer 좌표를 MenuSnapshot에 보존해 다음 frame도 같은 위치를 사용합니다. 이전의 잘못된 keyboard anchor 복원은 아닙니다. 최종 compile6.09초/suite0.36초/exit0이며 실제 prefix exact wire·menu 열림·input epoch1·child exit0/owned join입니다.

수정 중 실패는 별도 보존합니다. private Memory.open_popup_at을 사용한 최초 compile E0624는 공개 Popup.open_memory/at_position으로 바꿨습니다. 첫 실행은 ui.input의 Context lock 내부에서 keyboard_focus_request_at을 재진입해 10초 deadlock panic(compile11.07초/suite10.35초/exit101)이었습니다. raw events를 lock 밖에 복사한 뒤 engine 조회를 하도록 근본 원인을 수정했으며 검사기/lock 정책을 우회하지 않았습니다. 같은 실패 가정 반복이 아니라 확인된 compile/API와 lock 원인을 각각 수정한 것입니다.

최종 runtime 영향은 최신 동일 binary에서 두 exact 메뉴 ownership/actions만1회 실행해 suite0.15초/exit0입니다. 이전 release-open 정책이 바뀌어 이 두 검사는 새 영향 검사였으며, F10/AX/prepared/mousedown/passive wire 등 변경 없는 성공을 또 돌리지 않았습니다. `cargo clippy ... -p taide-native-app --lib --tests -- -D warnings`24.40초/exit0, Rust2 exactfmt·신규 TS/HTML/JSON prettier·git diff --check exit0입니다. inherited Wry17 warning은 그대로이고 vendor/API 공개 범위를 늘리지 않았습니다.

남은 실제 제품 범위: menu content의 AX/keyboard owner 배정·Focus1004 loss/gain wire·menu가 열린 동안 global release report/다음 press/연속 open-close/닫힌 뒤 입력·window capture owner·새 target/current disabled/modal/viewport·다른 플랫폼의 contextmenu 시점·Touch/IME/물리 GUI입니다. source에서 관찰한 이 순서를 native가 구현했다고 쓰지 않습니다. 이번 prefix fixture는 mouse/focus reporting을 켜지 않은 합성 PTY입니다. full M8/N1~N8 0/8·active·전체완료 전 Git 없음이며 개인 profile/Keychain/clipboard/OS 설정·보호 앱·root/Tauri·제품 TS·의존성/manifest/lock은 변경하지 않았습니다.

## 2026-10-05 기본 menu focus owner·report/return

대상은 native `src/terminal_surface.rs`, `tests/terminal-host.rs`, terminal queue fixture와 이미 patch된 vendor egui의 `src/{context,pass_state}.rs`입니다. 원본 DOM plain/SGR 자료는 바로 앞 절의 성공을 재사용하며 브라우저를 다시 실행하지 않았습니다.

실제 scene에 pointer-focus trigger와 메뉴 owner ID를 pass 수명으로 선언합니다. 다음 viewport pass의 기존 enabled/focusable/interactive·layer/modal/input-region/transform hit-test가 인정한 secondary press에서만 메뉴 후행 owner를 생성합니다. press 전 terminal focus 요청과 press 후 menu focus 요청은 별개이며 Context의 local normalized/raw route·Button 기본 키·App window capture·Memory 최종 focus·terminal ordered wire가 같은 cache를 읽습니다. consumed raw focus의 local composition loss도 유지합니다. 선언만 존재하는 closed owner는 `is_context_menu_keyboard_owner`에서 false이며 처음 열리는 owner의 previous target 해석은 해당 pass에 실제 입장한 후행 요청이 있을 때만 허용합니다. 뒤 AX 요청이 있는 최종 owner를 menu 초기 focus로 덮지 않습니다. 메뉴 generic global scope는 기존 일반 control과 같이 유지하고 terminal scope로 처리하지 않습니다.

메뉴 본문에는 실제 focusable owner widget을 만들고 Menu role/Focus action을 설정했습니다. wire는 기존 Initial/PointerFocus→CapturedInput 뒤에 ContextMenuFocus loss/gain phase를 두며 같은 event의 SGR press가 loss보다 먼저 전송됩니다. 실제 global release는 기존 captured mouse 경로를 유지하며 아직 열린 메뉴의 terminal local key mask를 보존합니다. Escape close의 실제 Response.request_focus와 다음 입력 pass의 Focus gain을 사용합니다. 임의 sleep·OS key 합성·가짜 report나 새 별도 transport를 추가하지 않았습니다. 실제 AX tree/parent/items와 menu 키보드 navigation 전체가 완료됐다는 뜻은 아닙니다.

- [x] `terminal_menu는_실제_focus_owner와_press_loss_global_release_restore_wire를_보존한다`: 실제 valid layout/owned PTY에서 plain와 SGR1000/1006 각1사례입니다. RED compile7.39초/suite3.44초/exit101은 focus 보고가 부족해 child 완료 timeout이었습니다. 최종 단일 검사 suite0.39초/exit0입니다. prefix `con`, press에서 실제 keyboard focus가 terminal과 다른 owner로 이동, global release에서도 메뉴 유지, Escape의 메뉴 폐쇄·원래 terminal focus 복귀, 다음 `tinue\n`와 child exit0·owned session/task 정리를 확인했습니다. exact plain wire는 `ESC[O ESC[I con ESC[O ESC[I tinue\n`, SGR은 `ESC[O ESC[I con ESC[<2;1;1M ESC[O ESC[<2;1;1m ESC[I tinue\n`입니다. 실제 첫 cell 좌표를 사용하며 menu 중 `blocked`는 전송되지 않습니다.
- [x] `terminal_surface::tests::window_capture는_secondary_press_뒤_메뉴_owner와_후행_ax를_보존한다`: suite0.02초/exit0입니다. actual previous Button/terminal hit와 secondary press에서 pre terminal/post menu target·처음 admitted menu의 target 해석·closed 선언 단독 false를 확인했습니다. 일반 Button prefix Enter와 메뉴의 Enter는 generic global group action으로 정확히2회 처리하고 terminal local override로 오인하지 않으며 실제 Button click은 중복되지 않습니다. 후행 AX 없음/있음 두 경우의 최종 Memory menu/Button focus도 확인했습니다. context.enable_accesskit을 사용하지만 실제 제품 메뉴의 AX tree 전체 검사는 아닙니다.
- [x] 최신 동일 binary에서 실제 secondary ownership/actions 및 press-only prefix 영향3건만1회 실행해 suite0.15초/exit0입니다. Focus1004/owner 변경 때문에 이 검사는 새 영향 검사였고 F10·primary/middle·AX/prepared/locked/passive wire의 변경 없는 입력 성공은 재사용했습니다. 새 메뉴 cache가 없는 경로의 pre 요청/상대 순서는 유지하며 전체 영향 gate를 대체하지 않습니다.
- [x] `cargo clippy ... -p egui -p taide-native-app --lib --tests -- -D warnings`28.11초/exit0입니다. authored Rust3 exactfmt, vendor2파일 rustfmt parse-only/비재포맷, diff exit0입니다. registry와 공통116파일을 byte 비교해106동일/10변경을 확인했고 MIT/native-only patch·vendored authored99% 제외는 유지합니다. standalone UI manifest/registry egui·root/Tauri·의존성/manifest/lock/MSRV·제품TS·보호 앱/OS/clipboard/Keychain·Git은 이 작업에서 바꾸지 않았습니다.

### 빌드 경로 정체 분리

old target에서 수정 후 검사 세션15686은 rustc가 컴파일 전 SearchPath::new→ReadDir→getdirentries64에 대기했습니다. 소유 PID cargo32328/rustc32347의1초 sample86/86회와 낮은 CPU 시간·target/debug/deps directory metadata size13,246,144바이트를 확인했습니다. 이 값으로 파일 수나 APFS 결함을 단정하지 않습니다. 정확한 소유 실행 파일 재확인 후 두 PID에 SIGINT를 보내 exit130으로 종료했습니다. 외부 앱/사용자 프로세스를 종료하지 않았습니다.

같은 old target의 scoped unsandboxed 세션46262도 cargo32533/rustc32534의 sample85/85회가 같은 열거 지점이었습니다. 따라서 sandbox 권한 원인이라고 단정하지 않으며 이를 성공/제품 RED로 세지 않습니다. 정확한 두 PID만 SIGINT·exit130으로 종료했습니다. 이후 기존 cache를 삭제/이동하거나 wrapper로 우회하지 않고 Cargo의 정식 --target-dir 옵션에 새 `/private/tmp/taide-m8-menu-build.j6Efnw`를 지정했습니다. 같은 manifest/locked/offline의 `--lib --test terminal-host --no-run`이1분13초/exit0으로 끝났습니다. 실제 검사는 위 두 exact binary 실행과 영향3이며 no-run을 runtime PASS로 쓰지 않습니다. generated diagnostics는 `/private/tmp/taide-terminal-menu.Ppxh7N/rustc{,-second}.sample.txt`이고 프로세스 인수/환경변수·파일 본문·키를 수집하지 않았습니다. 새로운 성능 baseline 측정이 아닙니다. 새 빌드 target은 다음 관련 검사에서 재사용하며 old target과 보호 bundle은 보존했습니다. 현재 모든 해당 세션은 종료됐습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p taide-native-app --lib --test terminal-host --no-run
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/taide_native_app-6ae1e98e5aed8a62 --exact terminal_surface::tests::window_capture는_secondary_press_뒤_메뉴_owner와_후행_ax를_보존한다 --nocapture
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/terminal_host-3226916b98261227 --exact terminal_menu는_실제_focus_owner와_press_loss_global_release_restore_wire를_보존한다 --nocapture
/private/tmp/taide-m8-menu-build.j6Efnw/debug/deps/terminal_host-3226916b98261227 --exact terminal_menu는_실제_secondary_popup과_원자적_split_new_kill의_소유권을_보존한다 terminal_menu_actions는_실제_click으로_disabled_copy_select_clear_paste와_focus를_연결한다 terminal_menu는_우클릭_이전_문자를_잃지_않고_이후_문자를_차단한다 --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw -p egui -p taide-native-app --lib --tests -- -D warnings
```

- [ ] menu actual AX tree/parent/items·full keyboard/submenu·window→document capture 전체, 복수 press/open/close/동일-frame Escape 뒤 input, 다른 view/current disabled/hidden/modal/viewport topology·다른 OS/WKWebView/Touch/IME/실기, full M8/N1~N8는 미완료입니다. 기본 plain/SGR 분리 pass와 final AX 성공으로 이 범위를 완료 처리하지 않습니다.
