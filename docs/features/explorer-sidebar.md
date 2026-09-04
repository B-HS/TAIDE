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
- 클릭 = preview 탭으로 열기, 더블클릭 = 고정 탭(FR-C4, `tabs.md` §3).
- 키보드: ↑↓ 이동, ←→ 접기/펼치기, Enter 열기, 타이핑 시 이름 점프(typeahead).
- context menu: 새 파일/폴더, 이름 변경(인라인 입력), 삭제(휴지통 이동 + 확인), 복사/붙여넣기,
  경로 복사, Finder 에서 열기, (git 있으면) 하위 항목 — 전부 Rust fs 명령 경유.
- 파일 조작 후 watcher 이벤트로 트리 갱신(자기 쓰기 echo 는 origin 플래그로 낙관 갱신).
- 트리 내 DND(파일 이동)는 2차(dnd-kit 재사용).

#### 활성 파일 자동 표시 (autoReveal, 2026-09-04 구현)

VS Code `explorer.autoReveal` 파리티. 활성 에디터 탭이 파일이면 트리가 그 파일까지 **펼쳐 선택**한다.

- **게이트 3축** — 셋 다 만족할 때만 동작한다. ① 설정 `explorerAutoReveal`(기본 on) ② 사이드바가
  실제로 보임(`shellView.sidebarCollapsed` 아님 + Zen 아님) ③ 사이드바 뷰가 `files`. 판정은
  `widgets/explorer/explorer-auto-reveal.ts` 의 순수 함수 `decideAutoReveal`(`skip`/`select-only`/
  `reveal-then-select`)이 전담하고, 훅은 입력 공급과 실행만 한다.
- **이미 보이는 행은 IPC 0** — `visiblePaths` 에 있으면 `select-only` 로 기존 `selectPathRequest`
  경로만 태운다. `tree_reveal` 은 성공 시 트리 페이지 전체를 재직렬화·교체하므로(`ipc-contract.md`
  "응답 형태 불변"), 스크롤만 필요한 경우까지 왕복시키지 않는다.
- **중복 억제** — 마지막으로 처리한 경로를 훅의 ref 에 기록해 같은 파일로는 다시 reveal 하지 않는다.
  `skip` 은 기록하지 않으므로, 사이드바를 접은 채 파일을 바꾼 뒤 다시 펼치면 그때 reveal 된다.
- **포커스 불탈취** — 선택은 `file-tree.tsx` 의 `selectByIndex`(`setSelectedId` + `scrollToIndex`)
  로만 이뤄지고 DOM focus 를 건드리지 않는다. 에디터 타이핑이 끊기지 않는다.
- **주 트리만 본다** — 사이드바는 주창에만 마운트되므로 활성 경로는 `layout.root`/`layout.focusedPane`
  에서만 읽는다(`resolveWindowPaneTree` 금지). 보조창의 활성 탭이 주창 트리를 흔들지 않는다.
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
  재조회 — 루트 재스캔 금지. 무시 목록(`.git`, `node_modules`, `target`, `dist`, `.next` 등)은
  워처·트리·검색이 **동일 규칙 공유**(`shared` 상수 + Rust 상수 동기). 다만 목록이 가리키는 것은
  **디렉토리 이름**이므로 적용 지점이 셋 다 다르다 — 트리는 항목이 디렉토리일 때만 감추고, 워처는
  경로의 **조상 성분에만** 적용한다(마지막 성분 제외 — d-50 S6, `ipc-contract.md`). 그래서 루트에
  `build` 라는 이름의 파일이 있으면 트리에도 보이고 그 변경 이벤트도 도착한다.
- Linux inotify watch 한도 초과는 조용히 실패 — 에러를 UI 배너로 노출(research 함정).
- 이벤트는 경로 배열 1건으로 묶어 emit(파일당 1 emit 금지).
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
