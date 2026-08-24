import { useState } from 'react'
import { toast } from 'sonner'
import type { useTranslation } from 'react-i18next'
import type { ProjectId } from '@shared/api/bindings'
import type { useCreateEntry, useDeleteEntry, useRenameEntry } from '@entities/file/file.query'
import type { useRefreshTreeDir, useRevealTreeNode, useToggleTreeNode } from '@entities/tree/tree.query'
import type { FileTreeNodeKind, FileTreeRow } from '@features/explorer/file-tree-row'
import type { FileTreeDraft, FileTreeRenameTarget } from '@features/explorer/file-tree'
import { resolveEntryParentDir, validateEntryName } from '@shared/lib/entry-name'
import { fileNameOf } from '@shared/lib/relative-path'
import { joinPath, parentDirOf } from '@widgets/explorer/explorer-path'

type UseExplorerEntryCrudInput = {
    projectId: ProjectId
    rows: FileTreeRow[]
    selectedRow: FileTreeRow | null
    targetDirFor: (row: FileTreeRow | null) => string | null
    openFileTab: (row: FileTreeRow, preview: boolean) => void
    notifyError: (error: unknown) => void
    setSelectPathRequest: (path: string) => void
    toggleNodeAsync: ReturnType<typeof useToggleTreeNode>['mutateAsync']
    createEntry: ReturnType<typeof useCreateEntry>['mutateAsync']
    refreshTreeDir: ReturnType<typeof useRefreshTreeDir>['mutateAsync']
    revealTreeNode: ReturnType<typeof useRevealTreeNode>['mutateAsync']
    renameEntryAsync: ReturnType<typeof useRenameEntry>['mutateAsync']
    deleteEntryAsync: ReturnType<typeof useDeleteEntry>['mutateAsync']
    t: ReturnType<typeof useTranslation>['t']
}

export const useExplorerEntryCrud = ({
    projectId,
    rows,
    selectedRow,
    targetDirFor,
    openFileTab,
    notifyError,
    setSelectPathRequest,
    toggleNodeAsync,
    createEntry,
    refreshTreeDir,
    revealTreeNode,
    renameEntryAsync,
    deleteEntryAsync,
    t,
}: UseExplorerEntryCrudInput) => {
    const [draft, setDraft] = useState<FileTreeDraft | null>(null)
    const [draftError, setDraftError] = useState<string | null>(null)
    const [renameTarget, setRenameTarget] = useState<FileTreeRenameTarget | null>(null)
    const [renameError, setRenameError] = useState<string | null>(null)
    const [deleteTarget, setDeleteTarget] = useState<FileTreeRow | null>(null)

    const startDraft = async (kind: FileTreeNodeKind) => {
        const targetDir = targetDirFor(selectedRow)
        if (!targetDir) return
        const targetRow = rows.find((row) => row.path === targetDir)
        if (targetRow && !targetRow.expanded) await toggleNodeAsync({ projectId, path: targetDir })
        setDraft({ kind, parentDir: targetDir })
        setDraftError(null)
    }

    const cancelDraft = () => {
        setDraft(null)
        setDraftError(null)
    }

    const commitDraft = async (name: string) => {
        if (!draft) return
        const trimmedName = name.trim()
        if (!trimmedName) {
            cancelDraft()
            return
        }

        const targetDir = resolveEntryParentDir(draft.parentDir, trimmedName)
        const siblingNames = rows.filter((row) => parentDirOf(row.path) === targetDir).map((row) => row.name)
        const errorKey = validateEntryName(trimmedName, siblingNames)
        if (errorKey) {
            setDraftError(t(errorKey, { name: trimmedName }))
            return
        }

        const path = joinPath(draft.parentDir, trimmedName)
        try {
            await createEntry({ path, isDir: draft.kind === 'directory' })
            await refreshTreeDir({ projectId, dir: draft.parentDir })
            await revealTreeNode({ projectId, path })
            if (draft.kind === 'file') {
                openFileTab({ id: path, path, name: fileNameOf(path), depth: 0, kind: 'file', expanded: false, gitStatus: null }, false)
            }
            setDraft(null)
            setDraftError(null)
            setSelectPathRequest(path)
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error)
            setDraftError(message)
            toast.error(message, { action: { label: t('common.retry'), onClick: () => void commitDraft(trimmedName) } })
        }
    }

    const startRename = (row: FileTreeRow) => {
        setRenameTarget({ path: row.path, name: row.name })
        setRenameError(null)
    }

    const cancelRename = () => {
        setRenameTarget(null)
        setRenameError(null)
    }

    const commitRename = async (name: string) => {
        if (!renameTarget) return
        const trimmedName = name.trim()
        if (!trimmedName || trimmedName === renameTarget.name) {
            cancelRename()
            return
        }

        const parentDir = parentDirOf(renameTarget.path)
        const targetDir = resolveEntryParentDir(parentDir, trimmedName)
        const siblingNames = rows.filter((row) => parentDirOf(row.path) === targetDir && row.path !== renameTarget.path).map((row) => row.name)
        const errorKey = validateEntryName(trimmedName, siblingNames)
        if (errorKey) {
            setRenameError(t(errorKey, { name: trimmedName }))
            return
        }

        const destination = joinPath(parentDir, trimmedName)
        try {
            await renameEntryAsync({ from: renameTarget.path, to: destination })
            await refreshTreeDir({ projectId, dir: parentDir })
            await revealTreeNode({ projectId, path: destination })
            setRenameTarget(null)
            setRenameError(null)
            setSelectPathRequest(destination)
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error)
            setRenameError(message)
            toast.error(message, { action: { label: t('common.retry'), onClick: () => void commitRename(trimmedName) } })
        }
    }

    const confirmDelete = async () => {
        if (!deleteTarget) return
        const parentDir = parentDirOf(deleteTarget.path)
        try {
            await deleteEntryAsync(deleteTarget.path)
            await refreshTreeDir({ projectId, dir: parentDir })
            setDeleteTarget(null)
        } catch (error) {
            notifyError(error)
        }
    }

    return {
        draft,
        draftError,
        renameTarget,
        renameError,
        deleteTarget,
        setDeleteTarget,
        startDraft,
        cancelDraft,
        commitDraft,
        startRename,
        cancelRename,
        commitRename,
        confirmDelete,
    }
}
