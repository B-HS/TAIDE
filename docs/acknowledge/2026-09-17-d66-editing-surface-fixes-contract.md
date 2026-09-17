# d-66 — 편집 표면 전수조사 확인 결함 일괄 수정 (2026-09-17)

> 사용자 지시(d-65 직후): "버그 전수조사해봐 믿을 수가 없네 다른 부분들도 버그될 만한 건 다 바꿔 확실하게 수정".
> 조사 정본 `docs/research/2026-09-17-editing-surface-bug-audit.md`(8관점 finder → 반박 검증 생존 20건, 기각 3건, info 1건).
> 이 계약은 그 문서의 항목 번호(#1~#20, info)를 그대로 쓴다. 증상·재현·원인 인용은 조사 문서 §2 가 정본이고 여기서는
> **수정 결정과 소유·순서**만 정한다.

## 0. 판정 (메인)

- 생존 20건은 검증자가 소스로 재추적해 살아남은 것이고, 메인이 §1 표의 각 항목을 조사 문서 §2 의 인용 소스 라인으로
  재확인했다. 중복 5쌍(#1↔persistence-2, #3↔persistence-1, #4↔focus-keymap-1, #14↔#15, #16↔#17)은 한 지점에서 한 번만 고친다.
- **#12(rescan 후 트리 낡음)** 는 d-57 F4 가 "기록만" 으로 닫았던 항목이다. 사용자의 이번 포괄 지시("버그될 만한 건 다 바꿔")를
  그 결정을 대체하는 새 지시로 보고 수정한다. 수정 지점은 새 커맨드가 아니라 기존 `syncTreeRowsForChangedDirs` 재사용이다.
- **#19(Open to the Side)** 는 `open_tab_in_split` 로 바꾸면 "이미 열린 파일도 항상 분할해서 한 번 더 연다" 로 동작이 바뀐다.
  "옆에 열기" 의 의미상 더 옳으므로 채택하고 문서에 명시한다.
- **info(untitled ⌘S 무반응)** 는 d-65 에서 "별건" 으로 미뤘던 기지 결함이다. 같은 지시로 포함한다.
- **#1 의 방어심층 제안(clean preview 교체분을 closed_tabs 에 기록)** 은 채택하지 않는다 — 단일 클릭 미리보기마다 ⌘⇧T 스택이
  차는 UX 부작용이 검증자 지적대로 크다. `set_dirty`/`pin_tab` 승격 + 교체 대상에서 dirty·pinned 제외로 유실 경로 자체를 없앤다.
- 기각 3건(`stale-ids-4` L1-07 / `open-paths-3` d-58 B-3 / `fileops-tabs-3` d-50 #2)은 기존 결정 유지 — 재론 금지.

## 1. 수정 표 (소유자별)

### R — Rust (1 에이전트, 직렬)

| # | 파일 | 수정 |
|---|---|---|
| #1·#11 | `src-tauri/src/domain/layout/service.rs` | `set_dirty(dirty=true)` 와 `pin_tab(pinned=true)` 에서 `tab.preview = false`(단방향 승격). `open_tab` 의 preview 교체 대상을 `existing.preview && !existing.dirty && !existing.pinned` 로 좁힌다. 테스트: dirty preview 탭 옆에 preview 열기 → 공존 / set_dirty 후 preview false / pinned+preview 탭 유지 + 새 탭 추가 / pin 후 preview false |
| #10 | 같은 파일 `move_tab` | `extract_tab` 뒤·`insert_tab` 직전에 대상 leaf 의 `pinned_count` 로 index 클램프(이동 탭이 pinned 면 `min(pinned_count)`, 아니면 `max(pinned_count)`). 테스트 3건(조사 §2 #10 의 (a)(b)(c)) |
| #16·#17 | 같은 파일 `open_tab` | dedupe 에서 `Terminal{session_id:""}` 제외(`is_dedupable` 가드, 다른 kind 의 dedupe·preview 치환·`next_untitled_index` 무영향). 테스트: 빈 세션 터미널 2개 공존 / 세션 id 채워진 터미널은 여전히 dedupe |
| #20 | `src-tauri/src/domain/project/shell_slots.rs` + `project/service.rs` | `successor_slot_after_prune(tree, slot_id)`(이진 분할이라 "다른 쪽 자식" 의 첫/마지막 리프) 추가. `close_shell_slot` 과 `close_project` 둘 다 prune 전에 후보를 계산하고 `reconcile_focus` 가 후보를 선택적으로 받아 ①분기로 흡수하게 한다. 비포커스 슬롯 닫기는 포커스 불변. 테스트: d-65 R1 (a)~(d) 의 슬롯판 |

### TS-A — 보조 창 정합 (파일 소유: 아래 목록만)

| # | 파일 | 수정 |
|---|---|---|
| 공통 | `src/shared/lib/pane-tree.ts`(+test) | `currentWindowActiveFilePath(layout)` 헬퍼 추가(= `activeFilePathOf(resolveWindowPaneTree(layout, getWindowContext()))`, `currentWindowFocusedPane` 과 대칭) |
| #5 | `src/widgets/command-palette/command-palette.tsx` | `activePath` 를 위 헬퍼로(줄 134 근처만. 줄 354 근처의 `requestReveal` 호출부는 TS-D 소유 — 건드리지 않는다) |
| #6 | `src/widgets/outline-panel/outline-panel-container.tsx`(+신규 test) | 동일 헬퍼, null 가드 |
| #4 | `src/widgets/terminal-pane/terminal-session.tsx`(+test) | `findPaneTab(resolveWindowPaneTree(layout, getWindowContext())?.root …)` 로 자기 창 트리에서 조회(최소안). 테스트: aux 컨텍스트에서 aux 전용 터미널 탭의 cwd 가 `ptyDefaultOptions` 인자로 전달 |
| #2 | `src/app/providers/ide-sync-provider.tsx`(+test) | 로컬 `findFileTabByPath` 제거 → `collectAllPaneTabs(layout)` 로 file 탭 검색. 거짓 `saved:true` 제거가 목표(보조 창 실저장은 후속 — 문서에 구분) |
| #14·#15 | `src/widgets/window-chrome/status-bar-content.tsx`, `src/app/providers/keybindings-runtime-provider.tsx`, 신규 `src/shared/hooks/use-editor-font-size.ts`(위치는 기존 관례 따라) | 폰트 크기 증감 로직을 훅 1개로 추출, 키맵 등록은 `KeybindingsRuntimeProvider`(두 창 분기 공통)에서만. 상태바 버튼은 훅 재사용, 상태바의 `useGlobalKeymap` 두 엔트리 제거 |

### TS-B — 탐색기 (파일 소유: `src/widgets/explorer/**`, `src/widgets/app-shell/project-shell.tsx`, `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx`, `src/app/providers/ipc-sync-provider.tsx` 의 rescan 구간만(TS-A 가 같은 파일의 ide-sync 는 건드리지 않음 — ipc-sync-provider 는 TS-B 단독 소유))

| # | 파일 | 수정 |
|---|---|---|
| #13 | `use-explorer-entry-crud.ts`, `explorer-container.tsx`, `file-tree.tsx`(+test) | 선택을 rows 의 함수로: `displayRows` 에 `selectedId` 가 없으면 selection 클리어(삭제·외부 변경 공통). `targetDirFor` 가 rows 에 없는 경로면 루트 폴백. `buildDisplayRows` 가 targetRow 를 못 찾으면 draft 불성립 |
| #19 | `explorer-container.tsx`(+test) | `openToTheSide` 를 `useOpenTabInSplit`(`layout_open_tab_in_split`, edge right, target = 현재 창 focusedPane, preview false) 단일 mutation 으로 교체. 동작 변경(항상 분할) 문서화 |
| #18 | `use-explorer-auto-reveal.ts`(+test), `explorer-container.tsx`, `project-shell.tsx`, `auxiliary-window-shell.tsx` | `activePath` 를 자기 창 트리로(`activeFilePathOf(layout ? resolveWindowPaneTree(layout, getWindowContext()) : null)` 인라인 — TS-A 의 헬퍼에 의존하지 않는다). `sidebarVisible` 은 `shellView.sidebarCollapsed` 대신 두 셸이 `isCollapsed` state(패널 onLayout/토글에서 갱신)를 prop 으로 내려 주입. 낡은 JSDoc·`docs/features/explorer-sidebar.md:77` 정정 |
| #12 | `ipc-sync-provider.tsx`(+test) | `fs:rescan-required` 에서 `TREE.ROWS` invalidate 대신 기존 `syncTreeRowsForChangedDirs` 로 실제 재조회 — dirs = 캐시된 rows 중 `expanded` 디렉토리 + 프로젝트 루트. 나머지 3개 invalidate 는 유지. JSDoc(225-227) 갱신 |

### TS-C — 편집 지속성 (파일 소유: `src/widgets/editor-pane/**`, `src/features/editor/code-editor.tsx`, `src/shared/lib/monaco/editor-draft-sync.ts`, `src/entities/editor/model-registry.ts`, `src/entities/layout/tab-path-change.ts`(+test), `src/widgets/editor-area/focused-editor-tab.ts`(+test))

| # | 파일 | 수정 |
|---|---|---|
| #3 | `tab-path-change.ts` `releaseClosedFileTabPath` | wait 마커 take 루프·`clearMirror`·`invalidateQueries(FILE.MIRRORS)` 셋을 `stillOpenElsewhere` 가드 아래로. 기존 테스트 `tab-path-change.test.ts:327-339` 의 `clearedMirrors` 기대값을 `[]` 로 정정 + "마커 있는 경로 + stillOpenElsewhere → releasedMarkers=[]" 케이스 추가 |
| #7 | `editor-draft-sync.ts`(`syncModelFromDisk`), `editor-pane.tsx` | `hasUnobservedModelEdit` 를 "`consumeExternallyDirtyModel(path)` 또는 모델 현재 값 ≠ 쓰려는 디스크 내용" 으로 일반화 → 기존 `adoptUnobservedModelEdit` 경로(setDirty + setTabDirty). 테스트: 형제 pane 이 편집한 공유 모델을 두 번째 마운트가 덮지 않고 dirty 로 채택 |
| #8 | `use-editor-file-persistence.ts`, `code-editor.tsx` | 마운트 시 "이 path 의 모델이 이미 존재(라이브)했는가" 를 판별해(`getModel(path)` 선확인) 기존 모델이면 미러 복원 대신 현재 값을 draft 로 채택(`restoreNotice='none'`). 테스트: 형제 dirty 상태에서 마운트 시 `applyExternalContent` 미호출 + 배너 없음 |
| info | `untitled-pane.tsx`, `focused-editor-tab.ts`(+test) | `<CodeEditor registryTabId={tabId}>` 등록 + `SAVE_ROUTABLE_TAB_KINDS` 에 `'untitled'`. ⌘S 가 `handleSaveAs` 에 닿는지 확인(`taide.saveFile` 액션 → `onSave`). 테스트 갱신 |

### TS-D — reveal 재설계 (TS-A·TS-C 완료 후 직렬. 파일 소유: `src/entities/editor/reveal-registry.ts`(+test), `src/entities/layout/layout.query.ts`(`useOpenFileTab` 만), `src/widgets/editor-pane/editor-pane.tsx`(consume 구간만), 호출부 6곳의 reveal 호출 줄만: `search-panel-container.tsx`, `problems-panel-container.tsx`, `command-palette.tsx`(354 근처), `search-editor-pane.tsx`, `editor-area.tsx`(340 근처), `breadcrumbs-bar.tsx`)

| # | 수정 |
|---|---|
| #9 | reveal 대상을 "경로" 가 아니라 "이번에 연 탭" 으로. `useOpenFileTab` 요청에 `reveal?: { line, column }` 을 받아 `onSuccess` 에서 연 탭 id 를 확정(대상 pane 의 `active`)하고, 이미 마운트된 에디터면 `getEditorInstance(tabId)` 로 즉시, 아니면 `subscribeEditorInstance(tabId, …)` 로 마운트를 기다렸다 1회 적용(기존 `REVEAL_PENDING_TTL_MS` 만료 유지). `requestReveal` 의 "경로로 아무 에디터나 찾아 즉시 적용" 분기와 path 키 pending 은 제거하고, 호출부 6곳을 `openFileTab({..., reveal})` 로 통일. `setTimeout` 지연 우회 금지. 테스트: 같은 파일이 다른 pane 에 열려 있어도 새로 연 탭의 에디터에만 reveal 이 간다 / 이미 활성인 탭(dedupe)에도 간다 / TTL 만료 |

### 문서 (마지막, 1 에이전트)

- 이 계약 §3 기록, `docs/bug/2026-09-17-editing-surface-audit-fixes.md`(항목별 증상→수정 요약, 조사 문서 링크), `docs/features/tabs.md`(preview 승격·pinned 구역 불변식·터미널 dedupe·Open to the Side 항상 분할), `explorer-sidebar.md`(autoReveal 창별·삭제 후 선택), `layout-shell.md`(슬롯 포커스 승계), `keymap.md`(폰트 크기 등록처), `editor.md`(공유 모델·미러 복원 규칙·reveal 탭 키), `ipc-contract.md` 해당 항목, d-57 F4 결정 대체 각주.

## 2. 실행·검토 계획

- 웨이브 1: R ∥ TS-A ∥ TS-B ∥ TS-C(opus·xhigh) → 웨이브 2: TS-D(opus·xhigh, TS-A·TS-C 완료 후) → 통합 검증(sonnet·high:
  typecheck·lint·format:check·bun test 전체·cargo fmt·`cargo test -p taide --lib`·clippy) → 렌즈 3(sonnet·xhigh: Rust 불변식 /
  편집 지속성·reveal / 보조 창·탐색기·슬롯) → major·검증 실패 있으면 수정 1회 + 재검증 → 문서.
- 적대적 검증은 조사 단계에서 건별로 이미 수행(생존 20 = 반박 실패). 렌즈 major 는 메인이 소스로 판정.
- 메인 2차 검증: `bun run verify` + `bunx vite build` → 사용자 실기 → 커밋 분할(d-65 와 분리).
- 병행: 조사가 못 본 축(조사 문서 §5)의 2차 finder wf(읽기 전용) — 결과는 별도 계약(d-67)으로.

## 3. 기록 (구현·검토·검증)

> 웨이브 1(R ∥ TS-A ∥ TS-B ∥ TS-C) → 웨이브 2(TS-D) → 통합 검증 → 렌즈 3 순으로 실행했다. 아래는 각 소유자의 보고를
> 메인이 실물 diff 로 대조해 정리한 것이다. 사용자 가시 서술의 정본은 `docs/bug/2026-09-17-editing-surface-audit-fixes.md`.

### 3.1 구현 — 소유자별 요지

**R — Rust (3파일: `domain/layout/service.rs`, `domain/project/service.rs`, `domain/project/shell_slots.rs`)**

- #1·#11 — `set_dirty(dirty=true)`·`pin_tab(pinned=true)` 에 `tab.preview = false`(단방향). `open_tab` 의 preview 교체
  대상을 `existing.preview && !existing.dirty && !existing.pinned` 로 축소. §0 대로 `closed_tabs` 적재는 미채택.
- #16·#17 — `is_dedupable(kind)` 가드로 `Terminal { session_id: "" }`(cwd 무관)만 dedupe 제외. **결정**: fixNote 의
  "모든 Terminal 제외" 가 아니라 조사 §2 의 좁은 형태를 채택했다 — 계약이 요구한 "세션 id 채워진 터미널은 여전히
  dedupe" 테스트와 일치한다.
- #10 — 순수 헬퍼 `clamp_to_pinned_zone(leaf, pinned, index)` 를 `extract_tab` 뒤·`insert_tab` 앞에 삽입.
  **결정**: 프런트 드롭 핸들러 3곳이 아니라 `move_tab` 한 곳에 뒀다(드래그 3경로·⌘K ⌘⇧←/→·향후 호출부가 구조적으로
  덮인다). `APPEND_AT_END`(usize::MAX)는 `insert_tab` 의 기존 `.min(tabs.len())` 과 그대로 맞물린다.
- #20 — `successor_slot_after_prune(tree, slot_id)` 신설(d-65 `successor_leaf_after_prune` 와 동형),
  `reconcile_focus(session, successor)` 가 후보를 선택적으로 받아 ①분기로 흡수. `close_shell_slot`·`close_project`
  둘 다 prune 전에 후보를 계산해 넘긴다.
- **이탈 2건**: ① 계약은 `reconcile_focus` 만 지목했으나 `close_project` 는 `normalize_shell_slots` 를 거쳐야 retain/
  fallback-leaf 를 함께 타므로 `pub fn normalize_shell_slots(session, successor: Option<ShellSlotId>)` 로 파라미터를
  관통시켰다(무관한 호출부 3곳에 `None` 인자만 추가, 동작 불변, 레포 밖 호출자 없음을 전수 grep 확인). ② 계약에 없는
  private 헬퍼 `focus_successor(session, focus_is_doomed)` 를 추가해 "포커스 슬롯이 이번 제거에 포함되는가 + prune 전에
  읽는다" 조건을 두 경로가 공유하게 했다.
- **테스트 18건 신규**(layout 10 + project 8). `close_project` 회귀 테스트는 `session.projects`(사이드바 순서)를 슬롯
  배치와 일부러 다르게 둔다 — 순서가 같으면 옛 폴백이 우연히 정답 이웃을 골라 검출력이 사라진다(실측 확인).

**TS-A — 보조 창 정합 (17파일)**

- 공통 `currentWindowActiveFilePath(layout)`(`shared/lib/pane-tree.ts`) 추가 — `layout` 미로딩 널가드, 슬롯이 사라진
  보조 창은 main 폴백 없이 `null`.
- #5 `command-palette.tsx`(줄 134 한 곳만, TS-D 소유인 reveal 호출부는 미접촉) · #6 `outline-panel-container.tsx` ·
  #4 `terminal-session.tsx`(`resolveWindowPaneTree(...)?.root` 에서 조회) · #2 `ide-sync-provider.tsx`
  (`collectAllPaneTabs`) · #14·#15 `use-editor-font-size` 훅 + `KeybindingsRuntimeProvider` 단일 등록 + 상태바
  `useGlobalKeymap` 두 엔트리 제거.
- **이탈 3건**: ① 신규 훅 위치가 계약의 `src/shared/hooks/` 가 아니라 **`src/entities/settings/use-editor-font-size.ts`**
  다 — eslint 레이어 규칙이 `shared → @entities` 를 금지하고(훅이 `settingsQueryOptions`·`useUpdateSettings` 를 쓴다)
  소비자가 app·widgets 둘이라 공통 하위 레이어가 맞다(선례 `entities/search/use-search-run.ts`). 계약이 "위치는 기존
  관례 따라" 로 열어 둔 부분. ② 소유 목록 밖 파일 2개를 확장자만 바꿔 rename 했다(`ide-sync-provider.test.ts`·
  `keybindings-runtime-provider.test.ts` → `.tsx`, JSX 렌더 테스트 필요, 기존 케이스 보존). ③ 보조 창 URL 전환 헬퍼
  (`window.history.replaceState`, 2줄)를 공용 하네스 대신 4개 테스트 파일에 중복 배치(하네스는 소유 밖 — 후속으로 승격).
- **결정**: 보조 창 컨텍스트는 `mock.module` 이 아니라 `replaceState` 로 만든다(모듈 목·타입 캐스트 불필요, 부작용
  범위가 좁다). #14/#15 의 "메인 창에서 1회만" 은 provider 단독 1회 / 상태바 단독 0회 두 단언의 곱으로 잠갔다.

**TS-B — 탐색기 (16파일)**

- #13 — 조사 fixNote 의 근본 대안을 채택하되 `FileTree` 를 controlled 로 뒤집는 대신 **선택의 진실을 id 로 축소**했다
  (`onSelectionChange` 페이로드 `FileTreeRow` → `string | null`, 중복이던 `onClearSelection` 제거, 컨테이너는
  `rows.find(id)` 로 파생). 문자열 id 는 stale 이어도 조회가 실패해 무해하지만 row 객체는 삭제된 경로를 들고 다닌다.
  `resolveTargetDir(row, rows, projectRoot)` 순수 함수 + `startDraft` 가드를 함께 넣었다.
- #19 — 순수 planner `open-to-the-side-plan.ts` + `useOpenTabInSplit` 단일 mutation. 죽은 `findLeafPane` 헬퍼와
  `useSplitPane` 배선 제거.
- #18 — 자기 창 트리 + `sidebarCollapsed` prop. 두 셸이 `Group.onLayoutChanged` **한 지점**에서만 갱신한다
  (react-resizable-panels 4.12.2 문서상 이 콜백은 드래그뿐 아니라 명령형 `.collapse()`/`.expand()` 와 초기 마운트에서도
  발화 → 토글·Zen·브리지 확장이 한 번에 덮인다).
- #12 — `rescanInvalidations` 에서 `TREE.ROWS` 제거 + 순수 헬퍼 `rescanTreeRefreshDirs(page, projectRoot)` 신설,
  기존 `syncTreeRowsForChangedDirs` 재사용(신규 Rust 커맨드 없음), dirs 가 비면 종전 invalidate 로 폴백.
- **이탈 3건**: ① 계약의 "`buildDisplayRows` 가 targetRow 를 못 찾으면 draft 불성립" 을 **`startDraft` 형성 시점**에
  적용했다 — 트리 rows 는 루트의 자식부터라 정상 경로인 루트 draft 도 targetRow 가 없고(문자 그대로면 "새 파일(루트)" 가
  깨진다), 렌더에서만 빼면 `draft` state 가 남아 소프트락이 된다. ② ①의 "selection 클리어" 를 보정 effect 가 아니라
  파생으로 만족시켰다(컨벤션상 useEffect 는 외부 시스템 동기화 전용, 파생이 더 강한 보장). ③ `explorer-sidebar.md`
  정정은 문서 에이전트 몫이라 코드 JSDoc 만 고쳤다(→ 3.4 렌즈 info, 이번 문서 단계에서 반영).
- 계약 표의 `file-tree.tsx` 실제 경로는 `src/features/explorer/`(widgets 아님)이며 그대로 수정했다. 계약에 없던
  파일 3개(`open-to-the-side-plan.ts`+test, `use-explorer-entry-crud.test.tsx`)는 소유 범위 안의 회귀 테스트 지점이다.

**TS-C — 편집 지속성 (12파일)**

- #3 — `releaseClosedFileTabPath` 의 `stillOpenElsewhere` 판정을 맨 앞으로, `takeWaitMarkers` 루프·`clearMirror`·
  `invalidateQueries(FILE.MIRRORS)` 셋을 그 가드 아래로. 기존 테스트의 `clearedMirrors` 기대값을 `[]` 로 정정하고
  `takeWaitMarkers` 호출 자체를 관측하는 recorder 를 추가했다(take 는 파괴적이라 호출 자체를 막아야 한다).
- #7·#8 — 두 결함을 한 메커니즘으로 닫았다: `code-editor.tsx` 의 `onModelAttach({ path, hadLiveModel })`(신규 optional
  prop, `isPathChange` 일 때만 발화) + `editor-draft-sync.ts` 의 순수 판정 `shouldAdoptLiveModelEdit` +
  `use-editor-file-persistence.ts` 의 마운트 reconciliation(one-shot ref). 인수되면 미러 복원을 건너뛰고 draft 를 모델의
  lazy reader 로 잡는다. `setDirty` 가 `dirtyRef` 를 동기로 옮기므로 같은 커밋의 디스크 sync 가 라이브 dirty 가드에서
  bail → #7 도 함께 닫힌다.
- info — `untitled-pane.tsx` 에 `registryTabId={tabId}`, `SAVE_ROUTABLE_TAB_KINDS` 에 `'untitled'`. ⌘S 는
  `taide.saveFile` → `onSave` = save-as 다이얼로그로 간다(파일 저장 경로가 아니다).
- **이탈 3건**: ① 계약이 적은 "`syncModelFromDisk` 의 `hasUnobservedModelEdit` 일반화" 는 **채택하지 않았다** —
  워처 외부 변경 경로(`fs:changed` → `FILE.CONTENT` 무효화 → `setSyncedContent(새 내용)`)에서는 모델이 아직 옛 내용이라
  `모델값 !== syncedContent` 가 참이 되어 정상 재로드가 "미관측 편집 인수" 로 오판되고 외부 변경이 조용히 사라진다.
  같은 비교에 `hadLiveModelOnAttach` 를 AND 로 묶고 호출 지점을 마운트 reconciliation 으로 옮겼다(결과 동작은 계약과
  동일, 회귀 방지는 `hadLiveModelOnAttach: false` 테스트로 잠금). ② 계약 §1 의 `#7` 행 파일 경로
  (`src/shared/lib/monaco/editor-draft-sync.ts`)는 실재하지 않는다 — 실제 파일은
  `src/widgets/editor-pane/editor-draft-sync.ts`(소유 범위 안)이고 새 파일을 만들지 않았다. ③ `code-editor.tsx` 에
  optional prop 1개를 추가했다(판정 결과를 호스트로 전달할 채널이 없으면 선확인이 무의미하다. 다른 호스트는 무영향).
- **알려진 결과 변화**: 개명 직후 dirty 파일은 `retargetModel` 때문에 항상 "라이브" 로 판정돼 **"복구됨" 배너가 뜨지
  않는다**(본문·dirty 결과는 동일, 복구가 아니라 개명이라 오히려 정확). "미러 == 모델 값이면 기존 복원 유지" 로
  우회하면 #8 의 주 증상을 못 고치므로 채택하지 않았다.

**TS-D — reveal 재설계 (14파일, 웨이브 2)**

- registry API 교체: `requestReveal(path,…)` 제거 → `revealInTab(tabId, target, ttlMs?)` +
  `consumePendingReveal(tabId, editor)`. `monaco.editor.getEditors()` 전역 스캔과 path 키 pending 삭제, pending 은
  `Map<TabId, …>`, TTL 만료 정리는 유지. 부수효과로 registry 가 monaco 런타임 의존을 잃어 테스트의 `mock.module` 우회를
  걷어내고 static import 로 되돌렸다.
- `useOpenFileTab` 요청에 `reveal?` 추가 + `openedFileTabIdOf(layout, target, path)`(대상 pane 의 active 탭을
  **kind/path 재확인 후에만** 채택, 불일치면 reveal 을 버린다).
- 호출부 **9곳** 통일: 열기+reveal 6곳은 `openFileTab({…, reveal})`, 현재 활성 에디터 reveal 3곳은 `revealInTab`.
  브레드크럼의 `{line:1,column:1}` 은 매직넘버 금지 규칙에 따라 `BREADCRUMB_FILE_REVEAL` 상수로 분리.
- **결정**: 적용 지점을 registry 의 `subscribeEditorInstance` 가 아니라 `EditorPane` 의 consume effect 로 뒀다 —
  `CodeEditor` 의 인스턴스 등록은 자식 effect 라 부모의 viewState 복원보다 한 커밋 빠르고, 구독으로 밀면 배경 탭 활성화
  경로에서 복원이 reveal 을 덮어쓴다. `requestReveal` 은 남기지 않고 완전히 제거했다(남기면 #9 증상이 그대로 남는 경로가
  3곳 생긴다).
- **이탈 3건**: ① 계약이 나열한 6곳 외에 `command-palette.tsx` 의 심볼/라인 모드 2곳과 `outline-panel-container.tsx`
  1곳도 마이그레이션했다(남겨두면 path 기반 전역 스캔을 제거할 수 없다). ② TS-B 소유인 `explorer-panel.tsx`(prop 타입
  1줄)·`explorer-container.tsx`(2줄)를 수정했다 — `search-panel-container` 는 실제 open 을 `onOpenMatch` 로 위임하므로
  콜백 시그니처를 `(path, line, column)` 으로 넓히지 않으면 검색 결과만 reveal 불가가 된다(값 전달만 추가, 동작 변화
  없음). ③ 공용 헬퍼를 TS-A 소유 파일에 올리지 못해 두 파일에 2줄 유도를 중복시켰다(→ 후속).

### 3.2 검증 (통합, 전부 exit 0)

| 명령 | 결과 |
|------|------|
| `bun run typecheck` | 에러 0 |
| `bun run lint` | error 0 / warning 11 (기존 `react-hooks/incompatible-library` 6 + `exhaustive-deps` 5, 신규 0) |
| `bun run format:check` | 전부 통과 |
| `bun test` | **2819 pass / 0 fail**, 6259 expect, 282 파일 |
| `cargo fmt --all -- --check` | 위반 0 |
| `cargo test -p taide --lib` | **1685 passed / 0 failed** |
| `cargo clippy --workspace --all-targets -- -D warnings` | 경고 0(`#[allow]` 미사용) |

- 변경 규모: 추적 파일 53개(+1957/-389), 신규 미추적 14개(계약·버그·리서치 문서 4 + 소스·테스트 10). d-65 의 미커밋
  변경이 같은 워킹트리에 섞여 있다(커밋은 분리한다 — §2).
- 각 소유자가 **검출력 실측**(수정만 임시로 되돌려 대상 테스트가 실패하는지 확인 후 원상 복구)을 수행했다. 항목별 수치는
  `docs/bug/2026-09-17-editing-surface-audit-fixes.md` 의 각 "테스트" 줄.
- **앱 실기는 하지 않았다**(지시). UI 동작은 §3.4 의 실기 대상이 정본이다.

### 3.3 렌즈 3 — 발견 전건

| severity | 항목 | 제목 | 처리 |
|---|---|---|---|
| minor | #9 | `openedFileTabIdOf` 의 `target: null` 재계산이 실제 오픈 pane 과 어긋나면 다른 탭으로 reveal 이 샐 수 있다(`layout.query.ts`) | **후속으로 이관.** `withCurrentWindowTarget` 이 뮤테이션 시점 캐시로 확정해 보낸 pane 과 `onSuccess` 가 신선한 레이아웃에서 다시 읽는 pane 이 그 사이 다른 IPC 로 갈릴 수 있고, 그 pane 의 active 탭이 우연히 같은 경로면 kind/path 검사를 통과한다. `open_tab` 은 `focused_pane` 을 바꾸지 않으므로 창이 매우 좁다 — 근본 해법(`mutationFn` 이 보낸 paneId 관통)은 `docs/backlog.md` |
| info | #18 | `docs/features/explorer-sidebar.md` 가 수정 후에도 옛 단일-창 전제를 서술(`resolveWindowPaneTree 금지`·`shellView.sidebarCollapsed`) | **이번 문서 단계에서 반영.** 코드·테스트는 전수 확인·통과였고 문서만 미반영이었다(TS-B 의 이탈 ③이 예고한 항목) |

- **major 0** → 계약 §2 의 "수정 1회 + 재검증" 패스는 실행하지 않았다.
- 적대적 검증은 조사 단계에서 건별로 이미 수행했다(생존 20 = 반박 실패) — 렌즈는 구현 결과만 봤다.

### 3.4 미확인 · 실기 대상

**실기 대상**은 `docs/bug/2026-09-17-editing-surface-audit-fixes.md` §7 표(17행)가 정본이다. 특히 테스트로 잠기지 않은
축만 다시 적는다.

- **#18** — `sidebarCollapsed` 초기값이 `false`(펼침)이고 `onLayoutChanged` 의 초기 마운트 발화에 의존한다. 발화하지
  않는 경로가 있으면 "접혀 있는데 autoReveal 이 도는" 상태가 잠깐 생긴다(보이지 않는 트리에 `tree_reveal` 1회 — 무해하나
  낭비). happy-dom 은 패널 측정이 0이라 이 축을 덮지 못했다.
- **#16·#17** — `open_tab` 이 빈 세션 터미널을 더 이상 dedupe 하지 않으므로, "터미널 토글" 류 프런트 경로가 기존 dedupe 에
  기대 탭 재활성화를 하고 있었다면 탭이 하나 더 열린다. "새 터미널" 과 "토글" 을 구분해 확인할 것.
- **#1** — `set_dirty(true)` 가 preview 를 해제하므로 첫 타이핑 순간 탭 제목의 이탤릭이 사라진다(스펙상 의도된 승격이나
  눈에 보이는 변화).
- **#19** — 이미 열린 파일에 "Open to the Side" 를 하면 같은 파일 탭이 하나 더 생긴다(§0 에서 채택한 시맨틱).
- **#2** — 보조 창 dirty 파일의 저장 요청이 이제 `saved:false` 로 정직하게 실패한다. Claude Code 쪽 표시 방식은 미확인.
- **#12** — 대형 저장소에서 디렉토리를 수십 개 펼쳐 둔 채 체크아웃하면 `펼친 디렉토리 수 + 1` 회의 `tree_refresh` 가
  짧은 스파이크로 몰릴 수 있다(2초 스로틀 안, 창 수만큼 배수).
- **#7·#8** — 루트 밖 파일 편집 후 두 번째 pane 열기, 개명 후 배너 미표시.
- **#9** — 분할 화면의 검색·진단·정의로 이동이 "연 탭" 에서만 움직이는지.
- **배선 미검증**: `fs:rescan-required → syncTreeRowsForChangedDirs`(#12)와 `openToTheSide → openTabInSplit`(#19)의
  호출 자체는 렌더 테스트가 없고 순수 함수 테스트 + typecheck 로만 담보된다.

## 4. 후속

- 사용자 실기 대상은 §3 기록 뒤에 확정.
- 보조 창 파일의 IDE `save_document` 실저장(#2 의 남은 절반)은 요청 릴레이 설계가 필요 — 백로그.
