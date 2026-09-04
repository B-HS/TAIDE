import type { FC } from 'react'
import { useState } from 'react'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { AgentActivity, DetectedAgent, ProjectDisplayPatch, ProjectRef } from '@shared/api/bindings'
import { CLEARED_PROJECT_DISPLAY_PATCH } from '@shared/constants/project-display'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { isProjectDisplayCustomized, resolveProjectDisplay } from '@shared/lib/project-display'
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuSeparator, ContextMenuTrigger } from '@shared/ui/context-menu'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { ProjectDisplayDialog } from '@features/project/project-display-dialog'
import { ProjectIconButton } from '@features/project/project-icon-button'
import { useCloseProject, useSetProjectDisplay } from '@entities/project/project.query'
import { systemOpenPath } from '@entities/system/system.ipc'

const DRAGGING_OPACITY = 0.4

const ACTIVITY_PRIORITY: Record<AgentActivity, number> = { awaitingInput: 3, working: 2, idle: 1, unknown: 0 }

const aggregateActivity = (agents: DetectedAgent[]): AgentActivity | null => {
    if (agents.length === 0) return null
    return agents.reduce((top, agent) => (ACTIVITY_PRIORITY[agent.activity] > ACTIVITY_PRIORITY[top] ? agent.activity : top), agents[0].activity)
}

type SortableProjectIconProps = {
    project: ProjectRef
    active: boolean
    dragging: boolean
    agents: DetectedAgent[]
    badgeEnabled: boolean
    onActivate: () => void
}

export const SortableProjectIcon: FC<SortableProjectIconProps> = ({ project, active, dragging, agents, badgeEnabled, onActivate }) => {
    const [displayDialogOpen, setDisplayDialogOpen] = useState(false)

    const { t } = useTranslation()
    const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id: project.id.toString() })
    const { mutate: closeProject } = useCloseProject()
    const { mutate: setProjectDisplay, isPending: isDisplayPending } = useSetProjectDisplay()

    const display = resolveProjectDisplay(project)

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
                                        agentActivity={aggregateActivity(agents)}
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
                                            {t('agent.sessionTooltip', { name: agent.name, status: t(`agent.status.${agent.activity}`) })}
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
                    <ContextMenuItem onSelect={() => void systemOpenPath(project.root).catch((error: Error) => toast.error(describeIpcError(error)))}>
                        {t('project.openInFileManager')}
                    </ContextMenuItem>
                    <ContextMenuItem onSelect={() => void navigator.clipboard.writeText(project.root)}>{t('project.copyPath')}</ContextMenuItem>
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
