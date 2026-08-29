# 테마 시스템

> FR-I. 색상 토큰을 최대한 세밀하게 나누고, 앱 셸·codeview(Monaco)·터미널(xterm)이
> 하나의 테마 정의에서 파생되게 한다. Tailwind v4 연계 세부는 `docs/research/tailwind-shadcn.md`,
> Monaco 테마 API 는 `docs/research/monaco.md` 확정안을 따른다.

## 1. 원칙

1. **단일 정의, 삼중 파생**: 테마 파일 하나에서 (a) 앱 CSS 변수, (b) Monaco defineTheme,
   (c) xterm ITheme(ANSI 16색 포함) 을 전부 생성한다. 세 소비자가 색을 따로 정의하는 것을 금지한다.
2. **토큰은 시맨틱**: 컴포넌트가 raw 색을 쓰지 않고 시맨틱 토큰만 참조한다(VSCode color ID 방식).
3. **Rust 소유**: 현재 테마 선택·커스텀 테마 파일은 Rust(theme 도메인)가 소유하고,
   view 는 `theme_get` query 로 받아 CSS 변수로 주입한다(ADR-0004). 테마 파일 변경은 watcher 로 핫리로드.

## 2. 테마 파일 스키마 (`themes/*.json`)

```jsonc
{
    "version": 1,
    "id": "taide-dark",
    "name": "TAIDE Dark",
    "type": "dark",                    // dark | light — 파생 기본값 결정
    "palette": {                       // 원료 색 — 여기서만 hex 사용
        "bg0": "#1e1e2e", "bg1": "#27273a", "...": "..."
    },
    "colors": {                        // 시맨틱 토큰 → palette 참조 또는 hex (아래 §3 전체 목록)
        "app.background": "$bg0",
        "sidebar.background": "$bg1"
    },
    "syntax": {                        // 구문 토큰 → 오버레이 근거 (아래 §4)
        "keyword": { "fg": "#c678dd", "bold": false },
        "string": { "fg": "#98c379" }
    },
    "terminal": {                      // ANSI 16 + 커서·선택 (아래 §5)
        "black": "#...", "red": "#...", "brightBlack": "#..."
    },
    "tokenColors": [                   // 선택 — 원본 TextMate 룰 전량 보존 (7.10-W7)
        { "scope": ["comment", "punctuation.definition.comment"],
          "settings": { "foreground": "#6a9955", "fontStyle": "italic" } }
    ]
}
```

- `$key` 는 palette 참조. colors/syntax/terminal 은 palette 를 참조하거나 직접 hex 를 쓸 수 있다.
- 내장 테마(dark/light)는 앱 리소스로 포함하고, 사용자 테마는 `themes/` 에 두면 목록에 나타난다.
- 사용자 테마는 내장 테마를 `extends` 로 상속해 일부 토큰만 오버라이드할 수 있다
  (누락 토큰은 base 에서 채움 — 전체 나열 강제 금지).

### 2.1 `tokenColors` (7.10-W7)

- 형태는 VS Code `tokenColors`/TextMate `settings` 배열과 동일: `{ scope: string[], settings:
  { foreground?, background?, fontStyle? } }`. `scope` 는 항상 **배열로 정규화**한다(원본이
  문자열/콤마구분이어도 변환 단계에서 배열로 접는다). `name` 필드는 버린다(페이로드 절약, TextMate
  동작에 무관).
- `fontStyle` 은 **원문 문자열을 그대로 보존**한다(`"bold italic underline"` 등) — 기존 `syntax`
  절의 `SyntaxStyle`(fg + bold/italic 2불린)과 달리 underline·strikethrough 도 표현 가능하다.
- **순서가 의미를 가진다**(TextMate 매칭은 나중 룰이 이긴다) — 배열이며 키 병합 대상이 아니다.
- **선택 필드**다. `#[serde(default, skip_serializing_if = "Option::is_none")]` 로 기존 테마
  파일·사용자 테마와 하위호환(없으면 `None`, 파일에 `null` 이 새로 찍히지 않는다).
- **extends 상속 규칙**: 자식이 `tokenColors` 를 **명시하지 않으면(`None`)** base 의 것을 그대로
  상속한다. 자식이 `tokenColors` 를 **명시하면(`Some`)** base 것을 **전체 교체**한다(배열이라 키
  단위 병합이 불가능 — colors/syntax/terminal 의 키 단위 or_insert 와 다르다).
- **테마 에디터의 저장 규칙**(d-51 F6 정정 · 감사 §4-B B6): 에디터는 색 토큰만 diff 로 저장하지만,
  `tokenColors` 와 attribution(`author`/`license`/`source`)은 **드래프트가 그대로 실어 나른다**
  (`ThemeDraftMetadata`). 단 `tokenColors` 는 **base 가 상속시켜 줄 값과 다를 때만** 파일에 쓴다
  (`resolveThemeDraftMetadata`) — 번들 테마를 복제한 사용자 테마는 여전히 `None` 이라 원본의 TextMate
  충실도가 자동 유지되고, base(내장 dark/light)에 `tokenColors` 가 없는 **vsix 임포트 테마**는 자기
  규칙을 파일에 남긴다. 이 구분이 없던 동안에는 임포트 테마를 **아무것도 바꾸지 않고 한 번 저장하기만
  해도** 구문 강조가 `syntax` 31토큰 폴백으로 영구히 납작해졌다(편집 중 라이브 프리뷰도 같은 이유로
  실제 테마와 달랐다).
- `syntax` 절과의 관계: `tokenColors` 가 있으면 **그것이 구문 강조의 원본 진실**이고, `syntax` 31
  토큰은 그 위에 얹는 **오버레이**(사용자가 앱 UI 에서 편집한 토큰만 추가 반영)다. 상세 합성 규칙은
  §4.2.

## 3. 시맨틱 토큰 (colors) — 영역별 세분화

VSCode 의 color ID 체계를 참조해 다음 네임스페이스로 나눈다. 초기 정의 시 각 네임스페이스의
토큰을 전부 명세하고, 구현 중 임의 토큰 추가 대신 이 문서를 갱신한다.

| 네임스페이스 | 대상 토큰 예 |
|--------------|-------------|
| `app.*` | background, foreground, border, focusBorder, shadow, accent |
| `appSidebar.*` | background, itemHover, itemActive, iconDefault, iconAgentRunning, iconAgentWorking, iconAgentAwaiting, iconAgentIdle, iconAgentUnknown, badge |
| `tabBar.*` | background, tabActiveBackground, tabInactiveBackground, tabActiveForeground, tabInactiveForeground, tabBorder, tabActiveIndicator, dirtyDot, previewForeground(이탤릭용), dropTarget |
| `explorer.*` | background, itemHover, itemSelected, itemFocused, indentGuide, folderIcon, gitModified, gitAdded, gitDeleted, gitUntracked, gitIgnored |
| `panel.*` (검색·git 등 사이드 패널 공통) | background, sectionHeader, inputBackground, inputBorder, matchHighlight |
| `editor.*` | background, foreground, lineHighlight, cursor, selection, inactiveSelection, lineNumber, lineNumberActive, indentGuide, whitespace, bracketMatch, findMatch, findMatchHighlight, hoverBackground, widgetBackground(찾기/자동완성 위젯), widgetBorder |
| `editorGutter.*` | addedBackground, modifiedBackground, deletedBackground (VSCode 규칙: 추가=초록, 수정=파랑, 삭제=빨강) |
| `editorBlame.*` | foreground (인라인 blame 회색 계열), background |
| `diff.*` | insertedBackground, insertedLineBackground, removedBackground, removedLineBackground, border |
| `terminal.*` | background, foreground, cursor, selection, commandBlockBorder(OSC133), linkForeground |
| `git.*` | added, modified, deleted, renamed, untracked, conflicted, staged — changes 목록·상태문자 색 |
| `graph.*` | lane1~lane12(그래프 레인 순환색 — Git Graph 확장의 12색 팔레트 참조), refBranch, refTag, refHead |
| `statusIndicator.*` | info, warning, error, success |
| `menu.*` / `popover.*` / `tooltip.*` / `modal.*` | background, border, itemHover, separator |
| `scrollbar.*` | thumb, thumbHover, track |
| `input.*` / `button.*` / `list.*` | shadcn 컴포넌트 계열 매핑 (shadcn CSS 변수와 연결) |

## 4. 파생 규칙

### 4.1 앱 CSS 변수 (Tailwind v4)

- 토큰을 `--taide-{namespace}-{token}` CSS 변수로 `:root` 에 주입한다(테마 전환 = 변수 재주입, 리렌더 불필요).
- Tailwind v4 는 **"일반 CSS 변수 → `@theme inline` 매핑" 2단 구조**가 필수다(`@theme` 직접 값은
  빌드 타임 고정 — `docs/research/tailwind-shadcn.md` §4·함정). `@theme inline` 에서 `--taide-*` 를
  참조해 유틸리티로 노출하고, shadcn 이 기대하는 변수(`--background`, `--primary` 등)는
  `input.*`/`button.*` 계열 토큰에서 매핑해 채운다.
- 기동 FOUC 방지: localStorage 선주입 대신 **`tauri.conf.json` 의 window `backgroundColor` +
  `visible: false` → 테마 적용 완료 후 `show()`** 를 쓴다(ADR-0004 — view 는 도메인 상태를
  저장하지 않음).
- 컴포넌트에서 hex·palette 직접 사용 금지 — 시맨틱 토큰 클래스/변수만 사용.

### 4.2 Monaco — shiki(TextMate) 경유 전량 토큰화 (7.10-W7 재작성)

> 이전 버전의 이 절은 "31 토큰 고정 집합을 Monaco `defineTheme` 룰로 직접 변환"을 정의했다.
> 7.10-W7 로 **TextMate scope 전량을 shiki 로 토큰화**하는 방식으로 대체됐다. 아래는 확정 설계이며,
> 실기(WKWebView) 검증은 QA6 대기(`docs/quality-assurance/2026-08-11-qa6-checklist.md`).

**엔진**: `@shikijs/core` + `@shikijs/engine-javascript`(JS RegExp 엔진) + `@shikijs/langs` +
`@shikijs/monaco` 4.4.3 고정(`shiki` 메타 패키지는 미설치 — oniguruma WASM 이 기본이라 CSP 위반).
`createHighlighterCore({ themes, langs, engine: createJavaScriptRegexEngine() })` 로 highlighter 를
만들고, `shikiToMonaco(highlighter, monaco)` 로 `monaco.languages.setTokensProvider` 를 부착한다.
CSP(`script-src 'self'`)는 변경하지 않는다(정적 검증 완료 — `docs/research/shiki.md`).

**언어 등록**: `shikiToMonaco` 는 `monaco.languages.register` 를 하지 않는다 — highlighter 가 로드한
언어와 monaco 에 이미 등록된 언어 id 의 교집합에만 토크나이저가 붙는다. TAIDE 31언어(+plaintext)
전부를 `monaco.languages.register({ id })` 로 **우리가 선행 등록**한다(기존 등록 id 재호출은 병합돼
안전). TAIDE id ↔ shiki lang id 매핑: `typescriptreact`→`tsx`, `javascriptreact`→`jsx`,
`heex`→`html`(폴백, §4.2.1), 나머지는 동일명. `plaintext` 는 shiki 대상에서 제외(monaco 내장 유지).
highlighter 에는 TAIDE id 를 lang name 으로 재명명해 등록한다(`setTokensProvider` 가 TAIDE id 기준
이어야 하므로).

**grammar 온디맨드 로드** (d-51 F7 · 감사 §1-7): highlighter 를 만들 때 싣는 TAIDE grammar 는
`TAIDE_CORE_LANGUAGE_IDS`(`json`·`jsonc`·`markdown` — 앱이 사용자 동작 없이 스스로 여는
`settings.json`/`keybindings.json`·마크다운 표면) 뿐이다. 나머지는 **그 언어의 모델이 처음 생길 때**
`ensureShikiLanguage(languageId)` 가 `highlighter.loadLanguage` 로 실어 넣고 tokens provider 를
재부착한다. 요구 시점은 `monaco.editor.onDidCreateModel`+`onDidChangeLanguage` 구독(부팅 시 이미 만들어진
모델은 `getModels()` 로 일괄 훑음)이라 에디터 탭·diff 한쪽·peek 미리보기 등 모델을 만드는 모든 경로가
자동으로 포함된다. 이전에는 31종 전량(빌드 청크 30개 합계 2266kB, `cpp` 단독 778kB)을 부팅 때 받아
파싱했다 — 대부분의 세션이 열지 않는 언어들이다. 코어 3종 합계는 64kB 다(실측, d-51 F7). 지금까지 요구된 언어 집합은 모듈 상태로 남아
`reinitShiki`(플러그인 재구성)가 그 집합으로 다시 만든다. 플러그인 grammar 의 `embeddedLangs` 가
가리키는 TAIDE 언어도 재생성 전에 같은 집합에 합류한다(`sanitizePluginGrammarEmbeddedLangs` 가
미로드 언어를 떨어뜨리므로).

**단일 테마명**: shiki 에는 항상 `taide` 테마 하나만 로드한다(라이트/다크 전환 시 `loadTheme` 로
같은 이름을 교체 — `@shikijs/core` 의 `Map.set` 동작으로 교체가 성립). 테마 전환 시 절차:
① `highlighter.loadTheme(buildShikiTheme(resolved))`(§4.2.2), ② `monaco.editor.setTheme`/`create`
몽키패치 원본 복원, ③ `shikiToMonaco(highlighter, monaco)` 재호출(패치 재중첩 방지, 말미에
`setTheme('taide')` 자동 호출). 플러그인 grammar 변경 시(§3 grammar 재구성)는 highlighter 를
`dispose()` 후 재생성하고 동일 절차를 반복한다.

**§4.1 fallback 대비 위상 변화**: 과거 문서가 정의한 "31 토큰 → Monaco 룰" 변환은 이제 **①
`tokenColors` 가 없는 테마의 폴백 경로**, **② 사용자가 앱 UI 에서 편집한 토큰의 오버레이 경로**
두 가지로만 쓰인다(§4.2.2). `tokenColors` 를 가진 테마(번들 대다수 — §8.2)는 원본 TextMate 룰이
그대로 shiki 에 전달되어 훨씬 세밀하게 토큰화된다.

#### 4.2.1 `heex` grammar 폴백

`@shikijs/langs` 4.4.3 에 `heex` grammar 가 없다(shiki 전체에 없음). shiki `html` grammar 로
매핑한다(현재 plaintext 대비 순개선 — HEEx 는 HTML + `<%= %>` 확장이라 태그·속성·문자열은 정확해진다).
사용자 결정(계약 §2 결정 2).

#### 4.2.2 `buildShikiTheme` — ResolvedTheme → shiki 테마 조립

```
buildShikiTheme(resolved) = {
    name: 'taide',
    type: resolved.type,
    colors: buildThemeColors(resolved.colors) (기존 MONACO_COLOR_SOURCE 매핑 재사용)
            + editor.background/editor.foreground,
    tokenColors: raw ++ overlay
}
```

- `raw = resolved.tokenColors ?? fallbackFromSyntax(resolved.syntax)`. `tokenColors` 가 없는
  테마는 31 토큰에서 TextMate 룰을 역생성한다(스코프 중복은 "먼저 등장한 토큰이 소유"하는 규칙으로
  충돌을 없앤다 — 예: `docComment` 는 `comment` 후보를 잃지 않도록 `comment.block.documentation`
  만 남긴다).
- `overlay`: `resolved.syntaxOverrides`(자식 테마가 스스로 명시한 syntax 키 목록)에 있는 토큰만
  `SYNTAX_SCOPE_CANDIDATES[token]` 스코프로 raw 뒤에 **append** 한다. 사용자가 테마 에디터에서
  건드린 토큰만 raw 위에 얹히는 구조다.
- **한계(의도된 제약, 문서화)**: TextMate 매칭은 룰의 등장 순서(specificity)를 따른다 — 넓은
  scope 후보를 쓰는 오버레이 룰이 raw 안의 더 깊이 한정된 scope 룰을 못 이길 수 있다. 오버레이는
  "보정"이지 "강제 치환"이 아니다.
- `colors` 는 shiki 가 실제로 읽는 키가 `editor.background`/`editor.foreground` 2개뿐이라는 사실에
  기반한다(`@shikijs/primitive` `normalizeTheme` 실측) — 나머지 UI 색은 `shikiToMonaco` 가 그대로
  `defineTheme` colors 로 통과시켜 monaco 에 적용된다.

**semantic token**(LSP semantic highlighting)은 이 변경의 대상이 아니다 — colors/syntax/terminal
토큰 집합 자체는 불변이므로 "테마 토큰 5곳 동기"(`docs/HANDOFF.md`)에 걸리지 않는다.

### 4.3 xterm

- `terminal` 절(ANSI 16 + cursor/selection/background/foreground)을 xterm `ITheme` 객체로 변환.
- 터미널 배경은 `terminal.background` 토큰을 따르되 기본값은 `app.background` 와 동일 계열로 정의해
  이질감을 없앤다.

## 5. 테마 전환 흐름

```
사용자 선택(설정 UI) → mutation theme_set → Rust: settings 갱신+영속화, theme:changed 이벤트
→ view: theme query invalidate → CSS 변수 재주입 + Monaco defineTheme 재적용 + 열린 xterm 들 setOption
```

- 커스텀 테마 파일 저장 시 watcher 가 감지해 같은 흐름으로 핫리로드(테마 개발 편의).
- 시스템 다크/라이트 추종 옵션: OS 변경 이벤트 수신 시 지정된 dark/light 테마로 자동 전환.

## 6. 검증

- 테마 로더는 스키마(버전·필수 토큰)를 검증하고, 누락 토큰은 base 테마 값으로 채운 뒤 경고 목록을 반환한다.
- 내장 테마 2종은 §3 토큰 전량을 명시해 base 로서 완전해야 한다(CI 테스트로 전량 존재 검증).


## 7. Phase 7.5 확장

### 7.1 폰트 커스텀 (사용자 지적 1번)

- **시스템 폰트 열거**: Rust `font_list()` 커맨드 신설. macOS 는 CoreText,
  Linux 는 fontconfig, Windows 는 GDI 열거 — `infra/` 안에서 `#[cfg(target_os)]` 분기
  (`architecture.md` §7 플랫폼 분기 격리 원칙).
  **monospace 여부를 함께 반환**해 에디터/터미널 선택 목록을 기본으로 걸러준다.
- 설정 항목: `editor_font_family` · `terminal_font_family` · `ui_font_family`(선택).
  값이 비면 현재의 시스템 기본 스택을 쓴다.
- 폰트 변경 시 터미널은 **반드시 `fit.fit()` 재호출**(`terminal.md` §7 함정 16).
- 폴백 체인을 항상 붙인다 — 사용자가 고른 폰트가 특정 글리프를 못 그릴 수 있다.

### 7.2 codeview 에 테마가 반영되지 않는 버그 (사용자 지적 2번)

**증상**: "code view 에서 테마가 변경이 안 된 건가? 흰색에 그냥 에디터 라이브러리의 테마만 들어가있네"

**원인 확정(2026-08-06 코드 검증)**: **`applyMonacoTheme` 을 아무도 호출하지 않는다.**

```
$ grep -rn "applyMonacoTheme" src/ | grep -v "shared/lib/monaco/theme.ts"
(결과 없음)
```

변환 로직은 정상이다 — `theme.ts:88` 이 `base: theme.type === 'dark' ? 'vs-dark' : 'vs'` 로
올바르게 파생하고 있고 테스트도 있다. **함수는 있는데 배선이 없어서** Monaco 가 기본 테마
(`vs`, 흰 배경)로 남은 것이다. 사용자가 본 "에디터 라이브러리의 테마만 들어가있네"가 정확한 관찰이다.

**수정**: 테마가 로드/변경될 때 `applyMonacoTheme(theme, monaco.editor)` 를 호출한다.
- 호출 위치는 **`ThemeProvider`(CSS 변수 주입과 같은 지점)** 가 자연스럽다.
  단 `ThemeProvider` 는 `@app` 레이어이므로 monaco 를 직접 import 하면 안 된다 —
  `shared/lib/monaco/setup` 재export 경유 원칙을 지킨다.
- **에디터 마운트보다 테마 적용이 늦어도 괜찮다**(`setTheme` 은 전역이라 이후 생성분에도 적용).
  반대로 **테마 전환 시에는 `defineTheme` 재호출이 필요**하다 — 같은 이름으로 재정의해야
  새 색이 반영된다(`setTheme` 만 부르면 이전 정의가 남는다).

**완료 조건**: 다크/라이트 전환 시 에디터 배경·구문색이 함께 바뀌는 것을 눈으로 확인.

### 7.3 테마 편집기 + 저장 (사용자 지적 14번)

별도 페이지(`TabKind::Settings` 의 하위 뷰 또는 전용 탭)로 만든다.

- §3 의 시맨틱 토큰을 **네임스페이스별 섹션**으로 나열하고 각 토큰에 색 피커.
- **native `<input type="color">` 를 쓰지 않는다**(acknowledge §3.1) — 자체 색 피커 컴포넌트.
- 좌: 토큰 목록 / 우: **라이브 프리뷰**(에디터·터미널·탭바 미니어처)로 즉시 반영.
- 저장 = `{app_data}/themes/{id}.json` (§2 스키마). 내장 테마를 `extends` 로 상속해
  **바꾼 토큰만** 저장한다(전체 나열 강제 금지 — §2).
- 내보내기/가져오기(JSON 파일)와 복제(내장 테마 → 사용자 테마) 제공.
- 저장 즉시 watcher 가 감지해 핫리로드(§5)되므로 별도 새로고침이 필요 없어야 한다.

**삭제·이탈 규약** (d-51 F6 · 감사 §4-B B5·D6)

- **활성 테마를 삭제하면 먼저 같은 타입의 내장 테마로 `settings.themeId` 를 옮기고 나서 지운다**
  (`entities/theme/theme-selection.ts` 의 `resolveThemeIdAfterDelete`). `theme_delete` 는 설정을
  건드리지 않으므로, 그냥 지우면 `theme_get_current` 가 `NotFound` 로 실패하고 `ThemeProvider` 가
  **아무 테마도 적용하지 못한다** — CSS 변수도 shiki 테마도 없어 다음 실행은 오류 배너 + 하이라이트
  전무 상태로 뜬다. `followSystemTheme` 이 켜져 있으면 표시 중인 테마가 아니므로
  `settings_set_theme`(그 플래그를 끄는 명시적 선택 커맨드) 대신 `settings_update` 패치로 id 만 고친다.
- **저장하지 않은 편집을 들고 나가면 확인 다이얼로그**를 띄운다(`common.unsavedChangesTitle`).
  라이브 프리뷰 때문에 화면은 이미 편집 결과를 보여주고 있어, 그냥 닫으면 "적용된 것처럼 보이는"
  변경이 조용히 사라진다. 판정 기준은 **로드 직후 서명과 지금 서명의 차이**이며 `create`(복제) 모드도
  같다 — 아무도 편집하지 않은 복제본은 원본과 바이트 단위로 같아 버려도 잃는 것이 없다(초기 구현은
  create 를 항상 "미저장" 으로 봐서 복제를 열었다 닫기만 해도 매번 확인이 떴다).

### 7.4 윈도우 배경색도 테마를 따라야 한다 (사용자 지적 17번)

§4.1 의 FOUC 방지가 `tauri.conf.json` 의 **정적** `backgroundColor` 에 의존한다.
테마 전환 시 이 값이 갱신되지 않으면 타이틀바 주변이 이전 테마 색으로 남는다.
→ 테마 적용 흐름(§5)에 **윈도우 배경색 갱신**을 포함한다. 상세는 `window-chrome.md` §1.2.

## 8. 번들 테마 (VS Code 테마 변환 · QA 8번)

내장 2종(TAIDE Dark/Light) 외에 인기 VS Code 테마 38종을 **번들 테마**로 함께 내장한다
(2026-08-28 d-46 으로 Dark/Light (Visual Studio - C/C++) 2종 추가 — 이 문서의 과거 감사
서술에 남은 "36종" 수치는 해당 감사 시점의 카탈로그 기준이며 재감사 전까지 그대로 둔다).
`{app_data}/themes`(사용자 테마 디렉터리)가 아니라 **Rust `include_str!`** 로 바이너리에
내장한다 — 이유는 두 가지다.

1. `{app_data}/themes` 로 두면 `theme_list` 상 `builtin: false` 가 되어 사용자가 실수로
   삭제할 수 있고, 최초 실행 시드/재시드 로직이 필요해진다.
2. **내장(`builtin_by_id`)이어야 `extends` 의 base 로 해석된다** (`service.rs` `load_theme`).
   번들로 두면 "Dracula 를 상속해 3개 토큰만 바꾼 사용자 테마"가 가능해진다.

### 8.1 목록

| id | 이름 | 유형 | 출처 |
|----|------|------|------|
| `one-dark-pro` | One Dark Pro | dark | github.com/Binaryify/OneDark-Pro |
| `dracula` | Dracula | dark | github.com/dracula/visual-studio-code |
| `github-dark` | GitHub Dark | dark | github.com/primer/github-vscode-theme |
| `github-light` | GitHub Light | light | github.com/primer/github-vscode-theme |
| `tokyo-night` | Tokyo Night | dark | github.com/enkia/tokyo-night-vscode-theme |
| `catppuccin-mocha` | Catppuccin Mocha | dark | github.com/catppuccin/vscode |
| `nord` | Nord | dark | github.com/nordtheme/visual-studio-code |
| `gruvbox-dark` | Gruvbox Dark | dark | github.com/jdinhify/vscode-theme-gruvbox |
| `monokai` | Monokai | dark | VS Code 내장 확장(microsoft/vscode) |
| `solarized-light` | Solarized Light | light | VS Code 내장 확장(microsoft/vscode) |
| `vscode-abyss` | Abyss | dark | VS Code 내장 확장(microsoft/vscode) |
| `vscode-monokai-dimmed` | Monokai Dimmed | dark | VS Code 내장 확장(microsoft/vscode) |
| `vscode-solarized-dark` | Solarized Dark | dark | VS Code 내장 확장(microsoft/vscode) |
| `vscode-tomorrow-night-blue` | Tomorrow Night Blue | dark | VS Code 내장 확장(microsoft/vscode) |
| `intellij-islands-light` | IntelliJ Islands Light | light | github.com/a-havrysh/vscode-intellij-theme |
| `ayu-dark` | Ayu Dark | dark | github.com/ayu-theme/vscode-ayu |
| `ayu-light` | Ayu Light | light | github.com/ayu-theme/vscode-ayu |
| `palenight` | Palenight | dark | github.com/whizkydee/vscode-palenight-theme |
| `night-owl` | Night Owl | dark | github.com/sdras/night-owl-vscode-theme |
| `night-owl-light` | Night Owl Light | light | github.com/sdras/night-owl-vscode-theme |
| `rose-pine` | Rosé Pine | dark | github.com/rose-pine/vscode |
| `rose-pine-dawn` | Rosé Pine Dawn | light | github.com/rose-pine/vscode |
| `everforest-dark` | Everforest Dark | dark | github.com/sainnhe/everforest-vscode |
| `everforest-light` | Everforest Light | light | github.com/sainnhe/everforest-vscode |
| `kanagawa-wave` | Kanagawa Wave | dark | github.com/paccodes/kanagawa-vscode-theme |
| `vitesse-dark` | Vitesse Dark | dark | github.com/antfu/vscode-theme-vitesse |
| `vitesse-light` | Vitesse Light | light | github.com/antfu/vscode-theme-vitesse |
| `one-monokai` | One Monokai | dark | github.com/azemoh/vscode-one-monokai |
| `vscode-dark-plus` | Dark+ (Default Dark) | dark | VS Code 내장 확장(microsoft/vscode) |
| `vscode-light-plus` | Light+ (Default Light) | light | VS Code 내장 확장(microsoft/vscode) |
| `vscode-dark-modern` | Dark Modern | dark | VS Code 내장 확장(microsoft/vscode) |
| `vscode-light-modern` | Light Modern | light | VS Code 내장 확장(microsoft/vscode) |
| `vscode-kimbie-dark` | Kimbie Dark | dark | VS Code 내장 확장(microsoft/vscode) |
| `vscode-red` | Red | dark | VS Code 내장 확장(microsoft/vscode) |
| `vscode-quiet-light` | Quiet Light | light | VS Code 내장 확장(microsoft/vscode) |
| `darcula` | Darcula | dark | github.com/rokoroku/vscode-theme-darcula (IntelliJ Darcula 포트) |
| `visual-studio-cpp-dark` | Dark (Visual Studio - C/C++) | dark | github.com/microsoft/vscode-cpptools (cpptools-themes 확장) |
| `visual-studio-cpp-light` | Light (Visual Studio - C/C++) | light | github.com/microsoft/vscode-cpptools (cpptools-themes 확장) |

전부 MIT. 저작권 표시는 루트 `THIRD_PARTY_LICENSES.md` 를 따른다(MIT 는 저작권·허가
표시를 모든 사본에 포함해야 한다 — 색상값만 재가공한 파생물도 대상으로 취급).
`vscode-dark-plus`/`vscode-light-plus`/`vscode-dark-modern`/`vscode-light-modern`/
`vscode-kimbie-dark`/`vscode-red`/`vscode-quiet-light`/`darcula`/
`visual-studio-cpp-dark`/`visual-studio-cpp-light` 10종은 원본에
`terminal.ansi*` 색이 전혀 없어 §8.2 "VS Code 기본 ANSI 팔레트 폴백"이 적용됐다.

### 8.2 변환 파이프라인

`scripts/convert-vscode-theme.ts` (Bun 스크립트, `bun run themes:convert` 로 실행)가
VS Code 테마 JSON(JSONC 허용)을 §2 스키마로 변환한다.

```
bun run scripts/convert-vscode-theme.ts \
  --input <vscode-theme.json> --id <kebab-id> --name <display-name> \
  --type dark|light --source-url <repo-url> --author <name> --license MIT \
  --out src-tauri/resources/themes/
```

- VS Code `colors`(353개 color ID) → TAIDE `colors`(133 토큰) 는 **fallback 체인**
  (`A ?? B ?? C`, 전부 없으면 파생 규칙)으로 매핑한다. `graph.*`(15) 처럼 VS Code 에
  대응이 없는 토큰은 ANSI 팔레트에서 전량 파생한다.
- VS Code `tokenColors`(TextMate scope) → TAIDE `syntax`(31 토큰) 는 **최장-prefix
  scope 해석**(가장 구체적인 scope 우선, VS Code 자체 규칙과 동일)으로 매핑한다. 이 매핑은
  7.10-W7 이후 **주 경로가 아니라 폴백/앱 UI(오버레이) 용**으로 위상이 바뀌었다(§4.2.2) — 원본
  `tokenColors` 를 그대로 보존하는 경로가 별도로 생겼기 때문이다.
- VS Code `tokenColors` 원문(scope 배열 + `settings.foreground`/`background`/`fontStyle`)은
  **손실 없이 그대로** TAIDE `tokenColors` 절로 passthrough 된다(§2.1) — `fontStyle` 도 원문
  문자열 그대로 보존한다(`syntax` 절의 bold/italic 2불린 축약과 별개 경로).
- `terminal`(20 토큰) 은 ANSI 16색 + background/foreground/cursor/selection(TAIDE
  `colors.terminal.*` 와 동일 값 미러링)으로 구성한다. 원본에 ANSI 16색이 없는 테마는
  **8.2.1 의 VS Code 기본 ANSI 팔레트 폴백**으로 채운다 — 이 폴백은 값을 발명하는
  것이 아니라, VS Code 자신이 런타임에 적용하는 공식 기본값을 그대로 재현한 것이다.
- `syntax.fg` 는 Monaco 룰이 6자리 hex 만 허용하므로, VS Code 의 8자리(`#rrggbbaa`)/
  4자리(`#rgba`) 알파 값은 `editor.background` 위에 합성해 6자리로 낮춘다. `colors`/
  `terminal` 은 8자리 알파를 그대로 허용한다.
- `panel.matchHighlight`(팔레트·검색 매치 강조의 **전경**색, 오버레이 배경이 아니다)는
  `chain()` 이 아니라 `derived()` 로 계산한다 — `list.highlightForeground` →
  `editor.findMatchHighlightBackground` 순으로 후보를 검사하되, `isOpaqueForegroundCandidate`
  (`mapping-tables.ts`) 가 **의미 있는 알파(불투명 미만)를 가진 후보를 배제**한다. 불투명
  가드를 통과한 후보도 `isDistinctFromBodyForeground`(`mapping-tables.ts`)가 해석된
  `app.foreground` 와 CIE76 ΔE < 2.3(거의 동일한 색)이면 추가로 배제한다 — WCAG 대비만으로는
  명도가 같고 색상만 다른 두 색을 구별하지 못하기 때문이다(구별성 가드, §8.2.3 참고).
  `editor.findMatchHighlightBackground` 는 VS Code 자신도 반투명 오버레이 전용으로 설계한
  값이라, 그대로 전경 텍스트색으로 쓰면 배경 위에서 흡수돼 거의 안 보인다(§8.2.3 참고). 두
  후보가 모두 배제되면 이 토큰의 `status` 카테고리 공용 안전값(`SAFE_DEFAULT_COLORS`)으로
  떨어진다 — 관련 없는 값을 새로 만드는 게 아니라 원래도 후보가 전혀 없을 때 쓰는 경로다.
- 출력 대비는 `contrast.ts` 의 `CONTRAST_PAIRS`(`app`/`editor`/`panel`/`tooltip`/
  `matchHighlight` 5쌍, `MIN_CONTRAST_RATIO = 3`)로 검사한다. 전경이 8자리 hex(알파 포함)면
  대비를 재기 전에 **배경 위에 합성**해 실제로 화면에 렌더되는 색으로 낮춘 뒤 잰다
  (`compositeOverBackground`, `shared/lib/color.ts`) — 합성 없이 재면 화면에 존재하지 않는
  색의 대비를 재는 셈이 된다. 6자리(비알파) 전경에는 합성이 항등이라 결과가 그대로다. 미달
  쌍은 `repairContrastPairs` 가 배경/전경 후보 사슬로 수리를 시도하고(§9.3), 수리 후에도
  미달이면 VSIX 임포트가 거부된다.
- 출력이 133 colors + 31 syntax + 20 terminal 을 **전량** 채우지 못하면 스크립트가
  누락 토큰 목록을 출력하고 `exit 1` 한다 — 번들 테마는 항상 `extends` 없는 완전한
  base 여야 한다(§2, §6). ANSI 16색 자체는 8.2.1 폴백이 항상 채우므로 이 실패 경로에
  걸리지 않는다.
- 원본 VS Code 테마 JSON 은 레포에 커밋하지 않는다. 변환 산출물(TAIDE 스키마 JSON)만
  `src-tauri/resources/themes/*.json` 에 커밋하고, 출처는 `--source-url`/`--author`/
  `--license` 로 받아 출력 JSON 의 `source`/`author`/`license` 필드에 남긴다.

#### 8.2.1 VS Code 기본 ANSI 팔레트 폴백 (출처: terminalColorRegistry)

VS Code 는 테마가 `terminal.ansi*` 를 정의하지 않아도 터미널을 무채색으로 두지 않고,
`src/vs/workbench/contrib/terminal/common/terminalColorRegistry.ts` 에 정의된 **공식
기본 ANSI 16색**으로 폴백한다. `convert-vscode-theme.ts` 는 이 동작을 그대로 재현한다
— 원본에 값이 없다고 임의 팔레트를 발명하는 게 아니라, VS Code 가 실제로 적용하는
기본값을 이식하는 것이다.

이 폴백은 `terminal.*` 출력뿐 아니라, `COLOR_MAPPING` 에서 ANSI 색을 후보로 삼는 다른
시맨틱 토큰(`git.*`, `editorGutter.*`, `graph.lane*`, `statusIndicator.*` 등)에도 동일하게
적용된다 — 원본에 `terminal.ansiGreen` 이 없으면 이들도 전부 VS Code 의 기본 초록색을
쓰지, 관련 없는 카테고리 공용 안전값(`SAFE_DEFAULT_COLORS`)으로 뭉개지지 않는다.

폴백이 쓰이면 변환 스크립트가 콘솔에 경고 1줄을 출력한다(어느 테마가 기본 팔레트를
썼는지 표기). 대상 8종은 §8.1 목록의 각주를 참고한다.

| 토큰 | dark 기본값 | light 기본값 |
|------|------------|-------------|
| black | `#000000` | `#000000` |
| red | `#cd3131` | `#cd3131` |
| green | `#0dbc79` | `#107c10` |
| yellow | `#e5e510` | `#949800` |
| blue | `#2472c8` | `#0451a5` |
| magenta | `#bc3fbc` | `#bc05bc` |
| cyan | `#11a8cd` | `#0598bc` |
| white | `#e5e5e5` | `#555555` |
| brightBlack | `#666666` | `#666666` |
| brightRed | `#f14c4c` | `#cd3131` |
| brightGreen | `#23d18b` | `#14ce14` |
| brightYellow | `#f5f543` | `#b5ba00` |
| brightBlue | `#3b8eea` | `#0451a5` |
| brightMagenta | `#d670d6` | `#bc05bc` |
| brightCyan | `#29b8db` | `#0598bc` |
| brightWhite | `#e5e5e5` | `#a5a5a5` |

### 8.2.2 VS Code 기본 리스트 상태 배경 폴백 (출처: listColors.ts)

`explorer.itemHover`/`explorer.itemSelected`/`explorer.itemFocused` 는 각각 원본 VS Code
테마의 `list.hoverBackground`/`list.activeSelectionBackground`/`list.focusBackground`
(없으면 `list.inactiveSelectionBackground`) 를 그대로 옮긴 값이다. 그런데 일부 원본
테마는 이 색을 **실질적으로 제공하지 않는다** — 값 자체가 없어 배경색 계열
(`editor.background`)로 같이 흘러가거나(`itemHover`: `vscode-dark-plus`·`darcula`·
`night-owl`, `itemSelected`/`itemFocused`: `vscode-dark-plus`·`darcula`·
`vscode-light-plus`), 값은 있지만 알파가 `00`이라 실제로는 투명이다(`itemHover`:
`everforest-dark`·`everforest-light`). 두 경우 모두 파일트리에서 해당 상태가
`explorer.background` 와 구분되지 않아 사라진 것처럼 보인다 — `itemSelected`/
`itemFocused` 가 사라지면 선택된 항목 자체를 알아볼 수 없다(파일트리 배선은
`src/features/explorer/file-tree-row.tsx` 가 포커스 시 `itemSelected`, 비포커스 시
`itemFocused` 를 쓴다).

`convert-vscode-theme.ts` 는 이 세 토큰을 해석할 때 원본 후보를 그대로 쓰기 전에
**사용 가능성**을 먼저 검사한다(`isUsableListBackground`) — 알파가 0 이거나, 같은
시점에 이미 해석된 `explorer.background` 와 RGB 가 동일하면 "제공되지 않은 것"으로
취급한다. 이 경우 §8.2.1 과 같은 이유로, 관련 없는 카테고리 공용 안전값
(`SAFE_DEFAULT_COLORS`) 대신 **VS Code 가 실제로 적용하는 공식 기본값**
(`src/vs/platform/theme/common/colors/listColors.ts` 의 각 `registerColor(...)`)으로
폴백한다 — 값을 새로 만드는 게 아니라 VS Code 자신의 기본값을 이식하는 것이다.

**이 보정 로직은 변환기 코드 자체에 내장돼 있다**(`mapping-tables.ts` 의 `derived(...)` +
`isUsableListBackground` + `VSCODE_LIST_*_DEFAULT` 상수). 즉 **재변환해도 이 보정은 그대로
재적용된다** — 아래 문단의 "재변환이 불가능해 산출물을 직접 보정했다"는 서술은 낡은 것이었다
(2026-08-12 코드 재확인으로 정정). 다만 재변환 결과가 지금 커밋된 값과 바이트 단위로 완전히
같다는 보장은 없다(원본 저장소가 그 사이 갱신됐을 수 있음) — 재변환 시에는 colors/syntax/terminal
3절의 diff 가 0 인지 게이트로 확인하고, 0 이 아니면 원인(원본 갱신 여부)을 규명한다.

| 토큰 | 대응 TAIDE 토큰 | dark 기본값 | light 기본값 |
|------|-----------------|------------|-------------|
| `list.hoverBackground` | `explorer.itemHover` | `#2A2D2E` | `#F0F0F0` |
| `list.activeSelectionBackground` | `explorer.itemSelected` | `#04395E` | `#ADD6FF`* |
| `list.inactiveSelectionBackground` | `explorer.itemFocused` | `#37373D` | `#E4E6F1` |

번들 테마 36종 전수 스캔 결과, `itemHover` 는 위 두 조건(≈ 배경, 또는 알파 0)에 해당한
5종(`darcula`·`vscode-dark-plus`·`night-owl`·`everforest-dark`·`everforest-light`)의
`resources/themes/*.json` 을 이 폴백값으로 직접 보정했다. `itemSelected`/`itemFocused`
는 배경과 완전 동일했던 3종(`darcula`·`vscode-dark-plus`·`vscode-light-plus`)을 같은
방식으로 직접 보정했다(당시 원본 소스 JSON 을 레포에 두지 않는 정책이라 즉시 재변환 대신
산출물을 직접 보정한 것 — 보정 로직 자체는 변환기에 있으므로 재변환해도 유실되지 않는다,
위 단락 참고).
재보정 후 36종 전수 재스캔 결과 `explorer.itemSelected`/`itemFocused` 가
`explorer.background` 와 동일한 테마는 0건이다. `appSidebar.itemHover`/
`popover.itemHover`/`tooltip.itemHover`/`modal.itemHover`/`list.hoverBackground` 등
같은 `list.hoverBackground` 후보를 쓰는 다른 네임스페이스에도 동일한 결함이 있을 수
있으나, 이번 스캔·수정 범위는 파일트리에서 실측된 `explorer.itemHover`/`itemSelected`/
`itemFocused` 로 한정했다 — 나머지는 별도 확인이 필요하다.

\* `list.activeSelectionBackground` 의 **light 기본값은 VS Code 자신의 값(`#0060C0`)을
그대로 쓰지 않는다.** VS Code 는 `#0060C0` 을 `list.activeSelectionForeground`(흰색)와
짝지어 등록하지만, TAIDE 의 `explorer.*` 네임스페이스에는 그 짝이 되는 선택 전경색
토큰이 없다 — 선택된 행은 `app.foreground`(검정, `#000000`)를 그대로 상속한다.
`#0060C0` 위 검정 텍스트는 대비 약 3.4:1 로 12px 소형 텍스트의 WCAG AA 기준(4.5:1)에
미달해 가독성이 무너진다(`file-tree-row.tsx`·`search-panel.tsx`·`explorer-panel.tsx`
등 전경색 미지정/명시 소비처 다수 영향). 대신 VS Code 자신의 light 테마
`editor.selectionBackground` 기본값이었던 `#ADD6FF` 를 쓴다 — 검정 위 대비
약 13.8:1 로 여유 있게 AA 를 통과한다. `vscode-light-plus.json` 의
`explorer.itemSelected` 도 동일하게 `#ADD6FF` 로 고정했다. 선택 전경색 토큰을 별도로
도입해 VS Code 원본값(`#0060C0`+흰색)을 그대로 재현하는 방안은 `explorer.*` 전체
스키마·5곳 동기(§ "테마 토큰은 5곳 동기") 변경이 필요해 범위를 넘어선다고 판단해
보류했다 — 필요해지면 별도 작업으로 진행한다.

### 8.2.3 재변환 비재현 예외 — `panel.matchHighlight` 손수정 8종

대상: `github-dark`·`github-light`·`ayu-light`·`solarized-light`·`monokai`·`palenight`·
`night-owl-light`·`vscode-quiet-light` 8종의 `panel.matchHighlight`. 이 8개는 §8.2.2 가
"재변환해도 이 보정은 그대로 재적용된다"고 못박은 일반 원칙의 **예외**다 — 번들 36종 중 이
8종만, 변환기를 다시 돌려도 지금 커밋된 값이 재현되지 않는다(앞 4종은 d-31
`docs/acknowledge/2026-08-24-d31-t2b-ts-batch-contract.md` §3-A, 다음 3종은 d-33
`docs/acknowledge/2026-08-24-d33-restructure-carryover-contract.md` "임무 C", `vscode-quiet-light`
는 d-40 `docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §3-B — 아래 8.2.4 의
선택 행 손수정(`list.foreground`/`list.activeBackground`)과 같은 배치에서 나왔지만, 대상 토큰이
`panel.matchHighlight` 자신이고 근거 논리도 이 표의 앞 7종과 같은 부류(같은 업스트림 파일의
`tokenColors` 재사용)라 여기 합류시켰다).

**재변환 시 나오는 값과 이유가 두 가지 서로 다른 가드에서 갈린다:**

- `github-dark`/`github-light`: 업스트림 classic 팔레트(`primer/github-vscode-theme`)가
  `list.highlightForeground` 를 정의하지 않고, 남는 유일한 후보
  `editor.findMatchHighlightBackground` 는 반투명(`#ffd33d22`/`#ffdf5d66`)이라
  `isOpaqueForegroundCandidate`(불투명 가드, d-31)가 배제한다. 두 후보가 모두 사라지면
  `status` 카테고리 공용 안전값(`SAFE_DEFAULT_COLORS.status`)으로 떨어져 `#569CD6`(dark)/
  `#0066BF`(light)가 나온다.
- `ayu-light`/`solarized-light`: `list.highlightForeground` 자체가 이미 불투명해
  불투명 가드를 그대로 통과하므로, 재변환은 업스트림 원본값(`#f29718`/`#B58900`)을
  **그대로** 재현한다 — 이 값이 바로 `panel.background` 대비 2.16:1/2.62:1 로 파이프라인
  대비 게이트(`MIN_CONTRAST_RATIO = 3`)에 미달했던 원래 결함값이다.
- `vscode-quiet-light`: 위 두 항목과 같은 이유로 재변환은 업스트림 원본값(`#9769dc`)을 그대로
  재현하지만, 이 값은 `panel.background` 축(3.49:1)은 **이미 통과**한다 — 실패하는 것은
  d-40(§8.2.4)이 추가한 `list.activeBackground` 축(2.59:1)뿐이다. 즉 §8.2.2 의 일반 대비
  게이트만 보면 결함이 아니지만, 선택 행 축까지 포함하면 재변환 시 결함값으로 되돌아간다.
- `monokai`/`palenight`/`night-owl-light`: `list.highlightForeground` 가 불투명하게
  정의돼 있지만(monokai `#f8f8f2`, palenight `#ffffff`, night-owl-light `#403f53`) 그
  값이 `app.foreground`(본문 전경)와 **픽셀 단위로 동일**하다 — 검색·팔레트 매치 강조가
  `font-semibold` 하나로만 남는 결함(d-33 "임무 C", CIE76 ΔE 0.0). 이를 막는
  `isDistinctFromBodyForeground`(구별성 가드, d-33 — `mapping-tables.ts`)가 세 값을 모두
  배제한다. 다음 후보인 `editor.findMatchHighlightBackground` 도 palenight·night-owl-light
  는 반투명(`#7e57c233`/`#93a1a16c`)이라 불투명 가드가, monokai 는 애초에 upstream 이 이
  토큰 자체를 정의하지 않아 후보가 사라져, 셋 다 결국 `status` 안전값으로 떨어진다 —
  monokai·palenight 는 dark(`#569CD6`), night-owl-light 는 light(`#0066BF`).

**그럼에도 손수정값을 유지하는 근거** — 어느 쪽도 값을 새로 발명하지 않고, 각 업스트림
팔레트 **안에서** 재선정했다(github-light `#735c0f` 선정과 동일한 "같은 스케일 내
인덱스/변형 이동" 논리 — 뒤 3종은 UI 색이 아니라 같은 테마의 `tokenColors` 구문 강조색
중 하나를 재사용했다):

| 테마 | 손수정값 | 대비비 | 재변환 시 나오는 값 | 그 값의 대비비 | 근거 |
|---|---|---|---|---|---|
| github-dark | `#ffd33d` | 10.91 | `#569CD6`(safe-default) | 5.31 | classic `colors.json` 의 yellow 스케일에서 원 반투명값과 같은 인덱스를 불투명화 |
| github-light | `#735c0f` | 6.04 | `#0066BF`(safe-default) | 5.40 | 같은 yellow 스케일 10개 인덱스 중 유일하게 AA(4.5) 통과하는 index9 |
| ayu-light | `#7e4b01` | 6.88 | `#f29718`(원본 그대로) | 2.16 | 같은 업스트림 `vscode-ayu` 테마의 `button.foreground`(accent 의 어두운 "on" 변형 — 같은 색상 계열, `scheme.common.accent.on`) |
| solarized-light | `#584c27` | 6.92 | `#B58900`(원본 그대로) | 2.62 | 같은 업스트림 `theme-solarized-light` 의 `activityBar.foreground`(같은 gold/olive 색상 계열의 어두운 변형) |
| monokai | `#E6DB74` | 11.63 | `#569CD6`(safe-default) | 5.62 | 업스트림 `microsoft/vscode` `theme-monokai` 의 `tokenColors`(string/regexp/link 등) 에서 반복 사용되는 노랑 — `app.foreground`(`#f8f8f2`)와 ΔE 50.65 로 뚜렷이 구별 |
| palenight | `#ffcb6b` | 9.10 | `#569CD6`(safe-default) | 4.63 | 업스트림 `whizkydee/vscode-palenight-theme` 의 `tokenColors`(variable/type/attribute 등) 에서 반복 사용되는 금색 — `app.foreground`(`#ffffff`)와 ΔE 56.85 로 뚜렷이 구별 |
| night-owl-light | `#aa0982` | 6.00 | `#0066BF`(safe-default) | 5.04 | 업스트림 `sdras/night-owl-vscode-theme` 의 `tokenColors`(number) 에서 사용되는 마젠타 — `app.foreground`(`#403f53`)와 ΔE 62.74 로 뚜렷이 구별 |
| vscode-quiet-light | `#7A3E9D` | 6.25 | `#9769dc`(원본 그대로) | 3.49 | `panel.background` 축은 원본값도 통과(3.49)하지만, d-40 이 추가한 `list.activeBackground` 축(§8.2.4)에는 2.59 로 미달 — 같은 업스트림 `microsoft/vscode` `theme-quietlight` 의 `tokenColors`(보라 계열) 재사용, `list.activeBackground` 대비도 4.63 로 함께 개선 |

뒤 3종은 safe-default(`#569CD6`/`#0066BF`)도 이미 AA 를 통과하므로(4.63~5.62), 손수정의
목적이 대비 확보가 아니라 **테마 정체성 보존**(외지 파랑 대신 그 테마 고유의 accent 색을
유지)이라는 점이 앞 4종(대비 미달 해소가 목적)과 다르다 — 그럼에도 대비는 손수정값 쪽이
더 크다(11.63/9.10/6.00 vs 5.62/4.63/5.04).

**대비 수리(`repairContrastPairs`, `contrast.ts`) 단계도 구별성 가드를 반영한다.** `panel.matchHighlight`
의 초기 후보가 배경 대비 3:1(`MIN_CONTRAST_RATIO`)에 미달해 수리가 발동하면(위 §8.2 마지막
불릿의 `repairContrastPairs`), 전경 후보 사슬은 이제 상태색 우선(`textLink.foreground` →
`button.background` → `focusBorder` → `activityBarBadge.background` → `badge.background` →
`tab.activeBorderTop`, `app.accent`/`appSidebar.badge`/`tabBar.tabActiveIndicator` 가 쓰는 것과
같은 사슬)이다 — 이 중 불투명·대비 3:1·`app.foreground` 와의 ΔE ≥ 2.3 을 모두 만족하는 첫
값을 채택한다(1패스). 1패스에서 아무 후보도 세 조건을 모두 만족하지 못하면(예: 업스트림이
상태색 자체를 정의하지 않았거나, 정의된 상태색도 본문 전경과 구별되지 않는 테마) 과거와 같은
`editor.foreground`/`foreground` 사슬로 폴백한다(2패스, 대비만 검사). 2패스로 떨어져 결과가
본문 전경과 동일해지는 경우에도 임포트를 **차단하지 않고** `repairs` 문구에 "본문 전경과
동일색 — 구별 가능한 후보 없음" 고지를 남긴다 — `panel.matchHighlight` 는 장식적 강조 토큰
하나이므로, 이 토큰 때문에 나머지 132개 토큰이 멀쩡한 테마 전체의 임포트를 막기보다는 사용자에게
알리는 쪽을 택했다. safe-default(`#569CD6`/`#0066BF`) 자체가 배경 대비 3:1 에 미달해 수리가
발동하는 경우는 번들 36종 어디서도 일어나지 않는다(safe-default 대비 최저 4.23) — 하지만
`bundled-theme-contrast.test.ts` 가 대비 게이트 예외로 등재한 `everforest-light`/
`rose-pine-dawn`처럼 원본 accent(`list.highlightForeground`)가 직접 3:1 미달인 테마를 오늘
재변환하면 이 수리 경로를 그대로 거친다(전자는 상태색 후보가 전부 탈락해 2패스 고지로,
후자는 `textLink.foreground` 의 accent 로 낙착).

**운영 지시**: 이 8개 테마를 재변환하면 `panel.matchHighlight` 1개 토큰의 diff 가 항상
non-zero 다 — 이는 예상된 것이므로 **산출물을 채택하지 말고 위 표의 손수정값을 다시
적용**한다. 그 외 토큰(colors/syntax/terminal 나머지 전부)의 diff 는 §8.2.2 게이트대로
원인(원본 갱신 여부)을 규명한다. (뒤 3종은 실제로 colors/syntax/terminal 나머지 전량이
diff 0 임을 오늘 시점 업스트림 재취득으로 직접 확인했다 — d-33 계약 참고.)

상호 참조: `docs/acknowledge/2026-08-24-d31-t2b-ts-batch-contract.md` §3-A(f-1 가드·f-3
정정 근거)·§3-E(번들 36종 재스윕 표) · `docs/acknowledge/2026-08-24-d33-restructure-carryover-contract.md`
"임무 C"(구별성 가드·번들 3종 정정·vs-전경 축 재스윕).

### 8.2.4 재변환 비재현 예외 — 선택 행 표면(`list.foreground`/`list.activeBackground`/`explorer.itemSelected`) 손수정

d-40(`docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §1-a)이 선택 행 배경
(`list.activeBackground`) 대비 게이트를 `panel.matchHighlight`/`list.foreground` 두 축에 추가하면서,
§8.2.3 과 같은 성격의 재변환 비재현 손수정이 두 토큰에도 생겼다. 손수정 방식은 §8.2.3 과 동일
(d-31 §3-A 방식 — 값을 새로 발명하지 않고 업스트림 팔레트 **안에서** 재선정, 재변환이 아님).
아래 표는 이 배치(d-40 §3-B) 손수정 12종 + 이후 검토에서 재정정된 3종(비고 열 참고)의 현재
상태다 — "재변환 시 나오는 값"은 두 경우 모두 파이프라인(`mapping-tables.ts`)이 바뀐 적이 없어
d-40 이전부터 지금까지 동일하다(손수정은 항상 커밋된 JSON 만 바꾼다).

| 테마 | 토큰 | 손수정값(현재) | 재변환 시 나오는 값 | 그 값의 `list.activeBackground` 대비비 | 근거 | 비고 |
|---|---|---|---|---|---|---|
| nord | `list.foreground` | — (해당 없음) | `#d8dee9` | 5.46(현재 `list.activeBackground` `#4c566a` 기준) | 검토 재정정으로 커밋값을 파이프라인 자연값(`#d8dee9`)으로 되돌려, 더 이상 손수정이 아니다 — 아래 §8.2.4 부기 참고 | **레지스트리 이탈**(재정정) |
| nord | `list.activeBackground` | `#4c566a`(nord3, Polar Night) | `#88c0d0`(nord8, frost 액센트) | matchHl 1.00 / listFg 1.48 | 새 배경 기준으로는 `list.foreground`(5.46)·`panel.matchHighlight`(3.69) 두 축을 동시에 통과시키고 `list.hoverBackground`/`list.background` 와도 ΔE 8.8/15.5 로 구별되는 값으로 재선정(업스트림 `list.inactiveSelectionBackground`) | **신규**(검토 재정정) |
| nord | `explorer.itemSelected` | `#4c566a` | `#88c0d0` | (해당 없음 — 이 축은 게이트 대상 아님) | `list.activeBackground` 와 분리되면 한 테마 안에 "선택" 색이 두 개가 되는 것을 막기 위해 동기화 | **신규**(검토 재정정) |
| vscode-monokai-dimmed | `list.activeBackground` | `#4e4e4e`(업스트림 `list.inactiveSelectionBackground`/`explorer.itemFocused`) | `#707070`(d-40 이전 원본) | matchHl 1.82 / listFg 2.94 | d-40 원안(`#404040`, `tabBar.tabInactiveBackground`)이 `list.hoverBackground`(`#444444`)와 ΔE 1.8 로 JND 미달이라 재선정 — matchHl 3.05·listFg 4.93·ΔE(hover) 4.3·ΔE(listBg) 17.5 전부 통과 | **재정정**(검토, d-40 값 대체) |
| vscode-abyss | `list.activeBackground` | `#000c18`(= `app.background`) | `#08286b`(d-40 이전 원본) | matchHl 2.18 / listFg 3.90 | 검토에서 재선정을 시도했으나 이 팔레트 안에 matchHl·listFg·ΔE(hover)·ΔE(listBg) 4 조건을 동시에 만족하는 값이 `#000c18` 외에 없음을 확인 — d-40 원안 유지 | 유지(검토 재확인) |
| vscode-tomorrow-night-blue | `list.activeBackground` | `#003f8e`(업스트림 `editor.selectionBackground`) | `#ffffff60`(반투명, d-40 이전 원본) | matchHl 1.44 / listFg 1.00 | d-40 §3-B — 같은 업스트림 파일의 다른 "선택" 개념 재사용 | 유지 |
| vscode-solarized-dark | `list.activeBackground` | `#274642`(업스트림 `editor.selectionBackground`) | `#005A6F`(d-40 이전 원본) | matchHl 3.36 / listFg 2.47 | d-40 §3-B | 유지 |
| everforest-light | `list.foreground` | `#5c6a72`(업스트림 `list.activeSelectionForeground`) | `#939f91`(d-40 이전 원본) | 2.12 | d-40 §3-B | 유지 |
| palenight | `list.foreground` | `#ffffff`(업스트림 `list.activeSelectionForeground`) | `#6C739A`(d-40 이전 원본) | 1.13 | d-40 §3-B | 유지 |
| rose-pine | `list.foreground` | `#e0def4`(업스트림 `list.activeSelectionForeground`) | `#908caa`(d-40 이전 원본) | 1.60 | d-40 §3-B | 유지 |
| ayu-dark | `list.foreground` | `#bfbdb6`(업스트림 `list.activeSelectionForeground`) | `#5a6378`(d-40 이전 원본) | 1.99 | d-40 §3-B | 유지 |
| ayu-light | `list.foreground` | `#5c6166`(업스트림 `list.activeSelectionForeground`) | `#828e9f`(d-40 이전 원본) | 2.19 | d-40 §3-B | 유지 |
| everforest-dark | `list.foreground` | `#d3c6aa`(업스트림 `list.activeSelectionForeground`) | `#859289`(d-40 이전 원본) | 2.47 | d-40 §3-B | 유지 |
| solarized-light | `list.foreground` | `#6C6C6C`(업스트림 `list.activeSelectionForeground`) | `#657B83`(d-40 이전 원본) | 2.75 | d-40 §3-B | 유지 |

**nord `list.foreground` 가 레지스트리를 이탈한 이유**: d-40 원안은 업스트림 `list.activeSelectionForeground`
(선택 행 전용, `#2e3440`)를 TAIDE 의 선택/비선택 공용 `list.foreground` 에 그대로 이식했다 —
`global.css` 의 `--accent-foreground: var(--taide-list-foreground)` 가 이 토큰을
`list.hoverBackground`(`--accent`) 위에도 그리므로(드롭다운/컨텍스트 메뉴·ghost 버튼 hover·focus
전체), 선택 행 1곳을 고치려다 훨씬 넓게 쓰이는 hover/메뉴 표면을 대비 7.45→1.24 로 무너뜨렸다
(검토 findings d40-listfg-multisurface-regression/d40-l2-01/D40-L3-01). 검토에서 `list.foreground`
는 파이프라인 자연값(`#d8dee9`)으로 되돌리고, 선택 행 축 자체는 `list.activeBackground` 를
nord3(`#4c566a`)로 재선정해 해소했다 — 두 축을 모두 만족하는 배경이 존재했으므로(위 표), 예외
등재도, `list.foreground` 손수정도 필요하지 않다.

**운영 지시**: 위 표에서 "재변환 시 나오는 값"이 "손수정값(현재)"과 다른 행(전부, `nord`
`list.foreground` 제외)은 재변환해도 그대로 채택하지 말고 표의 손수정값을 다시 적용한다. 그 외
132개 토큰의 diff 는 §8.2.2 게이트대로 원인을 규명한다.

### 8.3 Rust 등록

`src-tauri/src/domain/theme/service.rs` 의 `BUNDLED_THEME_SOURCES` 에
`(id, include_str!("../../../resources/themes/{id}.json"))` 항목을 추가하면
`builtin_by_id`/`list_themes`(`builtin: true`)/`extends` 해석에 자동으로 반영된다.
테스트는 번들 테마 전부가 §3 토큰 전량을 포함하고 `resolve_theme` 경고가 없는지
검증한다(`service.rs` `번들_테마는_모두_시맨틱_토큰_전량을_포함하고_경고가_없다`).

### 8.4 UI 노출

`ThemePicker`(`src/features/settings/theme-picker.tsx`) 는 `theme_list` 를
`builtin` 커스텀 여부가 아니라 **내장(TAIDE) / 번들(VS Code 변환) / 사용자**
3개 섹션으로 나눠 그린다(`settings.builtinThemesSection` /
`settings.bundledThemesSection` / `themeEditor.customThemes`). 각 카드에는
복제 버튼(`onDuplicate`)이 있어 번들 테마를 곧바로 `extends` 상속 복제할 수 있다.

## 9. VSIX 테마 임포트 (7.10-W5)

> 범위: **테마만.** VS Code 확장(`.vsix`)의 `contributes.themes` 기여점만 추출·변환해
> 로컬에 저장한다. **확장 실행(코드 실행형 extension host)은 공식적으로 미지원**이고,
> MS Marketplace 와의 네트워크 연동도 하지 않는다(사용자가 로컬에 내려받은 `.vsix`
> 파일만 dialog 로 선택). VS Code 확장의 `contributes.grammars`(TextMate 문법) 임포트·
> 스니펫·커맨드 등 다른 기여점은 다루지 않는다 — **7.10-W7 에서 TextMate 문법 렌더링
> 엔진(shiki, §4.2) 자체는 확정됐지만, VSIX 에서 grammar 를 추출해 신규 언어를 늘리는
> 기능은 W7 에서도 범위 밖이다**(`docs/backlog.md` — 언어 id 충돌 정책과
> `LANGUAGE_ID_BY_EXTENSION` 런타임화가 선행돼야 하는 별개 축). 플러그인의 `grammar`
> 기여(`docs/features/plugins.md` §2)로 같은 목적을 사용자가 직접 달성할 수는 있다.
> 설계 근거 전문은 `docs/features/vsix-theme-import.md`, IPC 계약은
> `docs/ipc-contract.md` "vsix" 절.

### 9.1 파이프라인

```
설정 > 외관 > "VSIX 에서 테마 가져오기…" 버튼
  → plugin-dialog open(.vsix 필터)
  → vsix_extract_themes(vsixPath)  (Rust, §9.2)
  → 테마별 include 체인 병합 + convertVscodeTheme()  (프론트, §8.2 변환 파이프라인 재사용)
  → 선택 목록(다크/라이트 배지 · 변환 경고 수 · 실패 항목 비활성 표시)
  → theme_save (선택한 테마마다)
  → theme_list invalidate + toast
```

- 변환은 `scripts/convert-vscode-theme.ts` 가 쓰던 **순수 로직을 `src/shared/lib/theme-convert/`
  로 이식**한 것을 그대로 재사용한다(CLI 스크립트와 임포트 플로우 2곳에서 쓰여 "2회 이상"
  공통화 기준을 충족) — `convertVscodeTheme(rawChain, type)`. 파일 IO·CLI 인자 파싱만
  `scripts/convert-vscode-theme.ts` 에 남아 있다. 포팅은 동작을 바꾸지 않았다 — 대표 1종
  (Monokai, 원본은 `microsoft/vscode` 레포에서 재취득)으로 재변환한 산출물이 기존
  `src-tauri/resources/themes/monokai.json` 과 바이트 단위로 diff 0 임을 실측했다.
- `vsix_extract_themes` 가 돌려주는 `includeChain[]` 은 **가장 구체적인 파일이 먼저**
  온다(현재 테마의 직속 부모, 그 다음 조부모 순 — `vsix-theme-import.md` §6). `convertVscodeTheme` 이
  기대하는 병합 순서(base 가 먼저, 가장 구체적인 것이 마지막에 와서 덮어쓴다)와 반대라
  프론트에서 `[...includeChain].reverse()` 로 뒤집은 뒤 테마 본문(`rawJson`)을 마지막에
  붙여 병합한다(`src/shared/lib/vsix-theme-import.ts` `buildRawChain`).
- `uiTheme`(`vs`/`vs-dark`/`hc-black`/`hc-light`) → TAIDE `ThemeType` 매핑은 `vs`/`hc-light`
  는 `light`, `vs-dark`/`hc-black` 은 `dark` 다(고대비 변형도 dark/light 두 갈래로 접는다 —
  TAIDE 는 별도 고대비 테마 타입이 없다).

### 9.2 id 충돌 — 조용한 덮어쓰기 금지

`theme_save` 는 같은 id 파일을 **조용히 덮어쓴다**(`service.rs` `save_theme` — 존재 여부를
확인하지 않고 바로 write). 임포트 id 는 확장의 `publisher`+`name`(매니페스트 필드, 항상
ASCII 인 안정 식별자)을 슬러그화해 만든다(`slugifyThemeId`, 여러 테마가 있으면 각 테마 라벨을
덧붙여 구분) — `displayName` 은 쓰지 않는다. NLS 플레이스홀더(`%displayName%`)가 남아 있거나
비ASCII 문자열이면 슬러그가 무너져 서로 다른 확장이 같은 id 로 충돌할 수 있기 때문이다
(`vsix-theme-import.md` §4-1·§9). 그래도 같은 확장을 다시 가져오거나 슬러그가 우연히 기존
커스텀 테마와 겹치면 사용자 모르게 기존 테마가 사라질 위험은 남는다.

임포트 목록에서 **저장 전에** 기존 `theme_list` id 와 겹치는 항목을 표시하고, 저장을
누르면 겹치는 항목이 있는 경우 확인 다이얼로그를 먼저 띄운다. 확인하면 겹치는 항목만
`generateUniqueThemeId`(기존 테마 편집기의 복제 로직과 동일 — `theme-draft.ts`)로 새
id 를 받아 **사본으로 저장**한다 — 기존 테마를 덮어쓰지 않는다(`settings.themeImportDuplicate`
로케일 문구 그대로: "이미 있어 사본으로 가져왔습니다"). 겹치지 않는 항목은 슬러그 id
그대로 저장된다.

### 9.3 변환 실패 항목

`convertVscodeTheme` 은 133 색상 전량을 항상 폴백으로 채우므로(§8.2 SAFE_DEFAULT_COLORS·
family fallback) `missingColors`/`missingSyntax`/`missingTerminal` 은 사실상 매핑 테이블
자체의 내부 정합성 검사에 가깝다. 임포트가 실제로 실패하는 경우는 둘뿐이다.

1. `rawJson`/`includeChain[].rawJson` 이 `parseJsonc` 로도 파싱되지 않는 손상된 JSON.
2. 대비 보정(`repairContrastPairs`) 후에도 `validateOutputColors` 가 최소 대비 미달을
   보고하는 경우(`outputColorErrors`).

두 경우 모두 해당 테마만 목록에서 **비활성(체크 불가)** 으로 표시하고, 다른 테마는 정상
가져올 수 있다(vsix 하나에 여러 테마가 있을 때 하나가 깨져도 나머지는 계속 진행 — Rust
쪽 `vsix_extract_themes` 의 "항목 하나가 깨져도 전체를 실패시키지 않는다" 원칙과 같은
맥락을 프론트 변환 단계에도 유지한다).
