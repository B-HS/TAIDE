import type { FC } from 'react'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { PaneId, PaneNode, ProjectId, TreeRow } from '@shared/api/bindings'
import type { FileTreeNodeKind, FileTreeRow } from '@features/explorer/file-tree-row'
import { EntryDeleteDialog } from '@features/explorer/entry-delete-dialog'
import { resolveEntryParentDir, validateEntryName } from '@shared/lib/entry-name'
import { buildUniqueEntryName } from '@shared/lib/unique-entry-name'
import { requestOpenFileHistory } from '@shared/lib/file-history-panel-bridge'
import { toRelativePath } from '@shared/lib/relative-path'
import { requestOpenSearchPanel } from '@shared/lib/search-panel-bridge'
import { setOpenWithOverride } from '@entities/editor/open-with-registry'
import { treeRowsQueryOptions, useRefreshTreeDir, useRevealTreeNode, useToggleTreeNode } from '@entities/tree/tree.query'
import { useOpenTab, useSplitPane } from '@entities/layout/layout.query'
import { useCopyEntry, useCreateEntry, useDeleteEntry, useRenameEntry } from '@entities/file/file.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { systemOpenInBrowser, systemRevealPath } from '@entities/system/system.ipc'
import type { FileTreeContextMenuHandlers, FileTreeDraft, FileTreeRenameTarget } from '@features/explorer/file-tree'
import { ExplorerPanel } from '@widgets/explorer/explorer-panel'
import { FileHistoryPanel } from '@widgets/file-history/file-history-panel'

type ExplorerContainerProps = {
    projectId: ProjectId
}

type ClipboardEntry = { mode: 'cut' | 'copy'; path: string }

const PATH_SEPARATOR = '/'

const toFileTreeRow = (row: TreeRow): FileTreeRow => ({
    id: row.path,
    path: row.path,
    name: row.name,
    depth: row.depth,
    kind: row.kind === 'directory' ? 'directory' : 'file',
    expanded: row.expanded,
    gitStatus: null,
})

const parentDirOf = (path: string) => {
    const index = path.lastIndexOf(PATH_SEPARATOR)
    return index <= 0 ? PATH_SEPARATOR : path.slice(0, index)
}

const joinPath = (dir: string, name: string) => `${dir.endsWith(PATH_SEPARATOR) ? dir.slice(0, -1) : dir}${PATH_SEPARATOR}${name}`

const findLeafPane = (node: PaneNode, paneId: PaneId): PaneNode | null => {
    if (node.node === 'leaf') return node.id === paneId ? node : null
    for (const child of node.children) {
        const found = findLeafPane(child, paneId)
        if (found) return found
    }
    return null
}

export const ExplorerContainer: FC<ExplorerContainerProps> = ({ projectId }) => {
    const { t } = useTranslation()
    const [selectedRow, setSelectedRow] = useState<FileTreeRow | null>(null)
    const [draft, setDraft] = useState<FileTreeDraft | null>(null)
    const [draftError, setDraftError] = useState<string | null>(null)
    const [renameTarget, setRenameTarget] = useState<FileTreeRenameTarget | null>(null)
    const [renameError, setRenameError] = useState<string | null>(null)
    const [deleteTarget, setDeleteTarget] = useState<FileTreeRow | null>(null)
    const [clipboard, setClipboard] = useState<ClipboardEntry | null>(null)
    const [selectPathRequest, setSelectPathRequest] = useState<string | null>(null)
    const [compareSourcePath, setCompareSourcePath] = useState<string | null>(null)

    const { data: page } = useQuery(treeRowsQueryOptions(projectId))
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { mutate: toggleNode, mutateAsync: toggleNodeAsync } = useToggleTreeNode(projectId)
    const { mutateAsync: refreshTreeDir } = useRefreshTreeDir(projectId)
    const { mutateAsync: revealTreeNode } = useRevealTreeNode(projectId)
    const { mutateAsync: createEntry } = useCreateEntry()
    const { mutateAsync: renameEntryAsync } = useRenameEntry(projectId)
    const { mutateAsync: copyEntryAsync } = useCopyEntry(projectId)
    const { mutateAsync: deleteEntryAsync } = useDeleteEntry(projectId)
    const { mutate: openTab, mutateAsync: openTabAsync } = useOpenTab(projectId)
    const { mutate: splitPane } = useSplitPane(projectId)

    const rows = (page?.rows ?? []).map(toFileTreeRow)

    const fileNameOf = (path: string) => path.slice(path.lastIndexOf(PATH_SEPARATOR) + 1)

    const notifyError = (error: unknown) => toast.error(error instanceof Error ? error.message : String(error))

    const targetDirFor = (row: FileTreeRow | null) => {
        if (row) return row.kind === 'directory' ? row.path : parentDirOf(row.path)
        return project?.root ?? null
    }

    const openFileTab = (row: FileTreeRow, preview: boolean) => {
        if (row.kind === 'directory') return
        openTab({ projectId, kind: { kind: 'file', path: row.path }, title: row.name, target: null, preview }, { onError: notifyError })
    }

    const openSearchMatch = (path: string) =>
        openTab({ projectId, kind: { kind: 'file', path }, title: fileNameOf(path), target: null, preview: true }, { onError: notifyError })

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

    const openToTheSide = async (row: FileTreeRow) => {
        if (row.kind !== 'file') return
        try {
            const layout = await openTabAsync({ projectId, kind: { kind: 'file', path: row.path }, title: row.name, target: null, preview: false })
            const pane = findLeafPane(layout.root, layout.focusedPane)
            const activeTabId = pane && pane.node === 'leaf' ? pane.active : null
            if (!activeTabId) return
            splitPane({ paneId: layout.focusedPane, edge: 'right', tabId: activeTabId })
        } catch (error) {
            notifyError(error)
        }
    }

    const openInTerminal = (row: FileTreeRow) => {
        const dir = targetDirFor(row)
        if (!dir) return
        openTab(
            { projectId, kind: { kind: 'terminal', sessionId: '', cwd: dir }, title: t('terminal.title'), target: null, preview: false },
            { onError: notifyError },
        )
    }

    const findInFolder = (row: FileTreeRow) => {
        if (row.kind !== 'directory' || !project) return
        requestOpenSearchPanel({ includeGlob: `${toRelativePath(project.root, row.path)}/**` })
    }

    const compareWithSelected = (row: FileTreeRow) => {
        if (!compareSourcePath) return
        openTab(
            {
                projectId,
                kind: { kind: 'diff', path: row.path, staged: false, compareWith: compareSourcePath },
                title: `${fileNameOf(compareSourcePath)} vs ${fileNameOf(row.path)}`,
                target: null,
                preview: true,
            },
            { onError: notifyError },
        )
        setCompareSourcePath(null)
    }

    const contextMenuHandlers: FileTreeContextMenuHandlers = {
        onOpenToTheSide: (row) => void openToTheSide(row),
        onOpenWithEditor: (row) => {
            setOpenWithOverride(row.path, 'editor')
            openFileTab(row, true)
        },
        onOpenWithPreview: (row) => {
            setOpenWithOverride(row.path, null)
            openFileTab(row, true)
        },
        onOpenInBrowser: (row) => void systemOpenInBrowser(row.path).catch(notifyError),
        onRevealInFinder: (row) => void systemRevealPath(row.path).catch(notifyError),
        onOpenInTerminal: openInTerminal,
        onFindInFolder: findInFolder,
        onSelectForCompare: (row) => setCompareSourcePath(row.path),
        onCompareWithSelected: compareWithSelected,
        canCompareWithSelected: compareSourcePath !== null,
        onFileHistory: (row) => requestOpenFileHistory(row.path),
        onCut: (row) => setClipboard({ mode: 'cut', path: row.path }),
        onCopy: (row) => setClipboard({ mode: 'copy', path: row.path }),
        onPaste: (row) => void pasteClipboard(row),
        onCopyPath: (row) => void navigator.clipboard.writeText(row.path),
        onCopyRelativePath: (row) => project && void navigator.clipboard.writeText(toRelativePath(project.root, row.path)),
        onStartRename: startRename,
        onRequestDelete: setDeleteTarget,
        onClearSelection: () => setSelectedRow(null),
    }

    const collapseAllExpanded = async () => {
        const expandedDirPaths = rows.filter((row) => row.kind === 'directory' && row.expanded).map((row) => row.path)
        for (const path of expandedDirPaths) {
            await toggleNodeAsync({ projectId, path })
        }
    }

    const refreshVisibleTree = async () => {
        if (!project) return
        const expandedDirPaths = rows.filter((row) => row.kind === 'directory' && row.expanded).map((row) => row.path)
        for (const dir of [project.root, ...expandedDirPaths]) {
            await refreshTreeDir({ projectId, dir })
        }
    }

    return (
        <>
            <ExplorerPanel
                projectId={projectId}
                rows={rows}
                draft={draft}
                draftError={draftError}
                renameTarget={renameTarget}
                renameError={renameError}
                selectPathRequest={selectPathRequest}
                canPaste={clipboard !== null}
                contextMenuHandlers={contextMenuHandlers}
                onToggleExpand={(row) => toggleNode({ projectId, path: row.path })}
                onOpenPreview={(row) => openFileTab(row, true)}
                onOpenPinned={(row) => openFileTab(row, false)}
                onSelectionChange={setSelectedRow}
                onOpenSearchMatch={openSearchMatch}
                onNewFile={() => void startDraft('file')}
                onNewFolder={() => void startDraft('directory')}
                onRefresh={() => void refreshVisibleTree()}
                onCollapseAll={() => void collapseAllExpanded()}
                onDraftCommit={(name) => void commitDraft(name)}
                onDraftCancel={cancelDraft}
                onRenameCommit={(name) => void commitRename(name)}
                onRenameCancel={cancelRename}
                onSelectPathRequestHandled={() => setSelectPathRequest(null)}
                onRevealInExplorerRequest={(path) => {
                    void (async () => {
                        await revealTreeNode({ projectId, path })
                        setSelectPathRequest(path)
                    })()
                }}
            />
            <EntryDeleteDialog entryName={deleteTarget?.name ?? null} onCancel={() => setDeleteTarget(null)} onConfirm={() => void confirmDelete()} />
            <FileHistoryPanel projectId={projectId} />
        </>
    )
}
