import type { FC } from 'react'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { PaneId, PaneNode, ProjectId, TreeRow } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { EntryDeleteDialog } from '@features/explorer/entry-delete-dialog'
import { requestOpenFileHistory } from '@shared/lib/bridge/file-history-panel-bridge'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { fileNameOf, toRelativePath } from '@shared/lib/relative-path'
import { requestOpenSearchPanel } from '@shared/lib/bridge/search-panel-bridge'
import { setOpenWithOverride } from '@entities/editor/open-with-registry'
import { treeRowsQueryOptions, useRefreshTreeDir, useRevealTreeNode, useToggleTreeNode } from '@entities/tree/tree.query'
import { useOpenFileTab, useOpenTab, useSplitPane } from '@entities/layout/layout.query'
import { useCopyEntry, useCreateEntry, useDeleteEntry, useRenameEntry } from '@entities/file/file.query'
import { gitStatusQueryOptions } from '@entities/git/git.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { systemOpenInBrowser, systemRevealPath } from '@entities/system/system.ipc'
import type { FileTreeContextMenuHandlers } from '@features/explorer/file-tree'
import { buildFileTreeGitStatusByPath } from '@widgets/explorer/file-tree-git-status'
import { parentDirOf } from '@widgets/explorer/explorer-path'
import { useExplorerAutoReveal } from '@widgets/explorer/use-explorer-auto-reveal'
import { useExplorerClipboard } from '@widgets/explorer/use-explorer-clipboard'
import { useExplorerEntryCrud } from '@widgets/explorer/use-explorer-entry-crud'
import type { ExplorerView } from '@widgets/explorer/explorer-panel'
import { ExplorerPanel } from '@widgets/explorer/explorer-panel'
import { FileHistoryPanel } from '@widgets/file-history/file-history-panel'

type ExplorerContainerProps = {
    projectId: ProjectId
}

const toFileTreeRow = (row: TreeRow, gitStatus: FileTreeRow['gitStatus']): FileTreeRow => ({
    id: row.path,
    path: row.path,
    name: row.name,
    depth: row.depth,
    kind: row.kind === 'directory' ? 'directory' : 'file',
    expanded: row.expanded,
    gitStatus,
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
    const [view, setView] = useState<ExplorerView>('files')
    const [selectedRow, setSelectedRow] = useState<FileTreeRow | null>(null)
    const [selectPathRequest, setSelectPathRequest] = useState<string | null>(null)
    const [compareSourcePath, setCompareSourcePath] = useState<string | null>(null)

    const { data: page } = useQuery(treeRowsQueryOptions(projectId))
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { data: gitStatus } = useQuery(gitStatusQueryOptions(projectId))
    const { mutate: toggleNode, mutateAsync: toggleNodeAsync } = useToggleTreeNode(projectId)
    const { mutateAsync: refreshTreeDir } = useRefreshTreeDir(projectId)
    const { mutateAsync: revealTreeNode } = useRevealTreeNode(projectId)
    const { mutateAsync: createEntry } = useCreateEntry(projectId)
    const { mutateAsync: renameEntryAsync } = useRenameEntry(projectId)
    const { mutateAsync: copyEntryAsync } = useCopyEntry(projectId)
    const { mutateAsync: deleteEntryAsync } = useDeleteEntry(projectId)
    const { mutate: openTab } = useOpenTab(projectId)
    const openFileTab = useOpenFileTab()
    const { mutate: splitPane } = useSplitPane(projectId)

    const gitStatusByPath = buildFileTreeGitStatusByPath(gitStatus?.rows ?? [], project?.root ?? null)
    const rows = (page?.rows ?? []).map((row) => toFileTreeRow(row, gitStatusByPath.get(row.path) ?? null))

    const notifyError = (error: unknown) => toast.error(describeIpcError(error))

    const targetDirFor = (row: FileTreeRow | null) => {
        if (row) return row.kind === 'directory' ? row.path : parentDirOf(row.path)
        return project?.root ?? null
    }

    const openRowFileTab = (row: FileTreeRow, preview: boolean) => {
        if (row.kind === 'directory') return
        openFileTab({ projectId, path: row.path, title: row.name, target: null, preview })
    }

    const openSearchMatch = (path: string) => openFileTab({ projectId, path, target: null, preview: true })

    const crud = useExplorerEntryCrud({
        projectId,
        rows,
        selectedRow,
        targetDirFor,
        openFileTab: openRowFileTab,
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

    useExplorerAutoReveal({
        projectId,
        projectRoot: project?.root ?? null,
        rows,
        explorerViewActive: view === 'files',
        setSelectPathRequest,
        revealTreeNode,
    })

    const openToTheSide = (row: FileTreeRow) => {
        if (row.kind !== 'file') return
        openFileTab(
            { projectId, path: row.path, title: row.name, target: null, preview: false },
            {
                onSuccess: (layout) => {
                    const pane = findLeafPane(layout.root, layout.focusedPane)
                    const activeTabId = pane && pane.node === 'leaf' ? pane.active : null
                    if (!activeTabId) return
                    splitPane({ paneId: layout.focusedPane, edge: 'right', tabId: activeTabId })
                },
            },
        )
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
        onOpenToTheSide: openToTheSide,
        onOpenWithEditor: (row) => {
            setOpenWithOverride(row.path, 'editor')
            openRowFileTab(row, true)
        },
        onOpenWithPreview: (row) => {
            setOpenWithOverride(row.path, null)
            openRowFileTab(row, true)
        },
        onOpenInBrowser: (row) => void systemOpenInBrowser(row.path).catch(notifyError),
        onRevealInFinder: (row) => void systemRevealPath(row.path).catch(notifyError),
        onOpenInTerminal: openInTerminal,
        onFindInFolder: findInFolder,
        onSelectForCompare: (row) => setCompareSourcePath(row.path),
        onCompareWithSelected: compareWithSelected,
        canCompareWithSelected: compareSourcePath !== null,
        onFileHistory: (row) => requestOpenFileHistory(row.path),
        onCut: (row) => setClipboard({ mode: 'cut', path: row.path, kind: row.kind }),
        onCopy: (row) => setClipboard({ mode: 'copy', path: row.path, kind: row.kind }),
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
                view={view}
                onViewChange={setView}
                rows={rows}
                draft={crud.draft}
                draftError={crud.draftError}
                renameTarget={crud.renameTarget}
                renameError={crud.renameError}
                selectPathRequest={selectPathRequest}
                canPaste={clipboard !== null}
                contextMenuHandlers={contextMenuHandlers}
                onToggleExpand={(row) => toggleNode({ projectId, path: row.path })}
                onOpenPreview={(row) => openRowFileTab(row, true)}
                onOpenPinned={(row) => openRowFileTab(row, false)}
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
