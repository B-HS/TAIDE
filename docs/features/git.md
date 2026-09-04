# 기능 — Git 통합

> FR-F. SCM 패널(changes·commit·push)·diff·그래프. 백엔드 결정은 ADR-0006,
> API 확정 근거는 `docs/research/git2.md`, UI 참조 동작은 `docs/research/vscode-behaviors.md` §1~§7.
> 에디터 쪽 git 표시(gutter·inline blame)는 `editor.md` 가 정본이고 데이터 소스만 여기(git 도메인)다.

## 1. 상태 모델

- Rust git 도메인이 `StatusRow { path, absPath, origPath, origAbsPath, staged, unstaged, isConflicted }` 로
  **정규화해서** 넘긴다(비트플래그를 프론트로 넘기지 않음 — research 적용 가이드). 한 파일이
  staged·unstaged 를 동시에 가질 수 있으므로 두 그룹에 각각 나타난다(VSCode 동일). `path`/`origPath`
  는 저장소-상대 경로(git 커맨드에 그대로 되돌려 쓴다 — stage/unstage/discard/diff), `absPath`/
  `origAbsPath`(T0 감사 #18, `docs/data-model.md` §13)는 파일 도메인이 요구하는 절대경로다. 소비부
  (`git-panel.tsx`)는 "파일 열기"·"경로 복사"·"탐색기에 표시" 에서만 `absPath` 를 쓰고, git 동작
  (stage/unstage/discard/diff 탭 열기)에는 계속 `path` 를 쓴다 — repo-relative 경로를 현재 활성
  프로젝트 루트 기준으로 잘못 재해석해 다른 파일을 열거나 덮어쓰는 사고(다중 프로젝트·워크트리)를
  막기 위한 구분이다.
- 상태 분류: Modified(M)·Added(A)·Deleted(D)·Renamed(R)·Untracked(U)·TypeChange(T)·Conflicted(!).
  Copied(C)는 statuses 로 검출 불가(research 확인) — 표시하지 않는다.
- 상태 문자·색은 `git.*` 테마 토큰만 사용(하드코딩 금지).
- 갱신 경로: watcher 가 `.git/index` 변경 → status invalidate, `.git/HEAD`·`refs/**` 변경 →
  log·branch·ahead/behind invalidate, `.git/objects/**` 는 무시. 워킹트리(외부) 변경은
  `fs:changed` 배치가 GIT.STATUS + 열린 경로의 GUTTER/DIFF 쿼리를 무효화한다(d-44 계약).
  pathspec 부분 갱신·주기적 전체 보정은 도입하지 않았다 — status 는 매 조회 전체 스캔이다(§7).
- `git:status-changed`·`git:refs-changed` 두 이벤트의 무효화도 뮤테이션과 **같은
  `isGitQueryScopeMutable` predicate** 로 좁힌다(d-51 F7 · 감사 §1-8). 그러지 않으면 뮤테이션이
  지켜 둔 `QUERY_KEY.GIT.REV_IMMUTABLE_SCOPES`(commit-files·show — 불변 SHA 키, `staleTime:
  Infinity`) 보장이 그 뮤테이션의 이벤트 에코에서 무너져, 커밋 상세·파일 히스토리 패널이 스테이지·
  스태시·커밋마다 블롭을 다시 받았다.

## 2. SCM 패널 (탐색 사이드바의 Git 뷰)

구성(위→아래):

1. **헤더 바**: 현재 브랜치(클릭 시 브랜치 목록/전환 — 2차), ahead/behind 배지(`graph_ahead_behind`),
   **Stash 버튼**(`disabled={!canStash}` — 변경이 0건이면 비활성), Sync 버튼(pull+push),
   remote 이름/주소 표시(FR-F1 — remote url 은 `Remote::url()`). Stash 실행은 저장소 레벨 액션이라
   Sync 옆이 제자리다(2026-09-04 이전에는 stash 섹션 헤더의 hover 액션이었다 — 아래 §2.1).
2. **커밋 입력**: 멀티라인 메시지 박스 + Commit 버튼 + 드롭다운.
3. **섹션**(순서 고정): `Merge Changes`(충돌) → `Staged Changes` → `Changes` → `Stashes` → `Graph`(§5).
   다섯 섹션 전부 같은 접이식 헤더 컴포넌트(`features/git/git-section-header.tsx`)를 쓴다 — chevron +
   카운트 배지 + hover/focus-within 액션(Stage All / Unstage All).

### 2.1 섹션 가시성·접힘 (2026-09-04, 사용성 배치 4 ⑤)

- **가시성 판정 단일 출처는 순수 함수 `widgets/git-panel/git-sections.ts` 의 `buildGitSections`**
  (+ `git-sections.test.ts`). 이 저장소에는 DOM 렌더 하네스가 없으므로 이 UX 의 회귀 검증은 이
  함수에 모여 있다.
- **빈 섹션은 그리지 않는다.** 변경 그룹 3종은 0건이면 미렌더(기존), `Stashes` 는
  `stashes.length > 0` 일 때만 렌더한다. 이전에는 stash 섹션 조건이 `stashes.length > 0 || canStash`
  라서 **스태시가 0건이어도 워킹트리가 더러우면 빈 "스태시" 헤더가 목록 맨 위에** 그려졌고, 이것이
  "stash 와 changes 영역 구분이 모호하다"는 보고의 실제 원인이었다(색 대비 문제가 아니었다).
- 변경 그룹 3종이 모두 비면 `git.noChanges` 한 줄을 그린다(신규 i18n 키 0 — 기존 미사용 키 재사용).
- **접힘 상태는 `entities/git/git-section-collapse-memory.ts` 모듈 스코프 메모리**에 둔다
  (`commit-message-memory.ts` 와 같은 계층·같은 수명 모델, `architecture.md` §6.4 등재). 기본값은
  **Stashes 만 접힘**이고 나머지는 펼침. 사이드바 뷰를 파일/검색으로 바꿔 패널이 언마운트돼도,
  프로젝트를 바꿔도 유지되며 앱 재시작 시 기본값으로 돌아간다(설정·레이아웃 스키마 무변경).
- **접힌 섹션도 카운트 배지를 계속 보여준다.** 접은 `Staged Changes` 가 개수를 숨기면 "스테이지된 게
  없다"고 오인해 `resolveCommitGate` 의 stage-all 확인 경로(§3)로 흘러 워킹트리 전체를 커밋할 수 있다.
- **헤더 대비는 구분선(`border-t`) + `sticky top-0`(배경 `bg-explorer-background` 명시) + 행 `pl-4`
  들여쓰기**로만 만든다. 헤더에 정적 배경 톤을 주지 않는 것은 의도적이다 — vsix 임포트 테마에서
  list 상태 배경이 배경색과 충돌한 전례(`theme-system.md` §8.2.2·§8.2.4) 때문에 일부 테마에서
  헤더 배경과 행 hover 가 같은 색이 되어 오히려 구분이 사라진다.

파일 row:

- 표시: 파일명 + 흐린 디렉토리 경로 + 상태 문자(우측). Renamed 는 `old → new` 툴팁.
- 클릭 = **Open Changes**: 해당 파일의 diff 를 preview 탭으로 연다(§4).
- **키보드 (2026-08-29 구현, 2026-09-04 헤더 포함으로 확장 · 같은 날 인덱스 기반으로 재작성)**: 행과
  **섹션 헤더**가 함께 `role='button'` + Enter/Space 활성화 대상이고, ↑↓ 는 두 종류를 패널의 항목
  순서로 가로지르며 로빙 포커스를 옮긴다(`widgets/git-panel/change-row-navigation.ts` — 파일트리와
  같은 진입 규칙: 밖에서 ↓=첫 항목·↑=마지막 항목, 끝에서 비순환). 헤더에 포커스가 있을 때 **→ 는
  펼치고 ← 는 접는다**(무의미한 방향은 활성화 처리로 폴백). 포커스 항목은 `focus-within` 배경으로
  표시. 헤더를 로빙에 넣은 것은 접기 때문만이 아니라, hover 로만 나타나던 Stage All / Unstage All 이
  키보드로 도달 불가였던 결함(`group-focus-within:flex` 로 동시 해소)을 함께 고치기 위해서다.
  접힌 섹션의 행은 렌더 자체를 건너뛰므로 로빙 시퀀스에서도 빠진다.
  - **순서의 정본은 DOM 이 아니라 인덱스다**(§2.2 가상화 전제). 각 항목은 위치 인덱스를
    `data-git-roving-index` 로 들고 있고(가상 항목은 위치 래퍼가, 스태시·그래프 섹션은 섹션
    래퍼가), 이동은 `resolveNextChangeRowIndex(key, activeIndex, rovingItemCount)` 로 계산한 뒤
    필요하면 `scrollToIndex` 로 먼저 스크롤하고 마운트된 뒤 포커스한다. `querySelectorAll` 로
    문서 순서를 세던 이전 방식은 뷰포트 밖 행이 DOM 에 없어 조용히 끊긴다.
    `data-git-change-row`·`data-git-section-header` 는 e2e 스펙 07·19 가 행·헤더를 집는 마커로
    그대로 유지한다.
  - 로빙 항목 수 = 가상화된 변경 목록 행 수 + 보이는 스태시 섹션(1) + 보이는 그래프 섹션(1).
    스태시의 Apply 버튼·그래프 커밋 행·행/헤더의 hover 액션 버튼처럼 **항목 내부 컨트롤**에
    포커스가 있을 때는 ↑↓ 를 건드리지 않는다(그 항목의 콘텐츠이지 패널의 항목이 아니다).
    이전에는 이 경우 셀렉터가 아무것도 못 찾아 포커스가 **목록 맨 위로 튀었다**.
- hover 액션 아이콘: Changes 그룹 `+`(stage)·`↺`(discard)·Open File,
  Staged 그룹 `−`(unstage)·Open File.
- context menu(FR-F4):
  - Open File — diff 가 아니라 **실파일을 에디터 탭으로** 연다
  - Open Changes
  - Stage Changes / Unstage Changes (그룹에 따라)
  - Discard Changes — 확인 다이얼로그 필수. **untracked 파일은 즉시 삭제가 아니라 OS 휴지통으로**
    (VSCode 동일 — 데이터 손실 방지, research 함정 절). tracked discard 는
    `checkout_index/checkout_head + force()`.
  - Copy Path / Reveal in Explorer(파일 트리에서 보기)

### 2.2 변경 목록 가상화 · 컨텍스트 메뉴 단일화 (2026-09-04, 사용성 배치 4 ③ C.2-7)

- **변경 그룹 3종(Merge·Staged·Changes)은 하나의 가상 목록**이다. `change-row-navigation.ts` 의
  `buildGitChangeListRows` 가 "헤더 1행 + 그 섹션의 행들" 을 평탄화하고
  (`search-result-rows.ts`·`problem-list-rows.ts` 와 같은 확립 패턴),
  `@tanstack/react-virtual` 이 뷰포트 안 행만 마운트한다. 헤더·행 모두 `h-6`(24px) 고정이라
  재측정 없이 배치된다. 이전에는 브랜치 전환·대량 생성 리포에서 **수천 행이 통째로 DOM 에**
  들어갔다(파일트리·검색·Problems·커밋그래프는 이미 가상화돼 있었고 변경 목록만 빠져 있었다 —
  조사 3a M1).
- **가상 목록은 스크롤 뷰포트의 첫 요소여야 한다.** 패널은 변경·스태시·그래프를 스크롤바 하나로
  훑는 구조라 가상 목록의 오프셋이 0 이라는 전제 위에서 측정한다(위에 뭔가를 끼우려면
  `scrollMargin` 이 필요하다).
- **sticky 헤더는 `rangeExtractor` 로 유지한다.** 가상 행은 절대배치라 그 안의 `sticky` 는 자기
  24px 상자를 벗어날 수 없다. 그래서 화면 맨 위에 붙어야 할 헤더(`resolveStickyHeaderIndex`)를
  렌더 범위에 강제로 포함시키고, 그 항목만 translate 없이 흐름에 그린다. 어느 그룹의 행을 보고
  있는지 알려주는 sticky 는 §2.1 의 헤더 대비 규칙이 기대는 축이라 포기하지 않는다.
- **컨텍스트 메뉴는 목록당 1개**다. 이전에는 행마다 Radix `ContextMenu` 루트 + 항목 7개를 미리
  만들었다. 지금은 우클릭 이벤트에서 대상 행을 역산하고, 행이 아닌 곳(헤더·빈 영역)에서는
  `preventDefault()` 로 메뉴를 열지 않는다(Radix `composeEventHandlers` 가 defaultPrevented 인
  이벤트에서 자기 핸들러를 건너뛴다). 메뉴 대상은 인덱스가 아니라 **경로**로 기억한다 — 메뉴가
  열려 있는 사이 status 가 갱신돼도 다른 파일에 액션이 걸리지 않게.
- `features/git/git-change-group.tsx` 는 컴포넌트가 아니라 **그룹 설정 빌더**다
  (`buildGitChangeGroupConfig` → 제목·헤더 액션·정규화된 행·행 액션·컨텍스트 메뉴 항목·클릭 동작).
  세 그룹이 더 이상 자기 DOM 서브트리를 갖지 않으므로, 패널이 섹션 id 로 그룹 동작을 찾는다.

## 3. Commit / Push (FR-F5)

- 커밋: 메시지 입력 후 `⌘Enter`(macOS)/`Ctrl+Enter` 또는 Commit 버튼.
  - **커밋 대상 규칙(사용자 확정, 2026-08-06)**: staged 가 있으면 그것만 커밋. 없으면
    "변경 전체를 스테이지하고 커밋할까요?" 확인 다이얼로그 후 진행(현행 VSCode 실동작과 동일).
  - **미해결 충돌이 하나라도 있으면 커밋을 차단**한다(버튼 비활성 + `git.commitBlockedByConflicts`
    안내, d-51 F3 / 감사 §4-B A4). 충돌 행은 Staged/Changes 어느 그룹에도 속하지 않아 위의
    "staged 0건" 판정이 stage-all 경로로 흘렀고, 그 경로가 충돌 마커째 스테이지·커밋했다.
    판정 단일 출처는 `widgets/git-panel/commit-gate.ts` 의 `resolveCommitGate` — 패널(버튼·
    ⌘Enter)과 컨테이너(`stageAll` 결정)가 같은 함수를 쓴다. git 자체도 unmerged path 가 있으면
    커밋을 거부하므로 동작이 도구와 일치한다.
  - 빈 메시지면 커밋 버튼 비활성. user.name/email 미설정이면(`repo.signature()` 실패) 안내 배너.
  - 입력 중인 커밋 메시지는 **projectId 별로 기억**한다(`entities/git/commit-message-memory.ts`).
    사이드바 뷰를 파일/검색으로 바꾸면 `GitPanelContainer` 가 언마운트되지만 메시지는 남고,
    프로젝트를 바꾸면 그 프로젝트의 메시지로 갈아끼운다(감사 §4-B C6 — 뷰 전환 소실과 프로젝트
    교차 잔존을 한 축에서 해소). 커밋 성공 시 해당 항목을 지운다.
- 커밋 입력란 우상단 Sparkles 버튼으로 AI 가 diff 로부터 커밋 메시지를 생성한다(Wave G,
  §6 의 `git_diff_staged_text` 를 소비) — 흐름·프롬프트·취소는 `ai.md` §4 가 정본. staged 변경이
  없어도 unstaged 변경(충돌 중인 파일 포함)이 있으면 버튼이 활성화된다(Wave H — 워킹트리 diff 로
  폴백, `ai.md` §7 의 `usedFallback`). 충돌 파일만 남은 상태(머지 중, 아직 아무것도 stage 하지
  않음)도 버튼을 활성화한다 — 백엔드 폴백은 conflicted 여부와 무관하게 워킹트리 델타를 만들 수
  있으므로, 프론트 게이트를 그 능력에 맞춰 열어 둔다(그러지 않으면 "생성 가능한데 버튼은 비활성"인
  표시-능력 불일치가 생긴다).
- Commit 드롭다운(1차 범위): `Commit` / `Commit (Amend)` / `Commit All` / `Undo Last Commit`.
  (`Commit & Push`·`Commit & Sync` 는 2차.)
- Push / Pull / Fetch / Sync: 헤더 버튼 + 드롭다운.
- **실행 경로는 ADR-0006**: commit·push·pull·fetch 는 git CLI 서브프로세스(hook·credential helper·서명
  전부 사용자 환경 그대로). stage/unstage/discard 는 git2.
  - CLI 실패 시 stderr 를 요약해 에러 토스트 + 상세 보기(원문). push 인증 실패는
    "ssh-agent/credential helper 확인" 안내를 함께.
  - 진행 표시: push/pull 은 mutation 의 `isPending` 으로 헤더 스피너를 그린다(별도 진행/완료
    이벤트는 없다 — §6).
- 커밋/푸시 후: status·log·ahead/behind invalidate 이벤트 발행.

## 4. Diff 뷰 (FR-F3)

- Monaco DiffEditor. **기본 side-by-side**, `⌥\`/`Alt+\` 로 inline 토글(설정 키 `diffEditor.renderSideBySide`
  — VSCode 와 같은 이름 사용, research §4).
- diff 원본 선택:
  - Changes 그룹 파일: index ↔ workdir
  - Staged 그룹 파일: HEAD ↔ index
  - 커밋 상세에서: parent ↔ commit — 손 QA 1차 수정(2026-08-18)으로 **에디터 탭**(패널 내부 인라인
    아님)에 뜬다. 아래 §9 참조.
- 원본(old side) 내용은 Rust 가 blob 을 추출해 제공(`git_show_file(rev, path)`), 새 쪽은 파일/버퍼.
- diff 탭은 `TabKind::Diff{ path, mode }` — 같은 파일 diff 재클릭 시 기존 탭 재사용(preview 규칙).
  커밋 상세 diff 는 같은 variant 를 `rev`/`parentRev`/`beforePath` 로 확장해 재사용한다(§9).
- **`TabKind::Diff.path` 는 언제나 절대경로다**(d-51 F3 / 감사 §4-B B10). 탭 재사용은 `kind` 전체
  동등성으로 판정하므로, SCM 패널만 저장소 상대경로를 쓰던 동안 탭바 "Open Changes" 와 같은 파일이
  서로 다른 탭이 됐고, `fs:changed`(절대경로)가 그 탭의 `GIT.DIFF` 캐시를 무효화하지 못했으며,
  `DiffPane` 의 충돌 판정도 행과 만나지 못했다. 커밋 상세 diff 도 같은 이유로 `absPath`/
  `origAbsPath` 를 쓴다 — 파일 히스토리 패널이 이미 절대경로로 `GIT.SHOW` 를 키잉하고 있었다.
  `DiffPane` 의 충돌 판정(`widgets/diff-pane/diff-stageability.ts`)은 절대·상대 두 표기를 모두
  받아, 통일 이전에 영속된 레이아웃의 diff 탭도 인식한다.
- 워킹트리·staged diff 도 `beforePath`(개명 원경로)를 `git_diff_file` 에 전달한다 — 탭 kind →
  `DiffPane` → `gitDiffFileQueryOptions` → `QUERY_KEY.GIT.DIFF`(끝에 덧붙여 `fs:changed` 의
  path 인덱스 3 을 보존) 축이 이어져 있다. 다만 **상태 행(`StatusRow`)에는 아직 원경로 소스가
  없다** — 감사 §2 M-2(양방향 rename) 전까지 `origPath`/`origAbsPath` 는 구조적으로 항상 `null`
  이라, 이 축이 실제로 값을 나르는 것은 커밋 diff 쪽뿐이다(d-50 S3 §5 발견).
- diff 에디터는 `ignoreTrimWhitespace: false` 로 만든다 — monaco 기본값(`true`)은 앞뒤 공백만 다른
  줄을 같다고 보아 (a) 공백만 바뀐 변경을 아예 안 그리고 (b) 변경 구간 중간의 공백 전용 줄에서
  hunk 를 둘로 쪼갠다. `git_stage_hunk`/`git_unstage_hunk` 는 넘어온 `(start, end)` 를 libgit2 의
  hunk 경계와 **정확일치**로 찾으므로, 쪼개진 범위는 백엔드에 존재하지 않아 거터 스테이지가
  `error.git.hunkNotFound` 로 실패했다(감사 §4-B D8).
- 툴바: inline 토글, 파일로 이동 버튼.
- **diff 표시 설정 2종**(d-53 U1 · 감사 §5): `editorDiffHideUnchangedRegions`(monaco
  `hideUnchangedRegions.enabled` — 변경 없는 영역 접기) · `editorDiffShowMoves`(monaco
  `experimental.showMoves` — 이동한 블록을 삭제+추가가 아니라 이동으로 표시). 둘 다 기본 **false**
  (VS Code 파리티)이고 설정 화면 Editor 섹션에 있다. **`DiffView` 를 쓰는 모든 표면에 같은 값이
  걸린다** — `DiffPane`(SCM), `CommitFileDiff`(커밋 상세), `ConflictCompareDialog`(충돌 비교).
  앞의 둘은 각자 `settingsQueryOptions()` 를 읽고, `features` 레이어라 쿼리를 읽지 않는 충돌
  다이얼로그는 `EditorPane` 이 `shared/lib/diff-view-settings.ts` 의
  `resolveDiffViewSettingsProps(settings)` 결과를 prop 으로 내려준다. 툴바의 Collapse Unchanged
  Regions 버튼(2차)은 이 설정과 별개로 남아 있다.
- **`git_diff_staged_text(projectId)`**(Wave G, git2 native — `diff_tree_to_index` + `Diff::print`):
  파일별 `DiffSides` 가 아니라 staged 전체를 **하나의 unified diff 텍스트**로 반환한다. 에디터 diff
  뷰가 아니라 AI 커밋 메시지 생성(`ai.md` §4·§7)의 유일한 소비처 — 32KiB 상한(초과 시 UTF-8 경계
  안전 절삭 + `truncated`), 바이너리·lock 파일(`bun.lock`·`Cargo.lock`·`package-lock.json`·
  `pnpm-lock.yaml`·`yarn.lock`)·시크릿류 파일(`.env`·`.env.*`·`.netrc`·`.npmrc`·`*.pem`·`*.key`·
  `*.p12`·`*.pfx`·`*.jks`·`*.keystore`·`*.ppk`·`*.der`·`*.crt`·`id_rsa`/`id_dsa`/`id_ecdsa`/
  `id_ed25519`)은 본문에서 제외하고 `skippedFiles` 에 경로만 나열한다(시크릿류는 외부 AI provider
  로 평문 전송되지 않도록 — security.md §1, 이름/확장자 휴리스틱이므로 완전하지 않다 — 콘텐츠 스캔이
  아니다). 절삭·제외 사실은 `truncated`/`skippedFiles` 필드뿐 아니라 `diffText` 본문에도 안내
  문자열로 남는다(`ai.md` §7). staged 0건이라 워킹트리로 폴백할 때는 `recurse_untracked_dirs(true)`
  도 함께 켠다 — 그러지 않으면 새로 만든 untracked *디렉토리*가 개별 파일이 아니라 디렉토리 델타
  하나로 뭉뚱그려져, 그 안 파일 내용이 diff 에 전혀 반영되지 않는다.

## 5. 커밋 그래프 (FR-F6)

- 데이터: Rust 가 `LogEntry { id, parents[], summary, author, timeUnix, refs[] }` 페이지 단위 반환
  (`Revwalk`, `Sort::TOPOLOGICAL | TIME`, `push_glob("refs/heads/*")` + HEAD. skip/take 페이지네이션,
  무한 스크롤).
- **레인 배치는 프론트에서 계산**(research 적용 가이드 — Rust 는 부모 관계만):
  1. 정렬된 커밋을 위→아래 순회하며 열린 레인 집합 유지
  2. 커밋이 어떤 열린 레인의 기대 부모면 그 레인 차지, 아니면 가장 왼쪽 빈 레인 할당
  3. 머지 커밋: 첫 부모는 자기 레인, 나머지 부모는 새 레인 열어 곡선 연결
  4. 레인 색 = `graph.lane{n%12}` 토큰(12색 순환)
- 렌더: SVG(rounded 곡선), 행 = 점 + 메시지 + author + 상대시각 + short hash.
  브랜치 라벨(로컬/원격 병합 표시)·태그 라벨·HEAD 마커. 미커밋 변경은 최상단 회색 열린 원.
- 행 클릭: 커밋 상세(변경 파일 목록 → 파일 클릭 시 diff 에디터 탭, §9).
- 성능: 가상 스크롤(행 높이 고정), 페이지 100 커밋 단위 로드.
- **섹션 헤더는 §2.1 의 공용 접이식 헤더**를 쓴다(2026-09-04 통일 — 그전까지 이 섹션만 인라인
  마크업이라 패널 안에 헤더 스타일이 2종 공존했다). 커밋이 0건이면 섹션을 렌더하지 않고, 접으면
  그래프와 그 아래 커밋 상세 패널이 함께 숨는다. 커밋 상세 패널의 "변경된 파일" 줄은 섹션 헤더가
  아니라 그 패널 내부의 라벨이므로 접이식이 아니고 로빙 시퀀스에도 들어가지 않는다.

## 6. IPC (상세: `docs/ipc-contract.md`)

- query: `git_status`, `git_diff_file(path, mode)`, `git_diff_staged_text(projectId)`(Wave G,
  staged 0건 시 워킹트리 폴백은 Wave H — 상세는 `ai.md` §7),
  `git_show_file(rev, path)`, `git_log(skip, take)`, `git_ahead_behind`, `git_remotes`,
  `git_gutter(path)`, `git_blame_range(path, from, to)`, `git_stash_list`
- mutation: `git_stage(paths)`, `git_unstage(paths)`, `git_discard(paths)`, `git_commit(message, opts)`,
  `git_push`, `git_pull`, `git_fetch`, `git_stash_push/apply/drop`, `git_undo_last_commit`
- 위 목록은 대표 발췌다 — 브랜치/태그/훙크·라인 스테이징/충돌 해석/리버트/파일 로그 등 **전체
  커맨드 표면은 `ipc-contract.md` 가 전수 정본**이다.
- event: `git:status-changed`, `git:refs-changed` — 이 2종이 전부다(진행/완료 이벤트 없음, §3)
- **부팅 워처 재부착 직후 합성 `git:status-changed` 1회(d-25)**: 상세는 `ipc-contract.md` 의 같은
  항목 참조 — attach 공백 구간 git 상태 정체 보정, 실제 변경 여부 무관.

## 7. 수명주기 · 성능

- git capability attach 시: discover(`.git` 탐색) → 초기 status → watcher 필터 등록.
  detach 시 대칭 해제. bare repo(workdir 없음)면 SCM 뷰 비활성.
- blame·log·대형 diff 는 `spawn_blocking` + 경로로 새 `Repository::open` (ADR-0006 스레드 모델).
  d-35 부터 stage/unstage/discard/commit/branch/stash/hunk/tag/checkout-remote 등 git2 인프로세스
  뮤테이션 13종도 동일하게 guard 보유 채 `spawn_blocking` (async 워커 스레드 비점유).
- push/fetch 는 repo 경로를 키로 한 `tokio::sync::Mutex` 로 동일 repo 요청끼리만 직렬화 대기(경합
  대신 큐잉 — d-35 §1-b). `git_pull` 은 이 락을 잡지 않는다(전체 `begin_mutation` 유지 불변) —
  같은 repo 의 fetch-vs-pull 경합은 이전처럼 git 자체 락 실패로 남는다.
- status 옵션 고정: `recurse_untracked_dirs(false)`, `include_ignored(false)`,
  `exclude_submodules(true)`(서브모듈은 2차), `renames_*(true)`. **`update_index` 는 의도적으로
  켜지 않는다**(감사 R4#11 — guard 없는 status 조회 경로가 인덱스를 쓰면 가드된 뮤테이션과 경합.
  `collect_status_rows` doc comment 정본).
- 대형 리포: 증분 캐시 없이 `git_status` 가 매 호출 전체 스캔으로 `Vec<StatusRow>` 를 반환하고,
  `GitStatusChanged` 이벤트는 project_id 만 담아 프론트가 status 쿼리를 통째로 무효화한다.
  (초안의 `HashMap<Path, StatusRow>` 증분 emit 은 도입하지 않았다 — 필요가 실측되면 재론)

## 8. 범위 (1차/2차)

| 1차 (MVP) | 2차 |
|-----------|-----|
| status·그룹·stage/unstage/discard·commit(⌘Enter, Amend, Commit All, Undo)·push/pull·diff(split/inline)·remote 표시·ahead/behind·그래프(레인·라벨·무한스크롤)·gutter·blame 데이터 | 브랜치 생성/전환/머지 UI, 커밋 상세 diff, 파일 히스토리, stash UI, Collapse Unchanged, Commit & Push/Sync, 서브모듈, 충돌 해결 UI |

> 위 표는 MVP 시점 스코프 구분이며, 브랜치/커밋 상세/파일 히스토리/stash UI/충돌 해결 UI 는 Wave C
> (`acknowledge/2026-08-15-wave-c-git-contract.md`)로 이미 구현됐다. §9 는 그중 커밋 상세 diff 의
> **배치**가 손 QA 1차 수정(2026-08-18)으로 다시 바뀐 부분만 다룬다.

## 9. 커밋 diff — 에디터 탭 승격 (손 QA 1차 수정, 2026-08-18)

> 계약: `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md` §2.6·§3. `ipc-contract.md`(§3 "손 QA
> 1차 발견 6건 수정")·`data-model.md`(§12) 교차 참조.

- **증상·원인**: Wave C 구현은 커밋 상세 패널(`commit-detail-panel.tsx`) 내부에 `h-80` 고정 높이
  영역을 두고 그 안에 `CommitFileDiff` 를 인라인 렌더했다 — 사이드바만한 폭에서 diff 를 보게 되는
  좁은 UX. Wave C 원계약 §3.2 원문은 "파일 클릭 → 기존 diff 탭(rev vs parent, `git_show_file`)"
  이었고, 검토 중 정정 L0-2 가 "`TabKind::Diff` 의 rev 확장은 별도 설계 결정이라 범위 밖"으로 미뤄
  두면서 인라인 렌더로 착지했었다. 이번 수정은 그 L0-2 가 미뤄 둔 확장을 채워 **원계약 문언으로
  복귀**한 것이지, 한 번 기각된 안(전용 `TabKind::CommitDiff` 신설)의 재론이 아니다.
  스크린샷 비교 등 실기 검증 없이 코드/문서 대조로 확정된 결함이라 `docs/bug/` 에 별도 리포트는
  없다 — 근거는 계약 §1 항목 6.
- **수정**: `commit-detail-panel.tsx` 의 `selectedFile` 상태·인라인 `CommitFileDiff` 렌더를 제거하고,
  파일 클릭 시 `useOpenTab` 으로 `TabKind::Diff{ path, staged: false, rev: commit.id, parentRev:
  commit.parents[0] ?? null, beforePath }`(rename 파일은 `beforePath` 가 `origPath`) 탭을
  `preview: true` 로 연다. 탭 제목은 `"{파일명} @ {짧은 해시}"`(`COMMIT_SHORT_HASH_LENGTH = 7` —
  `entities/git/git.constant.ts`, 커밋 그래프·파일 히스토리 패널의 짧은 해시 표기와 공용 상수).
- **렌더 분기**: `pane-node-view.tsx` 가 `kind === 'diff'` 탭을 `rev` 유무로 나눈다 — `rev` 가
  `null`(워킹트리/스테이지 diff, 기존 동작)이면 `DiffPane`, `rev` 가 있으면(커밋 diff)
  `CommitFileDiff`(`renderSideBySide: true` — 이제 패널이 아니라 에디터 영역 전체를 쓰므로 인라인이
  아니라 기본 side-by-side)를 그린다. `DiffPane` 자체는 무변경이다 — hunk stage 거터가 커밋 diff
  에 붙을 여지를 원천 차단하기 위해 두 렌더 경로를 분리했다(기각안 참조).
  `CommitFileDiff`(`widgets/commit-file-diff/commit-file-diff.tsx`)는 Wave C 때 이미 만들어져 있던
  컴포넌트를 그대로 재사용한다 — 신규 컴포넌트는 없다.
  `git_show_file(rev, path)` / `git_show_file(parentRev, beforePath ?? path)` 두 기존 커맨드를
  그대로 부른다(신규 git 커맨드 없음 — §6).
  file-history 모달(파일 단위 히스토리)의 diff 는 **무변경**이다 — 동일 방식 에디터 탭 승격 여부는
  이번 계약 범위 밖의 후속 별도 결정이다.
