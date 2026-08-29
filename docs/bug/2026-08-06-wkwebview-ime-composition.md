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

## 2026-08-07 추가 — 빠른 입력 시 글자 씹힘 (교체 범위 desync)

증상: Claude Code 터미널에서 한글을 빠르게 치면 글자가 유실된다 (iTerm 은 정상 — 어댑터 경로가 없으므로).

원인 가설(코드 분석으로 특정): 어댑터가 `insertReplacementText` 를 항상 "직전 조합 문자열 전체 교체"로
가정하고 `composing.length` 만큼 `\x7f` 를 보냈다. 빠른 입력에서 음절 분리·재조합(`간` → `가` 확정 + `나` 새 조합)이
일어나면 실제 교체 범위는 조합의 꼬리 일부뿐이라, 백스페이스 수가 어긋나 확정 글자를 지우거나 덜 지운다.

수정: `beforeinput` 의 `getTargetRanges()` 로 **실제 교체 범위 길이**를 받아 그만큼만 지운다.
`resolveImeInput(inputType, data, composing, replaceLength)` — 범위가 없으면(null) 기존 전체 교체 폴백.
조합 상태는 `composing` 의 꼬리 `eraseCount` 만큼을 `data` 로 치환해 부분 교체를 정확히 미러링한다.
분리 시나리오 테스트(`가나` 수렴) 포함 10건.

검증 방법: 내장 터미널(Claude Code 포함)에서 한글 문장을 최대 속도로 입력 → 유실·중복 없이 표시되면 정상.
재현되면 `beforeinput` 이벤트의 inputType/data/getTargetRanges 를 로깅해 실측 시퀀스를 확보한 뒤 재분석한다.

## 2026-08-08 확정 — 실기 로그로 판명된 진짜 원인 (insertText 간헐 미전송)

IME 계측(링버퍼 + 팔레트 복사)으로 사용자 실기 로그를 확보해 분석한 결과:

1. **getTargetRanges 는 WKWebView 에서 항상 빈 배열** — 위의 교체 범위 가설은 무효였다.
   단, 폴백(조합 전체 교체)은 로그 전 구간에서 정확했다. 교체 이벤트는 결백.
2. **확정 원인: 음절 시작 `insertText` 를 xterm 이 간헐적으로 pty 에 보내지 않는다.**
   로그에서 `니`(70661ms)·`ㅆ`(72025)·`ㅇ`(73252·77522)은 `beforeinput→input` 만 있고 xterm 의
   `onData` 발화가 없다. 어댑터는 "insertText 는 xterm 이 보낸다"를 전제로 출력을 생략했으므로
   그 글자는 전송 자체가 누락되고, 직후 교체 이벤트의 `\x7f` 가 앞의 확정 글자까지 지운다.
   WKWebView 의 뒤집힌 이벤트 순서(input↔keydown)로 xterm 내부 중복 방지 플래그가 어긋나는 것으로
   추정되며, 간헐성과 정확히 일치한다.

해결: `createInsertTextDeduper` — 어댑터가 insertText 전송을 직접 책임진다.
- xterm 이 직전 `IME_DUPLICATE_WINDOW_MS`(50ms) 내에 동일 data 를 이미 보냈으면(1회 소비) 중복 전송 안 함
- 안 보냈으면 자체 전송하고, 늦게 도착하는 xterm 중복은 1회만 억제(다른 데이터가 지나가면 억제 해제)
- 실기 로그의 "아니" 씹힘 시퀀스를 그대로 재현하는 테스트 포함 (총 16건)

계측(링버퍼·팔레트 커맨드)은 재발 검증용으로 유지한다. 로그의 inputType 에 `:self-send`/`:already-sent`,
data 행에 `forward`/`drop` 이 표기되어 판정 경로를 바로 확인할 수 있다.

## 2026-08-29 후속 — 터미널 밖의 2차 피해 (isComposing 가드 전면 사망)

위 실측표의 `KeyboardEvent.isComposing = false` 는 xterm 만의 문제가 아니었다. **앱 전체에서
`event.isComposing` 으로 쓴 조합 가드가 전부 죽어 있었다**(전수조사 §4-B B13). 같은 표의
`keyCode = 229` 는 Safari 와 동일하게 살아 있으므로, 공통 가드
`isImeCompositionKeydown`(`src/shared/lib/ime-composition.ts`)을 `isComposing || keyCode === 229`
로 정의해 한 곳에서 판정한다(cmdk 가 자기 `Command` 루트에 쓰는 것과 같은 식).

적용 지점 — `keymap-dispatch.ts`(단일 키 매칭·chord 1단/2단·monaco 유예 전부),
`ai-inline-edit.ts`(기존 `isComposing` 가드 교체), 검색 패널·Search Editor 의 Enter 실행,
탐색기 draft row 의 Enter 확정·Escape 취소, 커밋 박스 ⌘Enter, 원격 허용 호스트 Enter 추가,
Zen 모드 Escape 이탈, `DialogContent`·`PopoverContent` 의 Escape 닫힘(radix 는 자체 조합 가드가
없다 — `@radix-ui/react-use-escape-keydown` 은 `event.key === 'Escape'` 만 본다).

검증 방법: 조합 중(예: `ㅎ`→`호`→`화`) 확정용 Enter 로 검색이 실행되거나 파일 생성이 확정되지
않고, 조합 취소 Escape 로 팔레트·다이얼로그가 닫히지 않으면 정상. 확정 뒤에 다시 누른 Enter/Escape
는 정상 동작해야 한다(가드가 조합 상태에서만 걸린다).

**범위 정정 (2026-08-29 검토)** — 키맵 디스패치(`decideKeymapDispatch`)의 가드는 **수식키 없는
keydown 에만** 적용한다(`isImeCompositionKeydownWithoutCommandModifier`). 입력기는 Cmd/Ctrl 조합
keydown 을 소비하지 않으므로 조합 중이라도 ⌘S·⌘W·⌘P 는 그대로 실행돼야 하고, 반대로 229 를 수식
조합에도 실어 보내는 환경이 있다면 예외가 없을 때 조합 중 **모든 앱 단축키가 무음으로 죽는다**.
Option/Alt 는 예외가 아니다(macOS 는 Option 으로 dead key 를 조합한다). 위 목록의 나머지 적용
지점(Enter/Escape 축)은 수식키가 없으므로 판정이 그대로다.

추가 검증 항목: 조합 중(확정 전)에 **⌘S** 를 눌러 저장이 실행되는지 — 실행되면 정상.

## 후속

- Tauri/wry/tao 상위 버전에서 수정되면 이 어댑터를 걷어낸다. 걷어낼 때는
  `resolveImeInput` 배선만 제거하면 되도록 격리해 두었다.
  `isImeCompositionKeydown` 은 그때도 남긴다 — `isComposing` 이 되살아나도 그 판정을 먼저 보고,
  `keyCode` 는 폴백으로 물러날 뿐이다.
- 검증 방법: 터미널에 "환" 입력 → 화면에 `환` 이 한 글자로 표시되면 정상.
