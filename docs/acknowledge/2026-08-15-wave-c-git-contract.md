# Wave C 구현 계약 — Git 확장 (2026-08-15)

> 정찰 wf_563a88b2-20f(opus+high 3축: Rust git 도메인·프론트 git UI·git2 API, 축2는 재시도 후 성공).
> git 커맨드 27종·git2+일부 CLI 하이브리드·전역 mutation 락·파리티 3곳 배선을 코드로 확인.
> 캠페인 계약: 2026-08-14-remaining-features-pro-qa-plan.md(완벽 우선·4렌즈·역할 상향).

## 1. 사용자 결정 (2026-08-15)

| # | 결정 | 선택 |
|---|------|------|
| ① 3-way 머지 UI | 방식 | **인라인 충돌 데코레이터**(일반 에디터 위 Accept Current/Incoming/Both, VS Code 구형·비용 최소). 충돌 사이드는 index :1/:2/:3 스테이지 추출. 해결 후 자동 stage |
| ② hunk stage | 구현·범위 | **git2 native(Repository::apply, ApplyLocation::Index) + 선택 라인 단위까지**(patch 합성). stage/unstage 대칭 |
| ③④⑦ 뷰 배치 | 위치 | **경량 배치** — 커밋 상세=git 패널 인라인 확장·파일 히스토리=사이드 패널·blame=에디터 거터 오버레이 토글. diff 는 기존 diff 탭 재사용, 새 TabKind 신설 회피 |
| ⑤⑥ 정책·팔레트 | 패키지 | **추천** — revert=git2 native(충돌 시 ① 연계)·tag=annotated 기본+lightweight 옵션·원격 checkout=로컬 추적 브랜치+upstream·팔레트 GIT 카테고리 신설+그래프 우클릭 |

## 2. 확정 사실 (정찰·메인 확인)

- git 도메인 5파일(commands/service/types/mod/watch), compose 없음(함수 직접 호출). 커맨드 27종.
  대부분 git2(libgit2), commit/push/pull/fetch/init/undo_last_commit 6종만 `run_git()` CLI shell-out.
- 쓰기 커맨드는 전역 `begin_mutation().await`(tokio::Mutex 단일 가드) 직렬화. 무거운 조회(diff/log/
  blame/discard_hunk)는 spawn_blocking. Repository 는 매 호출 open(repo_root 만 GitStore 캐시).
- 신규 커맨드 배선 3곳: lib.rs collect_commands · dispatch.rs IMPLEMENTED_JSON_COMMANDS 배열+match arm
  · specta bindings.ts(자동생성). 파리티 테스트가 이름 집합 완전 일치 강제(dispatch.rs:731-749).
  dispatch match 는 `arg!(args,"camelCaseKey")`. **IPC 시간 f64, 정수 인자 u32.**
- watch.rs 가 .git 변경 분류 → GitStatusChanged/GitRefsChanged 이벤트. 신규 쓰기 커맨드도
  emit_status_changed/emit_refs_changed 호출 필요.
- 대상 7기능 현황: ①3-way backend 전무(is_conflicted 불리언만·index.conflicts 미사용, 프론트 mergeChanges
  일반 diff 표시만) / ②hunk discard 만(stage 대칭 없음) / ③git_log 에 파일목록·diff 없음(git_show_file
  은 있으나 프론트 미소비, 그래프 드릴다운 0) / ④revwalk 에 pathspec 필터 없음 / ⑤revert·tag 전무
  (tag_names 는 ref 표시용만) / ⑥git_branches 는 원격 나열하나 checkout 시 detached HEAD(로컬 추적 브랜치
  생성 없음) / ⑦blame_range 임의 범위 지원(from=to=커서만 사용, 전체 뷰 없음).
- 프론트: entities/git(ipc·query) + widgets/git-panel(패널·commit-graph) + features/git(diff-view·
  branch-switcher·hunk-discard-dialog) + widgets/diff-pane(monaco 2-way). 재사용: DiffView·거터 데코+
  onMouseDown·formatBlameLine·useGitMutation 무효화(QUERY_KEY.GIT.PROJECT coarse). **monaco 3-way 없음
  (createDiffEditor 2-way 전용) → ① 자작 확정.** 커맨드 팔레트에 git 카테고리 전무.

## 3. 확정 설계

### 3.1 백엔드 (Rust 단일 소유 — 한 시점 한 에이전트)

신규 커맨드(전부 begin_mutation 쓰기는 락+emit, 조회는 spawn_blocking):
- **③-way**: `git_conflict_sides(projectId, path)` → index conflict entry 의 stage 1/2/3 blob
  (base/ours/theirs, read_index_blob stage 파라미터화) + workdir 현재 내용. `git_resolve_conflict(projectId,
  path, content)` → workdir write + index.add_path(충돌 자동 해소·stage 0). git2 index.conflicts()·
  conflict_get·get_path. base 없는 add/add 는 base=None 반환.
- **②hunk/line stage**: `git_stage_hunk(projectId, path, hunkStart, hunkEnd)` + `git_stage_lines(projectId,
  path, lineStart, lineEnd)` + unstage 대칭 2종. git2 diff_index_to_workdir → Repository::apply(&diff,
  ApplyLocation::Index, hunk_callback 로 대상 hunk 선별). 선택 라인은 Patch::to_buf → 선택 라인만 남기고
  hunk 헤더 재계산 → Diff::from_buffer → apply. unstage 는 diff_tree_to_index + reverse. 기존
  discard_hunk 도 같은 apply 방식으로 일관화 검토(회귀 위험 시 보고만).
- **③commit detail**: `git_commit_files(projectId, rev)` → 부모 tree vs 커밋 tree diff_tree_to_tree →
  변경 파일+GitChangeKind(collect_status_rows kind 매핑 재사용). 병합 커밋은 first-parent 기준. 초기
  커밋(부모 없음)은 empty tree 대비. per-file diff 는 기존 git_show_file(parent)/git_show_file(rev) 재사용.
- **④file history**: `git_file_log(projectId, path, skip, take)` → revwalk + 각 커밋 부모 diff pathspec=path
  필터(LogEntry 재사용, 페이지네이션). rename 추적(--follow)은 이번 제외(git2 revwalk 필터 방식).
- **⑤revert/tag**: `git_revert_commit(projectId, rev)`(git2 Repository::revert — 충돌 시 conflicted 상태로
  ① 흐름 연계) · `git_tag_create(projectId, name, target, message?, annotated)`(annotated 기본, message
  없으면 lightweight) · `git_tag_delete(projectId, name)` · `git_tags(projectId)`.
- **⑥원격 checkout**: `git_checkout_remote_branch(projectId, remoteRef)` → 로컬 추적 브랜치 생성
  (origin/foo→foo) + set_upstream + checkout. 동명 로컬 브랜치 존재 시 그냥 checkout(에러 아님).
  기존 branch_create·checkout_ref 재사용.
- **⑦blame**: 신규 없음 — 기존 git_blame_range(1, lineCount) 재사용. 편의용 얇은 커맨드 불필요.
- 신규 커맨드 배선 3곳 전부 + 파리티 테스트 통과. camelCase 키·f64 시간·u32 정수.

### 3.2 프론트 (경량 배치 — 기능별 파일 소유 분리)

- **① 인라인 충돌 데코레이터**(features/git + editor-pane): 충돌 파일 열 때 conflict marker(`<<<<<<<`/
  `=======`/`>>>>>>>`) 파싱해 블록별 monaco 데코레이션 + Accept Current/Incoming/Both/Compare 액션
  (content widget 또는 거터). git_conflict_sides 로 3버전 확보. 해결 시 git_resolve_conflict(자동 stage).
  git-panel mergeChanges 그룹 클릭 → 충돌 파일 열기 진입점.
- **② hunk/line stage UI**(editor-pane 거터): 기존 거터 데코+onMouseDown(hunk discard) 패턴 확장 —
  stage 액션 추가(거터 클릭/컨텍스트). 선택 라인은 onDidChangeCursorSelection 범위. diff-pane 거터에도.
- **③ 커밋 상세**(git-panel·commit-graph): commit-graph 행 onClick → git 패널 인라인 변경 파일 목록
  확장, 파일 클릭 → 기존 diff 탭(rev vs parent, git_show_file). QUERY_KEY.GIT.COMMIT_FILES(rev).
- **④ 파일 히스토리**(신규 widget·사이드 패널): 탐색기 우클릭 + 탭 우클릭 + 팔레트 진입 → git_file_log
  → 커밋 리스트, 클릭 → 해당 파일 diff. QUERY_KEY.GIT.FILE_LOG(path).
- **⑤⑥ revert/tag/checkout**(features/git·commit-graph·branch-switcher): 그래프 우클릭 메뉴(Revert/Create
  Tag/Delete Tag) + branch-switcher 원격 브랜치 checkout 액션(UI 거의 완성). 팔레트 GIT 카테고리 신설
  (KEYMAP_CATEGORY.GIT) + bridge 이미터로 컴포넌트 훅 도달(revert/tag/blame 토글/파일히스토리 진입).
- **⑦ blame 오버레이**(editor-pane): 전체 파일 blame 토글 → git_blame_range(1, lineCount) → 거터 라인별
  커밋·author 오버레이(formatBlameLine 재사용). 팔레트/탭 컨텍스트 토글.
- i18n: locale/service.rs 신규 문자열(충돌 액션·커밋 상세·파일 히스토리·revert/tag/checkout·blame 토글·
  GIT 카테고리) 4곳 동기(required·en·ko·ja) + en⊆required.

### 3.3 실행 구조

- **Phase A 백엔드(sonnet+xhigh, Rust 단독)**: §3.1 커맨드 전량 + git types 신규 + locale 신규 키(4곳) +
  배선 3곳 + 파리티 + bindings 재생성 + cargo fmt/clippy/test. 컴파일·파리티 그린 종료. exports 로
  커맨드 시그니처·타입명·locale 키를 프론트에 전달.
- **Phase B 프론트 병렬 3(sonnet+xhigh, 파일 소유 분리)**: B1=①충돌 데코레이터+②hunk/line stage
  (editor-pane·features/git 충돌·거터) / B2=③커밋 상세+④파일 히스토리(git-panel·commit-graph·신규
  history widget·entities/git query) / B3=⑤⑥⑦ revert·tag·원격 checkout·blame 오버레이+팔레트 GIT
  카테고리(features/git·branch-switcher·editor-pane blame·command 카탈로그·bridge). QUERY_KEY.GIT 확장은
  B2 가 소유(키 추가), B1·B3 은 사용만.
- **Phase C 검토**: 4렌즈(계약·정확성·보안(git apply 경로·충돌 해결 원자성·revert 파괴성)·설계, opus+xhigh)
  → 적대적 검증(opus+high) → 수정(sonnet+xhigh) → 메인 2차(verify 전체+vite build+하중 재검증) → 커밋.

## 4. 기각·보류

| 안 | 처리 |
|----|------|
| 3패널 merge editor 완전 자작 | 기각 — 인라인 데코레이터 채택(비용 대비 P0 충족) |
| hunk stage CLI apply --cached | 기각 — git2 native apply(원자적·의존 없음) 채택 |
| 신규 TabKind(커밋상세/히스토리/blame 전용 탭) | 기각 — 경량 배치(패널·오버레이·기존 diff 탭 재사용) |
| 파일 히스토리 rename 추적(--follow) | 보류 — 이번 revwalk pathspec 필터만 |
| 파일 히스토리에 로컬 저장 이벤트(비-git) | 보류 — git 커밋 히스토리만 |
| discard_hunk apply 방식 리팩터 | 조건부 — 회귀 위험 없으면 일관화, 있으면 보고만 |

## 5. 완료 조건

- `bun run verify` 전체 + vite build. locale 4곳·en⊆required·파리티(신규 커맨드 반영).
- 4렌즈+적대적 검증+메인 2차 통과. 초점: hunk/line stage 의 라인 오프셋·patch 합성 정확성, 충돌 해결
  index 조작 원자성, revert 파괴성·충돌 연계, 원격 checkout 동명 브랜치, commit_files 병합/초기 커밋.
- 문서: features/git(신규 커맨드·3-way·hunk stage·커밋 상세·파일 히스토리·blame)·ipc-contract(신규
  커맨드)·qa6-checklist Wave C 실기 항목. 갭 분석 §5 항목 종결 표기.
