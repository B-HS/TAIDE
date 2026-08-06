import { describe, expect, test } from 'bun:test'
import { computeGraphLanes } from '@shared/lib/graph-lanes'

describe('computeGraphLanes', () => {
    test('선형 히스토리는 모두 레인 0 을 차지한다', () => {
        const nodes = computeGraphLanes([
            { id: 'c3', parents: ['c2'] },
            { id: 'c2', parents: ['c1'] },
            { id: 'c1', parents: [] },
        ])

        expect(nodes.map((node) => node.lane)).toEqual([0, 0, 0])
        expect(nodes.map((node) => node.color)).toEqual([0, 0, 0])
    })

    test('분기가 생기면 새 브랜치 팁은 새 레인을 받는다', () => {
        const nodes = computeGraphLanes([
            { id: 'c2', parents: ['c1'] },
            { id: 'b1', parents: ['c1'] },
            { id: 'c1', parents: [] },
        ])

        const byId = Object.fromEntries(nodes.map((node) => [node.id, node]))
        expect(byId.c2.lane).toBe(0)
        expect(byId.b1.lane).toBe(1)
        expect(byId.b1.continuesFromAbove).toBe(false)
        expect(byId.c1.lane).toBe(0)
    })

    test('머지 커밋의 두 번째 부모는 새 레인을 열고, 브랜치가 합류하면 레인이 회수된다', () => {
        const nodes = computeGraphLanes([
            { id: 'm', parents: ['c2', 'b1'] },
            { id: 'c2', parents: ['c1'] },
            { id: 'b1', parents: ['c1'] },
            { id: 'c1', parents: [] },
        ])

        const byId = Object.fromEntries(nodes.map((node) => [node.id, node]))
        expect(byId.m.lane).toBe(0)
        expect(byId.m.edges).toEqual([
            { toLane: 0, parentId: 'c2' },
            { toLane: 1, parentId: 'b1' },
        ])
        expect(byId.b1.lane).toBe(1)
        expect(byId.b1.edges).toEqual([{ toLane: 0, parentId: 'c1' }])
        expect(byId.c1.lane).toBe(0)

        const c1Index = nodes.findIndex((node) => node.id === 'c1')
        expect(nodes[c1Index].passthroughLanes).toEqual([])
    })

    test('12개를 초과하는 레인은 12색을 순환한다', () => {
        const parentCount = 13
        const parents = Array.from({ length: parentCount }, (_, index) => `p${index}`)
        const nodes = computeGraphLanes([{ id: 'm', parents }, ...parents.map((id) => ({ id, parents: [] }))])

        const byId = Object.fromEntries(nodes.map((node) => [node.id, node]))
        expect(byId.p12.lane).toBe(12)
        expect(byId.p12.color).toBe(0)
        expect(byId.p11.lane).toBe(11)
        expect(byId.p11.color).toBe(11)
    })

    test('레인이 회수된 뒤에는 재사용된다', () => {
        const nodes = computeGraphLanes([
            { id: 'm', parents: ['c2', 'b1'] },
            { id: 'c2', parents: ['c1'] },
            { id: 'b1', parents: ['c1'] },
            { id: 'c1', parents: ['c0'] },
            { id: 'd1', parents: ['c0'] },
            { id: 'c0', parents: [] },
        ])

        const byId = Object.fromEntries(nodes.map((node) => [node.id, node]))
        expect(byId.d1.lane).toBe(1)
    })
})
