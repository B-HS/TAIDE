# 툴팁(Tooltip) 사용 규범

> QA6 후속(B3, 2026-08-12) 감사에서 side 값이 파일마다 산발적으로 지정되던 문제를 정리했다.
> `shared/ui/tooltip.tsx`(radix 래핑) · `shared/ui/icon-button.tsx`(aria-label + Tooltip 강제) 를 쓰는
> 모든 신규 코드는 아래 규범을 따른다.

## 1. side 규범 — 위치별 고정값

| 위치 | side | 근거 / 예시 |
|------|------|------|
| 앱 사이드바(세로 아이콘 레일 — `widgets/app-sidebar`) | `right` | 화면 왼쪽 끝에 고정된 세로 바 — 툴팁이 오른쪽으로 펼쳐져야 콘텐츠를 가리지 않는다. `sortable-project-icon.tsx`, `app-sidebar.tsx` |
| 툴바 · 탭(가로로 나열된 아이콘 버튼 행 — 패널 헤더, 탭 바, 프리뷰 툴바 등) | `bottom` | `file-tree-toolbar.tsx`, `tab-item.tsx`, `search-panel.tsx`, `git-panel.tsx`, 테마 에디터 각 행 등 대다수의 아이콘 버튼이 여기 해당한다. |
| 상태바(화면 최하단 바 — `features/window/status-bar.tsx`) | `top` | 상태바는 화면 맨 아래이므로 툴팁이 위로 펼쳐져야 한다. `font-size-stepper.tsx` 가 이미 `top` 을 쓰고 있었다. |

- 위 세 범주에 속하지 않는 위치(패널 안쪽 리스트 행, 팝오버 트리거 등)는 **`bottom`** 을 기본값으로 한다.
  `shared/ui/icon-button.tsx` 의 `side` 기본값도 `'bottom'` 이다.
- 새 위치가 위 세 범주 중 하나에 해당하면 표를 갱신한다. 애매하면 이 문서에 사례를 추가하고
  진행한다 — 사용자에게 다시 묻지 않는다(원칙: 명백한 유사 사례가 있으면 그 규범을 따른다).

## 2. 공용 컴포넌트

- **아이콘 전용 버튼**은 `shared/ui/icon-button.tsx` 의 `IconButton` 을 쓴다. `label`(prop 으로 전달되는
  렌더된 문자열 — `shared/ui` 는 i18n 에 의존하지 않는다) + `icon` + `side` 로 aria-label 과
  Tooltip 을 동시에 강제한다.
- **아이콘이 아닌 트리거**(shadcn `Button` variant, `PopoverTrigger`, `DropdownMenuTrigger` 등)는
  `Tooltip`/`TooltipTrigger asChild`/`TooltipContent` 를 직접 감싼다. `asChild` 는 중첩 가능하다
  (`Tooltip > TooltipTrigger asChild > PopoverTrigger asChild > button` 형태 — `color-picker.tsx`,
  `branch-switcher.tsx` 참고).
- **`aria-label` 문구와 `TooltipContent` 문구는 항상 같은 i18n 키를 재사용**한다. 별도 문구를
  만들지 않는다.
- `TooltipProvider` 의 `delayDuration` 은 `app-providers.tsx` 에서 전역 400ms 로 이미 고정되어
  있다. 개별 `Tooltip` 에서 `delayDuration` 을 다시 지정하지 않는다.

## 3. native `title=` 금지

- radix `Tooltip` 이 있는 프로젝트에서 `title=` HTML 속성을 툴팁 대용으로 쓰지 않는다.
  스타일·지연·테마가 radix Tooltip 과 다르게 동작한다.
- 검증(validation) 메시지처럼 hover 없이 항상 보여야 하는 경우는 `Tooltip open={condition}` 으로
  **제어형**으로 연다(`file-tree-draft-row.tsx` 의 에러 메시지 사례).
- 컴포넌트 prop 이름이 우연히 `title` 인 경우(`TabItem.title`, `SortableTab title={tab.title}` 등)는
  네이티브 HTML 속성이 아니므로 이 규범과 무관하다 — 승격 대상이 아니다.
