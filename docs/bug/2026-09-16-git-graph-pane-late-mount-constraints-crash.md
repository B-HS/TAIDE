# git 뷰 첫 오픈·프로젝트 전환 시 "Sidebar Panel / Something went wrong" — 그래프 pane 지연 마운트 크래시 (2026-09-16)

> 사용자 실기 보고(2026-09-16): "git 창을 여니까 Sidebar Panel / Something went wrong / Retry 가 뜬다. Retry 하면 파일 트리로
> 돌아가고 다시 git 창을 열면 열린다. git 창이 열린 상태로 다른 레포지토리로 가면 또 Retry 가 뜬다."
> 기능 정본은 `docs/features/git.md` §1(본문 세로 2-pane), 진행 상태는 `docs/PROCESS.md` 해당 절.

## 증상

1. 사이드바 git 뷰를 **처음** 열면 패널 자리에 `errorBoundary.sidebarPanel` 폴백("Sidebar Panel — Something went wrong. Retry")이 뜬다.
2. Retry 를 누르면 파일 트리가 보인다 — 경계가 `ExplorerContainer` 를 통째로 다시 마운트하므로 `view` state 가 `'files'` 로 초기화되기 때문이다.
3. 다시 git 뷰를 열면 정상. git 뷰가 열린 채 **다른 프로젝트로 전환**하면 1 이 재현된다.

앱 로그(`~/Library/Logs/net.gumyo.taide/TAIDE.log`)에 매번 같은 예외가 남는다.

```
[ERROR] Panel constraints not found for Panel git-graph
  r@…               (react-resizable-panels 내부 lookup)
  isCollapsed@…     (PanelImperativeHandle.isCollapsed)
  @…                (git-panel.tsx 그래프 pane 접힘 동기화 useEffect)
  … React passive effect 순회
```

릴리스 빌드라 폴백에 스택이 안 보였을 뿐(`import.meta.env.DEV` 게이트), 경계가 잡은 것은 이 throw 다.

## 원인 (소스·라이브러리 dist 로 확인)

`src/widgets/git-panel/git-panel.tsx` 는 `git-graph` `<Panel>` 을 `sections.graph.visible`(= 커밋 수 > 0) 일 때만 렌더하고,
설정의 접힘 상태를 `useEffect` 에서 `graphPanelRef.current.isCollapsed()` / `collapse()` / `resize()` 로 pane 에 밀어 넣는다
(deps 에 `sections.graph.visible` 포함).

- 콜드 오픈·프로젝트 전환 시 log 쿼리가 아직 비어 `graphCommits = []` → pane 없이 마운트 → log 도착 후 pane 이 **이미 마운트된
  `Group` 안으로 나중에** 마운트된다.
- react-resizable-panels 4.12.2 에서 `Panel` 은 layout effect 에서 `registerPanel` 로 Group 의 panels 배열에 자신을 넣고 state
  bump 로 Group 재렌더를 예약할 뿐이고, Group 이 `derivedPanelConstraints` 를 다시 계산하는 것은 **다음 커밋의 layout effect**
  에서다. 그런데 React 는 다음 커밋을 시작하기 전에 현재 커밋의 passive effect 를 먼저 flush 한다.
- 그래서 pane 이 마운트된 그 커밋의 passive phase 에서 우리 effect 가 `isCollapsed()` 를 부르면, 핸들의 lazy lookup 이 아직
  `git-graph` 가 없는 이전 constraints 를 보고 `Panel constraints not found for Panel git-graph` 를 던진다.
- 캐시가 따뜻한 두 번째 오픈은 Group 과 Panel 이 한 커밋에 같이 마운트된다(자식 layout effect 가 먼저 등록 → Group 의 등록이
  포함해 계산) → 재현되지 않는다. 프로젝트 전환은 새 프로젝트의 log 가 캐시에 없을 때만 재현되는 것도 같은 이유다.

같은 클래스의 코드는 이 한 곳뿐이다. `project-shell.tsx`·`auxiliary-window-shell.tsx` 의 `usePanelRef` 는 항상 마운트되는
패널에 쓰이고, `editor-area.tsx` 의 조건부 Problems `Panel` 은 명령형 핸들을 쓰지 않는다.

## 해결

핸들을 ref(`usePanelRef`)가 아니라 라이브러리의 state 기반 콜백 ref **`usePanelCallbackRef`** 로 받는다.

- `Panel` 의 `useImperativeHandle` 이 마운트 커밋의 layout phase 에서 setter 를 호출 → 이 state 업데이트가 `registerPanel` 의
  bump 와 같은 재렌더로 배치된다 → 그 다음 커밋의 layout phase 에서 Group 이 constraints 를 재계산하고, 같은 커밋의 passive
  phase 에서 핸들을 deps 로 가진 동기화 effect 가 돈다. 즉 핸들이 `null → 객체` 로 바뀌는 시점이 정확히 "Group 등록 완료 후" 다.
- effect deps 는 `[graphPanel, collapsedSections.graph, graphPanelSizePx]` — 핸들의 null↔객체 전이가 pane 의 마운트/언마운트를
  그대로 대신하므로 `sections.graph.visible` 은 뺐다. `onLayoutChanged` 핸들러도 같은 state 를 읽는다.
- 기각한 대안: `Group` 에 그래프 가시성 key 를 붙여 리마운트(변경 목록 스크롤 위치·포커스 리셋), try/catch·rAF 지연(우회).
  커밋 0건이면 pane 을 렌더하지 않는 규칙(`docs/features/git.md` §1)은 그대로다.

## 검증

- 회귀 테스트: `src/widgets/git-panel/git-panel.test.tsx` "커밋이 늦게 도착해 pane 이 뒤늦게 마운트돼도 그래프가 그대로 열린다" —
  커밋 없이 마운트한 뒤 `rerender` 로 커밋을 늦게 넘겨 pane 이 나중에 마운트되는 시나리오. 수정 전 코드에서는 happy-dom 에서도
  앱 로그와 같은 예외(`git-panel.tsx:442 isCollapsed` → `commitHookPassiveMountEffects`)로 실패(15 pass / 1 fail), 수정 후
  16 pass / 0 fail. `tsc --noEmit`·eslint·prettier 통과(2026-09-16).
- happy-dom 은 모든 엘리먼트가 0px 라 `collapse`/`resize` 의 실제 레이아웃 효과와 "커밋 0건으로 되돌아가는" 전이는 테스트로
  잠기지 않는다 — 실기 확인 항목.
- 사용자 실기(릴리스 빌드 재빌드 후): git 뷰 첫 오픈 → 폴백 없이 패널 표시 / git 뷰 열린 채 프로젝트 전환 → 폴백 없음 /
  로그에 `Panel constraints not found` 미출력.
