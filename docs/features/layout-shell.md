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
  `agent:state-changed(projectId, agents[])` (`agent-integration.md`)
  (`project:focus-kind-changed(projectId, kind)` 는 X-A 배치(2026-08-19)에서 제거됐다 — 소비자가
  0 이었고, 레이아웃 변이마다 무조건 발행돼 트래픽만 2배였다. §2.2 의 focus 종류는 이미
  `layout:changed` 가 무효화하는 `LAYOUT.DETAIL` 캐시(`ProjectLayout.focused_pane` + 활성 탭의
  `kind`)에서 프론트가 그대로 유도할 수 있어, 이 파생값을 Rust 가 별도 이벤트로 다시 계산해
  내보낼 필요가 없었다 — FR-A4 아이콘은 이 캐시 기반 유도로 구현하는 것이 정본이다)

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
- 보조 창은 **사이드바·상태바가 없는 에디터 전용 크롬**이다(`AuxiliaryWindowShell` — 탭바+pane
  트리만, `window-chrome.md` §5). 자기 `(projectId, windowSlot)` 에 **고정**되어 렌더링하고, main
  창의 활성 프로젝트가 바뀌어도(`ProjectActivated`) 영향받지 않는다.
- `AppShell`/`CommandPalette`/`KeybindingsEditor`/`TaskRunnerDialog` 는 main 창에만 마운트된다 —
  넷 다 내부적으로 "전역 활성 프로젝트" 세션을 읽어 동작하는데, 보조 창은 정의상 그 세션과 무관한
  고정 프로젝트를 보여줘야 하기 때문이다(`app.tsx`). 보조 창에서 "Move back to Main Window"/"Move to
  Window N" 은 탭 컨텍스트 메뉴로만 실행 가능하다(팔레트는 보조 창에 없다 — 알려진 범위 제한, 아래
  "알려진 제한" 참조).

### 7.2 창을 여는/이동하는 경로

- 탭 컨텍스트 메뉴(`tabs.md` §3.1) · 커맨드 팔레트(main 창에서만) 에 3가지 액션이 있다:
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

- 팔레트가 보조 창에 마운트되지 않으므로(§7.1), "Move back to Main Window"/"Move to Window N" 팔레트
  커맨드는 오늘 시점엔 탭 컨텍스트 메뉴로만 도달 가능하다.
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
