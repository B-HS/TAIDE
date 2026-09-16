import type { FC } from 'react'
import { useState } from 'react'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { AgentActivity, DetectedAgent, ProjectDisplayPatch, ProjectGroup, ProjectGroupId, ProjectRef, ShellSlotEdge } from '@shared/api/bindings'
import { CLEARED_PROJECT_DISPLAY_PATCH } from '@shared/constants/project-display'
import { agentStatusLabelKey } from '@shared/lib/agent-status-text'
import { copyTextToClipboard } from '@shared/lib/copy-text-to-clipboard'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { isProjectDisplayCustomized, resolveProjectDisplay } from '@shared/lib/project-display'
import type { ProjectDragData } from '@shared/lib/project-drag'
import { PROJECT_DRAG_TYPE } from '@shared/lib/project-drag'
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuSeparator,
    ContextMenuSub,
    ContextMenuSubContent,
    ContextMenuSubTrigger,
    ContextMenuTrigger,
} from '@shared/ui/context-menu'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { ProjectDisplayDialog } from '@features/project/project-display-dialog'
import { ProjectIconButton } from '@features/project/project-icon-button'
import { useCloseProject, useSetProjectDisplay } from '@entities/project/project.query'
import { systemOpenPath } from '@entities/system/system.ipc'

const DRAGGING_OPACITY = 0.4

/**
 * "Open to the Right/Below/…" — the keyboard-reachable route to a shell split (contract §0.1, 2a),
 * alongside dragging the icon onto a slot. It always targets the *focused* slot, which is what a
 * drag says with its pointer instead. Ordered the way the contract names them, and labelled by
 * direction rather than by edge id so the wording survives a future right-to-left layout.
 */
const SHELL_SLOT_OPEN_EDGES: { edge: ShellSlotEdge; labelKey: string }[] = [
    { edge: 'right', labelKey: 'shellSlot.openToTheRight' },
    { edge: 'bottom', labelKey: 'shellSlot.openBelow' },
    { edge: 'left', labelKey: 'shellSlot.openToTheLeft' },
    { edge: 'top', labelKey: 'shellSlot.openAbove' },
]

const ACTIVITY_PRIORITY: Record<AgentActivity, number> = { awaitingInput: 3, working: 2, idle: 1, unknown: 0 }

/**
 * The one agent the project's single badge speaks for, so the badge shape and the reason beside it
 * describe the same session rather than a shape from one agent and a reason from another. Picked by
 * {@link ACTIVITY_PRIORITY} — a project with one blocked and one working agent needs the user to
 * come back, which is the state the badge has to show.
 */
const topPriorityAgent = (agents: DetectedAgent[]): DetectedAgent | null => {
    if (agents.length === 0) return null
    return agents.reduce((top, agent) => (ACTIVITY_PRIORITY[agent.activity] > ACTIVITY_PRIORITY[top.activity] ? agent : top), agents[0])
}

type SortableProjectIconProps = {
    project: ProjectRef
    active: boolean
    dragging: boolean
    agents: DetectedAgent[]
    badgeEnabled: boolean
    /** `false` while the window has no shell slot to split — projects are closed, or the tree has not loaded yet. */
    canOpenInShellSlot: boolean
    /** Every group the rail knows, for the "add to group" submenu. Membership is owned one level up (`AppSidebar`), which is the only place that can take a project out of the group it was in. */
    groups: ProjectGroup[]
    /** The group this project belongs to, or `null` when no group claims it — a project belongs to at most one (`types::ProjectGroup`). */
    groupId: ProjectGroupId | null
    onActivate: () => void
    onOpenInShellSlot: (edge: ShellSlotEdge) => void
    onAddToGroup: (groupId: ProjectGroupId) => void
    onCreateGroup: () => void
    onRemoveFromGroup: () => void
}

export const SortableProjectIcon: FC<SortableProjectIconProps> = ({
    project,
    active,
    dragging,
    agents,
    badgeEnabled,
    canOpenInShellSlot,
    groups,
    groupId,
    onActivate,
    onOpenInShellSlot,
    onAddToGroup,
    onCreateGroup,
    onRemoveFromGroup,
}) => {
    const [displayDialogOpen, setDisplayDialogOpen] = useState(false)

    const { t } = useTranslation()
    const { attributes, listeners, setNodeRef, transform, transition } = useSortable({
        id: project.id.toString(),
        data: { type: PROJECT_DRAG_TYPE, projectId: project.id } satisfies ProjectDragData,
    })
    const { mutate: closeProject } = useCloseProject()
    const { mutate: setProjectDisplay, isPending: isDisplayPending } = useSetProjectDisplay()

    const display = resolveProjectDisplay(project)
    const topAgent = topPriorityAgent(agents)

    const applyDisplay = (patch: ProjectDisplayPatch, onApplied?: () => void) =>
        setProjectDisplay(
            { projectId: project.id, patch },
            { onSuccess: () => onApplied?.(), onError: (error) => toast.error(describeIpcError(error)) },
        )

    return (
        <>
            <ContextMenu>
                <ContextMenuTrigger asChild>
                    <div
                        ref={setNodeRef}
                        style={{ transform: CSS.Translate.toString(transform), transition, opacity: dragging ? DRAGGING_OPACITY : 1 }}
                        {...attributes}
                        {...listeners}>
                        <Tooltip>
                            <TooltipTrigger asChild>
                                <div>
                                    <ProjectIconButton
                                        name={project.name}
                                        display={display}
                                        active={active}
                                        agentActivity={topAgent?.activity ?? null}
                                        agentBlockedReason={topAgent?.blockedReason ?? null}
                                        badgeEnabled={badgeEnabled}
                                        onActivate={onActivate}
                                    />
                                </div>
                            </TooltipTrigger>
                            <TooltipContent side='right'>
                                <div className='flex flex-col'>
                                    <span>{project.name}</span>
                                    <span className='opacity-70'>{project.root}</span>
                                    {agents.map((agent) => (
                                        <span key={agent.sessionId} className='opacity-70'>
                                            {t('agent.sessionTooltip', {
                                                name: agent.name,
                                                status: t(agentStatusLabelKey(agent.activity, agent.blockedReason)),
                                            })}
                                        </span>
                                    ))}
                                </div>
                            </TooltipContent>
                        </Tooltip>
                    </div>
                </ContextMenuTrigger>

                <ContextMenuContent>
                    <ContextMenuItem onSelect={() => closeProject(project.id)}>{t('project.close')}</ContextMenuItem>
                    <ContextMenuSeparator />
                    {SHELL_SLOT_OPEN_EDGES.map(({ edge, labelKey }) => (
                        <ContextMenuItem key={edge} disabled={!canOpenInShellSlot} onSelect={() => onOpenInShellSlot(edge)}>
                            {t(labelKey)}
                        </ContextMenuItem>
                    ))}
                    <ContextMenuSeparator />
                    <ContextMenuSub>
                        <ContextMenuSubTrigger>{t('projectGroup.addTo')}</ContextMenuSubTrigger>
                        <ContextMenuSubContent>
                            {groups.map((group) => (
                                <ContextMenuItem key={group.id} disabled={group.id === groupId} onSelect={() => onAddToGroup(group.id)}>
                                    {group.name}
                                </ContextMenuItem>
                            ))}
                            {groups.length > 0 && <ContextMenuSeparator />}
                            <ContextMenuItem onSelect={onCreateGroup}>{t('projectGroup.newGroup')}</ContextMenuItem>
                        </ContextMenuSubContent>
                    </ContextMenuSub>
                    <ContextMenuItem disabled={!groupId} onSelect={onRemoveFromGroup}>
                        {t('projectGroup.removeFrom')}
                    </ContextMenuItem>
                    <ContextMenuSeparator />
                    <ContextMenuItem onSelect={() => void systemOpenPath(project.root).catch((error: Error) => toast.error(describeIpcError(error)))}>
                        {t('project.openInFileManager')}
                    </ContextMenuItem>
                    <ContextMenuItem onSelect={() => void copyTextToClipboard(project.root)}>{t('project.copyPath')}</ContextMenuItem>
                    <ContextMenuSeparator />
                    <ContextMenuItem onSelect={() => setDisplayDialogOpen(true)}>{t('project.displayMenu')}</ContextMenuItem>
                    <ContextMenuItem disabled={!isProjectDisplayCustomized(display)} onSelect={() => applyDisplay(CLEARED_PROJECT_DISPLAY_PATCH)}>
                        {t('project.displayReset')}
                    </ContextMenuItem>
                </ContextMenuContent>
            </ContextMenu>

            <ProjectDisplayDialog
                open={displayDialogOpen}
                projectName={project.name}
                display={project.display}
                isPending={isDisplayPending}
                onOpenChange={setDisplayDialogOpen}
                onSubmit={(patch) => applyDisplay(patch, () => setDisplayDialogOpen(false))}
            />
        </>
    )
}
