# shiki — TextMate 문법 렌더링 엔진 (7.10-W7)

> 조사일 2026-08-12. 정찰 리서치(영역 1: shiki+monaco 통합·CSP·번들 임팩트)를 메인이 npm
> tarball 실물·GitHub 소스·로컬 `node_modules/monaco-editor@0.56.0` 으로 교차검증한 뒤 정리했다.
> 확정 결정은 `docs/acknowledge/2026-08-12-w7-textmate-contract.md`, 확정 설계는
> `docs/theme-system.md` §4.2. 확인하지 못한 항목은 `미확인` 으로 명시했다.

## 1. 패키지 구성·버전

**최신 안정 버전 4.4.3**(2026-08-10 publish, npm registry 직접 조회로 확인). `shiki` 및
`@shikijs/*` 전 패키지가 같은 버전으로 락스텝 배포된다.

| 패키지 | 역할 | 채택 |
|---|---|---|
| `@shikijs/core` | `createHighlighterCore` 등 코어 API | 채택 |
| `@shikijs/engine-javascript` | JS 네이티브 RegExp 엔진(CSP 안전) | 채택 |
| `@shikijs/langs` | TextMate grammar 모음(서브패스 개별 import) | 채택 |
| `@shikijs/monaco` | `shikiToMonaco` | 채택 |
| `shiki`(메타 패키지) | 전 언어·전 테마 풀번들 + subpath re-export | **미설치**(§2) |
| `@shikijs/engine-oniguruma` | WASM 엔진 | **채택 안 함**(§3, CSP 위반) |
| `@shikijs/langs-precompiled` | 사전 컴파일 grammar | **채택 안 함**(공식 "not yet supported" 경고, shikijs/shiki#918) |

`monaco-editor-core` 를 devDependency(타입 전용)로 추가한다 — `@shikijs/monaco` 의
`dist/index.d.mts` 가 `monaco-editor-core` 를 import 하는데 우리 dependencies 에는 없어
`tsc` 모듈 해석이 실패하기 때문이다(`@shikijs/monaco` 의 devDependencies 자체가
`monaco-editor-core: ^0.56.0` — 우리 monaco-editor 0.56.0 과 같은 세대).

공식 fine-grained 패턴(`docs/guide/best-performance.md`):

```ts
import { createHighlighterCore } from '@shikijs/core'
import { createJavaScriptRegexEngine } from '@shikijs/engine-javascript'

const highlighter = await createHighlighterCore({
    themes: [taideTheme],
    langs: [import('@shikijs/langs/typescript'), import('@shikijs/langs/rust')],
    engine: createJavaScriptRegexEngine(),
})
```

## 2. `createHighlighter` 금지 — `createHighlighterCore` 강제

`shiki` 메타 패키지의 `createHighlighter` 는 **oniguruma WASM 이 기본 엔진**이다. 소스 확인:
`packages/shiki/src/bundle-full.ts` → `engine: () => createOnigurumaEngine(import('shiki/wasm'))`.
그대로 쓰면 CSP 위반이 즉시 발생한다. **반드시 `createHighlighterCore` + 명시적
`engine: createJavaScriptRegexEngine()`** 를 써야 한다 — 리뷰 체크 항목으로 삼을 것.

## 3. JS RegExp 엔진과 CSP — 정적 검증 통과

- `@shikijs/engine-javascript` 는 `oniguruma-to-es`(Oniguruma 패턴 → 네이티브 JS 정규식 transpile)
  에 의존한다.
- **`eval`/`new Function` 사용 여부**: npm tarball 을 받아 `@shikijs/engine-javascript`,
  `oniguruma-to-es`, `@shikijs/core`, `@shikijs/primitive`, `@shikijs/vscode-textmate`,
  `@shikijs/monaco` 의 dist 전체를 정규식 grep(`\beval\s*\(`, `new Function\s*\(`) 한 결과
  **매칭 0건**(메인 재검증). 정규식 생성은 `new RegExp(...)` 로 이뤄지며, `new RegExp` 는 CSP
  `script-src` 의 제약 대상이 **아니다**(eval/Function 만 대상).
  → **현행 CSP(`script-src 'self'`) 를 한 글자도 바꾸지 않고 동작할 것으로 판단.** 단 실제
  WKWebView(Tauri macOS) 런타임 실행 검증은 **미수행** — W7 구현 직후 1순위 실기 확인 항목(QA6).
- **호환율**(공식 수치, `docs/references/engine-js-compat.md`): 총 238개 중 supported 237,
  mismatched 0, unsupported 1(`ahk2`). TAIDE 의 31개 언어는 전부 supported 범위.
- 비호환 grammar 를 만나면 JS 엔진은 기본이 strict(throw). `forgiving: true` 옵션으로 억제
  가능하나 "highlighting mismatches" 를 공식 문서가 경고한다 — 커스텀 grammar 를 넣지 않는 한
  불필요.
- `v` 플래그(ES2024/Node 20+)를 기본 사용, 미지원 환경은 `u` 플래그로 자동 폴백.

## 4. Oniguruma WASM — 폴백으로도 두지 않는다

`@shikijs/engine-oniguruma` dist 는 `WebAssembly.instantiate`/`instantiateStreaming` 을
사용한다(실물 grep 확인). MDN CSP 문서: `'wasm-unsafe-eval'` 없이는 WebAssembly 로딩·실행이
차단된다. 폴백으로 쓰려면 CSP 를 `script-src 'self' 'wasm-unsafe-eval'` 로 완화해야 하고, W6
원격 서빙(`src-tauri/src/domain/remote/serving.rs`)의 파생 CSP 도 함께 완화된다 — **보안 후퇴이므로
폴백 자체를 두지 않는다.**

## 5. `shikiToMonaco` 내부 동작 (`@shikijs/monaco` 4.4.3 소스 실물)

- **토크나이저 부착**: `monaco.languages.setTokensProvider(lang, {...})` 를
  `highlighter.getLoadedLanguages() ∩ monaco.languages.getLanguages()` 에만 건다.
  `monaco.languages.register` 는 **하지 않는다** — 미등록 id 는 우리가 선행 register 해야 한다
  (`docs/theme-system.md` §4.2, `docs/research/monaco.md` "미등록 languageId" 절).
- **스냅샷 동작**: 위 부착은 호출 시점의 언어 목록으로 1회 수행된다. 이후
  `highlighter.loadLanguage(...)` 로 언어를 추가해도 자동 반영되지 않는다 — 재호출 필요.
- **테마 등록**: `highlighter.getLoadedThemes()` 전부를 `monaco.editor.defineTheme(themeId,
  { base, inherit: false, colors, rules })` 로 등록한다. **`inherit: false`** 다(TAIDE 기존은
  `inherit: true`) — shiki 테마의 `colors` 에 없는 monaco 색 키는 우리 매핑이 아니라 monaco
  기본 `vs`/`vs-dark` 값으로 떨어진다. `colors` 는 `theme.colors` 를 정규화해 통째로 넣는다.
  `rules` 는 테마의 tokenColors 전 scope 를 전개해 기존 수기 31 rule 을 대체한다.
- **몽키패치**: `monaco.editor.setTheme`/`monaco.editor.create` 를 몽키패치하고, 함수 말미에
  `monaco.editor.setTheme(themeIds[0])` 를 강제 호출한다. 패치 클로저가 호출 시점의
  `themeMap`(테마→룰)을 캡처하므로 **호출 이후 로드된 테마는 토큰 스코프 매핑이 비어 렌더가
  깨진다.** 재호출 시 패치가 중첩된다(멱등하지 않음).
- **`textmateThemeToMonacoTheme`** 가 export 되어 있다(직접 변환이 필요할 때 재사용 가능).
- **토큰화 품질(구조적 한계)**: `tokenize()` 는 색상+fontStyle 로 스코프를 역추적한 "근사
  scope 문자열 1개" 를 monaco 에 돌려준다(monaco 가 토큰당 scope 1개만 지원하기 때문 — 소스
  주석 확인). 색은 정확하지만 scope 자체는 근사값이다.
  `tokenizeMaxLineLength` 기본 20000, `tokenizeTimeLimit` 기본 500ms(초과 시 `console.warn`,
  옵션으로 조정 가능 — 플러그인 grammar 의 ReDoS 완화 근거로 재사용, `docs/features/plugins.md` §4).

## 6. TAIDE 수명 관리 설계 — 몽키패치·`inherit:false` 대응

리서치가 제시한 문제(§5)에 대해 W7 이 확정한 대응(`docs/theme-system.md` §4.2 정본):

- **단일 테마명 전략**: `@shikijs/core` 의 `loadTheme` 은 동일 이름이면 `Map.set` 으로
  교체한다(실물 확인) — 라이트/다크를 별도 이름으로 등록하지 않고 항상 `taide` 1개만 로드,
  전환 시 `loadTheme` 로 교체한다.
- **재호출 절차**: 테마 변경마다 ① `highlighter.loadTheme(buildShikiTheme(resolved))`, ②
  `monaco.editor.setTheme`/`create` 원본 복원(모듈이 최초 1회 보관), ③
  `shikiToMonaco(highlighter, monaco)` 재호출(패치 재중첩 방지, `themeMap`·프로바이더 재구축,
  말미 `setTheme('taide')` 자동 적용). 이 절차로 몽키패치 누적과 스냅샷 미반영 문제를 함께
  해소한다.
- **`inherit: false` 대응**: `buildShikiTheme` 의 `colors` 에 기존 `MONACO_COLOR_SOURCE`
  매핑을 그대로 재사용해 채운다 — shiki 가 실제로 읽는 키는 `editor.background`/
  `editor.foreground` 2개뿐이지만(`@shikijs/primitive` `normalizeTheme` 실측), `shikiToMonaco`
  가 `defineTheme` 에 전달하는 `colors` 는 테마 객체의 `colors` 전체이므로 여기에 133개 UI
  토큰 전량을 실어 monaco 기본값으로의 회귀를 막는다.

## 7. 번들 임팩트 (npm dist 실측)

### 런타임 코어(트리셰이킹 전 상한)

| 모듈 | raw | gzip |
|---|---|---|
| `@shikijs/engine-javascript` | 4.4 KB | 1.5 KB |
| `oniguruma-to-es` + transitive | 163.4 KB | 46.0 KB |
| `@shikijs/core` | 49.0 KB | 12.1 KB |
| `@shikijs/primitive` | 28.2 KB | 7.8 KB |
| `@shikijs/vscode-textmate` | 97.7 KB | 18.7 KB |
| `@shikijs/monaco` | 5.7 KB | 2.0 KB |
| **합계** | **~348 KB** | **~88 KB** |

### Grammar 30종(TAIDE 언어 매핑, 의존 그래프 실측 합집합)

- 37개 모듈 / 2,433 KB raw / 237 KB gzip. `erb`/`ruby` 가 각각 ~2MB raw 로 최대(html·js·tsx·sql
  등 20개를 끌고 온다).
- **개별 dynamic import 로 초기 로드를 분리해야 한다** — 31종을 전부 선로드하면 237KB gzip 이
  첫 페인트 경로에 얹힌다. W7 은 "highlighter 생성 시 전량 로드, 단 개별 dynamic import(vite
  청크 분리)"로 절충한다(`docs/theme-system.md` §4.2) — 에디터 모듈 로드 후 비동기라 첫 페인트와
  비경합이라고 판단했으나, 실측(첫 페인트·타이핑 지연)은 QA6 대상이다.

## 8. 대안 비교

| 대안 | 평가 |
|---|---|
| `vscode-textmate` + `vscode-oniguruma` 직접 통합 | 정확도는 동일하나 WASM 필수 → `wasm-unsafe-eval` CSP 완화 강제. grammar 수집·번들링·테마 변환·monaco 브릿지 전부 자체 구현 필요. `vscode-oniguruma` 최신 2.0.1(2023-09) 로 사실상 정체 — **비추천** |
| `monaco-editor-textmate` + `monaco-textmate` + `onigasm` | 유지보수 중단(각 2022-09/2019-08/2020-09 최종 배포). monaco 0.56 호환 보장 없음, onigasm 도 WASM — **채택 불가** |
| `modern-monaco`(shiki 내장, shiki 공식이 "highly recommend") | 활발(0.4.2, 2026-06)하나 monaco 를 감싸는 상위 프레임워크라 이미 monaco 를 직접 세팅한 TAIDE 에 침습적 — **비추천(현 시점)** |
| **shiki 4.4.3 + `@shikijs/monaco`(채택)** | CSP 무변경, 99.6% grammar 호환, 활발한 유지보수, MIT |

## 9. 미확인·위험 (QA6 대기)

**미확인**

1. 실제 WKWebView(Tauri macOS) 런타임에서 CSP 무변경 동작 — 정적 grep 으로 eval/Function 부재는
   확인했으나 런타임 실행은 하지 않았다. **W7 구현 직후 1순위 실기 확인.**
2. shiki 토크나이저 교체 후 monaco 내장 **TS/JSON worker 의 리치 기능(진단·완성) 유지 여부**.
3. embedded language(erb=html+ruby, html 내 script/style) 토큰화 정확도.
4. 첫 페인트·타이핑 지연의 실제 체감치, 테마 에디터 프리뷰(입력마다 재적용)의 체감.
5. `shikiToMonaco` 재호출 시 몽키패치 중첩의 실제 부작용 정도(§6 절차로 회피를 시도했으나 실행
   검증은 없음).
6. `@shikijs/langs` 개별 grammar 의 upstream 라이선스 준수 상세(패키지 자체는 MIT 로 재배포하나
   grammar 마다 다른 upstream 라이선스를 승계 — `THIRD_PARTY_LICENSES.md` "Bundled TextMate
   Grammars" 절 참고).

**위험**

- 플러그인 grammar 의 ReDoS — `tokenizeTimeLimit`(기본 500ms) 에만 의존한다. 완전한 방어가
  아니며 한계로 문서화한다(`docs/features/plugins.md` §4).
- GPL-3.0 grammar 유입 방지: `@shikijs/langs` 전체에는 `ada`·`gnuplot`·`nginx`·`org`·`racket`
  이 GPL-3.0 이다. TAIDE 가 번들하는 30종에는 없지만, **배럴 import(`@shikijs/langs` 전체)를
  금지하고 개별 서브패스 import 만 허용**해야 하는 실질적 근거다.

## 출처

[shiki.style/guide/regex-engines](https://shiki.style/guide/regex-engines) ·
[shiki.style/packages/monaco](https://shiki.style/packages/monaco) ·
[shiki.style/blog/v4](https://shiki.style/blog/v4) ·
[packages/monaco/src/index.ts](https://raw.githubusercontent.com/shikijs/shiki/main/packages/monaco/src/index.ts) ·
[packages/shiki/src/bundle-full.ts](https://raw.githubusercontent.com/shikijs/shiki/main/packages/shiki/src/bundle-full.ts) ·
[docs/references/engine-js-compat.md](https://raw.githubusercontent.com/shikijs/shiki/main/docs/references/engine-js-compat.md) ·
[MDN CSP script-src](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/script-src) ·
[shikijs/shiki#918](https://github.com/shikijs/shiki/issues/918) ·
npm registry(`shiki`, `@shikijs/*`, `oniguruma-to-es`) tarball 실물 ·
로컬 `node_modules/monaco-editor@0.56.0` 소스.
