# M8 keybindings Tab 스크롤 진단·수정

현재 상태(2026-10-05): 조건부 row control의 역할별 identity와 현재 모델의 Tab 순서를 연결했습니다. unbind ID 변경 RED 뒤 신규2·영향11 PASS·native lib/tests strict19.77초입니다. 직전 동적 행 제거·목록 축소와 기존 clipping 성공은 아래 역사·재사용 근거로 보존합니다. 전체 M8은 미완료입니다.

## 대상 파일

`native/taide-native-app/src/keybinding-editor.rs`

## 조건부 row control identity·현재 모델 Tab 배정 (2026-10-05 후속2)

- [x] 원본 row의 조건부 reset/unbind sibling과 native override 제거 시 unbind ID 변경을 대조·재현했습니다.
- [x] row ID·viewport·control 역할의 stable scope와 현재 row 모델 기반 Tab 순서를 연결했습니다.
- [x] 신규2·영향11 PASS·native lib/tests strict19.77초·exact rustfmt/diff exit0입니다.
- [ ] raw Text/AX/포인터로 같은 batch 안에서 query/filter/capture topology가 변경되는 순서·여러 Tab·disabled/상위 modal·search input/capture 전환·full App/auxiliary/OS/픽셀은 남아 있습니다.

### 대상·원본 계약

이번 변경은 `keybinding-editor.rs` 한 파일입니다. `src/features/settings/keybinding-row.tsx`의 Change·조건부 Reset·조건부 Unbind, conflict resolve와 capture/confirm 순서를 그대로 사용했습니다. Row는 원본 React row ID로 유지되고 unbind는 reset 조건과 독립된 sibling 슬롯이므로 reset만 제거해 다른 역할의 버튼으로 바꾸지 않습니다. [React identity/위치](https://react.dev/learn/preserving-and-resetting-state)와 원본 JSX가 근거입니다. 새 source DOM 측정은 하지 않았고 직전 source 제거 fallback 실측은 동작이 같은 계약에서 재사용합니다.

### 구현

`RowControl`은 row ID·ResolveConflict/Capture/Confirm/Change/Reset/Unbind 역할을 담습니다. 각 실제 button에 viewport/row/role의 명시적 UiBuilder.id scope를 사용합니다. 앞선 sibling의 수·행 순서·conditional badge에 따른 auto ID 재사용으로 역할을 바꾸지 않습니다. `can_unbind`는 원본 key/default label 조건을 렌더와 모델에서 공유합니다.

Tab 대상은 이전 response의 순번이 아니라 현재 raw_overrides/catalog·현재 검색/필터/locale 정렬·capture/conflict/override shape의 역할 순서로 계산합니다. header/close는 실제 등록 ID를 유지합니다. 사라진 역할은 source container로 회수하고, 새 역할은 해당 widget을 만드는 scope 안에서 실제 `Ui.next_auto_id()`로 request_focus한 뒤 그립니다. 내부 ID 산식을 복제하거나 이미 그린 response에 뒤늦게 focus를 붙이지 않습니다. source ordinary Tab/capture/IME/popup gate는 유지합니다. focus_rows에는 RowControl metadata만 담으며 focus_target은 show 끝/close에서 회수합니다.

icon button은 add_sized의 추가 auto child 대신 원본 24px min_size를 실제 button에 적용해 focus 준비 ID와 widget ID를 일치시킵니다. 기존 glyph/행 간격 검사와 Tooltip 실제 AX/caller 영향 검사로 이 변경의 독립된 geometry 위험을 덮습니다. [Ui.next_auto_id](https://docs.rs/egui/0.36.2/egui/struct.Ui.html#method.next_auto_id)와 pinned ui.rs/ui_builder.rs의 공개 API 문서·구현이 근거입니다. UiBuilder 전용 docs.rs 조회의 Internal Error는 설치된 정본 문서·구현으로 확인했으며 웹 조회 성공으로 기록하지 않았습니다.

### 실제 검증

1. `keybinding_override변경은_남은_unbind_focus와_제거된_reset_회수를_보존한다` baseline은 surviving unbind ID6230519078993315154/previous8730125973026454617 RED(compile7.41초/suite0.09초)였습니다. `keybinding_새_reset의_tab_배정은_현재_override_모델을_사용한다` baseline1 PASS(0.06초)는 기존 auto ID의 새 역할 재사용도 통과할 수 있어 identity 문제 부재 근거로 쓰지 않았습니다.
2. 위 role scope/현재 모델 router 수정 뒤 첫 검사1 PASS(compile12.61초/suite0.14초), 변경된 router의 새-reset 검사1 PASS(0.06초)입니다. override 제거 때 남은 unbind ID/owner 유지·reset 제거 시 container 회수·save 무발생, override 추가와 동일 frame Tab 시 새 Reset owner/현재 rect 가시성·save 무발생을 덮습니다. baseline PASS를 현재 수정의 성공으로 재사용하지 않았으며 같은 현재 성공을 재실행하지 않았습니다.
3. 정확한 영향11 필터1회11 PASS(suite0.68초)입니다. 이전 forward/reverse/capture-blur/wrap/capture Tab·arrow/popup Escape/실제 modal 저장·검색·창 chord/Tooltip reset-unbind2테마, 직전 row 제거/축소2와 row grid의 reset-unbind28px/glyph/source gap입니다. role scope와 focus_rows 타입·현재 모델 router가 바뀌어 해당 기존 성공 재사용 대신 변경 영향으로 한 번 검사했습니다. Icons/font raster·engine·source JSON/개발 도구는 불변이므로 재검사하지 않았습니다.
4. `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml -p taide-native-app --lib --tests -- -D warnings` exit0(19.77초), native exact rustfmt/diff exit0입니다. 기존 Wry dependency17경고는 억제하지 않았습니다. test frame helper는 actual tooltip response를 Output에만 반환하며 Context.data에는 계속 ID/rect 값만 저장합니다.

실제 파일 mapping은 lib.rs의 ui_icons→keybinding-icons.rs입니다. 조사 중 ui-icons.rs/ui_icons.rs 직접 경로 조회 오류를 mapping 확인으로 정정했으며 파일 생성이나 module rename을 하지 않았습니다. 제품TS/vendor/dependency/manifest/lock/MSRV/보호bundle/OS/Git 불변·live command 없음·N1~N8 0/8·후속46 미완료·goal active·전체완료 전 commit/push 없음입니다.

현재 graph는 show 시작의 모델을 기준으로 합니다. 모든 raw 사건의 중간 topology 변경/다중 key·상위 modal/disabled·새 header/capture 전환까지 재현한 전체 포커스 그래프가 아닙니다. 원본 모든 화면/213view/41action/Monaco21/Rust remote UI/성능·보안·배포/M8 gate를 이 검사의 성공으로 완료 처리하지 않습니다.

## 동적 행 제거·남은 행 포커스 (2026-10-05 후속)

대상은 위 native 파일과 개발 도구 `tools/m8-dialog-removal-focus-measure.ts`, 도구 범위의 `experiments/native-tooltip-reference/tsconfig.json`입니다. 제품 TS와 vendor는 변경하지 않았습니다.

- [x] 원본 shared Dialog/FocusScope의 제거 계약과 최소 RED를 확인했습니다.
- [x] 현재 검색/필터 목록에서 제거된 행의 ID를 Tab 배정 전에 제외하고 modal 컨테이너로 회수했습니다. 행 UI scope를 viewport·row ID로 고정해 앞선 행이 사라져도 남은 행의 실제 control ID를 보존합니다.
- [x] 좁은 신규2·영향8·native lib/tests strict·도구 TS와 exact format을 검증했습니다.
- [ ] 조건부 reset/unbind/capture shape·locale 재정렬·신규 control의 current-pass 배정·disabled/더 높은 modal·여러 key/AX/pointer/IME 혼합·전체 App/auxiliary/VoiceOver/픽셀은 남아 있습니다.

### 원본 근거·실측

원본 `src/widgets/keybindings-editor/keybindings-editor.tsx`는 row ID로 KeybindingRow를 유지하고 공유 Dialog를 사용합니다. installed `@radix-ui/react-focus-scope/dist/index.mjs`의 trapped MutationObserver는 focused DOM 제거 뒤 activeElement가 body이면 container.focus(preventScroll)를 호출합니다. 임의로 검색 input을 autofocus하지 않습니다. 원본 `src/features/settings/keybinding-row.tsx`의 reset/unbind 조건은 별도 current-control gate로 남겼습니다.

새 source 빌드 없이 기존 `/private/tmp/taide-tooltip-escape.aWvZH4/built`의 실제 공유 Dialog를 로드했습니다. 합성 input/버튼을 그 Dialog 안에 추가해 포커스된 합성 버튼을 제거한 뒤 Tab/Shift+Tab을 각각 독립 BrowserContext에서 한 번 측정했습니다. source 파일 자체나 사용자 앱은 조작하지 않았습니다. sandbox launch SIGABRT/EPERM은 제품 RED가 아니며 scoped escalation 뒤 단일 성공(exit0·0.5718초)입니다. 임시 Chrome 프로필·mock Keychain·외부 요청 차단을 유지하고 종료했습니다.

원자료는 `assets/2026-10-05-dialog-removal-focus-source.json`입니다. 제거 직후 두 사례 모두 actual Dialog container를 가집니다. forward Tab 뒤 button, Shift+Tab 뒤 container입니다. synthetic first/last는 둘 다 false이며 해당 원본 fixture에는 먼저 existing Commit 버튼이 있습니다. 이 측정은 실제 전체 KeybindingsEditor의 DOM/row 제거 측정이나 forward target label의 직접 계측이 아닙니다. browser 기본 순서와 FocusScope trap 코드로 그 경계를 구분합니다. [Radix Dialog](https://www.radix-ui.com/primitives/docs/components/dialog), [Playwright Page.evaluate](https://playwright.dev/docs/api/class-page#page-evaluate)·[Keyboard.press](https://playwright.dev/docs/api/class-keyboard#keyboard-press)가 API 근거입니다.

### native 실패·수정·검증

1. 최초 Cargo의 짧은 필터+`--exact`는 compile7.08초/0tests였으며 통과 근거로 세지 않았습니다. 컴파일된 실제 binary에 full test 이름으로 실행해 `keybinding_제거된_행_focus는_modal로_회수하고_다음_tab을_보존한다`의 focus None/expected container RED(0.06초)를 확인했습니다. 제거된 행을 현재 visible 목록으로 pruning하고 source와 같은 비-Tab 대상 container를 등록했습니다. 이 검사는 forward·reverse 제거/회수·다음Tab·저장 무발생·창 열림 보존1 PASS(compile8.06초/suite0.17초)입니다.
2. 남은 행의 source keyed identity와 새 현재 Tab 대상 검사는 첫 실행에서 actual rect y-127~-103/clip start336.5 RED였습니다(compile7.96초/suite0.11초). 목록을 축소한 첫 pass에 ScrollArea가 이전 큰 offset으로 layout하고 끝에서 clamp하므로 그 pass의 그림만 화면 밖에 남았습니다. 실제 반환 ScrollAreaOutput.id로 이전 State를 조회하고, displayed row IDs 변경과 실제 offset 변경이 함께 있을 때 표준 `Context.request_discard`로 동일 run_ui 안에서 재배치합니다. 추가 대기 frame·offset 무조건0·직접 유도 State ID는 사용하지 않았습니다. pinned run_dyn의 RawInput.take는 재배치 pass에 사건을 다시 전달하지 않습니다. full App의 모든 effects와 max_passes=1 환경을 별도로 완료했다고 주장하지 않습니다.
3. 실패 검사1 PASS(compile7.88초/suite0.13초)입니다. 마지막 visible row 포커스 ID·현재 pass 전체 rect, query 축소 뒤 동일 ID, header filter에서 no-results+Tab 시 close 배정·사라진 row ID 회수를 덮습니다. 앞선 제거1 성공은 그 검사의 offset 불변 분기에서 재사용했습니다. [egui UiBuilder.id](https://docs.rs/egui/0.36.2/egui/struct.UiBuilder.html#method.id)·[Context.request_discard](https://docs.rs/egui/0.36.2/egui/struct.Context.html#method.request_discard)와 installed 구현을 확인했습니다.
4. 정확한 영향8 필터(기존 forward/reverse/capture-blur/양끝 wrap/capture Tab·arrow/popup Escape/실제 modal 저장·검색·창 chord/Tooltip modal)1회8 PASS(suite0.68초)입니다. `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml -p taide-native-app --lib --tests -- -D warnings` exit0(17.73초)입니다. 엔진 불변이며 앞선 engine strict를 재사용합니다. 기존 Wry dependency17경고는 억제하지 않았습니다.
5. `bunx --no-install tsc --noEmit -p experiments/native-tooltip-reference/tsconfig.json` exit0(0.646초), native exact rustfmt와 도구2파일 Prettier exit0입니다. 성공 상태가 같은 검사는 반복하지 않았습니다. 제품TS/dependency/manifest/lock/MSRV/보호bundle/OS/Git 불변이며 전체 M8 N1~N8 0/8·goal active·전체완료 전 commit/push 없음입니다.

focus_rows와 rows_scroll에는 ID·row ID 문자열만 저장하며 pass/close에서 회수합니다. Context/Response/GUI owner를 보관하지 않습니다. 기존 Vec router는 조건부 control 의미나 새 목록 정렬 전체를 아직 모델링하지 않으며 이를 전체 current-pass focus graph 완료로 계산하지 않습니다.

## 리포트

2026-10-03 당시 신규 `keybinding_modal_tab은_화면밖_행을_표시하고_끝까지_순환한다`는 RED였습니다. 당시 결과만으로 현재 실패나 전체 focus trap 완료를 판정하지 않습니다. 아래 2026-10-05 절이 최신 구현·검증의 정본입니다.

## 실제 결과·중단

- baseline1 FAIL(suite0.21초): 버튼 rect y667~691 대비 interact clip end y655.
- `Response.gained_focus()` 시 `scroll_to_me_animation(None, ScrollAnimation::none())`를 row text/icon/capture에 적용한 뒤1 FAIL(suite0.21초), 동일 clipping입니다.
- modal의 이전 focus를 별도로 비교하는 수정 뒤1 FAIL(suite0.21초), 동일 clipping입니다.
- 동일 실패3회 규칙에 따라 반복 수정을 중단하고 async 질문으로 상태 계측/다른 M8 항목 선택을 요청했습니다. 실패한 두 구현은 제거했고 RED 재현 검사만 유지합니다. live Cargo handle은 없습니다.

Cargo는 app manifest·locked/offline·기존 target·CARGO_HOME을 사용한 serial 검사입니다. 이전 native-toasts strict exit0(11.32초)는 Tab 재현 검사를 추가하기 전 checkpoint입니다. 전체 현재 suite green으로 주장하지 않습니다.

## 다음 진단

- [x] focus registration/gained signal·실제 scroll offset·current/previous widget rect·RawInput frame을 구분해 계측했습니다. 실제 반환 ScrollAreaOutput과 UI origin, Context.end_pass의 buffer swap을 대조했습니다.
- [x] 현재 프레임 좌표 검사·focused control 노출·양방향 순환과 화면 밖 capture/blur를 수정·검증했습니다. 기존 caller/팝업/capture/Tooltip 영향5 성공도 재사용 가능한 증거로 남겼습니다.
- [ ] 다른 창·같은 batch의 여러 Tab/포인터/필터 상태 변경·전체 App/AX/픽셀과 동적 current-pass focus graph입니다. 아래 성공을 전체 M8 gate로 확장하지 않습니다.

## 실제 수정·검증 (2026-10-05)

대상은 `keybinding-editor.rs` 한 파일입니다. source KeybindingsEditor의 DOM 순서·ScrollContainer의 overflow-auto와 원본 capture/blur gate, pinned egui Focus의 Previous 예약·ScrollArea/Response API를 확인했습니다. [ScrollArea0.36.2](https://docs.rs/egui/0.36.2/egui/containers/scroll_area/struct.ScrollArea.html)와 설치된 코드가 근거이며 source DOM을 새로 실측하지 않았습니다.

1. 실제 helper의 focused response를 임시 계측한 baseline은 gained_focus=true/had=false·rect y667~691·clip end655·content802px/inner318.5px·offset0의 RED였습니다(compile7.22초/suite0.21초). 신호 누락으로 판단하지 않았습니다. focused control이 일부라도 clip 밖이면 ScrollAnimation::none으로 노출을 요청하고 목록의 animated(false)를 연결했습니다. text/icon/capture에 동일 처리합니다.
2. 첫 수정은 실제 offset52를 만들었지만 종료 뒤 read_response의 이전 좌표 때문에 검사만 계속 FAIL했습니다(5.98초/0.21초). 추가 origin 계측(7.16초/0.21초)에서 현재 content origin이337→285로 이동했는데 검사 rect는667로 남았습니다. Context.end_pass가 this/prev buffer를 교환하므로 종료 뒤 read_response는 현재 프레임의 좌표 증거가 아니었습니다. fixture가 ID를 직접 유도한 State 값0 역시 실제 ScrollAreaOutput과 같은 ID임이 보장되지 않아 offset 회수 실패 근거에서 제외했습니다. ScrollArea.id_salt는 IdSalt를 한 번 만들고 Ui.make_persistent_id가 다시 salting하며 단순 문자열의 직접 조회와 같다고 가정하지 않습니다.
3. test frame closure 안에서 editor.show 뒤 실제 focused ID/rect/interact_rect만 저장하고 종료 뒤 그 값과 최종 owner를 비교합니다. Context/Response를 data에 보관해 순환 참조를 만들지 않습니다. 기존 center 가시성·중복 없음·close 마지막·search 복귀 조건을 약화하지 않고 정방향1 PASS(7.16초/0.36초)입니다. 임시 production 계측은 전부 제거했습니다.
4. 새 역방향 검사에는 current-pass rect y1101~1125/clip655의 별개 RED가 나왔으며 화면 밖 capture/blur는 동시에 PASS했습니다(compile7.04초/suite0.11초). native FocusDirection::Previous가 다음 pass의 포커스를 예약해 scroll 노출까지 늦는 것이 원인입니다. 실제 enabled focusable response의 DOM 렌더 순서 ID를 매 pass 갱신하고 ordinary Tab/Shift+Tab만 해당 목록으로 직접 배정합니다. capture/IME/popup/disabled gate와 close 회수를 유지합니다. 대기 프레임을 추가하지 않았습니다. 변경된 router의 정방향 영향1·역방향 신규1은 함께2 PASS(7.93초/0.37초)이며 앞선 capture/blur 성공은 동작이 바뀌지 않은 해당 분기에서 재사용합니다.
5. 양끝 wrap/capture Tab·arrow/popup Escape/실제 modal capture·저장·검색·창 chord/Tooltip modal의 정확한 영향5 필터는5 PASS(suite0.59초)입니다. `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml -p taide-native-app --lib --tests -- -D warnings` exit0(17.50초), exact rustfmt/diff exit0입니다. 성공 상태가 같은 검사는 반복하지 않았습니다. 엔진은 이번 수정에서 불변이며 직전 engine 검증을 재사용합니다.

추가 probe patch의 문맥이 다른 map에 적용된 syntax/format 인자 오류, Target::Row fixture의 E0164, focus_order 갱신 closure와 ConflictIndex borrow의 E0500은 compile 오류로 분리했습니다. 실제 문맥·struct variant를 확인하고 filter response를 closure 밖에서 등록해 정정했습니다. 존재하지 않는 underscore 파일/잘못된 keymap glob 경로도 actual kebab-case/source 파일로 정정했습니다. 예전 실패 수정이 실제로 무효였다는 주장은 buffer swap 오류 때문에 철회하며, 현재 구현의 실제 검사만 완료 근거로 씁니다.

focus_order는 ID만 보관하고 pass/close에서 갱신·회수합니다. 여러 raw Tab·현재 pass의 목록/disabled 변경·전체 모달/auxiliary·browser 픽셀/VoiceOver/OS는 미완료입니다. 제품TS·vendor/manifest/lock/MSRV·보호 bundle·OS·Git은 불변이며 live command는 없습니다. full M8 N1~N8 0/8·goal active·전체완료 전 commit/push 없음입니다.

## 보존한 완료

toast의5개 위험 검사·Settings host2개 PASS와 source/권한 경계는 `docs/quality-assurance/2026-10-03-m8-native-toasts.md`가 정본입니다. 보호 bundle·OS·사용자 데이터·제품 TS·root/MSRV를 조작하지 않았고 commit/push는 전체 M8 완료 뒤입니다. 목표는 active이며 일시 정지나 완료로 변경하지 않았습니다.
