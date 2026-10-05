# M8 AppFile 읽기 화면·소유자·닫기 연결

## 대상 파일

- `native/taide-native-app/src/{app-file-views,app-file-views-tests,application,host,lib}.rs`
- `native/taide-native-app/src/{settings-view,settings-view-tests,keybinding-icons,presentation-refresh,tabs,tab_close_batch}.rs`
- `native/taide-native-app/resources/themes/file-json.svg`

## 리포트

원본 AppFilePane/SettingsHeader의 설정 파일 열기와 편집 화면 읽기 연결을 구현했습니다. 문서 target은 공유하고 읽기 요청의 pending/error/Session은 owning project/pane/tab별로 관리합니다. SettingsChanged 재읽기와 dirty/숨긴 탭·late/duplicate 보호입니다. 원본 대조에서 임의로 추가한 AppFile 닫기 확인과 model 폐기 차이를 찾아 RED 뒤 제거했습니다. source처럼 즉시 닫고 공유 model은 유지합니다. 실제 저장 API는 [쓰기 경계 후속](2026-10-03-m8-native-app-file-write.md)에 구현했지만 화면 CmdS/HostBridge write와 실제 integration 포트는 미연결입니다. M8 N1~N8은 0/8, 목표는 active입니다.

## 실제 계약과 연결

- 원본 `src/widgets/app-file-pane/app-file-pane.tsx`, Settings header, `src/entities/layout/layout.query.ts` 및 기존 runtime layout open을 확인했습니다. 현재 창의 focused pane을 사용하며 auxiliary scope는 project/slot을 일치시킵니다. runtime의 pane별 AppFile kind dedup·preview=false를 재사용했습니다. 실제 native auxiliary OS 창 배선은 별도입니다.
- Settings header에 높이24/padding6/gap4/text12/icon14의 outlined 버튼을 추가했습니다. Lucide React1.28.0의 설치된 FileJson은 FileBraces alias이며 실제4path를 재사용하고 기존 ISC 고지를 유지했습니다. egui0.36.2 공식 설치 source의 image_and_text·fill/stroke 계약을 확인했습니다. 고정 fill이 hover를 덮지 않도록 기존 scope visuals를 사용합니다. 전체 픽셀·focus ring/AX parity를 주장하지 않습니다.
- AppSurfaces의 AppFile branch는 target별 DocumentKey를 공유하는 기존 NativeEditor를 사용합니다. 실제 project path·file grant·LSP/mirror/autosave/watcher는 붙이지 않습니다. loading은 editor background의 빈 영역이고 실패는 번역된 editor.openFailed 중앙 label입니다. 실제 키맵 등록은 기존 show_document를 경유하지만 AppFile 저장 라우팅은 아직 없습니다.
- owned read guard/lease를 UI admission까지 유지하는 기존 읽기 코어를 HostBridge에 연결했습니다. 동일 owner/session/pending operation을 검사하고 stale/duplicate 또는 폐기된 owner의 응답을 버립니다. SettingsChanged만 별도 settings revision을 올립니다. 테마/터미널 이벤트는 Settings 문서를 다시 읽게 하지 않습니다. 재읽기는 기존 clean 갱신/dirty 보존 코어를 사용합니다.
- 문서가 이미 읽힌 target이면 다른 owning 탭에서 같은 DocumentId를 재사용합니다. 비활성 탭은 살아 있는 owner로 보존합니다. 실제 편집의 dirty 발행은 현재 보이는 view뿐 아니라 동일 target의 모든 살아 있는 TabId에 전달합니다. 닫힌 view와 owner Session은 회수하지만 target4종의 공유 model은 마지막 탭을 닫아도 보존합니다. Session clear/drop은 pending read/write 권한을 폐기합니다. closed/new 탭의 initialDirty/sync와 view state 전체 parity는 아직 남습니다.
- `use-request-close-tab.tsx`의 isDirtyGatedTab은 file/untitled만 포함하며 `layout.query.ts`의 useCloseTab도 file만 모델을 dispose합니다. CodeEditor는 editor를 dispose하고 registry model은 유지합니다. 원본에 없는 AppFile 확인·CloseAll/Exit gate를 추가하지 않으며 app 종료는 기존 mirror/drain 경계를 따릅니다. load_layout은 미러 없는 AppFile의 persisted dirty를 지우므로 재시작 복구를 임의로 추가하지 않습니다. pinned/project/실제 auxiliary/전체 Exit 실기는 미완료입니다.

## 변경 위험별 최소 검증

Cargo는 모두 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`으로 직렬 실행했습니다. 동일 상태의 core/ThemeEditor 성공을 다시 실행하지 않았습니다.

1. `cargo test --lib native_app_file_`: 최초 신규2건 중 공유 문서/SettingsChanged/late/hidden/dirty/read 수명1 PASS, suite0.01초·컴파일10.12초입니다. 당시 닫기 기대는 원본과 달랐으므로 아래 후속이 최종 근거입니다. Host 검사1 FAIL은 실행 중인 HostBridge 작업자를 종료 전에 tracked_count=0으로 기대한 fixture 오류였습니다. disconnect/shutdown 이후0을 검사하도록 정정해 `cargo test --lib native_app_file_host`만 재실행했고1 PASS, suite0.02초·컴파일2.51초입니다.
2. `cargo test --lib native_settings_view는_테마입력`: 기존 영향 검사에 새 header24px geometry/실제 pointer click 출력을 추가했습니다.1 PASS, suite0.08초·컴파일0.23초입니다. 기존 테마/목차/번역/컨트롤 클릭도 같은 검사에서 확인했습니다.
3. `cargo test --lib native_presentation_theme_revision`: 변경된 settings/theme/event counter 분리1 PASS, suite0.00초·컴파일0.21초입니다.
4. `cargo test --lib keybinding_icons는`: 변경된9개 shared SVG의 기존 배율·캐시 수명 검사1 PASS, suite0.01초·컴파일0.20초입니다. 동일 기능의 반복 계측이 아니라 source/texture 배율 속성 검사입니다.
5. `cargo clippy --lib --bins --tests -- -D warnings`: 새 버튼 처리의 collapsible_if1건으로 exit101 후 동일 동작의 let-chain으로 수정했습니다. 해당 정적 검사만1회 재실행해 exit0,12.75초입니다. Wry17개 기존 경고는 dependency의 별도 기록이며 suppression을 추가하지 않았습니다.

닫기 후속: `cargo test --lib native_app_file_views는`의 원본 즉시 닫기 기대를 RED(0.01초·컴파일7.92초)로 재현했습니다. 임의 dirty 확인/Exit latch와 마지막 model 폐기를 제거한 뒤 같은 관련 검사만1회 재실행해 PASS(0.02초·컴파일6.49초)입니다. final 검사는 dirty AppFile의 Batch immediate Close/실제 host 즉시 닫기·dirty closed metadata/aux survivor·마지막 view 회수/model 유지도 덮습니다. [닫기 차이 bug](../bug/2026-10-03-native-app-file-close-parity.md)에 원본과 원인을 기록했습니다.

서로 다른 관련 테스트5건 PASS입니다. 변경되지 않은 성공을 반복하지 않았고, 전수 suite를 실행하거나 기존 keybinding Tab RED를 green으로 보고하지 않았습니다. 최종 native app lib/bin/tests strict는 쓰기 추가 뒤17.21초, 닫기 수정 뒤13.10초 exit0이며 마지막이 현재 근거입니다. authored Rust exact rustfmt와 관련 docs Prettier·whitespace도 확인했습니다.

## 남은 구현·검증

- [x] Settings header/창별 typed open/read·AppSurfaces NativeEditor 기본 읽기 연결
- [x] owner/session/pending 수명·SettingsChanged/clean/dirty/hidden·survivor와 원본 즉시 닫기/model 유지
- [x] 서로 다른 관련 테스트5건과 app lib/bin/tests strict 검사
- [x] 실제 atomic prompt/settings 쓰기 API·canonical 완료/추가 편집·대기 취소는 write QA의 관련2건으로 확인했습니다. 실제 integration 조립 완료는 아닙니다.
- [ ] AppFile CmdS/HostBridge write·pending/오류 toast·닫힌/new 탭 initialDirty/sync/view state
- [ ] 전체 Settings JSON의 실제 IDE→hooks→remote reconcile 포트·시작/종료/실패 관측과 SettingsChanged 이전 순서
- [ ] pinned/project/window/전체 Exit·실제 auxiliary·GUI/IME/VoiceOver/AX/픽셀·큰 읽기/전체 메모리/성능

원본 runtime app_file_write의 mutation guard와 admitted 본문은 쓰기 후속에서 분리했습니다. 원본 SettingsApply의 sanitize/persist/live state→IDE/hooks/remote await→SettingsChanged 순서는 실제 포트 조립에서 유지해야 하며 presentation-only no-op reconcile로 대체하지 않습니다. 저장 실패 원본 메시지는 Settings의 settingsJsonInvalid와 Prompt의 describeIpcError입니다.

실제 저장 API를 화면에 연결하기 전 전체 구현 완료나 M8 완료를 주장하지 않습니다. OS/설정/clipboard/보호 bundle·제품TS·dependency/lock/MSRV·Git은 변경하지 않았고 고유 synthetic 디렉토리만 사용했습니다. 전체 M8 완료 뒤만 선별 commit·일반 push합니다.
