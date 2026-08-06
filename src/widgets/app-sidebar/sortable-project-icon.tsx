import type { FC } from 'react'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { openPath } from '@tauri-apps/plugin-opener'
import { Folder } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { ProjectRef } from '@shared/api/bindings'
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuSeparator, ContextMenuTrigger } from '@shared/ui/context-menu'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { ProjectIconButton } from '@features/project/project-icon-button'
import { useCloseProject } from '@entities/project/project.query'

const DRAGGING_OPACITY = 0.4

type SortableProjectIconProps = {
    project: ProjectRef
    active: boolean
    dragging: boolean
    agentRunning: boolean
    onActivate: () => void
}

export const SortableProjectIcon: FC<SortableProjectIconProps> = ({ project, active, dragging, agentRunning, onActivate }) => {
    const { t } = useTranslation()
    const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id: project.id.toString() })
    const { mutate: closeProject } = useCloseProject()

    return (
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
                                    icon={<Folder className='size-5' />}
                                    active={active}
                                    agentRunning={agentRunning}
                                    onActivate={onActivate}
                                />
                            </div>
                        </TooltipTrigger>
                        <TooltipContent side='right'>
                            <div className='flex flex-col'>
                                <span>{project.name}</span>
                                <span className='opacity-70'>{project.root}</span>
                            </div>
                        </TooltipContent>
                    </Tooltip>
                </div>
            </ContextMenuTrigger>

            <ContextMenuContent>
                <ContextMenuItem onSelect={() => closeProject(project.id)}>{t('project.close')}</ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem onSelect={() => void openPath(project.root).catch((error: Error) => toast.error(error.message))}>
                    {t('project.openInFileManager')}
                </ContextMenuItem>
                <ContextMenuItem onSelect={() => void navigator.clipboard.writeText(project.root)}>{t('project.copyPath')}</ContextMenuItem>
            </ContextMenuContent>
        </ContextMenu>
    )
}
