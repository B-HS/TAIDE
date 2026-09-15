import { describe, expect, test } from 'bun:test'
import type { PaneNode, Tab } from '@shared/api/bindings'
import type { WindowPaneTree } from '@shared/lib/pane-tree'
import { collectClosableTabIdsInFocusedGroup, resolveFocusGroupPaneId } from '@widgets/editor-area/group-shortcut-targets'

const buildTab = (id: string, pinned = false): Tab => ({ id, kind: { kind: 'file', path: `/${id}.ts` }, title: id, pinned })

const buildSplitTree = (): PaneNode => ({
    node: 'split',
    id: 'root',
    dir: 'horizontal',
    sizes: [50, 50],
    children: [
        { node: 'leaf', id: 'left', tabs: [buildTab('a'), buildTab('b')], active: 'b' },
        { node: 'leaf', id: 'right', tabs: [buildTab('c')], active: 'c' },
    ],
})

const treeFocusedOn = (focusedPane: string): WindowPaneTree => ({ root: buildSplitTree(), focusedPane })

describe('resolveFocusGroupPaneId — 방향 타깃', () => {
    test('인접 그룹이 있으면 그 pane id 를 돌려준다', () => {
        expect(resolveFocusGroupPaneId(treeFocusedOn('left'), { kind: 'direction', direction: 'right' })).toBe('right')
    })

    test('그 방향에 인접 그룹이 없으면 null 이다(무동작 — 순환하지 않는다)', () => {
        expect(resolveFocusGroupPaneId(treeFocusedOn('left'), { kind: 'direction', direction: 'left' })).toBeNull()
        expect(resolveFocusGroupPaneId(treeFocusedOn('right'), { kind: 'direction', direction: 'right' })).toBeNull()
    })

    test('분할 축과 다른 방향(수평 분할에서 위·아래)도 null 이다', () => {
        expect(resolveFocusGroupPaneId(treeFocusedOn('left'), { kind: 'direction', direction: 'up' })).toBeNull()
        expect(resolveFocusGroupPaneId(treeFocusedOn('left'), { kind: 'direction', direction: 'down' })).toBeNull()
    })

    test('단일 그룹이면 어느 방향도 null 이다', () => {
        const single: WindowPaneTree = { root: { node: 'leaf', id: 'only', tabs: [buildTab('a')], active: 'a' }, focusedPane: 'only' }
        expect(resolveFocusGroupPaneId(single, { kind: 'direction', direction: 'right' })).toBeNull()
    })
})

describe('resolveFocusGroupPaneId — 위치 타깃(⌘1~9)', () => {
    test('리프 순서 기준 1-based 위치의 pane 을 돌려준다', () => {
        expect(resolveFocusGroupPaneId(treeFocusedOn('left'), { kind: 'position', position: 2 })).toBe('right')
    })

    test('이미 포커스된 그룹을 가리키면 null 이다(무동작)', () => {
        expect(resolveFocusGroupPaneId(treeFocusedOn('left'), { kind: 'position', position: 1 })).toBeNull()
        expect(resolveFocusGroupPaneId(treeFocusedOn('right'), { kind: 'position', position: 2 })).toBeNull()
    })

    test('그룹 수보다 큰 번호는 null 이다', () => {
        expect(resolveFocusGroupPaneId(treeFocusedOn('left'), { kind: 'position', position: 3 })).toBeNull()
        expect(resolveFocusGroupPaneId(treeFocusedOn('left'), { kind: 'position', position: 9 })).toBeNull()
    })
})

describe('resolveFocusGroupPaneId — 트리 부재', () => {
    test('paneTree 가 null 이면 두 타깃 모두 null 이다', () => {
        expect(resolveFocusGroupPaneId(null, { kind: 'direction', direction: 'right' })).toBeNull()
        expect(resolveFocusGroupPaneId(null, { kind: 'position', position: 1 })).toBeNull()
    })
})

describe('collectClosableTabIdsInFocusedGroup', () => {
    test('포커스된 그룹의 탭을 스트립 순서대로 돌려준다', () => {
        expect(collectClosableTabIdsInFocusedGroup(treeFocusedOn('left'))).toEqual(['a', 'b'])
    })

    test('고정 탭은 제외한다', () => {
        const tree: WindowPaneTree = {
            root: { node: 'leaf', id: 'only', tabs: [buildTab('a'), buildTab('pinned', true), buildTab('c')], active: 'a' },
            focusedPane: 'only',
        }
        expect(collectClosableTabIdsInFocusedGroup(tree)).toEqual(['a', 'c'])
    })

    test('전부 고정이면 빈 배열이다(닫기가 한 건도 일어나지 않는다)', () => {
        const tree: WindowPaneTree = {
            root: { node: 'leaf', id: 'only', tabs: [buildTab('a', true), buildTab('b', true)], active: 'a' },
            focusedPane: 'only',
        }
        expect(collectClosableTabIdsInFocusedGroup(tree)).toEqual([])
    })

    test('다른 그룹의 탭은 포함하지 않는다', () => {
        expect(collectClosableTabIdsInFocusedGroup(treeFocusedOn('right'))).toEqual(['c'])
    })

    test('paneTree 가 null 이거나 포커스 pane 이 트리에 없으면 빈 배열이다', () => {
        expect(collectClosableTabIdsInFocusedGroup(null)).toEqual([])
        expect(collectClosableTabIdsInFocusedGroup(treeFocusedOn('missing'))).toEqual([])
    })
})
