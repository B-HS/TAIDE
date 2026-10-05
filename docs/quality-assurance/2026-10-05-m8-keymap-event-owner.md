# M8 window keymap 사건별 owner

현재 상태: 등록된 Button/editor/terminal의 선행 window capture·known local AX demux/조합 취소·동일 session 두 terminal의 기본 Text/Focus wire 순서를 구현했습니다. prepared/pass·잠긴 탐색키 후속에 이어 마우스 이동/휠과 AX의 실제 순서 RED를 수정해 신규2건·app strict19.50초가 통과했습니다. full mixed graph·후속46·N1~N8 전체는 미완료이며 목표active·전체완료 전 commit/push 없음입니다. 서브에이전트 없이 직접 수행했습니다.

## 근거와 변경

원본 `src/shared/hooks/use-global-keymap.ts`, `src/shared/lib/keymap/keymap-context.ts`는 keydown마다 실제 DOM activeElement의 editor/terminal 조상을 읽고 window capture에서 dispatch합니다. [DOM dispatch 표준](https://dom.spec.whatwg.org/#concept-event-dispatch)과 설치된 egui0.36.2 Focus/WidgetRects 구현을 확인했습니다. 이번에는 원본 브라우저 측정이나 build를 실행하지 않았습니다.

- `native/taide-native-app/src/terminal_surface.rs`: 이전 Button-only gate를 `capture_window_keymap`으로 교체했습니다. 입력 전 owner와 root AX Focus의 실제 등록 ID마다 scope를 갱신합니다. 일반 Button, 등록된 editor response, terminal view의 viewport/ID/처리 frame/preedit만 사용하며 UI label·EventFilter·focused tab으로 종류를 추정하지 않습니다.
- 같은 raw index의 기존 Windows 결정 cache를 재사용합니다. app handler와 fallback의 결정 처리는 공용 함수로 유지하고 선행 local target의 unsupported ResolveChord는 소비하지 않습니다. 실제 terminal/editor listener가 같은 cached 결정을 처리할 수 있습니다.
- `application.rs`: 실제 `NativeEditor` response ID를 viewport/ViewId/frame과 연결했습니다. 활성 pane/tab·실제 Store view 및 현재/직전 frame을 확인하고 composition을 읽습니다. App의 기존 modal/busy/shutdown/keybinding capture guard 및 Provider.begin 이전 순서는 유지했습니다.
- engine Context/Memory: 입력 시작 focus를 이전 frame focus와 분리했습니다. 예약된 다음-frame focus를 적용한 뒤 raw key 처리 전 ID를 저장하며 pending Button도 Escape 선행 blur 제외 대상에 반영합니다. previous enabled/focusable/interactive widget의 AX node와 Button default 등록을 읽기 전용으로 조회합니다. generic/menu Escape 정책은 그대로입니다.

pointer/Touch/WindowFocused/IME·미확정 navigation·AX Click처럼 owner/default 동작이 아직 확정되지 않은 사건 뒤에는 이전 owner를 재사용하지 않습니다. 아래 잠긴 탐색키 후속에서 초기 실제 filter 안의 navigation만 owner를 유지하도록 연결했습니다. 이후 알려진 AX Focus는 다시 실제 등록 target으로 연결합니다. 알 수 없는 대상은 기존 target/fallback 경로에 남깁니다. 이 제한을 전체 DOM 입력 graph 구현으로 주장하지 않습니다.

## 실행 결과

기본 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, native manifest·`--locked --offline --target-dir experiments/native-shell-spike/target`입니다. Cargo는 직렬 실행했습니다.

- [x] 최소 RED: 새 AX Focus 뒤 Enter가 window action보다 먼저 실제 Button을 클릭했습니다. 첫 compile9.81초의 `--exact`에 module prefix가 빠져 0tests였으며 이를 성공으로 세지 않았습니다. 같은 binary에서 정확한 `button_key_tests::app_window_capture는_ax_focus_뒤_button_default_action보다_먼저_소비한다` 1건은 exit101/suite0.01초, `key=Enter reverse=false`로 실패했습니다.
- [x] owner 수정 후 compile12.96초·새 AX/기존 stable/unknown guard3건 PASS, suite0.06초입니다. 이후 enabled target/pending focus 처리 변경의 영향 때문에 최종 영향 묶음에서 해당 경계를 다시 확인했습니다. 같은 상태의 무의미한 반복이 아닙니다.
- [x] terminal/editor/button scope·composition 및 unsupported terminal chord1건 PASS, compile7.96초/suite0.03초입니다. 테스트 fixture의 `Pos2` 미수식 E0433을 `egui::Pos2`로 수정했습니다. 실제 egui target과 production Views/cache를 사용하지만 PTY 출력·실제 OS AX 검사는 아닙니다. 예약 focus 변경 전의 이 검사는 id_next_frame이 없는 입력이라 입력 시작 ID가 동일하며 성공을 재사용했습니다.
- [x] 실제 NativeEditor 기존 검사에 선행 capture 등록을 연결했습니다. 이 테스트 수정만의 no-run compile7.23초 뒤 아직 실행하지 않았으며, pending focus 수정과 합쳐 compile14.99초 후 최종 binary에서 실행했습니다.
- [x] 최종 정확한9건 PASS, suite0.06초: 새 예약 Shift+Tab1·새 AX Button1·기존 stable capture/unknown guard2·Button ordered AX1·Tooltip window/document Escape/완료 click2·실제 NativeEditor chord1·동일 raw window listener cache1입니다. 별도 scope/chord 성공1과 합쳐 고유 신규3/영향7입니다.
- [x] `cargo clippy ... -p egui -p taide-native-app --lib --tests -- -D warnings` exit0, 22.04초입니다. 기존 Wry dependency warning17개는 그대로이며 검사기를 끄지 않았습니다.
- [x] 수정 Rust7파일 exact rustfmt check exit0이며 문서 갱신 후 `git diff --check`도 exit0입니다.

최종 binary는 `experiments/native-shell-spike/target/debug/deps/taide_native_app-6ae1e98e5aed8a62`입니다. 실제 새 검사는 다음 정확한 필터입니다.

```text
--exact button_key_tests::window_capture의_입력전_focus는_예약된_역방향_tab을_반영한다
--exact button_key_tests::app_window_capture는_ax_focus_뒤_button_default_action보다_먼저_소비한다
--exact terminal_surface::tests::window_capture는_사건별_terminal_editor_button_scope와_로컬_chord를_보존한다
```

## Local AX 입력 demux·조합 수명 후속

이 절은 위 선행 capture의 후속입니다. 새 OS/브라우저 측정 없이 기존 원본 코드·설치된 egui Focus/Ime 계약을 사용했습니다. 실제 제품 `AppSurfaces::show_document`는 File/Untitled/AppFile의 공통 editor 경로이며 `show_with_input_route`에 actual Response ID 기반 route를 주입합니다. `taide-native-ui`의 독립 workspace/registry egui와 기존 `show`/`show_with_keymap`은 유지했습니다. UI가 native patch API를 직접 호출하지 않습니다.

- Context의 기존 normalized/raw matcher를 App과 공유했습니다. root AX Focus가 없는 batch는 route None으로 기존 입력 정책을 유지합니다. AX가 있으면 입력 시작 ID와 raw Focus로 현재 normalized survivor의 Keyboard/Text/Copy/Cut/Paste/IME owner를 배정합니다. consumed navigation은 focus transition으로 재사용하지 않고 surviving pointer/Touch/navigation/AX Click과 window regain은 unknown fallback에 남깁니다. IME 자체는 local 상태이며 owner를 없애지 않습니다.
- 실제 editor는 최종 response focus가 없어도 자신의 prefix를 처리하고 다른 owner 사건은 보존합니다. 조합을 먼저 버려 prefix commit을 잃지 않으며 final known focus를 caret/preedit/플랫폼 IME 렌더에 사용합니다.
- 실제 terminal의 running/attaching 버퍼는 동일한 owner filter를 사용합니다. 기존 keymap/pointer 순서·enabled/running/menu guard는 유지합니다. 같은 viewport에서 다른 target이 아직 처리할 local prefix가 있으면 선행 blur를 연기합니다. 다른 viewport ID는 현재 batch의 owner를 빌리지 않습니다. 이 정책은 여러 terminal view의 focus-report wire 순서 전체를 보장한 것으로 세지 않습니다.
- 추가 RED에서 최종 focus만으로 취소하면 `editor0→editor1→editor0`의 old preedit가 남았습니다. Context는 known focus loss를 raw index별로 보존하고 다음 survivor/배치 끝에서 취소 신호를 전달합니다. editor의 ime_revision/Store composition과 terminal의 preedit를 사건 전에 취소합니다. consumed AX의 loss 전달도 코드에 포함했으나 그 variant의 별도 runtime 검사는 아직 없습니다.

### 실제 결과와 재사용

아래는 고유 신규4·기존 영향8입니다. Text/IME fixture를 변경한 검증을 같은 입력의 반복 측정으로 세지 않습니다. Cargo는 동일 native 환경에서 직렬 실행했고 모든 PTY는 합성 fixture·owned Hub/task로 종료/회수했습니다.

- [x] 실제 두 editor의 prefix 오배정 RED: compile7.80초/suite0.02초/exit101, `reverse=false view0`가 `beforeafter`이고 기대는 `before`였습니다. 구현 후 plain Text GREEN compile13.20초/suite0.03초입니다. `Id::accesskit_id` map의 owned/ref E0631은 closure로 수정했습니다. 이어 다른 IME fixture는 compile6.24초/suite0.04초 PASS였습니다.
- [x] 실제 PTY/editor prefix plain Text GREEN compile17.91초/suite0.54초, IME commit·suppressed Text fixture는 compile6.47초/suite0.47초 PASS였습니다. 각 정/역 draw에서 fixture가 정확한 `continue\n`을 받아 Phase::Exited(Some(0)), editor는 `after`, owned task 회수를 확인했습니다. 마지막 동일-viewport guard 변경은 이 입력들의 predicate가 동일해 성공을 재사용했고 strict로 컴파일했습니다.
- [x] 기본 영향6: actual NativeEditor chord·same raw listener cache·Button AX capture·Tooltip window/document Escape4건 suite0.04초 및 실제 PTY hidden/view focus·measured attach/IME2건 suite0.07초 PASS입니다. 기본 demux 당시 app strict `-p egui -p taide-native-app --lib --test terminal-host -- -D warnings` exit0/6.16초는 lib unit fixture 전체 strict로 주장하지 않습니다.
- [x] 독립 UI default-API 검사: 새 코드 `iter().any()`의 Clippy manual_contains 실패를 `contains()`로 수정해 독립 lib/editor_surface strict exit0/0.95초입니다. 기존 actual 입력/선택/IME/stale1건 compile4.48초/suite0.02초·disabled editor1건 direct suite0.01초 PASS입니다. 후속 focus-loss 메타데이터 변경에도 default route None의 동작은 동일하며 이 성공을 재사용했습니다.
- [x] 추가 실제 editor focus 왕복/미확정 조합 RED: compile9.59초/suite0.02초/exit101, composition.is_none 실패입니다. raw focus-loss 취소 구현 후 같은 정확한 신규1건 compile12.94초/suite0.04초 PASS입니다. 기존 prefix IME와 chord/cache의 변경 영향3건은 같은 lib binary에서 suite0.02초 PASS입니다. 이 binary 뒤 loss counter를 usize로 명시했지만 작은 raw batch의 loss/ownership 값은 같아 성공을 재사용했고 최종 strict는 현재 코드를 검사했습니다.
- [x] 실제 PTY/editor 왕복·미확정 조합 취소 신규1 및 변경 영향 prefix1: compile15.72초/suite0.44초 PASS입니다. 왕복은 terminal old preedit와 editor new preedit를 확정하지 않고 취소하며 terminal의 일반 Text `continue\n` 전달·editor 빈 본문/composition None·Phase::Exited(Some(0))를 확인합니다. 기존 hidden/view focus·attach/IME2건도 final integration binary에서 suite0.07초 PASS입니다.
- [x] 최종 engine/app lib/tests strict: `cargo clippy ... -p egui -p taide-native-app --lib --tests -- -D warnings` exit0/21.89초입니다. 독립 UI `--lib --test editor_surface -- -D warnings`도 exit0/0.89초입니다. 기존 Wry dependency warning17개는 유지하며 lint를 끄지 않았습니다. Rust6 exactfmt 및 문서 갱신 후 `git diff --check` exit0입니다. untracked native 파일을 tracked diff 검사로 검증했다고 세지 않습니다.

현재 exact runtime filters는 다음과 같습니다. 여러 필터는 `--exact`를 한 번만 전달합니다. 중복 `--exact`로 실패한 CLI 호출은 테스트 실행/RED로 계산하지 않았습니다. 존재하지 않는 terminal fixture/app-file 파일명 조회는 실제 Cargo manifest와 kebab-case 경로로 정정했고 제품 파일을 만들지 않았습니다.

```text
App lib binary: experiments/native-shell-spike/target/debug/deps/taide_native_app-6ae1e98e5aed8a62
--exact keymap::tests::local_editor_입력은_ax_focus_전후_실제_view에_전달된다
--exact keymap::tests::local_editor는_ax_focus_왕복에서_확정하지_않은_조합을_취소한다
Integration binary: experiments/native-shell-spike/target/debug/deps/terminal_host-3226916b98261227
--exact terminal_editor의_ax_focus_전후_문자는_실제_pty와_문서에_분리된다
--exact terminal_editor의_ax_focus_왕복은_미확정_조합을_취소하고_pty입력을_보존한다
Standalone UI binary: experiments/native-shell-spike/target/debug/deps/editor_surface-2cc7ad1a38c126f2
```

## Ordered AX terminal wire 후속

대상은 `native/taide-native-app/src/terminal_host.rs`, `src/terminal_surface.rs`, `tests/terminal-host.rs` 및 실제 앱 bin으로 컴파일하는 `native/taide-native-terminal/tests/fixtures/session.rs`입니다. source/egui/core/writer의 기존 읽은 계약과 실제 `SharedTerminal::try_input/try_write/accept_input`을 확인했습니다. 새 브라우저/OS 측정·의존성/manifest/lock/engine 변경은 없습니다.

실제 두 terminal은 서로 다른 pane/tab/Response ID로 동일 Hub session의 shared Outbox를 사용합니다. raw IME commit `con`, AX Focus 두 번째 target, 일반 Text `tinue\n` 순서입니다. 기존 helper를 공유하되 editor/조합취소 입력 조건은 유지했습니다. fixture가 정확한 `continue\n`을 받아 종료0이어야 하며 정/역 draw 각각 owned Hub/session/task를 종료/회수한 뒤 assert합니다.

- `Session::prepare_input`은 실제 현재 core mode로 한 번 인코딩하고 local action을 즉시 반환합니다. 아직 writer order를 예약하지 않은 prepared payload는 private session identity로 보호합니다. bounded stage 입장 뒤 `admit_prepared_input`이 user input epoch를 기록하며, 실제 submit/retry는 recorded bit로 두 번 기록하지 않도록 구현했습니다. 이 입장/재시도 epoch는 아직 별도 actual runtime 검사로 입증하지 않았습니다.
- Outbox는 일반 staged packet을 기존 receipt count/queued byte budget, Focus packet을 기존 focus count/byte budget 안에 둡니다. actual payload capacity와 StagedInput header를 합산하고 epoch 입장 전 초과를 거절합니다. raw frame/index와 InitialFocusLoss·InitialFocusGain·FocusLoss·FocusGain·Input phase로 정렬한 뒤 writer order를 예약합니다. Focus는 정렬된 dispatch 시 실제 Core focus를 관찰/인코딩합니다. payload/receipt 과압·foreign/retired/new staged 예외 검사는 별도 gate에 남깁니다.
- show 시작의 poll/flush는 현재 frame staged 입력을 보내지 않습니다. actual `finish_frame`에서 현재 viewport batch를 처리하고, 이전-frame staged 입력은 일반 flush에서 회수합니다. AX batch의 시작 release_focus는 final memory focus를 prefix 전에 적용하지 않으며 final release를 보존합니다. source input owner와 wire 전송 순서를 viewport의 draw 순서로 대신하지 않습니다.
- 순서를 아직 확정하지 못하는 pointer/Touch/wheel/navigation/AX Click/window regain은 명시적 barrier로 기존 경로를 보존합니다. raw_input에서 capture가 제거하기 전 barrier도 기록합니다. 이 제한은 안전한 단계 경계이며 전체 mixed-event 동등성의 완료/대체로 세지 않습니다. 모든 그런 배치의 후속 구현은 상위 M8 scope에 남아 있습니다.

### 실행 결과

- [x] 실제 wire RED: compile4.22초/suite0.49초/exit101, `reverse=true`가 `Exited(Some(1))`이고 기대는 `Exited(Some(0))`입니다. 기존 plain/editor 성공을 새 다중 terminal 성공으로 재사용하지 않았습니다.
- [x] 최초 prepared 구현의 compile E0433은 `Phase` import 누락이며 수정했습니다. 이후 compile17.52초/suite0.39초에도 reverse=true wire RED가 남았습니다. 실제 코드에서 각 show 시작 `flush_inputs`가 현재 staged 입력을 먼저 보내는 호출을 발견했습니다. 같은 가정을 재시도한 것이 아니라 flush 경계와 선행 final-focus 회수를 고쳤습니다.
- [x] 수정 후 정확한 새1건 compile6.77초/suite0.54초 PASS입니다. 최종 Focus fixture 추가는 해당 plain 입력/production 경계를 바꾸지 않아 이 성공을 재사용했습니다.
- [x] 실제 Focus wire 신규1건 compile4.99초/suite0.46초 PASS입니다. fixture는 합성 전용 `TAIDE_NATIVE_FIXTURE_AX_FOCUS=1`로 raw stdin과 기존 focus-report mode를 사용하며 정확한 `ESC[O ESC[I con ESC[O ESC[I tinue\n`의 결합 바이트를 검증합니다. 다른 fixture mode와 동시 사용은 거절합니다. 이 성공은 실제 OS AX/일반 UI 전체 검증이 아닙니다.
- [x] 변경 영향 기존6건 final integration binary에서 suite1.30초 PASS: terminal/editor prefix·focus 왕복 조합취소, hidden/view Focus protocol, 포화 Focus coalescing, 기존 pending 뒤 query response, actual measured attach/IME입니다. 기존 no-AX 경로의 Pending recorded 기본값은 false이며 그 동작을 유지했습니다. 새 prepared 재시도의 recorded=true 경계를 이 영향6으로 입증했다고 주장하지 않습니다.
- [x] `cargo clippy ... -p taide-native-app --lib --tests -- -D warnings` exit0/25.42초입니다. Phase import와 기존 unit PendingInput factory의 recorded=false를 포함한 lib/tests/bin을 컴파일했습니다. 뒤이어 예약 초기 focus의 loss/gain을 서로 다른 phase로 분리했고 최종 `--lib --test terminal-host -- -D warnings`도 exit0/2.00초입니다. 기존 신규2/영향6 입력은 초기 reported focus와 before-events ID가 일치해 InitialFocus packet이 생기지 않고 나머지 phase의 상대 순서가 동일합니다. 그래서 그 성공을 재사용했으며 초기 예약 focus wire의 별도 actual fixture는 미실행입니다. 기존 Wry dependency warning17개는 유지하며 suppression은 없습니다. 변경 Rust4 exactfmt 및 문서 갱신 후 diff check exit0입니다.

정확한 신규 필터는 `terminal_views의_ax_focus_전후_문자는_draw_순서와_무관하게_pty에_전달된다`, `terminal_views의_ax_focus_보고는_문자_사이_실제_wire_순서를_보존한다`입니다. binary는 기존 `experiments/native-shell-spike/target/debug/deps/terminal_host-3226916b98261227`입니다. 같은 성공을 반복하지 않고 engine/독립 UI/root/원본 측정은 변경 없는 기존 증거를 재사용했습니다.

## Prepared 입력·기본 pass 후속

대상은 `native/taide-native-app/src/terminal-input-tests.rs`, 이를 cfg(test)로 연결한 `src/terminal_surface.rs`, 기존 `tests/terminal-host.rs` helper와 `native/taide-native-terminal/tests/fixtures/session.rs`입니다. 이번 후속은 테스트만 추가했으며 제품 입력 구현·engine/vendor·의존성·manifest/lock을 바꾸지 않았습니다.

- [x] `terminal_surface::input_tests::prepared_input은_소유_입장_퇴역과_재시도_epoch를_보존한다`: 실제 감독 `/bin/cat` PTY에서 prepare가 epoch를 변경하지 않고 admission만 1 증가하며 deferred submit/retry 완료가 중복 증가하지 않음을 확인했습니다. 미입장 submit·foreign admit/submit·퇴역 prepare/admit/submit은 거절하고 Focus는 일반 user epoch를 변경하지 않습니다. owned Hub/session/task를 종료해 tracked_count=0을 확인했습니다.
- [x] `terminal_surface::input_tests::staged_input은_count_byte_focus_상한을_입장전에_검사한다`: 실제 live session의 Outbox에 일반64개·독립 Focus64개까지만 입장하고 초과는 epoch 증가 전에 거절합니다. max payload에서는 header를 포함한 byte quota가 count보다 먼저 작동했습니다. 포화에서도 local Preedit/Ignore는 가능하며 clear 뒤 두 byte counter와 pending이 회수됩니다.
- [x] 두 lib 테스트는 compile16.03초/suite0.03초/exit0입니다. 최초 compile E0616은 private PendingInput 필드에 대한 테스트 접근이 원인이었습니다. public 노출을 추가하지 않고 그 접근을 제거했으며 실제 epoch/receipt 결과로 검사했습니다.
- [x] `terminal_views의_ax_focus_wire는_discard_재렌더에서_중복되지_않는다`: 실제 UI num_completed_passes=2와 정/역 draw에서 기존 정확한 Focus/문자 wire·child exit0·owned 정리를 확인했습니다. Context의 기존 raw input take 경계를 그대로 사용합니다. compile10.04초/suite0.47초/exit0입니다.
- [x] `terminal_views의_입력전_focus_loss_gain은_draw_순서와_무관하게_먼저_전송된다`: 실제 memory focus를 입력 배치 전에 바꾼 뒤 정확한 `ESC[O ESC[I ESC[O ESC[I continue\n` 결합 바이트를 정/역 draw에서 확인했습니다. 합성 fixture mode2이며 기존 mode1은 유지합니다. compile4.82초/suite0.39초/exit0입니다. 입력 전 focus 변경 검사이지 실제 Shift+Tab/OS 예약 검사가 아닙니다.
- [x] 최종 `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target -p taide-native-app --lib --tests -- -D warnings` exit0/18.98초입니다. CARGO_HOME은 기존 `/Users/hyunseokbyun/development/rust/cargo`입니다. 변경 Rust4 exactfmt exit0입니다. 제품 구현 불변이라 직전 신규2/영향6 및 engine/독립 UI 성공은 재실행하지 않고 재사용합니다.

## 잠긴 탐색키 혼합 입력 후속

대상은 vendor egui-input의 `src/context.rs`·`src/memory/mod.rs`, app `src/terminal_surface.rs`·`tests/terminal-host.rs`, 실제 fixture `native/taide-native-terminal/tests/fixtures/session.rs`입니다. source `terminal-view.tsx`의 custom handler/onData와 설치된 같은 버전 egui EventFilter/Focus begin_pass를 읽었습니다. docs.rs의 버전 URL 두 조회는 도구에서 접근 실패했고 설치된 공식 package source 계약으로 확인했습니다. 새 source 브라우저 측정은 아닙니다.

- Memory는 초기 focus ID와 실제 filter를 같은 begin_pass 경계에 저장합니다. Context의 조회는 초기 owner와 직전 pass enabled/focusable/interactive 등록을 함께 검사하며 press Tab/방향키만 인정합니다. 영속 cache·새 의존성·Context/Response 데이터 보관을 추가하지 않았습니다. 다른 target은 미확정이며 실제 이동을 잠긴 입력으로 바꾸지 않습니다.
- 선행 keymap·local AX route·ordered batch가 같은 조회로 잠긴 키 뒤 owner를 유지합니다. raw capture의 일반 pointer barrier는 유지합니다. 현재 topology나 새 AX owner의 filter 전체를 추정하는 구현은 아닙니다.
- [x] 실제 PTY RED: 신규 `terminal_views의_잠긴_tab과_방향키는_ax_focus_이전_wire_순서를_보존한다` compile6.38초/suite3.35초/exit101입니다. reverse=false에서 기대 wire 완료 timeout이며 child exit1이라는 주장은 하지 않습니다.
- [x] 수정 뒤 같은 신규 검사 compile15.28초/suite0.40초/exit0입니다. 정/역 draw 모두 exact `ESC[O ESC[I con Tab ESC[D ESC[O ESC[I tinue\n`의 결합 바이트·owned child/session/task 정리를 확인했습니다. fixture mode3만 추가하고 mode1/2 입력은 유지했습니다.
- [x] 신규 `terminal_surface::tests::탐색키_owner는_입력시작_filter와_실제_등록을_검사한다` compile15.23초/suite0.02초/exit0입니다. locked/unlocked·enabled/disabled·등록/제거와 모든5 탐색키·다른 node·release·일반 Text의 거절을 확인했습니다. 이는 직전 등록 경계 검사이지 current-pass 전체 topology 완료가 아닙니다.
- [x] 신규 `terminal_surface::tests::window_capture는_잠긴_탐색키_뒤_terminal_scope를_유지한다`의 최초 compile7.64초/suite0.01초/exit101은 fixture가 generic click에 ordinary Button scope를 기대하고 두 Enter press를 release 없이 보낸 오류입니다. 실제 Button Response ID와 release로 fixture만 수정해 compile4.94초/suite0.02초/exit0입니다. 잠긴 키 뒤 terminal-local Enter를 남기고 AX 이후 ordinary Button global action만 처리합니다.
- [x] 최종 `cargo clippy ... -p egui -p taide-native-app --lib --tests -- -D warnings` exit0/22.55초입니다. Wry 기존 dependency warning17개는 별도이며 검사 suppression은 없습니다. authored Rust3 exactfmt exit0입니다. upstream 비교는 기대 exit1로 공통106동일/10변경·MIT·native-only patch·authored99% 제외를 확인했습니다.

명령 공통은 기존 CARGO_HOME, `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 각 runtime은 `cargo test --test terminal-host <신규wire필터> -- --nocapture` 또는 `cargo test --lib <신규unit필터> -- --nocapture`입니다. 같은 성공을 반복하지 않았습니다. wire 성공 뒤 추가한 non-navigation/release 거절 guard는 이 성공의 pressed Tab/ArrowLeft 입력을 바꾸지 않습니다. 이후 테스트만 추가했으며 신규 wire·predicate 성공을 재사용했습니다. 이전 AX plain/Focus/기본 pass4·prepared2·local prefix/조합취소 입력에는 탐색키가 없어 새 분기가 없고, no-AX의 local route None/ordered batch false는 유지하므로 기존 성공을 재사용합니다.

## 마우스 이동·휠과 AX wire 후속

대상은 app `src/terminal_surface.rs`·`tests/terminal-host.rs`, 실제 fixture `native/taide-native-terminal/tests/fixtures/session.rs`입니다. 기존 engine Focus/keyboard route와 mouse adapter의 설치된 source 계약을 재사용했습니다. engine/vendor·원본 TS·의존성·manifest/lock 변경이나 새 브라우저/OS 측정은 없습니다.

- pointer move·wheel 자체는 keyboard focus를 바꾸지 않아 일괄 wire barrier에서 제외했습니다. 클릭/Touch/AX Click/window regain과 미확정 navigation은 유지합니다. 현재 viewport/frame의 known AX 캡처 packet만 같은 bounded stage로 넣습니다.
- 제거된 wheel은 다음 survivor와 같은 index이므로 `CapturedInput` phase를 InitialFocus 이후·일반 Focus/Input 이전에 배치했습니다. 기존 pointer 전역 sequence 순서대로 입장하고 stable sort로 같은 boundary의 여러 packet을 보존합니다. 이전 frame·다른 viewport packet은 기존 즉시 경로를 유지합니다.
- pass 끝에서 캡처 drain을 먼저 하고 합쳐진 stage를 전송합니다. local submit 뒤 기존 input_order를 복원합니다. 기존 count/byte/Focus budget을 우회하거나 새 무상한 queue를 만들지 않았습니다.
- [x] 최초 테스트 compile E0308/E0063은 Response rect를 추가한 tuple의 sort 패턴과 실제 MouseWheel phase 누락입니다. 실제 타입/기존 fixture 계약으로 수정했으며 이 compile 실패를 wire RED로 세지 않습니다.
- [x] 신규 `terminal_views의_motion과_캡처_wheel은_ax_focus_전후_wire를_보존한다`의 실제 RED는 compile4.77초/suite0.44초/exit101, reverse=false의 child exit1≠0입니다. 수정 뒤 compile10.07초/suite0.40초/exit0이며 정/역 draw의 exact `ESC[O ESC[I con ESC[<35;1;1M ESC[<64;1;1M ESC[O ESC[I tinue\n` 결합 바이트를 확인했습니다.
- [x] 신규 `terminal_views의_alt_wheel은_ax_focus_이전_방향키를_보존한다`는 compile5.20초/suite0.46초/exit0입니다. tracking mouse와 별도 captured_wheel 경로에서 위/아래 두 event가 같은 boundary index에 있고 exact `ESC[O ESC[I con ESC OA ESC OB ESC[O ESC[I tinue\n` 순서로 전달됐습니다. 정/역 draw·owned Hub/session/task 회수까지 확인했습니다. fixture mode4/5만 추가하고 기존 mode1~3은 유지했습니다.
- [x] `cargo clippy ... -p taide-native-app --lib --tests -- -D warnings` exit0/19.50초·authored Rust3 exactfmt exit0입니다. Wry 기존 dependency warning17개는 그대로이며 suppression은 없습니다. engine/vendor가 불변이라 직전 engine strict22.55초·106동일/10변경/MIT 증거는 재사용했습니다.

Cargo 공통 CARGO_HOME/manifest/locked/offline/target-dir는 앞 절과 같습니다. 각 runtime은 해당 정확한 신규 필터로 `cargo test --test terminal-host <필터> -- --nocapture` 한 번 성공했습니다. SGR 성공 뒤 helper의 passive mode를 test-only enum으로 구분했지만 SGR의 raw event/fixture mode4/Response geometry/제품 경계는 동일해 성공을 재사용했습니다. 이전 AX plain/Focus/기본 pass·잠긴 navigation에는 캡처 packet이 없어 phase 추가/finish drain 이동이 입력 순서를 바꾸지 않으며 prepared budget도 동일합니다. no-AX local route None/ordered batch false·원래 pointer queue/drain은 유지해 기존 mouse/wheel/hidden/pressure 성공을 재사용했습니다. 이 성공은 클릭/Touch·full App current topology·전체 viewport/OS source parity의 증거가 아닙니다.

## 남은 범위와 보존

- [ ] 다음 클릭 owner 작업의 실제 source는 설치된 xterm6.0.0 `node_modules/@xterm/xterm/src/browser/CoreBrowserTerminal.ts` bindMouse의 always-on mousedown입니다. `preventDefault`→`focus`→mode/forced-selection 검사→mouse report 순서이며 primary만으로 제한하지 않습니다. native `Views::show_with_keymap`은 현재 Response clicked/drag_started를 통해 focus를 요청하고, engine route는 PointerButton 뒤 owner를 unknown으로 둡니다. 이 source 경계와 실제 등록/geometry/modal/viewport를 연결해야 하며 클릭 이후 final focus를 앞선 문자에 소급하지 않습니다. 아직 새 클릭 runtime 재현은 실행하지 않았습니다.
- [ ] pointer/Touch/Tab/IME의 실제 사건별 focus/default 동작, current-pass 신규/제거/disabled/higher modal topology와 editor/terminal 내부 overlay 조상 scope가 남습니다. 알 수 없는 owner에 최종 focus를 대신 적용하지 않는 선행 경계만 완료했습니다.
- [x] known root AX의 actual 두 editor·PTY/editor local prefix Text/IME 확정 및 focus 왕복 조합 취소는 위 후속에서 완료했습니다. 실제 source browser focus/IME 실기 또는 모든 default action graph의 완료로 세지 않습니다.
- [x] 현재 live 동일 session의 알려진 AX 두 terminal에서 기본 Text/Focus wire 순서는 위 후속의 실제 PTY로 확인했습니다.
- [x] prepared admission epoch·recorded retry·foreign/retired/budget 실패와 기본 discard 두 pass·입력 전 focus loss/gain wire는 위 신규4건에서 확인했습니다.
- [ ] 실제 예약 Tab·dynamic/disabled/hidden/current viewport topology·ordered attaching·consumed AX loss의 별도 runtime variant는 남습니다. 재현 시 기본 성공은 반복하지 않고 해당 위험의 실제 wire/현재 survivor만 확인합니다. App capture의 사건별 composition/overlay 조상 scope도 이 local 수명 검사로 대체하지 않습니다.

위 ordered 후속 전 조사에서 session별 shared Outbox의 즉시 submit과 captured pointer만 EventOrder로 정렬하는 차이를 확인했습니다. 이를 실제 두 target RED로 확인한 뒤 known AX에 prepared stage를 연결했습니다. 일반/unknown 배치의 즉시 submit과 기존 pointer queue를 전체 사건 graph에 통합하는 일은 여전히 남습니다. 새 개인 셸/OS 설정은 사용하지 않았습니다.
- [ ] 모든 viewport registry 수명·full App/auxiliary/OS AX·CJK/VoiceOver·GUI 및 전체 source parity는 남습니다. 사용자 실기 검증은 마지막 순위이고 시스템 설정을 변경하지 않았습니다.

보호 spike bundle, 실제 사용자 데이터·clipboard·Keychain·OS 설정, 제품 TS, root/Tauri/의존성/manifest/lock/MSRV 및 Git은 이번 변경에서 유지했습니다. engine의 공통106byte동일/10변경·MIT 고지와 authored99% 산정 제외를 다시 확인했습니다. `diff -qr` exit1은 알려진10변경·설치 메타데이터4개·native 추가 LICENSE-MIT/UPSTREAM.md의 기대 차이입니다. 전체 test/bin/root/GUI 검증 또는 M8 완료를 주장하지 않습니다.

## Terminal mousedown focus·혼합 AX 후속

대상은 app `src/terminal_surface.rs`·`tests/terminal-host.rs`, vendor egui-input `src/{context,pass_state}.rs`·`src/memory/mod.rs`, 실제 fixture `native/taide-native-terminal/tests/fixtures/session.rs`입니다. source는 설치된 xterm6.0.0 `CoreBrowserTerminal.ts`의 always-on mousedown `preventDefault → focus → mode/forced selection 확인 → sendEvent`와 egui0.36.2의 실제 hit-test/Memory begin_pass를 확인했습니다. 새 source DOM 측정은 아닙니다.

1. 직전 pass에 실제 running/enabled terminal ID만 pointer focus를 선언합니다. 엔진은 이전 widgets·interactable 영역·z-order·transform·layer input region·modal 허용 여부의 정확한 hit-test를 사용합니다. 미선언/disabled/가려진 target은 요청을 만들지 않습니다. viewport raw index cache에 root AX와 선언된 누름을 함께 저장하며 release는 keyboard owner를 옮기지 않습니다. 선언은 pass별 비우고 cache는 매 raw 배치 교체하며 Context/Response를 data에 넣지 않습니다.
2. keymap/local demux/ordered wire가 같은 cache를 사용합니다. Memory는 시작 focus/filter snapshot을 보존한 채 마지막 선언된 pointer를 적용하고 늦은 AX를 덮지 않습니다. 원본처럼 pointer loss/gain은 같은 index의 mouse report보다 먼저 stage합니다. 다음 survivor와 index가 합쳐진 제거 wheel의 CapturedInput 선행 순서는 유지합니다. 별도 raw wire_barrier 필드는 제거하고 surviving unknown press/Touch/window regain/AX Click/navigation은 기존 ordered gate에서 검사합니다.
3. 일반 Button은 배치 최종 Memory가 아니라 시작 snapshot과 raw 요청 index로 prefix를 처리합니다. 실제 terminal의 완료 click/drag도 알려진 최종 owner가 다른 target이면 focus를 다시 요청하지 않습니다. explicit App request와 unknown fallback은 유지합니다. secondary 메뉴의 집계 입력 차단을 해결한 것으로 주장하지 않습니다.

- [x] 실제 primary/middle PTY 신규1건 `terminal_views의_mousedown은_기존_문자_뒤_focus_보고_앞_입력을_보존한다`입니다. 최초 compile4.85초/suite0.37초/exit101, reverse=false child exit1 RED입니다. 수정 뒤 compile17.39초/suite0.45초/exit0이며 각 button의 정/역 draw에서 초기 focus·IME `con`·loss/gain·정확한 SGR press·`tinue\n`·정확한 release 바이트와 owned 정리를 확인했습니다. AX 없이 누름 자체로 owner를 이동합니다. 합성 fixture mode6/7이며 기존1~5는 유지합니다.
- [x] 이전 일반 Button Enter prefix 신규1건 `terminal_surface::tests::pointer_focus는_이전_button_enter를_소급_취소하지_않는다`입니다. 초기 compile13.67초/suite0.01초/exit101에서 앞선 Enter가 사라졌고 Button snapshot/index 수정 뒤 compile11.43초/suite0.02초/exit0입니다. 정/역 draw의 실제 Button과 선언된 target을 검사했습니다. 새 test의 unused_mut는 제거했고 성공 입력은 재실행하지 않았습니다.
- [x] hit/선언 신규1건 `terminal_surface::tests::pointer_focus요청은_실제_enabled_hit과_선언에만_배정한다`입니다. declared/enabled/Foreground cover4조합·press/release/out-of-range cache·prefix/post-press/post-release Text owner를 확인했습니다. compile7.86초/suite0.02초/exit0입니다. 현재 topology의 동적 제거 전체 검사가 아닙니다.
- [x] window 혼합 신규1건 `terminal_surface::tests::window_capture는_pointer와_ax_focus의_사건별_scope를_보존한다`입니다. pointer→AX/AX→pointer 두 순서에서 terminal local Enter pair와 global Button Enter capture·최종 Memory focus를 확인했습니다. 최초 compile7.69초/suite0.02초/exit101은 fixture가 global keyup도 제거한다고 기대한 오류입니다. 기존 `keymap::Windows::route`의 keydown-only 계약을 확인하고 release survivor 순서만 정정했습니다. 제품 keyup 정책 불변·수정 뒤 compile4.99초/suite0.03초/exit0입니다.
- [x] 실제 완료 click 뒤 AX 신규1건 `terminal_views의_완료_click은_뒤에_온_ax_focus를_덮지_않는다`입니다. compile13.17초/suite0.37초/exit101에서 실제 최종 Memory가 terminal1/기대 terminal0였습니다. aggregate request 수정 뒤 compile5.80초/suite0.50초/exit0입니다. 실제 정/역 draw의 최종 Memory ID와 `con → loss/gain → SGR press → tinue\n → SGR release → loss/gain` exact wire·owned 정리를 확인했습니다. fixture mode8의 mouse mode 출력 누락도 정정했으며 최초 RED를 wire 확인 성공으로 취급하지 않습니다.

영향 검사와 최소 실행 명령:

- [x] 변경 Button snapshot/index의 기존 AX Enter/Space16조합 및 Space 소비/blur/disabled2건은 이미 빌드한 최신 lib binary에서 함께1회 실행해 suite0.03초/exit0입니다.
- [x] 선언된 no-AX mouse press가 새 stage 분기로 들어가므로 기존 actual binaryFF/liveSGR·backpressure/replay/discard PTY1건을 추가 영향으로 실행했습니다. 최초 suite0.04초/exit101은 중간 epoch 기대3/실제8입니다. 기존 prepared admission 계약상 첫 batch의6개 입력은 writer 완료 전에 stage admission에서 각1회 record되고 다음 unknown pane의 `d`는 pending입니다. 중간 기대만 `epoch_before + MIXED_INPUT_COUNT - 1`로 정정했습니다. 제품 epoch 구현 불변이며 compile7.09초/suite0.60초/exit0에서 정확한 wire·최종 총7회·child exit0·owned join을 확인했습니다. 원래 writer-submit 시점의 중간 기대를 현재 stage admission 경계와 구분합니다.
- [x] engine/app `cargo clippy ... -p egui -p taide-native-app --lib --tests -- -D warnings` exit0/23.07초입니다. 실제 완료 click 뒤 AX 수정 후 최종 app-only 같은 strict exit0/19.55초입니다. 이후 영향 test의 기대식만 바뀌어 해당 compile/runtime 성공을 사용하며 같은 정적 검사도 반복하지 않습니다. authored Rust3 exactfmt와 diff 검사 exit0입니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test terminal-host terminal_views의_mousedown은_기존_문자_뒤_focus_보고_앞_입력을_보존한다 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test terminal-host terminal_views의_완료_click은_뒤에_온_ax_focus를_덮지_않는다 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib pointer_focus는_이전_button_enter를_소급_취소하지_않는다 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib pointer_focus요청은_실제_enabled_hit과_선언에만_배정한다 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib window_capture는_pointer와_ax_focus의_사건별_scope를_보존한다 -- --nocapture
experiments/native-shell-spike/target/debug/deps/taide_native_app-6ae1e98e5aed8a62 --exact button_key_tests::button_key의_ax_focus_배정은_최종_focus나_draw_순서에_몰리지_않는다 button_key_tests::button_space는_소비된_press_release와_blur_disabled에서_활성화하지_않는다 --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test terminal-host mouse_surface는_actual_pty의_binary_ff와_live_sgr_전환을_보존하고_join한다 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target -p taide-native-app --lib --tests -- -D warnings
```

성공 재사용: primary/middle 기본 wire는 완료 click 뒤 AX guard가 true인 동일 분기라 재실행하지 않았습니다. 앞선 unit·Button2건은 실제 Views.show의 aggregate request 경로를 사용하지 않아 결과를 재사용합니다. 기존 passive motion/wheel·AX 기본/prepared/pass/locked navigation은 선언된 press가 없어 cache/owner/phase의 해당 입력 분기가 같으며 성공을 재사용합니다. no-AX mouse press 영향1건은 새 stage 분기라 재사용으로 넘기지 않았습니다. root/standalone UI/제품TS 검사는 해당 구현 불변으로 반복하지 않습니다.

- [ ] secondary context-menu release/F10 전후의 배치 입력과 메뉴 focus return, Touch·editor/generic press·현재 disabled/hidden/modal/viewport topology·새 target/전체 graph는 남습니다. stale geometry/변환 전체 검증을 위 cover4조합으로 대체하지 않습니다. 실제 GUI/App composition·CJK/VoiceOver는 여전히 마지막 사용자 실기 범위입니다.

vendor common106동일/10변경·MIT·authored99% 제외를 재확인했습니다. `diff -qr` exit1은 알려진 차이이며 제품TS·root/Tauri·새 의존성·manifest/lock/MSRV·보호 bundle·사용자 데이터/OS/clipboard/Keychain·Git은 변경하지 않았습니다. 후속46 및 M8 N1~N8 0/8은 미완료·목표active이며 전체 완료 전 commit/push는 없습니다.
