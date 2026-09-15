import type { FC } from 'react'
import { FolderInput, FolderOpen, Plus, TriangleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { Project } from '@shared/api/bindings'
import { resolveProjectDisplay } from '@shared/lib/project-display'
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@shared/ui/dropdown-menu'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

const ADD_MENU_TRIGGER_CLASS =
    'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover flex size-10 shrink-0 items-center justify-center rounded-md'

type SidebarAddProjectMenuProps = {
    /** Recent projects to offer, already capped and already filtered by the widget — this component
     *  renders the list verbatim so the cap (`RECENT_PROJECT_MENU_LIMIT`) and the "skip what is
     *  already open" rule stay with the data owner rather than being re-decided here. */
    recentProjects: Project[]
    onOpenByPath: () => void
    onOpenViaFinder: () => void
    onSelectRecent: (project: Project) => void
}

/**
 * The app sidebar's `+` control. Follows `tab-bar-add-menu.tsx`'s
 * `DropdownMenu > Tooltip > TooltipTrigger asChild > DropdownMenuTrigger` nesting rather than
 * reusing `IconButton`: that component wraps its button in its own tooltip trigger span, which a
 * `DropdownMenuTrigger asChild` would have to wrap again — two triggers competing for the same
 * element. Pure UI (props and callbacks only); the opening itself belongs to `AppSidebar`.
 */
export const SidebarAddProjectMenu: FC<SidebarAddProjectMenuProps> = ({ recentProjects, onOpenByPath, onOpenViaFinder, onSelectRecent }) => {
    const { t } = useTranslation()

    return (
        <DropdownMenu>
            <Tooltip>
                <TooltipTrigger asChild>
                    <DropdownMenuTrigger aria-label={t('sidebar.addProjectMenu')} className={ADD_MENU_TRIGGER_CLASS}>
                        <Plus className='size-5' />
                    </DropdownMenuTrigger>
                </TooltipTrigger>
                <TooltipContent side='right'>{t('sidebar.addProjectMenu')}</TooltipContent>
            </Tooltip>
            <DropdownMenuContent align='start' side='right'>
                <DropdownMenuItem onSelect={onOpenByPath}>
                    <FolderInput className='size-4' />
                    {t('sidebar.openByPath')}
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={onOpenViaFinder}>
                    <FolderOpen className='size-4' />
                    {t('sidebar.openViaFinder')}
                </DropdownMenuItem>
                {recentProjects.length > 0 && <DropdownMenuSeparator />}
                {recentProjects.length > 0 && <DropdownMenuLabel className='text-xs font-normal'>{t('app.recentItems')}</DropdownMenuLabel>}
                {recentProjects.map((project) => (
                    <DropdownMenuItem
                        key={project.id}
                        disabled={project.rootMissing}
                        onSelect={() => onSelectRecent(project)}
                        className='max-w-72 flex-col items-start gap-0'>
                        <span className='w-full truncate'>{resolveProjectDisplay({ display: project.display }).label ?? project.name}</span>
                        <span className='text-app-sidebar-icon-default flex w-full min-w-0 items-center gap-1 text-xs'>
                            {project.rootMissing && (
                                <span className='flex shrink-0 items-center gap-1'>
                                    <TriangleAlert className='size-3 shrink-0' />
                                    {t('app.recentProjectRootMissing')}
                                </span>
                            )}
                            <span className='truncate' title={project.root}>
                                {project.root}
                            </span>
                        </span>
                    </DropdownMenuItem>
                ))}
            </DropdownMenuContent>
        </DropdownMenu>
    )
}
