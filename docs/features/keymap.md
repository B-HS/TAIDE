# 기능 — 키맵 엔진: chord · when (Wave H)

> Wave I(F2) 추가: `toggle-zen-mode`(⌘K Z)가 `open-keybindings-editor`(⌘K ⌘S)와 같은 1단(⌘K)을
> 공유하는 두 번째 실제 chord 엔트리로 들어오면서, §1·§2 가 서술하던 "pending 은 엔트리 하나만
> 기억한다" 전제가 깨졌다. `KeymapChordPendingState.entryId`(단수) → `entryIds`(복수,
> `KeymapActionId[]`)로 바뀌었고, `findMatchingChordPrefixEntry`(단수, 첫 후보만) 옆에
> `findMatchingChordPrefixEntries`(복수, 전체 후보)가 신설되어 `decideKeymapDispatch` 가 실제로
> 이걸 쓴다 — §8 이 진작부터 서술하던 "같은 1단을 여러 chord 가 공유할 수 있다"는 지금까지는
> 충돌*판정*에만 해당하고 실제 *디스패치*는 첫 엔트리만 영구 우선했던 결함이었다(2단 pending 이
> 하나의 `entryId` 만 기억). 아래 본문(Wave H 원문)은 이 변경 이전 기준으로 "엔트리 하나"를
> 전제로 서술되어 있으니, `entryId`를 읽는 곳은 전부 `entryIds`(배열)로 치환해서 읽는다 — 자세한
> 근거는 `keymap-chord-store.ts`/`keymap-dispatch.ts`의 최신 doc comment 참고.
>
> 계약: `docs/acknowledge/2026-08-16-wave-h-keymap-contract.md`.
> ContextKeyExpr 딥임포트 선례: `docs/acknowledge/2026-08-16-monaco-contextkeyexpr-deep-import.md`.
> 핵심 파일: `shared/lib/keymap.ts`(엔트리·매칭·직렬화) · `shared/lib/keymap-dispatch.ts`(순수
> 상태머신) · `shared/lib/keymap-chord-store.ts`(전역 chord/monaco유예 스토어) ·
> `shared/lib/keymap-context.ts` + `shared/lib/keymap-when.ts`(when 평가) ·
> `shared/hooks/use-global-keymap.ts`(소비처 훅, 부작용 지점).

## 1. 엔트리 모델

`APP_KEYMAP: KeymapEntry[]`(22건 — 기존 21건 + Wave H 신설 chord 표본 1건)의 각 엔트리:

```ts
type KeymapEntry = {
    id: KeymapActionId
    key: string
    mods: KeymapModifier[]
    chord?: { key: string; mods: KeymapModifier[] } // 2단(선택) — 있으면 key/mods 는 1단(프리픽스)
    when?: string // ContextKeyExpr 문자열(선택) — 없으면 항상 매칭(구조적 보장)
    descriptionKey: string
}
```

- `chord` 는 **선택 필드**다. 구버전 파서(`isKeymapOverrideEntry` 의 얕은 가드)는 이 필드를 몰라도
  조용히 통과한다 — 데이터 소실 없이 1단만 아는 것으로 해석된다(전방호환).
- `when` 이 있는 엔트리는 기존 21건 중 **`terminal-jump-to-previous/next-command` 2건뿐**이다
  (마이그레이션 전략 1 — 보수). 나머지 19건은 `when` 이 없어 `findMatchingKeymapEntry` 가
  `isWhenSatisfied` 콜백을 **아예 호출하지 않는다**(구조적 보장, 조건문이 아니라 `entry.when ===
  undefined` 단락 평가) — 어떤 컨텍스트 게터를 넘겨도 이 19건의 행동은 절대 변하지 않는다.
- `KeymapOverrideEntry`(사용자 재바인딩 저장 포맷)도 동일한 선택 `chord?` 필드를 가진다.
  **재바인딩은 완전한 재선언**이다 — `applyKeymapOverrides` 가 오버라이드에 `chord` 가 없으면 base
  엔트리의 `chord` 를 무조건 제거한다("이전 chord 유지"가 아니라 "단일 키로 확정"으로 해석).

## 2. 디스패치 상태머신 (`decideKeymapDispatch`)

순수 함수(DOM·스토어 부작용 없음) — 매 `keydown` 마다 아래 순서로 판정한다.

1. **`monacoDeferral` 이 무장됨** → 이 keydown 은 monaco 것(수식어 단독 keydown 은 예외 —
   `ignore-modifier-only`, 그 외는 `defer-to-monaco`, 앱은 아무것도 하지 않고 이벤트를 그대로
   통과시킨다).
2. **chord `pending`** → 2단 판정. 일치하면 `resolve-chord-matched`, **불일치해도 무조건**
   `resolve-chord-no-match`(오입력 방지 — 수식어 단독 keydown 은 예외).
3. 앱이 소유한 chord 엔트리의 1단이 매칭 && `!editorTextFocus` → `enter-chord`(에디터 포커스 중엔
   앱 chord 진입 자체가 억제된다 — monaco 에 양보).
4. 에디터 포커스 && 이벤트가 monaco 전용 프리픽스(⌘K/Ctrl+K, `MONACO_CHORD_PREFIX_KEY`) →
   `observe-monaco-chord-prefix`(삼키지 않고 관찰만, monaco 유예 창 무장).
5. 그 외 — 기존과 동일한 단일 키 `when`-게이팅 매칭(`dispatch` 또는 `none`).

실제 `preventDefault`/`stopPropagation`/스토어 mutation/핸들러 호출은 전부
`use-global-keymap.ts::handleKeyDown` 이 수행한다 — `decideKeymapDispatch` 자체는 "무엇을 할지"만
answer 한다.

## 3. 멀티 리스너 팬아웃과 그 레이스 (중요 — Phase D 에서 발견·수정)

**앱에는 `useGlobalKeymap` 소비처가 동시에 여러 개(app-shell·editor-area·command-palette·
status-bar-content·터미널 패널마다 각 1개) 떠 있고, 각각 자신만의 `window` `keydown` capture
리스너를 등록한다.** 브라우저는 **같은 물리 keydown 이벤트를 이 모든 리스너에 순서대로** 전달한다
— `event.stopPropagation()` 은 다른 DOM 노드로의 전파만 막을 뿐, **같은 타겟(`window`)에 등록된
형제 리스너는 막지 못한다**(`stopImmediatePropagation()` 이어야 막힌다. 이 앱은 안 쓴다). 즉 chord
1단·monaco 프리픽스 관찰처럼 **부작용(스토어 mutation)을 일으키는 판정은 하나의 물리 keydown 마다
N 번(리스너 수만큼) 반복 실행된다.**

### 3.1 증상이었던 레이스

`armKeymapMonacoDeferral()`/`enterKeymapChordPending()` 은 스토어 상태를 **동기적으로 즉시**
바꾼다. 리스너 #1 이 이 함수를 호출하면, 같은 이벤트를 처리 중인 리스너 #2..N 이 `decideKeymapDispatch`
를 호출할 때는 **이미 바뀐(무장된) 상태**를 보게 된다 — 그 결과 리스너 #2..N 은 **지금 막 진입한
바로 그 keydown 을, "그다음" keydown(유예/대기를 해소하는 키)으로 오판**한다:

- monaco 유예: 리스너 #1 은 `observe-monaco-chord-prefix`(올바름), 리스너 #2..N 은 `defer-to-monaco`
  로 오판 → `consumeKeymapMonacoDeferral()` 이 마이크로태스크로 유예 해제를 **예약**한다. 이
  마이크로태스크는 현재 동기 디스패치가 끝나자마자(=사용자의 **진짜** 다음 물리 keydown 이 오기
  한참 전에) 플러시되어 `monacoDeferral` 을 도로 `false` 로 되돌린다. 그 결과 진짜 두 번째 keydown
  (예: ⌘B)이 도착했을 때는 이미 유예가 풀려 있어 `toggle-sidebar` 가 그대로 발동·`preventDefault`
  해버린다 — **이 wave 가 상환하려던 monaco chord 회귀 4건이 그대로 재현된다.**
- 앱 자체 chord 1단: 리스너 #1 은 `enter-chord`(올바름), 리스너 #2..N 은 방금 그 1단 keydown 을
  "2단"으로 오판해 `resolve-chord-no-match` 를 반환 → `notifyKeymapChordNoMatch()`(무매칭 인디케이터
  오발) + `resolveKeymapChordPending()` 이 pending 해제를 예약 → 마찬가지로 진짜 2단(⌘S)이 도착하기
  전에 `pending` 이 사라져 있어 **⌘K ⌘S 자체가 동작하지 않는다.**

두 시나리오 모두 `docs/acknowledge/2026-08-16-wave-h-keymap-contract.md` §5 완료조건이 요구하는
**"monaco chord 21건 실보존"** 을 직접 위반한다 — 재현은 `keymap-chord-store.ts`/`keymap-dispatch.ts`
를 그대로 호출하는 유닛 테스트로 결정적으로 재현했다(아래 3.3 의 테스트가 그 회귀 고정본).

### 3.2 수정 — 이벤트 참조 아이덴티티로 메모이즈

`getKeymapChordDispatchSnapshot(event)`(`keymap-chord-store.ts`) 를 신설했다. `use-global-keymap.ts`
는 더 이상 `getKeymapChordStoreSnapshot()`(항상-최신, UI 구독용 — `status-bar-content.tsx` 가 그대로
계속 쓴다)을 직접 부르지 않고, 이 함수에 **자신이 받은 `KeyboardEvent` 참조**를 넘긴다.

```ts
export const getKeymapChordDispatchSnapshot = (event: KeymapEvent): KeymapChordStoreState => {
    if (event !== lastDispatchEvent) {
        lastDispatchEvent = event
        lastDispatchSnapshot = state
    }
    return lastDispatchSnapshot
}
```

같은 물리 이벤트 객체로 다시 불리면(=형제 리스너가 같은 keydown 을 처리 중) **첫 호출 시점에
캐시해 둔 스냅샷을 그대로 반환**한다 — 그 사이 다른 리스너가 스토어를 mutate 했더라도 무시된다.
그 결과 N 개 리스너 전부가 **정확히 같은 판정**(`observe-monaco-chord-prefix` 전부, 또는
`enter-chord` 전부)에 도달하고, `armKeymapMonacoDeferral`/`enterKeymapChordPending` 을 N 번 불러도
멱등(타이머만 재시작, 부작용 무해)하다. **진짜 다음** keydown(별개의 `KeyboardEvent` 객체)이 오면
`event !== lastDispatchEvent` 가 되어 캐시가 새로고침되고, 그제서야 실제 최신 상태(무장/대기 중)가
보인다.

이 메모이즈는 **타이머가 아니라 이벤트 참조 동등성**에만 의존한다 — "리스너가 몇 개인지", "몇
번 불렸는지" 를 전혀 몰라도 되고, 마이크로태스크 타이밍 가정도 없다. `getKeymapChordStoreSnapshot()`
(UI 구독 경로)는 건드리지 않았다 — 상태바 chord 인디케이터가 이 메모이즈 때문에 stale 하게 보이는
일은 없다.

### 3.3 command-palette 의 두 번째 window 리스너도 이 팬아웃의 일부다

`command-palette.tsx` 는 `useGlobalKeymap`(APP_KEYMAP 디스패치) 과 **별개로** 자기만의 window
keydown capture 리스너를 하나 더 등록한다 — `runsViaCommand` 커맨드 행(`keymapId` 도 없고
`monaco.*` 도 아닌 `AppCommand`, 예: `window.reload`)은 대응하는 `APP_KEYMAP` 엔트리가 없어
`useGlobalKeymap` 이 아예 디스패치하지 못하기 때문이다. 이 리스너도 §3 이 말하는 "같은 물리
keydown 을 받는 형제 리스너" 하나이므로, chord `pending`/`monacoDeferral` 을 스스로 확인하지 않으면
2단 무조건 삼킴과 monaco 유예 창을 전부 우회해버린다 — 사용자가 `runsViaCommand` 행을 chord/유예
창과 충돌하는 키로 재바인딩하면 재현된다. `getKeymapChordDispatchSnapshot(event)` 로 같은 메모이즈
스냅샷을 읽어 `pending`/`monacoDeferral` 중 하나라도 참이면 즉시 return 한다. 또한
`findRunnableCommandBinding`(`keybinding-catalog.ts`)은 `row.chord` 가 있는 행을 매칭 대상에서
제외한다 — 이 경로는 2단 상태머신에 참여하지 않으므로, chord 로 재바인딩된 행이 1단 keydown 만으로
즉시 발동(표시는 두 단계인데 동작은 한 단계)하는 것을 막기 위해서다.

### 3.4 회귀 고정 테스트

`keymap-chord-store.test.ts` 의 `getKeymapChordDispatchSnapshot`·"멀티 리스너 팬아웃 통합" 두
`describe` 블록이 이 레이스를 결정적으로 재현·고정한다 — 4개의 가짜 리스너가 `APP_KEYMAP` +
`decideKeymapDispatch` + 실제 스토어로 같은 `KeyboardEvent` 객체를 순차 처리하는 시나리오다.

## 4. monaco chord 유예 창 (기존 회귀 4건 상환)

에디터 포커스 상태에서 ⌘K/Ctrl+K 를 누르면(§2 단계 4) 앱은 그 keydown 을 **삼키지 않고**
`armKeymapMonacoDeferral()` 만 호출한다 — monaco 자신의 domNode 리스너(window capture 보다 나중에
실행됨)가 정상적으로 이 keydown 을 받아 자기 chord 대기 상태로 들어간다. 다음 keydown(예: ⌘B) 은
§2 단계 1 에서 `defer-to-monaco` 로 판정되어 **앱은 이 keydown 에 대해 아무것도 하지 않는다**(단일
키 바인딩 포함, `toggle-sidebar`/`find`/`font-size-up`/`font-size-down` 전부 이 keydown 에 한해
비활성) — monaco 가 자신의 ⌘K ⌘B/⌘K ⌘F/⌘K ⌘=/⌘K ⌘- chord 를 그대로 처리한다.

- **타임아웃 5000ms**(`KEYMAP_CHORD_PENDING_TIMEOUT_MS`, monaco 자신의
  `abstractKeybindingService.js` chord 타임아웃과 동일 상수 — 미러링).
- **이탈**: window `blur`(OS 포커스 상실) 시 `clearKeymapChordState()` 로 pending·유예 둘 다 즉시
  해제(`use-keydown-capture.ts`). 유예만 별도로도 이탈한다 — `document` `focusin` 리스너가 유예가
  무장된 상태에서 포커스가 `editorTextFocus` 밖으로 나가면(같은 앱 안에서 에디터→사이드바/터미널
  등으로 클릭 이동, OS blur 없음) `consumeKeymapMonacoDeferral()` 을 호출한다 — 그러지 않으면 유예가
  더 이상 받을 수 없는 monaco 인스턴스를 향해 다음 keydown 1개를 계속 양보해 그 키가 조용히
  사라진다. `pending`(앱 자체 chord 대기)은 이 focusin 리스너로 건드리지 않는다 — 앱 chord 는 에디터
  포커스에 스코프되지 않으므로(예: ⌘K 대기 중 팔레트를 여는 것 자체가 포커스 이동이다) 취소하면 안
  된다.
- **수식어 단독·키 자동반복(`event.repeat`)·IME 조합 중(`event.isComposing`) keydown 은 유예·2단
  소비에서 제외**된다(`isIgnorableKeydown`, `keymap-dispatch.ts`) — Cmd 키를 누르고 있는 것,
  ⌘K 를 길게 눌러 생기는 반복 keydown, 한글/일본어 등 조합 중인 keydown 모두 "다음 keydown" 으로
  카운트되지 않는다.
- `MONACO_CHORD_PREFIX_KEY`(⌘K/Ctrl+K)는 앱이 정의한 chord 유무와 **독립적으로** 항상 추적한다 —
  앱이 자기 chord 를 하나도 안 가지고 있어도 monaco 의 21개 기본 chord 를 보호해야 하기 때문이다.
  사용자가 `monaco.*` 행을 **다른** 프리픽스의 chord 로 재바인딩하면(예: ⌘J ⌘S) 그 1단도
  `deriveMonacoChordPrefixes`(`monaco-keybinding.ts`)로 동적으로 뽑혀 `decideKeymapDispatch` 의
  `monacoChordPrefixes` 인자에 합류한다 — 그러지 않으면 그 프리픽스는 유예 대상이 아니어서, 2단이
  앱의 다른 단일 키 바인딩에 먼저 잡아먹혀 버린다.
- 상태바 chord 인디케이터는 `pending` 뿐 아니라 `monacoDeferral` 도 표시한다
  (`status-bar-content.tsx`) — monaco standalone 자신의 chord 상태 표시가 no-op 스텁이라(§ "확정
  사실" 4), 유예 창이 무장된 동안 사용자가 받는 유일한 피드백이기 때문이다.

## 5. 앱 자체 chord — ⌘K 프리픽스

에디터 포커스가 **아닌** 상태에서만 앱은 자기 chord 를 소유할 수 있다(§2 단계 3). 1단이 매칭되면
`enter-chord` → 상태바에 대기 인디케이터(`keymap.chordPending`, "( {{shortcut}} ) 다음 키 대기")가
뜬다. 2단은 **무조건 삼킨다** — 일치하면 핸들러 실행, 불일치해도(오타 등) 그냥 흡수하고
`keymap.chordNoMatch`("일치하는 단축키가 없습니다") 를 1.5초 플래시로 보여준다(오입력이 엉뚱한
단일 키 바인딩으로 새는 것을 막기 위한 설계 — 계약 §3.1).

**표본 엔트리**: `open-keybindings-editor` = `⌘K` → `⌘K ⌘S`(2단도 `mod` 필요 — 계약 §3.1 문언
"VS Code 관성" 그대로. 표시는 `formatKeymapShortcut` 이 "⌘K ⌘S" 로 렌더링한다, monaco
`defaultBindingLabel` 관례와 동일한 공백 join). 핸들러는 `requestOpenKeybindingsEditor()`
(`keybindings-bridge.ts`) — 키바인딩 에디터를 연다. `command-catalog.ts`(d-30 분할 이전 command-registry.ts) 의 `keybindings.open`
커맨드에도 `keymapId: 'open-keybindings-editor'` 가 붙어 있어(Phase D 접합) 팔레트·카탈로그에
**행이 하나로 통일**된다(붙지 않았다면 "바인딩 있는 keymap 전용 행"과 "바인딩 없는 커맨드 행"
2개로 쪼개져 보였을 것). 이 리네임으로 예전 `keybindings.open` actionId 로 저장된 오버라이드가
고아가 되는 것을 막기 위해 `parseKeymapOverrides` 가 `keybindings.open → open-keybindings-editor`
1회성 별칭 마이그레이션을 수행한다(`keymap.ts::LEGACY_KEYMAP_OVERRIDE_ACTION_ID_ALIASES`).

이 엔트리는 `when: '!terminalFocus'` 도 함께 가진다 — 그러지 않으면 터미널에 포커스가 있을 때 ⌘K
를 누르는 것만으로(macOS 터미널 관용구 "화면 지우기") chord 대기에 들어가 다음 keydown 1개를 xterm
에 전혀 전달하지 않고 삼켜버린다. `terminalFocus` 는 §6 화이트리스트 게터이므로 `findMatchingChordPrefixEntry`
가 소비하는 `isWhenSatisfied` 콜백을 그대로 통과한다.

## 6. when 컨텍스트

- **평가기 = monaco `ContextKeyExpr` 딥임포트**(`shared/lib/keymap-when.ts`,
  `monaco-editor/platform/contextkey/common/contextkey`). bun:test(DOM 없음) 환경에서도 top-level
  정적 import 로 정상 로드됨을 실측 확인(`keymap-when.test.ts`) — 동적 import 대신 정적 import 를
  택한 이유는 `docs/acknowledge/2026-08-16-monaco-contextkeyexpr-deep-import.md` 참조(when 평가가
  keydown 핸들러 안에서 **동기적으로** 끝나야 `preventDefault` 판단이 가능하기 때문).
- **소스 = pull 게터 맵**(`keymap-context.ts`, 1차 화이트리스트 2종):
  - `editorTextFocus`: `document.activeElement?.closest('.monaco-editor')`
  - `terminalFocus`: `document.activeElement?.closest('.xterm')`
  - 둘 다 **전역** 판정이다 — "어떤 monaco 인스턴스가 포커스됐는가" 가 아니라 "포커스된 요소가
    monaco/xterm 컨테이너 *안*에 있는가"만 본다. 인스턴스별 구분은 없다(§7 에서 이게 왜 중요한지
    설명).
- `when === undefined` → 항상 매칭(구조 보장). deserialize 실패(빈 문자열·문법 오류) →
  **`false`**(엔트리 비활성 — "항상 매칭"으로 폴백하지 않는다, 안전한 실패).
- **마이그레이션은 보수(전략 1)**: 기존 21건 중 `terminal-jump-to-previous/next-command` 2건만
  `when: 'terminalFocus'` 로 데이터화했다. 나머지 19건은 `when` 이 없다 — §1 의 구조적 보장 덕에
  이 19건의 매칭 로직 자체가 when 도입 전과 **바이트 단위로 동일**하다.

## 7. terminal-pane 의 `isFocused` 게이팅 — 계약 문구와 다른, 검증된 유지 결정

계약 §3.2 는 "터미널jump 2건... 핸들러 undefined 게이팅 제거" 라고 명시했지만, **Phase D 에서
코드 추적으로 재검증한 결과 이 게이팅은 제거하지 않고 유지했다.** 이유:

- `useGlobalKeymap` 은 §3 의 팬아웃 구조 그대로 **탭된 터미널 패널마다 독립적으로** 등록된다
  (`pane-node-view.tsx` 가 스플릿 리프마다 `TerminalSession`/`TerminalPane` 을 각각 마운트).
- `when: 'terminalFocus'` 게터는 §6 처럼 **전역** 판정이다 — "지금 어느 xterm 이든 포커스됐는가"
  만 알지 "이 특정 `TerminalPane` 인스턴스가 포커스됐는가" 는 구분하지 못한다.
- 만약 `isFocused ? handler : undefined` 삼항을 제거하고 모든 인스턴스가 핸들러를 무조건 등록하면
  — 터미널 패널을 스플릿으로 2개 이상 열어 둔 상태에서 `mod+ArrowUp` 을 누르면, **포커스되지 않은
  패널까지** `jumpToPreviousCommand()` 를 실행해버린다(전역 `terminalFocus` 는 참이지만, dispatch
  는 이벤트당 N 개 리스너 전부에서 독립적으로 일어나므로 각자의 `attachRef` 를 건드린다).
- 이는 계약 §5 완료조건의 **"기존 21 엔트리 행동 변화 0"** 을 직접 위반하는 회귀다(스플릿 터미널
  시나리오는 기존에 정상 동작하던 경우다) — 계약의 "게이팅 제거" 문구를 문자 그대로 따르는 것보다
  이 완료조건을 우선했다.

`when: 'terminalFocus'` 데이터 자체는 유지한다(인스펙터 표시·충돌 판정·문서 일관성 목적) — **두
게이트가 함께 있어도 기존 동작과 동일하다**(둘 다 참이어야 발동 = 로컬 `isFocused` 하나만으로
게이팅하던 기존과 관측 가능한 차이가 없다). 인스턴스 스코프의 when 컨텍스트(예: pane id 별 포커스
게터)는 이번 범위 밖으로 남겨 둔다 — 필요해지면 §6 의 게터 맵을 pane 단위로 확장해야 한다.

## 8. 충돌 판정 — when-aware + chord-aware

`findKeymapConflict`(`keymap.ts`) 는 아래 둘 다에 해당하지 **않아야** 같은 key+mods 를 충돌로
본다.

- **when 이산**: 양쪽 모두 `when` 이 있고 서로 다르면 비충돌(한쪽만 있거나 둘 다 없거나 같으면
  여전히 충돌 — "한쪽만 스코프됨"은 안전하게 충돌로 간주).
- **chord 2단 이산**: 양쪽 모두 `chord` 가 있고 2단(key+mods)이 서로 다르면 비충돌 — VS Code 의
  ⌘K 네임스페이스처럼 같은 1단 프리픽스를 여러 chord 가 공유할 수 있다. **한쪽만 chord 거나 둘 다
  chord 가 없으면 여전히 충돌**로 본다 — chord 엔트리는 `decideKeymapDispatch` §2 단계 3 에서
  **항상 먼저** 매칭되므로(단계 5 보다 우선), 같은 1단을 쓰는 비-chord 단일 키 엔트리는 영구히
  가려진다(진짜 충돌).

이 chord-aware 판정은 Phase D 에서 추가했다 — 원래 `findKeymapConflict` 는 chord 를 전혀 몰라서,
같은 프리픽스를 쓰는 두 chord(예: 미래에 ⌘K ⌘X 를 새로 추가)를 항상 오탐 충돌로 잘못 표시했다.
`keybinding-catalog.ts::findConflictingRow` 는 `KeybindingRow` 가 이미 `chord` 필드를 나르고 있어
카탈로그 쪽 변경 없이 이 수정의 혜택을 그대로 받는다.

키바인딩 에디터의 `handleChangeBinding` 도 Phase D 에서 접합했다 — 저장 전 미리보기 충돌 검사
(`findConflictingRow(rows, { ...currentRow, key, mods })`) 가 **새로 캡처한 `chord` 를 후보에
포함하지 않던** 미배선 갭을 고쳐 `{ ...currentRow, key, mods, chord }` 로 전달한다(그렇지 않으면
2단까지 입력했는데도 낡은 1단짜리 후보로 충돌을 검사해 경고 토스트가 부정확했다).

## 9. 캡처 UI · 컨텍스트 인스펙터 (`widgets/keybindings-editor`)

- 행을 클릭하면 1단 캡처 → "대기" 배지(`settings.keymapChordWaitingBadge`) + 프롬프트
  (`settings.keymapChordCapturePrompt`, `{{shortcut}}` 보간) + 즉시 확정 버튼
  (`settings.keymapChordConfirmSingle`, Enter 로도 확정) 노출. 2타째로 chord 확정, Escape 로 취소,
  블러 시 정리.
- monaco 재바인딩 불가 키 가드(`isKeyBindable`)는 1단·2단 **양쪽 모두**에 동일하게 적용한다.
- **컨텍스트 인스펙터**: `settings.keymapInspectorTitle` 아래 현재 활성 컨텍스트 키 배지
  (`settings.keymapInspectorEmpty` — 없을 때) — `DEFAULT_KEYMAP_CONTEXT_GETTERS` 를 500ms
  (`KEYMAP_CONTEXT_INSPECTOR_POLL_MS`) 폴링해 표시한다. **다이얼로그가 닫혀 있을 때만 폴링한다**
  (열려 있는 동안은 멈춘다) — Radix `Dialog` 는 기본 `modal=true` 로 포커스를 다이얼로그 안에
  가둬서, 열려 있는 동안 `document.activeElement` 가 `.monaco-editor`/`.xterm` 안에 있을 수 없다
  (포인터 이벤트도 뒤쪽 배경으로 전달되지 않는다). 그 상태에서 계속 폴링하면 인스펙터는 항상
  "활성 컨텍스트 키가 없습니다"만 보여주는 죽은 UI가 된다. 대신 닫혀 있는 동안 배경에서 계속
  추적하다가 `open` 이 `true` 로 바뀌는 순간 이펙트 클린업으로 폴링을 멈춰, **다이얼로그를 열기
  직전에 실제로 포커스돼 있던 컨텍스트를 그대로 얼려 보여준다** — "지금 이 순간"은 아니지만
  "방금까지 뭘 하고 있었는지"는 여전히 유효한 when 디버깅 신호다. 값이 안 바뀌면 이전 배열
  참조를 그대로 유지해(`setActiveContextKeys` 콜백 내 얕은 비교) 매 폴링마다 카탈로그 전체가
  재렌더되는 것도 함께 막는다.

## 10. 커스텀 `taide.*` monaco 액션 카탈로그

`monaco-actions.ts::TAIDE_CUSTOM_ACTIONS`(7건 — saveFile·toggleMinimap·aiInlineEdit·
gitStageSelection·toggleBlame·openFileHistory·runSelectedTextInTerminal)가 `MONACO_ACTIONS` 에
병합돼 있다 — `addAction` 으로만 등록되던 커스텀 monaco 액션이 이제 키바인딩 에디터·팔레트·재바인딩·
충돌 탐지 대상이 된다.

**주의(검토 단계에서 확정)**: `editor.addAction` 은 그 액션의 `id` 를 **진짜 전역 커맨드로 등록하지
않는다** — monaco 는 `${editor.getId()}:${id}` 라는 에디터-인스턴스별 "unique id" 로 커맨드를
만들고(`standaloneCodeEditor.js::addAction`), `InternalEditorAction.id` 도 그 unique id 다. 이 사실이
두 곳에서 문제가 됐다:

1. **팔레트 게이팅** — `editor-area.tsx` 가 `editor.getSupportedActions().map(a => a.id)` 로
   `activeEditorActionIds` 를 채우던 방식은 그 unique id 를 그대로 담아, `monaco-action-commands.ts`
   의 `isEnabled`(원본 id 로 조회)가 항상 실패해 7행 전부가 팔레트에서 영구 비활성(회색)이었다.
   `editor-area.tsx` 가 이제 `editor.getId()` 접두사를 벗겨 원본 id 로 정규화한다.
2. **재바인딩** — `addKeybindingRules` 로 만든 오버라이드 규칙(`command: 'taide.toggleBlame'`)은
   CommandsRegistry 에 존재하지 않는 원본 id 를 가리켜, 키를 삼키기만 하고 아무 동작도 하지
   않았다. `monaco-action-commands.ts::registerTaideCustomActionCommands()` 가 부트스트랩 시점에
   각 `taide.*` 원본 id 로 `monaco.editor.registerCommand`(공식 API — `lsp/command-relay.ts` 와 같은
   패턴)를 호출해 실제 전역 커맨드를 만든다. 핸들러는 포커스된 에디터에 그 액션을 `trigger`(팔레트
   실행과 동일 경로 — `requestEditorPaneCommand`).
3. **잔여 한계**: `taide.aiInlineEdit` 의 기본 `⌘I` 는 `addAction` 의 dynamic keybinding 이라
   `isDefault: false` 로 등록된다 — monaco 의 unbind 규칙(`-command`)은 `isDefault: true` 항목만
   제거하므로, 이 행을 재바인딩해도 `⌘I` 자체는 계속 살아 있다(새 키가 **추가**로 동작하지, 기존
   키를 대체하지는 못한다). 다른 6건은 기본 바인딩이 없어 해당 없음.

**알려진 부작용(결정 보류)**: `git.toggleBlame`/`git.openFileHistory`/`ai.inlineEdit` 는
`entities/*/*.commands.ts` 에도 손으로 등록된 `AppCommand` 가 이미 있어, 팔레트에 같은 동작이 2행
보인다 — 위 수정 이전에는 `monaco.taide.*` 쪽이 항상 비활성이라 사실상 무해했지만, 이제는 **양쪽
다 실행 가능한 진짜 기능 중복**이다. 어느 쪽을 남길지(손등록 `AppCommand` 제거 vs `keymapId` 접합—
`keybindings.open` 이 쓴 방식)는 제품 판단이 필요해 이번 수정 범위에서는 보류했다.

### 10.1 monaco 소스 행의 chord 재바인딩 — 인코딩 지원 (Phase D 접합)

`monaco.editor.addKeybindingRules` 의 `keybinding: number` 는 monaco 자신의 `decodeKeybinding`
언패킹 규칙(`vs/base/common/keybindings.js`)을 따라 **1단을 하위 16비트, 2단을 상위 16비트로
시프트**해 인코딩하면 2단 chord 를 표현할 수 있다(`(first & 0xffff) | (second << 16)`). Phase D
이전에는 `monaco-keybinding.ts::buildMonacoKeybindingRuleGroup` 이 `override.chord` 를 완전히
무시해, monaco 소스 행에 chord 를 캡처해 저장하면 카탈로그·팔레트 표시(`formatKeymapShortcut`)는
정확히 두 단계를 보여주는데 **실제 monaco 에는 1단만 단일 바인딩으로 적용되고 2단은 전혀 동작하지
않는** 표시-동작 불일치가 있었다. `buildMonacoChordKeybinding(firstStage, secondStage)` 를 신설해
이 인코딩을 지원하도록 고쳤다 — held-modifier 위젯-탐색 동반 규칙(`buildHeldModifierNavRules`)은
chord 오버라이드에는 만들지 않는다(그 스킴은 "쥔 채로 화살표" 라는 동시-입력 개념이라, 트리거 자체가
2키 *순차* chord 가 되면 대응 개념이 없다).

## 11. 남은 갭 (의도적 보류, 소유 밖 표시)

- `KeybindingRow` 에 `when` 필드가 없다 — `findConflictingRow` 가 when-aware 판정의 이점을 못
  받는다. 현재는 실질적 영향 0(같은 key+mods 를 공유하면서 서로 다른 `when` 을 가진 행 쌍이
  카탈로그에 없다 — terminal-jump 2건은 키 자체가 다르다). 인스펙터 완성 이후 필요해지면 §8 과
  같은 패턴(3개 매핑 지점에 `when` 전달)으로 확장.
- `taide.*` 커스텀 액션과 손으로 등록된 `AppCommand` 의 중복 행(§10) — 통합 여부 미결정.
- `taide.runSelectedTextInTerminal`(에디터 컨텍스트 메뉴) 과 `terminal.runSelectedText`
  (`AppCommand`) 가 같은 기능의 서로 다른 구현 경로로 병존 — 회귀는 아니나 통합 검토 권장.
