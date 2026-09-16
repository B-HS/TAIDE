# 기능 — 앱 레이아웃 셸 · 멀티 프로젝트

> FR-A. 앱 최상위 구조와 프로젝트 수명주기. 탭·스플릿은 `tabs.md`, 탐색 사이드바는 `explorer-sidebar.md`.

## 1. 레이아웃 구조

```
윈도우
└── 앱 사이드바(세로, 고정폭 48px 급) | 셸 슬롯 트리
    셸 슬롯 트리 (§8 — 기본은 슬롯 1개, 분할하면 리사이즈 가능한 중첩 그룹)
    └── 슬롯 = 프로젝트 셸 1개
        └── 탐색 사이드바(접기 가능, 리사이즈 가능) | 에디터 영역(탭 바 + pane 트리)
```

- 앱 사이드바 = VSCode Activity Bar 위치에 **프로젝트 목록**을 두는 TAIDE 고유 구조.
- **슬롯이 하나면 화면은 분할 도입 이전과 똑같다** — 슬롯 헤더도 구분선도 없다(§8).
- 탐색 사이드바(explorer/search/git)와 에디터 영역은 **그 슬롯의 프로젝트의 것**만 렌더된다.
  어느 슬롯에도 없는 프로젝트의 view 는 unmount 하되, Rust 상태(pty·watcher·git 캐시)는 유지된다.
- 탐색 사이드바 토글: `⌘B` / `Ctrl+B` (VSCode 동일). 폭은 드래그 리사이즈, 프로젝트별로 저장.
  분할 상태에서는 **포커스 슬롯**에만 적용된다(§8.3).

### 1.1 빈 에디터 영역 = Welcome (메인 창 · 설정, 2026-09-04)

Welcome 화면(`widgets/welcome/welcome-container.tsx` — 최근 프로젝트 + 폴더 열기)이 나타나는 표면은
셋이고 **전부 같은 컴포넌트**다(d-27 §1.2 "적용 면 통일").

| 표면 | 조건 | 렌더 지점 | `projectId` |
|------|------|-----------|-------------|
| 프로젝트 0개 전체화면 | `projects.length === 0` | `app-shell.tsx` | `null` (파일·터미널 열기 비활성) |
| Welcome 탭 | 활성 탭이 `TabKind::Welcome` | `pane-node-view.tsx` | 활성 프로젝트 |
| **빈 에디터 영역** | 활성 프로젝트 O + 그 창의 탭 0 | `pane-node-view.tsx` | 활성 프로젝트 |

- 액션 줄은 **폴더 열기 · 파일 열기 · 터미널 열기** 3개다(터미널은 d-58). 터미널은 프로젝트 소속
  (Rust `ensure_project_open`)이라 파일 열기와 같은 조건(`projectId !== null`)으로 비활성되고, 둘 중
  하나라도 막히면 `app.openFileHint` 안내가 붙는다. 실행은 팔레트 `new-terminal` 과 공유하는
  `useOpenTerminalTab`(`entities/layout/layout.query.ts`)이며, 새 탭은 `target: null` 이 아니라
  **그 창의 focused pane** 에 열린다 — 보조 창의 Welcome 탭이 메인 창에 터미널을 여는 것을 막는다.
- **빈 에디터 영역은 탭이 아니라 렌더 교체다.** 레이아웃(`PaneNode` 트리)·`revision`·닫은 탭 스택·
  hot-exit 영속을 전혀 건드리지 않으므로 `⇧⌘T`(reopen closed)·복원·원격 세션과 구조적으로 충돌하지
  않는다. 탭으로 원할 때는 `⌘⇧P` → `view.welcome`(`command-palette.md` §2.1) 또는 탭 바 여백 메뉴를 쓴다.
- 판정은 `pane-node-view.tsx` 의 `!activeTab` 한 조건이다 — `normalize_owned` 가 split 의 빈 leaf 를
  모두 제거하므로 "한쪽만 빈 split pane" 은 존재할 수 없고, `!activeTab` 은 곧 **그 창의 탭 0** 과 동치다.
- **메인 창 전용이다(옵션 아님).** 보조 편집 창은 트리가 비면 스스로 닫히고(§7.3 ·
  `auxiliary-window-shell.tsx`), Welcome 의 "최근 프로젝트" 클릭은 전역 활성 프로젝트를 바꾸는 —
  보조 창에 금지된 — 동작이다. 보조 창의 빈 상태는 기존 `editor.noFileOpen` 문구를 유지한다.
- 끄기: 설정 > 인터페이스의 `welcomeOnEmptyEditor`(기본 `true`). 끄면 메인 창도 `editor.noFileOpen`
  문구로 돌아간다.
- 알려진 체감 이슈: 마지막 탭을 닫는 즉시 Welcome 이 화면을 채워 "탭이 안 닫혔나?" 로 읽힐 수 있다
  (`docs/quality-assurance` 실기 확인 항목).

## 2. 앱 사이드바

### 2.1 구성 (위→아래)

1. 그룹 섹션 — 그룹 헤더 + 그 그룹의 열린 멤버 아이콘 (그룹이 있을 때만, §2.4)
2. 미분류 프로젝트 아이콘 목록 (세션의 프로젝트 순서)
3. `+` 프로젝트 추가 메뉴 — 경로로 열기 / Finder 로 열기 / 최근 프로젝트 (§2.1.1)
4. (하단 고정) 설정 버튼 — 설정 탭을 활성 프로젝트에 연다

그룹이 하나도 없으면 1 이 사라지고 2 가 열린 프로젝트 전체가 되므로, 그룹 도입 전 레일과 배열이 같다.

#### 2.1.1 `+` 프로젝트 추가 메뉴 (사용성 배치 5, 2026-09-15)

`+` 는 더 이상 폴더 피커를 곧바로 띄우지 않고 드롭다운을 연다
(`features/project/sidebar-add-project-menu.tsx`, 순수 UI).

| 항목 | 동작 |
|------|------|
| **경로로 열기…** (`sidebar.openByPath`) | 경로 입력 다이얼로그 (아래) |
| **Finder 로 열기…** (`sidebar.openViaFinder`) | 기존 OS 폴더 선택 다이얼로그 (`useOpenFolderDialog`) |
| **최근 항목** (`app.recentItems`) 아래 최근 프로젝트 | `project_open(root)` |

- 최근 목록은 `project_list_recent` 를 **이미 열려 있는 프로젝트를 빼고** 최대
  `RECENT_PROJECT_MENU_LIMIT`(10, `shared/constants/project.ts`)개 보여준다. 열린 프로젝트는 바로 위
  아이콘 줄에 이미 있어 중복이고, 상한은 Rust `constants::RECENT_PROJECT_MENU_LIMIT` 와 같은 값이라
  네이티브 `File > Open Recent`(`window-chrome.md` §7)과 같은 깊이를 보여준다. Welcome 의 최근 목록
  상한(`RECENT_PROJECT_DISPLAY_LIMIT` = 8)도 같은 파일에 있다.
- 항목 라벨은 §2.2 의 `resolveProjectDisplay` 사다리를 그대로 따른다(짧은 라벨 > 이름). 보조 줄은
  `root`, `rootMissing` 이면 항목이 비활성이고 `app.recentProjectRootMissing` 이 붙는다.
- **선택은 `project_open(root)` 한 경로다.** 이미 열린 root 면 Rust 가 `already_open` 으로 재활성화하므로
  (`domain::project::service::open_project`) 프론트가 열림/활성 분기를 따로 두지 않는다.
- 트리거는 `IconButton` 이 아니라 `tab-bar-add-menu.tsx` 와 같은
  `DropdownMenu > Tooltip > TooltipTrigger asChild > DropdownMenuTrigger` 중첩이다 — `IconButton` 은 자체
  Tooltip 트리거 `span` 을 품고 있어 `DropdownMenuTrigger asChild` 와 트리거가 겹친다.

**경로로 열기 다이얼로그** (`features/project/open-project-by-path-dialog.tsx`)

- Enter 제출, 좌우 공백 trim, 빈 값이면 확인 비활성, 진행 중이면 재제출 없음. 닫힘→열림 전이에서 입력을
  비운다(`create-tag-dialog.tsx` 선례) — 취소한 경로가 다음 열기에서 확인 한 번에 열리면 안 된다.
- `~`/`~/…` 확장과 존재·디렉토리 검사는 **Rust `open_project`**(`resolve_open_root`) 몫이다. 실패는
  `error.project.pathNotFound` / `error.project.pathNotDirectory` 토스트로 그대로 뜨고, 성공하면
  다이얼로그가 닫힌다.
- 다른 창에서 프로젝트가 열리거나 닫히거나 `Clear Recent` 되면 `project:list-changed` 가 `PROJECT.LIST` 와
  **`PROJECT.RECENT` 를 함께** 무효화한다(`ipc-sync-provider.tsx` 의 `PROJECT_LIST_CHANGED_INVALIDATIONS`).
  최근 목록은 열린 목록과 별개 쿼리라 하나만 무효화하면 이 메뉴와 Welcome 이 낡은 채로 남는다.
- 표시 라벨 변경(`project_set_display`)도 같은 `project:list-changed` 를 쏘므로 이 메뉴는 다른 창에서 바꾼
  라벨까지 곧바로 따라간다. Welcome 최근 목록은 여전히 `project.name` 을 그린다(§2.2 의 적용 범위).

### 2.2 프로젝트 아이콘 — 표시 설정 (사용성 배치 4, 2026-09-04)

프로젝트마다 아이콘·짧은 라벨·색을 지정할 수 있다. 영속 스키마는 `data-model.md` §20
(`ProjectDisplay { icon, label, color }` — `project.json` 정본 + `session.json` 의 `ProjectRef` 미러),
커맨드는 `ipc-contract.md` 의 `project_set_display` 다.

**폴백 사다리 (배타 3택)** — 정본은 `shared/lib/project-display.ts` 의 `resolveProjectDisplay` **한 함수**다.
specta 가 `display?:` 로 내보내므로 소비처마다 `??` 를 적으면 값이 갈린다(감사 R5#5). 컴포넌트는 이
함수의 결과만 읽는다.

| 순위 | 조건 | 표시 | 모드 |
|------|------|------|------|
| 1 | `label` 이 비어 있지 않음 | 라벨 텍스트(1~4 코드포인트) | `label` |
| 2 | `icon` 이 비어 있지 않음 | 카탈로그 아이콘 | `icon` |
| 3 | 그 외 | 폴더 아이콘 (기존 동작 무변경) | `default` |

- 라벨과 아이콘이 **같은 40px 버튼을 다투므로 배타**다. 라벨이 아이콘을 이긴다.
- **overflow 없음**: 글리프는 `max-w-full overflow-hidden` 래퍼 안에 그리고, 길이별 타이포 사다리
  (`shared/constants/project-display.ts` 의 `PROJECT_LABEL_CLASS_BY_LENGTH`)로 CJK 4자도 40px 안에 넣는다.
  래퍼가 버튼이 아닌 이유는 활성 인디케이터가 버튼 바깥(`-translate-x-1.5`)에 그려지기 때문이다.
- **색은 글리프/텍스트 색만** 바꾼다(`style={{ color: colorVar }}`). 팔레트는 테마 토큰 신설 없이
  `graph.lane1..lane12`(`var(--taide-graph-laneN)`)를 재사용한다 — 36종 번들 테마가 이미 전부 정의한다.
  배경(`appSidebar.itemActive`)을 물들이지 않으므로 활성 표시의 대비가 무너지지 않는다.
- **아이콘 카탈로그는 TS 단독 정본**이다 — `shared/icons/project-icon-registry.ts` 의 큐레이션 56종
  (`file-icon-registry.ts` 와 같은 정적 `Record`). Rust 는 `[a-z0-9-]` 문자셋만 검사하는 불투명 문자열로
  다루므로, 카탈로그에서 아이콘을 빼도 기존 `project.json` 이 파싱 실패하지 않고 `folder` 로 폴백한다.
  `lucide-react/dynamic` 전량(2007종)은 청크 수천 개 문제로 기각했다.
- **`aria-label` 은 항상 프로젝트 이름**이다. 짧은 라벨은 시각 축약일 뿐이라 접근성 트리에 새어나가면
  안 된다. hover 툴팁도 이름 + 루트 경로를 그대로 보여준다.
- 에이전트 상태는 표시 설정과 무관하게 **오버레이 배지**(`appSidebar.iconAgentRunning` 점)로 겹쳐 표시한다
  — 백그라운드 프로젝트에서 에이전트가 돌고 있음을 항상 인지 가능해야 한다.
- 적용 범위는 **1차로 앱 사이드바만**이다. Welcome 최근 목록·보조창 타이틀바로 넓힐 때는 다른 창 즉시
  동기화를 위해 `ProjectDisplayChanged` 이벤트가 필요하다(백로그).

**FR-A4(focus 타입별 아이콘)는 보류다.** PRD FR-A4 는 "focus 된 content 타입에 따라 아이콘이 바뀐다"를
요구했고 아래 표가 그 설계였으나 구현은 0건이며(`FocusKind`·`project:focus-kind-changed` 는 X-A 배치에서
제거), 사용자 지정 아이콘이 같은 슬롯을 쓴다. 되살릴 경우 **사용자 지정이 항상 이기고**, 지정이 없는
프로젝트에만 focus 아이콘을 적용한다(§4 의 `LAYOUT.DETAIL` 캐시 기반 유도가 여전히 정본).

| focus 타입 | 아이콘 |
|-----------|--------|
| 파일뷰(에디터) | 문서 아이콘 |
| 터미널 | 터미널 아이콘 |
| 설정 | 톱니 아이콘 |
| diff 뷰 | diff 아이콘 |
| code agent 실행 중 | 에이전트 아이콘 (터미널에서 에이전트 감지 시 — `agent-integration.md`) |

### 2.3 프로젝트 context menu

- 닫기(Close Project) — pty·LSP 등 실행 중 자원이 있으면 확인 다이얼로그
- Finder(파일 관리자)에서 열기 / 경로 복사
- **프로젝트 표시…**(`project.displayMenu`) — §2.2 의 표시 설정 다이얼로그
  (`features/project/project-display-dialog.tsx`)를 연다.
- **기본값으로 되돌리기**(`project.displayReset`) — 세 축을 한 번에 해제한다. 표시 설정이 하나도 없으면
  비활성이다. 해제는 축마다 빈 문자열을 보내는 규약(`null` = 유지)이라 상수
  `CLEARED_PROJECT_DISPLAY_PATCH` 하나로 표현된다.
- **오른쪽/아래/왼쪽/위에 열기**(`shellSlot.open*`) — 포커스 슬롯을 그 방향으로 분할한다(§8.2).
- **그룹에 추가 ▸**(`projectGroup.addTo`) — 그룹 목록 서브메뉴 + **새 그룹…**(`projectGroup.newGroup`).
  이미 속한 그룹은 비활성. / **그룹에서 제거**(`projectGroup.removeFrom`) — 속한 그룹이 없으면 비활성.
  (§2.4)
- 사이드바 내 순서 변경은 DND(세로 sortable) — 순서는 세션에 저장

#### 2.3.1 표시 설정 다이얼로그

- 표시 모드 라디오 3택(아이콘 / 텍스트 라벨 / 폴더 아이콘) — 배타.
- 아이콘 모드: `shared/ui/command` 검색이 붙은 8열 그리드(카탈로그 56종, 이름으로 검색).
- 라벨 모드: 입력은 **코드포인트 기준**으로 잘린다(`maxLength` 는 UTF-16 단위라 서로게이트 쌍을 반으로
  자른다). 저장 직전에만 trim 하므로 `A B` 처럼 가운데 공백이 있는 라벨을 입력할 수 있다.
- 색 12 스와치는 모드와 무관하게 선택 가능하고, 선택된 스와치를 다시 누르면 해제된다.
- 미리보기는 사이드바와 **같은 `ProjectDisplayGlyph` 컴포넌트**를 그린다(클래스 이중 관리 금지).
- 닫힘→열림 전이에서 렌더 중 state 를 리셋한다(`create-tag-dialog.tsx` 선례) — 취소한 편집이 다음
  프로젝트의 다이얼로그에 남아 잘못된 프로젝트에 저장되는 사고를 막는다.

### 2.4 프로젝트 그룹 (d-62 2c, 2026-09-16)

프로젝트를 이름 있는 그룹으로 묶어 레일에서 접고 펼치고 한 번에 열 수 있다. 영속 스키마는
`data-model.md` §23(`ProjectGroup { id, name, color, members, collapsed }` — `session.json` 정본),
커맨드는 `ipc-contract.md` 의 "project group" 절이다.

**두 축은 분리돼 있다.**

| 축 | 정본 | 뜻 |
|----|------|----|
| 열림 여부·전역 순서 | `session.projects` | 레일에 그려지는 것과 그 순서 |
| 소속 | `ProjectGroup.members` | "열 수 있는 것" 의 조직화 — **닫힌 프로젝트도 멤버일 수 있다** |

- 그러므로 섹션의 멤버는 `members` 를 순회해 만들지 않고 **열린 목록을 필터**해서 만든다
  (`shared/lib/project-group.ts` 의 `resolveProjectGroupSections`). 닫힌 멤버는 레일에 없고, 멤버 순서는
  언제나 `session.projects` 순서다.
- 한 프로젝트는 **최대 한 그룹**에 속한다. "그룹에 추가" 는 대상 그룹의 `members` 에 붙이는 한 번의
  `project_group_set_members` 이고, 이전 그룹에서 빼는 것은 서버가 한다.
- 프로젝트를 닫아도 멤버십은 남는다. 레코드를 `forget` 하면 그때 멤버에서도 빠진다(계약 §0.1 S-7).

**헤더** (`widgets/app-sidebar/sortable-project-group-header.tsx`)

- 접기 토글이 곧 드래그 핸들이다. 접힘은 로컬 state 가 아니라 `project_group_set_collapsed` — 세션에
  살아야 재시작·다른 창까지 같은 상태가 된다.
- 색은 §2.2 의 `PROJECT_COLOR_TOKENS`(`graph.lane1..lane12`) 를 그대로 쓴다. 새 테마 토큰 없음.
- context menu: **그룹 열기**(`projectGroup.open`) / **이름·색 변경…**(`projectGroup.edit`) /
  **그룹 삭제**(`projectGroup.delete`, 확인 다이얼로그 — 그룹만 사라지고 멤버는 열린 채로 남는다).
- **그룹 열기**는 첫 멤버만 활성화하고 나머지는 `activate: false` 로 순차 열기(계약 §0.1 S-2). 결과
  `{ opened, skipped }` 를 `projectGroup.openResult` 토스트로 알린다(이미 열림·레코드 없음·셧다운은 skip).
  **슬롯 자동 배치는 하지 않는다**(결정 §3 #3).

**2단 정렬** — 레일의 `DndContext` 는 여전히 `AppShell` 의 그것 하나다(계약 §0.1 U-4).

| 단 | `SortableContext` | 드롭 결과 |
|----|-------------------|-----------|
| 바깥 | 그룹 헤더들 | `project_group_reorder(ids)` |
| 안쪽 | 그룹별 멤버 + 미분류 | `project_reorder(ids)` — **전역** 순서를 고쳐 쓴다 |

- 두 단은 `active.data.current.type`(`project` / `projectGroup`)으로 갈린다. 헤더를 아이콘 위에, 아이콘을
  헤더 위에 떨구면 `indexOf` 가 대상을 못 찾아 **아무 일도 일어나지 않는다**(`shared/lib/project-drag.ts`).
- 따라서 **드래그로 그룹을 옮길 수는 없다.** 크로스 컨테이너 이동은 context menu 전용이다(계약 §0.1 U-6).

**다이얼로그** (`features/project/project-group-dialog.tsx`) — 생성·편집 공용. 이름(1~40 코드포인트,
제출 직전에만 trim) + §2.2 와 같은 12 스와치. 닫힘→열림 전이에서 렌더 중 리셋하는 것도 §2.3.1 과 같다.

**동기화** — 모든 그룹 쓰기는 `project:groups-changed` 를 쏘고 `ipc-sync-provider.tsx` 가
`QUERY_KEY.PROJECT_GROUP.ALL` 을 무효화한다. `project_group_open` 만 예외로 그룹이 아니라 열린 프로젝트를
바꾸므로 멤버마다 `project:list-changed` 로 나타난다.

**Welcome 최근 목록·File > Open Recent 는 그룹과 직교**한다(변경 없음).

## 3. 프로젝트 수명주기

| 단계 | 동작 |
|------|------|
| 열기 | 폴더 선택 → Rust `project_open` → ProjectId 발급(기존 열림 이력 있으면 기존 id 재사용) → capability 자동 부착(`.git` 감지 → Git, 파일 감지 → LSP lazy) → 기본 레이아웃 생성(Welcome 1 + 터미널 1, `tabs.md` §2) → `project:opened` 이벤트 |
| 활성화 | `project_activate` → 세션의 active 갱신 → view 는 해당 프로젝트 레이아웃으로 스왑 |
| 닫기 | `project_close` → capability detach(pty kill, LSP shutdown, watcher stop — 대칭 해제) → 레이아웃·버퍼 영속화 후 메모리 해제 → `project:closed` |
| 재시작 복원 | `session.json` 의 프로젝트 전량 재-open (레이아웃은 `layout.json` 복원). root 부재 시 사용자에게 재연결/제거 선택 |

- 동일 폴더 중복 열기는 금지 — 기존 프로젝트를 활성화한다.
- 하위/상위 폴더 관계의 프로젝트는 각각 독립 프로젝트로 허용한다(watcher 중복 비용은 감수, 문서화).
- **열기·활성화 모두 `Project.last_opened_at`(밀리초 epoch)을 갱신하고 `project.json` 에 영속한다**
  (d-27) — 재열기 분기 포함 `project_open` 과 `project_activate` 둘 다. Welcome 화면의 "최근
  프로젝트" 정렬 기준이며, `project_list_recent`(§4)가 이 값의 내림차순으로 반환한다.

## 4. IPC (상세: `docs/ipc-contract.md`)

- query: `project_list`, `project_get(projectId)`, `project_list_recent()`(d-27 — 디스크의 영속
  프로젝트 기록 전수를 `last_opened_at` 내림차순으로 반환, Welcome 화면 "최근 프로젝트" 전용, 원격
  dispatch 거부)
- mutation: `project_open(path)`, `project_close(projectId)`, `project_activate(projectId)`,
  `project_reorder(ids)`
- event: `project:opened`, `project:closed`, `project:activated`, `project:list-changed`,
  `agent:state-changed(projectId, agents[])` (`agent-integration.md`)
  (`project:focus-kind-changed(projectId, kind)` 는 X-A 배치(2026-08-19)에서 제거됐다 — 소비자가
  0 이었고, 레이아웃 변이마다 무조건 발행돼 트래픽만 2배였다. §2.2 의 focus 종류는 이미
  `layout:changed` 가 무효화하는 `LAYOUT.DETAIL` 캐시(`ProjectLayout.focused_pane` + 활성 탭의
  `kind`)에서 프론트가 그대로 유도할 수 있어, 이 파생값을 Rust 가 별도 이벤트로 다시 계산해
  내보낼 필요가 없었다 — FR-A4 아이콘을 되살릴 때도 이 캐시 기반 유도가 정본이다. FR-A4 자체는
  §2.2 대로 보류 상태다)
- mutation(신규, 사용성 배치 4): `project_set_display(projectId, patch)` — §2.2 의 표시 설정.
  신규 이벤트 없이 기존 `project:list-changed` 를 재발행한다(`ProjectRef.display` 미러가 payload 에
  실려 오므로 모든 창·원격 세션이 기존 fanout 으로 갱신된다).

## 5. 수명주기 · 누수 방지

- 프로젝트 전환 시 이전 프로젝트의 위젯 트리는 unmount 된다. Monaco/xterm 인스턴스는
  각 위젯 정책(`editor.md`·`terminal.md`)에 따라 dispose 또는 캐시 반납.
- 앱 사이드바가 구독하는 이벤트는 `project:*`, `agent:state-changed` 두 계열뿐이며
  `useTauriEvent` 훅으로만 구독한다(cleanup 자동).
- 프로젝트 close 는 Rust 에서 capability detach 완료를 보장한 뒤 이벤트를 발행한다
  — view 가 먼저 사라져도 자원 해제는 Rust 책임(ADR-0004).

## 6. 엣지 케이스

- 마지막 프로젝트를 닫으면 빈 상태 화면(웰컴: 최근 프로젝트 목록 + 열기 버튼).
- 프로젝트 루트 폴더가 실행 중 삭제/이동되면: watcher 가 감지 → 배너 표시(재연결/닫기 선택).
- 권한 없는 폴더 열기 실패 시 명확한 에러 토스트(errno 전달).

## 7. 멀티 윈도우 — 보조 편집 창 (Wave I, 완전 구현)

> 계약: `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.1/§3.2. 스키마:
> `data-model.md` §8. IPC: `docs/ipc-contract.md` "Wave I 계약 확정 추가". 사용자 지시: "MVP 가
> 아니라 제대로 완벽하게" — 이 절에서 **§3 갭 분석 문서(`research/2026-08-13-vscode-cursor-gap.md`)가
> P1 로 남겨뒀던 "멀티 윈도우 / Move into New Window"는 종결됐다.**

### 7.1 창 모델

- **main 창 하나 + 보조 편집 창(`editor-<n>`) 0개 이상.** 라벨은 Rust 가 발급한다(현재 열려 있는
  라벨 중 재사용 가능한 최소 번호 — 창을 닫았다 다시 열어도 라벨이 무한정 늘지 않는다).
- 보조 창은 **앱 사이드바(프로젝트 아이콘 레일)·상태바가 없는 크롬**이다(`AuxiliaryWindowShell`,
  `window-chrome.md` §5). 자기 `(projectId, windowSlot)` 에 **고정**되어 렌더링하고, main 창의 활성
  프로젝트가 바뀌어도(`ProjectActivated`) 영향받지 않는다.
- **d-62 §1.D 로 보조 창이 완성됐다**: 탐색 사이드바(`ExplorerContainer` — 파일/검색/SCM/아웃라인
  뷰 스위처가 딸려 있으므로 `SearchPanelContainer`·`GitPanelContainer` 도 이 한 번의 마운트로 들어
  온다)와 `CommandPalette`·`TaskRunnerDialog` 가 보조 창에도 마운트된다. 셋 다 이 창의 고정
  `projectId` 로 스코프된다 — 팔레트/TaskRunner 는 전역 활성 프로젝트 세션을 스스로 읽던 것을
  `projectId` prop 으로 바꿨고(메인 창은 `app/main-window-dialogs.tsx` 가 활성 프로젝트를 주입),
  그 자기-읽기가 바로 Wave I 가 둘을 main 창 전용으로 묶어 뒀던 유일한 이유였다. `AppShell` 만
  여전히 main 창 전용이다(프로젝트 1개에 고정된 창에는 프로젝트 전환 레일이 의미가 없다).
  `KeybindingsRuntimeProvider` 는 원래부터 양쪽 창에 붙는다(그 JSDoc 참고).
- 보조 창의 탐색 사이드바 접힘/펼침(`⌘B`)은 **창 로컬**이다. `shell_view.sidebarCollapsed` 는
  프로젝트 단위 필드라 같은 프로젝트를 연 main 창과 서로 덮어쓰기 때문이다(재분배는 d-62 §0.1 S-6).
- 사이드바 토글·뷰 전환·reveal·rename·search 열기 브리지는 **JS 렘 단위 모듈 상태**이고 Tauri 는
  창마다 webview(=렘)를 따로 주므로, 보조 창에서 publish 한 요청은 그 창의 패널에만 도달한다 —
  `AppShell` 의 배선을 그대로 재사용할 수 있는 근거다(창 스코프 채널이 따로 필요 없다).

### 7.2 창을 여는/이동하는 경로

- 탭 컨텍스트 메뉴(`tabs.md` §3.1) · 커맨드 팔레트(d-62 §1.D 이후 양쪽 창) 에 3가지 액션이 있다:
  **Move into New Window**(항상 가능) · **Move back to Main Window**(보조 창일 때만) · **Move to
  Window N**(다른 열려 있는 보조 창마다 하나씩).
- 셋 다 `layout_move_tab_to_window(tabId, target)` 커맨드 하나로 처리된다. 탭의 dirty·미러 연결·
  pinned 상태는 `Tab` 구조체 자체가 통째로 옮겨지므로 **무손실**이다(내용을 다시 만들지 않는다).
- 새 보조 창을 만드는 경로(팔레트/컨텍스트 메뉴의 "New Window", 부팅 시 복원, 탭 이동으로 인한 신규
  창 생성)는 전부 `window::commands::open_auxiliary_window` 하나의 가드-프리 코어를 재사용한다 —
  각 호출부가 자기 mutation guard 를 잡으므로 재진입 데드락 없이 안전하게 공유된다.
- **"Copy into New Window"(같은 탭을 두 창에 동시에 열기)는 구현하지 않았다.** `tabs.md` §3.1 의
  기존 계획 표에 있던 항목이지만 이번 계약(§3.1/§3.2)이 요구한 것은 "이동"(Move)뿐이다 — 동일 파일을
  다른 pane 에 여는 것과 달리 동일 *탭*(같은 dirty/미러/pinned 상태)을 두 창이 공유하는 것은 별도
  동기화 설계가 필요해 이번 웨이브 범위 밖으로 남았다(backlog).

### 7.3 닫기·복귀 (0-손실 철학)

- **보조 창을 닫아도 앱 전체가 종료되지 않는다.** Wave I 이전에는 `WindowEvent::CloseRequested` 가
  창 라벨과 무관하게 전역 hot-exit 종료 경로로 들어가 두 번째 창을 닫으면 앱 전체가 죽는 차단급
  결함이 있었다(§2 확정 사실 1) — 이제 보조 창의 close 는 그냥 진행되고, main 창만 기존 hot-exit
  플러시 시퀀스(§`data-model.md` §6.3)를 탄다.
- 보조 창을 닫으면(✕, ⌘W 로 마지막 탭까지 닫아 트리가 빈 경우 모두) 그 창의 탭 전체가 **main 창
  포커스 pane 말미로 복귀**한다 — VS Code 는 보조 창을 닫으면 탭이 사라지지만, TAIDE 는 항상
  보존한다. 상세 판정(플레인 닫기 vs `layout_move_tab_to_window`)은 `data-model.md` §8 참조.
- 메뉴 Quit(⌘Q)·single-instance 는 여전히 `main` 라벨 고정 — 보조 창을 별도로 종료하지 않는다(main
  창의 hot-exit 완료가 전체 종료를 대표한다).
- 앱 재시작 시 열려 있던 모든 프로젝트의 보조 창을 Rust 가 재생성한다(활성 프로젝트로 제한하지
  않는다 — 계약 문언을 문자 그대로 구현).

### 7.4 창 사이 공유 자원 — LSP · 터미널

- **LSP 세션**: 세션 재사용(`lsp_spawn` 의 reuse 경로)은 **같은 창(owner)** 안에서만 일어난다 —
  `lsp_spawn` 은 호출한 창의 라벨(`getCurrentWindow().label`)을 `owner` 로 함께 보내고,
  `find_reusable_entry` 는 그 owner 가 이미 구독 중인 세션만 재사용 후보로 본다. 같은 프로젝트를
  main+보조 창에 나눠 열면(`sharesSessions` 서버라도) 각 창이 **독립된 프로세스**를 갖는다 — 두
  창의 LSP 클라이언트는 서로 다른 JS 렘이라 각자 자기 세션에 대해 `initialize` 핸드셰이크를 한 번씩
  수행하고 자기 요청 id 를 독립적으로 발급하는데, 세션을 공유하면 두 번째 창의 `initialize` 가 서버의
  중복-초기화 거부(`-32600`)로 실패해 그 창의 LSP 기능이 전멸하고, 두 창의 요청 id 공간이 겹쳐 응답이
  엉뚱한 창으로 라우팅될 수 있었다(하이라이트: Wave I 검토에서 확정된 차단급 결함, §2 확정 사실 2 의
  근본 원인). 같은 창 안에서(같은 프로젝트를 여러 탭/pane 으로 여는 경우)의 재사용은 그대로 동작한다.
  세션은 `owner` 별 구독 채널 맵을 유지하고 서버 메시지를 전 구독자에 브로드캐스트하며, `lsp_stop` 은
  자신의 `owner` 항목만 명시적으로 제거한다(창이 닫혀 `send` 가 실패하는 구독자는 그와 별개로 다음
  메시지에서 자동 제거).
- **터미널(pty)**: 같은 세션을 여러 창(또는 데스크톱+원격)이 동시에 attach 하면, Wave I 이전에는
  나중 attach 가 이전 구독자를 탈취했다(§2 확정 사실 3) — 이제 attach 마다 자기만의 ring-buffer
  replay 를 먼저 받은 뒤 구독자 목록에 합류하고, 출력은 전 구독자에 브로드캐스트된다. `pty_attach` 가
  돌려주는 구독 id 로 `pty_detach` 를 호출하면(탭 언마운트 시 프론트가 자동 호출) 창이 열린 채로도
  그 구독만 명시적으로 제거된다 — send 실패에 의한 자동 제거만으로는 "창은 열려 있지만 이 세션 시청을
  그만둔" 경우를 잡지 못하기 때문이다.

### 7.5 capability

- `capabilities/main.json` 의 `windows` 는 `["main", "editor-*"]` 글로브다 — 보조 창도 core/plugin
  권한(이벤트 구독 포함)을 정상적으로 받는다. Wave I 이전에는 `["main"]` 만 매칭돼 보조 창이 열려도
  갱신 이벤트를 받지 못하는 "조회는 되는데 갱신이 안 오는" 반쪽짜리 창이 될 뻔했다(§2 확정 사실 4).
- 앱 커스텀 커맨드(`window_set_fullscreen`·`layout_move_tab_to_window` 등, 과거엔
  `window_open_auxiliary` 도 — X-A 배치(2026-08-19)에서 중복 커맨드로 제거됨)는 이 프로젝트가 app
  ACL manifest 를 두지 않아 창 라벨과 무관하게 항상 허용된다 — capability 글로브의 영향을 받는 것은
  core/plugin 권한뿐이다.

### 7.6 알려진 제한

- "Settings" 탭을 보조 창으로 옮길 수 있다 — `SettingsView` 는 자신이 속한 `projectId`(props 로
  전달, 전역 활성 프로젝트가 아니다)를 기준으로 `settings.json`/프롬프트 탭을 연다.
- 메인 창이 프로젝트를 닫는 동안 그 프로젝트의 보조 창이 열려 있으면, 그 보조 창은 레이아웃 조회가
  `NotFound` 로 실패하는 것을 감지해 스스로 닫힌다(`auxiliary-window-shell.tsx`).
- 새 보조 창의 기본 크기는 1000×700, 최소 640×420(계약이 구체 수치를 정하지 않아 임의로 정한 값 —
  `domain/window/types.rs` 의 `AUXILIARY_WINDOW_DEFAULT_*`/`AUXILIARY_WINDOW_MIN_*`).
- 보조 창은 `tauri-plugin-window-state` 의 위치·크기 추적 대상에서 제외된다(`lib.rs` 의
  `with_filter`) — 계약 §3.1 이 허용한 두 선택지(단일 키로 정규화 / 제외) 중 후자. `map_label` 로
  모든 `editor-<n>` 을 하나의 캐시 키로 접기만 하고 제외하지 않으면, 보조 창을 2개 이상 열었을 때
  둘 다 그 하나의 캐시 항목에 저장된 같은 좌표·크기로 복원되어 완전히 겹쳐 열린다 — 매번 기본
  크기(1000×700)로 여는 편이 `.window-state.json` 무한 증식과 겹침 둘 다를 동시에 피한다.

## 8. 셸 슬롯 — 한 창에 여러 프로젝트 (d-62 2a·2b)

> 계약 `docs/acknowledge/2026-09-15-d62-project-split-groups-contract.md` §1.A·§1.B + §0.1.
> Rust 스키마·연산은 `data-model.md` §22 와 `domain::project::shell_slots`, IPC 는
> `ipc-contract.md` 의 "session — 셸 슬롯·창 크롬" 절이 정본이다. 창 크롬(Zen·레일)은
> `window-chrome.md` §6.
>
> **이름 주의**: 여기서 말하는 "슬롯"은 `ShellSlotId`(한 창 *안*의 영역)다. `AuxWindowLayout.slot`
> (보조 OS 창, §7)과는 다른 개념이며 계약 §0.1 S-8 이 둘을 구분해 명명했다.

### 8.1 슬롯 트리

- `SessionState.shell_slots` = 이진 트리. 리프 하나가 **프로젝트 셸 한 벌**(탐색 패널 + 에디터
  영역 + Problems 패널)이고, split 노드는 방향(`horizontal`/`vertical`)과 자식 2개·크기 비율을
  갖는다. 프로젝트의 pane 트리(`tabs.md`)는 그 리프 *안*에 그대로 중첩된다.
- 렌더는 `widgets/app-shell/shell-slot-tree-view.tsx` → 리프마다
  `widgets/app-shell/project-shell.tsx`. 중첩 `react-resizable-panels` 그룹이라 pane 트리
  (`pane-node-view.tsx`)와 같은 구조를 한 단계 위에서 반복한다.
- **슬롯이 1개면 헤더가 없다.** 2개 이상일 때만 슬롯 헤더(프로젝트 이름 + ✕)가 붙고, 마지막 슬롯의
  ✕ 는 비활성이다 — `shell_slot_close` 가 서버에서도 같은 이유로 거부한다.
- split 노드에는 id 가 없다. 크기 영속(`session_set_shell_slot_sizes`)은 루트부터의 **자식 인덱스
  경로**로 노드를 지정하고, 드래그는 pane 리사이즈와 같은 debounce(`pane-resize-commit.ts`)를 탄다.
- **Zen 은 렌더 트리를 바꾸지 않는다.** 포커스 슬롯만 보이는 화면(`window-chrome.md` §6.1)은
  비포커스 리프의 `Panel` 과 그 사이 구분선에 **`hidden` 속성**을 얹어서 만든다(Tailwind preflight 의
  `[hidden]{display:none!important}`). 슬롯 컴포넌트는 Zen 중에도 **전부 마운트된 채로 남아**,
  토글이 에디터 모델·스크롤 위치·터미널 버퍼를 버리지 않는다. 슬롯이 1개인 창도 같은 경로를 탄다 —
  Zen 일 때만 다른 컴포넌트를 루트에 두면 분할하지 않은 창에서도 프로젝트 셸이 재마운트된다.
  슬롯 헤더는 Zen 에서 렌더되지 않는다(상태를 갖지 않아 숨길 필요가 없다).
- 같은 프로젝트를 두 슬롯에 띄우는 것은 **금지**다(hot-exit 미러 상호작용 미검증). 서버가
  `error.shellSlot.projectAlreadyInSlot` 로 거부하고 프론트는 그 메시지를 토스트한다.

### 8.2 분할·닫기 조작

| 조작 | 경로 |
|------|------|
| 오른쪽/아래/왼쪽/위에 열기 | 사이드바 프로젝트 아이콘 context menu → `project_open_in_slot(projectId, 포커스 슬롯, edge)` |
| 아이콘을 끌어서 분할·교체 | 사이드바 아이콘을 슬롯의 드롭존에 드래그 → `project_open_in_slot(projectId, 그 슬롯, edge)` — §8.3 |
| 프로젝트 전환(분할 없음) | 아이콘 클릭 → `project_activate` — 그 프로젝트가 이미 어느 슬롯에 있으면 **그 슬롯으로 포커스**, 없으면 **포커스 슬롯의 프로젝트를 교체**(계약 §0.1 S-5, 단일 슬롯 시절 동작과 동일) |
| 슬롯만 닫기 | 슬롯 헤더 ✕ → `shell_slot_close` (프로젝트는 계속 열려 있다) |
| 프로젝트 닫기 | 기존 `project_close` — 프로젝트 제거·슬롯 축약·포커스 재계산이 한 뮤테이션이다(§0.1 S-1) |

- 메뉴는 언제나 **포커스 슬롯**을 겨냥하고, 드래그는 포인터가 가리킨 슬롯을 겨냥한다. 둘 다 같은
  커맨드를 부른다.

### 8.3 드래그 분할 — 레일에서 슬롯으로 (d-62 2b)

- 창 전체가 **프로젝트 전용 `DndContext` 하나**를 갖는다(`widgets/app-shell/app-shell.tsx` 최상위,
  배선은 `use-project-drag.ts`). 사이드바 재정렬(`SortableContext`)과 슬롯 드롭존이 그 안에 함께
  들어 있다 — 레일에서 시작해 슬롯에서 끝나는 드래그는 하나의 드래그이기 때문이다(계약 §0.1 U-4).
  사이드바는 자기 `DndContext` 를 잃고 `SortableContext` 만 남았다.
- **슬롯마다 드롭존 5종**: 좌/우/상/하는 그 방향으로 분할, 가운데는 그 슬롯의 프로젝트를 교체.
  탭 분할이 쓰는 `features/split/split-drop-zones.tsx` 를 한 단계 위에서 그대로 재사용한다.
  **드래그 중에만** 그린다 — 슬롯을 통째로 덮기 때문에 평소에 두면 클릭을 전부 먹는다.
- 드롭 → `project_open_in_slot(projectId, 대상 슬롯, edge)`. UI 의 `center` 는 커맨드의 `replace`
  로 번역된다(`shared/lib/project-drag.ts` — 탭 분할의 `center` 와 의미가 다르다).
- **프론트가 스스로 막는 것은 하나뿐**이다: 이미 그 슬롯에 있는 프로젝트를 그 슬롯 **가운데**에
  떨어뜨리는 것(자기 자신으로 교체) → 아무 요청도 하지 않는다. 그 외 거부(다른 슬롯에 이미 있음
  등)는 서버가 판정하고 프론트는 `describeIpcError` 로 토스트한다(§0.1 S-3).
- **중첩 `DndContext` 는 서로를 보지 못한다.** 슬롯마다 `EditorArea` 가 자기 탭 `DndContext` 를
  갖는데, dnd-kit 은 등록부를 React 컨텍스트에 두므로 안쪽 프로바이더가 자기 하위 트리에서 바깥을
  가린다 — 탭의 드롭존은 안쪽에만 등록되고, 탭 드래그는 안쪽 센서만 깨운다. 구현 전 스파이크로 양쪽을
  실측해 고정했다(`widgets/app-shell/project-drag-nesting.test.tsx`). 성립하지 않았다면 대안인 포인터
  이벤트 오버레이로 갔어야 했다.
- 충돌 판정은 `pointerWithin` 우선 + **포인터가 레일 안일 때만 도는** `closestCenter` 폴백이다.
  폴백 후보에서는 슬롯 드롭존을 뺀다 — 안 그러면 아무 드롭 대상도 없는 자리에서 손을 떼도 "가장
  가까운 슬롯"이 분할된다. 레일을 벗어난 폴백까지 허용하면 슬롯 헤더·리사이저·타이틀바 위에서 뗀
  손이 "가장 가까운 아이콘"의 재정렬로 읽히므로, 레일 밖에서 드롭존을 빗나간 릴리스는 **대상 없음**
  으로 끝낸다. 레일 rect 는 `AppSidebar` 가 `nav` 에 거는 droppable(`PROJECT_RAIL_DROPPABLE_ID`,
  **측정 전용** — 드롭 대상이 아니라 후보에서 항상 빠진다)을 dnd-kit 이 측정한 값이다. 레일 안
  아이콘 사이 틈에서 떼는 재정렬은 폴백 덕분에 예전 그대로다.
- 포인터를 따라가는 것은 `features/project/project-drag-preview.tsx`(글리프만, 버튼 아님).
  레일의 원본 아이콘은 제자리에서 흐려진다 — `DragOverlay` 가 있으면 dnd-kit 이 드래그 소스를 직접
  옮기지 않기 때문이다.

### 8.4 포커스 슬롯 — 창 크롬이 누구를 가리키는가

- 포커스는 **DOM 이 정본**이다: 앱 루트의 `ShellSlotProvider`(`app/providers/shell-slot-provider.tsx`)
  가 캡처 단계 `pointerdown`/`focusin` 리스너 하나로 이벤트 대상이 속한 슬롯을 찾는다. 슬롯 밖
  (사이드바·상태바·Radix 포털의 다이얼로그/메뉴)은 `null` 로 해소되어 **포커스를 옮기지 않는다** —
  팔레트나 context menu 가 "열 때 보고 있던 슬롯"에 계속 작용하도록.
- Rust 의 `session_focus_shell_slot` 은 **영속용**이다. 화면은 로컬 override 로 같은 프레임에 먼저
  움직이고, 서버 값이 따라오면 override 는 스스로 사라진다.
- 창 하나짜리 전역 크롬은 전부 포커스 슬롯의 프로젝트를 읽는다: 타이틀바 · 상태바 · 커맨드 팔레트 ·
  TaskRunner · IDE 진단 push · 앱 사이드바의 활성 표시. 여섯 곳 다 자기 자신이
  `project_get_active` 를 읽던 것을 prop/컨텍스트로 바꿨다.
- 상태바의 Problems 토글은 **포커스 슬롯의** Problems 패널만 여닫는다(슬롯별 상태, `AppShell` 소유).

### 8.5 중복 리스너 게이팅 (§0.1 S-4/U-1)

슬롯마다 프로젝트 셸이 통째로 한 벌씩 마운트되므로, 창 단위 싱글턴이던 두 메커니즘이 슬롯 수만큼
복제된다. 둘 다 **구독 측에서** `useIsShellSlotFocused()` 로 막는다 — 발행 측(팔레트·메뉴·에디터
액션 12곳+)은 손대지 않는다.

| 대상 | 게이트 위치 |
|------|-------------|
| 전역 키맵(⌘S·⌘W·⌘F·그룹 포커스 등 약 27개) | `editor-area.tsx` — 핸들러를 `isFocused ? fn : undefined` 로. `undefined` 면 `useGlobalKeymap` 이 `preventDefault` 조차 하지 않아 포커스 슬롯이 그 키를 가져간다 |
| 터미널 점프(`terminal-jump-to-*`) | `terminal-pane.tsx` — 기존 xterm 포커스 게이트와 **AND** |
| `editor-pane-command` · `editor-opener` | `editor-area.tsx` 구독 핸들러 선두 |
| `active-editor-actions` 발행 | `editor-area.tsx` — 포커스 슬롯만 자기 액션 목록을 싣는다(팔레트의 `isEnabled` 가 이 집합을 읽는다) |
| `explorer-panel`(뷰 전환) · `explorer-reveal` · `explorer-rename` · `search-panel` | `explorer-panel.tsx`(뷰 전환·reveal·rename 수행) + `project-shell.tsx`(접힌 패널 펼치기) |
| `file-history-panel` | `file-history-panel.tsx` |

- **슬롯 스코프가 없으면 게이트는 항상 열려 있다.** 보조 창과 컴포넌트 테스트가 정확히 그 경우이고,
  그래서 이 기능은 그 두 실행 환경의 동작을 전혀 바꾸지 않는다.
- `terminal-write-bridge` 는 `tabId` 로 주소가 찍히므로 게이팅 대상이 아니다.
- 커맨드가 슬롯을 **지명**해야 할 때를 위해 `CommandContext.focusedShellSlotId`(읽기 전용)가 있다.

### 8.6 알려진 제한 (2c 시점)

- **그룹과 슬롯은 아직 무관하다** — 그룹(§2.4)을 열어도 멤버가 슬롯에 자동 배치되지 않는다
  (계약 §1.E 범위 외).
- 같은 프로젝트를 여러 슬롯에 띄울 수 없다(§8.1).
- **키보드로 분할할 수 없다** — 분할 경로는 context menu 와 드래그 둘뿐이고(§8.2), `APP_KEYMAP` 에
  슬롯 액션이 없다.
- 슬롯 **사이**로 탭을 끌어 옮길 수 없다(계약 §1.E 범위 외) — 슬롯마다 pane 트리가 따로이고, 탭
  `DndContext` 도 슬롯 안에 갇혀 있다.
- 슬롯별 탐색 패널 **폭**은 영속되지 않는다(폭은 원래 뷰 로컬 — `window-chrome.md` §6.2).
- `ShellSlotId` 는 세션 안의 주소이지 영속 핸들이 아니다. 구버전 세션은 첫 슬롯 변경 전까지 부팅마다
  새 id 를 받으므로, 낡은 id 로 온 포커스 요청은 첫 슬롯으로 물러난다(`resolveShellSlotFocus`).
