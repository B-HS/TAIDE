# 기능 — 탭 · 스플릿 뷰

> FR-B. 탭 바, DND 재정렬, content 드래그 스플릿, 복원. 데이터 구조는 `docs/data-model.md` §3(PaneNode 트리).
> VSCode 참조 동작 근거: `docs/research/vscode-behaviors.md` §10.

## 1. 모델

- 에디터 영역은 **pane 트리**다: `Split(dir, children, sizes)` / `Leaf(tabs, active)` (data-model §3).
- 탭 타입(`TabKind`): `File{path}` · `Terminal{sessionId}` · `Settings` · `Welcome` · `Diff{spec}` ·
  `AppFile{target}`(Wave I — 앱 소유 파일. `data-model.md` §8) · `SearchEditor{query}` — 확장 가능 enum.
- 모든 변형(열기·닫기·이동·분할·리사이즈·활성화)은 **mutation → Rust layout 도메인**이 수행하고
  `layout:changed(projectId)` 이벤트로 view 가 갱신된다(ADR-0004). view 는 낙관적 UI 를 쓰지 않는다
  (로컬 IPC 라 지연이 체감되지 않음 — 문제 시 재검토).

## 2. 기본 탭 (FR-B2)

- 새 프로젝트의 초기 레이아웃: Leaf 하나에 `[Welcome, Terminal]` 2탭, 활성 = Welcome
  (`layout::service::default_layout()`). 2026-09-04 정정 — 이전 서술의 `File(웰컴/빈 에디터)` 는
  실제 코드(`TabKind::Welcome`)와 달랐다.
  - Welcome 탭은 `WelcomeContainer`(최근 프로젝트·폴더 열기)를 렌더한다. 실파일을 열어도 자동으로
    대체되지 않는 **독립 탭**이며, 닫은 뒤에는 `⌘⇧P` 의 `view.welcome` 커맨드나 탭 바 여백 메뉴로
    다시 연다. 탭 제목은 로케일이 아니라 리터럴 `Welcome` 이다(`command-palette.md` §2.1).
  - 탭이 하나도 없는 메인 창의 에디터 영역은 같은 Welcome 화면을 **탭 없이** 렌더한다
    (`layout-shell.md` §1.1, 설정 `welcomeOnEmptyEditor`).
- 탭 생성 경로: 파일 트리 클릭(File), 터미널 새로 열기(`⌃⇧\``/메뉴), 설정 버튼(Settings),
  git changes 클릭(Diff), 터미널 파일 링크 cmd+click(File — `terminal.md`), CLI/에이전트 요청(File).

## 3. 탭 바 UI · 동작

- VSCode 규칙 채택 (research §10):
  - **preview 탭**: 파일 트리 단일 클릭 = preview(제목 이탤릭, `previewTabId` 는 Leaf 당 최대 1개 —
    다음 단일 클릭이 같은 탭을 재사용). 더블 클릭·편집 시작·pin 시 일반 탭으로 승격.
  - **pin**: 고정 탭은 좌측 정렬 유지, 닫기 버튼 대신 pin 아이콘, `⌘W` 로 닫히지 않음(경고 후 유지).
    d-51 F4 에서 실제로 그렇게 구현됐다 — 그전에는 그 자리 버튼의 라벨만 "고정 해제" 이고 동작은
    닫기였다. 지금은 고정 탭에서 그 버튼이 `onTogglePin`(고정 해제), 휠 클릭은 무시,
    `⌘W` 는 `tab.pinnedCloseBlocked` 경고 후 유지(`tab-item.tsx` · `editor-area.tsx`). context menu 의
    **Close 는 그대로 닫는다** — 명시적 메뉴 선택은 실수로 발동하는 제스처가 아니라 탈출구다.
  - **dirty dot**: 미저장 파일 탭은 닫기 버튼 자리에 점 표시(`tabBar.dirtyDot`).
  - 탭 클릭 = 활성화, 휠 클릭(middle) = 닫기, 우클릭 context menu → §3.1.
- 탭 폭 정책: 내용 기반 폭 + 최대폭 말줄임. 넘치면 탭 바 가로 스크롤(휠 지원) — VSCode 동일.
- 같은 파일 재열기: 같은 Leaf 에 이미 있으면 그 탭 활성화. 다른 Leaf 에는 명시적 분할 이동으로만 중복 허용
  (동일 파일 다중 뷰 — Monaco 모델 공유, `editor.md` §모델 관리).


### 3.1 탭 context menu (Phase 7.5 확정 — 사용자 지정 목록)

**네이티브 OS 메뉴를 쓰지 않는다.** `shared/ui/context-menu`(Radix) 기반 view component 로만 만들어
macOS/Windows/Linux 에서 동일하게 보이게 한다(acknowledge §3.1).

| 그룹 | 항목 | 비고 |
|------|------|------|
| 닫기 | Close · Close Others · Close to the Right · **Close Saved** · Close All | pinned 탭은 제외. Close Saved 는 dirty 아닌 탭만 |
| 고정 | Pin/Unpin · Keep Open | Keep Open 은 preview 탭에서만 — 고정 승격 |
| 복사 | Copy Path · Copy Relative Path | - |
| 탐색·열기 | Reveal in Finder · Reveal in Explorer View · **이름 바꾸기** · Open Changes · File History · Reopen Editor With… | Reopen With 는 editor/preview 전환(`preview.md`). 이름 바꾸기는 아래 설명 |
| 분할 | Split → Up · Down · Left · Right | `layout_split` |
| 창 | Move into New Window · Move back to Main Window(보조 창에서만) · Move to Window N(다른 열린 보조 창마다) | Wave I 로 완전 구현(§4.4). **Copy into New Window 는 미구현**(backlog) — 동일 탭을 두 창이 공유하는 동기화 설계가 범위 밖 |

> 초안에 있던 Copy Remote File URL(From…)·Open on Remote (Web)·Add File to Agent Thread·
> Find File References·Split in Group 은 **구현하지 않았다** — 필요가 생기면 backlog 로.

- 조건에 맞지 않는 항목(git 없음·에이전트 없음·File 탭 아님)은 **숨긴다**(비활성 대신).
- `when` 판정은 커맨드 레지스트리(`command-palette.md` §2)와 같은 컨텍스트를 공유한다.

**이름 바꾸기 (2026-09-04, file 탭 한정)** — 탭 바에는 인라인 편집기가 없다. 대신 앱에 하나뿐인
rename UI(탐색기 트리의 인라인 편집기 — `use-explorer-entry-crud.ts`)에 위임한다: 메뉴 선택 →
`shared/lib/bridge/explorer-rename-bridge.ts` 발행 → ① `app-shell` 이 접힌 사이드바를 펼치고
② `explorer-panel` 이 files 뷰로 전환한 뒤 ③ `explorer-container` 가 경로를 reveal·선택하고 그 행에서
`startRename` 을 시작한다. "Reveal in Explorer View"(`explorer-reveal-bridge`)와 같은 3단 구조이며,
검증·충돌 규칙·`layout_apply_path_change` 추종(§7.1)을 탐색기 경로 하나로 유지하기 위한 설계다.
reveal 응답의 트리 페이지에서 대상 행을 찾으므로, 아직 펼쳐지지 않은 디렉토리 안의 파일도 동작한다.

### 3.2 탭 바 여백 메뉴 (2026-09-04)

탭이 아닌 **탭 바의 빈 공간**을 우클릭하면 pane 단위 메뉴가 열린다. 트리거는 두 곳이다 — 탭 뒤의
filler(더블클릭 = 새 파일과 같은 요소)와 스크롤 컨테이너 밖의 우측 `+` 액션 영역. 탭 바 전체를 하나로
감싸지 않는 이유는 Radix `ContextMenuTrigger` 가 `contextmenu` 를 `preventDefault` 만 하고 버블을
막지 않아, 탭 위 우클릭에서 탭 메뉴와 여백 메뉴가 **동시에** 열리기 때문이다. 두 트리거 모두
`asChild` 로 기존 div 에 합성되므로 dnd 드롭 타깃(filler)의 ref 가 유지된다.

| 항목 | 실행 | 표시 조건 |
|------|------|-----------|
| 새 파일 | `layout_open_untitled(target: 우클릭한 pane)` | 항상 |
| 새 터미널 | `layout_open_tab(terminal, target: 우클릭한 pane)` | 항상 |
| 닫은 탭 다시 열기 | `layout_reopen_closed` | `layout.closedTabs` 가 있을 때 (untitled 탭은 스택에 쌓이지 않는다) |
| 저장된 탭 닫기 | 탭 메뉴와 같은 `handleCloseSaved` | 탭이 1개 이상 |
| 모든 탭 닫기 | 탭 메뉴와 같은 `handleCloseAll` | 탭이 1개 이상 |
| 분할 ▸ 위·아래·왼쪽·오른쪽 | `layout_split(활성 탭)` — 탭 메뉴 Split 과 같은 "활성 탭 이동" | **활성 탭이 있을 때만** (`layout_split` 이 tabId 를 요구 → 탭 0 이면 NotFound) |
| Welcome 열기 | `layout_open_tab(welcome)` — 같은 kind 는 Rust 가 dedupe | 항상 |

- 표시 조건 판정은 순수 빌더 `features/tab/tab-bar-menu-items.ts` 의 `buildTabBarMenuItems` 가
  단독으로 갖는다(`bun test` 로 검증). 우측 `+` 드롭다운도 같은 빌더를 `surface: 'addMenu'` 로 소비해
  생성 항목 2개만 노출한다 — 파괴적 항목은 `+` 버튼에 올리지 않는다.
- 조건 불충족 항목은 탭 메뉴와 같은 정책으로 **숨긴다**(§3.1). 단축키 힌트(`ContextMenuShortcut`)는
  앱 전체 관례가 아직 없어 도입하지 않았다.
- 신규 IPC 는 0건이다. 모든 항목이 기존 mutation 을 재사용한다.
\n## 4. DND (FR-B3·B5)

dnd-kit 사용(구현 세부: `docs/research/react-frontend-stack.md`). 드래그 소스는 탭, 드롭 대상은 두 종류다.

### 4.1 탭 바 내 재정렬

- 가로 sortable. 드래그 중 실시간 자리 양보 미리보기(기존 컨벤션과 동일 동작).
- pinned 탭은 pinned 구역(좌측) 안에서만 재정렬 가능.
- 다른 Leaf 의 탭 바로 드롭하면 그 Leaf 로 **이동**(탭 소속 변경, 대상 위치에 삽입).

### 4.2 content 영역으로 드래그 → 스플릿 (FR-B5)

- 탭을 에디터 content 위로 끌면 **5분할 드롭 존**이 하이라이트된다(`tabBar.dropTarget` 오버레이):
  - 중앙: 해당 Leaf 로 탭 이동
  - 좌/우: 세로 분할(가로 배치) 후 새 Leaf 에 탭 배치
  - 상/하: 가로 분할(세로 배치) 후 새 Leaf 에 탭 배치
- 존 판정: 포인터 위치가 pane 사각형의 중앙 50% 면 중앙, 가장자리 25% 대역이면 해당 방향.
- 드래그 중 표시는 view 로컬 상태(zustand 불필요 — 컴포넌트 로컬), **드롭 확정 시에만** mutation 발행.

### 4.3 트리 변형 규칙

- 분할: 대상 Leaf 가 부모 Split 과 같은 방향이면 부모 children 에 형제로 삽입,
  다르면 대상 Leaf 를 새 `Split` 으로 감싼다. 새 pane 크기는 대상의 1/2 분배.
- 탭 이동으로 Leaf 가 비면 그 Leaf 를 제거하고 트리를 정규화한다:
  자식 1개짜리 Split 은 자식으로 치환, 같은 방향 중첩 Split 은 평탄화.
- 마지막 남은 Leaf 의 마지막 탭을 닫으면 빈 placeholder Leaf 하나를 유지한다(영역 소멸 금지).


### 4.4 새 창 규칙 (Phase 7.5 확정, Wave I 로 멀티 윈도우 완전 구현)

사용자가 "새창의 기준이 모호하다"고 지적해 아래로 확정했다(acknowledge §1.1).

| 조작 | 결과 |
|------|------|
| 파일 트리 단일 클릭 | preview 탭 (기존 §3) |
| 더블 클릭 · 편집 시작 | 고정 탭 승격 (기존 §3) |
| **`⌘`(Ctrl) + 클릭** | **옆 pane 으로 분할해서 열기** |
| context menu → Move into New Window | **별도 OS 창(보조 편집 창)** |
| context menu → Move back to Main Window / Move to Window N | 보조 창 ↔ main 창, 보조 창 ↔ 보조 창 간 탭 이동 |

**파일 클릭만으로는 새 OS 창이 절대 열리지 않는다** — 이 표의 세 "창" 액션으로만 연다는 원칙은
그대로다. 다만 멀티윈도우 자체는 더 이상 "추후" 항목이 아니다: **Wave I
(`docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md`)가 완전 구현했다** — 창 생성·탭
창간 이동·창별 닫기(main 복귀)·다중 창의 LSP/터미널 세션 공유까지 전부 동작한다. 모델·엣지 케이스
정본은 `layout-shell.md` §7, 크롬은 `window-chrome.md` §5. `Copy into New Window`(같은 탭을 두 창이
동시에 공유)만 이번 범위에서 제외됐다(backlog).
\n## 5. 스플릿 리사이즈

- react-resizable-panels 로 Split 의 `sizes` 렌더·드래그. 드래그 종료 시 `layout_resize` mutation 으로
  Rust 에 반영(드래그 중 매 프레임 IPC 금지 — 종료 시 1회).
- 최소 pane 크기(예: 120px) 보장. 윈도우 리사이즈는 비율 유지.
- **Separator 두께는 설정값**(Phase 7.5, 사용자 지적 12번). `settings` 의 `resizer_thickness`
  (기본 1px, 히트 영역은 그보다 넓게 — react-resizable-panels 의 `resizeTargetMinimumSize`).
  시각적 두께와 잡는 영역을 분리해야 "굵어 보이지 않으면서 잡기 쉬운" 상태가 된다.

## 6. 키보드 (VSCode 기준 — research §8)

| 동작 | macOS | Win/Linux |
|------|-------|-----------|
| 탭 닫기 | `⌘W` | `Ctrl+W` |
| 분할 | `⌘\` | `Ctrl+\` |
| pane 포커스 순환 | `⌘1`·`⌘2`… (그룹 n 포커스) | `Ctrl+1`… |
| 탭 순환 | `⌃Tab` / `⌃⇧Tab` | `Ctrl+Tab` / `Ctrl+Shift+Tab` |
| 닫은 탭 재열기 | `⇧⌘T` | `Ctrl+Shift+T` |

- `⌘W` 등 WebView/OS 기본 동작과 충돌하는 키는 캡처 단계 preventDefault + Tauri 메뉴 accelerator 로 선점
  (research 함정 절). 키 처리 계층 규칙은 `editor.md` §키바인딩(에디터 내/외 구분)과 함께 정의.
- 닫은 탭 재열기를 위해 Rust layout 도메인이 프로젝트별 "최근 닫은 탭" 스택(상한 20)을 유지한다.

## 7. 복원 (FR-B4)

- 트리·탭 순서·활성 탭·pinned·preview 여부·pane sizes 전부 `layout.json` 으로 영속(debounce, data-model §4).
- 복원 시 검증: File 탭은 경로 존재 확인(부재 시 탭 제거 + 알림 1회), Terminal 탭은 동일 cwd·셸로
  새 세션 생성(`terminal.md`), Diff 탭은 대상 유효성 확인 후 무효면 제거.
- view reload 는 Rust 트리를 그대로 재조회 — 검증 불필요(런타임 상태 그대로).

## 7.1 탐색기 개명·삭제 추종 (d-50 S8, 2026-08-29)

> 감사 §4-B A3. 이전에는 탭의 경로를 갱신하는 코드가 아예 없어, 개명한 파일의 탭이 사라진 경로를
> 계속 가리켰다(⌘S 가 `write_atomic` 의 `create_dir_all` 로 옛 폴더를 되살려 파일이 조용히 갈라졌다).

- `file_rename` 성공 직후 `layout_apply_path_change(projectId, {kind:'renamed', from, to})` 가
  **열린 File 탭의 경로·제목**을 새 경로로 치환한다. 폴더 개명이면 그 아래 **모든 창·페인**의 탭이
  대상이고, 닫은 탭 스택(재열기)도 같이 옮긴다. 경로 비교는 성분 단위라 `src` 개명이 `src-old` 를
  건드리지 않는다. Diff·ClaudeDiff 탭은 비교 뷰라 제외한다(`ipc-contract.md` d-50 S8 절).
- 프론트는 같은 왕복에서 그 경로가 들고 있던 상태를 전부 새 경로로 옮긴다 —
  monaco 모델(버퍼·뷰 상태·부착 중인 에디터), 핫엑시트 미러, `FILE.CONTENT` 캐시,
  "reopen with" 오버라이드. **미저장 편집은 미러 경유로 살아남는다**: `EditorPane` 은 경로가 바뀌면
  자기 dirty 를 초기화하므로, 새 경로에 미러를 먼저 써 두면 그 페인의 미러 복원이 draft 를 되살린다.
- monaco 모델은 uri 를 바꿀 수 없어 **새 모델 생성 + 옛 모델 폐기**로 옮긴다 → 그 문서의 **undo
  스택은 초기화**된다(§8 의 닫기 dispose 와 같은 트레이드오프).
- 열린 탭이 하나도 없어도 **닫은 탭 스택은 따라간다** — 그 경우 프론트에 보고할 것이 없어
  `moved`/`closedPaths` 가 비지만, 백엔드는 바뀐 레이아웃을 저장한다(revision·`layout:changed` 는
  없다 — 화면에 보이는 것이 바뀌지 않았으므로). ⇧⌘T 가 사라진 경로를 여는 것을 막기 위함이다.
- `from`/`to`/`path` 는 **그 프로젝트 루트 하위**여야 한다(성분 단위 검사). 원격에서도 부를 수 있는
  커맨드라, 가드가 없으면 `{kind:'deleted', path:'/'}` 한 번으로 프로젝트의 모든 파일 탭이 닫히고 그
  경로들의 핫엑시트 미러(= 미저장 초안)가 함께 지워진다.
- 다른 OS 창에서만 편집 중이던 파일을 개명하면 그 창의 화면 버퍼는 새 경로의 디스크 내용으로
  리셋된다(창마다 모델 레지스트리·쿼리 캐시가 별개). 초안 자체는 `useRenameEntry` 가 **개명 전에**
  미러 목록을 재조회해 새 경로 미러로 옮기므로 재시작 복구는 가능하다. 창 간 라이브 이관은 백로그.
- `file_delete` 성공 직후에는 `{kind:'deleted', path}` 로 **그 경로(및 하위)의 File 탭을 닫는다**.
  손으로 닫은 것과 동일 경로(`close_tab`)라 닫은 탭 스택·활성 탭 인계도 같다. 그 부작용으로 삭제로
  닫힌 탭도 닫은 탭 스택에 쌓여 ⇧⌘T 가 이미 없는 파일을 연다(개명 축과의 비대칭 — 백로그).

## 8. 수명주기 · 누수 방지

- 탭 닫기 시:
  - File: 에디터 위젯 unmount → viewState 저장 → **어느 창·페인에도 그 파일 탭이 남아 있지 않으면
    monaco 모델을 `disposeModel`** (감사 §1-6, d-50 S8). 이전에는 untitled 탭만 폐기해 세션 중 연
    모든 파일의 본문이 앱 종료까지 메모리에 남았다. 다시 열면 새 모델이므로 **undo 스택은
    보존되지 않는다** — 메모리 누수 제거를 우선한 결정(계약 §3 S8).
    dirty 면 저장/버리기/취소 다이얼로그.
  - Terminal: 실행 중 포그라운드 프로세스가 있으면 확인 다이얼로그 → pty kill(Rust). xterm dispose.
  - Diff/Settings: 위젯 unmount 만.
- 탭 바 위젯 구독은 `layout:changed` 단일 이벤트 + layout query 재조회로 한정.
- DND 센서·리스너는 dnd-kit context 가 관리 — 전역 document 리스너 직접 부착 금지.
