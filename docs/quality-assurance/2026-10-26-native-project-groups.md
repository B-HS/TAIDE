# 프로젝트 rail 그룹 헤더·그룹 dialog·그룹 메뉴

## 대상

- shell-45: 그룹 헤더 색 tint입니다.
- shell-46: 그룹 생성/수정 dialog(이름 40코드포인트 상한, 12색)입니다.
- shell-47: 그룹 context menu(그룹 열기 결과 toast, 수정, 삭제 확인)입니다.
- shell-48: 프로젝트 메뉴의 그룹에 추가/제거(`project_group_set_members`)입니다.
- shell-36: 프로젝트 메뉴 "새 그룹" 항목 연결입니다. 직전 단계(2026-10-26-native-project-rail-menu.md)에서 이 항목만 남았습니다.
- 원본: `src/widgets/app-sidebar/sortable-project-group-header.tsx`, `src/features/project/project-group-dialog.tsx`, `src/widgets/app-sidebar/app-sidebar.tsx`, `src/shared/lib/project-group.ts`, `src/shared/constants/project-group.ts`, `src/entities/project-group/project-group.query.ts`, `src/entities/project-group/project-group.ipc.ts`

## 원본 동작

- 그룹 헤더(`sortable-project-group-header.tsx` 69-108):
    - `w-11`(44px) 버튼이고 `px-1 py-0.5`, `gap-0.5`, `rounded-sm`(4px)입니다. hover 배경은 `app-sidebar-item-hover`입니다.
    - 펼침이면 ChevronDown, 접힘이면 ChevronRight(12px)입니다. 이름은 9px이고 넘치면 말줄임입니다.
    - 기본 글자색은 `app-sidebar-icon-default`입니다. `resolveProjectGroupColorVar(group.color)`(`project-group.ts` 22-25)가 유효한 palette token일 때만 chevron과 이름에 그 색을 씁니다. 잘못된 token은 기본색입니다.
    - 클릭은 `project_group_set_collapsed`로 접힘을 뒤집습니다(98-100).
    - 높이는 base `line-height: 1.5`(Tailwind preflight)의 9px 줄 13.5px에 `py-0.5` 위아래 2px를 더한 17.5px입니다.
- 그룹 메뉴(111-116): "그룹 열기", 구분선, "이름·색 변경…", "그룹 삭제" 순서입니다. 비활성 조건이 없습니다.
    - 그룹 열기(73-77): `project_group_open` 결과로 `toast.success(t('projectGroup.openResult', { opened, skipped }))`, 실패는 `describeIpcError` toast입니다.
    - 수정(79-87, 119-127): edit mode dialog입니다. 이름이 바뀌었으면 `project_group_rename`, 그다음 해석된 색 token이 바뀌었으면 `project_group_set_color`를 차례로 기다립니다. 둘 다 성공하면 닫고, 실패하면 toast와 함께 열어 둡니다. 바뀐 것이 없으면 쓰기 없이 닫습니다. 대기 표시는 두 쓰기 중 하나라도 대기 중인 동안입니다.
    - 삭제(129-144): AlertDialog입니다. 제목은 `projectGroup.deleteConfirmTitle`(이름 인자), 설명은 `projectGroup.deleteConfirmDescription`입니다. 버튼은 취소와 destructive "그룹 삭제"입니다. 확인하면 `project_group_delete`를 보내고 실패는 toast입니다. Radix AlertDialog는 열릴 때 취소에 focus하고, Escape로 닫히고, 바깥 클릭으로는 닫히지 않습니다.
- 그룹 dialog(`project-group-dialog.tsx`):
    - 제목은 create `projectGroup.createTitle`, edit `projectGroup.editTitle`입니다. 확인 버튼은 create `projectGroup.create`, edit `common.save`입니다(15-17).
    - 열릴 때마다 이름은 `clampProjectGroupName(initialName)`, 색은 `resolveProjectColorToken(initialColor)`로 다시 채웁니다(47-53).
    - 이름 줄은 라벨 `projectGroup.nameField`, 12px 글자, `gap-3`입니다. 입력은 autoFocus이고 남은 폭을 채웁니다. 입력마다 `clampProjectGroupName`(제어문자 제거, 40코드포인트, trim 없음)을 적용합니다. Enter는 확인입니다(70-80).
    - 색은 `projectGroup.color` 라벨과 12개 24px 원입니다. 누르면 토글되고 눌린 색은 accent ring입니다. 이름은 `색 N`입니다(82-100). 직전 단계 표시 dialog의 색 선택과 같은 모양입니다.
    - 확인은 `normalizeProjectGroupName`(제어문자 제거 → trim → 40코드포인트, `project-group.ts` 18-19)이 비지 않고 대기 중이 아닐 때만 됩니다. `{ name: 정규화 이름, color }`를 보냅니다(55-60, 107-109).
    - 40은 `PROJECT_GROUP_NAME_MAX_CODEPOINTS`(`constants/project-group.ts`)이고 백엔드 `GROUP_NAME_MAX_CODEPOINTS`(`crates/taide-project/src/service.rs` 34)와 같습니다. 백엔드 `sanitize_group_name`(727-734)도 같은 순서입니다.
- 그룹 생성 진입(`app-sidebar.tsx` 66, 124-128, 144, 191-199): 프로젝트 메뉴 "새 그룹…"이 `newGroupProjectId`를 정해 create dialog를 엽니다. 확인하면 `project_group_create({ name, color, members: [projectId] })`를 보내고, 성공하면 닫고 실패하면 toast입니다.
- 그룹에 추가/제거(`app-sidebar.tsx` 111-122): `withProjectGroupMember`/`withoutProjectGroupMember` 목록을 `project_group_set_members`로 보내고 실패는 toast입니다. 메뉴 쪽은 직전 단계가 구현했습니다.
- runtime(`crates/taide-runtime/src/project_actions.rs`): `project_group_create`(292), `project_group_rename`(313), `project_group_open`(503), `project_group_set_color`(630), `project_group_set_members`(670), `project_group_delete`(690)입니다. Tauri command(`src-tauri/src/domain/project/commands.rs` 457-565)는 이것을 그대로 부릅니다. `project_group_open`만 `run_nonabortable_result`로 감쌉니다. create/set_members의 멤버는 저장된 프로젝트 기록이 있어야 합니다(`service.rs` 747-760).

## 구현

- UI(`native/taide-native-ui`):
    - `shell.rs`
        - `group_header`(546): 44×17.5px 헤더를 rail 중앙에 둡니다. 40px scroll 영역보다 넓으므로 hit test는 `ui.scope` 안의 `set_clip_rect`로, 그리기는 복제한 painter의 `set_clip_rect`로 clip을 rail 폭(56px)으로 덮어씁니다. `project_button`과 같은 방식입니다. 교집합을 만드는 `Painter::with_clip_rect`는 넓힐 수 없으므로 쓰지 않습니다(아래 "리뷰 수정" 참조).
        - 헤더는 hover 배경(`rail.item_hover`, 4px 반경), 12px chevron, 9px 이름(남은 22px에서 말줄임)을 그립니다. chevron은 lucide `chevron-down`/`chevron-right` 좌표의 1px polyline입니다(`CHEVRON_*_POINTS`, 75-76). settings-view의 chevron과 같은 방식입니다.
        - 색은 `resolve_project_color_lane(group.color)`가 있으면 `rail.lanes[lane]`, 없으면 `muted`(`appSidebar.iconDefault`)입니다.
        - 접근성 이름은 그룹 이름입니다. 클릭은 기존 `SetGroupCollapsed`, 우클릭은 `group_menu`(1610)입니다. 이전에는 이 자리가 egui 기본 Button이었습니다.
        - `group_menu`는 원본 순서로 `ShellIntent::OpenProjectGroup`/`EditProjectGroup`/`DeleteProjectGroup`을 냅니다.
    - `commands.rs`: 위 세 `ShellIntent`를 더했습니다. `ShellMutation`은 바꾸지 않아 동결 remote-web 소스는 그대로입니다.
    - `project-rail.rs`: `PROJECT_GROUP_NAME_MAX_CODEPOINTS`(40), `clamp_project_group_name`(120), `normalize_project_group_name`(130)을 원본 보조 함수 그대로 옮겼습니다.
    - `project-group-dialog.rs`(새 파일)
        - `ProjectGroupDialog`: `create(project)`, `edit(group)`, `finish`, `close`, `show`가 있습니다. 열 때마다 필드를 다시 채웁니다.
        - 이름 입력은 자동 focus, `char_limit(40)`, 변경마다 clamp입니다. Enter는 이름에 focus가 있고 IME 조합 중이 아닐 때만 확인합니다.
        - 확인/Enter는 정규화 이름이 비지 않고 대기 중이 아닐 때만 `Submission { target, name, color }`를 돌려줍니다. 취소, Escape, 바깥 클릭은 닫습니다.
        - `GroupEdit::new`는 `handleSubmitEdit`의 비교입니다. 이름은 그대로, 색은 저장값을 `resolve_project_color_lane`로 해석한 token과 비교합니다.
        - `ProjectGroupDeleteDialog`: 열리면 취소에 focus합니다. 취소, 삭제, Escape로 닫히고 바깥 클릭으로는 닫히지 않습니다. 삭제를 고른 경우에만 그룹 id를 돌려줍니다.
    - `project-display-dialog.rs`: 색 선택(`color_picker`, 441)과 dialog 폭(`dialog_width`, 435)을 자유 함수로 꺼내 두 dialog가 함께 씁니다. 두 번째 사용처가 생겨서입니다. 표시 dialog 동작은 바뀌지 않았고 기존 검사가 그대로 통과합니다. `SECTION_GAP`/`ROW_GAP`/`SMALL_TEXT_SIZE`는 `pub(crate)`로 열었습니다.
- App(`native/taide-native-app`):
    - `host.rs`에 `HostCommand::CreateProjectGroup/EditProjectGroup/DeleteProjectGroup/OpenProjectGroup`(196-207)과 `HostReply::ProjectGroupSaved/ProjectGroupOpened`(458-459)를 더했습니다. dispatch는 1679-1745입니다.
        - 생성은 `project_group_create(members: Some([project]))`입니다.
        - 수정은 이름이 있으면 rename, 그다음 색이 있으면 set_color입니다. 앞이 실패하면 멈추고 하나의 결과로 답합니다.
        - 삭제 실패는 `HostReply::Failed`입니다.
        - 열기는 remote `project_group_open`(`remote-projects.rs` 188-208)과 같은 순서입니다. 그룹 존재 확인, view state flush, `HostIntegrations::projects`(terminal hub 포함 `NativeProjects`) lifecycle로 `run_nonabortable_result` 안에서 runtime을 부릅니다.
    - `application.rs`
        - intent 처리(8090-8107): `NewProjectGroup`은 snapshot에 있는 프로젝트면 create dialog를 엽니다. `EditProjectGroup`/`DeleteProjectGroup`은 snapshot의 그룹으로 dialog를 엽니다. `OpenProjectGroup`은 host 명령입니다.
        - `project_group_dialogs`(3493): 매 프레임 그립니다. 편집/삭제 대상 그룹이 사라지면 닫습니다(원본 헤더 unmount). 생성은 대상 프로젝트가 닫혀도 유지합니다(원본과 같음).
        - 수정 제출은 `GroupEdit`가 비면 쓰기 없이 닫고, 아니면 `EditProjectGroup`을 보냅니다.
        - 응답(1881-1907): `ProjectGroupSaved`는 `finish`와 실패 toast입니다. `ProjectGroupOpened` 성공은 `projectGroup.openResult` success toast(opened/skipped 개수), 실패는 ipc 오류 toast입니다.
        - 그룹 dialog가 열린 프레임에는 Zen 해제 Escape를 막습니다(8332, 8342). 표시 dialog와 같은 처리입니다.

## 작은 결정과 근거

- 이름 상한은 코드포인트 40만 지킵니다. 원본 `<input maxLength={40}>`은 UTF-16 단위로 세므로 BMP 밖 문자(이모지 등)는 20자에서 막힙니다. 이것은 상수 주석("codepoints")과 백엔드 상한과 다른 브라우저 부작용이므로 재현하지 않았습니다. 직전 표시 dialog의 `char_limit(4)`와 같은 방식입니다.
- Enter 확인은 IME 조합 중에는 하지 않습니다. 한국어 입력 확정 Enter가 곧바로 그룹을 만드는 것을 막기 위함이며, 표시 dialog 검색의 `Composition` 처리와 같습니다.
- 그룹 열기 응답은 진행 중 tracked 작업으로 감쌌습니다(`run_nonabortable_result`). Tauri·remote와 같은 수명 규칙입니다.
- 열기 전 view state flush는 remote 경로를 따랐습니다. Tauri command에는 없지만 native host의 다른 열기 명령(`OpenProject`, `OpenProjectInSlot`)이 모두 먼저 flush합니다.
- 헤더 chevron은 lucide 좌표의 polyline으로 그렸습니다. glyph 카탈로그에 chevron-down이 없고, `ShellSurfaces::rail_icon`에 새 variant를 더하면 앱의 아이콘 칠하기를 바꿔야 하기 때문입니다.
- 삭제 확인 버튼은 탐색기 삭제 확인(`delete_dialog.rs`)처럼 `error_fg_color` 글자입니다. 배치는 원본 `AlertDialogFooter`(오른쪽 정렬)를 따랐습니다.

## 실제 앱 경로

- `main.rs` → `NativeApplication` → `eframe::App::ui` → `NativeShell::project_rail` → `group_header`(우클릭 `group_menu`) 또는 `project_button` 메뉴 "새 그룹…" → `ShellIntent` → `application.rs` intent 처리 → `project_group_dialogs` → `HostBridge` → `host.rs` dispatch → `taide_runtime::project_actions`입니다.

## 검증

- UI 단위: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib … -- project_group_dialog:: project_display_dialog:: project_rail::`는 15건 통과입니다. 새 그룹 dialog 검사는 5건입니다.
    - 이름 보조 함수: 입력 중 clamp는 trim 없이 제어문자만 뺍니다. 정규화는 trim 뒤 40코드포인트이고, 공백·제어문자뿐이면 빈 값입니다.
    - `GroupEdit`: 같으면 비어 있습니다. 이름만, 색 지우기만 바뀌는 경우를 확인했습니다. 잘못된 저장 색(`lane99`)은 없음과 같고, 새 색은 `Some(Some("lane5"))`입니다.
    - 새 그룹 dialog: 제목·라벨, 빈 이름 비활성과 Enter 무시, 공백 이름 비활성을 확인했습니다. 자동 focus 입력 뒤 Enter로 trim된 이름 `"팀 a"`가 제출됩니다. 대기 중에는 비활성이고 중복 제출이 없습니다. 실패하면 열려 있고, 4번 색 토글 뒤 만들기는 `lane4`입니다. 성공하면 닫히고, 다시 열면 초기화됩니다.
    - 수정 dialog: 43자 저장 이름이 40자로 clamp되고 추가 입력이 막힙니다. 저장 색 3번이 눌린 채로 열리고, 다시 누르면 `color: None`입니다. 잘못된 저장 색이면 모든 swatch가 꺼져 있습니다. 취소와 Escape로 닫힙니다.
    - 삭제 확인: 이름이 들어간 제목·설명, 취소 focus를 확인했습니다. 바깥 클릭으로 닫히지 않고, 취소와 Escape는 그룹 없이 닫히며, 삭제는 그룹 id를 돌려주고 닫힙니다.
- UI 실제 egui(`--test workbench`): 새 `프로젝트_그룹_헤더는_…` 검사(2684)입니다.
    - 두 헤더가 44×17.5px이고 rail 중앙에 있습니다. 접힌 그룹의 멤버는 숨고 펼친 그룹의 멤버는 헤더 사이에 있습니다.
    - `lane3` 그룹은 이름 글자와 chevron이 lane 색이고, `lane99` 그룹은 기본색입니다. 이름은 9px, 폭 22px 이하로 말줄임되어 헤더 안에 있습니다.
    - chevron은 펼침이면 아래, 접힘이면 오른쪽입니다. hover하면 헤더 크기의 `item_hover` 배경이 생기고, 그 도형의 `clip_rect`가 헤더 rect를 포함합니다(리뷰 수정 뒤 보강).
    - 클릭은 각 그룹의 `SetGroupCollapsed`를 반대로 냅니다. 메뉴는 열기·수정·삭제 순서로 모두 활성이고, 각각 해당 intent를 내며 접힘을 바꾸지 않습니다.
- UI 전체: `cargo test … --features native-host,inspection --test workbench --lib`는 lib 152/152, workbench 21/21 통과입니다. 마지막 서식 정리 뒤 다시 실행한 결과입니다. workbench의 `unused mut` 경고(현재 1653행, HEAD 1640행)는 기존 탭 메뉴 검사의 것입니다.
- App: `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib -- project_rail_tests:: zen:: exit_tests::`는 10건 통과(76.82초)입니다.
    - 새 `실제_app_그룹은_…` 검사(626)는 실제 `NativeApplication`(합성 temp 데이터, ide/hook 끔, 세 프로젝트의 합성 기록 파일)에서 다음을 runtime 상태로 확인합니다.
        - alpha 메뉴 → "그룹에 추가" 하위 메뉴 → "새 그룹…"으로 dialog가 열립니다. "QA team" 입력, 색 3, 만들기를 하면 그룹 `{name: "QA team", color: "lane3", members: [alpha]}`가 생기고 dialog가 닫힙니다(shell-36, shell-46 생성).
        - beta 메뉴 → 하위 메뉴의 "QA team"을 고르면 멤버가 `[alpha, beta]`입니다(shell-48 추가). 제거는 직전 단계 rail 메뉴 검사가 확인합니다.
        - 헤더 우클릭 → "이름·색 변경…"으로 기존 이름이 채워집니다. " 2" 입력, 색 3 해제, 저장을 하면 `{name: "QA team 2", color: None}`이고 닫힙니다(shell-46 수정, shell-47).
        - 닫힌 gamma를 멤버로 넣은 뒤 헤더 메뉴 "그룹 열기"를 고르면 success toast "Opened 1, skipped 2"가 뜨고, gamma가 열려 watcher가 붙습니다(shell-47 열기).
        - 헤더 메뉴 "그룹 삭제" → 확인 dialog → 삭제를 하면 그룹이 없어지고 세 프로젝트는 열린 채입니다(shell-47 삭제).
    - 그룹 열기 단계만 60초 제한(`WATCHER_ATTACH_DEADLINE`)입니다. 진단용 임시 출력으로 재보니 이 환경에서 `NativeProjects::attach`의 파일 watcher 시작(`build_files`)이 8.6~18.5초 걸렸습니다. 앱 시작 때 alpha/beta watcher 복원도 같았습니다. 기존 경로의 환경 의존 시간이며 이번 변경과 무관합니다. 진단 출력은 지웠고 `projects.rs`에는 diff가 없습니다. 이 단계만 OS watch 등록 잠금(`exclusive_os_watch_registration`)을 잡습니다.
    - 기록 파일의 root는 canonical 경로로 씁니다. 그래야 `open_project`의 root 조회가 같은 프로젝트 id를 다시 씁니다.
- `cargo check --all-targets`: native-ui(`--features native-host,inspection`)와 native-app 모두 exit 0입니다. 경고는 기존 vendor·dead code·`unused mut`뿐입니다.
- 동결 remote-web `cargo check --target wasm32-unknown-unknown --lib`는 마지막 UI 변경 뒤 다시 실행해 exit 0입니다. 기존 dead_code 경고만 있고 소스·Cargo.lock 변경은 없습니다.
- `cargo fmt -- --check`: native-ui는 diff가 없습니다. native-app은 이번 변경 파일에 diff가 없습니다. 기존 `application.rs` 908/10327행, `lib.rs`, `experiments/lsp-coordinator-spike` diff만 남았고 손대지 않았습니다.
- 바뀐 표시를 단언하던 기존 테스트: 그룹 헤더를 egui Button으로 찾는 검사는 `rg`로 찾지 못했습니다. `SetGroupCollapsed`를 쓰는 기존 검사는 remote-web `tests/shell.rs`(mutation 직렬화)와 workbench dispatch 검사뿐이고, 헤더 UI와 무관합니다. 표시 dialog 색 선택 추출 뒤 기존 표시 dialog 검사 4건은 통과합니다.
- 처음부터 실패하는 상태를 따로 재현하지는 않았습니다. 새 검사는 이전 구현에 없던 헤더 모양·메뉴·dialog·host 명령을 단언합니다. 구현을 끄는 재현은 작업 규칙상 하지 않았습니다. App 검사는 하위 메뉴 trigger 이름(화살표 포함), 비canonical 기록 root, watcher 시작 시간 때문에 처음 세 번 실패했고, 각각 검사 쪽 조회·fixture·제한을 고쳐 통과했습니다.

## 리뷰 수정 (2026-10-27)

- 차단 항목: 그룹 헤더 hover 배경이 44px이 아니라 40px로 잘려 그려졌습니다.
    - 원인: `group_header`가 `ui.painter().with_clip_rect(clip)`로 그렸습니다. egui 0.36.2 `Painter::with_clip_rect`는 인자와 부모 painter clip의 교집합입니다(`native/taide-native-app/vendor/egui-input/src/painter.rs` 67-75). 바깥 ScrollArea 내용 ui의 clip은 rail 56px에서 좌우 8px를 뺀 x 8..48이라, rail 폭으로 넓히려던 clip이 무효가 됐습니다. hit test는 `ui.scope` 안 `set_clip_rect`로 44px였으므로 상호작용 영역과 표시가 어긋났습니다.
    - 재현 먼저: workbench `hover_fills`가 `shape.rect`만 비교하던 것을, 맞는 도형의 `ClippedShape.clip_rect`를 돌려주고 그 clip이 헤더 rect를 포함하는지 단언하도록 바꿨습니다(`tests/workbench.rs` 2767-2797). 수정 전 실행은 `clip [[8.0 36.0] - [48.0 720.0]]`이 헤더 `[[6.0 36.0] - [50.0 53.5]]`를 포함하지 못해 실패했습니다.
    - 수정: `shell.rs` 582-583에서 `let mut painter = ui.painter().clone(); painter.set_clip_rect(clip);`로 바꿨습니다. `project_button`(727-728)과 같은 덮어쓰기라 hover 배경·chevron·이름이 hit 영역과 같은 rail 폭 clip 안에서 44px 그대로, 4px 모서리까지 그려집니다. clip의 y 범위는 이전과 같이 바깥 ui clip을 따르므로 rail scroll 밖으로 넘치지 않습니다.
    - 같은 교집합 문제를 다른 rail 코드에서 찾았습니다. `project-rail.rs` 203의 `with_clip_rect(rect)`는 glyph 글자를 버튼 안으로 좁히려는 용도라 교집합이 맞으므로 그대로 둡니다.
- 비차단 반영: 이 문서의 줄 번호를 현재 작업 트리 기준으로 갱신했고, 기능표 shell-45 행의 낡은 "헤더 drag는 shell-39/49/52" 문구를 shell-49 참조로 바꾸고 hover clip 단언을 근거에 더했습니다.
- 비차단 미반영:
    - 비공개 함수·테스트 helper의 영어 `///` 설명(`application.rs` `project_group_dialogs`, `application-project-rail-tests.rs` `menu_entry`, `project-group-dialog-tests.rs` `settle`, `workbench.rs` `group_chevron`)은 남겼습니다. 기존 native 코드 전반이 같은 관행이고, 같은 diff의 drag 작업 파일에도 같은 형태가 있어 한쪽만 지우면 관행이 갈립니다. 일괄 정리 여부는 메인 판단으로 넘깁니다.
    - 헤더 `aria-expanded` 대응 정보와 chevron round cap은 배치 31 픽셀·실기 확인 범위로 둡니다.
- 검증(리뷰 수정 뒤):
    - `cargo test --features native-host,inspection --test workbench 프로젝트_그룹_헤더는`: 수정 전 1건 실패(위 clip 불일치), 수정 뒤 1건 통과입니다.
    - `cargo test --features native-host,inspection --test workbench`(전체): 24/24 통과입니다. 같은 작업 트리의 drag 검사 3건이 더해져 21에서 24가 됐습니다.
    - `cargo check --all-targets --features native-host,inspection`(native-ui): exit 0, 경고는 기존 workbench `unused mut`(1653행) 하나입니다.
    - 동결 remote-web `cargo check --target wasm32-unknown-unknown --lib`: exit 0, 기존 dead_code 경고만 있습니다.
    - `cargo fmt -- --check`(native-ui): diff 없습니다.
    - native-app `cargo check --all-targets`: exit 0입니다. 경고는 기존 vendor wry-preview deprecated·unused unsafe, `editor-code-actions.rs`·`open_path_dialog.rs`·`application-inlay-hints-tests.rs` dead code, `zen.rs` `unused mut`뿐입니다.
    - 바뀐 표시를 단언하던 기존 검사: `rg`로 그룹 헤더 hover·clip을 보는 검사는 workbench `hover_fills` 하나뿐이었고, 이번에 보강했습니다. App lib 검사는 이번 수정이 그리기 clip만 바꾸고 hit 영역·intent는 그대로라 다시 돌리지 않았습니다.

## 남은 범위

- 그룹 헤더 drag(그룹 순서, `project_group_reorder`)와 멤버 drag는 같은 단계의 drag 작업(2026-10-26-native-project-drag.md, shell-39/49/52)이 다룹니다.
- 공통 dialog 기반은 표시·경로 dialog와 같은 남은 범위입니다. 닫기 X 버튼, Radix 열림 focus·focus trap·애니메이션, swatch ring-offset 색이 해당합니다. dialog가 열린 동안 셸 단축키 차단도 공통 modal 범위이며 Zen Escape만 막았습니다.
- 헤더 키보드 focus 표시: 원본 버튼에 별도 focus 스타일이 없어 그리지 않았습니다.
- 화면 픽셀과 실기 확인은 배치 31입니다.

## 테스트 부채

1. `ProjectGroupSaved` 응답에 요청 식별자 없음
    - 재현 조건: 그룹 저장이 대기 중일 때 dialog를 닫고 다른 그룹 dialog를 열어 저장합니다.
    - 생략 이유: 표시 dialog의 테스트 부채 2번과 같은 구조입니다. `finish`는 `is_pending`만 봅니다. 재현하려면 host 응답 순서 제어가 필요합니다.
    - 남은 위험: 이전 요청의 늦은 응답이 새 dialog를 일찍 닫거나, 새 저장 실패가 toast만 남길 수 있습니다.
    - 실행 시점: 저장 응답에 요청 식별자를 붙이거나 공통 modal 작업을 할 때입니다.
2. App 경로의 생성/수정 실패 응답
    - 재현 조건: 기록 없는 프로젝트를 멤버로 만들거나 저장 중 그룹이 삭제됩니다.
    - 생략 이유: dialog 쪽 실패 유지는 UI 단위 검사가 확인합니다. App 응답 분기는 성공 경로와 같은 `finish`/`report` 호출이고, 실패를 만들려면 외부에서 그룹을 지우는 타이밍 제어가 필요합니다.
    - 남은 위험: 낮습니다. 실패 시 toast 문구는 기존 `error.projectGroup.*` 카탈로그를 씁니다.
    - 실행 시점: 그룹 관련 오류 문구나 응답 처리를 바꿀 때입니다.
