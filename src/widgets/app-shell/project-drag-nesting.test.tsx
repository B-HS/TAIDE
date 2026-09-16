import type { FC, PropsWithChildren } from 'react'
import { describe, expect, test } from 'bun:test'
import { DndContext, useDndContext, useDndMonitor, useDraggable, useDroppable } from '@dnd-kit/core'
import { act, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The spike contract §0.1 U-4 asked for before building the sidebar-to-slot drag: does a project
 * `DndContext` at the window root coexist with the per-slot tab `DndContext` inside every
 * `EditorArea`, or do the two leak into each other?
 *
 * dnd-kit keeps its whole registry in one React context (`@dnd-kit/core`'s `InternalContext`), so a
 * nested provider shadows the outer one for everything below it: `useDroppable` dispatches into the
 * nearest context's reducer, and a draggable's sensor activates the nearest context's drag. This
 * file pins both halves of that claim, because the feature is only safe if they hold.
 */
const CLICK_SUPPRESSION_TEARDOWN_MS = 60

const OUTER_ZONE_ID = 'outer-zone'
const INNER_ZONE_ID = 'inner-zone'
const OUTER_ITEM_ID = 'outer-item'
const INNER_ITEM_ID = 'inner-item'

const Probe: FC<{ label: string }> = ({ label }) => {
    const { droppableContainers } = useDndContext()
    return <div>{`${label}:${[...droppableContainers.keys()].join(',')}`}</div>
}

const Zone: FC<{ id: string }> = ({ id }) => {
    const { setNodeRef } = useDroppable({ id })
    return <div ref={setNodeRef} />
}

const Item: FC<{ id: string }> = ({ id }) => {
    const { setNodeRef, listeners, attributes } = useDraggable({ id })
    return (
        <button type='button' ref={setNodeRef} {...listeners} {...attributes}>
            {id}
        </button>
    )
}

const Monitor: FC<{ started: string[]; label: string }> = ({ started, label }) => {
    useDndMonitor({ onDragStart: ({ active }) => started.push(`${label}:${active.id}`) })
    return null
}

const Nested: FC<PropsWithChildren<{ outerStarted: string[]; innerStarted: string[] }>> = ({ outerStarted, innerStarted }) => (
    <DndContext>
        <Monitor started={outerStarted} label='outer' />
        <Probe label='outer' />
        <Zone id={OUTER_ZONE_ID} />
        <Item id={OUTER_ITEM_ID} />
        <DndContext>
            <Monitor started={innerStarted} label='inner' />
            <Probe label='inner' />
            <Zone id={INNER_ZONE_ID} />
            <Item id={INNER_ITEM_ID} />
        </DndContext>
    </DndContext>
)

const renderNested = () => {
    const outerStarted: string[] = []
    const innerStarted: string[] = []
    renderWithProviders(<Nested outerStarted={outerStarted} innerStarted={innerStarted} />)
    return { outerStarted, innerStarted }
}

/**
 * Down, straight back up, and then a real wait. Both halves matter beyond this file: a started drag
 * puts a capture-phase `click` swallower on the document, and dnd-kit only removes it 50ms after the
 * drag ends (`AbstractPointerSensor.detach`). Leaving either step out lets this file silently
 * neutralize the clicks of whichever test runs next, in this process (`test-conventions.md` §3).
 */
const dragOnce = (itemId: string) =>
    act(async () => {
        fireEvent.pointerDown(screen.getByRole('button', { name: itemId }), { isPrimary: true, button: 0 })
        fireEvent.pointerUp(document)
        await new Promise((resolve) => setTimeout(resolve, CLICK_SUPPRESSION_TEARDOWN_MS))
    })

describe('중첩 DndContext 격리 (2b 스파이크)', () => {
    test('바깥 컨텍스트는 안쪽 드롭존을 등록 목록에 갖지 않는다', () => {
        renderNested()

        expect(screen.getByText(/^outer:/).textContent).toBe(`outer:${OUTER_ZONE_ID}`)
        expect(screen.getByText(/^inner:/).textContent).toBe(`inner:${INNER_ZONE_ID}`)
    })

    test('안쪽 드래그는 안쪽에서만 시작된다 — 바깥으로 새지 않는다', async () => {
        const { outerStarted, innerStarted } = renderNested()

        await dragOnce(INNER_ITEM_ID)

        expect(innerStarted).toEqual([`inner:${INNER_ITEM_ID}`])
        expect(outerStarted).toEqual([])
    })

    test('바깥 드래그는 바깥에서만 시작된다 — 안쪽 드롭존에 반응할 수 없다', async () => {
        const { outerStarted, innerStarted } = renderNested()

        await dragOnce(OUTER_ITEM_ID)

        expect(outerStarted).toEqual([`outer:${OUTER_ITEM_ID}`])
        expect(innerStarted).toEqual([])
    })
})
