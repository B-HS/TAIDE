import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { AlertTriangle, FolderTree, GitBranch, ListTree, Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { ProjectId } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { FileTreeToolbar } from '@features/explorer/file-tree-toolbar'
import { cn } from '@shared/lib/cn'
import { subscribeOpenSearchPanel } from '@shared/lib/search-panel-bridge'
import { FileTree } from '@widgets/explorer/file-tree'
import { GitPanelContainer } from '@widgets/git-panel/git-panel-container'
import { OutlinePanelContainer } from '@widgets/outline-panel/outline-panel-container'
import { ProblemsPanelContainer } from '@widgets/problems-panel/problems-panel-container'
import { SearchPanelContainer } from '@widgets/search-panel/search-panel-container'

type ExplorerView = 'files' | 'search' | 'git' | 'problems' | 'outline'

const EXPLORER_VIEWS: { id: ExplorerView; labelKey: string; icon: typeof FolderTree }[] = [
    { id: 'files', labelKey: 'explorer.title', icon: FolderTree },
    { id: 'search', labelKey: 'search.title', icon: Search },
    { id: 'git', labelKey: 'git.title', icon: GitBranch },
    { id: 'problems', labelKey: 'problems.title', icon: AlertTriangle },
    { id: 'outline', labelKey: 'outline.title', icon: ListTree },
]

type ExplorerPanelProps = {
    projectId: ProjectId
    rows: FileTreeRow[]
    onToggleExpand: (row: FileTreeRow) => void
    onOpenPreview: (row: FileTreeRow) => void
    onOpenPinned: (row: FileTreeRow) => void
    onSelectionChange: (row: FileTreeRow) => void
    onOpenSearchMatch: (path: string) => void
    onCreateFile: (name: string) => void
    onCreateFolder: (name: string) => void
    onRefresh: () => void
    onCollapseAll: () => void
}

export const ExplorerPanel: FC<ExplorerPanelProps> = ({
    projectId,
    rows,
    onToggleExpand,
    onOpenPreview,
    onOpenPinned,
    onSelectionChange,
    onOpenSearchMatch,
    onCreateFile,
    onCreateFolder,
    onRefresh,
    onCollapseAll,
}) => {
    const { t } = useTranslation()
    const [view, setView] = useState<ExplorerView>('files')

    useEffect(() => subscribeOpenSearchPanel(() => setView('search')), [])

    return (
        <div className='bg-explorer-background flex h-full min-h-0 w-full flex-col'>
            <div
                role='tablist'
                aria-label={t('explorer.sidebarSwitchLabel')}
                className='border-app-border flex shrink-0 items-center justify-between gap-1 border-b px-2 py-1.5'>
                <div className='flex items-center gap-1'>
                    {EXPLORER_VIEWS.map(({ id, labelKey, icon: Icon }) => (
                        <button
                            key={id}
                            type='button'
                            role='tab'
                            aria-selected={view === id}
                            aria-label={t(labelKey)}
                            onClick={() => setView(id)}
                            className={cn(
                                'flex size-6 items-center justify-center rounded-sm',
                                view === id
                                    ? 'bg-explorer-item-selected text-app-foreground'
                                    : 'text-app-sidebar-icon-default hover:bg-explorer-item-hover',
                            )}>
                            <Icon className='size-4' />
                        </button>
                    ))}
                </div>

                {view === 'files' && (
                    <FileTreeToolbar
                        onCreateFile={onCreateFile}
                        onCreateFolder={onCreateFolder}
                        onRefresh={onRefresh}
                        onCollapseAll={onCollapseAll}
                    />
                )}
            </div>

            <div className='min-h-0 flex-1'>
                {view === 'files' && (
                    <FileTree
                        rows={rows}
                        onToggleExpand={onToggleExpand}
                        onOpenPreview={onOpenPreview}
                        onOpenPinned={onOpenPinned}
                        onSelectionChange={onSelectionChange}
                    />
                )}
                {view === 'search' && <SearchPanelContainer projectId={projectId} onOpenMatch={onOpenSearchMatch} />}
                {view === 'git' && <GitPanelContainer projectId={projectId} />}
                {view === 'problems' && <ProblemsPanelContainer projectId={projectId} />}
                {view === 'outline' && <OutlinePanelContainer projectId={projectId} />}
            </div>
        </div>
    )
}
