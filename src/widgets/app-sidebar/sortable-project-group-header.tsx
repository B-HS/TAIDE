import type { FC } from 'react'
import { useState } from 'react'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { ChevronDown, ChevronRight } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { ProjectGroup } from '@shared/api/bindings'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { resolveProjectColorToken } from '@shared/lib/project-display'
import type { ProjectGroupDragData } from '@shared/lib/project-drag'
import { PROJECT_GROUP_DRAG_TYPE } from '@shared/lib/project-drag'
import { resolveProjectGroupColorVar } from '@shared/lib/project-group'
import {
    useDeleteProjectGroup,
    useOpenProjectGroup,
    useRenameProjectGroup,
    useSetProjectGroupCollapsed,
    useSetProjectGroupColor,
} from '@entities/project-group/project-group.query'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@shared/ui/alert-dialog'
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuSeparator, ContextMenuTrigger } from '@shared/ui/context-menu'
import { ProjectGroupDialog } from '@features/project/project-group-dialog'

type SortableProjectGroupHeaderProps = {
    group: ProjectGroup
}

/**
 * One group's row in the 56px rail: a fold toggle that doubles as the group's drag handle, tinted by
 * the group color, with the group's own menu on it.
 *
 * The fold is a command rather than local state because `ProjectGroup.collapsed` lives in
 * `session.json` — a fold has to survive a restart and reach every window, like every other group
 * axis (contract §1.A).
 *
 * Deleting is confirmed but not destructive to the projects: the group disappears and its members
 * stay open, exactly as `project_close` leaves a project's record behind. The confirmation exists
 * because the grouping itself is work the user did and nothing restores it.
 *
 * The header's `useSortable` registers with the window-level `DndContext` in `AppShell`, the same
 * one the rail's icons and the slots' drop zones share (contract §0.1 U-4) — the outer of the two
 * sortable tiers, `use-project-drag.ts` tells the two apart by the drag payload's `type`.
 */
export const SortableProjectGroupHeader: FC<SortableProjectGroupHeaderProps> = ({ group }) => {
    const [isEditOpen, setIsEditOpen] = useState(false)
    const [isDeleteOpen, setIsDeleteOpen] = useState(false)

    const { t } = useTranslation()
    const { attributes, listeners, setNodeRef, transform, transition } = useSortable({
        id: group.id,
        data: { type: PROJECT_GROUP_DRAG_TYPE, groupId: group.id } satisfies ProjectGroupDragData,
    })
    const { mutate: setCollapsed } = useSetProjectGroupCollapsed()
    const { mutateAsync: renameGroup, isPending: isRenamePending } = useRenameProjectGroup()
    const { mutateAsync: setColor, isPending: isColorPending } = useSetProjectGroupColor()
    const { mutate: deleteGroup } = useDeleteProjectGroup()
    const { mutate: openGroup } = useOpenProjectGroup()

    const isCollapsed = group.collapsed ?? false
    const colorVar = resolveProjectGroupColorVar(group.color)
    const ChevronIcon = isCollapsed ? ChevronRight : ChevronDown

    const handleOpenGroup = () =>
        openGroup(group.id, {
            onSuccess: (result) => toast.success(t('projectGroup.openResult', { opened: result.opened.length, skipped: result.skipped.length })),
            onError: (error) => toast.error(describeIpcError(error)),
        })

    const handleSubmitEdit = async ({ name, color }: { name: string; color: string | null }) => {
        try {
            if (name !== group.name) await renameGroup({ groupId: group.id, name })
            if (color !== resolveProjectColorToken(group.color)) await setColor({ groupId: group.id, color })
            setIsEditOpen(false)
        } catch (error) {
            toast.error(describeIpcError(error))
        }
    }

    return (
        <>
            <ContextMenu>
                <ContextMenuTrigger asChild>
                    <button
                        ref={setNodeRef}
                        type='button'
                        aria-expanded={!isCollapsed}
                        style={{ transform: CSS.Translate.toString(transform), transition }}
                        onClick={() =>
                            setCollapsed({ groupId: group.id, collapsed: !isCollapsed }, { onError: (error) => toast.error(describeIpcError(error)) })
                        }
                        className='hover:bg-app-sidebar-item-hover text-app-sidebar-icon-default flex w-11 shrink-0 items-center gap-0.5 rounded-sm px-1 py-0.5'
                        {...attributes}
                        {...listeners}>
                        <ChevronIcon aria-hidden className='size-3 shrink-0' style={colorVar ? { color: colorVar } : undefined} />
                        <span className='min-w-0 flex-1 truncate text-left text-[9px]' style={colorVar ? { color: colorVar } : undefined}>
                            {group.name}
                        </span>
                    </button>
                </ContextMenuTrigger>

                <ContextMenuContent>
                    <ContextMenuItem onSelect={handleOpenGroup}>{t('projectGroup.open')}</ContextMenuItem>
                    <ContextMenuSeparator />
                    <ContextMenuItem onSelect={() => setIsEditOpen(true)}>{t('projectGroup.edit')}</ContextMenuItem>
                    <ContextMenuItem onSelect={() => setIsDeleteOpen(true)}>{t('projectGroup.delete')}</ContextMenuItem>
                </ContextMenuContent>
            </ContextMenu>

            <ProjectGroupDialog
                open={isEditOpen}
                mode='edit'
                initialName={group.name}
                initialColor={group.color ?? null}
                isPending={isRenamePending || isColorPending}
                onOpenChange={setIsEditOpen}
                onSubmit={(value) => void handleSubmitEdit(value)}
            />

            <AlertDialog open={isDeleteOpen} onOpenChange={setIsDeleteOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('projectGroup.deleteConfirmTitle', { name: group.name })}</AlertDialogTitle>
                        <AlertDialogDescription>{t('projectGroup.deleteConfirmDescription')}</AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction
                            variant='destructive'
                            onClick={() => deleteGroup(group.id, { onError: (error) => toast.error(describeIpcError(error)) })}>
                            {t('projectGroup.delete')}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </>
    )
}
