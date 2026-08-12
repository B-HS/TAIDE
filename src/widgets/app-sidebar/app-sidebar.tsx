import { useState } from 'react'
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors } from '@dnd-kit/core'
import type { DragEndEvent } from '@dnd-kit/core'
import { SortableContext, arrayMove, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { useQueries, useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { Plus, Settings } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { DetectedAgent, ProjectId } from '@shared/api/bindings'
import { projectListQueryOptions, useActivateProject, useOpenProject, useReorderProjects } from '@entities/project/project.query'
import { projectAgentsQueryOptions, useAgentStateSync } from '@entities/agent/agent.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { IconButton } from '@shared/ui/icon-button'
import { SortableProjectIcon } from '@widgets/app-sidebar/sortable-project-icon'

const DRAG_ACTIVATION_DISTANCE_PX = 4

type AppSidebarProps = {
    activeProjectId: ProjectId | null
    onOpenSettings: () => void
}

export const AppSidebar = ({ activeProjectId, onOpenSettings }: AppSidebarProps) => {
    const { t } = useTranslation()
    const [draggingId, setDraggingId] = useState<string | null>(null)

    const { data: projects = [] } = useQuery(projectListQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: openProject } = useOpenProject()
    const { mutate: activateProject } = useActivateProject()
    const { mutate: reorderProjects } = useReorderProjects()

    const agentQueries = useQueries({
        queries: projects.map((project) => projectAgentsQueryOptions(project.id)),
    })
    const agentsByProjectId = new Map<ProjectId, DetectedAgent[]>(
        agentQueries.flatMap((result) => (result.data ? [[result.data.projectId, result.data.agents] as const] : [])),
    )
    const badgeEnabled = settings?.agentStatusBadgeEnabled ?? true

    const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: DRAG_ACTIVATION_DISTANCE_PX } }))

    const handleOpenProject = async () => {
        const selected = await open({ directory: true, multiple: false })
        if (typeof selected !== 'string') return
        openProject(selected, { onError: (error) => toast.error(error.message) })
    }

    const handleDragEnd = ({ active, over }: DragEndEvent) => {
        setDraggingId(null)
        if (!over || active.id === over.id) return

        const from = projects.findIndex((project) => project.id === active.id)
        const to = projects.findIndex((project) => project.id === over.id)
        if (from < 0 || to < 0) return

        reorderProjects(arrayMove(projects, from, to).map((project) => project.id))
    }

    useAgentStateSync()

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
                            onActivate={() => activateProject(project.id)}
                        />
                    ))}
                </SortableContext>
            </DndContext>

            <IconButton
                label={t('sidebar.openFolderAriaLabel')}
                icon={<Plus className='size-5' />}
                onClick={handleOpenProject}
                side='right'
                className='text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover flex size-10 shrink-0 items-center justify-center rounded-md'
            />

            <IconButton
                label={t('sidebar.settingsAriaLabel')}
                icon={<Settings className='size-5' />}
                onClick={onOpenSettings}
                side='right'
                className='text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover mt-auto flex size-10 shrink-0 items-center justify-center rounded-md'
            />
        </nav>
    )
}
