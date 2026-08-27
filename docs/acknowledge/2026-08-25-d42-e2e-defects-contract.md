# d-42 — e2e 파일럿 발견 앱 결함 수정 계약 (2026-08-25)

> 정본: `docs/quality-assurance/2026-08-25-d39-e2e-pilot-run.md`(발견 원자료·재현 기록).
> 사용자 방침: e2e+전문 QA 완주가 Phase 8 진입 조건 — QA 가 찾은 결함 수정은 그 완주의 일부.

## 0. 메인 실사 (2026-08-25)

- **a. Cmd+S appFile 무동작 — 확정**: `editor-area.tsx:150` `getFocusedFileTabId` 가
  `kind.kind === 'file'` 만 통과 → `saveActiveTab`(⌘S)이 appFile(settings.json 탭)에서 no-op.
  재현 100%(파일럿 스펙 10·11 차단).
- **c. project_open ProjectActivated 미방출 — 확정**: `project/commands.rs` 방출 지점은
  `project_close`(:109)·`project_activate`(:130)뿐, `project_open`(:145~)엔 없음. 수정 전
  실사 필수: project_open 이 상태상 활성화를 수행하는지(하면 fanout 누락 결함, 안 하면
  ipc-contract 서술과 하네스 기대 중 어느 쪽이 정본인지 판단·기록).
- b(dirty 표시 미해제)·d(퀵오픈 미확장 폴더)는 파일럿 보고서의 재현 기록 기준 — 구현이
  근본 원인 실사 후 수정(b: `layout_get` 은 dirty:false 인데 프런트 dirty dot 10초+ 잔존 —
  fanout/쿼리 무효화 경로 추적. d: `command-palette.md` §3 "트리 lazy 로딩 의존 금지" 위반 —
  파일 퀵오픈 인덱스 소스 추적).
- e(원격 웹뷰 재탐색 폭주)는 재현 신뢰 낮음 — **수정 금지, 기록만**(Vite dev 재연결 상관 추정).

## 1. 범위

- a·b·c·d 4건의 **근본 수정**(증상 가림 금지). c 는 실사 결과에 따라 코드 수정 또는 문서
  정정 중 근거 있는 쪽(판단 기록 필수). 각 건 가능하면 실패 테스트 선행 재현(버그 수정 원칙)
  — e2e 로만 재현 가능한 건 해당 스펙이 재현 테스트 역할(보류 스펙 01·10·11).
- 범위 외: e(기록만)·e2e 스펙 추가 수정(d-39 몫)·기능 추가.
- 검증: verify 전량 + bindings 실토큰 0(c 에서 이벤트 방출 추가 시 wire 표면 무변경 확인) +
  관련 단위 테스트. e2e 보류 스펙 재실행은 앱 재시작(사용자) 후 별도.

## 2. 실행·검토

- 구현 1 에이전트(sonnet+xhigh, TS+Rust) → 검토 2렌즈(opus+xhigh — ① 근본성·회귀(수정이
  증상 가림이 아닌지·기존 저장/활성화/팔레트 경로 무회귀) ② 경계·컨벤션) → major 적대적 →
  수정 → 메인 2차(verify). 커밋은 사용자 규칙.

---

## 3. 구현 완료 기록 (구현 에이전트가 기록)

> **[중단 기록 — 메인, 2026-08-25 16:5x]** 사용자 지시("한 거까지만 하고 멈춰")로 구현
> 에이전트(wf_7decbb38-479)를 검증 단계 도중 중단. 에이전트 자체 §3 기록은 미작성 — 아래는
> 메인 중단 실사가 정본.

## 3. 중단 시점 실사 (메인 직접, 2026-08-25)

- **구현 상태: 4건 전부 코드 반영된 것으로 관측** — 변경 파일로 역추적: a(⌘S appFile) =
  `focused-editor-tab.ts`(+test) 신설·`editor-area.tsx`·`app-file-pane.tsx` / b(dirty 잔존) =
  `layout-revision.ts`(+test) 신설·`layout.query.ts`(+test)·`ipc-sync-provider` /
  c(ProjectActivated) = `project/commands.rs`·`service.rs`·`dispatch.rs` / d(퀵오픈) =
  `search/{commands,service}.rs`·`search.ipc/query.ts`·`command-palette*`·`lib.rs`·bindings·
  `query-key`·`docs/features/command-palette.md`·`ipc-contract.md`.
- **메인 실측(중단 직후)**: `bun run typecheck` OK · `bun test` **1499 pass/0 fail**(+9) ·
  `cargo build --lib` 클린 · `cargo test --lib` **1104 pass/0 fail**(+5) · clippy(-D warnings,
  --lib) 통과 · `cargo fmt` 1곳 미적용을 메인이 적용 후 `--check` 클린 재확인.
- **미실시(재개 시 잔여)**: ① 구현 상세의 에이전트 기록 부재 — 재개 시 에이전트 원문 로그
  (`subagents/workflows/wf_7decbb38-479/agent-a63798cc203bf69a6.jsonl`) 실사 또는 diff 직접
  정독으로 §3 보완 ② 검토 2렌즈(계약 §2) 미실시 ③ `cargo test --workspace`(통합 테스트
  3+6+17)·`bun run verify` 전체 사슬·vite build 미실측(lib·bun 전량만 실측) ④ bindings 델타
  정밀 대조 미실시(d 의 신규 커맨드 표면 추정 — 재개 시 재생성·대조 필수) ⑤ e2e 보류 6스펙
  재실행(앱 재시작 필요) ⑥ c 의 판단 기록(코드 수정 vs 문서 정정 어느 쪽을 택했는지) 미확인.
- 결론: **워킹트리는 정합·그린 상태로 보존됨. 단 d-42 는 "구현 완료·검토 미실시" 상태이지
  완결이 아니다** — 재개 시 위 ①~⑥ 순서로.

## 3.1 구현 기록 보완 (재개 실사 ① — 메인, 2026-08-27)

> 근거: 에이전트 원문 로그(`3f7e62d1-.../subagents/workflows/wf_7decbb38-479/agent-a63798cc203bf69a6.jsonl`,
> 634줄) 내레이션 35블록 실사 + 미커밋 diff 전량 정독(메인 직접). 구현 순서는 b → a → c → d,
> 중단 지점은 bindings 재생성 직전(검증 단계 도중) — §3 중단 기록과 일치.

- **a (⌘S appFile 무동작) — 짝수정 2건**: ① `focused-editor-tab.ts` 신설 —
  `SAVE_ROUTABLE_TAB_KINDS = {'file','appFile'}` 기반 `resolveSaveRoutableTabId`(순수 함수,
  단위 테스트 6케이스)로 `editor-area.tsx` 의 inline `kind.kind === 'file'` 필터 대체.
  ② `app-file-pane.tsx` 의 `CodeEditor` 에 `registryTabId={tabId}` 추가 — 이것 없이는 kind
  필터를 넓혀도 `getEditorInstance(tabId)` 가 인스턴스를 못 찾아 여전히 no-op(에디터 인스턴스
  레지스트리 미등록). ⌘S → `saveActiveTab` → `getAction('taide.saveFile').run()` →
  `AppFilePane.handleSave` 사슬을 메인이 정적 추적으로 확인. `terminal`/`untitled` 등 나머지
  kind 는 기존 no-op/`handleSaveAs` 경로 유지.
- **b (dirty 점 잔존) — 근본 원인 = mutation 응답 역전**: `setTabDirty({dirty:true})` 와 후속
  `⌘S` 의 `{dirty:false}` 가 각각 독립 IPC 왕복이라 응답 정착 순서가 호출 순서에 비구속 —
  구식 응답이 나중에 도착하면 무조건 `setQueryData` 가 신선한 캐시를 덮어씀. 수정:
  `isStaleLayoutRevision` 을 `ipc-sync-provider` 로컬에서 `shared/lib/layout-revision.ts` 로
  승격(2소비처 공통화 — 기존 테스트도 파일 이동)하고, `layout.query.ts` 에 `applyFreshLayout`
  (revision 비교 후 조건부 `setQueryData`) 신설 — `useLayoutMutation`·`useOpenTabInProject`·
  `useCloseTab` 3곳의 raw `setQueryData` 전부 이 경로로 통일. 재현 테스트
  `layout.query.test.ts`(역전 응답 시나리오) 신설. Rust `ProjectLayout.revision` 은
  `#[serde(default)] u32` 라 응답에 항상 실림(TS 옵셔널은 입력 관용 표기) — `?? 0` 은 안전벨트.
- **c (ProjectActivated 미방출) — 판단: 코드 수정 채택(문서 정정 아님·재개 실사 ⑥ 해소)**:
  근거 = `service::open_project` 가 신규·이력 재사용·`already_open` 3경로 전부에서
  `session.active_project` 를 설정하므로 활성화는 무조건 발생 — 방출만 누락된 fanout 결함.
  `project_open` 말미에 `ProjectActivated` 무조건 방출(첫-오픈 전용 `!already_open` 분기
  밖 — 활성화 자체가 무조건이므로). 전제 고정 테스트(`open_project은_already_open_여부와_
  무관하게...`)를 `service.rs` 에 신설, 판단 근거는 `commands.rs` doc 주석에 영구 기록.
- **d (퀵오픈 미확장 폴더 미발견) — 전용 인덱스 신설**: Rust `service::list_project_files` —
  기존 `build_walk` 재사용(IGNORED_DIR_NAMES 공유·신규 크레이트 없음)·`respect_gitignore:
  false` 고정(트리 표시 집합과 퀵오픈 검색 집합 일치 보장 — doc 주석에 근거). 신규 커맨드
  `search_list_files(projectId) → Vec<String>`(절대 경로·`spawn_blocking`) → `lib.rs` specta 등록·
  `dispatch.rs` 3등재(IMPLEMENTED_JSON_COMMANDS·REMOTE_ALLOWED_COMMANDS·match arm — 형제
  `search_*` 와 동일 허용, 감사 주석 156→157 갱신). Rust 테스트 4케이스(미확장 폴더·
  IGNORED_DIR_NAMES·바이너리 포함·gitignore 포함). FE: `QUERY_KEY.SEARCH.PROJECT_FILES`
  (PROJECT_SCOPED_KEYS 등재+분류 테스트)·`search.ipc.listProjectFiles`·
  `projectFilesQueryOptions` → `command-palette.tsx` 파일 모드 데이터 소스를 `tree_rows` 에서
  교체(`FuzzyRankedItem<TreeRow>` → `<string>`)·`ipc-sync-provider` 가 `fs:changed` 중
  `modified` 제외 3종(created/renamed/removed)에서 무효화. 문서: `command-palette.md` §3
  위반 서술 → 수정 반영, `ipc-contract.md` 총수 178/181·허용 157 현행화.

## 4. 검토 (재개 실사 ② — 2렌즈 opus+xhigh, wf_561c16f9, 2026-08-27)

> 렌즈 1 근본성·회귀 / 렌즈 2 경계·컨벤션. **major 0 · minor 8 · info 5**(렌즈 간 수렴 중복
> 포함) — major 부재로 적대적 생략(다렌즈 수렴+메인 실물 재검증 선례). 원문 JSON:
> `/private/tmp/claude-501/.../tasks/wx4u78t15.output`(세션 소멸 가능 — 요지는 아래가 정본).
> 메인이 핵심 주장 2건(L1-01 untitled dead path·L2-6 배치)을 코드 실물로 재검증 후 판정.

### 4.1 검증 확인(verified_ok 요지)

- a: ⌘S→`saveActiveTab`→레지스트리→`AppFilePane.handleSave` 사슬 정합·타 kind no-op 유지·
  레지스트리 오염 없음. **필터 확장+registryTabId 등록이 짝**이라는 §3.1 서술 코드 일치.
- b: revision 은 응답에 항상 실림(serde default u32·skip 없음 — `?? 0` 안전벨트)·모든 layout
  변경 경로가 `revision += 1` 수행(동률=동일 상태 전제 성립)·`LAYOUT.DETAIL` raw `setQueryData`
  잔존 0·이벤트/뮤테이션 경로가 동일 규칙 공유.
- c: 방출 페이로드가 `project_activate` 와 동형·FE 핸들러 멱등(payload 무시, PROJECT.ALL
  무효화)·원격 dispatch 동일 fanout·부팅 복원은 별경로라 기동 방출 폭주 없음.
- d: tree_rows 잔존 참조 0·`FsChangeKind` 4종과 부정형 조건 정합·심볼릭 루프 없음(follow_links
  false)·트리 표시 집합의 상위집합 성립. dispatch 3등재 상호 일관(178/157 실측 재확인)·bindings
  파리티 178=178·문서 수치 일치·주석/네이밍/FSD/쿼리 규약 전건 통과.
- e2e: 단언 약화 없음 재확인(01 오라클 유지·06 은 glob_match 실의미 반영·08 순서 교정·09 결정적
  재작성·10/11 입력 수단만 교체). 관련 bun 테스트 41/0 실측.

### 4.2 판정 표 (메인 확정)

| ID | 심각도 | 요지 | 판정 |
|----|--------|------|------|
| L1-01 | minor | `focused-editor-tab.ts` JSDoc 이 untitled ⌘S 를 "handleSaveAs 라우팅" 으로 오서술(실은 도달 불가 dead path — 동일 원인 별건 결함) | **수용 F1**(서술 정정) + 별건 후보 **C6 기록**(파일럿 보고서 §4 — 수정은 범위 외, 보류가 잘못된 수정보다 낫다) |
| L1-02 | minor | 팔레트 파일 모드 첫 콜드 페치 동안 "결과 없음" 오표시(전엔 트리 캐시가 warm) | **수용 F3**(isPending 로딩 게이트 합류) |
| L1-03 | minor | 전 파일 fuzzy 전량 스캔의 대형 프로젝트 성능(추정·실측 없음) | **이월 기록**(§5) — 실측 없는 선제 최적화 배제(React Compiler 메모이제이션이 닫힘 상태 재계산을 상쇄, FE 표시 상한은 기존 `FILE_RESULT_LIMIT`). 실기 QA 에서 대형 프로젝트 체감 확인 후 판단 |
| L1-04/L2-1/L2-2 | minor(수렴) | e2e `fixture-project.ts` 낡은 JSDoc 2곳 + 잉여 `project_activate` 가 c 회귀 검증을 가림 | **수용 F7**(호출 제거 — heading waitFor 가 c 오라클로 승격·JSDoc 2곳 현행화·평탄 픽스처는 유지) |
| L1-05/L2-2 | minor(수렴) | d(미확장 폴더 퀵오픈)의 e2e 커버리지 0 | **수용 F7④+F8**(중첩 픽스처 `src/nested-only.ts` 추가+신규 스펙 13 — 실행은 ⑤ 재실행 시) |
| L1-06 | info | `code-editor.tsx` registryTabId JSDoc 낡음 | **수용 F2** |
| L1-07 | info | 재조회(refetch) 경로는 revision 무가드 — 단 재유입 창이 극협·자가 치유 | **기록만**(이 줄이 그 기록) — layout:changed 후속 이벤트+cancelRefetch 로 닫힘, 조회 경로 가드는 TanStack 계약 비틀기라 배제 |
| L1-08 | info | 레지스트리 등록 부수효과: 상태바 Ln/Col·모나코 액션 게이팅이 appFile 탭에도 적용 | **기록만**(의도된 확장 — 액션 집합은 인스턴스 `getSupportedActions()` 파생이라 file 전용 액션 유입 불가. QA 회귀 오인 방지용 기록) |
| L2-3 | minor | `search_list_files` 무상한 payload — 상한 규약 부재 | **문서 명시로 확정**(ipc-contract.md 신규 항목에 무상한=의도 규약 추가 — 상한은 상한 밖 파일 검색 불가로 결함 d 재도입이라 기각, 표시 상한은 FE 몫) |
| L2-4 | minor | `fileRows` 명칭 부정확(string[])·잉여 람다 | **수용 F3** |
| L2-5 | minor | `getFocusedFileTabId` 등 호출부 이름이 file 전용 시절 그대로 | **수용 F5**(SaveRoutable 계열로 파일 내 일관 개명) |
| L2-6 | info | SEARCH.PROJECT_FILES 무효화가 self-echo early-return 아래라 주석과 제어 흐름 불일치(현재는 결과 동일) | **수용 F6**(게이트 위로 이동+주석 정정 — 향후 게이트 확대 시 조용한 누락 방지) |
| L2-7 | info | `projectFilesQueryOptions` 가 null 미허용 → 호출부 `?? ''` | **수용 F4**(null 허용+enabled 내장 — 대체된 treeRowsQueryOptions 관용 복원) |
| L2-8 | info | e2e `'Meta+V'` 리터럴(KEY_CHORD 이탈) | **수용 F9** |

- 수정 실행: wf_526f8bd3 fixer(sonnet+xhigh, TS/e2e 만·Rust 무접촉) — 결과는 §4.3.

### 4.3 수정 반영 (wf_526f8bd3 적용 9/9 · 메인 실물 재검증, 2026-08-27)

- F1~F9 전건 적용·skip 0. 메인이 diff 실물 재확인: F6 무효화가 self-echo 게이트 위(:215)로
  이동·F7 잉여 `project_activate` 제거(+heading waitFor :119 유지 = c 오라클)·F5 개명 3종
  전 사용처 일관·F8 스펙 13 신설(탭+에디터 내용 2중 단언)·F4 팩토리 null 허용+enabled 내장.
  동반 수정 1건(범위 내): F2 로 `code-editor.tsx` 가 43파일 목록 밖에서 신규 변경됨(JSDoc 만).
- **메인 2차 검증(직접 재실행)**: `typecheck` OK · e2e tsc OK · eslint **0 error**(경고 6은
  기왕 파일의 react-hooks 계열 — 무관) · prettier OK · `bun test` **1499/0** · `vite build`
  exit 0. Rust 는 fixer 무접촉(diff stat 불변)이라 §재개 ③ 의 workspace 그린이 유효.
- 스펙 13 실행은 Playwright 몫 — 재개 실사 ⑤(앱 재시작 후 e2e)에서 검증.

## 5. 이월 (Phase 8 전 처리 여부는 사용자 결정 대기)

- **L1-03 팔레트 전량 fuzzy 성능**: 후보 조치 — 상대경로 1회 파생 캐시 / fuzzyFilter 조기
  컷오프 / ASCII fast path. 실기 QA(대형 프로젝트)에서 입력 지연 체감 시에만 착수.
- **C6 untitled ⌘S 무동작**(파일럿 보고서 §4-C6, L1-01 파생): 후보 조치 —
  `untitled-pane` `registryTabId` 전달 + `SAVE_ROUTABLE_TAB_KINDS` 에 `'untitled'` 편입 +
  Save As 다이얼로그 실동작 확인. a 와 동일 계열이나 계약 §1 범위 외라 기록만.
