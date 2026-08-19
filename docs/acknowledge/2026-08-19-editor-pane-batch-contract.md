# editor-pane 묶음 배치 — 분해 + T1-C/D 잔여 해소 계약 (2026-08-19)

> 정본: T1 3차 계약 `2026-08-19-audit-t1-batch3-contract.md` §5.1-1(이월 1순위) + 감사
> `2026-08-18-architecture-audit.md` C7("editor-pane 은 결함 생성기 — 분해를 T1 로 올릴 가치",
> §6-4)·T2-B. 사용자 승인(2026-08-19): "전부 추천안대로" — prod 병합(main=46d5504) + 본 배치 착수.
> 착수 전 메인 실물 확인: editor-pane.tsx 1087줄 / blame effect 2종(:849·:877)+conflict(:563) /
> 직접 무효화 editor-pane 5곳(:265·:480·:481·:505·:548-549)+untitled-pane 3곳(:84·:121·:122) /
> 저수준 구독 use-lsp-session.ts:92·shared/hooks/use-global-keymap.ts:35 / root-agnostic LSP 소비
> 4파일(editor-pane:350·:395, breadcrumbs-bar, outline-panel-container:43, command-palette:251).

## 1. 범위

### 1.1 Phase A — editor-pane.tsx 기계적 분해 (T2-B 선행, 동작 무변경)

- 1087줄을 관심사별 훅으로 추출: 감사 C7 이 지목한 8관심사(미러·저장·git gutter·충돌·blame·
  프리뷰·IDE selection·LSP) 기준. `widgets/editor-pane/use-*.ts` 훅 파일 분리(1파일 1책임).
- **동작 무변경이 절대 조건**: 로직 재작성·개선 금지, 이동만. 기존 테스트 전량 그린 + typecheck.
  훅 시그니처는 추출 경계에서 자연 도출(과추상화 금지 — 이 컴포넌트 전용 훅이므로 재사용 설계
  불필요). 컴포넌트 본문 작성 순서(useRef→useState→로직→useEffect) 준수.
- 분해 근거: 감사 §6-4 — F3 critical 2건이 전부 "타이머/등록이 의존성을 갖지 않아서"였고 그
  상류가 이 파일의 관심사 응집. 이후 결함 수정(Phase B)을 훅 단위 diff 로 검토 가능하게 만든다.

### 1.2 Phase B — 잔여 결함 해소 (분해 위에서)

- **F1#17 (blame·conflict effect fetch)**: `getGitBlameRange` 커서 라인(:849)·전체 오버레이(:877)
  effect+setState 를 TanStack Query 로(entities/git/git.query.ts 에 queryOptions 신설 — 커서 라인
  blame 은 고빈도이므로 staleTime·debounce·enabled 게이팅을 실사용 패턴에 맞게 설계, 기존 debounce
  타이머 의미론 보존). `getGitConflictSides`(:563) 동일 원칙.
- **F3#4 (직접 무효화 8곳)**: GIT.PROJECT·FILE.MIRRORS·FILE.CONTENT·FILE.UNTITLED_MIRRORS 직접
  invalidate 를 entities mutation onSuccess 로 회수. 기존 훅(useSaveFile 등) 재사용 우선. 위젯
  오케스트레이션이 자연스러운 경우(query.md 예외)는 판단 근거 기록 후 유지 허용.
- **F3#18 (저수준 구독 2곳)**: use-lsp-session.ts:92 의 SETTINGS 쿼리 해시 감시·use-global-keymap
  .ts:35 의 전 캐시 구독을 표준 경로(useQuery select 구독 등)로 교체. **use-lsp-session 의 외부
  시그니처는 불변**(editor-pane 소비 접점 보호).
- **root-aware 전환 (T1 3차에서 API 만 신설, 소비처 5곳 미전환)**: editor-pane
  notifyLspSessionsOfSave(:350 peek)·runCodeActionsOnSave(:395 wait) + breadcrumbs-bar·
  outline-panel-container·command-palette 를 `peek/waitForLspSessionForRoot` 로 전환. root 결정은
  lsp-session-registry 의 acquire 선례(문서 경로의 소속 root)를 따름 — 다중 루트에서 엉뚱한 세션
  참조(didSave·code actions·outline·breadcrumbs·팔레트 '@') end-to-end 해소.

### 1.3 범위 외

- §5.1 잔여 2~8(open-with 생명주기·ws writer·Rust 실패-확인 커맨드·throwaway race·ide 브로드캐스트
  게이트·TREE_ROWS 센티널·경로 키 커버) — 후속. T1 후반(T1-H·T1-I)·X-A·T2 — 기결 트랙.

## 2. 실행 구조

- **Phase A(분해 단독, sonnet+xhigh)**: editor-pane.tsx 훅 추출. 소유: widgets/editor-pane/
  editor-pane.tsx + 신설 훅 파일. 완료 조건: 동작 무변경(기존 테스트 그린·typecheck·diff 가 이동
  중심임을 자가 증명).
- **Phase B 병렬 2(sonnet+xhigh, A 후)**:
  - B1 = editor-pane 훅들·untitled-pane·entities/git 쿼리 신설(F1#17·F3#4·root-aware 2곳).
  - B2 = use-lsp-session(내부만)·use-global-keymap·breadcrumbs-bar·outline-panel-container·
    command-palette(F3#18·root-aware 3곳).
- **메인 통합**: 접합부 확인 + verify·vite build 직접 실행(통합 에이전트 생략 — 규모 중).
- **Phase E 검토(별도 Workflow)**: 4렌즈(opus+xhigh — 정확성: 분해 전후 동작 동등성(전이 오류)·
  blame 쿼리화의 타이밍 의미론·root 결정 정합 / 계약: 8관심사 커버·시그니처 불변 / 설계: 훅
  경계 적정선) → 적대적(opus+high) → 수정 → 메인 2차 → 커밋(dev).

## 3. 완료 조건

- `bun run verify` + vite build 그린. 분해 후 기존 테스트 무손실. root-aware 전환 후 다중 루트
  회귀 테스트. blame 쿼리화 회귀 테스트(커서 이동 debounce 의미론).
- 실기 이월(qa6): 다중 루트 프로젝트에서 저장 시 didSave·outline·breadcrumbs 정상 세션 참조·
  blame 표시 지연 체감 무변화.

---

## 4. 구현 완료 기록 (2026-08-19, Phase E 검토 전)

> 구현 wf_f51e26dd-845(A 분해→B 병렬 2, sonnet+xhigh) + 메인 소규모 2차. 메인 재검증: 스팟 체크
> 6건 실물 일치 + `bun run verify`·`bunx vite build` 직접 실행 exit 0(프론트 1330·Rust 1000).

- **Phase A**: editor-pane.tsx 1087→369줄, 훅 6개(markdown-preview·ide-selection·lsp-integration·
  file-persistence·git-gutter-and-conflicts·blame — 실응집 경계 기준, 8개 기계 분할 대신 mirror↔save·
  gutter↔conflict 불가분 확인해 병합). 동작 무변경: 의존성 배열 19개 원본 대조·git stash 대조·기존
  테스트 무손실(1304). syncModelOrPickUpExternalEdit 는 순환 의존 회피로 컴포넌트 잔류(기록).
- **B1**: blame 커서 라인(keepPreviousData·debounce 보존)·오버레이(lineCount 쿼리키 제외)·conflict
  sides(compareRequested 온디맨드) queryOptions 3종 신설. applyConflictResolution 무효화 →
  useResolveGitConflict 회수. root-aware 2곳(resolveLspSessionRootForSave — acquire 선례 재사용).
- **B2**: use-lsp-session 저수준 구독 → QueryObserver(select·enabled:false — 자체 fetch 없음,
  observeSemanticHighlightingSetting 분리로 실 QueryClient 테스트)·use-global-keymap →
  useQuery(skipToken+select — entities 미참조로 FSD 유지). root-aware 3곳(breadcrumbs·outline·
  팔레트 @). ⌘T Workspace Symbol 은 root-agnostic 유지(계약 예외 — 문서 경로 없음).
- **메인 2차**: `useSaveFile(projectId?)` 확장(useRenameEntry 선례) — handleSave GIT.PROJECT+
  FILE.MIRRORS 2곳·handleConvertSuccess GIT.PROJECT 1곳 entities 회수(F3#4 완결). 예외 유지 3곳
  (persistMirror 2·handleViewDisk 1 — fire-and-forget 미러 IPC, 에폭/경합 소유가 위젯. query.md
  위젯 오케스트레이션 예외로 판단 기록).
- **잔여·검토 초점**: buildDocumentSymbolWaiters 3파일 중복(소유 경계 회피 — 2회 이상 룰 위반,
  Phase E 통합 승격 후보) / 훅 렌더 테스트 불가(DOM 환경 부재 — 순수 로직만 커버, 실기 이월) /
  blame 쿼리화의 타이밍 의미론·분해 전이 오류가 검토 최우선.

---

## 5. Phase E 검토 결함 수정 반영 (2026-08-19)

> 검토 wf_8fea2fb4-435(4렌즈: 분해충실성·수정정확성·계약·설계, opus+xhigh — 대상 dev `500dbce`
> diff) — 발견 38(major 12·minor 26) → 적대적 검증(opus+high, major 전건) **confirmed 9·refuted 3**
> → 수정 wf_5ead0fdc-b2c(FA·FB 병렬 sonnet+xhigh). 메인 2차: 스팟 7건 실물 재검증 + verify·vite
> build 직접 재실행.

- **분해 충실성 총평(검토)**: 원본 1087줄 비주석 라인 중 훅에 부재한 54건 전부가 Phase B 의도
  변경분으로 설명·역방향 스캔 신규 로직 0·effect 의존성 배열 20개 전건 일치 — 분해 자체는 충실.
- **[confirmed·3렌즈 수렴] compareRequested 고착** — Compare 실패 시 플래그 영구 true(재시도
  불가·타 파일 자동 fetch/팝업·반복 토스트). → 클릭마다 `refetchCompareSides()` 명시 호출 + path
  전환 리셋에 `setCompareSides(null)` + 에러 토스트 `compareRequested` 게이팅(캐시 에러 리마운트
  토스트 차단 동시 해소).
- **[confirmed] root-aware waiter 소진** — 다른 root 세션 획득이 waiter 를 null 로 소진, 재등록
  없음. → 대기자를 (projectId, serverId, root) 정확 세션키 큐(`waitersBySessionKey`)로 이전 —
  목표 root 획득까지 대기 유지. 다중 root 순차 spawn·cancel 회귀 테스트.
- **[confirmed·2렌즈] buildDocumentSymbolWaiters 3중복** — `shared/lib/lsp/document-symbol-
  session-waiters.ts` 로 승격(TSession 제네릭으로 widgets 역참조 제거 — FSD 안전 확인). 테스트
  3벌→1벌, 컴포넌트 파일의 비컴포넌트 export(SFC 위반)도 해소.
- **[confirmed] 쿼리 키 중앙화** — blame-line·blame-overlay·conflict-sides 를 QUERY_KEY.GIT
  팩토리로. PROJECT_SCOPED_KEYS prefix 커버 테스트 동반.
- **[confirmed] persistence ref 스레딩** — `settleAfterDiskWrite`(에폭 bump+타이머 해제+pending
  해제+dirty 정리) 단일 함수로 캡슐화, ref 3종 반환면 제거.
- **minor 실질 수정 6**: 탭 A→B→A blame 푸터 공백(deps+`isPlaceholderData` 가드 — refuted #10 이
  지적한 placeholder 노출 위험 동시 방어)·오버레이 lineCount state 화(렌더 순수성)+staleTime 0
  (토글마다 재조회 원본 의미론 복원)·IDE selection effect 등록 순서 원본 복원·
  observeSemanticHighlightingSetting 빈 캐시 baseline(가짜 최초 refresh 차단)·root-agnostic
  peek/waitForLspSession dead code 제거(소비처 0 grep 확증)·qa6 실기 이월 절 신설(메인 — 7항목).
- **기각(수정 안 함)**: refuted 3(staleTime 설계 부재 주장·untitled 회계·blame state 재복사) +
  blame-line 60초 캐시 minor — FA 가 백엔드 `blame_range`(디스크 기반 HEAD blame, 미저장 편집
  무관·저장 시 GIT.PROJECT 즉시 무효화)를 독립 확인해 refuted 와 동일 결론 도달.
- **사고 기록**: FA 가 작업 중 `git stash` 오실행 → 즉시 pop 복구. 메인 확증: qa6 추가분·FB
  산출물 전량 생존(스팟 체크·전체 테스트 그린). 이후 해당 파일 무접촉.
