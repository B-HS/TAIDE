# HANDOFF — 2026-08-12 세션 스냅샷

> 최종 갱신: 2026-08-12 / 대응 커밋: **`0846b8e`** (`dev`, origin 푸시 완료)
> 이 문서는 세션 인수인계 **단일 진입점**이다. 새 세션은 이것부터 읽는다.
> 수치는 이 문서 작성 시점에 재실측했다(§7).

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유하고(ADR-0004), view 는 표시 전용이다.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J 전량 구현 → Phase 8 배포(서명·공증) |
| 현재 마일스톤 | **Phase 7.10 — QA 5차 신규 요구 8그룹**을 7개 웨이브(W1~W7)로 구현 중 |
| 직전 작업 | **W6(remote-control) 완료·커밋·푸시**. **다음은 W7(TextMate 문법 엔진) 착수** |

**진행률**: W1 ✅ · W2 ✅ · W3 ✅ · W4 ✅ · W5 ✅ · **W6 ✅** · **W7 ⏳ 미착수(다음)**

## 3. 완료 / 진행 중 / 미착수

### 3.1 이번 세션에서 완료 (W6, 커밋 `0846b8e`)

**W6 — remote-control**: 동일 React 번들을 Rust HTTP 서버(axum)로 서빙하고, 브라우저에서
`window.__TAURI_INTERNALS__` shim 으로 Tauri IPC 를 WebSocket 브리지로 대체한다. 데스크톱과 상태 완전 공유.

| 영역 | 핵심 파일 |
|------|-----------|
| axum 서버(1회용 링크 토큰→HttpOnly 쿠키 승격·Origin/Host 화이트리스트·graceful shutdown) | `src-tauri/src/domain/remote/server.rs`, `service.rs`, `commands.rs`(RemoteStore·remote_status/start/stop/issue_link/revoke_sessions) |
| 커맨드 **136종** 직접 디스패치 테이블(macro 없음, 원본 fn 직접 호출) + bindings 파리티 테스트 | `src-tauri/src/domain/remote/dispatch.rs` |
| WS 루프·채널 브리지(search/lsp JSON·pty 바이너리 프레임, index·end-guard 재현) | `src-tauri/src/domain/remote/ws.rs` |
| 서빙(prod `asset_resolver`·dev vite 프록시·CSP override·convertFileSrc 대체 Range 파일 라우트, root_guard 화이트리스트) | `src-tauri/src/domain/remote/serving.rs` |
| WS 프로토콜 타입·상수 | `src-tauri/src/domain/remote/types.rs` |
| `constant_time_eq` 공용 승격 | `src-tauri/src/infra/crypto.rs`(+ `infra/mod.rs`, `domain/ide/service.rs` 재노출) |
| 이벤트 23종 `listen_any` 팬아웃(구독자 있을 때만) + 부트 자동시작 + Exit stop 배선 | `src-tauri/src/lib.rs`, `events.rs`(RemoteStateChanged) |
| settings.remote_access_enabled 토글(부트/토글 시 서버 start/stop) | `src-tauri/src/domain/settings/{types,service,commands}.rs`, `sync/service.rs`(동기 페이로드 제외) |
| locale remote 네임스페이스(13키: 12 + securityWarning) | `src-tauri/src/domain/locale/service.rs`(en/ko/ja 4곳 동기) |
| 프론트 shim(self-gate·WS invoke/channel/event·convertFileSrc·dialog no-op) | `src/shared/lib/remote/{tauri-internals-shim,remote-ws-client,callback-registry,remote-json}.ts`, `src/main.tsx`(first-import) |
| 설정 REMOTE 섹션(토글·상태·접속 수·링크 발급·세션 폐기) | `src/features/settings/remote-section.tsx`, `src/entities/remote/{remote.ipc,remote.query}.ts`, `src/widgets/settings-view/settings-view.tsx`, `src/shared/constants/query-key.ts` |

### 3.2 진행 중

**없음.** W6 은 완료·커밋·푸시됐다. 워킹트리는 이 HANDOFF/PROCESS 문서 갱신만 남는다.

### 3.3 미착수 / 대기

- **W7 — TextMate 문법 엔진** (§8 1순위). 현재 syntax 하이라이팅은 monaco 0.56 내장 Monarch +
  테마 변환이 VSCode tokenColors 를 축소된 고정 SYNTAX_TOKENS 집합으로 매핑
  (`src/shared/lib/theme-convert/resolve-syntax.ts`). W7 은 이를 TextMate 엔진으로 교체해
  (1) 전체 TextMate scope 토큰화, (2) 테마의 전체 tokenColors 보존. **방침**: shiki JS 엔진 우선
  검증(CSP `script-src 'self'` 무변경), 실패 시 vscode-oniguruma WASM + CSP `wasm-unsafe-eval` 폴백.
  테마 스키마에 원본 tokenColors 보존 필드 추가 → 번들 36종 재변환(원본 재다운로드는 메인 직접 —
  `THIRD_PARTY_LICENSES.md` 출처 URL). ADR-0010(Monarch 전제)·`docs/features/plugins.md`·
  `docs/theme-system.md` 개정 동반. **정찰 리서치부터 시작한다**(§8 참조).
- **QA 6회차 실기 검증** — W1~W7 전량을 사용자가 한 번에 실기 검증한다. 체크리스트:
  `docs/quality-assurance/2026-08-11-qa6-checklist.md`(W1~W6 항목 누적, W7 완료 시 추가).

### 3.4 알려진 미검증 (KNOWN ISSUE)

- **W1~W6 전량 실기 미검증** — 에이전트 셸 앱 실행 금지로 기계 검증(build·clippy·test·typecheck·
  lint·bun test·vite build)만 통과. 실토큰·실네트워크·실기 렌더는 QA 6회차 몫.
- **W6 remote-control 실기 미검증** — WS 브리지·채널·이벤트 팬아웃·서빙·shim 은 기계 검증만.
  특히 원격 브라우저 실접속·pty 스트리밍·미디어 Range·쿠키 승격·cloudflare tunnel(HTTPS)은 QA6.
- **W6 원격 dev 서빙** — dev 는 vite(5173) 리버스 프록시 경유(HMR 미지원 정상). vite allowedHosts
  거부 여부·shim 로드 순서는 실측 전.
- **LSP 미서명 다운로드 바이너리의 Gatekeeper 정책 미검증**(QA6 항목).
- **Codex/Gemini hooks 실왕복 미검증**(배지 3상태 전이는 실 에이전트로만 확인 — QA6).
- **Windows 코드 컴파일 미검증**(`#[cfg(windows)]`). macOS arm64 만 검증됨.

## 4. 의사결정 요약

상세는 `docs/acknowledge/`(결정 1건 = 파일 1개).

### 4.1 W6 채택 + 이유

| 결정 | 이유 |
|------|------|
| remote = 동일 React 번들 서빙 + `__TAURI_INTERNALS__` shim + WS-IPC 브리지, 전권 | 프론트 교체 지점 1곳(shim). ADR-0004 로 상태를 Rust 가 이미 소유. VS Code Server 와 동형 |
| 커맨드 디스패치 = macro 없이 원본 fn 직접 호출 + bindings 파리티 테스트 | tauri 매크로가 원본 fn 재방출. IPC 계층 흉내 불필요. 파리티는 bindings.ts 정규식 대조로 드리프트 차단 |
| 136종 전부 디스패치(remote_* 5 포함) | 인증된 동일-상태-공유. remote_* 도 브라우저에서 호출 가능(무해·인증됨), 파리티 단순화 |
| 채널 end 신호 = sink 클로저가 소유한 end-guard Drop | `Channel::new` 은 on_drop=None. Channel 드롭 시 클로저·guard 드롭 → `{end,index}` 프레임 전송으로 데스크톱 규약 재현 |
| 이벤트 팬아웃 = `listen_any` 무수정 | tauri `Emitter::emit` 이 Rust 리스너에도 배달. emit 커맨드 무수정 |
| 인증 = 1회용 URL 토큰 → HttpOnly 쿠키 승격 + Origin/Host 화이트리스트 | LAN 평문은 secure context 아님(Secure 쿠키 불가) → 짧은 TTL·Origin 검사로 보완. 외부는 사용자가 cloudflare tunnel(HTTPS)로 직접 |
| shim = 프론트 번들 first-import + self-gate(`!window.__TAURI_INTERNALS__`) | metadata 는 호출 시점 읽음 → 번들 first-import 로 충분. 데스크톱은 이미 존재 → no-op(무영향). 별도 bootstrap.js 라우트 불필요 |
| 서빙 = prod asset_resolver / dev vite 프록시 | dev(devUrl)는 asset_resolver 가 `../dist` 를 읽어 스테일 → vite 프록시로 최신 서빙(HMR 포기) |
| 원격 다이얼로그 = shim 이 null(취소) 반환 | 호스트에 네이티브 다이얼로그 띄우면 원격 요청 hang. 원격 파일 피커는 backlog |

### 4.2 참조 정본

- W6 구현 계약 전문: `docs/acknowledge/2026-08-12-w6-remote-contract.md` (소스 검증 사실·설계·원격 UX 정책)
- W6 이전 QA5 배치 결정: `docs/acknowledge/2026-08-11-qa5-batch-decisions.md`
- Codex/위임/이전 결정: `docs/acknowledge/*.md`, `docs/feedback/*.md`

## 5. 사용자 방향성 & 작업 규칙

### 5.1 운영 방식 (역할 5단)

| 역할 | 담당 |
|------|------|
| 오케스트레이팅·상세플랜·계약 확정·배선 판단·2차 검토 | 메인 세션 |
| 리서치·판단·최종 검증 | opus + medium |
| 구현 | sonnet + high |
| 버그 1차 검토·해결 | opus + high |

- **메인은 직접 구현하지 않는다.** 예외: 취약한 네트워크 다운로드(체크섬 등), 2차 검토의 소규모 수정.
- **서브에이전트 위임은 규모와 무관하게 Workflow 도구로만.** 단일 작업도 agent() 1개 워크플로로
  (effort 지정·검토 단계 유지). Agent 도구 직접 사용 금지.
- 검증된 워크플로 패턴: **계약(스파인 단일) → 구현 웨이브(파일 소유권 분리 병렬, 공유 파일은 직렬) →
  검토(렌즈별 탐색 → 적대적 검증 → 수정) → 메인 2차(verify 재실행 + 실측)**.
- 스파인 파일(lib.rs·events·locale/service.rs·settings·bindings·Cargo.toml)은 한 시점에 한 에이전트만.
- **에이전트 보고는 검증 후 채택.** 외부 API 가능성은 사용자 기존 구현·실제 엔드포인트 확인 후 판정.

### 5.2 답변·보고 스타일

- 한국어+존댓말, 간결, 미사여구·자축 금지, 이모지·아스키아트 금지.
- 검증 안 된 "완벽/잘 됨" 단언 금지. 보고만 하고 턴을 끝내지 말 것 — goal 이 있으면 이어서 진행.

### 5.3 코드 규칙 (ESLint 강제 + 파리티 테스트)

- arrow fn만, 반환 타입 명시 금지, any/enum 금지, **코드 주석 금지(JSDoc 영어만 — Rust `///` 도 금지)**,
  매직넘버 금지, useCallback/useMemo 금지, effect 동기 setState 금지, named export, 1파일 1컴포넌트,
  FSD 위→아래, barrel 금지, 색은 시맨틱 토큰만(테마 JSON 데이터 파일은 예외)
- **i18n 키는 4곳 동기**: `domain/locale/service.rs` 의 MESSAGE_NAMESPACES + en/ko/ja (파리티 테스트)
- **테마 토큰은 5곳 동기**: theme/service.rs + theme-tokens.ts + global.css + docs
- IPC 타입에 i64/u64/usize 금지 (u32/f64)
- **Rust 수정 후 반드시 `cargo fmt`**
- 시크릿(토큰)은 keyring 에만. settings.json·로그·IPC 응답·이벤트·에러 문자열에 비노출.

### 5.4 금지 사항

- main 직접 커밋 금지 (dev 에서 작업, **자동 커밋·푸시 ON — dev 한정**, main 머지는 지시 시)
- git add -A 금지(선별 스테이징), force push 금지, Co-Authored-By 금지, .env 접근 금지
- HACK·검사기 끄기(`#[allow]`·@ts-ignore·eslint-disable) 금지
- 신규 패키지 임의 설치 금지(승인분: reqwest·keyring·sha2·flate2·tar·zip·xz2·**axum**. W7 신규 프론트
  의존(shiki 등)은 도입 시 사용자 보고)
- **에이전트 셸에서 앱 실행 금지**, Chrome 을 localhost:5173 에 붙인 채 두지 말 것

## 6. 미해결 질문 / 사용자 확인 필요

1. **QA 6회차 실기 재검증 결과** (§8) — W1~W7 전량. 특히 W6 원격 실접속·auto-tab·Gist 동기화·hooks 배지·LSP 설치.
2. W7 착수 시 신규 프론트 의존(shiki 등) 도입 확정 — 엔진 선택(shiki JS vs oniguruma WASM)은 정찰 리서치 결과로 판단 후 보고.
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

**현재 기준선 (2026-08-12 재실측)**
- 프론트 **358 tests** / Rust **538 tests**(515 lib + 6 통합 + 17 CLI)
- IPC 커맨드 **133종**(collect_commands, remote_* 5 포함), 이벤트 **23종**, RAW 채널 커맨드 **3종**
  (pty_spawn·pty_attach·file_read_raw) → 원격 디스패치 대상 **136종**
- Rust 도메인 **21개**(신설: remote), 번들 테마 **36종**, LSP 매니페스트 **19종**
- 승인 의존성: reqwest·keyring·sha2·flate2·tar·zip·xz2·**axum**(W6 도입)
- b-hub 레퍼런스(읽기 전용): `~/development/b-hub/service/domain/ai/providers/codex-provider.ts`

## 8. 다음 세션 TODO (우선순위 순)

### 1순위 — W7 TextMate 문법 엔진 (미착수)

- **정찰 리서치부터 시작한다** (opus+medium 워크플로 3영역):
  1. **shiki+monaco 통합·CSP** — `@shikijs/monaco`(shikiToMonaco)로 monaco 0.56 토크나이저 교체 방법,
     shiki JS regex 엔진(`@shikijs/engine-javascript`)이 CSP `script-src 'self'`(no unsafe-eval/
     wasm-unsafe-eval) 하에서 동작하는지, WASM 엔진의 CSP 요구, 번들 임팩트. context7 로 공식 확인.
  2. **테마 파이프라인·raw tokenColors 보존** — 현재 `domain/theme/{types,service}.rs`·
     `src/shared/lib/theme-convert/*`·`code-editor.tsx`·`theme-provider.tsx` 흐름, Theme 스키마에
     원본 tokenColors 보존 필드 설계, 번들 36종 재변환 범위(`scripts/convert-vscode-theme.ts`).
  3. **grammar 소싱·VSIX·문서 개정** — `@shikijs/langs` 언어 커버리지 vs TAIDE 지원 언어, VSIX
     `contributes.grammars` 경로(이번 범위 포함 여부), ADR-0010/plugins.md/theme-system.md 개정 목록.
- 이후 W1~W5 와 동일한 **계약 → 스파인 → 구현 웨이브 → 검토 → 메인 verify** 패턴.
- **완료 조건**: 검증(build·clippy·test·typecheck·lint·bun test·vite build) 통과 후 커밋,
  QA6 체크리스트에 W7 항목 추가.
- **관련 파일**: `src/features/editor/code-editor.tsx`, `src/app/providers/theme-provider.tsx`,
  `src/shared/lib/theme-convert/*`, `src-tauri/src/domain/theme/*`, `src-tauri/resources/themes/*.json`,
  `scripts/convert-vscode-theme.ts`, `docs/theme-system.md`, `docs/adr/0010-*.md`, `docs/features/plugins.md`,
  `THIRD_PARTY_LICENSES.md`, `src-tauri/tauri.conf.json`(CSP).

### 2순위 — QA 6회차 일괄 실기 검증

- `docs/quality-assurance/2026-08-11-qa6-checklist.md` 를 사용자가 `bun run tauri dev` 로 검증.
  W1~W6 항목 누적됨(W6 16항목 포함). 버그는 역할 분담대로 수정.

### 3순위 — backlog·Phase 8

- `docs/backlog.md`: Gemini IDE companion, Codex app-server 패널, chord/when 키맵 엔진, 원격 파일 피커,
  앱데이터 파일 에디터 편집.
- `docs/roadmap.md` Phase 8(서명·공증) — QA 6회차 통과 후에만.

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | Phase 0~7.10 시간순 체크리스트. 7.7 이후 구현 순서 정본. W1~W6 상세 완료 기록 |
| `docs/acknowledge/2026-08-12-w6-remote-contract.md` | **W6 구현 계약** (소스 검증 사실·WS 프로토콜·인증·서빙·원격 UX 정책) |
| `docs/acknowledge/2026-08-11-qa5-batch-decisions.md` | QA 5차 전 웨이브 결정 집약 |
| `docs/acknowledge/2026-08-11-*.md` | 웨이브별 세부 결정(LSP·GitHub sync 등) |
| `docs/feedback/*.md` | 교정 리포트(상황별 적용) |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | **W1~W7 일괄 실기 검증 체크리스트**(W1~W6 누적) |
| `docs/features/remote-control.md`·`docs/research/remote-control.md` | remote-control 기획·기술 검토(잠자기 제약 등) |
| `docs/features/vsix-theme-import.md` | VSIX 테마 임포트 (W5) |
| `docs/features/agent-integration.md` | Claude·Codex·Gemini hooks·IDE 연동 |
| `docs/theme-system.md` | 테마 토큰·번들 36종·변환 스크립트·폴백·§9 임포트 (**W7 에서 개정 대상**) |
| `docs/adr/0010-*.md` | 플러그인 선언적 매니페스트(Monarch 전제 — **W7 에서 개정 대상**) |
| `docs/features/plugins.md` | 플러그인 매니페스트 정본 (**W7 에서 개정 대상**) |
| `THIRD_PARTY_LICENSES.md` | 번들 테마 36종 + 다운로드 LSP 출처·라이선스 |
| `docs/PRD.md`·`architecture.md`·`tech-stack.md`·`ipc-contract.md`·`data-model.md`·`adr/`·`roadmap.md`·`backlog.md` | 기존 정본 |

## 10. 복기 신뢰도

- **높음**: W6 은 커밋 `0846b8e` 로 고정, 전 산출물을 파일 경로 단위로 기록. §7 수치는 작성 시점 재실측
  (프론트 358·Rust 538·커맨드 133·이벤트 23·RAW 3·디스패치 136·도메인 21·테마 36·LSP 19).
- **중간**: W6 내부 로직(디스패치 136암·WS 채널 브리지·shim 프로토콜·서빙 Range)은 기계 검증(파리티·
  컴파일·타입·빌드)으로 확인했으나 실기 렌더·실네트워크는 QA 6회차 몫.
- **낮음**: 없음.
