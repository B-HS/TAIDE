# HANDOFF — 2026-08-19 세션 스냅샷 (전문 QA 착수·손 QA 수정·아키텍처 감사 T0/T1)

> 최종 갱신: 2026-08-19 / dev HEAD: **`45adf9d`** / prod(origin/main): **`682ca6a`**(T0·flaky#1·T1-1차까지 반영,
> T1-2차 `acadace`·문서·flaky#3 는 dev 선행) / 워킹트리 클린(HANDOFF·PROCESS 문서만 미커밋).
> **진행 중 작업 없음**. flaky 3건 전부 근절 완료(§3.3). 다음 세션은 §6 확인 → §8 TODO 착수.
> 이 문서가 세션 인수인계 **단일 진입점**이다. 직전 스냅샷(캠페인 A~I 완료)은 `git show b4e7318:docs/HANDOFF.md`.

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유하고(ADR-0004), view 는 표시 전용. macOS(arm64) 우선.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J → Phase 8 배포(서명·공증) |
| 이전 마일스톤 | 잔여 기능 캠페인 A~I **전량 완료**(직전 세션, prod 반영) |
| 현재 마일스톤 | **전문 QA(d)** — 손 QA 수정 완료 + e2e 하네스 구축 완료 + **아키텍처 전수 감사 T0·T1 정비 진행 중** |
| 직전 작업 | 감사 T1 2차 배치(`acadace`) + PATH env flaky 근본 수정(`45adf9d`) 완료. 세션 핸드오프 준비 |

전문 QA 설계 정본: `docs/acknowledge/2026-08-18-pro-qa-design.md`(e2e 승인·감사 16배치·QA-W0~W7 편성).
감사 정본: `docs/quality-assurance/2026-08-18-architecture-audit.md`(297발견·16클러스터·T0~T2 티어).

## 3. 완료 / 진행 중 / 미착수

### 3.1 이번 세션 완료 (커밋 18건: 0f9898f..1471261)

각 대형 작업 패턴: **정찰/진단(opus·fable) → 하중 주장 메인 실물 재검증 → 계약(acknowledge) → 구현(sonnet+xhigh
Workflow) → 4렌즈 검토(opus+xhigh)+적대적 검증(opus+high) → confirmed 수정 → 메인 2차(실물 재검증+verify+
vite build) → 커밋(dev) → 다음 착수 시 직전 산출물 prod 병합**. 검토가 매번 실결함을 잡음(값어치 입증).

| 작업 | 커밋 | 계약/정본 | 핵심 |
|------|------|-----------|------|
| **문서 정리분 prod 병합** | (`0f9898f` 포함) | — | 직전 세션 문서 커밋 prod 반영 |
| **손 QA 6건** | `b00c192`(수정)·`09e0e0f`(검토) | `2026-08-18-hand-qa-fix-contract.md` | 터미널 링크 외부열기·부트 테마 reveal 게이트·allowed hosts 와일드카드·**LSP 정지 전역 락 4초 해소**(파일 열기 무반응)·peek 트위스티 정렬·커밋 diff 에디터 탭 승격. 검토 major 3(window.open noopener 오탐·원격 라벨 'main'→'remote'·verify 자충) |
| **#12 원격 거부 전환** | `4229020` | hand-qa 계약 §5 | system_open 계열 4커맨드(open_path·reveal_path·open_in_browser·open_app_data_path) 원격 dispatch 거부 |
| **QA-W0 e2e 하네스** | `5f15a84` | `2026-08-18-e2e-harness.md` | Playwright webkit+node·파일럿 12스펙(`e2e/`)·검토 10건 반영(H1 게이트 스펙 하드스톱 등) |
| **아키텍처 감사** | `4cfd158` | `2026-08-18-architecture-audit.md`(+부록 2) | 16배치 opus+xhigh 읽기전용 → 297발견(고유 289·critical 19)·16클러스터·T0 24/T1 11묶음/T2 10묶음/X1 계약 별도 |
| **flaky#1 (실은 데이터 버그)** | `d2204f7` | `bug/2026-08-18-mirror-mtime-serde-json-float-roundtrip.md` | serde_json f64 유손실 파싱 → 미러 복원 시 가짜 conflict 배지. Cargo.toml `float_roundtrip` |
| **감사 T0 24항목** | `eecb493` | `2026-08-18-audit-t0-fix-contract.md` | 보안 5(원격 게이팅)·데이터 손상 4·기능 무력화·자원 회수. 검토 major 3(#15 forbid_directory 롤백·#18 isConflicted 절대경로·flaky#2 kill 폴링) |
| **T1 1차 배치** | `6917978` | `2026-08-18-audit-t1-batch1-contract.md` | T1-E 계약 파리티 테스트·T1-J 자원 회수·T1-B Settings union. 검토 major 1(LSP kill 재사용 PID SIGKILL — 내 R7#9 회귀) |
| **T1 2차 배치** | `acadace` | `2026-08-18-audit-t1-batch2-contract.md` | T1-G 보안 하드닝 8·asset scope 재등록(register_uri_scheme_protocol)·T1-F 레이어 이동 20파일. 검토가 중복→`infra/range_file.rs` 공유 모듈로 근본 해소하며 parse_range 언더플로 패닉(serving.rs 원격 DoS)·m4v MIME 동시 수정 |

**flaky 3건 근절**(세션 내내 "무관 flaky"로 방치돼 온 것을 근본 진단): #1 serde_json(데이터 버그)·#2 kill
테스트 커널 SIGKILL 레이스(유한 폴링)·#3 PATH env 병렬 경합(`45adf9d`). 전부 근절 — verify 게이트 완전 안정.

### 3.2 기준선 (2026-08-19 실측, T1-2차 시점)

- 프론트 테스트 **1195** / Rust **1010 lib** + 6 통합(session_restore) + 17 taide-cli. e2e 스펙 12(파일럿, 미실행).
- `bun run verify` 전체(typecheck→lint→format→bun test→cargo fmt/clippy/test) + `bunx vite build` 그린.
- 신규 승인 의존성(이번 세션): `@playwright/test`@1.62.1(devDep)+webkit(e2e). Rust 는 기존 crate 피처만
  (serde_json `float_roundtrip`·futures-util unfold — 신규 crate 0). **argon2·toml 여전히 미승인.**

### 3.3 진행 중

**없음.** flaky 3건 전부 근절 완료: #1 serde_json float_roundtrip(`d2204f7`, 데이터 버그)·#2 kill 테스트
커널 SIGKILL 레이스(유한 폴링, `eecb493` 포함)·#3 PATH env 병렬 경합(`45adf9d`, `docs/bug/2026-08-19-
lsp-detect-path-env-parallel-race.md`). #3 은 find_in_path 를 PATH 파라미터 주입형으로 리팩터해 전역 env
조작 제거(테스트 set_var 0건). confirms_healthy_restart 는 비한정 `sh` 스폰이 오염 PATH 로 검색 실패한
collateral damage 로 자연 해소. 병렬 6회 소멸 확증·verify 그린. **이후 verify 게이트 완전 안정.**

### 3.4 미착수 — 감사 후속 (우선순위 §8)

1. **T1 3차 배치**: T1-C 서버상태(effect fetch·무효화 책임·캐시 store 전용, 14건)·T1-D 레지스트리 정리
   (bridge 싱글톤·멀티윈도우 격리, **17건 최대 클러스터 C3**)·T1-A 잔여(monaco 키바인딩·키맵 브리지 프로바이더).
2. **T1 후반(고위험)**: T1-H 전역 락 IO 분리(96 호출지점 동시성 위험)·T1-I 도메인 경계 재조립(C13 30엣지·
   순환)·T1-K 원격 기본거부 전환. **착수 시 각각 동시성/순환 위험 재고지**(사용자 결정 — T1 포함이나 신중).
3. **X 트랙**: X-A 계약 배선(살리기 3: cwd-changed·from_app·revision / 지우기: focus-kind-changed·중복 커맨드
   5종 — 판정 승인됨, 구현 미착수). X-B 테스트(T1-E 로 일부 이관됨). X-C 문서 7건.
4. **T2 백로그**(10묶음): 중복 제거·비대 파일 분해·shared/lib 재구조화·접근성·i18n·dead code 등.
5. **T2-E AppError 코드화**(369지점, 한/영 혼재 UX) — **별도 캠페인 권장**(규모).
6. **e2e 파일럿 실행**(d-6): 사용자 준비 필요(§7). **QA-W1 멀티윈도우 사람 실기**(감사 e2e 사각지대).
7. **Phase 8**: 서명·공증(미구성).

### 3.5 알려진 미검증·미해결 (KNOWN ISSUE)

- **캠페인 A~I + 이번 세션 수정 전량 실기 미검증** — 기계 검증(verify·vite build)만. 앱 실행은 사용자만.
  qa6-checklist 에 실기 항목 누적(손 QA 재검·감사 T0/T1 재검·Wave I 멀티윈도우 40항목 등).
- **asset:// 실 webview 라운드트립 미검증**(T1-2차) — `infra/asset_protocol.rs` 의 재등록 핸들러(Range·CSP·
  닫힌 프로젝트 즉시 차단·CORS 생략 무해성)는 코드/단위 테스트로만 검증. WKWebView video/audio Range 탐색은
  **qa6 최우선 실기 항목**. CORS 헤더는 UriSchemeContext 가 window_origin 미노출이라 문서화만.
- **감사 미수정 잔여**: T1 3차·후반·T2·X 트랙(§3.4) 전량이 감사가 확정했으나 미수정. 각 클러스터·티어는
  `architecture-audit.md` §3~§7 이 정본.
- **e2e 파일럿 실측 미완**(하네스 문서 §6): 터미널 WebGL(WKWebView WebGL2 부재 시 마운트 실패 위험)·포트
  발견 신뢰성·Origin 자동 부착·monaco 모듈 재획득 — 앱 켜야 확정.
- **Wave I deferred 4건**(직전 세션): 보조 창 hot-exit flush 미핸드셰이크·restore 정책·팔레트 미마운트·chord
  1단 삼킴. 직전 계약 `2026-08-16-wave-i-shell-workspace-contract.md` §6.

## 4. 의사결정 요약 (상세·기각 대안은 acknowledge 정본)

핵심 결정은 이번 세션 계약 5건에 있다. 요약:

| 영역 | 채택 | 주요 기각/롤백 |
|------|------|----------------|
| 전문 QA(d) | e2e 경로 A(remote 미러+Playwright webkit)·감사 표준 16배치·QA-W0~W7 편성 | e2e 보류·감사 축소 10배치·순수 playwright 라이브러리 |
| 손 QA 터미널 링크 | ⌘·⌥ 겸용+window.open 선시도→IPC 폴백·altClickMovesCursor false | opener 플러그인 개방·url 크레이트 / noopener(항상 null 오탐 — 검토가 잡음) |
| 손 QA 부트 테마 | reveal 게이트 테마+로케일 정합·follow_system_theme 자동 해제 | 창 OS 배경 동적 갱신(backlog) |
| allowed hosts | RFC 6125 와일드카드(`*.` 선두 1레이블·베이스 불포함) | glob 크레이트·베이스 포함 |
| 커밋 diff | TabKind::Diff rev 확장(원계약 §3.2 복귀) | 전용 TabKind::CommitDiff |
| 감사 #12 | system_open 4종 원격 거부(system_open_path 포함) | 3종만 |
| 감사 #15 asset scope | **T0 롤백**(forbid_directory add-only 재오픈 회귀) → **T1-2차 재등록**(register_uri_scheme_protocol+AppState 라이브 조회) | forbid_directory·재이월 유지 |
| Settings 해제 | 빈 문자열=해제 일반화(Option<String> 6필드) | Option<Option<T>> |
| flaky 3건 | 전부 **근본 수정**(데이터 버그·레이스·env 주입) | epsilon 우회·테스트 격리(HACK) |
| T2-E AppError·T1-H 락 | **별도 캠페인/후반**(규모·동시성 위험) | T1 즉시 포함 |

## 5. 사용자 방향성 & 작업 규칙 (직전 세션 계승 + 이번 세션 확인)

### 5.1 운영 방식 (역할 5단)

- 오케스트레이팅·계약·2차 검토 = 메인(**직접 구현 금지**, 예외: 소규모 2차 수정 — 이번 세션 flaky#2 kill
  테스트 폴링·fanout 문서 정정 등). 구현 = **sonnet+xhigh** / 렌즈 검토 = **opus+xhigh**(4렌즈: 계약·정확성·
  보안·설계추상화) / 적대적 검증 = **opus+high** / 정찰·리서치 = **opus+high** / 버그 진단 = **fable+high**.
- 위임은 규모 무관 **Workflow 로만**. 백엔드 Rust 는 한 시점 한 에이전트(단독 소유, 순차 Phase). 프론트는
  파일 소유권 분리 병렬. **에이전트 보고 불신** — 핵심 주장·검토 발견은 메인이 실물(소스·grep·bun/cargo 실행)
  재검증. **적대적 검증 API 에러로 미검증된 건은 메인이 직접 재검증**(이번 세션 X1#9 경합 기각).
- **확인 질문은 추천안 패키지로 묶어** "전부 추천안대로" 한마디로 답할 수 있게. 다음 착수 질문에 **직전 산출물
  prod 병합 여부**를 항상 포함.
- **반쪽 기능 금지** — 끝까지 배선. **검토·검증 단계 축소 금지**(효율보다 완벽). **flaky 를 "무관"으로 방치
  금지** — 근본 진단(이번 세션 3건 전부 실결함으로 판명).

### 5.2 답변·코드 규칙

- 한국어+존댓말·간결·이모지 금지. 검증 안 된 단언 금지. 보고만 하고 멈추지 말 것.
- arrow fn만·반환 타입 명시 금지·**TS any/enum 금지**(단 **Rust enum 은 백엔드 관행 허용**)·주석 금지
  (영어 JSDoc·러스트 doc comment 만)·매직넘버 금지(상수화)·useCallback/useMemo 금지(useEffect 는 외부
  동기화만)·삼항 2중첩 금지·named export·1파일 1컴포넌트·FSD 위→아래·barrel 금지·서버 상태 TanStack Query
  (queryOptions 팩토리+QUERY_KEY 배열 키).
- i18n locale/service.rs 4곳(required·en·ko·ja) 동기 + en⊆required. **IPC 시간 f64·정수 u32.** Rust 후
  **반드시 cargo fmt**. 신규 커맨드 배선 3곳(lib.rs collect_commands·dispatch.rs IMPLEMENTED+match arm·
  bindings.ts 자동생성) + 파리티. **시크릿/비밀번호는 keyring 에만.**

### 5.3 금지 사항

- **에이전트 셸에서 앱 실행 금지**(`bun run tauri dev`·`tauri build`·`cargo run` 은 사용자만).
- HACK·검사기 끄기(#[allow]·@ts-ignore·eslint-disable) 금지. 우회 대신 근본 수정.
- 승인 외 신규 패키지 금지. 승인 누적: reqwest·keyring·sha2·flate2·tar·zip·xz2·axum·@shikijs 4종·
  monaco-editor-core(dev)·ignore·emmet-monaco-es·**@playwright/test(devDep)**. serde_json 은 float_roundtrip
  피처 추가(신규 crate 아님). **argon2·toml 미승인.**
- 각 계약의 기각 대안 재론 금지. 구현 완료분(캠페인 A~I·손 QA·T0·T1 1/2차) 재구현 금지 — 검토·검증 완료.
- git: main=prod, dev=개발, **dev 자동 커밋·푸시 ON**. main 직접 커밋·git add -A·force push·Co-Authored-By·
  .env 금지. 종료 전 `bun run verify` 통과. **prod 병합은 `git branch -f main dev`(무출력)와 `git push origin
  main` 을 분리 실행**(가드 훅이 한 줄 실행을 force 로 오판). **재시도 대기 60초 고정**(지수 백오프 금지).
- 무관한 변경 커밋 분리(flaky 수정은 배치와 별도 커밋). 선별 스테이징(트리 단위 `git add src-tauri/src src docs`
  허용 — Cargo.toml 등 분리 필요분 확인 후).

## 6. 미해결 질문 / 사용자 확인 필요

1. **prod 병합** — T1-2차(`acadace`)+문서+flaky#3(`45adf9d`)를 함께 main 병합할지(prod=682ca6a 이후 dev 선행분).
2. **다음 착수** — T1 3차 배치(T1-C·T1-D) vs T1 후반(T1-H·T1-I 위험) vs 정지. 사용자는 직전에 "T1 2차" 후
   추가 지시 없이 세션 준비로 전환.
3. **원격 라우팅 dispatch() 테스트의 tauri `test` feature**(dev 전용 1줄) 승인 여부 — fixer 가 테이블
   재구성으로 우회했으나 완전한 mock 테스트를 원하면 필요(선택).
4. **e2e 파일럿·QA-W1 실기** — 사용자 준비(§7) 후 실행.

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS(arm64), bun 1.3.x. cargo PATH 밖 — `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"`(zsh 변수명 `path` 금지) |
| 실행 | dev = `bun run tauri dev`(사용자만). 프로덕션 = `bun run tauri build` |
| 검증 | `bun run verify` = typecheck→lint→format→bun test→cargo fmt/clippy/test. bindings 재생성 = `cargo test`. **rust:test 는 `cargo test --workspace`(병렬)** — flaky 3건 근절로 병렬 안정 |
| e2e 파일럿 준비(사용자) | ① `bun run tauri dev` ② 설정 REMOTE: 비밀번호(8자+)·"비밀번호만으로 접속 허용" ON·활성화 ON ③ `export TAIDE_E2E_PASSWORD='<비밀번호>'` ④ `bun run e2e`. 스펙11(원격 게이트) 실패 시 **실제 보안 회귀** — 수동 복구 절차 먼저(`quality-assurance/2026-08-18-e2e-harness.md`) |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` |
| 스크래치패드 | 정찰·검토 원문은 세션 스크래치패드에만 — 요지는 계약·이 문서·PROCESS 에 반영. Workflow 스크립트는 세션 dir 하위 보존 |

## 8. 다음 세션 TODO (우선순위 순)

1. **prod 병합 확인**(§6) — dev 선행분(T1-2차·문서·flaky#3, `45adf9d`)을 main 병합. `git branch -f main dev` + `git push origin main` 분리 실행.
2. **다음 배치 확인**(§6) — T1 3차(T1-C·T1-D) vs T1 후반(T1-H·T1-I 위험) vs 정지. 추천안 패키지로. 확인 전 코드 수정 금지.
3. **감사 T1 3차 배치** — T1-C 서버상태·T1-D 레지스트리(C3 17건, 멀티윈도우 격리 최대 리스크). 계약 작성
   → Workflow. 관련: `src/shared/lib/*-registry.ts`·`*-bridge.ts`(25개)·`src/entities/*.query.ts`.
4. **감사 T1 후반**(고위험) — T1-H 락 IO(`src-tauri/src/state.rs` begin_mutation·96 호출지점)·T1-I 도메인
   경계(`architecture.md:77` 30엣지·layout↔window 순환). **착수 시 위험 재고지.**
5. **e2e 파일럿·QA-W1 실기** — 사용자 준비 후. `e2e/`·`src/widgets/auxiliary-window-shell/`.
6. **X-A 배선·T2 백로그·T2-E AppError 별도 캠페인·Phase 8**.

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | 시간순 체크리스트. 전문 QA 절 d-0~d-9(각 배치 구현·검토·수정 상세). 캠페인 완료분은 history 아카이브 |
| `docs/acknowledge/2026-08-18-pro-qa-design.md` | **전문 QA 설계 정본** — e2e 승인·감사 16배치·QA-W0~W7·티어링 |
| `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md` | 손 QA 6건 계약(§5 Phase E 검토·#12 해소) |
| `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` | 감사 T0 24항목 계약(§6 검토·#15 롤백) |
| `docs/acknowledge/2026-08-18-audit-t1-batch1-contract.md` | T1 1차 계약(§5 검토·fanout 23·kill 회귀) |
| `docs/acknowledge/2026-08-18-audit-t1-batch2-contract.md` | T1 2차 계약(§5 검토·range_file 공유·언더플로) |
| `docs/quality-assurance/2026-08-18-architecture-audit.md` | **아키텍처 감사 정본** — 297발견·C1~C16·T0~T2·X1(+부록: batch-summaries·raw-R4-X1) |
| `docs/quality-assurance/2026-08-18-e2e-harness.md` | e2e 하네스 사용법·격리 규약·시나리오·실측 항목 |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | 실기 QA 마스터(캠페인 A~I + 손 QA 재검 + 감사 T0/T1 재검) |
| `docs/bug/2026-08-18-*.md` | 손 QA(peek·lsp-stop-lock)·감사 T0(fixes·adversarial-fixes)·**mirror-mtime-serde-json**(flaky#1 데이터 버그) |
| `docs/ipc-contract.md` | IPC 커맨드·이벤트·원격 dispatch 정책(원격 거부 20종·CSP/nosniff·nonce) |
| `docs/data-model.md` | 도메인 타입·설정(Settings union·해제 규약)·레이아웃·git 절대경로 |
| `docs/architecture.md`(=ARCHITECTURE.md 없음, 이 파일이 정본) | 전체 구조·§5 레이어 배치 기준(C1 판정)·§6.3 자원 회수·asset 프로토콜 |
| `docs/features/`·`tech-stack.md`·`PRD.md`·`roadmap.md`·`theme-system.md`·`adr/`·`backlog.md` | 기능·스택·요구·순서·테마·ADR·백로그 |
| `docs/research/2026-08-14-*.md` | e2e 경로·VS Code 갭(직전 세션) |

## 10. 복기 신뢰도

- **높음**: 이번 세션 커밋 18건(0f9898f..1471261)이 dev 에 고정. 계약 5건·감사 보고서·bug 5건이 실시간 동기.
  각 배치마다 메인 실물 재검증(§5.1) — 코드-문서 정합성 스팟 체크 통과(e2e 12스펙·T0 deny arm·range_file 실재).
  기준선 수치 세션 말 실측(프론트 1195·Rust 1010).
- **중간**: 감사 T1 3차 이후 미수정
  잔여는 `architecture-audit.md` 가 정본(항목 재론 시 실코드 확인).
- **낮음**: 없음. 정찰·검토 원문(스크래치패드)은 세션 소멸로 접근 불가 — 요지는 계약·이 문서에 반영.
