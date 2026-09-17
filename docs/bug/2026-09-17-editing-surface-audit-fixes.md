# 편집 표면 전수조사 확인 결함 21건 일괄 수정 (2026-09-17)

> 사용자 지시(d-65 직후): "버그 전수조사해봐 믿을 수가 없네 다른 부분들도 버그될 만한 건 다 바꿔 확실하게 수정".
> 조사 정본은 `docs/research/2026-09-17-editing-surface-bug-audit.md`(8관점 finder → 반박 검증 생존 20건 + info 1건,
> 기각 3건), 수정 결정·소유·순서는 `docs/acknowledge/2026-09-17-d66-editing-surface-fixes-contract.md`.
> 아래 `#N` 은 전부 그 조사 문서의 항목 번호이며, 증상·재현의 정본은 조사 문서 §2 다. 이 문서는 **무엇을 실제로
> 고쳤는가**를 기록한다. 선행 배치는 `docs/bug/2026-09-17-stale-focused-pane-and-focus-not-following-click.md`(d-65).

## 1. Rust — 레이아웃 불변식 · 셸 슬롯

### #1·#11 — dirty/pinned preview 탭이 다음 preview 열기에 파괴된다

- **증상**: preview 탭을 편집해 dirty 로 만든 뒤 다른 파일을 preview 로 열면 편집 중이던 탭이 확인 없이 사라지고,
  `close_tab` 을 거치지 않아 ⌘⇧T 로도 못 되살린다. preview 탭을 고정해도 preview 플래그가 남아 다음 단일 클릭이
  고정 탭을 덮어썼다.
- **원인**: `src-tauri/src/domain/layout/service.rs` — `set_dirty`·`pin_tab` 이 `tab.preview` 를 건드리지 않고,
  `open_tab` 의 preview 교체가 `existing.preview` 만 봤다. `docs/features/tabs.md` §3 이 요구하는 "편집 시작·pin 시
  일반 탭 승격" 이 구현 자체가 없었다.
- **수정**: `set_dirty(dirty=true)` 와 `pin_tab(pinned=true)` 가 `tab.preview = false` 로 **단방향 승격**한다(저장·
  고정 해제는 preview 로 되돌리지 않는다). 방어심층으로 `open_tab` 의 교체 대상도
  `existing.preview && !existing.dirty && !existing.pinned` 로 좁혔다. 계약 §0 대로 "교체로 밀려나는 clean preview 를
  `closed_tabs` 에 적재" 는 채택하지 않았다(단일 클릭마다 ⌘⇧T 스택이 미리보기로 도배된다).
- **테스트**: `domain::layout` 4건(dirty preview 공존 / `set_dirty` 후 preview false / pinned preview 유지 + 새 탭 추가 /
  pin 후 preview false). 검출력 실측 — 세 수정을 되돌리면 4건 전부 FAILED.

### #10 — 핀 탭 드래그가 raw 탭 순서를 깬다

- **증상**: 고정 탭이 있는 pane 에서 탭을 드래그하면 화면 순서(핀 좌측 정렬)와 배열 순서가 어긋난 채 영속돼,
  ⌃Tab 순환과 "오른쪽 탭 닫기" 가 화면과 다른 탭을 잡았다.
- **원인**: 같은 파일 `move_tab` — 핀 구역 불변식이 `pin_tab` 의 정렬에만 있었고 이동 경로에는 없었다.
- **수정**: 순수 헬퍼 `clamp_to_pinned_zone(leaf, pinned, index)` 를 추가해 `extract_tab` **뒤**·`insert_tab` **앞**에서
  대상 leaf 의 `pinned_count` 로 index 를 보정한다(이동 탭이 pinned 면 `min`, 아니면 `max`). 추출 후에 세므로 같은
  pane 안의 핀 탭 이동이 자기 자신을 세지 않고, `APPEND_AT_END` 는 `insert_tab` 기존 클램프와 그대로 맞물린다.
  프런트 드롭 핸들러 3곳이 아니라 mutation 한 곳에 둬 드래그 3경로·⌘K ⌘⇧←/→·향후 호출부가 구조적으로 덮인다.
- **테스트**: 3건(조사 §2 #10 의 (a)(b)(c)). 검출력 실측 — 호출을 빼면 3건 전부 FAILED.

### #16·#17 — 세션 id 가 없는 터미널 탭이 kind 동등으로 dedupe 된다

- **증상**: 터미널을 연속 두 번 열면 하나만 열리고, spawn 에 실패한 터미널 탭이 남아 있으면 그 pane 에서 새 터미널이
  영구히 열리지 않았다.
- **원인**: `open_tab` 의 kind 동등 dedupe 가 `Terminal { session_id: "" }` 둘을 같은 탭으로 봤다(세션 id 는 spawn
  왕복이 끝나야 박힌다).
- **수정**: `is_dedupable(kind)` 가드로 **빈 `session_id` 인 터미널만** dedupe 대상에서 제외했다. 다른 kind 의 dedupe·
  preview 치환·`next_untitled_index` 는 무영향이고, 세션 id 가 채워진 터미널은 여전히 dedupe 된다.
- **테스트**: 3건(빈 세션 터미널 2개 공존 / 세션 id 있는 터미널은 여전히 dedupe / 파일 탭 kind dedupe 보존).
  검출력 실측 — 가드를 빼면 첫 건 FAILED.

### #20 — 셸 슬롯 포커스가 이웃이 아니라 첫 슬롯으로 점프

- **증상**: 3분할 이상에서 포커스된 슬롯을 닫으면 포커스가 옆 슬롯이 아니라 항상 맨 첫 슬롯으로 갔다. 슬롯 포커스가
  `active_project` 를 재유도하므로 상태바·타이틀바·⌘P 대상과 네이티브 File 메뉴까지 함께 끌려갔다.
- **원인**: `src-tauri/src/domain/project/service.rs` `close_shell_slot` 이 `reconcile_focus` 의 ③ 폴백
  (`first_slot`)에만 의존했다. 실사용 빈도가 더 높은 `close_project` 도 같은 폴백을 탔다.
- **수정**: `project/shell_slots.rs` 에 d-65 `successor_leaf_after_prune` 와 동형인 순수 헬퍼
  `successor_slot_after_prune(tree, slot_id)` 를 추가하고, `reconcile_focus` 가 후보를 **선택적으로** 받아 ①분기로
  흡수하게 했다. `close_shell_slot`·`close_project` 둘 다 prune **전에** 후보를 계산해 넘긴다(포커스 슬롯이 이번
  제거에 포함될 때만 — private 헬퍼 `focus_successor` 로 공유). 비포커스 슬롯·프로젝트를 닫으면 포커스는 불변이다.
  `close_project` 는 `normalize_shell_slots` 를 거쳐야 retain/fallback-leaf 를 함께 타므로 후보를 그 함수로 관통시켰다
  (`normalize_shell_slots(session, successor)` — 무관한 호출부 3곳에는 `None` 인자만 추가, 동작 불변).
- **테스트**: 헬퍼 4건 + 경로 4건(`close_shell_slot` 승계 / `close_project` 승계 / 첫 슬롯 스펙 고정 / 비포커스 슬롯
  포커스 불변). 검출력 실측 — successor 분기를 빼면 승계 2건 FAILED. `close_project` 테스트는 `session.projects`
  순서를 슬롯 배치와 다르게 둔다(같으면 옛 폴백이 우연히 정답을 골라 검출력이 사라진다 — 실측 확인).

## 2. TypeScript — 보조 창 정합

### 공통 — `currentWindowActiveFilePath(layout)`

`src/shared/lib/pane-tree.ts` 에 `currentWindowFocusedPane` 과 대칭인 헬퍼를 추가했다
(`activeFilePathOf(resolveWindowPaneTree(layout, getWindowContext()))`). `layout` 미로딩은 내부에서 널가드하고,
슬롯이 사라진 보조 창은 **main 트리로 폴백하지 않고 `null`** 이다. 이후 위젯이 보조 창에 새로 마운트될 때 같은
실수를 구조적으로 막는다.

### #5·#6 — 보조 창 팔레트·아웃라인이 메인 창 파일을 본다

- **증상**: 보조 창에서 ⌘P `@`(심볼)·`:`(라인) 모드와 아웃라인 패널이 그 창이 아니라 메인 창 활성 파일을 대상으로 했다.
- **수정**: `command-palette.tsx`·`outline-panel-container.tsx` 의 `activeFilePathOf(layout)` → 위 헬퍼.
  아웃라인은 기존 `if (!activePath) return`·`hasActiveFile` 경로가 null 을 그대로 흡수해 추가 가드가 필요 없었고,
  그 동작을 테스트로 고정했다(슬롯 사라진 보조 창 → `outline.noActiveFile`).
- **테스트**: 아웃라인 3건(신규 파일) + 팔레트 2건. 검출력 실측 — 되돌리면 각각 2건·1건 FAILED.

### #4 — 보조 창 터미널이 cwd 를 잃는다

- **증상**: 보조 창 탐색기의 "Open in Terminal" 이 지정한 폴더가 아니라 프로젝트 루트에서 스폰됐다(무음).
- **수정**: `terminal-session.tsx` 의 `findPaneTab(layout.root, tabId)` → `resolveWindowPaneTree(layout,
  getWindowContext())?.root` 에서 조회. 왜 자기 창 트리여야 하는지 JSDoc 에 적었다.
- **테스트**: 2건(보조 창 컨텍스트에서 `ptyDefaultOptions` 인자가 그 cwd / 메인 창은 루트). 검출력 실측 — 되돌리면
  보조 창 케이스가 cwd `null` 로 호출돼 FAILED.

### #2 — 보조 창 더티 파일에 "저장됨" 거짓 응답

- **증상**: 보조 창에만 있는 dirty 파일을 Claude Code 가 `save_document` 로 저장 요청하면 아무것도 저장하지 않고
  성공을 돌려줬다.
- **수정**: `ide-sync-provider.tsx` 의 로컬 재귀 `findFileTabByPath` 를 제거하고 `collectAllPaneTabs(layout)` 로 교체.
  **거짓 성공을 없애는 것까지**가 이번 범위다 — 보조 창에는 모델 레지스트리가 없으므로 실제 요청은 `!model` 분기를 타
  `saved:false` 로 정직하게 실패한다. 실저장(요청 릴레이)은 후속(§6).
- **테스트**: 2건(보조 창 dirty → `saved:false` / 어느 창에도 dirty 탭이 없으면 종전대로 `true`).

### #14·#15 — 보조 창에서 ⌘= / ⌘− 가 동작하지 않는다

- **증상**: 두 액션의 유일한 등록처가 메인 창 전용 상태바라 보조 창에서 폰트 크기 변경이 무반응이었다.
- **수정**: 증감·리셋 로직을 훅 하나(`src/entities/settings/use-editor-font-size.ts`)로 추출하고, 키맵 등록은 두 창
  분기에 공통으로 마운트되는 `KeybindingsRuntimeProvider` **한 곳**으로 옮겼다. 상태바 버튼은 같은 훅을 재사용하고
  자기 `useGlobalKeymap` 두 엔트리를 제거했다(메인 창 이중 등록 방지).
- **이탈**: 훅 위치가 계약의 `src/shared/hooks/` 가 아니다 — 훅이 `@entities/settings` 를 쓰는데 eslint 레이어 규칙이
  `shared → @entities` 를 금지한다. 소비자가 app·widgets 둘이라 공통 하위 레이어인 entities 가 맞고, 선례는
  `src/entities/search/use-search-run.ts` 다.
- **테스트**: provider 3건(메인·보조 창 ⌘= 가 `editorFontSize` 를 정확히 1회 갱신 / 보조 창 ⌘−) + 상태바 2건
  (⌘= 로는 호출 0 = 이중 등록 없음 / 버튼 클릭은 여전히 동작).

## 3. TypeScript — 탐색기

### #13 — 삭제된 폴더가 "새 파일"로 되살아난다

- **증상**: 디렉터리를 삭제해도 선택이 객체로 남아, 다음 "새 파일" 이 삭제된 폴더를 대상으로 잡아 그 폴더를 되살렸다.
- **수정**: 선택의 단일 진실을 **id 로 축소**했다 — `file-tree.tsx` 의 `onSelectionChange` 페이로드가
  `FileTreeRow` → `string | null` 이 되고(중복이던 `onClearSelection` 핸들러는 제거), `explorer-container.tsx` 는
  `selectedId` state 에서 `rows.find(...)` 로 행을 **파생**한다. 행이 사라지면 자동으로 `null` 이라 "존재하지 않는 행을
  가리키는 stale 객체" 가 표현 불가능해진다(보정 effect 불필요). 더해 `explorer-path.ts` 의 순수 함수
  `resolveTargetDir(row, rows, projectRoot)` 가 rows 멤버십을 확인하고 루트로 폴백하며, `use-explorer-entry-crud.ts`
  의 `startDraft` 는 "rows 에도 없고 루트도 아닌 대상" 이면 초안을 만들지 않는다.
- **이탈**: 계약의 "`buildDisplayRows` 가 targetRow 를 못 찾으면 draft 불성립" 을 **렌더가 아니라 형성 시점**에 적용했다.
  트리 rows 는 루트의 *자식*부터라 정상 경로인 루트 draft 도 targetRow 가 없고(문자 그대로면 "새 파일(루트)" 가 깨진다),
  렌더에서만 빼면 `draft` state 가 남아 입력 행도 취소 수단도 없는 소프트락이 된다.
- **테스트**: `resolveTargetDir` 4건 + `startDraft` 5건(삭제돼 사라진 디렉터리 선택 → 루트에 초안 / 트리에도 루트에도
  없는 대상 → 초안 없음 / 루트는 행이 없어도 정상 대상 / 접힌 디렉터리는 펼친 뒤 초안). 검출력 실측 — 멤버십 검사와
  가드를 함께 되돌리면 4건 FAILED.

### #19 — "Open to the Side" 가 분할 없이 끝난다

- **증상**: 포커스 그룹이 그 파일 하나만 갖게 되는 상황(에디터가 비었거나 그 탭 하나뿐)에서 파일만 열리고 화면이
  분할되지 않았다(`open_tab` → `split` 2단계의 빈 leaf 를 `normalize` 가 접었다).
- **수정**: 순수 planner `open-to-the-side-plan.ts` 의 `planOpenToTheSide(projectId, row, layout)` 가 **현재 창의**
  focusedPane 으로 `OpenTabInSplitRequest`(edge `right`, `preview: false`)를 만들고, 컨테이너는
  `useOpenTabInSplit` **단일 mutation** 으로 연다. 죽은 `findLeafPane` 헬퍼와 `useSplitPane` 배선을 함께 제거했다.
- **의도된 동작 변경**: `open_tab_in_split` 은 kind dedupe 를 하지 않으므로, 이미 열린 파일에 "Open to the Side" 를
  하면 **같은 파일의 새 탭이 하나 더 생기며 항상 분할된다**. "옆에 열기" 의 의미상 더 옳다고 계약 §0 에서 정했다
  (`docs/features/tabs.md` §4.5).
- **테스트**: planner 6건(탭 0개 그룹 / 같은 파일 1탭 / 디렉터리 행·레이아웃 미로딩 → null / 보조 창은 그 창 pane /
  보조 창 슬롯 없음 → null).

### #18 — 보조 창 autoReveal 이 메인 창을 따라간다

- **증상**: 보조 창 트리가 그 창 탭이 아니라 메인 창 활성 파일을 따라갔고, 메인 창 사이드바를 접으면 보조 창은
  펼쳐져 있는데도 autoReveal 이 통째로 멈췄다.
- **수정**: `use-explorer-auto-reveal.ts` 의 `activePath` 를 자기 창 트리로 바꾸고(TS-A 헬퍼에 의존하지 않고 인라인),
  `sidebarVisible` 판정을 `layout.shellView.sidebarCollapsed` 대신 새 입력 `sidebarCollapsed` prop 으로 옮겼다.
  `project-shell.tsx`·`auxiliary-window-shell.tsx` 두 셸이 `Group.onLayoutChanged` 에서 `isCollapsed()` 로 갱신하는
  state 를 `ExplorerContainer` 에 내려 준다 — 이 콜백은 드래그뿐 아니라 명령형 `.collapse()`/`.expand()` 와 초기
  마운트에서도 발화하므로(라이브러리 계약) 토글·Zen·브리지 확장까지 한 지점으로 덮인다. 낡은 JSDoc("사이드바는
  주창에만 마운트된다")과 `docs/features/explorer-sidebar.md` §2.2 의 같은 서술을 함께 정정했다.
- **테스트**: 2건 추가(보조 창은 그 창 트리의 활성 파일 / 메인 창이 접어 둔 사이드바가 보조 창을 막지 않는다) +
  기존 게이트 테스트를 새 prop 으로 정정. 검출력 실측 — 두 줄을 되돌리면 3건 FAILED.

### #12 — rescan 후 파일 트리가 영구히 낡는다

- **증상**: 워처 오버플로(`fs:rescan-required`) 뒤 이미 펼쳐 둔 디렉터리의 트리 목록이 다음 `fs:changed` 나 명시적
  새로고침 전까지 오버플로 이전 상태로 남았다.
- **원인**: `ipc-sync-provider.tsx` 가 `TREE.ROWS` 를 invalidate 했지만 `tree_rows` 는 Rust 트리 스토어를 재직렬화할 뿐
  캐시된 디렉터리를 디스크에서 다시 읽지 않는다.
- **수정**: `rescanInvalidations` 에서 `TREE.ROWS` 를 빼고(나머지 3종 유지), 순수 헬퍼
  `rescanTreeRefreshDirs(page, projectRoot)`(= 프로젝트 루트 + 캐시된 rows 중 `expanded` 디렉터리)가 돌려준 dirs 로
  기존 `syncTreeRowsForChangedDirs`(디렉터리별 `tree_refresh` = 실제 디스크 재조회)를 돌린다. dirs 가 비면(첫 로드 전·
  루트 미상) 종전 invalidate 로 폴백한다. 새 Rust 커맨드는 만들지 않았다.
- **결정 대체**: 이 항목은 d-57 F4 가 "기록만" 으로 닫았던 기지 한계다. 이번 포괄 지시를 그 결정을 대체하는 새 지시로
  보고 수정했다(`docs/acknowledge/2026-09-06-d57-infra-hardening-wave1-contract.md` F4 행 각주).
- **테스트**: `rescanInvalidations` 기대값 정정 + "TREE.ROWS 는 목록에 없다" 1건, `rescanTreeRefreshDirs` 4건.
  검출력 실측 — 파일을 HEAD 버전으로 되돌리면 8건 FAILED.

## 4. TypeScript — 편집 지속성

### #3 — 미러·wait 마커를 "다른 pane 에 아직 열림" 판정 전에 해제

- **증상**: 같은 파일이 두 pane 에 열린 상태에서 한쪽 탭만 닫으면 살아 있는 pane 의 hot-exit 미러가 지워지고
  `taide --wait` 마커도 풀렸다(미저장 편집의 안전망 소실 + CLI 조기 반환).
- **수정**: `entities/layout/tab-path-change.ts` `releaseClosedFileTabPath` 에서 `stillOpenElsewhere` 판정을 함수 맨
  앞으로 올리고, `takeWaitMarkers` 루프·`clearMirror`·`invalidateQueries(FILE.MIRRORS)` 세 줄을 전부 그 가드 아래로
  옮겼다(마커 take 는 파괴적이라 호출 자체를 막아야 한다).
- **테스트**: 버그 동작을 기대값으로 고정하던 기존 단언(`clearedMirrors: ['/repo/split.ts']`)을 `[]` 로 정정하고,
  `takeWaitMarkers` 호출 자체를 관측하는 recorder 로 마커 케이스 2건(메인 pane / 보조 창)을 추가했다. 검출력 실측 —
  순서를 되돌리면 3건 FAILED.

### #7·#8 — 두 번째 pane 이 공유 모델을 덮고, "복구됨" 배너가 오탐한다

- **증상**: 미저장 편집이 있는 파일을 두 번째 pane 에 열면 공유 monaco 모델이 디스크 내용으로 덮여 편집이 사라졌다
  (dirty 표시는 남아 ⌘S 가 디스크 내용을 다시 썼다). 미러가 있으면 대신 "미저장 내용을 복구했습니다" 배너가 잘못 뜨고
  버퍼가 마지막 미러 시점으로 롤백됐다.
- **수정**: 두 결함이 같은 뿌리("이미 라이브인 모델 위에 마운트")라 한 메커니즘으로 닫았다.
  `code-editor.tsx` 가 `getOrCreateModel` **직전에** `getModel(path)` 로 선확인해 `onModelAttach({ path, hadLiveModel })`
  로 보고하고(경로가 바뀌는 attach 에서만 — 확장자 변경 후의 re-language 재attach 는 보고하지 않는다),
  `editor-draft-sync.ts` 의 순수 판정 `shouldAdoptLiveModelEdit({ hadLiveModelOnAttach, modelContent, diskContent })` 이
  마운트 reconciliation 에서 **미러 복원보다 먼저** 돈다. 인수되면 draft 를 모델의 lazy reader 로 잡고 dirty 만 세우며
  (`restoreNotice` 는 `'none'`), 미러 복원을 건너뛴다. `setDirty` 가 `dirtyRef` 를 동기로 옮기므로 같은 커밋의 디스크
  sync 가 라이브 dirty 가드에서 bail 해 #7 도 함께 닫힌다.
- **이탈**: 계약이 적은 "`syncModelFromDisk` 의 `hasUnobservedModelEdit` 를 `모델값 !== syncedContent` 로 일반화" 는
  **채택하지 않았다** — 워처 재로드(외부 변경 → `FILE.CONTENT` 무효화 → `setSyncedContent(새 내용)`) 시점에는 모델이
  아직 옛 디스크 내용이라 그 조건이 참이 되어 정상 재로드가 "미관측 편집 인수" 로 오판되고 외부 변경이 조용히 사라진다.
  같은 비교를 쓰되 `hadLiveModelOnAttach`(attach 시점 1회 신호)를 AND 로 묶었고, 이 회귀 방지는
  `hadLiveModelOnAttach: false` 케이스로 잠갔다.
- **알려진 결과 변화**: 개명(rename) 직후 dirty 파일은 `retargetModel` 이 새 경로에 모델을 먼저 등록하므로 인수 경로를
  타 **"복구됨" 배너가 더는 뜨지 않는다**. 미러 내용과 모델 값이 같아 본문·dirty 결과는 동일하고, 복구가 아니라 개명이라
  오히려 정확하다(`docs/features/editor.md` §3).
- **테스트**: 훅 마운트 5건(형제가 편집 중인 모델 → 인수·배너 없음 / 미러 없어도 인수 / 크래시 후 첫 마운트는 종전대로
  미러 복원 + 배너 / 아무도 편집 안 한 파일의 두 번째 pane 은 clean 유지 / 라이브지만 디스크와 같으면 미러 복원 정상),
  판정 함수 5건, `onModelAttach` 4건. 검출력 실측 — reconciliation 을 되돌리면 인수 2건만 FAILED(대조군 3건은 통과).

### info — untitled 탭에서 ⌘S 가 무반응

- **증상**: untitled 탭에서 ⌘S 를 눌러도 아무 일도 일어나지 않았다(d-65 가 "별건" 으로 미룬 기지 결함).
- **수정**: `untitled-pane.tsx` 의 `<CodeEditor>` 에 `registryTabId={tabId}` 를 넘겨 인스턴스를 등록하고,
  `focused-editor-tab.ts` 의 `SAVE_ROUTABLE_TAB_KINDS` 에 `'untitled'` 를 더했다. ⌘S 는 이제
  `resolveSaveRoutableTabId` → `getEditorInstance(tabId)` → `taide.saveFile` → `onSave` 로 닿아 **save-as 다이얼로그**를
  연다(파일 저장 경로가 아니다). 낡은 JSDoc(미수정 결함 기록)도 정정했다.
- **동반 효과**: 같은 상수가 `runMonacoAction`·`runSelectedTextInTerminal`·상태바 커서·active-action-ids 도 태우므로
  untitled 탭에서 그것들이 함께 동작한다 — `appFile` 과 같은 정상화라 회귀로 보지 않았다.
- **테스트**: 버그를 기대값으로 고정하던 단언("untitled 탭이 포커스면 null")을 뒤집고, 자체 위젯을 그리는 kind(diff)가
  계속 no-op 인지 보는 케이스를 추가했다.

## 5. TypeScript — reveal 재설계

### #9 — reveal 이 다른 pane 에서 먼저 소비된다

- **증상**: 검색 결과·진단·심볼·정의로 이동이 분할 화면에서 목적 줄로 가지 않았다. 같은 파일이 다른 pane 에 열려 있으면
  그 pane 이 점프하며 포커스까지 가져가고, 정작 새로 열린 탭은 1행에 머물렀다.
- **원인**: `entities/editor/reveal-registry.ts` 의 `requestReveal(path, …)` 가 `monaco.editor.getEditors()` 전역 스캔으로
  **경로에 맞는 아무 에디터**에나 즉시 적용하고 pending 을 큐잉하지 않았다.
- **수정**: reveal 대상을 "경로" 가 아니라 **"이번에 연 탭"** 으로 재설계했다.
  - registry API 교체: `requestReveal` 제거 → `revealInTab(tabId, { line, column }, ttlMs?)` +
    `consumePendingReveal(tabId, editor)`. pending 은 `Map<TabId, …>` 이고 `REVEAL_PENDING_TTL_MS`(5s) 만료 정리는
    유지하되 의미가 "그 경로가 열릴 때까지" → "그 탭이 마운트될 때까지" 로 바뀌었다. 이미 마운트된 탭은
    `getEditorInstance(tabId)` 로 즉시 적용(dedupe 경로), 아니면 `EditorPane` 이 마운트 시 소비(신규 탭·배경 탭 경로).
  - `useOpenFileTab` 요청이 `reveal?: { line, column }` 을 받고, `onSuccess` 에서 `openedFileTabIdOf` 가 대상 pane 의
    active 탭을 확인해 **kind 가 file 이고 path 가 일치할 때만** 큐잉한다(불일치면 reveal 을 버린다 — 엉뚱한 탭에서
    커서가 움직이는 것보다 낫다).
  - 호출부 9곳 통일: 열기+reveal 6곳은 `openFileTab({ …, reveal })` 로, 현재 활성 에디터 reveal 3곳
    (브레드크럼 심볼·팔레트 심볼/라인·아웃라인)은 `revealInTab(tabId, …)` 로. `setTimeout` 지연 우회는 쓰지 않았다.
  - 적용 지점은 registry 구독이 아니라 `EditorPane` 의 consume effect 다 — `CodeEditor` 의 인스턴스 등록은 자식 effect 라
    부모의 viewState 복원보다 한 커밋 빠르고, 구독으로 밀면 배경 탭 활성화 경로에서 복원이 reveal 을 덮어쓴다.
- **테스트**: registry 9건(같은 파일을 연 다른 탭의 에디터는 건드리지 않는다 / 보류는 그 탭에만 붙는다 / TTL 만료·
  만료 전 / 1회 소비 / 이미 마운트된 탭 즉시 적용), `useOpenFileTab` 배선 4건. 검출력 실측 — pending 키를 공유 키로,
  `openedFileTabIdOf` 를 경로 탐색으로 되돌리면 각각 1건·2건 FAILED.

## 6. 검증

- Rust: `cargo test -p taide --lib` **1685 passed / 0 failed**(`domain::layout` 108, `domain::project` 109 — 신규 18건
  포함), `cargo fmt --all -- --check`·`cargo clippy --workspace --all-targets -- -D warnings` 전부 exit 0(`#[allow]` 미사용).
- TS: `bun test` **2819 pass / 0 fail**(282 파일, 6259 expect), `bun run typecheck`·`bun run lint`(error 0 / 기존
  warning 11건 유지)·`bun run format:check` 전부 exit 0.
- 수정 항목마다 **검출력 실측**(수정만 임시로 되돌려 대상 테스트가 실패하는지)을 수행하고 원상 복구를 확인했다.
  결과는 각 항목의 "테스트" 줄에 적었다.
- **테스트로 잠기지 않는 것**: 앱 실기(`tauri dev`/build)는 하지 않았다. 실제 WKWebView 의 키 이벤트, pty 스폰,
  패널 접힘 측정(happy-dom 에는 레이아웃이 없다), 드래그 미리보기는 아래 실기 목록이 정본이다.

## 7. 실기 대상 (사용자)

| # | 조작 |
|---|------|
| #1 | 탐색기 단일 클릭으로 preview 탭을 열고 한 글자 입력 → 다른 파일 단일 클릭 → 두 탭이 공존하고 첫 탭 제목의 이탤릭이 풀려 있다 |
| #11 | preview 탭을 고정 → 다른 파일 단일 클릭 → 고정 탭이 유지되고 새 탭이 따로 열린다 |
| #10 | 탭 A 를 고정한 pane 에서 A 를 맨 뒤로 드래그 → 핀 구역 안에 머물고, ⌃Tab 순환과 "오른쪽 탭 닫기" 가 화면 순서와 일치한다 |
| #16·#17 | 터미널을 연속 두 번 열기 → 탭이 두 개 생긴다 (⌃⇧` 새 터미널과 터미널 토글을 구분해 확인) |
| #20 | 프로젝트 3개를 슬롯으로 분할 → 포커스 슬롯(세 번째)을 헤더 ✕ 로 닫기 → 포커스가 첫 슬롯이 아니라 이웃으로 간다. 같은 슬롯을 프로젝트 닫기로 닫아도 동일 |
| #5·#6 | 보조 창에서 ⌘P `@`·`:` 와 아웃라인 패널이 그 창의 파일을 대상으로 동작한다 |
| #4 | 보조 창 탐색기에서 하위 폴더 우클릭 → Open in Terminal → `pwd` 가 그 폴더다 |
| #2 | 보조 창에만 있는 dirty 파일을 Claude Code 로 저장 요청 → "저장됨" 으로 거짓 성공하지 않는다(실패 표시는 미확인) |
| #14·#15 | 보조 창에서 ⌘= / ⌘− 로 에디터 폰트 크기가 바뀌고, 메인 창에서 한 번 누르면 한 단계만 바뀐다 |
| #13 | 폴더를 선택한 채 삭제 → 곧바로 "새 파일" → 초안이 삭제된 폴더가 아니라 프로젝트 루트에 열린다 |
| #19 | 모든 탭을 닫은 상태에서 파일 우클릭 → Open to the Side → 화면이 실제로 분할된다(이미 열린 파일이면 탭이 하나 더 생긴다) |
| #18 | 보조 창에서 탭을 바꾸면 보조 창 트리가 그 파일을 따라가고, 메인 창 사이드바를 ⌘B 로 접어도 보조 창 autoReveal 이 계속 돈다 |
| #12 | 대형 checkout·`npm install` 로 워처 오버플로를 낸 뒤 이미 펼쳐 둔 디렉터리 목록이 자동으로 최신화된다 |
| #3 | 같은 파일을 두 pane 에 열고 편집 → 1초 대기 → 한쪽만 ⌘W → ⌘Q 후 재실행 시 편집이 복구된다 |
| #7·#8 | 루트 밖 파일을 열어 편집 → 분할 후 같은 파일을 두 번째 pane 에 열기 → 편집이 유지되고 "복구됨" 배너가 뜨지 않는다 |
| info | untitled 탭에서 ⌘S → save-as 다이얼로그가 뜬다 |
| #9 | 좌/우 분할에서 왼쪽에 파일을 띄운 채 오른쪽을 포커스 → 검색 결과 클릭 → **오른쪽에 열린 새 탭**이 그 줄로 이동하고 왼쪽은 그대로다 |

## 8. 이번 범위 밖 · 후속

- **보조 창 파일의 IDE 실저장**(#2 의 남은 절반): 모델 레지스트리가 창별 싱글턴이라 요청 릴레이 설계가 필요하다 —
  `docs/backlog.md`.
- **`openedFileTabIdOf` 의 `target: null` 재계산**(#9 잔여, 렌즈 minor): `withCurrentWindowTarget` 이 뮤테이션 시점
  캐시로 확정한 pane 과 `onSuccess` 가 신선한 레이아웃에서 다시 읽는 pane 이 그 사이 다른 IPC 로 갈릴 수 있다. 그때
  그 pane 의 active 탭이 우연히 같은 경로면 kind/path 검사를 통과해 좁은 레이스로 #9 증상이 재현된다. 근본 해법은
  mutationFn 이 실제로 보낸 paneId 를 `onSuccess` 까지 관통시키는 것 — `docs/backlog.md`.
- **프런트 드래그 미리보기의 자체 pinned 클램프**(#10 잔여): `editor-area.tsx` 는 추출 **전** 카운트로 클램프해
  한 칸 어긋날 수 있다. Rust 가 최종 위치를 보정하므로 결과는 옳고, 미리보기만 다르게 보일 수 있다.
- **`onFocusCapture` 가 프로그램적 focus 와 사용자 클릭을 구분하지 못함**(d-65 F2): reveal 대상이 항상 "연 탭" 이 되어
  실질 영향은 사라졌다고 본다. 구조적 구분은 여전히 없다.
- 조사 문서 §3 의 기각 3건(`stale-ids-4` L1-07 / `open-paths-3` d-58 B-3 / `fileops-tabs-3` d-50 #2)은 기존 결정
  유지 — 재론 금지.
