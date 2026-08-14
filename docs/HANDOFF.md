# HANDOFF — 2026-08-14 세션 스냅샷 (QA6 후속·기능 확장 1~3차 완료)

> 최종 갱신: 2026-08-14 / 대응 커밋: **`06b0397`** (`dev`, origin 푸시 완료, 워킹트리 클린)
> 이 문서는 세션 인수인계 **단일 진입점**이다. 새 세션은 이것부터 읽는다.
> 직전 스냅샷(W7 완료 시점)은 `git show 6a25673:docs/HANDOFF.md`.

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유하고(ADR-0004), view 는 표시 전용이다.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J → Phase 8 배포(서명·공증) |
| 현재 마일스톤 | Phase 7.10 W1~W7 + **QA6 후속 1차 + 기능 확장 1~3차 전량 구현 완료** |
| 직전 작업 | IconButton 레이아웃 회귀 수정(`c675d6f`). **다음 = 실기 QA 일괄**(사용자) → backlog P0 또는 Phase 8 |

사용자 명시: "이거까지만 하면 일단 거의 다 된 듯" — 기능 스트림 일단락 선언.

## 3. 완료 / 진행 중 / 미착수

### 3.1 이번 세션 완료 (커밋 8건, 시간순)

| 커밋 | 내용 | 핵심 파일 |
|------|------|-----------|
| `53e8969` | 한글 조합 상하 떨림 수정 — 폰트 폴백 스택에 Apple SD Gothic Neo | `src/shared/lib/font-stack.ts` |
| `a1dd3de` | **QA6 후속 1차**: 단축키 편집 모달(전 커맨드 23행·검색·재바인딩·unbind·충돌, `KeymapOverrideEntry.actionId: string` A안) / OMLX provider(`jundot/omlx`, FIM 센티널 패밀리 6종→chat 폴백, base URL settings·key keyring) / 툴팁·i18n(재렌더 결함 `bindI18nStore`+`data-locale-ready` 수정 — 누락 키 0건이 진실, IconButton 신설·37건, git 패널 15건, 87키 동기) | `src/widgets/keybindings-editor/`, `src-tauri/src/domain/ai/providers/omlx.rs`, `src/shared/ui/icon-button.tsx`, `src/shared/i18n/i18n.ts` |
| `e3c13bd` | **기능 확장 1차**: blame 줄 위 view zone / monaco 액션 150종 노출(**B안: 실행은 monaco**·`addKeybindingRules`·하이브리드 i18n tier1 42) / taide CLI(externalBin 사이드카·`/usr/local/bin` 설치/제거·열기 배선 3끊김 해소·pending 큐) / **갭 분석**(`docs/research/2026-08-13-vscode-cursor-gap.md` P0 10·P1 10) / 퀵윈 5(sticky scroll·커서위치 상태바·documentHighlight·selectionRange 어댑터·LSP capabilities 확충) | `src/shared/lib/monaco-actions.ts`, `crates/taide-cli`(무변경—배선만), `scripts/{tauri,build-cli-sidecar}.ts`, `src-tauri/tauri.bundle.conf.json` |
| `23c3fb2` | **기능 확장 2차**: Alt 조합 재바인딩 수정(`normalizeKeymapEventKey` — event.key 우선·합성 문자만 code 유도, 레거시 OR 매칭 하위호환) / blame → **에디터 페인 footer 바**(view zone 삭제) / usage 상세 모달(`system_usage_breakdown`, 자손 BFS+도메인 PID 매핑, System 인스턴스 분리) | `src/shared/lib/keymap.ts`, `src/features/editor/blame-footer-bar.tsx`, `src/widgets/system-usage-modal/` |
| `4c08d31` | suggest 위젯 화살표 누수 수정 — 원인은 **⌥ 유지 타건 커버리지 공백**(unbind `keybinding:0` 은 resolver 리프로로 무혐의 실증). 팝업 트리거 재바인딩 시 그 modifier+화살표를 위젯 탐색에 바인딩하는 **동반 규칙**(+parameter hints 양보 가드) | `src/shared/lib/monaco-keybinding.ts` |
| `39c17fc` | **기능 확장 3차**: Hot Exit(미러 복원 IPC 7종·untitled 비휘발성화·baseline mtime 충돌→ConflictBanner·CloseRequested 인터셉트+⌘Q 메뉴 교체·디바운스 500ms·블러/언마운트 플러시·GC) / Remote 비밀번호(2요소 게이트·keyring salt+sha256·세션 7일 만료·지수 백오프·JS 없는 로그인 페이지 3언어·fail-closed·**WS epoch 즉시 단절**·원격 settings_update 게이트 필드 스트립) | `src-tauri/src/domain/{file,remote}/`, `src/app/providers/hot-exit-flush-provider.tsx`, `src-tauri/src/domain/remote/login_page.rs`, `src/features/settings/remote-password-row.tsx` |
| `c675d6f` | IconButton 레이아웃 회귀 수정 — 툴팁 span 래퍼가 flex 자식이 되어 위치 클래스 무효(동종 5곳). `containerClassName`(위치=래퍼/시각·상태=버튼) 분리 + 래퍼 기본 `shrink-0` | `src/shared/ui/icon-button.tsx` 외 4 |
| `06b0397` | PROCESS 기록 | docs |

각 웨이브는 "계약 → 스파인 → 구현 병렬(파일 소유권 분리) → 렌즈 검토 → 적대적 검증 → 수정 → 메인 2차(verify+실측)" 패턴으로 진행됐고, 검토 웨이브가 잡은 확정 결함(경로 탈출·keyring fail-open·⌘Q 인터셉트 우회·탭 전환 편집 소실·상태바 CPU% 붕괴 등)은 전부 수정 후 커밋됨.

### 3.2 진행 중

**없음.** 전 작업 커밋·푸시 완료, verify 그린.

### 3.3 미착수 / 대기

1. **실기 QA 일괄**(사용자만 실행 가능) — `docs/quality-assurance/2026-08-11-qa6-checklist.md`:
   W1~W7 원본 + "QA6 후속 1차" + "기능 확장 1차"(10) + "기능 확장 2차"(3) + "기능 확장 3차"(13)
   + suggest 키 항목. W7 CSP 는 하이라이팅 동작 확인됨(devtools 콘솔 무오류 확인만 잔여).
2. **backlog**(`docs/backlog.md`) — 갭 P0 대형(Code Action/Quick Fix 가 최대 공백, 3-way 머지,
   태스크 러너, 터미널 OSC133, Semantic Tokens, Breadcrumbs, 팔레트 @/: 모드, Workspace Symbol,
   git 줄 단위 stage·커밋 상세·파일 히스토리, chord/when 키맵), 프로젝트 스코프 원격 세션,
   Remote 하드닝 minor 9건, Hot Exit 미세 3건, monaco IME 상류 대응, shift+기호 캡처.
3. **Phase 8(서명·공증)** — 사이드카(`taide-cli`)가 tauri bundler 로 함께 서명되는 것은 확인됐으나
   **공증 파이프라인은 미구성**(`tauri.conf.json` 에 macOS 서명 섹션 없음).
4. `docs/PROCESS.md` 아카이브 — 1,145줄(규정 ~300줄 초과). 완료 섹션을 `docs/history/` 로 이전 필요.

### 3.4 알려진 미검증 (KNOWN ISSUE)

- **이번 세션 신기능 전량 실기 미검증** — 기계 검증(verify·vite build)만. 특히:
  suggest 화살표 수정의 전제("⌥ 를 유지한 채 타건")는 소스 논증이며 실기 미확증 —
  ⌥ 뗀 ↑ 도 새면 별개 원인(제안 1건 시 ↑=커서 이동은 VS Code 동일 스펙임을 사용자에게 안내).
- CLI `taide <경로>` 실동작은 **번들 빌드에서만** 확인 가능(`bun run tauri build` — dev 는 의도적 거부).
- OMLX FIM 은 실서버·코더 모델로만 검증 가능. Remote 로그인 왕복·WS 즉시 단절도 실기 몫.
- 이전 세션 잔여: W1~W7 실기, 번들 테마 10종 색 육안, W6 원격 실접속, Windows 컴파일.

## 4. 의사결정 요약 (상세·기각 대안은 acknowledge 계약 3건이 정본)

| 계약 문서 | 핵심 결정 |
|-----------|-----------|
| `docs/acknowledge/2026-08-12-qa6-followup-contract.md` | IME 2건 판정(떨림=폰트/자모 분리=WKWebView 층·수정 불가→backlog), 단축키 A안(actionId string 확장), OMLX FIM 필수(chat-only 기각 — 사용자 질책 "tab 기능이 안 되면 무슨 의미"), i18n 재렌더가 raw key 원인(키 추가 아님), 기각 8건 |
| `docs/acknowledge/2026-08-13-feature-expansion-contract.md` | blame IViewZone(CodeLens·오버레이 기각), 액션 B안(실행 monaco — A안은 chord 기본값 표현 불가로 기각), 하이브리드 i18n, CLI 열기 배선 포함(심링크만 = "설치 성공 착시" 기각), 갭 P0 퀵윈만 편입, 기각 12건 |
| `docs/acknowledge/2026-08-14-hotexit-remote-password-contract.md` | Hot Exit 전체 패키지(untitled 포함·0-손실), Remote 2요소+keyring sha256(+salt)·7일 만료·fail-closed, **프로젝트별 비밀번호는 전역 1개만**(스코프 세션은 10배 규모·계약 번복이라 backlog, 암호만 나누는 안은 보안 착각 유발로 기각), 기각 7건 |

세션 중 추가 판정(계약 외): 자모 분리 monaco 업그레이드 불가(0.56.0 이 최신·상류 수정 0건 —
`docs/bug/2026-08-12-editor-korean-ime.md`), suggest 화살표의 unbind 규칙 무혐의(resolver 실로드 리프로),
Alt 캡처 정규화는 "code 무조건 우선" 아닌 "합성 문자만 code"(비US 레이아웃 보호 — 에이전트 정제를 채택).

## 5. 사용자 방향성 & 작업 규칙

### 5.1 운영 방식 (역할 5단 — 기존 유지 + 이번 세션 추가분)

- 오케스트레이팅·계약·2차 검토 = 메인 / 리서치·최종검증 = opus+medium / 구현 = sonnet+high /
  버그 1차 = opus+high. **입력 관련 민감 버그(IME·키바인딩)는 fable+high**(사용자 지정, 이번 세션 3회 적용).
- 서브에이전트 위임은 규모 무관 Workflow 로만. 패턴: 계약(스파인 단일) → 구현 웨이브(파일 소유권
  분리, **Rust 는 한 시점 한 에이전트** — enum variant 선추가로 컴파일 깨진 사고 1회, settings 필드는
  sync/service.rs 리터럴까지) → 렌즈 검토(보안 렌즈 포함) → 적대적 검증 → 수정 → 메인 2차.
- 에이전트 보고 불신·핵심 주장 메인 실물 재검증(이번 세션: monaco 소스·npm·react-i18next·
  WS revoke 코드 등 매회 수행 — 실결함 추가 발견 2회).
- **확인 질문은 추천안 패키지로 묶어** "전부 추천안대로" 한마디로 답할 수 있게(사용자가 매회 그렇게 답함).
- **반쪽 기능 금지** — 설치는 되는데 안 열리는 CLI, FIM 없는 auto-tab 같은 "동작 착시"에 사용자가
  강하게 반응. 기능은 끝까지 배선한다.

### 5.2 답변·코드 규칙 (기존 유지)

- 한국어+존댓말·간결·이모지 금지. 검증 안 된 단언 금지. 보고만 하고 멈추지 말 것.
- arrow fn만·반환 타입 명시 금지·any/enum 금지·주석 금지(영어 JSDoc 만)·매직넘버 금지·
  useCallback/useMemo 금지·삼항 2중첩 금지·named export·1파일 1컴포넌트·FSD 위→아래·barrel 금지.
- i18n 4곳 동기(locale/service.rs — en/ko/ja+MESSAGE_NAMESPACES, en⊆required 역방향 테스트가 강제),
  IPC 숫자 f64(i64/u64/usize 금지), Rust 후 cargo fmt, @shikijs/langs 배럴 import 금지.
- 시크릿·비밀번호는 keyring 에만(settings.json 은 gist 동기화됨 — 화이트리스트 None 필수 확인).

### 5.3 금지 사항 (기존 + 추가)

- 에이전트 셸에서 앱 실행 금지(`bun run tauri dev` 는 사용자만). main 커밋 금지(dev 자동 커밋·푸시 ON).
- git add -A·force push·Co-Authored-By·.env 금지. HACK·검사기 끄기(#[allow] 포함) 금지.
- 신규 패키지 임의 설치 금지. 승인 누적분: reqwest·keyring·sha2·flate2·tar·zip·xz2·axum·
  @shikijs 4종·monaco-editor-core(dev). **argon2 는 미승인**(원하면 승격 — 저장 형식 호환 설계됨).
- createHighlighter(WASM) 금지·CSP wasm-unsafe-eval 금지. 계약들의 기각 대안 재론 금지.
- TAIDE 레포를 앱에서 열고 테스트 타이핑 금지(전역 ⌘S 우발 저장으로 소스 오염 사고 1회 — 복원함).

## 6. 미해결 질문 / 사용자 확인 필요

1. **실기 QA 결과 전반**(§3.3-1) — 특히 Hot Exit 재시작 복원, Remote 로그인 왕복·비밀번호 변경 시
   웹 화면 즉시 단절, ⌥Space 후 ⌥ 유지 화살표, blame footer 체감, 설정 버튼 위치 복구.
2. 자모 분리 실험 옵션(`accessibilitySupport: 'off'` — 1줄·가역) 적용 시도 여부.
3. QA 후 다음 방향: backlog P0(추천 1순위 = Code Action/Quick Fix) vs Phase 8(서명·공증).
4. 번들 테마 10종 색 변경 육안 수용(이월 — `git show fddb584:src-tauri/resources/themes/<id>.json`).

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS(arm64), bun 1.3.x. cargo 는 PATH 밖 — `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"` (zsh 변수명 `path` 금지) |
| 실행 | dev = `bun run tauri dev`(사용자만). **프로덕션 빌드 = `bun run tauri build`**(신규 `scripts/tauri.ts` 가 CLI 사이드카 빌드+`tauri.bundle.conf.json` 자동 합성 — externalBin 을 기본 conf 에 넣으면 dev 컴파일이 깨져 분리함) |
| 검증 | `bun run verify` = typecheck→lint→format→bun test→cargo fmt/clippy/test. bindings 재생성은 `cargo test`(`typescript_바인딩을_생성한다`) |
| 기준선(2026-08-14 실측) | 프론트 **472 tests**(46 files) / Rust **635**(612 lib+6+17) / 커맨드 **148**(JSON 145+RAW 3 — 디스패치 파리티 테스트 강제) / 이벤트 24 / `src-tauri/binaries/` 는 gitignore(사이드카 산출물) |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` — buffers/(미러)·themes/·locales/·plugins/·lsp/·prompts/ |
| 스크래치패드(세션 소멸) | 정찰·검토 보고서 원문은 세션 스크래치패드에만 있음 — **요지는 전부 계약·PROCESS 에 반영됨**, 새 세션은 문서만 믿으면 됨 |

## 8. 다음 세션 TODO (우선순위 순)

1. **실기 QA 지원** — 사용자가 `bun run tauri dev`(+CLI 는 `bun run tauri build`)로 체크리스트 진행.
   버그 보고 시: 입력 관련 = fable+high, 그 외 1차 = opus+high 워크플로 → 메인 2차.
2. QA 통과 후 사용자 지시에 따라: backlog P0(갭 분석 `docs/research/2026-08-13-vscode-cursor-gap.md`
   §9 의 10건 — Code Action 이 단일 최대 공백) 또는 Phase 8(서명·공증 — 사이드카 공증 미구성 유의).
3. `docs/PROCESS.md` 아카이브(1,145줄 → 완료 섹션을 `docs/history/` 로).

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | 시간순 체크리스트 — "QA6 후속 1차"·"기능 확장 1~3차" 절에 이번 세션 상세 |
| `docs/acknowledge/2026-08-12-qa6-followup-contract.md` | 후속 1차 계약(결정·기각 8) |
| `docs/acknowledge/2026-08-13-feature-expansion-contract.md` | 확장 1차 계약(결정·기각 12·tier1 42 목록) |
| `docs/acknowledge/2026-08-14-hotexit-remote-password-contract.md` | 확장 3차 계약(결정·기각 7) |
| `docs/research/2026-08-13-vscode-cursor-gap.md` | **VS Code·Cursor 갭 분석**(P0/P1/우위/비추천) |
| `docs/bug/2026-08-12-editor-korean-ime.md` | IME 2건 판정·상류 조사·실험 옵션 |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | **실기 체크리스트 전체**(W1~W7+후속+확장 1~3차) |
| `docs/memory/tooltip-conventions.md` | 툴팁 side 규범·IconButton 사용법 |
| `docs/backlog.md` | P0 대형·스코프 세션·하드닝 잔여 등 후속 후보 전량 |
| `docs/ipc-contract.md` | 커맨드 정본 — 이번 세션 신규(미러·CLI·usage·remote 비밀번호) 반영 완료 |
| `docs/features/editor.md` | Hot Exit 미러 규약(재작성됨) |
| `docs/theme-system.md`·`data-model.md`·`PRD.md`·`architecture.md`·`roadmap.md`·ADR | 기존 정본 |

## 10. 복기 신뢰도

- **높음**: 커밋 8건이 전부 dev 에 고정, 계약 3건·PROCESS·ipc-contract 가 웨이브마다 동기 갱신됨.
  §7 수치는 세션 말 실측.
- **중간**: 각 웨이브 내부 구현 디테일(예: WS epoch 구독 지점, FIM 센티널 문자열)은 문서보다 코드가
  정본 — 재론 시 해당 파일을 직접 읽을 것.
- **낮음**: 없음. 단, 정찰·검토 보고서 원문(스크래치패드)은 세션 소멸로 접근 불가 — 요지는 계약에 있음.
