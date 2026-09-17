import { afterEach, describe, expect, test } from 'bun:test'
import type { PaneNode, ProjectLayout, Tab } from '@shared/api/bindings'
import {
    activeFilePathOf,
    collectAllPaneTabs,
    collectPaneLeaves,
    collectPaneTabs,
    findActiveTab,
    findAdjacentPaneLeaf,
    findPaneLeaf,
    findPaneTab,
    currentWindowActiveFilePath,
    isPaneTreeEmpty,
    paneLeafAtPosition,
    resolveWindowPaneTree,
} from '@shared/lib/pane-tree'

const buildTab = (id: string): Tab => ({ id, kind: { kind: 'file', path: `/${id}.ts` }, title: id })

const buildTree = (): PaneNode => ({
    node: 'split',
    id: 'root',
    dir: 'horizontal',
    sizes: [50, 50],
    children: [
        { node: 'leaf', id: 'left', tabs: [buildTab('a'), buildTab('b')], active: 'b' },
        { node: 'leaf', id: 'right', tabs: [buildTab('c')], active: 'c' },
    ],
})

describe('findPaneLeaf', () => {
    test('중첩된 split 트리에서 id 가 일치하는 leaf 를 찾는다', () => {
        const leaf = findPaneLeaf(buildTree(), 'right')
        expect(leaf?.id).toBe('right')
    })

    test('존재하지 않는 paneId 는 null 을 반환한다', () => {
        expect(findPaneLeaf(buildTree(), 'missing')).toBeNull()
    })
})

describe('findPaneTab', () => {
    test('트리 전체를 순회해 tabId 로 tab 을 찾는다', () => {
        expect(findPaneTab(buildTree(), 'c')?.title).toBe('c')
    })

    test('존재하지 않는 tabId 는 null 을 반환한다', () => {
        expect(findPaneTab(buildTree(), 'missing')).toBeNull()
    })
})

describe('collectPaneTabs', () => {
    test('중첩된 split 트리의 모든 leaf 탭을 순서대로 모은다', () => {
        expect(collectPaneTabs(buildTree()).map((tab) => tab.id)).toEqual(['a', 'b', 'c'])
    })

    test('단일 leaf 노드는 그 탭만 반환한다', () => {
        const leaf: PaneNode = { node: 'leaf', id: 'only', tabs: [buildTab('x')], active: 'x' }
        expect(collectPaneTabs(leaf).map((tab) => tab.id)).toEqual(['x'])
    })
})

describe('findActiveTab', () => {
    test('leaf 의 active tabId 에 해당하는 tab 을 반환한다', () => {
        expect(findActiveTab(buildTree(), 'left')?.id).toBe('b')
    })

    test('active 가 없는 leaf 는 null 을 반환한다', () => {
        const tree = buildTree()
        if (tree.node === 'split') tree.children[0] = { ...tree.children[0], active: null } as PaneNode
        expect(findActiveTab(tree, 'left')).toBeNull()
    })
})

describe('activeFilePathOf', () => {
    test('포커스된 pane 의 활성 탭이 파일이면 그 경로를 반환한다', () => {
        expect(activeFilePathOf({ root: buildTree(), focusedPane: 'left' })).toBe('/b.ts')
    })

    test('활성 탭이 파일이 아니면 null 을 반환한다', () => {
        const root: PaneNode = {
            node: 'leaf',
            id: 'only',
            tabs: [{ id: 'term', kind: { kind: 'terminal', sessionId: 's-1' }, title: 'terminal' }],
            active: 'term',
        }
        expect(activeFilePathOf({ root, focusedPane: 'only' })).toBeNull()
    })

    test('트리가 없으면(레이아웃 미로딩) null 을 반환한다', () => {
        expect(activeFilePathOf(null)).toBeNull()
        expect(activeFilePathOf(undefined)).toBeNull()
    })

    test('포커스된 pane 이 트리에 없으면 null 을 반환한다', () => {
        expect(activeFilePathOf({ root: buildTree(), focusedPane: 'missing' })).toBeNull()
    })
})

const buildLayout = (overrides: Partial<ProjectLayout> = {}): ProjectLayout => ({
    version: 2,
    root: { node: 'leaf', id: 'main-leaf', tabs: [buildTab('main-a')], active: 'main-a' },
    focusedPane: 'main-leaf',
    ...overrides,
})

describe('resolveWindowPaneTree', () => {
    test('main 창은 layout.root/focusedPane 을 그대로 반환한다', () => {
        const layout = buildLayout()
        expect(resolveWindowPaneTree(layout, { kind: 'main' })).toEqual({ root: layout.root, focusedPane: 'main-leaf' })
    })

    test('보조 창은 자기 slot 의 AuxWindowLayout 을 반환한다', () => {
        const auxRoot: PaneNode = { node: 'leaf', id: 'aux-leaf', tabs: [buildTab('aux-a')], active: 'aux-a' }
        const layout = buildLayout({ auxiliaryWindows: [{ slot: 1, root: auxRoot, focusedPane: 'aux-leaf' }] })
        expect(resolveWindowPaneTree(layout, { kind: 'auxiliary', projectId: 'prj-1', windowSlot: 1 })).toEqual({
            root: auxRoot,
            focusedPane: 'aux-leaf',
        })
    })

    test('slot 이 layout 에 없는 보조 창은 null 을 반환한다', () => {
        const layout = buildLayout({ auxiliaryWindows: [] })
        expect(resolveWindowPaneTree(layout, { kind: 'auxiliary', projectId: 'prj-1', windowSlot: 1 })).toBeNull()
    })

    test('auxiliaryWindows 필드 자체가 없어도(v1 레이아웃) null 을 반환한다', () => {
        const layout = buildLayout()
        expect(resolveWindowPaneTree(layout, { kind: 'auxiliary', projectId: 'prj-1', windowSlot: 1 })).toBeNull()
    })

    test('focusedPane 이 트리에 없으면(닫힌 pane) 첫 leaf 로 폴백한다', () => {
        const layout = buildLayout({ root: buildTree(), focusedPane: 'closed-pane' })
        expect(resolveWindowPaneTree(layout, { kind: 'main' })).toEqual({ root: layout.root, focusedPane: 'left' })
    })

    test('focusedPane 이 트리에 있으면 그대로 둔다 (첫 leaf 로 바꾸지 않는다)', () => {
        const layout = buildLayout({ root: buildTree(), focusedPane: 'right' })
        expect(resolveWindowPaneTree(layout, { kind: 'main' })).toEqual({ root: layout.root, focusedPane: 'right' })
    })

    test('보조 창의 focusedPane 이 stale 이면 그 창 트리의 첫 leaf 로 폴백한다 (main 은 영향 없음)', () => {
        const layout = buildLayout({
            auxiliaryWindows: [{ slot: 1, root: buildGridTree(), focusedPane: 'closed-pane' }],
        })
        expect(resolveWindowPaneTree(layout, { kind: 'auxiliary', projectId: 'prj-1', windowSlot: 1 })?.focusedPane).toBe('top-left')
        expect(resolveWindowPaneTree(layout, { kind: 'main' })?.focusedPane).toBe('main-leaf')
    })

    test('leaf 가 하나도 없는 트리는 원래 focusedPane 을 유지한다', () => {
        const layout = buildLayout({ root: { node: 'split', id: 'root', dir: 'horizontal', sizes: [], children: [] }, focusedPane: 'closed-pane' })
        expect(resolveWindowPaneTree(layout, { kind: 'main' })?.focusedPane).toBe('closed-pane')
    })
})

describe('isPaneTreeEmpty', () => {
    test('탭이 없는 leaf 는 비어있다고 판단한다', () => {
        expect(isPaneTreeEmpty({ node: 'leaf', id: 'l', tabs: [], active: null })).toBe(true)
    })

    test('탭이 있는 leaf 는 비어있지 않다고 판단한다', () => {
        expect(isPaneTreeEmpty({ node: 'leaf', id: 'l', tabs: [buildTab('a')], active: 'a' })).toBe(false)
    })

    test('split 노드는 비어있지 않다고 판단한다', () => {
        expect(isPaneTreeEmpty(buildTree())).toBe(false)
    })
})

describe('collectAllPaneTabs', () => {
    test('main 트리와 모든 보조 창 트리의 탭을 전부 모은다', () => {
        const layout = buildLayout({
            root: buildTree(),
            auxiliaryWindows: [
                { slot: 1, root: { node: 'leaf', id: 'aux-1', tabs: [buildTab('aux-x')], active: 'aux-x' }, focusedPane: 'aux-1' },
                { slot: 2, root: { node: 'leaf', id: 'aux-2', tabs: [buildTab('aux-y')], active: 'aux-y' }, focusedPane: 'aux-2' },
            ],
        })
        expect(collectAllPaneTabs(layout).map((tab) => tab.id)).toEqual(['a', 'b', 'c', 'aux-x', 'aux-y'])
    })

    test('보조 창이 없으면 main 트리의 탭만 반환한다', () => {
        const layout = buildLayout()
        expect(collectAllPaneTabs(layout).map((tab) => tab.id)).toEqual(['main-a'])
    })
})

const buildLeaf = (id: string): PaneNode => ({ node: 'leaf', id, tabs: [buildTab(`${id}-tab`)], active: `${id}-tab` })

/**
 * Two stacked columns side by side — the smallest tree that exercises every branch of
 * `findAdjacentPaneLeaf`: an axis mismatch (a vertical parent ignored while moving horizontally),
 * a sibling lookup one level further out, and the "which end of the neighbour subtree" choice.
 *
 * ```
 * root (horizontal)
 * ├── left  (vertical)  : top-left, bottom-left
 * └── right (vertical)  : top-right, bottom-right
 * ```
 */
const buildGridTree = (): PaneNode => ({
    node: 'split',
    id: 'root',
    dir: 'horizontal',
    sizes: [50, 50],
    children: [
        { node: 'split', id: 'left-column', dir: 'vertical', sizes: [50, 50], children: [buildLeaf('top-left'), buildLeaf('bottom-left')] },
        { node: 'split', id: 'right-column', dir: 'vertical', sizes: [50, 50], children: [buildLeaf('top-right'), buildLeaf('bottom-right')] },
    ],
})

describe('collectPaneLeaves', () => {
    test('중첩 split 의 모든 leaf 를 DFS 순서(좌→우·상→하)로 모은다', () => {
        expect(collectPaneLeaves(buildGridTree()).map((leaf) => leaf.id)).toEqual(['top-left', 'bottom-left', 'top-right', 'bottom-right'])
    })

    test('단일 leaf 트리는 그 leaf 하나만 반환한다', () => {
        expect(collectPaneLeaves(buildLeaf('only')).map((leaf) => leaf.id)).toEqual(['only'])
    })
})

describe('paneLeafAtPosition', () => {
    test('1-based 위치로 그룹을 찾는다 (⌘1~⌘9 라벨과 같은 번호)', () => {
        expect(paneLeafAtPosition(buildGridTree(), 1)?.id).toBe('top-left')
        expect(paneLeafAtPosition(buildGridTree(), 4)?.id).toBe('bottom-right')
    })

    test('그룹 수보다 큰 위치나 0 이하는 null 을 반환한다', () => {
        expect(paneLeafAtPosition(buildGridTree(), 5)).toBeNull()
        expect(paneLeafAtPosition(buildGridTree(), 0)).toBeNull()
    })
})

describe('findAdjacentPaneLeaf', () => {
    test('같은 split 의 좌우 형제를 찾는다', () => {
        expect(findAdjacentPaneLeaf(buildTree(), 'left', 'right')?.id).toBe('right')
        expect(findAdjacentPaneLeaf(buildTree(), 'right', 'left')?.id).toBe('left')
    })

    test('가장자리에서는 순환하지 않고 null 을 반환한다', () => {
        expect(findAdjacentPaneLeaf(buildTree(), 'left', 'left')).toBeNull()
        expect(findAdjacentPaneLeaf(buildTree(), 'right', 'right')).toBeNull()
    })

    test('요청한 방향의 축과 다른 split 밖에 없으면 null 을 반환한다', () => {
        expect(findAdjacentPaneLeaf(buildTree(), 'left', 'up')).toBeNull()
        expect(findAdjacentPaneLeaf(buildTree(), 'left', 'down')).toBeNull()
    })

    test('세로 split 안에서는 위아래 형제를 찾는다', () => {
        expect(findAdjacentPaneLeaf(buildGridTree(), 'top-left', 'down')?.id).toBe('bottom-left')
        expect(findAdjacentPaneLeaf(buildGridTree(), 'bottom-left', 'up')?.id).toBe('top-left')
    })

    test('축이 다른 부모는 건너뛰고 바깥쪽 split 에서 이웃 서브트리를 찾는다', () => {
        expect(findAdjacentPaneLeaf(buildGridTree(), 'bottom-left', 'right')?.id).toBe('top-right')
        expect(findAdjacentPaneLeaf(buildGridTree(), 'top-right', 'left')?.id).toBe('bottom-left')
    })

    test('오른쪽으로 갈 때는 이웃 서브트리의 첫 leaf, 왼쪽으로 갈 때는 마지막 leaf 에 들어간다', () => {
        expect(findAdjacentPaneLeaf(buildGridTree(), 'top-left', 'right')?.id).toBe('top-right')
        expect(findAdjacentPaneLeaf(buildGridTree(), 'bottom-right', 'left')?.id).toBe('bottom-left')
    })

    test('셋 이상 나란한 split 에서는 가장 가까운 형제만 고른다', () => {
        const row: PaneNode = {
            node: 'split',
            id: 'row',
            dir: 'horizontal',
            sizes: [33, 33, 34],
            children: [buildLeaf('one'), buildLeaf('two'), buildLeaf('three')],
        }
        expect(findAdjacentPaneLeaf(row, 'one', 'right')?.id).toBe('two')
        expect(findAdjacentPaneLeaf(row, 'three', 'left')?.id).toBe('two')
    })

    test('단일 leaf 트리와 존재하지 않는 paneId 는 null 을 반환한다', () => {
        expect(findAdjacentPaneLeaf(buildLeaf('only'), 'only', 'right')).toBeNull()
        expect(findAdjacentPaneLeaf(buildGridTree(), 'missing', 'right')).toBeNull()
    })
})

const MAIN_WINDOW_URL = '/'
const AUXILIARY_WINDOW_URL = '/?projectId=prj-1&windowSlot=1'

/**
 * `currentWindowActiveFilePath` reads `getWindowContext()`, which parses `location.search`, and the
 * harness pins that to empty so every test starts in the main window
 * (`docs/memory/test-conventions.md` §4). `history.replaceState` is the one same-document way to move
 * it without a navigation, and the URL is put back after each case because bun shares one process —
 * and so one `location` — across every test file.
 */
const enterWindowUrl = (url: string) => window.history.replaceState({}, '', url)

afterEach(() => enterWindowUrl(MAIN_WINDOW_URL))

describe('currentWindowActiveFilePath', () => {
    const buildSplitWindowsLayout = () =>
        buildLayout({
            auxiliaryWindows: [
                { slot: 1, root: { node: 'leaf', id: 'aux-leaf', tabs: [buildTab('aux-a')], active: 'aux-a' }, focusedPane: 'aux-leaf' },
            ],
        })

    test('main 창에서는 main 트리의 활성 파일을 반환한다', () => {
        expect(currentWindowActiveFilePath(buildSplitWindowsLayout())).toBe('/main-a.ts')
    })

    test('보조 창에서는 그 창 트리의 활성 파일을 반환한다 (main 창 파일이 아니라)', () => {
        enterWindowUrl(AUXILIARY_WINDOW_URL)
        expect(currentWindowActiveFilePath(buildSplitWindowsLayout())).toBe('/aux-a.ts')
    })

    test('slot 이 레이아웃에 없는 보조 창은 main 트리로 폴백하지 않고 null 을 반환한다', () => {
        enterWindowUrl(AUXILIARY_WINDOW_URL)
        expect(currentWindowActiveFilePath(buildLayout({ auxiliaryWindows: [] }))).toBeNull()
    })

    test('레이아웃을 아직 못 읽었으면 null 을 반환한다', () => {
        expect(currentWindowActiveFilePath(undefined)).toBeNull()
    })

    test('활성 탭이 파일이 아니면 null 을 반환한다', () => {
        const terminalTab: Tab = { id: 'term', kind: { kind: 'terminal', sessionId: '', cwd: null }, title: 'term' }
        const layout = buildLayout({ root: { node: 'leaf', id: 'main-leaf', tabs: [terminalTab], active: 'term' } })
        expect(currentWindowActiveFilePath(layout)).toBeNull()
    })
})
