import { useState } from 'react'
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors } from '@dnd-kit/core'
import type { DragEndEvent } from '@dnd-kit/core'
import { SortableContext, arrayMove, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { useQueries, useQuery } from '@tanstack/react-query'
import { Settings } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { DetectedAgent, ProjectId, ShellSlotEdge } from '@shared/api/bindings'
import { RECENT_PROJECT_MENU_LIMIT } from '@shared/constants/project'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { useShellSlotFocus } from '@shared/lib/shell-slot-context'
import {
    projectListQueryOptions,
    recentProjectsQueryOptions,
    useActivateProject,
    useOpenFolderDialog,
    useOpenProject,
    useReorderProjects,
} from '@entities/project/project.query'
import { projectAgentsQueryOptions } from '@entities/agent/agent.query'
import { useOpenProjectInSlot } from '@entities/session/session.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { IconButton } from '@shared/ui/icon-button'
import { OpenProjectByPathDialog } from '@features/project/open-project-by-path-dialog'
import { SidebarAddProjectMenu } from '@features/project/sidebar-add-project-menu'
import { SortableProjectIcon } from '@widgets/app-sidebar/sortable-project-icon'

const DRAG_ACTIVATION_DISTANCE_PX = 4

type AppSidebarProps = {
    activeProjectId: ProjectId | null
    onOpenSettings: () => void
}

/**
 * `activeProjectId` is the *focused shell slot's* project as of d-62 §1.B — a plain click still goes
 * through `project_activate`, which Rust redefined as "focus the slot this project is already in, or
 * else swap the focused slot's project" (contract §0.1 S-5), so the single-slot behaviour is
 * unchanged. The context menu's "Open to the Right/Below/…" is the split route: it targets the
 * focused slot, and the server refuses a project that is already in another slot (§0.1 S-3) rather
 * than this side pre-checking it.
 */
export const AppSidebar = ({ activeProjectId, onOpenSettings }: AppSidebarProps) => {
    const { t } = useTranslation()
    const [draggingId, setDraggingId] = useState<string | null>(null)
    const [isOpenByPathDialogOpen, setIsOpenByPathDialogOpen] = useState(false)

    const { data: projects = [] } = useQuery(projectListQueryOptions())
    const { data: recentProjects = [] } = useQuery(recentProjectsQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: activateProject } = useActivateProject()
    const { mutate: reorderProjects } = useReorderProjects()
    const { mutate: openProject, isPending: isOpeningProject } = useOpenProject()
    const { mutate: openProjectInSlot } = useOpenProjectInSlot()
    const { focusedShellSlotId } = useShellSlotFocus()
    const handleOpenViaFinder = useOpenFolderDialog()

    const agentQueries = useQueries({
        queries: projects.map((project) => projectAgentsQueryOptions(project.id)),
    })
    const agentsByProjectId = new Map<ProjectId, DetectedAgent[]>(
        agentQueries.flatMap((result) => (result.data ? [[result.data.projectId, result.data.agents] as const] : [])),
    )
    const badgeEnabled = settings?.agentStatusBadgeEnabled ?? true

    const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: DRAG_ACTIVATION_DISTANCE_PX } }))

    const openProjectIds = new Set(projects.map((project) => project.id))
    const menuRecentProjects = recentProjects.filter((project) => !openProjectIds.has(project.id)).slice(0, RECENT_PROJECT_MENU_LIMIT)

    const handleOpenByPath = (path: string) => {
        openProject(path, {
            onSuccess: () => setIsOpenByPathDialogOpen(false),
            onError: (error) => toast.error(describeIpcError(error)),
        })
    }

    const handleOpenInShellSlot = (projectId: ProjectId, edge: ShellSlotEdge) => {
        if (!focusedShellSlotId) return
        openProjectInSlot(
            { path: null, projectId, targetSlot: focusedShellSlotId, edge },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
    }

    const handleDragEnd = ({ active, over }: DragEndEvent) => {
        setDraggingId(null)
        if (!over || active.id === over.id) return

        const from = projects.findIndex((project) => project.id === active.id)
        const to = projects.findIndex((project) => project.id === over.id)
        if (from < 0 || to < 0) return

        reorderProjects(arrayMove(projects, from, to).map((project) => project.id))
    }

    return (
        <nav
            aria-label={t('sidebar.projectsAriaLabel')}
            className='bg-app-sidebar-background border-app-border flex h-full w-14 shrink-0 flex-col items-center gap-1 border-r py-2'>
            <DndContext
                sensors={sensors}
                collisionDetection={closestCenter}
                onDragStart={({ active }) => setDraggingId(String(active.id))}
                onDragCancel={() => setDraggingId(null)}
                onDragEnd={handleDragEnd}>
                <SortableContext items={projects.map((project) => project.id.toString())} strategy={verticalListSortingStrategy}>
                    {projects.map((project) => (
                        <SortableProjectIcon
                            key={project.id}
                            project={project}
                            active={project.id === activeProjectId}
                            dragging={draggingId === project.id}
                            agents={agentsByProjectId.get(project.id) ?? []}
                            badgeEnabled={badgeEnabled}
                            canOpenInShellSlot={!!focusedShellSlotId}
                            onActivate={() => activateProject(project.id)}
                            onOpenInShellSlot={(edge) => handleOpenInShellSlot(project.id, edge)}
                        />
                    ))}
                </SortableContext>
            </DndContext>

            <SidebarAddProjectMenu
                recentProjects={menuRecentProjects}
                onOpenByPath={() => setIsOpenByPathDialogOpen(true)}
                onOpenViaFinder={handleOpenViaFinder}
                onSelectRecent={(project) => openProject(project.root, { onError: (error) => toast.error(describeIpcError(error)) })}
            />
            <OpenProjectByPathDialog
                open={isOpenByPathDialogOpen}
                isPending={isOpeningProject}
                onOpenChange={setIsOpenByPathDialogOpen}
                onConfirm={handleOpenByPath}
            />

            <IconButton
                label={t('sidebar.settingsAriaLabel')}
                icon={<Settings className='size-5' />}
                onClick={onOpenSettings}
                side='right'
                containerClassName='mt-auto'
                className='text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover flex size-10 shrink-0 items-center justify-center rounded-md'
            />
        </nav>
    )
}
