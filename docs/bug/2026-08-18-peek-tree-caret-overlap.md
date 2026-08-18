# Peek Definitions 우측 파일 트리 — chevron 이 파일명을 침범 (수정)

> 손 QA 1차 발견 6건 중 항목 5(사용자 실기 보고, 2026-08-18). 진단
> `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md` §1 항목 5·§2.5. 스크린샷:
> `docs/bug/assets/2026-08-18-peek-tree-caret-overlap.png`.

## 증상

Peek Definitions(⌥클릭 정의로 이동, "N개 정의" 패널) 오른쪽의 파일 트리에서, 각 행 앞의 펼침/접힘
chevron(caret) 글리프가 파일명 텍스트와 거의 붙어 겹쳐 보인다. 스크린샷에서 `header.tsx`·`index.ts`
행 모두 caret 과 파일명 사이 여백이 정상 monaco 트리(익스플로러 사이드바 등)보다 눈에 띄게 좁다.

## 원인

monaco 의 트리 렌더러(`.monaco-tl-twistie`, upstream `tree.css`)는 다음 값으로 caret 컬럼의 폭을
계산한다.

```css
.monaco-tl-twistie {
    font-size: 10px;
    text-align: right;
    padding-right: 6px;
    flex-shrink: 0;
    width: 16px;
}
```

`padding-left`(행 컨테이너에서 넘어오는 8px) + `width: 16px` + `padding-right: 6px` = **30px** 를
전제로 하는 값들이며, monaco CSS 는 이 요소의 `box-sizing` 을 명시적으로 선언하지 않는다 —
즉 브라우저 기본값인 `content-box`(padding 이 width 에 더해짐)를 전제로 30px 폭이 나오도록
설계돼 있다.

TAIDE 는 Tailwind preflight 를 쓰는데, preflight 의 전역 리셋이 `*, ::before, ::after { box-sizing:
border-box }` 를 모든 요소에 적용한다. 이 규칙이 `.monaco-tl-twistie` 에도 그대로 걸려
`box-sizing: border-box` 로 강제되면, `padding-right: 6px` 가 `width: 16px` **안에 포함**돼 실제
content 영역이 16px 가 아니라 `16 - 6 = 10px`, 거기서 `text-align: right` 로 우측 정렬되는 16px
codicon 글리프가 그 좁은 영역 밖으로 밀려나며 파일명 쪽을 침범한다(padding-left 8px 가 별도로 더
붙는 행 레벨 여백과 별개로, twistie 자체의 우측 여백이 붕괴하는 문제).

TAIDE 측에서 monaco 트리 CSS 를 오버라이드하는 기존 규칙은 0건이었다 — preflight 의 광역 리셋이
upstream 이 전제한 content-box 를 깬 것이 유일한 원인이다.

## 수정

`src/shared/styles/global.css` 의 언레이어드(Tailwind preflight 보다 우선 적용되는) 섹션에 한 줄만
추가해 upstream 전제를 복원한다.

```css
.monaco-tl-twistie {
    box-sizing: content-box;
}
```

전면적으로 `.monaco-editor *`(또는 유사 범위)에 `content-box` 예외를 거는 방식은 검토 후
기각했다 — monaco 자신도 일부 요소에 `box-sizing: border-box` 를 명시적으로 선언하는 규칙들이
있어(예: 코드 렌더 영역), 광역 예외를 걸면 그 규칙들과 경쟁해 다른 회귀를 만들 위험이 있다.
`.monaco-tl-twistie` 하나만 정확히 겨냥하는 것이 근본 수정이자 최소 diff다.

## 잔여 관찰 (이번 수정 범위 밖)

`.monaco-icon-label::before`(파일 아이콘 글리프)등, 마찬가지로 `box-sizing` 을 명시하지 않고
content-box 를 전제하는 다른 monaco CSS 규칙들도 이론상 preflight 의 영향을 받을 수 있는 동류
패턴이다. 이번 세션에서는 실제 UI 침범이 스크린샷으로 확인된 `.monaco-tl-twistie` 만 수정했고,
다른 규칙들은 실기로 겹침이 관찰되면 같은 방식(정확한 selector + `box-sizing: content-box` 1줄)
으로 개별 대응한다.

## 대상 파일

- `src/shared/styles/global.css` — 수정 (`.monaco-tl-twistie { box-sizing: content-box }` 1규칙)
