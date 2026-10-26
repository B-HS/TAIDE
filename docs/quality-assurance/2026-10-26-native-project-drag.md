# 프로젝트 rail drag와 slot drop

## 대상

- shell-39: rail 프로젝트 drag 재정렬(`project_reorder`)
- shell-49: 그룹 header drag 재정렬(`project_group_reorder`)
- shell-52: 프로젝트를 slot 가장자리·가운데에 drop해 분할·교체(`project_open_in_slot`)
- 원본: `src/widgets/app-shell/use-project-drag.ts`, `src/shared/lib/project-drag.ts`, `src/widgets/app-shell/shell-slot-tree-view.tsx`, `src/features/split/split-drop-zones.tsx`, `src/widgets/app-sidebar/app-sidebar.tsx`, `src/widgets/app-sidebar/sortable-project-icon.tsx`, `src/widgets/app-sidebar/sortable-project-group-header.tsx`, `src/features/project/project-drag-preview.tsx`, `src/widgets/app-shell/app-shell.tsx`

## 원본 동작

- drag는 창 하나의 `DndContext`입니다(`app-shell.tsx` 192-224, `use-project-drag.ts` 79-142).
    - `PointerSensor`는 주 버튼만 받고, `distance: 4`(22, 90)를 엄격히 넘어야 시작합니다(`@dnd-kit/core` `hasExceededDistance`). 시작 뒤에는 클릭을 막고, Escape·창 resize·`pointercancel`이면 취소합니다(`AbstractPointerSensor` `attach`·`handleKeydown`).
- 충돌 판정은 `projectCollisionDetection`(59-65)입니다.
    - rail droppable을 뺀 후보에 `pointerWithin`을 적용합니다. 포인터가 들어 있는 사각형 중 네 꼭짓점까지 평균 거리가 가장 작은 것을 고릅니다.
    - 하나도 없으면 포인터가 rail 위일 때만 slot zone을 뺀 rail 항목에 `closestCenter`를 적용합니다. 끌고 있는 사각형의 중심과 가장 가까운 항목입니다. rail 밖에서 빗나가면 대상이 없습니다(slot header·resizer·title bar 위 drop은 무동작).
- slot zone은 프로젝트를 끄는 동안에만 등록됩니다(`shell-slot-tree-view.tsx` 99-104).
    - 가장자리 1/4 띠 네 개와 가운데 1/2입니다(`split-drop-zones.tsx` 8-14). 가장자리 띠는 모서리에서 겹칩니다.
    - 위에 있는 zone의 미리보기는 해당 절반 또는 전체입니다. `tabBar.dropTarget` 25% 채움과 1px 테두리입니다(16-22, 31).
    - zone은 slot header 아래 본문만 덮습니다(82-104).
- drop 결과는 `handleDragEnd`(121-128)가 정합니다.
    - 그룹 header drag는 header 위에서만 `project_group_reorder(전체 그룹 순서)`입니다(112-119).
    - 프로젝트를 zone에 놓으면 `project_open_in_slot { path: null, projectId, targetSlot, edge }`를 보냅니다. 가운데는 `replace`이고(`project-drag.ts` 42-48), 실패는 toast입니다.
    - 자기 slot 가운데는 요청하지 않습니다(`resolveShellSlotDrop` 79-83). 다른 slot에 이미 있는 프로젝트는 서버가 거부합니다.
    - 그 밖에는 `project_reorder(전역 session.projects 순서)`이고 `arrayMove` 결과입니다(`reorderById` 92-97, 97-104). 다른 그룹 멤버 위에 놓아도 전역 순서만 바꾸고 소속은 바꾸지 않습니다(`use-project-drag.test.tsx` 251-260).
    - header↔아이콘처럼 단이 다르거나 제자리면 아무것도 쓰지 않습니다.
- 표시
    - 원본 아이콘은 drag 중 opacity 0.4입니다(`sortable-project-icon.tsx` 33, 128).
    - 포인터를 따라가는 미리보기는 40px `appSidebar.itemActive` 타일, glyph(lane 색 또는 `app.foreground`), opacity 0.9, 큰 그림자입니다(`project-drag-preview.tsx`). 끈 항목의 처음 사각형에 포인터 이동량을 더한 위치입니다(`DragOverlay` `PositionedOverlay`).
    - 그룹 header drag의 overlay는 비어 있습니다.
    - 같은 `SortableContext`(그룹 header들, 각 그룹 멤버, 미분류) 안의 대상 위에 있으면 `verticalListSortingStrategy`로 형제가 밀립니다. 원본 항목은 놓일 자리로 옮겨 그려집니다.

## 구현

- UI `project-drag.rs`(새 모듈)
    - 순수 함수: `slot_drop_zones`, `slot_drop_preview`, `shell_slot_edge`, `reorder_by_id`, `sorting_offset`, `resolve_drop`와 충돌 판정 `resolve_target`
    - 상태는 egui temp 메모리 하나입니다. 이번 frame에 배치된 rail 항목·rail·slot 본문을 기록하고, 누른 항목 하나의 `Pending/Active/Cancelled`를 둡니다.
    - frame 시작에 앞 frame 배치로 활성화·취소·대상을 갱신해 같은 frame의 그림이 현재 포인터를 반영합니다. frame 끝에서 놓기 요청이나 미리보기를 냅니다.
- UI `shell.rs`
    - rail 아이콘과 그룹 header를 `Sense::click_and_drag`로 바꿔 rail ScrollArea가 drag를 가로채지 않게 했습니다. 단(tier)을 넘기고, drag 활성·취소 뒤의 클릭을 막습니다.
    - 원본 아이콘은 0.4 opacity, 밀림 offset으로 그립니다. slot 본문(header 아래)을 등록합니다.
    - `project_drag_feedback`은 slot 미리보기를 Foreground 층에, 프로젝트 미리보기 타일을 Tooltip 층에 그리고, 놓기 요청을 `ShellIntent`로 바꿉니다.
    - `ShellColors.drop_target`(`tabBar.dropTarget`)을 추가했습니다. remote-web은 `presentation::shell_colors`를 쓰므로 동결 코드는 바뀌지 않습니다.
- 명령
    - `ShellIntent::ReorderProjects`·`ReorderProjectGroups`를 추가했습니다. `ShellMutation`은 동결 remote-web이 전수 match하므로 건드리지 않았습니다.
    - App은 `HostCommand::ReorderProjects`·`ReorderProjectGroups`로 runtime `project_reorder`·`project_group_reorder`를 호출합니다.
    - slot drop은 기존 `ShellIntent::OpenProjectInSlot` → `HostCommand::OpenProjectInSlot`(실패 toast) 경로를 재사용합니다.
- App의 web preview 동기화는 drag가 활성인 동안 egui 파일 drag처럼 꺼집니다(`project_drag::is_dragging`). 네이티브 web view가 zone 미리보기와 포인터를 가리지 않게 하기 위함입니다.

- 범위 밖 수정과 사유
    - `presentation.rs`: `drop_target` 색 연결
    - `project-display-dialog-tests.rs`·`project-group-dialog-tests.rs`: `ShellColors` 필드 추가에 맞춘 테스트 색
    - `application.rs`: 위 web preview 조건과 새 intent 두 개의 host 전달
- 작은 결정
    - 원본 `useReorderProjects`·`useReorderProjectGroups`는 `onError`가 없어 실패가 화면에 나오지 않습니다. native는 다른 그룹 명령처럼 `HostReply::Failed` toast로 보고합니다. 오류를 삼키는 것은 결함으로 보고 옮기지 않았습니다.
    - Escape 취소 뒤 버튼을 놓을 때까지는 클릭을 막습니다. pending 중 Escape는 press만 버려 원본처럼 클릭이 남습니다.

## 검증

- `cargo test --manifest-path native/taide-native-ui/Cargo.toml … --lib project_drag`는 6통과입니다(`project-drag-tests.rs`).
    - zone·미리보기 좌표와 가운데→replace 대응을 확인했습니다.
    - 원본 `use-project-drag.test.tsx` 충돌 판정 세 사례(slot 가운데, slot header 무대상, rail 틈→가장 가까운 아이콘)와 겹친 모서리, header drag의 zone 제외, 보이지 않는 항목 제외를 확인했습니다.
    - drop 결과(아래 가장자리 분할, 가운데 교체, 자기교체 무동작, 다른 slot의 프로젝트는 서버 판정), 재정렬(전역 순서, 그룹 넘는 멤버, 제자리, header 순서, header↔아이콘 무동작), `verticalListSortingStrategy` 이동량(아이콘·header 간격)과 pending/active/cancelled 보기를 확인했습니다.
- `cargo test --manifest-path native/taide-native-ui/Cargo.toml … --features native-host,inspection --test workbench`는 24/24통과입니다. 새 실제 egui 검사는 3개입니다.
    - rail 재정렬: 3px 이동은 pending이고 activate 클릭입니다. 5px은 drag이며 제자리 놓기는 무동작입니다. 끌어 놓으면 `ReorderProjects([p1,p2,p0])`입니다. 들고 있는 frame에서 유령(0.4)이 놓일 자리에 있고, 지나친 두 아이콘이 한 칸 올라가며, 0.9 미리보기 glyph와 `itemActive` 40px 타일이 포인터 위치에 있음을 확인했습니다. 놓은 뒤 `is_dragging`은 false입니다.
    - slot drop: 오른쪽 zone은 `OpenProjectInSlot(Right)`와 오른쪽 절반 `dropTarget` 미리보기(25% 채움·1px 테두리)입니다. 다른 slot 가운데는 `Replace`이고, 자기 slot 가운데·slot header 위 drop(미리보기 없음)·Escape 취소는 요청이 없음을 확인했습니다.
    - 그룹 header: header→header는 `ReorderProjectGroups([g1,g0])`이고 접기와 프로젝트 순서는 바뀌지 않습니다. 들고 있는 frame에서 두 header의 chevron 색이 서로 자리를 바꾸고 미리보기 타일이 없음을 확인했습니다. header→아이콘은 무동작입니다.
- `cargo test --manifest-path native/taide-native-ui/Cargo.toml … --features native-host,inspection --lib`는 158/158통과입니다(ShellColors 필드를 쓰는 dialog 검사 포함).
- `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib project_rail_tests`는 4/4통과(86초)입니다. 새 실제 App 검사는 임시 AppState·합성 egui 입력으로 다음 runtime session 결과를 확인했습니다.
    - gamma→beta drag는 `session.projects`를 alpha·gamma·beta로 바꾸고 그룹 소속은 그대로입니다.
    - second header→first는 `session.groups`를 second·first로 바꿉니다.
    - gamma→창 오른쪽 가장자리는 slot 트리를 [alpha | gamma] 분할로 바꿉니다.
- check와 형식
    - UI `--all-targets`(기본·`native-host,inspection`)와 App `--all-targets` check는 exit 0입니다. 경고는 기존 것뿐입니다(workbench 1653 `unused_mut`, vendor wry, zen.rs 등).
    - 동결 remote-web `--target wasm32-unknown-unknown --lib` check는 exit 0입니다(기존 dead_code 경고).
    - UI `cargo fmt -- --check`는 exit 0입니다. App은 exit 1이지만 diff는 기존 `application.rs` 908·10334, `lib.rs` 모듈 순서, `experiments/lsp-coordinator-spike`뿐이고 이번에 바꾼 구간에는 없습니다.
    - UI `cargo clippy --all-targets`는 기존 경고가 매우 많고 exit 101입니다. 출력이 잘려 새 경고 여부는 판정하지 못했습니다.

## 남은 범위

- dnd-kit의 정렬 전환(200ms ease)과 DragOverlay drop 애니메이션(250ms)은 재현하지 않았습니다. 밀림과 놓기는 즉시 반영됩니다.
- 미리보기 그림자는 Tailwind `shadow-lg` 수치 대신 테마의 `popup_shadow`를 씁니다.
- 원본 sensor의 `visibilitychange`·`pointercancel` 취소와, drag 중 브라우저 context menu 억제에 대응하는 egui 입력은 없습니다. 창 resize와 Escape 취소만 옮겼습니다.
- web preview 숨김은 실제 네이티브 web view가 필요해 자동 검사가 없습니다. `is_dragging` 신호만 workbench에서 확인했습니다. 화면 픽셀과 실기는 배치 31입니다.
- 아이콘·header가 drag를 받으므로, 그 위에서 시작한 drag는 rail ScrollArea를 스크롤하지 않습니다. 원본 rail(`nav`)은 스크롤 영역이 아닙니다.
