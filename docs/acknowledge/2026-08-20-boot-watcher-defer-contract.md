# 부팅 워처 attach 후절화 계약 (2026-08-20, d-25)

> 사용자 결정(d-23): 부팅 ~5초 수정 A안 채택("다 추천안으로"). 진단 정본: 광역
> wf_14010876-849 rank 1(high) — 스크래치패드 `incident-broad-diagnosis.json`.
> 확정 병목: `lib.rs setup()` 이 **창 생성 전에 동기로** 복원 프로젝트 전수의
> `attach_watcher`/`attach_git_watcher` 를 수행 — notify-debouncer-full 의
> RecommendedCache(FileIdMap)가 `debouncer.watch` 시 프로젝트 루트 전체(WalkDir·
> node_modules 포함)를 파일별 stat 워크(RESUME 15,320파일 ~0.5-1초·대형 루트는 수 초,
> 프로젝트 수만큼 곱·git 워처가 .git 에 추가 1회). 이벤트용 ignore 필터는 이 워크에 미적용.
> dev 모드 monaco eager 로드(2~4초)는 별개 축(범위 외 — release 에서 자연 소멸).

## 1. 범위

- **워처 attach 를 부팅 임계 경로 밖으로**: setup() 의 복원 프로젝트 워처 attach(파일·git)를
  창 생성·표시를 막지 않는 비동기 경로(spawn — 표시 후)로 이동. `restore_state`(세션·레이아웃
  복원)와 프로젝트 Store 등재는 동기 유지(트리·에디터가 즉시 뜨는 데 필요) — **워처만 후절**.
- **의미 보존 실사(절대 조건)**:
  - attach 지연 구간(부팅~attach 완료)에 발생한 파일 변경은 이벤트로 안 옴 — 기존 코드가
    attach 후/트리 조회 시 재스캔으로 자연 수렴하는지 실사(tree 는 조회 시 빌드 — 확인).
    수렴 안 하는 소비자(git status 갱신 등)가 있으면 attach 완료 시 1회 refresh 로 보정.
  - `project_open`(신규 열기) 경로의 동기 attach 는 **무변경**(사용자 조작 문맥 — 진단 범위
    밖·T1-I capability 순회 의미 보존).
  - 부팅 attach 실패의 기존 처리(log-warn 삼킴)와 동일 수위 유지. AppState 락 문맥(begin_
    mutation 규약 — T1-H) 준수: 후절 태스크의 Store 접근은 기존 attach 경로와 동일 가드 의미.
  - 다중 프로젝트 순차/병렬은 기존 순서 의미 실사 후 결정(순서 의존 없으면 순차 유지가 안전).
- **계측(경량)**: setup 구간·attach 태스크 완료에 기존 로깅 수위로 elapsed 1줄씩(진단
  discriminator — 재발 검증용. TRACE 홍수 금지·info 1~2줄).
- 범위 외: NoCache 전환(rename 경로 의미 변화 — 후속 검토)·dev 모드 monaco lazy 경계·
  reveal 게이트 재설계·워처 ignore 정책 변경.

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독, Rust 전용): 실사 → 이동 → 회귀 테스트(부팅 attach 가
  결국 수행됨·프로젝트 상태 동일 — 기존 테스트 유지+가능 범위 보강) → cargo fmt → verify +
  vite build exit 0 + bindings 무변경.
- Phase E 4렌즈(동시성: 후절 태스크와 사용자 조작(project_open/close·파일 조작) 경합·락 문맥 /
  정확성: 지연 구간 이벤트 공백 수렴·attach 실패 처리 / 설계: spawn 위치·순서 / 계약: 표면
  무변경·문서) → 적대적(major 이상) → confirmed 수정 → 메인 2차 → 커밋 → 병합.
- 실기 확증(사용자): 부팅 체감(창 표시까지)·표시 직후 파일 트리/git 상태 정상·부팅 직후
  외부 파일 변경 반영.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

### 3.1 실사 결과 — 이벤트 공백 수렴 여부

- **tree**: `domain::tree::commands::rows_page_from_store`는 `TreeStore`(`app.manage(TreeStore::default())`,
  매 부팅 빈 맵)에 캐시 미스면 곧바로 `service::ensure_root_loaded`로 디스크를 재스캔해 채운다.
  즉 부팅 후 첫 `tree_rows` 호출은 워처 attach 완료 여부와 무관하게 **호출 시점의 실제 디스크
  상태**를 반환한다 — 자연 수렴. 남는 위험은 "그 첫 조회 이후 ~ 이 프로젝트의 워처 attach 완료
  이전" 구간에만 발생하는 변경으로, 이는 `project_open`의 동기 attach가 이미 감수하던 "워처 시작
  ~ 디바운서 첫 틱" 구간과 같은 성격의 유계(有界) 레이스이며 이번 변경이 새로 만든 위험이 아니다.
  → 보정 불필요.
- **git status/refs**: `domain::git::commands::git_status`는 매 호출마다 `service::status`로
  라이브 계산해 백엔드 캐시가 없다. 하지만 프런트(`entities/git/git.query.ts`의
  `gitStatusQueryOptions`)는 전역 `staleTime`(60s, `app/query-client.ts`)만 걸려 있고,
  `app/providers/ipc-sync-provider.tsx`가 `GitStatusChanged`/`GitRefsChanged` 이벤트로만
  `QUERY_KEY.GIT.PROJECT` 캐시를 무효화한다 — attach 지연 구간에 git 상태가 바뀌고 그 뒤로
  **추가 변경이 없으면** 워처가 살아난 뒤에도 이 캐시는 최대 60초(또는 그 이상, 리페치 트리거가
  없으면 무기한) 정체된다. → **자연 수렴 안 함**, 보정 필요 확정.

### 3.2 순서 결정

- `infra::watcher::start_watch`는 프로젝트마다 **독립된 `Debouncer` 인스턴스**를 새로 만든다
  (공유 상태 없음) — 서로 다른 프로젝트의 attach 사이에 순서 의존이 없음을 코드로 확인했다.
  계약 §1의 "순서 의존 없으면 순차 유지가 안전"에 따라 `restore_project_watchers`는 복원
  프로젝트를 **순차**로 attach한다(병렬 fan-out 안 함) — `restore_auxiliary_windows`처럼 창
  하나하나가 독립적으로 화면에 나타나야 하는 경우와 달리, 워처 attach는 사용자에게 보이는 결과가
  없어 병렬화로 얻는 체감 이득이 없고, 순차 쪽이 `AppState::begin_mutation` 가드를 한 번에 한
  프로젝트분만 쥐어 다른 `project_open`/`project_close` 대기 시간을 최소로 유지한다.
- 락 문맥: `project_open`이 `attach_all`(워처 attach 포함) 전체를 `begin_mutation` 가드 아래
  동기로 수행하는 것과 "동일한 가드 의미"를 유지하기 위해, `restore_project_watchers`도 프로젝트당
  `begin_mutation().await` 가드를 **attach 전체(‵spawn_blocking‵ 포함) 동안** 쥔다 — `git_commit`이
  `git` 서브프로세스 호출을 `spawn_blocking`으로 비동기 리액터 스레드 밖으로 내보내면서도 가드는
  그대로 쥐고 있는 패턴(Phase E T1H-C-02)과 동일 형태다. "락 보유 중 장기 IO 금지"는 이 리액터
  스레드 점유 금지를 뜻하며 — `spawn_blocking`이 `FileIdMap` 워크를 블로킹 스레드 풀로 내보내
  충족한다 — 가드 자체를 짧게 유지하라는 뜻이 아니다(가드가 보호해야 할 대상은 `state.projects`
  스냅샷과 `state.watchers`/`state.git_watchers` 등록 사이의 정합성이며, 그 정합성은 attach 전체
  구간 유지되어야 한다). 가드 획득 직후 `state.projects.read().contains_key(...)` 재확인 한 번으로
  스냅샷~루프 도달 사이에 닫힌 프로젝트를 걸러내고, 이후로는 같은 가드가 동시 `project_open`/
  `project_close`을 완전히 배제하므로 추가 재확인은 불필요하다.

### 3.3 계측 위치

- `lib.rs::run()`의 `.setup()` 진입 시각부터 `Ok(())` 직전까지 — `log::info!("setup 완료:
  elapsed_ms=...")`.
- `restore_project_watchers`가 스폰한 백그라운드 태스크의 시작부터 전 프로젝트 attach 완료까지 —
  `log::info!("복원 프로젝트 워처 attach 완료: projects=..., elapsed_ms=...")`.
- 두 지점 모두 TRACE가 아닌 info 1줄씩이며, 재발 시 "setup 자체가 여전히 느린가" vs "attach가
  후절 이후에도 오래 걸리는가"를 바로 구분할 수 있다.

### 3.4 구현 요약

- `src-tauri/src/lib.rs`: `setup()`의 복원 프로젝트 동기 attach 루프를 제거하고,
  `projects_pending_watcher_restore`(순수 선택 함수, 단위 테스트 추가)로 스냅샷만 동기로 뜬 뒤
  `app.manage(state)` 이후 `restore_project_watchers`를 스폰해 attach를 후절했다.
  `restore_state`/`app.manage(state)`/`restore_auxiliary_windows` 등 나머지 부팅 시퀀스는 무변경.
  git 워처가 실제로 attach된 프로젝트에 한해 attach 직후 `GitStatusChanged`/`GitRefsChanged`를
  1회 방송해 §3.1의 git status 정체를 보정한다. `project_open`/`project_close` 경로는 무변경.
