import { describe, expect, test } from 'bun:test'
import type { PaneNode, ProjectLayout, Tab } from '@shared/api/bindings'
import {
    collectAllPaneTabs,
    collectPaneTabs,
    findActiveTab,
    findPaneLeaf,
    findPaneTab,
    isPaneTreeEmpty,
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
