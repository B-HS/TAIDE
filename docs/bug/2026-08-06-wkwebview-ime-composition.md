# WKWebView 가 웹 표준 조합(composition) 이벤트를 발생시키지 않는다 (2026-08-06)

> 증상: 내장 터미널에서 한글 입력이 깨진다. "환" 을 치면 "ㅎ" 만 입력되거나 자모가 낱개로 찍힌다.

## 대상 파일

- `src/features/terminal/terminal-view.tsx` — 어댑터 배선
- `src/shared/lib/ime-input.ts` · `ime-input.test.ts` — 변환 규칙
- `src/shared/styles/global.css` — xterm 헬퍼 textarea 위치 보정

## 근본 원인 (실측으로 확정)

**Tauri 2 의 macOS WKWebView 가 `compositionstart` / `compositionupdate` / `compositionend` 를
전혀 발생시키지 않는다.** IME 결과는 `input` 이벤트의 `insertReplacementText` 로 전달된다.

같은 xterm 페이지를 두 환경에서 돌려 이벤트를 비교해 확정했다.

| | Safari 22 (정상) | Tauri 앱 WKWebView (고장) |
|---|---|---|
| `compositionstart/update/end` | 발생 | **전혀 없음** |
| `KeyboardEvent.isComposing` | `true` | **`false`** |
| `input.inputType` | `insertCompositionText` | **`insertReplacementText`** |
| `keydown.keyCode` | 229 | 229 (동일) |
| xterm `onData` | `"환"` 1회 | `"ㅎ"` 1회뿐 |
| 이벤트 순서 | keydown → input | **input → keydown** (역순) |

- **앱 전역 문제다.** `document` 에 캡처 리스너를 달고 커맨드 팔레트에 한글을 입력해도
  `compositionstart` 가 하나도 발생하지 않는다.
- 커맨드 팔레트·Monaco 가 멀쩡해 보이는 이유는 **그들이 요소의 `value` 를 그대로 렌더**하기 때문이다.
  Monaco 는 textarea 값을 diff 해 입력을 계산하므로 조합 이벤트가 없어도 동작한다.
  **xterm 만 조합 이벤트에 의존**해서 깨진다.
- xterm 쪽 정확한 지점: `_inputEvent` 가 `inputType === 'insertText'` 만 처리한다
  (`node_modules/@xterm/xterm/src/browser/CoreBrowserTerminal.ts:1196`).
  그래서 첫 `insertText "ㅎ"` 만 pty 로 가고 이후 `insertReplacementText` 로 오는
  `호`→`화`→`환` 은 전부 버려진다.

## 상위 이슈

- xterm.js [#5887](https://github.com/xtermjs/xterm.js/issues/5887) — macOS WKWebView(Tauri 2)에서
  IME 가 모든 키에 `keyCode=229` 를 보고하면서 조합 이벤트는 발생하지 않는 케이스. **open, 미수정.**
  제보자도 `terminal._core._inputEvent` 몽키패치로 우회 중이라고만 적었고 코드는 비공개.
- xterm.js [#5894](https://github.com/xtermjs/xterm.js/issues/5894) — WKWebView dead-key 이슈(별건).

## 해결 (어댑터)

**근본 수정이 아니다.** 근본 원인은 WKWebView 이고 우리 코드 밖이다.
Monaco 가 같은 조건에서 멀쩡한 방식(요소 값 직접 해석)을 xterm 입력 경로에 적용했다.

`resolveImeInput(inputType, data, composing)` 규칙:

| inputType | 동작 |
|-----------|------|
| `insertText` | xterm 이 이미 전송하므로 **출력 없이 조합 상태만 기록** |
| `insertReplacementText` | 이전 조합 길이만큼 `\x7f` + 새 문자열 전송, 조합 상태 갱신 |
| 그 외 | 조합 상태 초기화 |

`환` 입력 시 `ㅎ` → `\x7f호` → `\x7f화` → `\x7f환` 순으로 나가 화면상 `환` 으로 수렴한다.

### 알려진 한계

- 조합 중 중간 글자가 실제로 pty 에 입력됐다 지워진다. 셸 라인 편집·vim 모두 백스페이스가
  문자 삭제로 동작하므로 결과는 맞지만, 원시 키를 기대하는 전체화면 앱에서는 이상 동작 가능.
- 조합 문자열 전체를 교체하는 IME(한국어·일본어·중국어) 기준으로 만들었다. 다른 교체 범위를
  쓰는 IME 는 미검증.

## 시도했으나 원인이 아니었던 것 (재시도 금지)

| 시도 | 결과 |
|------|------|
| Rust reader 타이머 flush (`OutputBatch` 5ms 틱) | 잔상은 해결. 한글과 무관 |
| `pty_write` 등에서 전역 `begin_mutation` 제거 | 지연 개선. 한글과 무관 |
| `attachCustomKeyEventHandler` 로 조합 중 키 차단 | **가설 자체가 틀림.** Safari 로그가 `keyCode: 229` 를 보여줘 "WebKit 은 229 를 보고하지 않는다"는 전제가 반증됨. 제거함 |
| `.xterm-helper-textarea` 를 1×1 로 키우기 | 조합 이벤트는 여전히 미발생. **조합 버그의 해결책이 아님.** 다만 IME 후보창이 `left:-9999em` 로 화면 밖에 뜨는 것을 막는 실익이 있어 유지 |
| tao 가 IME 를 전역 가로챈다는 가설 | `input`→`keydown` 역순을 근거로 세웠으나, 어댑터로 해결된 것으로 보아 웹뷰 레벨 동작임 |

## 후속

- Tauri/wry/tao 상위 버전에서 수정되면 이 어댑터를 걷어낸다. 걷어낼 때는
  `resolveImeInput` 배선만 제거하면 되도록 격리해 두었다.
- 검증 방법: 터미널에 "환" 입력 → 화면에 `환` 이 한 글자로 표시되면 정상.
