# 커맨드 팔레트 열림 시 ">" 프리픽스 전체선택 — 첫 타이핑이 프리픽스를 삭제 (수정)

> 사용자 실기 보고(2026-08-29): "⌘⇧P 를 누르면 '>' 가 selected 되어 있어 방향키를 한 번 눌러야
> 안 지워진다". 구현·검토는 wf `wf_a068d4a0` 팔레트 트랙, 결정 기록은
> `docs/acknowledge/2026-08-30-usability-batch-decisions.md`, 기능 정본은
> `docs/features/command-palette.md` §5.

## 증상

⌘⇧P(커맨드 모드)로 팔레트를 열면 입력창의 ">" 프리픽스가 전체선택된 상태로 시작한다. 이 상태에서
바로 타이핑하면 ">" 가 지워져 팔레트가 파일 검색 모드로 떨어지고, 사용자는 방향키로 선택을 해제한
뒤에야 커맨드를 입력할 수 있다. "@"(심볼) 등 프리픽스가 있는 모든 진입 경로에서 동일하다.

## 원인 (둘이 겹침 — 소스로 확인)

1. **Radix `FocusScope` 의 명시적 `select()`**: Dialog 마운트 시 첫 tabbable 후보를 자체 헬퍼
   `focus(element, { select: true })` 로 포커스하는데, 이 헬퍼는 `element.focus()` 직후
   `isSelectableInput(element) && select` 이면 `element.select()` 를 호출한다
   (`@radix-ui/react-focus-scope/dist/index.mjs` 187~192행). 즉 매 오픈마다 전체선택이 걸린다.
2. **macOS WKWebView 의 포커스 동작**: 프로그램적 포커스 시 텍스트 컨트롤의 cached selection
   (값 전체)을 복원해 같은 상태를 만든다 — Radix 를 우회해도 단독으로 재현 가능.

cmdk 는 selection 을 건드리지 않는다(`select()`/`setSelectionRange` 호출 0건) —
`shared/ui/command.tsx` 는 원인이 아니다.

## 해결

캐럿을 값 끝에 두고 선택을 없앤다. 경로 2개:

- `DialogContent.onOpenAutoFocus` 에서 `event.preventDefault()` 로 Radix 자동포커스를 **대체**하고
  `focusTextInputCaretAtEnd(inputRef.current)` 호출 — `focus({ preventScroll: true })` 후
  `setSelectionRange(len, len)`. (`src/shared/lib/text-input-caret.ts` 신설, 같은 패턴 선례:
  `features/theme/color-picker.tsx` 의 onOpenAutoFocus + preventDefault)
- 닫힘 애니메이션(약 200ms) 중 재열림은 `FocusScope` 가 재마운트되지 않아 onOpenAutoFocus 가
  발화하지 않는다 — `command-palette.tsx` 의 `[open]` 이펙트가 같은 처리를 반복해 커버.

## 검증

- `src/shared/lib/text-input-caret.test.ts` 5건 — 프리픽스/빈 쿼리/기존 쿼리의 캐럿 위치,
  focus→setSelectionRange 호출 순서, null 입력 무해성.
- DOM 포커스 실동작은 테스트 인프라 부재로 수동 QA: 팔레트 열기 직후 바로 타이핑해 ">" 가
  유지되는지, ⌘P·⌘⇧P·⌘T 세 진입 경로와 닫힘 중 재열림에서 각각 확인.
