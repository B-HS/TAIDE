# Radix asChild 트리거에 프로젝트 FC 를 자식으로 두면 이벤트가 소실된다

## 무엇을 지적받았나

탭 우클릭 시 커스텀 `TabContextMenu` 가 뜨지 않고 WKWebView 네이티브 메뉴(Reload·Inspect Element 등)가 노출됐다.
`src/widgets/editor-area/sortable-tab.tsx` 에서 `ContextMenuTrigger asChild` 의 자식으로 함수형 컴포넌트
`TabItem` 을 직접 넘기고 있었다.

## 왜 틀렸나

Radix 의 `asChild` 는 트리거가 렌더할 `Primitive.span` 을 `Slot` 으로 바꿔, `onContextMenu`(내부에서
`event.preventDefault()` 수행)·`ref`·`data-state` 등을 `cloneElement` 로 **자식 엘리먼트에** 병합한다.
이 병합은 자식이 **실제 DOM 엘리먼트**일 때만 유효하다. 자식이 `TabItem` 같은 함수형 컴포넌트면 이 props 는
`TabItem` 함수의 인자로 전달됐다가 `TabItem` 이 rest props 를 받거나 스프레드하지 않으므로 그대로 버려진다.
결과적으로 `onContextMenu` 가 실제 DOM 에 붙지 않아 `preventDefault()` 가 실행되지 않고, 브라우저 기본 동작
(네이티브 컨텍스트 메뉴)이 그대로 노출된다. TypeScript 로도 잡히지 않는다 — `ContextMenuTrigger` 의 `children`
타입이 `ReactNode` 라 자식이 DOM 엘리먼트인지 컴파일 타임에 검증하지 않기 때문이다.

동일 프로젝트의 `sortable-project-icon.tsx`, `file-tree.tsx` 는 `asChild` 의 자식으로 실제 `<div>` 를 두어
정상 동작하고 있었다 — 탭만 유일하게 FC 를 자식으로 둔 조합이었다.

## 어떻게 고치나

`asChild` 를 쓰는 트리거의 자식은 항상 실제 DOM 엘리먼트여야 한다. 프로젝트 FC 를 감싸야 한다면, 그 FC 를
DOM 엘리먼트(예: dnd-kit 의 `ref`/`listeners` 가 이미 붙어 있는 `div`) **안쪽에** 두고, 트리거가 그 DOM
엘리먼트를 감싸도록 중첩 순서를 잡는다.

```tsx
// X — 트리거 asChild 의 자식이 FC
<ContextMenuTrigger asChild>
    <TabItem ... />
</ContextMenuTrigger>

// O — 트리거 asChild 의 자식이 실제 DOM 엘리먼트, FC 는 그 내부
<ContextMenuTrigger asChild>
    <div ref={setNodeRef} {...listeners}>
        <TabItem ... />
    </div>
</ContextMenuTrigger>
```

`ref`/`style` 은 Radix 의 `composeRefs`/`composeEventHandlers` 가 기존 값과 병합하므로, dnd-kit 같은 다른
라이브러리가 이미 그 DOM 엘리먼트에 걸어 둔 `ref`·이벤트 핸들러·style 은 그대로 유지된다.

## 언제 적용하나

- `ContextMenuTrigger`/`TooltipTrigger`/`DialogTrigger`/`PopoverTrigger` 등 Radix 의 `asChild` 를 쓰는 모든
  트리거를 새로 작성하거나 리뷰할 때 — 자식이 실제 DOM 엘리먼트인지(또는 `forwardRef` 로 DOM ref 를 그대로
  전달하는 컴포넌트인지) 반드시 확인한다.
- 전수 점검 커맨드: `grep -rn "asChild" src/ -A2 | grep -B1 "<[A-Z]"` — 매치된 것 중 자식이 대문자로 시작하는
  프로젝트 컴포넌트면 DOM 전달 여부를 개별 확인한다.
- 컴포넌트 테스트가 없어 이런 회귀는 기계 검증으로 잡히지 않는다. 우클릭 시 메뉴 항목이 렌더되는지 확인하는
  Testing Library 테스트를 해당 컴포넌트에 추가하는 것을 권장한다(도입 여부는 사용자 결정).
