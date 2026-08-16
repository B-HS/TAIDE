# HANDOFF — 2026-08-16 세션 스냅샷 (잔여 기능 캠페인 A~I 완료·문서 부채 정리)

> 최종 갱신: 2026-08-16 / dev HEAD: **`c79e853`** / prod(origin/main): **`40f85e1`**(캠페인 A~I 전량 반영, 문서 정리분만 dev 선행) / 워킹트리 클린
> 이 문서는 세션 인수인계 **단일 진입점**이다. 새 세션은 이것부터 읽는다.
> 직전 스냅샷(A~E 완료 시점)은 `git show d525640:docs/HANDOFF.md`.

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유하고(ADR-0004), view 는 표시 전용이다. macOS(arm64) 우선.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J → Phase 8 배포(서명·공증) |
| 현재 마일스톤 | **잔여 기능 캠페인**(VS Code·Cursor 갭 P0+P1) — 웨이브 A→I **전량 완료(9/9)** |
| 직전 작업 | 문서 부채 정리(ipc-contract·data-model·qa6 실코드 기준 갱신·PROCESS 아카이브, `c79e853`) 후 정지 |

캠페인 정본: `docs/acknowledge/2026-08-14-remaining-features-pro-qa-plan.md`
(범위=P0 잔여+P1, 완벽 우선, 4렌즈 검토, 역할 상향, 웨이브 A→I 편성).
전체 순서: **잔여 기능(A~I) 완료 → 문서 부채 정리 완료 → 전문 QA(e2e·아키텍처 감사) → Phase 8**.
실기 QA 는 사용자 지시로 취소(스모크 가정) — 캠페인 전량 실기 미검증이 KNOWN ISSUE 로 존속.

## 3. 완료 / 진행 중 / 미착수

### 3.1 이번 세션 완료 — 캠페인 Wave F~I + 문서 정리 (커밋 12건: d525640..c79e853)

각 웨이브 패턴: **정찰(opus+high 4축 Workflow) → 하중 주장 메인 재검증 → 계약(acknowledge/*.md,
추천안 패키지 질문) → 구현(sonnet+xhigh, Rust 단독 소유 → 프론트 파일 소유 분리 병렬) → 4렌즈 검토
(opus+xhigh) → 적대적 검증(opus+high) → 수정(sonnet+xhigh) → 메인 2차(verify 전체+vite build+실물
재검증) → 커밋(dev) → 다음 웨이브 착수 시 직전 웨이브 prod 병합**. 검토가 매 웨이브 critical/RCE급
결함을 실제로 잡음 — "완벽 우선"의 값어치가 매 웨이브 입증됨.

| 웨이브 | 커밋 | 계약 정본 | 핵심 내용 | 검토가 잡은 critical/major(수정됨) |
|--------|------|-----------|-----------|-----------------------------------|
| **F** 에디터 표현 | `56712b0` | `2026-08-15-wave-f-editor-presentation-contract.md` | Semantic Tokens full+delta(재인코딩·미매핑 드롭·테마 rule 주입)·사용자 스니펫(Rust 도메인·completion provider·편집 UI)·Format on Type/Paste(어댑터 2종)·Emmet(emmet-monaco-es) | **[critical] semantic 테마 rule 의 bare scope append 가 monaco 트라이 last-wins 로 덮어써 번들 테마 11종 실색 변화**(→`taideSemantic.<token>` 네임스페이스)·스니펫 prefetch gcTime GC(→QueryObserver)·Windows 경로탈출·설정 토글 미반영 |
| **G** AI | `2c93289` | `2026-08-16-wave-g-ai-contract.md` | Inline Edit ⌘I(ContentWidget·모델 무변경 프리뷰(삭제 데코+ViewZone)·수락 1회 executeEdits)·AI 커밋 메시지(git2 staged diff·Sparkles)·provider `instruct`·설정 리네임(ai_provider/ai_model) | **[major×5] instruct 가 auto-tab 256 토큰 캡 상속(→전용 4096+length 감지)·팔레트 precondition 무동작(→keybindingContext)·코드펜스 스트립 실패(→스캔 파서)·truncated/시크릿 미전달(→본문 안내+.env 제외)·제출 시 포커스 이탈** |
| **H** 키맵 엔진 | `fcb5db9` | `2026-08-16-wave-h-keymap-contract.md`·`2026-08-16-monaco-contextkeyexpr-deep-import.md` | chord(⌘K ⌘S·전역 스토어·순수 dispatch·monaco 유예 창으로 기존 회귀 4건 상환)·when(ContextKeyExpr 딥임포트·pull 게터)·훅 반전·taide.* 카탈로그 편입·Sparkles unstaged 폴백 | **[critical→major×2] taide.* 편입 무동작(addAction 이 editorId:id 등록 →registerCommand 전역 등록)·팔레트 독립 리스너의 chord/유예 우회** + 인스펙터 modal 공허·⌘K S 계약 이탈 정정 |
| **I** 셸·워크스페이스 | `40f85e1` | `2026-08-16-wave-i-shell-workspace-contract.md` | **멀티 윈도우 완전 구현**(창=탭 분리·domain/window·레이아웃 v1→v2 무손실 마이그레이션·layout_move_tab_to_window·창별 close·전 창 hot-exit 집계)·Zen(shell_view 영속·⌘K Z)·설정탭(AppFile·SettingsChanged 3경로 통합)·플러그인 설치 UI·VSIX grammar·monaco 동적 언어 | **[critical×3] 보조 창 layout 커맨드 전부 NotFound(→all_roots)·원격 app_file_write 가 shell_override RCE 게이트 우회(→strip 가드)·LSP 세션 재사용 창간 id 충돌+중복 initialize(→owner 스코프 창별 독립 세션)** + major 다수 |

**문서 부채 정리**: `c79e853` — ipc-contract 커버리지 72%→100%(178커맨드·25이벤트 전수, 누락 49·잔재 6·
시그니처 4 정정)·data-model(설정 리네임·TabKind 9종·레이아웃 v2·영속/비영속 타입·SettingsChanged)·
qa6(Wave B/C/D 섹션 신설+E~I 보강, 275항목)·PROCESS 아카이브(589→471줄).

**prod 병합**: 매 웨이브 착수 시 직전 웨이브를 `git branch -f main dev` 후 `git push origin main`(fast-forward)
으로 병합. 현재 main=`40f85e1`(A~I 전량). **dev 의 `c79e853`(문서 정리)만 prod 미반영** — 다음 세션이
전문 QA 착수 시 함께 병합 여부 판단(문서 전용이라 병합 안전).

### 3.2 기준선 (2026-08-16 실측)

- 프론트 테스트 **1127** / Rust **881 lib** + 6 통합(session_restore) + 17 taide-cli. IPC 커맨드 **178종**
  (raw 3종 별도) + 이벤트 25종.
- `bun run verify` 전체(typecheck→lint→format→bun test→cargo fmt/clippy/test) + `bunx vite build` 그린.
- 신규 승인 의존성(전 캠페인 누적): `ignore`(Wave D)·`emmet-monaco-es`@5.7.0(Wave F). **argon2·toml 미승인**.

### 3.3 진행 중

**없음.** 캠페인 A~I 커밋 완료·문서 부채 정리 완료·워킹트리 클린. 사용자 지시로 전문 QA 착수 전 정지.

### 3.4 미착수 — 캠페인 후속 (PROCESS.md d·e)

1. **전문 QA(d)**: 기능 전수 체크리스트(qa6-checklist 로 갱신 완료·275항목) → 기능별 심층 검토
   (opus+xhigh, 심층 opus+max) + e2e + 아키텍처·추상화 전수 감사. e2e 경로 리서치 완료
   (`docs/research/2026-08-14-e2e-path-research.md`: A=remote 미러+Playwright(webkit·node) 1차 축 /
   B=embedded WebDriver 파일럿). **의존성 승인·감사 범위가 착수 전 결정 사항**(Playwright+webkit,
   필요 시 tauri-plugin-wdio-webdriver).
2. **Phase 8(e)**: 서명·공증 (미구성 — `tauri.conf.json` 에 macOS 서명 섹션 없음).

### 3.5 알려진 미검증·미해결 (KNOWN ISSUE)

- **캠페인 전량 실기 미검증** — 기계 검증(verify·vite build)만. 앱 실행은 사용자만(`bun run tauri dev`).
  특히 **Wave I 멀티 윈도우는 실기 확증이 절실**(보조 창 열기/이동/닫기·같은 프로젝트 2창 LSP·같은
  터미널 2창 시청·전 창 flush·Zen·설정탭 반영·플러그인 설치→하이라이팅). qa6-checklist 에 실기 항목 정리됨.
- **Wave I 검토 deferred 4건**(계약 §6 기록, backlog 이월): ①보조 창 hot-exit flush 미핸드셰이크
  (✕ 클릭 시 최대 500ms 미저장 편집 유실 — blur/언마운트가 대부분 커버, minor) ②restore_auxiliary_windows
  가 전 프로젝트 보조 창 일괄 복원(정책 결정 필요) ③보조 창 팔레트 미마운트(tab.moveToMainWindow 팔레트
  도달 불가 — 컨텍스트 메뉴로는 도달) ④보조 창 소유자 없는 chord 1단 삼킴(Wave H idempotent 불변식 충돌).
- **문서 부산물 3건**(코드 정본, 문서만 뒤처짐 — 다음 문서 세션 대상): `app:ready` 죽은 이벤트 배선
  (events.rs 에 타입·collect 등록되나 emit 호출 0)·`docs/features/agent-integration.md` 의
  `release_wait_marker` 잔재(실제 커맨드는 `agent_release_marker`)·`docs/ipc-contract.md` snippet 절이
  bindings 의 `_Serialize`/`_Deserialize` 유니온 분할 미반영(기능 영향 0).
- **flaky 테스트 1건**(캠페인 무관·별도 트리아지): `domain::file::service::tests::dirty_미러는_저장하고_삭제할_수_있다`
  가 mtime 기반 assertion 으로 cargo test 병렬 실행 시 ~1/15 실패. 단독/재실행은 항상 통과. base 커밋에서도
  동일 재현(Wave 무관).
- 직전 세션 잔여(d525640): W1~W7·QA6 후속·기능 확장 1~3차도 실기 미검증
  (`docs/history/2026-08-16-process-archive-qa6-feature-waves.md` 로 아카이브).

## 4. 의사결정 요약 (상세·기각 대안은 acknowledge 계약이 정본)

핵심 결정은 각 웨이브 계약(`docs/acknowledge/2026-08-1{5,6}-wave-*-contract.md`)의 §1(결정)·§4(기각)·
§6/§7(검토 정정)에 있다. 이번 세션 웨이브별 채택/기각 요약:

| 웨이브 | 채택 | 주요 기각 대안 |
|--------|------|----------------|
| F | 풀 패키지+emmet-monaco-es 승인·Semantic **delta 포함**·포매팅 둘 다·스니펫 Rust 저장소 | Semantic 기본 OFF·full only·ResolvedTheme 스키마 신설·서버 legend 직통과(워시아웃)·Emmet 자작 |
| G | 공통 기반+**설정 리네임**(ai_provider/ai_model)·Inline Edit **⌘I**(모델 무변경 프리뷰·일괄 응답)·커밋 메시지 git2 native | ⌘K 기본키(chord 충돌)·스트리밍(provider 2종 신설)·diff 탭 프리뷰·모델 선반영·run_git·별도 trait·auto-tab 파일 확장 |
| H | chord ⌘K 프리픽스+**monaco 유예 창**·when ContextKeyExpr 딥임포트·**마이그레이션 보수**(행동 변화 0)·Sparkles unstaged 폴백 | softDispatch·chords 배열 직렬화·마이그레이션 전략 2/3·when 오버라이드 노출·비US 역매핑·when 자체 파서 |
| I | **멀티 윈도우 완전 구현**(사용자: "MVP 아니라 제대로 완벽하게")·창=탭 분리·차단급 4종 근본 수정·Zen 레이아웃 영속·AppFile·플러그인 설치 UI | MVP(창=프로젝트)·전면 보류·보조 창 탭 폐기(VS Code 동형)·가상 스킴·비활성 토글·extract_zip 재사용·마켓플레이스 |
| 다음 방향 | **문서 부채 정리 먼저**(완료) → 전문 QA | 전문 QA 조기 착수·멀티윈도우 마감 우선 |

세션 레벨 불변 규칙(직전 세션 계승): 실기 QA 취소(스모크 가정)·효율보다 완벽·4렌즈 상설.

## 5. 사용자 방향성 & 작업 규칙

### 5.1 운영 방식 (역할 5단 — 캠페인 상향, 이번 세션 그대로 유지)

- 오케스트레이팅·계약·2차 검토 = 메인(**직접 구현 금지**, 예외: 소규모 2차 수정 — 이번 세션엔
  Wave F stripCodeFence 반환타입·Wave H PROCESS 아카이브 등 극소수). 구현 = **sonnet+xhigh** /
  렌즈 검토 = **opus+xhigh**(4렌즈: 계약·정확성·보안·설계/추상화) / 적대적 검증 = **opus+high** /
  정찰·리서치 = **opus+high**. (QA·버그 = fable+high 는 이번 세션 미사용)
- 위임은 규모 무관 **Workflow 로만**. 백엔드 Rust 는 한 시점 한 에이전트(단독 소유). 프론트는 파일
  소유권 분리 병렬. **에이전트 보고 불신** — 핵심 주장·검토 발견은 메인이 실물(소스·grep·bun/cargo 실행)
  재검증. 정찰 하중 주장은 계약 전 메인이 실물 확인.
- **확인 질문은 추천안 패키지로 묶어** "전부 추천안대로" 한마디로 답할 수 있게. 다음 웨이브 착수 질문에
  **직전 웨이브 prod 병합 여부**를 항상 포함.
- **반쪽 기능 금지** — 끝까지 배선. **검토·검증 단계 축소 금지**(효율보다 완벽).

### 5.2 답변·코드 규칙

- 한국어+존댓말·간결·이모지 금지. 검증 안 된 단언 금지. 보고만 하고 멈추지 말 것.
- arrow fn만·반환 타입 명시 금지·**TS any/enum 금지**(단 **Rust enum 은 백엔드 관행 허용**)·주석 금지
  (영어 JSDoc·러스트 doc comment 만 — 비자명 결정에 doc comment 필수)·매직넘버 금지·useCallback/useMemo
  금지(useEffect 는 외부 동기화만)·삼항 2중첩 금지·named export·1파일 1컴포넌트·FSD 위→아래·barrel 금지·
  서버 상태 TanStack Query(queryOptions 팩토리+QUERY_KEY 배열 키).
- i18n locale/service.rs 4곳(required·en·ko·ja) 동기 + en⊆required 테스트. **IPC 시간 f64·정수 u32.**
  Rust 후 **반드시 cargo fmt**. 신규 커맨드 배선 3곳(lib.rs collect_commands·domain/remote/dispatch.rs
  IMPLEMENTED+match arm·bindings.ts 자동생성) + 파리티 테스트. dispatch match 는 `arg!(args,"camelCase")`.
- 시크릿/비밀번호는 keyring 에만. 설정 gist 동기화는 화이트리스트·strip_remote_gated_settings 확인.

### 5.3 금지 사항

- **에이전트 셸에서 앱 실행 금지**(`bun run tauri dev`·`tauri build` 는 사용자만).
- HACK·검사기 끄기(#[allow]·@ts-ignore·eslint-disable) 금지. 우회 대신 근본 수정
  (이번 세션 clippy too_many_arguments 도 구조체·타입 별칭으로 근본 해결).
- 승인 외 신규 패키지 금지. 승인 누적: reqwest·keyring·sha2·flate2·tar·zip·xz2·axum·@shikijs 4종·
  monaco-editor-core(dev)·**ignore**·**emmet-monaco-es**. **argon2·toml 미승인**.
- 각 계약의 기각 대안 재론 금지. 구현 완료분(A~I) 재구현 금지 — 검토·검증 완료.
- git: main=prod, dev=개발, **dev 자동 커밋·푸시 ON**. main 직접 커밋·git add -A·force push·
  Co-Authored-By·.env 금지. 종료 전 `bun run verify` 통과. **가드 훅이 `git branch -f main dev && push`
  를 한 줄로 하면 force 로 오판**하니 `git branch -f main dev`(무출력)와 `git push origin main` 을
  **분리 실행**(fast-forward). **재시도 대기 타이머는 60초 고정**(지수 백오프 금지).

## 6. 미해결 질문 / 사용자 확인 필요

1. **문서 정리분(`c79e853`) prod 병합 여부** — 캠페인 A~I 는 병합됨(`40f85e1`), 문서 전용 커밋만 dev.
2. **전문 QA 설계**(다음 착수 작업) — e2e 의존성 승인(Playwright+webkit / 필요 시 embedded WebDriver)·
   감사 범위(기능 심층 QA 대상 우선순위·아키텍처 전수 감사 축)를 착수 시 추천안 패키지로 확정.
3. **Wave I deferred 4건**(§3.5) 처리 — 전문 QA 로 실기 확인 후 backlog 승격/수정 결정.
4. **문서 부산물 3건**(§3.5) — 별도 문서 정리 세션 또는 전문 QA 동반.

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS(arm64), bun 1.3.x. cargo PATH 밖 — `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"` (zsh 변수명 `path` 금지) |
| 실행 | dev = `bun run tauri dev`(사용자만). 프로덕션 빌드 = `bun run tauri build`(`scripts/tauri.ts` 가 CLI 사이드카+bundle conf 자동 합성) |
| 검증 | `bun run verify` = typecheck→lint→format→bun test→cargo fmt/clippy/test. bindings 재생성 = `cargo test`(`typescript_바인딩을_생성한다`). 파리티 = `bindings와_dispatch_테이블은_커맨드_이름_집합이_일치한다` |
| 기준선(2026-08-16 실측) | 프론트 **1127 tests** / Rust **881 lib**(+6 통합 +17 CLI) / IPC **178 커맨드**+25 이벤트 |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` — buffers/·themes/·locales/·plugins/·lsp/·prompts/·snippets/(Wave F)·settings.json·session.json·projects/<id>/layout.json·.window-state.json |
| 멀티 윈도우(Wave I) | 보조 창 라벨 `editor-<n>`(Rust 발급). capabilities/main.json windows=['main','editor-*'] 글로브. window_open_auxiliary 는 원격 거부. 레이아웃 v2(auxiliary_windows·shell_view) |
| 스크래치패드(세션 소멸) | 정찰·검토 보고서 원문은 세션 스크래치패드에만 — **요지는 전부 계약·PROCESS·이 문서에 반영** |

## 8. 다음 세션 TODO (우선순위 순)

1. **사용자 확인**(§6) — 문서 정리분 prod 병합·전문 QA 설계(의존성·범위). 확인 전 코드 수정 금지.
2. **전문 QA(d) 착수** — qa6-checklist(`docs/quality-assurance/2026-08-11-qa6-checklist.md`, 275항목)를
   마스터로. e2e 경로는 `docs/research/2026-08-14-e2e-path-research.md`. 기능별 심층 검토(opus+xhigh)+
   아키텍처·추상화 전수 감사(FSD·entities 계층·Rust 도메인 경계·타입 파생). **Wave I 멀티 윈도우 실기
   최우선**(정찰·검토가 차단급 다수 잡은 영역). 관련: `src-tauri/src/domain/{window,layout,lsp,terminal}/`·
   `src/widgets/auxiliary-window-shell/`·`src/shared/lib/window-context.ts`.
3. **Wave I deferred 4건**(§3.5) 실기 확인 후 처리 결정.
4. **문서 부산물 3건**(§3.5) 정리 — app:ready·agent-integration.md·ipc-contract snippet 절.
5. **Phase 8(e)** — 서명·공증(`tauri.conf.json` macOS 서명 섹션 신설).

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | 시간순 체크리스트 — 캠페인 절 c-A~c-I 에 각 웨이브 상세(구현·검토·수정·검증). 완료분 4섹션은 `history/2026-08-16-process-archive-qa6-feature-waves.md` 로 아카이브 |
| `docs/acknowledge/2026-08-14-remaining-features-pro-qa-plan.md` | **캠페인 정본** — 범위·순서·역할·품질 원칙 |
| `docs/acknowledge/2026-08-15-wave-f-editor-presentation-contract.md`(+`-wave-f-kickoff-decisions.md`) | Wave F 계약(§6 검토 정정 포함) |
| `docs/acknowledge/2026-08-16-wave-g-ai-contract.md` | Wave G 계약(§6 검토 정정) |
| `docs/acknowledge/2026-08-16-wave-h-keymap-contract.md`(+`-monaco-contextkeyexpr-deep-import.md`) | Wave H 계약(§6 구현 정정·§7 검토)·ContextKeyExpr 딥임포트 사유 |
| `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` | Wave I 계약(§6 검토 정정 — critical 3 클러스터·deferred 4건) |
| `docs/acknowledge/2026-08-15-wave-{b,c,d,e}-*.md` | 직전 세션 Wave B~E 계약(A~E 상세) |
| `docs/research/2026-08-13-vscode-cursor-gap.md` | **VS Code·Cursor 갭 분석**(P0/P1 — 웨이브 근거). Wave F/H/I 관련 §1·§2·§3·§7 종결 표기 |
| `docs/research/2026-08-14-e2e-path-research.md` | e2e 실행 경로 리서치(전문 QA 재료) |
| `docs/ipc-contract.md` | **IPC 커맨드 178종·이벤트 25종 전수**(2026-08-16 100% 정합 갱신)·원격 dispatch 정책 |
| `docs/data-model.md` | 도메인 타입·설정 필드·레이아웃 v2 스키마·영속/비영속 타입(2026-08-16 갱신) |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | **실기 QA 마스터 체크리스트**(Wave A~I 275항목·전문 QA·사용자 실기 공용) |
| `docs/features/`(ai·keymap·layout-shell·tabs·plugins·git·editor·tasks·lsp·terminal 등) | 기능별 정본(각 웨이브 D 가 갱신) |
| `docs/tech-stack.md`·`architecture.md`(=ARCHITECTURE.md)·`PRD.md`·`roadmap.md`·`theme-system.md`·`adr/` | 스택·아키텍처·요구·순서·테마·ADR 정본 |
| `docs/backlog.md` | P0 대형·하드닝 잔여·후속 후보(Wave I deferred·Copy into New Window 등) |

## 10. 복기 신뢰도

- **높음**: 이번 세션 커밋 12건(d525640..c79e853)이 dev 에 고정, 계약 12건·PROCESS·이 문서가 웨이브마다
  실시간 동기. §7 기준선 수치는 세션 말 실측(프론트 1127·Rust 881+6+17·커맨드 178). 검토 결함·수정은
  각 계약 §6/§7 에 근거와 함께 기록.
- **중간**: 각 웨이브 내부 구현 디테일(예: LSP owner 스코프 재사용의 initialize 캐시, 레이아웃 v2
  마이그레이션 arm)은 문서보다 코드가 정본 — 재론 시 해당 파일 직접 확인.
- **낮음**: 없음. 정찰·검토 보고서 원문(스크래치패드)은 세션 소멸로 접근 불가 — 요지는 계약·이 문서에 있음.
