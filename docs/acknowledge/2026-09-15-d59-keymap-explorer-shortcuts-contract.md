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

<!-- d59-record-K1 -->
<!-- d59-record-K2 -->

## 4. 후속

- MRU 탭 순환(⌃Tab 을 VS Code 처럼 MRU 로) — 실수요 확인 후.
- 탐색기 단축키 리바인딩(전역 키맵 + explorer when 컨텍스트) — 1.C 완료로 선행 조건은 해소됨.
