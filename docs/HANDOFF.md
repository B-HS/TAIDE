# HANDOFF — 2026-08-12 세션 스냅샷 (W7 완료)

> 최종 갱신: 2026-08-12 / 대응 커밋: **`bc27352`** (`dev`, origin 푸시 완료)
> 이 문서는 세션 인수인계 **단일 진입점**이다. 새 세션은 이것부터 읽는다.
> 수치는 이 문서 작성 시점에 재실측했다(§7).

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유하고(ADR-0004), view 는 표시 전용이다.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J 전량 구현 → Phase 8 배포(서명·공증) |
| 현재 마일스톤 | **Phase 7.10 — QA 5차 신규 요구 8그룹** 7개 웨이브(W1~W7) **전량 구현 완료** |
| 직전 작업 | **W7(TextMate 문법 엔진) 완료·커밋·푸시**. **다음은 QA 6회차 일괄 실기 검증(사용자)** |

**진행률**: W1 ✅ · W2 ✅ · W3 ✅ · W4 ✅ · W5 ✅ · W6 ✅ · **W7 ✅** → **QA6 실기 검증 대기**

## 3. 완료 / 진행 중 / 미착수

### 3.1 이번 세션에서 완료 (W7, 커밋 `bc27352`)

**W7 — TextMate 문법 엔진**: monaco 0.56 내장 Monarch + 31토큰 축소 매핑을 **shiki 4.4.3
(JS RegExp 엔진, CSP `script-src 'self'` 무변경)** 로 교체. 테마 스키마에 원본 tokenColors
보존 필드 추가, 번들 36종 재변환(원본 재취득), 플러그인 grammar 기여를 TextMate 로 재정의·실배선.

| 영역 | 핵심 파일 |
|------|-----------|
| shiki 통합(수명 관리: 원본 setTheme/create 캡처→복원→shikiToMonaco 재호출, 단일 테마명 'taide', init 실패 재시도, reinit swap-then-dispose) | `src/shared/lib/shiki/shiki-monaco.ts` |
| 언어 매핑 31종(재명명 방식 — langAlias 는 core 미적용, heex→html 폴백, 배럴 import 금지) | `src/shared/lib/shiki/lang-map.ts` |
| 테마 조립(colors=buildThemeColors 재사용, tokenColors=raw??31토큰 역생성 폴백, syntaxOverrides 오버레이 — 스코프 선점 소유 규칙 공유) | `src/shared/lib/shiki/build-shiki-theme.ts` |
| monaco 31종 언어 선등록·applyMonacoTheme 제거(빌더 함수는 폴백용 유지) | `src/shared/lib/monaco/{setup,theme}.ts` |
| 테마 스키마: `Theme`/`ResolvedTheme.token_colors: Option<Vec<TokenColorRule>>`(fontStyle 원문·순서 보존·extends 는 자식 Some=전체 교체/None=상속), `ResolvedTheme.syntax_overrides`(**base 있는 자식만** — 검토에서 잡은 치명 결함 수정) | `src-tauri/src/domain/theme/{types,service}.rs` |
| 변환 파이프라인 tokenColors passthrough(fontStyle/background 원문 보존·전역 룰 보존·빈 settings 제외)·CLI prettier 출력·경고 2종 | `src/shared/lib/theme-convert/{types,merge,convert}.ts`, `scripts/convert-vscode-theme.ts` |
| 번들 36종 재변환(+tokenColors). 26종은 기존 3절 diff 0, **10종은 색 일부 변경 — 원인 규명 후 신규 값 채택**(W5 include 체인 순서 결함 수정 4종·상류 갱신·미기록 출처. `docs/PROCESS.md` W7-C) | `src-tauri/resources/themes/*.json` |
| 플러그인 grammar: TextMate 재정의(manifestVersion 1 유지)·검증 3에러코드(grammar-missing/invalid/conflict, 충돌은 해당 grammar 만 무효)·`plugin_read_grammar` IPC·프론트 조립(개별 실패 스킵·embeddedLangs 화이트리스트)·reload 시 reinitShiki | `src-tauri/src/domain/plugin/*`, `src/entities/plugin/{plugin-grammar,plugin.ipc,plugin.query}.ts` |
| VSIX 임포트·테마 에디터 프리뷰의 tokenColors 보존 | `src/features/theme/vsix-theme-import.ts`, `src/entities/theme/theme.query.ts` |
| 디스패치 137종(+plugin_read_grammar)·locale 3키(en/ko/ja)·bindings 재생성 | `src-tauri/src/domain/remote/dispatch.rs`, `domain/locale/service.rs`, `src/shared/api/bindings.ts` |
| 문서 개정 9건 + 신설: W7 계약·shiki 리서치·원본 취득 매니페스트·QA6 W7 16항목 | `docs/acknowledge/2026-08-12-w7-textmate-contract.md`, `docs/research/shiki.md`, `docs/utils/2026-08-12-w7-theme-original-sources.md`, ADR-0010·plugins.md·theme-system.md·vsix-theme-import.md·ipc-contract.md·research/monaco.md·backlog.md·`THIRD_PARTY_LICENSES.md`(Bundled TextMate Grammars 절) |

신규 의존(사용자 승인 2026-08-12): `@shikijs/core`·`@shikijs/engine-javascript`·`@shikijs/langs`·
`@shikijs/monaco` 4.4.3 고정 + `monaco-editor-core@0.56.0`(devDep, 타입 전용).

### 3.2 진행 중

**없음.** W7 완료·커밋·푸시됨.

### 3.3 미착수 / 대기

- **QA 6회차 일괄 실기 검증**(§8 1순위) — W1~W7 전량, `docs/quality-assurance/2026-08-11-qa6-checklist.md`.
  W7 16항목 추가됨. **W7 최우선 항목 = CSP 무변경 실기 동작**(실패 시 W7 무효 — 정적 검증만 완료 상태).
- backlog: VSIX contributes.grammars 임포트, `.tf`→terraform/`.mdx`→mdx 정밀화(W7 에서 분리),
  Gemini IDE companion, Codex app-server 패널, chord/when 키맵, 원격 파일 피커 등(`docs/backlog.md`).

### 3.4 알려진 미검증 (KNOWN ISSUE)

- **W1~W7 전량 실기 미검증** — 기계 검증(verify·vite build)만 통과. 실기는 QA6 몫.
- **W7 특유**: WKWebView 런타임 CSP 동작(정적 grep 만), shiki 교체 후 TS/JSON worker 리치 기능 유지,
  embedded language(erb 등) 정확도, 첫 페인트·타이핑 체감, 테마 에디터 프리뷰 연타 성능,
  monaco 몽키패치 수명 관리(단위 테스트 불가 — 실기), 플러그인 grammar 실물 왕복.
- **번들 10종 색 변경**(W7-C) — 사용자 육안 확인 전. 되돌리려면 `git show fddb584:src-tauri/resources/themes/<id>.json`.
- W6 원격 실접속·pty·Range·쿠키 승격·tunnel, LSP Gatekeeper, Codex/Gemini hooks 실왕복, Windows 컴파일 — 이전과 동일.

## 4. 의사결정 요약

상세는 `docs/acknowledge/`(결정 1건 = 파일 1개). **W7 구현 계약 전문:
`docs/acknowledge/2026-08-12-w7-textmate-contract.md`** (확정 사실·사용자 결정 4건·설계·
**§3.7 기각된 대안 21건**·위험 — 리서치 원문은 세션 소멸, 이 문서가 유일 기록).

### 4.1 W7 채택 + 이유

| 결정 | 이유 |
|------|------|
| shiki 4.4.3 JS RegExp 엔진, WASM 폴백 없음 | dist 실물 grep eval/new Function 0건 → CSP `script-src 'self'` 무변경. WASM 은 `wasm-unsafe-eval` 필요(보안 후퇴). 31언어 전부 JS 엔진 호환 |
| `createHighlighterCore` 강제, `createHighlighter` 금지 | 메타 패키지 기본 엔진이 oniguruma WASM — 실수 방지 리뷰 관문 |
| 몽키패치 수명 관리 = 원본 복원 후 shikiToMonaco 재호출, 단일 테마명 | shikiToMonaco 는 호출 시점 스냅샷 + setTheme 패치 중첩. loadTheme 동일 이름 교체(Map.set) 실측으로 성립 |
| tokenColors = `Vec<TokenColorRule>`(fontStyle 원문), extends 는 전체 교체/상속 | 룰 순서가 의미. BTreeMap·키 병합 불가. 테마 에디터 diff 저장과 정합 |
| syntax_overrides 는 base 있는 자식 테마만 | 루트/번들에 채우면 31토큰 오버레이가 raw 를 도로 덮음(검토 웨이브가 잡은 치명 결함) |
| heex→html 폴백 / 회색 라이선스 4종 포함+고지 / 플러그인 grammar 실배선 / VSIX grammars 는 backlog | 사용자 결정(계약 §2). grammar 30종 개별 서브패스 import 로 GPL 유입 차단 |
| 번들 10종 색 변경 채택 | 격리 실험(구변환기+신원본=동일 diff)으로 변환기 무혐의 입증. Dark+/Modern 은 실 VS Code 렌더 일치·파일트리 선택 불가시 버그 수정. 출처는 이번에 문서화 |

## 5. 사용자 방향성 & 작업 규칙

### 5.1 운영 방식 (역할 5단)

| 역할 | 담당 |
|------|------|
| 오케스트레이팅·상세플랜·계약 확정·배선 판단·2차 검토 | 메인 세션 |
| 리서치·판단·최종 검증 | opus + medium |
| 구현 | sonnet + high |
| 버그 1차 검토·해결 | opus + high |

- **메인은 직접 구현하지 않는다.** 예외: 취약한 네트워크 다운로드(체크섬·원본 취득), 2차 검토의 소규모 수정.
- **서브에이전트 위임은 규모와 무관하게 Workflow 도구로만.** 단일 작업도 agent() 1개 워크플로로.
- 검증된 워크플로 패턴: **계약(스파인 단일) → 구현 웨이브(파일 소유권 분리 병렬, 공유 파일은 직렬) →
  검토(렌즈별 탐색 → 적대적 검증 → 수정) → 메인 2차(verify 재실행 + 실측)**. W7 에서도 검토 웨이브가
  치명 결함 1건 포함 실결함 5건을 잡음 — 이 패턴 유지.
- 스파인 파일(lib.rs·events·locale/service.rs·settings·bindings·Cargo.toml)은 한 시점에 한 에이전트만.
- **에이전트 보고는 검증 후 채택.** 핵심 주장은 메인이 실물(소스·tarball·레지스트리)로 재확인.

### 5.2 답변·보고 스타일

- 한국어+존댓말, 간결, 미사여구·자축 금지, 이모지·아스키아트 금지.
- 검증 안 된 "완벽/잘 됨" 단언 금지. 보고만 하고 턴을 끝내지 말 것 — goal 이 있으면 이어서 진행.

### 5.3 코드 규칙 (ESLint 강제 + 파리티 테스트)

- arrow fn만, 반환 타입 명시 금지, any/enum 금지, **코드 주석 금지(JSDoc 영어만 — Rust `///` 도 금지)**,
  매직넘버 금지, useCallback/useMemo 금지, named export, 1파일 1컴포넌트, FSD 위→아래, barrel 금지,
  색은 시맨틱 토큰만(테마 JSON 데이터 파일 예외)
- **i18n 키는 4곳 동기**: `domain/locale/service.rs` MESSAGE_NAMESPACES + en/ko/ja (파리티 테스트)
- **테마 토큰은 5곳 동기**(tokenColors 는 토큰 집합이 아니라 비대상), IPC 타입에 i64/u64/usize 금지
- **Rust 수정 후 반드시 `cargo fmt`**, `@shikijs/langs` 배럴 import 금지(개별 서브패스만 — GPL 차단)
- 시크릿(토큰)은 keyring 에만. settings.json·로그·IPC 응답·이벤트·에러 문자열에 비노출.

### 5.4 금지 사항

- main 직접 커밋 금지 (dev 에서 작업, **자동 커밋·푸시 ON — dev 한정**, main 머지는 지시 시)
- git add -A 금지(선별 스테이징), force push 금지, Co-Authored-By 금지, .env 접근 금지
- HACK·검사기 끄기(`#[allow]`·@ts-ignore·eslint-disable) 금지
- 신규 패키지 임의 설치 금지(승인분: reqwest·keyring·sha2·flate2·tar·zip·xz2·axum + **@shikijs 4종·
  monaco-editor-core(W7 승인)**)
- **에이전트 셸에서 앱 실행 금지**, Chrome 을 localhost:5173 에 붙인 채 두지 말 것
- VSCode extension "그대로 실행" 금지(테마/문법 임포트만), MS Marketplace 연동 금지

## 6. 미해결 질문 / 사용자 확인 필요

1. **QA 6회차 실기 검증**(§8) — W1~W7 전량. W7 은 CSP 실기 동작이 최우선(체크리스트 첫 항목).
2. 번들 10종 색 변경(§3.4) 육안 수용 여부 — QA6 스팟 확인 항목에 포함됨.
3. 추가 테마 4종(Winter is Coming 등) 확충 여부(이월).

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS (arm64). Windows/Linux 미검증 |
| 패키지 매니저 | bun 1.3.x |
| **cargo 경로** | PATH 에 없음 — `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"` |
| **셸 주의** | zsh 에서 스크립트 변수명 `path` 사용 금지(PATH 연동 예약 변수 — 세션 중 1회 사고) |
| 리모트 | origin=github.com/B-HS/TAIDE (비공개). main=prod, dev=개발. 자동 커밋·푸시 ON(dev) |
| 실행 | `bun run tauri dev` (**사용자만** — 에이전트 셸 금지) |
| 검증 | `bun run verify` = typecheck→lint→format:check→bun test→cargo fmt/clippy/test |
| bindings 재생성 | `cargo test`(lib.rs 의 `typescript_바인딩을_생성한다`)가 `src/shared/api/bindings.ts` 생성 |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` (themes/·locales/·plugins/·lsp/·prompts/ 하위) |

**현재 기준선 (2026-08-12 W7 후 재실측)**
- 프론트 **392 tests** / Rust **550 tests**(527 lib + 6 통합 + 17 CLI)
- IPC 커맨드 **134종**(collect_commands, plugin_read_grammar 포함), 이벤트 **23종**, RAW 채널 **3종**
  → 원격 디스패치 대상 **137종**
- Rust 도메인 **21개**, 번들 테마 **36종**(전부 tokenColors 보유), LSP 매니페스트 **19종**
- 프론트 의존 신규: @shikijs/core·engine-javascript·langs·monaco 4.4.3 / monaco-editor-core 0.56.0(dev)
- 번들 테마 원본 재취득 매니페스트: `docs/utils/2026-08-12-w7-theme-original-sources.md`

## 8. 다음 세션 TODO (우선순위 순)

### 1순위 — QA 6회차 일괄 실기 검증 (사용자 실기)

- `docs/quality-assurance/2026-08-11-qa6-checklist.md` 를 사용자가 `bun run tauri dev` 로 검증.
  W1~W7 누적(W7 16항목 — **첫 항목 CSP 가 최우선**, 실패 시 W7 재설계 필요).
- 버그 발견 시 역할 분담대로: 1차 검토·해결 = opus+high 워크플로, 메인 2차.

### 2순위 — QA6 통과 후

- `docs/backlog.md` 후보 착수 또는 `docs/roadmap.md` Phase 8(서명·공증) — 사용자 지시에 따름.

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 단일 진입점 |
| `docs/PROCESS.md` | Phase 0~7.10 시간순 체크리스트. W1~W7 상세 완료 기록(W7-C 재변환 판정 포함) |
| `docs/acknowledge/2026-08-12-w7-textmate-contract.md` | **W7 구현 계약**(확정 사실·사용자 결정 4건·설계·위험) |
| `docs/acknowledge/2026-08-12-w6-remote-contract.md` | W6 구현 계약 |
| `docs/acknowledge/2026-08-11-qa5-batch-decisions.md` | QA 5차 전 웨이브 결정 집약 |
| `docs/research/shiki.md` | shiki 패키지 구성·CSP 검증·shikiToMonaco 동작·수명 관리 설계·대안 비교 |
| `docs/utils/2026-08-12-w7-theme-original-sources.md` | 번들 36종 원본 취득 URL 매니페스트(재변환 재현) |
| `docs/quality-assurance/2026-08-11-qa6-checklist.md` | **W1~W7 일괄 실기 검증 체크리스트** |
| `docs/theme-system.md` | 테마 토큰·번들 36종·변환·tokenColors(W7 개정 완료) |
| `docs/features/plugins.md` | 플러그인 매니페스트 정본(TextMate grammar — W7 개정 완료) |
| `docs/adr/0010-plugin-system.md` | 선언적 매니페스트 + W7 보강 항 |
| `THIRD_PARTY_LICENSES.md` | 테마 36종 + LSP + **TextMate grammar 30종**(W7 신설 절) |
| `docs/PRD.md`·`architecture.md`·`tech-stack.md`·`ipc-contract.md`·`data-model.md`·`adr/`·`roadmap.md`·`backlog.md` | 기존 정본 |

## 10. 복기 신뢰도

- **높음**: W7 은 커밋 `bc27352` 로 고정, 전 산출물 파일 경로 단위 기록. §7 수치는 verify 실행 결과
  (프론트 392·Rust 550·커맨드 134·디스패치 137·테마 36 전부 tokenColors). 번들 10종 색 변경은
  격리 실험으로 원인 규명됨.
- **중간**: W7 내부 로직(shiki 수명 관리·오버레이·플러그인 grammar)은 기계 검증(파리티·컴파일·테스트·
  빌드)까지. 실기 렌더·CSP 런타임은 QA6 몫.
- **낮음**: 없음.
