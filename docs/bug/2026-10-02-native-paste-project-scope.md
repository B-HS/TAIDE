# Native 붙여넣기의 프로젝트 전환 권한 차이

## 원본과 증상

원본 `useExplorerClipboard`는 component-local state이므로 같은 shell slot에서 projectId prop을 바꿔도 이전 source를 유지합니다. `taide-runtime::file_actions::file_copy/file_rename`은 source와 destination을 각각 열린 project에 resolve합니다. 반면 native clipboard copy는 source까지 선택 project root에 한정했고 explorer move_entry와 workspace_rename은 프로젝트 root 간 이동을 거절했습니다.

실제 LspBridge·TaskSupervisor·두 합성 프로젝트의 신규 복사 검사에서 원본은 열린 source project에 있고 destination은 현재 선택 project에 있는 요청이 `error.path.outsideProjectRoot`로 실패했습니다. 1건 FAIL, 0.00초·exit 101입니다. OS clipboard·실제 앱/사용자 파일을 사용하지 않았습니다.

## Copy 수정과 검증

`native/taide-native-app/src/explorer_clipboard.rs`의 실제 blocking copy 안에서 source를 기존 `resolve_owning_project`로 검사하도록 수정했습니다. destination은 현재 선택 project의 root로 검증하고 live project·absolute path·shutdown·owned mutation guard·TaskSupervisor·self-write 정책을 유지합니다. CLI-opened path나 열린 project 바깥으로 권한을 넓히지 않았습니다.

- `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer-clipboard 실제_프로젝트전환_copy --locked --offline --target-dir experiments/native-shell-spike/target`: 수정 후 신규 1건 PASS, 0.01초·compile 2.31초입니다.
- 열린 source project→현재 target의 실제 파일 복사·내용/원본 보존·target tree row·자동 tab 없음·typed owner가 bounded 요청/회신을 왕복하는 것을 확인했습니다. source project를 닫은 뒤에는 source 파일이 여전히 disk에 있어도 거절하고 새 후보 파일을 만들지 않습니다. 유효 source에서 열린 root 밖 destination과 relative source/destination도 거절합니다. 실제 worker disconnect/shutdown·tracked_count 0도 확인했습니다.
- 첫 filter의 잘못된 문자열은 0 tests/3 filtered였고 성공 검사로 계산하지 않았습니다. 정확한 filter의 RED를 확인한 뒤 수정 영향 검사만 1회 실행했습니다. 이전 충돌/dirty move·입력/owner/Shell 성공은 반복하지 않았습니다.
- app lib/bin/explorer-clipboard strict clippy exit 0, 0.59초입니다.
- app cargo fmt·git diff --check와 docs ignore를 제외한 대상 QA/bug Prettier가 exit 0입니다.

## 분리 루트 간 Cut 수정

`workspace_rename::apply`의 LSP root-transfer 보호는 유지했습니다. GUI Paste에만 `explorer_move::try_cross_project_move`와 typed ExplorerMovePrepare/ExplorerMoved를 추가했습니다. source가 선택 root 밖일 때 source/target을 각각 열린 entry-owning project로 검증하고 target은 선택 root 안으로 한정합니다. mutation guard와 tracked operation·ActivityOwner를 GUI 최신 문서 준비, blocking identity 재검증, 실제 rename, source-missing 문서 해제 ack까지 유지합니다.

원본 `useRenameEntry(projectId)`의 `followRenamedPathInTabs`는 선택 project를 대상으로 하지만 runtime `layout_apply_path_change`의 `ensure_change_within_root`가 from/to **모두** 선택 root 안인지 검사합니다. disjoint cross-root에서는 후처리가 거절되고 TS callback은 이를 잡아 물리적 이동은 성공으로 유지합니다. 따라서 기존 source/선택 project 탭·ID는 old path 그대로이며 target tab을 자동 생성하거나 기존 탭을 target path로 retarget하지 않습니다. source-parent tree refresh는 외부 경로의 cache invalidation/no-op일 수 있습니다.

`explorer_source_missing`으로 기존 GUI 삭제의 snapshot 준비·revision/key 검증·editor view 해제·최신 survivor mirror 캡처를 공유합니다. 미저장 문서가 있으면 기존 stale mirror보다 최신 rope를 우선합니다. foreign path 탭을 소유한 project가 source canonical을 허용하지 않을 경우 실제 source project에만 mirror를 저장하므로 root 밖 mirror/Save As owner를 만들지 않습니다. GUI host는 삭제와 같은 autosave 취소·누락 source 보호와 readonly draft 연결을 사용합니다.

- 최초 fixture의 E0502 borrow compile 실패는 view ID를 먼저 바인딩해 수정했습니다. runtime 검사 성공으로 세지 않습니다.
- 기존 native 경로의 `native explorer rename is outside its project` RED 1건(0.01초)을 확인했습니다. 원본 탭 후처리 guard를 확인한 뒤 테스트 기대를 원본에 맞췄습니다.
- 신규 cross-root worker 1건 PASS(0.04초·compile 3.62초) 뒤, 이동 후 fallible root scan/잠금 재진입 경로를 수정했습니다. fixture에 invalid unrelated root와 FsRescanRequired callback의 projects.try_write 검사를 추가한 최종 영향 1건 PASS(0.04초·compile 2.55초)입니다. 두 루트/실제 이동·disk 내용·기존 두 탭/ID·최신 mirror/readonly draft 데이터·foreign project mirror 없음·문서/view 해제·tree row·worker 종료/tracked_count 0을 확인했습니다.
- 공유 해제 변경 영향 `--test explorer-delete 삭제_ack` 1건 PASS(0.00초), Cut branch 변경 영향 `--test explorer-clipboard 실제붙여넣기는` 1건 PASS(0.02초)입니다. actual Trash/앱 재시작/OS clipboard는 반복하지 않았습니다.
- app lib/bin/explorer-clipboard/explorer-delete/lsp strict clippy exit 0(0.95초)입니다. unchanged Copy/owner/Shell/keyboard 성공은 재사용했습니다.

## 중첩 root GUI 이동/이름 변경 수정

source가 선택 root 안이어도 더 좁은 opened root를 벗어나면 기존 workspace_rename이 거절하는 RED 1건(0.01초)을 실제 worker로 확인했습니다. GUI origin만 `explorer_move::move_selected`로 분리하고 일반 Explorer 이름 변경도 이 경로를 사용합니다. source/target 모두 선택 root 안인지 blocking 준비·identity 재검증 때 확인하고 선택 layout만 retarget합니다. 다른 project layout의 old 탭은 유지하며 source_missing 최신 draft를 이동 전에 캡처합니다.

typed `Renamed.survivor_drafts`를 GUI 경로에서 채우고 기존 LSP 경로에서는 빈 map으로 유지합니다. 문서 key/revision 검증 뒤 missing tab의 editor view만 해제하고 canonical 문서는 new path/metadata로 retarget합니다. GUI host는 readonly draft map을 반영하며 기존 autosave 취소·LSP 조정과 Activity/owned guard ack를 유지합니다. survivor draft가 selected project의 old mirror를 사용하면 그 mirror를 지우지 않고 new mirror와 함께 보존합니다. LSP의 root-transfer 거절과 all-matching layout 정책은 변경하지 않았습니다.

- fixture의 Option에 Result 메서드를 쓴 E0599와 ViewStore.iter 부재 E0599는 실제 API를 확인해 수정했고 runtime 실패와 구분했습니다.
- 최초 신규 중첩 검사 1건 PASS(0.03초·compile 3.45초) 뒤, 두 소비 경로의 알림 계산을 공유하고 survivor mirror 삭제를 막았습니다. 해당 공유 변경의 최종 `--test explorer-clipboard cut` 2건 PASS(0.05초·compile 2.56초)입니다. 중첩 root의 실제 이동·선택 탭/ID retarget·내부 project 탭/ID 유지·new canonical dirty body·old view만 해제·두 project의 latest mirrors/source_missing 분리·LSP root-transfer 요청의 실제 거절·worker 회수/tracked_count 0을 확인했습니다.
- 변경된 일반 GUI Cut 경로의 영향 `--test explorer-clipboard 실제붙여넣기는` 1건 PASS(0.01초)이며 기존 충돌/dirty retarget/미러·회수를 유지합니다. 같은 GUI 이름 변경 배선의 `--test explorer 실제_탐색기_이름변경` 1건 PASS(0.03초·compile 2.90초)입니다.
- 최종 app lib/bin/explorer/clipboard/delete/lsp/lsp-workspace-worker strict clippy exit 0(1.04초)입니다. unchanged Copy/owner/Shell/keyboard·Trash/실기 앱/OS clipboard 성공은 반복하지 않았습니다.

## 남은 Cut 경계

alias/symlink/root-directory·case-only·동시 변경/종료·late ack·원본 non-preview/provider 상태와 실제 GUI/OS 및 전체 clipboard·M8은 미완료입니다. canonical store·typed reply의 worker 검사는 실제 host 화면 페인트/Save As 전체 성공을 뜻하지 않습니다.

관련 정본은 `docs/quality-assurance/2026-10-02-m8-native-explorer-clipboard.md`와 `docs/PROCESS.md`입니다. Copy의 좁은 수정은 전체 clipboard·M8·213 view·OS/보안 gate의 완료 근거가 아닙니다.
