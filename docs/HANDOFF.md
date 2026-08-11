# HANDOFF — 2026-08-11 세션 스냅샷

> 최종 갱신: 2026-08-11 / 대응 커밋: **`a0df11e`** (`dev`, origin 푸시 완료, 워킹트리 깨끗함)
> 이 문서는 세션 인수인계 **단일 진입점**이다. 새 세션은 이것부터 읽는다.
> 수치는 이 문서 작성 시점에 **재실측**했다(§7).

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유하고(ADR-0004), view 는 표시 전용이다.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J 전량 구현 → Phase 8 배포(서명·공증) |
| 현재 마일스톤 | **Phase 7.10 — QA 5차 신규 요구 8그룹**을 7개 웨이브(W1~W7)로 구현 중 |
| 직전 작업 | **W1~W5 완료·커밋·푸시** (+ zls 반영). **다음은 W6(remote-control) 착수 직전에 세션 종료** |

**진행률**: W1 ✅ · W2 ✅ · W3 ✅ · W4 ✅ · W5 ✅ · **W6 ⏳ 미착수** · **W7 ⏳ 미착수**

## 3. 완료 / 진행 중 / 미착수

### 3.1 이 세션에서 완료 (전부 커밋·푸시됨, `34cf0ee..a0df11e`)

QA 5차 = 사용자 신규 요구를 7웨이브로 분해. 웨이브별 결과와 핵심 파일:

**W1 — UI/UX·단축키·dead-end·테마 플래시** (`34578b0`)
| 내용 | 핵심 파일 |
|------|-----------|
| 헤더 1px 정렬(explorer 1행 36px)·메뉴바 하단 전폭 border·border 토큰 tabBar 통일 | `widgets/explorer/explorer-panel.tsx`, `widgets/app-shell/app-shell.tsx`, `shared/constants/layout.ts` |
| 줄번호 여백 축소(glyphMargin false·minChars 3)·설정 좌측 패딩 축소 | `features/editor/code-editor.tsx`, `widgets/settings-view/settings-view.tsx` |
| 파일트리 hover 강화·`explorer.itemFocused` 첫 배선·결함 테마 8종 VSCode 공식 list 색 폴백 | `features/explorer/file-tree-row.tsx`, `shared/styles/global.css`, `scripts/convert-vscode-theme.ts`, `resources/themes/*` |
| 테마 reload 플래시 수정(`data-theme-ready` 게이트 + `visibility`) | `app/providers/theme-provider.tsx`, `shared/styles/global.css` |
| 단축키: 죽은 바인딩 8종 실핸들러(브리지 패턴)·⌘=/⌘- 폰트·팔레트 활성화·⌘S 중복/2-pane 저장 수정 | `shared/lib/keymap.ts`, `shared/lib/{explorer-panel,editor-pane-command}-bridge.ts`, `entities/editor/editor-instance-registry.ts` |
| dead-end: `system_open_app_data_path`·`git_init` IPC, 플러그인 목록 실렌더, 폴더 열기·외부 앱·복사·재시도 토스트, 플러그인 오류 코드화 | `domain/system/commands.rs`, `domain/git/{service,commands}.rs`, `features/settings/plugin-list.tsx` |

**W2 — LSP 원클릭 설치** (`5416689`, zls 추가 `71558c5`)
| 내용 | 핵심 파일 |
|------|-----------|
| 서버 스펙 데이터화(enum→문자열 newtype + `lsp-servers.json`, 기존 5종 무손실) | `domain/lsp/{types,manifest,service}.rs`, `resources/lsp-servers.json` |
| 설치 인프라(스트리밍 다운로드·증분 sha256·tar.gz/zip/gz/tar.xz 해제·zip-slip 방어·원자 설치·버전 수치 정렬) | `infra/lsp_install.rs` |
| IPC `lsp_install`/`lsp_install_cancel` + `lsp:install-progress`, 탐지 확장(전략·설치 버전·툴체인/SDK) | `domain/lsp/commands.rs` |
| 신규 14종(다운로드 8: jdtls·clangd·expert(alpha)·lua-ls·taplo·terraform-ls·kotlin-lsp·zls / 툴체인 4: gopls·ruby-lsp·metals·hls / SDK 2: dartls·sourcekit-lsp), 확장자 맵 21종 | `resources/lsp-servers.json`, `domain/file/service.rs` |
| 설정 UI 설치 버튼·진행률·취소·툴체인/SDK 안내 | `features/settings/lsp-server-status-list.tsx` |

**W3 — AI Provider·auto-tab·GitHub 동기화** (`76da94c`)
| 내용 | 핵심 파일 |
|------|-----------|
| keyring 시크릿 인프라(토큰은 OS 키체인에만, settings·로그·IPC 응답 비노출) | `infra/secret.rs` |
| Ollama Cloud provider(FIM `/api/generate`+suffix, chat 폴백) | `domain/ai/providers/ollama.rs` |
| **Codex provider — b-hub `codex-provider.ts` 를 Rust 이식** (whoami 계정 검증, `/responses` SSE 조립, 폴백 모델 7종) | `domain/ai/providers/codex.rs` |
| auto-tab ghost text(디바운스 300ms·취소·캐시·LSP 간섭 방지), 설정 AI 섹션 | `shared/lib/ai/inline-completion.ts`, `features/settings/*`, `features/editor/code-editor.tsx` |
| 프롬프트 템플릿 번들 + `{app_data}/prompts/` 오버라이드 | `domain/ai/prompt.rs`, `resources/prompts/auto-tab-default.json` |
| GitHub 동기화(secret Gist + PAT, 화이트리스트 페이로드, schemaVersion 게이트, 충돌 2택) | `domain/sync/{types,service,github,commands}.rs`, `features/settings/sync-section.tsx` |

**W4 — Codex·Gemini hooks 배지 3상태** (`2870140`)
| 내용 | 핵심 파일 |
|------|-----------|
| `taide-cli hook --url` shim(stdin→127.0.0.1 HTTP, 1.5s 타임아웃, 항상 exit 0·stdout 무오염) | `crates/taide-cli/src/main.rs` |
| hooks URL 에 agent 축, override `(ProjectId, agentName)` 키 전환, 에이전트별 이벤트 매핑 | `domain/agent/{types,service,hooks,commands}.rs` |
| 사용자 레벨 주입기(`~/.codex/hooks.json`·`~/.gemini/settings.json` type:command 멱등 주입·항목 보존·OFF 자동 제거·재시작 시 URL 자가 치유) | `domain/agent/{service,commands,hooks}.rs` |
| 설정 UI 에이전트별 행(사용자 레벨 경고·CLI 미설치 시 비활성) | `features/settings/agent-hooks-*.tsx` |

**W5 — VSIX 테마 임포트 + hooks 파일 안전성** (`d966efc`)
| 내용 | 핵심 파일 |
|------|-----------|
| `vsix_extract_themes`(zip 추출·include 체인 순환 차단·항목 2MiB/총량 64MiB 상한·경로 탈출 방어) | `domain/vsix/{types,service,commands}.rs` |
| 테마 변환 로직 CLI→shared 공유화(9모듈, monokai 재변환 diff 0)·tokenColors leaf 우선 병합 | `shared/lib/theme-convert/*`, `scripts/convert-vscode-theme.ts` |
| 임포트 UI(추출→후보 목록→선택 저장, id 충돌 시 확인 후 새 id 사본 — 무단 덮어쓰기 금지) | `features/theme/vsix-theme-import*.{ts,tsx}` |
| hooks 파일 안전성(파싱 실패 비파괴 거부·기존 권한 보존·신규만 0600·임시파일 권한 노출창 제거) | `domain/agent/commands.rs`, `infra/persist.rs`(`write_atomic_with_mode`) |

### 3.2 진행 중

**없음.** 모든 커밋이 푸시됐고 워킹트리는 깨끗하다. **W6 착수 직전에 멈췄다.**

### 3.3 미착수 / 대기

- **W6 — remote-control** (§8 1순위). 계약(HTTP 서버·`__TAURI_INTERNALS__` shim·WS-IPC 브리지·커맨드 디스패치 테이블)은 이미 상세 설계돼 있다(§4·acknowledge). **다음 한 줄**: W6 워크플로를 W1~W5 와 동일한 계약→구현→검토 패턴으로 띄운다. `docs/features/remote-control.md`·`docs/research/remote-control.md` 참조.
- **W7 — TextMate 문법 엔진** (§8 2순위). shiki JS 엔진(CSP 무변경) 우선 검증, 테마 스키마에 원본 tokenColors 보존, 번들 36종 재변환(원본 재다운로드는 메인 직접). backlog 에서 승격됨(`34cf0ee`).
- **QA 6회차 실기 검증** — W1~W7 전량을 사용자가 한 번에 실기 검증한다(§8). 체크리스트: `docs/quality-assurance/2026-08-11-qa6-checklist.md`(W1~W5 항목 누적됨, W6·W7 완료 시 추가).

### 3.4 알려진 미검증 (KNOWN ISSUE)

- **W1~W5 전량 실기 미검증** — 사용자 지시로 W7 까지 논스톱 구현 후 QA 6회차에 일괄 검증. 에이전트 검토 + 메인 2차 verify 는 통과했으나 실토큰·실네트워크·실기 렌더는 미확인.
- **LSP 미서명 다운로드 바이너리의 Gatekeeper 정책 미검증** — 실설치 시 실행 차단 여부 확인 필요(QA6 항목).
- **W7(TextMate) 미착수** — shiki JS 엔진의 grammar throw 리스크는 실측 전 추측 영역.
- **Codex/Gemini hooks 실왕복 미검증** — 배지 3상태 전이는 실 에이전트로만 확인 가능(QA6).
- **Windows 코드 컴파일 미검증** (`#[cfg(windows)]` — agent·pty·hooks shim 셸 인용). macOS arm64 만 검증됨.
- one-monokai foreground 안전 기본값(이전 세션 이월).

## 4. 의사결정 요약

상세는 `docs/acknowledge/`(결정 1건 = 파일 1개). 이 세션 핵심은 **`2026-08-11-qa5-batch-decisions.md`**(전 웨이브 결정 집약).

### 4.1 채택 + 이유 (이 세션)

| 결정 | 이유 |
|------|------|
| **W3~W7 논스톱, 검증은 W7 후 일괄** (사용자 확정) | 실기 검증 없이 구현을 쌓되 각 웨이브 검토+메인 2차로 품질 유지 |
| **auto-tab = Ollama Cloud + Codex access token 2종** | 사용자 원 요구 유지. Codex 는 b-hub 검증 구현 이식(§4.2 정정) |
| **remote-control = 동일 React 번들 서빙 + `__TAURI_INTERNALS__` shim + WS-IPC 브리지, 전권** | 프론트 교체 지점 1곳(실측). 비용은 Rust 커맨드 디스패치 테이블. VS Code Remote 와 동형 |
| **멀티 에이전트 = 감지 배지 3종 + hooks 브리지 3상태** | Codex·Gemini 공식 hooks 존재. 사용자 레벨 주입만(프로젝트 파일 오염 금지) |
| **extension = "테마 임포트"로 명명, 코드 실행형 미지원 공식 선언** | extension host 실행은 VSCode 재구현급(212파일 3MB). VSIX 정적 자산 추출만 현실적 |
| **TextMate 문법 엔진을 W7 로 승격** (backlog→정식) | 없으면 VSIX 임포트가 반쪽. 부수효과로 번들 36종 테마 syntax 색 절반이 살아남 |
| **dead-end Edit = 폴더 열기 커맨드**(`system_open_app_data_path`) | 에디터 탭 내 편집(C안)은 file_save 가드+탭 종속 동시 해제 필요라 backlog |
| **hooks 파일 안전성 = 파싱 실패 비파괴 거부·권한 보존** | 서드파티 소유 파일(`~/.codex`·`~/.gemini`)을 덮어쓰거나 권한 변경하면 안 됨 |
| **VSIX id 충돌 = 확인 후 새 id 사본** | 무단 덮어쓰기 금지를 "확인 후 덮어쓰기"보다 강하게 충족 |
| **zls = xz2 승인 + 다운로드형** (사용자 A안) | 다른 서버와 동일 UX. 자산 실측(sha256 `b93ec549…`)은 메인 직접 |
| **서브에이전트 위임은 규모 무관 Workflow 로만** | Agent 도구는 effort 지정 불가·검토 단계 누락(feedback 기록) |

### 4.2 기각·정정 (**같은 삽질 반복 금지**)

| 기각·정정 | 이유 |
|-----------|------|
| ~~"Codex access token 으로 completion 불가"~~ (리서치 오판) | **정정됨.** b-hub `service/domain/ai/providers/codex-provider.ts` 가 `chatgpt.com/backend-api/codex/responses`(SSE) + `at-` 토큰 + `chatgpt-account-id` + `originator: codex_cli_rs` 로 이미 운용 중. 비공식 프로토콜(깨질 수 있음)로 인지하고 이식. `docs/feedback/2026-08-11-codex-token-feasibility-misjudgment.md` |
| VSCode extension "그대로 실행" | extension host(Node 프로세스)+vscode API 21,235줄 재구현 = 불가. 정적 자산만 |
| MS Marketplace 연동 | 약관상 서드파티 제품 접근 금지. 로컬 .vsix 만(Open VSX 는 후속) |
| Codex IDE 연동(터미널→에디터 발견) | 프로토콜 방향이 반대(클라이언트가 app-server spawn). backlog(app-server 패널) |
| 테마 플래시 = 초기화 스크립트 주입 | 테마 변경 후 reload 시 직전 테마 플래시. `data-theme-ready` 게이트가 정답 |
| hooks 파일 주석 허용 파서(comment-json)·직접 스트리퍼 | 신규 패키지 금지 + 스트리퍼는 문자열 리터럴 오염 해크. 엄격 JSON 거부가 정답 |
| Agent 도구로 소규모 위임 | effort 지정 불가·검토 누락. Workflow 로만(feedback 기록) |
| (기존 유지) IME getTargetRanges·휴리스틱 awaitingInput·opener 광역 권한·pty 배지 즉시전이·TS7·Vec<u8> 채널·shadcn resizable·xterm 교체·Tauri 버전업 IME·TabKind Preview | 이전 HANDOFF 참조 — 전부 유효 |

## 5. 사용자 방향성 & 작업 규칙

### 5.1 운영 방식 (역할 5단 — 이전 세션 확정, 유지)

| 역할 | 담당 |
|------|------|
| 오케스트레이팅·상세플랜·계약 확정·배선 판단·2차 검토 | 메인 세션 (Fable) |
| 리서치·판단·최종 검증 | opus + medium |
| 구현 | sonnet + high |
| 버그 1차 검토·해결 | opus + high |

- **메인은 직접 구현하지 않는다.** 예외: fable 지정 디버그, **취약한 네트워크 다운로드(체크섬 계산 등)**, 2차 검토의 소규모 수정.
- **서브에이전트 위임은 규모와 무관하게 Workflow 도구로만** 한다. 단일 작업도 agent() 1개짜리 워크플로로(effort 지정·검토 단계 유지). Agent 도구 직접 사용 금지. (`docs/feedback/2026-08-11-use-workflow-not-agent-tool.md`)
- 검증된 워크플로 패턴: **계약(스파인 단일 에이전트) → 구현 웨이브(파일 소유권 분리 병렬, 공유 파일은 직렬) → 검토(렌즈별 탐색 → 적대적 검증 → 수정) → 메인 2차(verify 재실행 + 실측)**.
- 스파인 파일(lib.rs·events·locale/service.rs·settings·bindings·Cargo.toml)은 **한 시점에 한 에이전트만**.
- **에이전트 보고는 검증 후 채택.** 이 세션 실사례: W1~W5 검토에서 확정 결함 40건+ 를 잡았고, 메인 2차에서 W1 잔여 테마 결함(리뷰도 놓친 것)을 전수 스캔으로 추가 발견.

### 5.2 답변·보고 스타일

- 한국어+존댓말, 간결, 미사여구·자축 금지, 이모지·아스키아트 금지.
- 검증 안 된 "완벽/잘 됨" 단언 금지. 보고만 하고 턴을 끝내지 말 것 — goal 이 있으면 이어서 진행.
- **버그·판정은 추측 금지** — 이 세션 교훈: Codex "불가" 오판은 사용자 기존 자산(b-hub)을 확인하지 않아 발생. 외부 API 가능성은 사용자 기존 구현·실제 엔드포인트 확인 후 판정.

### 5.3 코드 규칙 (ESLint 강제 + 파리티 테스트)

- arrow fn만, 반환 타입 명시 금지, any/enum 금지, **코드 주석 금지(JSDoc 영어만 — Rust `///` 도 금지)**,
  매직넘버 금지, useCallback/useMemo 금지, effect 동기 setState 금지, named export, 1파일 1컴포넌트,
  FSD 위→아래, barrel 금지, 색은 시맨틱 토큰만(테마 JSON 데이터 파일은 예외)
- **i18n 키는 4곳 동기**: `domain/locale/service.rs` 의 MESSAGE_NAMESPACES + en/ko/ja (파리티 테스트)
- **테마 토큰은 5곳 동기**: theme/service.rs(네임스페이스·dark·light) + theme-tokens.ts + global.css + docs
- IPC 타입에 i64/u64/usize 금지 (u32/f64)
- **Rust 수정 후 반드시 `cargo fmt`**
- 시크릿(토큰)은 keyring 에만. settings.json·로그·IPC 응답·이벤트·에러 문자열에 비노출.

### 5.4 금지 사항

- main 직접 커밋 금지 (dev 에서 작업, **자동 커밋·푸시 ON — dev 한정**, main 머지는 지시 시)
- git add -A 금지(선별 스테이징), force push 금지, Co-Authored-By 금지, .env 접근 금지
- HACK·검사기 끄기(`#[allow]`·@ts-ignore·eslint-disable) 금지
- 신규 패키지 임의 설치 금지(승인분: reqwest·keyring·sha2·flate2·tar·zip·xz2. axum 은 W6 승인분 미도입)
- **에이전트 셸에서 앱 실행 금지**(샌드박스·TCC 오진 전례), Chrome 을 localhost:5173 에 붙인 채 두지 말 것

## 6. 미해결 질문 / 사용자 확인 필요

1. **QA 6회차 실기 재검증 결과** (§8) — W1~W7 전량. 특히 auto-tab 실동작·Gist 동기화·hooks 배지·LSP 설치.
2. remote-control 세부(§4 acknowledge 에 메인 확정안 있음 — 세션 라우팅·토큰 승격·디스패치 테이블). 착수 시 재확인 불필요, 그대로 진행.
3. 추가 테마 4종(Winter is Coming 등) 확충 여부(이전 세션 이월).

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS (arm64). Windows/Linux 미검증 |
| 패키지 매니저 | bun 1.3.x |
| **cargo 경로** | PATH 에 없음 — `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"` |
| 리모트 | origin=github.com/B-HS/TAIDE (비공개). main=prod, dev=개발. 자동 커밋·푸시 ON(dev) |
| 실행 | `bun run tauri dev` (**사용자만** — 에이전트 셸 금지) |
| 검증 | `bun run verify` = typecheck→lint→format:check→bun test→cargo fmt/clippy/test |
| bindings 재생성 | `cargo test`(lib.rs 의 `typescript_바인딩을_생성한다`)가 `src/shared/api/bindings.ts` 생성 |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` (themes/·locales/·plugins/·lsp/·prompts/ 하위) |
| b-hub 레퍼런스 | `~/development/b-hub/service/domain/ai/providers/codex-provider.ts` (Codex 이식 원본, 읽기 전용) |

**현재 기준선 (2026-08-11 재실측)**
- 프론트 **358 tests**(37파일) / Rust **약 516 tests**(직전 W5 verify 기준: 493 lib + 6 통합 + 17 CLI)
- IPC 커맨드 **128종**, 이벤트 **22종**, Rust 도메인 **20개**(신설: ai·sync·vsix)
- 번들 테마 **36종**, LSP 매니페스트 **19종**(다운로드 8·툴체인 4·SDK 2 + 기존 5)
- 신규 승인 의존성: reqwest·keyring·sha2·flate2·tar·zip·xz2 (W6 axum 미도입)
- 이 세션 커밋 18개 (`34cf0ee..a0df11e`)

**세션 소멸 주의**: 이 세션 워크플로 scratchpad(리서치 8영역·웨이브별 계약/구현/검토 결과)는 세션 종료로 접근 불가.
요지는 전부 docs(acknowledge·PROCESS·features·feedback·quality-assurance)에 반영돼 있다.

## 8. 다음 세션 TODO (우선순위 순)

### 1순위 — W6 remote-control (미착수, 최대 규모)

- **범위**: 동일 React 번들을 Rust HTTP 서버로 서빙 + `__TAURI_INTERNALS__` shim(프론트 1곳) + WS-IPC 브리지 + 커맨드 128종 디스패치 테이블(macro + collect_commands! 파리티 테스트) + 이벤트 22종 팬아웃 + Channel(pty/search/lsp) 브리지.
- **메인 확정 세부**(acknowledge §2): 앱당 서버 1개 + session-id=프로젝트 매핑 불투명 난수, 1회용 URL 토큰→HttpOnly 쿠키 승격, 데스크톱과 상태 완전 공유, convertFileSrc→HTTP 라우트(Range). axum 도입(승인분).
- **관련 파일**: `docs/features/remote-control.md`, `docs/research/remote-control.md`, `src-tauri/src/domain/ide/server.rs`(WS 서버 선례), `src/shared/api/bindings.ts`(생성물, shim 대상), `src-tauri/src/lib.rs`(collect_commands!).
- **완료 조건**: 계약→구현→검토→메인 verify 통과 후 커밋, QA6 체크리스트에 항목 추가.

### 2순위 — W7 TextMate 문법 엔진

- shiki JS 엔진(`@shikijs/*`, 신규 프론트 의존 — 도입 시 사용자 보고) 우선 검증(CSP 무변경). 실패 시 vscode-oniguruma WASM + CSP `wasm-unsafe-eval`.
- 테마 스키마에 원본 tokenColors 보존 필드 추가 → 번들 36종 재변환(원본 재다운로드는 메인 직접 — `THIRD_PARTY_LICENSES.md` 의 출처 URL).
- `docs/features/plugins.md`·ADR-0010(Monarch 전제) 개정 동반.

### 3순위 — QA 6회차 일괄 실기 검증

- `docs/quality-assurance/2026-08-11-qa6-checklist.md` 를 사용자가 `bun run tauri dev` 로 검증. 버그는 역할 분담대로 수정.

### 4순위 — backlog·Phase 8

- `docs/backlog.md`: Gemini IDE companion, Codex app-server 패널, chord/when 키맵 엔진, 앱데이터 파일 에디터 편집.
- `docs/roadmap.md` Phase 8(서명·공증) — QA 6회차 통과 후에만.

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | Phase 0~7.10 시간순 체크리스트. **7.7 이후 구현 순서 정본**(roadmap 은 7.6 까지). 7.10-W1~W5 상세 완료 기록 |
| `docs/acknowledge/2026-08-11-qa5-batch-decisions.md` | **QA 5차 전 웨이브 결정 집약** (사용자 확정 8건 + 메인 세부 + Codex 정정) |
| `docs/acknowledge/2026-08-11-lsp-w2-*.md`, `github-sync-domain-*.md`, `lsp-install-ui-*.md` | 웨이브별 세부 결정 |
| `docs/feedback/2026-08-11-codex-token-feasibility-misjudgment.md` | Codex "불가" 오판 — 외부 API 판정 시 사용자 기존 자산 확인 |
| `docs/feedback/2026-08-11-use-workflow-not-agent-tool.md` | 위임은 Workflow 로만 (Agent 도구 금지) |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | **W1~W7 일괄 실기 검증 체크리스트** (W1~W5 항목 누적) |
| `docs/features/vsix-theme-import.md` | VSIX 테마 임포트 (W5) |
| `docs/features/agent-integration.md` | Claude·Codex·Gemini hooks·IDE 연동 (§7.5 다중 에이전트, §7.6 파일 안전성) |
| `docs/features/remote-control.md`·`docs/research/remote-control.md` | **W6 착수 시 필독** |
| `docs/bug/2026-08-06-wkwebview-ime-composition.md` | IME 정본 (이전 세션) |
| `docs/theme-system.md` | 테마 토큰·번들 36종·변환 스크립트·§8.2 폴백·§9 임포트 |
| `THIRD_PARTY_LICENSES.md` | 번들 테마 36종 + 다운로드 LSP 7종 출처·라이선스 |
| `docs/PRD.md`·`architecture.md`·`tech-stack.md`·`ipc-contract.md`·`data-model.md`·`adr/`·`roadmap.md`·`backlog.md` | 기존 정본 |

## 10. 복기 신뢰도

- **높음**: 세션 전체가 커밋 18개로 고정, 각 웨이브 결정을 acknowledge/PROCESS/features 에 즉시 기록하며 진행. §7 수치는 작성 시점 재실측(프론트 358·커맨드 128·이벤트 22·도메인 20·테마 36·LSP 19).
- **중간**: 워크플로 에이전트가 구현한 세부 내부 로직(디스패치·SSE 파서·설치 인프라·변환 모듈) — 검토 웨이브+메인 verify 로 확인했으나 실기 렌더·실네트워크는 QA 6회차 몫.
- **낮음**: 없음.
