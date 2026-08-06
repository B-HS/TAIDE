import { useState } from 'react'
import { DndContext, PointerSensor, closestCenter, useSensor, useSensors } from '@dnd-kit/core'
import type { DragEndEvent } from '@dnd-kit/core'
import { SortableContext, arrayMove, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { useQueries, useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { Plus, Settings } from 'lucide-react'
import { toast } from 'sonner'
import type { ProjectId } from '@shared/api/bindings'
import { projectListQueryOptions, useActivateProject, useOpenProject, useReorderProjects } from '@entities/project/project.query'
import { projectAgentsQueryOptions } from '@entities/agent/agent.query'
import { SortableProjectIcon } from '@widgets/app-sidebar/sortable-project-icon'

const DRAG_ACTIVATION_DISTANCE_PX = 4

type AppSidebarProps = {
    activeProjectId: ProjectId | null
    onOpenSettings: () => void
}

export const AppSidebar = ({ activeProjectId, onOpenSettings }: AppSidebarProps) => {
    const [draggingId, setDraggingId] = useState<string | null>(null)

    const { data: projects = [] } = useQuery(projectListQueryOptions())
    const { mutate: openProject } = useOpenProject()
    const { mutate: activateProject } = useActivateProject()
    const { mutate: reorderProjects } = useReorderProjects()

    const agentQueries = useQueries({
        queries: projects.map((project) => projectAgentsQueryOptions(project.id)),
    })
    const agentProjectIds = new Set(agentQueries.flatMap((result) => (result.data && result.data.agents.length > 0 ? [result.data.projectId] : [])))

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

    return (
        <nav
            aria-label='프로젝트'
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
                            agentRunning={agentProjectIds.has(project.id)}
                            onActivate={() => activateProject(project.id)}
                        />
                    ))}
                </SortableContext>
            </DndContext>

            <button
                type='button'
                aria-label='폴더 열기'
                onClick={handleOpenProject}
                className='text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover flex size-10 shrink-0 items-center justify-center rounded-md'>
                <Plus className='size-5' />
            </button>

            <button
                type='button'
                aria-label='설정'
                onClick={onOpenSettings}
                className='text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover mt-auto flex size-10 shrink-0 items-center justify-center rounded-md'>
                <Settings className='size-5' />
            </button>
        </nav>
    )
}
