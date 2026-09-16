import type { DataRef } from '@dnd-kit/core'
import { arrayMove } from '@dnd-kit/sortable'
import type { DropEdge, ProjectGroupId, ProjectId, ShellSlotEdge, ShellSlotId, ShellSlotTree } from '@shared/api/bindings'
import { projectOfShellSlot } from '@shared/lib/shell-slot'

/** Tags the rail icon's drag so the one window-level `DndContext` can tell a project apart from anything else that might join it later (contract §0.1 U-4). */
export const PROJECT_DRAG_TYPE = 'project'

/** The outer of the rail's two sortable tiers (d-62 2c): a group header reorders the groups, a project icon reorders `session.projects`, and the payload's `type` is what tells the one `DndContext` which of the two just ended. */
export const PROJECT_GROUP_DRAG_TYPE = 'projectGroup'

export const SHELL_SLOT_DROP_TYPE = 'shellSlot'

/**
 * The rail container itself, registered as a droppable by `AppSidebar` for one reason only: so the
 * window's collision detection can ask dnd-kit where the rail is (`use-project-drag.ts`). Nothing is
 * ever dropped *on* it, and every candidate list drops it before a collision is picked.
 */
export const PROJECT_RAIL_DROP_TYPE = 'projectRail'

export const PROJECT_RAIL_DROPPABLE_ID = 'project-rail'

export type ProjectDragData = { type: typeof PROJECT_DRAG_TYPE; projectId: ProjectId }

export type ProjectGroupDragData = { type: typeof PROJECT_GROUP_DRAG_TYPE; groupId: ProjectGroupId }

export type ShellSlotDropData = { type: typeof SHELL_SLOT_DROP_TYPE; slotId: ShellSlotId; edge: DropEdge }

export type ProjectRailDropData = { type: typeof PROJECT_RAIL_DROP_TYPE }

export type ShellSlotDropRequest = { projectId: ProjectId; targetSlot: ShellSlotId; edge: ShellSlotEdge }

/** Whatever dnd-kit hands back — the active draggable, the droppable under the pointer, a registered container — carries its payload the same way, and that is all the readers below need of it. */
type DragPayloadCarrier = { data: DataRef } | null | undefined

/**
 * The drop zones speak the editor's vocabulary (`DropEdge`, shared with the tab split one level
 * down) while the command speaks the slot tree's (`ShellSlotEdge`), and the two differ in exactly
 * one member: dropping on a pane's centre appends a tab, dropping on a slot's centre *replaces* the
 * project in it.
 */
const SHELL_SLOT_EDGE_BY_DROP_EDGE: Record<DropEdge, ShellSlotEdge> = {
    left: 'left',
    right: 'right',
    top: 'top',
    bottom: 'bottom',
    center: 'replace',
}

export const projectDragDataOf = (carrier: DragPayloadCarrier) => {
    const data = carrier?.data.current as ProjectDragData | undefined
    return data?.type === PROJECT_DRAG_TYPE ? data : null
}

export const projectGroupDragDataOf = (carrier: DragPayloadCarrier) => {
    const data = carrier?.data.current as ProjectGroupDragData | undefined
    return data?.type === PROJECT_GROUP_DRAG_TYPE ? data : null
}

export const shellSlotDropDataOf = (carrier: DragPayloadCarrier) => {
    const data = carrier?.data.current as ShellSlotDropData | undefined
    return data?.type === SHELL_SLOT_DROP_TYPE ? data : null
}

export const projectRailDropDataOf = (carrier: DragPayloadCarrier) => {
    const data = carrier?.data.current as ProjectRailDropData | undefined
    return data?.type === PROJECT_RAIL_DROP_TYPE ? data : null
}

/**
 * What a finished project drag asks the server for, or `null` when it asks for nothing.
 *
 * Only one refusal is decided here: dropping a project onto the centre of the very slot it already
 * fills is a request to replace itself, which the server would answer with a perfectly correct error
 * about a project already being in a slot — a toast the user would read as a malfunction. Every
 * other refusal (the same project in *another* slot, a target slot that no longer exists) stays with
 * the server (contract §0.1 S-3); guessing at those here would only let this side disagree with it.
 */
export const resolveShellSlotDrop = (drag: ProjectDragData | null, drop: ShellSlotDropData | null, tree: ShellSlotTree | null) => {
    if (!drag || !drop) return null
    if (drop.edge === 'center' && projectOfShellSlot(tree, drop.slotId) === drag.projectId) return null
    return { projectId: drag.projectId, targetSlot: drop.slotId, edge: SHELL_SLOT_EDGE_BY_DROP_EDGE[drop.edge] } satisfies ShellSlotDropRequest
}

/**
 * One sortable tier's new order, or `null` when the release was not a reorder of it — the pointer
 * ended on something that is not a sibling of the dragged item (a slot's drop zone, a header while a
 * project moves, empty space), or came back to where it started. `indexOf` is what separates the
 * tiers: a member dropped on a group header, or a header dropped on an icon, finds no index and
 * yields nothing rather than shuffling the wrong list.
 */
const reorderById = (ids: string[], activeId: string, overId: string | null) => {
    const from = ids.indexOf(activeId)
    const to = overId === null ? -1 : ids.indexOf(overId)
    if (from < 0 || to < 0 || from === to) return null
    return arrayMove(ids, from, to)
}

/**
 * The rail's project order after a reorder drop. The guards are the ones `AppSidebar` applied while
 * it owned a `DndContext` of its own; a project drag now has two possible endings, and this answers
 * for the rail half of it.
 *
 * Still the *global* `session.projects` order even once the rail is grouped (contract §1.C): a
 * group's members are a filtered view of that one list, so moving a member is an edit to it and
 * nothing else — membership is a separate axis that a reorder never touches.
 */
export const resolveProjectReorder = (projectIds: ProjectId[], activeId: string, overId: string | null) => reorderById(projectIds, activeId, overId)

/** The group order after a header's drop — the outer tier's counterpart to {@link resolveProjectReorder}, sent as `project_group_reorder`. */
export const resolveProjectGroupReorder = (groupIds: ProjectGroupId[], activeId: string, overId: string | null) =>
    reorderById(groupIds, activeId, overId)
