# 기능 — 앱 레이아웃 셸 · 멀티 프로젝트

> FR-A. 앱 최상위 구조와 프로젝트 수명주기. 탭·스플릿은 `tabs.md`, 탐색 사이드바는 `explorer-sidebar.md`.

## 1. 레이아웃 구조

```
윈도우
└── 앱 사이드바(세로, 고정폭 48px 급) | content
    content
    └── 탐색 사이드바(접기 가능, 리사이즈 가능) | 에디터 영역(탭 바 + pane 트리)
```

- 앱 사이드바 = VSCode Activity Bar 위치에 **프로젝트 목록**을 두는 TAIDE 고유 구조.
- 탐색 사이드바(explorer/search/git)와 에디터 영역은 **활성 프로젝트의 것**만 렌더된다.
  비활성 프로젝트의 view 는 unmount 하되, Rust 상태(pty·watcher·git 캐시)는 유지된다.
- 탐색 사이드바 토글: `⌘B` / `Ctrl+B` (VSCode 동일). 폭은 드래그 리사이즈, 프로젝트별로 저장.

## 2. 앱 사이드바

### 2.1 구성 (위→아래)

1. 프로젝트 아이콘 목록 (세션의 프로젝트 순서)
2. `+` 프로젝트 열기 버튼 — OS 폴더 선택 다이얼로그
3. (하단 고정) 설정 버튼 — 설정 탭을 활성 프로젝트에 연다

### 2.2 프로젝트 아이콘 (FR-A4)

- 기본 표시: 프로젝트 이름 이니셜(1~2자) 또는 폴더 아이콘. 활성 프로젝트는 accent 인디케이터
  (`appSidebar.itemActive`).
- **focus 된 content 타입에 따라 아이콘이 바뀐다**: 해당 프로젝트의 활성 pane 의 활성 탭 기준.

| focus 타입 | 아이콘 |
|-----------|--------|
| 파일뷰(에디터) | 문서 아이콘 |
| 터미널 | 터미널 아이콘 |
| 설정 | 톱니 아이콘 |
| diff 뷰 | diff 아이콘 |
| code agent 실행 중 | 에이전트 아이콘 (터미널에서 에이전트 감지 시 — `agent-integration.md`) |

- 에이전트 상태는 focus 와 무관하게 **오버레이 배지**(`appSidebar.iconAgentRunning` 점)로도 표시한다
  — 백그라운드 프로젝트에서 에이전트가 돌고 있음을 항상 인지 가능해야 한다.
- 우선순위: 에이전트 실행 중이면 에이전트 아이콘 > 그 외 focus 타입 아이콘.
- hover 툴팁: 프로젝트 이름 + 루트 경로 (+ 에이전트 실행 중이면 에이전트 이름).

### 2.3 프로젝트 context menu

- 닫기(Close Project) — pty·LSP 등 실행 중 자원이 있으면 확인 다이얼로그
- Finder(파일 관리자)에서 열기 / 경로 복사
- 사이드바 내 순서 변경은 DND(세로 sortable) — 순서는 세션에 저장

## 3. 프로젝트 수명주기

| 단계 | 동작 |
|------|------|
| 열기 | 폴더 선택 → Rust `project_open` → ProjectId 발급(기존 열림 이력 있으면 기존 id 재사용) → capability 자동 부착(`.git` 감지 → Git, 파일 감지 → LSP lazy) → 기본 레이아웃 생성(파일뷰 1 + 터미널 1, `tabs.md` §2) → `project:opened` 이벤트 |
| 활성화 | `project_activate` → 세션의 active 갱신 → view 는 해당 프로젝트 레이아웃으로 스왑 |
| 닫기 | `project_close` → capability detach(pty kill, LSP shutdown, watcher stop — 대칭 해제) → 레이아웃·버퍼 영속화 후 메모리 해제 → `project:closed` |
| 재시작 복원 | `session.json` 의 프로젝트 전량 재-open (레이아웃은 `layout.json` 복원). root 부재 시 사용자에게 재연결/제거 선택 |

- 동일 폴더 중복 열기는 금지 — 기존 프로젝트를 활성화한다.
- 하위/상위 폴더 관계의 프로젝트는 각각 독립 프로젝트로 허용한다(watcher 중복 비용은 감수, 문서화).

## 4. IPC (상세: `docs/ipc-contract.md`)

- query: `project_list`, `project_get(projectId)`
- mutation: `project_open(path)`, `project_close(projectId)`, `project_activate(projectId)`,
  `project_reorder(ids)`
- event: `project:opened`, `project:closed`, `project:activated`, `project:list-changed`,
  `project:focus-kind-changed(projectId, kind)` (FR-A4 아이콘용 — layout 도메인이 발행),
  `agent:state-changed(projectId, agents[])` (`agent-integration.md`)

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
