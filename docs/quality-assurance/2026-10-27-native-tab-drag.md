# 탭 drag 재정렬·pane 이동과 5-zone drop

## 대상

- shell-81: 탭 drag 재정렬과 pane 간 이동. pinned 경계 규칙을 포함합니다.
- shell-82: pane 5-zone drop(가장자리 분할, 가운데 이동)과 `DragOverlay` 탭 미리보기.
- 원본: `src/widgets/editor-area/editor-area.tsx`, `src/widgets/editor-area/pane-node-view.tsx`, `src/widgets/editor-area/pane-tab-bar.tsx`, `src/features/split/split-drop-zones.tsx`, `src/features/tab/sortable-tab.tsx`, `src/features/tab/tab-item.tsx`

## 원본 동작

- 시작과 취소
    - `PointerSensor`의 활성화 거리는 4px입니다(`editor-area.tsx` 53, 445).
    - Escape와 창 resize는 drag를 취소합니다(`@dnd-kit/core` `core.esm.js` 1444-1448).
    - 활성화된 뒤의 click은 막힙니다(같은 파일 1479, 1506).
    - `DndContext`는 `EditorArea`마다 하나입니다(`editor-area.tsx` 509-515). 그래서 탭은 자기 shell slot이나 보조창 안에서만 떨어집니다.
- 충돌 판정은 `pointerWithin`입니다(`editor-area.tsx` 511, `core.esm.js` 472). 포인터를 담은 droppable 중 네 꼭짓점까지 평균 거리가 가장 짧은 것을 고릅니다.
- droppable은 세 종류입니다.
    - 각 탭: `sortable-tab.tsx` 11, 75
    - 탭바 빈 칸 filler: `pane-tab-bar.tsx` 96-99, 275
    - drag 중 각 leaf 본문의 5개 zone: `pane-node-view.tsx` 69-73, 275-280. 가장자리 1/4 띠 네 개와 가운데 1/2입니다(`split-drop-zones.tsx` 8-14).
- drop(`editor-area.tsx` 471-504)
    - zone 가운데나 filler: 그 pane 끝으로 `moveTab`
    - zone 가장자리: `splitPane`
    - 탭 위: 그 탭의 index로 `moveTab`. 끄는 탭이 pinned면 `min(index, pinnedCount)`, 아니면 `max(index, pinnedCount)`입니다.
    - 자기 자리에 놓아도 `moveTab`을 그대로 보냅니다.
    - backend `layout_move_tab`/`layout_split`도 같은 경계를 지킵니다(`crates/taide-layout/src/service.rs` 830-871, 925-939).
- 표시
    - pinned 탭과 unpinned 탭은 서로 다른 `SortableContext`입니다(`pane-tab-bar.tsx` 266-273). 같은 단 안의 탭 위에서만 `horizontalListSortingStrategy`로 밀립니다(`@dnd-kit/sortable` `sortable.esm.js` 74, 507).
    - 원본 자리의 탭은 0.4 불투명도입니다(`sortable-tab.tsx` 9).
    - 포인터에는 `DragOverlay` 탭이 따라옵니다(`editor-area.tsx` 536-552). 0.9 불투명도이고, active 모양으로 제목·종류 아이콘·dirty·pinned·preview를 보입니다.
    - 걸린 zone은 `tabBar.dropTarget` 25% 채움과 1px 테두리로 절반 또는 전체를 덮습니다(`split-drop-zones.tsx` 16-34).
    - 탭은 간격 없이 붙어 있습니다(`tab-item.tsx`).
- 자동 스크롤: `DndContext`의 `autoScroll` 기본값이 켜져 있습니다(`core.esm.js` 2861).
    - 대상: 걸린 droppable의 스크롤 조상입니다. 걸린 것이 없으면 끄는 탭의 조상입니다(2956).
    - 동작: 그 탭바 너비의 가장자리 20% 띠에서 포인터가 이미 움직인 방향으로만 스크롤합니다. 5ms마다 `10 × 띠 안 깊이 비율`만큼 이동합니다(827-880, 1765-1874).
    - pane zone 아래(overflow hidden)에는 스크롤 조상이 없습니다.

## 구현

- UI `native/taide-native-ui/src/tab-drag.rs`
    - 새 모듈입니다. rail drag와 같은 frame 기록 방식을 씁니다.
    - frame 시작에 이전 frame 배치로 진행하고, 탭·filler·pane을 기록한 뒤, frame 끝에 정리합니다.
    - 상태는 `TabArea`(project, slot)로 나뉩니다.
    - pinned 경계 index 계산, 같은 단 가로 정렬 이동량, 클릭 억제, 자동 스크롤을 담습니다.
    - 탭의 drop 영역에는 뒤따르는 행 간격(egui `item_spacing`)도 포함했습니다. 원본 탭은 붙어 있어 사이에 drop이 안 되는 틈이 없기 때문입니다.
- 공용화 `project-drag.rs`
    - 활성화·Escape/resize 취소 판정을 `PointerInput::advance`/`Gesture`로 뺐습니다.
    - `pointerWithin`을 `pointer_within`으로 뺐습니다.
    - rail drag도 이 둘을 그대로 씁니다. 동작 변화는 없고 기존 rail 검사로 확인했습니다.
    - zone·미리보기 기하(`slot_drop_zones`/`slot_drop_preview`)와 세로 정렬식(축을 바꿔 가로에 사용)을 재사용합니다.
- `shell.rs`
    - 두 창 범위 모두에서 `show` 처음과 끝에 탭 drag를 진행합니다.
    - leaf마다 본문과 탭 목록을, 탭바 filler를 기록합니다.
    - 탭은 `UiBuilder::sense(drag)` 바탕 위에 그립니다. 제목·닫기 버튼 클릭은 그대로이고 drag는 바탕이 받습니다.
    - drag 중 원본 탭은 0.4 불투명도로 그리고, 화면 변환만으로 밀어 그립니다. 가림 영역은 원래 탭바 viewport로 유지합니다.
    - zone 미리보기는 rail drag와 같은 그리기 함수를 공용으로 씁니다.
    - `DragOverlay`는 상호작용 없는 그리기로 그립니다. tooltip 층에 0.9 불투명도이고, 아이콘은 `ShellSurfaces::tab_icon`입니다.
    - drop은 기존 `ShellMutation::MoveTab`/`SplitTab` → `layout_move_tab`/`layout_split` 경로를 씁니다(`commands.rs` 203-214). 실패는 기존 mutation 오류 경로입니다.
    - `ShellSurfaces`는 바꾸지 않아 remote-web은 그대로입니다.
- App `application.rs`
    - 탭 drag 중에도 web preview 같은 native overlay를 숨깁니다. 그래야 zone이 보이고 포인터를 받습니다.
    - 새 사용자 문자열은 없습니다.

## 검증

- UI `cargo test --features native-host,inspection --test workbench --lib`: lib 163/163, workbench 30/30 통과입니다.
    - `tab_drag` 단위 5개
        - 가로 정렬식
        - pinned 경계·filler·가운데·가장자리·빈 drop
        - 같은 영역만 고르는 충돌·꼭짓점 판정과 가려진 탭 제외
        - 활성 전후 보기와 단 분리
        - 자동 스크롤 속도·방향 의도
    - workbench 실제 egui 검사 4개
        - 3px는 클릭, 5px부터 drag이고 drag 뒤에는 활성화하지 않음
        - 유령 탭이 놓일 자리로 밀리고 지나간 탭이 한 칸 당겨짐, 미리보기는 포인터를 따라감
        - 고정탭 경계 index, 행 간격과 filler drop
        - 다른 pane 탭·가운데·가장자리·자기 pane 분할, + 버튼 무동작, 오른쪽 절반 미리보기, Escape 취소
        - 다른 slot 무동작
        - 넘친 탭바의 가장자리 자동 스크롤과 pane zone 위 정지
- App `cargo test --lib tab_drag_tests`는 1/1 통과입니다.
    - 실제 `NativeApplication`과 임시 project의 파일 탭 3개를 씁니다.
    - drag 재정렬과 오른쪽 가장자리 분할이 runtime layout에 반영됩니다.
- 영향 회귀
    - App `--lib project_rail_tests` 4/4 통과(rail drag 공용화)
    - App `--lib tab_menu_tests` 1/1 통과(탭 그리기 구조 변경)
- check와 형식
    - UI·App `cargo check --all-targets`, remote-web Wasm lib check는 exit 0입니다.
    - UI `cargo fmt -- --check`는 exit 0입니다.
    - App `cargo fmt -- --check`의 차이는 모두 이번 변경 밖의 기존 차이입니다: `application.rs` 908·10401 부근, `lib.rs` 모듈 순서, `experiments/lsp-coordinator-spike`.
- 앱 전체 lib 검사는 실행하지 않았습니다(배치 끝 메인 담당).

## 남은 범위

- 정렬 전환·drop 애니메이션 시간은 재현하지 않았습니다.
- 화면 픽셀과 실기 drag 확인은 배치 31입니다.
- 보조창 범위는 같은 `pane_tree` 경로를 쓰지만 보조창 전용 자동 검사는 없습니다.
