import { describe, expect, spyOn, test } from 'bun:test'
import type { Active, ClientRect, DragEndEvent, DroppableContainer, Over } from '@dnd-kit/core'
import { toast } from 'sonner'
import type { ProjectGroup, ProjectRef, SessionShellState } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import * as projectIpc from '@entities/project/project.ipc'
import { QUERY_KEY } from '@shared/constants/query-key'
import type { ShellSlotDropData } from '@shared/lib/project-drag'
import {
    PROJECT_DRAG_TYPE,
    PROJECT_GROUP_DRAG_TYPE,
    PROJECT_RAIL_DROPPABLE_ID,
    PROJECT_RAIL_DROP_TYPE,
    SHELL_SLOT_DROP_TYPE,
} from '@shared/lib/project-drag'
import { act, createTestQueryClient, renderHookWithProviders } from '@shared/testing/render'
import { useProjectDrag } from '@widgets/app-shell/use-project-drag'

/**
 * What the window's project drag does with a finished drag, one step above the pure mapping in
 * `shared/lib/project-drag.test.ts`: which command it sends, and which one it sends *instead* when
 * the release landed back on the rail.
 *
 * The drag events are built by hand rather than acted out with a pointer, because dnd-kit picks a
 * target by comparing element boxes and the DOM harness has none — every rect is zero
 * (`docs/memory/test-conventions.md` §5). Deciding the target is dnd-kit's job and is covered for
 * nesting in `project-drag-nesting.test.tsx`; what this file owns is what happens after it decided.
 */
const LEFT_SLOT_ID = 'shellslot-left'
const RIGHT_SLOT_ID = 'shellslot-right'
const LEFT_PROJECT_ID = 'project-left'
const RIGHT_PROJECT_ID = 'project-right'
const THIRD_PROJECT_ID = 'project-third'

const PROJECTS: ProjectRef[] = [
    { id: LEFT_PROJECT_ID, root: '/tmp/left', name: 'left' },
    { id: RIGHT_PROJECT_ID, root: '/tmp/right', name: 'right' },
    { id: THIRD_PROJECT_ID, root: '/tmp/third', name: 'third' },
]

const FIRST_GROUP_ID = 'group-first'
const SECOND_GROUP_ID = 'group-second'

const GROUPS: ProjectGroup[] = [
    { id: FIRST_GROUP_ID, name: 'first', members: [LEFT_PROJECT_ID] },
    { id: SECOND_GROUP_ID, name: 'second', members: [RIGHT_PROJECT_ID] },
]

const SHELL_STATE: SessionShellState = {
    tree: {
        node: 'split',
        dir: 'horizontal',
        sizes: [50, 50],
        children: [
            { node: 'leaf', slotId: LEFT_SLOT_ID, projectId: LEFT_PROJECT_ID },
            { node: 'leaf', slotId: RIGHT_SLOT_ID, projectId: RIGHT_PROJECT_ID },
        ],
    },
    focused: LEFT_SLOT_ID,
    windowChrome: { zen: false, sidebarRailCollapsed: false },
}

const EMPTY_RECT = { width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 }

const railIcon = (projectId: string) => ({ id: projectId, data: { current: { type: PROJECT_DRAG_TYPE, projectId } } })

const activeIcon = (projectId: string): Active => ({ ...railIcon(projectId), rect: { current: { initial: null, translated: null } } })

const overIcon = (projectId: string): Over => ({ ...railIcon(projectId), rect: EMPTY_RECT, disabled: false })

const groupHeader = (groupId: string) => ({ id: groupId, data: { current: { type: PROJECT_GROUP_DRAG_TYPE, groupId } } })

const activeHeader = (groupId: string): Active => ({ ...groupHeader(groupId), rect: { current: { initial: null, translated: null } } })

const overHeader = (groupId: string): Over => ({ ...groupHeader(groupId), rect: EMPTY_RECT, disabled: false })

const overSlotZone = (slotId: string, edge: ShellSlotDropData['edge']): Over => ({
    id: `${slotId}:${edge}`,
    data: { current: { type: SHELL_SLOT_DROP_TYPE, slotId, edge } },
    rect: EMPTY_RECT,
    disabled: false,
})

const CENTER_ZONE_ID = `${RIGHT_SLOT_ID}:center`

/**
 * One rail, one icon in it and one slot drop zone beside it, laid out the way the window is: the
 * rail down the left edge, the slot filling the rest below its header. Every rect is written out
 * because the DOM harness measures nothing (`docs/memory/test-conventions.md` §5), and geometry is
 * the whole subject here — the collision detection now answers differently depending on which of
 * these boxes the pointer is in.
 */
const RAIL_RECT: ClientRect = { top: 0, left: 0, right: 56, bottom: 600, width: 56, height: 600 }

const RAIL_ICON_RECT: ClientRect = { top: 8, left: 8, right: 48, bottom: 48, width: 40, height: 40 }

const SLOT_CENTER_ZONE_RECT: ClientRect = { top: 24, left: 56, right: 1000, bottom: 600, width: 944, height: 576 }

const RAIL_GAP_POINT = { x: 28, y: 300 }

const SLOT_HEADER_POINT = { x: 400, y: 8 }

const SLOT_CENTER_POINT = { x: 400, y: 300 }

const droppableContainerOf = (id: string, data: Record<string, unknown>): DroppableContainer => ({
    id,
    key: id,
    disabled: false,
    data: { current: data },
    node: { current: null },
    rect: { current: null },
})

const COLLISION_CONTAINERS: DroppableContainer[] = [
    droppableContainerOf(PROJECT_RAIL_DROPPABLE_ID, { type: PROJECT_RAIL_DROP_TYPE }),
    droppableContainerOf(THIRD_PROJECT_ID, { type: PROJECT_DRAG_TYPE, projectId: THIRD_PROJECT_ID }),
    droppableContainerOf(CENTER_ZONE_ID, { type: SHELL_SLOT_DROP_TYPE, slotId: RIGHT_SLOT_ID, edge: 'center' }),
]

const COLLISION_RECTS = new Map<string, ClientRect>([
    [PROJECT_RAIL_DROPPABLE_ID, RAIL_RECT],
    [THIRD_PROJECT_ID, RAIL_ICON_RECT],
    [CENTER_ZONE_ID, SLOT_CENTER_ZONE_RECT],
])

const dropOf = (active: Active, over: Over): DragEndEvent => ({
    activatorEvent: new Event('pointerdown'),
    active,
    collisions: null,
    delta: { x: 0, y: 0 },
    over,
})

/**
 * The rail's reorder is watched one layer in from the command table, unlike the slot command below:
 * `@entities/project/project.ipc` is faked process-wide by another file (`test-conventions.md` §3),
 * so a spy on `commands.projectReorder` would sit under a module that never calls it.
 */
const spyOnReorder = () => spyOn(projectIpc, 'reorderProjects').mockResolvedValue(null)

/** Drains the mutation's own promise chain so the IPC call it makes lands inside the `act` scope the drop was fired in. */
const settleMutation = () => new Promise((resolve) => setTimeout(resolve, 0))

/** Both reads are seeded and frozen, the way `app-shell.test.tsx` does it: another file's process-global `@entities/project/project.ipc` fake would otherwise win the mount refetch and empty the rail. */
const renderDrag = () => {
    const queryClient = createTestQueryClient()
    queryClient.setQueryDefaults(QUERY_KEY.PROJECT.LIST, { staleTime: Infinity, gcTime: Infinity })
    queryClient.setQueryDefaults(QUERY_KEY.PROJECT_GROUP.LIST, { staleTime: Infinity, gcTime: Infinity })
    queryClient.setQueryDefaults(QUERY_KEY.SESSION.SHELL_STATE, { staleTime: Infinity, gcTime: Infinity })
    queryClient.setQueryData(QUERY_KEY.PROJECT.LIST, PROJECTS)
    queryClient.setQueryData(QUERY_KEY.PROJECT_GROUP.LIST, GROUPS)
    queryClient.setQueryData(QUERY_KEY.SESSION.SHELL_STATE, SHELL_STATE)

    return renderHookWithProviders(() => useProjectDrag(), { queryClient })
}

const drop = async (result: { current: ReturnType<typeof useProjectDrag> }, event: DragEndEvent) => {
    await act(async () => {
        result.current.dndContextProps.onDragEnd(event)
        await settleMutation()
    })
}

describe('useProjectDrag 슬롯 드롭', () => {
    test('슬롯의 방향 드롭존에 떨구면 그 슬롯을 그 방향으로 분할한다', async () => {
        const openInSlot = spyOn(commands, 'projectOpenInSlot').mockResolvedValue({ status: 'ok', data: SHELL_STATE })
        const { result } = renderDrag()

        await drop(result, dropOf(activeIcon(THIRD_PROJECT_ID), overSlotZone(RIGHT_SLOT_ID, 'bottom')))

        expect(openInSlot).toHaveBeenCalledWith({ path: null, projectId: THIRD_PROJECT_ID, targetSlot: RIGHT_SLOT_ID, edge: 'bottom' })
    })

    test('가운데 드롭존은 그 슬롯의 프로젝트를 교체한다', async () => {
        const openInSlot = spyOn(commands, 'projectOpenInSlot').mockResolvedValue({ status: 'ok', data: SHELL_STATE })
        const { result } = renderDrag()

        await drop(result, dropOf(activeIcon(THIRD_PROJECT_ID), overSlotZone(LEFT_SLOT_ID, 'center')))

        expect(openInSlot).toHaveBeenCalledWith({ path: null, projectId: THIRD_PROJECT_ID, targetSlot: LEFT_SLOT_ID, edge: 'replace' })
    })

    test('이미 그 슬롯에 있는 프로젝트를 가운데에 떨구면 아무 요청도 하지 않는다', async () => {
        const openInSlot = spyOn(commands, 'projectOpenInSlot').mockResolvedValue({ status: 'ok', data: SHELL_STATE })
        const { result } = renderDrag()

        await drop(result, dropOf(activeIcon(LEFT_PROJECT_ID), overSlotZone(LEFT_SLOT_ID, 'center')))

        expect(openInSlot).not.toHaveBeenCalled()
    })

    test('서버가 거부하면 그 메시지를 토스트한다 — 중복 배치는 서버가 판정한다', async () => {
        spyOn(commands, 'projectOpenInSlot').mockResolvedValue({
            status: 'error',
            error: { code: 'InvalidArgument', message: 'project already in a slot' },
        })
        const toastError = spyOn(toast, 'error')
        const { result } = renderDrag()

        await drop(result, dropOf(activeIcon(RIGHT_PROJECT_ID), overSlotZone(LEFT_SLOT_ID, 'right')))

        expect(toastError).toHaveBeenCalledWith('project already in a slot')
    })
})

describe('useProjectDrag 레일 재정렬 회귀', () => {
    test('아이콘을 다른 아이콘 위에 떨구면 순서만 바꾸고 슬롯은 건드리지 않는다', async () => {
        const openInSlot = spyOn(commands, 'projectOpenInSlot').mockResolvedValue({ status: 'ok', data: SHELL_STATE })
        const reorder = spyOnReorder()
        const { result } = renderDrag()

        await drop(result, dropOf(activeIcon(LEFT_PROJECT_ID), overIcon(THIRD_PROJECT_ID)))

        expect(reorder.mock.calls.map((call) => call[0])).toEqual([[RIGHT_PROJECT_ID, THIRD_PROJECT_ID, LEFT_PROJECT_ID]])
        expect(openInSlot).not.toHaveBeenCalled()
    })

    test('제자리에 떨구면 순서를 쓰지 않는다', async () => {
        const reorder = spyOnReorder()
        const { result } = renderDrag()

        await drop(result, dropOf(activeIcon(LEFT_PROJECT_ID), overIcon(LEFT_PROJECT_ID)))

        expect(reorder).not.toHaveBeenCalled()
    })
})

describe('useProjectDrag 2단 정렬', () => {
    test('그룹 헤더를 다른 헤더 위에 떨구면 그룹 순서만 바꾼다', async () => {
        const reorderGroups = spyOn(commands, 'projectGroupReorder').mockResolvedValue({ status: 'ok', data: null })
        const reorder = spyOnReorder()
        const { result } = renderDrag()

        await drop(result, dropOf(activeHeader(SECOND_GROUP_ID), overHeader(FIRST_GROUP_ID)))

        expect(reorderGroups.mock.calls.map((call) => call[0])).toEqual([[SECOND_GROUP_ID, FIRST_GROUP_ID]])
        expect(reorder).not.toHaveBeenCalled()
    })

    test('헤더를 프로젝트 아이콘 위에 떨구면 어느 쪽 순서도 쓰지 않는다 — 두 단은 서로 섞이지 않는다', async () => {
        const reorderGroups = spyOn(commands, 'projectGroupReorder').mockResolvedValue({ status: 'ok', data: null })
        const reorder = spyOnReorder()
        const { result } = renderDrag()

        await drop(result, dropOf(activeHeader(FIRST_GROUP_ID), overIcon(THIRD_PROJECT_ID)))

        expect(reorderGroups).not.toHaveBeenCalled()
        expect(reorder).not.toHaveBeenCalled()
    })

    test('그룹 안의 멤버를 옮기면 전역 프로젝트 순서를 쓴다 — 멤버 순서는 session.projects 다', async () => {
        const reorderGroups = spyOn(commands, 'projectGroupReorder').mockResolvedValue({ status: 'ok', data: null })
        const reorder = spyOnReorder()
        const { result } = renderDrag()

        await drop(result, dropOf(activeIcon(RIGHT_PROJECT_ID), overIcon(LEFT_PROJECT_ID)))

        expect(reorder.mock.calls.map((call) => call[0])).toEqual([[RIGHT_PROJECT_ID, LEFT_PROJECT_ID, THIRD_PROJECT_ID]])
        expect(reorderGroups).not.toHaveBeenCalled()
    })
})

/** The ids dnd-kit would be handed for a release at `pointerCoordinates`, in the order it ranked them. */
const collisionsAt = (result: { current: ReturnType<typeof useProjectDrag> }, pointerCoordinates: { x: number; y: number }) =>
    result.current.dndContextProps
        .collisionDetection({
            active: activeIcon(LEFT_PROJECT_ID),
            collisionRect: EMPTY_RECT,
            droppableRects: COLLISION_RECTS,
            droppableContainers: COLLISION_CONTAINERS,
            pointerCoordinates,
        })
        .map((collision) => collision.id)

describe('useProjectDrag 충돌 판정', () => {
    test('포인터가 슬롯 드롭존 안이면 그 드롭존을 고른다', () => {
        const { result } = renderDrag()

        expect(collisionsAt(result, SLOT_CENTER_POINT)).toEqual([CENTER_ZONE_ID])
    })

    test('레일 밖에서 드롭존을 빗나가면 아무 대상도 고르지 않는다 — 슬롯 헤더·리사이저·타이틀바 위 드롭은 무동작이다', () => {
        const { result } = renderDrag()

        expect(collisionsAt(result, SLOT_HEADER_POINT)).toEqual([])
    })

    test('레일 안 아이콘 사이 틈은 가장 가까운 아이콘으로 재정렬을 유지한다 — 슬롯은 폴백 후보가 아니다', () => {
        const { result } = renderDrag()

        expect(collisionsAt(result, RAIL_GAP_POINT)).toEqual([THIRD_PROJECT_ID])
    })
})

describe('useProjectDrag 드래그 상태', () => {
    test('드래그가 시작되면 그 프로젝트를, 끝나면 아무것도 들고 있지 않는다', async () => {
        spyOnReorder()
        const { result } = renderDrag()

        act(() => result.current.dndContextProps.onDragStart({ activatorEvent: new Event('pointerdown'), active: activeIcon(LEFT_PROJECT_ID) }))

        expect(result.current.draggingProjectId).toBe(LEFT_PROJECT_ID)

        act(() => result.current.dndContextProps.onDragOver(dropOf(activeIcon(LEFT_PROJECT_ID), overSlotZone(RIGHT_SLOT_ID, 'top'))))

        expect(result.current.slotDropTarget).toEqual({ type: SHELL_SLOT_DROP_TYPE, slotId: RIGHT_SLOT_ID, edge: 'top' })

        await drop(result, dropOf(activeIcon(LEFT_PROJECT_ID), overIcon(LEFT_PROJECT_ID)))

        expect(result.current.draggingProjectId).toBeNull()
        expect(result.current.slotDropTarget).toBeNull()
    })
})
