# 프로젝트 rail 메뉴·닫기·슬롯 열기·표시 편집

## 대상

- shell-36: rail 프로젝트 context menu. 닫기, Open to the Right/Below/Left/Above, 그룹 추가/새 그룹/제거, 파일 관리자 열기, 경로 복사, 표시 편집/초기화를 포함합니다.
- shell-37: 프로젝트 닫기(`project_close`)입니다.
- shell-53: "Open to the Right/Below/…" 메뉴(slot 분할 열기)입니다.
- shell-38: 프로젝트 표시 편집 dialog(mode·아이콘 검색·라벨·12색·미리보기)와 초기화입니다.
- 원본: `src/widgets/app-sidebar/sortable-project-icon.tsx`, `src/widgets/app-sidebar/app-sidebar.tsx`, `src/features/project/project-display-dialog.tsx`, `src/shared/lib/project-display.ts`, `src/shared/constants/project-display.ts`, `src/shared/lib/project-group.ts`

## 원본 동작

- 메뉴 순서(`sortable-project-icon.tsx` 169-203): 닫기, 구분선, 슬롯 열기 4개, 구분선, "그룹에 추가" 하위 메뉴, "그룹에서 제거", 구분선, 파일 관리자 열기, 경로 복사, 구분선, 표시 편집, 표시 초기화 순서입니다.
- 닫기(170): `closeProject(project.id)` → `project_close`입니다. 실패 처리 옵션이 없습니다(`project.query.ts` 98-104).
    - Tauri `project_close`(`src-tauri/src/domain/project/commands.rs` 340-408)는 먼저 모든 창에 이 프로젝트의 dirty 버퍼를 hot-exit mirror로 flush하게 하고 기다립니다. 그다음 runtime `project_actions::project_close`(`crates/taide-runtime/src/project_actions.rs` 462-500)가 세션·상태를 지우고 capability를 detach합니다.
- 슬롯 열기(41-46, 172-176): 순서는 Right, Bottom, Left, Top이고 키는 `shellSlot.openToTheRight/openBelow/openToTheLeft/openAbove`입니다.
    - `canOpenInShellSlot`이 false면 비활성입니다. 값은 `!!focusedShellSlotId`입니다(`app-sidebar.tsx` 138).
    - `focusedShellSlotId`는 `resolveShellSlotFocus`입니다(`src/shared/lib/shell-slot.ts` 41-45). 요청한 slot이 leaf면 그것, 아니면 첫 leaf, leaf가 없으면 null입니다.
    - 선택하면 `project_open_in_slot({path: null, projectId, targetSlot: focused, edge})`를 보내고 실패는 toast입니다(`app-sidebar.tsx` 103-109).
- 그룹(178-192, `app-sidebar.tsx` 111-128):
    - 하위 메뉴는 모든 그룹을 순서대로 보입니다. 현재 그룹은 비활성입니다. 그룹이 있으면 구분선 뒤에 "새 그룹"이 옵니다.
    - 현재 그룹은 `groupIdByProjectId`입니다. 여러 그룹이 같은 프로젝트를 가지면 마지막 그룹입니다(`project-group.ts` 42-52).
    - 추가는 `withProjectGroupMember`입니다. 이미 있으면 그대로, 없으면 끝에 붙여 `project_group_set_members`를 보냅니다(59-60).
    - 제거는 현재 그룹이 없으면 비활성이고, 있으면 그 그룹 멤버에서 이 프로젝트를 빼 보냅니다(62-63).
    - "새 그룹"은 그룹 dialog(`project-group-dialog.tsx`)를 엽니다.
- 파일 관리자 열기(194-196): `systemOpenPath(project.root)` → `system_open_path`입니다(`src-tauri/src/domain/system/commands.rs` 88-92). 실패는 toast입니다.
- 경로 복사(197): `copyTextToClipboard(root)`입니다. 실패만 `common.copyFailed` toast입니다(`src/shared/lib/copy-text-to-clipboard.ts` 22-30).
- 표시 초기화(200-202): `isProjectDisplayCustomized(display)`가 false면 비활성입니다(`project-display.ts` 98). 선택하면 `CLEARED_PROJECT_DISPLAY_PATCH`(세 축 모두 `""`)를 `project_set_display`로 보냅니다. 실패는 toast입니다.
- 표시 편집 dialog(`project-display-dialog.tsx`):
    - 열 때마다 필드를 다시 채웁니다(66-74). mode는 `resolveProjectDisplay(display).mode`, 라벨은 `clampProjectLabel(display.label)`, 아이콘은 `resolveProjectIconName`(정확한 카탈로그 id, 아니면 folder), 색은 `resolveProjectColorToken`입니다.
    - mode 라디오는 아이콘, 텍스트 라벨, 폴더 아이콘 순서입니다(31-37, 95-108).
    - 아이콘 mode(110-139)는 cmdk 검색 입력(`project.displayIconSearch`), 8열 격자, 최대 높이 176px입니다. 결과가 없으면 `palette.noResults`입니다. 고른 아이콘은 accent 링과 itemActive 배경입니다.
    - 라벨 mode(141-153)는 자동 focus, 4코드포인트 제한, 입력마다 `clampProjectLabel`(제어문자 제거, trim 없음)입니다. 옆에 힌트가 있습니다.
    - 색(155-173)은 12개 원(24px)입니다. 누르면 토글되고, 눌린 색은 accent ring입니다. 이름은 `Color N`입니다.
    - 미리보기(175-183)는 40px 사이드바 배경 상자에 glyph와 프로젝트 이름을 보입니다.
    - 저장(80-83, 186-193)은 `mode !== 'label' || normalizedLabel`일 때와 대기 중이 아닐 때만 됩니다. 세 축을 모두 보내고, 쓰지 않는 축은 `""`입니다. 성공하면 닫고 실패하면 toast와 함께 열어 둡니다(`sortable-project-icon.tsx` 116-120, 206-213).
- cmdk 1.1.1(`package.json` 53) 기본 filter는 `commandScore` > 0만 남깁니다. 검색이 있으면 점수 내림차순으로 정렬하고, 검색이 바뀌면 첫 항목을 선택합니다. ↑/↓/Home/End로 선택을 옮기고 Enter로 고르며, 반복(loop)은 하지 않습니다.

## 구현

- UI(`native/taide-native-ui`):
    - `shell.rs`의 `project_button`이 버튼 응답에 context menu를 붙입니다(622). `project_menu`(1338)가 원본 순서와 비활성 조건으로 항목을 그립니다. `SHELL_SLOT_OPEN_EDGES`(1329)가 슬롯 순서를 고정하고, `enabled_menu_item`(1311)은 비활성 항목을 그립니다. 기존 `menu_item`은 이것을 씁니다.
    - 현재 그룹은 `groups`를 뒤에서 찾은 첫 그룹입니다. 원본 sections의 "마지막 그룹" 규칙과 같습니다. 추가는 이미 멤버면 그대로, 아니면 끝에 붙입니다.
    - `ShellSnapshot::focused_slot`(`snapshot.rs` 54)은 `resolveShellSlotFocus`를 옮긴 것입니다. 슬롯 열기의 활성 여부와 대상 slot에 씁니다.
    - `ShellIntent`에 7개를 추가했습니다(`commands.rs`): `OpenPath`, `CloseProject`, `OpenProjectInSlot`, `SetProjectGroupMembers`, `NewProjectGroup`, `EditProjectDisplay`, `SetProjectDisplay`. `ShellMutation`은 바꾸지 않았으므로 동결 remote-web의 `mutation_call` 전수 match와 소스는 그대로입니다.
    - `project-rail.rs`에 원본 보조 함수를 옮겼습니다(82-130). `resolve_project_color_lane`, `resolve_project_icon_name`, `clamp_project_label`, `normalize_project_label`(백엔드 `sanitize_display_label`처럼 `is_control` 제거 후 trim), `is_project_display_customized`, `cleared_project_display_patch`입니다.
    - 라벨/아이콘 glyph 그리기는 `paint_project_glyph`(159)로 옮겼습니다. rail 버튼과 dialog 미리보기가 같이 씁니다.
    - `project-display-dialog.rs`를 추가했습니다. `ProjectDisplayDialog`가 `open`(86), `finish`(116), `close`, `show`(127)를 제공합니다.
        - mode 라디오, cmdk 방식 아이콘 검색(`icon_matches` 477은 `command_score` > 0, 점수 내림차순, 안정 정렬), 격자(8열·4px 간격·176px 스크롤)를 그립니다. 키는 ↑/↓/Home/End/Enter를 search focus일 때만, IME 조합 중이 아닐 때 소비합니다. hover 이동은 선택을 옮기고 클릭은 고릅니다.
        - 그 밖에 라벨 입력(자동 focus·`char_limit` 4·변경마다 clamp), 12색 토글, 40px 미리보기, Cancel/Save가 있습니다.
        - Save는 `can_save`와 대기 상태를 지키고, 세 축 전체 patch를 돌려줍니다.
- App(`native/taide-native-app`):
    - `host.rs`에 `HostCommand::CloseProject/OpenProjectInSlot/SetProjectGroupMembers/SetProjectDisplay`와 `HostReply::ProjectDisplaySet`를 추가했습니다(dispatch 1590-1659). 리뷰 수정에서 `HostReply::ProjectClosed`를 더했습니다.
        - 닫기와 슬롯 열기는 remote `project_close`/`project_open_in_slot`(`remote-projects.rs` 142-218)과 같은 순서입니다. 존재·배치 검증, view state flush(닫기는 layout flush도), `HostIntegrations::projects`(473)가 만든 `NativeProjects`(terminal hub 포함) lifecycle로 runtime 함수를 부르는 순서입니다.
        - 그룹 멤버와 표시는 runtime `project_group_set_members`/`project_set_display`를 그대로 부릅니다.
    - `application.rs`가 새 intent를 처리합니다(7972-7998, 리뷰 수정 뒤 줄 번호). `OpenPath`는 기존 `HostCommand::OpenPath`(system_open_path), 경로 복사는 기존 `CopyText`를 씁니다. 실패는 `common.copyFailed` toast입니다.
    - `close_project`(3462)는 `flush_dirty` 뒤 이 프로젝트 파일/untitled 문서의 대기 중 mirror job을 먼저 보내고 `CloseProject`를 보냅니다. host 명령 큐는 직렬이므로 mirror가 close보다 먼저 씁니다. 원본 `await_project_flush`에 대응합니다.
    - 표시 dialog는 앱이 소유합니다. 메뉴 intent가 snapshot의 `ProjectRef`로 엽니다. 매 프레임 `show` 결과를 `SetProjectDisplay`로 보내고, `ProjectDisplaySet` 응답이 `finish`와 실패 toast를 처리합니다(1866). 프로젝트가 사라지면 닫습니다.
    - 아이콘은 rail과 같은 `paint_rail_icon`(8461)으로 칠합니다. 두 곳에서 쓰므로 `AppSurfaces::rail_icon`에서 꺼냈습니다.
    - dialog가 열린 프레임에는 Zen 해제 Escape를 막습니다(8221).

## 작은 결정과 근거

- 파일 관리자 열기: 작업 지시는 "Finder reveal 경로 재사용"이었습니다. 원본은 `systemOpenPath`(`system_open_path`, 폴더 자체를 엶)이고 native에 같은 `HostCommand::OpenPath`가 이미 있어 그것을 썼습니다. reveal은 상위 폴더에서 항목을 선택하므로 동작이 다릅니다.
- 프로젝트 닫기 실패: 원본 `useCloseProject`에는 onError가 없어 조용히 실패합니다. native는 다른 rail 동작처럼 `HostReply::Failed` toast로 알립니다. 실패를 숨기는 원본의 결함은 재현하지 않았습니다.
- "새 그룹": 그룹 생성 dialog(shell-46)는 다음 단계입니다. 메뉴 항목과 `ShellIntent::NewProjectGroup(project)`까지 만들었고, 앱은 아직 이 intent에 동작이 없습니다.
- dialog 외형: 직전 경로 열기 dialog와 같이 `egui::Modal` 기본 틀을 썼습니다. 내용 폭은 `sm:max-w-lg`(512)에서 `p-6`을 뺀 464px이고 좁은 창에서는 `calc(100%-2rem)`처럼 줄어듭니다.
- cmdk 선택 표시: `data-selected`는 `list.activeBackground`(egui `selection.bg_fill`)와 accent 1px 링입니다. 고른 아이콘은 `appSidebar.itemActive`와 accent 링입니다. 둘이 겹치면 원본 CSS 특이성처럼 선택 배경이 이깁니다.

## 실제 앱 경로

- `main.rs` → `NativeApplication` → `eframe::App::ui` → `NativeShell::project_rail` → `project_button` context menu → `ShellIntent` → `application.rs` intent 처리 → `HostBridge` → `host.rs` dispatch → `taide_runtime::project_actions`입니다.
- 표시 dialog는 `application.rs`가 메뉴 intent로 열고 같은 프레임 끝에서 그립니다.

## 검증

- UI 단위: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib … -- project_rail:: project_display_dialog::`는 새 검사 4건을 포함해 통과합니다. dialog 3건, 보조 함수 1건입니다.
    - 열 때 저장값 재해석(라벨 우선, clamp는 trim 없음, 모르는 아이콘·`constructor`는 folder, 잘못된 색은 없음)을 확인했습니다. 세 축 patch(`["", "가나", "lane3"]`), 대기 중 Save 비활성·중복 없음, 실패 유지, 대기 아닐 때 finish 무시, 성공 닫힘, Escape 닫힘도 확인했습니다.
    - 라벨 mode는 자동 focus, 제어문자 제거, 4코드포인트 제한(추가 입력 차단), 빈 라벨 Save 비활성, 빈 라벨 미리보기의 folder를 확인했습니다. 기본 mode 미리보기는 20px folder와 iconDefault 색입니다.
    - 아이콘 mode는 56개 격자, 고른 아이콘의 toggled, 첫 항목 선택, `git` → `[git-branch, git-fork]`, 대소문자 무시(`XML`)를 확인했습니다. ↓ 끝에서 멈춤, Home/End, Enter 선택(dialog 유지), 결과 없음 문구, mode 전환 뒤 검색 초기화, 클릭 선택도 확인했습니다. 12번 색 토글 시 미리보기 아이콘·lane 색과 patch `["globe", "", "lane12"]`, Cancel 닫힘도 확인했습니다.
- UI 실제 egui(`--test workbench`): 새 `프로젝트_rail_메뉴는_…` 검사입니다.
    - 원본 순서의 세로 배치를 확인했습니다. 각 항목의 intent는 닫기, 4방향 × focused slot, 제거 멤버, OpenPath/CopyText root, EditProjectDisplay, 세 축 `""` 초기화입니다.
    - 하위 메뉴는 그룹 순서·"새 그룹" 위치, 현재 그룹 비활성, 다른 그룹 추가(끝에 붙임), NewProjectGroup을 확인했습니다.
    - 그룹 없는 기본 표시 프로젝트는 제거·초기화가 비활성입니다. 낡은 focus는 첫 leaf로 갑니다. slot이 없으면 슬롯 열기 4개가 비활성이고 intent가 없습니다.
- UI 전체: `cargo test … --features native-host,inspection --test workbench --lib`는 lib 147/147, workbench 20/20 통과입니다. workbench의 `unused mut` 경고(1640행)는 기존 탭 메뉴 검사의 것입니다.
- App: `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib -- project_rail_tests:: zen::`는 6통과(51.54초)입니다. 그 전 `-- project_rail_tests:: problems_icons:: open_path_dialog::`는 4통과입니다.
    - 새 `실제_app_rail_메뉴는_…` 검사는 실제 `NativeApplication`(합성 temp 데이터, ide/hook 끔)에서 rail 버튼 우클릭과 메뉴 클릭으로 다음을 runtime 상태로 확인합니다.
        - beta Open to the Right → slot tree가 `[alpha, beta]` split입니다.
        - beta 그룹 제거 → 그룹 멤버가 비었습니다.
        - alpha 표시 초기화 → 표시 기본값입니다.
        - 표시 편집 dialog → 라벨 mode, "QA" 입력, Save → `label: "QA"`이고 dialog가 닫힙니다.
        - beta 닫기 → `state.projects`와 session에서 빠집니다.
    - 닫기 전 beta에 untitled 문서를 만들고 mirror debounce가 60초 뒤에야 오도록 대기 상태를 넣었습니다. 닫힌 뒤 untitled mirror에 그 본문이 남는 것을 확인했습니다. 일반 debounce 경로로는 닫기 전에 쓰이지 않으므로 이 결과는 닫기 전 mirror flush에서 나옵니다.
    - App 검사는 rail 버튼을 rail 폭 안에서만 찾습니다. slot header가 같은 이름을 갖기 때문입니다. 메뉴를 열기 전에는 shell snapshot이 session을 따라잡을 때까지 그립니다.
- `cargo check --all-targets`: native-ui(`--features native-host,inspection`)와 native-app 모두 exit 0입니다. App check는 Zen 가드와 닫기 mirror 검사를 추가하기 전에 실행했고, 이후 변경은 위 lib test 빌드로 컴파일됐습니다. 경고는 기존 vendor·dead code뿐입니다.
- 동결 remote-web `cargo check --target wasm32-unknown-unknown --lib`는 마지막 UI 변경 뒤 다시 실행해 exit 0입니다. 기존 dead_code 경고만 있고 소스·Cargo.lock 변경은 없습니다.
- `cargo fmt -- --check`: native-ui는 diff가 없습니다. native-app은 소유 파일 diff가 없고, 기존 `application.rs` 904/10122행, `lib.rs`, `experiments/lsp-coordinator-spike` diff만 남아 손대지 않았습니다.
- 처음부터 실패하는 상태를 재현한 뒤 고치지는 않았습니다. 새 검사는 이전 구현에 없던 메뉴·intent·dialog·host 명령을 단언합니다. 구현을 끄는 재현은 작업 규칙상 하지 않았습니다.

## 남은 범위

- "새 그룹" 항목은 `ShellIntent::NewProjectGroup`까지입니다. 그룹 생성 dialog와 `project_group_create(members: [project])` 연결은 shell-46 단계입니다.
- 공통 dialog 기반은 경로 열기 dialog와 같은 남은 범위입니다.
    - 원본 DialogContent의 닫기 X 버튼, 검색 입력의 돋보기 아이콘(현재 glyph 카탈로그에 search가 없음), Radix 열림 focus·focus trap·열림/닫힘 애니메이션입니다.
    - swatch의 ring-offset 색, cmdk Ctrl+N/P/J/K vim 키입니다.
- dialog가 열린 동안 셸 단축키 차단은 경로 열기 dialog와 같이 공통 modal 처리 범위입니다. Zen Escape만 막았습니다.
- 닫기 직전 mirror가 이미 전송 중인데 그 뒤 수정이 생긴 경우, 그 최신 revision은 기존 `Persistence`가 in-flight 중 새 job을 만들지 않아 close 전에 쓰지 못합니다. 원본 handshake와 다른 좁은 경계입니다. 아래 "테스트 부채" 1번에 기록했습니다.
- 화면 픽셀, 실제 Finder 열기·클립보드, 실기 확인은 배치 31입니다.

## 리뷰 수정(차단 1건, 과대 판정 shell-37)

### 차단: 닫힌 프로젝트의 문서가 남아 종료가 실패함

- 실제 결함이었습니다. 리뷰 시나리오대로 재현했습니다.
    - 닫기 성공 뒤에도 그 프로젝트의 `files`/`untitled` 항목, editor store 문서, view가 남았습니다. 앱에는 프로젝트 단위 해제 경로가 없었습니다.
    - 남은 dirty 문서가 `drafts()`에 들어가면 `shutdown`의 draft 루프가 `root_guard::project_root`/`file_mirror_dirty`에서 `NotFound("project not open")`로 멈춥니다. 그러면 뒤의 다른 프로젝트 draft mirror, layout flush, view state 적용도 건너뜁니다.
- 원본 근거:
    - `src/app/providers/hot-exit-flush-provider.tsx` 46-48, 60: `{ project }` scope는 그 프로젝트 mirror만 쓰고 확인합니다. 프로젝트가 `AppState`에서 빠지는 순간 그 mirror 쓰기는 더 받지 않습니다.
    - `src/app/providers/ipc-sync-provider.tsx` 313-331: `projectClosed`에서 open-with override, 프로젝트 범위 query(`PROJECT_SCOPED_KEYS`, `src/shared/constants/query-key.ts` 195-208), root 아래 파일 내용 query(220-223), LSP 세션을 정리합니다.
    - 닫힌 프로젝트의 editor pane은 layout에서 사라져 unmount되므로 그 model도 버려집니다. 앱 종료의 `flushAllMirrors`는 남은 pane만 씁니다.
    - runtime `project_close`(`crates/taide-runtime/src/project_actions.rs` 462-500)는 session과 `state.projects`에서 프로젝트를 지웁니다. 그 안의 `detach_all`은 native `NativeProjects`(`native/taide-native-app/src/projects.rs` 201-202)에서 그 프로젝트 layout을 지웁니다. 그래서 응답 시점의 `state.layouts`에는 닫힌 프로젝트 탭이 없습니다.
- 수정:
    - `host.rs` `CloseProject`(1590-1609)는 성공하면 `HostReply::ProjectClosed(project)`를 보냅니다. 실패는 이전처럼 `Failed` toast입니다.
        - 응답은 host 큐 순서를 따릅니다. 그래서 앞서 보낸 Save·MirrorDraft 응답이 모두 처리된 뒤에 도착합니다.
    - `application.rs` `release_project_documents`(3492)는 응답 처리(1873)에서 그 프로젝트 소속 `files`/`untitled` 문서를 해제합니다.
        - 남은 layout에 없는 탭의 view를 `detach_view`로 뗍니다.
        - 깨끗한 문서는 `release_document`, dirty 문서는 현재 revision으로 `discard_document`합니다. 본문은 `close_project`가 close보다 먼저 보낸 mirror에 있습니다.
        - 그 뒤 `files`/`untitled`, `persistence.settled`, `last_edited_views`, LSP 진단, 관찰 대기 상태를 정리합니다. 탭 닫기(`release_tab`)와 explorer 삭제 commit이 쓰는 정리 항목과 같습니다.
        - 열린 다른 프로젝트 탭이 같은 문서를 아직 보이면 문서를 두고, 소유 프로젝트를 `resolve_owning_project`로 다시 정합니다(없으면 외부 문서). 열기 때 소유자를 정하는 방식(`HostReply::Opened`의 `opened.project_id`, `UntitledSaved`)과 같습니다.
    - `shutdown`(8389)은 이미 닫힌 프로젝트의 draft를 건너뛰고 `log::warn`을 남깁니다. 리뷰의 보조 제안(`drafts()` 필터)을 `shutdown` 안에 두었습니다.
        - `drafts()`는 `close()`/`on_exit()`에서 host 큐가 비기 전에 계산됩니다. 그래서 그 사이 큐에 있던 `CloseProject`나 remote client의 닫기가 실행되면 `drafts()` 필터로는 막지 못합니다. 루프 시점의 `state.projects`가 실패 조건과 같은 기준입니다.
- 검증(재현 먼저):
    - 수정 전 `cargo test … --lib -- 실제_app_rail_메뉴는 닫힌_프로젝트의_draft는`: 2건 모두 실패했습니다.
        - rail 검사는 `closed project releases its documents`(문서가 남음)에서 실패했습니다.
        - 새 exit 검사는 `NotFound("project not open: …")`로 실패했습니다.
    - 수정 뒤 같은 명령은 2건 통과입니다.
    - `-- exit_tests:: project_rail_tests::`는 5건 통과입니다(기존 exit 2건, rail 표시 1건 포함, 51.95초).
    - rail 검사 추가 단언(`application-project-rail-tests.rs` 530-547): 닫기 뒤 beta untitled 항목이 사라지고, store 문서가 없으며, `drafts()`에 beta가 없습니다.
    - exit 검사 `닫힌_프로젝트의_draft는_건너뛰고_열린_프로젝트의_draft를_mirror한_뒤_종료한다`(`application-exit-tests.rs` 87)를 추가했습니다. 닫힌 프로젝트 draft가 앞에 있어도 `shutdown`·`finish_direct_exit`가 Ok입니다. 열린 프로젝트 draft는 mirror에 남고 추적 task는 0입니다.
    - rail 검사에 실제 `close()` → `is_exit_ready` 대기도 넣어 봤습니다. 혼자 돌린 첫 실행은 통과했습니다. 그 뒤 두 번은 10초 안에 끝나지 않았습니다(`closing`은 유지, 오류 없음, `shutting_down=true`, 추적 task 2개). 원인은 `drain_services`가 끝나기를 기다리는 abort 불가 blocking task(실제 프로세스 목록을 읽는 agent probe 등)의 환경 의존 시간으로 봅니다. 닫기 수정과 무관하고 흔들리는 검사가 되므로 뺐습니다. 종료 루프 자체는 위 exit 검사가 직접 덮습니다.
    - `cargo check --all-targets`(native-app)는 exit 0입니다. 경고는 기존 vendor·dead code·`unused mut`뿐입니다.
    - `cargo fmt -- --check`(native-app)는 이 수정 파일에 diff가 없습니다. 기존 `application.rs` 904/10212행, `lib.rs`, `experiments/lsp-coordinator-spike` diff만 남았고 손대지 않았습니다.
    - native-ui와 동결 remote-web 소스는 이번 수정에서 바뀌지 않았으므로 wasm check는 다시 돌리지 않았습니다. 직전 결과(exit 0)를 그대로 씁니다.
- 바뀐 동작을 단언하던 기존 테스트: `CloseProject` 응답 개수나 `shutdown`의 NotFound 실패를 기대하는 검사는 `rg`로 찾지 못했습니다. `shutdown`을 직접 부르는 기존 exit 검사 2건은 위 실행에서 통과했습니다.

### 과대 판정: shell-37

- 차단 결함은 고쳤지만, 원본 `projectClosed` 처리와 아직 맞추지 않았거나 확인하지 않은 부분이 남아 대응표 상태를 `partial`로 낮췄습니다.
    - 닫힌 프로젝트의 explorer 상태(`explorers`)와 tree 캐시(`trees`)는 그대로 남습니다. 원본은 `QUERY_KEY.TREE.ROWS`를 지웁니다. 다시 열 때 이전 tree를 쓰는지는 확인하지 않았습니다.
    - 닫힌 프로젝트의 dirty peek 문서(`peek_models`)는 해제하지 않습니다. preview view를 `editor_locations`가 소유하므로 이번 수정에서 떼지 않았습니다. 아래 테스트 부채 3번입니다.
    - 아래 테스트 부채 1번(전송 중 mirror 뒤 수정)입니다.

### 비차단 항목 처리

- "새 그룹" 항목 비활성 제안: 원본은 항목이 늘 활성입니다(`sortable-project-icon.tsx` 187, `disabled` 없음). shell-46 연결 단계에서 동작을 붙이는 편이 원본과 맞으므로 이번에는 바꾸지 않았습니다.
- 테마 키(`appSidebar.itemHover`, `app.accent`, `graph.lane1..12`)와 remote-web `shell_colors`: rail 표시 작업(shell-33/35) 소관이라 이번 범위에서 다루지 않았습니다.

## 테스트 부채

1. 닫기 직전 전송 중인 mirror 뒤의 수정
    - 재현 조건: 문서 mirror가 host 큐에서 쓰이는 중(`mirror_inflight`)에 사용자가 더 고치고 곧바로 rail에서 닫기를 누릅니다.
    - 생략 이유: `Persistence::mirror_job`이 in-flight 동안 새 job을 만들지 않는 기존 규칙과 얽혀 있습니다. 재현에 host 큐 타이밍 제어가 필요합니다.
    - 남은 위험: 마지막 수정분이 mirror에 없을 수 있습니다. 닫기 응답 뒤 문서는 해제되므로 그 수정분은 메모리에서도 사라집니다.
    - 실행 시점: 닫기 전 flush를 원본 handshake(그 프로젝트 문서의 최신 본문을 항상 씀)에 맞출 때입니다.
2. `ProjectDisplaySet` 응답에 요청 식별자 없음
    - 재현 조건: dialog 저장이 대기 중일 때 닫고 다시 열어 저장합니다. 또는 메뉴 "표시 초기화" 응답이 늦게 오는 사이 dialog를 열어 저장합니다.
    - 생략 이유: 두 요청이 같은 응답 variant를 쓰고 `finish`는 `is_pending`만 봅니다. 재현하려면 응답 순서 제어가 필요합니다.
    - 남은 위험: 이전 요청의 늦은 성공 응답이 새 dialog를 일찍 닫을 수 있습니다. 새 저장이 실패하면 toast만 뜨고 dialog는 이미 닫혀 있습니다.
    - 실행 시점: 표시 저장 응답에 요청 식별자를 붙이거나 공통 modal 작업(shell-41 계열)을 할 때입니다.
3. 닫힌 프로젝트의 dirty peek 문서
    - 재현 조건: peek(정의 미리보기)에서 다른 파일을 고친 채 그 파일의 프로젝트를 닫습니다.
    - 생략 이유: peek preview view는 `editor_locations`가 소유하고, dirty peek 문서는 `peek_models::sweep`가 계속 유지합니다.
    - 남은 위험: 그 문서의 debounce mirror가 닫힌 프로젝트로 가서 `log::warn`만 남깁니다. 종료는 막지 않습니다(`drafts()`에 peek 문서가 없음).
    - 실행 시점: peek 모델 수명 작업이나 shell-37 완료 판정 전입니다.
4. 다른 프로젝트 탭이 아직 보이는 문서의 소유자 재해석
    - 재현 조건: 프로젝트 B 루트 아래 파일이 열린 프로젝트 A의 탭에도 열려 있을 때 B를 닫습니다.
    - 생략 이유: 다른 프로젝트 layout에서 B 소유 파일 탭을 만드는 실제 경로를 이번에 확인하지 못했습니다.
    - 남은 위험: 재해석 결과가 외부 문서(소유자 없음)이면, 그 dirty 문서가 남은 동안 앱 종료는 기존 규칙대로 "save the external native document before closing"으로 멈춥니다.
    - 실행 시점: 프로젝트 간 탭 이동·중첩 프로젝트 열기를 구현하거나 검증할 때입니다.
