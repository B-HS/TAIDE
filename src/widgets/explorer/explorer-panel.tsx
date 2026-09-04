import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { FolderTree, GitBranch, ListTree, Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { projectQueryOptions } from '@entities/project/project.query'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { FileTreeToolbar } from '@features/explorer/file-tree-toolbar'
import { cn } from '@shared/lib/cn'
import { subscribeShowExplorerView } from '@shared/lib/bridge/explorer-panel-bridge'
import type { SearchPanelRequest } from '@shared/lib/bridge/search-panel-bridge'
import { subscribeOpenSearchPanel } from '@shared/lib/bridge/search-panel-bridge'
import { subscribeRevealInExplorer } from '@shared/lib/bridge/explorer-reveal-bridge'
import type { FileTreeContextMenuHandlers, FileTreeDraft, FileTreeRenameTarget } from '@features/explorer/file-tree'
import { FileTree } from '@features/explorer/file-tree'
import { GitPanelContainer } from '@widgets/git-panel/git-panel-container'
import { OutlinePanelContainer } from '@widgets/outline-panel/outline-panel-container'
import { SearchPanelContainer } from '@widgets/search-panel/search-panel-container'
import { IconButton } from '@shared/ui/icon-button'

export type ExplorerView = 'files' | 'search' | 'git' | 'outline'

const EXPLORER_VIEWS: { id: ExplorerView; labelKey: string; icon: typeof FolderTree }[] = [
    { id: 'files', labelKey: 'explorer.title', icon: FolderTree },
    { id: 'search', labelKey: 'search.title', icon: Search },
    { id: 'git', labelKey: 'git.title', icon: GitBranch },
    { id: 'outline', labelKey: 'outline.title', icon: ListTree },
]

type ExplorerPanelProps = {
    projectId: ProjectId
    view: ExplorerView
    onViewChange: (view: ExplorerView) => void
    rows: FileTreeRow[]
    draft: FileTreeDraft | null
    draftError: string | null
    renameTarget: FileTreeRenameTarget | null
    renameError: string | null
    selectPathRequest: string | null
    canPaste: boolean
    contextMenuHandlers: FileTreeContextMenuHandlers
    onToggleExpand: (row: FileTreeRow) => void
    onOpenPreview: (row: FileTreeRow) => void
    onOpenPinned: (row: FileTreeRow) => void
    onSelectionChange: (row: FileTreeRow) => void
    onOpenSearchMatch: (path: string) => void
    onNewFile: () => void
    onNewFolder: () => void
    onRefresh: () => void
    onCollapseAll: () => void
    onDraftCommit: (name: string) => void
    onDraftCancel: () => void
    onRenameCommit: (name: string) => void
    onRenameCancel: () => void
    onSelectPathRequestHandled: () => void
    onRevealInExplorerRequest: (path: string) => void
}

/**
 * The sidebar's view switcher. Nothing above this remounts when the active project changes, so the
 * search view is keyed by `projectId` and the pending search request is dropped with it: without
 * that, switching projects left the previous project's query, results and folder scope on screen,
 * and clicking one of those stale matches asked the *new* project's layout to open a path that does
 * not belong to it (audit §4-B B9).
 */
export const ExplorerPanel: FC<ExplorerPanelProps> = ({
    projectId,
    view,
    onViewChange,
    rows,
    draft,
    draftError,
    renameTarget,
    renameError,
    selectPathRequest,
    canPaste,
    contextMenuHandlers,
    onToggleExpand,
    onOpenPreview,
    onOpenPinned,
    onSelectionChange,
    onOpenSearchMatch,
    onNewFile,
    onNewFolder,
    onRefresh,
    onCollapseAll,
    onDraftCommit,
    onDraftCancel,
    onRenameCommit,
    onRenameCancel,
    onSelectPathRequestHandled,
    onRevealInExplorerRequest,
}) => {
    const { t } = useTranslation()
    const [searchRequest, setSearchRequest] = useState<SearchPanelRequest | null>(null)
    const [openNonce, setOpenNonce] = useState(0)
    const [scopedProjectId, setScopedProjectId] = useState(projectId)

    const { data: project } = useQuery(projectQueryOptions(projectId))

    if (scopedProjectId !== projectId) {
        setScopedProjectId(projectId)
        setSearchRequest(null)
    }

    useEffect(
        () =>
            subscribeOpenSearchPanel((request) => {
                onViewChange('search')
                setSearchRequest(request)
                setOpenNonce((nonce) => nonce + 1)
            }),
        [onViewChange],
    )

    useEffect(() => subscribeShowExplorerView((requestedView) => onViewChange(requestedView)), [onViewChange])

    useEffect(
        () =>
            subscribeRevealInExplorer((path) => {
                onViewChange('files')
                onRevealInExplorerRequest(path)
            }),
        [onViewChange, onRevealInExplorerRequest],
    )

    return (
        <div className='group/explorer bg-explorer-background flex h-full min-h-0 w-full flex-col'>
            <div
                role='tablist'
                aria-label={t('explorer.sidebarSwitchLabel')}
                className='border-tab-bar-tab-border flex h-9 shrink-0 items-center gap-1 border-b px-2'>
                <div className='flex items-center gap-1'>
                    {EXPLORER_VIEWS.map(({ id, labelKey, icon: Icon }) => (
                        <IconButton
                            key={id}
                            role='tab'
                            aria-selected={view === id}
                            label={t(labelKey)}
                            icon={<Icon className='size-4' />}
                            onClick={() => onViewChange(id)}
                            side='bottom'
                            className={cn(
                                'flex size-6 items-center justify-center rounded-sm',
                                view === id
                                    ? 'bg-explorer-item-selected text-app-foreground'
                                    : 'text-app-sidebar-icon-default hover:bg-explorer-item-hover',
                            )}
                        />
                    ))}
                </div>
            </div>

            {view === 'files' && (
                <div className='border-tab-bar-tab-border flex h-8 shrink-0 items-center justify-between gap-1 border-b px-2'>
                    <h2 className='text-app-foreground truncate text-xs font-medium tracking-wide uppercase'>
                        {project?.name ?? t('explorer.title')}
                    </h2>
                    <FileTreeToolbar onNewFile={onNewFile} onNewFolder={onNewFolder} onRefresh={onRefresh} onCollapseAll={onCollapseAll} />
                </div>
            )}

            <div className='min-h-0 flex-1'>
                {view === 'files' && (
                    <FileTree
                        rows={rows}
                        draft={draft}
                        draftError={draftError}
                        renameTarget={renameTarget}
                        renameError={renameError}
                        selectPathRequest={selectPathRequest}
                        canPaste={canPaste}
                        contextMenuHandlers={contextMenuHandlers}
                        onToggleExpand={onToggleExpand}
                        onOpenPreview={onOpenPreview}
                        onOpenPinned={onOpenPinned}
                        onSelectionChange={onSelectionChange}
                        onDraftCommit={onDraftCommit}
                        onDraftCancel={onDraftCancel}
                        onRenameCommit={onRenameCommit}
                        onRenameCancel={onRenameCancel}
                        onSelectPathRequestHandled={onSelectPathRequestHandled}
                        onNewFile={onNewFile}
                        onNewFolder={onNewFolder}
                    />
                )}
                {view === 'search' && (
                    <SearchPanelContainer
                        key={projectId}
                        projectId={projectId}
                        onOpenMatch={onOpenSearchMatch}
                        includeGlob={searchRequest?.includeGlob ?? null}
                        onClearScope={() => setSearchRequest((current) => (current ? { ...current, includeGlob: null } : null))}
                        seedText={searchRequest?.seedText ?? null}
                        openReplace={searchRequest?.openReplace ?? false}
                        openNonce={openNonce}
                    />
                )}
                {view === 'git' && <GitPanelContainer projectId={projectId} />}
                {view === 'outline' && <OutlinePanelContainer projectId={projectId} />}
            </div>
        </div>
    )
}
