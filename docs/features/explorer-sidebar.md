# 기능 — 탐색 사이드바 (파일 / 검색 / Git 전환)

> FR-C. content 좌측 사이드바 — Cursor/VSCode 구성. Git 뷰 내용은 `git.md`,
> 성능 전략 근거: `docs/research/performance-memory.md` §4, `docs/research/react-frontend-stack.md` §5.

## 1. 구성

- 상단 아이콘 전환: **파일(Explorer) / 검색(Search) / Git(SCM)** — Cursor 스타일(FR-C2).
  단축키 `⇧⌘E` / `⇧⌘F` / `⌃⇧G` (VSCode 동일). 아이콘에 배지(검색 결과 수, git 변경 수).
- 사이드바 토글 `⌘B`. 폭 리사이즈(react-resizable-panels `Panel collapsible`) — 프로젝트별 저장.

## 2. 파일 트리 (FR-C3·C4)

### 2.1 데이터 구조 — 확정 (사용자 결정, 2026-08-06)

**Rust 소유 트리 + flat rows + @tanstack/react-virtual.** 트리 상태(펼침 포함)를 Rust 가 소유하고
`tree_rows(offset, limit)` 로 보이는 행만 페이지네이션, view 는 가상 스크롤 렌더만 담당한다.
ADR-0004 와 완전 일치(view reload 복원 공짜), 수만 파일 규모 최적, 외부 트리 라이브러리 불필요.
rename 인라인 편집·트리 DND 는 직접 구현(비용 감수 — 결정 근거).

기각: @headless-tree(트리 상태가 JS 로 이동 — Rust 소유 원칙 예외 증가),
react-arborist(redux5+react-dnd14+react-window 동반 + dnd-kit 과 DnD 이중화).

### 2.2 동작

- **lazy load**: 폴더 펼침 시 그 한 단계만 `read_dir`(재귀 프리로드 금지). 펼침 상태는 Rust 트리
  (`TreeStore` = `RwLock<HashMap<ProjectId, TreeState>>`)가 소유하지만 **프로세스 메모리에만 남는다**
  — 앱 실행 중에는 프로젝트를 오가도 유지되고, 종료하면 사라진다. `layout` 영속화에는 펼침 상태가
  들어가지 않는다(`domain/layout` 에 `expanded` 필드 0). `tree::service::expanded_paths`·
  `restore_expanded` 는 존재하지만 **호출부가 테스트뿐**이라 재시작 복원은 아직 구현되지 않았다
  (backlog — 이 문서는 2026-09-04 이전까지 "재시작 복원"으로 잘못 적고 있었다).
- 렌더: 고정 행높이 22px, `getItemKey` = 안정 노드 id, overscan 12. 행 = 들여쓰기 + 폴더/파일
  아이콘 + 이름 + git 상태색(`explorer.git*` 토큰 — M/A/D/U/ignored 흐림).
- **git 상태 데코레이션 (2026-08-29 구현)**: `gitStatusQueryOptions` 의 `StatusRow[]` 를
  `widgets/explorer/file-tree-git-status.ts` 가 절대경로→상태 맵으로 유도해 행에 주입. 파일은
  이름 색 + 우측 문자 뱃지(M/A/D/R/U/!), 디렉토리는 자식 최우선 상태의 색 + 점 뱃지(루트 제외
  조상 전파, VS Code 파리티). 우선순위 conflicted > added > untracked > renamed > modified >
  deleted (근거는 모듈 JSDoc). 갱신은 `git:status-changed` → `GIT.STATUS` 무효화로 자동.
  ignored 흐림은 미구현(별도 ignore 판정 IPC 필요 — backlog).
- 클릭 = preview 탭으로 열기, 더블클릭 = 고정 탭(FR-C4, `tabs.md` §3). **행이 없는 빈 공간 더블클릭 =
  프로젝트 루트에 새 파일 초안**(d-59 §1.E) — 선택을 먼저 지우고 루트를 명시 대상으로 넘긴다.
- **선택은 행 목록의 함수다**(d-66). 컨테이너는 선택된 **id** 하나만 들고 행은 `rows` 에서 파생하므로, 삭제·
  외부 변경으로 그 행이 사라지면 선택은 자동으로 없음이 된다. 그전에는 행 객체를 state 로 들어, 폴더를 지운 뒤
  "새 파일" 을 누르면 **삭제된 폴더가 대상으로 잡혀 그 폴더가 되살아났다**. 대상 디렉토리 해소(`resolveTargetDir`)도
  rows 멤버십을 확인해 없으면 프로젝트 루트로 폴백하고, 트리에도 루트에도 없는 대상에는 초안 자체를 열지 않는다.
  → `docs/bug/2026-09-17-editing-surface-audit-fixes.md` §3
- 키보드: ↑↓ 이동, ←→ 접기/펼치기, 타이핑 시 이름 점프(typeahead), 그 외 단축키는 §2.5.
- context menu: 새 파일/폴더, 이름 변경(인라인 입력), 삭제(휴지통 이동 + 확인), 복사/붙여넣기,
  경로 복사, Finder 에서 열기, (git 있으면) 하위 항목 — 전부 Rust fs 명령 경유.
- 파일 조작 후 watcher 이벤트로 트리 갱신(자기 쓰기 echo 는 origin 플래그로 낙관 갱신).
- 트리 내 DND(파일 이동)는 2차(dnd-kit 재사용).

#### 활성 파일 자동 표시 (autoReveal, 2026-09-04 구현)

VS Code `explorer.autoReveal` 파리티. 활성 에디터 탭이 파일이면 트리가 그 파일까지 **펼쳐 선택**한다.

- **게이트 3축** — 셋 다 만족할 때만 동작한다. ① 설정 `explorerAutoReveal`(기본 on) ② **이 창의**
  사이드바가 실제로 보임(호출부가 주입한 `sidebarCollapsed` 아님 + Zen 아님 — d-66, 아래 "창마다 따로 본다")
  ③ 사이드바 뷰가 `files`. 판정은
  `widgets/explorer/explorer-auto-reveal.ts` 의 순수 함수 `decideAutoReveal`(`skip`/`select-only`/
  `reveal-then-select`)이 전담하고, 훅은 입력 공급과 실행만 한다.
- **이미 보이는 행은 IPC 0** — `visiblePaths` 에 있으면 `select-only` 로 기존 `selectPathRequest`
  경로만 태운다. `tree_reveal` 은 성공 시 트리 페이지 전체를 재직렬화·교체하므로(`ipc-contract.md`
  "응답 형태 불변"), 스크롤만 필요한 경우까지 왕복시키지 않는다.
- **중복 억제** — 마지막으로 처리한 경로를 훅의 ref 에 기록해 같은 파일로는 다시 reveal 하지 않는다.
  `skip` 은 기록하지 않으므로, 사이드바를 접은 채 파일을 바꾼 뒤 다시 펼치면 그때 reveal 된다.
- **포커스 불탈취** — 선택은 `file-tree.tsx` 의 `selectByIndex`(`setSelectedId` + `scrollToIndex`)
  로만 이뤄지고 DOM focus 를 건드리지 않는다. 에디터 타이핑이 끊기지 않는다.
- **창마다 따로 본다**(d-66 — 종전 서술 "주 트리만 본다" 정정). 사이드바가 주창 전용이라는 전제는 d-62 §1.D
  이후 거짓이다: `auxiliary-window-shell.tsx` 가 같은 `ExplorerContainer` 를 보조 창에도 마운트한다. 활성 경로는
  `activeFilePathOf(resolveWindowPaneTree(layout, getWindowContext()))` 로 **그 창의 트리**에서 읽고, 게이트 ②의
  사이드바 접힘도 `layout.shellView.sidebarCollapsed`(주창 전용 값)가 아니라 두 셸(`project-shell.tsx`·
  `auxiliary-window-shell.tsx`)이 패널 `onLayoutChanged` 에서 갱신해 내려 주는 **창별 state** 를 쓴다. 그전에는
  보조 창 트리가 주창 활성 파일을 따라가고, 주창 사이드바를 ⌘B 로 접으면 보조 창 autoReveal 까지 멈췄다.
  두 창이 각자 `lastRevealedRef` 를 갖고 같은 경로로 `tree_reveal` 을 중복 발행하던 것도 함께 사라진다.
  → `docs/bug/2026-09-17-editing-surface-audit-fixes.md` §3
- **프로젝트 루트 밖 경로는 skip** — `tree_reveal` 이 걸어 올라갈 조상이 없으므로 훅에서 선판정한다.
- **실패는 조용히 무시** — 사용자가 요청한 동작이 아니므로 토스트를 띄우지 않는다(명시적 "탐색기에서
  보기"는 기존대로 에러를 알린다).
- **사이드바를 자동으로 펼치지 않는다** — 접힌 사이드바를 펼치는 것은 명시적 reveal(탭 우클릭
  "탐색기에서 보기" → `explorer-reveal-bridge`)뿐이다. `app-shell.tsx` 가 그 브리지를 구독해
  `explorerPanelRef.expand()` 를 부르며(2026-09-04 동반 수정 — 그전엔 접힌 상태에서 무반응이었다),
  autoReveal 은 브리지를 타지 않으므로 이 구분이 자동으로 성립한다. Zen 모드에서는 이 `expand()` 가
  Zen 의 강제 접힘과 부딪히지 않는다 — 브리지의 유일한 발행처인 탭 우클릭 메뉴가 Zen 에서는 탭 바째
  렌더되지 않기 때문이다(`pane-node-view.tsx` 의 `{!zen && <PaneTabBar/>}`).
- 구성: `widgets/explorer/use-explorer-auto-reveal.ts`(훅) + `widgets/explorer/explorer-auto-reveal.ts`
  (판정 — 소비처가 이 위젯 하나뿐이라 슬라이스 안에 둔다. `paste-plan.ts`·`file-tree-git-status.ts`
  와 같은 자리이며, 다른 슬라이스가 쓰게 되면 그때 `shared` 로 승격한다) + `shared/lib/pane-tree.ts`
  `activeFilePathOf`(활성 파일 경로 유도). 사이드바 뷰 상태는 이 게이트를 위해 `ExplorerPanel` 에서
  `ExplorerContainer` 로 올라간 controlled state 다(`view`/`onViewChange`). Rust 신규 커맨드는
  없다(`tree_reveal` 재사용).

### 2.3 갱신·성능

- watcher(notify 8.2 + notify-debouncer-full 0.7, 300ms debounce)가 **변경 디렉토리의 자식만**
  재조회 — 루트 재스캔 금지.
- **트리는 무시 목록을 적용하지 않는다 — 전부 표시한다**(d-64 T1, 사용자 결정 2026-09-16 —
  2026-09-15 G5 의 "현행 유지" 번복). `node_modules/`·`.next/`·`.git/`·`target/` 도 다른 디렉토리와
  똑같이 행으로 뜨고 펼칠 수 있다(`domain::tree::service::read_children` 에 이름 필터가 없다).
  무시 목록(`constants::IGNORED_DIR_NAMES` — `.git`, `node_modules`, `target`, `dist`, `.next` 등)을
  쓰는 곳은 이제 **워처(`infra::watcher`)와 검색·퀵오픈(`domain::search`) 둘뿐**이고, 성능 근거는
  그대로다. 적용 방식도 둘이 다르다 — 워처는 경로의 **조상 성분에만** 적용한다(마지막 성분 제외 —
  d-50 S6, `ipc-contract.md`). 그래서 루트에 `build` 라는 이름의 **파일**이 있으면 트리에 보이고 그
  변경 이벤트도 도착한다.
- **그에 따른 절충 두 가지**:
  - 워처가 무시 디렉토리 **내부**를 감시하지 않으므로, `node_modules/` 를 펼쳐 둔 상태에서 그 안의
    파일이 바뀌어도 트리 행이 실시간으로 갱신되지 않을 수 있다(무시 디렉토리 **자신**의 생성·삭제
    이벤트 1건은 통과 — `ipc-contract.md` §4-A-4). 최신 목록이 필요하면 그 폴더를 새로고침한다
    (`tree_refresh`). VS Code 의 `files.watcherExclude` 기본값과 같은 절충이다.
  - 퀵오픈(`search_list_files`)·검색은 여전히 무시 디렉토리를 건너뛰므로, **트리가 보여주는 파일
    집합과 검색이 찾는 파일 집합이 더는 일치하지 않는다**(`ipc-contract.md` 의 `search_list_files`
    항목이 근거로 들던 등식은 트리 쪽이 넓어진 만큼 깨졌다 — 검색 쪽 동작은 미변경).
- Linux inotify watch 한도 초과는 조용히 실패 — 에러를 UI 배너로 노출(research 함정).
- 이벤트는 경로 배열 1건으로 묶어 emit(파일당 1 emit 금지).
- **워처가 이벤트를 흘리면 `fs:rescan-required(projectId)` 를 먼저 1회 보낸다**(d-57). FSEvents 큐
  오버플로(대형 checkout·`npm install`·슬립 웨이크)에서 `notify` 가 세우는 rescan 플래그를 그대로
  전달하는 신호로, 경로 목록이 없어 프론트는 퀵오픈 인덱스·git·열린 파일 캐시를 통째로
  무효화한다(한 감시당 최소 간격 2초 — `ipc-contract.md` §d-57). 그전에는 이 플래그가 통째로
  버려져 다음 변경이 오기 전까지 트리·인덱스가 틀린 채로 남았다.
  - **트리만은 무효화가 아니라 실제 재조회다**(d-66 — d-57 F4 의 "기록만" 결정을 대체). `tree_rows` 는 Rust 트리
    스토어를 재직렬화할 뿐 이미 캐시된 디렉토리를 디스크에서 다시 읽지 않으므로, `TREE.ROWS` 무효화로는 **이미
    펼쳐 둔 디렉토리**가 영영 교정되지 않았다(접었다 펴도 마찬가지 — `expand` 도 캐시가 있으면 디스크를 건너뛴다).
    이제 `TREE.ROWS` 는 무효화 목록에서 빠지고, 대신 **프로젝트 루트 + 캐시된 rows 중 펼쳐진 디렉토리** 전부를
    기존 `syncTreeRowsForChangedDirs`(디렉토리별 `tree_refresh`) 로 다시 읽는다 — 툴바 새로고침이 쓰는 경로와 같다.
    보여 줄 목록이 아직 없으면(첫 로드 전·루트 미상) 종전 무효화로 폴백한다. 신규 Rust 커맨드는 없고, 대신 rescan
    한 번의 IPC 가 `펼친 디렉토리 수 + 1` 회로 늘어난다(2초 스로틀 안).
- **폴더 삭제·개명은 그 하위의 펼침 상태·캐시까지 정리된다**(d-50 S7). `tree_refresh` 가 갱신한
  목록에 없는 자식 디렉토리는 경로 접두사로 하위 전체를 버리고, 남아 있지만 자식이 없어진 자식
  디렉토리는 캐시된 목록만 버린다(펼침 표시는 유지 — 빈 폴더도 펼칠 수 있다). 그전에는 지운 폴더의
  자식 목록과 펼침 표시가 그대로 남아, 같은 이름으로 폴더를 다시 만들면 그 폴더가 **펼쳐진 채 옛
  자식들로 채워져** 보였다(감사 §4-B C2).
- **트리의 `read_dir` 은 전역 mutation guard 를 잡기 전에 블로킹 풀에서 선수행**한다(d-50 S7 §2 H-4).
  잠금 안에서는 미리 읽은 목록을 꽂기만 하므로, 차가운 폴더 스캔이 파일·git 뮤테이션 전체를 뒤에
  줄 세우지 않는다. 미리 읽지 못한 디렉토리(읽기 실패·계획 이후 상태 변화)는 잠금 안에서 예전처럼
  그 자리에서 읽어 같은 결과·같은 오류를 낸다.

### 2.4 붙여넣기·인라인 편집 확정 (d-51 F4)

- **잘라내기 → 같은 폴더 붙여넣기는 no-op** 이다(클립보드만 비운다). 그전에는 자기 자신이 형제
  이름에 포함돼 "이름 복사본" 으로 제자리 개명됐다. 판정은 `widgets/explorer/paste-plan.ts` 의
  `isSamePlaceCutPaste`.
- **충돌 접미는 항목 종류를 안다** — 폴더 `v1.2` 는 `v1.2 복사본`(확장자 분해 없음), 파일 `a.ts` 는
  `a 복사본.ts`. `buildUniqueEntryName(…, kind)` 의 4번째 인자이며 기본값은 기존 동작(파일)이다.
- **접힌 폴더의 숨은 충돌은 백엔드가 잡고 프런트가 재시도한다.** 트리는 lazy 라 형제 이름 목록이
  화면에 보이는 행뿐이므로 로컬 유일 이름이 디스크에서 충돌할 수 있다. d-50 S2 가 세운 목적지 가드가
  `error.file.destinationExists` 로 거절하면 그 이름을 taken 에 접고 다음 후보로 재시도한다(최대
  `PASTE_DESTINATION_ATTEMPT_LIMIT` 8회, 그 외 오류는 재시도 없이 그대로 토스트).
- **인라인 생성·개명은 in-flight 가드**를 가진다(`use-explorer-entry-crud.ts` 의 ref 2개) — 입력 행이
  왕복 동안 계속 떠 있어 Enter 를 두 번 치면 같은 이름으로 두 번째 생성이 날아가 "이미 존재" 오류
  토스트가 뜨던 경로를 닫는다.
- **개명·삭제는 열린 탭까지 따라간다**(d-50 S8) — 개명은 `entities/file/file.query.ts` 의
  `useRenameEntry` 가, 삭제는 `useDeleteEntry` 가 `layout_apply_path_change` 를 이어 부른다. 위젯이
  아니라 이 mutation 에 붙였기 때문에 인라인 개명뿐 아니라 **잘라내기 → 붙여넣기(이동)** 도 같은
  추종을 받는다. 상세 규약은 `tabs.md` §7.1.

### 2.5 단축키 (d-59 §1.D)

파일 트리가 포커스를 가진 동안만 도는 **로컬 단축키**다. VS Code macOS 탐색기 기본키를 따른다.

| 키 | 동작 |
|----|------|
| `↩` / `F2` | 이름 바꾸기(인라인 편집 시작) |
| `⌘↓` | 고정 탭으로 열기 |
| `␣` | 미리보기 탭으로 열기 |
| `⌘⌫` | 삭제(휴지통 이동 — 기존 확인 다이얼로그 그대로) |
| `⌘X` / `⌘C` / `⌘V` | 잘라내기 / 복사 / 붙여넣기 |
| `⌘N` / `⇧⌘N` | 새 파일 / 새 폴더(선택 폴더 하위, 파일 선택 시 그 부모) |
| `⌥⌘R` | Finder 에서 표시 |
| `⌥⌘C` / `⌥⇧⌘C` | 경로 복사 / 상대 경로 복사 |

- **리바인딩 불가.** 이 키들은 `APP_KEYMAP` 엔트리가 아니라 `features/explorer/explorer-shortcuts.ts`
  의 정적 표를 `file-tree.tsx` 의 `onKeyDown` 이 직접 해석하므로 키바인딩 에디터에 나타나지 않는다.
  전역 키맵으로 올리는 것은 `explorerFocus` when 컨텍스트가 생긴 뒤의 후속 과제다(d-59 §4).
- **전역 키맵이 우선한다.** 창 capture 리스너가 먼저 돌고 매칭 시 `preventDefault`+`stopPropagation`
  하므로, `when` 없는 `APP_KEYMAP` 엔트리와 겹치는 키는 트리에 도달하지 못한다. `⌘↓` 만 겹치는데
  그 엔트리(`terminal-jump-to-next-command`)가 `terminalFocus` 로 묶여 있어 안전하다 —
  `explorer-shortcuts.test.ts` 가 이 불변식을 전 바인딩에 대해 검사한다.
- **디렉터리 `Enter` 는 더 이상 펼치기가 아니다**(이름 바꾸기로 바뀜, VS Code 동일). 펼치기/접기는
  `→`/`←` 와 클릭이 담당한다.
- 인라인 편집(생성·개명) 중에는 핸들러 전체가 비활성이고, IME 조합 중 keydown 은
  `isImeCompositionKeydown` 으로 걸러낸다.
- 컨텍스트 메뉴 항목에는 같은 표에서 온 `ContextMenuShortcut` 라벨이 붙는다(macOS 기호 고정 —
  Finder 표시처럼 macOS 전용 동작이 섞여 있다). 수식키 표기 순서는 앱의 기존
  `formatKeymapShortcut`(⌃⌥⇧⌘)을 따르므로 상대 경로 복사는 `⌥⇧⌘C` 로 그려진다.
- **[미확인] `␣` 미리보기 뒤 포커스**: 트리는 포커스를 넘기지 않지만, 열린 파일이 에디터를 새로
  붙이면 `code-editor.tsx` 의 `editor.focus()` 가 포커스를 가져간다(클릭 미리보기와 동일). 실기
  확인 후 필요하면 별도로 다룬다.

## 3. 검색 (FR-C5)

- 프로젝트 전역 텍스트 검색: Rust `domain/search` 가 **ripgrep 라이브러리(grep 크레이트 계열)** 또는
  자체 병렬 스캔으로 수행(구현 시 grep-searcher 채택 검토 — 무시 목록·.gitignore 존중).
- UI: 검색어 + 옵션(대소문자/단어/정규식) + 포함/제외 glob, 결과는 파일별 그룹 트리(매치 라인
  하이라이트), 클릭 시 해당 위치로 열기. 결과 상한(예: 10,000 매치) + "더 보기".
- 스트리밍: 결과는 Channel 로 점진 수신(대형 리포에서 첫 결과 즉시 표시), 새 검색 시작 시
  이전 검색 태스크 취소(CancellationToken).
- 치환(replace)은 2차.

### 3.1 구현 확정 (d-51 F2)

- **결과 리스트는 가상화**돼 있다(`features/search/search-results-list.tsx`). 그룹 헤더 + 매치 행을
  `buildSearchResultRows` 로 한 줄 목록으로 평탄화하고 `@tanstack/react-virtual` 로 창 안의 행만
  마운트한다 — 컨텍스트 줄 때문에 행 높이가 가변이라 `estimateSearchResultRowHeight` + 실측
  (`measureElement`)을 함께 쓴다. 리스트가 자체 스크롤 뷰포트를 소유하므로 호출부는 결과가 있을
  때 `ScrollContainer` 로 감싸지 않는다. 행 key 는 인덱스가 아니라 경로 기준이다(백엔드 병렬 워크
  라 파일 도착 순서가 비결정 — d-50 S1a).
- **실행 상태**는 `SearchRunStatus`(`idle`·`running`·`completed`·`failed`) 4종이다. 미실행·실패를
  "0건"으로 표기하던 문제를 없애고, 본문은 `resolveSearchResultsView` 하나로 갈린다.
- **결과 잘림**: 총계가 `SEARCH_MATCH_LIMIT`(10,000, `shared/constants/search.ts` — Rust
  `domain::search::types::SEARCH_MATCH_LIMIT` 미러)에 도달하면 `search.truncated` 안내를 띄운다.
- **Replace All 은 결과의 출처 쿼리(`ranQuery`)로만 치환**한다. 검색 후 입력·토글을 바꾸면 버튼이
  비활성되고 `search.replaceStaleHint` 가 뜬다 — 화면에 없던 매치를 되돌릴 수 없게 치환하던 경로를
  차단한다. 치환 후 재검색도 같은 스냅샷으로 돈다.
- **치환 스킵 보고**: `SearchReplaceResult.skipped`/`skippedCount`(d-50 S1a)를 경고 토스트로 사유별
  나열한다(`entities/search/replace-skip-report.ts`, 최대 5건 + "외 N개").
- **제외 글롭은 단순 `*` 매처**다(`domain/search/service.rs::glob_match`) — 프로젝트 상대 경로
  전체와 대조하고 `*` 는 `/` 도 넘는다. `**/`·중괄호·쉼표 목록은 지원하지 않으므로 placeholder
  예시도 `*.test.ts` 형태다.
- **projectId 스코프**: 검색 뷰는 `key={projectId}` 로 마운트된다. 프로젝트를 바꾸면 쿼리·결과·
  폴더 범위가 함께 초기화돼 이전 프로젝트의 경로를 새 프로젝트 레이아웃에 여는 교차 오염이 없다.
- **결과 그룹 헤더의 파일 아이콘**은 탐색기·탭 바와 같은 레지스트리를 거친다
  (`shared/ui/file-group-header.tsx` → `FileTypeIcon` → `resolveFileIcon`, d-58 §1.B). 같은 헤더를
  쓰는 문제(Problems) 패널도 함께 적용된다. 색은 레지스트리의 `colorClass` 가 준다.
- **취소 정책**: 새 검색은 명시 `search_cancel` 을 앞세우지 않는다 — `search_run` 의 `begin_search`
  가 같은 `(owner, sessionId)` 의 직전 토큰을 이미 취소하고, 앞세운 fire-and-forget 취소는 순서가
  뒤집히면 **새 실행**을 취소해 결과 0건·무에러가 됐다. 대신 **언마운트 시**에는 취소한다.

### 3.2 검색 에디터 탭

- 검색 에디터 탭(`widgets/search-editor/search-editor-pane.tsx`)은 pane 의 **활성 탭일 때만**
  마운트되므로, 매치를 클릭해 같은 pane 에 파일을 열면 그대로 언마운트된다.
- `entities/search/search-editor-memory.ts` 가 **탭 id 키**로 입력값(쿼리·토글·제외 글롭·컨텍스트
  줄 수)과 결과 스냅샷을 들고 있다가 복귀 시 복원한다. 복원된 탭은 마운트 자동 실행을 하지 않는다
  (원래 쿼리로 재실행해 결과를 덮어쓰던 동작 제거). 항목은 `SEARCH_EDITOR_MEMORY_LIMIT`(16) LRU 로
  제한되고, 저장된 projectId 가 다르면 폐기된다.
- 편집된 쿼리·토글을 탭 레코드(`TabKind::SearchEditor.query`)에 영속화하는 것은 백엔드 표면 변경
  이라 이 배치 범위 밖이다 — 앱 재시작 시에는 탭이 열릴 때의 원래 쿼리로 돌아온다.

### 3.3 실행 트리거 — 실시간 검색 (d-58 §1.B)

- 검색 패널은 Enter 전용이 아니다. `settings.searchOnType`(기본 on, Settings > 인터페이스 > 검색)이
  켜져 있으면 검색어·옵션(대소문자/단어/정규식/.gitignore/제외 글롭/폴더 범위)이 바뀐 뒤 입력이
  `settings.searchOnTypeDebounceMs`(기본 300ms, `[50, 2000]` 클램프) 동안 멈추면 실행한다.
  **트레일링 전용**이라 첫 글자에서는 돌지 않는다. 안내 문구도 설정에 따라 갈린다
  (`search.liveSearchHint` / `search.pressEnterHint`).
- **Enter 는 그대로 즉시 실행**하고, 대기 중이던 디바운스를 취소한다(중복 실행 방지).
  **검색 이력(`recent_searches`)에 쌓는 것은 Enter 뿐이다** — 입력 중 실행은 `recordHistory: false`
  라, 최종 검색어로 가는 길에 지나간 접두사가 이력을 채우지 않는다.
- **실시간 실행의 실패는 토스트를 띄우지 않는다**(`RunSearchOptions.live`). 실제로 걸리는 실패는
  `(foo` 처럼 **아직 덜 쓴 정규식**이라, 토스트로 알리면 글자마다 에러 알림이 쌓인다. 대신 패널
  본문의 기존 실패 표시(`search.failed`)만 뜨고, Enter 실행은 현행대로 토스트까지 띄운다.
- 실시간 실행은 성능 표본에서 제외한다 — `search.run-requested` 마크를 **찍지도 닫지도** 않는다
  (`quality-assurance/2026-09-04-perf-baseline.md` 전역 검색 지표).
- 이전 실행 취소는 추가 작업이 없다. 위 §3.1 "취소 정책" 그대로 `search_run` 의 `begin_search`
  supersede 에 맡긴다.
- **IME**: WKWebView 가 composition 이벤트를 주지 않으므로(`shared/lib/ime-composition.ts`) 한글·
  일본어 입력은 조합 중 상태 그대로 검색될 수 있다. keydown 229 가드를 걸면 해당 언어 입력 전체가
  실시간에서 빠지므로, **디바운스만으로 감수**하고 확정은 Enter 에 맡긴다. 최소 질의 길이는 두지
  않는다(VS Code 동형).
- **검색 에디터 탭(§3.2)은 범위 밖**이다 — Enter 전용을 유지한다.

## 4. 수명주기

- 사이드바 뷰 전환은 탭 유지(마운트 유지 여부: 검색 상태 보존을 위해 Search 뷰는 keep-alive,
  Explorer 는 가상화라 재마운트 저렴 — 상태는 Rust 소유이므로 어느 쪽이든 복원됨).
- 트리 row 캐시는 view 메모리에 페이지 단위 LRU(최대 수천 행) — reload 시 Rust 에서 재조회.


## 5. Phase 7.5 확장 — 파일 아이콘 (사용자 지적 6번)

실구현은 SVG vendoring 없이 **`lucide-react` 아이콘 컴포넌트로 매핑**한다(라이선스 표기 불요) —
매핑 로직은 `shared/lib/file-icon.ts`, 아이콘 컴포넌트 레지스트리는
`shared/icons/file-icon-registry.ts` 2파일 구성. 아이콘 세트를 끄는(단색화) 설정은 도입하지
않았다. (초안의 material-icon-theme vendoring 안은 폐기.)

- 매핑은 **데이터로 분리**돼 있다: 확장자 맵 + 파일명 맵(예: `package.json`, `Cargo.toml`,
  `Dockerfile`) + 폴더명 맵 + 폴백. **매칭 우선순위(파일명 > 확장자 > 폴백)는 순수 함수 +
  테스트**(`shared/lib/file-icon.ts`).
- ignored/삭제 상태의 흐림 처리는 테마 토큰(`explorer.git*`)으로.
