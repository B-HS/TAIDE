# 전문 QA(d) 설계 확정 (2026-08-18)

> 정찰 wf_7e69c774-358(opus+high 4축: qa6 분류·아키텍처 감사 인벤토리·기능 우선순위·e2e 하네스).
> 하중 주장 16건 메인 실물 재검증 전건 일치. 사용자 결정 4건 **전부 추천안**(2026-08-18):
> ① 손 QA 6건 수정 선행(`2026-08-18-hand-qa-fix-contract.md`) ② e2e 의존성 승인 ③ 감사 표준
> 16에이전트 ④ QA-W0~W7 편성안. 캠페인 정본: `2026-08-14-remaining-features-pro-qa-plan.md`.

## 1. e2e 의존성 승인 (신규 — 승인 목록 갱신)

- **`@playwright/test@1.62.1`**(devDependency — npm 실조회 확정) + webkit 바이너리 1종
  (`bunx playwright install webkit`, 약 180MB, `~/Library/Caches/ms-playwright/`).
- 러너는 **node**(bun 러너 미지원 — 공식 not planned), bun 은 설치 전용. 스펙 파일명
  **`*.e2e.ts`**(bun test 가 `*.spec.ts` 를 수집 — verify 충돌 회피).
- 통합 비용 3건 한정: package.json 스크립트(e2e·e2e:ui·typecheck:e2e)·`e2e/tsconfig.json`
  (node 전역)·eslint e2e 블록 1개. gitignore: `e2e/.auth/`·`e2e/.tmp/`.

## 2. e2e 하네스(경로 A) 설계 요지 (정찰 §4 — 메인 재검증 완료)

- **전제**: 하네스는 앱을 기동하지 않는다(사용자가 `bun run tauri dev` 로 켜 둔 앱의
  remote-control 서버에 접속만, webServer 블록 없음·부재 시 fail-fast).
- **포트 발견 3단**: 로그 파싱(`원격 접속 서버 기동: port=`) → lsof 후보 수집 →
  `/__taide/login` 프로브 확정. 로그 단독은 불안정 — tauri-plugin-log 기본 로테이션
  KeepOne(파일 삭제)+40KB 실측 확인.
- **로그인**: 사용자 1회 준비(REMOTE 비밀번호 8자+·password_only ON·활성화 ON) 후
  `POST /__taide/login`(**Origin 헤더 명시 필수** — server.rs:145-150 확인) → storageState
  재사용. 비밀번호는 `TAIDE_E2E_PASSWORD` 환경변수로만. **로그인 POST 는 런당 1회**(실패
  카운터는 성공 전까지 비리셋 — 잠금 60초 회피 규약), 429/401 분기 처리.
- **격리 규약**: workers 1·직렬·retries 0. 픽스처 프로젝트는 `e2e/.tmp/` 에 생성 후
  `project_open`(루트 가드 무접촉 확인 — app data 검증에도 활용 가능). 설정은 globalSetup
  스냅샷→teardown 원복. git 은 픽스처 저장소에서만·커밋 생성 금지. **조작은 UI, 검증만
  invoke**(자기충족 테스트 방지).
- **오라클 정정(리서치 §4 대체)**: monaco·xterm 전역 미노출 + data-testid 0건 실측 —
  ① 저장 후 FS 비교 ② DOM(aria·상태바 Ln/Col·CSS 변수) ③ IPC(invoke 조회). dev 모드 monaco
  모듈 동일 인스턴스 재획득은 파일럿 실측 항목.
- **파일럿 12 시나리오**: 저장 왕복·팔레트·@/:/# 모드·전역 검색·git stage/unstage·터미널
  스트림(**WebglAddon 무보호 new 가 1순위 실측 리스크** — terminal-view.tsx:156)·테마 전환·
  AppFile 저장→반영·**원격 게이트 강제 복원(RCE 상환 — 경로 A 만의 고유 검증)**·팬아웃
  2세션. dev 프록시는 CSP 미부착(serving.rs:100-101) — CSP 판정은 경로 A 로 불가.

## 3. qa6 275항목 실행 경로 분류 (정찰 §1 — 총합 검산 완료)

- **a(e2e-A 유효) 165 / b(사람 실기 전용) 51 / c(부분+실기 보완) 59.**
- b 51건 중 **25건이 Wave I 멀티 윈도우 집중** — 창 생명주기 11항목(차단급 상환 2건 포함)은
  e2e 가능분 0. 근거 실코드 확정: `window_open_auxiliary`(dispatch.rs:968)·
  `layout_move_tab_to_window`(:512)·`window_set_fullscreen`(:969) 원격 거부 + shim 의
  `label='main'` 고정. **예외 1건**: 같은 터미널 세션 2뷰(pty raw 채널 원격 지원 — 데스크톱
  1+원격 1 로 절반 실증 가능).
- 원격 거부 arm 은 **9 arm/11 커맨드**(Wave B·I 확대분 — 리서치 §2 "3종" 은 stale, 이 문서가
  정정본). 팬아웃 이벤트 24종(등록 25종 중 HotExitFlushRequested 제외 — lib.rs:646-671).
- b 대표 축: 네이티브 다이얼로그(shim null)·keyring 실물·⌘Q/NSMenu·Gatekeeper·번들 빌드·
  IME·OS 단축키·tunnel secure context·멀티 OS 창·픽셀. e2e 최고 가치 20선은 정찰 원문 기준
  — 1위 AppFile 원격 게이트 우회 차단, 2위 gist sanitize, 3위 플러그인/VSIX 원격 거부.

## 4. 아키텍처·추상화 전수 감사 — 표준 16에이전트 (사용자 승인)

- **프론트 7 배치**(F1 app+entities / F2 features / F3 widgets 에디터 계열 / F4 widgets
  셸·패널 / F5 widgets 설정·확장 / F6 shared/lib 루트 / F7 shared/lsp+비-lib) + **Rust 8 배치**
  (R1 셸 코어+app·window·layout / R2 infra / R3 remote / R4 git·project·file·tree / R5
  locale·settings·theme·sync / R6 ai·agent·ide / R7 lsp·plugin·vsix·search / R8 terminal·task·
  system·snippet·font) + **X1 계약 정합**(collect_commands↔bindings↔dispatch↔문서 4중 —
  bindings↔dispatch 만 테스트 강제인 공백 조준. F1·R1·R3 뒤 마지막 실행).
- 기계 신호 실측(전건 메인 재검증): 프론트 역방향 import·barrel·any·enum·useCallback/useMemo
  **전부 0** — 감사는 판단 렌즈에 집중. 실신호: features 비즈니스 로직 후보 9건·bridge/
  registry 모듈 싱글톤 25개(멀티윈도우·원격 격리 리스크)·수동 타입 212개(Omit 3건뿐)·widgets
  직접 무효화 8건 / Rust 도메인 간 의존 14엣지(service 기준)·**layout↔window 순환**
  (layout/commands.rs:9↔window/service.rs:7)·window/service.rs tauri 직접 의존·prod 경로
  unwrap 11건·locale/service.rs 4,073 LOC 비대.
- 렌즈 세트: L-FSD·L-SFC·L-TYPE·L-EFFECT·L-CODE·L-A11Y·L-I18N·L-QUERY·L-BRIDGE /
  L-RUST-LAYER·BOUND·ERR·STATE / L-CONTRACT (정찰 원문 정의 준수).

## 5. 심층 QA 웨이브 편성 (사용자 승인 — QA-W0~W7)

| 순서 | 웨이브 | 묶음 | 주 수단 |
|------|--------|------|---------|
| 0 | QA-W0 하네스 부트스트랩 | e2e 경로 A 하네스+파일럿 12 (+경로 B 파일럿 여부는 W1 실기 후 판단) | Playwright webkit+node |
| 1 | QA-W1 창·워크스페이스 코어 | T0: 멀티 윈도우·레이아웃 v2 + B8 Zen (+C9·C11·C15 회귀) | **사람 실기 주축**(qa6 Wave I 40항목) |
| 2 | QA-W2 데이터 무결성·보안 | T0: Hot Exit·원격 게이트·AppFile + B9 gist | 경로 A 최적 + 강제종료·⌘Q 실기 |
| 3 | QA-W3 언어 지능 | T0: LSP 세션 + B7 Semantic + C3~C5 | 경로 A 중심 |
| 4 | QA-W4 입력·AI | B1 키맵 + B2 Inline Edit + C1·C2 | 경로 A + OS 단축키·IME 실기 |
| 5 | QA-W5 도구 축 | B4 터미널 + C6 태스크 + B5 Git + B6 검색 | 경로 A + 실셸 |
| 6 | QA-W6 확장·표현 | B3 플러그인/VSIX + C7·C8·C12 | 사람 실기(다이얼로그) |
| 7 | QA-W7 잔여 스윕 | C13·C14·C16 + deferred 4건 정책 결정 | 실기+잔량 소진 |

- 티어링: **T0 6**(멀티 윈도우·레이아웃 v2 마이그레이션·Hot Exit·remote 게이트·AppFile·LSP
  세션) / T1 9 / T2 16. 근거·영역별 검토 카드는 정찰 원문(요지: 각 T0 의 불변식 — 보조 창
  닫기≠앱 종료·0-손실 복귀·마이그레이션 무손실·게이트 필드 강제 복원·창별 독립 세션).
- 아키텍처 감사(§4)와 flaky 1건(`dirty_미러` mtime) 트리아지는 QA-W1 과 병행.
- **선행 작업**: 손 QA 6건 수정(`2026-08-18-hand-qa-fix-contract.md`)이 QA-W0 앞에 온다 —
  특히 항목 4(LSP 블로킹) 수정 없이는 e2e 파일럿의 타이밍 판정이 오염된다.

## 6. 기각·보류

| 안 | 처리 |
|----|------|
| e2e 보류(실기+논증만) | 기각 — 승인(165항목 자동화 수익) |
| 감사 축소 10에이전트 | 기각 — 표준 16(완벽 우선, 배치 상한 준수) |
| 경로 B(embedded WebDriver) 즉시 병행 | 보류 — QA-W1 사람 실기 결과 후 파일럿 여부 판단(가치 구간은 창 생명주기 13항목 한정) |
| 순수 `playwright` 라이브러리(러너 자작) | 기각 — @playwright/test(러너·fixture·storageState 내장) |
| bunx playwright 로 러너 실행 | 기각 — bun 런타임으로 러너가 뜸(비지원), node 실행 고정 |
