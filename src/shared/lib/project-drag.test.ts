import { describe, expect, test } from 'bun:test'
import type { ShellSlotTree } from '@shared/api/bindings'
import type { ProjectDragData, ShellSlotDropData } from '@shared/lib/project-drag'
import {
    PROJECT_DRAG_TYPE,
    SHELL_SLOT_DROP_TYPE,
    resolveProjectGroupReorder,
    resolveProjectReorder,
    resolveShellSlotDrop,
} from '@shared/lib/project-drag'

/**
 * The two decisions a finished project drag makes, away from dnd-kit entirely. Both are pure on
 * purpose: the collision detection that picks the target needs real element boxes, which the DOM
 * harness does not have (`docs/memory/test-conventions.md` §5), so the mapping from "this zone" to
 * "this command" is what has to be pinned here — and it is also the half that would silently send
 * the wrong edge.
 */
const LEFT_SLOT_ID = 'shellslot-left'
const RIGHT_SLOT_ID = 'shellslot-right'
const LEFT_PROJECT_ID = 'project-left'
const RIGHT_PROJECT_ID = 'project-right'

const TREE: ShellSlotTree = {
    node: 'split',
    dir: 'horizontal',
    sizes: [50, 50],
    children: [
        { node: 'leaf', slotId: LEFT_SLOT_ID, projectId: LEFT_PROJECT_ID },
        { node: 'leaf', slotId: RIGHT_SLOT_ID, projectId: RIGHT_PROJECT_ID },
    ],
}

const dragOf = (projectId: string): ProjectDragData => ({ type: PROJECT_DRAG_TYPE, projectId })

const dropOf = (slotId: string, edge: ShellSlotDropData['edge']): ShellSlotDropData => ({ type: SHELL_SLOT_DROP_TYPE, slotId, edge })

describe('resolveShellSlotDrop', () => {
    test('네 방향 드롭존은 같은 이름의 슬롯 edge 로 간다', () => {
        const edges = (['left', 'right', 'top', 'bottom'] as const).map(
            (edge) => resolveShellSlotDrop(dragOf(LEFT_PROJECT_ID), dropOf(RIGHT_SLOT_ID, edge), TREE)?.edge,
        )

        expect(edges).toEqual(['left', 'right', 'top', 'bottom'])
    })

    test('가운데 드롭존은 replace 로 간다 — 탭 분할의 center 와 이름이 다르다', () => {
        expect(resolveShellSlotDrop(dragOf(LEFT_PROJECT_ID), dropOf(RIGHT_SLOT_ID, 'center'), TREE)).toEqual({
            projectId: LEFT_PROJECT_ID,
            targetSlot: RIGHT_SLOT_ID,
            edge: 'replace',
        })
    })

    test('이미 그 슬롯에 있는 프로젝트를 가운데에 떨구면 아무것도 요청하지 않는다', () => {
        expect(resolveShellSlotDrop(dragOf(LEFT_PROJECT_ID), dropOf(LEFT_SLOT_ID, 'center'), TREE)).toBeNull()
    })

    test('같은 슬롯이라도 방향 분할은 서버가 판단하도록 그대로 보낸다', () => {
        expect(resolveShellSlotDrop(dragOf(LEFT_PROJECT_ID), dropOf(LEFT_SLOT_ID, 'right'), TREE)?.edge).toBe('right')
    })

    test('드래그나 드롭 어느 한쪽이 프로젝트 드래그가 아니면 요청이 없다', () => {
        expect(resolveShellSlotDrop(null, dropOf(RIGHT_SLOT_ID, 'left'), TREE)).toBeNull()
        expect(resolveShellSlotDrop(dragOf(LEFT_PROJECT_ID), null, TREE)).toBeNull()
    })
})

describe('resolveProjectReorder', () => {
    const ORDER = ['a', 'b', 'c']

    test('다른 아이콘 위에 떨구면 그 자리로 옮긴 순서를 돌려준다', () => {
        expect(resolveProjectReorder(ORDER, 'a', 'c')).toEqual(['b', 'c', 'a'])
    })

    test('제자리·rail 밖·모르는 id 는 전부 재정렬이 아니다', () => {
        expect(resolveProjectReorder(ORDER, 'a', 'a')).toBeNull()
        expect(resolveProjectReorder(ORDER, 'a', null)).toBeNull()
        expect(resolveProjectReorder(ORDER, 'a', `${LEFT_SLOT_ID}:left`)).toBeNull()
        expect(resolveProjectReorder(ORDER, 'zz', 'b')).toBeNull()
    })

    test('원본 배열을 건드리지 않는다', () => {
        resolveProjectReorder(ORDER, 'a', 'c')

        expect(ORDER).toEqual(['a', 'b', 'c'])
    })
})

describe('resolveProjectGroupReorder', () => {
    const GROUP_ORDER = ['group-a', 'group-b', 'group-c']

    test('다른 헤더 위에 떨구면 그 자리로 옮긴 그룹 순서를 돌려준다', () => {
        expect(resolveProjectGroupReorder(GROUP_ORDER, 'group-c', 'group-a')).toEqual(['group-c', 'group-a', 'group-b'])
    })

    test('그룹 목록에 없는 대상 위에 떨구면 아무것도 아니다 — 멤버 아이콘 위에서 떼도 그룹 순서는 그대로다', () => {
        expect(resolveProjectGroupReorder(GROUP_ORDER, 'group-a', 'a')).toBeNull()
        expect(resolveProjectGroupReorder(GROUP_ORDER, 'group-a', null)).toBeNull()
        expect(resolveProjectGroupReorder(GROUP_ORDER, 'group-a', 'group-a')).toBeNull()
    })
})
