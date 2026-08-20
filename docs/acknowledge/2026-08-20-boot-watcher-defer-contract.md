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

## 4. Phase E 검토 반영 (2026-08-20)

> 검토: 4렌즈(동시성/정확성/설계/계약) + major 적대적 재검증. 발견 20건(major 7·minor 13) →
> 적대적 검증 전건 7 대상 **confirmed 6·refuted 1**. 수정: sonnet+xhigh 단독 Rust fixer(1차
> 세션이 네트워크 오류로 중단 — 산출물 5파일 상속 검증 후 재개), 5파일(lib.rs·state.rs·
> file/capability.rs·git/watch.rs·layout/service.rs).

### 4.1 refuted — concurrency-1 (전역 가드가 대기를 "안 보이던 시간"에서 "보이는 시간"으로 이전)

- claim: 프로젝트당 `begin_mutation` 을 FileIdMap 워크 전 구간 보유해, 부팅 직후 첫 뮤테이션
  커맨드가 최대 한 프로젝트 워크만큼 정지 — "time-to-visible 만 개선, time-to-interactive 는
  불변".
- **반증(적대적 재검증, refuted)**: 부팅 직후 렌더가 실제로 의존하는 읽기 경로(`tree_rows`·
  `git_status`·`git_log`/`git_diff`/`git_gutter`·`file_open`·`layout_get`·
  `file_list_mirrors`/`file_list_untitled_mirrors` 등)는 전부 `begin_mutation` 을 타지 않는
  **무가드** 커맨드다 — 트리·에디터·git 뷰는 attach 진행과 무관하게 즉시 채워진다("인터랙티브
  창에서 멈춤"의 전제 불성립). 인용된 "부팅 직후 호출" 근거 다수(`file_prune_mirrors` 등)도
  fire-and-forget 정리 스윕이라 지연돼도 체감 무응답이 아니다. 결정적으로, 구코드(09688ff)의
  동기 루프는 `app.manage(state)` **이전** 메인 스레드에서 돌아 그 구간엔 `State<AppState>`
  를 받는 커맨드 자체가 실행 불가였다 — 즉 신코드의 "최대 한 프로젝트 워크 정지"는 구코드의
  "전체 프로젝트 워크 동안 완전 불가"보다 **항상 같거나 짧다**(엄격 개선). claim 의 정량
  비교("창은 4초 일찍 뜨지만 그 4초 동안 무응답")는 그 4초가 구코드에선 창도 조작도 없던
  구간이라는 사실을 누락한 비대칭 비교였다.
- 이 문서는 이 반증을 그대로 채택한다 — "체감이 더 나쁠 수 있다"는 claim 의 결론을 인용·수정
  근거로 쓰지 않는다. (design-1 은 이 claim 과 별개 판정으로 confirmed — §4.2 참조)

### 4.2 confirmed — 적용한 수정

- **design-1(major, keep) — 가드 범위 축소(attach 를 build/register 로 분리)**: 4.1 의 반증에도
  불구하고, 가드가 실제로 보호하는 대상은 `state.watchers`/`state.git_watchers` insert 뿐이고
  FileIdMap 워크는 `AppState` 를 전혀 만지지 않는다는 사실은 남는다(별개 판정 — confirmed
  유지). `domain::file::capability::build_watcher_handle`/`register_watcher_handle`,
  `domain::git::watch::build_git_watcher_handle`/`register_git_watcher_handle` 로 분리 — walk
  는 완전 무가드 `spawn_blocking`, 가드는 등록 직전 재검증(`projects.contains_key`·
  `watchers.contains_key`)과 insert 두 줄만 마이크로초 단위로 보유한다
  (`domain::tree::commands::rows_page_from_store` 의 락-밖-빌드/락-안-재검증 선례와 동형,
  Phase E C11-TREE-2). `project_open`(`FileWatcherCapability::attach`)은 `attach_watcher`/
  `register_git_watcher` 그대로 호출해 **무변경** — 이미 `attach_all` 전체를 한
  `begin_mutation` 아래 수행하므로 내부 분리가 그 경로의 가드 보유 시점에 영향을 주지 않는다.
  - **concurrency-4(minor, hot-exit viewState 굶김) 자연 해소로 판정**: 가드 보유가 walk 의
    0.5~3초에서 등록 한 번의 마이크로초로 줄어, `layout_set_view_state` 가 이 가드를 기다릴
    최악 시간도 마이크로초로 준다 — 2200ms 예산을 넘길 여지가 사실상 소멸했다. 별도 수정
    불필요.
  - **concurrency-7(minor, `restore_auxiliary_windows` 와 가드 경합) 자연 해소로 판정**:
    워처 쪽 가드 보유가 마이크로초가 되어, 스케줄러가 워처 태스크를 먼저 `lock()` 시켜도
    보조 창 복원이 실질적으로 지연되지 않는다. 순서를 명시적으로 고정할 필요가 사라졌다.
  - **design-3(minor, T1-H 재해석 근거 누락)·contract-3(minor, architecture.md §2.1 원장
    미등재) — 전제 소멸로 조치 불요 판정**: 두 finding 모두 "가드를 spawn_blocking 전 구간
    보유"가 전제였다. 이 분리 이후 코드에는 **가드 보유 중 `spawn_blocking` 호출이 없다**
    (walk 는 가드 밖, 가드 안은 두 줄의 HashMap 삽입뿐) — `git_commit`/`git_pull` 류의
    "가드 쥔 채 `spawn_blocking`" 예외 패턴 자체를 이 경로가 더 이상 쓰지 않으므로, T1-H
    §1.0 규칙 인용의 정확성도 architecture.md §2.1 원장 등재도 대상이 없다. lib.rs 의 관련
    doc 문단도 이번 수정으로 `git_commit` 선례 인용 문장을 통째로 걷어내고 "build outside
    the guard, register inside it" 문단으로 교체됐다 — 걷어낸 문장이 바로 design-3 가
    지적한 인용이다. **조치**: architecture.md 는 수정하지 않는다(등재할 신규 인스턴스가
    없다는 판정 자체가 조치).
  - **design-5(minor, lib.rs 가 `state.git_watchers` 를 훔쳐봐 attach 성공을 역추론) 해소로
    판정**: `build_git_watcher_handle` 이 이제 `Option<WatcherHandle>` 을 직접 반환한다 —
    lib.rs 는 그 반환값(`git_handle.is_some()`)으로 attach 성공 여부를 판정하고, 더 이상
    `state.git_watchers.read().contains_key(...)` 를 훔쳐보지 않는다. suggestedFix 두 안
    (반환값으로 알려주기 / 도메인에 emit 헬퍼 두기) 중 첫 안이 채택된 형태다 — repo 이지만
    워처 시작이 실패한 경우도 이제 `None` 으로 정확히 구분된다(이전엔 이 구분이 없었다).
- **design-2(major, keep)=concurrency-2(major→minor) — 활성 프로젝트 우선 attach**:
  `projects_pending_watcher_restore` 가 `session: &SessionState` 를 추가로 받아
  `session.active_project` 를 선두, 나머지는 `session.projects` 순서(드리프트로 세션에 없는
  프로젝트는 끝에 append)로 정렬한다. 순수 함수 테스트
  `활성_프로젝트가_session_순서와_무관하게_항상_첫_원소다`·
  `session_projects에_없는_프로젝트도_재부착_대상에서_누락되지_않는다` 로 순서를 고정.
- **concurrency-3(major→minor) — GitRefsChanged 중복 방송 제거**: attach 직후 emit 을
  `GitStatusChanged` 1건으로 줄였다(`GitRefsChanged` 삭제 — 프런트가 둘 다 동일
  `QUERY_KEY.GIT.PROJECT` 무효화로 매핑해 둘째 건은 순수 중복). `.git/index`·`.git/HEAD`
  mtime 비교로 조건화하는 안은 검토 후 기각 — 매 부팅 IO 를 추가하면서도 mtime 을 보존하지
  않는 도구의 변경은 못 잡고, `invalidateQueries` 자체가 비표시 쿼리엔 no-op 이라 절감액이
  작다(코드 doc 에 판단 근거 기록).
  - **concurrency-6(minor, 재열기 프로젝트 중복 attach) 명시 수정**: 가드 재검증에
    `state.watchers.read().contains_key(&project_id)` 를 추가(walk 시작 전 1차, 가드 안에서
    2차) — `project_open` 이 이미 재부착한 프로젝트는 walk 조차 시작하지 않고 스킵한다(합성
    방송도 함께 스킵 — 이미 라이브라 필요 없음).
  - **concurrency-5(minor, 종료 취소 부재) 명시 수정**: `AppState::shutting_down`
    (`AtomicBool`) 신설, `handle_close_requested`(메인 창 close)와 `RunEvent::
    ExitRequested`/`Exit`(Cmd+Q 의 `terminate:` 우회 백스톱) 양쪽에서 `begin_shutdown()`.
    루프가 매 반복 선두에서 `is_shutting_down()` 을 확인해 `break`.
- **correctness-1(major, keep)=contract-1 — FILE.CONTENT attach 공백 보정**:
  `domain::layout::service::open_file_paths`(신규 순수 함수, 테스트 2건)로 이 프로젝트의
  메인+보조 창에 열린 `File` 탭 경로를 모아, 파일 워처 attach 직후 그 경로가 있으면
  `FsChanged { kind: Modified, from_app: false, paths }` 를 1회 합성 발신한다 — **기존
  이벤트 재사용**(신규 커맨드·이벤트·쿼리키 없음). 프런트 `ipc-sync-provider.tsx` 의
  `fsChanged` 핸들러가 각 경로에 대해 이미 `FILE.CONTENT` 를 무효화하므로 별도 프런트 수정
  불필요. §4.3 에 이 판단의 근거인 이벤트→무효화 전수 재실사를 기록한다.
  - **FILE.RAW 는 이번 보정 대상에서 제외 — 실사 결과 기록**: `entities/file/file.query.ts`
    의 `fileRawQueryOptions`(이미지·PDF·스프레드시트 등 `preview-pane.tsx` 전용 — path 는
    같은 `File` 탭 path 집합의 부분집합)는 §4.3 전수 재실사 결과 **어떤 이벤트로도
    무효화되지 않는다** — `fsChanged` 핸들러도 `QUERY_KEY.FILE.CONTENT` 만 invalidate 하고
    `FILE.RAW` 는 건드리지 않는다(코드베이스 전수 grep: `QUERY_KEY.FILE.RAW` 호출부는
    `fileRawQueryOptions` 자신과 테스트뿐). 이는 d-25 가 만든 공백이 아니라 **d-25 이전부터
    있던, 라이브 워처 상태에서도 재현되는 선재 결함**이다(부팅 attach 공백과 무관 — 워처가
    살아 있어도 미리보기 탭은 외부 변경에 반응하지 않는다). 이 계약(attach 공백 보정) 범위로
    끌어들이면 "보정"이 실제로는 아무 이벤트 소비자도 없는 캐시를 무효화하는 죽은 동작이
    되어 오히려 오도한다 — 근본 수정은 `ipc-sync-provider.tsx` 의 `fsChanged` 핸들러에
    `FILE.RAW` invalidate 를 추가하는 **별도 배치**가 맞다. 이번 배치는 발견해 보고만
    한다(범위 밖 기존 결함 — ai-process §6.8).
- **correctness-2(minor) — tree 잔여 위험 서술 정정**: "project_open 의 유계 레이스와 동일
  성격"이라는 등가 판정을 lib.rs doc 에서 제거하고, 실제로는 project_open 에 없는 구간(첫
  `tree_rows` 조회~이 프로젝트 attach 완료, 순차 누적이라 대기열 위치만큼 넓어짐)이라고
  정정 기술.
- **correctness-3(minor) — 계측 정확화**: `started` 를 태스크 본문이 아니라
  `restore_project_watchers` 호출 시점(스폰 직전)으로 이동해 창 표시~attach 완료 구간을
  그대로 재도록 했고, 로그를 `projects=<시도>, attached=<성공>, elapsed_ms=...` 로 분리해
  스킵·실패 건도 시도 수에서 구분되게 했다.
- **design-4(minor)=contract-2 — git/watch.rs 낡은 근거 정정**: "복원 루프가 capability
  레지스트리 등재 전에 돈다"는 타이밍 근거(d-25 로 이제 거짓 — attach 는
  `app.manage(project_capabilities())` 이후 스폰된 태스크에서 돈다)를 제거하고, 시점 무관한
  이유("영속화된 `Project.capabilities` 를 앱이 꺼진 사이의 `.git` 생성/삭제까지 신뢰할 순
  없다")만 남겼다.
- **contract-4(minor) — 이벤트 발신 조건 확장 문서화**: `docs/ipc-contract.md`(`fs:changed`·
  `git:status-changed` 항목)와 `docs/features/git.md` §6 에 이번 부팅 합성 발신 조건과
  "no-op" 서술의 정확한 범위(활성·마운트된 쿼리에만 실비용)를 추가.
- **design-6(minor) — 실기 QA 이월**: `docs/quality-assurance/2026-08-11-qa6-checklist.md`
  에 "d-25 부팅 워처 attach 후절화" 섹션을 신설 — 이 배치가 도입한 세 동작(활성 우선
  attach·git/FILE.CONTENT 합성 보정·종료 중 취소)의 실기 확증 체크리스트.

### 4.3 근거 — `ipc-sync-provider.tsx` 이벤트→무효화 전수 재실사

correctness-1 의 FILE.CONTENT 보정과 FILE.RAW 비보정 판단의 근거. `src/app/providers/
ipc-sync-provider.tsx` 가 이 세 이벤트(`fsChanged`/`gitStatusChanged`/`gitRefsChanged`)의
**유일한** 소비자임을 코드베이스 전수 grep(`useTauriEvent(events.` 전체 호출부)으로 확인했다 —
다른 provider/entity 는 다른 이벤트만 구독한다.

| 이벤트 | 무효화/반영 대상 | 비고 |
|---|---|---|
| `projectListChanged` | `PROJECT.LIST` | |
| `projectOpened` | `PROJECT.DETAIL(id)` | |
| `projectClosed` | `PROJECT_SCOPED_KEYS(id)` 전량 remove + `isQueryKeyUnderProjectRoot` predicate 로 프로젝트 루트 하위 `FILE.CONTENT`/`FILE.RAW` remove + open-with 레지스트리 정리 + LSP 세션 flush | `FILE.RAW` 도 여기선 정리 **되지만**(closing 시 캐시 제거) 그 뒤 다시 열렸을 때 신선하게 채우는 것과는 별개 — 최초 fetch 는 항상 신선하므로 이 경로 자체엔 문제 없음 |
| `projectActivated` | `PROJECT.ALL` | |
| `layoutChanged` | `LAYOUT.DETAIL(projectId)` | `revision` 단조 게이트로 stale 재조회 스킵 |
| `themeChanged` | `THEME.ALL` | |
| `gitStatusChanged` | `GIT.PROJECT(projectId)` | 이번 배치의 부팅 합성 발신 대상 |
| `gitRefsChanged` | `GIT.PROJECT(projectId)` | `gitStatusChanged` 와 완전 동일 무효화 — 부팅 합성은 발신 안 함(§4.2) |
| `settingsChanged` | `SETTINGS.CURRENT`(setQueryData) + `APP_FILE.CONTENT({kind:'settings'})` | |
| `remoteStateChanged` | `REMOTE.STATUS`(setQueryData) | |
| `syncStateChanged` | `SYNC.STATUS`(setQueryData) + `SETTINGS.CURRENT`/`THEME.ALL`/`LOCALE.ALL` | |
| `fsChanged` | 경로별 `FILE.CONTENT(path)` + (self-echo 아니면) 변경 디렉토리의 `TREE.ROWS` | **`FILE.RAW` 는 무효화 안 함** — 이번 배치의 부팅 합성 발신 대상은 `FILE.CONTENT` 뿐이고, 그 이유가 바로 이 행 |

결론: `FILE.CONTENT` 는 `fsChanged` 로 정확히 커버되므로 이번 배치의 합성 발신으로 attach
공백이 닫힌다. `FILE.RAW` 는 애초에 살아있는 워처의 실시간 `fsChanged` 로도 무효화되지
않으므로, 부팅 공백만 골라 보정하는 것은 이 계약의 범위(§1 "attach 지연 구간" 한정)를 벗어난
별개 결함 수정이 된다 — §4.2 의 판단대로 이번 배치는 발견·기록만 한다.
