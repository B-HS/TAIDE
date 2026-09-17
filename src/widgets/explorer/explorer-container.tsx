import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { ProjectId, TreeRow } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { EntryDeleteDialog } from '@features/explorer/entry-delete-dialog'
import { requestOpenFileHistory } from '@shared/lib/bridge/file-history-panel-bridge'
import { copyTextToClipboard } from '@shared/lib/copy-text-to-clipboard'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { PERF_MARK, PERF_MEASURE, perfMark, perfMeasure } from '@shared/lib/perf-mark'
import { fileNameOf, toRelativePath } from '@shared/lib/relative-path'
import { requestOpenSearchPanel } from '@shared/lib/bridge/search-panel-bridge'
import { setOpenWithOverride } from '@entities/editor/open-with-registry'
import { treeRowsQueryOptions, useCollapseAllTree, useRefreshTreeDir, useRevealTreeNode, useToggleTreeNode } from '@entities/tree/tree.query'
import { layoutQueryOptions, useOpenFileTab, useOpenTab, useOpenTabInSplit } from '@entities/layout/layout.query'
import { useCopyEntry, useCreateEntry, useDeleteEntry, useRenameEntry } from '@entities/file/file.query'
import { gitStatusQueryOptions } from '@entities/git/git.query'
import { projectQueryOptions, useCloseProject, useOpenProject } from '@entities/project/project.query'
import { systemOpenInBrowser, systemRevealPath } from '@entities/system/system.ipc'
import type { FileTreeContextMenuHandlers } from '@features/explorer/file-tree'
import { buildFileTreeGitStatusByPath } from '@widgets/explorer/file-tree-git-status'
import { resolveTargetDir } from '@widgets/explorer/explorer-path'
import { planOpenToTheSide } from '@widgets/explorer/open-to-the-side-plan'
import { useExplorerAutoReveal } from '@widgets/explorer/use-explorer-auto-reveal'
import { useExplorerClipboard } from '@widgets/explorer/use-explorer-clipboard'
import { useExplorerEntryCrud } from '@widgets/explorer/use-explorer-entry-crud'
import type { ExplorerView } from '@widgets/explorer/explorer-panel'
import { ExplorerPanel } from '@widgets/explorer/explorer-panel'
import { FileHistoryPanel } from '@widgets/file-history/file-history-panel'

type ExplorerContainerProps = {
    projectId: ProjectId
    /** Zen mode is a property of the window, not of this project (d-62 §0.1 S-6), so it comes in as a prop — an auxiliary window passes `false` because it has no Zen mode of its own. */
    zen: boolean
    /**
     * Whether the shell has this panel collapsed. Window-level for the same reason `zen` is: the
     * main window persists its own collapse to `shell_view.sidebarCollapsed` while an auxiliary
     * window keeps a view-local one (`auxiliary-window-shell.tsx`), so reading the persisted field
     * here would let the main window's ⌘B silence an auxiliary window's auto-reveal (d-66 #18).
     */
    sidebarCollapsed: boolean
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

export const ExplorerContainer: FC<ExplorerContainerProps> = ({ projectId, zen, sidebarCollapsed }) => {
    const { t } = useTranslation()
    const [view, setView] = useState<ExplorerView>('files')
    const [selectedId, setSelectedId] = useState<string | null>(null)
    const [selectPathRequest, setSelectPathRequest] = useState<string | null>(null)
    const [compareSourcePath, setCompareSourcePath] = useState<string | null>(null)

    const { data: page, isError: isTreeRowsError } = useQuery(treeRowsQueryOptions(projectId))
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { data: gitStatus } = useQuery(gitStatusQueryOptions(projectId))
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { mutate: toggleNode, mutateAsync: toggleNodeAsync } = useToggleTreeNode(projectId)
    const { mutate: collapseAllTree } = useCollapseAllTree(projectId)
    const { mutateAsync: refreshTreeDir } = useRefreshTreeDir(projectId)
    const { mutateAsync: revealTreeNode } = useRevealTreeNode(projectId)
    const { mutateAsync: createEntry } = useCreateEntry(projectId)
    const { mutateAsync: renameEntryAsync } = useRenameEntry(projectId)
    const { mutateAsync: copyEntryAsync } = useCopyEntry(projectId)
    const { mutateAsync: deleteEntryAsync } = useDeleteEntry(projectId)
    const { mutate: openTab } = useOpenTab(projectId)
    const openFileTab = useOpenFileTab()
    const { mutate: openTabInSplit } = useOpenTabInSplit(projectId)
    const { mutate: closeProject } = useCloseProject()
    const { mutate: openProject } = useOpenProject()

    /**
     * Either signal that this project's folder cannot be read: the flag Rust computed when it restored
     * the session (`ProjectRef.root_missing`), or a `tree_rows` failure for a folder that went away
     * while the app was running — which the restore flag never notices, because nothing recomputes it
     * for an already-open project (d-67 #15). Both leave the tree empty and both are recovered the
     * same way, so they share one empty state rather than one of them staying silent.
     */
    const rootUnavailable = project?.rootMissing === true || isTreeRowsError

    const gitStatusByPath = buildFileTreeGitStatusByPath(gitStatus?.rows ?? [], project?.root ?? null)
    const rows = (page?.rows ?? []).map((row) => toFileTreeRow(row, gitStatusByPath.get(row.path) ?? null))

    /**
     * The selection is held as an id and the row is looked up in the page every render, so a row
     * that the tree has since lost (a deleted directory, a watcher-driven rename) resolves to `null`
     * instead of lingering as a stale object that later actions would still aim at (d-66 #13).
     */
    const selectedRow = rows.find((row) => row.id === selectedId) ?? null

    const notifyError = (error: unknown) => toast.error(describeIpcError(error))

    const targetDirFor = (row: FileTreeRow | null) => resolveTargetDir(row, rows, project?.root ?? null)

    const openRowFileTab = (row: FileTreeRow, preview: boolean) => {
        if (row.kind === 'directory') return
        openFileTab({ projectId, path: row.path, title: row.name, target: null, preview })
    }

    const openSearchMatch = (path: string, line: number, column: number) =>
        openFileTab({ projectId, path, target: null, preview: true, reveal: { line, column } })

    const crud = useExplorerEntryCrud({
        projectId,
        projectRoot: project?.root ?? null,
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
        zen,
        sidebarCollapsed,
        setSelectPathRequest,
        revealTreeNode,
    })

    /** One mutation that opens the file straight into a new pane — see {@link planOpenToTheSide} for why the open-then-split pair it replaced could end without a split. */
    const openToTheSide = (row: FileTreeRow) => {
        const request = planOpenToTheSide(projectId, row, layout)
        if (request) openTabInSplit(request, { onError: notifyError })
    }

    const openInTerminal = (row: FileTreeRow) => {
        const dir = targetDirFor(row)
        if (!dir) return
        openTab(
            { projectId, kind: { kind: 'terminal', sessionId: '', cwd: dir }, title: t('terminal.title'), target: null, preview: false },
            { onError: notifyError },
        )
    }

    /**
     * Sends the folder as an explicit `scopeDir`, not as a `<dir>/**` include glob. The glob form was
     * silently unreachable for exactly the folders this entry is most used on: the backend walk prunes
     * `IGNORED_DIR_NAMES` before any glob is consulted, so right-clicking `node_modules`/`dist`/
     * `target` and searching a string that is certainly there answered "no results" with no hint that
     * the scope had been dropped (audit wave 2 #23). A scope moves the walk root and suspends that
     * pruning inside the subtree the user pointed at, which is what makes the answer truthful.
     */
    const findInFolder = (row: FileTreeRow) => {
        if (row.kind !== 'directory' || !project) return
        requestOpenSearchPanel({ scopeDir: toRelativePath(project.root, row.path) })
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
        onCopyPath: (row) => void copyTextToClipboard(row.path),
        onCopyRelativePath: (row) => project && void copyTextToClipboard(toRelativePath(project.root, row.path)),
        onStartRename: crud.startRename,
        onRequestDelete: crud.setDeleteTarget,
    }

    /**
     * Reveals a path in the tree and selects it — the shared half of the two bridges the tab bar
     * drives ("Reveal in Explorer View" and "Rename"). The reveal mutation answers with the tree
     * page it just expanded, so the rename request reads its row from that response instead of the
     * `rows` snapshot this closure captured before the await.
     */
    const revealTreePath = async (path: string) => {
        const page = await revealTreeNode({ projectId, path })
        setSelectPathRequest(path)
        return page
    }

    const startRenameAtPath = async (path: string) => {
        const page = await revealTreePath(path)
        const revealed = page.rows.find((row) => row.path === path)
        if (!revealed) return
        crud.startRename(toFileTreeRow(revealed, gitStatusByPath.get(revealed.path) ?? null))
    }

    /**
     * The only recovery from a project whose capabilities were never attached: `close_project` drops
     * the in-memory record entirely, so reopening the same root takes `open_project`'s fresh-open
     * branch and attaches the watcher and git capability this project has been missing all session
     * (d-67 #15). Both mutations invalidate `PROJECT.ALL`, so the rail follows the new project id.
     */
    const reopenProject = () => {
        if (!project) return
        const { root } = project
        closeProject(projectId, { onSuccess: () => openProject(root, { onError: notifyError }), onError: notifyError })
    }

    /** Opens the tree-expand span (metric 5) around the mutation the user's click starts. */
    const handleToggleExpand = (row: FileTreeRow) => {
        perfMark(PERF_MARK.TREE_TOGGLE_REQUESTED)
        toggleNode({ projectId, path: row.path })
    }

    const refreshVisibleTree = async () => {
        if (!project) return
        const expandedDirPaths = rows.filter((row) => row.kind === 'directory' && row.expanded).map((row) => row.path)
        for (const dir of [project.root, ...expandedDirPaths]) {
            await refreshTreeDir({ projectId, dir })
        }
    }

    /**
     * Closes whichever tree span is open now that a new page has been committed — the project switch
     * marked by `app-shell.tsx` (metric 2) or the expand marked just above (metric 5). Both live here
     * because this is the commit that actually paints rows, and both are no-ops unless their own
     * start mark is still unconsumed, so a page arriving from a refresh, a reveal, or a watcher
     * invalidation measures nothing (`perf-mark.ts`, consume-on-measure).
     */
    useEffect(() => {
        if (!page) return
        perfMeasure(PERF_MEASURE.PROJECT_SWITCH, PERF_MARK.PROJECT_SWITCH_REQUESTED)
        perfMeasure(PERF_MEASURE.TREE_TOGGLE, PERF_MARK.TREE_TOGGLE_REQUESTED)
    }, [page])

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
                rootUnavailable={rootUnavailable}
                contextMenuHandlers={contextMenuHandlers}
                onToggleExpand={handleToggleExpand}
                onOpenPreview={(row) => openRowFileTab(row, true)}
                onOpenPinned={(row) => openRowFileTab(row, false)}
                onSelectionChange={setSelectedId}
                onOpenSearchMatch={openSearchMatch}
                onNewFile={() => void crud.startDraft('file')}
                onNewFolder={() => void crud.startDraft('directory')}
                onNewFileAtRoot={() => project && void crud.startDraft('file', project.root)}
                onRefresh={() => void refreshVisibleTree()}
                onCollapseAll={() => collapseAllTree({ projectId })}
                onDraftCommit={(name) => void crud.commitDraft(name)}
                onDraftCancel={crud.cancelDraft}
                onRenameCommit={(name) => void crud.commitRename(name)}
                onRenameCancel={crud.cancelRename}
                onSelectPathRequestHandled={() => setSelectPathRequest(null)}
                onRevealInExplorerRequest={(path) => void revealTreePath(path)}
                onRenameInExplorerRequest={(path) => void startRenameAtPath(path)}
                onReopenProject={reopenProject}
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
