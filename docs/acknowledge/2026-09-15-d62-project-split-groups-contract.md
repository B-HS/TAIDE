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
2. 구현 wf(opus·xhigh): **R**(Rust: 1.A 타입·서비스·커맨드·이벤트·dispatch·bindings, 단일) → **F1**(1.D 보조 창) ∥ **F2**(1.B 슬롯 트리·포커스 컨텍스트·브리지/키맵 스코프) → **F3**(1.B 드래그 분할 UI) ∥ **F4**(1.C 그룹 UI). 테스트: Rust 슬롯 트리 연산(split/remove/축약)·그룹 CRUD·멤버십 정리, TS 브리지 필터·포커스 판정·드롭존·2단 정렬.
3. 렌즈 검토 wf(3렌즈 + major 반박 2표) → 메인 2차 verify·vite build → 분할 커밋 → dev 푸시 → main ff → 사용자 실기(2 프로젝트 좌우 분할·포커스 전환 시 ⌘B/⌘⇧F 대상·그룹 열기).

## 3. 구현 기록

<!-- d62-record-design-review -->
<!-- d62-record-R -->
<!-- d62-record-F -->

## 4. 미결·후속

- 같은 프로젝트 다중 슬롯(미러 prune 상호작용 실측 후), 그룹 열기 자동 배치, 보조 창 위치·크기 복원, 슬롯별 explorer 폭 영속.
