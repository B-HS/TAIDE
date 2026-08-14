import { describe, expect, test } from 'bun:test'
import type { PaneNode, Tab } from '@shared/api/bindings'
import { collectPaneTabs, findActiveTab, findPaneLeaf, findPaneTab } from '@shared/lib/pane-tree'

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
