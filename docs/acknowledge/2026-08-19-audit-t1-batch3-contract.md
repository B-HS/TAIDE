# 아키텍처 감사 T1 정비 — 3차 배치(T1-C·T1-D·T1-A 잔여) 수정 계약 (2026-08-19)

> 감사 보고서 정본: `docs/quality-assurance/2026-08-18-architecture-audit.md`(§6.2 T1 묶음·§3 C3~C5).
> 사용자 결정(2026-08-19): 직전 산출물(T1-2차·flaky#3) prod 병합 완료(main=85ea29b) + T1 3차 착수
> 승인("전부 추천안대로"). 세부 결정 3건(§1.0)도 **전부 추천안 확정**(2026-08-19 2차 확인).
> 착수 전 메인 실물 재확인 13건 전건 일치: settings 무조건 2스코프 무효화(settings.query.ts:13-14)·
> tree 고정 limit(tree.query.ts:11)·fsChanged 반환 폐기+재무효화(ipc-sync-provider.tsx:80-81)·
> projectClosed 3스코프(:28-30)·브리지 12종(shared/lib/*-bridge.ts)·useAgentStateSync 소비처
> app-sidebar 1곳·applyMonacoKeybindingOverrides 호출부 keybindings-editor:166 1곳·ThemeProvider
> effect fetch(theme-provider.tsx:39-40)·테마 프리뷰 setQueryData(theme.query.ts:49)·IdeStore
> current_selection 단일 슬롯(ide/commands.rs:54·127·209)·원격 sink `let _ =`+Ok(())(ws.rs:90-96)·
> 검색 useId(search-panel-container.tsx)·LSP 키 `${projectId}::${serverId}` root 누락
> (widgets/editor-pane/lsp-session-registry.ts:76).

## 1. 3차 배치 범위

### 1.0 세부 결정 (사용자 확인)

| # | 결정 | 채택 |
|---|------|------|
| 1 | R7#6 — LSP 다중 창 구독 실현 vs 제거 확정 | **제거 확정(창별 독립 세션 정본화)**. 실물 확인: find_reusable_entry 가 `channels.contains_key(owner)` 요구(같은 창만 재사용) + SessionEntry.channels 필드 doc 이 창 간 공유 금지 사유(initialize 이중 전송·요청 id 충돌)를 명시 — Wave I 계약 §2.2 의 의도된 설계. 실현은 LSP 클라이언트 멀티플렉싱 재설계급 + 실기 검증 불가. 조치 = ipc-contract.md:693-696 다중 창 구독 서술 정정 + channels 1-슬롯 불변식 doc 명문화(구조 무변경 — lsp_stop owner 로직과 정합) |
| 2 | 캐시 store 전용 2건(F1#10 LSP 설치 진행률·F1#11 테마 프리뷰)의 이관 목적지 | **zustand 미도입 — T1-D 신설 external-store 브리지 팩토리 재사용**(감사 문언 "store 이관"을 기존 브리지 패턴으로 충족. frontend.md §6 사다리상 zustand 실수요 아님) |
| 3 | F6#5 — 원격 고정 라벨 `'remote'` 로 동시 원격 2접속 시 LSP 채널 소유권 충돌 | **범위 외(단일 원격 세션 전제 유지)** — 동시 다중 원격 접속 지원 여부는 T1-K(원격 기본거부 전환)와 함께 후속 결정. 이번 배치는 owner 스코프 계열(R7#8 등)만 처리 |

### 1.1 T1-C 서버 상태 계약 정리 (C5, 14건 — 프론트)

effect fetch 제거·무효화 책임 entities 회수·캐시 store 전용 해소.

- **effect fetch 제거 3**: F1#2 ThemeProvider 의 `listPlugins`/`readPluginGrammar` 직접 호출 →
  entities 쿼리화 / F1#17 git 읽기 4종 effect fetch → query 훅 / F4#6 git tags→useState → query.
- **캐시 store 전용 2**: F1#10 LSP 설치 진행률 → external-store 브리지(결정 2) / F1#11≡R5#10
  테마 프리뷰 `THEME.CURRENT` setQueryData 덮어쓰기 → 프리뷰 전용 external store(서버 캐시 불변).
- **무효화 책임 4**: F1#3 settings 무효화를 patch 필드 조건화(무관 patch 의 THEME/LOCALE 무효화
  제거 — 과소 무효화 회귀 주의, 관련 필드 매핑 명시) / F1#4 projectClosed 캐시 정리를
  `PROJECT_SCOPED_KEYS` 파생 상수로 전량화(현 3/15) / F3#4 위젯 직접 무효화(GIT.PROJECT·
  FILE.CONTENT) entities 회수 / F5#10 remote 무효화 entities 회수.
- **entities 우회 3**: F4#4 openTab raw IPC+setQueryData 수기 복제 3곳 → entities 훅 재사용 /
  F4#5 AI 커밋 메시지·검색 치환 수기 mutation → useMutation 훅 / F3#18 쿼리 캐시 저수준 구독 제거.
- **2차 증분 2**: R4#12 tree 페이지네이션 계약 확정(고정 `TREE_PAGE_LIMIT`+`total` 미사용과
  mutation 의 full_page setQueryData 주입이 "전량↔절단" 사이를 오가는 상태 — 계약 하나로 통일) /
  R4#13 fsChanged N+1 제거(`refreshTreeDir` 반환값 활용 또는 무효화 단일화 — 이중 작업 제거).

### 1.2 T1-D 레지스트리 정리 계약 (C3 잔여, 17건 — 프론트+Rust)

멀티윈도우 격리 최대 리스크 클러스터. T0 해소분(F3#1 editor-instance·F1#1 wait-marker)은 제외.

- **브리지 팩토리 2종 신설**(F6#2+F6#3): fire-and-forget 형 / external-store 형. 12종
  `Set<Listener>` 보일러플레이트 마이그레이션 + 구독자 0 정책(무시 10/보류큐 1/false 1 로 갈린
  현행)을 팩토리 계약으로 명문화. shared/lib 소유.
- **정리 경로 4**: F1#15 reveal-registry pending 만료·정리 / F1#16·F4#18 open-with-registry 정리
  경로(무한 증가 Map) / claude-diff-registry 미해결 탭 종료 잔존 / F6#18 terminal-write-bridge
  큐 상한·TTL·탭 종료 정리.
- **LSP 렌더러 5**: F7#4 codeLens/refresh 전역 브로드캐스트 → 세션 스코프(semanticTokens 선례) /
  F7#5 diagnostics owner `serverId` → 세션 스코프(동일 서버 두 세션 상호 덮어씀) / F7#12
  server-request-handler-registry 해제 동일성 검사(client.ts 선례) / F7#17 useMonacoMarkers
  모듈 로드 전역 구독 → 해제 가능화 / R7#7 세션 키 `${projectId}::${serverId}` 에 root 포함
  (백엔드 멀티루트 기구 활성화 — widgets/editor-pane/lsp-session-registry.ts:76).
- **shiki**(F6#19): 전역 하이라이터 5개 재초기화 경합·monaco 공개 API 몽키패치 정리.
- **Rust 스코프 4**: R6#12 IdeStore.current_selection 창/세션 키(원격이 데스크톱 선택 덮어씀) /
  R6#20 AiRequestStore requestId 전역 공유 스코프화 / R8#3≡R3#1 원격 채널 sink 실패를 `Ok(())`
  로 위장 → 실패 전파(pty·lsp `retain(.is_ok())` 프루닝 불변식 복원, ws.rs:90-96) / R7#8 검색
  세션 키 owner 스코프(useId realm 로컬 → 두 번째 창이 메인 창 검색 취소. lsp_stop owner 선례).
- **R7#1 근본**: LSP 자동 재시작 세션 세대(generation) 이벤트 발행(Rust) + 렌더러
  재핸드셰이크(initialize 재실행)·`lsp:session-status-changed` 소비 신설(X1#2 일부 — T0 #24
  의 Crashed 유지 완화를 근본 대체. QUERY_KEY.LSP.SESSIONS 무효화 지점 0 도 함께 해소).

### 1.3 T1-A 잔여 (C4, 3건 — 프론트)

T0 #9(IDE 프로바이더 승격)와 동일 패턴. 조건부 렌더 위젯의 앱 수명 부수효과를 상시 프로바이더로.

- F4#2 `useAgentStateSync`(현 app-sidebar 단독 — Zen 에서 소실) → 상시 프로바이더.
- F5#1 monaco 키바인딩 오버라이드 적용(현 keybindings-editor 다이얼로그 단독 — 보조 창 전체
  미적용, major 하향 기승인) → 상시 프로바이더(보조 창 포함).
- 키맵 에디터 열기 브리지 구독(동상 — 보조 창 미마운트) → 프로바이더 이동.

## 2. 실행 구조

- **Phase R(Rust 단독, sonnet+xhigh)**: §1.2 Rust 스코프 4 + R7#1 세대 이벤트 발행 + R7#6 문서
  정정분의 doc 명문화. 회귀 테스트(sink 실패 프루닝·IdeStore 키·검색 owner). cargo fmt/clippy/
  test + bindings 재생성.
- **Phase F0(브리지 팩토리 선행, sonnet+xhigh 단독)**: 팩토리 2종 신설 + 12종 마이그레이션
  (F6#2·F6#3) — 후속 병렬이 소비하므로 선행 단독.
- **Phase F 병렬 3(sonnet+xhigh, 파일 소유 분리 — F0 후)**:
  - F1 = T1-C 전량(entities/*.query·providers/ipc-sync·theme-provider·widgets 무효화 회수).
  - F2 = T1-A 3건 + 정리 경로 4(app/providers·shared/lib 레지스트리·widgets 소비처).
  - F3 = LSP 렌더러 5 + shiki + R7#1 재핸드셰이크 소비(widgets/editor-pane/lsp-session-registry·
    shared/lib/lsp·shared/lib/monaco 계열).
- **Phase D 통합(단독)**: 문서(ipc-contract.md:693-696 R7#6 정정·architecture.md C3/C4 소유권
  절·data-model·qa6 T1-3차 재검 절) + 전체 verify + vite build.
- **Phase E 검토(별도 Workflow)**: 4렌즈(opus+xhigh — 정확성: 멀티윈도우 owner/root 키 변경의
  단일 창 무회귀·재핸드셰이크 경합(crash 중 didOpen)·브리지 마이그레이션 동작 동등성 / 계약:
  PROJECT_SCOPED_KEYS 전수성·settings 조건 무효화 과소화 위험 / 보안: IdeStore 원격 격리 /
  설계: 팩토리 추상화 적정선) → 적대적 검증(opus+high) → 수정 → 메인 2차 → 커밋(dev).

## 3. 기각·범위 밖

| 안 | 처리 |
|----|------|
| R7#6 다중 창 LSP 구독 실현 | 기각(결정 1) — 창별 독립 세션 정본화, 문서 정정 |
| zustand 도입(store 이관 목적지) | 기각(결정 2) — external-store 브리지 팩토리 재사용 |
| F6#5 동시 원격 다중 접속 라벨 충돌 | 범위 외(결정 3) — T1-K 와 함께 후속 |
| open-with `'preview'` 도달 불가 제거 | 범위 외 — dead code 는 T2-F |
| T1-H 락 IO·T1-I 도메인 경계(C13)·T1-K 원격 기본거부 | T1 후반 — 착수 시 위험 재고지(기결) |
| X-A 배선·T2 전체·T2-E AppError | 후속 트랙(기결) |

## 4. 완료 조건

- `bun run verify` 전체 + vite build. bindings 재생성 정합·en⊆required(신규 키 시 4곳)·레이어
  방향 위반 0. 신설 회귀 테스트(sink 프루닝·owner/root 키·조건 무효화·페이지네이션) 통과.
- 4렌즈+적대적+메인 2차 통과. 초점: 멀티윈도우 격리 변경이 단일 창 사용을 깨지 않는지·
  재핸드셰이크가 기존 didOpen/didClose 순서와 경합하지 않는지·브리지 팩토리 마이그레이션 12종
  동작 동등성·PROJECT_SCOPED_KEYS 가 실제 프로젝트 스코프 키 전수를 덮는지·settings 조건
  무효화가 THEME/LOCALE 실의존 필드를 놓치지 않는지.
- 실기 이월(qa6): 보조 창 monaco 키바인딩 적용·Zen 에이전트 배지·멀티윈도우 검색 독립·LSP
  크래시 자동 복구 실동작.
