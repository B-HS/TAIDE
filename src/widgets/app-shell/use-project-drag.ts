import { useState } from 'react'
import { PointerSensor, closestCenter, pointerWithin, useSensor, useSensors } from '@dnd-kit/core'
import type { CollisionDetection, DragEndEvent, DragOverEvent, DragStartEvent } from '@dnd-kit/core'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ProjectId } from '@shared/api/bindings'
import { projectListQueryOptions, useReorderProjects } from '@entities/project/project.query'
import { projectGroupListQueryOptions, useReorderProjectGroups } from '@entities/project-group/project-group.query'
import { shellStateQueryOptions, useOpenProjectInSlot } from '@entities/session/session.query'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import type { ShellSlotDropData } from '@shared/lib/project-drag'
import {
    projectDragDataOf,
    projectGroupDragDataOf,
    projectRailDropDataOf,
    resolveProjectGroupReorder,
    resolveProjectReorder,
    resolveShellSlotDrop,
    shellSlotDropDataOf,
} from '@shared/lib/project-drag'

const DRAG_ACTIVATION_DISTANCE_PX = 4

type CollisionDetectionArgs = Parameters<CollisionDetection>[0]

/**
 * Whether the pointer is over the rail itself, read from the rect dnd-kit measured for the droppable
 * `AppSidebar` puts on its `nav`. `false` whenever the rail is not on screen to be measured — Zen and
 * a collapsed rail both take it away, and neither can be the source of a project drag anyway.
 */
const isPointerOverProjectRail = ({ droppableContainers, droppableRects, pointerCoordinates }: CollisionDetectionArgs) => {
    if (!pointerCoordinates) return false
    const rail = droppableContainers.find((container) => projectRailDropDataOf(container))
    const rect = rail ? droppableRects.get(rail.id) : undefined
    if (!rect) return false
    return (
        pointerCoordinates.x >= rect.left &&
        pointerCoordinates.x <= rect.right &&
        pointerCoordinates.y >= rect.top &&
        pointerCoordinates.y <= rect.bottom
    )
}

/**
 * Pointer-first, because a slot's five drop zones tile it completely and only the one the pointer is
 * actually inside may win — the same reason the tab drag one level down uses `pointerWithin`.
 *
 * The `closestCenter` fallback is what keeps the rail's reorder as forgiving as it was while it had
 * a context to itself: a pointer in the gap between two icons still sorts. It is allowed *only while
 * the pointer is over the rail*, because "nearest centre" answers from anywhere in the window — a
 * release on a slot header, on a resizer or on the title bar would otherwise be read as a reorder of
 * whichever icon happened to be nearest, and every one of those is a release the user meant to do
 * nothing. Outside the rail, missing every drop zone means exactly that: no collision at all.
 *
 * The rail's own droppable is never a candidate (it is only there to be measured), and the slot
 * zones are dropped from the fallback as well — a slot must be split by pointing at it, not by being
 * the closest thing to a pointer that is inside the rail.
 */
const projectCollisionDetection: CollisionDetection = (args) => {
    const targets = args.droppableContainers.filter((container) => !projectRailDropDataOf(container))
    const pointerCollisions = pointerWithin({ ...args, droppableContainers: targets })
    if (pointerCollisions.length > 0) return pointerCollisions
    if (!isPointerOverProjectRail(args)) return []
    return closestCenter({ ...args, droppableContainers: targets.filter((container) => !shellSlotDropDataOf(container)) })
}

/**
 * The window's single project drag (contract §0.1 U-4): one `DndContext` at the root of `AppShell`
 * owning both the rail's reorder — which used to have a context of its own inside `AppSidebar` — and
 * the drop zones every shell slot renders, because a drag that starts on a rail icon and ends inside
 * a slot is one drag and must live in one context.
 *
 * Each slot's own tab `DndContext` (`editor-area.tsx`) stays nested *inside* this one. dnd-kit keeps
 * its registry in a React context, so the nested provider shadows this one for its whole subtree: a
 * tab's drop zones register only with the inner context, and a tab's sensor activates only the inner
 * drag. Neither drag can see the other's targets, which is what makes the nesting safe rather than
 * merely convenient — `project-drag-nesting.test.tsx` pins both halves of that claim.
 */
export const useProjectDrag = () => {
    const [draggingProjectId, setDraggingProjectId] = useState<ProjectId | null>(null)
    const [slotDropTarget, setSlotDropTarget] = useState<ShellSlotDropData | null>(null)

    const { data: projects = [] } = useQuery(projectListQueryOptions())
    const { data: groups = [] } = useQuery(projectGroupListQueryOptions())
    const { data: shellState } = useQuery(shellStateQueryOptions())
    const { mutate: reorderProjects } = useReorderProjects()
    const { mutate: reorderProjectGroups } = useReorderProjectGroups()
    const { mutate: openProjectInSlot } = useOpenProjectInSlot()

    const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: DRAG_ACTIVATION_DISTANCE_PX } }))

    const clearDragState = () => {
        setDraggingProjectId(null)
        setSlotDropTarget(null)
    }

    const reorderRail = ({ active, over }: DragEndEvent) => {
        const order = resolveProjectReorder(
            projects.map((project) => project.id),
            String(active.id),
            over ? String(over.id) : null,
        )
        if (order) reorderProjects(order)
    }

    /**
     * The rail's outer tier (d-62 2c). A header drag never reaches a slot — the drop zones are only
     * rendered while a *project* is in flight — so it is decided before the slot branch is consulted
     * at all, and the whole group list is sent because `project_group_reorder` takes an order, not a
     * move.
     */
    const reorderGroups = ({ active, over }: DragEndEvent) => {
        const order = resolveProjectGroupReorder(
            groups.map((group) => group.id),
            String(active.id),
            over ? String(over.id) : null,
        )
        if (order) reorderProjectGroups(order)
    }

    const handleDragEnd = (event: DragEndEvent) => {
        clearDragState()
        if (projectGroupDragDataOf(event.active)) return reorderGroups(event)

        const request = resolveShellSlotDrop(projectDragDataOf(event.active), shellSlotDropDataOf(event.over), shellState?.tree ?? null)
        if (!request) return reorderRail(event)
        openProjectInSlot({ path: null, ...request }, { onError: (error) => toast.error(describeIpcError(error)) })
    }

    return {
        draggingProjectId,
        slotDropTarget,
        dndContextProps: {
            sensors,
            collisionDetection: projectCollisionDetection,
            onDragStart: ({ active }: DragStartEvent) => setDraggingProjectId(projectDragDataOf(active)?.projectId ?? null),
            onDragOver: ({ over }: DragOverEvent) => setSlotDropTarget(shellSlotDropDataOf(over)),
            onDragEnd: handleDragEnd,
            onDragCancel: clearDragState,
        },
    }
}
