# 기능 — Git 통합

> FR-F. SCM 패널(changes·commit·push)·diff·그래프. 백엔드 결정은 ADR-0006,
> API 확정 근거는 `docs/research/git2.md`, UI 참조 동작은 `docs/research/vscode-behaviors.md` §1~§7.
> 에디터 쪽 git 표시(gutter·inline blame)는 `editor.md` 가 정본이고 데이터 소스만 여기(git 도메인)다.

## 1. 상태 모델

- Rust git 도메인이 `StatusRow { path, origPath, staged, unstaged, isConflicted }` 로 **정규화해서** 넘긴다
  (비트플래그를 프론트로 넘기지 않음 — research 적용 가이드). 한 파일이 staged·unstaged 를 동시에 가질 수
  있으므로 두 그룹에 각각 나타난다(VSCode 동일).
- 상태 분류: Modified(M)·Added(A)·Deleted(D)·Renamed(R)·Untracked(U)·TypeChange(T)·Conflicted(!).
  Copied(C)는 statuses 로 검출 불가(research 확인) — 표시하지 않는다.
- 상태 문자·색은 `git.*` 테마 토큰만 사용(하드코딩 금지).
- 갱신 경로: watcher 가 `.git/index` 변경 → status invalidate, `.git/HEAD`·`refs/**` 변경 →
  log·branch·ahead/behind invalidate, `.git/objects/**` 는 무시. 워킹트리 변경은 200~300ms debounce 후
  변경 경로 pathspec 부분 갱신 + 주기적 전체 보정(rename 정확도, research 함정 절).

## 2. SCM 패널 (탐색 사이드바의 Git 뷰)

구성(위→아래):

1. **헤더**: 현재 브랜치(클릭 시 브랜치 목록/전환 — 2차), ahead/behind 배지(`graph_ahead_behind`),
   Sync 버튼(pull+push), remote 이름/주소 표시(FR-F1 — remote url 은 `Remote::url()`).
2. **커밋 입력**: 멀티라인 메시지 박스 + Commit 버튼 + 드롭다운.
3. **리소스 그룹**: `Merge Changes`(충돌) / `Staged Changes` / `Changes` — 각 그룹 헤더에 카운트 배지,
   hover 시 Stage All / Unstage All / Discard All 아이콘.
4. **Graph 섹션**(§5).

파일 row:

- 표시: 파일명 + 흐린 디렉토리 경로 + 상태 문자(우측). Renamed 는 `old → new` 툴팁.
- 클릭 = **Open Changes**: 해당 파일의 diff 를 preview 탭으로 연다(§4).
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

## 3. Commit / Push (FR-F5)

- 커밋: 메시지 입력 후 `⌘Enter`(macOS)/`Ctrl+Enter` 또는 Commit 버튼.
  - **커밋 대상 규칙(사용자 확정, 2026-08-06)**: staged 가 있으면 그것만 커밋. 없으면
    "변경 전체를 스테이지하고 커밋할까요?" 확인 다이얼로그 후 진행(현행 VSCode 실동작과 동일).
  - 빈 메시지면 커밋 버튼 비활성. user.name/email 미설정이면(`repo.signature()` 실패) 안내 배너.
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
  - 진행 표시: 장시간 작업(push/pull)은 헤더에 스피너 + 완료/실패 이벤트.
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
- 툴바: inline 토글, Collapse Unchanged Regions(2차), 파일로 이동 버튼.
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

## 6. IPC (상세: `docs/ipc-contract.md`)

- query: `git_status`, `git_diff_file(path, mode)`, `git_diff_staged_text(projectId)`(Wave G,
  staged 0건 시 워킹트리 폴백은 Wave H — 상세는 `ai.md` §7),
  `git_show_file(rev, path)`, `git_log(skip, take)`, `git_refs`, `git_ahead_behind`, `git_remotes`,
  `git_gutter(path)`, `git_blame_range(path, from, to)`, `git_stash_list`
- mutation: `git_stage(paths)`, `git_unstage(paths)`, `git_discard(paths)`, `git_commit(message, opts)`,
  `git_push`, `git_pull`, `git_fetch`, `git_stash_push/apply/pop/drop`, `git_undo_last_commit`
- event: `git:status-changed`, `git:refs-changed`, `git:operation-progress`, `git:operation-finished`

## 7. 수명주기 · 성능

- git capability attach 시: discover(`.git` 탐색) → 초기 status → watcher 필터 등록.
  detach 시 대칭 해제. bare repo(workdir 없음)면 SCM 뷰 비활성.
- blame·log·대형 diff 는 `spawn_blocking` + 경로로 새 `Repository::open` (ADR-0006 스레드 모델).
- status 옵션 고정: `recurse_untracked_dirs(false)`, `include_ignored(false)`,
  `exclude_submodules(true)`(서브모듈은 2차), `renames_*(true)`, `update_index(true)`.
- 대형 리포: Rust 가 `HashMap<Path, StatusRow>` 캐시를 갖고 **변경분만** 이벤트로 emit
  (전체 배열 반복 직렬화 금지 — research 성능 절).

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
