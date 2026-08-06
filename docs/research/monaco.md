# Monaco Editor — Tauri 2 + Vite 기반 IDE 구축 전체 지식 (2026-08 기준)

> 조사일 2026-08-06. 아래 사실은 npm registry / jsDelivr CDN / GitHub 원본 / 실제 `npm i monaco-editor@0.56.0` 후 Node 모듈 해석 실험으로 확인했다.
> 확인하지 못한 항목은 `미확인` 으로 명시했다. 미확인 항목을 사실처럼 구현 전제로 삼지 말 것.

---

## 버전 확정 (2026-08 기준)

| 패키지 | 버전 | 출시일 | 확인 방법 |
|---|---|---|---|
| `monaco-editor` | **0.56.0** | 2026-07-20 | npm registry `latest`, GitHub releases |
| 번들 TypeScript (내장 ts worker) | **5.9.3** | - | `esm/vs/languages/features/typescript/lib/typescriptServices.js` 내 `version = "5.9.3"` |
| `monaco-languageclient` | **10.7.0** (stable) / `11.0.0-next.1` (next) | 10.7.0 = 2026-02-04, 11.0.0-next.1 = 2026-07-16 | npm dist-tags |
| `@codingame/monaco-vscode-api` | **36.0.0** | 2026-07-22 | npm registry |
| `vscode-languageclient` | **10.1.0** | - | npm registry `latest` |
| `vscode-jsonrpc` | **9.0.1** | - | npm registry `latest` |
| `vite` | **8.2.0** (Rolldown 기반) | 2026-07-30 | npm registry |
| `@tauri-apps/api` | **2.11.1** | - | npm registry |
| `tauri` (crate) | **2.11.5** | 2026-07-01 | crates.io API |
| `vite-plugin-monaco-editor` | 1.1.0 (**2022-07-02 이후 방치**) | - | npm registry |
| `vite-plugin-monaco-editor-esm` | 2.0.2 (2025-01-20) | - | npm registry |
| `@monaco-editor/react` | 4.7.0 (2025-02-13) | - | npm registry |

### 0.53 → 0.56 의 파괴적 변경 (TAIDE 에 직접 영향)

CHANGELOG 원문 확인 결과:

- **0.53**: AMD 빌드 deprecated. 번들러 없는 `<script>` 방식 불가. 커스텀 AMD worker 동작 중단.
- **0.55**: 중첩 네임스페이스 `languages.css` / `languages.html` / `languages.json` / `languages.typescript` 를 **최상위로 이동**. 그리고 **`lsp` 네임스페이스(네이티브 LSP 지원) 도입**.
- **0.56**: "Reorganizes the exported ESM modules to provide supported, tree-shakeable entry points" — **package.json `exports` 맵 신설**. `IOverlayWidgetPosition.stackOridinal` → `stackOrdinal` 로 이름 수정. deprecated `IMirrorModel` / `IWorkerContext` worker 타입 제거. diff 알고리즘 `advanced-external`, `advanced-wasm` 추가.

### 0.56 의 `exports` 맵 (실측)

```json
{
  ".":      { "types": "./esm/vs/index.d.ts", "import": "./esm/vs/index.js", "require": "./min/vs/index.js" },
  "./*":    "./esm/vs/*.js",
  "./*.js": "./esm/vs/*.js"
}
```

**이 때문에 기존 문서·블로그·플러그인에 널리 퍼진 `monaco-editor/esm/vs/...` 경로가 0.56 에서 전부 깨진다.**
Node `import.meta.resolve` 실측 결과:

```
monaco-editor                                  -> node_modules/monaco-editor/esm/vs/index.js            (존재)
monaco-editor/editor/editor.worker.js          -> node_modules/monaco-editor/esm/vs/editor/editor.worker.js       (존재)
monaco-editor/language/typescript/ts.worker.js -> node_modules/monaco-editor/esm/vs/language/typescript/ts.worker.js (존재)
monaco-editor/esm/vs/editor/editor.worker.js   -> node_modules/monaco-editor/esm/vs/esm/vs/editor/editor.worker.js  (존재하지 않음)
```

즉 `esm/vs/` 접두사를 **빼야** 한다. (`ls node_modules/monaco-editor/esm/vs/esm` → No such file or directory 로 확인)

### 0.56 진입점 지도 (실측한 실제 파일)

| import 스펙 | 실파일 | 내용 |
|---|---|---|
| `monaco-editor` | `esm/vs/index.js` | 전체 번들: `editor`/`languages` API + **모든 Monarch 언어 등록** + `css`/`html`/`json`/`typescript` 리치 기능 + `lsp` |
| `monaco-editor/editor.js` | `esm/vs/editor.js` | 코어만 재export (`editor`, `languages`, `Range`, `KeyMod` 등). 언어 등록 없음 → **tree-shaking 용 권장 진입점** |
| `monaco-editor/editor/editor.api.js` | `esm/vs/editor/editor.api.js` | 순수 API 표면 |
| `monaco-editor/editor/editor.main.js` | `esm/vs/editor/editor.main.js` | 구 "전부 포함" 진입점 (index.js 와 사실상 동일 구성) |
| `monaco-editor/languages/definitions/<lang>/register.js` | 동일 | Monarch 문법 1개만 등록 |
| `monaco-editor/languages/features/typescript/register.js` | 동일 | `typescriptDefaults`, `javascriptDefaults`, `getTypeScriptWorker`, `typescriptVersion` |
| `monaco-editor/editor/editor.worker.js` | 동일 | 코어 worker |
| `monaco-editor/language/{json,css,html}/{json,css,html}.worker.js` | 동일 | 각 언어 worker |
| `monaco-editor/language/typescript/ts.worker.js` | 동일 | TS/JS worker |

`esm/vs/languages/definitions/` 실측 목록 (일부): `abap apex azcli bat bicep cameligo clojure coffee cpp csharp csp css cypher dart dockerfile ecl elixir flow9 freemarker2 fsharp go graphql handlebars hcl html ini java javascript julia kotlin less lexon liquid lua m3 markdown mdx mips msdax mysql objective-c pascal pascaligo perl pgsql php pla postiats powerquery powershell protobuf pug python qsharp r razor redis redshift restructuredtext ruby rust sb scala scheme scss shell solidity sophia sparql sql st swift systemverilog tcl twig typescript typespec vb wgsl xml yaml`

---

## 핵심 API·사용법

### 1. Vite ESM 통합과 worker 설정

**결론: 플러그인 불필요. `?worker` 수동 배선을 쓴다.**
`vite-plugin-monaco-editor` 는 2022년 이후 미유지 + 구 `esm/vs/...` 경로를 하드코딩하므로 0.56 과 맞지 않는다. `vite-plugin-monaco-editor-esm`(2025-01) 도 0.56 대응 여부 **미확인**.

`src/shared/lib/monaco/setup.ts`:

```ts
import * as monaco from 'monaco-editor'
import editorWorker from 'monaco-editor/editor/editor.worker.js?worker'
import jsonWorker from 'monaco-editor/language/json/json.worker.js?worker'
import cssWorker from 'monaco-editor/language/css/css.worker.js?worker'
import htmlWorker from 'monaco-editor/language/html/html.worker.js?worker'
import tsWorker from 'monaco-editor/language/typescript/ts.worker.js?worker'

self.MonacoEnvironment = {
    getWorker: (_workerId: string, label: string) => {
        if (label === 'json') return new jsonWorker()
        if (label === 'css' || label === 'scss' || label === 'less') return new cssWorker()
        if (label === 'html' || label === 'handlebars' || label === 'razor') return new htmlWorker()
        if (label === 'typescript' || label === 'javascript') return new tsWorker()
        return new editorWorker()
    },
}

export { monaco }
```

- `getWorkerUrl` 이 아니라 **`getWorker`** 를 구현한다 (공식 `docs/integrate-esm.md` 명시).
- 공식 문서의 예제는 아직 구 `monaco-editor/esm/vs/...` 경로를 쓰고 있다 → **0.56 에서는 위처럼 `esm/vs` 를 제거해야 한다.** (경로 해석은 실측 확인, `?worker` 쿼리와 subpath exports 조합의 Vite 8 실동작은 **미확인** — 실패 시 `optimizeDeps.exclude: ['monaco-editor']` 를 먼저 시도)

`vite.config.ts`:

```ts
export default defineConfig({
    worker: { format: 'es' },              // 기본값은 'iife' (Vite 공식 문서 확인)
    optimizeDeps: { exclude: ['monaco-editor'] },
    build: { target: 'es2022' },
})
```

- `optimizeDeps.include` 에 monaco worker 경로를 **넣지 말 것.** Vite 이슈(vitejs/vite#21426)의 근본 원인이 `optimizeDeps` 경로와 `?worker` 임포트 경로 불일치다. 또한 **플러그인 방식과 수동 배선을 섞지 말 것.**
- Monaco ESM 은 `.css` 를 직접 import 한다(패키지 내 `esm/**/*.css` 98개 실측). Vite 가 기본 처리하므로 별도 설정 불필요.

트리셰이킹 최소 번들이 필요하면 진입점을 바꾼다:

```ts
import { editor, languages, KeyMod, KeyCode, Range } from 'monaco-editor/editor.js'
import 'monaco-editor/languages/definitions/typescript/register.js'
import 'monaco-editor/languages/definitions/rust/register.js'
import 'monaco-editor/languages/features/typescript/register.js'
```

### 2. defineTheme 테마 시스템과 앱 CSS 변수 연동

타입 (0.56 `editor.api.d.ts` 실측):

```ts
interface IStandaloneThemeData {
    base: 'vs' | 'vs-dark' | 'hc-black' | 'hc-light'
    inherit: boolean
    rules: ITokenThemeRule[]          // { token, foreground?, background?, fontStyle? }
    encodedTokensColors?: string[]
    colors: { [colorId: string]: string }
}
```

**핵심 제약 (소스 실측):**

- `colors` 값은 `Color.fromHex()` 로 파싱된다. `standaloneThemeService.js` 의 `getColors()` 가 `Color.fromHex(this.themeData.colors[id])` 를 호출하고, `color.js` 의 `fromHex` 는 `Color.Format.CSS.parseHex(hex) || Color.red` 다.
  → **`var(--x)`, `rgb()`, `hsl()`, 색상 이름 전부 불가. 잘못 넣으면 조용히 빨강이 된다.**
- `rules[].foreground` / `background` 는 **`#` 없는 6자리 hex** 관례를 따른다 (내장 테마 규칙과 동일).
- `IStandaloneThemeData` 에는 **`semanticTokenColors` 필드가 없다.** VSCode 테마 JSON 을 그대로 못 넣는다.

**앱 CSS 변수 → Monaco 테마 연동 패턴** (CSS 변수를 진실 소스로 두고, 계산값을 hex 로 변환해 defineTheme):

```ts
const readVar = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim()

const toHex = (cssColor: string) => {
    const ctx = document.createElement('canvas').getContext('2d')!
    ctx.fillStyle = '#000'
    ctx.fillStyle = cssColor
    return ctx.fillStyle as string          // 브라우저가 항상 #rrggbb 로 정규화
}

const applyTaideTheme = (mode: 'light' | 'dark') => {
    monaco.editor.defineTheme('taide', {
        base: mode === 'dark' ? 'vs-dark' : 'vs',
        inherit: true,
        rules: [
            { token: 'comment', foreground: toHex(readVar('--syntax-comment')).slice(1), fontStyle: 'italic' },
            { token: 'keyword', foreground: toHex(readVar('--syntax-keyword')).slice(1) },
            { token: 'string', foreground: toHex(readVar('--syntax-string')).slice(1) },
            { token: 'number', foreground: toHex(readVar('--syntax-number')).slice(1) },
            { token: 'type', foreground: toHex(readVar('--syntax-type')).slice(1) },
        ],
        colors: {
            'editor.background': toHex(readVar('--bg-editor')),
            'editor.foreground': toHex(readVar('--fg-default')),
            'editorLineNumber.foreground': toHex(readVar('--fg-muted')),
            'editorCursor.foreground': toHex(readVar('--accent')),
            'editor.selectionBackground': toHex(readVar('--bg-selection')),
            'editorGutter.background': toHex(readVar('--bg-editor')),
        },
    })
    monaco.editor.setTheme('taide')
}
```

- `toHex` 의 canvas `fillStyle` 정규화는 알파가 있으면 `rgba(...)` 를 돌려줄 수 있다 → 불투명 색만 넣거나 `#rrggbbaa` 를 직접 관리한다. (알파 반환 형태는 **미확인**)
- 테마 전환은 `defineTheme` 재호출 + `setTheme` 재호출. 같은 이름으로 재정의하면 갱신된다.

**역방향 (Monaco → 앱):** `standaloneThemeService.js` 실측상 Monaco 는 모든 color id 를 `--vscode-<id를 -로 치환>` CSS 변수로 만들어 아래 규칙에 주입한다.

```
.monaco-editor, .monaco-diff-editor, .monaco-component { --vscode-editor-background: #...; ... }
```

즉 이 변수들은 **에디터 서브트리 안에서만** 유효하다. 앱 전역(사이드바·탭바)에서 쓰려면 그 규칙을 `:root` 로 재선언하거나, 위처럼 앱 CSS 변수를 진실 소스로 두는 단방향이 안전하다.

**semantic token:** `editor.api.d.ts` 실측 옵션

```ts
monaco.editor.create(el, { 'semanticHighlighting.enabled': true })  // true | false | 'configuredByTheme'
monaco.languages.registerDocumentSemanticTokensProvider(selector, {
    getLegend: () => ({ tokenTypes: ['variable', 'function', 'class'], tokenModifiers: ['declaration', 'readonly'] }),
    provideDocumentSemanticTokens: (model, lastResultId, token) => ({ data: new Uint32Array([...]) }),
    releaseDocumentSemanticTokens: (resultId) => {},
})
```

semantic token 타입이 정확히 어떤 테마 규칙에 매핑되는지(표준 token classification 매핑 테이블)는 **미확인**. `IStandaloneThemeData` 에 semantic 전용 색 필드가 없다는 사실만 확인됨.

### 3. 내장 TS/JS worker 로 되는 것 / 안 되는 것

`languages/features/typescript/register.d.ts` 의 `ModeConfiguration` 실측 — **되는 것**:

`completionItems`, `hovers`, `documentSymbols`, `definitions`, `references`, `documentHighlights`, `rename`, `diagnostics`, `documentRangeFormattingEdits`, `signatureHelp`, `onTypeFormattingEdits`, `codeActions`(quick fix), `inlayHints`

**설정 API**:

```ts
import { typescript } from 'monaco-editor'
// 0.55 부터 monaco.languages.typescript 는 사라지고 최상위로 이동

typescript.typescriptDefaults.setCompilerOptions({
    target: typescript.ScriptTarget.ESNext,
    module: typescript.ModuleKind.ESNext,
    moduleResolution: typescript.ModuleResolutionKind.NodeJs,
    jsx: typescript.JsxEmit.ReactJSX,
    strict: true,
    allowNonTsExtensions: true,
})
typescript.typescriptDefaults.setDiagnosticsOptions({ noSemanticValidation: false, noSyntaxValidation: false })
typescript.typescriptDefaults.setEagerModelSync(true)      // 모든 model 을 worker 로 동기화 (멀티파일 필수)
typescript.typescriptDefaults.addExtraLib(dtsSource, 'file:///node_modules/@types/foo/index.d.ts')

const worker = await typescript.getTypeScriptWorker()
const client = await worker(model.uri)                     // TypeScriptWorker 프록시 (emit, 진단 등)
console.log(typescript.typescriptVersion)                  // "5.9.3"
```

**안 되는 것 (외부 LSP 필요):**

- **`node_modules` 실제 타입 해석.** worker 는 브라우저 안에서 돌고 파일시스템이 없다. `setEagerModelSync(true)` + 열려 있는 model, 또는 `addExtraLib` 로 직접 넣어준 `.d.ts` 만 본다. 프로젝트 전체 타입 그래프는 불가.
- **`tsconfig.json` 자동 로드.** paths/baseUrl/references 를 직접 파싱해 `setCompilerOptions` 로 주입해야 한다.
- **프로젝트 전역 검색** (find all references across project, workspace symbol), call/type hierarchy, code lens, folding range, selection range, semantic tokens, document link, organize imports 등 워크스페이스 스코프 기능.
- **TS 외 언어의 지능형 기능 일체.** 다른 언어는 Monarch 문법 하이라이팅 + word-based completion 뿐.
- JSON/CSS/HTML worker 는 스키마 기반 검증·완성만 제공(별도 worker).

→ **TAIDE 는 TS/JS 도 결국 `typescript-language-server` 를 붙이는 게 맞다.** 그 경우 내장 TS 기능과 충돌하므로 `typescript.typescriptDefaults.setModeConfiguration({...전부 false})` 로 끄고 LSP 로 일원화한다. 다만 `setModeConfiguration` 의 존재는 `LanguageServiceDefaults` 인터페이스에서 확인했으나 전체 시그니처는 **미확인**.

### 4. Decorations — gutter 에 git 상태 표시

핵심 옵션 (`IModelDecorationOptions` 실측, 0.56):

| 필드 | 렌더 위치 |
|---|---|
| `glyphMarginClassName` | glyph margin (line number 왼쪽). `glyphMargin: true` 옵션 필요 |
| `linesDecorationsClassName` | lines decorations 영역 (line number 오른쪽, 텍스트 왼쪽) — **git gutter 바 표준 위치** |
| `linesDecorationsTooltip` | 위 영역 툴팁 (문자열) |
| `firstLineDecorationClassName` | 줄바꿈 시 첫 줄에만 |
| `lineNumberClassName` | 라인 번호 자체 |
| `marginClassName` | margin 전체 폭 |
| `overviewRuler` | 오른쪽 오버뷰 룰러 (`{ color, position }`) |
| `minimap` | 미니맵 (`{ color, position }`) |
| `className` / `inlineClassName` / `isWholeLine` / `zIndex` | 본문 |
| `after` / `before` (`InjectedTextOptions`) | 텍스트 주입 (인라인 blame 용) |
| `glyphMarginHoverMessage` / `hoverMessage` / `lineNumberHoverMessage` | 호버 마크다운 |

git gutter 구현:

```ts
const editor = monaco.editor.create(el, {
    glyphMargin: true,
    lineDecorationsWidth: 6,     // gutter 바 폭. number | string
    lineNumbersMinChars: 3,
})

const gitDecorations = editor.createDecorationsCollection()

type GitHunk = { kind: 'added' | 'modified' | 'deleted'; startLine: number; endLine: number }

const renderGitGutter = (hunks: GitHunk[]) => {
    gitDecorations.set(
        hunks.map((h) => ({
            range: new monaco.Range(h.startLine, 1, h.kind === 'deleted' ? h.startLine : h.endLine, 1),
            options: {
                isWholeLine: true,
                linesDecorationsClassName: `taide-git-gutter taide-git-${h.kind}`,
                linesDecorationsTooltip: h.kind === 'deleted' ? '삭제된 줄' : h.kind === 'added' ? '추가됨' : '수정됨',
                stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
                overviewRuler: {
                    color: h.kind === 'added' ? '#587c0c' : h.kind === 'modified' ? '#0c7d9d' : '#94151b',
                    position: monaco.editor.OverviewRulerLane.Left,
                },
            },
        })),
    )
}
```

```css
.taide-git-gutter { width: 3px !important; margin-left: 3px; }
.taide-git-added { background: var(--git-added); }
.taide-git-modified { background: var(--git-modified); }
/* 삭제는 높이 0 이므로 삼각형 마커로 */
.taide-git-deleted {
    width: 0 !important;
    border-left: 4px solid transparent;
    border-right: 4px solid transparent;
    border-top: 5px solid var(--git-deleted);
    transform: translateY(-2px);
}
```

- **`createDecorationsCollection()` 을 쓴다.** `deltaDecorations` 는 `@deprecated` 로 표시돼 있다(0.56 d.ts 확인). collection 은 `.set()` / `.clear()` / `.getRanges()` 를 제공하고 id 배열을 직접 들고 다니지 않아도 된다.
- gutter 클릭 처리는 `editor.onMouseDown` + `MouseTargetType` (실측 값): `GUTTER_GLYPH_MARGIN = 2`, `GUTTER_LINE_NUMBERS = 3`, `GUTTER_LINE_DECORATIONS = 4`.

```ts
editor.onMouseDown((e) => {
    if (e.target.type === monaco.editor.MouseTargetType.GUTTER_LINE_DECORATIONS) {
        openHunkPeek(e.target.position!.lineNumber)
    }
})
```

### 5. 인라인 blame — content widget vs injected text vs view zone

세 가지 수단이 있고 용도가 다르다.

**(a) injected text (`after`) — GitLens 스타일 "현재 줄 끝 blame". 가장 권장.**
레이아웃/스크롤에 자동으로 따라가고 별도 위치 계산이 없다.

```ts
const blameDecorations = editor.createDecorationsCollection()

const showInlineBlame = (lineNumber: number, text: string) => {
    const col = editor.getModel()!.getLineMaxColumn(lineNumber)
    blameDecorations.set([
        {
            range: new monaco.Range(lineNumber, col, lineNumber, col),
            options: {
                after: {
                    content: `    ${text}`,               // 반드시 한 줄
                    inlineClassName: 'taide-blame',
                    cursorStops: monaco.editor.InjectedTextCursorStops.None,
                },
                showIfCollapsed: true,
                stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
            },
        },
    ])
}

editor.onDidChangeCursorPosition(async (e) => {
    const blame = await getBlame(editor.getModel()!.uri, e.position.lineNumber)
    showInlineBlame(e.position.lineNumber, `${blame.author}, ${blame.relativeDate} · ${blame.summary}`)
})
```

```css
.taide-blame { color: var(--fg-subtle); font-style: italic; opacity: 0.6; }
```

`InjectedTextOptions` 실측 필드: `content`(단일 라인 필수), `inlineClassName`, `inlineClassNameAffectsLetterSpacing`, `attachedData`, `cursorStops`.

**(b) content widget — 우측 정렬 배지, 클릭 가능한 blame 버튼 등.**

```ts
const blameWidget: monaco.editor.IContentWidget = {
    allowEditorOverflow: true,
    suppressMouseDown: false,
    getId: () => 'taide.blame',
    getDomNode: () => node,
    getPosition: () => ({
        position: { lineNumber: currentLine, column: currentColumn },
        preference: [monaco.editor.ContentWidgetPositionPreference.EXACT],
    }),
}
editor.addContentWidget(blameWidget)
// 위치 변경 후에는 반드시
editor.layoutContentWidget(blameWidget)
// 제거
editor.removeContentWidget(blameWidget)
```

`ContentWidgetPositionPreference` 실측: `EXACT = 0`, `ABOVE = 1`, `BELOW = 2`. `IContentWidgetPosition` 에 `secondaryPosition`, `positionAffinity` 가 있고, `IContentWidget` 에 `beforeRender()`/`afterRender()` 훅이 있다.

**(c) view zone — 줄 사이에 실제 공간을 밀어내는 블록** (커밋 상세 패널, inline diff, 리뷰 코멘트).

```ts
let zoneId: string
editor.changeViewZones((accessor) => {
    zoneId = accessor.addZone({
        afterLineNumber: line,
        heightInLines: 3,
        domNode: panelNode,
        marginDomNode: gutterNode,          // 선택: margin 영역에도 붙는 DOM
        suppressMouseDown: false,
        onDomNodeTop: (top) => {},
        onComputedHeight: (h) => {},
    })
})
editor.changeViewZones((accessor) => accessor.removeZone(zoneId))
```

`IViewZone` 실측 필드: `afterLineNumber`, `afterColumn`, `afterColumnAffinity`, `showInHiddenAreas`, `ordinal`, `suppressMouseDown`, `heightInLines`, `heightInPx`, `minWidthInPx`, `domNode`, `marginDomNode`, `onDomNodeTop`, `onComputedHeight`.
`IViewZoneChangeAccessor`: `addZone` / `removeZone` / `layoutZone`.

### 6. DiffEditor (side-by-side)

```ts
const diff = monaco.editor.createDiffEditor(el, {
    renderSideBySide: true,
    renderSideBySideInlineBreakpoint: 900,       // 폭이 이보다 좁으면 inline 으로
    useInlineViewWhenSpaceIsLimited: true,
    originalEditable: false,
    ignoreTrimWhitespace: false,
    renderIndicators: true,
    renderMarginRevertIcon: true,
    renderGutterMenu: true,
    renderOverviewRuler: true,
    diffAlgorithm: 'advanced',                   // 'legacy' | 'advanced' | 'advanced-external' | 'advanced-wasm'
    diffWordWrap: 'inherit',
    maxComputationTime: 5000,
    maxFileSize: 50,                             // MB 단위로 추정 — 단위 미확인
    enableSplitViewResizing: true,
    splitViewDefaultRatio: 0.5,
    compactMode: false,
    hideUnchangedRegions: { enabled: true, revealLineCount: 20, minimumLineCount: 3, contextLineCount: 3 },
    experimental: { showMoves: true, showEmptyDecorations: true, useTrueInlineView: false },
})

diff.setModel({
    original: monaco.editor.createModel(headText, 'typescript', monaco.Uri.parse('taide://git/HEAD/src/a.ts')),
    modified: workingModel,
})

diff.onDidUpdateDiff(() => {
    const changes = diff.getLineChanges()          // ILineChange[] | null
})

const left = diff.getOriginalEditor()              // ICodeEditor
const right = diff.getModifiedEditor()             // ICodeEditor — decoration/widget 은 여기에 붙인다
diff.saveViewState() / diff.restoreViewState(s)    // IDiffEditorViewState
```

- `IDiffEditor.setModel` 은 `IDiffEditorModel | IDiffEditorViewModel | null` 을 받는다. `createViewModel(model)` 로 뷰모델을 만들고 `await vm.waitForDiff()` 하면 diff 계산 완료를 기다릴 수 있다.
- `advanced-wasm` 은 0.56 신규. wasm 애셋 로딩 방법과 Vite/Tauri 에서의 배선은 **미확인** → 우선 `'advanced'` 로 시작할 것.
- diff 에디터 모델은 **직접 dispose** 해야 한다 (아래 §"함정" 참조).

### 7. 키바인딩

**세 가지 API (0.56 실측):**

```ts
// (1) 특정 에디터 인스턴스에 액션 추가 — 컨텍스트 메뉴/명령팔레트에도 노출
editor.addAction({
    id: 'taide.saveFile',
    label: '파일 저장',
    keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
    precondition: undefined,
    keybindingContext: undefined,
    contextMenuGroupId: '1_modification',
    contextMenuOrder: 1.5,
    run: (ed) => saveModel(ed.getModel()!),
})

// (2) 인스턴스에 단축키만 (액션 목록에 안 뜸)
editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyP, () => openQuickFile(), 'editorTextFocus')

// (3) 전역 (모든 에디터)
monaco.editor.addEditorAction({ id, label, keybindings, run })
monaco.editor.addCommand({ id: 'taide.commandPalette', run: () => openPalette() })
monaco.editor.addKeybindingRules([
    { keybinding: monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyP, command: 'taide.commandPalette' },
    { keybinding: monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyP, command: 'taide.quickOpen', when: 'editorTextFocus' },
    { keybinding: monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyB, command: null },   // 기본 바인딩 해제
])
```

`IKeybindingRule` 실측: `{ keybinding: number; command?: string | null; commandArgs?: any; when?: string | null }`.
**`command: null` 로 기본 바인딩을 제거**할 수 있다.

**VSCode 기본 키맵과의 일치 정도:**

- **일치하는 것 (에디터 스코프 contribution 이 그대로 들어있음):** F12 정의로 이동, Alt+F12 peek, Shift+F12 참조, F2 이름 바꾸기, Ctrl/Cmd+D 다음 항목 추가 선택, Ctrl/Cmd+/ 주석, Shift+Alt+F 서식, Ctrl/Cmd+Shift+K 줄 삭제, Alt+↑/↓ 줄 이동, Ctrl/Cmd+F/H 찾기/바꾸기, Ctrl/Cmd+G 줄 이동, Ctrl/Cmd+Space 제안, Ctrl/Cmd+. 퀵픽스, Ctrl/Cmd+Shift+O 심볼 이동, F8 다음 마커 등.
- **다른 것:** 명령 팔레트가 **F1** 이다. VSCode 소스 `standaloneCommandsQuickAccess.ts` 확인 결과 `editor.action.quickCommand` 의 `kbOpts.primary` 가 `KeyCode.F1` 단일이다. **Ctrl/Cmd+Shift+P 는 기본 바인딩이 없다** → 위 `addKeybindingRules` 로 직접 추가해야 한다.
- **아예 없는 것 (workbench 레벨이므로 TAIDE 가 직접 구현):** Ctrl/Cmd+P 파일 퀵오픈, Ctrl/Cmd+S 저장, Ctrl/Cmd+W 탭 닫기, Ctrl/Cmd+Shift+E 탐색기, Ctrl/Cmd+` 터미널, Ctrl/Cmd+B 사이드바 토글, Ctrl/Cmd+Shift+F 전역 검색, 탭 전환(Ctrl+Tab, Ctrl+1..9).

**보강 전략:** 에디터가 포커스일 때만 필요한 것은 `editor.addAction` / `addKeybindingRules`(`when: 'editorTextFocus'`), 에디터 밖에서도 먹어야 하는 것은 **앱 레벨 전역 핸들러**(window keydown 또는 Tauri global shortcut)로 처리하고 에디터 안에서는 `addKeybindingRule({ keybinding, command: null })` 로 monaco 기본 동작을 막아 충돌을 없앤다.

### 8. multi-model 탭 관리와 view state

```ts
type OpenTab = { uri: string; model: monaco.editor.ITextModel; viewState: monaco.editor.ICodeEditorViewState | null }

const tabs = new Map<string, OpenTab>()

const openFile = (path: string, content: string, languageId: string) => {
    const uri = monaco.Uri.file(path)                      // 파일당 고유 URI 필수
    const key = uri.toString()
    let tab = tabs.get(key)
    if (!tab) {
        const model = monaco.editor.getModel(uri) ?? monaco.editor.createModel(content, languageId, uri)
        tab = { uri: key, model, viewState: null }
        tabs.set(key, tab)
    }
    return tab
}

const activateTab = (nextKey: string) => {
    const current = editor.getModel()
    if (current) {
        const prev = tabs.get(current.uri.toString())
        if (prev) prev.viewState = editor.saveViewState()   // 반드시 setModel 전에
    }
    const next = tabs.get(nextKey)!
    editor.setModel(next.model)
    if (next.viewState) editor.restoreViewState(next.viewState)
    editor.focus()                                          // restoreViewState 는 포커스를 주지 않는다
}

const closeTab = (key: string) => {
    const tab = tabs.get(key)
    if (!tab) return
    tabs.delete(key)
    tab.model.dispose()                                     // 안 하면 누수 + TS worker 가 계속 들고 있음
}
```

- `ICodeEditorViewState` 는 커서/선택/스크롤/접힘(fold) 상태를 담은 **직렬화 가능 객체**다 → `JSON.stringify` 해서 세션 복원에 그대로 쓸 수 있다.
- 모델 URI 는 `monaco.Uri.file(absPath)` 로. TS worker 의 크로스파일 해석, LSP 의 `textDocument/didOpen` URI 가 전부 여기에 의존한다.
- **한 에디터 인스턴스 + model 교체**가 정석이다. 탭마다 에디터 인스턴스를 만들면 DOM/worker 비용이 폭증한다.
- 언어 변경은 `monaco.editor.setModelLanguage(model, languageId)`.

### 9. 대형 파일 성능

**하드코딩된 임계값** (VSCode `src/vs/editor/common/model/textModel.ts` main 브랜치 실측 — monaco 0.56 번들 값은 근사치로 봐야 함):

```
_MODEL_SYNC_LIMIT                 = 50 MB    → 초과 시 worker 동기화 중단 (TS/LSP 기능 죽음)
LARGE_FILE_SIZE_THRESHOLD         = 20 MB    → 초과 시 tokenization 중단
LARGE_FILE_LINE_COUNT_THRESHOLD   = 300,000 줄 → 위와 동일 (OR 조건)
LARGE_FILE_HEAP_OPERATION_THRESHOLD = 256M chars
```

`largeFileOptimizations: true`(기본값)일 때만 위 20MB/300K 판정이 적용된다. `false` 로 두면 판정을 건너뛰고 무조건 토크나이즈하려다 멈출 수 있다.

**튜닝 옵션** (실측 기본값):

```ts
monaco.editor.create(el, {
    largeFileOptimizations: true,      // 기본 true
    maxTokenizationLineLength: 20000,  // 기본 20000 — 이보다 긴 줄은 토크나이즈 안 함
    stopRenderingLineAfter: 10000,     // 기본 10000, -1 이면 무제한
    wordWrap: 'off',                   // wrap 은 대형 파일에서 매우 비쌈
    minimap: { enabled: false },
    folding: false,
    occurrencesHighlight: 'off',
    renderWhitespace: 'none',
    renderValidationDecorations: 'off',
    wordBasedSuggestions: 'off',       // 'off' | 'currentDocument' | 'matchingDocuments' | 'allDocuments'
    bracketPairColorization: { enabled: false },
    guides: { indentation: false, bracketPairs: false },
    codeLens: false,
    'semanticHighlighting.enabled': false,
})
```

TAIDE 권장: 파일 열기 전에 바이트 크기를 Rust 쪽에서 확인해 임계값(예: 2MB/50K줄)을 넘으면 위 옵션을 적용한 "대형 파일 모드"로 열고 LSP `didOpen` 도 보내지 않는다.

### 10. LSP 연결 — 두 가지 경로

#### 경로 A (권장): monaco-editor 0.56 내장 `lsp` 네임스페이스 + Tauri IPC 커스텀 transport

0.55 에서 도입되고 0.56 에 포함된 `monaco.lsp` 는 `esm/external/monaco-lsp-client/out/index.js`(실측 136KB)에 있고 다음을 export 한다.

```ts
export { MonacoLspClient, WebSocketTransport, createTransportToIFrame, createTransportToWorker }
```

`MonacoLspClient` 의 생성자는 **`IMessageTransport` 하나만** 받는다 — WebSocket 강제가 아니다. 소스 실측:

```ts
class MonacoLspClient {
    constructor(transport: IMessageTransport)
}
interface IMessageTransport {
    get state(): IValueWithChangeEvent<ConnectionState>   // { value, onChange }
    send(message: Message): Promise<void>                  // Message = JSON-RPC 객체 (프레이밍 없음)
    setListener(listener: ((message: Message) => void) | undefined): void
    toString(): string
}
type ConnectionState = { state: 'connecting' } | { state: 'open' } | { state: 'closed'; error: Error | undefined }
```

내부적으로 `createFeatures()` 가 등록하는 것(실측): completion, hover, signatureHelp, definition, declaration, typeDefinition, implementation, references, documentHighlight, documentSymbol, rename, codeAction, codeLens, documentLink, formatting, rangeFormatting, onTypeFormatting, foldingRange, selectionRange, inlayHints, semanticTokens, diagnostics. `TextDocumentSynchronizer` 가 `editor.onDidCreateModel` 로 **모든 model 을 자동 didOpen/didChange** 한다.

**Tauri IPC transport 구현:**

```ts
// src/shared/lib/lsp/tauri-transport.ts
import { invoke, Channel } from '@tauri-apps/api/core'

type JsonRpcMessage = { jsonrpc: '2.0'; [k: string]: unknown }
type ConnectionState = { state: 'connecting' } | { state: 'open' } | { state: 'closed'; error: Error | undefined }

class ValueWithChangeEvent<T> {
    private _value: T
    private listeners = new Set<(v: T) => void>()
    constructor(initial: T) { this._value = initial }
    get value() { return this._value }
    set value(v: T) { this._value = v; this.listeners.forEach((l) => l(v)) }
    onChange = (listener: (v: T) => void) => {
        this.listeners.add(listener)
        return { dispose: () => { this.listeners.delete(listener) } }
    }
}

export class TauriLspTransport {
    readonly state = new ValueWithChangeEvent<ConnectionState>({ state: 'connecting' })
    private listener: ((m: JsonRpcMessage) => void) | undefined
    private queue: JsonRpcMessage[] = []
    private serverId = ''

    static async spawn(serverName: string, command: string, args: string[], cwd: string) {
        const transport = new TauriLspTransport()
        const channel = new Channel<JsonRpcMessage>()
        channel.onmessage = (message) => {
            if (transport.listener) transport.listener(message)
            else transport.queue.push(message)
        }
        transport.serverId = await invoke<string>('lsp_spawn', { serverName, command, args, cwd, onMessage: channel })
        transport.state.value = { state: 'open' }
        return transport
    }

    send = async (message: JsonRpcMessage) => {
        await invoke('lsp_send', { serverId: this.serverId, message })
    }

    setListener = (listener: ((m: JsonRpcMessage) => void) | undefined) => {
        this.listener = listener
        if (!listener) return
        const pending = this.queue
        this.queue = []
        pending.forEach((m) => listener(m))
    }

    toString = () => `TauriLspTransport(${this.serverId})`

    close = async () => {
        await invoke('lsp_stop', { serverId: this.serverId })
        this.state.value = { state: 'closed', error: undefined }
    }
}
```

```ts
import { lsp } from 'monaco-editor'

const transport = await TauriLspTransport.spawn('rust-analyzer', 'rust-analyzer', [], projectRoot)
const client = new lsp.MonacoLspClient(transport as unknown as Parameters<typeof lsp.MonacoLspClient>[0])
```

Rust 측 (Tauri 2 Channel, `tauri::ipc::Channel` — 공식 문서 확인):

```rust
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use tauri::ipc::Channel;

pub struct LspServer { child: Child, stdin: ChildStdin }
pub struct LspState(pub Mutex<std::collections::HashMap<String, LspServer>>);

#[tauri::command]
fn lsp_spawn(
    server_name: String,
    command: String,
    args: Vec<String>,
    cwd: String,
    on_message: Channel<serde_json::Value>,
    state: tauri::State<'_, LspState>,
) -> Result<String, String> {
    let mut child = Command::new(&command)
        .args(&args).current_dir(&cwd)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| e.to_string())?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stdin = child.stdin.take().ok_or("no stdin")?;

    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            // Content-Length 헤더 프레이밍 파싱
            let mut content_length = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 { return; }
                let trimmed = line.trim_end();
                if trimmed.is_empty() { break; }
                if let Some(v) = trimmed.strip_prefix("Content-Length:") {
                    content_length = v.trim().parse().unwrap_or(0);
                }
            }
            if content_length == 0 { continue; }
            let mut buf = vec![0u8; content_length];
            if reader.read_exact(&mut buf).is_err() { return; }
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&buf) {
                let _ = on_message.send(value);
            }
        }
    });

    let id = format!("{server_name}-{}", child.id());
    state.0.lock().unwrap().insert(id.clone(), LspServer { child, stdin });
    Ok(id)
}

#[tauri::command]
fn lsp_send(server_id: String, message: serde_json::Value, state: tauri::State<'_, LspState>) -> Result<(), String> {
    let mut map = state.0.lock().unwrap();
    let server = map.get_mut(&server_id).ok_or("unknown server")?;
    let body = serde_json::to_vec(&message).map_err(|e| e.to_string())?;
    server.stdin.write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes()).map_err(|e| e.to_string())?;
    server.stdin.write_all(&body).map_err(|e| e.to_string())?;
    server.stdin.flush().map_err(|e| e.to_string())
}
```

**프레이밍은 Rust 쪽에서만 처리한다.** JS 로는 파싱된 JSON 객체가 그대로 오간다 (`IMessageTransport.send(message: Message)` 는 객체를 받는다).

**경로 A 의 실측 한계 (소스에서 직접 확인):**

- `_init()` 이 `initialize({ processId: null, capabilities, rootUri: null })` 를 **하드코딩**한다. `rootUri`/`workspaceFolders`/`initializationOptions` 를 넣을 수 없다 → **rust-analyzer·gopls·tsserver 처럼 워크스페이스 루트가 필수인 서버는 제대로 동작하지 않을 가능성이 높다.**
- `workspace/didChangeConfiguration`, `workspace/didChangeWatchedFiles`, workspace symbol, call/type hierarchy, `textDocument/didSave`(`didSave: false`), `willSave` 미지원.
- URI 를 `toLowerCase()` 로 정규화해 관리한다 → 대소문자 구분 파일시스템에서 충돌 가능.
- 공식 문서·마이그레이션 가이드가 **아직 없다**(CHANGELOG 한 줄 언급뿐). API 안정성 보장 없음.

#### 경로 B: monaco-languageclient 10.7.0 (+ vscode-languageclient)

**주의: 이건 monaco-editor 를 그대로 쓰는 게 아니다.** 10.7.0 은 `@codingame/monaco-vscode-api ^25.1.2` 및 `@codingame/monaco-vscode-editor-api` 에 의존한다 — monaco-editor 를 **VSCode 포크로 교체**하고 30개 이상의 service-override / language-pack 패키지를 끌어온다. 번들 크기와 초기화 복잡도가 급증한다. (버전 정합 주의: stable 10.7.0 은 codingame `^25.1.2`+`vscode-languageclient ~9.0.1` 을 쓰는데 codingame latest 는 이미 36.0.0, `vscode-languageclient` latest 는 10.1.0 이다. `11.0.0-next.1` 이 codingame `^35.0.1` + `vscode-languageclient ~10.1.0` 으로 올라와 있다.)

대신 얻는 것: 완전한 LSP 클라이언트(initialize 옵션 전체, workspace folders, 설정 동기화, 파일 감시, 서버 재시작, 제안된 기능(proposed features)).

커스텀 transport 는 두 방법:

```ts
// (1) MonacoLanguageClient 직접 (packages/client/src/index.ts 실측)
import { MonacoLanguageClient } from 'monaco-languageclient'
import { AbstractMessageReader, AbstractMessageWriter, type DataCallback, type Message } from 'vscode-jsonrpc'

class TauriMessageReader extends AbstractMessageReader {
    private callback: DataCallback | undefined
    constructor(private readonly channel: Channel<Message>) {
        super()
        channel.onmessage = (m) => this.callback?.(m)
    }
    listen(callback: DataCallback) {
        this.callback = callback
        return { dispose: () => { this.callback = undefined } }
    }
}

class TauriMessageWriter extends AbstractMessageWriter {
    constructor(private readonly serverId: string) { super() }
    async write(msg: Message) { await invoke('lsp_send', { serverId: this.serverId, message: msg }) }
    end() {}
}

const client = new MonacoLanguageClient({
    name: 'rust-analyzer',
    clientOptions: {
        documentSelector: [{ language: 'rust' }],
        workspaceFolder: { uri: monaco.Uri.file(projectRoot), name: 'taide', index: 0 },
        initializationOptions: { cargo: { allFeatures: true } },
        errorHandler: { error: () => ({ action: ErrorAction.Continue }), closed: () => ({ action: CloseAction.Restart }) },
    },
    messageTransports: { reader: new TauriMessageReader(channel), writer: new TauriMessageWriter(serverId) },
})
await client.start()
```

```ts
// (2) LanguageClientWrapper 사용 시 — connection.messageTransports 로 주입 (lcconfig.ts 실측)
export interface ConnectionConfig {
    options: ConnectionConfigOptions            // 'WebSocketDirect'|'WebSocketParams'|'WebSocketUrl'|'WorkerConfig'|'WorkerDirect'
    messageTransports?: MessageTransports        // 지정하면 WebSocket/Worker 생성 로직을 건너뛴다
}
```

`lcwrapper.ts` 실측상 `messageTransports` 가 있으면 기본 reader/writer 생성을 건너뛴다. 다만 `options.$type` 은 여전히 필수이고 WebSocket/Worker 분기에만 쓰이므로, IPC 전용이면 **(1) MonacoLanguageClient 직접 생성이 더 깨끗하다.**

---

## TAIDE 적용 가이드

1. **의존성 고정**: `monaco-editor@0.56.0`. `vite-plugin-monaco-editor` 계열 설치하지 않는다. `@monaco-editor/react` 도 쓰지 않고(0.56 레이아웃 대응 여부 미확인 + loader 가 CDN 기본이라 오프라인 데스크톱에 부적합) 얇은 React 래퍼를 직접 만든다.

2. **레이어 배치(FSD)**
   - `shared/lib/monaco/setup.ts` — `MonacoEnvironment` 배선, 1회 실행
   - `shared/lib/monaco/theme.ts` — CSS 변수 → `defineTheme` 브릿지
   - `shared/lib/monaco/keymap.ts` — `addKeybindingRules` 전역 키맵
   - `entities/editor/model-registry.ts` — URI ↔ model ↔ viewState 레지스트리
   - `shared/lib/lsp/tauri-transport.ts` — IPC transport
   - `features/editor/code-editor.tsx` — DOM 마운트/언마운트만 하는 순수 컴포넌트
   - `widgets/editor/editor-pane.tsx` — 탭 상태, git decoration, blame, LSP 라이프사이클

3. **React 래퍼 원칙**: monaco 인스턴스는 React 렌더링 밖에 있다. `useRef` 로 컨테이너를 잡고 마운트 시 1회 `create`, 언마운트 시 `editor.dispose()`. **model 은 에디터가 아니라 레지스트리가 소유**하고 탭 닫을 때만 `model.dispose()`.

4. **LSP 전략 (권장 순서)**
   - 1단계: TS/JS 만 내장 worker 로. `setEagerModelSync(true)` + `tsconfig` 를 Rust 에서 읽어 `setCompilerOptions` 주입. 빠르게 동작하는 베이스라인 확보.
   - 2단계: Rust/Python 등은 **경로 B(monaco-languageclient + `MonacoLanguageClient` 직접 + Tauri IPC transport)**. `rootUri`/`initializationOptions` 가 필요하므로 경로 A 로는 부족할 가능성이 높다.
   - 경로 A(`monaco.lsp`)는 **워크스페이스 루트가 필요 없는 단순 서버**(예: 단일 파일 대상 linter LSP)나 프로토타이핑에만.
   - 두 경로를 동시에 쓰지 말 것 — 경로 B 는 monaco-editor 자체를 codingame 포크로 교체하므로 공존 불가.

5. **git gutter/blame 데이터 경로**: Rust(`git2` 등)에서 hunk/blame 을 계산 → Tauri command 로 반환 → `linesDecorationsClassName`(gutter) + `after` injected text(blame). blame 은 `onDidChangeCursorPosition` 디바운스(150~300ms) 후 현재 줄만 요청.

6. **키맵**: 명령 팔레트는 F1 이 기본이므로 `addKeybindingRules` 로 Ctrl/Cmd+Shift+P 를 `editor.action.quickCommand` 에 추가. 워크벤치 단축키(Ctrl+P/S/W/B, Ctrl+Tab)는 앱 전역 리스너로 구현하고, 에디터 안에서 monaco 기본과 겹치는 것은 `{ keybinding, command: null }` 로 해제.

7. **세션 복원**: 탭 목록 + `saveViewState()` 결과 + 활성 탭을 JSON 으로 Tauri store 에 저장 → 재시작 시 `restoreViewState`.

---

## 함정·주의

1. **`monaco-editor/esm/vs/...` 경로는 0.56 에서 전부 깨진다.** `exports` 맵 때문에 `esm/vs/esm/vs/...` 로 해석된다(Node 실측 확인). 인터넷의 대부분의 예제·플러그인·공식 `integrate-esm.md` 조차 아직 구 경로다. `monaco-editor/editor/editor.worker.js` 처럼 `esm/vs` 를 제거할 것.

2. **`monaco.languages.typescript` 는 0.55 부터 없다.** `import { typescript } from 'monaco-editor'` 또는 `monaco-editor/languages/features/typescript/register.js`.

3. **`defineTheme` 의 `colors` 는 hex 전용.** `Color.fromHex(hex) || Color.red` (소스 실측) — 잘못된 값이면 예외 없이 **빨간색**이 된다. CSS 변수·`rgb()`·색이름 금지.

4. **`deltaDecorations` 는 deprecated.** `createDecorationsCollection()` 사용.

5. **`stackOridinal` → `stackOrdinal`** 오타 수정(0.56 breaking). overlay widget 쓰면 확인.

6. **model 누수**: `editor.dispose()` 는 model 을 파괴하지 않는다. 같은 URI 로 `createModel` 을 두 번 부르면 예외가 난다 → 항상 `monaco.editor.getModel(uri)` 로 먼저 확인. DiffEditor 의 original/modified model 도 수동 dispose.

7. **`restoreViewState` 는 포커스를 주지 않는다.** 탭 전환 후 `editor.focus()` 를 따로 호출.

8. **injected text(`after.content`)는 단일 라인만 허용.** 개행이 들어가면 렌더가 깨진다. blame 요약에 `\n` 이 없도록 잘라낼 것.

9. **content widget 은 위치가 바뀌면 `layoutContentWidget()` 을 불러야** 다시 배치된다. 에디터 밖으로 넘칠 수 있으면 `allowEditorOverflow: true`.

10. **Vite `worker.format` 기본값은 `'iife'`** (Vite 8 공식 문서 확인). 모듈 worker 를 기대한다면 `worker: { format: 'es' }` 를 명시. 또한 Vite 8 은 Rolldown 기반이라 `worker.rollupOptions` 는 deprecated(→ `worker.rolldownOptions`).

11. **플러그인 방식과 수동 `MonacoEnvironment` 배선을 섞지 말 것.** vitejs/vite#21426 의 원인.

12. **Tauri CSP**: Monaco 는 런타임에 `<style>` 를 주입하고 별도 worker 파일을 로드한다. `tauri.conf.json` 에 CSP 를 설정한다면 `style-src 'unsafe-inline'` 와 `worker-src 'self' blob:` 를 반드시 허용해야 한다. (Tauri 커스텀 프로토콜에서의 module worker 실동작은 **미확인** — 초기에 CSP `null` 로 검증 후 조이는 것을 권장)

13. **경로 A(`monaco.lsp`)의 `rootUri: null` 하드코딩** — 워크스페이스 기반 서버에는 치명적. 소스에서 직접 확인한 사실이며 우회 방법 **미확인**.

14. **monaco-languageclient 버전 정합**: `latest`(10.7.0, 2026-02)가 의존하는 `@codingame/monaco-vscode-api ^25.1.2` 는 현재 latest(36.0.0)보다 한참 뒤다. `pnpm`/`npm` overrides 로 억지 승격하지 말 것 — API 파손 가능. 최신 조합이 필요하면 `11.0.0-next.1` 을 평가한다(prerelease).

15. **대형 파일 임계값은 VSCode main 브랜치 소스 기준**이다. monaco 0.56 번들에 그대로 반영됐는지는 별도 확인이 필요하다(**미확인**). 실제 운영 임계값은 자체 측정으로 정할 것.

16. **`diffAlgorithm: 'advanced-wasm'`** 은 wasm 애셋을 필요로 할 가능성이 높다. Vite/Tauri 배선 방법 **미확인** → `'advanced'` 로 시작.

17. **`monaco-editor` 기본 진입점은 모든 언어를 등록한다** (`esm/vs/index.js` 가 90개 이상의 `languages/definitions/*/register.js` 를 import). 번들 크기가 문제면 `monaco-editor/editor.js` + 필요한 언어만 개별 import.

18. **`@monaco-editor/react` 4.7.0(2025-02)** 은 0.56 의 새 레이아웃 이전 버전이고 기본적으로 CDN loader 를 쓴다. 오프라인 데스크톱 앱에는 부적합 — 직접 래퍼를 쓸 것.

---

## 참고 링크

- monaco-editor npm: https://www.npmjs.com/package/monaco-editor
- monaco-editor 릴리즈: https://github.com/microsoft/monaco-editor/releases
- monaco-editor CHANGELOG: https://raw.githubusercontent.com/microsoft/monaco-editor/main/CHANGELOG.md
- ESM 통합 가이드(Vite 섹션 포함, 경로는 0.56 기준으로 보정 필요): https://github.com/microsoft/monaco-editor/blob/main/docs/integrate-esm.md
- 0.56 타입 정의(전체 API 표면): https://cdn.jsdelivr.net/npm/monaco-editor@0.56.0/esm/vs/editor/editor.api.d.ts
- 0.56 내장 LSP 클라이언트 타입: https://cdn.jsdelivr.net/npm/monaco-editor@0.56.0/esm/external/monaco-lsp-client/out/index.d.ts
- 0.56 TypeScript 기능 타입: https://cdn.jsdelivr.net/npm/monaco-editor@0.56.0/esm/vs/languages/features/typescript/register.d.ts
- VSCode 대형 파일 임계값 원본: https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/model/textModel.ts
- VSCode standalone 명령 팔레트(F1) 원본: https://github.com/microsoft/vscode/blob/main/src/vs/editor/standalone/browser/quickAccess/standaloneCommandsQuickAccess.ts
- VSCode 테마 color id 목록: https://code.visualstudio.com/api/references/theme-color
- Vite worker 옵션: https://vite.dev/config/worker-options
- Vite monaco worker 해석 이슈: https://github.com/vitejs/vite/discussions/21426
- monaco-languageclient 저장소: https://github.com/TypeFox/monaco-languageclient
- monaco-languageclient `MonacoLanguageClient` 원본: https://github.com/TypeFox/monaco-languageclient/blob/main/packages/client/src/index.ts
- monaco-languageclient `ConnectionConfig`/`messageTransports`: https://github.com/TypeFox/monaco-languageclient/blob/main/packages/client/src/wrapper/lcconfig.ts
- monaco-languageclient transport 사용부: https://github.com/TypeFox/monaco-languageclient/blob/main/packages/client/src/wrapper/lcwrapper.ts
- @codingame/monaco-vscode-api: https://github.com/CodinGame/monaco-vscode-api
- vscode-jsonrpc (MessageReader/Writer): https://github.com/microsoft/vscode-languageserver-node/tree/main/jsonrpc
- Tauri 2 Channel(프론트엔드 호출): https://v2.tauri.app/develop/calling-frontend/
- Tauri 2 커맨드: https://v2.tauri.app/develop/calling-rust/
