# Wave F 구현 계약 — 에디터 표현 (2026-08-15)

> 정찰 wf_12eece29-284(opus+high 4축: 프론트 구조·monaco 0.56 실물·LSP 스펙+서버 3종·Emmet/스니펫).
> 하중 주장 전건을 메인이 재검증 완료 — monaco 소스 5곳 직독 + 웹 결정적 3건 fetch 대조.
> **반증 1건**: 정찰의 "gopls initializationOptions 반영 불보장" 주장은 gopls 소스
> `options.Set(params.InitializationOptions)`(general.go) 확인으로 반증 — initializationOptions 는
> 정식 설정 경로다. 캠페인 계약: `2026-08-14-remaining-features-pro-qa-plan.md`(완벽 우선·4렌즈·역할 상향).

## 1. 사용자 결정 (2026-08-15)

| # | 결정 | 선택 |
|---|------|------|
| ① 범위·Emmet | 풀 패키지 + 의존성 승인 | **4축 전부**(Semantic Tokens·사용자 스니펫·Format on Type/Paste·Emmet) + **emmet-monaco-es 5.7.0 신규 승인**(전이 의존 emmet ^2.4.11, MIT, min 96KB, CSP 적합·eval 0건 확인). tokenizer:'standard' 고정 |
| ② Semantic Tokens | **delta 포함** | 추천 패키지(매핑표+미매핑 필터+기본 ON+세션 스코프 refresh+gopls initializationOptions) **+ full/delta 구현**(resultId 캐시·splice·서버 데이터 보관). range 는 1차 제외 |
| ③ Format on Type/Paste | 둘 다 + 어댑터 2종 | rangeFormatting·onTypeFormatting 어댑터 신설 + capability 3종 선언 + 설정 2필드(기본 false) + 트리거 문자 서버 선언 그대로 |
| ④ 스니펫 | 추천 패키지 | Rust snippets_dir + theme 형 CRUD 3커맨드 + VS Code 호환 포맷 + 언어별 completion provider + 설정 SNIPPETS 섹션 + ThemeEditor 형 편집 폼 |

## 2. 확정 사실 (메인 재검증 완료 — 근거 실물)

1. **[워시아웃] monaco standalone 의 semantic 색은 `[type,...modifiers].join('.')` 을 defineTheme rules
   트라이에 매칭해 결정되며, `getTokenStyleMetadata` 는 미매칭 시에도 기본 rule 로 폴백해 undefined 를
   반환하지 않는다**(standaloneThemeService.js:149-162). styling 층은 반환값의 foreground·4종 폰트스타일
   비트를 전부 세팅하므로(semanticTokensProviderStyling.js:48-78) 미매핑 semantic 토큰은 shiki 색을
   기본 전경색으로 전면 덮는다. shiki 테마 rule 은 TextMate scope 문자열뿐(build-shiki-theme.ts:73-88,
   @shikijs/monaco 가 tokenColors 에서 rules 파생)이라 방어 없이 켜면 시각 퇴행.
2. **`StandaloneTheme.semanticHighlighting = false` 하드코딩**(standaloneThemeService.js:40) + 에디터 옵션
   기본 'configuredByTheme' → **`'semanticHighlighting.enabled': true` 를 create 옵션에 명시하지 않으면
   provider 가 호출조차 안 된다.** 이 옵션은 전역 StandaloneConfigurationService 로 승격되어 앱 전역이다.
3. **formatOnPaste 는 documentRangeFormattingEditProvider 등록이 필수 게이트**(formatActions.js:165-167,
   합성 폴백 없음). formatOnType 은 provider 의 autoFormatTriggerCharacters 기반 onDidType 리스너
   (formatActions.js:71-85). TAIDE 는 문서 포매팅 어댑터 1종뿐, client.ts:59 rangeFormatting 은 호출자
   없는 dead entry.
4. **monaco 에 스니펫 completion provider 없음**(suggest.js 스니펫 블록이 즉시 return 하는 dead branch —
   직접 확인), Tab 확장 컨트롤러 없음, `editor.action.insertSnippet` 공개 커맨드 없음(grep 0). 반면 스니펫
   파서·변수(VS Code 전수+TM_DIRECTORY_BASE)·choice·transform 7종은 완비 — 만들 것은 저장소·provider·UI.
5. **rust-analyzer**: semanticTokensProvider full{delta:true}+range:true, legend 는 `SupportedType::iter()`
   조립(비표준 ~30종 포함— capabilities.rs fetch 확인). onTypeFormatting 트리거 8종('.','=','<','>','{','(','|','+').
6. **gopls**: `options.Set(params.InitializationOptions)` 로 initializationOptions 를 설정으로 파싱(소스
   확정). semanticTokens 기본 false 이나 ConfigurationSupported 면 provider 를 광고하고 요청 시 빈 결과 —
   TAIDE 의 workspace/configuration null 응답과 결합하면 "조용한 0토큰". onType/range formatting 둘 다 미선언.
7. **vtsls**(TAIDE 의 TS 서버): semanticTokens full+range(delta 없음), onTypeFormatting ';','}','\n'(추정 —
   VS Code TS 확장 래핑 구조 근거). TAIDE 매니페스트 19종 서버.
8. **emmet-monaco-es 5.7.0**: shiki(setTokensProvider) 환경은 `{ tokenizer: 'standard' }` 필수(README 명시,
   fetch 확인), 기본 monarch 경로는 monaco 내부 필드 리플렉션(브리틀). 'standard' 도
   `model.tokenization.getLineTokens/getStandardTokenType` 비공개 API 사용 — monaco-internal.d.ts +
   acknowledge 기록 선례 범주. emmetHTML/CSS/JSX 는 dispose 반환. dist 에 eval/new Function 0건.
9. **TAIDE 현행**: buildInitializeParams 에 semanticTokens·onTypeFormatting·rangeFormatting 선언 없음
   (formatting:{} 만), protocol.ts ServerCapabilities 에 두 필드 없음, src 에 관련 구현 0건(재grep 확인).
   Settings 신규 필드 스파인 6곳·locale 4곳·배선 3곳 선례는 Wave A~E 와 동일. SYNTAX_TOKENS 31종
   (mapping-tables.ts:177-209)이 LSP 표준 타입과 다수 동명 — 매핑 소스. 폴백 테마(monaco/theme.ts)는
   이미 SYNTAX_TOKENS 이름 rule 이라 semantic 매칭이 자연 성립.

## 3. 확정 설계

### 3.1 Semantic Tokens (full+delta)

- **capability 선언**(lsp-session-registry): textDocument.semanticTokens { requests: { full: { delta: true } },
  tokenTypes: 표준 23종, tokenModifiers: 표준 10종, formats: ['relative'], overlappingTokenSupport: false,
  multilineTokenSupport: false, serverCancelSupport: false, augmentsSyntaxTokens: true } +
  workspace.semanticTokens.refreshSupport: true(핸들러와 **같은 커밋** — 미등록 시 -32601 에러 로그 회귀).
  range 는 미선언(1차 제외). 상수(SEMANTIC_TOKEN_TYPES/MODIFIERS)는 protocol.ts 에 as const + 유도.
- **ServerCapabilities 확장**: semanticTokensProvider { legend: { tokenTypes, tokenModifiers },
  full?: boolean | { delta?: boolean }, range?: boolean | 빈 객체 } · documentOnTypeFormattingProvider
  { firstTriggerCharacter, moreTriggerCharacter? }. FEATURE_CAPABILITY_CHECKS 4종 추가
  (semanticTokens/full·full/delta·onTypeFormatting·rangeFormatting).
- **어댑터(adapters/semantic-tokens.ts) — 재인코딩 전략**(워시아웃 1차 방어): 서버 legend 문자열을 매핑표로
  TAIDE 토큰명에 대응시키고, **어댑터가 자체 monaco legend(매핑된 TAIDE 토큰명 집합)를 노출**한다. 토큰
  스트림은 서버 인덱스→TAIDE 인덱스로 변환하고 **미매핑 타입 토큰은 방출하지 않는다**(드롭 — shiki 색
  보존). modifier 는 표준 10종 이름으로 통과(트라이가 '.' 접두 폴백하므로 무해).
- **delta 처리**(결정 ②): 모델별 캐시 { 서버 resultId, 서버 원본 data } 보관. 서버가
  SemanticTokensDelta(edits splice)를 주면 **서버 원본 데이터에 적용 후 전체 재인코딩해 monaco 에는 full
  SemanticTokens 로 반환**(자체 resultId 부여 — 어차피 재인코딩하므로 monaco 측 edits 변환은 하지 않는다.
  delta 의 이득은 서버 재계산·와이어 절감으로 실현). monaco 의 lastResultId→캐시 조회,
  releaseDocumentSemanticTokens(resultId)·dispose 시 캐시 해제(메모리 상한: 모델당 1엔트리).
  직전 캐시 부재 시 delta 요청 생략하고 full 재요청. LSP data 는 number[] → Uint32Array 변환.
- **테마 매핑**: mapping-tables.ts 에 `SEMANTIC_TOKEN_TYPE_MAP`(LSP 표준 23종 + ra 비표준 별칭
  (builtinType→type·selfKeyword/selfTypeKeyword→keyword·lifetime→storage·formatSpecifier/escapeSequence→
  regexp·punctuation 계열→punctuation·attribute 계열→decorator·boolean→constant·character→string 등 실
  legend 기준) + vtsls 별칭 → SYNTAX_TOKENS 31종). buildShikiTheme 의 tokenColors **끝에** 매핑된
  TAIDE 토큰명을 scope 로 하는 rule 을 테마 syntax 색으로 append(끝 append 라 colorStyleToScopeMap
  first-wins 역매핑 불훼손 — 동일 색 재사용으로 신규 색 도입 없음). 폴백 테마(theme.ts)는 기존 rule 이
  이미 토큰명이라 무변경.
- **활성**: code-editor.tsx create 옵션 `'semanticHighlighting.enabled': true` 상시 + Settings
  `editor_semantic_highlighting`(기본 **true**) 은 어댑터 게터(isCodeLensEnabled 선례 — 재등록 없이 요청
  게이트, off 면 빈 결과+onDidChange 발화). largeFile tier 는 LSP 미부착이라 자연 비활성.
- **refresh**: workspace/semanticTokens/refresh 는 **세션 스코프** client.registerRequestHandler
  (workspace/applyEdit 선례) — 그 세션 어댑터의 onDidChange 만 발화(전 세션 재계산 폭풍 방지).
- **gopls**: resources/lsp-servers.json 의 gopls 항목에 `initializationOptions: { "semanticTokens": true }`
  (Wave A 의 `initialization_options` 매니페스트 필드 재사용 — 코드 변경 0). TAIDE 의
  workspace/configuration null 응답이 이 값을 되돌리지 않는지는 검토 렌즈 확인 항목.

### 3.2 Format on Type / Paste

- adapters/formatting.ts 에 형제 함수 2종 신설: registerRangeFormatting(textDocument/rangeFormatting —
  formatOnPaste 의 전제 게이트 상환) + registerOnTypeFormatting(textDocument/onTypeFormatting,
  autoFormatTriggerCharacters = [firstTriggerCharacter, ...moreTriggerCharacter] 서버 선언 그대로).
  toMonacoTextEdits 3자 공유, CancellationToken 관행 동일.
- code-editor.tsx: formatOnType·formatOnPaste 옵션을 설정 props → updateOptions effect 로 배선.
  Settings `editor_format_on_type`·`editor_format_on_paste`(기본 **false** — VS Code 동일).
- gopls 는 둘 다 서버 미지원(문서화 — features 문서에 서버별 지원표).

### 3.3 사용자 스니펫

- **Rust(신규 도메인 snippet)**: paths.rs `snippets_dir()` + domain/snippet(types/service/commands/mod) —
  `snippet_list`(snippets/ 스캔: `<languageId>.json` + `*.code-snippets`, 파싱 실패 파일은 스킵+로그,
  theme_list 관용) · `snippet_save(fileName, content)`(파일명 sanitize — '/'·'\\'·'..' 거부, theme_save
  선례. content 는 JSON 유효성 검증) · `snippet_delete(fileName)`. 배선 3곳+파리티. 원격 dispatch 정책은
  theme_save 와 동일 계열로 판단해 적용. 캐시는 Rust 무보관 — 프론트 TanStack Query 가 캐시하고
  save/delete 시 invalidate(파일 워처 없음 — 외부 편집 반영은 보류 §4).
- **파일 포맷(VS Code 호환)**: 키=스니펫 이름, 값={ prefix: string|string[], body: string|string[],
  description?, scope?(.code-snippets 만 — 콤마 언어 목록) }. isFileTemplate·include/exclude 는 1차
  무시(문서 명시).
- **프론트**: entities/snippet(snippet.ipc.ts·snippet.query.ts) + shared/lib 스니펫 completion provider —
  **부트스트랩 1회, TAIDE_LANGUAGE_IDS 각각 정확 languageId 로 registerCompletionItemProvider**('*' 금지 —
  점수 5 그룹으로 밀려 LSP 있는 언어에서 영구 미노출. 재검증 확정), kind=Snippet·InsertAsSnippet·
  body join('\n')·sortText 미지정(label 폴백)·snippetSuggestions 기본값 유지. provider 는 쿼리 캐시를
  읽어 언어 해당 스니펫(언어 파일 + scope 매칭 .code-snippets)만 반환.
- **편집 UI**: settings-view 에 SNIPPETS 섹션(SETTINGS_SECTION_ID/TOC 2곳) + widgets/snippet-editor
  (ThemeEditor 선례 — 파일 선택·스니펫 목록·prefix/body/description/scope 폼·저장/삭제·AlertDialog).
- gist 동기화 미포함(sync 는 settings.json 만 — 후속 후보로 문서화).

### 3.4 Emmet

- package.json 에 `emmet-monaco-es@5.7.0` 추가(승인 완료 — 승인 목록 갱신). 부트스트랩에서
  emmetHTML(monaco, [html 계열])·emmetCSS(monaco, [css·scss])·emmetJSX(monaco, [javascriptreact·
  typescriptreact]) — 실제 TAIDE_LANGUAGE_IDS 명칭 대조 후 등록, **전부 { tokenizer: 'standard' }**.
  Settings `emmet_enabled`(기본 **true**) 토글 → dispose/재등록.
- 비공개 API 리치인(라이브러리의 getLineTokens/getStandardTokenType 사용) 사유는 이 계약이 기록
  (monaco-command-service-deep-import 선례 범주). THIRD_PARTY_LICENSES.md 에 emmet-monaco-es·emmet 고지 추가.

### 3.5 설정·i18n 스파인

- Settings 신규 4필드: `editor_semantic_highlighting`(true)·`editor_format_on_type`(false)·
  `editor_format_on_paste`(false)·`emmet_enabled`(true) — 6곳 배관(types/service/sync 리터럴/
  emptySettingsPatch/bindings/settings-view) + gist sync 포함(비밀 아님, 기존 editor_* 동일).
- locale 4곳(required·en·ko·ja): 설정 라벨·설명 4쌍 + SNIPPETS 섹션·스니펫 편집 폼 문자열 + en⊆required.

### 3.6 실행 구조 (역할 상향)

- **Phase S(Rust 단독, sonnet+xhigh) ∥ Phase B0(프론트 LSP 코어 단독, sonnet+xhigh)** 병렬:
  S = §3.5 Rust 측 + snippet 도메인 + gopls 매니페스트 + 배선 3곳 + 파리티 + bindings + cargo fmt/clippy/test.
  B0 = protocol.ts(상수·타입 2필드)·client.ts(게이트 4종)·lsp-session-registry.ts(capability 선언·refresh
  세션 핸들러 등록 자리) — 어댑터 실체 배선은 D.
- **Phase C 병렬 3(sonnet+xhigh, 파일 소유 분리)**: C1 = semantic-tokens 어댑터+delta 캐시+매핑표+테마
  주입(adapters/semantic-tokens.ts·mapping-tables.ts·build-shiki-theme.ts) / C2 = 포매팅 어댑터 2종+에디터
  옵션 배선(adapters/formatting.ts·code-editor.tsx·editor-pane.tsx props) / C3 = 스니펫 프론트+편집 UI+
  Emmet(package.json·entities/snippet·provider·widgets/snippet-editor·settings-view.tsx 4토글+섹션·emmet
  부트스트랩).
- **Phase D 통합(단독)**: 어댑터 등록 배선(lsp-session-registry — B0 파일 재소유)·use-lsp-session 게터·앱
  부트스트랩 연결·문서(features·tech-stack·ipc-contract·qa6)·전체 verify+vite build.
- **Phase E 검토**: 4렌즈(opus+xhigh) → 적대적 검증(opus+high) → 수정(sonnet+xhigh) → 메인 2차 → 커밋(dev).

## 4. 기각·보류

| 안 | 처리 |
|----|------|
| Semantic 기본 OFF 옵트인 | 기각 — 기본 ON(매핑+필터 이중 방어 전제) |
| full only(delta 미구현) | 기각 — 사용자가 delta 포함 선택(결정 ②) |
| range provider(viewport) | 보류 — 1차 제외, 후속 후보 |
| ResolvedTheme.semanticTokenColors 스키마 신설 | 기각 — 테마 스키마 5곳+36테마 재변환, 범위 초과(theme-system.md 보류 선례) |
| 서버 legend 를 monaco 에 직통과 | 기각 — 워시아웃(§2-1). 재인코딩+드롭 채택 |
| gopls workspace/configuration 서버별 설정 맵 | 기각 — initializationOptions 정식 경로 확정(반증)으로 불필요. Wave A backlog(vtsls 매핑 테이블)는 별개 유지 |
| Emmet @vscode/emmet-helper 자작·emmet 코어 최소 구현 | 기각 — emmet-monaco-es 채택(결정 ①) |
| Emmet Tab 키 직접 확장 | 기각 — Tab 3중 경합(indent·suggest·snippet-jump). completion 경유 |
| onType 만(Paste 보류)·트리거 화이트리스트 | 기각 — 둘 다+서버 선언 그대로(결정 ③) |
| 스니펫 settings 블롭·팔레트 삽입만 | 기각 — 파일 저장소+completion(결정 ④) |
| 스니펫 isFileTemplate·include/exclude | 보류 — 1차 무시, 문서 명시 |
| 스니펫 파일 워처(외부 편집 즉시 반영) | 보류 — 명시 저장 경로 invalidate 만 |
| 스니펫 gist 동기화 | 보류 — sync 는 settings.json 한정, 후속 후보 |

## 5. 완료 조건

- `bun run verify` 전체 + vite build. locale 4곳·en⊆required·파리티(신규 커맨드 snippet_list/save/delete).
- 4렌즈+적대적 검증+메인 2차 통과. 초점: **워시아웃 방어 실효**(재인코딩·미매핑 드롭·테마 rule 주입이
  실제 색을 내는지)·**delta 캐시 정합**(resultId 수명·release·모델 폐기 누수)·colorStyleToScopeMap
  불오염·formatOnPaste 게이트 실동작·스니펫 파일명 sanitize 경로 탈출·'*' selector 금지 준수·emmet
  dispose 수명주기(tokenizer standard 확인)·gopls null configuration 상호작용·`'semanticHighlighting.enabled'`
  전역성의 부작용.
- 문서: features/editor.md(semantic·포매팅·스니펫·emmet — 서버별 지원표 포함)·ipc-contract(스니펫 3커맨드)·
  data-model(설정 4필드)·tech-stack(emmet-monaco-es 승인)·qa6-checklist Wave F 실기 항목. 갭 분석 §1·§2
  해당 항목 종결 표기.
