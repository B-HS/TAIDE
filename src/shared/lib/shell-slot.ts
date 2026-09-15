import type { ProjectId, ShellSlotId, ShellSlotTree } from '@shared/api/bindings'

/**
 * Marks the DOM subtree one shell slot owns. The app-root focus tracker resolves a pointerdown /
 * focusin to a slot by walking up from the event target to the nearest element carrying it, which is
 * why every slot leaf renders it on its outermost element.
 */
export const SHELL_SLOT_ID_ATTRIBUTE = 'data-shell-slot-id'

export type ShellSlotLeaf = { slotId: ShellSlotId; projectId: ProjectId }

/** Left-to-right, top-to-bottom — the order the slots are painted in, so `[0]` is a stable "first slot" for every fallback below. */
export const shellSlotLeaves = (tree: ShellSlotTree | null): ShellSlotLeaf[] => {
    if (!tree) return []
    if (tree.node === 'leaf') return [{ slotId: tree.slotId, projectId: tree.projectId }]
    return tree.children.flatMap(shellSlotLeaves)
}

export const projectOfShellSlot = (tree: ShellSlotTree | null, slotId: ShellSlotId | null) =>
    shellSlotLeaves(tree).find((leaf) => leaf.slotId === slotId)?.projectId ?? null

export const shellSlotOfProject = (tree: ShellSlotTree | null, projectId: ProjectId | null) =>
    shellSlotLeaves(tree).find((leaf) => leaf.projectId === projectId)?.slotId ?? null

/**
 * The slot the window should treat as focused: the requested one when it still exists in the tree,
 * otherwise the first slot. A slot id is an address inside the current arrangement, not a durable
 * handle (`domain::project::shell_slots`), so a stale id — a closed slot, a tree restored under
 * fresh ids — must degrade to a real slot rather than leave the window with no focus at all.
 */
export const resolveShellSlotFocus = (tree: ShellSlotTree | null, requested: ShellSlotId | null) => {
    const leaves = shellSlotLeaves(tree)
    if (leaves.length === 0) return null
    return leaves.some((leaf) => leaf.slotId === requested) ? requested : leaves[0].slotId
}

/** `null` for anything outside a slot — a Radix portal's content, the sidebar, the status bar — which the focus tracker reads as "keep the last focused slot". */
export const resolveShellSlotIdFromEventTarget = (target: EventTarget | null) => {
    if (!(target instanceof Element)) return null
    return target.closest(`[${SHELL_SLOT_ID_ATTRIBUTE}]`)?.getAttribute(SHELL_SLOT_ID_ATTRIBUTE) ?? null
}

/**
 * Adds or removes one slot id from a set-like list. The Problems panel's open/closed flag is tracked
 * this way — per slot, above every slot — because the status bar's toggle is window-level and has to
 * flip whichever slot currently has focus (contract §0.1 U-3).
 */
export const withShellSlotToggled = (slotIds: readonly ShellSlotId[], slotId: ShellSlotId) =>
    slotIds.includes(slotId) ? slotIds.filter((current) => current !== slotId) : [...slotIds, slotId]
