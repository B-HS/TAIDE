# M8 native 탐색기 내부 파일 클립보드

## 원본과 코드 연결

대상은 `native/taide-native-app/src/{explorer,explorer_clipboard,lsp,application,lib}.rs`, `tests/{explorer,explorer-clipboard,lsp}.rs`입니다. 원본은 `src/widgets/explorer/{use-explorer-clipboard,paste-plan,explorer-path,explorer-container}.ts*`, `src/features/explorer/{file-tree,file-tree-context-menu,explorer-shortcuts}.ts*`, `src/shared/lib/unique-entry-name.ts`, `src/entities/file/file.query.ts`입니다. 실제 egui-winit 0.36.2의 clipboard key 변환도 확인했습니다.

- mode/path/kind 한 항목을 저장합니다. 최초 기반은 project-only Explorer owner였으며 아래 후속 구현에서 main shell의 슬롯별 owner로 연결했습니다. multi-selection 전체를 batch 복사하지 않고 primary 하나만 캡처하며 OS clipboard를 읽거나 변경하지 않습니다. 행 Cut/Copy와 행·blank Paste를 locale/shortcut label로 연결했고 clipboard가 없으면 Paste 메뉴만 비활성화합니다. 보조 창 component의 실제 연결·전체 mount/unmount 수명은 별도 미완료입니다.
- 키 순회의 실제 Copy/Cut/Paste와 직접 Key C/X/V를 exact modifier로 연결했습니다. Paste의 OS text를 경로나 파일 내용으로 사용하지 않습니다. Alt/Shift path copy와 내부 Copy를 구분하며 기존 focus/IME/inline editing/popup gate를 유지합니다. 실제 adapter가 발생시키지 않은 이벤트까지 처리 가능하다고 주장하지 않습니다.
- 대상은 현재 page에 있는 directory 자체/file parent/선택 없음 또는 사라진 row의 root입니다. 같은 parent의 Cut→Paste는 clipboard를 비우고 mutation을 제출하지 않습니다. 이는 string 비교이며 경로 canonicalization으로 원본의 비교를 바꾸지 않습니다.
- visible sibling 이름의 case-sensitive set으로 첫 후보를 정합니다. file의 마지막 확장자는 suffix 뒤에 유지하고 dotfile/directory는 확장자를 분리하지 않습니다. locale의 pasteConflictSuffix를 사용합니다. `error.file.destinationExists`만 재시도하고 실패 후보를 taken set에 추가하며 원본과 같은 8회 제한 뒤 실제 마지막 오류를 반환합니다. 숨은 항목을 덮어쓰거나 다른 오류를 재시도하지 않습니다.
- typed Action→LspBridge의 bounded command→기존 TaskSupervisor transient/operation→owned mutation/blocking 작업→typed reply를 연결했습니다. 복사는 live selected project/absolute/canonical root/shutdown을 blocking 작업 안에서 재검사하고 기존 taide-file copy_entry/self-write를 사용합니다. 별도 runtime·새 package·제품 dependency/MSRV 변경은 없습니다.
- Cut은 기존 rename_entry의 mutation만 move_entry로 공유해 workspace_rename의 GUI prepare/commit ack·canonical dirty 문서/탭/초안·generation 수명을 재사용합니다. 원본 GUI rename의 source refresh 뒤 clipboard를 비우고 destination refresh/reveal→선택을 연결합니다. source refresh 전 실패는 Cut을 유지하며 destination refresh/reveal 실패에도 이미 source refresh를 성공했으면 비웁니다. 완료된 filesystem mutation을 후처리 오류에서 rollback했다고 주장하지 않습니다.
- 성공 후 singleton 선택/reveal만 수행하고 파일/폴더 탭을 자동으로 열지 않습니다. Copy는 clipboard를 유지합니다. 원본 capture의 Cut completion이 현재 clipboard를 비우는 규칙도 유지하며, 새 clipboard를 임의로 보호하는 다른 정책을 추가하지 않았습니다. 오류는 현재 native status이며 원본 locale toast surface의 완전한 재현은 남습니다.

## 검사와 실제 결과

Cargo 공통 옵션은 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 동일 코드·입력·환경의 성공은 재사용했습니다.

- [x] lib/bin/explorer/lsp check exit 0, 1.19초입니다.
- [x] `cargo test ... --test explorer 내부클립보드`: 신규 1건 PASS, 0.04초입니다. 실제 형태의 modifier/Copy/Cut/Paste, multi-selection 중 primary만 캡처, OS text 무시, 선택 directory 대상/locale suffix, same-place cut no-op, 실제 row Copy와 enabled/disabled·blank Paste 클릭, root sibling 이름과 완료 후 singleton/clipboard 유지·해제를 확인했습니다. OS에서 직접 누른 키나 픽셀/AX 성공은 아닙니다.
- 최초 explorer-clipboard fixture compile은 존재하지 않는 ProjectLayout.file_tabs 메서드 사용으로 E0599 exit 101입니다. 실제 기존 tabs::tabs_in과 File 필터로 정정했으며 실행 성공으로 세지 않습니다.
- [x] `cargo test ... --test explorer-clipboard`: 신규 2건 PASS, suite 0.03초입니다. 순수 이름 검사와 단일 실제 LspBridge/TaskSupervisor/합성 파일 세션을 연속 실행했습니다.
- 이름 검사는 Unicode file·마지막 tar.gz 확장자·dotfile·v1.2 directory·여러 숫자 충돌·무충돌을 확인합니다. 실제 worker 검사는 보이지 않는 destination collision 뒤 다음 이름 복사/원본·충돌 파일 내용 보존/새 tree row·자동 tab 없음, directory 복사, 숨은 충돌 8개 뒤 오류·추가 파일 없음, missing/outside/relative 거절, dirty 파일 이동의 추가 두 충돌→최신 본문/mirror/key·단일 기존 tab·disk 원본 보존, disconnect/shutdown/tracked_count 0을 확인합니다. OS clipboard·Trash를 사용하지 않았습니다.
- [x] 새 keyboard branch/메뉴 geometry로 영향받은 `cargo test ... --test explorer 복사`: 기존 관련 2건 PASS, 0.08초입니다. 이전 상태의 성공을 이유 없이 반복한 것이 아니라 Copy branch와 메뉴 삽입 후의 경계를 확인했습니다.
- [x] 공통 move_entry 분리로 영향받은 `cargo test ... --test explorer 실제_탐색기_이름변경`: 기존 관련 1건 PASS, 0.01초입니다. 같은 이전 Trash/생성/GUI startup·전체 suite는 반복하지 않았습니다.
- [x] lib/bin/explorer/explorer-clipboard/lsp strict clippy exit 0, 1.22초입니다. 정적 검사에는 lsp exhaustive reply 연결도 포함됐지만 실제 전체 LSP 테스트를 다시 실행하지 않았습니다.
- [x] 최종 app fmt --check·대상 clipboard/system-menu QA와 empty-clipboard bug Prettier·git diff --check exit 0입니다.

## 남은 필수 경계

- [x] **빈/읽기 실패 clipboard의 키 의도 코드 경계**: 기존 실험 eframe 0.36.2 vendor에 기본 비활성 `preserve_empty_paste_shortcuts` feature를 추가하고 격리 native app만 연결했습니다. upstream 변환 후 새 이벤트가 없는 실제 비합성 winit key-down에서만 원본 logical/physical key와 창별 modifier를 확인해 semantic Key::Paste 눌림·해제 명령을 전달합니다. 비어 있지 않은 Paste·다른 event·key-up에는 추가하지 않으며 Text/빈 Paste payload·OS clipboard 쓰기를 만들지 않습니다. 실제 OS 키·clipboard read 실패 실기는 별도 미검증입니다.
- [ ] **terminal·viewport request**: native terminal surface/event adapter는 아직 미연결입니다. Key::V 대신 의미가 분리된 Key::Paste를 전달해 Ctrl+V 바이트와 혼동하지 않게 했지만 실제 PTY 비전송을 이번 검사로 증명하지 않았습니다. 현재 app이 사용하지 않는 ViewportCommand::RequestPaste의 ActionRequested 경로는 변경하지 않았으며 전체 OS 메뉴/terminal gate에서 검토해야 합니다.
- [ ] 현재 main의 슬롯별 clipboard owner와 typed 늦은 회신 수명은 아래 코드 검사만 완료했습니다. 실제 auxiliary Explorer/window host·React tree 재배치 시 component remount parity·프로젝트 전환 뒤 source/target root 정책·pointer 및 keyboard popup focus 복원·메뉴 max-height/scroll/icon/locale toast·실제 픽셀·IME·VoiceOver는 미완료입니다.
- [ ] 기존 rename의 중첩 root/case-only/alias 정책·mutation 후 refresh 실패 주입·취소/종료 race·동시 paste·대형 directory 성능, recursive copy의 descendant symlink/권한·외부 교체 보안은 전체 file gate에서 확인해야 합니다. 이번 plain 합성 directory 성공으로 symlink 재귀/TOCTOU 봉쇄를 주장하지 않습니다.
- [ ] Explorer의 나머지 Open to Side/Open With/terminal/find/compare/history/Git와 전체 N1~N8·213 view·native editor/LSP/terminal·배포/rollback/성능/보안/Rust99%·TS 제거는 미완료입니다.

기존 사용자 앱·실기 bundle·OS 입력기/VoiceOver·실제 clipboard·사용자 파일을 조작하지 않았습니다. 합성 디렉터리만 검사 뒤 fixture owner가 정리했으며 기존 M8 dirty 변경과 TypeScript UI를 유지했습니다. 전체 M8 미완료이므로 commit·push하지 않았습니다.

## 빈 Paste adapter 후속 검사 (2026-10-02)

추가 대상은 실험 vendor의 `src/native/{mod,paste_shortcuts,epi_integration,wgpu_integration,glow_integration}.rs`·Cargo.toml과 격리 app의 Cargo.toml/lock·`tests/paste-shortcuts.rs`·explorer Key::Paste 소비입니다. app lock의 eframe registry source/checksum만 기존 path vendor로 바꾸고 이미 고정된 winit 0.30.13의 직접 dev edge를 추가했습니다. 새로운 package/root manifest/root lock 변경이나 기존 실기 bundle 재빌드는 하지 않았습니다. vendor의 앞선 child AccessKit 수정도 이 격리 path에 포함되지만 이번 headless 성공을 해당 GUI gate의 통과로 계산하지 않습니다.

- [x] 같은 vendor helper source를 path로 컴파일한 실제 modifier/key 분기 검사: 창별 격리·nonempty 기존 Paste 중복 방지·release 제외·Dvorak logical 우선/비Latin physical fallback·Alt 보존·focus loss·Destroyed를 확인했습니다. backend의 실제 live window map 기준 prune도 연결했고 남아 있는 창은 유지/제거된 창은 폐기하는 검사 1건 PASS, 0.00초입니다.
- 최초 source 공유 fixture의 `use egui`는 E0432로 compile 실패해 `crate::egui`와 test root alias로 수정했습니다. 실험 vendor에 새 egui dependency를 추가하지 않았습니다.
- 초기 adapter/실제 Explorer·NativeEditor 소비 2건은 PASS, 0.03초였으나 이후 egui keys_down 검사를 추가하자 눌림만 전달한 Key::Paste의 잔류를 RED로 재현했습니다. 소비 검사 1건 FAIL, 0.01초·exit 101이며 물리 V release는 semantic Paste를 해제하지 못하는 것이 원인입니다. 동일 frame의 의미상 눌림·해제 쌍으로 수정했습니다.
- [x] 최종 `cargo test ... --test paste-shortcuts`: 영향받은 2건 PASS, suite 0.03초·compile 1.42초입니다. 실제 helper 출력이 파일 Paste action을 1회만 만들고 다음 빈 frame/focus 상실에는 실행하지 않으며 Paste keys_down이 남지 않는 것을 확인했습니다. 같은 출력을 focused NativeEditor에 넣어 revision/본문/전체 선택 유지·changed/save/error 없음도 확인했습니다.
- [x] 최종 lib/bin/paste-shortcuts strict clippy exit 0, 0.75초입니다. wgpu 경로와 공유 helper를 compile/lint했으며 비활성 glow backend의 추가 compile은 실행하지 않았습니다. 그 backend prune 배선은 source 대조만 했고 실기/전체 후보 gate에서 확인합니다. 이전 기본 worker·Trash·startup·clipboard 검사와 성능 계측은 반복하지 않았습니다.

공식 고정 egui-winit/egui/winit source를 로컬 registry에서 읽었습니다. docs.rs 조회는 cache miss/접근 실패였고 조회 성공으로 세지 않습니다. helper 검사는 private 플랫폼 필드가 있는 full winit KeyEvent나 실제 OS clipboard를 합성해 통과시킨 것이 아니라 common integration이 넘기는 실제 primitive key/state와 RawInput 변환 경계에 한정됩니다. 원본 이벤트 배선은 wgpu app compile로 확인했습니다.

## Component-local owner 후속 구현 (2026-10-02)

원본 `ShellSlotTreeView`의 같은 leaf 안 `ProjectShell`→`ExplorerContainer`는 projectId prop 교체·Zen·collapsed sidebar에도 같은 component state를 유지합니다. `useExplorerClipboard`의 state는 project별 전역 저장소가 아닙니다. native executable은 현재 Main만 만들고 Auxiliary shell에는 Explorer를 아직 렌더하지 않습니다. 이전 project-only map을 실제 다중 창 clipboard 연결 완료로 세지 않았습니다.

추가 파일은 `src/explorer_clipboard_owners.rs`, `tests/explorer-clipboard-owners.rs`와 app의 application/explorer/clipboard/lib, UI의 shell/workbench fixture입니다. `ShellSurfaces::explorer`에 실제 ShellSlotId를 전달하고 main app이 clipboard를 슬롯별 mount에 보유하도록 연결했습니다. owner는 실제 egui viewport와 단조 mount generation의 typed 값입니다. 다른 창 namespace와 제거 후 같은 slot의 재사용을 구별합니다. 익명 전역 clipboard·OS clipboard·새 dependency는 추가하지 않았습니다.

- [x] snapshot의 전체 slot tree를 기준으로 owner를 retain하므로 숨긴 Zen/접힌 sidebar를 unmount로 오인하지 않습니다. 같은 슬롯의 project prop 교체에도 clipboard를 유지하고 슬롯 제거/전체 close에서는 회수합니다. loading page에서는 clipboard를 덮어쓰지 않습니다.
- [x] 실제 UI에서 캡처한 clipboard를 mount에 반영하고 Paste Request에 owner를 넣어 기존 bounded worker/reply로 전달합니다. Cut completion은 원본처럼 그 살아 있는 component의 현재 clipboard를 비우며 Copy는 유지합니다. 삭제된 owner의 늦은 reply는 새 mount/다른 viewport의 clipboard나 reveal 선택을 변경하지 않습니다. reveal은 살아 있는 mount에서 한 번 소비합니다.
- [x] 최초 slot/generation 검사 1건 PASS, 0.00초였습니다. 추가로 viewport namespace를 typed owner에 포함한 최종 `cargo test ... --test explorer-clipboard-owners` 1건 PASS, 0.00초·compile 4.01초입니다. 두 slot/두 synthetic window owner의 격리·같은 slot의 다른 project로 prop 교체 유지·retain frame·현재 Cut/Copy 완료·reveal 1회·새 mount/닫힌 mount/다른 window의 late reply 무변경을 직접 확인했습니다. 실제 OS 두 창을 연 검사는 아닙니다.
- [x] owner를 받는 실제 Shell 렌더 배선 변경으로 영향받은 UI `--test workbench 실제_레이아웃_렌더` 1건 PASS, 0.06초·compile 1.91초입니다. 호출의 project/slot 쌍이 실제 snapshot tree와 일치하는 것을 새 assertion으로 확인했고 기존 main/Zen/auxiliary 범위도 유지합니다. auxiliary Explorer가 실제 렌더한다고 주장하지 않습니다.
- [x] app lib/bin/explorer-clipboard-owners/explorer-clipboard/explorer strict clippy exit 0, 1.42초입니다. 변경된 Request owner 필드의 기존 worker/input fixture compile도 포함했습니다. UI lib/workbench strict clippy exit 0, 0.66초입니다. 변경 없는 실제 파일/Trash/PTY 성공을 다시 실행하지 않았습니다.

원본 runtime file_copy는 source/target을 각각 열린 project에 resolve합니다. 최초 native copy/move는 source까지 선택 project root에 한정했으며 component-local clipboard의 이전 열린 project source를 거절했습니다. 아래 Copy 후속 수정은 완료했지만 cross-project Cut의 GUI/LSP origin·layout/mirror/source-missing view 경계는 아직 필수 미완료입니다.

- [x] 이번 owner/API 배선 변경의 app/UI cargo fmt, 해당 QA/bug Prettier와 git diff --check exit 0입니다. 실제 OS 앱·clipboard·설정·전체 M8 완료·Git commit/push는 수행하지 않았습니다.

## 프로젝트 전환 뒤 Copy 권한 후속 수정 (2026-10-02)

- [x] 실제 worker가 다른 열린 project의 source를 `error.path.outsideProjectRoot`로 거절하는 RED 1건을 0.00초·exit 101로 재현했습니다. 잘못된 첫 filter의 0 tests 실행은 성공으로 세지 않았습니다.
- [x] 같은 blocking copy 내부에서 기존 root_guard::resolve_owning_project로 source를 검사합니다. target은 현재 선택 project 안으로 유지하고 source의 열린-project/absolute·target live project/absolute·shutdown·owned guard/self-write를 유지합니다. 별도 package·권한 우회·CLI 파일 허용은 없습니다.
- [x] `cargo test ... --test explorer-clipboard 실제_프로젝트전환_copy`: 수정 후 영향 신규 1건 PASS, 0.01초·compile 2.31초입니다. 두 실제 합성 root/Unicode 파일의 내용·source 유지·target row·자동 tab 없음·typed clipboard owner 회신, source project 닫힘 뒤 disk가 남아 있어도 거절/새 후보 없음, 유효 source에서 root 밖 target 거절·relative source/target 거절, worker disconnect/shutdown·tracked_count 0을 확인했습니다.
- [x] app lib/bin/explorer-clipboard strict clippy exit 0, 0.59초입니다. 변경 없는 기존 충돌/dirty 이동/clipboard owner·Shell·입력/Trash/PTY 성공은 재사용합니다.
- [x] 분리 루트 간 GUI Cut의 기본 경로는 아래 별도 worker로 수정했습니다. 선택 root 내부의 중첩-project 경계 이동과 전체 alias/동시 변경/실기 경계는 계속 미완료입니다. 기존 LSP root-transfer 보호는 유지합니다.

새 bug 문서 `docs/bug/2026-10-02-native-paste-project-scope.md`에 증거와 남은 origin 경계를 기록했습니다. 실제 GUI·OS clipboard·사용자 파일/설정·전체 M8·commit/push는 실행하지 않았습니다.

- [x] app cargo fmt와 git diff --check exit 0입니다. `.prettierignore`가 docs를 제외하므로 최종 관련 QA/bug 3개는 `prettier --write --ignore-path /dev/null`로 명시 검사했고 각각 unchanged·exit 0을 확인했습니다. 기본 ignore 실행의 exit 0만으로 문서 포맷 검사 성공을 판단하지 않습니다.

## 분리 루트 간 Cut과 source-missing 후처리 (2026-10-02)

대상은 app `src/{explorer_move,explorer_source_missing,explorer_delete,explorer_clipboard,application,lsp,lib}.rs`와 `tests/{explorer-clipboard,lsp}.rs`입니다. GUI-confirmed move를 LSP rename과 분리하고 기존 GUI 삭제의 문서 해제·survivor mirror capture를 두 실제 소비 경로에서 공유했습니다. 새 dependency·외부 root/CLI 권한·실기 bundle 변경은 없습니다.

원본 runtime `layout_apply_path_change`는 renamed from/to 모두 선택 root 안일 때만 후처리합니다. 따라서 disjoint root Cut은 실제 파일만 이동하며 선택/원본 project 탭·ID는 old path 그대로이고 최신 source draft를 readonly로 남깁니다. 후처리 대상을 선택 project라는 이유만으로 target path로 변경해야 한다는 초기 가정은 원본 guard 대조 후 폐기했습니다. foreign project 탭의 source mirror는 그 project 밖에 쓰지 않고 실제 source project에 저장합니다.

- [x] 실제 신규 worker의 기존 선택-root 거절 RED 1건(0.01초)→수정 검사 PASS 1건(0.04초·compile 3.62초)입니다. E0502 fixture compile 실패는 실제 실행과 구분했습니다.
- [x] 파일 이동 뒤 unrelated root canonicalize 오류가 GUI ack를 생략할 수 있고 event publish가 projects read 잠금 안에서 실행될 수 있음을 source에서 확인했습니다. 알림 대상을 이동 전에 계산하고 잠금을 해제한 후 nonfallible publish만 실행합니다. invalid unrelated root와 실제 FsRescanRequired callback의 projects.try_write assertion을 추가한 최종 신규 검사 1건 PASS(0.04초·compile 2.55초)입니다.
- [x] 실제 합성 두 root/파일 이동·disk 원본 내용·기존 source/foreign 탭과 ID 유지·최신 dirty rope mirror·source_missing draft·foreign project mirror 없음·두 editor view/원본 문서 해제·target tree row·Cut clear·disconnect/shutdown/tracked_count 0을 확인했습니다.
- [x] 공유 문서 해제의 변경 영향 `--test explorer-delete 삭제_ack` 1건 PASS(0.00초)이며 late revision 거절과 confirmed dirty/pinned history 해제를 유지했습니다. Cut branch 변경 영향 `--test explorer-clipboard 실제붙여넣기는` 1건 PASS(0.02초)입니다. 실제 Trash·Copy/owner/Shell/입력/PTY 성공은 반복하지 않았습니다.
- [x] app lib/bin/clipboard/delete/lsp strict clippy exit 0(0.95초)입니다. 원본 helper extraction은 compile 및 두 실제 사용 경계로 확인했으며 전체 suite/OS/앱 재시작은 실행하지 않았습니다.
- [x] 중첩 project의 source가 선택 root 안이면서 더 좁은 opened root를 벗어나는 기본 GUI 이동은 아래 move_selected 경로로 수정했습니다. LSP guard는 유지하며 전체 alias/수명/실기는 미완료입니다.
- [ ] alias/symlink/root-directory·case-only·late ack/동시 변경·실제 host UI/Save As·OS clipboard/terminal/auxiliary·전체 M8 gate는 미완료입니다. 이번 plain disjoint 검사로 보안·전체 파일 생명주기·N1~N8 통과를 주장하지 않습니다.

## 중첩 root의 선택-project 이동과 일반 GUI 이름 변경 (2026-10-02)

대상은 `src/{explorer_move,explorer,explorer_clipboard,workspace_rename,application}.rs`와 `tests/explorer-clipboard.rs`입니다. GUI move_selected는 선택 layout/mirror만 retarget하고 다른 project의 old 탭은 최신 source-missing draft로 유지합니다. 기존 LSP apply에는 survivor_drafts 빈 map만 추가했고 root-transfer guard/all-matching 정책은 그대로입니다. 두 actual move 경로에서 알림 대상 계산을 공유합니다.

- [x] 중첩 root Cut의 기존 LSP root-transfer 거절 RED 1건(0.01초·compile 0.78초)을 재현했습니다. Option.get 결과와 ViewStore의 실제 API에 맞지 않는 E0599 compile 실패는 runtime RED/PASS와 구분했습니다.
- [x] 최초 신규 중첩 worker 1건 PASS(0.03초·compile 3.45초)입니다. 실제 nested→outer 이동·disk 내용·선택 project 탭/ID만 retarget·내부 project old 탭/ID 유지·최신 dirty rope·canonical 이동·source readonly view만 해제·각 project mirror의 path/source_missing·LSP 동일 이동 거절·tracked_count 0을 확인했습니다.
- [x] 이동 전 affected project 계산을 두 경로에서 공유하고 source survivor가 selected mirror를 사용하면 old mirror 삭제를 생략합니다. 공유 변경 영향 최종 `--test explorer-clipboard cut` 2건 PASS(0.05초·compile 2.56초)입니다. 같은 변경 상태로 재반복하지 않습니다.
- [x] 선택-root 내부 일반 Cut도 GUI origin으로 분리했으므로 영향 기존 `--test explorer-clipboard 실제붙여넣기는` 1건 PASS(0.01초)입니다. 기존 선택/충돌/dirty canonical retarget·mirror·worker 회수를 유지합니다. 일반 Explorer 이름 변경의 move_entry도 같은 selected 경로로 배선한 뒤 관련 `--test explorer 실제_탐색기_이름변경` 1건 PASS(0.03초·compile 2.90초)입니다.
- [x] 최종 app lib/bin/explorer/clipboard/delete/lsp/lsp-workspace-worker strict clippy exit 0(1.04초)입니다. 변경된 shared Renamed 계약과 모든 해당 fixture를 compile/lint했으며 actual 전체 LSP/Trash/앱 재시작/OS clipboard·Copy/owner/Shell/키 입력 성공을 반복하지 않았습니다.
- [ ] canonical/display alias·nested overlapping/case-only mirror·동시 save/종료/late ack·실제 host readonly와 Save As·모든 파일 원자성/보안·전체 clipboard/N1부터 N8 gate는 미완료입니다. 새 package·root/MSRV/TS 삭제·실기 bundle 변경·commit/push는 없습니다.
