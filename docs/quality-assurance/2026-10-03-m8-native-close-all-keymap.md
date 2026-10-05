# M8 Close All의 묶음 확인·준비·순차 닫기

## 대상·원본 계약

- `native/taide-native-app/src/application.rs`, `tab_close_batch.rs`, `close_dialog.rs`, `shell_keymap.rs`, `lib.rs`와 `native/taide-native-ui/src/commands.rs`를 변경했습니다. PROCESS N4-C의 남은 APP_KEYMAP 실행 연결 범위이며 main이 직접 수행했습니다.
- 원본 `group-shortcut-targets.ts`, `use-request-close-tab.tsx`, `close-tabs-serially.ts`, `close-dirty-tab-dialog.tsx`를 읽었습니다. focused group의 snapshot strip 순서에서 pinned를 제외하고 File/Untitled dirty 전체를 한 번 질문합니다. Save는 모든 dirty 준비 성공 뒤만 닫고 Save As 취소/실패는 아무 탭도 닫지 않습니다. Discard는 dirty flag를 clean으로 보내고 Cancel은 전체 요청을 없앱니다.
- 이미 사라진 NotFound는 닫기 성공과 같은 결과로 건너뛰고 다른 close 실패는 첫 오류를 보존하면서 나머지를 계속 처리합니다. 현재 연결 수는 24 shell action과 terminal 자체 2 이동의 기본 경로이며 전체41/OS/뷰 완료율이 아닙니다.

## 구현

- [x] `RequestCloseTabs`→현재 snapshot/Core dirty 확인→Batch를 기존 `PendingTabClose` 안에 연결했습니다. 기존 busy/close-confirm guard를 공유해 두 번째 요청을 쌓지 않습니다. 기존 단일 탭 요청은 batch 없이 기존 경로를 유지합니다.
- [x] Batch는 Confirm→전체 Prepare→strip Close 단계입니다. Save/Discard 준비가 남아 있는 동안 Close 작업을 만들지 않습니다. 준비한 discard Core revision과 Save As가 반환한 실제 TabId를 유지합니다. Save failure/취소로 Confirm에 돌아온 preparing batch는 취소하고 다음 탭의 저장/닫기를 시작하지 않습니다.
- [x] 실제 기존 Save/File participant·Untitled/MissingDraft Save As·Close host/release를 사용합니다. batch Discard의 SetDirty는 pending dirty queue에 보내고 Close 전 기존 flush/serial host를 거칩니다. Core를 직접 지우거나 pinned/dirty 확인을 우회하는 새 backend를 만들지 않습니다.
- [x] 기존 single description과 Many description의 en/ko/ja 카탈로그를 그대로 사용해 모든 dirty title/count를 표시합니다. Modal ID는 동일하며 준비 중 busy 표시와 Escape/Cancel이 기존 UI 경로를 사용합니다.

## 실제 검사

Cargo는 기존 CARGO_HOME·locked/offline·격리 target으로 직렬 실행했습니다.

1. app `--lib close_all` 신규 1 PASS(compile 4.49초·suite 0.01초)입니다. 실제 focused intent의 pinned 제외/strip 순서, 한 번 Confirm의 전체 titles, Cancel/두 번째 Save 대기에서 Close 없음, 변환된 TabId, 준비 완료 뒤 실제 HostBridge의 순차 Close/중간 NotFound/뒤 탭 계속/dirty closed stack clean/pinned 보존/worker task 0을 확인했습니다. 취소와 Save 실패는 coordinator를 종료했을 때 상태가 불변임을 확인한 것이며 실제 OS Save As 실패 실측으로 확대하지 않습니다.
2. app `--lib close_all_cancel` 최종 1 PASS(compile 1.52초·suite 0.06초)입니다. 실제 `PendingTabClose::failed_batch_prepare`의 automatic action/Confirm failure/Saving/Ready/single 분기와 실제 Many Modal의 en/ko/ja 표시·placeholder 치환·Escape Cancel을 확인했습니다. locale fixture는 기존 runtime 경로를 사용하며 OS 설정/사용자 파일을 변경하지 않고 data dir도 만들지 않았습니다.
3. 실제 App 배선 check lib exit 0(1.26초), app strict 최종 exit 0(1.10초), native-ui lib strict exit 0(0.40초)입니다. authored 6파일 exact rustfmt와 tracked diff check exit 0입니다. 기존 Wry 경고 17개와 authored strict는 구분하며 suppression/unsafe/package 추가는 없습니다.

### 실패와 정정

- strict는 single dialog wrapper의 `[title.clone()]`를 불필요한 clone으로 거절했습니다. `std::slice::from_ref`로 고치고 해당 strict만 다시 실행했습니다.
- 새 UI fixture의 직접 `taide_locale` 접근은 app direct dependency가 없어 E0433, `AppPaths.clone()`은 Clone 구현이 없어 E0599로 compile되지 않았습니다. 기존 runtime locale action과 실제 AppPaths의 소유권 이동으로 정정했습니다. 테스트가 실행되지 않은 compile 오류를 통과나 반복 계측으로 세지 않습니다.
- UI 첫 실제 실행은 첫 프레임에 text shape가 있다고 가정해 실패했습니다. 설치 egui 0.36.2 Area source는 최초 sizing pass를 의도적으로 invisible로 만듭니다. 기존 Escape를 확인하는 표시 프레임에서 같은 문구를 확인하도록 옮겼고 texture delta는 assertion 전에 회수해 실패 시 destructor의 추가 abort도 방지했습니다. 해당 UI 검사만 수정 뒤 한 번 실행했습니다. 최초 host/coordinator 성공은 재사용했습니다.

## 남은 전체 gate

- [ ] 실제 NativeApplication 창에서 mounted/unmounted File·Untitled·missing draft의 Save As 성공/취소/readonly·format failure·late edit·중간 pin/close/rename·queue-full/shutdown·다중/aux 창을 전체 흐름으로 확인합니다. 이번 검사는 coordinator/실제 host/상태 판정/Modal component이며 전체 앱 OS 입력 실측이 아닙니다. 기존 save participant·단일 탭 release 성공은 변경 없는 경로에서 해당 QA를 재사용합니다.
- [ ] editor text focus의 ObserveEditorPrefix/DeferToEditor에 대응하는 native editor command binding이 아직 없습니다. 원본 CodeEditor는 editor-group-shortcut-actions를 등록하므로 기본 Cmd+K 그룹/Close All을 실제 editor 안에서 실행하는 연결은 다음 작업입니다. neutral/window scope의 공통 resolver 연결만으로 그 gate를 완료하지 않습니다.
- [ ] 나머지 action/palette·raw Unicode/IME/AX·aggregate·remount A/B 결정과 전체 M8 N1~N8 0/8·213 view/cutover/TS 제거·성능/보안/배포가 남습니다. 보호 bundle·OS clipboard/browser/입력기/VoiceOver·제품 TS/root/MSRV는 유지했고 전체 완료 뒤만 commit/push합니다.

## 문서·whitespace

docs를 기본 제외하는 `.prettierignore`를 무시해 이 QA 경로만 명시한 Prettier write unchanged/check exit 0입니다. authored Rust 6파일과 QA의 untracked no-index whitespace 출력은 모두 비어 있으며 /dev/null 내용 차이 exit 1은 실패로 분류하지 않습니다.
