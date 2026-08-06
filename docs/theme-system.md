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
    "syntax": {                        // 구문 토큰 → Monaco 토큰 룰로 변환 (아래 §4)
        "keyword": { "fg": "#c678dd", "bold": false },
        "string": { "fg": "#98c379" }
    },
    "terminal": {                      // ANSI 16 + 커서·선택 (아래 §5)
        "black": "#...", "red": "#...", "brightBlack": "#..."
    }
}
```

- `$key` 는 palette 참조. colors/syntax/terminal 은 palette 를 참조하거나 직접 hex 를 쓸 수 있다.
- 내장 테마(dark/light)는 앱 리소스로 포함하고, 사용자 테마는 `themes/` 에 두면 목록에 나타난다.
- 사용자 테마는 내장 테마를 `extends` 로 상속해 일부 토큰만 오버라이드할 수 있다
  (누락 토큰은 base 에서 채움 — 전체 나열 강제 금지).

## 3. 시맨틱 토큰 (colors) — 영역별 세분화

VSCode 의 color ID 체계를 참조해 다음 네임스페이스로 나눈다. 초기 정의 시 각 네임스페이스의
토큰을 전부 명세하고, 구현 중 임의 토큰 추가 대신 이 문서를 갱신한다.

| 네임스페이스 | 대상 토큰 예 |
|--------------|-------------|
| `app.*` | background, foreground, border, focusBorder, shadow, accent |
| `appSidebar.*` | background, itemHover, itemActive, iconDefault, iconAgentRunning, badge |
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

### 4.2 Monaco

- `syntax` 절을 Monaco `defineTheme` 의 토큰 룰(scope→색)로 변환하고, `editor.*`/`diff.*` 토큰을
  Monaco `colors` 맵으로 변환하는 **단일 변환 함수**를 둔다. 테마 전환 시 재정의·재적용.
- 구문 토큰 최소 집합: keyword, storage, operator, string, number, regexp, comment, docComment,
  function, method, variable, parameter, property, type, class, interface, enum, constant,
  namespace, decorator, tag(HTML/JSX), attribute, punctuation, invalid, link,
  markdown(heading, emphasis, strong, code, quote, listMarker).
- semantic token(LSP semantic highlighting) 매핑도 같은 집합에서 파생한다.

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

### 7.4 윈도우 배경색도 테마를 따라야 한다 (사용자 지적 17번)

§4.1 의 FOUC 방지가 `tauri.conf.json` 의 **정적** `backgroundColor` 에 의존한다.
테마 전환 시 이 값이 갱신되지 않으면 타이틀바 주변이 이전 테마 색으로 남는다.
→ 테마 적용 흐름(§5)에 **윈도우 배경색 갱신**을 포함한다. 상세는 `window-chrome.md` §1.2.
