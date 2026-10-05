# M8 production application ports owner

## 2026-10-05 실제 App constructor/start/Settings/Exit 연결

대상은 `application.rs`, `application-ports.rs`, `remote-assets.rs`, 새 `application-startup-tests.rs`와 loader fixture입니다. NativeApplication이 실제 bootstrap GitEvents·같은 서비스/Tabs Hub·현재 Views.remote_effects·실제 environment provider·live Settings scrollback·실제 AppInfo를 ApplicationPorts에 조립해 보관합니다. 기존 main/New signature와 격리 data-dir 정책을 유지했습니다. HostBridge 최초 생성과 close 실패 후 재연결은 같은 owner reconcile을 전달합니다. App 구성의 fallible 단계가 모두 성공한 뒤 start를 await하며 IDE→hooks→remote 순서는 기존 실제 Integrations입니다. Exit는 같은 stop_services의 remote→hooks→IDE 순서와 기존 drain을 사용합니다.

Catalog.packaged는 executable bundle/인접 remote-public의 엄격한 manifest를 요구합니다. 실제 Rust UI 자산이 없으면 원격 시작은 실패하며 빈/fixture/legacy TS 자산을 제품 fallback으로 넣지 않습니다. bundle 형식·header는 실제 Rust UI 구현의 증명이 아니며 실제 생성/패키징/browser 기능은 여전히 미완료입니다.

1. 최초 actual constructor 검사: compile13.29초/suite1.14초·FAIL입니다. 별도 기존 Exit의 loopback bind가 EPERM을 반환해 sandbox 원인을 확인했습니다. 해당 최초 결과를 성공으로 세지 않습니다.
2. localhost 권한 승격 뒤 actual constructor/기존 Exit2 exact: compile8.57초/suite10.41초·Exit1 PASS/새 driver1 FAIL입니다. timer 생성도 runtime 진입 안으로 정정했습니다. 내부 poll만 호출한 driver는 종료 완료 처리까지 돌지 않았습니다. 단계 진단 실행 compile8.47초/suite10.38초·FAIL에서 `app-exit/remote=false/shutdown=true/closing=true/host=false/tasks=0/status=None`을 확인했습니다. timeout을 늘리거나 제품 Exit를 바꾸지 않았습니다.
3. driver를 실제 eframe::App::logic/background_tick로 정정하고 실패한 검사만 실행: compile10.22초/suite0.41초·고유1 PASS입니다. 실제 public constructor→valid synthetic bundle 로딩/loopback 시작·IDE/hooks 비활성·metadata 비공개, App read/EditorStore transaction/CmdS save→실제 필수 reconcile await/서버 정지/live·disk 동일/clean/pending dirty false, 실제 close/App loop 완료/기존 on_exit·Ports/Hub weak 해제/task0을 확인했습니다. CreationContext/Frame의 toolkit kittest 경계이며 실제 창/GPU/OS 키 입력 검사로 확대하지 않습니다. 임시 synthetic8바이트 wasm과 HTML은 테스트 전용이며 완성된 Rust UI 자산이 아닙니다.
4. bundle/fixed loader2 PASS(12.68초/.02초)와 Catalog.prepare 취소·실패→성공 영향2 PASS(.02초)는 remote-assets QA 맨 위가 정본입니다. success한 Exit/constructor/원격 query/다른 Host·Views 결과는 재사용했습니다.
5. 마지막 native lib/bin/tests clippy `-- -D warnings` exit0/3.94초·authored5 rustfmt/check·tracked diff/check exit0입니다. Wry17 dependency warnings와 debug lib-test ld unwind 경고는 별도이며 억제/환경 변경은 없습니다. Cargo는 locked/offline/기존 CARGO_HOME·/private/tmp/taide-m8-menu-build.j6Efnw에서 직렬입니다.

PROCESS 후속47은2/4(50%)·전체363/433(83.83%, 비가중·공수비 아님)·최종 N1~N8 0/8·ETA 공수 미확정·goal active·전체완료 전 Git 없음입니다. 이제 실제 App의 Settings AppFile 저장은 기존 무통합 Forbidden이 아닌 실제 owner callback을 사용합니다. 아래의 미배선/Forbidden 기록은 선행 상태입니다. 실제 Rust 원격 UI 생성/패키징·전체 query handoff·failed-close 재연결 실측·enabled IDE/hooks 홈 통합·전체 remount/GUI/성능/beta/cutover/Rust99%는 미완료입니다. 보호 bundle/cache/사용자 데이터/OS 설정/입력기/VoiceOver/clipboard/Keychain·제품TS/vendor/의존성/manifest/lock/MSRV/Git은 이 slice에서 불변이며 임시 합성 fixture만 생성/회수했습니다.

최종 native5·신규 bug·QA2·resume의 총9파일 no-index whitespace/check는 진단 출력 없이 exit1(신규 diff)이었습니다. 현재 브랜치는 readonly 조회로 to_rust_native를 확인했으며 Git 쓰기는 하지 않았습니다.

## 2026-10-05 실제 Views의 원격 효과 제공

대상은 `terminal_surface.rs`, `remote-terminal.rs`, `application.rs`와 관련 fixture입니다. Views.remote_effects는 같은 palette의 현재 command 색상과 같은 egui Context의 repaint를 제공하며, 원격 renderer query를 대신하지 않는 ObservePorts만 생성합니다. 원래 local effect_ports와 마찬가지로 별도 Bell/stream 부가 동작은 없고 Dispatcher가 metadata/command clock/agent/event 관찰을 소유합니다. 임의 색상·pixel geometry 공급자나 새 사용자 기능을 추가하지 않았습니다.

Effects factory의 반환을 AppResult로 바꿔 미준비/poisoned palette를 remote 경계에서 오류로 전달합니다. 거절 시 initial channel은 Drop되고 environment future/child/actor를 시작하지 않습니다. 실제 NativeApplication constructor는 이미 resolve한 terminal_appearance로 같은 Views palette를 초기화합니다. 첫 terminal 화면을 그릴 때까지 palette를 기다리지 않습니다. 기존 theme refresh/set_palette caller는 그대로 재사용합니다.

`cargo test --lib production_views_effects는_palette_실패를_spawn_전에_거절하고_현재색과_repaint를_공유한다`: compile8.74초/suite0.04초·고유1 PASS입니다. actual Views factory·same Hub·remote pty_spawn/write/attach/kill 경로에서 미준비 palette 거절/환경0/channel Drop/task0, 준비 후 실제 `/bin/cat` 출력/환경1·색상 snapshot, set_palette 뒤 새 factory 호출 색상 변경·기존 snapshot 보존·Views 폐기 후 callback/context 및 palette 수명·root idle/task0을 확인했습니다. fixture environment는 합성이며 OS 환경 provider 호출이나 실제 App GUI 실행을 주장하지 않습니다.

최종 lib/bin/tests clippy `-- -D warnings` exit0/3.33초·이번 authored5 rustfmt/check·tracked diff/check exit0입니다. 미추적 native 변경8파일의 no-index/check는 출력 없이 exit1(신규 diff)입니다. Cargo는 locked/offline/CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo/target=/private/tmp/taide-m8-menu-build.j6Efnw에서 직렬이며 앞선 query/dispatcher/actor/remote 성공은 같은 동작을 다시 검사하지 않고 재사용했습니다. 기존 Wry17 dependency warnings는 authored strict와 구분합니다.

실제 NativeApplication은 여전히 ApplicationPorts owner/start를 생성하지 않고 legacy HostBridge constructor를 사용합니다. 공개 Rust UI 생성/패키징·같은 Hub/env/history를 포함한 owner의 actual App 조립·Settings reconcile/Exit·전체 query handoff/GUI는 미완료입니다. 이 factory 성공만으로 실제 앱의 원격 서비스가 완성됐다고 주장하지 않습니다. 후속47은1/4(25%)·전체362/433(83.60%, 비가중/공수비 아님)·N1~N8 0/8·전체 ETA 미확정·goal active·전체완료 전 Git 없음이며 사용자 OS/보호 bundle/cache/제품TS/vendor/dependency/lock/MSRV는 보존합니다.

## 2026-10-05 HostBridge reconcile 제공 경계

대상은 `src/host.rs`, `src/application-ports-tests.rs`, `tests/settings-controls.rs`입니다. HostBridge에 `connect_with_application_ports`를 추가해 기존 owner의 reconcile을 그대로 받으며, `connect_with_settings_ports`는 합성 clipboard/terminal과 필수 reconcile을 받는 검증 경계입니다. Settings·theme·양쪽 font·keymap 저장은 같은 callback을 await한 뒤 기존 이벤트를 발행합니다. terminal 포트와 reconcile은 동일 worker의 HostIntegrations에 보관하고 disconnect에서 회수합니다. 새 의존성·새 설정 프로토콜은 없습니다.

기존 constructors의 무통합 동작은 호환을 위해 유지했습니다. **실제 NativeApplication은 아직 connect_with_terminals를 사용하므로 이 변경만으로 실제 앱의 서버 toggle이 고쳐졌다고 주장하지 않습니다.** 실제 App owner/assets/effects/start/재연결/Exit와 AppFile save는 후속47의 미완료 항목입니다. 빈 resolver나 합성 effects를 생산용 입력으로 만들지 않았습니다.

- settings-controls의 기존 host 검사를 확장해 live/disk 저장→비동기 reconcile 완료→SettingsChanged/ThemeChanged, 이전/다음 Settings 전달, font2·keymap과 잘못된 경로/없는 theme/저장 실패의 callback 미호출·worker/task0을 확인했습니다. 최종 `cargo test --test settings-controls native_settings_controls_host -- --nocapture`는 compile8.74초/suite0.16초·고유1 PASS입니다.
- 기존 production graph 검사를 Tabs/동일 Hub로 조립하고 새 production HostBridge constructor에서 실제 Ports.reconcile을 소비하도록 확장했습니다. 실제 bootstrap·dispatcher 저장 성공은 유지하며 host terminal-font 저장→native/remote 이벤트·disabled IDE/hooks/remote 유지·asset read0·disconnect와 owner 회수를 확인했습니다. `cargo test --lib application_ports::tests::production_graph -- --nocapture`는 compile18.33초/suite0.02초·고유1 PASS입니다. enabled 서버/OS/전체 App startup 성공으로 확대하지 않습니다.
- Cargo는 기존 manifest·locked/offline·CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo·target=/private/tmp/taide-m8-menu-build.j6Efnw에서 직렬 실행했습니다. 첫 settings-controls 실행은 output만 전달하면서 진행 handle을 보존하지 못해 최종 종료 결과를 회수하지 못했습니다. 그 결과를 성공으로 세지 않으며, HostIntegrations 및 keymap 검사 수정 뒤 위 최종 검사 결과만 정본으로 사용합니다. ps는 sandbox에서 거절되어 재시도하지 않았습니다.
- host/lib와 초기 settings fixture clippy는 exit0·1.98초입니다. 이후 수정된 lib test/settings fixture와 기존 consumers를 포함한 `cargo clippy --lib --tests -- -D warnings`는 exit0·7.57초입니다. 성공 동작 검사를 이 strict 때문에 재실행하지 않았습니다. authored3 rustfmt/check·tracked diff/check는 exit0입니다. 기존 Wry17 dependency warnings는 authored 진단과 구분합니다.

관찰된 terminal_host의 can_reply는 단지 exit_code 없음입니다. 원격 subscriber 유무를 보고 query 응답을 비활성화한다는 가정은 채택하지 않습니다. 실제 geometry/palette 제공 계약과 원격 renderer의 응답 소유권을 더 확인해야 하며 임의 크기로 연결하지 않습니다. [기존 BoxFuture 공식 타입 계약](https://docs.rs/futures/latest/futures/future/type.BoxFuture.html)의 Send/owned future 경계를 유지합니다.

후속47은1/4(25%)·전체362/433(83.60%, 비가중·공수비 아님)·최종 N1~N8 0/8입니다. 이 두 직접 검사의 성공은 재실행하지 않습니다. 사용자 OS/clipboard/Keychain/home/보호 앱·제품TS/vendor/의존성/lock/MSRV/Git은 이 변경에서 불변입니다.

## 2026-10-05 actual caller 대조

실제 `main.rs`는 NativeApplication::new를 호출하고 `application.rs`의 new는 같은 Tabs를 HostBridge::connect_with_terminals에만 줍니다. ApplicationPorts owner/with_loading_assets/start 호출은 없으며 App struct에도 owner가 없습니다. `host.rs`의 settings_set_theme와 update_control_settings는 여전히 `|_, _| async {}` reconcile을 전달합니다. 따라서 선행 factory/177 backend/Settings Integrations 성공을 실제 App startup/toggle 완료로 주장하지 않습니다.

`terminal_surface.rs`의 effect_ports는 actual Views.palette·session별 geometry·Ui repaint를 캡처합니다. remote-terminal Ports는 같은 Hub/environment/history와 opts별 필수 Effects factory를 요구합니다. `application-ports.rs`의 Catalog startup 준비·Settings reconcile·Weak owner 경로는 존재하지만 actual caller가 제공해야 합니다. Rust public UI 생성/패키징도 남으며 legacy TS dist/빈 resolver/fixture effects를 제품 fallback으로 사용하지 않습니다.

이 절은 source 확인만이며 신규 제품 수정·앱/OS 실행·검사 성공은 없습니다. PROCESS 후속47의1/4(25%)·전체362/433(83.60%, 비가중/공수비 아님)·N1~N8 0/8·goal active입니다. source 경로 조회 실패는 실제 lib의 kebab-case 매핑으로 정정했으며 old target/bundle/cache는 변경하지 않았습니다. 다음은 현재 palette/geometry의 production 효과 제공과 실제 Rust asset/caller 계약입니다.

## 대상과 구현

대상은 native `application-ports.rs`, `application-ports-tests.rs`, `lib.rs`입니다. 기존 production App 조립 pending을 세분화한 코드이며 NativeApplication의 실제 caller를 아직 대체하지 않았습니다.

- 단일 Ports constructor가 실제 agent/Git/LSP/system/Gist 포트, IDE LayoutActions, Settings IDE→hooks→remote reconcile, 전체177 dispatcher와 remote socket_action을 조립합니다. caller의 실제 terminal Hub/environment/history/effects, assets, AppInfo는 필수 입력으로 유지합니다. 제품 constructor에서 fixture/no-op assets/effects를 만들지 않습니다.
- IDE는 caller의 동일 Hub를 사용하며 sync와 preferences는 같은 reconcile을 공유합니다. Gist HTTP/OS probe/PATH/font/system/emitter는 실제 기존 구현의 lazy callback입니다. 이 검사는 실제 사용자 OS/Gist/CLI/Keychain을 호출하지 않습니다.
- Arc::new_cyclic으로 생성 시 remote Weak를 reconcile에 전달하고 즉시 upgrade하지 않습니다. remote→dispatcher→reconcile→Weak remote이므로 소유권 순환이 없습니다. IDE/Hub의 의도된 strong ownership은 dispatcher 폐기 뒤 해제됩니다.
- startup 포트는 기존 설정 snapshot으로 IDE→hooks→remote를 순서대로 await합니다. stop 포트는 remote→hooks→IDE이며 실제 기존 shutdown 함수를 사용합니다. disabled startup/이미 정지한 stop만 이번 검사에서 실행했습니다. enabled actual lifecycle·동시 시작/종료·NativeApplication 호출부는 미완료입니다.

## 최소 검증

- [x] 최초 compile은 테스트에서 AgentStore에 없는 server_info를 사용해 exit101입니다. 원본 실제 필드 agent_hooks로 정정했습니다. factory 제품 코드는 이 실패를 이유로 바꾸지 않았습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib application_ports::tests --locked --offline --target-dir experiments/native-shell-spike/target -- --nocapture`: compile9.55초/suite0.02초·고유1 PASS, filtered208입니다. 실제 bootstrap/production factory/dispatcher로 AppInfo 전달·Settings persist/live→native event/remote JSON-string payload·원격 app_exit 거절·caller 자산 전달·disabled startup/task0·stop/shutdown·remote/IDE/Hub/services 소유권 해제를 확인했습니다.
- [x] 같은 manifest의 native lib/bin/tests clippy `-- -D warnings`: exit0·13.69초입니다. Wry dependency17 warnings는 authored strict와 구분합니다. authored3 exact rustfmt·tracked diff check exit0·신규2 no-index check 출력 없음(exit1은 신규 diff)을 확인했습니다.

CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo이며 모든 Cargo는 직렬입니다. 종료 handles4269/2331/51458, live handle 없음입니다. 이전 상태의 성공/전체 suite는 반복하지 않았습니다. manifest/lock/root/Tauri/MSRV/제품TS/보호 bundle/사용자 앱/OS/home/자격 증명/TAIDE Git은 이 slice에서 불변입니다. fixture의 assets/effects/environment는 테스트 전용이며 제품 constructor는 필수 입력입니다.

## 근거와 미완료

[Rust Arc::new_cyclic/Weak](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.new_cyclic)과 기존 production adapters/Settings Integrations/dispatcher/IDE/remote lifecycle 함수를 확인했습니다. new_cyclic의 초기화 완료 전 Weak upgrade 실패 계약을 지키며 constructor 안에서 async 서비스 시작을 하지 않습니다.

- [ ] NativeApplication의 필수 자산·terminal effect 제공, 이 owner의 실제 보유, HostBridge no-op 대체/AppFile save, startup/Exit caller·IDE diff/save/selection 화면 소비자는 다음 기존 gate입니다. factory 검사만으로 전체 App 완료로 계산하지 않습니다.
- [ ] native editor LSP recovery·keybinding RED/PTY remount·N1~N8 0/8·실기/성능/최종 cutover는 남습니다. 전체 M8 완료 뒤에만 commit/push합니다.
