# d-62 — 사용성 배치 5 웨이브 5: 프로젝트 split view(셸 슬롯 분할) · 프로젝트 그룹 — 설계 계약 (2026-09-15, 구현은 3·4 머지 후)

> 사용자 항목 3·12("프로젝트 묶음 및 한 화면에 여러 프로젝트 표시 — 프로젝트를 드래그해 상하좌우로 split"). 결정 전제: `acknowledge/2026-09-15-usability-batch5-user-decisions.md` §1 #4(완전한 프로젝트 셸 단위 분할, 이름 있는 그룹) · §3 #3(2단계·ProjectGroup 엔티티·`shell_slots` 가산·그룹 열기 첫 멤버 즉시 + 백그라운드 큐) · §4(구현은 웨이브 3·4 머지 후).
> 조사 정본 `research/2026-09-15-batch5-research.md` T2. 상태 소유 원칙 ADR-0004(Rust 소유·view 표시 전용), `docs/features/layout-shell.md` §7(보조 창), `docs/data-model.md` §2·§5(무마이그레이션 필드 추가 규칙)·§20(ProjectDisplay 선례).

## 0. 현행 (T2 — 메인이 파일 위치 재확인)

| 사실 | 근거 |
|------|------|
| "활성 프로젝트" 정본은 `SessionState.active_project: Option<ProjectId>` 1필드. Rust 는 이미 `projects`·`layouts` 를 `HashMap<ProjectId, _>` 로 N개 동시 보유 | `src-tauri/src/domain/project/types.rs:94`, `src-tauri/src/state.rs:35-36` |
| 프론트에서 전역 활성 세션(`activeProjectQueryOptions`)을 읽는 비-테스트 파일 6개: app-shell·title-bar-content·status-bar-content·command-palette·task-runner-dialog·ide-sync-provider. **패널 위젯(Explorer·EditorArea·GitPanel·SearchPanel·PaneNodeView·EditorPane)은 전부 `projectId` prop 구동** → 멀티 인스턴스화 가능 | `src/entities/project/project.query.ts:30`, `src/widgets/app-shell/app-shell.tsx:219-250` |
| 보조 창 = 프로젝트 고정(`?projectId=&windowSlot=`)·`ProjectActivated` 무시·에디터 전용 셸(explorer·git·search·팔레트·TaskRunner 없음 — Wave I F1 미결). 레이아웃은 `ProjectLayout.auxiliary_windows: Vec<AuxWindowLayout>` | `src/shared/lib/window-context.ts:13-34`, `src/widgets/auxiliary-window-shell/*`, `src-tauri/src/domain/layout/types.rs:134-140,207`, `src/app/app.tsx:22-81` |
| 창 단위 싱글턴 3종이 병목: fire-and-forget 브리지(`shared/lib/bridge/*` — 사이드바 토글·뷰 전환·reveal·rename·search 열기), 전역 키맵(`use-global-keymap` + document capture), `getWindowContext()`(`location.search` 파생, 창 안 분할 개념 없음) | T2 Q1 |
| root_guard 는 열린 프로젝트 전체를 훑어 가장 긴 canonical root 를 고름 → 다른 프로젝트 소유 경로도 열기·저장 통과. 프로젝트 스코프 자원(미러·트리·검색)은 인자 `projectId` 로 검증 | `src-tauri/src/infra/root_guard.rs:24-57` |
| LSP 세션은 창(owner) 단위 격리, pty 는 멀티 attach 브로드캐스트, hot-exit 미러는 projectId 별 → 슬롯 분할에서 무변경 | `docs/features/layout-shell.md` §7.4, `docs/data-model.md` §2·§6·§16 |
| 그룹 개념 0. 사이드바 = `session.projects` 1차원 + dnd-kit `verticalListSortingStrategy`. 순서 변경 `project_reorder`. `ProjectDisplay{icon,label,color}` 가 `#[serde(default)]` 무마이그레이션 필드 선례 | `src-tauri/src/domain/project/service.rs:155-177`, `src/widgets/app-sidebar/app-sidebar.tsx:40-76`, `types.rs:26,52-60` |
| `project_open` 은 capability attach 를 await(응답 시간 불변) → 그룹 열기 직렬은 멤버 수 × 열기 시간. 부팅 복원은 `restore_project_watchers` 순차·활성 우선·중단 가능 | `src-tauri/src/domain/project/commands.rs:85-95,343-382` |
| 탭 분할 드롭존 선례 `src/features/split/split-drop-zones.tsx`(4방향), 레이아웃 `PaneNode` split 트리 | `src/features/split/*`, `layout/types.rs:116` |
| 보조 창은 `tauri-plugin-window-state` 추적 제외(위치·크기 복원 없음) | `layout-shell.md` §7.6 |


## 0.1 설계 사전 검토 반영 (2026-09-15, wf `wf_fefded11`, sonnet·xhigh 2렌즈 — major 10·minor 4·info 2, 전건 수용)

| id | 지적 | 계약 보정 |
|----|------|----------|
| S-1 | 슬롯 트리 축약이 프론트 후속 IPC 라 close 와 원자성 없음·부팅 복구 부재 | **`close_project` 서비스 안에서** 프로젝트 제거 + 세션 목록 정리 + `shell_slots` 축약 + `focused_slot` 재계산을 한 뮤테이션·한 저장으로. `ProjectClosed` 와 `SessionShellSlotsChanged` 를 같은 커맨드에서 emit. 부팅 복원(`restore_state`)에 dangling leaf 정규화(세션에 없는 projectId 리프 제거, 빈 트리면 active 로 리프 1개) |
| S-2 | 그룹 백그라운드 열기가 `project_open` 재사용 시 매번 활성화·`ProjectActivated` 발화 | `open_project` 에 `activate: bool` 인자(기존 커맨드는 `true`). 그룹 큐는 첫 멤버만 `true`, 나머지 `false`(세션 목록 추가·워처·capability 만, 활성·이벤트 없음 — `ProjectListChanged` 만) |
| S-3 | 같은 프로젝트 다중 슬롯 금지가 드롭존에서만 강제 | `project_open_in_slot` 서비스가 다른 리프에 같은 projectId 가 있으면 `AppError::InvalidArgument`(로케일 키 `error.shellSlot.projectAlreadyInSlot`) 로 거부. 드롭존 거부는 UX 선검사 |
| S-4 / U-2 | EditorArea 약 20개·TerminalPane 의 `useGlobalKeymap` 핸들러가 슬롯 수만큼 복제 → ⌘S/⌘W 동시 발화. when 컨텍스트도 전역 activeElement | **슬롯 포커스 게이트**: `ShellSlotContext` 의 `isFocused` 로 EditorArea·TerminalPane 의 모든 `useGlobalKeymap` 핸들러를 `isFocused ? fn : undefined` 게이팅(terminal-pane 선례). 리스너 등록 위치는 현행(컴포넌트별) 유지 — 게이트가 동시 발화를 막는다. 규모에 반영 |
| U-1 | `editor-pane-command-bridge`·`active-editor-actions-bridge` 등 EditorArea/TerminalPane 이 구독하는 브로드캐스트 브리지가 slotId 필터 대상에서 빠짐 | 구독 측 전수 게이팅: EditorArea·TerminalPane 이 구독하는 모든 브리지(editor-pane-command·active-editor-actions·explorer-panel/reveal/rename·search-panel·terminal 관련)는 `isFocused` 일 때만 처리. 발행 측은 무변경(포커스 슬롯만 반응하므로) — 단 "특정 슬롯 지정" 이 필요한 발행(외부 파일 열기·CLI 진입)은 `slotId` 를 실어 보내고 구독자가 자기 것만 처리 |
| U-5 | 발행부 12곳+·순수 함수 run() 은 포커스 슬롯을 모름 | 위 구독 측 게이팅으로 발행부 개조를 회피. `CommandContext` 에 `focusedShellSlotId`(읽기)만 추가 — 팔레트·TaskRunner 가 프로젝트를 고를 때 사용 |
| S-5 | 사이드바 클릭(`project_activate`)의 슬롯 의미 부재 | `activate_project`: 대상이 어느 슬롯에 있으면 그 슬롯을 `focused_slot` 으로; 없으면 **포커스 슬롯의 프로젝트를 교체**(기존 단일 슬롯 동작과 동일). 원격 dispatch 도 같은 서비스 경유 |
| S-6 / U-3 | `ShellViewState`(zen·sidebarCollapsed) 가 프로젝트별인데 사이드바 레일은 공용 → 포커스 전환 시 깜빡임. Zen·Problems 패널 상태 미정의 | **재분배**: 창 크롬 축(zen·사이드바 아이콘 레일 접힘·상태바)은 `SessionState.window_chrome`(무마이그레이션 가산)으로 이동, 슬롯 로컬 축(explorer 패널 접힘·Problems 열림)은 `ProjectLayout.shell_view` 유지. Zen = 포커스 슬롯만 남기고 나머지 슬롯 숨김(트리 보존). Problems 토글(상태바)은 포커스 슬롯의 `ProjectShell` 상태를 제어(컨텍스트 콜백) |
| U-4 | 사이드바 DndContext 와 EditorArea 탭 DndContext 가 분리돼 사이드바→슬롯 드롭 불가 | **AppShell 최상위 프로젝트 전용 DndContext** 가 사이드바 재정렬(기존)과 슬롯 드롭존을 함께 담고 `active.data.current.type`('project') 으로 분기. EditorArea 의 탭 DndContext 는 중첩 유지 — 중첩 동작은 구현 초기 스파이크 테스트로 확인([미확인]); 불가 시 사이드바→슬롯 드래그는 pointer 이벤트 오버레이로 대체 |
| U-6 | 그룹 편입 조작 부재 | 컨텍스트 메뉴 "그룹에 추가 ▸"(그룹 목록 서브메뉴)·"그룹에서 제거" 추가. 크로스 컨테이너 드래그는 범위 외 |
| U-7 | 포커스 슬롯 전달 경로 미정 | React Context 단일 경로(앱 루트 Provider, DOM 캡처 갱신). `session_focus_slot` 은 영속화 용도 |
| S-7 | `forget_recent` 가 그룹 멤버를 못 지움·큐의 결손 멤버 정책 | `forget_recent_projects(&mut session)` 로 확장해 `groups[].members` 정리 + 저장. 그룹 열기 큐는 결손 멤버 스킵 + `log::warn` + 진행 |
| S-8 / U-8 | `windowSlot`(보조 창) 과 명칭 충돌 | 신규 개념은 **`ShellSlotId`/`shellSlot`** 로 명명, 문서에 구별 표기 |

**규모 재추정**: 2단계는 L → **XL**. 단계 재편: **2a**(Rust: 슬롯 트리·window_chrome·activate/close/open 의미 + 프론트: 슬롯 트리 렌더·포커스 컨텍스트·키맵/브리지 게이팅·Zen/Problems 재배치, 분할은 메뉴/커맨드 "오른쪽/아래에 열기" 로) → **2b**(사이드바 드래그 분할 DndContext 통합) → **2c**(그룹). 각 단계 끝에 렌즈 검토.

## 1. 설계

### 1.A 데이터 모델 (Rust, 무마이그레이션 가산)

- `SessionState` 에 `#[serde(default)] shell_slots: ShellSlotTree`(비면 `active_project` 단독 해석 = 슬롯 1개). `ShellSlotTree = { node: 'leaf', project_id } | { node: 'split', dir: Horizontal|Vertical, children: [tree, tree], sizes }` — 레이아웃 `PaneNode` 와 같은 모양(재사용 가능하면 제네릭화하지 않고 별도 타입, 이유: 슬롯 리프의 페이로드는 탭 목록이 아니라 프로젝트). `active_project` 는 유지하되 의미를 **포커스 슬롯의 프로젝트**로 재정의(하위 호환: 슬롯 트리가 없으면 그대로). `focused_slot: SlotId`.
- `ProjectGroup { id, name, color: Option<ProjectColorToken>(ProjectDisplay 팔레트 재사용), members: Vec<ProjectId>, collapsed: bool }`, `SessionState.groups: Vec<ProjectGroup>`(`#[serde(default)]`). **`session.projects` 가 열림 여부·전역 순서의 단일 진실**, `members` 는 소속 집합. 프로젝트 close 시 멤버십은 유지(그룹은 "열 수 있는 것" 의 조직화)하되 레코드(`projects/<id>/`)가 forget 되면 멤버에서 제거. 사이드바는 그룹 헤더 아래 멤버 순서 = `session.projects` 순서로 필터.
- IPC: `session_set_shell_slots(tree)` · `session_focus_slot(slot)` · `project_open_in_slot(path|project_id, target_slot, edge: Left|Right|Top|Bottom|Replace)` · `project_group_create/rename/set_color/delete/set_members/reorder` · `project_group_open(id)`. 이벤트: `SessionShellSlotsChanged`·`ProjectGroupsChanged`. dispatch 테이블·bindings·ipc-contract.
- 그룹 열기: 첫 멤버 `project_open` 동기 → 나머지는 `restore_project_watchers` 패턴의 백그라운드 순차 큐(`is_shutting_down` 중단 가능), 진행은 `ProjectListChanged` 로 자연 반영. 그룹 열기 = 사이드바 조직화만(슬롯 자동 배치 없음, 결정 §3 #3).

### 1.B 프론트 — 셸 슬롯 (L)

- `AppShell` → 공용 `AppSidebar` + `ShellSlotTreeView`(react-resizable-panels 중첩 Group, 선례 `pane-node-view.tsx`) → 리프마다 `ProjectShell{projectId, slotId}`(Explorer Panel + EditorArea + 터미널/Problems 하단 — 현재 app-shell 의 프로젝트 부분을 그대로 컴포넌트화). 슬롯 트리 크기는 세션에 영속(`sizes`, 120ms 디바운스 — `pane-resize-commit.ts` 선례).
- **포커스 슬롯 컨텍스트** `ShellSlotContext{slotId, projectId}`: 브리지 3계열(`explorer-panel/reveal/rename-bridge`·`search-panel-bridge`)에 `slotId` 채널 필터(publish 시 포커스 슬롯 id 를 싣고 구독자는 자기 슬롯만 처리), 전역 키맵 핸들러(⌘B·⌘⇧F·⌘P 등)는 포커스 슬롯의 프로젝트에 적용(`useGlobalKeymap` 등록은 슬롯 셸이 아니라 앱 1곳에서, 포커스 슬롯 컨텍스트를 읽어 위임). 포커스 판정 = 마지막 pointerdown/focusin 이 속한 슬롯(DOM 이벤트 캡처 1곳). `getWindowContext()` 는 창 단위 그대로.
- 전역 활성 세션 소비 6파일: title-bar·status-bar·command-palette·task-runner·ide-sync 는 **포커스 슬롯 프로젝트**를 읽도록(컨텍스트 또는 `session_focus_slot` 이벤트). `activeProjectQueryOptions` 는 "포커스 슬롯 프로젝트" 의미로 유지.
- 드래그 분할: 사이드바 프로젝트 아이콘을 드래그해 슬롯 영역에 드롭 → `split-drop-zones.tsx` 와 같은 4방향 드롭존(+ 가운데 = 교체) → `project_open_in_slot`. 같은 프로젝트를 두 슬롯에 띄우는 것은 **1차 금지**(hot-exit 미러 prune sweep 상호작용 [미확인] — T2 리스크) → 드롭 거부 토스트.
- 슬롯 닫기: 슬롯 헤더 X 또는 프로젝트 닫기 → 트리 축약(형제가 부모 자리로). 마지막 슬롯은 항상 존재(프로젝트 0개면 Welcome).
- Welcome·빈 pane 규칙은 슬롯별 pane 트리에 그대로.

### 1.C 프론트 — 그룹 (M)

- 사이드바: 그룹 헤더(이름·색·접기) + 멤버 아이콘, dnd-kit 2단 `SortableContext`(그룹 순서 / 그룹 내), 그룹 밖 프로젝트는 "미분류" 로 하단. 컨텍스트 메뉴: 새 그룹·이름/색 변경·그룹 열기·그룹에서 제거·그룹 삭제. 그룹 생성 다이얼로그(`project-display-dialog` 패턴). 로케일 키 3종.
- Welcome 최근 목록·File > Open Recent 는 그룹과 직교(변경 없음).

### 1.D 1단계 — 보조 창 완성 (S, 선행)

- `app.tsx` aux 분기에 `ExplorerContainer`/`GitPanelContainer`/`SearchPanelContainer` 를 `projectId={windowContext.projectId}` 로 마운트, `CommandPalette`·`TaskRunnerDialog` 를 `projectId` prop 으로 좁힘(Wave I F1 종결). 보조 창의 위치·크기 복원(§7.6 제외)은 유지.

### 1.E 범위 외

- 같은 프로젝트를 여러 슬롯에(공유 탭 동기화), 슬롯 간 탭 드래그 이동(프로젝트가 다른 pane 트리 — d-55 ‘Copy into New Window’ 와 같은 급), 그룹 열기 = 슬롯 자동 배치, 보조 창 위치 복원.

## 2. 실행 계획 (3·4 머지 후)

1. **설계 검토 wf(sonnet·xhigh 2렌즈: 상태 소유·동시성 / UI 상호작용)** 로 이 계약을 먼저 검토(구현 전 major 는 계약 수정).
2. 구현 wf(opus·xhigh, §0.1 재편): **1단계 F1**(1.D 보조 창 완성) → **2a R**(Rust 단일: 슬롯 트리·window_chrome·activate/close/open(activate 옵션)·open_in_slot 검증·forget 그룹 정리·복원 정규화·이벤트·dispatch·bindings) → **2a F2**(슬롯 트리 렌더·ShellSlotContext·EditorArea/TerminalPane 키맵·브리지 게이팅·Zen/Problems 재배치·"오른쪽/아래에 열기" 메뉴) → 렌즈 검토 → **2b F3**(DndContext 통합·드롭존, 스파이크 선행) → **2c R2+F4**(그룹 엔티티·IPC·UI) → 렌즈 검토. 테스트: Rust 슬롯 트리 연산(split/remove/축약)·그룹 CRUD·멤버십 정리, TS 브리지 필터·포커스 판정·드롭존·2단 정렬.
3. 렌즈 검토 wf(3렌즈 + major 반박 2표) → 메인 2차 verify·vite build → 분할 커밋 → dev 푸시 → main ff → 사용자 실기(2 프로젝트 좌우 분할·포커스 전환 시 ⌘B/⌘⇧F 대상·그룹 열기).

## 3. 구현 기록

<!-- d62-record-design-review -->
<!-- d62-record-R -->
<!-- d62-record-F -->

## 4. 미결·후속

- 같은 프로젝트 다중 슬롯(미러 prune 상호작용 실측 후), 그룹 열기 자동 배치, 보조 창 위치·크기 복원, 슬롯별 explorer 폭 영속.
