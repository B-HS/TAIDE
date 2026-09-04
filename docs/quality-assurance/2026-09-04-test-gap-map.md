# 테스트 공백 지도 — 무엇이 잠겼고 무엇이 남았나 (사용성 배치 4 H, 2026-09-04)

> 계약 `acknowledge/2026-09-04-usability-batch4-contract.md` §H.2-6 이 지정한 문서. 조사 원문
> `research/2026-09-04-batch4-topics1-5-research.md` 주제 3c. 하네스 사용법·규약은
> `docs/memory/test-conventions.md`, e2e 제약은 `docs/quality-assurance/2026-08-18-e2e-harness.md`,
> 성능 예산 테스트는 `docs/quality-assurance/2026-09-04-perf-baseline.md` §5 가 정본이다.
>
> **이 문서의 목적은 커버리지 숫자 자랑이 아니라 "이건 아직 아무도 안 본다" 를 명시하는 것**이다.
> 조사 3c 가 지목한 공백은 "테스트가 적다" 가 아니라 **"테스트가 있는데 그 종류가 한쪽에 몰려 있다"**
> 였다 — 순수 함수는 촘촘하고, 렌더·훅·통합은 0이었다.

## 1. 배치 4 전후

| 축 | 배치 4 착수 전 | 웨이브 2 종료 | 비고 |
|----|---------------|--------------|------|
| `bun test` 케이스 | 1,817 → (웨이브 1 후) 2,008 | **2,287** | 파일 203 → **231** |
| `cargo test` lib | 1,248 → (웨이브 1 후) 1,288 | **1,434** | 통합 9 → **13**(`capability_symmetry` 신설 4) + cli 17 |
| e2e 스펙 | 13 → (웨이브 1 후) 20 | **25** | 전부 미실행(앱 기동 필요) |
| DOM 렌더 하네스 | **없음** | happy-dom + RTL | `bunfig.toml` `[test] preload` |
| `.query.ts` 테스트 | 0 / 23 | **11 / 23** | 계약 H.2-2 지정 11종 완료 |
| CI 게이트 | 릴리스 태그 push 만 | **push(dev·main) + PR** | `.github/workflows/ci.yml` |

## 2. 이번 배치가 잠근 것

### 2.1 하네스 (H.2-1)

`bun test` 안에서 React 컴포넌트·훅을 **실제로 마운트**할 수 있게 됐다. 이전에는 "메뉴 컴포넌트는
이 저장소에서 단위 테스트가 불가" 가 여러 프로덕션 파일의 JSDoc 에 사실로 적혀 있었다.

- `src/shared/testing/dom-preload.ts` — happy-dom 전역 등록 + `beforeEach(mock.restore)` + `afterEach(cleanup)`
- `src/shared/testing/render.tsx` — `renderWithProviders` / `renderHookWithProviders` / `createTestQueryClient`
- 하네스를 실제로 쓰는 테스트 파일 **15개**, `.test.tsx` **11개**

### 2.2 종류별로 새로 열린 축

| 축 | 이전 | 지금 | 대표 파일 |
|----|------|------|-----------|
| 컴포넌트 렌더 | 0 | 7종 | `tab-bar-context-menu.test.tsx` · `terminal-context-menu.test.tsx` · `project-display-dialog.test.tsx` · `git-section-header.test.tsx` · `pane-node-view-welcome.test.tsx` · `file-icon-registry.test.tsx` · `render.test.tsx` |
| 훅 | 0 | 3종 | `use-explorer-auto-reveal.test.tsx` · `use-open-file-tab.test.tsx` · `use-overlay-scrollbar.test.tsx` |
| 쿼리 계약(`queryFn`·`QUERY_KEY`·무효화 대상) | 0 | 11종 | `settings`(선택적 무효화 6축) · `theme`(미리보기 스토어 격리) · `plugin`(install 후 목록 재조회) 외 8 |
| 연산 횟수 예산 | 0 | 3파일 | `perf-budget.test.ts` · `search-result-budget.test.ts` · `perf-mark.test.ts` |
| capability 대칭(Rust) | 0 | 4건 | `src-tauri/tests/capability_symmetry.rs` — attach/detach 1:1 을 소스 스캔으로 강제 |
| Rust "테스트 0" 파일 | 4개 | 0개 | `infra/archive.rs` 10 · `error.rs` 10 · `events.rs` 8 · `domain/window/commands.rs` 6 |

### 2.3 회귀 가드로 못박은 계약

- **성능 수정의 동작 불변** — 벽시계가 아니라 **연산 횟수**로 잠갔다(`fuzzyFilter` 후보당 라벨 1회,
  키바인딩 충돌 맵 조회 1회, `appendSearchFileMatches` 배치당 조회 1·기록 1, `list_themes` 재파싱
  프로세스당 1회, git status "무효화 1회당 계산 1회").
- **로케일 3언어 파리티** — 키 집합뿐 아니라 `{{…}}` 보간 플레이스홀더 집합까지 일치 강제(현재 드리프트 0).
- **커맨드 표면 완전분할** — `REMOTE_ALLOWED` ⊎ `REMOTE_DENIED` = 전체, `collect_commands!` ↔ dispatch 테이블 일치.
- **후절화 구조** — `project_open` 이 가드 스코프를 닫은 뒤 attach 하는지를 **소스 스캔**으로 고정
  (되돌리면 다른 테스트는 전부 통과하므로 이 2건이 유일한 기계 방어다).

## 3. 남은 공백 (우선순위 순)

### 3.1 P1 — 다음 테스트 배치의 기본 대상

| 공백 | 무엇이 안 잡히나 | 착수 메모 |
|------|-----------------|-----------|
| `.query.ts` 12종 미커버 | `agent` · `ai` · `app-file` · `font` · `ide` · `locale` · `remote` · `snippet` · `sync` · `system` · `terminal` · `vsix` | 계약 H.2-2 는 11종만 지정했다. 나머지 12종도 같은 형태(`queryFn` 반환 형태 · `QUERY_KEY` 계층 · mutation 무효화 **대상**)라 기계적으로 확장 가능하다 |
| `error-boundary.test.tsx` 재작성 | "크래시 → 폴백 → 재시도 클릭 → children 재마운트" 왕복. 그 파일 자신이 "하네스가 없어 검증 불가" 라고 **선언해 둔 정본**이다 | 이제 하네스로 가능하다. `test-conventions.md` §6 첫 항목 |
| `app/providers/*` 통합 | `ide-sync-provider` · `agent-state-sync-provider` · `native-notification-provider` — 이벤트 수신 → 쿼리 무효화 경로가 전부 미커버 | `mock.module` 로 `listen` 을 갈아 끼우는 기존 관례 + 하네스 `renderWithProviders` 조합 |
| `widgets/**` 부분 실행 깨짐 | `bun test src/widgets` 만 돌리면 **14건**이 `Export named setProjectDisplay not found` 로 실패한다(실측) — `use-editor-lsp-integration.test.ts` 의 `@entities/project/project.ipc` 목이 **부분 표면**이라서다. 전체 실행에서는 entities 가 먼저 로드돼 가려진다. 목을 세운 파일 자신(4건)만이 아니라 같은 실행에 묶인 `use-lsp-session.test.ts`(5건) · `pane-node-view-welcome.test.tsx`(5건)까지 도미노로 깨진다 — `mock.module` 이 프로세스 전역이라 그 파일들은 `project.ipc` 를 목하지 않고도 피해자가 된다 | 목 팩토리를 export 전체로 채우면 3파일 모두 해소. 같은 클래스의 파일이 더 있을 수 있어 **전수 점검**이 필요하다(§3.3) |

### 3.2 P2 — 구조상 하네스로는 못 잡는 것 (e2e·손 QA 소관)

| 공백 | 왜 하네스로 안 되나 | 어디서 잡나 |
|------|--------------------|------------|
| 레이아웃·시각 회귀 | happy-dom 에 레이아웃 엔진이 없다 — `getBoundingClientRect()` width/height = 0, `offsetHeight` = 0 (실측) | sticky 헤더 전환·가상화 스크롤·chevron 회전·CJK 라벨 클립은 **e2e 19·손 QA** |
| WKWebView 고유 결함 | happy-dom 은 WebKit 이 아니다 | IME `keyCode` 229 · 클립보드 손상 · WebKit #269922/#274700 — **e2e 전용** |
| monaco · xterm 실마운트 | monaco 는 실 DOM(`base/browser/window.js`), xterm 은 캔버스/WebGL | 현재 `mock.module` 유지. **하네스에서의 렌더 가능 여부 자체가 미검증** — 에디터·터미널 컴포넌트 테스트를 계획한다면 먼저 확인하고 결과를 `test-conventions.md` 에 적는다 |
| 보조 창 분기 | 하네스가 `location.search` 를 빈 값으로 고정해 `getWindowContext()` 가 항상 main 으로 해소된다 | 순수 함수 `readWindowContext(search)` 직접 호출 + e2e |
| OS 알림 · 앱 전체 포커스 게이트 | `notification_notify` 는 `REMOTE_DENIED` 이고 앱 전체 포커스를 Playwright 가 제어할 수 없다 | Rust 순수 함수 `decide_delivery`(6건) + **손 QA**(하네스 문서 §5.1) |

### 3.3 P3 — 인프라 부채

| 항목 | 내용 |
|------|------|
| 부분 목 표면 전수 점검 | `mock.module` 이 프로세스 전역·last-wins 라, 목 팩토리가 실제 export 보다 뒤처져도 **전체 실행에서는 서로를 가려 준다**. 파일 단독 실행이 깨지는 파일이 §3.1 의 1건 말고 더 있을 수 있다. `mock.module` 사용 파일 32개 / 호출 75회를 하나씩 단독 실행하는 점검을 별도로 걸 값이 있다 |
| `mock.restore` 실사용 1곳 | preload 의 `beforeEach(mock.restore)` 를 실제로 쓰는 파일은 현재 `shared/lib/perf-mark.test.ts` 한 곳뿐이다(`spyOn(console, 'info')` · `spyOn(console, 'table')`, `printPerfReport` 테스트). 그 스파이는 테스트 본문 안에서 세우므로 복원 시점이 문제되지 않고, 전체 실행에서 뒤따르는 파일로 새지 않는 것도 확인했다. 남은 부채는 `beforeAll` 로 세운 스파이가 매 테스트마다 풀리는 점(규약상 의도)이라 — `spyOn` 을 `beforeAll` 에서 쓰기 시작할 때 드러난다 |
| e2e 25스펙 미실행 | 21~25 는 물론 17~20 도 작성 시점 미실행이다. 하네스가 앱을 기동하지 않으므로 사용자가 `TAIDE_E2E_NO_HMR=1 bun run tauri dev` + REMOTE 준비 후 `bun run e2e` 를 돌려야 한다. C8(스위트 후반 webkit 열화) 해소 실측도 그때 함께 나온다 |
| 커버리지 임계 | 계약 H.2-5 가 **미도입**으로 확정했다. `bun test --coverage` 는 돌지만 CI 가 임계로 막지 않는다 — 숫자를 목표로 삼으면 의미 없는 테스트가 늘어난다는 판단 |

## 4. 체크리스트

- [ ] `.query.ts` 나머지 12종 테스트 (§3.1)
- [ ] `error-boundary.test.tsx` 를 하네스로 재작성 (§3.1)
- [ ] `app/providers/*` 이벤트 → 무효화 경로 통합 테스트 (§3.1)
- [ ] `use-editor-lsp-integration.test.ts` 의 `project.ipc` 목 표면 완성 → `bun test src/widgets` 단독 실행 14건 복구 (§3.1, 피해 파일 3개)
- [ ] `mock.module` 32파일 단독 실행 전수 점검 (§3.3)
- [ ] monaco·xterm 하네스 마운트 가능 여부 확인 후 `test-conventions.md` 에 기록 (§3.2)
- [ ] e2e 25스펙 1회 완주 + C8 해소 실측 (§3.3, 하네스 문서 §7.1)
