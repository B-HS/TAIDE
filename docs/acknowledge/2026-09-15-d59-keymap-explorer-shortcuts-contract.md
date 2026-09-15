# d-59 — 사용성 배치 5 웨이브 2: 탭·그룹 단축키 확장 · 탐색기 단축키 · 빈 공간 더블클릭 · 키바인딩 에디터 when 결함 (2026-09-15)

> 사용자 항목 1·2. 결정 전제: `acknowledge/2026-09-15-usability-batch5-user-decisions.md` §1 #1~#3 · §3 #4(웨이브 2 5건 추천안 — 착수 전 사용자 번복 가능).
> 조사 정본 `research/2026-09-15-batch5-research.md` T1. 실행 방식 `docs/agent-operations.md` §2. **d-58 커밋 후 착수**(파일 경합 방지).
> 규칙: 코드 주석 금지(JSDoc 만)·arrow only·매직넘버 상수화·이모지 금지·`#[allow]`/`@ts-ignore`/`eslint-disable` 금지·신규 의존성 0. 로케일 키는 `src-tauri/resources/locales/{en,ko,ja}.json` 3종 동시(기존 `keymap`·`explorer` 네임스페이스만 사용).

## 0. 현행 (메인이 소스로 재확인 — T1)

| 사실 | 근거 |
|------|------|
| 키맵 디스패치는 window capture 리스너 + 매칭 시 preventDefault/stopPropagation → `when` 없는 엔트리는 항상 Monaco/xterm 보다 우선 | `src/shared/hooks/use-keydown-capture.ts:38`, `src/shared/hooks/use-global-keymap.ts:91-98` |
| `APP_KEYMAP` 22개. 탭 관련은 `tab-cycle-next/prev`(⌃Tab/⌃⇧Tab, 인덱스 순환)뿐. 이전/다음 에디터·그룹 포커스·⌘1~9·전체 닫기·탭 좌우 이동 없음 | `src/shared/lib/keymap/keymap.ts:64-129` |
| ⌥⌘←/→ 는 Monaco 카탈로그에 없음(무충돌). ⌥⌘↑/↓ 는 Monaco `insertCursorAbove/Below` → **미배정** | `src/shared/lib/monaco/monaco-actions.ts:305-322` |
| chord 엔진은 ⌘K 아래 형제 chord 복수를 지원(`open-keybindings-editor` ⌘K ⌘S, `toggle-zen-mode` ⌘K Z) | `keymap.ts:207-212`, `keymap-dispatch.ts:107-117` |
| 커맨드 → 에디터 영역 브리지: `requestEditorPaneCommand({type:'cycle-tab', direction})` 선례. `useFocusPane`·`useMoveTab` 뮤테이션 존재(새 IPC 불필요) | `src/shared/lib/bridge/editor-pane-command-bridge.ts:6-14`, `src/entities/layout/layout.query.ts:214,222`, `src/widgets/editor-area/editor-area.tsx:242-256` |
| 키바인딩 에디터 행(`KeybindingRow`)에 `when` 이 없고 `buildKeybindingRows` 가 채우지 않아 스코프가 다른 바인딩(예: 터미널 ⌘↓)도 충돌로 오탐 | `src/shared/lib/keymap/keybinding-catalog.ts:12-26,41-75`, `keymap.ts:307` |
| 파일트리 키 처리는 컨테이너 로컬 `onKeyDown` 한 곳(↓↑→← Enter + 타이프어헤드). Enter = 디렉터리 토글 / 파일 프리뷰. 행 더블클릭 = 고정 열기 | `src/features/explorer/file-tree.tsx:132-145,156-203` |
| 빈 공간 히트테스트(`offsetY / FILE_TREE_ROW_HEIGHT_PX` → `displayRows[index]` 부재)가 컨텍스트 메뉴 핸들러에 이미 있음. 루트 초안 행은 `buildDisplayRows` 가 `parentDir` 미발견 시 depth 0/index 0 삽입으로 이미 동작 | `file-tree.tsx:72-79,205-223` |
| `startDraft(kind)` 는 대상 디렉터리 인자가 없어 `targetDirFor(selectedRow)` 를 읽는다(선택 해제 직후 호출 시 직전 selectedRow 적용 함정) | `src/widgets/explorer/use-explorer-entry-crud.ts:64-71`, `explorer-container.tsx:80-83` |
| 컨텍스트 메뉴 핸들러(`FileTreeContextMenuHandlers`)에 cut/copy/paste/copyPath/copyRelativePath/startRename/requestDelete/revealInFinder 가 이미 있어 키 핸들러가 재사용 가능. `ContextMenuShortcut` 컴포넌트 존재 | `file-tree.tsx:25-39`, `src/shared/ui/context-menu.tsx:165` |
| VS Code macOS 탐색기 기본키(1차 출처 `fileActions.contribution.ts`): Enter=rename(F2), ⌘⌫=휴지통, ⌥⌘⌫=영구 삭제, Space=미리보기(포커스 유지), ⌘X/⌘C/⌘V, ⌘↓=열기. ⌥⌘R/⌥⌘C/⇧⌥⌘C 는 통용값([미확인] — 구현 시 1차 출처 재확인) | T1 Q3 |

## 1. 수정 방향

### 1.A 탭 이동 ⌥⌘←/→ (S)

- `KeymapActionId` 에 `'editor-previous' | 'editor-next'` 추가, `APP_KEYMAP` 에 `{ key: 'ArrowLeft', mods: ['mod','alt'] }` / `ArrowRight`(when 없음). `command-catalog.ts` 에 `tab.previousEditor`/`tab.nextEditor`(`KEYMAP_CATEGORY.TAB`, `requestEditorPaneCommand({type:'cycle-tab', direction})` 재사용). `editor-area.tsx` `useGlobalKeymap` 2줄. 로케일 `keymap.editorPrevious`/`keymap.editorNext` × 3. ⌃Tab/⌃⇧Tab 은 유지.
- 테스트: `keymap.test.ts`·`keybinding-catalog.test.ts` 기대값 갱신(중복 바인딩 검사 통과), 커맨드 수 상수(`docs/PROCESS.md` 기준선 커맨드 185 → 갱신).

### 1.B 그룹 포커스·탭 좌우 이동·전체 닫기 — ⌘K chord (M)

- `⌘K ⌘←/→/↑/↓` = 인접 그룹(pane) 포커스(`focus-group-left/right/up/down`), `⌘K ⌘⇧←/→` = 활성 탭을 인접 그룹으로 이동(`move-tab-to-group-left/right`; 인접 그룹이 없으면 무동작), `⌘K ⌘W` = 현재 그룹 탭 전체 닫기(`close-all-tabs`, 기존 `closeAll` UI 로직 재사용, dirty 탭은 기존 닫기 규칙). `⌘1`~`⌘9` = N번째 그룹 포커스(`focus-group-1`~`-9`, 리프 순서 = 트리 DFS 좌→우·상→하). 전부 `when: '!terminalFocus'`(터미널 포커스 시 ⌘K 가 셸로 가도록 기존 chord 와 동일 규칙).
- 인접 판정 헬퍼: `src/shared/lib/pane-tree.ts` 에 리프 순서 수집(`collectPaneLeaves`)과 방향 인접 계산(분할 트리에서 현재 리프의 부모 `split` 방향과 형제 순서로 좌/우/상/하 결정; 대각 판정 없음 — 같은 축의 가장 가까운 형제 서브트리의 첫/마지막 리프) 추가 + 단위 테스트.
- 브리지: `EditorPaneCommand` 에 `{type:'focus-group', direction|index}`, `{type:'move-tab-to-group', direction}`, `{type:'close-all-tabs'}` 추가. `editor-area.tsx` 가 `useFocusPane`/`useMoveTab`/기존 닫기 뮤테이션으로 처리.
- 커맨드 카탈로그·로케일: 그룹 포커스 4 + 탭 이동 2 + 전체 닫기 1 + ⌘1~9 9 = 16 커맨드, `keymap.*` 16키 × 3. 키바인딩 에디터에는 자동 노출.
- 테스트: pane-tree 인접/순서, chord 형제 매칭(keymap-dispatch), editor-area 핸들러(RTL 또는 브리지 단위).

### 1.C 키바인딩 에디터 `when` 결함 (S)

- `KeybindingRow` 에 `when?: string` 추가, `buildKeybindingRows` 가 `APP_KEYMAP` 엔트리의 `when` 을 채움. 충돌 판정(`findKeymapConflict`/`hasDisjointKeymapWhenScopes`)이 카탈로그 경로에서도 스코프를 반영하도록. 키바인딩 에디터 행에 when 표기(기존 셀이 있으면 재사용, 없으면 작은 보조 텍스트). 테스트: 터미널 ⌘↓ vs 다른 스코프 ⌘↓ 가 충돌로 잡히지 않음.

### 1.D 탐색기 단축키 — 파일트리 로컬 핸들러 (S~M)

- `file-tree.tsx` `onKeyDown` 확장(초안 편집 중 `isEditing` 가드 유지, IME 조합 키 제외): **Enter = 이름 바꾸기**(`onStartRename(row)`; 루트 행 제외), **⌘↓ = 열기(고정 탭)**(`onOpenPinned`), **Space = 미리보기(포커스 유지)**(`onOpenPreview` 후 트리 포커스 유지 — 타이프어헤드의 `key.length === 1` 분기에서 Space 를 먼저 가로챔), **⌘⌫ = 삭제(휴지통)**(`onRequestDelete` — 기존 확인 UX 그대로), **⌘C/⌘X/⌘V**(`onCopy`/`onCut`/`onPaste(row ?? null)`), **⌘N = 새 파일 / ⌘⇧N = 새 폴더**(선택 폴더 하위, 파일 선택 시 그 부모), **⌥⌘R = Finder 표시**, **⌥⌘C = 경로 복사 / ⇧⌥⌘C = 상대 경로 복사**, **F2 = 이름 바꾸기**(보조). 디렉터리 Enter 는 이름 바꾸기로 바뀌므로 토글은 →/← 와 클릭이 담당(VS Code 동일). 핸들러는 `preventDefault` 로 전역 키맵(⌘W 등 when 없는 것)과 겹치지 않는 키만 사용 — ⌘W 는 손대지 않는다.
- 컨텍스트 메뉴(`file-tree-context-menu.tsx`)의 해당 항목에 `ContextMenuShortcut` 으로 단축키 라벨 표기(macOS 기호, 기존 `formatKeymapShortcut` 유틸 재사용 가능하면 사용 — 키맵 엔트리가 아니므로 정적 표 `EXPLORER_SHORTCUT_LABELS` 상수화).
- 문서: `docs/features/explorer.md`(또는 파일트리 절) 단축키 표 + "리바인딩 불가(로컬 핸들러)" 명시. Welcome 단축키 표는 변경 없음.
- 테스트: file-tree RTL 키 테스트(각 키 → 콜백 호출, 편집 중 무시, 루트 행 Enter 무시).

### 1.E 빈 공간 더블클릭 → 루트 새 파일 초안 (S)

- `file-tree.tsx` 컨테이너 `onDoubleClick`: 기존 히트테스트 3줄 재사용, 행이 잡히면 return(행 더블클릭은 행 핸들러 몫), 빈 공간이면 `onClearSelection()` 후 **루트 대상 새 파일 초안**. `startDraft(kind, targetDir?)` 시그니처에 명시 대상 인자를 추가해 `project.root` 를 넘긴다(직전 selectedRow 함정 회피). `explorer-container.tsx` 배선 + `FileTree` prop `onNewFileAtRoot`(또는 기존 `onNewFile` 을 대상 인자형으로 확장 — 2회 이상 사용이면 확장).
- 테스트: 빈 공간 더블클릭 → 루트 초안, 행 더블클릭 → 기존 동작 유지.

### 1.F 범위 외

- ⌥⌘↑/↓(미배정 확정), MRU 탭 순환, 전역 `explorerFocus` when 컨텍스트, 탐색기 키 리바인딩, 탭 바 드래그 외 탭 재정렬 UI.

## 2. 실행 계획

- 구현 wf(opus·xhigh, TS 전용 — Rust 무접촉): **K1** 1.A+1.B+1.C(키맵·카탈로그·브리지·editor-area·pane-tree — 같은 파일군이라 1 에이전트 순차) ∥ **K2** 1.D+1.E(파일트리·컨텍스트 메뉴·explorer-container). 로케일 JSON 은 Edit 로 자기 키만.
- 렌즈 검토 wf(sonnet·xhigh 2렌즈: 회귀(기존 바인딩·Monaco 충돌)·경계(chord/when)). 메인 2차 `bun run verify`·vite build → 커밋(feat(keymap)·feat(explorer)) → dev 푸시 → main ff.
- 사용자 실기: ⌥⌘←/→ 가 OS 에 선점되지 않는지([미확인]), ⌘K chord 4방향, ⌘1~9, 탐색기 키 전부, 빈 공간 더블클릭.

## 3. 구현 기록

- **§1.A·§1.B·§1.C 완료 (2026-09-15)** — 변경 파일: `src/shared/lib/keymap/keymap.ts`(`KeymapActionId` 18건 추가 + `APP_KEYMAP` 23→41 + `GROUP_SHORTCUT_TERMINAL_GUARD_WHEN` 상수 + `resolveOverriddenKeymapWhen` export·파라미터를 `Pick<KeymapEntry,'key'|'mods'|'chord'|'when'>` 로 확장), `src/shared/lib/pane-tree.ts`(`PaneDirection`·`collectPaneLeaves`·`paneLeafAtPosition`·`findPanePath`(private)·`findAdjacentPaneLeaf` 신설), `src/shared/lib/bridge/editor-pane-command-bridge.ts`(`PaneFocusTarget` + `focus-group`·`move-tab-to-group`·`close-all-tabs` 커맨드 3종), `src/shared/lib/command-catalog.ts`(커맨드 18건 추가, 25→43), `src/widgets/editor-area/editor-area.tsx`(`useFocusPane`·`closeTabAsync` 추가, `focusGroup`/`moveActiveTabToGroup`/`closeAllTabsInFocusedGroup` 핸들러 + `useGlobalKeymap` 18줄 + 브리지 분기 3줄), `src/shared/lib/keymap/keybinding-catalog.ts`(`KeybindingRow.when` 필드 + 두 행 빌더에서 채움 + 오버라이드 병합 시 `resolveOverriddenKeymapWhen` 적용 + `findConflictingRowInIndex` 후보에 `when: row.when` 전달), `src/widgets/keybindings-editor/keybindings-editor.tsx`(저장 전 충돌 미리보기도 재바인딩 후 실효 `when` 으로 판정), `src/features/settings/keybinding-row.tsx`(스코프 있는 행에 `when` 원문 표기), `src-tauri/resources/locales/{en,ko,ja}.json`(`keymap.*` 18키 × 3 = 54줄), `src-tauri/src/domain/locale/service.rs`(`MESSAGE_NAMESPACES` 의 `keymap` 배열에 18키 등재 — Edit 로 자기 키만), `docs/features/keymap.md`(§1 엔트리 수·`when` 보유 수 갱신, §5.1 "⌘K 네임스페이스 현황" 신설, §8 d-59 카탈로그 `when` 갭 문단, §9 행 `when` 표기 2줄), `docs/features/tabs.md`(§6 키보드 표 5행 추가 + d-59 구현 메모). / 테스트: **27건 추가**(`pane-tree.test.ts` +12 → 33, `editor-pane-command-bridge.test.ts` +4 → 9, `keymap.test.ts` +6 → 111(APP_KEYMAP 전 엔트리 쌍별 중복 바인딩 검사 mac·비mac 2건 포함), `keybinding-catalog.test.ts` +5 → 42), **갱신 2건**(`keymap-chord-store.test.ts` 의 ⌘K pending 후보 기대값 2→9건, `keybinding-catalog.test.ts` 의 전량 스캔 오라클에 `when` 전달 — 규칙이 바뀌었으므로 오라클도 같이 바뀌어야 "색인은 후보만 좁힌다" 가 유효하다). 전체 `bun test` 2493 pass 0 fail(246 파일, K2 분량 포함 / 착수 전 기준선 2438), `bun run typecheck` 통과, `bun run lint` 0 error(기존 `useVirtualizer` 경고 11건은 무관), `bunx prettier --check` 만진 파일 전부 통과, `cargo test -p taide --lib locale` 19 pass, `cargo fmt --all --check` 통과. / 이탈: ① 브리지 커맨드를 계약 문구의 `{type:'focus-group', direction|index}` 대신 **`{type:'focus-group', target: PaneFocusTarget}`**(`{kind:'direction'}` | `{kind:'position'}`) 로 뒀다 — 커맨드 타입은 하나로 유지하면서 한 필드에 두 의미를 얹는 선택 필드 조합을 피했고, 위치는 사용자가 누르는 ⌘1~⌘9 와 같게 **1-based `position`**(변환은 `paneLeafAtPosition` 한 곳)이라 `index` 라는 이름을 쓰지 않았다. ② 커맨드 id 는 `tab.closeAll` 이 아니라 **`tab.closeAllInGroup`** — 동작 범위가 포커스된 그룹이고, 라벨 로케일 키 `tab.closeAll`(탭 바 메뉴)과 혼동을 피했다. ③ `focus-group-1`~`-9` 를 배열 생성이 아니라 **엔트리·커맨드·핸들러 모두 리터럴 9건씩** 적었다 — `map` 은 `keymapId`/`titleKey` 를 `string` 으로 넓혀 "모든 커맨드가 실재하는 키맵 엔트리를 가리킨다" 는 컴파일 보장을 잃는다(`MONACO_ACTIONS` 와 같은 이유). ④ 계약 §1.C 범위를 **오버라이드 경로까지** 확장했다 — 행에 `when` 을 채우기만 하면 chord 를 다른 1단으로 옮긴 재바인딩에서 카탈로그가 디스패치가 이미 버린 게이트를 계속 주장한다. 그래서 `resolveOverriddenKeymapWhen` 을 export 해 카탈로그·에디터가 같은 규칙을 쓰게 했다(`keymap.ts` 의 기존 chord 2건은 리터럴 `'!terminalFocus'` 그대로 두고 신규 16건만 상수를 쓴다 — 최소 변경). ⑤ 행의 `when` 표기는 **원문 그대로**이고 새 로케일 키를 만들지 않았다(바로 위 컨텍스트 인스펙터가 쓰는 어휘와 같아야 한다). ⑥ `editor-area` RTL 테스트 대신 **브리지 단위 + pane-tree 단위**로 고정했다(계약 §1.B 가 허용한 선택지). ⑦ `docs/PROCESS.md` 의 기준선 커맨드 수(185)는 지시대로 손대지 않았다 — `DEFAULT_COMMANDS` 는 25→43 이므로 갱신 필요(메인 몫). / 미확인·후속: **⌥⌘←/→ 가 macOS/WebView 에 선점되지 않는지 실기 확인 필요**(계약 §2 의 `[미확인]` 그대로 — monaco 카탈로그 무충돌만 정적으로 확인했다). ⌘K ⌘←/→/↑/↓·⌘1~9·⌘K ⌘W·⌘K ⌘⇧←/→ 의 실제 동작, 키바인딩 에디터의 `when` 표기 렌더(라이트·다크)도 렌더 확인 전이다. `findAdjacentPaneLeaf` 는 기하가 아니라 트리 중첩 기준이라(픽셀 `sizes` 비율만 있어 참 좌표가 없다) 3분할 이상 비대칭 배치에서 사용자의 "시각적 이웃" 과 어긋날 수 있다 — 실사용 불만이 나오면 그때 재검토.
- **§1.D·§1.E 완료 (2026-09-15)** — 변경 파일: `src/features/explorer/explorer-shortcuts.ts`(신규 — 단축키 바인딩 표 + `findExplorerShortcutId` 매처 + `EXPLORER_SHORTCUT_LABELS` 정적 라벨), `src/features/explorer/file-tree.tsx`(로컬 `onKeyDown` 확장 + IME 가드 + 컨테이너 `onDoubleClick` + 히트테스트 헬퍼 `rowAtClientY` 추출), `src/features/explorer/file-tree-context-menu.tsx`(항목 10개에 `ContextMenuShortcut` 라벨), `src/widgets/explorer/explorer-panel.tsx`·`explorer-container.tsx`(`onNewFileAtRoot` 배선), `src/widgets/explorer/use-explorer-entry-crud.ts`(`startDraft(kind, explicitTargetDir?)`), `docs/features/explorer-sidebar.md`(§2.5 단축키 표 + "리바인딩 불가" 명시, §2.2 키보드·더블클릭 줄 갱신). / 테스트: 신규 2파일 `explorer-shortcuts.test.ts`(8) + `file-tree.test.tsx`(20) = 28건 추가, 갱신 0건. `bun test src/features/explorer src/widgets/explorer` 78 pass 0 fail, 전체 `bun test` 2493 pass 0 fail (246 파일, K1 분량 포함). `bun run typecheck`·`bun run lint`(0 error) · `bunx prettier --check` 만진 파일 전부 통과, `cargo test -p taide --lib locale` 19 pass. 로케일 키 신규 0건(기존 `explorer.*` 재사용, 단축키 라벨은 기호라 번역 대상 아님) → `MESSAGE_NAMESPACES` 무변경. / 이탈: ① **"루트 행 Enter 무시" 는 대상이 존재하지 않는다** — Rust `push_page_rows` 가 프로젝트 루트의 *자식* 부터 depth 0 으로 내므로 루트 행 자체가 `rows` 에 없다. 대신 유일한 depth 0 합성 행인 초안 행(`DRAFT_ROW_ID`)을 모든 행 의존 단축키에서 제외하고, "초안 편집 중 Enter 가 이름 바꾸기를 부르지 않는다" 를 테스트로 고정했다. ② 컨텍스트 메뉴 라벨의 수식키 순서는 앱 기존 `formatKeymapShortcut`(⌃⌥⇧⌘, Apple 표기)을 따르므로 계약 본문의 `⇧⌥⌘C` 가 화면에는 `⌥⇧⌘C` 로 그려진다(두 번째 표기 규약을 만들지 않기 위함). 라벨 표와 바인딩 표의 어긋남은 `explorer-shortcuts.test.ts` 가 수식키 접두사 대조로 막는다. ③ 라벨은 macOS 기호 고정이다 — Finder 표시처럼 macOS 전용 동작이 섞여 있어 플랫폼 분기를 두지 않았다(매칭 자체는 `matchesKeymapEntry` 로 mod→Ctrl 이 풀리며, 비 macOS 케이스도 테스트에 있다). ④ `onNewFileAtRoot` 는 기존 `onNewFile` 확장이 아니라 별도 prop 이다(대상 해석이 "선택 기준" vs "루트 고정" 으로 다르고 호출부가 각각 1곳). / 미확인·후속: **`␣` 미리보기 뒤 포커스** — 트리는 포커스를 넘기지 않지만 파일이 열리며 `code-editor.tsx` 의 `editor.focus()` 가 가져갈 수 있다(클릭 미리보기와 동일 경로). 되찾아오는 처리는 비동기 IPC 완료 시점과 경합이라 넣지 않았고, 실기 확인 항목으로 문서(§2.5)에 `[미확인]` 표기. 행 자체의 더블클릭(고정 열기, 미변경)은 happy-dom 에서 가상화 뷰포트 높이가 0 이라 행이 마운트되지 않아 RTL 로 못 잡는다 — 컨테이너가 행 위 더블클릭을 가로채지 않는다는 절반만 테스트로 고정했고 나머지는 e2e 영역이다. 탐색기 단축키 리바인딩은 계약 §4 후속 그대로.


- **메인 1차 검증 (2026-09-15)** — `bun run typecheck` 0 · `bun run lint` 0 error · `bun run format:check` 통과 · `bun test` 2493 pass/0 fail(246 파일) · `cargo fmt --check` 통과 · `cargo test -p taide --lib locale` 19 pass. 워킹트리 27파일 +980/-41.
- **렌즈 검토 (2026-09-15, wf `wf_6a2844f3`, sonnet·xhigh 2렌즈 + major 반박 2표)** — 발견 5: major 2·minor 3. 메인 판정: **전건 수용**.

| id | 심각도 | 내용 | 판정·수정 방향 |
|----|------|------|------|
| B-1 | major | `formatKeymapStage` 가 Arrow 키를 글리프로 바꾸지 않아 신규 9개 화살표 바인딩 라벨이 "ARROWLEFT" 등으로 렌더 | Arrow→`←→↑↓` 매핑 표(상수) + 라벨 단위 테스트 |
| G-1 | major(반박 2표 모두 실패, 심각도 의견 분열) | chord 엔진이 에디터 텍스트 포커스 중 앱 chord 진입을 억제(Monaco 양보, Wave H 규칙)해 신규 ⌘K 7개가 "편집 중" 에 무동작 — 기존 규칙의 확장 적용이지만 신규 기능의 주 사용 시나리오를 죽임 | (a) 채택: 7개 chord 를 Monaco 액션(`editor.addAction`, `KeyMod.chord`)으로 **에디터 포커스 상태에서도** 등록해 `requestEditorPaneCommand` 로 위임. Monaco 의 ⌘K 네임스페이스와 2단 무충돌(검토 확인). 키바인딩 재정의 반영은 유효 키맵 엔트리에서 Monaco 키코드로 변환해 등록(변환 유틸 존재 시), 불가하면 기본 키만 등록하고 keymap.md 에 제약 명시 |
| B-2 | minor | editor-area 실행부(focusGroup 무동작 분기·closeAll pinned 제외) 테스트 부재 | 얇은 단위 테스트 추가 |
| G-2 | minor | 에디터 포커스 중 앱 chord 가 `observe-monaco-chord-prefix` 로 빠지는 동작을 고정하는 테스트 부재 | keymap-dispatch 회귀 테스트 + Monaco 액션 등록 테스트 |
| B-3 | minor | PROCESS 기준선 커맨드 수 미갱신 | 메인이 문서 커밋 시 갱신(DEFAULT_COMMANDS 25→43, APP_KEYMAP 23→41, 로케일 +18키) |

- **검토 수정 완료 (2026-09-15)** — 수용 4건(B-1·G-1·B-2·G-2) 수정. B-3 은 메인 몫이라 손대지 않았다.
  - **B-1 (Arrow 글리프)** — `src/shared/lib/keymap/keymap.ts`: `ARROW_KEY_LABEL` 상수(`arrowleft|arrowright|arrowup|arrowdown → ←→↑↓`) + `formatKeymapStageKey` 헬퍼를 두고 `formatKeymapStage` 의 `stage.key.toUpperCase()` 자리를 그것으로 바꿨다(그 외 키는 종전대로 대문자 raw key). 플랫폼 분기 없음 — `MONACO_ACTIONS.defaultBindingLabel` 도 같은 글리프(`⌥⌘↑`)를 쓰므로 앱 행과 monaco 내장 행이 한 표에서 같은 표기로 읽힌다. 부수: `src/features/explorer/explorer-shortcuts.ts` 의 JSDoc 이 "`formatKeymapShortcut` 는 ARROWDOWN 을 낸다" 고 단언하던 문장을 실제 동작(`BACKSPACE`/`ENTER` 예시)으로 정정.
  - **G-1 (편집 중 ⌘K 7건 무동작)** — monaco 액션 미러로 해소. 신규 `src/shared/lib/monaco/monaco-group-shortcut-actions.ts`(순수: 키맵 id→`EditorPaneCommand` 표 7건 + `isEditorGroupShortcutKeymapId` + `buildEditorGroupShortcutActions(entries)`), 신규 `src/features/editor/editor-group-shortcut-actions.ts`(`attachEditorGroupShortcutActions(editor, keymapOverridesJson, t)` — `attachAiInlineEditAction` 과 동일한 `addAction` 형태, `keybindingContext: 'editorTextFocus'`, `run` 은 `requestEditorPaneCommand`), `src/features/editor/code-editor.tsx`(`useKeymapOverridesJson()` + 등록 effect 6줄), `src/shared/hooks/use-global-keymap.ts`(`useKeymapOverridesJson` 를 `export` 로 열고 사유 JSDoc 3줄). **키코드는 유효 키맵에서** — `applyKeymapOverrides(APP_KEYMAP, parseKeymapOverrides(json))` 결과를 기존 `buildMonacoChordKeybinding`(1단 저 16비트·2단 고 16비트)으로 인코딩하므로 재바인딩이 그대로 반영되고, effect 의존성이 오버라이드 **JSON 문자열**(렌더마다 새로 만들어지는 배열이 아니라)이라 오버라이드가 바뀔 때만 재등록된다. 문서 제약 명시는 불필요(재바인딩 반영됨).
  - **B-2 (실행부 테스트 부재)** — 신규 `src/widgets/editor-area/group-shortcut-targets.ts` 로 두 순수 함수(`resolveFocusGroupPaneId`·`collectClosableTabIdsInFocusedGroup`)를 뽑고 `src/widgets/editor-area/editor-area.tsx` 의 `focusGroup`/`closeAllTabsInFocusedGroup` 을 그 호출로 얇게 만들었다(동작 동일 — 원래도 렌더 시점 `leaf.tabs` 스냅샷을 순회했다). `paneLeafAtPosition` import 는 사용처가 옮겨가 제거.
  - **G-2 (양보 동작·등록 고정)** — `src/shared/lib/keymap/keymap-dispatch.test.ts` 에 회귀 5건(⌘K 가 `observe-monaco-chord-prefix` 로 빠짐 / 에디터 비포커스면 7건 전부 후보로 `enter-chord` / 무장 중 2단 `⌘←` 는 `defer-to-monaco` / ⌘1 은 chord 가 아니라 편집 중에도 `dispatch` / 7건이 전부 ⌘K 1단이라는 전제), 신규 `src/shared/lib/monaco/monaco-group-shortcut-actions.test.ts` 13건(7개 id·라벨키·커맨드 페이로드, 기본/재바인딩/chord 해제/미지원 키코드의 키 인코딩, monaco 내장 ⌘K chord 21건과 2단 교집합 0).
  - 문서: `docs/features/keymap.md` §5.2 신설(미러 등록 구조·유효 키맵 변환·`MONACO_ACTIONS` 에 넣지 않는 이유·⌘1~9 제외·무충돌 고정) + §9 에 화살표 글리프 라벨 항목, `docs/features/tabs.md` §6 에 "편집 중" 항목.
  - **테스트**: **36건 추가**(`keymap.test.ts` +5 → 121, `keymap-dispatch.test.ts` +5 → 40, `monaco-group-shortcut-actions.test.ts` 신규 13, `group-shortcut-targets.test.ts` 신규 13), 갱신 0건. 전체 `bun test` **2529 pass 0 fail**(248 파일 / 착수 전 2493·246). `bun run typecheck` 통과, `bun run lint` **0 error**(기존 `useVirtualizer` 경고 11건은 무관), `bunx prettier --check` 만진 파일 전부 통과. Rust 무접촉.
  - **이탈**: ① 계약 문구의 `KeyMod.chord(...)` 대신 기존 `monaco-keybinding.ts::buildMonacoChordKeybinding` 을 재사용했다 — 같은 `(first & 0xffff) | (second << 16)` 패킹이고, `monaco-editor` 는 모듈 로드 시 `window` 를 건드려 `bun test` 에서 import 할 수 없으므로 순수 빌더가 테스트 가능한 유일한 형태다. ② 액션 id 는 camelCase(`taide.focusGroupLeft`)가 아니라 **키맵 id 그대로**(`taide.focus-group-left`) — 계약 문구를 따랐고, 이들은 `TAIDE_CUSTOM_ACTIONS` 행이 아니라 미러라서 어느 엔트리의 미러인지 id 에 드러나는 편이 낫다. ③ 이 7개를 `MONACO_ACTIONS`/`TAIDE_CUSTOM_ACTIONS` 에 **넣지 않았다** — 넣으면 키바인딩 에디터에 원본과 별개 행이 한 벌 더 생겨 같은 키를 두 행이 주장한다(`getSupportedActions()` 집합에 id 가 섞이는 것은 무해: 그 집합은 `isEnabled` 조회용 게이트일 뿐 행을 열거하지 않는다). ④ 오버라이드가 chord 를 없애 단일 키가 되면 **키 없이 액션만** 등록한다 — 단일 단계 엔트리는 애초에 에디터 포커스 게이트를 받지 않아 앱 디스패치가 그 키의 단독 주인이고, monaco 에도 걸면 한 키에 주인이 둘이 된다. monaco KeyCode 가 없는 키로 재바인딩된 경우도 같게 처리(액션은 남아 F1 에서 실행 가능). ⑤ `formatKeymapStage` 는 private 이라 직접 테스트하지 않고 `formatKeymapShortcut`(1단·2단 모두 이 헬퍼를 탄다)으로 고정했다 — 테스트를 위해 내부 헬퍼를 export 하지 않았다. ⑥ B-2 에서 `moveActiveTabToGroup` 은 검토 항목에 없어 손대지 않았다(추출 2건만). ⑦ `useKeymapOverridesJson` 를 export 로 여는 것은 `use-global-keymap.ts` 를 건드리는 변경이지만 G-1 의 "오버라이드 반영" 요구에 직접 필요하다 — 파싱·적용을 복제하는 대신 같은 캐시 구독을 재사용했다.
  - **미확인·후속**: 실기 확인 전이다 — 편집 중 `⌘K ⌘←/→/↑/↓`·`⌘K ⌘⇧←/→`·`⌘K ⌘W` 가 실제로 발동하는지, 화살표 글리프 라벨이 키바인딩 에디터·팔레트·Welcome 에서 라이트/다크 모두 정상 렌더되는지. monaco 미러는 `editorTextFocus` 스코프라 Find 위젯처럼 에디터 컨테이너 안이지만 텍스트 입력이 아닌 포커스에서는 앱 경로도 monaco 경로도 잡지 않을 수 있다(앱은 `editorTextFocus` 게터가 `.monaco-editor` closest 로 참이라 양보, monaco 는 `editorTextFocus` 가 거짓) — 실기에서 재현되면 후속.


- **메인 2차 검증 최종 (2026-09-15, 검토 수정 반영 후)** — `bun run verify` exit 0(typecheck 0 · lint 0 error/11 기존 warning · prettier 통과 · bun test 2529 pass/0 fail(248 파일) · cargo fmt/clippy 0 · cargo test lib 1552 + 통합 3+6+17+4, 전부 0 fail) · `bunx vite build` exit 0 · `bun run typecheck:e2e` exit 0. 커밋 3분할(feat(keymap)·feat(explorer)·docs) → dev 푸시 → main ff. 기준선 갱신: APP_KEYMAP 23→41, DEFAULT_COMMANDS 25→43, 로케일 +18키. 사용자 실기 대상: ⌥⌘←/→ OS 선점 여부·편집 중 ⌘K ⌘←/→/↑/↓·⌘K ⌘⇧←/→·⌘K ⌘W·⌘1~9·키바인딩 에디터 when/화살표 글리프 렌더·탐색기 11개 키(Enter=이름 바꾸기 변경 체감·Space 미리보기 후 포커스)·빈 공간 더블클릭.

## 4. 후속

- MRU 탭 순환(⌃Tab 을 VS Code 처럼 MRU 로) — 실수요 확인 후.
- 탐색기 단축키 리바인딩(전역 키맵 + explorer when 컨텍스트) — 1.C 완료로 선행 조건은 해소됨.
