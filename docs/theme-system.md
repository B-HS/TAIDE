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

### 3.1 토큰 소비처 (d-61 §1.C)

토큰은 정의만으로 끝나지 않는다. **각 토큰은 실제로 색을 칠하는 소비처가 하나 이상 있어야 한다.**
소비처 없이 늘어난 토큰은 테마 저작자에게 "고쳐도 화면이 안 바뀌는 칸"으로 보이므로, 아래 표를
정본으로 두고 `src/shared/styles/theme-token-consumer.test.ts` 가 기계 검사한다. **소비 = 실제로 칠하는
곳**이다: `global.css` 의 칠하는 규칙(`@layer base` · `@utility`)에 있는 `var()` 참조, 컴포넌트 코드의
`var()` 참조(`commit-graph.tsx` 의 `var(--taide-graph-lane${n})` 처럼 런타임 조립분 포함),
Monaco/xterm 색 맵의 토큰 id, 또는 그 토큰의 `--color-*` 키에서 생성된 Tailwind 유틸리티
클래스(`bg-menu-background` · `focus:bg-menu-item-hover` · `border-b-popover-separator` 등)의 실사용.
**브릿지 줄(`@theme inline` · `:root` shadcn 별칭)만 있는 것은 소비가 아니다** — 변수를 다른 이름으로
다시 내보낼 뿐이어서 어떤 표면도 쓰지 않을 수 있다(d-61 검토 G-6). 정의 레이어인
`shared/lib/theme-convert/**` · `entities/theme/theme-tokens.ts` 는 소비 근거에서 제외한다.

| 네임스페이스 | 소비처 |
|--------------|--------|
| `app.*` | `global.css` shadcn 브릿지(`--background`/`--foreground`/`--border`/`--ring`). `app.shadow` 는 `@utility shadow-overlay` · `shadow-overlay-lg` · `modal-scrim` 으로 드롭다운·컨텍스트메뉴·팝오버·다이얼로그 그림자와 모달 오버레이 스크림에 쓰인다 |
| `appSidebar.*` / `tabBar.*` / `panel.*` | `@theme inline` 유틸리티 → 사이드바·탭바·패널 위젯 |
| `explorer.*` | `features/explorer/file-tree*.tsx`. `folderIcon` 은 `shared/lib/file-icon.ts` 의 기본 폴더 아이콘 색, `gitModified`/`gitAdded`/`gitDeleted`/`gitUntracked`/`gitIgnored` 는 파일트리 행 데코 색(변환기가 `git.*` 과 같은 `gitDecoration.*ResourceForeground` 원천에서 파생한다 — `mapping-tables.ts`) |
| `editor.*` / `editorGutter.*` / `editorBlame.*` / `diff.*` | `shared/lib/monaco/theme.ts` 색 맵 + `global.css` 의 gutter·blame·conflict 클래스 |
| `terminal.*` | `shared/lib/xterm-theme.ts`(background/foreground/cursor/selection) + Monaco `textLink.foreground`(`linkForeground`) |
| `git.*` / `graph.*` / `statusIndicator.*` | git 패널·커밋 그래프·상태 아이콘 유틸리티 |
| `menu.*` | `shared/ui/dropdown-menu.tsx` · `context-menu.tsx`(배경·테두리·항목 hover·구분선). 팝오버 토큰에 의존하지 않는다 |
| `popover.*` | `shared/ui/popover.tsx`(배경·테두리). `separator` 는 팝오버 안 `Command` 입력 구분선(`**:data-[slot=command-input-wrapper]`) |
| `tooltip.*` | `shared/ui/tooltip.tsx`(배경·테두리) |
| `modal.*` | `shared/ui/dialog.tsx` · `alert-dialog.tsx`(배경·테두리·닫기 버튼 hover). `separator` 는 명령 팔레트 입력 구분선과 `vsix-import-dialog` 섹션 구분선 |
| `scrollbar.*` | `shared/scroll/overlay-scrollbar.tsx`(thumb·thumbHover·track) + `global.css` 의 네이티브 `::-webkit-scrollbar` 규칙(오버레이 스크롤바를 안 쓰는 잔여 표면) |
| `input.*` / `button.*` / `list.*` | `global.css` shadcn 브릿지(`--input`/`--primary`/`--secondary`/`--accent`) |

**소비처 없음(예외 등재)** — 테스트의 `CONSUMER_EXEMPTIONS` 와 1:1 이며, 소비처가 생기면 테스트가
예외 쪽에서 실패해 등재를 지우게 한다.

| 토큰 | 사유 |
|------|------|
| `terminal.commandBlockBorder` | OSC133 명령 블록 테두리 데코레이션이 아직 없다(터미널은 블록 성공/실패를 `statusIndicator` 색으로만 그린다) |
| `tooltip.itemHover` · `tooltip.separator` | 앱의 툴팁은 전부 한 줄 텍스트라 hover 대상 항목도 구분선도 없다. 툴팁에 항목/구획이 생기면 배선한다 |
| `graph.refTag` · `graph.refHead` | 커밋 그래프가 ref 배지를 종류와 무관하게 전부 `graph.refBranch` 로 그린다(`widgets/git-panel/commit-graph.tsx`). 태그/HEAD 를 구분해 그리게 되면 배선한다 |
| `popover.itemHover` | 팝오버는 자유 콘텐츠만 담고 hover 대상 항목 목록이 없다. 드롭다운·컨텍스트메뉴는 `menu.itemHover`, 명령 팔레트는 `list.activeBackground` 를 쓴다 |
| `input.focusBorder` | 입력 포커스 테두리를 전부 `app.focusBorder` 로 그린다(`focus:border-app-focus-border`). 입력 전용 포커스 색을 분리하게 되면 배선한다 |

위 3행(`graph.refTag`/`graph.refHead`·`popover.itemHover`·`input.focusBorder`)은 브릿지 줄만 보던
이전 검사가 소비로 세던 것들이다 — 유틸리티 실사용까지 보게 되면서 드러났다(d-61 검토 G-6).

**§1.E 스크린샷 검수 항목 (d-61 검토 G-4)** — §1.C 가 제거한 `dark:` 분기 6곳(`shared/ui/button.tsx` 4 ·
`dropdown-menu.tsx` · `context-menu.tsx`)은 "죽은 분기"가 아니라 **live 분기**였다. `global.css` 의
`@custom-variant dark (&:where([data-appearance='dark'], …))` 와 `app/providers/theme-provider.tsx` 의
`documentElement.dataset.appearance = theme.type` 이 다크 테마에서 실제로 적용시키고 있었다. 코드는
그대로 둔다(TAIDE 토큰이 이미 테마별 명시값을 주므로 shadcn 시절의 알파 오버라이드는 두 번째 테마
레이어가 된다). 대신 **다크 테마 스크린샷에서 아래 3항목을 육안 검수**한다 — 라이트와 같아지는
방향이 맞는지가 판정 기준이다.

- destructive 버튼 배경·테두리·hover (`shared/ui/button.tsx` `destructive` variant)
- outline · ghost 버튼 hover 배경 (같은 파일 `outline`/`ghost` variant)
- `aria-invalid` 포커스 링 (같은 파일, 제거한 `dark:aria-invalid:ring-destructive/40` 분기)

### 3.2 raw 색 예외 (d-61 검토 G-1)

컴포넌트는 hex·palette 를 직접 쓰지 않고 시맨틱 토큰만 쓴다(§4.1). Tailwind 내장 무채색
유틸리티(`text-white`·`bg-black/50` 등)도 **테마가 못 바꾸는 색**이라 같은 금지 대상이며,
`src/shared/styles/theme-token-consumer.test.ts` 의 `RAW_COLOR_EXEMPTIONS` 가 예외를 등재하고
나머지를 전부 막는다(예외에 없는 raw 색이 들어오면 실패, 예외가 실제로 안 쓰이면 stale 로 실패).

| 위치 | raw 색 | 사유 |
|------|--------|------|
| `shared/ui/button.tsx` `destructive` variant | `text-white` | 배경이 어떤 테마에서도 붉은 `statusIndicator.error`(`--destructive`)라 라벨은 테마와 무관하게 흰색이어야 한다. §1.C 가 토큰(`button.primaryForeground`)으로 바꿨더니 그 토큰은 **기본 버튼의 배경**을 섬기느라 붉은 배경 위에서 47종 중 19종이 3:1 미만으로 떨어졌다(흰색은 10종). 계측은 §8.6 의 `FIXED_FOREGROUND_CONTRAST_PAIRS` 자문 린트가 남긴다 |

§0 이 "raw 색 5곳"으로 세던 나머지 4곳(`dialog.tsx`/`alert-dialog.tsx` `bg-black/50`,
`html-preview.tsx` `bg-white`, `color-picker.tsx` `border-white` 2곳)은 §1.C 가 토큰으로 배선해
사라졌다.

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

내장 2종(TAIDE Dark/Light) 외에 인기 VS Code 테마 47종을 **번들 테마**로 함께 내장한다
(2026-08-28 d-46 으로 Dark/Light (Visual Studio - C/C++) 2종 추가, 2026-09-15 d-61 §1.D 로
형제 variant 9종 추가 — 이 문서의 과거 감사 서술에 남은 "36종"·"38종" 수치는 해당 감사 시점의
카탈로그 기준이며 재감사 전까지 그대로 둔다).
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
| `catppuccin-latte` | Catppuccin Latte | light | github.com/catppuccin/vscode |
| `catppuccin-frappe` | Catppuccin Frappé | dark | github.com/catppuccin/vscode |
| `catppuccin-macchiato` | Catppuccin Macchiato | dark | github.com/catppuccin/vscode |
| `tokyo-night-storm` | Tokyo Night Storm | dark | github.com/tokyo-night/tokyo-night-vscode-theme |
| `tokyo-night-light` | Tokyo Night Light | light | github.com/tokyo-night/tokyo-night-vscode-theme |
| `gruvbox-light` | Gruvbox Light | light | github.com/jdinhify/vscode-theme-gruvbox |
| `rose-pine-moon` | Rosé Pine Moon | dark | github.com/rose-pine/vscode |
| `ayu-mirage` | Ayu Mirage | dark | github.com/ayu-theme/vscode-ayu |
| `github-dark-dimmed` | GitHub Dark Dimmed | dark | github.com/primer/github-vscode-theme |

전부 MIT. 저작권 표시는 루트 `THIRD_PARTY_LICENSES.md` 를 따른다(MIT 는 저작권·허가
표시를 모든 사본에 포함해야 한다 — 색상값만 재가공한 파생물도 대상으로 취급).
`vscode-dark-plus`/`vscode-light-plus`/`vscode-dark-modern`/`vscode-light-modern`/
`vscode-kimbie-dark`/`vscode-red`/`vscode-quiet-light`/`darcula`/
`visual-studio-cpp-dark`/`visual-studio-cpp-light` 10종은 원본에
`terminal.ansi*` 색이 전혀 없어 §8.2 "VS Code 기본 ANSI 팔레트 폴백"이 적용됐다.

#### 8.1.1 d-61 §1.D 형제 variant 9종 — 선정 근거와 원본 출처

이미 번들된 테마의 **형제 variant**(같은 업스트림이 같은 팔레트로 내는 다른 밝기·색조)만
고른다. 계약 후보 11종 중 **Gruvbox Dark Hard·Dark Soft 2종을 제외**했다 — 둘은 이미 번들된
`gruvbox-dark`(Dark Medium)와 배경 명도만 다르고 액센트 팔레트가 동일해, 후보 중 기존
카탈로그와 가장 덜 구별되는 쌍이기 때문이다. 대신 Gruvbox 는 **카탈로그에 없던 라이트**를
채우는 `gruvbox-light` 를 넣었다.

라이트 비율: 신규 9종 중 3종(`catppuccin-latte`·`tokyo-night-light`·`gruvbox-light`)이
light = **33.3%**, 카탈로그 전체로는 12 → 15 / 47 = **31.9%** 로 둘 다 계약의 ≥ 20% 를 만족한다.

| id | 원본 파일 | 고정 ref |
|----|-----------|----------|
| `catppuccin-latte` | `extension/themes/latte.json` | `catppuccin-vsc-v3.19.0` 릴리스 자산 `catppuccin-vsc-3.19.0.vsix` |
| `catppuccin-frappe` | `extension/themes/frappe.json` | 〃 |
| `catppuccin-macchiato` | `extension/themes/macchiato.json` | 〃 |
| `tokyo-night-storm` | `themes/tokyo-night-storm-color-theme.json` | 커밋 `7c0f11eaef322f293621ca7befe462214b7ea468` |
| `tokyo-night-light` | `themes/tokyo-night-light-color-theme.json` | 〃 |
| `gruvbox-light` | `extension/themes/gruvbox-light-medium.json` | 마켓플레이스 `jdinhlife.gruvbox` 1.29.1 VSIX (= 태그 `v1.29.1`) |
| `rose-pine-moon` | `themes/rose-pine-moon-color-theme.json` | 태그 `v2.15.2` |
| `ayu-mirage` | `ayu-mirage.json` (저장소 루트) | 커밋 `444ef92911cb75c3933c8003e3a7c79b6b6c914f` |
| `github-dark-dimmed` | `extension/themes/dark-dimmed.json` | 마켓플레이스 `GitHub.github-vscode-theme` 6.3.5 (= 태그 `v6.3.5`) |

**VSIX 경유 3 저장소의 이유**: `catppuccin/vscode`·`primer/github-vscode-theme`·
`jdinhify/vscode-theme-gruvbox` 는 테마 JSON 을 **릴리스 시 생성**하고 저장소에 커밋하지
않는다(각각 `build.ts`/`src` 빌드, `themes/` 가 `.gitignore` 대상). 그래서 "저장소 태그의 raw
파일"이 존재하지 않아, 그 버전의 **발행 산출물**에서 꺼냈다 — Catppuccin 은 프로젝트 자신의
GitHub 릴리스 자산이고, 나머지 둘은 마켓플레이스 발행본이되 **버전이 GitHub 태그와 정확히
일치**(6.3.5 = `v6.3.5`, 1.29.1 = `v1.29.1`)하는 것을 확인해 태그와 대응시켰다. 나머지 3
저장소는 계약대로 태그/커밋의 raw 파일을 그대로 받았다.

**`tokyo-night-light` 의 `type`**: 원본 테마 JSON 안의 `"type": "dark"` 는 업스트림 오기다.
확장 매니페스트(`package.json` `contributes.themes`)가 이 파일을 `"uiTheme": "vs"` 로 등록하고
있어 **light 로 변환**했다(`--type light`).

**변환 결과**: 9종 전부 `--include-dir` 불필요(include 체인 없음), 184 토큰 전량 명시,
`terminal.ansi*` 16색 원본 보유라 ANSI 폴백 0건, safe-default 폴백 0건. §1.A 구별성·§1.B 대비
수리는 변환 파이프라인 안에서 자동 적용됐고(`catppuccin-*` 3종·`tokyo-night-light`·
`gruvbox-light`·`github-dark-dimmed` 6종에서 발동), 잔존 `stateDistinctnessErrors` 는 0 이다.

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
아래 표는 이 배치(d-40 §3-B) 손수정 12종 + 이후 검토에서 재정정된 3종(비고 열 참고)
+ d-61 §1.D 신규 추가분 1종의 현재 상태다 — "재변환 시 나오는 값"은 모든 경우
파이프라인(`mapping-tables.ts`)이 바뀐 적이 없어 d-40 이전부터 지금까지 동일하다(손수정은 항상
커밋된 JSON 만 바꾼다).

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
| ayu-mirage | `list.foreground` | `#cccac2`(업스트림 `list.activeSelectionForeground`) | `#707a8c`(업스트림 `foreground` — 이 테마는 `sideBar.foreground` 를 정의하지 않아 체인이 여기로 떨어진다) | 2.77 | d-61 §1.D — 같은 업스트림(`ayu-theme/vscode-ayu`)의 `ayu-dark`/`ayu-light` 와 **동일 처방**. ayu 계열은 `list.activeSelectionBackground` 가 자기 `list.hoverBackground` 와 같아(`#63759926`) §8.2.2 가드가 VS Code 기본값 `#04395E` 로 폴백시키는데, 그 위에서 dim 한 chrome 전경(`#707a8c`)이 2.77 로 미달한다. nord 회귀(아래 부기) 재발 점검 결과 `#cccac2` 는 `list.background`(3.58→9.45)·`list.hoverBackground`(1.07→2.82)·`app.background`(3.36→8.85) 전 표면에서 개선이라 다표면 역효과 없음 | **신규**(d-61) |

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

**등록 누락·오등록 게이트 (d-61 §1.D).** `BUNDLED_THEME_SOURCES` 는 손으로 적는 목록이라 두 가지로 어긋날 수 있다 — 파일을 추가하고 행을
빠뜨리면 그 테마는 바이너리에 실려만 있고 `list_themes` 에 끝내 나타나지 않으며, 행과 파일을 잘못 짝지으면 `builtin_by_id` 가 한 id 에
다른 테마의 색을 돌려준다. `번들_테마_등록_배열은_리소스_디렉터리_및_theme_id_와_일치한다` 가 `resources/themes/*.json` 파일 목록과
등록 id 집합이 같은지, 각 행이 include 한 파일의 `theme.id` 가 그 행의 id 와 같은지를 함께 본다. TS 쪽 `bundled-theme-licenses.test.ts`
가 `THIRD_PARTY_LICENSES.md` 에 대해 같은 두 사실을 보는 것과 짝이다.

**카탈로그 린트 Rust 미러 (d-61 §1.A·§1.B).** §8.5·§8.6 의 쌍 표는 TS 가 정본이고, Rust 는 같은 표·임계·합성 규칙을 복제해 카탈로그
린트로 돌린다.

| Rust 테스트 | 대응 TS 정본 |
| --- | --- |
| `카탈로그_테마는_상태색이_바탕색과_구별된다` | `state-distinctness-pairs.ts` 50행 + `app.shadow` 알파 축 + `bundled-theme-state-distinctness.test.ts` |
| `카탈로그_테마는_컴포넌트_전경색이_실제_배경과_최소_대비를_가진다` | `component-contrast-pairs.ts` 36행 + `bundled-theme-contrast.test.ts` |
| `상태색_구별성_쌍_표는_ts_정본과_일치한다` · `컴포넌트_대비_쌍_표는_ts_정본과_일치한다` | 위 두 파일을 `include_str!` 로 다시 읽어 행 순서·라벨·키·표면·대안·형제 플래그·임계와 상수 4종(`STATE_MIN_DISTINCT_DELTA_E`·`SUBTLE_STATE_MIN_DISTINCT_DELTA_E`·`APP_SHADOW_MIN_ALPHA`·`MIN_CONTRAST_RATIO`)을 통째로 대조 |
| `형제_쌍은_양쪽을_표면_위에_독립_합성해_비교한다` | `state-distinctness.test.ts` 의 형제 쌍 단위 테스트 — 표가 아니라 *측정 방식*을 고정한다(아래 형제 규칙) |
| `컴포넌트_대비_예외_등재분은_실제로_최소_대비에_미달한다` | `bundled-theme-contrast.test.ts` 의 `예외 등재분은 실제로 등재된 축에서만 위반한다` |

Rust 미러에는 행마다 붙는 주석이 없다. 유래(어느 컴포넌트·클래스에서 나온 쌍인지)는 TS 정본이 단일 출처이고, 같은 산문을 양쪽에 두면
드리프트 테스트가 검사하지 못하는 곳이 하나 더 생기기 때문이다. ΔE76·알파 합성·WCAG 대비 계산은 `shared/lib/color.ts`·`contrast.ts` 의
함수를 상수·연산 순서까지 그대로 이식했다. 실측 최소 여유는 상태색 ΔE +0.0056(`catppuccin-macchiato` `panelInputBorder`), 컴포넌트 대비
+0.00014(`everforest-light` `statusBarSuccess`) 로 부동소수 오차보다 훨씬 크다.

**순회 범위는 `theme_catalog()` — 번들 47 + Rust 리터럴 2.** TS 게이트는 번들 JSON 47종만 보지만 Rust 린트는 d-36 §1-b 의 결정대로
`builtin_dark`/`builtin_light` 까지 본다. 그 결정의 계기가 정확히 이 사각지대였다(`taide-light` 의 `panel.matchHighlight` 결함이
`bundled_themes()` 만 도는 린트를 통과해 출시됐다). 예외 등재는 구별성 린트 0건, 컴포넌트 대비 린트 2건
(`COMPONENT_CONTRAST_EXEMPTIONS` = ayu 2종의 `menuItemHoverText`, §8.6)이고 Rust 는 TS 레지스트리를 그대로 복제한다. 선택 행 전경색
예외(`SELECTION_FOREGROUND_CONTRAST_EXEMPTIONS`)는 형제 규칙이 `rose-pine-dawn` 의 전제를 없애 **양쪽 모두 빈 레지스트리**가 됐다 —
등재분 역검증 테스트가 남아 있어 통과하는 테마가 등재되어 있으면 실패한다.

**형제 쌍·그림자 축 (d-61 검토 A-1·G-2, Rust 미러).** 형제 쌍(`sibling: true` 5행)은 state 를 container 위에 올려 재지 않고 **양쪽을
`surfaceKey` 위에 독립 합성한 뒤** 비교한다. 같은 반투명 오버레이를 쓰는 두 상태(ayu 의 `explorer.itemSelected`/`itemHover`)를 쌓아서
재면 두 번 틴트돼 ΔE 6+ 로 통과하기 때문이다. 이 규칙은 쌍 표 대조로는 잡히지 않아서(플래그를 읽고 무시해도 표는 일치한다) 드리프트
키에 `sibling` 을 넣고, 합성 순서 자체는 별도 단위 테스트가 고정한다. `app.shadow` 는 비교할 컨테이너가 없어 ΔE 대신 자기 알파를
`APP_SHADOW_MIN_ALPHA`(0.149 = VS Code 라이트 `widget.shadow` 기본값 `#00000026`)와 비교한다. 카탈로그 실측 최소 여유는 알파
+0.00002(38/255 정확값이라 결정적)다.

**`taide-light` `app.shadow` 정정 1토큰 (d-61 검토 G-2, Rust 리터럴).** `light_colors()` 의 `#00000022`(알파 0.133)가 위 하한에
미달해 `#00000026` 으로 올렸다. `taide-dark` 는 `#00000066` 으로 통과다.

**`taide-light` 정정 11토큰 (d-61 §1.B, Rust 리터럴).** 위 범위 확장으로 `light_colors()` 가 컴포넌트 대비 13축에서 미달했다(2.15~2.83).
§8.6 의 수리 규칙을 그대로 적용해(색상 유지, 검정·흰색 양 끝으로 최소 이동) 전경 토큰 11개를 옮겼다. `taide-dark` 는 두 린트 모두 정정
0건이다. 쌍 표에 없는 토큰(에이전트 상태 아이콘 `appSidebar.iconAgent*` 등)은 원래 값 그대로다.

| 토큰 | 전 → 후 | 걸린 축(대비) |
| --- | --- | --- |
| `explorer.gitAdded` · `git.added` · `statusIndicator.success` | #40a02b → #3d9829 | explorerGitAdded · gitAdded · statusBarSuccess/problemSuccess (2.75) |
| `explorer.gitModified` · `git.modified` · `statusIndicator.warning` | #df8e1d → #ba7718 | explorerGitModified · gitModified · statusBarWarning/problemWarning (2.15) |
| `git.renamed` | #209fb5 → #1d92a6 | gitRenamed (2.58) |
| `git.conflicted` | #fe640b → #e3590a | gitConflicted (2.45) |
| `input.placeholder` | #9ca0b0 → #878b99 | inputPlaceholder (2.30) |
| `tabBar.tabInactiveForeground` | #8c8fa1 → #828596 | tabInactive (2.63) |
| `tabBar.previewForeground` | #8c8fa1 → #878a9b | tabPreview (2.83) |

### 8.4 UI 노출

`ThemePicker`(`src/features/settings/theme-picker.tsx`) 는 `theme_list` 를
`builtin` 커스텀 여부가 아니라 **내장(TAIDE) / 번들(VS Code 변환) / 사용자**
3개 섹션으로 나눠 그린다(`settings.builtinThemesSection` /
`settings.bundledThemesSection` / `themeEditor.customThemes`). 각 카드에는
복제 버튼(`onDuplicate`)이 있어 번들 테마를 곧바로 `extends` 상속 복제할 수 있다.

### 8.5 상태색 구별성 린트 (d-61 §1.A)

번들 테마는 §3 토큰 184종을 **전량 명시**하므로 "누락 폴백"은 0이다. 실제 위험은 그 반대 —
**서로 다른 토큰이 같은 값으로 접혀 상태·경계가 화면에서 사라지는 것**이다(에디터 드래그 선택이
에디터 배경과 같은 색, 설정 토글의 켜짐 트랙이 카드 배경과 같은 색 등). 대비(§8.2 `contrast.ts`)는
전경/배경 축만 보므로 이 결함을 잡지 못한다.

**정본 파일**

| 파일 | 역할 |
|---|---|
| `src/shared/lib/theme-convert/state-distinctness-pairs.ts` | 쌍 표(`STATE_DISTINCTNESS_PAIRS`) + 임계 상수 2종. Rust 미러가 그대로 복제하는 원본 |
| `src/shared/lib/theme-convert/state-distinctness.ts` | `validateStateDistinctness` / `repairStateDistinctness` |
| `src/shared/lib/theme-convert/convert.ts` | 변환 파이프라인 통합(수리 → 대비 수리 → `stateDistinctnessErrors` 보고) |
| `src/shared/lib/theme-convert/bundled-theme-state-distinctness.test.ts` | 번들 47종 전수 게이트(예외 등재 0) + `terminal` 미러 일치 |
| `scripts/repair-theme-state-distinctness.ts` | 번들 JSON 정정(`bun run themes:repair-state-distinctness`, 멱등) |

**쌍 표 도출 기준.** 토큰 어휘가 아니라 **코드**에서 뽑는다 — `global.css` 의 `@theme inline` 이
각 `--taide-*` 토큰을 컴포넌트가 쓰는 Tailwind 색으로 연결하고, 나머지는 `monaco/theme.ts` 의
`MONACO_COLOR_SOURCE` 와 `xterm-theme.ts` 가 두 임베디드 렌더러로 연결한다. 한 쌍이 표에 오르려면
① 두 토큰이 동시에 화면에 있을 수 있고(또는 하나가 다른 하나 위에 직접 그려지고) ② 접혔을 때
사용자가 읽어야 할 정보가 사라져야 한다. 각 행의 유래(컴포넌트·클래스)는 파일 안 JSDoc 1줄로
남긴다.

**판정.**

1. `surfaceKey` 가 있으면 컨테이너를 그 표면 위에 먼저 합성한다. 반투명 컨테이너를 raw RGB 로 읽으면
   오탐이 난다 — dracula 는 `explorer.itemHover` 를 `#44475A75`, `explorer.itemSelected` 를
   `#44475A` 로 둬서 문자열은 다르지만 raw RGB 는 같고, 화면에서는 뚜렷이 다르다.
2. 상태색을 그 컨테이너 위에 합성한 뒤 `deltaE76`(CIE76 ΔE\*ab, `shared/lib/color.ts`)로 거리를 잰다.
3. **`sibling: true` 인 쌍은 예외다 (d-61 검토 A-1).** 컨테이너가 표면이 아니라 *형제 상태*인 쌍
   (선택 행 vs hover 행, 켜짐 트랙 vs 꺼짐 트랙 등 5쌍)은 둘 다 `surfaceKey` 위에 그려지므로
   **양쪽을 그 표면 위에 각각 합성한 뒤** 비교한다. 상태를 컨테이너 *위에* 얹으면 같은 오버레이가
   두 번 칠해져, 화면에서 똑같이 보이는 두 행이 ΔE 6+ 로 통과한다 — ayu 3종이 `explorer.itemHover`
   와 `itemSelected` 에 같은 `#47526640` 을 쓰는 것이 이 방식으로 통과하던 사례다.
4. `alternativeStateKey` 가 있으면 둘 중 하나만 임계를 넘으면 통과한다(평평한 탭 스트립 +
   활성 탭 인디케이터).
5. 어느 한쪽이 hex 가 아니면(`transparent`, 미해석 `@palette` 참조) **측정 불가로 건너뛴다** — 위반이
   아니다.
6. `app.shadow` 는 비교할 컨테이너가 없는 유일한 축이다(앱의 모든 표면 위에 그려진다). 대신 **자기
   알파**를 `APP_SHADOW_MIN_ALPHA` 와 비교한다 — 아래 "그림자 축" 참고.

**임계값 근거.** 기본 `STATE_MIN_DISTINCT_DELTA_E = 2.3` 은 CIE76 의 통상적 JND(just noticeable
difference)이자 이미 코드베이스가 쓰는 구별성 바닥값(`mapping-tables.ts` 의
`MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E`)이라 두 린트가 "다른 색"의 정의를 공유한다. 관례상 옅게
두는 3축(현재 줄 강조·비활성 선택·보조 찾기 매치)만 `SUBTLE_STATE_MIN_DISTINCT_DELTA_E = 1`
("육안 식별 불가" 경계)을 써서 **완전히 접힌 경우만** 잡는다. 정정 전 38종 실측 분포는 아래와 같고,
모든 축에서 임계가 빈 구간에 떨어진다(접힌 값 0.00~2.19, 임계 이상 최솟값 2.32+).

> 아래 표는 **d-61 §1.A 시점(번들 38종·쌍 표 37행)의 측정**이다. d-61 검토(A-1·A-2)가 형제 판정을
> 바꾸고 행을 14개 늘렸으며, 그 시점의 재측정(47종)은 뒤의 "검토 재측정" 표에 따로 둔다. 47종·전 축
> 기준으로도 임계는 여전히 빈 구간에 떨어진다 — 임계 미만 최댓값 2.26(`tokyo-night-light`
> `listHoverOnActiveTab`), 임계 이상 최솟값 2.31(`palenight` `switchCheckedVsUncheckedTrack`).

| 축(label) | 상태 → 바탕 | 임계 | ΔE 0.00 | 0<ΔE<임계 | 임계 이상 최솟값 | 중앙값 | 정정 |
|---|---|---|---|---|---|---|---|
| `editorSelection` | `editor.selection` → `editor.background` | 2.3 | 6 | 0 | 4.06 | 13.52 | 6 |
| `editorInactiveSelection` | `editor.inactiveSelection` → `editor.background` | 1 | 17 | 0 | 2.28 | 4.33 | 17 |
| `editorCurrentLine` | `editor.lineHighlight` → `editor.background` | 1 | 7 | 0 | 2.17 | 3.99 | 7 |
| `editorFindMatch` | `editor.findMatch` → `editor.background` | 2.3 | 0 | 0 | 4.08 | 42.55 | 0 |
| `editorFindMatchHighlight` | `editor.findMatchHighlight` → `editor.background` | 1 | 0 | 0 | 7.83 | 25.56 | 0 |
| `editorBracketMatch` | `editor.bracketMatch` → `editor.background` | 2.3 | 5 | 1 | 5.78 | 14.05 | 6 |
| `editorCursor` | `editor.cursor` → `editor.background` | 2.3 | 0 | 0 | 15.66 | 74.27 | 0 |
| `terminalSelection` | `terminal.selection` → `terminal.background` | 2.3 | 21 | 3 | 2.42 | 0.00 | 24 |
| `terminalCursor` | `terminal.cursor` → `terminal.background` | 2.3 | 0 | 0 | 15.66 | 76.21 | 0 |
| `listHover` | `list.hoverBackground` → `list.background` | 2.3 | 2 | 2 | 2.38 | 6.90 | 4 |
| `listActive` | `list.activeBackground` → `list.background` | 2.3 | 0 | 0 | 3.91 | 25.08 | 0 |
| `listActiveVsHover` | `list.activeBackground` → `list.hoverBackground` | 2.3 | 0 | 1 | 3.50 | 17.21 | 1 |
| `explorerItemHover` | `explorer.itemHover` → `explorer.background` | 2.3 | 0 | 2 | 2.38 | 7.12 | 2 |
| `explorerItemSelected` | `explorer.itemSelected` → `explorer.background` | 2.3 | 0 | 0 | 2.77 | 15.48 | 0 |
| `explorerItemFocused` | `explorer.itemFocused` → `explorer.background` | 2.3 | 0 | 0 | 2.77 | 10.30 | 0 |
| `explorerSelectedVsHover` | `explorer.itemSelected` → `explorer.itemHover` | 2.3 | 4 | 2 | 3.50 | 8.95 | 6 |
| `sidebarItemHover` | `appSidebar.itemHover` → `appSidebar.background` | 2.3 | 5 | 2 | 2.32 | 4.33 | 7 |
| `sidebarItemActive` | `appSidebar.itemActive` → `appSidebar.background` | 2.3 | 4 | 1 | 2.38 | 8.54 | 5 |
| `sidebarActiveVsHover` | `appSidebar.itemActive` → `appSidebar.itemHover` | 2.3 | 8 | 2 | 3.50 | 7.38 | 10 |
| `sidebarActiveOnCard` | `appSidebar.itemActive` → `panel.background` | 2.3 | 5 | 0 | 2.77 | 8.54 | 5 |
| `sidebarHoverOnCard` | `appSidebar.itemHover` → `panel.background` | 2.3 | 6 | 2 | 2.38 | 6.32 | 8 |
| `sidebarBadge` | `appSidebar.badge` → `appSidebar.background` | 2.3 | 0 | 0 | 30.34 | 63.66 | 0 |
| `tabActive` | `tabBar.tabActiveBackground` → `tabBar.background` | 2.3 | 0 | 0 | 4.06 | 54.47 | 0 |
| `tabActiveVsInactive` | `tabBar.tabActiveBackground` → `tabBar.tabInactiveBackground` | 2.3 | 0 | 0 | 4.06 | 54.47 | 0 |
| `switchCheckedTrackOnCard` | `button.primaryBackground` → `panel.background` | 2.3 | 5 | 0 | 5.99 | 48.75 | 5 |
| `switchCheckedTrackVsThumb` | `button.primaryBackground` → `app.background` | 2.3 | 6 | 0 | 5.99 | 49.00 | 6 |
| `switchUncheckedTrackOnCard` | `input.border` → `panel.background` | 2.3 | 3 | 0 | 2.39 | 9.68 | 3 |
| `switchUncheckedTrackVsThumb` | `input.border` → `app.background` | 2.3 | 2 | 0 | 3.29 | 11.62 | 2 |
| `switchCheckedVsUncheckedTrack` | `button.primaryBackground` → `input.border` | 2.3 | 1 | 0 | 8.69 | 43.42 | 1 |
| `buttonHover` | `button.hoverBackground` → `button.background` | 2.3 | 8 | 0 | 3.17 | 8.08 | 8 |
| `inputBorder` | `input.border` → `input.background` | 2.3 | 5 | 1 | 2.44 | 9.54 | 6 |
| `panelInputBorder` | `panel.inputBorder` → `panel.inputBackground` | 2.3 | 5 | 1 | 2.44 | 9.54 | 6 |
| `focusBorderOnApp` | `app.focusBorder` → `app.background` | 2.3 | 4 | 1 | 4.06 | 32.26 | 5 |
| `focusBorderOnCard` | `app.focusBorder` → `panel.background` | 2.3 | 4 | 1 | 4.06 | 30.08 | 5 |
| `inputFocusBorder` | `input.focusBorder` → `input.background` | 2.3 | 5 | 0 | 2.58 | 26.46 | 5 |
| `menuItemHover` | `menu.itemHover` → `menu.background` | 2.3 | 3 | 0 | 2.38 | 7.51 | 3 |
| `scrollbarThumb` | `scrollbar.thumb` → `scrollbar.track` | 2.3 | 0 | 0 | — | — | 0 |

> `정정` 열은 그 축이 임계 미달로 잡아낸 테마 수다. 합(163)이 실제 정정 토큰 수(139)보다 큰 것은 한
> 토큰을 고치면 그 토큰이 걸린 다른 축이 함께 해소되기 때문이다. `scrollbarThumb` 는 `scrollbar.track`
> 이 매핑 단계에서 항상 `transparent` 로 파생되어 번들 테마에서는 측정 불가(비활성) 축이며, 손수정·
> 사용자 저장 테마가 불투명 트랙을 넣는 경우를 위해 남겨 둔다.

**수리 규칙.**

- **바탕이 아니라 상태 토큰을 고친다.** 컨테이너는 다른 토큰들이 기준으로 삼는 표면이라, 그걸
  움직이면 접힌 쌍 하나를 여러 개로 늘린다.
- `alternativeStateKey` 가 있는 쌍은 **대안 쪽**을 고친다. 활성 탭 배경이 탭 스트립과 같은 색인 것은
  의도적인 플랫 디자인이고, 그 디자인이 포기한 구별을 대신 지라고 있는 토큰이 인디케이터다.
- 후보 선택: ① 테마 자신의 `palette` 중 조건을 만족하면서 **교체 대상 색에 가장 가까운** 값(동률은
  palette 키 순서로 결정), ② 없으면 `mix(container, app.foreground, t)` 의 t 를 256단계 sRGB 구간에서
  **이진 탐색**해 임계를 막 넘는 최소값으로 파생한다. 탐색은 상한을 항상 "검증된 통과 단계"에만
  두므로 결과는 반드시 임계를 넘는다.
- 결과는 **불투명 6자리 hex** 다. 원래 알파를 유지하면 컨테이너와 다시 섞여 방금 확보한 거리가
  줄어든다.
- 같은 토큰이 여러 쌍에 걸리므로 **표 전체를 변화가 없을 때까지 반복**한다(최대 4패스). 결정적·
  멱등이라 스크립트를 몇 번 돌려도 같은 결과다.
- 변환 파이프라인에서는 **상태색 수리를 대비 수리보다 먼저** 돌린다. 대비 수리는 배경을 기준으로
  전경을 고르므로 배경이 나중에 움직이면 결과가 무효가 되지만, 반대 방향은 안전하다(대비 수리가
  쓰는 토큰 중 구별성 쌍의 컨테이너·표면인 것이 없다).
- VSIX 임포트를 **막지 않는다**. 남은 위반은 `outputColorErrors` 가 아니라 별도
  `stateDistinctnessErrors` 로 나가고 CLI 는 경고만 찍는다(신규 임포트 거부 케이스 금지 —
  `docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §1-a).

**데이터 정정 결과.** `bun run themes:repair-state-distinctness` 1회 실행으로 **38종 중 36종, 139개
토큰**이 정정됐다(재실행 시 0건). 예외 등재는 0 — 모든 축이 구조적으로 수리 가능하므로(전경과
배경조차 구분되지 않는 테마는 이미 `validateOutputColors` 가 거른다) "예외"는 곧 "스크립트를 안
돌렸다"는 뜻이 된다.

| 테마 | 정정 토큰 (이전 → 이후, 축) |
|---|---|
| `ayu-dark` | `terminal.selection` #10141c → #12151d (terminalSelection) |
| `ayu-light` | `terminal.selection` #fcfcfc → #f1f2f4 (terminalSelection) |
| `catppuccin-mocha` | `editor.inactiveSelection` #1e1e2e → #212131 (editorInactiveSelection)<br>`input.border` #00000000 → #1d1d2b (switchUncheckedTrackOnCard)<br>`input.border` #1d1d2b → #232333 (switchUncheckedTrackVsThumb)<br>`panel.inputBorder` #00000000 → #36374a (panelInputBorder) |
| `darcula` | `editor.inactiveSelection` #242424 → #272727 (editorInactiveSelection)<br>`terminal.selection` #242424 → #292929 (terminalSelection)<br>`appSidebar.itemActive` #242424 → #292929 (sidebarActiveVsHover)<br>`appSidebar.itemHover` #242424 → #292929 (sidebarHoverOnCard)<br>`button.primaryBackground` #242424 → #292929 (switchCheckedTrackOnCard)<br>`button.hoverBackground` #242424 → #292929 (buttonHover)<br>`menu.itemHover` #242424 → #292929 (menuItemHover)<br>`appSidebar.itemActive` #292929 → #2e2e2e (sidebarActiveVsHover) |
| `dracula` | `editor.inactiveSelection` #282A36 → #2b2d39 (editorInactiveSelection)<br>`editor.lineHighlight` #282A36 → #2b2d39 (editorCurrentLine)<br>`terminal.selection` #282A36 → #2d2f3a (terminalSelection) |
| `everforest-dark` | `editor.bracketMatch` #2d353b00 → #323a3f (editorBracketMatch)<br>`terminal.selection` #2d353b → #323a3f (terminalSelection)<br>`list.hoverBackground` #2d353b00 → #323a3f (listHover)<br>`appSidebar.itemHover` #2d353b00 → #323a3f (sidebarItemHover)<br>`app.focusBorder` #2d353b00 → #323a3f (focusBorderOnApp)<br>`input.focusBorder` #2d353b00 → #323a3f (inputFocusBorder) |
| `everforest-light` | `editor.bracketMatch` #fdf6e300 → #f5efde (editorBracketMatch)<br>`terminal.selection` #fdf6e3 → #f5efde (terminalSelection)<br>`list.hoverBackground` #fdf6e300 → #f5efde (listHover)<br>`appSidebar.itemHover` #fdf6e300 → #f5efde (sidebarItemHover)<br>`app.focusBorder` #fdf6e300 → #f5efde (focusBorderOnApp)<br>`input.focusBorder` #fdf6e300 → #f5efde (inputFocusBorder) |
| `github-dark` | `editor.bracketMatch` #17E5E600 → #292e33 (editorBracketMatch) |
| `github-light` | `editor.bracketMatch` #34d05800 → #f8f8f8 (editorBracketMatch) |
| `gruvbox-dark` | `editor.inactiveSelection` #282828 → #2b2b2a (editorInactiveSelection)<br>`editor.bracketMatch` #28282800 → #2d2d2c (editorBracketMatch)<br>`terminal.selection` #282828 → #2d2d2c (terminalSelection)<br>`explorer.itemSelected` #3c383680 → #373533 (explorerSelectedVsHover)<br>`appSidebar.itemActive` #3c383680 → #373533 (sidebarActiveVsHover) |
| `intellij-islands-light` | `input.border` #00000000 → #f8f8f8 (switchUncheckedTrackOnCard)<br>`panel.inputBorder` #00000000 → #f8f8f8 (panelInputBorder) |
| `kanagawa-wave` | `editor.inactiveSelection` #1F1F28 → #22222a (editorInactiveSelection)<br>`input.border` #16161D → #1b1b21 (inputBorder)<br>`panel.inputBorder` #16161D → #1b1b21 (panelInputBorder) |
| `monokai` | `editor.inactiveSelection` #272822 → #2a2b25 (editorInactiveSelection)<br>`terminal.selection` #272822 → #2c2d27 (terminalSelection)<br>`input.border` #414339 → #47493f (inputBorder)<br>`panel.inputBorder` #414339 → #47493f (panelInputBorder) |
| `night-owl-light` | `editor.inactiveSelection` #FBFBFB → #f8f8f8 (editorInactiveSelection)<br>`terminal.selection` #FBFBFB → #efeff0 (terminalSelection)<br>`explorer.itemSelected` #d3e8f8 → #cde1f2 (explorerSelectedVsHover)<br>`appSidebar.itemActive` #d3e8f8 → #cde1f2 (sidebarActiveVsHover) |
| `night-owl` | `appSidebar.itemHover` #011627 → #061b2c (sidebarItemHover)<br>`menu.itemHover` #011627 → #061b2c (menuItemHover) |
| `nord` | `terminal.selection` #2e3440 → #343a46 (terminalSelection)<br>`input.border` #3b4252 → #414857 (inputBorder)<br>`panel.inputBorder` #3b4252 → #414857 (panelInputBorder)<br>`input.focusBorder` #3b4252 → #414857 (inputFocusBorder)<br>`menu.itemHover` #3b4252 → #414857 (menuItemHover) |
| `one-dark-pro` | `editor.inactiveSelection` #282c34 → #2b2f37 (editorInactiveSelection)<br>`explorer.itemSelected` #2c313a → #313740 (explorerSelectedVsHover)<br>`appSidebar.itemActive` #2c313a → #313740 (sidebarActiveVsHover)<br>`input.border` #21252b → #262a30 (switchUncheckedTrackOnCard)<br>`input.border` #262a30 → #2d3139 (switchUncheckedTrackVsThumb) |
| `one-monokai` | `editor.inactiveSelection` #282c34 → #2b2f37 (editorInactiveSelection)<br>`terminal.selection` #282c34 → #2d3138 (terminalSelection)<br>`list.activeBackground` #2c313a → #2e3239 (listActiveVsHover)<br>`explorer.itemSelected` #2c313a → #2e3239 (explorerSelectedVsHover)<br>`appSidebar.itemActive` #2c313a → #353942 (sidebarItemActive) |
| `palenight` | `editor.bracketMatch` #282B3C → #2e3243 (editorBracketMatch)<br>`terminal.selection` #292D3E → #2e3243 (terminalSelection)<br>`button.primaryBackground` #7e57c2cc → #815bc3 (switchCheckedVsUncheckedTrack)<br>`app.focusBorder` #282B3C → #2e3243 (focusBorderOnApp) |
| `solarized-light` | `editor.inactiveSelection` #FDF6E3 → #f9f3e1 (editorInactiveSelection)<br>`terminal.selection` #FDF6E3 → #f5f0de (terminalSelection) |
| `tokyo-night` | `list.hoverBackground` #13131a → #1b1b24 (listHover)<br>`explorer.itemHover` #13131a → #1b1b24 (explorerItemHover)<br>`appSidebar.itemHover` #13131a → #1b1b24 (sidebarItemHover) |
| `visual-studio-cpp-dark` | `editor.selection` #1E1E1E → #232323 (editorSelection)<br>`editor.lineHighlight` #1E1E1E → #212121 (editorCurrentLine)<br>`terminal.selection` #1E1E1E → #232323 (terminalSelection)<br>`appSidebar.itemHover` #1E1E1E → #232323 (sidebarItemHover)<br>`appSidebar.itemActive` #1E1E1E → #232323 (sidebarItemActive)<br>`appSidebar.itemActive` #232323 → #282828 (sidebarActiveVsHover)<br>`button.primaryBackground` #1E1E1E → #232323 (switchCheckedTrackOnCard)<br>`button.hoverBackground` #1E1E1E → #232323 (buttonHover) |
| `visual-studio-cpp-light` | `editor.selection` #FFFFFF → #f8f8f8 (editorSelection)<br>`editor.lineHighlight` #FFFFFF → #fcfcfc (editorCurrentLine)<br>`terminal.selection` #FFFFFF → #f8f8f8 (terminalSelection)<br>`appSidebar.itemActive` #FFFFFF → #f8f8f8 (sidebarItemActive)<br>`button.primaryBackground` #FFFFFF → #f8f8f8 (switchCheckedTrackOnCard)<br>`button.hoverBackground` #FFFFFF → #f8f8f8 (buttonHover) |
| `vitesse-dark` | `explorer.itemSelected` #181818 → #1d1d1d (explorerSelectedVsHover)<br>`appSidebar.itemActive` #181818 → #1d1d1d (sidebarActiveVsHover)<br>`button.hoverBackground` #4d9375 → #56977a (buttonHover)<br>`input.border` #191919 → #1d1d1d (inputBorder)<br>`panel.inputBorder` #191919 → #1d1d1d (panelInputBorder)<br>`app.focusBorder` #00000000 → #171716 (focusBorderOnApp)<br>`input.focusBorder` #00000000 → #1d1d1d (inputFocusBorder) |
| `vitesse-light` | `explorer.itemSelected` #f7f7f7 → #f0f0f0 (explorerSelectedVsHover)<br>`appSidebar.itemActive` #f7f7f7 → #f0f0f0 (sidebarActiveVsHover)<br>`button.hoverBackground` #1c6b48 → #1e6747 (buttonHover)<br>`app.focusBorder` #00000000 → #f8f8f8 (focusBorderOnApp)<br>`input.focusBorder` #00000000 → #f0f0f0 (inputFocusBorder) |
| `vscode-abyss` | `editor.inactiveSelection` #000c18 → #010d1a (editorInactiveSelection)<br>`terminal.selection` #000c18 → #020e1c (terminalSelection) |
| `vscode-dark-modern` | `editor.selection` #1F1F1F → #242424 (editorSelection)<br>`editor.lineHighlight` #1F1F1F → #222222 (editorCurrentLine)<br>`appSidebar.itemActive` #1F1F1F → #242424 (sidebarActiveVsHover) |
| `vscode-dark-plus` | `editor.selection` #1E1E1E → #232323 (editorSelection)<br>`editor.lineHighlight` #1E1E1E → #212121 (editorCurrentLine)<br>`terminal.selection` #1E1E1E → #232323 (terminalSelection)<br>`appSidebar.itemHover` #1E1E1E → #232323 (sidebarItemHover)<br>`appSidebar.itemActive` #1E1E1E → #232323 (sidebarItemActive)<br>`appSidebar.itemActive` #232323 → #282828 (sidebarActiveVsHover)<br>`button.primaryBackground` #1E1E1E → #232323 (switchCheckedTrackOnCard)<br>`button.hoverBackground` #1E1E1E → #232323 (buttonHover) |
| `vscode-kimbie-dark` | `editor.inactiveSelection` #221a0f → #241c10 (editorInactiveSelection)<br>`terminal.selection` #221a0f → #271e12 (terminalSelection) |
| `vscode-light-modern` | `editor.selection` #FFFFFF → #f8f8f8 (editorSelection)<br>`editor.lineHighlight` #FFFFFF → #fcfcfc (editorCurrentLine)<br>`list.hoverBackground` #F2F2F2 → #f1f1f1 (listHover)<br>`explorer.itemHover` #F2F2F2 → #f1f1f1 (explorerItemHover)<br>`appSidebar.itemHover` #F2F2F2 → #f1f1f1 (sidebarItemHover) |
| `vscode-light-plus` | `editor.selection` #FFFFFF → #f8f8f8 (editorSelection)<br>`editor.lineHighlight` #FFFFFF → #fcfcfc (editorCurrentLine)<br>`terminal.selection` #FFFFFF → #f8f8f8 (terminalSelection)<br>`appSidebar.itemActive` #FFFFFF → #f8f8f8 (sidebarItemActive)<br>`button.primaryBackground` #FFFFFF → #f8f8f8 (switchCheckedTrackOnCard)<br>`button.hoverBackground` #FFFFFF → #f8f8f8 (buttonHover) |
| `vscode-monokai-dimmed` | `editor.inactiveSelection` #1e1e1e → #212121 (editorInactiveSelection)<br>`terminal.selection` #1e1e1e → #232323 (terminalSelection) |
| `vscode-quiet-light` | `editor.inactiveSelection` #F5F5F5 → #f2f2f2 (editorInactiveSelection)<br>`terminal.selection` #F5F5F5 → #eeeeee (terminalSelection) |
| `vscode-red` | `editor.inactiveSelection` #390000 → #3b0303 (editorInactiveSelection)<br>`terminal.selection` #390000 → #3d0505 (terminalSelection) |
| `vscode-solarized-dark` | `editor.inactiveSelection` #002B36 → #042e39 (editorInactiveSelection)<br>`terminal.selection` #002B36 → #06303a (terminalSelection) |
| `vscode-tomorrow-night-blue` | `editor.inactiveSelection` #002451 → #022652 (editorInactiveSelection)<br>`terminal.selection` #002451 → #062955 (terminalSelection)<br>`button.primaryBackground` #002451 → #062955 (switchCheckedTrackVsThumb)<br>`button.hoverBackground` #002451 → #062955 (buttonHover) |

**그림자 축 (d-61 검토 G-2).** `app.shadow` 는 비교할 컨테이너가 없다 — `global.css` 의
`@utility shadow-overlay`/`shadow-overlay-lg` 가 떠 있는 표면 전부의 드롭섀도로, `modal-scrim` 이
그 절반 세기로 모달 스크림으로 쓴다. 그래서 다른 축과 달리 **자기 알파**를 본다:
`APP_SHADOW_MIN_ALPHA = 0.149` 미만이면 위반이고, 수리는 **검정 하한값 `#00000026` 으로 교체**한다.

- 임계 근거: `0.149` 는 VS Code 자신의 라이트 테마 `widget.shadow` 기본값(`#00000026`, 38/255)이자
  이 변환기의 라이트 `SAFE_DEFAULT_COLORS.shadow` 다. 이 카탈로그에 맞춰 고른 값이 아니라 플랫폼이
  출하하는 가장 옅은 그림자를 바닥으로 삼았다. 47종 실측 분포: **0.000 이 4종**(`tokyo-night`
  `#ffffff00` · `tokyo-night-storm` `#ffffff00` · `tokyo-night-light` `#ffffff00` · `vitesse-dark`
  `#00000000`), 0.071 1종(`ayu-light`), 0.125 2종(`everforest-light`·`intellij-islands-light`),
  그 위로 0.149(5종)·0.188~0.502·1.000(5종) 40종. 하한은 앞의 7종만 올리고 나머지를 건드리지 않는다.
- 선언 RGB 를 유지하지 않는 이유: 알파가 0 인 토큰의 RGB 는 쓸 수 있는 정보가 아니다.
  `tokyo-night` 는 `#ffffff00` 을 출하하는데 그 알파만 올리면 다크 테마에 **흰 글로우와 흰 스크림**이
  깔린다(§1.C 가 `rgb(from …)` 방식을 기각한 것과 같은 이유). 그림자는 뒤에 있는 것을 어둡게 하는
  것이고, 이 변환기와 VS Code 의 shadow 기본값은 라이트·다크 양쪽 다 검정이다.

**검토 재측정 (d-61 검토 A-1·A-2, 47종 · 정정 전).** 형제 판정으로 바뀐 5축과 새로 추가한 14축의
분포다. 나머지 축은 위 38종 표에서 바뀌지 않았다.

| 축(label) | 상태 → 바탕 | 임계 | ΔE 0.00 | 0<ΔE<임계 | 임계 이상 최솟값 | 중앙값 | 정정 |
|---|---|---|---|---|---|---|---|
| `listActiveVsHover` | `list.activeBackground` → `list.hoverBackground` (형제) | 2.3 | 0 | 1 | 2.40 | 15.93 | 1 |
| `listHoverOnActiveTab` | `list.hoverBackground` → `tabBar.tabActiveBackground` | 2.3 | 3 | 1 | 2.34 | 6.18 | 4 |
| `listHoverOnInactiveTab` | `list.hoverBackground` → `tabBar.tabInactiveBackground` | 2.3 | 0 | 3 | 2.34 | 5.98 | 3 |
| `explorerSelectedVsHover` | `explorer.itemSelected` → `explorer.itemHover` (형제) | 2.3 | 3 | 1 | 2.35 | 8.74 | 4 |
| `explorerHoverOnPanel` | `explorer.itemHover` → `panel.background` | 2.3 | 0 | 0 | 2.38 | 6.41 | 0 |
| `explorerSelectedOnPanel` | `explorer.itemSelected` → `panel.background` | 2.3 | 0 | 0 | 2.53 | 12.65 | 0 |
| `explorerFocusedOnPanel` | `explorer.itemFocused` → `panel.background` | 2.3 | 0 | 0 | 2.77 | 10.30 | 0 |
| `explorerHoverOnStatusBar` | `explorer.itemHover` → `appSidebar.background` | 2.3 | 0 | 0 | 2.32 | 6.31 | 0 |
| `explorerSelectedOnStatusBar` | `explorer.itemSelected` → `appSidebar.background` | 2.3 | 0 | 1 | 2.53 | 12.65 | 1 |
| `explorerHoverOnEditorWidget` | `explorer.itemHover` → `editor.widgetBackground` | 2.3 | 0 | 1 | 2.43 | 6.01 | 1 |
| `sidebarActiveVsHover` | `appSidebar.itemActive` → `appSidebar.itemHover` (형제) | 2.3 | 3 | 1 | 2.35 | 4.77 | 4 |
| `sidebarHoverOnEditor` | `appSidebar.itemHover` → `editor.background` | 2.3 | 1 | 3 | 2.34 | 4.85 | 4 |
| `sidebarHoverOnApp` | `appSidebar.itemHover` → `app.background` | 2.3 | 1 | 3 | 2.34 | 4.85 | 4 |
| `sidebarActiveOnApp` | `appSidebar.itemActive` → `app.background` | 2.3 | 0 | 0 | 2.42 | 8.07 | 0 |
| `sidebarHoverOnTabBar` | `appSidebar.itemHover` → `tabBar.background` | 2.3 | 0 | 1 | 2.34 | 4.33 | 1 |
| `tabActiveVsInactive` | `tabBar.tabActiveBackground` → `tabBar.tabInactiveBackground` (형제) | 2.3 | 11 | 4 | 2.42 | 3.51 | 0 (15 전부 인디케이터 대안 통과) |
| `switchCheckedVsUncheckedTrack` | `button.primaryBackground` → `input.border` (형제) | 2.3 | 1 | 0 | 2.31 | 44.80 | 1 |
| `modalItemHover` | `modal.itemHover` → `modal.background` | 2.3 | 5 | 3 | 2.45 | 4.91 | 8 |

> `sidebarHoverOnEditor` 와 `sidebarHoverOnApp` 의 수치가 같은 것은 우연이 아니다 — 변환기가
> `app.background` 를 `editor.background` 에서 파생하므로(`mapping-tables.ts`) 번들 테마에서는 두 토큰이
> 항상 같은 값이다. 소비처가 서로 다른 별개의 토큰이라 행은 둘 다 둔다(손수정·사용자 저장 테마는
> 둘을 다르게 둘 수 있다).

**검토 데이터 정정 결과 (d-61 검토).** 위 변경으로 `bun run themes:repair-state-distinctness` +
`bun run themes:repair-contrast` 를 다시 돌려 **47종 중 22종, 46개 토큰**이 추가로 정정됐다(둘 다
재실행 0건). 아래 표의 "전" 값은 d-61 §1.A·§1.B 반영본이다. `→ a → b` 로 두 번 적힌 것은 한 토큰이
서로 다른 축에 연달아 걸려 두 패스에 걸쳐 이동한 경우다.

| 테마 | 정정 토큰 (이전 → 이후, 축) |
|---|---|
| `ayu-dark` | `explorer.itemSelected` #47526640 → #202630 (explorerSelectedVsHover)<br>`appSidebar.itemActive` #47526640 → #202630 (sidebarActiveVsHover) |
| `ayu-light` | `explorer.itemSelected` #6b7d8f24 → #dde0e5 (explorerSelectedVsHover)<br>`appSidebar.itemActive` #6b7d8f24 → #dde0e5 (sidebarActiveVsHover)<br>`app.shadow` #6b7d8f12 → #00000026 (appShadow) |
| `ayu-mirage` | `explorer.itemSelected` #63759926 → #2e3646 (explorerSelectedVsHover)<br>`appSidebar.itemActive` #63759926 → #2e3646 (sidebarActiveVsHover) |
| `darcula` | `list.hoverBackground` #2A2D2E → #323232 (listHoverOnInactiveTab)<br>`explorer.itemHover` #2A2D2E → #363636 → #404040 (explorerHoverOnEditorWidget → explorerHoverOnStatusBar)<br>`modal.itemHover` #242424 → #292929 (modalItemHover) |
| `everforest-dark` | `modal.itemHover` #2d353b00 → #323a3f (modalItemHover) |
| `everforest-light` | `modal.itemHover` #fdf6e300 → #f5efde (modalItemHover)<br>`app.shadow` #3c474d20 → #00000026 (appShadow) |
| `gruvbox-dark` | `list.hoverBackground` #3c383680 → #423e3a (listHoverOnActiveTab) |
| `gruvbox-light` | `list.hoverBackground` #ebdbb280 → #e3d4ad (listHoverOnActiveTab) |
| `intellij-islands-light` | `app.shadow` #00000020 → #00000026 (appShadow) |
| `nord` | `list.hoverBackground` #3b4252 → #414857 (listHoverOnActiveTab) |
| `one-monokai` | `explorer.itemSelected` #2e3239 → #353942 (explorerSelectedOnStatusBar)<br>`appSidebar.itemHover` #292d35 → #2d3138 → #353942 (sidebarHoverOnEditor → sidebarItemHover)<br>`appSidebar.itemActive` #353942 → #3b3e47 (sidebarActiveVsHover) |
| `rose-pine-dawn` | `list.activeBackground` #6e6a8614 → #ece6e3 (listActiveVsHover)<br>`explorer.itemSelected` #6e6a8614 → #ece6e3 (explorerSelectedVsHover)<br>`appSidebar.itemActive` #6e6a8614 → #ece6e3 (sidebarActiveVsHover) |
| `tokyo-night` | `appSidebar.itemHover` #1b1b24 → #1e202b (sidebarHoverOnEditor)<br>`appSidebar.itemActive` #202330 → #232531 (sidebarActiveVsHover)<br>`modal.itemHover` #13131a → #1b1b24 (modalItemHover)<br>`app.shadow` #ffffff00 → #00000026 (appShadow) |
| `tokyo-night-light` | `list.hoverBackground` #e1e2e8 → #d3d5dd → #cfd1d9 (listHoverOnActiveTab → listHoverOnInactiveTab)<br>`list.activeBackground` #cfd1d9 → #c8cad2 (listActiveVsHover)<br>`appSidebar.itemHover` #e1e2e8 → #dfe0e7 (sidebarHoverOnEditor)<br>`app.shadow` #ffffff00 → #00000026 (appShadow) |
| `tokyo-night-storm` | `app.shadow` #ffffff00 → #00000026 (appShadow) |
| `visual-studio-cpp-dark` | `modal.itemHover` #1E1E1E → #232323 (modalItemHover) |
| `vitesse-dark` | `app.shadow` #00000000 → #00000026 (appShadow) |
| `vscode-dark-modern` | `appSidebar.itemHover` #1F1F1F → #242424 (sidebarHoverOnEditor)<br>`appSidebar.itemActive` #242424 → #292929 (sidebarActiveVsHover)<br>`modal.itemHover` #1F1F1F → #252525 (modalItemHover)<br>`menu.itemHover` #0078d4 → #2A2D2E (§8.6 menuItemHoverText) |
| `vscode-dark-plus` | `modal.itemHover` #1E1E1E → #232323 (modalItemHover) |
| `vscode-light-modern` | `modal.itemHover` #F2F2F2 → #f1f1f1 (modalItemHover)<br>`menu.itemHover` #005FB8 → #f1f1f1 (§8.6 menuItemHoverText) |
| `vscode-monokai-dimmed` | `list.hoverBackground` #444444 → #464646 (listHoverOnInactiveTab) |
| `vscode-solarized-dark` | `list.hoverBackground` #004454AA → #094657 (listHoverOnInactiveTab)<br>`appSidebar.itemHover` #004454AA → #094657 (sidebarHoverOnTabBar)<br>`button.primaryBackground` #2AA19899 → #267371 (switchCheckedVsUncheckedTrack)<br>`button.primaryForeground` #afbabb → #b7c0c1 (§8.6 buttonPrimary) |

**일부러 넣지 않은 축.** 표는 "실사용 전수"지만, 아래는 위 ①②를 만족하지 않아 의도적으로 제외했다.
재검토가 필요하면 근거부터 뒤집어야 한다.

| 제외 축 | 사유 |
|---|---|
| `explorer.itemSelected` vs `explorer.itemFocused` | 한 행의 상호배타적 렌더링(트리 포커스 유무)이라 동시에 보이지 않는다. 트리 포커스는 `app.focusBorder`/`list.focusOutline` 로도 표시된다 |
| `scrollbar.thumbHover` vs `scrollbar.thumb` | 썸은 하나뿐이라 자기 자신과 비교되지 않는다. 38종 중 33종이 동일값이고, 업스트림이 슬라이더 색을 하나만 정의하는 관행이라 hover 음영을 창작하는 셈이 된다 |
| `tabBar.tabInactiveBackground` vs `tabBar.background` | 비활성 탭이 스트립에 녹아드는 것은 플랫 디자인 자체이고, 활성/비활성 구별은 `tabActiveVsInactive` 가 이미 본다 |
| `app.border` vs `app.background`/`panel.background` | 테두리 없는(borderless) 디자인이 실재하는 미학 선택이다. 별도 판단이 필요해 후속으로 남긴다 |
| `editor.hoverBackground`·`editor.widgetBackground` vs `editor.background` | 상태가 아니라 표면이고 각자 `editor.widgetBorder` 로 경계를 가진다 |
| `popover.itemHover`·`tooltip.itemHover` | 소비처가 없다(§3.1 예외 등재분) — 팝오버는 항목 목록이 없고 툴팁은 한 줄 텍스트뿐이다. 배선되면 각각 한 행씩 추가한다. 함께 묶여 있던 `modal.itemHover` 는 §1.C 가 다이얼로그 닫기 버튼에 배선해 **`modalItemHover` 행으로 승격**됐다 |
| `list.hoverBackground` vs `explorer.itemHover`·`appSidebar.itemHover` | 같은 hover 개념의 서로 다른 표면용 토큰이라, 한 화면에서 나란히 비교되는 두 상태가 아니다. 각자 자기 컨테이너 축에서 이미 검사된다 |

**재실행.** 쌍 표나 임계를 바꿨거나 테마를 추가했으면
`bun run themes:repair-state-distinctness` 를 돌려 JSON 을 갱신하고
`bun test src/shared/lib/theme-convert` 로 게이트를 확인한다. Rust 카탈로그 린트는 같은 표·임계·합성
규칙을 복제하며, 라벨/개수 비교 테스트로 TS 정본과의 드리프트를 막는다(§8.3).

### 8.6 컴포넌트 전경/배경 대비 전수 린트 (d-61 §1.B)

§8.2 의 대비 가드는 7쌍(앱·에디터·패널 헤더·툴팁·매치 강조 5 blocking + 선택 행 2 advisory)만 본다.
그 7쌍이 통과해도 **실제 화면의 글자 대부분은 검사되지 않는다** — 상태바 문구, 설정 카드의 설명글,
파일트리의 git 데코, 문제 패널의 심각도 색, 버튼 라벨, 탭 제목, 찾기 입력의 placeholder 는 어느
쌍에도 걸리지 않았다. 이 절의 린트는 그 공백을 컴포넌트 실사용 기준으로 메운다. 임계는 기존과 같은
`MIN_CONTRAST_RATIO = 3` 이다.

**정본 파일**

| 파일 | 역할 |
|---|---|
| `src/shared/lib/theme-convert/component-contrast-pairs.ts` | 쌍 표(`COMPONENT_CONTRAST_PAIRS`) 36행 + 자문 표(`FIXED_FOREGROUND_CONTRAST_PAIRS`) + 예외 레지스트리(`COMPONENT_CONTRAST_EXEMPTIONS`). Rust 미러가 그대로 복제하는 원본 |
| `src/shared/lib/theme-convert/contrast.ts` | `validateComponentContrast` / `repairComponentContrast` / `validateFixedForegroundContrast` / `validateTerminalAnsiContrast` / `isExemptComponentContrastViolation`, `repairContrastPairs` 통합 |
| `src/shared/lib/theme-convert/bundled-theme-contrast.test.ts` | 번들 전수 게이트(예외 등재 2 — 아래) + 쌍 표 불변식 |
| `scripts/repair-theme-contrast.ts` | 번들 JSON 정정(`bun run themes:repair-contrast`, 멱등) |

**기존 `CONTRAST_PAIRS` 를 늘리지 않고 표를 나눈 이유.** 그 7쌍에는 깨면 안 되는 약속이 둘 붙어
있다 — blocking 5쌍은 VSIX 임포트 거부 여부를 결정하고(d-40 §1-a "기존 5쌍 판정·수리 불변"),
advisory 2쌍은 예외 등재 테마별 **위반 개수까지** 테스트가 고정한다. 같은 배열에 행을 더하면 두
약속이 모두 바뀐다. 새 표는 임포트를 거부할 수 없는 자문 축으로만 동작한다.

**도출·판정.** 쌍은 §8.5 와 같은 방식으로 **코드에서** 뽑는다(`global.css` 의 `@theme inline` 과
shadcn 브리지 변수 → 컴포넌트 클래스, `monaco/theme.ts` → 임베디드 에디터, `xterm-theme.ts` → 터미널).
각 행은 배경이 놓이는 **표면(`surfaceKey`)을 필수로** 선언하고, 판정 전에 배경을 그 표면 위에,
표면을 다시 `app.background` 위에 합성한다. 선택이 아니라 필수인 이유는 이 카탈로그에서 반투명
배경이 예외가 아니기 때문이다 — `button.background` 9종, `list.hoverBackground` 20종이 `#rrggbbaa`
이고, rose-pine 계열은 `tabBar.background`·`tabBar.tabInactiveBackground` 를 `#00000000` 으로 둔다.
raw RGB 로 읽으면 투명한 탭 스트립이 **검정**으로 측정돼 없는 결함이 잡힌다.

**수리 규칙 — 전경을 자기 색상(hue)대로 최소한만 옮긴다.**

1. 수리 단위는 쌍이 아니라 **전경 토큰**이다. 한 토큰이 여러 표면에 그려지면(`statusIndicator.error`
   는 상태바와 문제 패널, `appSidebar.iconDefault` 는 상태바·설정 카드·트리) 그 표면 전부를 동시에
   만족하는 값 하나로 한 번만 옮긴다. 쌍 단위로 고치면 같은 토큰을 서로 반대로 밀게 된다.
2. 후보 토큰 교체(blocking 쌍이 쓰는 방식)를 쓰지 않는다. 다른 토큰 값을 가져오면 색상이 통째로
   바뀌어 테마 정체성을 잃는다. 대신 현재 색에서 검정·흰색 양 끝을 향해 각각 한 스텝씩(256단계)
   걸어 **모든 표면을 만족하는 첫 지점**을 찾고, 두 방향 중 ΔE 가 작은 쪽을 고른다. 검정/흰색 혼합은
   세 채널을 같은 비율로 움직이므로 색상은 유지되고 명도만 바뀐다. 이분 탐색이 아니라 순차 탐색인
   것은, 표면 명도가 두 끝 사이에 있으면 광선 위 대비가 단조롭지 않아 이분 탐색이 최근접 해를 지나칠
   수 있기 때문이다.
3. 공유 축은 **제약으로만** 쓴다. `list.foreground` 는 이 표의 두 행과 d-40 `selectionForeground`
   의 전경을 겸한다. 치환값은 `list.activeBackground` 도 만족해야 하지만, 그 축의 미달이 수리를
   **촉발하지는 않는다** — 그 축의 예외 등재(everforest-light·rose-pine-dawn)가 어떤 테마가 왜 남아
   있는지를 고정하고 있기 때문이다.
4. 어느 방향으로도 모든 표면을 만족하지 못하면(두 표면이 서로 반대쪽 명도) **수리하지 않고 남긴다.**
   한쪽을 고치려고 다른 쪽을 깨지 않는다.
5. 결과값은 항상 불투명이다. 측정한 값이 표면 합성 결과이므로, 알파를 남기면 표면마다 다르게 읽히는
   색을 하나의 수리값에 다시 집어넣는 셈이 된다.
6. **예외적으로 배경을 옮기는 축이 하나 있다 (d-61 검토 G-3).** `menuItemHoverText`
   (`app.foreground` on `menu.itemHover`)는 전경이 전역 본문색이라 1~5 를 그대로 적용하면 메뉴 한
   행 때문에 앱 전체 글자색이 움직인다. 그래서 `COMPONENT_CONTRAST_BACKGROUND_SUBSTITUTES` 에 따라
   **`menu.itemHover` 를 `list.hoverBackground` 로 치환**한다 — 그 테마 자신의 행 hover 틴트이고,
   §8.2 매핑 체인도 이제 `menu.itemHover` 를 거기서 먼저 파생한다. 치환 후보는 ① 3:1 을 실제로
   넘고 ② `menu.background` 와 ΔE 2.3(§8.5 `menuItemHover` 축) 이상 떨어져 있어야만 채택한다 —
   못 넘으면 **원래 값을 그대로 둔다**(이 린트의 실패를 §8.5 의 실패로 바꾸지 않고, 더 나쁜 값으로
   바꾸지도 않는다). 배경 치환 패스는 전경 패스보다 **먼저** 돈다.

수리는 배경 치환 1패스 + 전경 1패스로 완결된다(전경 토큰이 어떤 쌍의 배경·표면도 아니라는 표
불변식 테스트가 고정). `convert.ts` 는 `repairContrastPairs` 안에서 blocking → 선택 행 → 컴포넌트
순으로 돌린다.

**정정 결과 — 번들 47종 중 37종 167토큰.** 아래 표의 "전" 값은 §8.5 정정까지 반영된 커밋 상태다.

| 테마 | 토큰 | 전 → 후 |
|---|---:|---|
| `ayu-dark` | 2 | appSidebar.iconDefault #5a637899→#5a606c · input.placeholder #5a637880→#5d626d |
| `ayu-light` | 14 | appSidebar.iconDefault #828e9f99→#8b9199 · appSidebar.badge #f29718→#cc7f14 · explorer.gitAdded #6cbf43→#5ba138 · explorer.gitDeleted #ff7383→#e66876 · explorer.gitUntracked #6cbf43→#5ba138 · terminal.linkForeground #f29718→#cd8014 · git.added #6cbf43→#5ba138 · git.deleted #ff7383→#e66876 · git.renamed #21a1e2→#1f99d7 · git.untracked #6cbf43→#5ba138 · statusIndicator.info #21a1e2→#1f99d7 · statusIndicator.warning #f29718→#cc7f14 · statusIndicator.success #6cbf43→#5ba138 · input.placeholder #828e9f80→#8e9399 |
| `darcula` | 1 | statusIndicator.error #cd3131→#d75c5c |
| `dracula` | 1 | appSidebar.iconDefault #6272A4→#7180ad |
| `everforest-dark` | 2 | explorer.gitDeleted #e67e80a0→#a76d70 · git.deleted #e67e80a0→#a76d70 |
| `everforest-light` | 21 | appSidebar.iconDefault #939f91→#879285 · appSidebar.badge #93b259→#7e984c · tabBar.tabInactiveForeground #a4ad9e→#8a9185 · tabBar.previewForeground #879686→#849383 · explorer.gitModified #3a94c5a0→#6a95a8 · explorer.gitAdded #8da101a0→#8d9441 · explorer.gitDeleted #f85552a0→#cd776f · explorer.gitUntracked #dfa000a0→#aa8b3e · terminal.linkForeground #8da101→#859801 · git.added #8da101a0→#8d9441 · git.modified #3a94c5a0→#6a95a8 · git.deleted #f85552a0→#cd776f · git.untracked #dfa000a0→#aa8b3e · git.conflicted #df69baa0→#b87d9e · git.staged #35a77ca0→#659a80 · statusIndicator.info #6cb3c6→#5b97a7 · statusIndicator.warning #e4b649→#ad8a38 · statusIndicator.error #f1706f→#e36968 · statusIndicator.success #8da101→#859801 · input.placeholder #a4ad9e→#8a9185 · button.primaryForeground #fdf6e3→#595750 |
| `github-dark` | 1 | appSidebar.badge #0366d6→#096ad7 |
| `github-light` | 8 | appSidebar.iconDefault #959da5→#899198 · explorer.gitAdded #28a745→#28a544 · explorer.gitUntracked #28a745→#28a544 · git.added #28a745→#28a544 · git.untracked #28a745→#28a544 · statusIndicator.warning #f9c513→#b08b0d · statusIndicator.success #28a745→#28a544 · input.placeholder #959da5→#8b939a |
| `gruvbox-dark` | 4 | explorer.gitDeleted #cc241d→#d13831 · git.deleted #cc241d→#d13831 · statusIndicator.error #cc241d→#d13831 · input.placeholder #ebdbb260→#7a7465 |
| `intellij-islands-light` | 4 | appSidebar.iconDefault #AEB3C2→#9094a1 · statusIndicator.info #88ADF7→#7494d3 · statusIndicator.warning #F2BF57→#b58f41 · input.placeholder #AEB3C2→#9094a1 |
| `monokai` | 3 | explorer.gitDeleted #C4265E→#c4285f · git.deleted #C4265E→#c4285f · statusIndicator.error #C4265E→#c9376b |
| `night-owl-light` | 7 | explorer.gitModified #E0AF02→#aa8502 · git.modified #E0AF02→#aa8502 · git.staged #E0AF02→#aa8502 · statusIndicator.warning #daaa01→#ab8501 · input.placeholder #93A1A1→#818d8d · button.foreground #F0F0F0→#464646 · button.primaryForeground #F0F0F0→#fbfbfb |
| `night-owl` | 3 | appSidebar.badge #44596b→#516575 · explorer.gitDeleted #EF535090→#944d52 · git.deleted #EF535090→#944d52 |
| `nord` | 1 | tabBar.tabInactiveForeground #d8dee966→#777d88 |
| `one-dark-pro` | 1 | statusIndicator.error #c24038→#c64c44 |
| `one-monokai` | 3 | statusIndicator.error #c24038→#ca5851 · button.foreground #D4D4D4→#f7f7f7 · button.primaryForeground #D4D4D4→#f7f7f7 |
| `palenight` | 3 | appSidebar.badge #7e57c2→#8763c6 · explorer.gitDeleted #EF535090→#aa6267 · git.deleted #EF535090→#aa6267 |
| `rose-pine-dawn` | 7 | appSidebar.badge #d7827e→#c77875 · explorer.gitModified #d7827e→#c77875 · explorer.gitUntracked #ea9d34→#bf802a · git.modified #d7827e→#c77875 · git.untracked #ea9d34→#bf802a · statusIndicator.warning #ea9d34→#bf802a · button.primaryForeground #faf4ed→#4d4c49 |
| `solarized-light` | 13 | appSidebar.badge #B58900→#a87f00 · explorer.gitModified #b58900→#a87f00 · explorer.gitAdded #859900→#7b8e00 · explorer.gitUntracked #859900→#7b8e00 · git.added #859900→#7b8e00 · git.modified #b58900→#a87f00 · git.untracked #859900→#7b8e00 · git.staged #b58900→#a87f00 · statusIndicator.warning #b58900→#987300 · statusIndicator.success #859900→#708000 · input.placeholder #586E75AA→#707b79 · button.foreground #657B83→#435156 · button.primaryForeground #657B83→#435156 |
| `tokyo-night` | 5 | appSidebar.iconDefault #3b3e52→#616373 · appSidebar.badge #3d59a1→#4661a5 · explorer.gitDeleted #914c54→#935057 · git.deleted #914c54→#935057 · input.placeholder #787c998A→#5f6172 |
| `vitesse-dark` | 1 | appSidebar.iconDefault #dedcd550→#62615f |
| `vitesse-light` | 1 | appSidebar.iconDefault #393a3450→#949493 |
| `vscode-abyss` | 1 | terminal.linkForeground #0063a5→#2077b0 |
| `vscode-kimbie-dark` | 6 | appSidebar.badge #7f5d38→#8c6d4c · explorer.gitDeleted #cd3131→#d03c3c · git.deleted #cd3131→#d03c3c · git.renamed #2472c8→#2673c8 · statusIndicator.info #2472c8→#2673c8 · statusIndicator.error #cd3131→#d03c3c |
| `vscode-light-modern` | 4 | explorer.gitModified #949800→#929600 · git.modified #949800→#929600 · git.staged #949800→#929600 · statusIndicator.warning #949800→#929600 |
| `vscode-monokai-dimmed` | 4 | appSidebar.badge #3655b5→#516cbf · explorer.gitDeleted #C4265E→#c9376b · git.deleted #C4265E→#c9376b · statusIndicator.error #C4265E→#d0517e |
| `vscode-quiet-light` | 7 | explorer.gitModified #949800→#8e9100 · git.modified #949800→#8e9100 · git.staged #949800→#8e9100 · statusIndicator.warning #949800→#8b8e00 · statusIndicator.error #f1897f→#c77169 · button.foreground #1E1E1E→#151515 · button.primaryForeground #1E1E1E→#151515 |
| `vscode-solarized-dark` | 3 | input.placeholder #93A1A1AA→#658185 · button.foreground #839496→#afbabb · button.primaryForeground #839496→#afbabb |
| `vscode-tomorrow-night-blue` | 1 | statusIndicator.error #a92049→#b43c60 |

§1.D 로 같은 배치에 추가된 9종은 변환 직후 값에서 정정됐고 그 "전" 값은 커밋된 적이 없다. 정정된
토큰과 결과값만 남긴다(rose-pine-moon 은 정정 0).

| 테마 | 토큰 | 정정 후 |
|---|---:|---|
| `ayu-mirage` | 2 | appSidebar.iconDefault #676e7b · input.placeholder #6c737f |
| `catppuccin-frappe` | 3 | input.placeholder #8a91a9 · button.foreground #ced7f6 · tabBar.previewForeground #777d97 |
| `catppuccin-latte` | 13 | explorer.gitAdded #3d9829 · explorer.gitModified #ba7718 · explorer.gitUntracked #3d9829 · git.added #3d9829 · git.modified #ba7718 · git.untracked #3d9829 · git.staged #ba7718 · appSidebar.iconDefault #7c7f8c · statusIndicator.warning #d95509 · statusIndicator.success #3a9127 · input.placeholder #717482 · tabBar.tabInactiveForeground #828592 · tabBar.previewForeground #878b99 |
| `catppuccin-macchiato` | 1 | input.placeholder #7e849e |
| `github-dark-dimmed` | 1 | input.placeholder #66717e |
| `gruvbox-light` | 8 | explorer.gitModified #b5811c · explorer.gitUntracked #908f19 · git.modified #b5811c · git.untracked #908f19 · git.staged #b5811c · statusIndicator.warning #b5811c · statusIndicator.success #908f19 · input.placeholder #918b75 |
| `tokyo-night-light` | 2 | appSidebar.iconDefault #777988 · statusIndicator.info #0b869c |
| `tokyo-night-storm` | 5 | explorer.gitDeleted #9a5b63 · git.deleted #9a5b63 · appSidebar.iconDefault #666c87 · appSidebar.badge #536cac · input.placeholder #616884 |

**제외 축.** 아래는 실제 렌더 쌍이지만 표에 넣지 않았다. 괄호 안은 현재 커밋된 47종 기준 실측
(미달 테마 수 / 최소 비율).

| 제외 축 | 실측 | 사유 |
|---|---|---|
| `explorer.gitIgnored` vs `explorer.background` | 27/47, 1.12 | 흐리게 보이는 것이 이 토큰의 **목적**이다(무시된 파일). 3:1 게이트는 의도와 정면으로 충돌하고, "의도적으로 흐림"에 원칙 있는 하한선이 없다 |
| `editor.lineNumber` vs `editor.background` | 25/47, 1.48 | 같은 이유. VS Code 기본 줄번호도 3:1 미만이고, 활성 줄은 `editor.lineNumberActive` 가 따로 있다 |
| `editorBlame.foreground` vs `editor.background` | 12/47, 1.58 | 인라인 blame 오버레이는 본문을 가리지 않도록 흐린 것이 설계다(`global.css` 가 `opacity: 0.8` 을 더한다) |
| `button.primaryForeground` vs `statusIndicator.error` (destructive 버튼) | 19/47, 1.09 | **더 이상 렌더 쌍이 아니다** — d-61 검토 G-1 이 이 버튼의 라벨을 `text-white` 로 되돌렸다(§3.2). 아래 자문 표가 대신 계측한다. 애초에 제외했던 이유도 유효하다: **토큰 하나가 서로 다른 색 배경 둘을 동시에 섬긴다.** 기본 버튼 배경과 파괴 버튼 배경을 함께 만족하는 값이 4종(dracula·vscode-abyss·vscode-red·vscode-tomorrow-night-blue)에는 아예 없고, 나머지에서도 파괴 축 때문에 **기본 버튼 라벨**이 최대 ΔE 93.7(github-dark #dcffe4→#0f110f)까지 끌려간다. 올바른 해법은 표면별 전경 토큰인데 §1.F 가 신규 토큰을 금지한다 → 후속 |
| `button.primaryForeground` vs `appSidebar.badge` (Monaco 배지) | 14/47, 1.79 | 같은 공유 토큰 문제. 배지·기본 버튼·파괴 버튼 셋을 함께 만족하는 값이 없는 테마가 4종으로 늘어난다 |
| `app.foreground`/`editor.foreground` vs 상태 오버레이 (`explorer.itemSelected` 5/47 · `explorer.itemHover` 2/47 · `diff.insertedLineBackground` 12/47 · `diff.removedLineBackground` 8/47 · `editor.findMatch` 12/47) | 최소 1.01 | 전경이 **전역 본문색**이라 이 절의 수리 규칙(전경 이동)으로는 앱 전체 글자색을 옮기는 수밖에 없다. 실제 원인은 배경 쪽 상태 토큰(§8.5 소관)이므로 여기서 게이트하면 잘못된 토큰을 고치게 된다. §1.E 스크린샷 매트릭스의 판정 대상으로 남긴다 |
| `panel.matchHighlight` vs `list.hoverBackground` | 3/47, 2.45 | `panel.matchHighlight` 는 이미 blocking·advisory 두 축에 묶여 있고 수리 경로가 그 둘로 포화 상태다. 세 번째 축은 새 예외 등재만 늘린다 |

`app.foreground` 가 표면(상태 오버레이가 아닌) 위에 놓이는 축 — `menu.background`(최소 3.09) ·
`panel.inputBackground`(최소 3.07) — 은 47종 전부 통과한다. 수리 불가라 게이트에 올리지 않았지만
현재 여유가 0.1 도 안 되므로, 새 테마를 추가할 때 함께 확인한다.

> `menu.itemHover` 축(`menuItemHoverText`)만은 이 그룹에서 **표로 승격**됐다(d-61 검토 G-3). 전경은
> 같은 전역 본문색이지만 **배경 쪽에 수리 수단이 있기 때문**이다(수리 규칙 6). 승격 계기는 §1.C 가
> 메뉴 hover 를 `menu.selectionBackground` 로 배선하면서 글자색은 그대로 둔 것이었다 —
> `vscode-light-modern` 이 1.78:1(`#005FB8` 위 `#3b3b3b`), `vscode-dark-modern` 이 2.82:1 이 됐고,
> 둘 다 `list.hoverBackground` 치환으로 9.92 / 8.64 가 됐다.

**예외 등재 2 (`COMPONENT_CONTRAST_EXEMPTIONS`).** 수리로도 3:1 을 못 넘는 테마는 사유·실측과 함께
등재한다. 게이트·수리 스크립트·Rust 린트가 **같은 레지스트리 하나**를 읽고, 게이트는 등재분이 실제로
그 축에서만 위반하는지도 함께 확인한다(등재가 낡으면 실패).

| 테마:축 | 실측 | 사유 |
|---|---|---|
| `ayu-dark:menuItemHoverText` | 2.72 (치환 후보 2.62) | `app.foreground`(`#5a6378`)가 **틴트 없는** `menu.background`(`#0f131a`) 위에서 이미 3.09:1 이다. hover 로 읽히는 틴트(ΔE 2.3 이상)는 그 여유를 반드시 소모한다. `list.hoverBackground` 치환은 2.62 로 더 나빠 적용하지 않는다 |
| `ayu-light:menuItemHoverText` | 2.88 (치환 후보 2.83) | 같은 구조. `app.foreground`(`#828e9f`) vs 흰 `menu.background` 가 3.32:1 이고 **흰색보다 밝은 배경은 없으므로** 어떤 hover 틴트도 대비를 낮춘다 |

두 테마 모두 근본 원인은 ayu 계열의 dim 한 UI 전경이다. 고치려면 `app.foreground` 를 옮겨야 하는데,
그것이 정확히 이 표가 절대 건드리지 않는 토큰이다(위 제외 축 표 참고).

**고정 전경 자문 린트 — `FIXED_FOREGROUND_CONTRAST_PAIRS`(게이트·수리 없음).** 전경이 컴포넌트 안의
리터럴이라 테마가 바꿀 수 없는 축이다. `validateFixedForegroundContrast(theme.colors)` 가 보고만
한다. 현재 1행 — `shared/ui/button.tsx` 의 파괴적 버튼 라벨 `text-white` 위
`statusIndicator.error`(§3.2) — 이고, 47종 중 **10종이 3:1 미만**이다(토큰이었을 때의 19종보다 적다).

| 테마 | 비율 | `statusIndicator.error` |
|---|---:|---|
| `vscode-red` · `vscode-solarized-dark` | 1.15 | `#ffeaea` |
| `vscode-abyss` | 1.98 | `#ff9da4` |
| `catppuccin-mocha` | 2.32 | `#f38ba8` |
| `catppuccin-macchiato` | 2.47 | `#ed8796` |
| `catppuccin-frappe` | 2.65 | `#e78284` |
| `github-dark` | 2.66 | `#f97583` |
| `ayu-mirage` | 2.86 | `#ff6666` |
| `rose-pine` · `rose-pine-moon` | 2.91 | `#eb6f92` |

수리하지 않는 이유는 `statusIndicator.error` 가 **전경 토큰**이기 때문이다 — 상태바·문제 패널에서
글자로 쓰이며 이미 이 표의 `statusBarError`/`problemError` 축이 그 용도로 값을 고정한다. 파괴 버튼의
배경으로 쓰인다는 이유로 같은 토큰을 어둡게 옮기면 글자 용도가 깨진다. 올바른 해법은 전용
`button.destructiveBackground` 토큰이고, §1.F 가 신규 토큰을 금지하므로 후속으로 남긴다.

**검토 재정정 3토큰 (d-61 검토).** 위 변경과 §8.5 재정정으로 이 린트가 추가로 옮긴 토큰이다.

| 테마 | 토큰 | 전 → 후 | 축 |
|---|---|---|---|
| `vscode-light-modern` | `menu.itemHover` | #005FB8 → #f1f1f1 | menuItemHoverText (1.78 → 9.92) |
| `vscode-dark-modern` | `menu.itemHover` | #0078d4 → #2A2D2E | menuItemHoverText (2.82 → 8.64) |
| `vscode-solarized-dark` | `button.primaryForeground` | #afbabb → #b7c0c1 | buttonPrimary — §8.5 가 `button.primaryBackground` 를 `#2AA19899`→`#267371` 로 옮긴 데 따른 연쇄 |

**터미널 ANSI 16색 — 자문 전용(게이트·수리 없음).** `validateTerminalAnsiContrast(theme.terminal)`
이 보고만 한다. 47종 실측(미달 수 / 최소 / 중앙값):

| ANSI | 미달 | 최소 | 중앙 |
|---|---:|---:|---:|
| `black` | 34/47 | 1.00 | 1.35 |
| `red` | 3/47 | 2.68 | 4.55 |
| `green` | 6/47 | 2.17 | 6.13 |
| `yellow` | 11/47 | 1.89 | 7.62 |
| `blue` | 1/47 | 2.74 | 5.34 |
| `magenta` | 2/47 | 2.34 | 5.09 |
| `cyan` | 6/47 | 2.23 | 5.96 |
| `white` | 7/47 | 1.00 | 8.89 |
| `brightBlack` | 21/47 | 1.00 | 3.12 |
| `brightRed` | 1/47 | 2.73 | 4.85 |
| `brightGreen` | 9/47 | 1.95 | 7.14 |
| `brightYellow` | 11/47 | 1.70 | 8.69 |
| `brightBlue` | 3/47 | 2.24 | 5.82 |
| `brightMagenta` | 3/47 | 1.56 | 5.67 |
| `brightCyan` | 7/47 | 2.16 | 7.01 |
| `brightWhite` | 12/47 | 1.00 | 10.84 |

게이트로 올리지 않는 이유는 둘이다. ① 무채색 양 끝(`black`·`brightBlack`·`white`·`brightWhite`)이
배경과 겹치는 것은 결함이 아니라 터미널 관행이다 — 어떤 터미널이든 검정 배경에 검정 글자를 찍으면
안 보인다. ② 유채색 미달까지 포함하면 137건인데, ANSI 색은 테마 팔레트 그 자체라 `graph.lane*`,
에이전트 상태 아이콘, 신택스 폴백이 함께 쓴다. 임계를 맞추려고 옮기면 터미널보다 훨씬 넓은 범위가
다시 칠해진다.

**재실행.** 쌍 표를 바꿨거나 테마를 추가했으면 `bun run themes:repair-contrast` 로 JSON 을 갱신하고
`bun test src/shared/lib/theme-convert` 로 게이트를 확인한다. 스크립트는 정정 후 §8.5 구별성 린트도
다시 돌려, 두 린트가 공유하는 토큰(`appSidebar.badge`)에서 한쪽 수리가 다른 쪽을 깨지 않았는지
확인한다.


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
