> 조사 워크플로(opus·high, 7주제 병렬, 주제당 10분 상한) 산출. 출처 등급: 소스 근거 / [커뮤니티] / [미확인].

# 2026-09-15 배치5 조사 종합

## 0. 종합 — 항목별 결론·결정 지점

### 항목 1 — 탭/그룹 이동 단축키 확장 (§T1)
- 현행: `APP_KEYMAP` 22개 엔트리 중 탭 관련은 `tab-cycle-next/prev`(⌃Tab/⌃⇧Tab, MRU 아님)뿐. 이전/다음 에디터(⌥⌘←/→)·그룹 포커스 이동·⌘1~9·전체 닫기·탭 좌우 이동은 전부 없음.
- 가능성 판정: 가능. ⌥⌘←/→ 는 monaco 액션과 무충돌(monaco-actions.ts 확인). 단 ⌥⌘↑/↓ 는 monaco 멀티커서(insertCursorAbove/Below)와 정면 충돌해 `when` 가드 필수.
- 추천 방향: A1(⌃Tab 유지 + ⌥⌘←/→ 신규 추가) + ⌘K chord 네임스페이스에 그룹 포커스·탭 좌우 이동·전체 닫기를 형제로 추가(엔진이 복수 chord 이미 지원).
- 규모: 탭 이동 S / 그룹 포커스·⌘1~9 M.
- 결정 지점: 탭 이동 키 정책(A1/A2), ⌘1~9 의미(VS Code 정합 그룹 포커스 vs 요청대로 탭 N번), 구현 방식(APP_KEYMAP 9엔트리 vs 전용 리스너).

### 항목 2 — 파일트리 단축키 확장 (§T1)
- 현행: 트리 키 처리는 `APP_KEYMAP` 밖 컨테이너 로컬 `onKeyDown` 한 곳(↓↑←→ Enter + 타이프어헤드)뿐이고, 컨텍스트 메뉴 항목(잘라내기/복사/붙여넣기/이름바꾸기/삭제 등)은 단축키 표기가 없다.
- 가능성 판정: 가능. `when` 엔진은 있으나 탐색기 전용 컨텍스트 getter가 없고(getter 화이트리스트 2개뿐), 키바인딩 에디터의 `when` 필드 부재로 `APP_KEYMAP`+`when` 경로를 쓰면 충돌 오탐이 발생하는 리스크가 확인됨.
- 추천 방향: C1(file-tree 로컬 `onKeyDown` 확장) — 즉시·무충돌, 리바인딩은 포기.
- 규모: S~M.
- 결정 지점: 탐색기 Enter 의미 변경(VS Code 정합 vs 현행 유지, 유일하게 기존 동작이 바뀌는 항목), C1/C2/C3 배치, 카탈로그 `when` 누락 리스크 선행 수정 여부.

### 항목 3 — 프로젝트 묶음(그룹)·한 화면 다중 프로젝트 (§T2)
- 현행: "활성 프로젝트" 정본은 Rust 1필드뿐이지만 `AppState` 는 이미 프로젝트 N개를 동시 보유하고, 패널 위젯 전부 `projectId` prop 구동이라 멀티 인스턴스화가 이미 가능. 보조 창이 프로젝트별 독립 셸을 부분 구현(explorer/git/search/팔레트 미탑재).
- 가능성 판정: 가능. 그룹/워크스페이스 개념은 현재 0.
- 추천 방향: 2단계 — 1단계(S) 보조 창에 explorer/git/search/팔레트 추가로 완성, 2단계(L) 셸 슬롯 분할로 한 창 안 좌우 배치. 그룹 모델은 별도 `ProjectGroup` 엔티티(G2) 추천.
- 규모: 1단계 S / 2단계 L.
- 결정 지점: "한 화면"의 정의(OS 창 분할 vs 보조 창), 그룹 모델(G1/G2), 그룹 열기 정책(전부 직렬/첫 멤버만+백그라운드 큐).

### 항목 4 — OS 알림 내용 강화 (§T3)
- 현행: 알림 텍스트는 전적으로 FE 소유(title/body 2필드뿐). `agentCompleted` 의 body 는 `agent.name` 한 단어(claude/codex/gemini 중 하나)뿐이고 프로젝트명·경과시간·세션 정보는 미사용.
- 가능성 판정: 프로젝트명·경과시간·완료/입력대기 구분은 새 IPC 없이 가능. 클릭 시 프로젝트 활성화·창 포커스는 플러그인이 콜백을 지원하지 않아 **불가**.
- 추천 방향: FE 문자열 조립 강화(프로젝트명+소요시간) + 완료/입력대기 제목 분리.
- 규모: S~M.
- 결정 지점: 포맷(제목=사건+프로젝트 vs 제목=프로젝트), 프로젝트명 항상 표기 vs 2개 이상일 때만, awaitingInput 분리 여부, 클릭 라우팅 포기 여부.

### 항목 5 — 에이전트 감지 다각화 (opencode·codex·pi) (§T4)
- 현행: 감지 대상은 `KNOWN_AGENT_NAMES` 3종(claude/codex/gemini) 한 줄. 활동 판정 4축 중 3개(인밴드 훅·다이얼로그 시그니처·타이틀 글리프)가 Claude 하드코딩. 이 기기에 opencode·codex 설치 확인(각각 이벤트/훅 시스템 실재 확인), pi·gemini 는 미설치.
- 가능성 판정: 신원 추가는 3파일 5줄로 즉시 가능. 활동 신호(AwaitingInput)까지 가려면 다이얼로그 시그니처 실측 또는 인밴드 훅 이식 필요.
- 추천 방향: A(신원만)+E(타이틀 파서 에이전트화) 우선, 이후 B(opencode 다이얼로그 시그니처 실측)·C(인밴드 OSC 777 확장, codex 포함).
- 규모: S(신원)~L(인밴드 훅 전체 확장).
- 결정 지점: 지원 범위(opencode만/+codex/+pi/전부), pi·gemini 설치 승인 여부, 구현 깊이.

### 항목 6 — Dock 우클릭 Recent (§T5)
- 현행: Tauri 2.11.5·muda 0.19.3·tao 0.35.3 어디에도 macOS Dock 메뉴 설정 API가 없다. `applicationDockMenu:` 는 AppKit 델리게이트 프로토콜 메서드로만 존재하고 tao가 이를 등록하지 않으며 앱 코드에 확장 지점을 주지 않는다.
- 가능성 판정: **정공법 없음.** 우회(objc2 런타임 클래스 패치)는 unsafe·컨벤션(§6.2 HACK 금지) 위반.
- 추천 방향: Dock 메뉴는 포기하고 메뉴바 `File > Open Recent` 신설로 대체.
- 규모: M.
- 결정 지점: Dock 처리(포기/우회/업스트림 PR), Open Recent 상한(10 권장), Clear Recent 포함 여부.

### 항목 7 — 퀵오픈(⌘P) 간헐적 파일 못 찾음/안 열림 (§T6)
- 현행: 결과 200건 상한 + MRU 없는 점수단독 정렬로 커트라인이 walk 순서에 의존해 간헐적으로 흔들림(유력). 유니코드 정규화 미적용으로 NFD 한글 파일명은 매칭 불가. `IGNORED_DIR_NAMES`(dist/build/target/.venv 등)는 gitignore 설정과 무관하게 항상 적용돼 그 안 파일은 절대 못 엶. `focused_pane` 소실 시 `NotFound("pane not found")` 가 파일 소실로 오분류돼 인덱스까지 무효화(백로그 등재된 미수정 결함).
- 가능성 판정: 원인 다수 확정(f 랭킹상한, h-3 무시디렉토리, h-2 pane버그가 유력). 실제 사용자 사례가 어느 것인지는 재현정보 없이는 미확정.
- 추천 방향: 즉시 착수(옵션1 랭킹 타이브레이크 + 옵션7 조용한 return 제거)로 간헐성 우선 완화, 병행해 재현정보 3가지 수집.
- 규모: S(옵션1,7)~M(NFC 정규화, pane 버그 분리).
- 결정 지점: 즉시착수 범위, `IGNORED_DIR_NAMES` 정책, 심링크 포함 여부, 보조 창 퀵오픈 지원 여부.

### 항목 8 — Welcome 화면에 "터미널 열기" 추가 (§T7)
- 현행: 터미널 열기 실제 구현은 `command-palette.tsx` 내부의 export 안 된 로컬 클로저뿐. Welcome 에는 버튼이 없다.
- 가능성 판정: 가능. 비활성 조건은 기존 "파일 열기" 버튼과 완전히 동일(`projectId === null`).
- 추천 방향: A1 — `useOpenTerminalTab` 을 `entities/layout` 로 추출해 팔레트·Welcome 이 공용 소비(선례: `useOpenAppFileTab`).
- 규모: S.
- 결정 지점: 재사용 방식(추출/복제/브리지), 라벨 키(`keymap.newTerminal` 재사용 추천), 프로젝트 0개 시 안내 문구.

### 항목 9 — 검색 패널 실시간 검색 + 결과 아이콘 (§T7)
- 현행: 검색은 Enter 전용, 이력은 `run()` 진입마다 무조건 push(디바운스 도입 시 설정 파일 쓰기 폭주 위험), macOS WKWebView는 IME composition 이벤트 자체를 쏘지 않아 조합 중 오검색 위험. 결과 목록 파일 아이콘은 `FileGroupHeader` 가 아이콘 레지스트리를 호출하지 않고 lucide `File` 을 하드코딩해서 탐색기와 다르게 보인다(문제 패널도 동일 증상, git 패널은 파일 아이콘 자체가 없음).
- 가능성 판정: 아이콘은 원인이 한 줄로 확정되어 즉시 수정 가능. 실시간 검색은 가능하나 이력·IME·정규식 토스트 대응이 필수 선행.
- 추천 방향: 아이콘은 `FileGroupHeader` 를 `FileTypeIcon` 으로 교체(즉시). 실시간 검색은 설정 토글(`searchOnType`, VS Code 동일 300ms trailing 디바운스) + 이력은 Enter/blur 만.
- 규모: 아이콘 S / 실시간 검색 M.
- 결정 지점: `searchOnType` 설정 신설 여부, 디바운스 값, IME 조합 처리 방식, 아이콘 수정 범위(검색+문제만 vs git 패널까지).

### 항목 10 — 사이드바 + 버튼 메뉴(경로로 열기/Finder로 열기) (§T5)
- 현행: 사이드바 `+` 버튼은 클릭 즉시 Finder 폴더 피커를 여는 단일 동작이고 드롭다운이 아니다. "경로로 열기" 기능은 없다.
- 가능성 판정: 가능. `tab-bar-add-menu.tsx` 에 Tooltip+DropdownMenu 정본 패턴이 이미 있어 그대로 재사용 가능.
- 추천 방향: IconButton을 DropdownMenu로 전환해 "Finder로 열기"(기존 동작)·"경로로 열기…"(신규 다이얼로그) 2항목 제공.
- 규모: S~M.
- 결정 지점: "Finder로 열기"의 정확한 의미(폴더 피커 유지 vs 프로젝트 루트를 Finder에 노출 — 후자는 `system_reveal_path` 게이트 완화 필요), `~` 경로 확장 위치(FE vs Rust).

---

## T1 — 단축키 확장(탭/그룹 이동) · 파일트리 단축키 및 빈공간 더블클릭

## 현행(근거)

### 1) 키맵 엔진 구조

| 파일 | 역할 |
|---|---|
| `src/shared/lib/keymap/keymap.ts:64-129` | `APP_KEYMAP` 정본 배열(22개 엔트리) |
| `src/shared/lib/keymap/keymap.ts:3-26` | `KeymapActionId` 플랫 union(22개) |
| `src/shared/lib/keymap/keymap-dispatch.ts:90-118` | 순수 상태머신 `decideKeymapDispatch`(IME → monaco-deferral → chord pending → chord stage1 → monaco prefix 관찰 → 단일 매칭) |
| `src/shared/hooks/use-global-keymap.ts:55-103` | 유일한 side-effect 지점(`preventDefault`/`stopPropagation`/핸들러 호출) |
| `src/shared/hooks/use-keydown-capture.ts:37-46` | `window.addEventListener('keydown', …, true)` — **capture 단계**, 즉 monaco/xterm보다 항상 먼저 |
| `src/shared/lib/keymap/keymap-when.ts:31-36` | monaco `ContextKeyExpr.deserialize` 로 `when` 평가 |
| `src/shared/lib/keymap/keymap-context.ts:27-30` | **getter 화이트리스트 2개뿐** — `editorTextFocus`(`.monaco-editor`), `terminalFocus`(`.xterm`) |
| `src/shared/lib/keymap/keymap-chord-store.ts` | chord pending / monaco-deferral 크로스 리스너 상태 |
| `src/shared/lib/keymap/keybinding-catalog.ts:38-81` | 커맨드 + 키맵-only 행을 합쳐 키바인딩 에디터 행 생성 |
| `src/shared/lib/command-registry.ts:15-23` | `AppCommand { id, titleKey, categoryKey, keymapId, run, isEnabled }` |
| `src/shared/lib/command-catalog.ts:18-195` | `DEFAULT_COMMANDS` 23개 |
| `src/shared/lib/monaco/monaco-actions.ts` | monaco 0.56 기본 액션 837줄 카탈로그(`defaultBindingLabel` 은 **표시 전용**) |

`useGlobalKeymap` 호출부 7곳: `app-shell.tsx:122`, `editor-area.tsx:242`, `terminal-pane.tsx:167`, `status-bar-content.tsx:131`, `command-palette.tsx:221`, `use-zen-mode.ts:43`, `keybindings-runtime-provider.tsx:50`.

### 2) 현재 APP_KEYMAP 전체 (keymap.ts:64-129)

| id | 기본 바인딩 | when | 핸들러 위치 |
|---|---|---|---|
| quick-open | ⌘P | — | command-palette |
| command-palette | ⇧⌘P | — | command-palette |
| workspace-symbol | ⌘T | — | command-palette |
| close-tab | ⌘W | — | editor-area |
| toggle-sidebar | ⌘B | — | app-shell |
| find / search / search-replace | ⌘F / ⇧⌘F / ⇧⌘H | — | editor-area |
| explorer / git | ⇧⌘E / ⌃⇧G | — | app-shell |
| split | ⌘\ | — | editor-area |
| **tab-cycle-next / prev** | **⌃Tab / ⌃⇧Tab** | — | editor-area:248-249 |
| reopen-closed-tab | ⇧⌘T | — | command-palette |
| save | ⌘S | — | editor-area |
| toggle-terminal / new-terminal | ⌃` / ⌃⇧` | — | editor-area / terminal |
| terminal-jump-to-previous/next-command | ⌘↑ / ⌘↓ | `terminalFocus` | terminal-pane |
| font-size-up / down | ⌘= / ⌘- | — | status-bar |
| open-keybindings-editor | ⌘K ⌘S (chord) | `!terminalFocus` | provider |
| toggle-zen-mode | ⌘K Z (chord) | `!terminalFocus` | use-zen-mode |

### 3) 파일트리 현행

- 키 처리는 **컨테이너 로컬 `onKeyDown`** 한 곳 (`file-tree.tsx:156-203`): ↓ ↑ → ← Enter + 타이프어헤드. `when` 컨텍스트 없음, `APP_KEYMAP` 미참여.
  - `Enter` = 디렉터리면 토글, 파일이면 `onOpenPreview` (`file-tree.tsx:191-199`)
  - 행 클릭 = 선택+토글/프리뷰, 더블클릭(행) = `onOpenPinned` (`file-tree.tsx:132-145`)
- 컨텍스트 메뉴 항목(`file-tree-context-menu.tsx:96-200`) → 핸들러(`explorer-container.tsx:175-201`): 새 파일/폴더, 옆에 열기, Open With, 브라우저, 비교 선택/비교, 파일 히스토리, Finder 표시, 터미널 열기, 폴더 내 찾기, 잘라내기/복사/붙여넣기, 경로/상대경로 복사, 이름 바꾸기, 삭제. **단축키 표기 없음**.
- **빈 공간 판정은 이미 존재**: `handleContainerContextMenu` (`file-tree.tsx:205-223`) 가 `offsetY / FILE_TREE_ROW_HEIGHT_PX` 로 인덱스를 계산해 행 밖이면 `setContextRow(null)` + `onClearSelection()`.
- 초안 행 인프라 존재: `FileTreeDraft { kind, parentDir }` (`file-tree.tsx:17`), `buildDisplayRows` (`file-tree.tsx:72-79`) — `parentDir` 가 rows 에 없으면 **depth 0 / index 0 에 삽입** = 루트 초안이 이미 동작.
- `startDraft(kind)` (`use-explorer-entry-crud.ts:64-71`) 는 인자로 대상 디렉터리를 못 받고 `targetDirFor(selectedRow)` 를 읽는다. `selectedRow` 가 null 이면 `project.root` (`explorer-container.tsx:80-83`).

---

## 핵심 질문별 답

### Q1-a. 있는 것 / 없는 것

| 액션 | VS Code macOS 기본 | TAIDE 현황 | 추가 위치 |
|---|---|---|---|
| 탭 간 이동(MRU) | ⌃Tab / ⌃⇧Tab | **있음** `tab-cycle-next/prev` (단 MRU 아님, 인덱스 순환 `editor-area.tsx:154-164`) | — |
| 이전/다음 에디터 | **⌥⌘← / ⌥⌘→** | **없음** | keymap.ts + command-catalog.ts + 로케일 3 + editor-area 핸들러 |
| 그룹 간 포커스 이동 | ⌘K ⌘← / ⌘K ⌘→ (focusLeft/RightGroup) | **없음** (`useFocusPane` 뮤테이션만 존재 `layout.query.ts:183`, 사용처는 `pane-tab-bar.tsx:92` 뿐) | 동일 + 페인 순서 헬퍼 신설 |
| N번째 그룹 포커스 | ⌘1~9 (`focusFirstEditorGroup` = ⌘1) | **없음**. VS Code 도 ⌘1~9 는 **그룹**이지 탭 N번이 아님 | 9개 엔트리 or 별도 리스너 |
| 전체 닫기 | ⌘K ⌘W | **UI만 있음** (`tab-bar-menu-items.ts:48` `closeAll`, `tab-context-menu.tsx:107`), 키맵 없음 | chord 엔트리(⌘K 네임스페이스에 형제 추가 가능 — `findMatchingChordPrefixEntries` 가 복수 후보 지원) |
| 탭 좌우 이동 | ⌘K ⌘⇧← / ⌘K ⌘⇧→ | **없음**. 단 `useMoveTab({tabId,paneId,index})` 존재(`editor-area.tsx:78`, 드래그 전용) → **새 IPC 불필요** | 동일 |

### Q1-b. ⌥⌘←/→ 추가 시 손대는 파일 (정확 목록)

1. `src/shared/lib/keymap/keymap.ts:3-26` — `KeymapActionId` union 에 `'editor-previous' | 'editor-next'` 추가
2. `src/shared/lib/keymap/keymap.ts:64-129` — `APP_KEYMAP` 엔트리 2개 (`{ key: 'ArrowLeft', mods: ['mod','alt'] }` / `ArrowRight`)
3. `src/shared/lib/command-catalog.ts` — `AppCommand` 2개(`keymapId` + `requestEditorPaneCommand({type:'cycle-tab',direction})` 재사용, `editor-pane-command-bridge.ts:8` 그대로)
4. `src/widgets/editor-area/editor-area.tsx:242-252` — `useGlobalKeymap` 에 2줄
5. `src-tauri/resources/locales/{en,ko,ja}.json` — `keymap.editorPrevious` / `keymap.editorNext` (플랫 키, 예: en.json:417 `"keymap.tabCycleNext"` 옆)
6. `keybinding-catalog.ts` — **수정 불필요**. `buildKeybindingRows` 가 `DEFAULT_COMMANDS` × `APP_KEYMAP` 로 자동 생성(:38-81). 카테고리 라벨이 필요하면 커맨드에 `categoryKey: KEYMAP_CATEGORY.TAB` 만 주면 됨
7. Welcome 단축키 표 — `welcome-container.tsx:31` `WELCOME_KEYMAP_HIGHLIGHT_IDS` 는 **대표 6개 고정**. 넣고 싶을 때만 id 추가(현재 tab-cycle 도 미포함이라 넣지 않는 게 일관)
8. 테스트: `keymap.test.ts`(579줄)에 중복 바인딩 부재 검증이 있는지 확인 필요, `keybinding-catalog.test.ts`(361줄) 행 수 기대값

### Q1-c. Monaco 선점 여부 — **선점하지 않음**

- 디스패치는 `window` **capture** 리스너(`use-keydown-capture.ts:38`)이고 매칭 시 `preventDefault + stopPropagation`(`use-global-keymap.ts:91-98`) 하므로, `when` 없는 APP_KEYMAP 엔트리는 **항상 monaco/xterm보다 우선**한다. monaco-deferral 은 ⌘K 계열 chord prefix 에만 적용(`keymap-dispatch.ts:83-87`).
- **⌥⌘← / ⌥⌘→ 는 `monaco-actions.ts` 에 없음**(⌥⌘ 조합은 `⌥⌘.`:114, `⌥⌘↑`:309, `⌥⌘↓`:321, `⌥⌘⌫`:496, `⌥⌘F`:612, `⌥⌘[`:752, `⌥⌘]`:810 뿐) → **무충돌**.
- 반대로 **⌥⌘↑ / ⌥⌘↓ 는 monaco 의 `insertCursorAbove` / `insertCursorBelow`**(`monaco-actions.ts:305-322`). `when` 없이 전역 등록하면 **멀티커서 기능이 죽는다** → 반드시 `when: '!editorTextFocus'` 를 달거나 다른 키 선택.

### Q2. 파일트리 — 빈 공간 더블클릭 훅 위치

- 훅 지점: `file-tree.tsx:274-284` 의 `parentRef` div (`role='tree'`, `tabIndex={0}`). `onContextMenu={handleContainerContextMenu}` 옆에 `onDoubleClick` 을 추가하고, **:210-213 의 hit-test(3줄)를 그대로 재사용**해 `displayRows[index]` 가 없으면 빈 공간으로 판정.
- 행 더블클릭과의 중복: 행의 `onDoubleClick`(`file-tree.tsx:334`)은 버블링되므로, 컨테이너 핸들러에서 hit-test 로 행이 잡히면 즉시 return 하면 됨(추가 stopPropagation 불필요).
- 초안 행은 **그대로 재사용 가능** — `buildDisplayRows` 가 `parentDir` 미발견 시 depth 0/index 0 에 넣음(`file-tree.tsx:74-78`), `useEffect`(:232-237)가 자동 `scrollToIndex`.
- **주의(함정)**: `onClearSelection()` 후 곧바로 `onNewFile()` 을 부르면 `startDraft` 클로저가 **직전 렌더의 `selectedRow`** 를 읽어 선택된 폴더 하위에 초안이 생긴다. `startDraft(kind, targetDir?)` 로 명시 인자를 추가하는 것이 정석(`use-explorer-entry-crud.ts:64-71`).

### Q3. VS Code macOS 탐색기 기본키 vs TAIDE 충돌

출처: `microsoft/vscode` `fileActions.contribution.ts`(1차 출처, WebFetch).

| 액션 | VS Code mac | when(VS Code) | TAIDE 기존 바인딩 충돌 |
|---|---|---|---|
| 이름 바꾸기 | **Enter** (primary F2) | `filesExplorerFocus && !explorerRoot && writable` | 현재 file-tree 로컬 Enter = 열기/토글(`file-tree.tsx:191`) → **동작 변경**. 전역 충돌 없음 |
| 휴지통 이동 | **⌘⌫** (secondary Delete) | `filesExplorerFocus && moveableToTrash` | monaco `deleteAllLeft` = ⌘⌫ (`monaco-actions.ts:81`) → **에디터 포커스 시 충돌** |
| 영구 삭제 | ⌥⌘⌫ | filesExplorerFocus | monaco `⌥⌘⌫`(:496) 충돌 |
| 미리보기(포커스 유지) | **Space** | `filesExplorerFocus && !folder` | 전역 미바인딩. 단 file-tree 타이프어헤드가 `key.length===1`(:200) 로 Space 를 먹음 → **내부 충돌** |
| 잘라내기/복사/붙여넣기 | ⌘X / ⌘C / ⌘V | filesExplorerFocus | APP_KEYMAP·monaco 카탈로그 모두 없음 → **무충돌**(단 전역 preventDefault 시 다른 곳 복사 방해 주의) |
| Finder 표시 | ⌥⌘R | — | APP_KEYMAP·monaco 없음 → 무충돌 |
| 경로 복사 / 상대경로 복사 | ⌥⌘C / ⇧⌥⌘C | — | 없음 → 무충돌 |
| F2 | 이름 바꾸기(win/linux primary) | — | monaco `editor.action.rename` = F2 (`monaco-actions.ts:510`), `⌘F2`(:125) 도 있음 → 에디터 포커스 시 충돌 |

전역 ⌘N 은 TAIDE 에 없음(탭바 메뉴 `newFile` 은 클릭 전용). ⌘W = `close-tab`(when 없음) — 탐색기 포커스 중에도 탭이 닫히므로 VS Code 와 동일하다.

**`when` 지원 여부**: 엔진은 지원한다(`keymap-when.ts` + `keymap-dispatch.ts:120~`). 단 **컨텍스트 getter 화이트리스트에 `explorer*` 가 없다**(`keymap-context.ts:27-30`, 2개뿐). `filesExplorerFocus` 상당 getter를 추가해야 하며, `isActiveElementWithin('[role="tree"]')` 는 **outline-panel 도 `role='tree'`**(`outline-panel.tsx:115`)이므로 부적합 → 전용 `data-*` 속성 또는 `.bg-explorer-background` 같은 고유 마커 필요.

---

## 설계 옵션

### A. ⌥⌘←/→ 탭 이동 (규모 S)

| 옵션 | 장점 | 단점 |
|---|---|---|
| **A1 (추천) 신규 id 2개 추가**, ⌃Tab 유지 | VS Code 양쪽 관용 모두 제공, 기존 사용자 muscle memory 보존 | 같은 동작에 id 4개 → 키바인딩 에디터 행 중복, 충돌 감지가 서로를 못 봄(키가 달라 무관) |
| A2 기존 `tab-cycle-*` 의 기본 키를 ⌥⌘←/→ 로 교체 | 행 중복 없음 | ⌃Tab 사라짐(기존 override 저장한 사용자에게는 영향 없음) |

영향 파일: Q1-b 목록 그대로. 규모 **S**.

### B. 그룹 포커스/탭 이동 (규모 M)

- `⌘K ⌘←/→`(그룹 포커스), `⌘K ⌘⇧←/→`(탭 좌우 이동), `⌘K ⌘W`(전체 닫기) 모두 **⌘K chord 네임스페이스에 형제로 추가 가능** — 엔진이 복수 형제 chord 를 이미 지원(`keymap.ts:207-212`, `keymap-dispatch.ts:107-117`).
- 신규 인프라 필요: ① `shared/lib/pane-tree.ts` 에 리프 순서 수집 헬퍼(현재 `collectPaneTabs` 만 있고 리프 목록 헬퍼 없음, :28) ② `editor-pane-command-bridge.ts` 에 커맨드 타입 3종 ③ `useFocusPane`/`useMoveTab` 를 `editor-area` 로 끌어오기(둘 다 이미 import 가능).
- **⌘1~9 그룹 포커스는 별도 문제**: `KeymapActionId` 가 플랫 union 이라 파라미터화 불가 → 엔트리 9개 + 로케일 9키 ×3 언어. 규모 **M**(기계적이지만 양이 큼). 대안: `APP_KEYMAP` 밖의 전용 리스너(command-palette 가 이미 `command-binding-dispatch.ts` 로 제2 리스너 선례를 만듦) → 리바인딩 불가 대신 코드량 최소. **결정 필요**.

### C. 탐색기 단축키 (규모 M)

| 옵션 | 장점 | 단점 |
|---|---|---|
| **C1 (추천) file-tree 로컬 `onKeyDown` 확장** (:156-203) | 새 컨텍스트 getter 불필요, monaco/터미널과 원천적으로 무충돌, 즉시 구현 | 키바인딩 에디터에 안 나오고 리바인딩 불가. 이미 이 파일이 그 자리(↑↓←→ Enter) |
| C2 `APP_KEYMAP` + `when: 'explorerTreeFocus'` | 리바인딩·충돌 감지·팔레트 노출 | getter 추가 + **아래 리스크 3(카탈로그 `when` 누락)** 로 오탐 경고 발생. 9~10개 엔트리 + 로케일 |
| C3 혼합 — 이름바꾸기/Space/⌘⌫ 등 트리 전용은 C1, ⌥⌘R·⌥⌘C 처럼 전역 성격은 C2 | 실용적 | 두 체계 공존 |

C1 기준 영향 파일: `file-tree.tsx`(핸들러 확장), `file-tree.tsx` props 확장(`onCut/onCopy/onPaste/onRevealInFinder/onCopyPath/onStartRename/onRequestDelete` 를 `contextMenuHandlers` 에서 이미 받고 있으므로 **신규 prop 거의 불필요**), 로케일 0. 규모 **S~M**.

### D. 빈 공간 더블클릭 → 루트 초안 (규모 S)

`file-tree.tsx` 컨테이너에 `onDoubleClick` 1개 + hit-test 재사용 + `startDraft(kind, targetDir)` 시그니처 확장(`use-explorer-entry-crud.ts:64`) + `explorer-container.tsx` 배선 1줄. 규모 **S**.

---

## 리스크·미확인

1. **키바인딩 에디터의 `when` 무시 (확인됨, 실제 영향)** — `KeybindingRow`(`keybinding-catalog.ts:12-26`)에 `when` 필드가 없고 `buildKeybindingRows`(:41-75)가 채우지 않는다. `findKeymapConflict` 의 `hasDisjointKeymapWhenScopes`(`keymap.ts:307`)는 양쪽 모두 `when` 이 있을 때만 무충돌 판정하므로, 카탈로그 경로에서는 **모든 행이 `when: undefined`** → `when` 스코프로 분리한 바인딩도 **충돌로 오탐**된다. 예: 탐색기 ⌘↓ 를 추가하면 `terminal-jump-to-next-command`(⌘↓, `terminalFocus`)와 충돌 경고가 뜬다. C2 를 택하면 이 결함부터 고쳐야 한다.
2. **Space 이중 처리** — 전역 바인딩으로 Space 를 잡으면 file-tree 타이프어헤드(`file-tree.tsx:200-202`)와 초안 입력이 영향받을 수 있다(초안 중에는 `isEditing` 가드 :157 로 로컬은 보호되나, 전역 capture 리스너에는 그 가드가 없다).
3. **⌥⌘↑/↓ 는 monaco 멀티커서와 정면 충돌** — `!editorTextFocus` 필수. 단 `editorTextFocus` 는 `.monaco-editor` 컨테이너 전체(`keymap-context.ts:28`)라 Find 위젯 포커스 시에도 true.
4. `[미확인]` WKWebView/macOS 가 ⌥⌘←/→ 를 OS 레벨에서 선점하는지 — capture-phase `preventDefault` 로 막힐 가능성이 높으나 실기 확인 필요(앱 실행 금지 제약으로 미검증).
5. `[미확인]` `revealFileInOS`(⌥⌘R), `copyFilePath`(⌥⌘C), `copyRelativeFilePath`(⇧⌥⌘C) 의 VS Code 기본키 — 통용되는 값으로 적었으나 1차 출처(`fileCommands.ts`/`fileActions.contribution.ts` 의 `KeybindingsRegistry` 절)에서 미확인. Space·Enter·⌘⌫·⌘X/C/V 는 1차 출처 확인 완료.
6. `[미확인]` `keymap.test.ts`(579줄)·`keybinding-catalog.test.ts`(361줄)에 "APP_KEYMAP 전체 중복 검사" 또는 행 개수 하드코딩이 있는지 — 엔트리 추가 시 깨질 테스트 범위를 확정하려면 통독 필요(시간 상한으로 미수행).
7. `[미확인]` `ko.json`/`ja.json` 이 `en.json` 과 키 집합이 완전 동일한지(누락 키 검증 스크립트 유무).

---

## 결정이 필요한 지점

1. **탭 이동 키 정책** — A1(⌃Tab 유지 + ⌥⌘←/→ 추가, 추천) / A2(⌥⌘←/→ 로 교체)
2. **⌘1~9 의 의미** — VS Code 정합(**그룹** 포커스, 추천) / 사용자 요청대로 "그룹별 탭 N번 포커스"(VS Code 와 불일치)
3. **⌘1~9 구현 방식** — `APP_KEYMAP` 엔트리 9개(리바인딩 가능, 로케일 27줄) / 전용 리스너(코드 최소, 리바인딩 불가)
4. **탐색기 단축키 배치** — C1 로컬 핸들러(추천, 즉시·무충돌) / C2 `APP_KEYMAP`+`when`(리바인딩 가능하나 리스크 1 선행 수정 필요) / C3 혼합
5. **탐색기 Enter 의미 변경** — VS Code mac 정합(Enter=이름 바꾸기, ⌘↓=열기)으로 바꿀지, 현행(Enter=열기) 유지하고 F2/⌘⌫ 등만 추가할지. **기존 사용자 동작이 바뀌는 유일한 항목**
6. **리스크 1(카탈로그 `when` 누락)을 이번 배치에서 고칠지** — C2 채택 시 필수, C1 채택 시 별건
7. **빈 공간 더블클릭의 생성 종류** — 새 파일 고정(추천) / 새 파일 vs 폴더 선택 UI

---

## T2 — 프로젝트 묶음(그룹) · 한 화면 다중 프로젝트 split

## 현행(근거)

### 1. "활성 프로젝트 1개" 전제가 실제로 박혀 있는 곳

전제는 **Rust 세션 1필드 + 프론트 6파일**에 국한된다. 그 아래 패널 위젯은 이미 전부 `projectId` prop 구동이다.

| 층 | 위치 | 내용 |
|----|------|------|
| Rust 세션 | `src-tauri/src/domain/project/types.rs:94` | `SessionState.active_project: Option<ProjectId>` — **단수 옵션 1개**가 전부 |
| Rust 상태 | `src-tauri/src/state.rs:35-36` | `projects: HashMap<ProjectId, Project>` · `layouts: HashMap<ProjectId, ProjectLayout>` — 이미 **N개 동시 보유** |
| Rust 서비스 | `project/service.rs:77,109,147` | `open_project`/`activate_project` 가 `active_project` 를 단일 값으로 덮어씀. `close_project`(`:128-129`)는 마지막 항목으로 폴백 |
| Rust 커맨드 | `project/commands.rs:51` `project_get_active` → `Option<ProjectId>` | 프론트가 읽는 유일한 "활성" 소스 |
| 프론트 정본 | `src/entities/project/project.query.ts:30` `activeProjectQueryOptions()` | 비-테스트 소비자 **6파일**뿐: `app-shell.tsx:50` · `title-bar-content.tsx:9` · `status-bar-content.tsx:43` · `command-palette.tsx:104` · `task-runner-dialog.tsx:25` · `ide-sync-provider.tsx:45` |
| 셸 | `src/widgets/app-shell/app-shell.tsx:219-250` | `activeProjectId` 하나를 `AppSidebar`·`ExplorerContainer`·`EditorArea` 에 내려주는 **단일 `<Group>` 2패널**(explorer | editor) |

**중요 — 패널 위젯은 이미 프로젝트 무관(멀티 인스턴스 가능)**:
`ExplorerContainer`(`explorer-container.tsx:31-54`), `EditorArea`(`editor-area.tsx:62-68`), `GitPanelContainer`(`git-panel-container.tsx:40-44`), `SearchPanelContainer`(`search-panel-container.tsx:20-33`), `PaneNodeView`(`pane-node-view.tsx:44-54`), `EditorPane`(`editor-pane.tsx:59-64`) 전부 `projectId: ProjectId` 를 **prop 으로 받고** 그 값으로만 쿼리/뮤테이션을 만든다. 전역 활성 세션을 읽는 위젯은 위 6파일뿐이다.

### 2. 보조 창(auxiliary window)이 이미 구현한 "프로젝트별 독립 셸"

`docs/features/layout-shell.md` §7 + 실제 코드 기준, **Option C 는 사실상 완성품이다.**

| 축 | 현행 | 근거 |
|----|------|------|
| 창 모델 | main 1 + `editor-<n>` N개, 라벨은 Rust 발급(최소 미사용 번호) | `layout-shell.md:176-182` |
| 프로젝트 고정 | 창이 `?projectId=&windowSlot=` 쿼리로 자기 프로젝트에 **핀 고정**, `ProjectActivated` 무시 | `src/shared/lib/window-context.ts:13-34`, `auxiliary-title-bar-content.tsx:15-20` |
| 레이아웃 분리 | `ProjectLayout.auxiliary_windows: Vec<AuxWindowLayout>`(슬롯별 독립 `PaneNode` 트리) | `layout/types.rs:134-140, 207` |
| 트리 해석 | `resolveWindowPaneTree(layout, windowContext)` — main/aux 분기 | `src/shared/lib/pane-tree.ts:55-59` |
| 크롬 | 사이드바·상태바 없는 **에디터 전용** | `auxiliary-window-shell.tsx:20-63` |
| 미구현 | 보조 창에 explorer·git·search·팔레트·TaskRunner 없음. 넷 다 전역 활성 세션을 읽어서 main 에만 마운트 | `src/app/app.tsx:22-55, 59-81` |
| 자원 | LSP 세션은 **창(owner)별 독립 프로세스**(`lsp/types.rs:258-267`), pty 는 멀티 attach 브로드캐스트 | `layout-shell.md:218-237` |

### 3. root_guard 는 이미 "프로젝트 단수" 전제가 아니다

- `resolve_owning_project(projects, path)`(`infra/root_guard.rs:24-57`)는 **열린 프로젝트 전체**를 훑어 가장 구체적(가장 긴 canonical root) 프로젝트를 고른다. 동률이면 `ProjectId` 사전순.
- 즉 `layout_open_tab`(`layout/commands.rs:53`)·`file_open`/`file_save`(`file/commands.rs:27,61`)는 **경로가 어느 열린 프로젝트 root 안이기만 하면** 통과한다 — 탭이 어느 프로젝트 레이아웃에 들어 있는지는 검사하지 않는다.
- 반대로 프로젝트 스코프 자원(미러·트리·검색 인덱스)은 `root_guard::project_root(&projects, &project_id)`(`file/commands.rs:149,164,174,...`)로 **인자로 받은 projectId** 를 검증한다.
- 프론트는 이미 "탭 경로가 그 탭 프로젝트 root 밖" 상태를 **정상 렌더 + 기능 강등**으로 처리한다: `editor-pane.tsx:86` `isOutsideProjectRoot` → LSP 비활성(`use-editor-lsp-integration.ts:61`)·hot-exit 미러 skip(`use-editor-file-persistence.ts:238`)·formatOnSave off(`editor-pane.tsx:122`).

### 4. 그룹/워크스페이스 개념은 현재 0

- `session.json` 에 `projects: Vec<ProjectRef>`(순서 = 사이드바 순서) + `active_project` 뿐(`project/types.rs:88-95`, `docs/data-model.md` §2).
- `ProjectRef { id, root, name, display }`(`types.rs:52-60`) — `display` 는 `ProjectDisplay { icon, label, color }`(2026-09-04 배치, `data-model.md` §20). 정본은 `project.json`, `session.json` 은 미러이며 동기화 지점은 `upsert_project_ref` **한 곳**.
- 순서 변경은 `project_reorder(ids)` → `service::reorder_projects`(`project/service.rs:155-177`) 가 `session.projects` 를 재배열하고 `save_session`. 프론트는 dnd-kit `verticalListSortingStrategy` 1차원 리스트(`app-sidebar.tsx:40-76`).
- 갭 리서치는 멀티루트 워크스페이스를 **P2 · 비용 "상(프로젝트 모델 변경)"** 으로 분류하고 "TAIDE 멀티프로젝트로 상당 부분 대체" 라 적어 뒀다(`docs/research/2026-08-13-vscode-cursor-gap.md:78`).

### 5. 부팅/열기 비용 (그룹 열기 = 멤버 전부 열기의 근거)

- `project_open` 은 **capability attach 를 await** 한다(`project/commands.rs:85-95`) → 응답 시간에 워처 walk 포함. 가드 점유 구간만 µs 대로 줄었을 뿐 **응답 시간은 의도적으로 불변**(`quality-assurance/2026-09-04-perf-baseline.md:151,158`).
- 워처 walk 실측: 인덱싱 엔트리 557,474 → 1,461, stat walk 3.79 s → **0.01 s**, 상주 105 MB → **0.2 MB per project**(같은 파일 C.2-6 ②). ignore 적용 후에는 프로젝트당 비용이 크지 않다.
- 부팅 복원은 `restore_project_watchers` 가 **순차**로 돌되 `active_project` 를 맨 앞에 정렬(`project/commands.rs:343-382`), `is_shutting_down()` 로 중단 가능(`state.rs:148`).
- 실기 8지표(지표 2 = 프로젝트 전환)는 **아직 미측정**(`perf-baseline.md:59`, §7 체크 미완).

---

## 핵심 질문별 답

### Q1. 활성 프로젝트 1개 전제의 소재 → **"얇다"**

Rust 는 `SessionState.active_project` **1필드**, 프론트는 `activeProjectQueryOptions` **6파일**이 전부다. `AppState.projects`/`layouts` 는 처음부터 `HashMap<ProjectId, _>` 로 N개를 동시에 들고 있고, 모든 패널 위젯은 `projectId` prop 구동이라 **멀티 인스턴스화 자체가 이미 가능**하다. 보조 창(§7)은 그 증거이자 부분 구현이다.

진짜 병목은 Rust 가 아니라 **프론트의 창 단위 싱글턴 3종**이다.

| 싱글턴 | 위치 | 한 창에 셸이 2개면 |
|--------|------|--------------------|
| fire-and-forget 브리지(사이드바 토글·뷰 전환·reveal·rename·search 열기) | `shared/lib/bridge/*.ts`(`createFireAndForgetBridge`) | 모듈 레벨 publish/subscribe → **모든 셸이 동시에 반응** |
| 전역 키맵 | `shared/hooks/use-global-keymap.ts` + `use-keydown-capture` | document 캡처 → ⌘B·⌘⇧F 가 **양쪽 다** 발화 |
| `getWindowContext()` | `window-context.ts:34` | `location.search` 파생 → 창당 1개 컨텍스트뿐, 창 안 분할 개념 없음 |

### Q2. 설계 옵션 A/B/C 비교

| 축 | **A. 프로젝트 셸 단위 좌우 분할** | **B. 에디터 영역만(탭이 projectId 보유)** | **C. 기존 보조 OS 창 활용** |
|----|---|---|---|
| **규모** | **L** | **XL** | **S** |
| 한 줄 요약 | `AppShell` 을 "셸 슬롯 N개" 로 바꾸고 사이드바만 공용 | 한 pane tree 안에 다른 프로젝트 탭이 섞임 | 보조 창에 explorer/git/search/팔레트를 추가 |
| Rust 변경 | `active_project: Option<ProjectId>` → 슬롯 목록(`Vec<ProjectId>` + focused). `project_get_active`/`project_activate` 시그니처·이벤트 payload 확장 | `Tab` 에 `project_id` 추가 → **레이아웃 스키마 v3 마이그레이션**. `locate_project_with_tab`·`PaneTreeRef`·`normalize_owned`·`ClosedTab`·`layout_move_tab_to_window` 전부 재설계 | **0~소** (창 라벨/슬롯 기구 그대로) |
| 프론트 핵심 변경 | `app-shell.tsx` 를 `<Group>` N슬롯으로. 브리지·키맵을 **포커스 셸로 스코프**(신규 컨텍스트) | `pane-node-view.tsx`·`pane-tab-bar.tsx`·`editor-area.tsx` 가 탭별 projectId 로 분기. explorer/git/search 는 "포커스 탭의 프로젝트" 를 따라감 | `app.tsx:59-81` aux 분기에 위젯 4종 추가 + 그것들을 `projectId` prop 화(팔레트·TaskRunner 는 현재 전역 세션 의존) |
| **root_guard** | 무변경 — 이미 `resolve_owning_project` 가 열린 전체를 훑음(`root_guard.rs:24`) | 무변경이지만 **의미가 바뀜**: `project_root(&projects, &project_id)` 계열(미러·트리·검색)이 "탭의 프로젝트" 를 받아야 함. 지금은 화면 프로젝트를 넘김 | 무변경 |
| **LSP** | 창 단위 owner 유지 → **한 창 안 두 프로젝트가 같은 owner** 를 공유. 같은 서버 id·다른 root 는 `SessionEntry.roots` 참조카운트로 이미 커버(`data-model.md` §16 R7#7) | 동일. 단 `isOutsideProjectRoot` 강등 로직(`editor-pane.tsx:86`)을 **탭 projectId 기준**으로 고쳐야 함 | 이미 창별 독립 프로세스(`layout-shell.md` §7.4) — 무변경 |
| **pty** | 세션은 `project_id` 보유(`terminal/types.rs:21,49`), attach 는 멀티 구독 브로드캐스트 → 무변경 | 무변경 | 무변경 |
| **hot-exit 미러** | `buffers/` 가 projectId 별(`data-model.md` §2·§6) → 무변경. 단 `EditorArea` 의 prune sweep(`editor-area.tsx:353-367`)이 셸마다 돌아 **서로의 미러를 prune 하지 않도록** 확인 필요(현재 `collectAllPaneTabs` 가 프로젝트 내부만 훑음) | **핵심 리스크** — 미러 키가 탭의 프로젝트를 따라가야 하고, prune sweep 은 cross-project 로 확장돼야 함 | 이미 aux 창이 같은 sweep 을 돌고 `collectAllPaneTabs` 로 해결됨(`pane-tree.ts:70-80`) |
| **layout 영속** | 프로젝트별 `layout.json` 그대로. 슬롯 구성은 `session.json` 에 신규 필드 | `ProjectLayout` 소유 모델 붕괴 — "이 탭은 어느 layout.json 에 저장되나" 가 미정의 | 그대로(`auxiliary_windows`) |
| **차단 요인** | 브리지·키맵 싱글턴(위 표). 공용 사이드바가 "누가 포커스인가" 를 알아야 함 | `ProjectLayout` = 프로젝트 1:1 이라는 **아키텍처 전제 자체**. 스키마 v2→v3 마이그레이션은 v1→v2(순수 가산, `data-model.md` §8)와 달리 **구조 변경** | 팔레트/TaskRunner 의 전역 세션 의존(`app.tsx:35-39`, Wave I 계약 F1 미결로 명시). aux 창은 `tauri-plugin-window-state` 추적 제외라 **위치·크기 복원 없음**(`layout-shell.md` §7.6) |
| 사용자 기대 부합 | "한 화면에 두 프로젝트" 를 문자 그대로 충족 | 탭 단위 혼합 — VS Code 에도 없는 모델 | 화면은 OS 창 2개 (요구가 "한 화면" 이면 미충족) |

### Q3. 그룹 데이터 모델

| 옵션 | 모양 | 장점 | 단점 |
|------|------|------|------|
| **G1. `ProjectRef`/`Project` 에 `group` 필드** | `#[serde(default)] group: Option<String>` | `data-model.md` §5 "필드 추가는 무마이그레이션 흡수" 규칙에 정확히 해당(§18 `last_opened_at`·§20 `display` 와 같은 경로). `upsert_project_ref` 미러 1지점 재사용. 변경 **S** | 그룹 자체에 이름·색·순서·접힘 상태를 못 붙임. 빈 그룹 표현 불가. 그룹 순서가 프로젝트 순서에 종속 |
| **G2. `SessionState.groups: Vec<ProjectGroup>` 별도 엔티티** | `ProjectGroup { id, name, color, members: Vec<ProjectId>, collapsed }` | 그룹 이름·색·접힘·순서를 1급으로. 사이드바 2단 sortable(그룹 ↔ 그룹 내부)과 자연스럽게 대응. "그룹 열기" 를 멤버 목록으로 그대로 표현 | 신규 IPC 4~5종(`project_group_create/rename/delete/set_members/reorder`)·이벤트 1종. `session.projects` 와 **멤버십 이중 진실** 위험 → 정규화 규칙 필요. 변경 **M** |

**추천: G2 (별도 `ProjectGroup` 엔티티), 단 `session.projects` 를 단일 진실로 유지**하고 `ProjectGroup.members` 는 "순서 힌트" 가 아니라 **소속 집합**으로만 쓰고, 열림 여부는 여전히 `session.projects` 가 정한다.

근거:
- 사이드바는 이미 `project_list` 의 `ProjectRef[]` **하나로만** 렌더된다(`app-sidebar.tsx:26,63-75`, `data-model.md` §20 의 "미러를 두는 이유"). 그룹 헤더를 그리려면 그룹 이름·색이 그 응답에 있어야 하므로, G1 의 문자열 필드로는 프로젝트당 `project_get` 추가 호출이 되살아난다.
- `display`(§20)가 이미 "아이콘·라벨·색" 을 프로젝트 단위로 갖는다 → 그룹 색과 **같은 `graph.laneN` 팔레트를 재사용**하면 신규 테마 토큰 0으로 끝난다.
- dnd: 현재 `verticalListSortingStrategy` 1차원(`app-sidebar.tsx:63`). 2단 중첩은 dnd-kit 에서 `SortableContext` 를 그룹마다 하나씩 두는 표준 패턴으로 확장 가능 — `reorder_projects` 는 유지하고 `project_group_reorder` 를 병렬로 추가.
- 최근 목록(`project_list_recent`, `project/service.rs:417`)은 **디스크 기록 전수 × `last_opened_at` 내림차순**이라 그룹과 직교한다. 그룹은 "열린 세션" 의 조직화, 최근 목록은 "닫힌 것 포함 이력" — 섞지 않는 편이 맞다.

**그룹 열기(멤버 전부 열기) 부팅 비용**: `project_open` 이 capability attach 를 await 하므로 **직렬이면 멤버 수 × 열기 시간**이 그대로 체감된다(`commands.rs:85`, `perf-baseline.md:158`). 다만 워처 walk 는 ignore 적용 후 0.01 s/0.2 MB 수준(C.2-6 ②)이라, 실제 지배 항목은 워처가 아니라 **레이아웃 로드 + 프론트 마운트**일 가능성이 높다 — 실기 지표 2 가 미측정이라 [미확인]. 완화책은 부팅 복원과 같은 패턴: 첫 멤버만 동기 열기 + 나머지는 `restore_project_watchers` 식 백그라운드 순차 큐(`commands.rs:386-...`).

---

## 설계 옵션

### 추천 경로: C → A (2단계)

**1단계 (S) — 보조 창 완성으로 "다중 프로젝트 동시 작업" 을 먼저 충족**
- `app.tsx:59-81` aux 분기에 `ExplorerContainer`/`GitPanelContainer`/`SearchPanelContainer` 를 `projectId={windowContext.projectId}` 로 마운트(세 위젯 모두 이미 prop 구동이라 **위젯 변경 0**).
- `CommandPalette`/`TaskRunnerDialog` 를 `activeProjectQueryOptions()` 대신 `projectId` prop 으로 받도록 좁힘(각 1파일: `command-palette.tsx:104`, `task-runner-dialog.tsx:25`) → Wave I 계약 F1 미결 항목 종결.
- 영향 파일: `src/app/app.tsx`, `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx`, `command-palette.tsx`, `task-runner-dialog.tsx` (+ 창 위치 복원은 `layout-shell.md` §7.6 제약 그대로 두거나 별건).

**2단계 (L) — 한 창 안 셸 분할**
- `SessionState.active_project: Option<ProjectId>` → 슬롯 모델. 가장 하위 호환적인 형태는 **필드 유지 + 신규 필드 추가**: `shell_slots: Vec<ProjectId>`(`#[serde(default)]`, 비면 `active_project` 단독 해석) — §5 의 무마이그레이션 규칙 안에 들어온다.
- `AppShell` 을 `ShellSlot` 컴포넌트 N개 + 공용 `AppSidebar` 로 분해. `AppSidebar` 는 슬롯 배정 UI(아이콘 우클릭 "오른쪽에 열기")를 갖는다.
- **선행 필수**: 브리지·키맵의 포커스 스코프. `getWindowContext()` 를 대체할 `ShellSlotContext`(React context)를 만들고, `createFireAndForgetBridge` 를 slot id 별 채널로 확장하거나 구독자에 slot 필터를 준다.

### 대안: B 는 비추천
`ProjectLayout` = 프로젝트 1:1 이 `state.rs:36`·`layout/service.rs` 전반(`locate_project_with_tab`, `PaneTreeRef`, `run_layout_mutation` 의 `LayoutLocate`)의 기반이다. 탭이 projectId 를 갖는 순간 "이 탭은 어느 `layout.json` 에 저장되나" 가 미정의가 되고, v1→v2 가 순수 가산이었던 것과 달리 **구조 마이그레이션**이 필요하다. 얻는 것("한 pane 에 다른 프로젝트 탭")은 A 로도 인접 슬롯으로 충족된다.

---

## 리스크·미확인

| 항목 | 내용 |
|------|------|
| 리스크 (A) | 브리지 3계열(`explorer-panel-bridge`·`explorer-reveal-bridge`·`explorer-rename-bridge`·`search-panel-bridge`)과 `use-global-keymap` 이 realm 싱글턴 — 스코프 없이 슬롯 2개를 띄우면 ⌘B 가 양쪽에 걸린다 |
| 리스크 (A) | `EditorArea` 의 hot-exit 미러 prune sweep(`editor-area.tsx:353-367`)이 슬롯마다 돈다. `collectAllPaneTabs`(`pane-tree.ts:78-80`)는 **프로젝트 내부**만 훑으므로 프로젝트별로는 안전하나, 같은 프로젝트를 두 슬롯에 띄우는 케이스는 재확인 필요 |
| 리스크 (C) | 보조 창은 `tauri-plugin-window-state` 추적 제외라 매번 1000×700 기본 크기(`layout-shell.md` §7.6). "묶음을 열면 배치가 복원" 기대와 충돌 |
| 리스크 (그룹) | `session.projects` ↔ `ProjectGroup.members` 이중 진실. 프로젝트 close 시 멤버 목록 정리 규칙 필요 |
| [미확인] | 실기 8지표 전부 미측정 — **지표 2(프로젝트 전환)** 수치가 없어 "그룹 열기 = 멤버 N개 열기" 의 실제 체감 비용을 수치로 말할 수 없다(`perf-baseline.md:59`, §7 잔여 항목) |
| [미확인] | 슬롯 2개 동시 렌더 시 Monaco 인스턴스·쿼리 캐시 메모리 증가량. 지표 9(메모리)도 미측정 |
| [미확인] | `docs/acknowledge/2026-09-04-usability-batch3-contract.md`·`batch4-contract.md`·`2026-09-06-d54-...-contract.md` 본문은 이번 조사에서 직접 읽지 않았다(시간 상한). 그룹/분할 관련 기존 결정이 그 안에 있을 수 있음 |
| [미확인] | dnd-kit 중첩 `SortableContext` 2단 구현의 구체 API 적합성 — 설치 버전 소스 미확인 |

---

## 결정이 필요한 지점

1. **"한 화면" 의 정의** — A안(한 OS 창 안 좌우 분할, L) / C안(보조 OS 창 병렬 배치, S) 중 무엇이 요구인가. 화면 = 모니터라면 C 로 충분하다.
2. **단계 진행 여부** — 추천대로 C(1단계) 먼저 하고 A 를 후속으로 둘지 / A 를 곧장 갈지.
3. **그룹 모델** — **G2 별도 `ProjectGroup` 엔티티(추천)** / G1 `ProjectRef.group` 문자열 필드.
4. **그룹 열기 정책** — 멤버 전부 즉시 열기(직렬, 체감 비용 수용) / 첫 멤버만 즉시 + 나머지 백그라운드 큐(부팅 복원 패턴 재사용, 추천) / 그룹은 표시만 하고 열기는 개별.
5. **그룹 ↔ 슬롯 관계** — "그룹 열기 = 슬롯 N개 배치" 로 묶을지, 그룹은 순수 사이드바 조직화이고 슬롯은 별개 조작으로 둘지.
6. **`active_project` 호환 전략** — 필드 유지 + `shell_slots` 추가(무마이그레이션, 추천) / `active_project` 를 `Vec` 으로 교체(스키마 버전 상승).
7. **보조 창 위치·크기 복원** — `layout-shell.md` §7.6 의 제외 결정을 그대로 둘지, 그룹 배치 복원을 위해 슬롯별 키로 되살릴지.

---

## T3 — OS 알림 내용 강화(항목 4): 발화 지점·플러그인 필드 한계·에이전트 활동 신호 활용

## 현행(근거)

### 알림 파이프라인 한 줄 요약

`발화 지점(FE, t() 로 번역된 title/body)` → `notifyNative()`(창/미러 게이트 + 200 코드포인트 절단, `src/entities/notification/notify.ts:42-56`) → `notification_notify` IPC → Rust `masked_notification_text`(시크릿 마스킹) + `decide_delivery`(마스터/카테고리/포커스 게이트) → `app.notification().builder().title(..).body(..).show()` (`src-tauri/src/domain/notification/commands.rs:49-57`).

- **텍스트 소유권은 FE**다: "Rust 는 *보낼지*, 프론트는 *무슨 말을 할지* 소유" — `src-tauri/src/domain/notification/commands.rs:28-30`, `docs/ipc-contract.md:1869-1870`. 즉 이번 작업은 **FE 문자열 조립 변경만으로 가능**(Rust 계약 무변경).
- 빌더에 넘기는 필드는 **title/body 2개뿐**이며, Rust 쪽에서 `.icon()`/`.sound()` 도 호출하지 않는다 (`commands.rs:51-55`).

### 1. 발화 지점 전수 (6곳)

| # | 파일:줄 | category | 현 title | 현 body | 그 자리에서 즉시 가용한 컨텍스트 |
|---|---|---|---|---|---|
| 1 | `src/app/providers/native-notification-provider.tsx:63` | `agentCompleted` | `t('notification.agentCompleted')` = "에이전트 작업 완료" | **`agent.name`** (= `"claude"`) | `payload.projectId`, `agent.sessionId`, `agent.workedForMs`, `agent.activity`(간접) |
| 2 | `native-notification-provider.tsx:84-89` | `taskCompleted` / `error` | `notification.taskCompleted{Succeeded,Failed}` | `payload.cwd ?? t('terminal.title')` | `payload.sessionId`, `exitCode`, `durationMs` |
| 3 | `native-notification-provider.tsx:94-98` | `lspInstall` / `error` | `notification.lspInstall{Succeeded,Failed}` | `payload.message ?? payload.serverId` | `serverId`, `phase`, 수신 바이트 |
| 4 | `src/entities/git/git.query.ts:213-217` | `gitRemote` | `notification.gitPush/Pull{Succeeded}` | 캐시된 `GitStatus.branch ?? ''` | `projectId`(훅 인자), `queryClient` |
| 5 | `src/entities/git/git.query.ts:218` | `error` | `notification.gitPush/PullFailed` | `describeIpcError(error)` | 동상 |
| 6 | `src/widgets/search-panel/search-panel-container.tsx:104` | `searchReplace` | `notification.searchReplaceDone` | `t('search.replaceDone', {files, matches})` | `projectId`, `ranQuery` |
| (참고) | `src/widgets/settings-view/settings-notification-section.tsx:50-54` | 설정값(테스트) | `settings.notificationsSendTest` | `notification.enableHint` | 테스트 버튼 전용, `notify.ts` 미경유 |

**로케일 키 위치**: 프론트 카탈로그가 아니라 **Rust 리소스**다 — `src-tauri/resources/locales/{ko,en,ja}.json`. `notification.*` 블록은 ko/en 모두 **424~434행**(`notification.agentCompleted` = ko "에이전트 작업 완료" / en "Agent finished"). i18next 는 빈 `resources: {}` 로 시작해 `applyLocaleMessages` 로 주입받는 구조(`src/shared/i18n/i18n.ts:9-19`), 키 세퍼레이터 없음(`keySeparator: false`) → 키는 점 포함 평면 문자열.

**"claude 만 띡 뜨는" 근거 줄**: `native-notification-provider.tsx:63` 의 `body: agent.name`. `agent.name` 은 Rust 가 `ps` 로 탐지한 프로세스명이고 값 집합은 `KNOWN_AGENT_NAMES = ["claude","codex","gemini"]`(`src-tauri/src/domain/agent/types.rs:8`)뿐이다. 즉 body 는 구조적으로 세 단어 중 하나로 고정.

### 2. 컨텍스트 조달처

| 넣고 싶은 값 | 조달 경로(근거) |
|---|---|
| 프로젝트 표시명 | `ProjectRef { id, root, name, display? }`(`src/shared/api/bindings.ts:1775-1785`) → `projectListQueryOptions`(`src/entities/project/project.query.ts:19`, 키 `QUERY_KEY.PROJECT.LIST`). 표시 오버라이드 해석은 `resolveProjectDisplay`(`src/shared/lib/project-display.ts:82-91`) 한 곳으로 통일돼 있다(라벨 우선 → 아이콘 → 기본) |
| 프로젝트 id | 이벤트 payload 에 이미 있다: `AgentStateChanged = { projectId, agents }`(`bindings.ts:975-978`), 현재는 타이밍 맵 키로만 쓰인다(`native-notification-provider.tsx:57-62`) |
| 경과 시간 | `evaluateAgentCompletions` 가 이미 `workedForMs` 를 반환하는데 **소비처가 없다**(`src/shared/lib/native-notification-gate.ts:36, 76-77`) |
| 에이전트/세션 | `DetectedAgent { sessionId, name, pid, activity }`(`bindings.ts:1154-1159`). `sessionId` 는 **pty 세션 id**(`src-tauri/src/domain/agent/commands.rs:408-409` 가 `probe.session_id` 를 그대로 싣는다) |
| 터미널 탭 제목 | `TabKind` 의 터미널 변형이 같은 `sessionId` 를 키로 갖는다(`bindings.ts:2532`) → `layoutQueryOptions(projectId)` 캐시(`src/entities/layout/layout.query.ts:34-36`, 키 `QUERY_KEY.LAYOUT.DETAIL`)에서 역인덱스 가능 |
| 마지막 명령 | `TerminalCommandFinished = { sessionId, cwd, exitCode, durationMs }`(`bindings.ts:2637-2642`) — **명령 문자열은 payload 에 없다**. OSC 133 `C`→`D` 구간의 커맨드 텍스트는 전달되지 않는다 |
| 캐시 읽기 선례 | `git.query.ts:214` 가 이미 `queryClient.getQueryData<GitStatus>(...)` 로 알림 body 를 만든다(주석 210-212: "부제일 뿐이라 IPC 왕복 대신 캐시를 읽는다") |

`NativeNotificationProvider` 는 `AppProviders`(= `QueryClientProvider`, `src/app/providers/app-providers.tsx:13`) 안쪽에 마운트되므로(`src/app/app.tsx:83-95`) **`useQueryClient()` 사용 가능**하다. 즉 프로젝트명·레이아웃 조회에 새 IPC 가 필요 없다.

### 3. 플러그인 필드·클릭 이벤트 (1차 출처: 플러그인 v2.4.0 소스)

`tauri-plugin-notification` 2.4.0 은 **로컬 cargo registry 에 추출돼 있지 않아**(`~/.cargo/registry/src/...` 에 부재) GitHub 태그 `notification-v2.4.0` 의 원본을 받아 확인했다(`plugins/notification/src/{desktop,lib,models,commands}.rs`, `guest-js/index.ts`).

| 필드 | 데스크톱(macOS) 반영 여부 |
|---|---|
| `title` | O — `notify_rust` 의 `summary` |
| `body` | O |
| `icon` | O(미지정 시 `auto_icon()`) — Rust 코드에서 미사용 |
| `sound` | O(macOS 시스템 사운드명) — Rust 코드에서 미사용 |
| `subtitle` | **필드 자체가 없음.** `NotificationData` 에 `summary`/`large_body`/`inbox_lines` 는 있으나 데스크톱 `show()` 가 **읽지 않는다** — 매핑되는 건 body/title/icon/sound 4개뿐 |
| `actionTypeId` / `extra` / `group` / `attachments` | 데이터 구조엔 존재하나 데스크톱 경로에서 **전부 무시** |

- 데스크톱 `show()` 는 `tauri::async_runtime::spawn(async move { let _ = notification.show(); })` — **결과를 버린다**. `permission_state()`/`request_permission()` 은 `Ok(PermissionState::Granted)` 스텁. (이미 `src-tauri/src/domain/notification/types.rs:37-44` 와 `docs/ipc-contract.md:1865-1868` 에 같은 사실이 기록돼 있다)
- **클릭/액션 이벤트는 데스크톱에 존재하지 않는다.** 플러그인의 `invoke_handler` 에 등록된 커맨드는 `notify`·`request_permission`·`is_permission_granted` **3개뿐**(lib.rs `init()`), `register_listener` 가 아예 없다. JS 의 `onAction`/`onNotificationReceived` 는 `addPluginListener('notification', ...)` → `invoke('plugin:notification|register_listener')`(@tauri-apps/api core.ts) 를 부르므로 **호출 즉시 실패**한다. 애초에 TAIDE 는 JS 게스트 패키지를 설치하지도 않았다(`package.json` 에 `@tauri-apps/plugin-notification` 없음, 의도적 결정 — `docs/ipc-contract.md:1893-1896`).
- dev 빌드에서는 `notify_rust::set_application("com.apple.Terminal")` 스푸핑 때문에 알림이 **Terminal 이름·아이콘**으로 뜬다(desktop.rs `#[cfg(target_os = "macos")]` 분기; 계약에도 기록 — `docs/acknowledge/2026-09-04-usability-batch4-contract.md:40`). "제목이 앱답지 않다"는 체감의 일부는 여기서 온다.

### 4. d-54 에이전트 활동 신호

- 프론트에 노출되는 타입은 **4상태뿐**: `AgentActivity = idle | working | awaitingInput | unknown` (`src-tauri/src/domain/agent/types.rs:110-117`, `src/shared/api/bindings.ts:963`). "Blocked" 라는 별도 상태는 없고 **차단은 `awaitingInput` 으로 합쳐진다**.
- Rust 내부는 더 세분돼 있다: `AgentEvent = PermissionRequest | QuestionAsked | ToolComplete | Stop | StopFailure | IdlePrompt` (`src-tauri/src/domain/agent/service.rs:340-347`). 그러나 `apply_agent_event` 가 `PermissionRequest | QuestionAsked` 를 **동일하게 `signals.blocked = Some(BlockedSource::Event)`** 로 접고(`service.rs:554-557`), `classify_session` 은 `blocked.is_some()` 이면 무조건 `AwaitingInput` 을 돌려준다(`service.rs:645-647`). 다이얼로그 스캔 유래는 `BlockedSource::Dialog`(`service.rs:616`)로 구분되고 `signals.last_event`(`service.rs:503`)에 원 이벤트가 남아 있으므로 **정보는 Rust 안에 살아 있지만 IPC 경계를 넘지 못한다.**
- **현재 completed 판정의 중요한 사실**: `AGENT_COMPLETION_ACTIVITIES = ['idle', 'awaitingInput']`(`src/shared/lib/native-notification-gate.ts:30`). 즉 **권한 요청으로 멈춘 것도 이미 "에이전트 작업 완료" 라는 제목으로 알림이 나가고 있다.** 새 신호를 추가하는 문제가 아니라, 이미 섞여 나가는 두 사건을 **구분해서 말하지 못하는** 문제다.
- 폭주 억제 구조: `evaluateAgentCompletions` 는 매 이벤트마다 맵을 **새로 만들고 `working` 인 세션만 다시 넣는다**(`native-notification-gate.ts:64-67`). 완료로 판정된 세션은 맵에서 사라지므로 같은 세션이 `awaitingInput` 에 머무는 동안 재알림되지 않는다. 임계값은 `AGENT_COMPLETION_NOTIFY_MIN_WORKING_MS = 10_000`(`src/shared/constants/notification.ts`). 프로젝트별 맵이며 `project:closed` 에 삭제(`native-notification-provider.tsx:66-68`).
- 남은 폭주 벡터: 한 턴 안에서 `working ≥10s → 권한대기 → 승인 → working ≥10s → 권한대기` 가 반복되면 **매 사이클마다 알림 1건**이 나간다. 승인 반복이 잦은 세션에서 이게 실제 노이즈원.

---

## 핵심 질문별 답

**Q1. 발화 지점과 텍스트, 가용 컨텍스트** → 위 표 6행. `agentCompleted` 가 `body = agent.name` 인 근거 줄은 `src/app/providers/native-notification-provider.tsx:63`, 값 집합 고정 근거는 `src-tauri/src/domain/agent/types.rs:8`. 프로젝트명·경과시간·세션·탭 제목 전부 **새 IPC 없이** 기존 쿼리 캐시와 이벤트 payload 에서 조달 가능(§2 표).

**Q2. 지원 필드와 클릭** → 데스크톱에서 실효 필드는 **title/body 뿐**(icon/sound 는 호출 가능하지만 미사용). `subtitle` 은 플러그인 경로에 존재하지 않는다. `onAction`/`onNotificationReceived` 는 **데스크톱 미지원**(플러그인이 `register_listener` 커맨드를 등록하지 않음). 따라서 **"알림 클릭 → 해당 프로젝트 활성화 + 창 포커스" 는 현재 의존성으로 구현 불가.**

**Q3. 권한요청/질문 전환의 알림화** → 신호는 Rust 안에 존재하나(`AgentEvent::PermissionRequest`/`QuestionAsked`, `BlockedSource`) **`AgentActivity` 4상태로 접혀 FE 에 오지 않는다.** 구분하려면 Rust 타입 변경 + specta 재생성이 필요하다. 다만 `awaitingInput` 자체는 이미 FE 에 오므로, **타입 변경 없이도 "완료" 와 "입력 대기" 를 다른 제목으로 가르는 것은 오늘 당장 가능**하다(단, 권한요청인지 질문인지는 구분 불가).

---

## 설계 옵션

### 포맷 후보 2안 (에이전트 알림 기준)

데스크톱은 title/body 2줄뿐이므로 "제목/부제/본문 3단" 은 물리적으로 불가. 2줄에 무엇을 담느냐의 선택이다.

**A안 — 제목=사건+프로젝트, 본문=주체+소요시간** (권장)

| 상황 | title | body |
|---|---|---|
| 완료(idle) | `에이전트 작업 완료 — {프로젝트}` | `claude · 3분 12초 작업` |
| 입력 대기(awaitingInput) | `에이전트가 입력을 기다립니다 — {프로젝트}` | `claude · {터미널 탭 제목}` |

- 장점: 알림 센터 목록(제목만 보이는 경우)에서 **어느 프로젝트인지** 즉시 식별. 프로젝트가 1개면 뒷부분이 군더더기가 되므로 `프로젝트 수 ≥ 2` 일 때만 붙이는 분기 가능(`project.query.ts` 캐시로 판단).
- 단점: 제목 길이가 길어져 배너에서 잘릴 수 있음(200 코드포인트 절단은 `notify.ts:52-53` 이 담당하지만 macOS 자체 시각 절단은 별개).

**B안 — 제목=프로젝트, 본문=사건+주체+소요시간**

| 상황 | title | body |
|---|---|---|
| 완료 | `{프로젝트}` | `claude 작업 완료 · 3분 12초` |
| 입력 대기 | `{프로젝트}` | `claude 가 입력을 기다립니다 · {탭 제목}` |

- 장점: 알림 센터에서 같은 프로젝트끼리 시각적으로 묶여 보인다. 제목이 짧아 잘리지 않음.
- 단점: 제목만 보이는 축약 상태에서 **무슨 일이 일어났는지 알 수 없다**. 프로젝트가 1개인 사용자에겐 제목이 전혀 정보를 주지 않음.

> 공통 적용 대상: task(#2)는 `body` 를 `cwd` 대신 `{프로젝트} · {탭 제목} · {exit code} · {소요시간}` 으로, git(#4)은 `{프로젝트} · {브랜치}` 로 같은 규칙을 태울 수 있다(§2 표의 조달처가 전부 이미 존재).

### 구현 옵션

| 옵션 | 내용 | 장단점 | 영향 파일 | 규모 |
|---|---|---|---|---|
| **O1. FE 문자열 조립만 강화** | `native-notification-provider.tsx` 에서 `useQueryClient()` 로 프로젝트명 조회 + `workedForMs` 포맷팅 + 로케일 키 보간화 | Rust·IPC·specta 무변경. 오늘 바로 가능. `awaitingInput` 세부 구분은 못 함 | `native-notification-provider.tsx`, `src-tauri/resources/locales/{ko,en,ja}.json`, (선택) `native-notification-gate.ts` | **S** |
| **O2. O1 + 완료/입력대기 제목 분리** | `evaluateAgentCompletions` 가 `completed` 에 `activity` 를 실어 보내고, 호출부가 `idle`/`awaitingInput` 별 다른 키를 고름 | 오분류("권한 대기인데 완료라고 알림")를 **타입 변경 없이** 해소. 여전히 권한요청 vs 질문은 구분 불가 | O1 + `native-notification-gate.ts`(+테스트) | **S~M** |
| **O3. Rust 가 차단 사유를 노출** | `DetectedAgent` 에 `blockedReason?: 'permission' \| 'question' \| 'dialog'` 추가(`BlockedSource`/`last_event` 에서 유도) → specta 재생성 | "권한 승인 필요" 같은 정확한 문구 가능 | `domain/agent/{types,service,commands}.rs`, `bindings.ts`(생성), 배지 UI, `docs/ipc-contract.md` | **M** |
| **O4. 클릭 라우팅** | 플러그인 우회 필요 — `mac-notification-sys`/`objc2-user-notifications` 직접 호출로 응답 수신 후 `project_activate` + `getCurrentWindow().setFocus()` | 사용자 체감은 가장 큼. 단 **새 macOS 네이티브 의존·플랫폼 분기·번들 권한** 부담, 플러그인과 이중 경로 | `domain/notification/*` 대수술, Cargo 의존 추가 | **L** |

### 클릭 동작 가능성 판정

- **현재 의존성(`tauri-plugin-notification` 2.4.0)으로는 불가.** 콜백 커맨드 자체가 등록돼 있지 않고, `show()` 결과도 버려진다.
- macOS 기본 동작상 릴리스 번들에서 배너를 클릭하면 발신 앱(= TAIDE 번들 id)이 전면으로 올라오는 것까지는 기대할 수 있으나, **어느 알림이었는지(=어느 프로젝트/세션인지)는 앱에 전달되지 않는다**. dev 빌드는 발신자가 `com.apple.Terminal` 로 스푸핑돼 Terminal 이 올라온다.
- 대안(비용 낮은 순): ① 알림은 "무슨 일인지" 만 정확히 말하고 라우팅은 포기, ② 앱 내부에 "최근 알림" 목록/점프 UI 를 두고 알림 클릭은 단순 앱 활성화에 맡김, ③ O4 네이티브 우회.

---

## 리스크·미확인

- [미확인] 릴리스 번들에서 macOS 배너 클릭이 실제로 TAIDE 를 전면화하는지 — 앱 실행 금지 제약으로 실측 못 했다. 소스로 확정한 것은 "**앱이 클릭을 콜백으로 받을 수 없다**" 까지다.
- [미확인] `notify_rust` 의 macOS 백엔드가 자체적으로 subtitle/응답을 지원하는지 — `notify-rust`/`mac-notification-sys` 크레이트 소스가 로컬 registry 에 추출돼 있지 않아 확인하지 못했다. 다만 **플러그인이 subtitle 세터를 호출하지 않으므로**(desktop.rs 매핑 4필드) 플러그인 경유로는 어차피 도달 불가다.
- [미확인] 터미널 탭 제목이 에이전트 세션에서 실제로 유의미한 문자열인지(자동 제목이 `claude` 로 동일할 가능성). `TabKind::terminal` 에 `sessionId`/`cwd` 만 있고 title 은 `LayoutTab` 쪽 필드로 보이는데, 해당 타입 정의는 이번 조사 범위에서 끝까지 열어보지 못했다.
- [미확인] 프로젝트명을 붙일 때 `resolveProjectDisplay` 의 라벨(이모지/짧은 라벨 가능)과 `ProjectRef.name`(폴더명) 중 무엇이 알림에 적합한지 — 라벨은 1~2자일 수 있어 알림 문맥에선 `name` 이 나을 수 있다.
- 리스크: 제목에 프로젝트명을 넣으면 **알림 그룹핑이 제목 기준으로 쪼개져** 알림 센터가 길어질 수 있다(macOS 는 동일 앱 알림을 스택하므로 실제 영향은 제한적일 것으로 보이나 미실측).
- 리스크: `AGENT_COMPLETION_ACTIVITIES` 에 `awaitingInput` 이 들어 있는 현 동작을 O2 로 가르면 **알림 건수 자체는 그대로지만 문구가 바뀐다** — "완료 알림이 줄었다"는 오해를 부를 수 있으므로 릴리스 노트 문구 필요.

---

## 결정이 필요한 지점

1. **포맷 A안(제목=사건+프로젝트) / B안(제목=프로젝트)** 중 택1 — 추천 A안(알림 센터 축약 상태에서도 사건이 보임).
2. **프로젝트명을 항상 붙일지 / 열린 프로젝트가 2개 이상일 때만 붙일지** — 추천 후자(1프로젝트 사용자에게 군더더기 방지).
3. **`awaitingInput` 을 "완료" 와 분리할지** (O2 채택 여부) — 추천 분리. 분리 시 로케일 키 3언어 신규 1~2개.
4. **권한요청/질문까지 구분할지** (O3, Rust 타입 확장 + specta 재생성 + ipc-contract 갱신) — 이번 배치 포함 여부.
5. **클릭 라우팅(O4)** 을 이번에 손댈지, "불가"로 확정하고 앱 내 점프 UI 로 대체할지.
6. **다른 카테고리(task/git/lsp/search)에도 같은 "프로젝트 · 컨텍스트" 규칙을 전수 적용할지**, 에이전트만 먼저 할지.
7. **표시할 이름을 `ProjectRef.name`(폴더명) 으로 할지 `resolveProjectDisplay` 라벨 우선으로 할지.**

---

## T4 — 에이전트 감지 다각화(opencode·codex·pi) 현행 구조·분기점·신호 가용성 조사

## 현행(근거)

### A. 신원 판정 — 한 곳에서만 갈라진다

| 요소 | 위치 | 내용 |
|---|---|---|
| 감지 대상 목록 | `src-tauri/src/domain/agent/types.rs:8` | `KNOWN_AGENT_NAMES: &["claude", "codex", "gemini"]` |
| 이름 상수 | `types.rs:69-71` | `AGENT_NAME_CLAUDE`/`_CODEX`/`_GEMINI` |
| 매칭 로직 | `service.rs:55-71` | `comm` basename 완전일치 → 실패 시 `comm ∈ NODE_RUNTIME_NAMES` 일 때만 `cmdline` 각 인자의 basename 완전일치 |
| 런타임 화이트리스트 | `service.rs:23` | `NODE_RUNTIME_NAMES: &["node", "bun", "deno"]` |
| 잘림 보정 | `service.rs:49-53` | `base.len() >= 15`(Linux comm) 일 때만 prefix 매칭 허용 |

신원 경로는 **`KNOWN_AGENT_NAMES` 한 줄 + `NODE_RUNTIME_NAMES`** 만 건드리면 확장된다(문서 근거: `docs/features/agent-integration.md` §1.1).

### B. 활동 판정 — 4개 축 중 3개가 Claude 하드코딩

| # | 신호 | 분기 지점 | 현재 상태 |
|---|---|---|---|
| 1 | 인밴드 OSC 777 | `service.rs:755-820`(`CLAUDE_HOOK_BINDINGS`·`claude_agent_event_payload`·`build_claude_agent_hook_command`·`claude_hook_entries`) | **Claude 전용**. 함수명·페이로드의 `"agent":"claude"` 가 상수로 박혀 있다 |
| 2 | 타이틀 글리프 | `service.rs:392-393`(`TITLE_WORKING_GLYPHS=['◐','◑']`, `TITLE_IDLE_GLYPH='✳'`), `service.rs:460 parse_title_glyph(title)` | **에이전트 인자가 없다** — 호출부 `service.rs:534` 도 `agent_name` 을 넘기지 않음. 즉 Claude 글리프 표가 전 에이전트에 적용된다 |
| 3 | 다이얼로그 시그니처 | `service.rs:406`(`CLAUDE_DIALOG_SIGNATURES`), `service.rs:408-413 dialog_signatures_for` | Claude 외 전부 `&[]`(빈 표). 주석에 "codex/gemini 문구 미검증이라 추측으로 채우지 않는다" 명시 |
| 4 | 실질 출력 | `service.rs:399`(`NON_SUBSTANTIVE_GLYPHS` 9종) | 에이전트 무관하지만 **글리프 표가 Claude 스피너 기준** |

→ 새 에이전트는 현재 구조상 **신호 4번(실질 출력)과 무출력 4초 유휴만** 얻는다. `AwaitingInput` 은 구조적으로 불가능하다(d-54 계약 §0.1 과 동일한 함정).

### C. 훅 설치 — 2갈래 (인밴드 vs HTTP)

| 갈래 | 대상 | 경로/분기 |
|---|---|---|
| 인밴드 command hook | claude | `.claude/settings.local.json`(`commands.rs:546`), `hooks.rs:92 reconcile_claude_project_hooks` |
| HTTP command hook | codex·gemini | `hooks.rs:121`·`hooks.rs:158` 의 `for agent_name in [AGENT_NAME_CODEX, AGENT_NAME_GEMINI]` 배열, URL 빌더 `hooks.rs:192` |
| 설치 스코프 | — | `service.rs:720-727 hook_scope_for_agent` (claude=Project, codex/gemini=User) |
| 사용자 레벨 경로 | — | `service.rs:855-869 user_level_hooks_path` + `CODEX_HOME_RELATIVE_PATH=".codex/hooks.json"`, `GEMINI_HOME_RELATIVE_PATH=".gemini/settings.json"` |
| 훅 타임아웃 단위 | — | `service.rs:847-853`(codex=초, gemini=밀리초) |
| 훅 이벤트→활동 매핑 | — | `service.rs:692-714 map_hook_event_to_activity` (에이전트별 match) |
| 관리 이벤트 목록 | — | `types.rs:92-107` + `service.rs:732-739 managed_hook_events_for` |
| 휴리스틱 폴백 게이트 | — | `commands.rs:392`: `if activity != AgentActivity::Unknown \|\| probe.name == AGENT_NAME_CLAUDE` (주석 `commands.rs:384` — codex/gemini 만 구형 폴백 유지) |

### D. 프론트 — 에이전트별 분기는 설정 UI 한 곳뿐

- 배지: `src/features/project/agent-status-badge.tsx:9-14` 는 **`AgentActivity` 키**(working/awaitingInput/idle/unknown)만 쓴다. **에이전트별 아이콘·배지 분기 없음** → 새 에이전트 추가 시 무변경.
- 설정 UI: `src/widgets/settings-view/agent-hooks-project-list.tsx:19-23` 의 `AGENT_HOOKS_AGENTS` 리터럴 배열(name·labelKey·scope). Claude 만 프로젝트별 행(`:41`), 나머지는 사용자 레벨 단일 행.
- 토글: `src/features/settings/agent-hooks-toggle.tsx` 는 전역 on/off 하나 — 에이전트 분기 없음.
- 문구: `src-tauri/resources/locales/{en,ko,ja}.json:575` 의 `settings.agentHooksAgentGemini` 등 3키.

---

## 핵심 질문별 답

### Q1. 새 에이전트 1종 추가 시 손대야 할 파일 체크리스트

**Rust — 신원만 (최소, S)**
1. `src-tauri/src/domain/agent/types.rs:8` — `KNOWN_AGENT_NAMES` 에 이름 추가
2. `types.rs:69-71` — `AGENT_NAME_X` 상수
3. `service.rs:23` — CLI 가 bun/node 셸 스크립트면 `NODE_RUNTIME_NAMES` 확인(현재 node·bun·deno 커버)

**Rust — 활동 신호까지 (M)**

4. `service.rs:406·408-413` — `X_DIALOG_SIGNATURES` + `dialog_signatures_for` 팔 추가
5. `service.rs:460` `parse_title_glyph` — **시그니처 변경 필요**(`agent_name` 인자 추가) + 호출부 `service.rs:534`. opencode 처럼 글리프 없는 타이틀은 현행 파서로 판정 불가
6. `service.rs:399` `NON_SUBSTANTIVE_GLYPHS` — 새 에이전트 스피너 글리프 실측 후 추가(누락 시 스피너가 "실질 출력"으로 읽혀 영구 Working)

**Rust — 훅 브리지까지 (L)**

7. `types.rs:92-107` — `X_MANAGED_HOOK_EVENTS`(+ 필요한 `HOOK_EVENT_*` 상수)
8. `types.rs:60-61` — 훅 타임아웃 상수(초/ms 단위 확인)
9. `service.rs:692-714` — `map_hook_event_to_activity` 팔
10. `service.rs:720-727` — `hook_scope_for_agent` 팔
11. `service.rs:732-739` — `managed_hook_events_for` 팔
12. `service.rs:847-853` — `user_level_hook_command_timeout` 팔
13. `service.rs:855-869` — `user_level_hooks_path` + `X_HOME_RELATIVE_PATH`
14. `service.rs:755-820` — 인밴드 방식을 쓰면 `CLAUDE_HOOK_BINDINGS`/`claude_agent_event_payload`/`build_claude_agent_hook_command`/`claude_hook_entries` 를 **agent 파라미터화**(현재 함수명·페이로드 상수가 claude 고정)
15. `hooks.rs:121`·`hooks.rs:158` — reconcile/remove 루프 배열
16. `commands.rs:392` — 휴리스틱 폴백 게이트의 claude 예외 조건 재검토

**프론트 (S)**

17. `src/widgets/settings-view/agent-hooks-project-list.tsx:19-23` — `AGENT_HOOKS_AGENTS` 한 줄
18. `src-tauri/resources/locales/en.json` / `ko.json` / `ja.json` (각 :575 인근) — `settings.agentHooksAgentX`
19. 배지·아이콘: **변경 없음**(`agent-status-badge.tsx:9-14` 는 activity 키)

**문서 (S)**

20. `docs/features/agent-integration.md` §1.1·§4 표, `docs/ipc-contract.md:621-647` agent 절, 새 acknowledge 계약

> 최소 신원만이면 **3파일 5줄**, 인밴드 훅까지 가면 **Rust 4파일 + 프론트 1 + locale 3 + 문서 2**.

---

### Q2. 세 에이전트의 정체와 네이티브 이벤트 기제

#### (a) opencode — 1차 출처: 설치된 바이너리(`/Users/gkn/.opencode/bin/opencode`, v1.18.29) + `opencode.ai/docs/plugins`

| 항목 | 값 | 출처 등급 |
|---|---|---|
| 프로세스명 | `opencode` (Mach-O 64-bit arm64, Bun 단일 실행파일 — JS 번들 내장) | 1차(로컬 바이너리) |
| 이벤트 시스템 | 플러그인 훅 `pi.on` 아님 — Bus 이벤트. 확인된 식별자: `session.idle`, `session.status`, `permission.asked`, `permission.replied`, `tool.execute.before`, `tool.execute.after`, `session.error`, `file.edited`, `message.updated` 등 | 1차(바이너리 strings + 공식 docs) |
| 권한 프롬프트 UI 문구 | `{optionId:"once", kind:"allow_once", name:"Allow once"}`, `{optionId:"always", kind:"allow_always", name:"Always allow"}`, `{optionId:"reject", kind:"reject_once", name:"Reject"}` / 다이얼로그 옵션 `{once:"Allow once", always:"Allow always", reject:"Reject"}`, `escapeKey:"reject"` | 1차(바이너리) |
| 터미널 타이틀 | **설정한다.** home 화면 `"OpenCode"`, 세션 `` `OC | ${title}` ``(40자 초과 시 37자+…), 플러그인 `` `OC | ...` ``. env `OPENCODE_DISABLE_TERMINAL_TITLE` 로 끌 수 있음. **선행 글리프 없음** → 작업/유휴 구분 불가 | 1차(바이너리) |
| 설정 파일 | `<cwd>/opencode.json`·`opencode.jsonc`, `<cwd>/.opencode/opencode.json(c)`, `~/.config/opencode` | 1차(바이너리 `P0()` 함수) |
| 플러그인 경로 | 공식 docs 표기: `.opencode/plugins/`, `~/.config/opencode/plugins/`; `opencode.json` 의 `"plugin": [...]` 로 npm 패키지 지정 | 1차(공식 docs) |
| 대체 신호 경로 | 로컬 HTTP 서버 존재 — `session.permission.request.list`("List pending permission requests"), `permission.reply`("Approve or deny a permission request"), `session.permission.create` 라우트 | 1차(바이너리) |

**TAIDE 관점 평가**: `permission.asked` → `AwaitingInput`, `permission.replied` → 해제, `session.idle` → `Idle` 로 1:1 매핑 가능. **Claude 보다 신호 품질이 좋다**(6초 타이머 없음). 타이틀은 신원 보조는 되지만 활동 판정에는 쓸 수 없다.

#### (b) codex — 1차 출처: 설치된 네이티브 바이너리(`@openai/codex` 0.144.6, `vendor/aarch64-apple-darwin/bin/codex`)

| 항목 | 값 |
|---|---|
| 프로세스명 | node 셸(`bin/codex.js`, `#!/usr/bin/env node`) → 네이티브 Rust 바이너리 `codex` spawn. **comm 이 `node` 또는 `codex` 둘 다 가능** → TAIDE 의 node-cmdline 폴백이 이미 커버 |
| 훅 시스템 | **존재.** 바이너리 내 `"hooks": "./hooks.json"`, `failed to serialize hooks.json`, 로거 필드 `hook.event_name`·`hook.handler_type`·`hook.execution_mode`·`hook.scope`·`hook.source`·`hook.command_outcome` |
| 훅 이벤트 전체 | `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`, `SessionStart`, `UserPromptSubmit`, `SubagentStart`, `SubagentStop`, `Stop` (kebab 별칭 `pre-tool-use`…`stop` 병존) |
| 핸들러 종류 | `ConfiguredHookHandler::{Prompt, Agent, Command}` — 필드 `matcher`, `timeoutSec`, `enabled`, `async`, `asyncRewake`, `statusMessage` |
| notify(별도) | `legacy_notify` + 이벤트명 `agent-turn-complete`, 페이로드 필드 `thread-id`·`turn-id`·`cwd`·`input-messages`·`last-assistant-message` |
| 승인 프롬프트 | app-server 프로토콜 쪽: `item/permissions/requestApproval`(`PermissionsRequestApprovalParams`), `execCommandApproval`, `applyPatchApproval`. TUI 표시 문구는 별도 확인 필요 |
| TAIDE 현행과의 차이 | TAIDE 가 쓰는 4종(`UserPromptSubmit`/`PermissionRequest`/`PostToolUse`/`Stop`, `types.rs:101-106`)은 **전부 실재 확인**. 다만 TAIDE 는 **HTTP command hook**(`hooks.rs:121`)이고 codex 는 command 훅이 stdout JSON 규약을 갖는다 → claude 와 같은 **인밴드 OSC 777 전환이 가능**(계약 무변경, 서버·토큰 제거) |

`docs/config.md` 에 `notify`·hooks 상세가 없고 `allow_managed_hooks_only`(requirements.toml) 만 언급된다 → 공식 문서 커버리지가 바이너리보다 뒤처져 있으므로 **바이너리를 1차 출처로 삼는 현행 방침이 맞다**.

#### (c) pi — 정체 확정

| 항목 | 값 |
|---|---|
| 정체 | `badlogic/pi-mono` 의 코딩 에이전트. **저장소가 `earendil-works/pi` 로 이관**, npm 패키지도 `@mariozechner/pi-coding-agent` → **`@earendil-works/pi-coding-agent`** 로 개명 |
| CLI 바이너리 | `pi` (`npm install -g --ignore-scripts @earendil-works/pi-coding-agent`) |
| 설정 경로 | 전역 `~/.pi/agent/settings.json`, 프로젝트 `.pi/settings.json`. 컨텍스트 `~/.pi/agent/AGENTS.md`·`.pi/AGENTS.md`, 세션 `~/.pi/agent/sessions/` |
| 확장(훅) 경로 | `~/.pi/agent/extensions/*.ts`(또는 `*/index.ts`), `.pi/extensions/*.ts`. CLI `pi -e ./path.ts`. `settings.json` 의 `"extensions": [...]`·`"packages": [...]` |
| 이벤트 API | `pi.on(eventName, handler)` — `session_start`(reasons: startup/reload/new/resume/fork), `session_shutdown`, `before_agent_start`, `agent_start`, `agent_end`, `agent_settled`, `turn_start`, `turn_end`, `tool_execution_start/update/end`, `tool_call`(블로킹 가능), `tool_result`, `input`, **`ui_prompt_start`/`ui_prompt_end`("User interaction blocking spans")**, `project_trust`, `model_select` |
| 유휴 판정 | `ctx.isIdle()` / `ctx.waitForIdle()` — 자동 재시도·auto-compaction·큐 대기까지 포함한 "완전 정착" |
| 권한 프롬프트 | 툴 승인 전용 다이얼로그 문구는 README 에 없음. 확인된 것은 **프로젝트 신뢰** 프롬프트(`--approve`/`-a`, `--no-approve`/`-na`, `/trust` → `~/.pi/agent/trust.json`) |

**TAIDE 관점**: `ui_prompt_start`/`ui_prompt_end` 가 `AwaitingInput` 래치 ON/OFF 에 정확히 대응하고 `agent_start`/`agent_settled` 가 Working/Idle 에 대응한다. **세 후보 중 이벤트 모델이 TAIDE 신호 축과 가장 깔끔하게 맞는다.** 단 확장이 TS 파일이라 TAIDE 가 설치하려면 `.ts` 파일을 써 넣어야 한다(JSON 한 줄이 아님).

---

### Q3. 이 Mac 의 설치 현황과 실측 탐침 준비물

| 에이전트 | 설치 | 경로 / 버전 |
|---|---|---|
| **opencode** | O | `/Users/gkn/.opencode/bin/opencode`, `~/.bun/bin/opencode` — **v1.18.29**. 설정 `~/.config/opencode/opencode.json` 존재 |
| **codex** | O | `~/.nvm/versions/node/v22.14.0/bin/codex`(node 셸), `~/.bun/bin/codex` — **codex-cli 0.144.6**. `~/.codex/config.toml`·`auth.json` 존재 |
| **claude** | O | `~/.local/bin/claude`, `~/.bun/bin/claude` |
| **gemini** | **X** | `which` 미검출 |
| **pi** | **X** | 바이너리 없음. `~/.pi/agent/` 는 존재하나 내용은 사용자 작성 `AGENTS.md`·`llm-rules` 뿐(설치 흔적 아님) |

**실측 탐침(`docs/debugging.md` §4, expect + pty 캡처) 준비물**

| 에이전트 | 로그인/키 | 비고 |
|---|---|---|
| opencode | 기존 인증 재사용 가능성 높음(`~/.config/opencode` 존재). 미인증이면 provider 인증 필요 — **키 이름만**: `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` 또는 `opencode auth login` 구독 로그인 | 탐침 목표: ① 권한 다이얼로그 정규화 텍스트(`Allow once`/`Allow always`/`Reject` 가 실제 pty 에 어떤 형태로 그려지는지 — Claude 처럼 `CSI n G` 삽입 여부) ② 타이틀 OSC 실제 시퀀스(`OC | …`) ③ 작업 중 스피너 글리프 |
| codex | `~/.codex/auth.json` 존재 → 추가 로그인 불필요 추정 | 탐침 목표: ① 승인 프롬프트 TUI 문구 ② 타이틀 설정 여부(현재 **미확인**) ③ `hooks.json` command 훅이 `/dev/tty` 쓰기 가능한지(인밴드 전환 타당성) |
| pi | **설치 필요**(`npm i -g --ignore-scripts @earendil-works/pi-coding-agent`) + provider 인증(구독 로그인 지원: Claude Pro/Max, ChatGPT Plus/Pro, GitHub Copilot) | 설치 승인 없이는 탐침 불가 |
| gemini | 설치 필요 | 현재 TAIDE 가 지원하지만 이 기기에서 회귀 검증 불가 |

---

## 설계 옵션

| 옵션 | 내용 | 장점 | 단점 | 영향 파일 | 규모 |
|---|---|---|---|---|---|
| **A. 신원만 확대** | `KNOWN_AGENT_NAMES` 에 `opencode`·`pi` 추가. 활동은 실질 출력/무출력 4초만 | 3파일 5줄. 사이드바에 세션이 "보이기는" 함 | `AwaitingInput` 구조적 불가 — d-54 가 고친 바로 그 증상이 새 에이전트에서 재현 | `types.rs:8,69-71` | S |
| **B. A + 다이얼로그 시그니처** | opencode `Allow once`/`Reject` 등을 `dialog_signatures_for` 에 실측 후 추가 | 훅 설치 없이 `AwaitingInput` 획득. opt-in 불필요 | 실측 필수(추측 금지). 문구는 i18n 대상이라 **로케일 바뀌면 깨진다**(opencode 는 `ui.message.interrupted` 등 다국어 번들 확인됨) | +`service.rs:406-413`, `service.rs:399` | M |
| **C. 인밴드 OSC 777 통일** | claude 훅 빌더를 agent 파라미터화하고 opencode 플러그인(JS)·pi 확장(TS)·codex `hooks.json` 이 모두 같은 OSC 777 을 뱉게 | 세션 단위 정확도, 서버·토큰·cwd 매칭 전부 제거(claude 와 동일 이점). codex 도 HTTP → 인밴드로 승격 | 설치물이 에이전트마다 형식이 다름(JSON / JS 플러그인 / TS 확장). 신뢰 경계 문제는 §4.1 과 동일 등급 | `service.rs:755-820` 리팩터, `hooks.rs:92-170`, `types.rs`, 설정 UI, locale | L |
| **D. opencode 로컬 HTTP/SSE 구독** | 플러그인 설치 없이 opencode 서버의 permission/이벤트 API 를 구독 | 사용자 파일을 안 건드림 | 포트 발견 수단 불명, 세션↔pty 매칭 수단 불명, 비공개 API 의존 | 신규 infra | L |
| **E. 타이틀 파서 에이전트화** | `parse_title_glyph(agent_name, title)` 로 시그니처 변경 | opencode 처럼 글리프 없는 타이틀을 "신원 확인용"으로만 쓰고 오판 방지. C 없이도 필요 | 호출부·테스트 전면 수정(테스트 20+건이 `parse_title_glyph`/`apply_scan_to_signals` 를 직접 호출) | `service.rs:460,534` + 테스트 | M |

권장 조합: **A + E 를 먼저**(오판 차단), 그 다음 **B(opencode 실측 후)**, 훅은 **C 를 codex→opencode→pi 순**으로.

---

## 리스크·미확인

### 리스크
1. **`pi` 는 2글자 프로세스명** — `match_agent_name_in`(`service.rs:49-53`)은 완전일치라 `node /x/y/pi` 는 잡지만, 인자로 `pi` 라는 파일명이 오는 임의 명령도 오탐한다. 이름 기반 감지만으로는 위험.
2. **`parse_title_glyph` 가 에이전트 무관**(`service.rs:460`, 호출부 `:534`) — opencode 타이틀 `OC | …` 는 `◐◑✳` 중 무엇도 아니라 `None` 이 되어 무해하지만, 앞으로 첫 글자가 겹치는 에이전트가 나오면 조용히 오판한다.
3. **`NON_SUBSTANTIVE_GLYPHS`(`service.rs:399`)가 Claude 스피너 기준** — 새 에이전트 스피너 글리프가 빠지면 스피너 프레임이 "실질 출력"으로 읽혀 **영구 Working + 차단 래치 상시 해제**가 된다. 이건 배지가 아예 안 움직이는 증상이라 눈에 잘 안 띈다.
4. **opencode 권한 문구는 i18n 번들** — `Allow once` 는 영어 로케일 전제. 시그니처 테이블이 로케일 의존이 되면 B 안의 신뢰도가 낮아진다.
5. **codex HTTP 훅이 현재 `commands.rs:392` 의 폴백 예외에서 빠져 있다** — codex/gemini 는 구형 휴리스틱 폴백을 계속 쓴다. 신규 에이전트를 어느 쪽에 넣을지 명시하지 않으면 판정 경로가 갈린다.
6. **pi 패키지 개명** — `@mariozechner/pi-coding-agent` 로 문서화된 자료가 남아 있으나 현재 정본은 `@earendil-works/pi-coding-agent`. 옛 이름으로 설치 안내를 쓰면 stale.

### [미확인]
- codex 가 터미널 타이틀(OSC 0/2)을 설정하는지 — 바이너리 strings 에서 확인 못 함.
- codex TUI 의 실제 승인 프롬프트 화면 문구(프로토콜 레벨 `PermissionsRequestApprovalParams` 만 확인).
- codex `hooks.json` 의 정확한 스키마(최상위 키 배치, `matcher` 형식) — 바이너리에 필드명은 있으나 전체 문서 없음. `openai/codex` `docs/` 에 `hooks.md` 자체가 없다(404, 디렉터리 목록 15개 파일에 미포함).
- opencode 플러그인 디렉터리가 `plugin/` 인지 `plugins/` 인지 — 공식 docs 페이지는 `.opencode/plugins/`·`~/.config/opencode/plugins/` 로 표기했으나 바이너리에서 교차확인 못 함.
- opencode 플러그인에서 **터미널(pty)로 직접 바이트를 쓸 수 있는지** — OSC 777 인밴드 전략의 전제. `process.stdout` 접근 가능 여부 미확인.
- opencode 가 각 세션의 pty 를 소유하는지(TUI 단일 프로세스 + 서버 분리 구조라면 훅 출력이 사용자가 보는 pty 로 안 갈 수 있음).
- pi 의 프로세스명이 `pi` 인지 `node`+cmdline 인지, 터미널 타이틀 설정 여부, 툴 승인 프롬프트 문구.
- pi 확장에서 `ui_prompt_start` 가 **툴 승인**에도 발화하는지(문서는 "User interaction blocking spans" 라고만 함).
- gemini 지원의 현행 동작 — 이 기기에 미설치라 회귀 검증 불가.

---

## 결정이 필요한 지점

1. **지원 범위** — ① opencode 만 / ② opencode + codex 개선 / ③ opencode + pi / ④ 셋 전부. (pi 는 설치가 선행되어야 하고, gemini 는 이 기기에서 검증 불가)
2. **신원만 vs 활동까지** — A(3파일 5줄, 배지는 Working/Idle 만) 로 먼저 출시할지, B/C 까지 묶어서 갈지.
3. **pi 설치 승인** — `npm i -g --ignore-scripts @earendil-works/pi-coding-agent` 를 이 기기에 설치해도 되는지. 미설치면 pi 는 전부 `[미확인]` 으로 남는다.
4. **gemini 설치 승인** — 기존 지원의 회귀 검증을 하려면 필요.
5. **다이얼로그 시그니처의 로케일 정책** — 영어 로케일만 지원할지, opencode 번들의 다국어 키(`permission.*`)를 전부 표에 넣을지, 아니면 B 를 건너뛰고 C(훅)로 직행할지.
6. **codex 를 HTTP → 인밴드로 옮길지** — 계약 변경 없이 서버·토큰·override 충돌을 제거할 수 있으나, `hooks.json` 스키마 실측과 회귀 검증이 필요하다. 이번 배치에 포함할지.
7. **`parse_title_glyph` 시그니처 변경(E안)** — 테스트 20여 건이 함께 바뀐다. 지금 할지, 실제 오판이 관측될 때 할지.
8. **탐침 실행 방식** — expect pty 캡처를 이 세션에서 돌릴지(앱 실행 금지 제약과는 별개로 CLI 실행은 허용 범위인지 확인 필요), 별도 배치로 뺄지.

---

## T5 — Dock 우클릭 Recent(항목 6) · 사이드바 + 버튼 메뉴(경로로 열기/Finder 로 열기)(항목 10)

## 현행(근거)

### 앱 메뉴 (Rust 단독 소유)

| 항목 | 현행 | 근거 |
|---|---|---|
| 메뉴 구성 | `TAIDE` / `Edit` / `Window` 3개 서브메뉴만. **File 메뉴 없음, Open Recent 없음** | `src-tauri/src/domain/window/commands.rs:164,176,186,193` |
| 문자열 | 전부 하드코딩 영문(`"TAIDE"`, `"Edit"`, `"Window"`, `"Quit"`) — 로케일 미연동 | `src-tauri/src/domain/window/commands.rs:160,164,176,186` |
| 설치 시점 | `setup()` 에서 1회 `app.set_menu(...)`. 반환된 `Menu` 핸들을 보관하지 않음 | `src-tauri/src/lib.rs:571` |
| 이벤트 라우팅 | `on_menu_event(handle_menu_event)` 하나. 본문은 `MENU_ID_QUIT` 외 전부 early return | `src-tauri/src/lib.rs:540`, `src-tauri/src/domain/window/commands.rs:269-276` |
| 사용자 정의 id | `const MENU_ID_QUIT: &str = "taide-quit"` (문자열 id 관례 이미 존재) | `src-tauri/src/domain/window/commands.rs:155` |

### 최근 프로젝트 데이터

| 항목 | 현행 | 근거 |
|---|---|---|
| 커맨드 | `project_list_recent(state) -> Vec<Project>`, 읽기 전용·`begin_mutation` 불필요 | `src-tauri/src/domain/project/commands.rs:39-41` |
| 서비스 | `list_recent_projects` — `projects/` 디렉토리 전수 스캔, 파싱 실패 레코드는 skip | `src-tauri/src/domain/project/service.rs:417-427` |
| 정렬 | `last_opened_at` 내림차순, **상한 없음(전량 반환)** | `src-tauri/src/domain/project/service.rs:426` |
| rootMissing | 조회 시점에 `!Path::is_dir()` 로 재계산 → 사라진 폴더도 목록에 남고 비활성 표시 | `src-tauri/src/domain/project/service.rs:421` |
| 상한 위치 | FE 가 `slice(0, 8)` 로 자름 (`RECENT_PROJECT_DISPLAY_LIMIT = 8`) | `src/widgets/welcome/welcome-container.tsx:23,62` |
| 원격 차단 | `project_list_recent` 은 원격 비도달(`RemoteDenialPolicy::LocalProjectHistoryExposure`) | `src-tauri/src/domain/project/commands.rs:35-36` |
| 기록 삭제 | `close_project` 는 세션에서만 제거, `projects/<id>/` 디스크 레코드는 남김 → **"Clear Recent" 백엔드 없음** | `src-tauri/src/domain/project/service.rs:120-132` |

### 사이드바 + 버튼

| 항목 | 현행 | 근거 |
|---|---|---|
| 버튼 | `IconButton` + `Plus`, `onClick={handleOpenProject}` 즉시 Finder 폴더 피커 | `src/widgets/app-sidebar/app-sidebar.tsx:78-84` |
| 핸들러 | `useOpenFolderDialog()` = `plugin-dialog open({directory:true})` → `useOpenProject` mutate → 실패 시 `toast.error(describeIpcError(...))` | `src/entities/project/project.query.ts:49-58` |
| 라벨 | `t('sidebar.openFolderAriaLabel')` = "폴더 열기" | `src/widgets/app-sidebar/app-sidebar.tsx:79`, `src-tauri/resources/locales/ko.json:857` |

### 재사용 가능한 FE 자산

| 자산 | 경로 | 비고 |
|---|---|---|
| `DropdownMenu` 래퍼 | `src/shared/ui/dropdown-menu.tsx` | radix-ui. 사용처 3곳 |
| **+ 버튼 → 드롭다운 정본 패턴** | `src/features/tab/tab-bar-add-menu.tsx:24-33` | `DropdownMenu > Tooltip > TooltipTrigger asChild > DropdownMenuTrigger` 중첩 순서까지 그대로 베낄 수 있음 |
| 텍스트 입력 다이얼로그 패턴 | `src/features/git/create-tag-dialog.tsx:23-34` | open 토글 시 렌더 중 state 리셋 가드 포함 |
| 프로젝트 다이얼로그 | `src/features/project/project-display-dialog.tsx` | `Dialog/DialogContent/DialogFooter` 구성 |
| Finder 노출 | `system_reveal_path` — **열린 프로젝트 루트 내부 경로만 허용**(`resolve_within_open_project` 게이트) | `src-tauri/src/domain/system/commands.rs:195-213` |

### 경로 검증 (project_open)

| 단계 | 에러 | 근거 |
|---|---|---|
| `canonicalize` | io 에러 그대로(NotFound 등), 로케일 키 없음 | `src-tauri/src/domain/project/service.rs:57` |
| 디렉토리 여부 | `AppErrorKind::InvalidArgument` + `"error.project.pathNotDirectory"` + arg `path` | `src-tauri/src/domain/project/service.rs:61-68` |
| 읽기 권한 | `read_dir` io 에러 | `src-tauri/src/domain/project/service.rs:69` |
| `~` 확장 | **어디에도 없음.** `HOME` 조회는 agent 도메인에만 존재 | `src-tauri/src/domain/agent/commands.rs:592-594` |

### 로케일 키 추가 비용

신규 키 1개 = ① `MESSAGE_NAMESPACES` 배열(`src-tauri/src/domain/locale/service.rs:13`) ② `en.json` ③ `ko.json` ④ `ja.json`. 3종 동일 키 집합을 강제하는 테스트가 있음(`src-tauri/src/domain/locale/service.rs:1343`).

---

## 핵심 질문별 답

### Q1. Tauri 2.11.5 에 macOS Dock 메뉴 API 가 있는가 → **없다**

| 계층 | 조사 결과 | 근거 |
|---|---|---|
| `tauri` 2.11.5 | `set_dock_visibility` 만 존재. `set_dock_menu`·dock menu 관련 API 전무 | `tauri-2.11.5/src/app.rs:660,1307` (grep `dock` 전체 결과에 메뉴 없음) |
| `tauri` macOS 메뉴 헬퍼 | `set_as_windows_menu_for_nsapp` / `set_as_help_menu_for_nsapp` 2개뿐 | `tauri-2.11.5/src/menu/plugin.rs:810,827` |
| `muda` 0.19.3 | `init_for_nsapp`(메인 메뉴바), `set_as_windows_menu_for_nsapp`, `set_as_help_menu_for_nsapp`. dock 항목 없음 | `muda-0.19.3/src/platform_impl/macos/mod.rs:211,718,728` |
| `tao` 0.35.3 앱 델리게이트 | 등록 셀렉터 8개(`applicationDidFinishLaunching:`·`applicationWillTerminate:`·`application:openURLs:`·`applicationShouldHandleReopen:hasVisibleWindows:` 등). **`applicationDockMenu:` 미등록** | `tao-0.35.3/src/platform_impl/macos/app_delegate.rs:57-84` |
| AppKit 자체 | `applicationDockMenu:` 는 **NSApplicationDelegate 프로토콜 메서드**로만 존재 — `NSApplication` 에 public setter 없음 | `objc2-app-kit-0.3.2/src/generated/NSApplication.rs:1213-1215` |

즉 Dock 메뉴는 **델리게이트에 셀렉터를 심는 것 외에 경로가 없고**, 그 델리게이트 클래스(`TaoAppDelegateParent`)는 tao 가 런타임에 `ClassBuilder` 로 만들어 소유하며 앱 코드용 확장 지점을 노출하지 않는다(`tao-0.35.3/src/platform_impl/macos/app_delegate.rs:47-52,86`).

우회 경로 두 가지를 확인했고, 둘 다 프로젝트 컨벤션(ai-process §6.2 "HACK·우회 금지")에 정면으로 걸린다:

- **런타임 메서드 주입**: `objc2` 로 `NSApp.delegate` 의 클래스에 `class_addMethod` 로 `applicationDockMenu:` 추가. `objc2`/`objc2-app-kit`/`objc2-foundation` 을 직접 의존성으로 추가해야 함(현재 `src-tauri/Cargo.toml` 에 없음, 레지스트리에는 tao/muda 경유로 0.5·0.6 과 0.2·0.3 이 함께 존재).
- **tauri 메뉴 객체 재사용**: muda 는 `ContextMenu::ns_menu()` 로 raw `NSMenu` 포인터를 노출하지만(`muda-0.19.3/src/menu.rs:493-496`), tauri 는 이를 `pub(crate) sealed` 안에 봉인해 앱 코드에서 도달 불가(`tauri-2.11.5/src/menu/mod.rs:745-759`). 따라서 tauri 의 `Submenu` 를 Dock 메뉴로 그대로 쓸 수 없고, `muda` 를 직접 의존성으로 추가해 별도 메뉴를 짓거나 NSMenu 를 직접 조립해야 한다.

**판정: 항목 6(Dock Recent)은 현재 스택에서 정공법이 없다.** 메뉴바 `Open Recent`(Q2)로 대체하거나, 상류(tao/tauri)에 `applicationDockMenu:` 지원을 올리는 것이 정도다.

메뉴 이벤트 → 프로젝트 열기 경로는 (Dock 을 포기하고 메뉴바로 가더라도) **Rust 서비스 직접 호출이 정답**이다. 이미 같은 패턴이 있다: `dispatch.rs:863` 이 `project::project_open(app.clone(), app.state(), path).await` 로 커맨드 함수를 직접 부른다. 프론트 이벤트 emit 후 `useOpenProject` 를 태우는 우회는 불필요하고(창이 0개인 상태에서도 동작해야 함), `project_open` 이 이미 `ProjectOpened`/`ProjectListChanged` fanout 과 active 전파를 자체 수행한다(`src-tauri/src/domain/project/commands.rs:60-66`).

동적 갱신은 `AppHandle::menu()` 로 현재 메뉴를 되받아(`tauri-2.11.5/src/app.rs:952`) 해당 서브메뉴의 `items()`/`remove_at()`/`append()` 로 교체 가능하다(`tauri-2.11.5/src/menu/submenu.rs:253,305,314,337`). 갱신 트리거는 `project_open`·`project_close`·`project_activate` 세 커맨드 뒤(= `last_opened_at` 이 바뀌는 지점, `service.rs:134-137` 참조).

### Q2. 메뉴바 Open Recent — 없음. 구조 제안

현재 File 메뉴 자체가 없다(`commands.rs:193` 의 3개 서브메뉴). 신설 시:

| 결정 | 제안 | 근거 |
|---|---|---|
| 위치 | `File` 서브메뉴 신설 후 `Open Folder…` / `Open Recent ▸` / (선택)`Clear Recent` | macOS 관례. `Window` 앞에 삽입 |
| 데이터 | `service::list_recent_projects(&state.paths)` 직접 호출(커맨드 래퍼 불필요) | `src-tauri/src/domain/project/service.rs:417` |
| 상한 | **10 권장**. FE Welcome 은 8 이지만 그 8 은 "자기 자신 제외 후"의 값이라 의미가 다름 | `src/widgets/welcome/welcome-container.tsx:61-62` |
| rootMissing | `MenuItemBuilder::enabled(false)` 로 비활성 — Welcome 의 `aria-disabled` 처리와 동형 | `src/features/welcome/welcome-screen.tsx:74-76` |
| id 규약 | `taide-recent:<projectId>` 접두 파싱. `MENU_ID_QUIT` 과 같은 문자열 id 관례 | `src-tauri/src/domain/window/commands.rs:155,271` |
| 빌더 공용화 | Dock 을 포기하면 공용화 대상이 없음 — 단일 `build_recent_submenu(handle, &[Project])` 하나면 충분 | — |
| Clear Recent | **백엔드 없음.** `close_project` 는 디스크 레코드를 남긴다 → 신규 커맨드(`project_forget_recent` 등)로 `projects/<id>/` 삭제 필요 | `src-tauri/src/domain/project/service.rs:120-132` |

신규 커맨드를 만들면 `dispatch.rs` 의 허용/거부 테이블 등록이 **강제**된다 — 두 테이블의 합집합이 전체 커맨드와 일치하는지 검증하는 테스트가 있음(`src-tauri/src/domain/remote/dispatch.rs:1751,1770,1943`). `docs/ipc-contract.md` §3 project 절(119행~) 갱신도 함께.

### Q3. + 버튼 드롭다운 전환

`tab-bar-add-menu.tsx` 를 그대로 복제하면 Tooltip↔DropdownMenuTrigger 중첩 문제까지 해결된다. 단 현재 `IconButton` 은 자체 Tooltip 을 품고 있어(`src/shared/ui/icon-button.tsx:30-46`) `DropdownMenuTrigger asChild` 와 겹치므로, **IconButton 을 쓰지 말고 tab-bar-add-menu 의 명시 조립을 따르는 쪽**이 안전하다.

| 메뉴 항목 | 동작 | 재사용 | 신규 로케일 키(제안) |
|---|---|---|---|
| Finder 로 열기 | 현행 `useOpenFolderDialog()` 그대로 | `src/entities/project/project.query.ts:49` | `sidebar.openFolderViaFinder` |
| 경로로 열기… | 입력 다이얼로그 → `useOpenProject` mutate | `create-tag-dialog.tsx` 패턴 | `sidebar.openByPath`, `sidebar.openByPathPlaceholder`, `sidebar.openByPathTitle` |
| (트리거 aria-label) | 기존 `sidebar.openFolderAriaLabel` 재활용 or `sidebar.addProjectMenu` 신설 | `ko.json:857` | — |

"Finder 로 열기"가 만약 **기존 프로젝트 루트를 Finder 에 노출**하는 뜻이라면 얘기가 달라진다: `system_reveal_path` 는 `resolve_within_open_project` 게이트 때문에 **열려 있지 않은 최근 프로젝트 경로를 거부**한다(`src-tauri/src/domain/system/commands.rs:195-213`). 그 경우 게이트 완화 또는 별도 커맨드가 필요하다. → 결정 지점 D3.

경로 검증은 전부 `project_open` 안에 이미 있다(디렉토리 아님 → `error.project.pathNotDirectory`, 그 외 io 에러). FE 는 `describeIpcError` 로 그대로 토스트하면 되고 사전 검증을 새로 만들 필요 없다. **단 `~` 확장은 전 코드베이스에 없으므로** 입력 다이얼로그를 만들면 반드시 별도로 처리해야 한다(FE 에서 `~/` → 홈으로 치환하거나, Rust `open_project` 진입부에서 확장).

---

## 설계 옵션

### 항목 6 (Recent)

| 옵션 | 내용 | 장점 | 단점 | 영향 파일 | 규모 |
|---|---|---|---|---|---|
| **A (권장)** | Dock 포기, 메뉴바 `File > Open Recent` 만 신설 | 공개 API 만 사용, unsafe 0, 테스트 가능 | Dock 우클릭은 그대로 없음 | `window/commands.rs`, `project/service.rs`(읽기), 로케일 4곳 | **M** |
| B | objc2 런타임 주입으로 Dock 델리게이트 패치 | 요구사항 그대로 충족 | unsafe + 런타임 클래스 패치 = 컨벤션 위반, tao 업그레이드마다 깨질 위험, `muda`/`objc2` 직접 의존 추가 | A + `Cargo.toml` + 신규 `infra/macos_dock.rs` | **L** |
| C | tao/tauri 업스트림에 `applicationDockMenu:` 지원 PR | 근본 해결 | 일정 통제 불가 | 외부 | L(외부) |

### 항목 10 (+ 버튼)

| 옵션 | 내용 | 장점 | 단점 | 영향 파일 | 규모 |
|---|---|---|---|---|---|
| **A (권장)** | `app-sidebar` 의 IconButton → `features/project/sidebar-add-project-menu.tsx`(신규, tab-bar-add-menu 복제) + `open-project-by-path-dialog.tsx`(신규) | 기존 패턴 100% 재사용, FSD 준수(features=순수 UI, widget 이 mutation 소유) | 클릭 1회로 끝나던 폴더 열기가 2클릭으로 | `app-sidebar.tsx`, features 2개 신규, 로케일 4곳 | **S~M** |
| B | + 클릭은 기존대로 Finder, 우클릭에 ContextMenu 로 "경로로 열기" | 기존 1클릭 유지 | 발견 불가능한 UI | 동일 | S |

---

## 리스크 · 미확인

- **[미확인]** `Submenu::append`/`remove_at` 이 내부적으로 메인 스레드 마샬링을 하는지 정확히 확인하지 않았다(`tauri-2.11.5/src/menu/submenu.rs:253-316` 이 클로저 래핑 형태인 것만 확인). macOS 에서 NSMenu 변경은 메인 스레드여야 하므로, 갱신 호출을 `run_on_main_thread` 로 감쌀지 결정 전에 확인 필요.
- **[미확인]** 옵션 B 에서 `muda` 를 직접 의존성으로 추가했을 때 `MenuEvent` 전역 채널이 tauri 가 쓰는 인스턴스와 동일한지(= Dock 항목 클릭이 `on_menu_event` 로 오는지). cargo 가 muda 0.19.3 단일 버전으로 통일하면 성립할 가능성이 높지만 소스로 확인하지 않았다.
- **[커뮤니티 아님/미확인]** `applicationDockMenu:` 외에 Dock 메뉴를 설정하는 공식 경로가 정말 없는지는 Apple 공식 문서로 재확인하지 않았다(objc2 생성 바인딩에 setter 가 없다는 정황 근거만 있음).
- 메뉴바 문자열은 현재 전부 영문 하드코딩이라, Open Recent 만 로케일을 태우면 메뉴바 안에서 언어가 섞인다. 메뉴 전체 로케일화는 별건(규모 M) — `warm_builtin_catalogs` 는 이미 setup 에서 돌고 있어(`src-tauri/src/lib.rs:567`) 기술적 장애는 없다.
- `list_recent_projects` 는 매 호출마다 `projects/` 전량 디스크 스캔 + 각 레코드 `is_dir()` stat 이다(`service.rs:417-427`). 메뉴 갱신을 프로젝트 열기/닫기마다 돌리면 그 빈도로 스캔이 발생한다(프로젝트 수가 적으면 무시 가능).

---

## 결정이 필요한 지점

1. **D1 — Dock 메뉴**: 옵션 A(포기, 메뉴바로 대체) / B(objc2 런타임 패치) / C(업스트림) 중 무엇인가. 컨벤션상 A 를 권장한다.
2. **D2 — Open Recent 상한**: 10(macOS 관례) / 8(Welcome 과 통일) / 전량.
3. **D3 — "Finder 로 열기"의 정확한 의미**: (a) 폴더 피커 열기(현행 동작, 추가 백엔드 0) / (b) 최근 프로젝트 루트를 Finder 에 노출(= `system_reveal_path` 게이트 완화 또는 신규 커맨드 필요).
4. **D4 — Clear Recent**: 넣을 것인가. 넣으면 디스크 레코드를 삭제하는 신규 커맨드 + `dispatch.rs` 허용/거부 테이블 + `docs/ipc-contract.md` 갱신이 세트로 따라온다.
5. **D5 — 메뉴바 로케일화 범위**: Open Recent 만 번역할 것인가, `File`/`Edit`/`Window`/`Quit` 까지 한 번에 로케일화할 것인가.
6. **D6 — `~` 확장 위치**: FE 다이얼로그에서 치환할 것인가, Rust `open_project` 진입부에서 확장할 것인가(후자면 원격 dispatch 경로에도 적용된다).

---

## T6 — 퀵오픈(⌘P) 간헐적 "파일 못 찾음/안 열림" 근본 원인 조사

## 현행(근거)

퀵오픈은 **Rust 캐시 없는 1회성 전체 walk → TanStack Query 캐시 1장 → 클라 fuzzy 랭킹 → 상한 200 → `layout_open_tab` 선검증** 의 직선 경로다. 배치 3(`docs/acknowledge/2026-09-04-usability-batch3-contract.md` A절)이 고친 것은 **인덱스 신선도(무효화 3경로)** 와 **열기 선검증(`ensure_file_tab_target_exists`)** 두 축뿐이고, "후보 목록에 애초에 들어가지 않는 파일" 과 "랭킹 상한에서 잘리는 파일" 은 그 계약의 사정거리 밖에 있었다.

- 목록 소스: `command-palette.tsx:107-114` → `search.query.ts:27-32` → `search.ipc.ts:43` → `commands.rs:228 search_list_files` → `service.rs:592 list_project_files` → `build_walk(root, false)`(`service.rs:469`) → `configure_walk`(`service.rs:446-466`).
- 배치 3 계약이 **명시적으로 기각한 원인**: cmdk index 밀림 · 다중 루트 혼입 · 심링크/대소문자/NFC · 경계 검사 (`2026-09-04-usability-batch3-contract.md:44-46`). 단 그 기각은 **"열기(open)" 경로 한정**이며, "매칭(list/rank)" 경로는 검토 대상이 아니었다.
- 배치 3 계약이 **범위 밖으로 미루고 백로그에만 올린 결함**: `target:null` → `focusedPane` 소실 시 `NotFound("pane not found")` 토스트 (`2026-09-04-usability-batch3-contract.md:47-49`, `docs/backlog.md:154`). **현재도 미수정.**
- 문서/코드 불일치: `docs/features/command-palette.md:84-86` 은 "정렬: 점수 내림차순 → MRU → 알파벳, MRU 는 settings 에 저장"이라고 적었으나, 실제 `fuzzyFilter` 는 **점수 내림차순 단독 정렬**이고 MRU·알파벳 타이브레이크가 없다(`src/shared/lib/fuzzy-match.ts:105`). 이 상태에서 `FILE_RESULT_LIMIT = 200`(`command-palette.tsx:60`)으로 잘린다(`command-palette.tsx:291`).

---

## 핵심 질문별 답

### Q1. 경로 추적 — 단계별 실패 분기와 사용자에게 보이는 결과

| # | 단계 (파일:줄) | "못 찾음/무반응" 분기 | 사용자에게 보이는 것 |
|---|---|---|---|
| 1 | ⌘P 디스패치 `command-palette.tsx:221-227`(`'quick-open'`) | 보조(분리) 창에는 `CommandPalette` 자체가 미마운트 (`app/app.tsx:57-80` aux 분기 vs `:97` main 분기, 사유 주석 `:34-39`). `'quick-open'` 핸들러는 이 위젯이 유일 | **아무 일도 안 일어남** (팔레트조차 안 뜸, 토스트 없음) |
| 2 | 목록 fetch `command-palette.tsx:107-114` → `search.query.ts:27-32` | `enabled: open && mode==='files' && !!activeProjectId` — 활성 프로젝트 없음 | `CommandEmpty` → `palette.noResults` "결과가 없습니다" (`ko.json:445`) |
| 3 | Rust walk `search/service.rs:592-598` + `configure_walk:446-466` | ① `IGNORED_DIR_NAMES` 하드 제외 dir 내부 파일 전부 누락 (`constants.rs:1-14`: `.git` `node_modules` `target` `dist` `build` `.next` `.turbo` `.venv` `venv` `__pycache__` `.cache` `.idea` `.DS_Store`) — `respect_gitignore=false` 여도 이 목록은 **무조건** 적용 ② 심링크 파일 누락: `entry.file_type().is_file()` 필터(`service.rs:595`) + ignore 크레이트 기본 `follow_links: false`(`~/.cargo/registry/.../ignore-0.4.33/src/walk.rs:567`) → 심링크 엔트리는 `is_file()==false` ③ 비-UTF8 경로 제외 (`search/commands.rs:246-252`, 의도된 계약) | 해당 파일이 **목록에 아예 없음** → "결과가 없습니다" |
| 4 | `project_root(&state, &project_id)` (`search/commands.rs:230`) | 프로젝트 미오픈 → `NotFound("project not open")` | 쿼리 에러이나 팔레트는 `isError` 를 읽지 않음(`command-palette.tsx:107-114`) → 조용히 "결과가 없습니다" |
| 5 | 상대경로 게이트 `command-palette.tsx:279,290` | `activeProject`(PROJECT.DETAIL) 미로딩 → `filePaths=[]` | `common.loading` (`command-palette.tsx:309`) |
| 6 | 필터·랭킹 `command-palette.tsx:291` → `fuzzy-match.ts:96-106` | ① 코드포인트 **정확 비교**, 정규화 0 (레포 전체 `NFC/NFD/normalize(` grep 0건) ② 다중 토큰은 **모든 토큰이 각각 매칭**돼야 통과(`fuzzy-match.ts:70-84`) ③ 점수만으로 정렬 후 `slice(0, 200)` | 파일이 **목록에서 사라짐** → "결과가 없습니다" 또는 다른 파일만 보임 |
| 7 | 선택 `command-palette-files-group.tsx:43` → `command-palette.tsx:324-328 openFile` | `if (!activeProjectId) return` — **조용한 return** | **아무 반응 없음**(팔레트도 안 닫힘, 토스트 없음) |
| 8 | `useOpenFileTab` `layout.query.ts:136-152` | `isNotFoundIpcError` 면 인덱스 무효화 + `toast.error(describeIpcError)` | 토스트 |
| 9 | Rust 선검증 `layout/commands.rs:41-57, 59-72` | ① `resolve_owning_project_or_cli_opened` 실패 → `Forbidden`/`error.path.outsideOpenProjects` ② `ensure_existing_file` 실패 → `NotFound`/`error.file.notFound` (`root_guard.rs:99-110`) | ① "열린 프로젝트 루트 밖의 경로입니다: {path}" (`ko.json:158`) ② **"파일을 찾을 수 없습니다: {path}"** (`ko.json:99`) ← 사용자 보고 문구와 일치 |
| 10 | `open_tab_and_finish` `layout/service.rs:1317-1345` | `pane_id = target.unwrap_or_else(\|\| layout.focused_pane.clone())` (`:1331`) → `open_tab`(`service.rs:428-430`) 에서 pane 부재 시 `AppError::NotFound("pane not found: …")` — **localeKey 없음** | `describeIpcError` 원문 폴백(`ipc-error-message.ts:26-29`) → "pane not found: …" 토스트 **+ 퀵오픈 인덱스까지 무효화**(코드가 같은 `NotFound`) — `backlog.md:154` 에 등재된 미수정 결함 |
| 11 | 중복 탭 처리 `layout/service.rs:437-444` | 같은 pane 에 동일 `kind` 탭 존재 → **활성화만** 하고 revision+1 | 이미 활성 탭이면 화면 변화 0 → "안 열린다"로 체감 |
| 12 | preview 게이트 `layout/service.rs:1330` | `preview && settings.enable_preview_tabs` | 설정에 따라 기존 preview 탭이 **교체**됨(`service.rs:447-452`) |

### Q2. 가설 채점 (코드 근거)

| 가설 | 판정 | 근거 |
|---|---|---|
| (a) 인덱스 스테일 (워처 300ms·rescan 사이) | **가능(잔존 창 존재, 설계상 수용)** | 무효화 3경로는 `command-palette.md:96-134` 대로 동작하나, 팔레트가 닫힌 채 마킹된 인덱스는 **다음 ⌘P 첫 프레임에 낡은 배열을 그대로 렌더**(`file.query.ts:17-26`, `command-palette-files-group.tsx:16-23`). 워처 디바운스 300ms(`constants.rs:17`). `kind==='modified'` 는 무효화 안 함(`ipc-sync-provider.tsx:412`). 이 경우 증상은 정확히 "파일을 찾을 수 없습니다" 토스트 |
| (b) 한글 NFC/NFD 불일치 | **가능 — 단 "열기"가 아니라 "매칭"에서만** | 열기: 인덱스 문자열 = 디스크 바이트 그대로(`service.rs:592-598`) → `std::fs::metadata` 성공 ⇒ 배치 3 의 기각은 이 경로에선 타당. 매칭: `fuzzyMatch` 가 `for (const targetChar of target)` 로 **코드포인트 단위 정확 비교**(`fuzzy-match.ts:43-56`), 프론트/백 어디에도 유니코드 정규화 없음(레포 grep 0건) ⇒ 디스크 이름이 NFD(자모 분해)인 한글 파일은 IME 가 만드는 NFC 질의로 **영원히 0건**. 파일마다 생성 경로가 달라 프로젝트 내에서 섞이므로 "간헐적"으로 체감 |
| (c) 심링크·상대/절대·trailing slash 로 root_guard 거부 | **거의 불가(거부) / 가능(누락)** | 거부: 루트·경로 모두 `std::fs::canonicalize`/`canonicalize_lenient` 후 `starts_with`(`root_guard.rs:121-135`) → `/tmp`↔`/private/tmp` 류는 양쪽 정규화로 상쇄. 누락: 심링크 **파일**은 목록에 아예 안 들어감(Q1 #3-②) |
| (d) 프로젝트 전환 직후 이전 목록 표시 | **불가** | 쿼리 키가 `SEARCH.PROJECT_FILES(projectId)` 로 프로젝트별 분리(`query-key.ts:41`), `openFile` 도 같은 `activeProjectId` 를 씀(`command-palette.tsx:324-328`) ⇒ 교차 오염 경로 없음 |
| (e) 갱신 중 빈/부분 목록(경합) | **불가(부분) / 가능(빈)** | `search_list_files` 는 원자적 `Vec` 반환이라 부분 목록 없음. 다만 첫 fetch 중 `projectFiles===undefined` → `filePaths=[]`(`command-palette.tsx:290`) 인데, 이 창은 `common.loading` 로 덮여 있음(`:309`) ⇒ "없다"고 오인될 여지는 낮음 |
| (f) 결과 상한·랭킹으로 탈락 | **가능 — 유력** | `FILE_RESULT_LIMIT = 200`(`command-palette.tsx:60`), `.slice(0, FILE_RESULT_LIMIT)`(`:291`). 정렬은 **점수 단독**(`fuzzy-match.ts:105`) — 문서가 약속한 MRU·알파벳 타이브레이크 **미구현**(`command-palette.md:84-86`). 짧은 질의(1~3자)는 수천 건이 동점이 되고, `toSorted` 는 안정 정렬이라 **walk 순서가 곧 커트라인**이 된다. walk 순서는 `ignore::Walk` 의 readdir 순서(정렬 미지정, `service.rs:446-471`)라 재-walk 때마다 달라질 수 있어 **같은 질의가 어떤 때는 보이고 어떤 때는 안 보이는** 정확한 간헐 메커니즘 |
| (g) preview 탭 전환이 같은 파일 재열기를 무시 | **가능(무해에 가까움)** | `open_tab` 은 동일 `kind` 탭을 찾으면 활성화만 하고 반환(`layout/service.rs:437-444`). 이미 활성 상태였다면 화면 변화 0. 추가로 팔레트가 `closedByActionRef` 로 포커스 복원을 억제하므로(`command-palette.tsx:413-415`) 새 모델 attach 가 없으면 캐럿이 어디에도 안 붙어 "타이핑이 안 먹는다"로 체감될 수 있음 |
| (h-1) **보조 창에서 ⌘P 자체가 무동작** | **가능 — 확정 코드 경로** | `CommandPalette` 는 main 분기에만 마운트(`app/app.tsx:97`), aux 분기에는 없음(`:57-80`, 사유 주석 `:34-39`). `'quick-open'` 키맵 핸들러는 이 위젯이 유일 |
| (h-2) **`target:null` → `focused_pane` 소실 시 `NotFound("pane not found")`** | **가능 — 이미 등재된 미수정 결함** | `layout/service.rs:1331` + `:429`. 로케일 키가 없어 원문 토스트가 뜨고, `isNotFoundIpcError`(`ipc-error-message.ts:41`)가 코드만 보므로 `useOpenFileTab` 이 **파일 소실로 오인**해 인덱스까지 무효화(`layout.query.ts:145-149`). `backlog.md:154`, `2026-09-04-usability-batch3-contract.md:47-49` |
| (h-3) **하드코딩 무시 디렉토리** | **가능 — 유력** | `IGNORED_DIR_NAMES`(`constants.rs:1-14`)는 `respect_gitignore=false` 와 무관하게 항상 적용(`search/service.rs:456-465`). `dist/`·`build/`·`target/`·`.venv/`·`.cache/` 안의 파일은 **퀵오픈으로 절대 열 수 없다**. 사용자 입장에선 "어떤 파일은 되고 어떤 파일은 안 됨" = 간헐 |
| (h-4) `openFile` 의 조용한 조기 return | 가능(저빈도) | `command-palette.tsx:325` `if (!activeProjectId) return` — 토스트도 없고 팔레트도 안 닫힘 |

### Q3. 재현·계측 계획

**켜야 할 계측 (실행은 사용자 몫 — 본 조사는 읽기 전용)**

| 채널 | 방법 | 무엇이 특정되는가 |
|---|---|---|
| 파일 로그 | `~/Library/Logs/net.gumyo.taide.dev/TAIDE.log` (dev) / `net.gumyo.taide` (prod), 40KB 회전, `console.error/warn` + `unhandledrejection` 포워딩 포함 (`docs/debugging.md:6-18`) | 토스트 문구를 놓쳐도 어떤 에러였는지(파일 NotFound vs pane NotFound vs Forbidden) 사후 판별 |
| `TAIDE_PERF=1` | 프로세스 전역 게이트(`src-tauri/src/infra/perf.rs:9,371-380`), 프론트도 같은 게이트 채택(`entities/app/perf.ipc.ts`) | `SpanSlot::SearchListFiles`(`perf.rs:52`) 로 walk 소요를 보고 "첫 프레임 스테일 창"의 실제 길이를 잼. `PERF_MEASURE.PALETTE_OPEN`/`PALETTE_FILTER`(`perf-mark.ts:37-38`)로 열기·필터 지연 확인 |
| 로그 레벨 | `tauri_plugin_log::Builder::new()`(`src-tauri/src/lib.rs:497-500`) — 현재 tungstenite 만 레벨 제한, 기본 레벨 사용 | 임시로 `layout_open_tab` 선검증 실패·`search_list_files` 반환 건수에 `log::warn!` 1줄씩 추가하면 다음 발생 시 즉시 확정 |

**계측을 위해 추가하면 결정적인 로그 3곳(현재 없음)**
1. `layout/commands.rs:41-57 ensure_file_tab_target_exists` 실패 시 `path` + 실패 사유(경계 vs 존재).
2. `layout/service.rs:429` `pane not found` 발생 시 `focused_pane` 값 — (h-2) 즉시 판별.
3. `search/commands.rs:233-236` 반환 건수 + 소요 — 인덱스가 몇 건짜리인지, 200 상한에 걸릴 규모인지.

**사용자에게 물어볼 최소 재현 정보 3가지**
1. **실패한 파일의 프로젝트 기준 상대 경로 전체** — `dist/`·`build/`·`node_modules/` 등 무시 목록 안인지, 한글/비ASCII 이름인지, 심링크인지를 한 번에 가른다.
2. **그때 입력한 질의 문자열(정확히)과, 목록에 안 보였는지 vs 눌렀는데 실패했는지** — (f)/(b)(목록 부재) 와 (a)/(h-2)(열기 실패) 를 가른다.
3. **토스트가 떴다면 그 문구 원문** — "파일을 찾을 수 없습니다: …"(스테일/부재) vs "pane not found: …"(h-2) vs "열린 프로젝트 루트 밖…"(경계) vs 무토스트(h-1/h-4)로 즉시 분기. 더해 **분리(보조) 창에서 눌렀는지** 여부.

---

## 설계 옵션

| # | 옵션 | 대상 가설 | 장점 | 단점 | 영향 파일 | 규모 |
|---|---|---|---|---|---|---|
| 1 | 랭킹 타이브레이크 추가(점수 → 파일명 우선 매칭 → 경로 길이/알파벳)로 상한 200 의 커트라인을 결정적으로 만듦 | (f) | 문서(`command-palette.md:84-86`)가 이미 약속한 동작으로 코드를 맞춤. 간헐성 제거 | 정렬 비용 소폭 증가(현재 5만 후보 6.6ms 수준) | `src/shared/lib/fuzzy-match.ts`, `command-palette.tsx` | **S** |
| 2 | 질의 문자열과 매칭 대상 라벨을 `normalize('NFC')` 로 통일(인덱스는 원본 경로 유지, 매칭용 라벨만 정규화) | (b) | 한글/일본어 파일명 0건 문제 근본 해소. 열기 경로는 원본 경로를 쓰므로 무영향 | 라벨 정규화로 하이라이트 인덱스가 원문과 어긋날 수 있어 `splitFileMatchForDisplay` 동반 점검 필요 | `command-palette.tsx`, `shared/lib/fuzzy-match.ts`, `command-palette-file-match.ts` | **M** |
| 3 | `target:null` fallback 을 서버에서 `ensure_focused_pane_valid` 후 해석하거나, pane 부재를 별도 코드/로케일 키로 분리 | (h-2) | 백로그 등재분 해소. 파일 부재 오진과 불필요한 walk 1회 제거 | Rust 에러 코드 추가 → bindings 재생성 | `layout/service.rs`, `layout/commands.rs`, `root_guard`/`error.rs`, 로케일 3언어 | **S~M** |
| 4 | 보조 창에도 퀵오픈 제공(팔레트를 창 스코프로 재설계) 또는 최소한 aux 창에서 ⌘P 시 안내 | (h-1) | "아무 반응 없음" 제거 | `activeProjectQueryOptions` 의존을 창 고정 project 로 바꾸는 작업 — Wave I F1 미해결 이슈 | `app/app.tsx`, `widgets/command-palette/*` | **L** |
| 5 | `IGNORED_DIR_NAMES` 를 퀵오픈에서 완화(설정화하거나 `dist`/`build`/`target` 만 gitignore 판정에 위임) | (h-3) | 실제로 존재하는 파일을 못 여는 문제 해소 | 대형 프로젝트 walk 비용·결과 상한 압박 증가 → 옵션 1과 함께여야 안전 | `src-tauri/src/constants.rs`, `search/service.rs` | **M** |
| 6 | 심링크 파일 포함(`file_type().is_file()` → 심링크는 `metadata()` 로 판정) | (c-누락) | 심링크 워크스페이스에서 퀵오픈 정상화 | 순환 심링크·중복 경로 방어 필요 | `search/service.rs:592-598` | **S~M** |
| 7 | `openFile` 의 조용한 return 을 `toast.info(t('app.openProjectFirst'))` 로(같은 파일의 다른 핸들러와 일관) | (h-4) | 무반응 제거 | 없음 | `command-palette.tsx:324-328` | **S** |

---

## 리스크·미확인

- 본 조사는 **읽기 전용·앱 미실행**이다. 위 가설 중 실제로 사용자 환경에서 발생한 것이 무엇인지는 Q3 의 재현 정보 3가지 없이는 확정 불가.
- (b) NFD 가설은 **해당 프로젝트의 실제 파일명 바이트가 NFD 인지**를 확인해야 확정된다(`ls | hexdump` 급 확인 필요). 코드 측 근거(정규화 부재 + 코드포인트 정확 비교)는 확정.
- (f) 의 "재-walk 마다 순서가 달라진다"는 `ignore`/`walkdir` 가 readdir 순서를 그대로 쓴다는 점(정렬 미지정)에 근거하며, APFS 에서 실제로 순서가 흔들리는지는 실측 미확인.
- 배치 3 계약이 "NFC 기각"이라 적은 것은 **열기 경로 한정**이라는 해석이 본 조사의 판단이며, 당시 조사 범위 문서 원문은 그 구분을 명시하지 않았다.

---

## 결정이 필요한 지점

1. 즉시 착수 범위 — **옵션 1(랭킹 타이브레이크) + 7(조용한 return)** 만 선반영해 간헐성을 먼저 줄일지, 아니면 재현 정보를 받은 뒤 원인 1개를 확정해 그것만 고칠지.
2. `IGNORED_DIR_NAMES` 정책(옵션 5) — 퀵오픈에서 `dist`/`build`/`target` 을 계속 완전 차단할지, gitignore 판정으로 위임할지, 설정으로 열지. 제품 판단 사항.
3. 보조 창 퀵오픈(옵션 4) — Wave I F1 미해결 이슈를 이번에 여는지, 아니면 aux 창 ⌘P 를 명시적 안내로만 막는지.

---

## T7 — Welcome 에 Open Terminal 추가(항목 8) · 검색 패널 실시간 검색 + 결과 아이콘(항목 9)

## 현행(근거)

### 1. Welcome 배선

| 항목 | 현행 | 근거 |
|---|---|---|
| 렌더 면 | 2곳 — 프로젝트 0개 전체화면(`projectId={null}`), 프로젝트 내 `welcome` 탭 / 빈 pane | `src/widgets/app-shell/app-shell.tsx:204,208` · `src/widgets/editor-area/pane-node-view.tsx:166,229` |
| 빈 pane 자동 Welcome | `welcomeOnEmptyEditor` 설정(기본 true) + main 창일 때만 | `src/widgets/editor-area/pane-node-view.tsx:127` · `src-tauri/src/domain/settings/types.rs:196,507` |
| 순수 UI | `WelcomeScreen` — props 만 받음(비즈니스 로직 0) | `src/features/welcome/welcome-screen.tsx:10~35` |
| 액션 | `onOpenFolder`(=`useOpenFolderDialog`), `onOpenFile`(로컬 dialog+`useOpenFileTab`), `onSelectRecent`(`useActivateProject`/`useOpenProject`) | `src/widgets/welcome/welcome-container.tsx:52~83` |
| 비활성 조건 | `canOpenFile={projectId !== null}` → 버튼 disabled + `app.openFileHint` 안내문 | `welcome-container.tsx:91` · `welcome-screen.tsx:52,57` |
| 단축키 표 | id 목록만 선언, 실효 keymap(`APP_KEYMAP`+overrides)에서 라벨/표기 유도 | `welcome-container.tsx:31,57~60` |

### 2. 새 터미널 커맨드

| 항목 | 현행 | 근거 |
|---|---|---|
| keymap | `new-terminal` = `Ctrl+Shift+\``, `descriptionKey: 'keymap.newTerminal'` | `src/shared/lib/keymap/keymap.ts:20,81` |
| 커맨드 | `terminal.new` → `run: (context) => context.openTerminalTab()` | `src/shared/lib/command-catalog.ts:34~40` |
| 실제 구현 | `command-palette.tsx` 내부 **로컬 클로저** — `activeProjectId` 없으면 `toast.info(app.openProjectFirst)` 후 return, 있으면 `openTab({kind:{kind:'terminal',sessionId:''}})` | `src/widgets/command-palette/command-palette.tsx:166~172` |
| 주입 경로 | `CommandContext.openTerminalTab` + `useGlobalKeymap({'new-terminal': openTerminalTab})` | `command-palette.tsx:225,234` · `src/shared/lib/command-registry.ts:9` |
| 프로젝트 소속 | 터미널은 프로젝트 소속 — Rust 가 `ensure_project_open` 으로 거부, `kill_project` 로 프로젝트 단위 정리 | `src-tauri/src/domain/terminal/commands.rs:357~361,426,165` |
| 기본 레이아웃 | 프로젝트 열면 이미 터미널 탭 1개가 seed 됨 | `docs/features/tabs.md:17` · `docs/features/layout-shell.md:124` |

→ **`openTerminalTab` 는 export 되지 않은 위젯 로컬 함수**다. Welcome 에서 재사용하려면 추출이 필요하다(선례: `useOpenAppFileTab`, `src/entities/layout/layout.query.ts:82~91`).

### 3. 검색 실행 흐름

| 항목 | 현행 | 근거 |
|---|---|---|
| 트리거 | Enter 키 전용 (IME 가드 후 `event.key !== 'Enter'` 이면 return) | `src/features/search/search-panel.tsx:118~121` |
| 제출 | `!query.trim()` 이면 return → `perfMark` → `run(buildQuery())` | `src/widgets/search-panel/search-panel-container.tsx:81~85` |
| 이전 실행 취소 | **FE 명시 cancel 없음.** `generationRef` 증가로 늦은 콜백 폐기 + Rust `begin_search` 가 같은 `(owner, sessionId)` 이전 토큰을 flag | `src/entities/search/use-search-run.ts:125,140,146` · `src-tauri/.../search/commands.rs:52~62` |
| 명시 cancel 금지 이유 | 과거 `searchCancel` 선행 호출이 supersede 와 레이스 → 새 토큰을 죽여 결과 0건 (audit §4-B D4) | `use-search-run.ts:45~51` |
| 중단 반응성 | 워커가 엔트리마다 `cancelled` 확인 후 `WalkState::Quit` | `src-tauri/.../search/service.rs:517~519` |
| 스트리밍 | 파일 단위 `SearchFileMatches` 채널 → 50ms 버퍼 flush, 증분 그룹 append | `src/entities/search/search.ipc.ts:19~28` · `use-search-run.ts:24,142` |
| 이력 push 시점 | **`run()` 진입 즉시**, `recordHistory` 기본 true | `use-search-run.ts:133` · `DEFAULT_RUN_SEARCH_OPTIONS` `use-search-run.ts:16` |
| 이력 저장 비용 | settings 문서 전체 read-modify-write 뮤테이션 1회 | `src/entities/search/search-history.ts:60~70` |
| 최소 질의 길이 | 없음 (`trim()` 비어있음만 차단). Rust 도 빈 문자열이면 `Ok(0)` | `search-panel-container.tsx:82` · `service.rs:500~502` |
| 정규식 오류 | `compile_regex` → `InvalidArgument`/`error.search.invalidRegex` → FE `toast.error` + `status='failed'` | `service.rs:126~136` · `use-search-run.ts:152~156` |
| 결과 상한 | 10,000 매치 (Rust 상수 미러) | `src/shared/constants/search.ts:11` |
| 동일 로직 2번째 면 | Search Editor 탭도 Enter 전용 동일 구조 | `src/widgets/search-editor/search-editor-pane.tsx:75~92` |
| 성능 기준선 | metric 7 "전역 검색 200건+" 측정값 **미기입**(`______ ms`) | `docs/quality-assurance/2026-09-04-perf-baseline.md:66` |

### 4. 아이콘

| 표면 | 아이콘 | 근거 |
|---|---|---|
| 검색 결과 파일 헤더 | lucide `File` **하드코딩** + `size-3.5 opacity-80`, 색 클래스 없음 | `src/shared/ui/file-group-header.tsx:2,29` |
| 문제 패널 파일 헤더 | 동일 `FileGroupHeader` 사용 → 같은 증상 | `src/features/problems/problems-panel.tsx:12,100` |
| 탐색기 행 | `FileTypeIcon` → `resolveFileIcon` → 레지스트리 + `spec.colorClass` | `src/features/explorer/file-tree-row.tsx:4,73` · `src/shared/icons/file-type-icon.tsx:11~15` |
| 탭 바 | 동일 `FileTypeIcon` | `src/widgets/editor-area/pane-tab-bar.tsx:60,67` |
| git 변경 목록 | **파일 아이콘 자체가 없음**(이름+디렉터리+상태 문자만) | `src/features/git/status-row-item.tsx:47~68` |

---

## 핵심 질문별 답

### Q1. Welcome 에 Open Terminal — 재사용할 커맨드와 비활성 조건

- **재사용 대상은 `terminal.new` 커맨드의 실행부**(`command-catalog.ts:34~40` → `context.openTerminalTab()`)이지만, 그 구현이 `command-palette.tsx:166~172` 안에 갇혀 있어 **직접 호출 불가**. 추출이 전제다.
- **비활성 조건 = `projectId === null`** — 기존 `canOpenFile`(`welcome-container.tsx:91`)과 **완전히 동일한 조건**이다. 근거: 터미널 탭은 레이아웃 소속이고 Rust 가 `ensure_project_open` 으로 거부한다(`terminal/commands.rs:357~361`). 프로젝트 0개 화면은 항상 `projectId={null}`(`app-shell.tsx:208`).
- **프로젝트 없는 상태에서 Welcome 이 뜨는 경우는 실재**한다 — ① 프로젝트 0개 전체화면(`app-shell.tsx:204`), ② 프로젝트 내 빈 pane 자동 Welcome(`pane-node-view.tsx:127,227`, 이 경우는 `projectId` 있음). 즉 ①만 비활성 대상.
- 팔레트 쪽은 `toast.info('app.openProjectFirst')` 로 처리하지만, Welcome 은 이미 disabled+hint 패턴을 쓰므로 **disabled 가 일관**이다.
- **로케일**: `keymap.newTerminal`("새 터미널")이 ko/en/ja 에 이미 존재 → 그대로 재사용 가능. 새 키를 만든다면 3개 로케일 + `MESSAGE_NAMESPACES`(`src-tauri/src/domain/locale/service.rs:13`) 동시 등재 필수(누락 시 Rust 테스트 실패 — 배치4 계약 §0).

### Q2. 실시간 검색으로 전환할 때 필요한 것

| 필요 항목 | 현행 상태 | 해야 할 일 |
|---|---|---|
| 디바운스 상수 | 없음 | `src/shared/constants/search.ts` 에 추가(파일 이미 존재). VS Code 기본값 300ms 와 맞춤 |
| 디바운서 | `createLeadingTrailingDebouncer`(`src/shared/lib/leading-trailing-debouncer.ts:24`) 존재하나 **leading edge 즉시 실행** → 첫 글자로 전역 검색이 돌아 부적합 | trailing-only 경로 필요(옵션 추가 또는 `shared` 에 `useDebouncedValue`) |
| 이전 실행 취소 | **추가 작업 불필요** — `run()` 이 generation 증가 + Rust `begin_search` 가 이전 토큰 flag, 워커는 엔트리마다 Quit | 명시 `searchCancel` 을 넣으면 안 됨(`use-search-run.ts:45~51` 의 회귀) |
| 이력은 Enter/blur 만 | `run()` 이 무조건 push(`use-search-run.ts:133`) → 디바운스 틱마다 settings 전체 write | `recordHistory` 기본값을 false 로 뒤집고 Enter/blur 경로만 true (선례: `search-editor-pane.tsx:108` 이 이미 false 를 명시 전달) |
| IME 조합 제외 | **불가능에 가까움.** `isImeCompositionKeydown` 은 keydown 전용 가드이고, Tauri 2 macOS WKWebView 는 `compositionstart/update/end` 를 **아예 안 쏜다**(`src/shared/lib/ime-composition.ts:5~10`, `docs/bug/2026-08-06-wkwebview-ime-composition.md`) | onChange 에는 조합 신호가 없다. 대안: 입력의 `onKeyDown` 에서 `keyCode===229` 를 ref 에 기록하고 그 직후 change 는 스케줄 생략 |
| 부하 가드 | 매치 상한 10,000 은 이미 있음. metric 7 실측치는 **미기입** | 최소 질의 길이 도입 여부가 유일한 실질 레버(VS Code 는 최소 길이 없이 디바운스만) |
| 정규식 오류 | 틱마다 `toast.error` 폭발 위험(`use-search-run.ts:154`) | 라이브 실행은 토스트 억제하고 `status='failed'` + 인라인 문구만 쓰거나, FE 선검증 |
| perf 계측 | `SEARCH_RUN_REQUESTED` 마크를 제출마다 찍고 첫 결과에서 measure(`search-panel-container.tsx:83,151~154`) | 틱마다 마크가 덮여 metric 6 표본 의미가 바뀜 — 라이브 실행은 마크 제외 권장 |
| 안내 문구 | `search.pressEnterHint` = "검색어를 입력하고 Enter 를 누르세요" | 문구 교체 필요(3개 로케일) |

**VS Code 근거**: 전문 검색 결과가 입력 중 갱신되며, `search.searchOnType` 로 끌 수 있고 `search.searchOnTypeDebouncePeriod` 는 **기본 300ms**. 이력 오염은 VS Code 에서도 실제 이슈로 보고됐다(microsoft/vscode#86288) — 위 "이력은 Enter/blur 만" 항목의 직접 선례다.

### Q3. 결과 목록 아이콘이 "제대로 안 나오는" 원인

**원인은 확장자 매핑 누락도, 아이콘 테마 미적용도, 크기 문제도 아니다. `FileGroupHeader` 가 레지스트리를 아예 호출하지 않고 lucide `File` 을 하드코딩한다.**

```tsx
// src/shared/ui/file-group-header.tsx:29
<File className='size-3.5 shrink-0 opacity-80' />
```

- 탐색기/탭 바는 `FileTypeIcon`(`file-type-icon.tsx:11~15`)을 거쳐 `resolveFileIcon`(`file-icon.ts:103~118`) → `FILE_ICON_COMPONENT_MAP`(`file-icon-registry.ts:34`) + `spec.colorClass` 를 적용한다. 검색/문제 헤더만 이 경로 밖에 있다.
- 크기는 양쪽 다 `size-3.5`(`file-tree-row.tsx:22`, `pane-tab-bar.tsx:48`, `file-group-header.tsx:29`)로 동일 → **크기는 원인 아님**. 다만 `opacity-80` + 색 클래스 부재로 색까지 죽어 "탐색기와 다르게 보인다"는 인상이 강화된다.
- **같은 표면 공유 확인**: 문제 패널(`problems-panel.tsx:100`)도 동일 증상. **git 패널은 `FileGroupHeader` 를 쓰지 않으며 파일 아이콘이 애초에 없다**(`status-row-item.tsx:47~68`) — 이번 수정으로 고쳐지지 않는 별건.
- **수정 위치**: `src/shared/ui/file-group-header.tsx:29` 한 줄. `fileNameOf` 는 같은 파일 5행에서 이미 import 중이므로 `<FileTypeIcon fileName={fileNameOf(path)} className='size-3.5 shrink-0' />` 로 교체하면 된다. FSD 상 `shared/ui → shared/icons` 는 동일 레이어 참조라 허용(fsd.md §2). `ROW_ICON_SIZE_CLASS` 는 `features` 에 있으므로 import 하지 말고 리터럴 유지.
- 부수 확인: 그룹 헤더 가상화 높이 추정치는 20px 고정(`src/features/search/search-result-rows.ts:10`). 아이콘 크기를 그대로 두면 영향 없다.

---

## 설계 옵션

### A. Welcome "터미널 열기" 버튼

| 옵션 | 내용 | 장점 | 단점 | 영향 파일 | 규모 |
|---|---|---|---|---|---|
| **A1 (권장)** | `useOpenTerminalTab(projectId)` 를 `entities/layout/layout.query.ts` 로 추출, 팔레트·Welcome 2곳이 소비 | `useOpenAppFileTab` 과 동일 선례, "2회 이상이면 공통화" 충족, 토스트/에러 처리 1곳 | 팔레트 리팩터가 diff 에 포함 | `entities/layout/layout.query.ts`, `widgets/command-palette/command-palette.tsx`, `widgets/welcome/welcome-container.tsx`, `features/welcome/welcome-screen.tsx` | S |
| A2 | `welcome-container` 에 `openTab` 호출을 복제 | diff 최소 | 동일 로직 2벌(컨벤션 위반), 에러 문구 드리프트 | 위 2개 파일 | S |
| A3 | `terminal-bridge` 신설(`search-panel-bridge` 류) | 위젯 간 결합 0 | terminal.new 는 pane 커맨드가 아님, 과설계 | +`shared/lib/bridge/*` | M |

버튼 배선(A1 기준): `WelcomeScreen` 에 `canOpenTerminal: boolean` + `onOpenTerminal: () => void` 추가, `variant='outline'` + lucide `Terminal`, 라벨 `t('keymap.newTerminal')`, `disabled={!canOpenTerminal}`. 컨테이너는 `canOpenTerminal={projectId !== null}`.

### B. 실시간 검색

| 옵션 | 내용 | 장점 | 단점 | 영향 파일 | 규모 |
|---|---|---|---|---|---|
| **B1 (권장)** | 패널 한정 + 설정 토글(`searchOnType`, 기본 on) + 300ms trailing 디바운스 + 이력은 Enter/blur, 라이브 실행은 `recordHistory:false`·토스트 억제·perfMark 제외 | VS Code 동형, 되돌릴 수 있음, 회귀 표면 최소 | Rust settings 필드 추가(3로케일+네임스페이스+bindings 재생성) | `shared/constants/search.ts`, `entities/search/use-search-run.ts`, `widgets/search-panel/search-panel-container.tsx`, `features/search/search-panel.tsx`, `src-tauri/.../settings/types.rs`, 로케일 3종 | M |
| B2 | 설정 없이 무조건 라이브 | 가장 작음 | 대형 프로젝트에서 되돌릴 수단 없음, metric 7 실측 부재 상태에서 위험 | 위에서 Rust 제외 | S |
| B3 | 패널 + Search Editor 동시 적용 | 일관성 | Search Editor 는 컨텍스트 라인까지 실어 배치가 더 무겁고 스냅샷 복원 로직과 얽힘 | +`widgets/search-editor/search-editor-pane.tsx` | M~L |

### C. 아이콘

| 옵션 | 내용 | 규모 |
|---|---|---|
| **C1 (권장)** | `file-group-header.tsx:29` 를 `FileTypeIcon` 으로 교체(검색·문제 패널 동시 해결) | S |
| C2 | C1 + git `StatusRowItem` 에도 `FileTypeIcon` 추가 | S~M |

---

## 리스크 · 미확인

- **[미확인] 대형 프로젝트 검색 실측치 없음** — `docs/quality-assurance/2026-09-04-perf-baseline.md:66` metric 7 이 미기입(`______ ms`). 실시간 검색의 부하 판단 근거가 현재 저장소에 없다. 도입 전 이 행을 채우는 게 맞다.
- **IME 리스크(높음)** — macOS WKWebView 에서 조합 이벤트가 오지 않으므로(`src/shared/lib/ime-composition.ts:5~10`) onChange 기반 라이브 검색은 한글 조합 중간 자모로도 검색을 돌릴 수 있다. keydown 229 를 ref 로 물고 가는 우회는 설계 가능하나 이 저장소에 **선례가 없다**.
- **이력 오염(확실)** — 이력 push 가 `run()` 안에 있고(`use-search-run.ts:133`) settings 전체 write 를 유발(`search-history.ts:69`). 손대지 않으면 타이핑마다 IPC settings 쓰기가 발생한다.
- **정규식 토스트 폭주(확실)** — `use-search-run.ts:154` 가 실패마다 토스트. 라이브 전환 시 필수 대응.
- **[미확인] `leading-trailing-debouncer` 의 trailing-only 지원 여부** — 파일의 doc 상 leading+trailing 고정으로 읽히며, 옵션 파라미터는 확인하지 않았다(구현 직전 재확인 필요).
- **[미확인] Welcome 에 버튼이 3개가 되는 레이아웃/디자인 확정** — `welcome-screen.tsx:47` 의 `flex-wrap` 이라 줄바꿈은 되지만 시각 확인은 못 했다(앱 실행 금지 제약).
- **[미확인] `app.openFileHint` 재사용 적절성** — 현재 문구가 "파일을 열려면 먼저 폴더를 여세요"라 터미널까지 포괄하려면 문안 조정이 필요할 수 있다.

---

## 결정이 필요한 지점

1. **터미널 열기 구현 재사용 방식** — A1(`useOpenTerminalTab` 추출, 추천) / A2(복제) / A3(브리지)
2. **터미널 버튼 라벨 키** — 기존 `keymap.newTerminal` 재사용(추천, 3로케일 이미 존재) / 신규 `app.openTerminal` 추가(로케일 3종 + `MESSAGE_NAMESPACES` 등재 필요)
3. **프로젝트 0개일 때 터미널 버튼 처리** — disabled + 기존 `app.openFileHint` 재사용(추천) / disabled + 신규 공통 안내 문구 / 버튼 자체 숨김
4. **실시간 검색 on/off 설정 신설 여부** — 신설(`searchOnType`, Rust settings 확장 필요, 추천) / 무조건 라이브(설정 없음)
5. **디바운스 값** — 300ms(VS Code 동일, 추천) / 다른 값
6. **적용 범위** — 검색 패널만(추천) / Search Editor 탭까지
7. **IME 조합 처리** — 디바운스만으로 감수 / keydown `keyCode===229` ref 가드 추가(선례 없음) / 최소 질의 길이 도입으로 완화
8. **정규식 모드에서의 라이브 동작** — 라이브 실행 시 토스트 억제 + 인라인 실패 표시(추천) / regex 켜져 있으면 라이브 비활성 / FE 선검증
9. **이력 push 리팩터 방식** — `recordHistory` 기본값을 false 로 뒤집고 호출부가 명시(최소 diff) / `addRecentSearch` 를 `useSearchRun` 밖으로 이동(책임 분리)
10. **아이콘 수정 범위** — `FileGroupHeader` 만(검색+문제 동시 해결, 추천) / git `StatusRowItem` 파일 아이콘까지 함께
11. **실시간 검색 착수 전 metric 7 실측 선행 여부**


---

> 아래 T8·T9 는 두 번째 조사 워크플로(`wf_5e4b8e2c`, opus·high)의 산출을 메인이 원문 그대로 이어 붙인 것이다(항목 11·13).

## T8 — git 패널 섹션 리사이즈·접힘·그래프 높이 (항목 11)

### 현행(근거)

### 1. git 패널 레이아웃 모델 — 단일 스크롤 뷰포트 + 콘텐츠 기반 높이

| 구성 | 위치 | 높이 결정 방식 |
|---|---|---|
| 루트 | `src/widgets/git-panel/git-panel.tsx:325` `flex h-full min-h-0 w-full flex-col` | 부모(사이드바 Panel) 높이 |
| 브랜치 헤더 바 | `git-panel.tsx:326` `h-8 shrink-0` | 고정 32px |
| CommitBox | `git-panel.tsx:369` | 콘텐츠 |
| 스크롤 뷰포트 | `git-panel.tsx:380` `<ScrollContainer className='min-h-0 flex-1'>` | 남은 공간 전부 (스크롤바 1개) |
| 변경 3그룹(merge/staged/changes) | `git-panel.tsx:384` `style={{ height: rowVirtualizer.getTotalSize() }}` | **가상화 행 수 × 24px** |
| 스태시 섹션 | `git-panel.tsx:453-465` | 흐름 배치, 콘텐츠 |
| 그래프 섹션 | `git-panel.tsx:467-493` | 흐름 배치, **내부에 고정 320px 상한** |

- `ScrollContainer` 는 뷰포트 `overflow-y-auto h-full w-full` + OverlayScrollbar (`src/shared/scroll/scroll-container.tsx:33-41,44`).
- 변경 목록은 `@tanstack/react-virtual`, 행·헤더 모두 24px 고정(`git-panel.tsx:51`), sticky 헤더는 `rangeExtractor` 로 렌더 범위에 강제 포함 후 흐름에 그림(`git-panel.tsx:222-233,388-397`, `change-row-navigation.ts:56-57`).
- **어느 섹션에도 flex·min-height·사용자 높이가 없다.** 전부 콘텐츠 기반이고, 유일한 고정 픽셀 뷰포트가 그래프다(`grep maxHeight:` 결과 레포 전체에서 commit-graph 1건).

### 2. 접힘 상태

- 상태: `collapsedSections` = 모듈 스코프 메모리 스냅샷(`git-panel.tsx:169` ← `src/entities/git/git-section-collapse-memory.ts:37-51`). 기본값은 `stashes` 만 접힘(`:13-19`). **재시작 시 초기화** — Settings/ProjectLayout 승격을 의도적으로 거절한 근거가 `:21-35` 주석에 명시.
- 변경 그룹 접힘 → 헤더만 남기고 행을 평탄화 목록에서 제외(`src/widgets/git-panel/change-row-navigation.ts:33-36`) → `getTotalSize()` 가 행 수만큼 줄어든다.
- 스태시·그래프 접힘 → 본문 자체를 렌더하지 않음(`git-panel.tsx:461-463, 475-491`).

### 핵심 질문별 답

### Q1. "접혀도 영역이 남는다" / "그래프가 과대" 의 코드상 원인

**(a) 그래프 과대 — 원인 특정됨 (하드코딩 320px).**

```
src/widgets/git-panel/commit-graph.tsx:45   const VIEWPORT_HEIGHT_PX = 320
src/widgets/git-panel/commit-graph.tsx:125  <div ref={parentRef} className='scrollbar-hidden overflow-y-auto' style={{ maxHeight: VIEWPORT_HEIGHT_PX }}>
```

- 패널 높이와 무관한 절대값이라 **짧은 사이드바에서는 남은 뷰포트를 통째로 삼키고, 긴 사이드바에서는 절대 커지지 않는다.**
- 100커밋 기본 로드(`src/entities/git/git.query.ts:49,115` `LOG_PAGE_SIZE = 100` → 2400px)를 320px 상자에 넣고 `scrollbar-hidden` 이라, **스크롤바 없는 중첩 스크롤 영역**이 생긴다(OverlayScrollbar 미적용 — 레포에서 `ScrollContainer` 를 안 쓰는 유일한 스크롤 뷰포트).
- 그래프가 흐름상 마지막이라 변경 파일이 많으면 화면 밖으로 밀린다.

**(b) "접힌 섹션이 영역을 차지" — 소스상 예약 박스는 없음. 관측 원인은 "회수된 공간이 재분배되지 않는 것".**

- 세 경로(`change-row-navigation.ts:34`, `git-panel.tsx:461`, `git-panel.tsx:475`) 모두 접힘 시 본문을 렌더하지 않고 헤더 24px만 남긴다 — 예약 높이·min-height·스켈레톤 없음.
- 그런데 레이아웃이 **위 정렬 단일 스크롤 흐름**이라, "Changes" 를 접어도 그래프는 320px 상한에 묶여 커지지 않고 **패널 하단이 빈 채로 남는다.** 사용자에겐 "접었는데 영역이 그대로"로 읽힌다.
- 잔여 [미확인]: 실제 화면에서 24px 초과의 잔존 박스가 보인다면 소스에서 설명되지 않는다 → 라이브 재현 1회 필요.

**(c) 배치 4 계약 E절 / scrollPaddingStart 확인**

- `docs/acknowledge/2026-09-04-usability-batch4-contract.md:213-238` E절은 **섹션 순서·공용 접이식 헤더·접힘 메모리·`buildGitSections`** 만 계약했고, **리사이즈·높이 배분은 계약 범위 밖**이다. 즉 T8은 E절 위반이 아니라 미계약 영역.
- 리뷰 결정 `scrollPaddingStart: GIT_CHANGE_ROW_HEIGHT_PX`(`git-panel.tsx:226`, 계약 `:900-903`)는 **sticky 헤더가 키보드 이동 대상 행을 가리는 문제**의 해법으로, 헤더가 24px 고정이라는 전제에 묶여 있다 → 섹션 높이 모델을 바꿔도 이 값은 유지돼야 한다.
- `docs/features/git.md:108-110` 이 **"가상 목록은 스크롤 뷰포트의 첫 요소"·"스크롤바 하나"** 를 명시 계약으로 박아 뒀다 — A/C안은 이 문장을 바꿔야 한다.

### Q2. 재사용 가능한 리사이즈 인프라

`react-resizable-panels@4.12.2`(`package.json:61`) 사용처:

| 사용처 | 코드 | 크기 영속화 |
|---|---|---|
| 셸 가로(사이드바↔에디터) | `src/widgets/app-shell/app-shell.tsx:221-250` (`Panel id='explorer' defaultSize='240px' minSize='180px' maxSize='40%' collapsible collapsedSize={0}`) | **접힘만** Rust `shell_view.sidebarCollapsed`(`:110-119`). **폭은 의도적으로 뷰 로컬 기본값**(주석 `:110`, 계약 §3.2) |
| 에디터↔Problems 세로 | `src/widgets/editor-area/editor-area.tsx:440-459` (`defaultSize='220px' minSize='120px'`) | **영속화 없음** (defaultSize 뿐) |
| 에디터 pane 분할 | `src/widgets/editor-area/pane-node-view.tsx:75-116` | **Rust ProjectLayout** — `useResizePane` → `layout_resize_pane`, `schedulePaneResizeCommit` 120ms 트레일링 디바운스(`src/entities/layout/pane-resize-commit.ts:16-53`) |
| 세퍼레이터 | `src/features/split/pane-separator.tsx:10-19` (`Separator` + `settings.resizerThickness`) | — |
| 히트 타깃 | `src/shared/constants/layout.ts:7` `RESIZE_HIT_TARGET_SIZE = { fine: 8, coarse: 20 }`, `:18` `MIN_PANEL_SIZE_PX = 120` | — |

→ **"Rust 영속화" 와 "영속화 없이 기본값" 두 선례가 모두 존재**한다. 접힘 상태가 이미 "재시작 시 초기화" 로 결정돼 있으므로(`git-section-collapse-memory.ts:21-35`), 높이만 Rust로 올리면 두 상태의 수명이 어긋난다.

**섹션별 Panel 전환 시 충돌 지점**

| 충돌 | 근거 | 영향 |
|---|---|---|
| 단일 스크롤 뷰포트 계약 | `git-panel.tsx:106-110`, `docs/features/git.md:108-110` | 섹션마다 스크롤 요소가 생겨 "스크롤바 하나" 계약 폐기. 단 가상화 오프셋 0 전제는 오히려 자명해짐 |
| sticky 헤더 기계장치 | `git-panel.tsx:222-233,387-397`, `change-row-navigation.ts:42-57` | 3그룹을 각각 Panel로 쪼개면 `buildGitChangeListRows`·`resolveStickyHeaderIndex`·`gitChangeListHeaderIndexes` + 테스트가 존재 이유를 잃음 |
| 로빙 포커스 | `git-panel.tsx:239-241,298` (`rovingItemCount`, 단일 `rowVirtualizer.scrollToIndex`), `docs/features/git.md:71-85` | 가상화기가 N개가 되면 인덱스 공간·scrollToIndex 디스패치 재설계. `change-row-navigation.test.ts`·`e2e/specs/19-git-sections-collapse.e2e.ts` 동반 수정 |
| 접힘=헤더만 | rrp `collapsible`+`collapsedSize` 는 Panel 전체를 접는다 | 헤더가 Panel 안에 있으면 같이 사라짐 → 헤더를 Panel 내부 `shrink-0` 로 두고 `minSize='24px'` + 명령형 리사이즈로 구현해야 함 |

[미확인] `node_modules` 가 이 체크아웃에 없어 rrp 4.12.2 실소스/타입을 읽지 못했다. 확인된 API는 레포 사용 실적(Group/Panel/Separator, `defaultSize`·`minSize`·`maxSize` px·% 문자열, `collapsible`·`collapsedSize`, `panelRef.collapse/expand/isCollapsed`, `onLayoutChanged(layout, meta.isUserInteraction)`)뿐. **명령형 "특정 px로 리사이즈 후 이전 크기로 복원"·Panel 단위 onCollapse 콜백·Group 자식에 Panel/Separator 외 노드 허용 여부는 미확인** — 구현 착수 전 공식 문서 확인 필요.

### Q3. 설계 옵션

| 안 | 내용 | 장점 | 단점 | 영향 파일 | 규모 |
|---|---|---|---|---|---|
| **A. 섹션별 독립 Panel** | 세로 `Group` 안에 merge/staged/changes/stashes/graph 각각 `Panel`(헤더 `shrink-0` + 본문 `flex-1` 자체 스크롤), 사이에 `PaneSeparator`, 접힘 = 24px 고정 | VS Code SCM 완전 파리티, 모든 섹션 드래그 리사이즈, 접힘이 실제로 공간을 양보 | 단일 스크롤바 계약 폐기, 가상화기 N개, 로빙/ sticky 기계장치 전면 재설계, 계약·테스트 대폭 수정 | `git-panel.tsx`, `change-row-navigation.ts`(+test), `git-section-header.tsx`, `commit-graph.tsx`, `docs/features/git.md` §2·§5, `e2e/specs/19-*` | **L** |
| **B. 단일 스크롤 유지 + 그래프 높이만 조절** | `VIEWPORT_HEIGHT_PX` 제거 → 그래프 높이를 props(사용자 드래그 핸들 + 상·하한)로. 나머지 구조 무변경 | 가상화 오프셋 0·sticky·로빙·스크롤바 1개 계약 전부 무변경, 가장 작은 diff | 변경/스태시 섹션은 여전히 리사이즈 불가(요구 일부만 충족), 드래그 핸들을 rrp 없이 자체 구현(중첩 스크롤 안의 핸들) | `commit-graph.tsx`, `git-panel.tsx`, 높이 메모리 1파일 | **S~M** |
| **C. 그래프만 하단 별도 Panel로 분리** | 패널 본문을 세로 `Group` 으로: `Panel[변경+스태시 스크롤]` / `PaneSeparator` / `Panel[그래프, collapsible, minSize≈24px]`. 그래프는 부모를 꽉 채움(320 상한 제거) | 실제 결함(그래프 고정 320·리사이즈 불가·중첩 숨은 스크롤)을 정면 해결, 변경 목록의 가상화·sticky·로빙 기계장치 거의 그대로, rrp 기존 인프라 재사용 | 스크롤바 2개(계약 문장 수정 필요), 그래프 로빙 항목이 다른 Panel로 이동 → 포커스 이동 시 스크롤 처리 점검 | `git-panel.tsx`(본문 Group 래핑), `commit-graph.tsx`, `git-section-header.tsx`(그래프 헤더 sticky→패널 내 고정), `docs/features/git.md` §2·§5, e2e 19 | **M** |

**추천: C (+ B의 "그래프가 부모를 채운다" 변경 포함).** 항목 11이 지목한 3증상 중 "그래프 과대"·"리사이즈 불가"를 근본 원인(`commit-graph.tsx:45`)에서 제거하면서, 배치 4에서 비싸게 세운 가상화·sticky·로빙 계약(`git-panel.tsx:104-125`, `docs/features/git.md:100-122`)을 깨지 않는다. A는 "모든 섹션을 드래그로 조절" 을 사용자가 명시 요구할 때만.

**접힘/높이 영속화 위치 권고**: 프론트 모듈 스코프(`entities/git/git-section-collapse-memory.ts` 옆에 높이 메모리 추가). 근거 — 접힘 상태가 이미 "Settings/ProjectLayout 승격 불가"로 결정돼 있어(`:21-35`) 높이만 Rust로 올리면 두 상태 수명이 어긋나고, 사이드바 폭도 같은 이유로 뷰 로컬 기본값이다(`app-shell.tsx:110`). 재시작 후에도 유지가 요구되면 Settings 필드(Rust 타입 + patch + bindings + locale 라벨) 한 세트가 필요하다.

### 리스크·미확인

- [미확인] rrp 4.12.2 실소스 미열람(`node_modules` 부재). 명령형 px 리사이즈/복원, Panel `onCollapse`, Group 자식 제약 — 구현 전 공식 문서 확인 필수.
- [미확인] "접힌 섹션이 영역을 차지" 의 잔존 박스는 소스에서 재현되지 않음. 24px 헤더 이상의 잔존이 실제로 보이는지 라이브 1회 확인 필요.
- 그래프 내부 `scrollbar-hidden`(`commit-graph.tsx:125`)은 레포 유일의 비-`ScrollContainer` 스크롤 뷰포트 — 어느 안을 택하든 `ScrollContainer` 로 통일할지 함께 결정해야 한다.
- 계약 문서 3곳이 현 모델을 명문화하고 있어(`docs/features/git.md:71-85,100-122,224-227`, 배치4 계약 §E·`:900-903`) 문서 갱신이 코드와 동시에 필요하다.
- e2e `19-git-sections-collapse.e2e.ts:40` 이 헤더의 `position: sticky` 를 직접 단언한다 → A/C에서 그래프·스태시 헤더가 패널 내부 고정으로 바뀌면 스펙 수정 대상.

### 결정이 필요한 지점

1. **범위** — **C(그래프만 분리·추천)** / B(그래프 높이만 조절, 최소 diff) / A(전 섹션 Panel, VS Code 완전 파리티)
2. **영속화** — **프론트 모듈 메모리(재시작 초기화, 접힘과 수명 일치·추천)** / Settings(Rust, 재시작 유지) / ProjectLayout(프로젝트별)
3. **"스크롤바 하나" 계약**(`docs/features/git.md:108-110`) 폐기 승인 여부 — A/C는 스크롤 영역이 2개 이상이 된다
4. **접힘 시 섹션 크기** — 헤더 24px 고정 + 리사이즈 대상에서 제외(VS Code 방식·추천) / 접혀도 드래그 가능

### keyFacts
- git 패널은 단일 스크롤 뷰포트(ScrollContainer, git-panel.tsx:380 'min-h-0 flex-1') 하나에 가상화된 변경 목록(384) + 흐름 배치 스태시(453) + 흐름 배치 그래프(467)를 쌓는 구조이고, 어느 섹션에도 flex/min-height/사용자 높이가 없다 — 전부 콘텐츠 기반
- 그래프 과대의 직접 원인은 commit-graph.tsx:45 `VIEWPORT_HEIGHT_PX = 320` 과 :125 `style={{ maxHeight: VIEWPORT_HEIGHT_PX }}` + `overflow-y-auto scrollbar-hidden` — 패널 높이와 무관한 절대값이라 짧은 패널에선 뷰포트를 삼키고 긴 패널에선 커지지 않으며, 100커밋(git.query.ts:49 LOG_PAGE_SIZE) 2400px가 스크롤바 없는 중첩 스크롤로 갇힌다
- 접힘은 소스상 헤더 24px만 남기고 본문을 렌더하지 않는다(change-row-navigation.ts:34, git-panel.tsx:461, 475) — 예약 박스 없음. '접혀도 영역이 남는' 관측은 회수된 공간이 위 정렬 단일 스크롤 흐름에서 재분배되지 않고 그래프가 320px에 묶여 있기 때문
- 접힘 상태는 모듈 스코프 메모리(entities/git/git-section-collapse-memory.ts:37-51)이고, :21-35 주석이 Settings/ProjectLayout 승격을 의도적으로 거절했다(재시작 시 기본값 복귀)
- react-resizable-panels 4.12.2 사용처 3곳 — app-shell.tsx:221-250(사이드바, 접힘만 Rust shell_view 영속·폭은 뷰 로컬), editor-area.tsx:440-459(Problems, defaultSize 220px·영속화 없음), pane-node-view.tsx:75-116(에디터 분할, layout_resize_pane + pane-resize-commit.ts 120ms 디바운스로 Rust ProjectLayout 영속)
- 섹션별 Panel 전환은 docs/features/git.md:108-110 의 '가상 목록은 스크롤 뷰포트 첫 요소·스크롤바 하나' 계약과 git-panel.tsx:222-233 rangeExtractor sticky 기계장치, :239-241/:298 단일 가상화기 기반 로빙과 정면 충돌한다
- 배치4 계약 §E(docs/acknowledge/2026-09-04-usability-batch4-contract.md:213-238)는 섹션 순서·공용 헤더·접힘 메모리만 계약했고 리사이즈/높이 배분은 미계약 영역이다. scrollPaddingStart 결정(:900-903, git-panel.tsx:226)은 헤더 24px 고정 전제에 묶여 있어 어느 안에서도 유지돼야 한다
- node_modules 미설치로 rrp 4.12.2 실소스를 읽지 못했다 — 명령형 px 리사이즈/복원, Panel onCollapse, Group 자식 제약은 [미확인]

### decisionInputs
- 범위: C(그래프만 하단 별도 Panel 분리, M·추천) / B(단일 스크롤 유지 + 그래프 높이만 사용자 조절, S~M) / A(전 섹션 Panel + 세퍼레이터, L)
- 그래프 높이·섹션 크기 영속화 위치: 프론트 모듈 스코프 메모리(접힘 상태와 수명 일치·추천) / Settings Rust 필드(재시작 유지, 타입+patch+bindings+locale 1세트) / ProjectLayout(프로젝트별)
- docs/features/git.md:108-110 의 '스크롤바 하나' 계약 폐기 승인 여부 — A·C는 스크롤 영역이 2개 이상이 된다
- 접힘 시 섹션 크기 정책: 헤더 24px 고정 + 리사이즈 대상 제외(VS Code 방식·추천) / 접혀도 드래그 가능
- commit-graph.tsx:125 의 raw overflow-y-auto + scrollbar-hidden 를 공용 ScrollContainer(OverlayScrollbar)로 통일할지 여부

### unverified
- react-resizable-panels 4.12.2 의 실 API 표면(명령형 특정 px 리사이즈 및 이전 크기 복원, Panel 단위 onCollapse 콜백, Group 자식에 Panel/Separator 외 노드 허용 여부) — 이 체크아웃에 node_modules 가 없어 실소스·타입 미열람. 확인된 것은 레포 내 사용 실적(Group/Panel/Separator, defaultSize·minSize·maxSize 문자열, collapsible·collapsedSize, panelRef.collapse/expand/isCollapsed, onLayoutChanged(layout, meta.isUserInteraction))뿐
- '접힌 섹션이 영역을 차지한다'는 증상에서 헤더 24px 를 초과하는 잔존 박스가 실제로 보이는지 — 소스(change-row-navigation.ts:34, git-panel.tsx:461·475)로는 설명되지 않아 라이브 재현 1회가 필요
- 짧은 사이드바 높이에서 그래프 320px 가 실제로 차지하는 비율(실측 미수행 — 앱 실행 금지 제약)
- C·A 안에서 로빙 포커스가 Panel 경계를 넘을 때의 스크롤 동작(scrollToIndex 디스패치)이 현행 e2e 19/change-row-navigation.test.ts 를 어느 범위까지 깨는지 — 정적 분석 기반 추정

## T9 — 스킨 테마 확충·전 UI 토큰 커버리지 (항목 13)

### 현행(근거)

### 토큰 스키마
| 축 | 수 | 정의 위치 |
|---|---|---|
| `colors` (시맨틱) | **133** (21 네임스페이스) | `src/shared/lib/theme-convert/ui-token-vocabulary.ts:1-108`, 미러 `src/entities/theme/theme-tokens.ts:3-105`, Rust 대조 테스트 `src-tauri/src/domain/theme/service.rs:2146` |
| `syntax` | **31** | `ui-token-vocabulary.ts:110-142` |
| `terminal` | **20** (ANSI 16 + bg/fg/cursor/selection) | `theme-tokens.ts:157-178` |
| 합계 | **184** / 테마 | — |

### 카탈로그 규모 — 문서·라이선스 문서가 stale
- 실제 번들 JSON: **38종** (`src-tauri/resources/themes/*.json`), Rust `include_str!` 등록도 38 (`service.rs:14-85`), + Rust 리터럴 builtin 2종 = **카탈로그 40종** (`service.rs:1045`).
- `docs/theme-system.md:286` 은 38종으로 갱신됨. 그러나 **`THIRD_PARTY_LICENSES.md:17` 은 아직 "36 color themes"** — 라이선스 게이트 테스트(`src/shared/lib/theme-convert/bundled-theme-licenses.test.ts`)가 **id 백틱 등장만** 검사하고 개수는 안 봐서 조용히 어긋나 있음. 과제 지시문의 "31종"도 실제와 불일치.

### 토큰 커버리지 집계 (기계 집계, 38종 전량)
전 테마 **133/31/20 전량 명시**(변환 스크립트가 미충족 시 `exit 1` — `docs/theme-system.md:415`). 따라서 "누락 폴백"은 0이고, 실제 위험은 **서로 다른 토큰이 같은 값으로 접혀 표면이 사라지는 것**이다. 아래는 `$palette` 해석 + `#hex` 정규화 후 집계.

| id | type | 고유색/133 | =SAFE_DEFAULT | =app.bg | tokenColors | ANSI 폴백 |
|---|---|---|---|---|---|---|
| vscode-dark-plus | dark | 32 | **65** | 29 | 65 | Y |
| visual-studio-cpp-dark | dark | 32 | **53** | 30 | 77 | Y |
| visual-studio-cpp-light | light | 27 | **50** | 26 | 78 | Y |
| vscode-light-plus | light | 25 | **50** | 26 | 64 | Y |
| vscode-quiet-light | light | 32 | 35 | 19 | 56 | Y |
| vscode-monokai-dimmed | dark | 34 | 32 | 15 | 72 | |
| darcula | dark | 29 | 24 | 27 | 65 | Y |
| one-monokai | dark | 34 | 20 | 6 | 54 | |
| vscode-kimbie-dark | dark | 32 | 19 | 10 | 43 | Y |
| intellij-islands-light | light | 43 | 19 | 18 | 135 | |
| vitesse-light | light | 32 | 18 | 18 | 56 | |
| vscode-light-modern | light | 32 | 15 | 13 | 64 | Y |
| github-light | light | 46 | 7 | 7 | 45 | |
| vscode-dark-modern | dark | 43 | 4 | 15 | 65 | Y |
| ayu-dark / ayu-light | dark/light | 45/44 | 1/1 | 6/6 | 66 | |
| dracula | dark | 32 | 1 | 14 | 85 | |
| solarized-light / vscode-solarized-dark / vscode-red / vscode-tomorrow-night-blue | — | 33/34/39/30 | 1 각 | 9/9/8/11 | 38/38/41/31 | red만 Y |
| 나머지 17종 (catppuccin-mocha, everforest×2, github-dark, gruvbox-dark, kanagawa-wave, monokai, night-owl×2, nord, one-dark-pro, palenight, rose-pine×2, tokyo-night, vitesse-dark, vscode-abyss) | — | 23~52 | **0** | 3~23 | 31~276 | |

- `=app.bg` 는 `app.background`·`editor.background` 자신 2건 포함(실질값 −2).
- `=app.bg` 최빈 토큰(38종 중 발생 수): `terminal.background`/`terminal.selection` 28, `tabBar.tabActiveBackground` 25, `editor.hoverBackground`·`appSidebar.background` 19, `button.hoverBackground` 18, `editor.inactiveSelection`·`explorer.background`·`panel.background`·`scrollbar.thumb`·`scrollbar.thumbHover`·`list.background` 17, `menu/tooltip/modal.background` 15, `popover.background`·`editor.widgetBackground`·`input.background` 12.
- `=SAFE_DEFAULT` 최빈: `app.shadow` 17, `panel.inputBorder`/`input.border` 9, `editor.hoverBackground` 9, `explorer.indentGuide`/`editor.bracketMatch`/`editor.widgetBorder`/`diff.border`/`terminal.commandBlockBorder`/`popover|tooltip|modal.border` 각 8.

### 변환 파이프라인
- CLI: `bun run themes:convert --input --id --name --type --source-url --author --license [--out] [--include-dir]` (`scripts/convert-vscode-theme.ts:10-70`). `--include-dir` 는 VS Code `include` 체인 해석(최대 깊이 5, `scripts/convert-vscode-theme.ts:8`).
- 매핑: `COLOR_MAPPING`(`mapping-tables.ts:110~`)이 `chain()`(후보 사슬)/`derived()`(파생)로 353 VS Code color ID → 133 토큰. 미해결 시 `FAMILY_FALLBACK_SOURCE_KEYS`(`mapping-tables.ts:11`) → `SAFE_DEFAULT_COLORS`(`:16-31`).
- VS Code 공식 기본값 이식 3종: ANSI 16색(`docs/theme-system.md:445-471`), `list.hoverBackground`(`mapping-tables.ts:41`), `list.activeSelection/inactiveSelection`(`:56,:70`). light 활성선택은 `#ADD6FF` 로 **의도적 발산**(근거 주석 `mapping-tables.ts:49-55`).
- `syntax` 는 최장-prefix scope, `tokenColors` 는 원문 passthrough(`docs/theme-system.md:49-74`).

### 대비 가드 현행 (d-40/d-48 이월 상태)
- `CONTRAST_PAIRS` 7쌍, `MIN_CONTRAST_RATIO = 3` (`contrast.ts:14, 70-83`). blocking 5 + advisory 2(`selectionMatchHighlight`, `selectionForeground`).
- **d-40 이월 없음(완료)**: 선택 행 2축 + `repairPair` 의 `protectedBackgroundKeys` + `ADVISORY_PROTECTED_BACKGROUND_KEYS`(`contrast.ts:124-128`) 구현됨. Rust 린트도 `builtin_light` 포함 40종 순회(`service.rs:1045`)로 확장 완료 — `service.rs:1732/1806/1878/1919` 선택 행 계열 테스트 존재.
- **ΔE 구별성 가드도 구현됨**: `isDistinctFromBodyForeground`, `MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E = 2.3`(`mapping-tables.ts:96-110`), Rust 대응 `service.rs:1484`.
- 단 `mapping-tables.ts:79`·`contrast.ts:57` 주석이 "36 bundled themes" — **주석 수치 stale**(실제 38).

---

### 핵심 질문별 답

### Q1. 테마 시스템 현행 / 커버리지
위 표 참조. 요점 3가지:
1. **커버리지는 형식상 100%**(184 토큰 전량). "폴백 토큰 수"의 의미 있는 대리 지표는 `=SAFE_DEFAULT`·`=app.bg` 접힘 수이며, VS Code 내장 계열 6종(dark-plus, light-plus, cpp-dark, cpp-light, quiet-light, monokai-dimmed)이 압도적으로 취약하다.
2. 폴백 값 자체는 "발명"이 아니라 VS Code 공식 기본값 이식이라는 원칙이 문서·코드 모두에 고정돼 있다.
3. `tokenColors` 보존량 편차가 큼(vscode-tomorrow-night-blue 31 ↔ everforest 276) — 구문 강조 세밀도 격차가 그대로 남는다.

### Q2. "모든 UI 고려" 갭 감사

**(a) raw 색 직접 사용 — 사실상 거의 없음**
- Tailwind 팔레트 클래스(`bg-zinc-*`·`text-gray-*` 등) **전수 0건** (`src/**/*.{ts,tsx,css}`, 테스트 제외).
- `oklch(` **0건**.
- hex 리터럴은 `src/shared/styles/global.css:7-131` 의 FOUC 방지 `:root` 시드값(Catppuccin Mocha 계열)과 `src/features/theme/color-picker.tsx:214,239` 의 색상환 그라디언트뿐 — 둘 다 정당.
- **실제 위반 5건**:

| 파일:줄 | 코드 | 분류 |
|---|---|---|
| `src/shared/ui/dialog.tsx:33` | `bg-black/50` 오버레이 | 토큰 없음 → 라이트 테마에서 과하게 어두움 |
| `src/shared/ui/alert-dialog.tsx:24` | `bg-black/50` 오버레이 | 동일 |
| `src/shared/ui/button.tsx:14` | `text-white` (destructive) | 시맨틱 토큰 아님 |
| `src/features/preview/html-preview.tsx:9` | `bg-white` iframe 배경 | 다크 테마에서 흰 판 |
| `src/features/theme/color-picker.tsx:217,242` | `border-white` 핸들 | 밝은 배경에서 핸들 소실 |

- **추가 발견 — 죽은 `dark:` 분기**: `button.tsx:8,14,16,18`, `dropdown-menu.tsx:56`, `context-menu.tsx:97` 에 shadcn 원본 `dark:` variant 가 남아 있으나 코드베이스 어디도 `.dark` 클래스를 붙이지 않는다(`documentElement` 조작은 `data-theme-ready`/`data-locale-ready` 뿐 — `global.css:304`, `src/shared/hooks/use-reveal-window.ts:21`). 다크 테마 선택 시에도 절대 발동하지 않는 영구 dead branch.

**(b) 토큰은 있으나 소비자가 없는 것 (앱 UI 배선 갭)** — 기계 집계

| 토큰 | 상태 |
|---|---|
| `menu.background` / `menu.border` / `menu.itemHover` / `menu.separator` | **완전 미배선**. `:root` 기본값은 있으나 `@theme inline` 미노출, 소비자 0. 실제 드롭다운·컨텍스트메뉴는 `--popover`/`--accent`/`--border`(=`popover.background`/`list.hoverBackground`/`app.border`)를 쓴다(`shared/ui/dropdown-menu.tsx`, `shared/ui/context-menu.tsx`). → 테마 에디터에서 `menu.*` 를 바꿔도 **화면이 안 바뀐다** |
| `popover.separator`, `tooltip.itemHover`, `tooltip.separator`, `modal.itemHover`, `modal.separator` | 소비자 0. 뒤 4개는 `:root` 기본값조차 없음 |
| `app.shadow` | 소비자 0. 모든 그림자가 Tailwind 기본 `shadow-md`/`shadow-lg`(`dialog.tsx`, `popover.tsx`, `dropdown-menu.tsx`) |
| `explorer.folderIcon`, `explorer.gitModified`, `explorer.gitAdded`, `explorer.gitDeleted` | 소비자 0, `:root` 기본값도 없음. 파일트리는 `git.*` 계열을 쓰고 `file-tree-row.tsx` 는 `explorer.gitUntracked`/`gitIgnored` 만 소비 |
| `scrollbar.thumb` / `thumbHover` / `track` | `@theme inline` 노출은 됐지만 `::-webkit-scrollbar` 규칙이 `global.css` 에 없고(있는 건 `scrollbar-hidden` 유틸뿐) 클래스 소비처 0 → **스크롤바가 테마를 안 따른다** |

정상 배선 확인: `editor.*`·`editorGutter.*`·`diff.*`·`terminal.*` 는 Monaco(`src/shared/lib/monaco/theme.ts`)·xterm(`src/shared/lib/xterm-theme.ts`)·`global.css` 의 `.taide-gutter-*`/`.taide-conflict-*` 규칙이 소비. `graph.lane*` 은 동적 `var(--taide-graph-lane${n})`(`src/widgets/git-panel/commit-graph.tsx:49`, `src/shared/constants/project-display.ts:48`)이 소비.

**(c) 라이트 테마 취약 표면**
1. `bg-black/50` 모달 오버레이 2건 — 라이트 테마 전용 결함.
2. `visual-studio-cpp-light`(고유 27/133, SAFE 50), `vscode-light-plus`(25/133, SAFE 50), `vscode-quiet-light`(32/133, SAFE 35), `intellij-islands-light`(SAFE 19), `vitesse-light`(SAFE 18) — 라이트 6종 중 5종이 상위 취약군.
3. `night-owl-light`(고유 24/133)·`rose-pine-dawn`(23/133)은 SAFE 0이지만 접힘이 심해 표면 구분이 거의 없다.
4. `app.shadow` 미소비 + 라이트 배경 접힘 → 팝오버/모달이 배경과 분리되지 않는 표면이 생긴다.

### Q3. 추가 테마 후보와 절차

**Tier A — 이미 MIT 확인·`THIRD_PARTY_LICENSES.md` 등재된 저장소의 형제 variant (1차 출처 확인 완료, 변환 난이도 S)**

| 이름 | 유형 | 저장소 (이미 등재) | 확인 근거 |
|---|---|---|---|
| Catppuccin Latte | light | catppuccin/vscode | LICENSE=MIT, `contributes.themes` 4종 확인 |
| Catppuccin Frappé | dark | 〃 | 〃 |
| Catppuccin Macchiato | dark | 〃 | 〃 |
| Tokyo Night Storm | dark | enkia/tokyo-night-vscode-theme | package.json license=MIT, 3 테마 확인 |
| Tokyo Night Light | light | 〃 | 〃 |
| Gruvbox Light Medium | light | jdinhify/vscode-theme-gruvbox | license=MIT, 6 테마 확인 |
| Gruvbox Dark Hard / Soft | dark | 〃 | 〃 |
| Rosé Pine Moon | dark | rose-pine/vscode | license=MIT, 6 테마 확인 |
| Ayu Mirage | dark | ayu-theme/vscode-ayu | license=MIT, 6 테마 확인 |
| GitHub Dark Dimmed | dark | primer/github-vscode-theme | license=MIT, 9 테마 확인 |
| GitHub Light/Dark High Contrast | light/dark | 〃 | 〃 (uiTheme `hc-light`/`hc-black` — 변환기 `--type` 은 dark/light 2값뿐이라 매핑 결정 필요) |

이 Tier 는 `THIRD_PARTY_LICENSES.md` 에 **새 섹션이 필요 없다** — 기존 섹션의 "Bundled as:" 목록에 id 만 추가하면 게이트 테스트 통과.

**Tier B — 별도 저장소, 라이선스 미검증 [미확인]**
Cobalt2(wesbos/cobalt2-vscode): package.json 에 **license 필드 없음** 확인 — 저장소 LICENSE 파일 별도 확인 필요. Synthwave '84(robb0wen)·Aura(daltonmenezes): 추정 LICENSE 경로 404 — 재확인 필요. Material Theme, Shades of Purple, Andromeda, Poimandres, Min, Noctis, Bluloco, Panda, Oceanic Next, One Light, Moonlight, Horizon, Winter is Coming, Nord Light, Solarized Dark(별도 저장소) — **전부 미검증**(조사 시간 상한). 참고로 sainnhe/everforest-vscode 는 Dark/Light 2종뿐임을 확인(hard/soft variant 없음).

**추가 절차 (코드 근거)**
1. `bun run themes:convert --input <vscode.json> --id <kebab> --name <표시명> --type dark|light --source-url <repo> --author <name> --license MIT [--include-dir <themes/>]`
2. 실패 조건: 133/31/20 미충족 시 `exit 1`; blocking 5쌍 대비가 수리 후에도 미달이면 거부(`contrast.ts` `validateOutputColors`)
3. Rust 등록: `src-tauri/src/domain/theme/service.rs:14-85` `include_str!` 배열에 `(id, include_str!(...))` 추가
4. `THIRD_PARTY_LICENSES.md` 에 id 를 백틱으로 등재 + **line 17 의 "36" 수치 갱신**
5. 검증 3종: `bundled-theme-licenses.test.ts`(등재 게이트), `bundled-theme-contrast.test.ts`(TS 대비), Rust `service.rs` 카탈로그 린트 7종(`:1349` 토큰 전량, `:1387` app 대비, `:1407` list 구분, `:1442/:1484` matchHighlight 불투명·ΔE, `:1636/:1732/:1806` 대비 3축, `:1878/:1919` list 전경). 예외 등재 테마는 "예외 등재분은 실제로 미달한다" 역검증 테스트(`:1671`,`:1767`,`:1841`)가 짝으로 존재하므로 **예외 목록에 넣으면 미달을 유지해야 한다**

---

### 설계 옵션

### O1. UI 배선 갭 메우기 (권장, 후보 테마보다 선행)
| 항목 | 내용 | 영향 파일 | 규모 |
|---|---|---|---|
| a. `menu.*` 4토큰 배선 | `dropdown-menu.tsx`/`context-menu.tsx` 를 `bg-menu-background`/`border-menu-border`/`focus:bg-menu-item-hover`/`bg-menu-separator` 로 전환 + `@theme inline` 노출 | `global.css`, `shared/ui/dropdown-menu.tsx`, `shared/ui/context-menu.tsx` | M |
| b. 모달 오버레이 토큰화 | `bg-black/50` → 기존 `app.shadow` 소비 또는 신규 토큰 | `shared/ui/dialog.tsx:33`, `alert-dialog.tsx:24` | S (**신규 토큰이면 5곳 동기 → L**) |
| c. 스크롤바 실배선 | `global.css` 에 `::-webkit-scrollbar*` 규칙 추가 | `global.css` | S |
| d. `text-white`/`bg-white`/`border-white` 제거 | `button-primary-foreground`, `editor-background`, `app-border` 로 | 3파일 5줄 | S |
| e. 죽은 `dark:` 제거 | shadcn 원본 잔재 정리 | `button.tsx`, `dropdown-menu.tsx`, `context-menu.tsx` | S |
| f. 미사용 토큰 처리 | `explorer.folderIcon/gitModified/gitAdded/gitDeleted`, `*.separator`, `*.itemHover` 배선 vs 스키마 제거 | 5곳 동기 전부 | **L** (제거는 사용자 테마 호환 파괴) |

장: 테마를 하나도 안 늘려도 "모든 UI 고려"의 실질을 채운다. 단: (f)는 스키마 변경이라 파괴적.

### O2. Tier A 형제 variant 11종 추가 (라이선스 리스크 0)
장: 1차 출처 확인 완료, 라이선스 문서 신규 섹션 불필요, 라이트 3종(Latte·Tokyo Night Light·Gruvbox Light Medium) 추가로 라이트/다크 비율 개선(현 6/38 → 9/49). 단: `hc-light`/`hc-black` 2종은 `--type` 2값 스키마에 안 맞아 결정 필요. 규모 **M**.

### O3. Tier B 확장 (인기 서드파티 16종)
장: 카탈로그 다양성. 단: 저장소별 LICENSE 1차 확인 + `THIRD_PARTY_LICENSES.md` 신규 섹션 16개 + 각 테마 대비 린트 통과(실패 시 예외 등재 사유 문서화) → 규모 **L**, 이번 조사에서 **미검증**이라 착수 전 별도 확인 필요.

### O4. 취약 테마 데이터 정정 (접힘 해소)
`vscode-dark-plus`·`vscode-light-plus`·`visual-studio-cpp-*`·`vscode-quiet-light` 등 SAFE ≥ 35 인 5종을 d-31 §3-A 방식(업스트림 팔레트 스케일 내 재선정, 재변환 아님·손수정)으로 정정. 규모 **M~L**. 단 `docs/theme-system.md:501-630` 이 이미 "재변환 비재현 예외"를 손수정으로 관리 중이라 예외 목록이 더 커진다.

---

### 리스크 · 미확인

1. **[미확인] Tier B 16종 라이선스 전부.** Cobalt2 는 package.json 에 license 필드 없음만 확인, LICENSE 파일 미확인. Synthwave '84·Aura 는 추정 경로 404. 나머지 13종 미착수.
2. **[미확인] 실제 렌더 대비.** 본 감사는 전부 정적 집계다. `~/personal-llm/guidelines.md` §11 기준으로 라이트·다크 실화면 확인 없이 "테마 OK"라 단정할 수 없다.
3. **[미확인] `=app.bg` 접힘이 실제 시각 결함인지.** 원본이 의도적 flat 디자인인 경우(rose-pine 계열)와 폴백으로 접힌 경우를 값만으로 구별 못 한다. 토큰별 `derived()` 경로 추적이 추가로 필요.
4. **주석·문서 수치 stale 3건**: `THIRD_PARTY_LICENSES.md:17`("36"), `mapping-tables.ts:79`, `contrast.ts:57`. 라이선스 게이트 테스트가 개수를 안 봐서 CI로 안 잡힌다.
5. **토큰 5곳 동기 제약**: 신규 토큰 추가는 `ui-token-vocabulary.ts`·`theme-tokens.ts`·Rust `types.rs`/`service.rs` builtin 2종·38개 JSON 전량을 동시에 건드려야 한다(`service.rs:2146` 대조 테스트가 강제).
6. **Rust 예외 등재 역검증 테스트**가 "예외 등재분은 실제로 미달"을 검증하므로, 정정과 예외 등재는 배타적으로 선택해야 한다.

---

### 결정이 필요한 지점

1. **작업 순서** — **A안(추천): O1 UI 배선 갭 먼저 → O2 Tier A** / B안: O2 먼저(가시적 성과) / C안: 병행
2. **추가 테마 수** — **A안(추천): Tier A 9종**(hc 2종 제외 → 카탈로그 49종) / B안: Tier A 11종 전량 / C안: Tier A + Tier B 라이선스 검증 후 선별(총 20종 내외, 규모 L)
3. **라이트/다크 비율 목표** — **A안(추천): 라이트 20% 이상**(현 15.8% = 6/38) / B안: 현행 유지 / C안: 30% 목표
4. **`hc-light`/`hc-black` 처리** — A안: 제외 / **B안(추천): 제외 + 별도 backlog**(고대비는 대비 린트 임계도 달라야 함) / C안: `--type` 에 `hc` 추가(5곳 동기 + 파생 규칙 전반, L)
5. **미배선 토큰 9종 처리** — **A안(추천): 전부 배선**(스키마 불변, 사용자 테마 호환 유지) / B안: 스키마 제거(파괴적) / C안: 현행 유지 + 테마 에디터에서 비활성 표시
6. **전수 감사 범위** — A안: raw 색 5건만 / **B안(추천): raw 색 5건 + 죽은 `dark:` 6곳 + 스크롤바 배선** / C안: B + 40테마 × 라이트/다크 실렌더 검수(규모 L)
7. **취약 테마 데이터 정정(O4)** — **A안(추천): 이번 배치 제외, 별도 계약** / B안: SAFE ≥ 50 인 4종만 선행 / C안: 전량 정정

### keyFacts
- 토큰 총수는 테마당 184개 = colors 133(21 네임스페이스) + syntax 31 + terminal 20. 정의: src/shared/lib/theme-convert/ui-token-vocabulary.ts:1-142, 미러 src/entities/theme/theme-tokens.ts:3-178, Rust 대조 테스트 src-tauri/src/domain/theme/service.rs:2146
- 번들 테마는 38종(src-tauri/resources/themes/*.json, Rust include_str! 등록도 38 — service.rs:14-85). Rust 리터럴 builtin 2종 포함 카탈로그 40종(service.rs:1045). 과제 지시문의 '31종'은 실제와 불일치
- 38종 전부 133/31/20 토큰을 전량 명시한다 — scripts/convert-vscode-theme.ts 가 미충족 시 exit 1(docs/theme-system.md:415). 따라서 '누락 폴백'은 0이고 실제 위험은 서로 다른 토큰이 같은 값으로 접히는 것
- SAFE_DEFAULT_COLORS 값과 동일한 토큰 수 최악 5종: vscode-dark-plus 65/133, visual-studio-cpp-dark 53, visual-studio-cpp-light 50, vscode-light-plus 50, vscode-quiet-light 35. 반대로 17종(catppuccin-mocha·nord·one-dark-pro·tokyo-night 등)은 0건
- 133 토큰 중 고유 색상값 수는 rose-pine/rose-pine-dawn 23개가 최저, night-owl 52개가 최고 — 표면 구분 다양성이 2배 이상 차이난다
- app.background 와 동일한 값으로 접히는 최빈 토큰(38종 중 발생 수): terminal.background·terminal.selection 28, tabBar.tabActiveBackground 25, editor.hoverBackground·appSidebar.background 19, scrollbar.thumb·list.background·panel.background·explorer.background 각 17
- SAFE_DEFAULT 와 동일해지는 최빈 토큰: app.shadow 17종, panel.inputBorder/input.border/editor.hoverBackground 각 9종, explorer.indentGuide·editor.bracketMatch·editor.widgetBorder·diff.border·terminal.commandBlockBorder·popover|tooltip|modal.border 각 8종
- src/**/*.{ts,tsx,css} 전수 grep 결과 Tailwind 팔레트 클래스(bg-zinc-*, text-gray-* 등) 0건, oklch( 0건 — 시맨틱 토큰 규율이 매우 잘 지켜지고 있다
- raw 색 위반은 정확히 5곳: src/shared/ui/dialog.tsx:33 과 alert-dialog.tsx:24 의 bg-black/50 오버레이, src/shared/ui/button.tsx:14 의 text-white, src/features/preview/html-preview.tsx:9 의 bg-white, src/features/theme/color-picker.tsx:217,242 의 border-white
- menu.* 4토큰(background/border/itemHover/separator)은 완전 미배선 — :root 기본값은 있으나 @theme inline 미노출이고 소비자 0. 실제 드롭다운·컨텍스트메뉴는 --popover/--accent/--border 를 쓴다(shared/ui/dropdown-menu.tsx, context-menu.tsx)
- scrollbar.thumb/thumbHover/track 은 @theme inline 에 노출됐지만 global.css 에 ::-webkit-scrollbar 규칙이 없고 클래스 소비처가 0 — 스크롤바가 테마를 따르지 않는다
- 소비자 0인 토큰 추가 목록: app.shadow(그림자는 전부 Tailwind 기본 shadow-md/lg), popover.separator, tooltip.itemHover, tooltip.separator, modal.itemHover, modal.separator, explorer.folderIcon, explorer.gitModified, explorer.gitAdded, explorer.gitDeleted
- shadcn 원본의 dark: variant 가 button.tsx:8,14,16,18 / dropdown-menu.tsx:56 / context-menu.tsx:97 에 남아 있으나 코드베이스 어디도 .dark 클래스를 붙이지 않는다(documentElement 조작은 data-theme-ready/data-locale-ready 뿐) — 영구 dead branch
- d-40 계약(선택 행 대비)은 이월 없이 완료: contrast.ts:70-83 에 selectionMatchHighlight·selectionForeground 2쌍 추가(advisory), repairPair 의 protectedBackgroundKeys 와 ADVISORY_PROTECTED_BACKGROUND_KEYS(contrast.ts:124-128) 구현, Rust 린트도 builtin_light 포함 40종 순회(service.rs:1045)
- ΔE 구별성 가드도 구현 완료: isDistinctFromBodyForeground + MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E = 2.3 (mapping-tables.ts:96-110), Rust 대응 service.rs:1484
- THIRD_PARTY_LICENSES.md:17 이 아직 '36 color themes' 로 stale — bundled-theme-licenses.test.ts 가 id 백틱 등장만 검사하고 개수는 안 봐서 CI가 못 잡는다. mapping-tables.ts:79 와 contrast.ts:57 주석도 '36 bundled' 로 stale
- 이미 라이선스 클리어된 저장소의 형제 variant 11종을 1차 출처로 확인: Catppuccin Latte/Frappé/Macchiato(catppuccin/vscode MIT), Tokyo Night Storm/Light(enkia MIT), Gruvbox Light Medium·Dark Hard/Soft(jdinhify MIT), Rosé Pine Moon(rose-pine MIT), Ayu Mirage(ayu-theme MIT), GitHub Dark Dimmed + Light/Dark High Contrast(primer MIT)
- sainnhe/everforest-vscode 는 Dark/Light 2종만 제공 — hard/soft variant 가 없어 추가 후보가 아니다
- 변환 CLI 는 bun run themes:convert --input --id --name --type dark|light --source-url --author --license [--out] [--include-dir] (scripts/convert-vscode-theme.ts:10-70). --include-dir 는 VS Code include 체인을 최대 깊이 5로 해석
- 신규 테마 등록 절차 4단계: 변환 → src-tauri/src/domain/theme/service.rs:14-85 include_str! 배열 추가 → THIRD_PARTY_LICENSES.md 에 백틱 id 등재 → 검증(bundled-theme-licenses.test.ts, bundled-theme-contrast.test.ts, Rust 카탈로그 린트 7종)
- Rust 카탈로그 린트는 service.rs:1349(토큰 전량)·1387(app 대비)·1407(list 구분)·1442/1484(matchHighlight 불투명·ΔE)·1636/1732/1806(대비 3축)·1878/1919(list 전경). 각 예외 등재분에는 '실제로 미달한다' 역검증 테스트(1671·1767·1841)가 짝으로 있어 정정과 예외 등재가 배타적이다
- 신규 토큰 추가는 5곳 동기가 필요하다: ui-token-vocabulary.ts, theme-tokens.ts, Rust types.rs, service.rs 의 builtin_dark/builtin_light 2종, 38개 번들 JSON 전량. service.rs:2146 대조 테스트가 이를 강제한다
- 라이트 테마는 38종 중 6종(15.8%)뿐이고, 그중 5종(visual-studio-cpp-light SAFE 50, vscode-light-plus 50, vscode-quiet-light 35, intellij-islands-light 19, vitesse-light 18)이 상위 취약군이다
- Cobalt2(wesbos/cobalt2-vscode) 는 package.json 에 license 필드가 없음을 확인했다 — LICENSE 파일 별도 확인이 필요하다

### decisionInputs
- 작업 순서 — A안(추천): O1 UI 배선 갭 먼저 → O2 Tier A 테마 추가 / B안: O2 먼저(가시적 성과) / C안: 병행
- 추가 테마 수 — A안(추천): Tier A 9종(hc 2종 제외, 카탈로그 49종) / B안: Tier A 11종 전량 / C안: Tier A + Tier B 라이선스 검증 후 선별(20종 내외, 규모 L)
- 라이트/다크 비율 목표 — A안(추천): 라이트 20% 이상(현 15.8% = 6/38) / B안: 현행 유지 다크 위주 / C안: 30% 목표로 라이트만 선별 추가
- GitHub High Contrast(hc-light/hc-black) 처리 — A안: 제외 / B안(추천): 제외하되 별도 backlog(고대비는 대비 린트 임계도 달라야 함) / C안: --type 스키마에 hc 추가(5곳 동기 + 파생 규칙 전반 영향, 규모 L)
- 미배선 토큰 9종(menu.* 4, *.separator 3, *.itemHover 2, app.shadow, explorer.folderIcon·git* 4) 처리 — A안(추천): 전부 배선(스키마 불변, 사용자 테마 호환 유지) / B안: 스키마에서 제거(38 JSON + Rust + 사용자 테마 마이그레이션, 파괴적) / C안: 현행 유지 + 테마 에디터에서 비활성 표시
- 전수 감사 범위 — A안: raw 색 5건만 / B안(추천): raw 색 5건 + 죽은 dark: 6곳 + 스크롤바 배선(정적으로 확정된 것 전부) / C안: B + 40테마 × 라이트/다크 실렌더 스크린샷 검수(규모 L, 시간 별도)
- 취약 테마 데이터 정정(O4) — A안(추천): 이번 배치 제외, 별도 계약으로 / B안: SAFE ≥ 50 인 4종만 선행 정정 / C안: 전량 정정
- 모달 오버레이 토큰화 방식 — 기존 app.shadow 재사용(규모 S) vs 신규 오버레이 토큰 신설(5곳 동기 필요, 규모 L)

### unverified
- Tier B 후보 16종(Material Theme, Shades of Purple, Andromeda, Poimandres, Min Dark/Light, Noctis 계열, Bluloco, Panda, Oceanic Next, One Light, Moonlight, Horizon, Winter is Coming, Nord Light, Solarized Dark 별도 저장소)의 라이선스 — 조사 시간 상한으로 1차 출처 확인 미착수
- Cobalt2(wesbos/cobalt2-vscode)의 실제 라이선스 — package.json 에 license 필드가 없음만 확인, 저장소 LICENSE 파일 미확인
- Synthwave '84(robb0wen/synthwave-vscode)와 Aura(daltonmenezes/aura-theme)의 라이선스 — 추정 LICENSE 경로가 404 반환, 재확인 필요
- 실제 렌더 화면에서의 대비·가독성 — 본 감사는 전부 정적 집계다. 라이트·다크 실화면 확인 없이 테마 품질을 단정할 수 없다(personal-llm guidelines §11)
- app.background 와 값이 같은 토큰이 실제 시각 결함인지 여부 — 원본 테마가 의도적 flat 디자인인 경우(rose-pine 계열)와 폴백으로 접힌 경우를 값만으로 구별하지 못한다. 토큰별 derived() 경로 추적이 추가로 필요
- d-26 배치 계약 문서 — docs/acknowledge/ 에서 d-26 파일명을 찾지 못했다(d-38 이후만 존재). 잔여 이월 여부 미확인
- Ayu Mirage / Gruvbox variant 의 VS Code 테마 JSON 이 include 체인을 쓰는지 여부 — 변환 시 --include-dir 필요 여부 미확인
- dark: variant 제거가 안전한지 — Tailwind v4 의 다크 모드 셀렉터 설정(@custom-variant dark)이 global.css 에 없음을 확인했으나, 빌드 설정(vite.config.ts) 레벨에서 주입되는지는 미확인
