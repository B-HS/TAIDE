# d-44 — 외부 워크트리 변경이 git UI 에 영영 반영되지 않는 설계 공백 수정 계약 (2026-08-27)

> 발견: d-42 재개 실사 ⑤에서 spec 07 이 격리에서도 결정 실패(파일럿 전 기간 클린 통과 0회의
> 실원인). 증거: 메인 프로브 — 외부 `appendFile` 후 백엔드 `git_status` 는 t=0 부터 정답
> (rows=[other.ts modified])·`fs:changed` 이벤트 정상 도달, 그러나 `git:status-changed` 는
> **영원히 미도달** → 패널 캐시(rows=[]) 미무효화("No changes to summarize" 고정).

## 0. 근본 원인 (메인 실증)

- git UI 캐시(`GIT.*`)의 이벤트 무효화 경로는 `git:status-changed`/`git:refs-changed` 뿐이고
  (`ipc-sync-provider.tsx`), 이 이벤트들은 **`.git` 디렉토리 워처**(`domain/git/watch.rs` —
  index/HEAD/refs 만 감시)와 git 뮤테이션 커맨드에서만 방출된다. 파일 워처(`file/capability.rs`)
  는 `fs:changed` 만 방출하며, FE 의 `fs:changed` 핸들러는 FILE/TREE/SEARCH 만 무효화한다.
- 따라서 **`.git` 을 건드리지 않는 워크트리 전용 변경**(외부 에디터·CLI 수정)은 어떤 경로로도
  git 패널·거터를 갱신하지 못한다. 인앱 저장만 `useSaveFile.onSuccess` 의 `GIT.PROJECT`
  무효화로 우회되어 이 공백이 가려져 있었다. `project/commands.rs` 의 복원 주석이 "60s
  staleTime 백스톱"을 전제하나, staleTime 은 재마운트 시에만 작용해 **열린 패널은 무기한
  낡는다**. 실사용 영향: 외부 도구로 파일을 고치는 동안 git 패널·요약 버튼·거터가 현실과 다름.

## 1. 수정 방향 (근본)

- `ipc-sync-provider.tsx` 의 `fs:changed` 핸들러에서, `isSelfEchoWithoutTreeImpact` early-return
  **아래**(= fromApp+modified 인앱 저장 에코 제외 — 그 경우는 `useSaveFile` 이 이미 커버,
  이중 리페치 회피)에 워크트리 변경의 git 영향 축을 무효화:
  - `QUERY_KEY.GIT.STATUS(projectId)` — 변경 목록·요약 버튼.
  - 변경 경로별 `QUERY_KEY.GIT.GUTTER(projectId, path)` · `['git', projectId, 'diff', path]`
    프리픽스(`GIT.DIFF` 의 mode 축 전체) — 열린 파일 거터·디프 뷰.
  - **`GIT.PROJECT` 전폭 무효화는 배제**: log/branches/remotes/stashes/tags 는 워크트리 변경으로
    안 변하고 `.git` 워처가 이미 소유한 축 — 과대 무효화 금지(쿼리 컨벤션).
- 근거 JSDoc(영어): 이 무효화가 `.git` 워처가 못 보는 워크트리 축의 보완이며, early-return
  아래 배치가 인앱 저장 에코 제외 의미임을 기술.
- 범위 외: Rust 이벤트 신설(기존 `fs:changed` 재사용으로 충분 — 신규 표면 금지)·git 패널
  UI 변경. 검증: typecheck/eslint/prettier/bun test + spec 07 격리 반복(실검증은 메인).

## 2. 실행·검토

- 구현 fixer(sonnet+xhigh) → 검토 1렌즈(opus+xhigh, 근본성·회귀 — 축소 근거: 소형 FE 단일
  지점·메인이 이벤트 계측으로 원인 실증·spec 07 반복 실행 검증 부가) → major 시 적대적 →
  메인 2차. 커밋 금지(사용자 /goal 지시).

## 1.1 §1 정정 (검토 반영 — 배치·형태 변경)

- **게이트 위 배치로 변경**(§1 의 "아래" 판단 철회): 전체치환(`useReplaceSearch`) 에코도 같은
  fromApp+modified 로 배제되는데 치환 경로엔 git 무효화가 전무하고, 같은 파일의 FILE.CONTENT
  선례 주석이 바로 그 이유로 게이트 위 배치를 명문화하고 있었다(검토 F1 — major, 메인이
  grep 3건으로 기계 확인). 게이트 위 배치는 타 창의 git 캐시 갱신 공백(F2 — 창별 QueryClient)
  도 함께 닫는다. 비용 = 인앱 저장 시 mutation 무효화와 300ms 디바운스 간격의 중복 리페치 1회.
- **per-path 루프 → 프리디킷 1패스**(F3): 경로당 invalidateQueries 2회는 O(paths×cache) —
  GIT.PROJECT prefix + 순수 헬퍼 프리디킷 1회로 대체(O(paths+cache), `isGitQueryScopeMutable`
  선례 동형). `GIT.DIFF_PATH` 팩토리는 철회(F5 — 신규 표면 0 원칙 복원), DIFF 는 mode 축 전체
  스윕 유지(indexVsHead 포함은 mode 열거 회피의 의도적 소폭 과대 — 비용은 해당 파일 staged
  diff 탭이 열려 있을 때 재조회 1회뿐). 헬퍼는 export + 단위 테스트(F6 — 파일 선례 패턴).
- **상시 비용 명기**(F4): 게이트를 통과하는 워크트리 배치당 `git_status` 리페치 1회(타이틀바가
  상시 구독 — 디바운스 300ms 상한). .gitignore 산출물 폭주·비 git 프로젝트 에러 재왕복은
  부담 확인 시 별건(스킵 조건·트레일링 디바운스 후보).
- **블레임/충돌 사이드 배제는 판단**(F7): BLAME_LINE(커서 키)·CONFLICT_SIDES(클릭 시 명시
  refetch)는 자가 치유. 잔여 = BLAME_OVERLAY 를 켜 둔 채 외부 편집이 오는 구간의 낡은 주석 —
  소폭·부분적이라 제외, 기록만.

## 3. 기록

- **구현**: wf_b6408dc9 fixer(초판 — STATUS+per-path GUTTER/DIFF_PATH) → 검토 1렌즈
  (opus+xhigh, wf_9261a4fe) **major 1**(F1 배제 과대 — 치환 에코 무보완·선례 상충)·minor 4
  (F2 타 창 갭·F3 비용·F4 상시 비용 명기·F6 헬퍼 추출)·info 2(F5 DIFF_PATH 과대·F7 블레임
  배제 기록) → 판정 §1.1 → wf_7d91f8bd refixer 재구성. 적대적 생략 근거: F1 증거 3건
  (self_writes.mark·useReplaceSearch 무 onSuccess·선례 주석)을 메인이 grep 기계 재현 +
  수정 방향이 파일 내 선례의 명문 그대로.
- **검증**: refixer 사슬(typecheck/eslint/prettier/bun test) + 메인 spec 07 격리 재실행 +
  전 스위트(§ 파일럿 보고서 §7). verified_ok 요지: STATUS 단독 충분성(패널·요약·stageAll 이
  전부 rows 파생)·ai-commit 축 무캐시·LOG 등 나머지 축은 .git 파생으로 서로소·경로 정합
  (양측 canonicalize)·status 리페치가 index 를 재기록하지 않아 피드백 루프 없음.
