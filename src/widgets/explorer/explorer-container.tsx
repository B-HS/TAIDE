import type { FC } from 'react'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { PaneId, PaneNode, ProjectId, TreeRow } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { EntryDeleteDialog } from '@features/explorer/entry-delete-dialog'
import { requestOpenFileHistory } from '@shared/lib/file-history-panel-bridge'
import { fileNameOf, toRelativePath } from '@shared/lib/relative-path'
import { requestOpenSearchPanel } from '@shared/lib/search-panel-bridge'
import { setOpenWithOverride } from '@entities/editor/open-with-registry'
import { treeRowsQueryOptions, useRefreshTreeDir, useRevealTreeNode, useToggleTreeNode } from '@entities/tree/tree.query'
import { useOpenTab, useSplitPane } from '@entities/layout/layout.query'
import { useCopyEntry, useCreateEntry, useDeleteEntry, useRenameEntry } from '@entities/file/file.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { systemOpenInBrowser, systemRevealPath } from '@entities/system/system.ipc'
import type { FileTreeContextMenuHandlers } from '@features/explorer/file-tree'
import { parentDirOf } from '@widgets/explorer/explorer-path'
import { useExplorerClipboard } from '@widgets/explorer/use-explorer-clipboard'
import { useExplorerEntryCrud } from '@widgets/explorer/use-explorer-entry-crud'
import { ExplorerPanel } from '@widgets/explorer/explorer-panel'
import { FileHistoryPanel } from '@widgets/file-history/file-history-panel'

type ExplorerContainerProps = {
    projectId: ProjectId
}

const toFileTreeRow = (row: TreeRow): FileTreeRow => ({
    id: row.path,
    path: row.path,
    name: row.name,
    depth: row.depth,
    kind: row.kind === 'directory' ? 'directory' : 'file',
    expanded: row.expanded,
    gitStatus: null,
})

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

    const crud = useExplorerEntryCrud({
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
    })

    const { clipboard, setClipboard, pasteClipboard } = useExplorerClipboard({
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
    })

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
        onStartRename: crud.startRename,
        onRequestDelete: crud.setDeleteTarget,
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
                draft={crud.draft}
                draftError={crud.draftError}
                renameTarget={crud.renameTarget}
                renameError={crud.renameError}
                selectPathRequest={selectPathRequest}
                canPaste={clipboard !== null}
                contextMenuHandlers={contextMenuHandlers}
                onToggleExpand={(row) => toggleNode({ projectId, path: row.path })}
                onOpenPreview={(row) => openFileTab(row, true)}
                onOpenPinned={(row) => openFileTab(row, false)}
                onSelectionChange={setSelectedRow}
                onOpenSearchMatch={openSearchMatch}
                onNewFile={() => void crud.startDraft('file')}
                onNewFolder={() => void crud.startDraft('directory')}
                onRefresh={() => void refreshVisibleTree()}
                onCollapseAll={() => void collapseAllExpanded()}
                onDraftCommit={(name) => void crud.commitDraft(name)}
                onDraftCancel={crud.cancelDraft}
                onRenameCommit={(name) => void crud.commitRename(name)}
                onRenameCancel={crud.cancelRename}
                onSelectPathRequestHandled={() => setSelectPathRequest(null)}
                onRevealInExplorerRequest={(path) => {
                    void (async () => {
                        await revealTreeNode({ projectId, path })
                        setSelectPathRequest(path)
                    })()
                }}
            />
            <EntryDeleteDialog
                entryName={crud.deleteTarget?.name ?? null}
                onCancel={() => crud.setDeleteTarget(null)}
                onConfirm={() => void crud.confirmDelete()}
            />
            <FileHistoryPanel projectId={projectId} />
        </>
    )
}
