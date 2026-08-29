import { useState } from 'react'
import type { useTranslation } from 'react-i18next'
import type { ProjectId } from '@shared/api/bindings'
import type { useCopyEntry, useRenameEntry } from '@entities/file/file.query'
import type { useRefreshTreeDir, useRevealTreeNode } from '@entities/tree/tree.query'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import type { ExplorerClipboardEntry } from '@widgets/explorer/paste-plan'
import { isSamePlaceCutPaste, pasteWithUniqueEntryName } from '@widgets/explorer/paste-plan'
import { joinPath, parentDirOf } from '@widgets/explorer/explorer-path'

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
    const [clipboard, setClipboard] = useState<ExplorerClipboardEntry | null>(null)

    const pasteClipboard = async (row: FileTreeRow | null) => {
        if (!clipboard) return
        const targetDir = targetDirFor(row)
        if (!targetDir) return
        if (isSamePlaceCutPaste(clipboard, targetDir)) {
            setClipboard(null)
            return
        }

        const siblingNames = rows.filter((candidate) => parentDirOf(candidate.path) === targetDir).map((candidate) => candidate.name)

        try {
            const destinationName = await pasteWithUniqueEntryName({
                clipboard,
                siblingNames,
                conflictSuffix: t('explorer.pasteConflictSuffix'),
                run: async (candidateName) => {
                    const candidatePath = joinPath(targetDir, candidateName)
                    if (clipboard.mode === 'copy') {
                        await copyEntryAsync({ from: clipboard.path, to: candidatePath })
                        return
                    }
                    await renameEntryAsync({ from: clipboard.path, to: candidatePath })
                },
            })

            const destination = joinPath(targetDir, destinationName)
            if (clipboard.mode === 'cut') {
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
