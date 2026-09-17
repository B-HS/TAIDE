# d-65 — focused pane 불변식(닫기 후 dangling 금지) + pane 포커스가 클릭을 추종 (2026-09-17)

> 사용자 보고 2건(2026-09-17):
> ① "파일을 열었다가 파일 패널을 옮기거나 끄거나 … `pane not found` 토스트, 저장도 안 되고, 열려 있는
> 파일을 트리/⌘P 로 열려 해도 안 열린다"
> ② "새 패널을 열면 기존 패널의 파일 수정사항이 저장되지 않는다 … 저장만이 아니라 ⌘W 등 단축키가
> 기존 패널에 아예 안 먹는다"
>
> 정본: 이 계약. 버그 기록 `docs/bug/2026-09-17-stale-focused-pane-and-focus-not-following-click.md`.

## 0. 근본 원인 (메인 직접, 소스 실물)

### 0.1 ① dangling `focused_pane` — 닫기 경로만 보정이 빠져 있다

- `src-tauri/src/domain/layout/service.rs` `close_tab`(562~): `extract_tab` → `normalize`(빈 리프 제거) →
  `push_closed` → `revision += 1`. **`ensure_focused_pane_valid` 를 부르지 않는다.** 분할 반쪽(포커스 pane)의
  마지막 탭을 ⌘W/✕ 로 닫으면 그 pane 이 `normalize` 로 사라지고 `focused_pane`(main 또는 aux 창의
  `focused_pane`)은 사라진 id 를 계속 가리킨다. `close_file_tabs_under`(파일 삭제 → 탭 닫기)도 같은
  `close_tab` 을 돈다. 반면 `move_tab`·`insert_new_leaf`(split / open_tab_in_split)·`return_auxiliary_window_tabs`
  는 이미 `ensure_focused_pane_valid` 를 부른다 — 닫기만 빠진 비대칭.
- `ensure_focused_pane_valid`(1168) 는 영속 경로(`strip_volatile_tabs` → `save_layout`)에서만 돌아, **앱을
  재시작해야** 고쳐진다. `finish_mutation`(1337) 은 보정 없이 `layout.clone()` 스냅샷을 프론트로 돌려준다.
- d-58 의 `resolve_default_open_pane`(453) 은 `target: None` 일 때만 폴백한다. 그런데 d-62 가 추가한 프론트
  `withCurrentWindowTarget`(`src/entities/layout/layout.query.ts` 96~)이 **모든 `target: null` 을 캐시의
  `focusedPane` 으로 치환**해 명시 target 으로 보내므로 Rust 폴백은 더 이상 닿지 않는다. 결과: `open_tab`(467)
  의 `pane_not_found` → 로케일 `error.layout.paneNotFound`("That editor pane no longer exists") 토스트.
  같은 파일이 이미 열려 있어도 dedupe 분기 전에 실패하므로 트리 클릭·⌘P 둘 다 죽는다(보고 ①의 "열려 있는
  파일이 안 열린다").
- 프론트 `widgets/editor-area/editor-area.tsx` 의 키맵 핸들러 전부(`closeFocusedTab`·`saveActiveTab`·
  `cycleTab`·`toggleTerminal`…)가 `findPaneLeaf(paneTree.root, paneTree.focusedPane)` 로 시작해 `null` 이면
  조용히 반환 → ⌘S·⌘W 가 무반응(보고 ①의 "저장도 안 된다").

### 0.2 ② pane 포커스가 클릭을 따라오지 않는다

- 레이아웃 `focused_pane` 을 바꾸는 프론트 경로는 **탭 바 mousedown**(`pane-tab-bar.tsx:276
  onMouseDown={() => focusPane(paneId)}`)과 키보드 그룹 이동(`editor-area.tsx` `focusGroup`)뿐이다. 에디터
  본문·터미널·diff·미리보기 안을 클릭해도 `layout_focus_pane` 이 나가지 않는다(`grep focusPane\|onDidFocus`
  로 `features/editor/code-editor.tsx`·`widgets/editor-pane`·`terminal-session.tsx` 전부 0건; 터미널의
  `onFocusChange` 는 로컬 `isFocused` state 뿐).
- 새 pane 이 열리면(탭 드래그 split·"오른쪽에 터미널 열기"·"Open to the Side") Rust 가 새 리프를 포커스한다
  (`insert_new_leaf` → `set_tree_focused_pane(new_leaf)`). 사용자가 **기존 pane 의 에디터 본문을 클릭해 편집**
  해도 `focused_pane` 은 새 pane 에 남으므로 ⌘S 는 새 pane 의 활성 탭을 저장(터미널이면 `resolveSaveRoutableTabId`
  가 null → 무반응)하고 ⌘W 는 새 pane 의 탭을 닫는다 → 보고 ②("기존 패널에 단축키가 안 먹고 수정사항이
  저장되지 않는다"). 셸 슬롯 층은 정확히 이 문제를 DOM 캡처(`app/providers/shell-slot-provider.tsx`
  pointerdown/focusin)로 풀었는데 pane 층에는 그 판정이 없다.
- ①과 ②는 연쇄한다: 새 pane 을 열어 작업하다 그 pane 을 닫으면 ① 상태로 떨어진다.

## 1. 수정 방향

### R1 (Rust, `src-tauri/src/domain/layout/service.rs` 단독)

1. **`close_tab` 후 포커스 승계**: 닫는 탭이 그 pane 의 마지막 탭이고 그 pane 이 해당 트리(main/aux)의
   `focused_pane` 이면, `normalize` **전에** 승계 pane 을 계산한다 — 부모 split 안의 **이전 형제의 마지막
   리프**, 없으면 **다음 형제의 첫 리프**(순수 헬퍼 1개, 예: `successor_leaf_after_prune(root, pane_id)`).
   `normalize` 뒤 그 승계 pane 이 살아 있으면 `set_tree_focused_pane`, 아니면 `ensure_focused_pane_valid`.
   포커스되지 않은 pane 의 탭을 닫을 때는 포커스를 건드리지 않는다.
2. **불변식 봉인**: `finish_mutation` 이 스냅샷을 만들기 전에 `ensure_focused_pane_valid(layout)` 를 부른다 —
   어떤 mutation 경로(장래 포함)도 dangling `focused_pane` 을 메모리·프론트에 남기지 못한다. `load_layout`
   (복원)이 이미 보정하는지 확인하고 아니면 같은 보정을 붙인다.
3. `resolve_default_open_pane`·`pane_not_found` 는 그대로 둔다(명시 target 이 정말 없을 때의 방어).
4. 테스트(`#[cfg(test)]` 기존 스타일 — 한글 함수명): (a) 2분할·오른쪽 포커스·오른쪽 마지막 탭 닫기 → 단일
   리프가 되고 `focused_pane` == 남은 리프 (b) 3분할 [A,B,C]·C 포커스·C 마지막 탭 닫기 → B (c) [A,B,C]·A
   포커스·A 마지막 탭 닫기 → B(다음 형제) (d) 포커스되지 않은 pane 의 마지막 탭 닫기 → `focused_pane` 불변
   (e) `apply_tab_path_change(Deleted)` 가 포커스 pane 을 비워도 유효 (f) aux 창 2분할에서 aux 포커스 pane
   의 마지막 탭 닫기 → 그 창의 `focused_pane` 보정, main 은 불변. 기존 테스트 `탭을_닫으면_비는_리프가_제거되고…`
   는 유지.

### F1 (TS, `src/shared/lib/pane-tree.ts` + `pane-tree.test.ts` + `src/entities/layout/use-open-file-tab.test.tsx`)

- `resolveWindowPaneTree` 가 돌려주는 `focusedPane` 을 **트리에서 검증**하고, 없으면 첫 리프(`collectPaneLeaves`
  의 첫 항목)로 폴백한다(Rust `ensure_focused_pane_valid` 의 프론트 미러). 리프가 하나도 없으면 원값 유지.
  이 한 곳으로 `currentWindowFocusedPane` → `withCurrentWindowTarget`(명시 target 이 절대 dangling 이 아니게)과
  `editor-area.tsx` 핸들러 28곳 소비처가 함께 방어된다. JSDoc 에 "Rust 가 R1 로 불변식을 보장하지만, 캐시된
  스냅샷과 `layout:changed` 에코 사이의 창을 위해 프론트도 같은 규칙을 미러한다"를 적는다.
- 테스트: `pane-tree.test.ts` — stale `focusedPane` → 첫 리프 id / 정상이면 그대로 / aux 창도 동일.
  `use-open-file-tab.test.tsx` — 캐시 레이아웃의 `focusedPane` 이 stale 일 때 `commands.layoutOpenTab` 의
  `target` 인자가 stale id 가 아니라 첫 리프 id 다.

### F2 (TS, `src/widgets/editor-area/pane-node-view.tsx` + `pane-tab-bar.tsx`(+테스트) + 신규 테스트)

- 리프 래퍼 `<div className='flex h-full min-h-0 w-full min-w-0 flex-1 flex-col'>` 에 `onPointerDownCapture`
  와 `onFocusCapture` 를 달아 **그 pane 이 포커스가 아니면 `focusPane(node.id)`** 를 부른다
  (`useFocusPane(projectId)`). 한 클릭이 내는 pointerdown → focusin 쌍이 React 커밋 전에 연달아 오므로
  `shell-slot-provider.tsx` G-1 과 같은 **ref 가드**로 IPC 1회만 보낸다(포커스 pane 이 바뀌면 가드 해제).
  이미 포커스된 pane 안의 이벤트는 무시(0회). Radix 포털(메뉴·다이얼로그)은 래퍼 밖이라 영향 없음. 탭 바도
  래퍼 안이므로 `pane-tab-bar.tsx` 의 `onMouseDown={() => focusPane(paneId)}` 와 그 `useFocusPane` 은
  **제거**한다(중복 IPC 제거, 판정 1곳). 관련 테스트가 mousedown → focusPane 을 단언하면 새 경로로 갱신.
- 포커스 루프 없음 확인: `focusPane` → 새 스냅샷 → `autoFocus` prop 변화는 `code-editor.tsx` 가 path 변경
  때만 `editor.focus()` 하므로 재귀 없음. 터미널 `autoFocus` 도 같은 pane 안이면 가드에 걸린다.
- 테스트(신규 `pane-node-view-focus.test.tsx`, `pane-node-view-welcome.test.tsx` 의 mock 골격 재사용): 2리프
  split·왼쪽 포커스 렌더 → 오른쪽 리프 내용 pointerdown → `commands.layoutFocusPane` 1회(오른쪽 id) → 이어
  focusin → 여전히 1회 / 왼쪽(포커스) 리프 pointerdown → 0회.

### 범위 외

- ⌘S 가 `untitled` 탭에서 무반응(`focused-editor-tab.ts` JSDoc 의 기지 결함) — 별건.
- VS Code 식 "가장 최근 활성 그룹" MRU 승계 — 형제 승계로 충분, 도입하지 않는다.
- e2e 스펙 — 단위/컴포넌트 테스트로 대신하고 실기는 사용자.

## 2. 실행·검토 계획

- 구현 wf: fixer 3 병렬(opus·xhigh) — R1 Rust 1 / F1 TS / F2 TS(파일 소유 분리) → 검증 1(sonnet·high:
  typecheck·lint·prettier·bun test 전체·cargo fmt·`cargo test -p taide --lib layout`·clippy) → 렌즈 2
  (sonnet·xhigh: 근본성/회귀·경계 — aux 창·Zen·dnd·포커스 루프) → major 있으면 수정 fixer 1 → 문서 fixer 1
  (opus·high: 버그 기록·`docs/features/tabs.md`·`layout-shell.md`·`ipc-contract.md`·이 계약 §3).
- 적대적 검증 생략 근거: 원인이 grep·소스 실물로 기계 재현됐고 메인이 직접 확인(§0). 렌즈 major 는 건별 반박
  없이 메인이 소스로 판정한다.
- 메인 2차 검증: `bun run verify` + `bunx vite build` → 커밋 분할(fix(layout) Rust / fix(editor) TS / docs) →
  사용자 실기.

## 3. 기록 (구현·검토·검증 — 실시간)

### 3.1 구현 (fixer 3 병렬)

**R1 — `src-tauri/src/domain/layout/service.rs` 단독**

- 순수 헬퍼 `successor_leaf_after_prune(root, pane_id) -> Option<PaneId>` 신설. 닫히는 pane 을 담은 split 에서
  **이전 형제의 마지막 리프**, 그 pane 이 첫 자식이면 **다음 형제의 첫 리프**. 루트 리프거나 트리에 없으면 `None`.
  형제 순회는 "직전 형제 1개" 로 한정했다(모든 서브트리가 리프를 최소 1개 가지므로 직전 형제가 항상 후보를 낸다).
- `close_tab`: `extract_tab` 직후 `normalize` 전에 ① `find_leaf(...).is_some_and(is_empty_leaf)` 로 **그 pane 이
  방금 비었는지 실측**하고(닫기 전 `tabs.len() == 1` 추정보다 정확 — extract 의 부수효과 이후 상태를 본다)
  ② 신설 헬퍼 `tree_focused_pane(layout, tree)` 로 그 pane 이 **그 트리의** 포커스인지 확인한다. 둘 다 참일 때만
  후보를 구하고, `normalize` 뒤 후보가 살아 있으면 `set_tree_focused_pane`, 아니면 `ensure_focused_pane_valid`.
  `tree_focused_pane` 이 없으면 aux 트리에서 닫을 때 main 포커스와 비교하게 되어 §1 R1 엣지를 어긴다.
- `finish_mutation`: `layout.clone()` 스냅샷 **전에** `ensure_focused_pane_valid(layout)`. `revision` 은 올리지
  않는다 — 보정이 스냅샷 생성 전에 끝나 `LayoutChanged` 의 revision 과 스냅샷이 이미 일치하고, 올리면 mutation
  1회당 revision 이 2 올라 기존 revision 단언 테스트와 계약을 깬다.
- `resolve_default_open_pane`·`pane_not_found` 무변경.
- **이탈 1**: §1 R1 2 의 "`load_layout` 이 이미 보정하는지 확인하고 아니면 붙인다" 는 확인 결과 이미
  `migrate_layout` 직후 `ensure_focused_pane_valid` 를 부르고 있어 **코드 추가 없음**(계약이 허용한 분기).
- **이탈 2**: 테스트 (d) 를 계약 예시(포커스=B, C 닫기)가 아니라 **포커스=A, C 닫기** 로 잡았다. 포커스=B 이면
  과교정 구현에서도 결과가 같아 검출력이 0 인데, 포커스=A 면 과교정 시 B 로 새므로 FAILED 가 난다(실측).
- 테스트 6종 (a)~(f) 를 기존 한글 `#[test]` 스타일로 추가하고, 3회 이상 재사용되는 트리 조립 헬퍼
  `가로_분할`·`레이아웃`·`삼분할_레이아웃(focused)` 를 함께 뒀다(sizes 는 기존 `SPLIT_TOTAL_PERCENT` 사용).

**F1 — `src/shared/lib/pane-tree.ts`(+`pane-tree.test.ts`, `use-open-file-tab.test.tsx`)**

- private 헬퍼 `withExistingFocusedPane(root, focusedPane)` 신설 — `findPaneLeaf` 로 존재를 확인하고 없으면
  `collectPaneLeaves(root).at(0)?.id`, 리프가 하나도 없으면 `?? focusedPane` 으로 원값 유지. `resolveWindowPaneTree`
  의 main·aux **두 분기** 모두 이 헬퍼를 통과한다. 순수 함수이고 `layout` 캐시를 mutate 하지 않으며, slot 이 없는
  보조 창은 종전대로 `null`.
- JSDoc 에 "Rust 가 R1 로 불변식을 봉인해도 각 창은 자기 캐시 스냅샷으로 렌더하므로 mutation 과 `layout:changed`
  에코 사이의 창을 프론트도 같은 규칙으로 막는다" 는 근거를 적었다(소비처 개수는 숫자로 박지 않았다 — 시간이 지나면
  주석이 거짓이 된다). 다른 함수(`activeFilePathOf` 등)는 무변경.
- 이탈 없음.

**F2 — `src/widgets/editor-area/pane-node-view.tsx` · `pane-tab-bar.tsx` · 신규 `pane-node-view-focus.test.tsx`**

- 리프 래퍼 `div` 에 `onPointerDownCapture`·`onFocusCapture` → 판정 함수 `requestPaneFocus`(3줄이라 분리, 리프 전용
  구역 `dropRefByEdge` 뒤·`return` 바로 위). 훅(`paneFocusRequestRef`, `useFocusPane(projectId)`)은 split 분기 앞
  훅 구역. 핸들러는 `preventDefault`/`stopPropagation` 을 부르지 않아 dnd-kit 센서·Radix 트리거 무영향.
- `pane-tab-bar.tsx` 의 `onMouseDown={() => focusPane(paneId)}` 와 그 `useFocusPane`·import 제거(탭 바가 래퍼 안이라
  캡처가 덮는다). 이 파일에 `focusPane` 의 다른 사용처는 없었고, `pane-tab-bar` 대상 기존 테스트도 없어 갱신할 단언
  없음.
- **이탈 1(가드 해제 조건)**: 계약 문구는 "포커스 pane 이 바뀌면 해제" 였으나, **in-flight boolean ref + `mutate` 의
  `onSettled` 해제**로 구현했다. 근거: `ShellSlotProvider` 는 프로바이더 1개가 ref 를 공유하지만 `PaneNodeView` 의 ref
  는 **리프마다 따로**라 형제 pane 클릭으로 갱신되지 않는다. 요청 당시 `focusedPaneId` 를 저장하면 L→R→L→R 순서에서
  ref(=L)와 현재 `focusedPaneId`(=L)가 다시 일치해 정당한 2번째 요청을 영구 억제한다. 왕복이 끝날 때 해제하면 그
  지점에서 `focusedPaneId` 는 이미 이 pane 이라 첫 번째 체크가 이어받고, 실패한 요청도 `onSettled` 로 풀린다.
  의도(한 클릭 1 IPC + 가드가 영구히 남지 않음)는 그대로이고 더 넓은 경우를 덮는다.
- **이탈 2**: 테스트를 계약의 3건이 아니라 **4건**으로 했다 — 위 설계를 잠그는 "포커스가 다른 pane 을 거쳐 돌아온 뒤
  재클릭" 케이스 추가.
- IPC 목은 `mock.module` 대신 `spyOn(commands, 'layoutFocusPane')`(프로세스 전역 목 충돌 회피, preload 의
  `mock.restore` 로 매 테스트 복원). 테스트 트리는 실제 split 노드(Group/Panel)를 렌더하고 `welcomeOnEmptyEditor:false`
  + `zen:true` 로 "본문 클릭" 을 확정했다.

### 3.2 검증 (검증 에이전트 1 — 전부 exit 0, allGreen)

| 명령 | 결과 |
|---|---|
| `bun run typecheck` | 에러 0 |
| `bun run lint` | error 0 / warning 11 — **전부 기존 알려진 warning**(useVirtualizer incompatible-library 6, useEffect exhaustive-deps 5) |
| `bun run format:check` | All matched files use Prettier code style |
| `bun test` | 2754 pass / 0 fail / 6152 expect / 276 파일 / 8.80s |
| `cargo fmt --all -- --check` | 출력 없음 |
| `cargo test -p taide --lib domain::layout` | 98 passed / 0 failed (기존 92 + 신규 6) |
| `cargo test -p taide --lib` | 1667 passed / 0 failed |
| `bun run rust:lint`(clippy `--workspace --all-targets -D warnings`) | 경고·에러 0 |

**검출력 실측(fixer 가 임시 변형으로 확인 후 원상 복구)**

- `close_tab` 승계 블록 제거 변형 → 신규 6개 중 5개 FAILED(a·b·c·e·f), exit 101.
- 포커스 조건(`tree_focused_pane == pane_id`) 제거한 과교정 변형 → (d) 1건 FAILED, exit 101.
- F1: 폴백 제거 시 stale 테스트 실패, 무조건 첫 리프로 덮는 과잉 구현 시 "유효하면 그대로" 테스트 실패.
- F2: 핸들러 제거 시 1·2번 0회로 실패, ref 가드 제거 시 2번이 2회로 실패, `onSettled` 해제 제거 시 4번이 1회로 실패.

### 3.3 렌즈 검토

렌즈 2개(sonnet·xhigh)가 근본성/회귀·경계(aux 창·Zen·dnd·포커스 루프)를 훑었고 **발견 0건**. 따라서 수정 반영·재검증
단계 없음.

### 3.4 남은 위험 · 미확인

- **테스트로 잠기지 않는 것**: happy-dom 에는 레이아웃·페인트가 없고 monaco·xterm 이 목이라 "에디터 본문·터미널을 실제로
  클릭하면 포커스가 옮겨가는가", react-resizable-panels 의 실제 분할 크기·리사이저는 단위 테스트가 말해주지 않는다.
  앱 실행(tauri dev) 검증은 구현 fixer 의 금지 범위라 하지 않았다 → §4 실기.
- **동작 변화**: 우클릭(pointerdown button 2)도 이제 pane 포커스를 옮긴다(계약이 VS Code 동등으로 허용). in-flight 중
  같은 pane 의 추가 요청은 억제되고, 서버가 거부해도 `onSettled` 로 풀리지만 토스트는 띄우지 않아 실패가 조용하다.
- **판정 지점 2곳 잔존**: `editor-area.tsx` 의 키보드 그룹 이동(`focusGroup`)은 소유 밖이라 그대로 자체 `focusPane` 을
  부른다. 중복 IPC 는 없지만 pane 포커스 판정이 pane 층과 키맵 층 2곳에 남는다.
- **`finish_mutation` 의 사정거리**: 이제 모든 레이아웃 커맨드가 dangling 포커스를 자동 보정한다. "존재하지 않는 pane 을
  의도적으로 포커스로 유지" 하는 호출부가 있었다면 동작이 바뀌지만, 전 호출부 5곳(service.rs 2 + commands.rs 3) 확인 결과
  그런 경로는 없었다. `commands.rs::layout_apply_path_change` 의 `outcome.is_empty()` 조기 반환은 `finish_mutation` 을
  거치지 않지만 pane 이 잘리지 않는 경로라 위험이 없다(commands.rs 는 소유 밖이라 미변경).
- **성능**: `close_tab` 이 매 호출마다 `ensure_focused_pane_valid`(트리 순회)를 탈 수 있어 `close_file_tabs_under` 의
  대량 삭제는 O(탭수 × 트리크기)다. 같은 루프가 이미 탭마다 `normalize`(전체 재구성)를 돌므로 상대 증가는 미미하다.
- **F1 폴백의 성격**: 폴백은 "첫 리프" 라 사용자가 마지막으로 보던 pane 과 다를 수 있다. 정상 경로에서는 R1 의 형제 승계가
  먼저 보정하므로 이 폴백은 캐시가 stale 한 짧은 창에서만 관측된다.
- **테스트 목 결합**: `pane-node-view-focus.test.tsx` 는 실제 `@entities/layout/layout.ipc` 가 `commands.layoutFocusPane`
  까지 닿는 것에 의존한다. `use-open-file-tab.test.tsx` 가 같은 모듈을 프로세스 전역 `mock.module` 로 대체하므로, 실행
  순서나 목 등록 방식이 바뀌면 spy 가 0회로 떨어질 수 있다(단독·역순 2파일·전체 276파일 실행 모두 그린임을 실측).

## 4. 후속

- 사용자 실기: ① 탭을 오른쪽으로 드래그 분할 → 새 pane 의 탭을 ⌘W 로 닫기 → 트리 클릭·⌘P·⌘S 정상 ② 분할 뒤
  기존 pane 본문 클릭 → 편집 → ⌘S 가 그 파일을 저장, ⌘W 가 그 탭을 닫음 ③ 오른쪽에 터미널 열기 뒤 왼쪽
  에디터 클릭 → ⌘S 저장.
