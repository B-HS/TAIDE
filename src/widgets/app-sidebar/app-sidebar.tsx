import { useState } from 'react'
import { useDroppable } from '@dnd-kit/core'
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { useQueries, useQuery } from '@tanstack/react-query'
import { Settings } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { DetectedAgent, ProjectGroupId, ProjectId, ProjectRef, ShellSlotEdge } from '@shared/api/bindings'
import { RECENT_PROJECT_MENU_LIMIT } from '@shared/constants/project'
import { cn } from '@shared/lib/cn'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import type { ProjectRailDropData } from '@shared/lib/project-drag'
import { PROJECT_RAIL_DROPPABLE_ID, PROJECT_RAIL_DROP_TYPE } from '@shared/lib/project-drag'
import { resolveProjectGroupSections, withProjectGroupMember, withoutProjectGroupMember } from '@shared/lib/project-group'
import { useShellSlotFocus } from '@shared/lib/shell-slot-context'
import {
    projectListQueryOptions,
    recentProjectsQueryOptions,
    useActivateProject,
    useOpenFolderDialog,
    useOpenProject,
} from '@entities/project/project.query'
import { projectGroupListQueryOptions, useCreateProjectGroup, useSetProjectGroupMembers } from '@entities/project-group/project-group.query'
import { projectAgentsQueryOptions } from '@entities/agent/agent.query'
import { useOpenProjectInSlot } from '@entities/session/session.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { IconButton } from '@shared/ui/icon-button'
import { OpenProjectByPathDialog } from '@features/project/open-project-by-path-dialog'
import { ProjectGroupDialog } from '@features/project/project-group-dialog'
import { SidebarAddProjectMenu } from '@features/project/sidebar-add-project-menu'
import { SortableProjectGroupHeader } from '@widgets/app-sidebar/sortable-project-group-header'
import { SortableProjectIcon } from '@widgets/app-sidebar/sortable-project-icon'

type AppSidebarProps = {
    activeProjectId: ProjectId | null
    /** The project currently being dragged, or `null`. Owned by `AppShell` together with the `DndContext` this rail's `SortableContext` now lives in (contract §0.1 U-4). */
    draggingProjectId: ProjectId | null
    onOpenSettings: () => void
}

/**
 * `activeProjectId` is the *focused shell slot's* project as of d-62 §1.B — a plain click still goes
 * through `project_activate`, which Rust redefined as "focus the slot this project is already in, or
 * else swap the focused slot's project" (contract §0.1 S-5), so the single-slot behaviour is
 * unchanged. The context menu's "Open to the Right/Below/…" is the split route: it targets the
 * focused slot, and the server refuses a project that is already in another slot (§0.1 S-3) rather
 * than this side pre-checking it.
 *
 * Dragging an icon is the other route to the same command, and its `DndContext` sits above this
 * component in `AppShell` — a drag that starts here and ends on a slot's drop zone has to be one
 * drag in one context (contract §0.1 U-4), so the rail keeps only its `SortableContext`s. The `nav`
 * is registered as a droppable purely so that context can measure the rail: its collision detection
 * only forgives a release that missed every icon while the pointer was still over the rail, and the
 * rail's rect is the one thing dnd-kit cannot tell it otherwise.
 *
 * Those are two tiers as of d-62 2c (§1.C): the group headers sort against each other, and each
 * group's members sort inside their own context. Members are the *open* projects the group claims,
 * in `session.projects` order, so a member drag edits that one global order and never the
 * membership — which is why moving a project between groups is a menu item and not a drag
 * (§0.1 U-6). Projects no group claims come last, unlabelled headers being the whole difference
 * between a rail with groups and the flat one that preceded it.
 */
export const AppSidebar = ({ activeProjectId, draggingProjectId, onOpenSettings }: AppSidebarProps) => {
    const { t } = useTranslation()
    const [isOpenByPathDialogOpen, setIsOpenByPathDialogOpen] = useState(false)
    const [newGroupProjectId, setNewGroupProjectId] = useState<ProjectId | null>(null)

    const { data: projects = [] } = useQuery(projectListQueryOptions())
    const { data: groups = [] } = useQuery(projectGroupListQueryOptions())
    const { data: recentProjects = [] } = useQuery(recentProjectsQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: activateProject } = useActivateProject()
    const { mutate: openProject, isPending: isOpeningProject } = useOpenProject()
    const { mutate: openProjectInSlot } = useOpenProjectInSlot()
    const { mutate: setProjectGroupMembers } = useSetProjectGroupMembers()
    const { mutate: createProjectGroup, isPending: isCreatingProjectGroup } = useCreateProjectGroup()
    const { focusedShellSlotId } = useShellSlotFocus()
    const { setNodeRef: setProjectRailRef } = useDroppable({
        id: PROJECT_RAIL_DROPPABLE_ID,
        data: { type: PROJECT_RAIL_DROP_TYPE } satisfies ProjectRailDropData,
    })
    const handleOpenViaFinder = useOpenFolderDialog()

    const agentQueries = useQueries({
        queries: projects.map((project) => projectAgentsQueryOptions(project.id)),
    })
    const agentsByProjectId = new Map<ProjectId, DetectedAgent[]>(
        agentQueries.flatMap((result) => (result.data ? [[result.data.projectId, result.data.agents] as const] : [])),
    )
    const badgeEnabled = settings?.agentStatusBadgeEnabled ?? true

    const openProjectIds = new Set(projects.map((project) => project.id))
    const menuRecentProjects = recentProjects.filter((project) => !openProjectIds.has(project.id)).slice(0, RECENT_PROJECT_MENU_LIMIT)
    const { sections, ungrouped, groupIdByProjectId } = resolveProjectGroupSections(projects, groups)

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

    const writeGroupMembers = (groupId: ProjectGroupId, members: ProjectId[]) =>
        setProjectGroupMembers({ groupId, members }, { onError: (error) => toast.error(describeIpcError(error)) })

    const handleAddToGroup = (projectId: ProjectId, groupId: ProjectGroupId) => {
        const target = groups.find((group) => group.id === groupId)
        if (target) writeGroupMembers(groupId, withProjectGroupMember(target.members, projectId))
    }

    const handleRemoveFromGroup = (projectId: ProjectId) => {
        const current = groups.find((group) => group.id === groupIdByProjectId.get(projectId))
        if (current) writeGroupMembers(current.id, withoutProjectGroupMember(current.members, projectId))
    }

    const handleCreateGroup = ({ name, color }: { name: string; color: string | null }) =>
        createProjectGroup(
            { name, color, members: newGroupProjectId ? [newGroupProjectId] : null },
            { onSuccess: () => setNewGroupProjectId(null), onError: (error) => toast.error(describeIpcError(error)) },
        )

    const renderProjectIcon = (project: ProjectRef) => (
        <SortableProjectIcon
            key={project.id}
            project={project}
            active={project.id === activeProjectId}
            dragging={draggingProjectId === project.id}
            agents={agentsByProjectId.get(project.id) ?? []}
            badgeEnabled={badgeEnabled}
            canOpenInShellSlot={!!focusedShellSlotId}
            groups={groups}
            groupId={groupIdByProjectId.get(project.id) ?? null}
            onActivate={() => activateProject(project.id)}
            onOpenInShellSlot={(edge) => handleOpenInShellSlot(project.id, edge)}
            onAddToGroup={(groupId) => handleAddToGroup(project.id, groupId)}
            onCreateGroup={() => setNewGroupProjectId(project.id)}
            onRemoveFromGroup={() => handleRemoveFromGroup(project.id)}
        />
    )

    return (
        <nav
            ref={setProjectRailRef}
            aria-label={t('sidebar.projectsAriaLabel')}
            className='bg-app-sidebar-background border-app-border flex h-full w-14 shrink-0 flex-col items-center gap-1 border-r py-2'>
            <SortableContext items={sections.map((section) => section.group.id)} strategy={verticalListSortingStrategy}>
                {sections.map(({ group, members }) => (
                    <div key={group.id} role='group' aria-label={group.name} className='flex w-full shrink-0 flex-col items-center gap-1'>
                        <SortableProjectGroupHeader group={group} />
                        {!group.collapsed && (
                            <SortableContext items={members.map((member) => member.id)} strategy={verticalListSortingStrategy}>
                                {members.map(renderProjectIcon)}
                            </SortableContext>
                        )}
                    </div>
                ))}
            </SortableContext>

            <div
                role='group'
                aria-label={t('projectGroup.ungrouped')}
                className={cn(
                    'flex w-full shrink-0 flex-col items-center gap-1',
                    sections.length > 0 && ungrouped.length > 0 && 'border-app-border border-t pt-1',
                )}>
                <SortableContext items={ungrouped.map((project) => project.id)} strategy={verticalListSortingStrategy}>
                    {ungrouped.map(renderProjectIcon)}
                </SortableContext>
            </div>

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
            <ProjectGroupDialog
                open={newGroupProjectId !== null}
                mode='create'
                initialName=''
                initialColor={null}
                isPending={isCreatingProjectGroup}
                onOpenChange={(open) => !open && setNewGroupProjectId(null)}
                onSubmit={handleCreateGroup}
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
