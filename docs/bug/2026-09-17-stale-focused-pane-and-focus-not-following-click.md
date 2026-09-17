# 분할 pane 을 닫으면 "pane not found" · 새 pane 을 열면 기존 pane 에 ⌘S/⌘W 가 안 먹는다 (2026-09-17)

> 사용자 보고 2건(2026-09-17):
> ① "파일을 열었다가 파일 패널을 옮기거나 끄거나 … `pane not found` 토스트, 저장도 안 되고, 열려 있는
> 파일을 트리/⌘P 로 열려 해도 안 열린다"
> ② "새 패널을 열면 기존 패널의 파일 수정사항이 저장되지 않는다 … 저장만이 아니라 ⌘W 등 단축키가
> 기존 패널에 아예 안 먹는다"
>
> 계약 정본은 `docs/acknowledge/2026-09-17-d65-pane-focus-invariant-contract.md`,
> 기능 정본은 `docs/features/tabs.md` §1, 진행 상태는 `docs/PROCESS.md` d-65 절.

## 증상

1. **①** 에디터를 분할한 뒤 포커스된 쪽의 마지막 탭을 ⌘W(또는 ✕)로 닫으면, 그 다음부터 파일 트리 클릭·
   ⌘P·에이전트 열기가 전부 `error.layout.paneNotFound`("That editor pane no longer exists") 토스트로
   끝난다. 이미 열려 있는 파일도 열리지 않는다. 같은 상태에서 ⌘S·⌘W·터미널 토글은 아무 반응이 없다.
   앱을 재시작하면 풀린다.
2. **②** 탭 드래그 분할·"오른쪽에 터미널 열기"·"Open to the Side" 로 새 pane 을 연 뒤, 기존 pane 의
   에디터 본문을 클릭해 편집하고 ⌘S 를 눌러도 그 파일이 저장되지 않는다(새 pane 이 터미널이면 무반응,
   파일이면 엉뚱한 파일이 저장된다). ⌘W 도 방금 보던 탭이 아니라 새 pane 의 탭을 닫는다.
3. 두 증상은 연쇄한다 — ②의 상태에서 새 pane 을 닫으면 ① 상태로 떨어진다.

## 재현

1. 파일 탭을 오른쪽으로 드래그해 분할 → 새 pane(포커스)의 탭을 ⌘W 로 닫기 → 파일 트리에서 아무 파일 클릭.
2. 파일 탭을 분할 → **기존 pane 의 에디터 본문**을 클릭해 한 글자 입력 → ⌘S.

## 원인 (메인이 소스 실물로 확인 — 계약 §0)

### ① dangling `focused_pane` — 닫기 경로만 보정이 빠져 있었다

- `src-tauri/src/domain/layout/service.rs` `close_tab` 은 `extract_tab` → `normalize`(빈 리프 제거) →
  `push_closed` → `revision += 1` 만 했고 **`ensure_focused_pane_valid` 를 부르지 않았다.** 포커스 pane 의
  마지막 탭을 닫으면 그 pane 이 `normalize` 로 사라지고 `focused_pane`(main 또는 aux 창)은 사라진 id 를
  계속 가리켰다. 파일 삭제 경로(`close_file_tabs_under`)도 같은 `close_tab` 을 돈다. 반면
  `move_tab`·`insert_new_leaf`·`return_auxiliary_window_tabs` 는 이미 보정하고 있던 **닫기만의 비대칭**.
- `ensure_focused_pane_valid` 는 영속 경로(`strip_volatile_tabs` → `save_layout`)에서만 돌아 **앱 재시작
  후에야** 고쳐졌다. `finish_mutation` 은 보정 없이 `layout.clone()` 스냅샷을 프론트로 돌려줬다.
- d-58 의 `resolve_default_open_pane` 폴백은 `target: None` 일 때만 작동하는데, d-62 의 프론트
  `withCurrentWindowTarget`(`src/entities/layout/layout.query.ts`)이 모든 `target: null` 을 캐시의
  `focusedPane` 으로 치환해 **명시 target** 으로 보내므로 폴백에 닿지 않았다 → `open_tab` 의
  `pane_not_found`. dedupe 분기보다 앞에서 실패하므로 이미 열린 파일도 열리지 않았다.
- `src/widgets/editor-area/editor-area.tsx` 의 키맵 핸들러(`closeFocusedTab`·`saveActiveTab`·`cycleTab`·
  `toggleTerminal` 등)는 전부 `findPaneLeaf(paneTree.root, paneTree.focusedPane)` 로 시작해 `null` 이면
  조용히 반환한다 → ⌘S·⌘W 무반응.

### ② pane 포커스가 클릭을 따라오지 않았다

- 레이아웃 `focused_pane` 을 바꾸는 프론트 경로는 **탭 바 mousedown**(`pane-tab-bar.tsx`)과 키보드 그룹
  이동(`editor-area.tsx` `focusGroup`) 둘뿐이었다. 에디터 본문·터미널·diff·미리보기 안을 클릭해도
  `layout_focus_pane` 이 나가지 않았다.
- 새 pane 이 열리면 Rust 가 새 리프를 포커스한다(`insert_new_leaf` → `set_tree_focused_pane`). 사용자가
  기존 pane 본문을 클릭해 편집해도 `focused_pane` 은 새 pane 에 남으므로 ⌘S/⌘W 가 새 pane 을 겨냥했다.
- 셸 슬롯 층은 같은 문제를 DOM 캡처(`app/providers/shell-slot-provider.tsx` 의 pointerdown/focusin)로
  이미 풀어 뒀는데, pane 층에는 그 판정이 없었다.

## 해결

세 갈래로 나눠 고쳤다(계약 §1).

- **R1 — 닫기 승계 + 불변식 봉인 (`src-tauri/src/domain/layout/service.rs` 단독)**
  - 순수 헬퍼 `successor_leaf_after_prune(root, pane_id)` 신설: 닫히는 pane 을 담은 split 에서 **이전 형제의
    마지막 리프**, 그 pane 이 첫 자식이면 **다음 형제의 첫 리프**를 승계 후보로 계산한다(루트 리프이거나
    트리에 없으면 `None`). MRU 가 아니라 형제 순서 기준 — 화면에 보이는 순서이고 추가 상태가 필요 없다.
  - `close_tab` 은 `extract_tab` 직후 `normalize` **전에** "그 pane 이 방금 비었는가"(`find_leaf` +
    `is_empty_leaf`)와 "그 pane 이 그 트리의 `focused_pane` 인가"(신설 헬퍼 `tree_focused_pane`)를 둘 다
    만족할 때만 후보를 구하고, `normalize` 뒤 후보가 살아 있으면 `set_tree_focused_pane`, 아니면
    `ensure_focused_pane_valid` 로 폴백한다. 포커스가 아닌 pane 을 닫으면 포커스는 그대로다. aux 창에서
    닫아도 main 포커스는 건드리지 않는다.
  - `finish_mutation` 이 `layout.clone()` 스냅샷을 만들기 **전에** `ensure_focused_pane_valid(layout)` 를
    부른다 — 어떤 mutation 경로든 dangling `focused_pane` 이 메모리(`state.layouts`)·영속본·프론트 스냅샷에
    남지 못한다. `revision` 은 올리지 않는다(보정이 원인 mutation 에 편승하므로 `LayoutChanged` 가 알리는
    revision 과 스냅샷이 이미 일치한다). 복원 경로(`load_layout`)는 `migrate_layout` 직후 이미 같은 보정을
    부르고 있어 추가 변경이 없다.
  - `resolve_default_open_pane`·`pane_not_found` 는 그대로 뒀다(명시 target 이 정말 없을 때의 방어).
- **F1 — 프론트 미러 (`src/shared/lib/pane-tree.ts`)**: private 헬퍼 `withExistingFocusedPane(root,
  focusedPane)` 를 추가해 `resolveWindowPaneTree` 의 main·aux 두 분기가 모두 반환 `focusedPane` 을
  `findPaneLeaf` 로 검증하고, 없으면 `collectPaneLeaves(root).at(0)?.id` 로 폴백한다(리프가 하나도 없으면
  원값 유지). Rust 가 불변식을 봉인해도 각 창은 **자기 캐시 스냅샷**으로 렌더하므로, 포커스 pane 이 잘린
  mutation 과 `layout:changed` 에코 사이의 창을 이 한 곳이 막는다 — `currentWindowFocusedPane` →
  `withCurrentWindowTarget`(명시 target 이 dangling 이 되지 않게)과 `editor-area.tsx` 핸들러 소비처가 함께
  방어된다.
- **F2 — pane 포커스가 클릭을 추종 (`src/widgets/editor-area/pane-node-view.tsx`·`pane-tab-bar.tsx`)**:
  리프 래퍼 `div` 에 `onPointerDownCapture`·`onFocusCapture` 를 달아, 그 pane 이 포커스가 아닐 때만
  `layout_focus_pane` 을 보낸다(`requestPaneFocus`). 캡처 단계라 자식의 `stopPropagation` 에 먹히지 않고,
  핸들러가 `preventDefault`/`stopPropagation` 을 부르지 않아 dnd-kit 센서·Radix 트리거는 그대로다. 한 클릭이
  내는 pointerdown → focusin 쌍은 in-flight `useRef` 가드로 IPC 1회만 내보내고, 요청이 settle 될 때
  (`mutate` 의 `onSettled`) 해제한다 — 리프마다 ref 가 따로라 "요청 당시 포커스 pane 저장" 방식은 L→R→L→R
  왕복에서 정당한 요청을 영구 억제하므로 해제 조건을 계약 문구와 달리 잡았다(계약 §3 이탈 항목).
  중복 경로였던 `pane-tab-bar.tsx` 의 `onMouseDown={() => focusPane(paneId)}` 와 그 훅·import 는 제거했다
  (탭 바가 래퍼 안이라 캡처가 덮는다).

## 검증

- Rust: `cargo test -p taide --lib domain::layout` 98 passed / 0 failed(기존 92 + 신규 6),
  `cargo test -p taide --lib` 1667 passed / 0 failed, `cargo fmt --all -- --check`·
  `cargo clippy --workspace --all-targets -- -D warnings` 전부 exit 0.
  신규 테스트 6종: (a) 2분할·오른쪽 포커스·마지막 탭 닫기 → 남은 리프 승계 (b) 3분할 C 포커스·C 닫기 → B
  (c) 3분할 A 포커스·A 닫기 → B(다음 형제) (d) 포커스 아닌 pane 의 마지막 탭 닫기 → 포커스 불변
  (e) `apply_tab_path_change(Deleted)` 로 포커스 pane 이 비어도 `focused_pane` 유효
  (f) aux 창 2분할에서 aux 포커스 pane 닫기 → 그 창만 승계, main 불변.
  검출력 실측: 승계 블록을 제거한 변형에서 6개 중 5개 FAILED, 포커스 조건을 뺀 과교정 변형에서 (d) FAILED.
- TS: `bun test` 전체 2754 pass / 0 fail(276 파일), `bun run typecheck`·`bun run lint`(신규 warning 0,
  기존 11건 유지)·`bun run format:check` 전부 exit 0.
  신규 테스트: `pane-tree.test.ts` 4건(stale → 첫 리프 / 유효하면 그대로 / aux stale 은 그 창 첫 리프이고
  main 불변 / 리프 0 이면 원값), `use-open-file-tab.test.tsx` 1건(stale 캐시일 때 `layoutOpenTab` 의
  `target` 이 첫 리프), `pane-node-view-focus.test.tsx` 4건(비포커스 pane 클릭 → IPC 1회 / 같은 클릭의
  focusin 은 추가 발사 없음 / 이미 포커스된 pane 은 0회 / 포커스가 왕복해 돌아온 뒤 재클릭은 다시 1회).
  네 건 모두 수정 전 코드·가드 제거 변형에서 실패함을 실측했다.
- **테스트로 잠기지 않는 것**: happy-dom 에는 레이아웃·페인트가 없고 monaco·xterm 이 목이라 "에디터 본문·
  터미널을 실제로 클릭했을 때 포커스가 옮겨가는가"는 실기 몫이다. 앱 실행 검증은 하지 않았다.

## 실기 대상 (사용자)

1. 탭을 오른쪽으로 드래그 분할 → 새 pane 의 탭을 ⌘W 로 닫기 → 파일 트리 클릭·⌘P·⌘S 가 정상 동작.
2. 분할 뒤 기존 pane 본문 클릭 → 편집 → ⌘S 가 그 파일을 저장하고 ⌘W 가 그 탭을 닫는다.
3. "오른쪽에 터미널 열기" 뒤 왼쪽 에디터 클릭 → ⌘S 저장.

## 이번 범위 밖

- ⌘S 가 `untitled` 탭에서 무반응(`focused-editor-tab.ts` JSDoc 의 기지 결함) — 별건.
- VS Code 식 "가장 최근 활성 그룹" MRU 승계 — 형제 순서 승계로 충분하다고 계약에서 정했다.
- `editor-area.tsx` 의 키보드 그룹 이동(`focusGroup`)은 여전히 자체 `focusPane` 을 부른다 — 중복 IPC 는
  없지만 pane 포커스 판정 지점이 2곳으로 남아 있다.
