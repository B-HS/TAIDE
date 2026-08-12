# W7 TextMate 문법 엔진 구현 계약 (2026-08-12)

> 정찰 리서치(wf_e7ad421e-c8b, opus+medium 3영역: shiki+monaco/CSP · 테마 파이프라인 · grammar/문서) 결과를
> 메인이 npm tarball·로컬 monaco 소스·레포 코드로 교차검증한 뒤 확정한 구현 계약.
> 상위 결정은 `2026-08-11-qa5-batch-decisions.md`, 직전 계약은 `2026-08-12-w6-remote-contract.md`.
> 리서치 보고서 원문은 세션 스크래치패드(세션 소멸 시 접근 불가) — 요지는 전부 이 문서에 반영.

## 1. 확정 사실 (소스 검증됨 — 메인 직접 재확인)

- **shiki 최신 안정 = 4.4.3** (2026-08-10, 전 패키지 락스텝, MIT). npm registry 직접 조회로 확인.
- **JS RegExp 엔진은 CSP `script-src 'self'` 하에서 정적으로 안전**: `@shikijs/engine-javascript`·
  `oniguruma-to-es`·`@shikijs/core` dist 전체 grep 에서 `eval(`·`new Function(` **0건**(메인 재검증).
  정규식 생성은 `new RegExp`(CSP 비대상). WKWebView 런타임 실측은 QA6.
- `shiki` 메타 패키지의 `createHighlighter` 는 **oniguruma WASM 이 기본 엔진** → 사용 금지.
  반드시 `createHighlighterCore` + `createJavaScriptRegexEngine()` 명시.
- WASM 엔진(`@shikijs/engine-oniguruma`)은 `WebAssembly.instantiate` 사용 → CSP `wasm-unsafe-eval`
  필요. **폴백으로도 두지 않는다**(보안 후퇴 + W6 원격 CSP 파생 완화 동반).
  `@shikijs/langs-precompiled` 는 공식 "not yet supported" 경고(shikijs/shiki#918) → 채택 안 함.
- **`shikiToMonaco`(`@shikijs/monaco` dist 실물, 메인 정독)**:
  - `monaco.languages.setTokensProvider` 로 **highlighter 로드 언어 ∩ monaco 등록 언어**에만 부착.
    `languages.register` 는 하지 않는다 → 미등록 id 는 우리가 선행 register 필수.
  - 테마는 `getLoadedThemes()` 전부를 `defineTheme(테마이름, {base, inherit: false, colors, rules})` 등록.
    `colors` 는 shiki 테마의 `colors` 를 그대로 통과(→ 우리가 monaco colorId 로 채워 넣으면 그대로 반영).
  - `monaco.editor.setTheme`·`create` 를 **몽키패치**하고 말미에 `setTheme(themeIds[0])` 강제 호출.
    패치 클로저가 호출 시점의 `themeMap`(테마→룰) 을 캡처 → **호출 이후 로드된 테마는 토큰 스코프
    매핑이 비어 렌더가 깨진다**. 재호출 시 패치가 중첩된다.
  - `textmateThemeToMonacoTheme` 가 **export** 되어 있다.
  - 토큰화는 색상+fontStyle 로 스코프를 역추적한 근사 스코프를 monaco 에 전달(구조적 한계, 색은 정확).
  - `tokenizeMaxLineLength` 기본 20000 · `tokenizeTimeLimit` 기본 500ms(옵션 조정 가능).
- **`@shikijs/core` `loadTheme` 는 동일 이름이면 Map.set 으로 교체**(primitive dist `Registry.loadTheme`
  실물 확인) → 단일 테마명 전략 성립. `dispose()` 로 highlighter 전체 재생성 가능.
- **monaco 0.56: 미등록 languageId 는 조용히 plaintext 폴백**(`languageService.js`
  `_createAndGetLanguageIdentifier` 실물 확인). basic-languages 전수 대조 결과 TAIDE id 중
  **`typescriptreact`·`javascriptreact`·`jsonc`·`toml`·`shellscript`·`erb`·`heex`·`haskell`·`zig` 9종 미등록**
  = 현재 .tsx/.jsx/.sh/.toml/.jsonc/.erb/.heex/.hs/.zig 는 **하이라이팅이 전혀 없다**(monaco 소스 확정,
  실기 화면은 QA6). monaco 의 `_registerLanguage` 는 기존 id 재호출 시 병합(안전).
- monaco 0.56 basic-languages 는 `registerTokensProviderFactory`(지연 등록)이고, 직접
  `setTokensProvider` 가 factory 보다 우선(`tokenizationRegistry.js`) → shiki 가 Monarch 를 이긴다.
- **`@shikijs/langs` 4.4.3 에 `heex` 없음**(dist 전수, 메인 재확인). 나머지 30종은 전부 존재.
  TAIDE id ↔ shiki id 차이: `typescriptreact`→`tsx`, `javascriptreact`→`jsx`(그 외 동일명).
  `plaintext` 는 shiki 대상 제외(monaco 내장 유지).
- **`@shikijs/monaco` 의 d.mts 가 `monaco-editor-core` 를 import**(deps 에 없음 — devDeps `^0.56.0`)
  → 타입 해결책 필요(메인이 package.json·d.mts 실물 확인).
- grammar 라이선스(tm-grammars 표 + 패키지 실물): 30종 중 MIT 24 · BSD-3(`haskell`) ·
  MPL-2.0(`hcl`) · NOASSERTION(`elixir`) · 상류 LICENSE 공란(`toml`·`yaml`·`erb`).
  **GPL 계열은 30종에 없음**(shiki 전체에는 ada·gnuplot·nginx·org·racket 이 GPL-3.0 →
  `@shikijs/langs` 배럴 import 금지·개별 서브패스 import 강제의 실질 근거).
- **theme-system.md §8.2.2 의 explorer.item* 보정은 이미 변환기 코드에 내장**
  (`mapping-tables.ts:344~360` `derived` + `isUsableListBackground` + `VSCODE_LIST_*_DEFAULT`,
  메인 재확인) → 재변환해도 보정 유실 없음. 문서의 "산출물 직접 보정(재변환 불가)" 서술만 낡음.
- 재변환 원본: 레포에 없음(정책). `THIRD_PARTY_LICENSES.md` 는 레포 단위 URL 만 보유.
  리서치 샘플 확인에서 dracula(소스가 YAML)·primer(경로 변경) 등 **원본 경로 재조사가 선행 필수**.
  VS Code 내장 8종은 include 체인 로컬 미러 파일명 규약(`INCLUDE_ROLE_FILENAME_MAP`) 재현 필요.
- 번들 임팩트(tarball 실측): 코어 합계 ~88KB gzip, 30종 grammar 합집합 ~237KB gzip(37모듈).
  `erb`/`ruby` 가 의존 포함 ~2MB raw 로 최대. `include_str!` 테마 원문 보존 시 바이너리 +0.6~1.1MB 추정.
- 현행 파이프라인: `readVscodeTheme` 이 tokenColors 정규화 시 **fontStyle 원문(underline 등)·background
  를 유실** → raw 보존은 정규화 전 원형 유지 필수. `Theme`/`ResolvedTheme` 는 BTreeMap 기반 —
  tokenColors 는 **순서 보존 Vec** 필수. 테마 에디터는 diff 만 저장(`buildThemeFromDraft`).
  `useThemePreview.setPreview` 가 ResolvedTheme 을 손조립(새 필드 누락 시 타입 에러로 강제 검출).

## 2. 사용자 결정 (2026-08-12 확정)

| # | 결정 | 선택 |
|---|------|------|
| 1 | 신규 의존 | **@shikijs/core·@shikijs/engine-javascript·@shikijs/langs·@shikijs/monaco 4.4.3 고정 + monaco-editor-core@0.56 devDependency(타입 전용)** |
| 2 | heex | **shiki `html` grammar 폴백 매핑**(현 plaintext 대비 순개선) |
| 3 | 라이선스 회색 4종(elixir·toml·yaml·erb) | **번들 포함 + THIRD_PARTY_LICENSES.md grammar 단위 상세 고지**(VS Code·shiki 동일 재배포 관행 근거) |
| 4 | 플러그인 grammar | **W7 에서 실배선까지 포함**(TextMate 재정의 + 검증 + IPC + shiki 주입) |

- 결정 4 의 범위 해석: **VSIX `contributes.grammars` 는 W7 제외(backlog)** — 언어 id 충돌 정책과
  `LANGUAGE_ID_BY_EXTENSION` 런타임화가 선행돼야 하는 별개 축(리서치 권고 그대로).
  플러그인 grammar 실배선만 포함한다.
- `.tf`→`terraform`·`.mdx`→`mdx` grammar 정밀화도 backlog(동작 변경이라 W7 범위 밖).

## 3. 확정 설계

### 3.1 엔진·highlighter 수명 (`src/shared/lib/shiki/`)

- `createHighlighterCore({ themes: [초기 taide 테마], langs: [30종 dynamic import + 플러그인 grammar],
  engine: createJavaScriptRegexEngine() })`. **`shiki` 메타 패키지 미설치** — `@shikijs/core` 등 직접.
- grammar 는 **highlighter 생성 시 전량 로드**(개별 dynamic import — vite 청크 분리, 에디터 모듈 로드
  후 비동기라 첫 페인트 비경합). 파일별 사후 lazy 로드는 하지 않는다(스냅샷 재부착 복잡도 회피).
- **몽키패치 수명 관리**: 최초 1회 `monaco.editor.setTheme`/`create` 원본을 모듈이 보관.
  테마 변경마다 ① `highlighter.loadTheme(buildShikiTheme(resolved))`(단일 이름 `taide` 교체),
  ② 원본 복원, ③ `shikiToMonaco(highlighter, monaco)` 재호출(패치 중첩 방지, themeMap·프로바이더
  재구축, 말미 `setTheme('taide')` 자동 적용). 로드 테마는 항상 `['taide']` 1개.
- 플러그인 다시 읽기(grammar 변경) = highlighter `dispose()` 후 재생성 + 동일 절차.
- 언어 등록: `monaco.languages.register({ id })` 를 TAIDE 31종 전부에 선행 호출(기존 등록 id 는 병합
  무해). TAIDE id→shiki id 매핑 상수(`typescriptreact`→`tsx`, `javascriptreact`→`jsx`,
  `heex`→`html` 폴백, 나머지 동일명, `plaintext` 제외). shiki 미로드 id 는 monaco 기본 동작 유지.
- `setTokensProvider` 는 TAIDE id 로 부착돼야 하므로 highlighter 에는 **TAIDE id 를 lang name 으로**
  등록한다(shiki grammar 모듈을 import 후 `{ ...grammar, name: 'typescriptreact' }` 로 재명명 주입 —
  alias 기능 대신 명시 재명명, 파리티 테스트 가능).

### 3.2 테마 스키마 (Rust `domain/theme`)

```rust
pub struct TokenColorSettings {
    foreground: Option<String>, background: Option<String>, font_style: Option<String>,
}
pub struct TokenColorRule { scope: Vec<String>, settings: TokenColorSettings }
```

- `Theme`·`ResolvedTheme` 에 `token_colors: Option<Vec<TokenColorRule>>`
  (`#[serde(default, skip_serializing_if = "Option::is_none")]` — 기존 파일에 `null` 오염 방지,
  하위호환: 없으면 None). serde_json::Value 는 specta 태그드 enum 불일치로 기각.
- `font_style` 은 **원문 문자열 보존**(`"bold italic underline"` — 기존 SyntaxStyle 2불린으로 접지 않음).
  scope 는 배열로 정규화(string/콤마분리는 변환기가 접음). `name` 필드는 버림(페이로드 절약).
- **extends 병합**: 자식 `None` → base 의 token_colors **그대로 상속**, `Some` → **전체 교체**
  (배열은 키 병합 불가·순서 의미 보존). 테마 에디터 diff 저장과 정합(사용자 테마는 사실상 항상 None).
- `ResolvedTheme` 에 `syntax_overrides: Vec<String>` 추가 — **자식 테마가 스스로 명시한 syntax 키 목록**
  (base 채움 전 원본 키). 프론트 오버레이 합성의 근거.
- 시맨틱 토큰 5곳 동기 **비대상**(colors/syntax/terminal 토큰 집합 불변 — theme-tokens.ts·global.css 무변경).

### 3.3 shiki 테마 조립 (프론트)

- `buildShikiTheme(resolved)` = `{ name: 'taide', type, colors: buildThemeColors(resolved.colors)
  (기존 MONACO_COLOR_SOURCE 매핑 재사용 — shikiToMonaco 가 defineTheme colors 로 그대로 통과) +
  editor.background/foreground, tokenColors: 합성 }`.
- tokenColors 합성: `raw = resolved.tokenColors ?? fallbackFromSyntax(resolved.syntax)` 뒤에
  `syntax_overrides` 에 있는 토큰만 `SYNTAX_SCOPE_CANDIDATES[token]` 스코프로 **append**(오버레이).
  한계 문서화: TextMate 매칭은 specificity 우선이라 넓은 오버레이가 raw 의 더 깊은 스코프를 못 이길
  수 있다(의도된 제약 — docs/theme-system.md 에 명시).
- `fallbackFromSyntax`: 31 토큰 → 룰 역생성. 스코프 중복은 "먼저 등장한 토큰이 소유"(comment vs
  docComment, type 계열 공유 스코프 충돌 방지).
- 기존 `applyMonacoTheme`(defineTheme 'taide')는 **shiki 경로로 대체**. `theme-provider` 는
  `applyShikiTheme(resolved)`(shared/lib 재export 경유) 호출로 교체.

### 3.4 변환 파이프라인·재변환

- `readVscodeTheme`/`VscodeTokenColorRule` 에 `fontStyle` 원문 필드 추가(기존 bold/italic 유지 —
  resolve-syntax 하위호환). `convertVscodeTheme` 반환에 `tokenColors`(스키마 §3.2 형태) 추가.
- CLI `convert-vscode-theme.ts`: output 에 tokenColors, 룰 0건 경고, 비hex foreground 경고,
  완료 로그에 룰 수. VSIX `buildTheme` 에 tokenColors 전달. `toResolvedThemeFromDraft` 에
  token_colors·syntax_overrides 반영(미리보기 정합).
- **재변환 절차(메인 직접 다운로드)**: ① 36종 원본 URL 재조사(§8.1 출처, 깨진 경로는 vsix 대체)
  ② 변환기 선수정 + `bun test` ③ 파일럿 monokai — tokenColors 절 제외 **diff 0 게이트**
  ④ 36종 일괄 + 경고 로그를 §8.1 각주와 대조 ⑤ colors/syntax/terminal 3절 diff 0 확인
  (불일치 = 원본 갱신 여부 규명 후 사용자 보고) + explorer.item* 8건 재현 확인
  ⑥ cargo 번들 전량 테스트(+ tokenColors 비어있지 않음 assert) ⑦ 바이너리 증가분 보고.

### 3.5 플러그인 grammar 실배선

- 매니페스트 `grammar` 필드 의미를 **TextMate(`.tmLanguage.json`)로 재정의**(manifestVersion 1 유지 —
  프론트 소비자 0건 실증, Monarch 경로는 한 번도 동작한 적 없음). `embeddedLanguages: string[]`
  선택 필드 신설. 이 "조용한 계약 변경"의 근거를 본 문서가 기록한다.
- Rust 검증 강화: 파일 실재(`grammarMissing`)·JSON 파싱+`scopeName` 존재(`grammarInvalid`)·
  scopeName/languageId 충돌 선착순+경고(`grammarConflict`)·크기 상한. `PluginErrorCode` 확장 →
  locale `settings.pluginError.*` 3키 추가(**en/ko/ja 4곳 동기**).
- 신규 IPC: `plugin_read_grammar(pluginId, languageId) -> String`(온디맨드 query — 매니페스트 스캔에
  본문 인라인 금지). bindings 재생성 + **remote dispatch.rs 추가(파리티 테스트가 강제) → 디스패치 137종**.
- 프론트: plugin_list 의 language 기여를 highlighter 생성 시 로드(`plugin_read_grammar` fetch →
  `LanguageRegistration` 주입, 실패 플러그인은 스킵+경고). 반영은 "플러그인 다시 읽기" 시
  highlighter 재생성(§3.1) 또는 재시작.

### 3.6 라이선스·문서

- `THIRD_PARTY_LICENSES.md`: 서두 2종→3종 확장, "Bundled TextMate Grammars" 절 신설 —
  (a) shiki 패키지 MIT 고지 (b) 30종 표(TAIDE id/shiki id/상류/라이선스) (c) 비MIT·회색 개별 고지.
  grammar 는 파일 재배포임을 명시(테마의 "값만 추출" 논리와 구분). **MPL-2.0(hcl) grammar 무변형
  방침 명문화**. GPL 유입 방지 = 개별 서브패스 import 강제(배럴 import 금지 — 검토 렌즈 항목).
- 개정 문서: ADR-0010(보강 항 추가 방식), plugins.md(§1·§2·§3·§4 + ReDoS 위험),
  theme-system.md(§4.2 재작성·§8.2 위상 변경·§8.2.2 서술 정정·§9), vsix-theme-import.md(L12·L153
  "W7 로 별도" → backlog 정정), ipc-contract.md(plugin 절 query 추가·에러코드·theme 페이로드),
  research/monaco.md(plaintext 폴백 사실 보강), research/shiki.md 신설, backlog.md(VSIX grammars·
  tf/mdx 행), acknowledge 2건 정정 참조, HANDOFF·qa6-checklist.

## 3.7 기각된 대안 (재론 방지 — 리서치 원문은 세션 소멸로 이 목록이 유일 기록)

| 기각안 | 사유 |
|--------|------|
| oniguruma WASM 엔진(폴백 포함) | CSP `wasm-unsafe-eval` 필요 = 보안 후퇴, W6 원격 파생 CSP 도 동반 완화 |
| `@shikijs/langs-precompiled` | 공식 "not yet supported"(shikijs/shiki#918) + ES2024 v 플래그 필수 |
| `shiki` 메타 패키지 설치 | `createHighlighter` 기본이 WASM — 실수 유입 위험. 개별 4종이 동등 |
| `vscode-textmate`+`vscode-oniguruma` 직접 통합 | WASM 강제 + grammar 수집·monaco 브리지 자체 구현 부담, oniguruma 2023 이후 정체 |
| `monaco-editor-textmate`/`onigasm` 계열 | 2019~2022 이후 유지보수 중단, 0.56 호환 보장 없음 |
| `modern-monaco` | monaco 를 감싸는 상위 프레임워크 — 기존 직접 세팅에 침습적 |
| monaco-editor-core 타입을 tsconfig paths 로 해결 | 타 패키지 내부 경로 의존 — devDep(타입 전용)이 정확 |
| shiki `langAlias` 로 TAIDE id 매핑 | `createHighlighterCore`(객체 주입 경로)에는 미적용 — 소스 확인. 재명명 주입 채택 |
| grammar 파일별 사후 lazy 로드 | shikiToMonaco 가 호출 시점 스냅샷 — 언어마다 재호출·패치 중첩 유발. init 전량 로드 채택 |
| tokenColors 를 `serde_json::Value` 로 | specta 의 Value 는 태그드 enum — serde 실제 직렬화와 불일치한 TS 생성 |
| tokenColors 를 원문 String 으로 | Rust 테스트·검증 불가, 프론트 매회 파싱 |
| extends 병합 = base++child concat | "전부 새로 쓰기" 표현 불가, 편집마다 룰 누적 |
| extends 병합 = scope 키 단위 | TextMate 매칭은 specificity 기반 — 키 병합 의미 불명 |
| syntax 편집 시 raw tokenColors 무효화 | 토큰 1개 편집에 테마 전체가 31토큰 근사로 퇴화 |
| syntax 편집을 raw 룰에 역반영 | 토큰↔scope 1:N — 정확한 역반영 불가. 오버레이 append 채택 |
| 플러그인 manifestVersion 2 상향 | 기존 v1 플러그인의 lsp/theme 기여까지 전멸(`!=` 검사). 필드 의미 재정의 채택 |
| Monarch grammar 필드 병행 유지 | 프론트 소비자 0건(사장)·shiki 전면 교체와 양립 불가 |
| grammar 본문을 plugin_list 응답에 인라인 | 매니페스트 스캔마다 수 MB IPC. 온디맨드 query 채택 |
| VSIX contributes.grammars W7 포함 | `LANGUAGE_ID_BY_EXTENSION` 런타임화 + 언어 id 충돌 정책(테마식 "사본 저장" 불가) 선행 필요 — backlog |
| 재변환 diff 10종을 커밋본(구값)으로 고정 | W5 취득 경로 미기록이라 재현 불가, Dark+/Modern 의 실제 결함(키워드색·파일트리 선택 불가시)을 보존하게 됨 |
| themes:convert 뒤에 npm script 로 prettier 체인 | `bun run` 이 인자를 스크립트 문자열 끝에 붙여 전달 — 체인 시 인자가 prettier 로 샘. 스크립트 내 prettier API 채택 |

## 4. 미결·위험 (구현·QA 중 확인)

- **WKWebView 런타임에서 JS 엔진 CSP 무변경 동작**(정적 확인만 완료) — 구현 직후 사용자 실기 1순위.
- shiki 토크나이저 교체 후 **TS/JSON worker 리치 기능(진단·완성) 유지** — QA6.
- embedded language(erb=html+ruby, html 내 script/style) 정확도 — QA6.
- 첫 페인트·타이핑 지연, 테마 에디터 프리뷰(입력마다 재적용) 체감 — QA6.
- IPC 페이로드 증가(ResolvedTheme 최대 ~45KB)·`include_str!` 증가분 — 실측 보고.
- 재변환 diff 0 게이트 실패 가능성(상류 갱신) — 발생 시 사용자 보고 후 결정.
- 플러그인 grammar ReDoS — `tokenizeTimeLimit`(기본 500ms) 의존, 한계 문서화.

## 5. 완료 조건

- `bun run verify` 전체 통과(typecheck→lint→format:check→bun test→cargo fmt/clippy/test) + `cargo fmt`.
- 파리티 테스트: 디스패치 137종 양방향 공집합, 번들 36종 tokenColors 보유, i18n 4곳 동기.
- 문서 개정 전량 반영, QA6 체크리스트에 W7 항목(9종 신규 하이라이팅 개별 확인 포함) 추가, dev 커밋·푸시.
