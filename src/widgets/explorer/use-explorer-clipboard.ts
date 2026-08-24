import { useState } from 'react'
import type { useTranslation } from 'react-i18next'
import type { ProjectId } from '@shared/api/bindings'
import type { useCopyEntry, useRenameEntry } from '@entities/file/file.query'
import type { useRefreshTreeDir, useRevealTreeNode } from '@entities/tree/tree.query'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { buildUniqueEntryName } from '@shared/lib/unique-entry-name'
import { fileNameOf } from '@shared/lib/relative-path'
import { joinPath, parentDirOf } from '@widgets/explorer/explorer-path'

type ClipboardEntry = { mode: 'cut' | 'copy'; path: string }

type UseExplorerClipboardInput = {
    projectId: ProjectId
    rows: FileTreeRow[]
    targetDirFor: (row: FileTreeRow | null) => string | null
    notifyError: (error: unknown) => void
    setSelectPathRequest: (path: string) => void
    copyEntryAsync: ReturnType<typeof useCopyEntry>['mutateAsync']
    renameEntryAsync: ReturnType<typeof useRenameEntry>['mutateAsync']
    refreshTreeDir: ReturnType<typeof useRefreshTreeDir>['mutateAsync']
    revealTreeNode: ReturnType<typeof useRevealTreeNode>['mutateAsync']
    t: ReturnType<typeof useTranslation>['t']
}

export const useExplorerClipboard = ({
    projectId,
    rows,
    targetDirFor,
    notifyError,
    setSelectPathRequest,
    copyEntryAsync,
    renameEntryAsync,
    refreshTreeDir,
    revealTreeNode,
    t,
}: UseExplorerClipboardInput) => {
    const [clipboard, setClipboard] = useState<ClipboardEntry | null>(null)

    const pasteClipboard = async (row: FileTreeRow | null) => {
        if (!clipboard) return
        const targetDir = targetDirFor(row)
        if (!targetDir) return

        const entryName = fileNameOf(clipboard.path)
        const siblingNames = rows.filter((candidate) => parentDirOf(candidate.path) === targetDir).map((candidate) => candidate.name)
        const uniqueName = buildUniqueEntryName(entryName, siblingNames, t('explorer.pasteConflictSuffix'))
        const destination = joinPath(targetDir, uniqueName)

        try {
            if (clipboard.mode === 'copy') {
                await copyEntryAsync({ from: clipboard.path, to: destination })
            } else {
                await renameEntryAsync({ from: clipboard.path, to: destination })
                await refreshTreeDir({ projectId, dir: parentDirOf(clipboard.path) })
                setClipboard(null)
            }
            await refreshTreeDir({ projectId, dir: targetDir })
            await revealTreeNode({ projectId, path: destination })
            setSelectPathRequest(destination)
        } catch (error) {
            notifyError(error)
        }
    }

    return { clipboard, setClipboard, pasteClipboard }
}
