import type { FC } from 'react'
import { Group, Panel } from 'react-resizable-panels'
import type { Layout, LayoutChangedMeta } from 'react-resizable-panels'
import type { ProjectRef, ShellSlotId, ShellSlotTree } from '@shared/api/bindings'
import { schedulePaneResizeCommit } from '@entities/layout/pane-resize-commit'
import { PaneSeparator } from '@features/split/pane-separator'
import { ShellSlotHeader } from '@features/shell-slot/shell-slot-header'
import { MIN_PANEL_SIZE_PX, RESIZE_HIT_TARGET_SIZE } from '@shared/constants/layout'
import { SHELL_SLOT_ID_ATTRIBUTE, resolveShellSlotFocus, shellSlotLeaves } from '@shared/lib/shell-slot'
import type { ShellSlotLeaf } from '@shared/lib/shell-slot'
import { ShellSlotScope } from '@shared/lib/shell-slot-context'
import { ProjectShell } from '@widgets/app-shell/project-shell'

const EQUAL_SPLIT_TOTAL_PERCENT = 100

/** A split node carries no id of its own (`domain::project::shell_slots`), so a node's child-index path from the root is both its panel id and the address `session_set_shell_slot_sizes` takes. */
const nodeKeyOf = (path: number[]) => ['shell-slot', ...path].join('-')

const containsShellSlot = (node: ShellSlotTree, slotId: ShellSlotId | null) => shellSlotLeaves(node).some((leaf) => leaf.slotId === slotId)

type SlotRenderContext = {
    projects: ProjectRef[]
    zen: boolean
    slotCount: number
    focusedShellSlotId: ShellSlotId | null
    resizerThickness: number
    problemsOpenSlotIds: readonly ShellSlotId[]
    onCloseProblems: (slotId: ShellSlotId) => void
    onCloseSlot: (slotId: ShellSlotId) => void
    onCommitSizes: (path: number[], sizes: number[]) => void
}

type ShellSlotLeafViewProps = SlotRenderContext & { leaf: ShellSlotLeaf }

/**
 * A leaf is kept a component of its own — rather than folded into {@link ShellSlotNodeView}'s
 * recursion the way `pane-node-view.tsx` does it one level down — because a leaf is the unit Zen
 * hides, and hiding must never change which component sits at a given position in the tree.
 */
const ShellSlotLeafView: FC<ShellSlotLeafViewProps> = ({ leaf, projects, zen, slotCount, problemsOpenSlotIds, onCloseProblems, onCloseSlot }) => {
    const project = projects.find((candidate) => candidate.id === leaf.projectId) ?? null

    return (
        <div className='flex h-full min-h-0 w-full min-w-0 flex-col' {...{ [SHELL_SLOT_ID_ATTRIBUTE]: leaf.slotId }}>
            {slotCount > 1 && !zen && (
                <ShellSlotHeader label={project?.name ?? leaf.projectId} canClose={slotCount > 1} onClose={() => onCloseSlot(leaf.slotId)} />
            )}
            <ShellSlotScope slotId={leaf.slotId} projectId={leaf.projectId}>
                <div className='flex min-h-0 min-w-0 flex-1'>
                    <ProjectShell
                        projectId={leaf.projectId}
                        zen={zen}
                        isProblemsOpen={problemsOpenSlotIds.includes(leaf.slotId)}
                        onCloseProblems={() => onCloseProblems(leaf.slotId)}
                    />
                </div>
            </ShellSlotScope>
        </div>
    )
}

type ShellSlotNodeViewProps = SlotRenderContext & { node: ShellSlotTree; path: number[] }

const ShellSlotNodeView: FC<ShellSlotNodeViewProps> = ({ node, path, ...context }) => {
    if (node.node === 'leaf') return <ShellSlotLeafView leaf={{ slotId: node.slotId, projectId: node.projectId }} {...context} />

    const children = node.children
    const handleLayoutChanged = (layout: Layout, meta: LayoutChangedMeta) => {
        if (!meta.isUserInteraction) return
        const sizes = children.map((_, index) => layout[nodeKeyOf([...path, index])] ?? EQUAL_SPLIT_TOTAL_PERCENT / children.length)
        schedulePaneResizeCommit(nodeKeyOf(path), () => context.onCommitSizes(path, sizes))
    }

    /**
     * `hidden` rather than a conditional render, on the `Panel` rather than on its contents: the
     * panel is the flex item that owns the slot's share of the group, so taking it out of flow is
     * what lets the one visible slot grow to the whole window while the arrangement's numbers stay
     * exactly as they were.
     */
    const isSubtreeHidden = (child: ShellSlotTree) => context.zen && !containsShellSlot(child, context.focusedShellSlotId)

    const items = children.flatMap((child, index) => [
        index > 0 && (
            <PaneSeparator
                key={`separator-${nodeKeyOf([...path, index])}`}
                orientation={node.dir === 'horizontal' ? 'horizontal' : 'vertical'}
                thickness={context.resizerThickness}
                hidden={isSubtreeHidden(children[index - 1]) || isSubtreeHidden(child)}
            />
        ),
        <Panel
            key={nodeKeyOf([...path, index])}
            id={nodeKeyOf([...path, index])}
            hidden={isSubtreeHidden(child)}
            defaultSize={`${node.sizes[index] ?? EQUAL_SPLIT_TOTAL_PERCENT / children.length}%`}
            minSize={MIN_PANEL_SIZE_PX}
            className='min-h-0 min-w-0'>
            <ShellSlotNodeView node={child} path={[...path, index]} {...context} />
        </Panel>,
    ])

    return (
        <Group
            orientation={node.dir}
            onLayoutChanged={handleLayoutChanged}
            resizeTargetMinimumSize={RESIZE_HIT_TARGET_SIZE}
            className='min-h-0 min-w-0 flex-1'>
            {items}
        </Group>
    )
}

type ShellSlotTreeViewProps = Omit<SlotRenderContext, 'slotCount'> & { tree: ShellSlotTree }

/**
 * Renders the window's shell-slot tree as nested `react-resizable-panels` groups, the same shape
 * `pane-node-view.tsx` already uses one level down for a project's own pane tree — each leaf is one
 * whole project shell rather than a tab strip.
 *
 * Zen mode keeps only the focused slot on screen (contract §0.1 S-6), and it does so by *hiding*
 * the other slots — the component tree is identical either way. Rendering a different tree for Zen
 * would remount the surviving `ProjectShell` on every toggle, throwing away its editors' models,
 * scroll positions and terminal buffers; `hidden` keeps every slot mounted and leaves the slot tree
 * itself untouched, so leaving Zen restores the arrangement exactly. The slot header and every
 * separator go with the other slots, which is what makes a Zen'd split window indistinguishable
 * from a Zen'd single-slot one.
 */
export const ShellSlotTreeView: FC<ShellSlotTreeViewProps> = ({ tree, focusedShellSlotId, ...context }) => {
    const leaves = shellSlotLeaves(tree)
    if (leaves.length === 0) return null

    return (
        <ShellSlotNodeView
            {...context}
            node={tree}
            path={[]}
            slotCount={leaves.length}
            focusedShellSlotId={resolveShellSlotFocus(tree, focusedShellSlotId)}
        />
    )
}
