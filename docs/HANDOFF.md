# HANDOFF — 2026-08-15 세션 스냅샷 (잔여 기능 캠페인 Wave A~E 완료)

> 최종 갱신: 2026-08-15 / dev HEAD: **`80c99ac`**(워킹트리 클린) / prod(origin/main): **`59f6993`**(Wave E 미반영)
> 이 문서는 세션 인수인계 **단일 진입점**이다. 새 세션은 이것부터 읽는다.
> 직전 스냅샷(QA6 후속·기능 확장 완료 시점)은 `git show 99b8772:docs/HANDOFF.md`.

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유하고(ADR-0004), view 는 표시 전용이다. macOS(arm64) 우선.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J → Phase 8 배포(서명·공증) |
| 현재 마일스톤 | **잔여 기능 캠페인**(VS Code·Cursor 갭 P0+P1) — 웨이브 A→I 중 **A~E 완료(5/9)** |
| 직전 작업 | Wave E(터미널·태스크) 검토·수정·커밋(`80c99ac`) 완료 후 **사용자 지시로 정지·근황보고** |

캠페인 정본: `docs/acknowledge/2026-08-14-remaining-features-pro-qa-plan.md`
(범위=P0 잔여+P1, 완벽 우선, 4렌즈 검토, 역할 상향, 웨이브 A→I 편성).
전체 순서: **잔여 기능(A~I) → 전문 QA(e2e) → Phase 8**. 실기 QA 는 사용자 지시로 취소(스모크 가정).

## 3. 완료 / 진행 중 / 미착수

### 3.1 이번 세션 완료 — 캠페인 Wave A~E (커밋 12건)

각 웨이브 = "정찰(opus+high Workflow) → 계약(acknowledge/*.md) → 구현(백엔드 Rust 단독 → 프론트
병렬, sonnet+xhigh) → 4렌즈 검토(opus+xhigh) → 적대적 검증(opus+high) → 수정(sonnet+xhigh) →
메인 2차(verify 전체+실물 재검증) → 커밋" 패턴. 검토가 매 웨이브 critical/데이터 손실/RCE급 결함을
실제로 잡음 — "완벽 우선"의 값어치가 입증됨.

| 웨이브 | 커밋 | 계약 정본 | 핵심 내용 | 검토 확정 결함(수정됨) |
|--------|------|-----------|-----------|------------------------|
| **A** LSP 인텔리전스 | `02e9c54` | `2026-08-14-wave-a-lsp-intelligence-contract.md` | Code Action/Quick Fix+on-save·이동 3종(impl/typedef/decl)·Peek/F8·CodeLens·LSP Folding·cross-file 오프너·서버→클라 요청 라우팅·WorkspaceEdit 적용기 | peek orphan 모델 편집 무음 소실(critical)·프리로드 dispose 가 탭 모델 파괴(critical)·백그라운드 탭 편집 덮어쓰기·file_open 루트 가드·applyEdit 프로젝트 간 침범 |
| **B** 하드닝 | `a3c42aa` | `2026-08-15-wave-b-hardening-contract.md` | gist 인바운드 sanitize·Hot Exit 0-손실 보강·Remote 보안(Host 허용목록·비밀번호 8자·잠금 nonce 분리·세션 만료 WS·XFP) | **gist shell_override RCE**·gist 게이트 필드·저장 타이핑 소실·cross-file 미러 구멍·**stale nonce 오라클**(메인 지시의 부작용) |
| **C** Git 확장 | `f73c8ea` | `2026-08-15-wave-c-git-contract.md`·`-wave-c-review-fix-decisions.md` | 3-way 충돌 인라인·hunk/line stage(git2 native)·커밋 상세·파일 히스토리·revert/tag·원격 checkout·blame 뷰 | **git_resolve_conflict 경로 트래버설(critical — 메인 직접 수정)**·에디터 unstage 좌표 불일치·미추적 파일 stage·revert clean-index 가드 |
| **D** 탐색·검색 | `14633ed` | `2026-08-15-wave-d-search-nav-contract.md`·`-wave-d-b3-search-editor-decisions.md` | 팔레트 @/#/: 모드·Workspace Symbol(⌘T)·Breadcrumbs·Search Editor(신규 TabKind)·gitignore 검색(ignore 크레이트)·검색 히스토리·capability 확충 | 검색 취소 상호 절단(세션 단위 키)·오케스트레이션 3중 중복(useSearchRun)·workspace-symbol file 스킴 가드·recent_searches 서버 상한 |
| **E** 터미널·태스크 | `80c99ac` | `2026-08-15-wave-e-terminal-tasks-contract.md` | OSC133 셸 통합(자동 주입)·태스크 러너(detect_tasks)·Run Selected Text·xterm 블록 모델·⌘↑↓ 이동 | **zsh 주입이 .zshenv/.zprofile 유실 → PATH·brew 손실(major, 실제 zsh 재현)**·posix_quote fish 백슬래시 탈출·Makefile ::= 가드·OSC133 블록 상한 |

기타 커밋: `49564cc`(캠페인 계획·PROCESS 아카이브)·`0b1f66a`(재시도 타이머 피드백)·계약 커밋 5건.

기준선 성장: 프론트 테스트 **472→856**, Rust **635→751**(+6 통합 +17 CLI). 전 웨이브 `bun run verify`
전체 + vite build 그린. 신규 승인 의존성: `ignore` 크레이트. `toml`·`argon2` 미승인.

### 3.2 진행 중

**없음.** Wave E 커밋 완료, 워킹트리 클린. 사용자 지시로 Wave F 착수 전 정지 상태.

### 3.3 미착수 — 캠페인 잔여 (PROCESS.md c-F~c-I·d·e·f)

1. **웨이브 F~I** (각각 정찰→계약→구현→검토 패턴 반복):
   - **F 에디터 표현**: Semantic Tokens·사용자 스니펫·Format on Type/Paste·Emmet
   - **G AI**: Inline Edit(⌘K)·AI 커밋 메시지 (provider 3종·prompt.rs 재사용)
   - **H 키맵 엔진**: chord(⌘K ⌘S)·when 컨텍스트·shift+기호 event.code (keymap 전면 개정 — 타 웨이브 안정화 후)
   - **I 셸·워크스페이스**: Zen/포커스 모드·멀티 윈도우·설정 파일 탭 편집·플러그인 설치 UI·VSIX grammar (최대 구조 변경 — 최후)
2. **전문 QA(d)**: 기능 전수 리스트업 → 체크리스트 → 심층 검토(opus+xhigh, 심층 opus+max) + e2e +
   아키텍처·추상화 전수 감사. e2e 경로 리서치 완료(`docs/research/2026-08-14-e2e-path-research.md`).
3. **Phase 8(e)**: 서명·공증 (미구성 — `tauri.conf.json` 에 macOS 서명 섹션 없음).
4. **문서 부채 정리**: 아래 §3.4 — 전문 QA 착수 전 문서화 워크플로로 일괄.
5. **PROCESS.md 아카이브(f)**: 다시 300줄 근접 시 완료 섹션을 `docs/history/` 로.

### 3.4 문서 부채 (KNOWN ISSUE — 코드는 정합, 문서만 미반영)

Wave D·E 계약 §5 문서 갱신이 일부 유예됨(검토 fixer 가 Phase C 몫으로 명시 유예):
- `docs/ipc-contract.md` — Wave A~E 신규 커맨드(codeAction 계열·git 13종·detect_tasks 등) **부분 누락**.
- `docs/data-model.md` — `recent_searches`·`remote_allowed_hosts`·`SearchQuery`(context_lines·respect_gitignore)·
  `TabKind::SearchEditor`·Task 타입 **누락**.
- `docs/quality-assurance/2026-08-11-qa6-checklist.md` — Wave A~E 실기 항목 일부만 추가됨.
- (반영됨: `docs/features/tasks.md` 신규·`docs/features/lsp.md`·`docs/features/git.md`·`docs/tech-stack.md` ignore·
  `docs/research/xterm-pty.md §8`·계약 10건·PROCESS c-A~c-E)
→ **조치**: 전문 QA(d) 착수 전 문서화 워크플로로 일괄 정리. 코드는 verify 그린이라 정합, 문서만 뒤처짐.

### 3.5 알려진 미검증 (KNOWN ISSUE)

- **캠페인 전량 실기 미검증** — 기계 검증(verify·vite build)만. 사용자가 실기 QA 를 스모크 가정으로
  취소(`2026-08-14-remaining-features-pro-qa-plan.md` §2-1). 앱 실행은 사용자만(`bun run tauri dev`).
- Wave E 특히: OSC133 명령 데코·⌘↑↓ 이동은 실제 터미널, 태스크 러너 감지·실행은 실제 프로젝트,
  셸 rc 주입은 Finder 실행 .app(GUI 최소 PATH)에서만 실동작 확증 가능.
- 직전 세션 잔여(99b8772): W1~W7·QA6 후속·기능 확장 1~3차도 실기 미검증.

## 4. 의사결정 요약 (상세·기각 대안은 acknowledge 계약이 정본)

핵심 결정은 각 웨이브 계약(`docs/acknowledge/2026-08-1{4,5}-wave-*-contract.md`)의 §1(결정)·§4(기각)에
있다. 세션 레벨 결정:

| 결정 | 채택 | 기각 대안 |
|------|------|-----------|
| 캠페인 범위 | P0 잔여 + P1 전부 | backlog 소품(북마크·접근성)은 전문 QA 후 재검토 |
| 실기 QA 시점 | **취소**(구현분 스모크 가정) | 원래 "전체 실기 선행"을 사용자가 직접 번복 |
| 품질 원칙 | 효율보다 완벽 — 4렌즈 상설·역할 상향 | 절차 축소·토큰 절약 금지 |
| e2e 경로 | remote 미러+Playwright(webkit·node) 1차 축 / embedded WebDriver 파일럿 후 보조 | tauri-driver 단독(macOS 불가)·CrabNebula(유료)·puppeteer(WebKit 없음)·bun 런타임 통합 |
| Wave A hunk stage | git2 native apply | CLI git apply --cached |
| Wave C 3-way | 인라인 데코레이터 | 3패널 완전 자작 |
| Wave C 뷰 배치 | 경량(패널·오버레이·기존 diff 탭) | 신규 TabKind 다수 |
| Wave D Search Editor | 신규 TabKind(영속 에디터 표면) | 경량 배치 |
| Wave D gitignore | ignore 크레이트 | 수동 파서 |
| Wave E OSC133 | 자동 주입+opt-out·순수 133 | OSC633 확장·수동 주입만 |
| Wave E fish | 주입 생략(네이티브 OSC133) | 이벤트 함수 주입(fish 4.0+ 네이티브라 불필요, 4.0 미만 폴백은 backlog) |
| Wave E 태스크 | Rust 감지+고정 명령 세트 | toml 크레이트(Cargo 정밀 파싱) |

## 5. 사용자 방향성 & 작업 규칙

### 5.1 운영 방식 (역할 5단 — 캠페인 상향)

- 오케스트레이팅·계약·2차 검토 = 메인(직접 구현 금지, 예외: 소규모 2차 수정 — Wave C 경로 트래버설·
  Wave D exhaustive-deps·Wave C/D fmt 누락을 메인이 직접 수정한 선례). 구현 = **sonnet+xhigh** /
  렌즈 검토 = **opus+xhigh**(4렌즈: 계약·정확성·보안·설계/추상화) / 적대적 검증 = **opus+high** /
  QA·버그 = fable+high / 정찰·리서치 = opus+high.
- 위임은 규모 무관 **Workflow 로만**. 백엔드 Rust 는 한 시점 한 에이전트(단독 소유). 프론트는 파일
  소유권 분리 병렬. 에이전트 보고 불신 — 핵심 주장은 메인이 실물(소스·grep·실행) 재검증.
- **확인 질문은 추천안 패키지로 묶어** "전부 추천안대로" 한마디로 답할 수 있게(사용자가 매회 그렇게 답함).
- **반쪽 기능 금지** — 설치만 되고 안 열리는 류의 동작 착시 금지. 끝까지 배선.

### 5.2 답변·코드 규칙

- 한국어+존댓말·간결·이모지 금지. 검증 안 된 단언 금지. 보고만 하고 멈추지 말 것.
- arrow fn만·반환 타입 명시 금지·**TS any/enum 금지**(단 **Rust enum 은 백엔드 관행 허용** — TabKind·
  TaskSource 등, specta 가 TS union 생성)·주석 금지(영어 JSDoc·러스트 doc comment 만)·매직넘버 금지·
  useCallback/useMemo 금지(useEffect 는 외부 동기화만)·삼항 2중첩 금지·named export·1파일 1컴포넌트·
  FSD 위→아래·barrel 금지·서버 상태 TanStack Query.
- i18n locale/service.rs 4곳(required·en·ko·ja) 동기 + en⊆required 테스트. IPC 시간 f64·정수 u32.
  Rust 후 **반드시 cargo fmt**(Wave C/D 에서 메인이 fmt 누락으로 fmt 게이트 깬 선례 — 주의).
- 신규 커맨드 배선 3곳(lib.rs collect_commands[RAW 는 RAW_CHANNEL_COMMANDS]·dispatch.rs
  IMPLEMENTED+match arm·bindings.ts 자동생성) + 파리티 테스트 강제. dispatch match 는 arg 매크로 camelCase.
- 시크릿/비밀번호는 keyring 에만(settings.json 은 gist 동기화 — 화이트리스트·strip_non_syncable 확인).

### 5.3 금지 사항

- **에이전트 셸에서 앱 실행 금지**(`bun run tauri dev`·`tauri build` 는 사용자만).
- HACK·검사기 끄기(#[allow]·@ts-ignore·eslint-disable) 금지. 우회 대신 근본 수정.
- 승인 외 신규 패키지 금지. 승인 누적: reqwest·keyring·sha2·flate2·tar·zip·xz2·axum·@shikijs 4종·
  monaco-editor-core(dev)·**ignore(신규)**. **argon2·toml 미승인**.
- 각 계약의 기각 대안 재론 금지. 구현 완료분(A~E) 재구현 금지 — 검토·검증 완료.
- git: main=prod, dev=개발, **dev 자동 커밋·푸시 ON**. main 직접 커밋·git add -A·force push·
  Co-Authored-By·.env 금지. 종료 전 `bun run verify` 통과. **가드 훅이 `dev:main` refspec 을
  force 로 오판**하니 prod 병합은 `git branch -f main dev` 후 `git push origin main`(fast-forward).
- **재시도 대기 타이머는 60초 고정**(지수 백오프 금지 — `docs/feedback/2026-08-14-retry-timer-granularity.md`).

## 6. 미해결 질문 / 사용자 확인 필요

1. **Wave E(`80c99ac`) 도 prod 병합할지** — A~D+계약(`59f6993`)은 병합됨, E 만 dev.
2. **다음 방향**: Wave F(에디터 표현) 진행 vs 문서 부채 먼저 정리 vs 전문 QA 조기 전환.
3. 캠페인 전량 실기 미검증 — 특히 Wave E OSC133·태스크 러너·셸 rc 주입은 실기 확증 필요(사용자만 실행).
4. 검색어 gist 동기화(recent_searches) — 계약 승인 사항이나 프라이버시 우려 시 제외 가능(사용자 재확인 여지).

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS(arm64), bun 1.3.x. cargo PATH 밖 — `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"` (zsh 변수명 `path` 금지) |
| 실행 | dev = `bun run tauri dev`(사용자만). 프로덕션 빌드 = `bun run tauri build`(`scripts/tauri.ts` 가 CLI 사이드카+bundle conf 자동 합성) |
| 검증 | `bun run verify` = typecheck→lint→format→bun test→cargo fmt/clippy/test. bindings 재생성 = `cargo test`(`typescript_바인딩을_생성한다`) |
| 기준선(2026-08-15 실측) | 프론트 **856 tests** / Rust **751 lib**(+6 통합 +17 CLI) / dispatch 파리티 테스트가 커맨드 이름 집합 일치 강제 |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` — buffers/·themes/·locales/·plugins/·lsp/·prompts/·settings.json |
| 스크래치패드(세션 소멸) | 정찰·검토 보고서 원문은 세션 스크래치패드에만 — **요지는 전부 계약·PROCESS·이 문서에 반영**, 새 세션은 문서만 믿으면 됨 |

## 8. 다음 세션 TODO (우선순위 순)

1. **사용자 확인 대기**(§6) — Wave E prod 병합 여부·다음 방향(F vs 문서부채 vs 전문QA). 확인 전 코드 수정 금지.
2. **Wave F(에디터 표현)** 진행 시: 정찰 Workflow(Semantic Tokens·스니펫·Format on Type·Emmet 현행 구조·
   monaco API·LSP semanticTokens capability) → 계약 → 구현 → 4렌즈. Emmet 은 `emmet-monaco-es` 의존성
   승인 필요(갭 분석 §1 P1). 관련 파일: `src/shared/lib/lsp/`·`src/features/editor/code-editor.tsx`·
   `src-tauri/src/domain/lsp/`. 갭 분석 정본 `docs/research/2026-08-13-vscode-cursor-gap.md` §1·§2.
3. **문서 부채 정리**(§3.4) — 문서화 Workflow 로 ipc-contract·data-model·qa6-checklist 를 Wave A~E
   실코드 기준 일괄 갱신. 전문 QA 착수 전 필수.
4. **전문 QA(d)** — e2e 경로는 `docs/research/2026-08-14-e2e-path-research.md`(remote 미러+Playwright
   webkit·node / embedded WebDriver 파일럿). 아키텍처·추상화 전수 감사 포함.
5. Phase 8(서명·공증).

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | 시간순 체크리스트 — 캠페인 절 c-A~c-E 에 각 웨이브 상세(구현·검토·수정·검증) |
| `docs/acknowledge/2026-08-14-remaining-features-pro-qa-plan.md` | **캠페인 정본** — 범위·순서·역할·품질 원칙 |
| `docs/acknowledge/2026-08-14-wave-a-lsp-intelligence-contract.md` | Wave A 계약(결정·기각·설계) |
| `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` | Wave B 계약 |
| `docs/acknowledge/2026-08-15-wave-c-git-contract.md`·`-wave-c-review-fix-decisions.md` | Wave C 계약·검토 수정 결정 |
| `docs/acknowledge/2026-08-15-wave-d-search-nav-contract.md`·`-wave-d-b3-search-editor-decisions.md` | Wave D 계약·B3 결정 |
| `docs/acknowledge/2026-08-15-wave-e-terminal-tasks-contract.md` | Wave E 계약(§6 검토 수정 반영 포함) |
| `docs/acknowledge/2026-08-14-monaco-command-service-deep-import.md` | monaco 내부 딥 임포트 사유(Wave A) |
| `docs/research/2026-08-13-vscode-cursor-gap.md` | **VS Code·Cursor 갭 분석**(P0/P1 — F~I 웨이브 근거) |
| `docs/research/2026-08-14-e2e-path-research.md` | e2e 실행 경로 리서치(전문 QA 재료) |
| `docs/research/xterm-pty.md` | 터미널·OSC133 셸 주입 설계(§8, Wave E 근거) |
| `docs/feedback/2026-08-14-retry-timer-granularity.md` | 재시도 타이머 1분 단위 규칙 |
| `docs/tech-stack.md` | 의존성 정본 — ignore 크레이트·toml 기각 반영 |
| `docs/backlog.md` | P0 대형·하드닝 잔여·후속 후보 |
| `docs/ipc-contract.md`·`data-model.md`·`quality-assurance/2026-08-11-qa6-checklist.md` | **문서 부채(§3.4)** — Wave A~E 부분 반영, 코드가 정본 |
| `docs/features/`(tasks·lsp·git·editor 등)·`PRD.md`·`architecture.md`·`roadmap.md`·`theme-system.md`·ADR | 기존 정본 |

## 10. 복기 신뢰도

- **높음**: 커밋 12건이 dev 에 고정, 계약 10건·PROCESS·이 문서가 웨이브마다 실시간 동기. §7 수치는 세션 말 실측.
- **중간**: 각 웨이브 내부 구현 디테일(예: OSC133 블록 인덱스 재계산, WorkspaceEdit apply anchor 보정)은
  문서보다 코드가 정본 — 재론 시 해당 파일 직접 확인. 문서 부채(§3.4) 구간은 코드를 믿을 것.
- **낮음**: 없음. 정찰·검토 보고서 원문(스크래치패드)은 세션 소멸로 접근 불가 — 요지는 계약·이 문서에 있음.
