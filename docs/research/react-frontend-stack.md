# TAIDE 프론트엔드(React) 스택 최신 확정 (2026-08-06 기준)

조사 방법: npm registry(`registry.npmjs.org/<pkg>/latest`), crates.io API, 각 프로젝트 공식 문서·README·GitHub Releases 를 직접 조회해 확정했다.
확인하지 못한 항목은 본문에 **미확인**으로 표기했다.

---

## 버전 확정 (2026-08 기준)

### 코어

| 패키지 | 확정 버전 | 최근 발행일 | 비고 |
|---|---|---|---|
| `react` / `react-dom` | **19.2.8** | 2026-07-21 | React 19 가 현재 메이저. 19.3 / 20 발표 없음 |
| `vite` | **8.2.0** | (8.0 stable 2026-03-12) | Rolldown(Rust) 단일 번들러로 통합 |
| `@vitejs/plugin-react` | **6.0.5** | - | peer: `vite ^8.0.0`, Node `^20.19.0 || >=22.12.0` |
| `babel-plugin-react-compiler` | **1.0.0** | - | React Compiler 1.0 stable (2025-10 릴리스) |
| `@rolldown/plugin-babel` | **0.2.3** | - | plugin-react v6 에서 Babel 실행에 필요 |
| `eslint-plugin-react-hooks` | **7.1.1** | - | React Compiler lint 규칙 포함 |

React 19 패치 라인: 19.2.8 / 19.1.9 / 19.0.8 이 2026-07-21 동시 릴리스됨.

### 상태·데이터

| 패키지 | 확정 버전 | 최근 발행일 | peer |
|---|---|---|---|
| `zustand` | **5.0.14** | 2026-05-28 | `react >=18` (optional) |
| `@tanstack/react-query` | **5.101.4** | 2026-07-21 | `react ^18 \|\| ^19` |
| `@tanstack/react-query-devtools` | **5.101.4** | 2026-07-21 | query `^5.101.4` |

TanStack Query 는 **v5 가 여전히 latest**다. dist-tags 에 v6 계열(alpha/beta/rc)이 존재하지 않는다(현재 alpha/beta/rc 태그는 모두 5.0.0-\* 잔재).

### DnD

| 패키지 | 확정 버전 | 최근 발행일 | 비고 |
|---|---|---|---|
| `@dnd-kit/core` (레거시 라인) | **6.3.1** | 2024년대 | 신규 릴리스 없음 |
| `@dnd-kit/sortable` | **10.0.0** | **2024-12-04** | 1년 8개월간 릴리스 없음 |
| `@dnd-kit/utilities` | 3.2.2 | - | |
| `@dnd-kit/react` (신 라인) | **0.5.0** | 2026-06-11 | latest 는 0.5.0, beta 태그 `0.5.1-beta-20260713030121` |
| `@atlaskit/pragmatic-drag-and-drop` | **2.0.2** | 2026-08-05 | 대안. 프레임워크 비의존 |

### 트리 / 가상화 / 패널

| 패키지 | 확정 버전 | 최근 발행일 | peer react |
|---|---|---|---|
| `react-arborist` | **3.16.0** | 2026-07-25 | `>= 16.14` |
| `@headless-tree/core` / `@headless-tree/react` | **1.7.0** | 2026-05-17 | `react *` |
| `react-complex-tree` | 2.6.2 | 2026-06-24 | `>=16.0.0` |
| `@tanstack/react-virtual` | **3.14.9** | - | `^16.8 ~ ^19` |
| `react-window` | **2.3.0** | 2026-07-20 | `^18 \|\| ^19` |
| `react-resizable-panels` | **4.12.2** | 2026-07-12 | `^18 \|\| ^19` |

### Tauri 측(참고)

| 대상 | 확정 버전 | 최근 |
|---|---|---|
| `tauri` (crate) | **2.11.5** | 2026-07-01 |
| `@tauri-apps/api` (npm) | **2.11.1** | 2026-06-17 |
| `specta` (crate) | **1.0.5** | 2026-05-07 |
| `tauri-specta` (crate) | **1.0.2** | 2026-05-08 |
| `taurpc` (npm) | 2.0.0 | 2026-07-02 |

npm 의 `tauri-specta` 패키지(0.0.2, 2022)는 무관한 사장된 패키지다. tauri-specta 는 crate 로 쓰고 TS 바인딩을 파일로 생성한다.

---

## 핵심 API·사용법

### 1. Vite 8 + React 19 + React Compiler 1.0

**중요 변경**: `@vitejs/plugin-react` v6 는 내부 Babel 을 제거하고 oxc(Rust) 로 JSX/Fast Refresh 를 처리한다.
따라서 과거의 `react({ babel: { plugins: ['babel-plugin-react-compiler'] } })` 형태는 **Vite 8 + plugin-react v6 에서 동작하지 않는다.**

설치:

```bash
pnpm add -D @vitejs/plugin-react @rolldown/plugin-babel @babel/core babel-plugin-react-compiler @types/babel__core
```

`vite.config.ts` (plugin-react 공식 README 형태):

```ts
import { defineConfig } from 'vite'
import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import babel from '@rolldown/plugin-babel'

export default defineConfig({
    plugins: [react(), babel({ presets: [reactCompilerPreset()] })],
    clearScreen: false,
    server: { port: 1420, strictPort: true, watch: { ignored: ['**/src-tauri/**'] } },
    build: { target: 'esnext', sourcemap: true },
})
```

`reactCompilerPreset()` 옵션:

```ts
reactCompilerPreset({
    compilationMode: 'annotation', // 'use memo' 지시가 있는 함수만 컴파일 (점진 도입용)
    target: '19',                  // '17' | '18' 지정 시 react-compiler-runtime 필요
})
```

컴파일 대상 필터 축소(불필요 파일 제외):

```ts
const preset = reactCompilerPreset()
preset.rolldown.filter.id.exclude = ['src/shared/lib/**', 'src/**/*.worker.ts']
```

주의: 공식 react.dev 문서와 커뮤니티 글에서 **플러그인 배치 순서 설명이 엇갈린다.**
- `@vitejs/plugin-react` README(정본): `plugins: [react(), babel({ presets: [reactCompilerPreset()] })]`
- 일부 블로그: `babel` 을 `react()` 앞에 두고 `babel({ include, babelConfig })` 사용
정본(README) 형태를 채택하고, 적용 여부는 아래 검증으로 확인한다.

ESLint (컴파일러 규칙 포함):

```js
// eslint.config.js
import reactHooks from 'eslint-plugin-react-hooks'

export default [reactHooks.configs['recommended-latest']]
```

**컴파일 적용 검증 방법** (추측하지 말고 반드시 확인):
1. React DevTools 컴포넌트에 `Memo ✨` 배지가 보이는지
2. 빌드 산출물에 `react/compiler-runtime` import 가 들어갔는지 (`rg 'compiler-runtime' dist/`)

React 19.2 에서 추가되어 IDE UI 에 유용한 API:
- `<Activity mode="hidden">` — 백그라운드 탭/패널의 상태를 유지한 채 렌더 비용을 낮춤. 에디터 탭 전환에 직접 대응
- `useEffectEvent` — effect 의존성에서 최신 값을 안전하게 읽음 (Tauri 이벤트 구독 핸들러에 유용)

### 2. zustand v5

```ts
// src/shared/store/layout.store.ts
import { create } from 'zustand'
import { persist, subscribeWithSelector } from 'zustand/middleware'
import { immer } from 'zustand/middleware/immer'
import { useShallow } from 'zustand/react/shallow'

type PaneId = string

type LayoutState = {
    tabsByPane: Record<PaneId, string[]>
    activePane: PaneId
    moveTab: (tabId: string, from: PaneId, to: PaneId, index: number) => void
    setActivePane: (paneId: PaneId) => void
}

export const useLayoutStore = create<LayoutState>()(
    subscribeWithSelector(
        persist(
            immer((set) => ({
                tabsByPane: { main: [] },
                activePane: 'main',
                moveTab: (tabId, from, to, index) =>
                    set((state) => {
                        state.tabsByPane[from] = state.tabsByPane[from].filter((id) => id !== tabId)
                        state.tabsByPane[to].splice(index, 0, tabId)
                    }),
                setActivePane: (paneId) => set({ activePane: paneId }),
            })),
            { name: 'taide-layout', partialize: (s) => ({ tabsByPane: s.tabsByPane, activePane: s.activePane }) },
        ),
    ),
)
```

셀렉터 규칙 (v5 필수):
- v5 는 `create` 기본 비교가 `Object.is` 다. **객체·배열을 새로 만들어 반환하는 셀렉터는 매 렌더 리렌더를 유발**한다.
- 다중 값은 `useShallow` 로 감싸거나 필드별로 호출을 쪼갠다.

```ts
const [tabs, activePane] = useLayoutStore(useShallow((s) => [s.tabsByPane.main, s.activePane]))
const moveTab = useLayoutStore((s) => s.moveTab) // 액션은 단일 참조라 그대로
```

React 외부(Tauri 이벤트 핸들러, Rust 브릿지)에서의 접근:

```ts
useLayoutStore.getState().moveTab(...)
const unsub = useLayoutStore.subscribe((s) => s.activePane, (pane) => { /* ... */ })
```

`createWithEqualityFn`(`zustand/traditional`)은 v4 호환용이다. 신규 코드는 `create` + `useShallow` 를 쓴다.

### 3. TanStack Query v5 + Tauri invoke 래핑

**타당성 판단**: Query 는 "비동기 요청 캐시"이지 HTTP 전용이 아니다. `queryFn` 은 Promise 를 반환하기만 하면 되므로 `invoke()` 래핑은 설계상 정당하다. 다만 Query 의 기본값(윈도우 포커스 refetch, 네트워크 모드)은 웹 전제라 Tauri 에서는 반드시 조정해야 한다.

TanStack 공식 문서에 **Tauri 전용 가이드는 없다(미확인)**. 아래는 Query 의 공개 API(`queryOptions`, `invalidateQueries`, `focusManager`, `onlineManager`)를 조합한 표준 패턴이다.

QueryClient 기본값:

```ts
// src/app/query-client.ts
import { QueryClient, focusManager, onlineManager } from '@tanstack/react-query'
import { getCurrentWindow } from '@tauri-apps/api/window'

export const queryClient = new QueryClient({
    defaultOptions: {
        queries: {
            staleTime: 60_000,
            gcTime: 10 * 60_000,
            retry: 0,                 // 로컬 IPC 는 재시도가 대체로 무의미
            networkMode: 'always',    // 필수: 브라우저 오프라인 판정으로 IPC 가 멈추는 것 방지
            refetchOnWindowFocus: false,
        },
    },
})

onlineManager.setOnline(true)

focusManager.setEventListener((handleFocus) => {
    const unlisten = getCurrentWindow().onFocusChanged(({ payload }) => handleFocus(payload))
    return () => { void unlisten.then((f) => f()) }
})
```

invoke 래핑 (entities 데이터 레이어):

```ts
// src/entities/fs/fs.api.ts
import { invoke } from '@tauri-apps/api/core'

export type DirEntry = { path: string; name: string; isDir: boolean }

export const readDir = (path: string) => invoke<DirEntry[]>('read_dir', { path })
export const readFile = (path: string) => invoke<string>('read_file', { path })
export const writeFile = (path: string, contents: string) => invoke<void>('write_file', { path, contents })
```

`queryOptions` 팩토리 + 훅:

```ts
// src/entities/fs/fs.query.ts
'use client'
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { readDir, readFile, writeFile } from './fs.api'

export const QUERY_KEY = {
    FS: {
        ALL: ['fs'] as const,
        DIR: (path: string) => ['fs', 'dir', path] as const,
        FILE: (path: string) => ['fs', 'file', path] as const,
    },
}

export const dirQueryOptions = (path: string) =>
    queryOptions({ queryKey: QUERY_KEY.FS.DIR(path), queryFn: () => readDir(path) })

export const useDir = (path: string) => useQuery(dirQueryOptions(path))

export const useWriteFile = () => {
    const qc = useQueryClient()
    return useMutation({
        mutationFn: ({ path, contents }: { path: string; contents: string }) => writeFile(path, contents),
        onSuccess: (_, { path }) => qc.invalidateQueries({ queryKey: QUERY_KEY.FS.FILE(path) }),
    })
}
```

Rust 이벤트 → invalidate (파일 워처 연동). Query 에는 이벤트 기반 무효화가 **내장돼 있지 않다**. 구독 → `invalidateQueries` 수동 호출이 공식 논의(TanStack/query Discussion #8618)에서도 권장되는 방식이다.

```tsx
// src/app/providers/fs-watch-provider.tsx
'use client'
import { useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useQueryClient } from '@tanstack/react-query'
import { QUERY_KEY } from '@/entities/fs/fs.query'

type FsChangedPayload = { paths: string[]; kind: 'create' | 'modify' | 'remove' }

export const FsWatchProvider = ({ children }: { children: React.ReactNode }) => {
    const qc = useQueryClient()

    useEffect(() => {
        const promise = listen<FsChangedPayload>('fs://changed', ({ payload }) => {
            for (const path of payload.paths) {
                qc.invalidateQueries({ queryKey: QUERY_KEY.FS.FILE(path) })
                qc.invalidateQueries({ queryKey: QUERY_KEY.FS.DIR(path.slice(0, path.lastIndexOf('/'))) })
            }
        })
        return () => { void promise.then((unlisten) => unlisten()) }
    }, [qc])

    return children
}
```

핵심 튜닝 포인트:
- Rust 워처는 **디바운스·배치**해서 이벤트를 보낸다(저장 1회에 modify 이벤트 수 회 발생). 프론트에서 초당 수백 invalidate 가 들어오면 UI 가 멈춘다.
- 에디터에서 편집 중인 파일은 자기 저장이 되돌아오는 echo 를 무시해야 한다. Rust payload 에 `origin`(앱 자체 쓰기 여부)을 실어 구분한다.
- 무효화 대신 페이로드로 캐시를 직접 갱신하는 편이 나은 경우(디렉터리 1건 추가 등)에는 `qc.setQueryData` 를 쓴다. 기본은 invalidate.
- 대용량 스트리밍(빌드 로그, LSP 진단)은 Query 가 아니라 **Tauri `Channel`** 또는 zustand 로 받는다.

```ts
import { Channel, invoke } from '@tauri-apps/api/core'

const channel = new Channel<{ line: string }>()
channel.onmessage = ({ line }) => appendLog(line)
await invoke('run_build', { onEvent: channel })
```

### 4. dnd-kit — 탭 재정렬 + 스플릿 드롭 존

두 라인이 병존한다. 아래는 **레거시 라인(`@dnd-kit/core` 6.3.1 + `@dnd-kit/sortable` 10.0.0)** 기준 구현이다. 이 라인이 API 가 안정적이고 자료가 많다.

```tsx
// widgets/editor-tabs/tab-bar.tsx
import { DndContext, DragOverlay, PointerSensor, pointerWithin, useDroppable, useSensor, useSensors } from '@dnd-kit/core'
import type { DragEndEvent, DragStartEvent } from '@dnd-kit/core'
import { SortableContext, horizontalListSortingStrategy, useSortable, arrayMove } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'

const SortableTab = ({ id, title }: { id: string; title: string }) => {
    const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id, data: { type: 'tab' } })
    return (
        <div
            ref={setNodeRef}
            style={{ transform: CSS.Translate.toString(transform), transition, opacity: isDragging ? 0.4 : 1 }}
            {...attributes}
            {...listeners}>
            {title}
        </div>
    )
}

const SplitZone = ({ paneId, edge }: { paneId: string; edge: 'left' | 'right' | 'top' | 'bottom' | 'center' }) => {
    const { setNodeRef, isOver } = useDroppable({ id: `${paneId}:${edge}`, data: { type: 'split', paneId, edge } })
    return <div ref={setNodeRef} data-over={isOver} className='absolute ...' />
}
```

컨텍스트와 드롭 처리:

```tsx
const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }))

<DndContext
    sensors={sensors}
    collisionDetection={pointerWithin}
    onDragStart={(e: DragStartEvent) => setActiveTabId(String(e.active.id))}
    onDragEnd={(e: DragEndEvent) => {
        const { active, over } = e
        setActiveTabId(null)
        if (!over) return

        const overType = over.data.current?.type
        if (overType === 'split') {
            const { paneId, edge } = over.data.current as { paneId: string; edge: string }
            edge === 'center' ? moveTabToPane(String(active.id), paneId) : splitPane(paneId, edge, String(active.id))
            return
        }
        if (active.id !== over.id) reorderTabs(arrayMove(tabs, tabs.indexOf(String(active.id)), tabs.indexOf(String(over.id))))
    }}>
    <SortableContext items={tabIds} strategy={horizontalListSortingStrategy}>
        {tabIds.map((id) => <SortableTab key={id} id={id} title={titleOf(id)} />)}
    </SortableContext>
    <SplitZone paneId='main' edge='left' />
    <SplitZone paneId='main' edge='right' />
    <DragOverlay>{activeTabId ? <TabGhost id={activeTabId} /> : null}</DragOverlay>
</DndContext>
```

포인트:
- 탭 재정렬과 스플릿 드롭은 **하나의 `DndContext`** 안에서 처리하고 `over.data.current.type` 으로 분기한다.
- 스플릿 존은 드래그 중에만 활성화(overlay div)하고 `pointerWithin` 을 쓴다. 겹치는 오버레이 존에서 `closestCenter` 는 오작동한다.
- 존은 `left/right/top/bottom/center` 5분할 오버레이(VS Code 방식)로 만들고 `isOver` 로 프리뷰를 그린다.
- 탭바는 스크롤되므로 `DragOverlay` 필수(원본 노드 transform 만으로는 컨테이너를 벗어나지 못함).
- IDE 는 키보드 접근성이 중요하다. `KeyboardSensor` + `sortableKeyboardCoordinates` 를 추가로 붙인다.

신 라인(`@dnd-kit/react` 0.5.0) 형태는 다음과 같이 크게 다르다.

```tsx
import { DragDropProvider } from '@dnd-kit/react'
import { useSortable } from '@dnd-kit/react/sortable'
import { move } from '@dnd-kit/helpers'

const Tab = ({ id, index, paneId }: { id: string; index: number; paneId: string }) => {
    const { ref, handleRef, isDragging } = useSortable({ id, index, group: paneId, type: 'tab', accept: ['tab'] })
    return <div ref={ref}>{/* ... */}</div>
}

<DragDropProvider
    onDragOver={(event) => setPanes((panes) => move(panes, event))}
    onDragEnd={(event) => { if (event.canceled) revert() }}
/>
```

`group` 으로 다중 리스트(=다중 pane) 간 이동이 1급 지원되고, `CollisionPriority.Low` 로 컨테이너 vs 아이템 우선순위를 조정한다. 다만 **0.x 이고 pre-1.0** 이다.

### 5. 파일 트리 후보 비교

| 항목 | react-arborist 3.16.0 | headless-tree 1.7.0 | @tanstack/react-virtual 3.14.9 커스텀 |
|---|---|---|---|
| 유지보수(2026) | 활발 (2026-07-25 릴리스, 7월에만 5회) | 활발 (2026-05) | 활발 |
| 가상화 | 내장 (react-window 1.8.x) | 가상화 라이브러리 연동 지원, 100k+ 목표 | 직접 구현(핵심 강점) |
| DnD | 내장 (react-dnd 14 + HTML5 backend) | 내장, 외부 drag 이벤트 연동 가능 | dnd-kit 으로 직접 |
| inline rename | 내장 (`onRename`, `node.edit()`) | 내장 기능 | 직접 |
| 검색/필터 | `searchTerm`/`searchMatch`, 3.16 에 `filteredCount` | typeahead 내장 | 직접 |
| 렌더 자유도 | 중간(Tree 컴포넌트 구조에 종속) | 최상(완전 headless) | 최상 |
| 구현 비용 | 낮음 | 중간 | 높음 |
| 추가 런타임 의존성 | redux 5, react-dnd 14, react-window 1.8, use-sync-external-store | 없음(코어 무의존) | 없음 |

**추천: 1차 react-arborist 3.16.0 로 시작하되, "트리에서 에디터 영역으로 드래그(파일 열기·스플릿 생성)"가 요구사항이면 처음부터 headless-tree(또는 react-virtual 커스텀) + dnd-kit 단일 DnD 체계로 간다.**

근거:
- react-arborist 는 DnD 를 react-dnd(HTML5 backend)로 처리한다. 탭·패널에 dnd-kit(포인터 이벤트 기반)을 쓰면 **한 앱에 서로 모르는 DnD 시스템이 2개** 생기고, 트리 → 탭바 크로스 드래그를 자연스럽게 구현할 수 없다. 이 요구가 있으면 react-arborist 채택은 나중에 되돌리기 비싼 결정이 된다.
- 트리 내부 DnD 만 필요하고(파일 이동/정렬) 크로스 서피스 드래그가 없으면 react-arborist 가 압도적으로 빠르게 붙는다.
- 수만 파일 성능: 세 후보 모두 가상화로 해결 가능하다. 실제 병목은 렌더가 아니라 **평탄화(flatten) 재계산과 Rust→JS 직렬화**다. 노드 수만 개 구간에서는 트리 라이브러리 선택보다 (a) 디렉터리 lazy 로딩, (b) 노드 데이터 정규화(`Record<path, node>`), (c) 워처 이벤트 배치가 성능을 좌우한다. 구체 벤치마크 수치는 확인하지 못했다(**미확인**).

react-arborist 최소 사용례:

```tsx
import { Tree } from 'react-arborist'

<Tree
    data={treeData}
    openByDefault={false}
    width='100%'
    height={containerHeight}
    rowHeight={22}
    indent={12}
    searchTerm={query}
    onRename={({ id, name }) => renameNode(id, name)}
    onMove={({ dragIds, parentId, index }) => moveNodes(dragIds, parentId, index)}
    onActivate={(node) => openInEditor(node.data.path)}>
    {({ node, style, dragHandle }) => (
        <div style={style} ref={dragHandle} onClick={() => node.toggle()}>
            {node.isEditing ? <RenameInput node={node} /> : node.data.name}
        </div>
    )}
</Tree>
```

3.13~3.16 에서 실제로 고쳐진 것(IDE 에서 바로 밟는 버그): `NodeApi` 메서드 바인딩(3.13.1), 중첩 input 의 Space 입력 가로채기(3.13.2), `canDrop()` false 인데 드롭 대상이 보고되던 문제(3.15.1), `onMove` 인덱스 보정 헬퍼 `adjustMoveIndex`(3.15.0).

### 6. react-resizable-panels 4.x — API 가 v2/v3 와 다르다

**v4 는 컴포넌트 이름 자체가 바뀌었다.** 구버전 지식(`PanelGroup`, `PanelResizeHandle`, `autoSaveId`)을 그대로 쓰면 안 된다.

실제 export(4.12.2 `.d.ts` 확인):
`Group`, `Panel`, `Separator`, `useDefaultLayout`, `useGroupRef`, `useGroupCallbackRef`, `usePanelRef`, `usePanelCallbackRef`, `isCoarsePointer`, 타입 `Layout` / `LayoutChangedMeta` / `LayoutStorage` / `Orientation` / `PanelSize` / `SizeUnit`.

```tsx
import { Group, Panel, Separator, useDefaultLayout, usePanelRef } from 'react-resizable-panels'

const EditorLayout = () => {
    const { defaultLayout, onLayoutChanged } = useDefaultLayout({ id: 'taide-main', storage: localStorage })
    const sidebarRef = usePanelRef()

    return (
        <Group orientation='horizontal' defaultLayout={defaultLayout} onLayoutChanged={onLayoutChanged}>
            <Panel id='sidebar' panelRef={sidebarRef} defaultSize='240px' minSize='180px' maxSize='40%' collapsible collapsedSize={0}>
                <FileTree />
            </Panel>
            <Separator />
            <Panel id='editor' minSize='30%'>
                <EditorPane />
            </Panel>
        </Group>
    )
}
```

- 사이즈는 숫자 = px, 단위 없는 문자열 = %, `em/rem/vh/vw` 지원(`SizeUnit`).
- `Panel` / `Separator` 는 **`Group` 의 직속 DOM 자식**이어야 한다. 래퍼 div 를 끼우면 동작하지 않는다.
- 레이아웃 저장은 `useDefaultLayout({ id, storage })` → `defaultLayout` + `onLayoutChanged` 연결. (`onLayoutChange` 와 `debounceSaveMs` 는 deprecated)
- 명령형 API: Group `getLayout()/setLayout()`, Panel `collapse()/expand()/resize()/getSize()/isCollapsed()`.
- 중첩 스플릿(IDE 그리드)은 `Panel` 안에 다시 `Group orientation='vertical'` 을 넣어 만든다.

### 7. React 19 / React Compiler 호환성 정리

| 라이브러리 | React 19 | 근거 | Compiler 1.0 |
|---|---|---|---|
| zustand 5.0.14 | 지원 | peer `react >=18.0.0`, `useSyncExternalStore` 기반 | 문제 보고 확인 못 함(**미확인**) |
| @tanstack/react-query 5.101.4 | 지원 | peer `^18 \|\| ^19` 명시 | 공식 TanStack Start 조합 사례 존재 |
| @dnd-kit/core 6.3.1 / sortable 10 | peer 상 허용(`>=16.8.0`) | 명시적 React 19 지원 선언은 확인 못 함(**미확인**) | 미확인 |
| @dnd-kit/react 0.5.0 | 명시 지원 | peer `^18.0.0 \|\| ^19.0.0` | 미확인 |
| react-arborist 3.16.0 | peer `>=16.14`, 하위 react-window 1.8.11 peer 에 `^19` 포함, react-dnd 14 peer `>=16.14` | 공식 "React 19 지원" 문구는 확인 못 함(**미확인**) | 미확인 |
| react-resizable-panels 4.12.2 | 명시 지원 | peer `^18 \|\| ^19` | 미확인 |
| @tanstack/react-virtual 3.14.9 | 명시 지원 | peer 에 `^19.0.0` | 미확인 |
| react-window 2.3.0 | 명시 지원 | peer `^18 \|\| ^19` | 미확인 |
| @headless-tree/react 1.7.0 | peer `react *` | 버전 제약 없음 | 미확인 |

Compiler 관련 일반 원칙: React Compiler 는 **React 규칙을 지키는 코드**만 안전하게 최적화한다. 라이브러리 자체보다, 렌더 중 store mutate·ref 읽기 같은 앱 코드 위반이 문제를 만든다. `eslint-plugin-react-hooks@7` 의 `recommended-latest` 로 사전 검출한다.

---

## TAIDE 적용 가이드

**확정 스택 (package.json 기준선)**

```json
{
    "dependencies": {
        "react": "19.2.8",
        "react-dom": "19.2.8",
        "zustand": "5.0.14",
        "@tanstack/react-query": "5.101.4",
        "@dnd-kit/core": "6.3.1",
        "@dnd-kit/sortable": "10.0.0",
        "@dnd-kit/utilities": "3.2.2",
        "react-resizable-panels": "4.12.2",
        "react-arborist": "3.16.0",
        "@tauri-apps/api": "2.11.1"
    },
    "devDependencies": {
        "vite": "8.2.0",
        "@vitejs/plugin-react": "6.0.5",
        "@rolldown/plugin-babel": "0.2.3",
        "@babel/core": "^7.29.0",
        "@types/babel__core": "^7",
        "babel-plugin-react-compiler": "1.0.0",
        "eslint-plugin-react-hooks": "7.1.1",
        "@tanstack/react-query-devtools": "5.101.4"
    }
}
```

**레이어 배치(FSD)**

- `entities/fs/fs.api.ts` — `invoke` 래퍼만. Tauri 의존은 여기서 끝낸다.
- `entities/fs/fs.query.ts` — `queryOptions` 팩토리 + `useQuery`/`useMutation`. `QUERY_KEY` 중앙 관리.
- `shared/store/layout.store.ts` — 탭·패널 트리(스플릿 구조), zustand + persist. **서버(=Rust) 데이터는 store 에 넣지 않는다.**
- `widgets/editor-tabs`, `widgets/file-tree`, `widgets/split-layout` — DnD·query 훅 소비.
- `features/*` — 순수 UI(탭 버튼, 트리 행, 스플릿 존 프리뷰).

**책임 경계 규칙 (IDE 에서 특히 중요)**

| 데이터 | 담당 |
|---|---|
| 파일 목록·파일 내용·git status·검색 결과 | TanStack Query (캐시·무효화) |
| 열린 탭, 스플릿 트리, 활성 pane, 패널 크기 | zustand(+persist) / react-resizable-panels `useDefaultLayout` |
| 편집 중 dirty 버퍼 | zustand 또는 에디터 인스턴스. Query 캐시에 넣지 않는다 |
| 빌드 로그·LSP 스트림 | Tauri `Channel` → zustand |

**도입 순서 제안**
1. Vite 8 + React 19 골격, Compiler 는 `compilationMode: 'annotation'` 으로 켜고 DevTools 배지로 검증 후 전체 모드 전환
2. `react-resizable-panels` 로 셸 레이아웃(사이드바 / 에디터 / 패널) 고정
3. `fs.api.ts` + Query 기본값(`networkMode: 'always'`, focusManager 대체) 세팅
4. Rust 워처 이벤트 → `invalidateQueries` 브릿지 1개 Provider 로 구현(디바운스·echo 무시 포함)
5. 탭 DnD(dnd-kit) → 스플릿 드롭 존 순으로 확장
6. 파일 트리: 크로스 서피스 드래그 요구 확정 후 react-arborist / headless-tree 결정

---

## 함정·주의

1. **Vite 8 + plugin-react v6 에서 `react({ babel: {...} })` 는 죽었다.** 인터넷 대부분의 React Compiler 설치 글이 이 형식이다. `@rolldown/plugin-babel` + `reactCompilerPreset()` 를 써야 한다.
2. **React Compiler 적용 여부를 "설정했으니 됐다"로 판단하지 않는다.** DevTools `Memo ✨` 또는 빌드 산출물의 `react/compiler-runtime` 로 실제 확인한다.
3. **react-resizable-panels 4.x 는 `PanelGroup`/`PanelResizeHandle`/`autoSaveId` 가 아니다.** `Group`/`Separator`/`useDefaultLayout` 이다. 구버전 예제 복붙 시 즉시 깨진다.
4. **`@dnd-kit/sortable` 은 2024-12 이후 릴리스가 없다.** 신 라인 `@dnd-kit/react` 는 0.5.0 (pre-1.0). 로드맵·deprecation 계획에 대한 **메인테이너 공식 답변은 확인되지 않았다**(GitHub Discussion #1842 는 2025-11 질문 이후 무응답, **미확인**). 레거시 라인 채택은 "당분간 안정적이나 신규 기능 유입 없음"을 감수하는 선택이다.
5. **TanStack Query 기본값이 Tauri 를 깨뜨린다.** `networkMode` 기본값('online')은 브라우저 오프라인 판정 시 IPC 쿼리를 pause 시킨다. `'always'` 로 바꾸고, 윈도우 포커스 refetch 는 `focusManager.setEventListener` 로 Tauri 이벤트에 다시 연결하거나 끈다.
6. **파일 워처 이벤트 폭주.** 저장 1회에 modify 이벤트가 여러 번 온다. Rust 쪽에서 디바운스·중복 제거 후 배치 emit 하지 않으면 `invalidateQueries` 폭풍으로 UI 가 정지한다. 앱 자체 쓰기의 echo 무시 플래그도 payload 에 넣는다.
7. **`listen()` 은 Promise 를 반환한다.** `useEffect` cleanup 에서 `promise.then((unlisten) => unlisten())` 로 정리하지 않으면 HMR 마다 리스너가 중첩된다.
8. **zustand v5 셀렉터.** 객체/배열 반환 셀렉터는 `useShallow` 없이 쓰면 무한에 가까운 리렌더를 만든다. v4 의 `createWithEqualityFn` 습관을 그대로 옮기지 않는다.
9. **DnD 시스템 이중화.** react-arborist(react-dnd) + 탭(dnd-kit) 조합은 트리→에디터 크로스 드래그를 막는다. 요구사항을 먼저 확정하고 트리 라이브러리를 고른다.
10. **`Panel`/`Separator` 를 래퍼 div 로 감싸지 않는다.** `Group` 직속 자식이어야 한다.
11. **탭바 DnD 는 `DragOverlay` 없이 만들지 않는다.** 스크롤 컨테이너 밖(다른 pane)으로 끌 수 없다.
12. **Compiler 와 조건부 훅/렌더 중 부수효과.** IDE 코드는 ref·DOM 측정이 많아 규칙 위반이 나기 쉽다. `eslint-plugin-react-hooks@7` `recommended-latest` 를 CI 게이트로 건다.
13. **버전 고정.** 위 표는 2026-08-06 스냅샷이다. `@dnd-kit/react`, `react-arborist` 처럼 변화가 빠른 패키지는 캐럿 대신 정확한 버전 고정 + 주기적 갱신을 권한다.

---

## 참고 링크

- React 릴리스: https://github.com/react/react/releases — 19.2.8 (2026-07-21) 확인
- React 버전 정책: https://react.dev/versions
- React Compiler 설치 문서: https://react.dev/learn/react-compiler/installation
- React Compiler 1.0 발표: https://react.dev/blog/2025/04/21/react-compiler-rc
- @vitejs/plugin-react README (React Compiler 정본 설정): https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react/README.md
- Vite 8 발표: https://vite.dev/blog/announcing-vite8
- Vite 릴리스: https://vite.dev/releases
- Vite 8 Beta / Rolldown 배경: https://voidzero.dev/posts/announcing-vite-8-beta
- Vite 8 Rust 번들러 보도: https://www.infoq.com/news/2026/05/vite-v8-rust/
- Vite Compiler 설정 변경 해설(커뮤니티): https://dev.to/recca0120/react-compiler-10-vite-8-the-right-way-to-install-after-vitejsplugin-react-v6-drops-babel-p0i
- zustand npm: https://www.npmjs.com/package/zustand
- zustand v5 셀렉터 베스트프랙티스 토론: https://github.com/pmndrs/zustand/discussions/2867
- zustand 미들웨어 우선순위 토론: https://github.com/pmndrs/zustand/discussions/2389
- TanStack Query 무효화 가이드: https://tanstack.com/query/v5/docs/framework/react/guides/query-invalidation
- TanStack Query mutation 기반 무효화: https://tanstack.com/query/v5/docs/framework/react/guides/invalidations-from-mutations
- 이벤트 기반 무효화 논의: https://github.com/TanStack/query/discussions/8618
- invalidateQueries 전역 리스너 부재 논의: https://github.com/TanStack/query/discussions/7798
- dnd-kit 신 문서(useSortable): https://dndkit.com/react/hooks/use-sortable
- dnd-kit 다중 정렬 리스트 가이드: https://dndkit.com/react/guides/multiple-sortable-lists/
- dnd-kit 레거시 SortableContext 문서: https://dndkit.com/legacy/presets/sortable/sortable-context/
- dnd-kit MultipleContainers 예제 소스: https://github.com/clauderic/dnd-kit/blob/master/stories/2%20-%20Presets/Sortable/MultipleContainers.tsx
- dnd-kit 로드맵 질문(무응답): https://github.com/clauderic/dnd-kit/discussions/1842
- react-arborist 저장소/릴리스: https://github.com/jameskerr/react-arborist/releases
- react-arborist 문서: https://react-arborist.netlify.app/
- headless-tree: https://headless-tree.lukasbach.com/
- react-complex-tree: https://www.npmjs.com/package/react-complex-tree
- react-resizable-panels 저장소: https://github.com/bvaughn/react-resizable-panels
- react-resizable-panels 문서 사이트: https://react-resizable-panels.vercel.app/
- TanStack Virtual: https://tanstack.com/virtual/latest
- Tauri 2 문서: https://v2.tauri.app/
- tauri crate: https://crates.io/crates/tauri (2.11.5)
- tauri-specta crate: https://crates.io/crates/tauri-specta (1.0.2)
