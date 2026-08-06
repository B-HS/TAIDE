import type { FC } from 'react'
import { useState } from 'react'
import { FolderTree, GitBranch, Search } from 'lucide-react'
import type { ProjectId } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { cn } from '@shared/lib/cn'
import { FileTree } from '@widgets/explorer/file-tree'
import { GitPanelContainer } from '@widgets/git-panel/git-panel-container'
import { SearchPanelContainer } from '@widgets/search-panel/search-panel-container'

type ExplorerView = 'files' | 'search' | 'git'

const EXPLORER_VIEWS: { id: ExplorerView; label: string; icon: typeof FolderTree }[] = [
    { id: 'files', label: '탐색기', icon: FolderTree },
    { id: 'search', label: '검색', icon: Search },
    { id: 'git', label: 'Git', icon: GitBranch },
]

type ExplorerPanelProps = {
    projectId: ProjectId
    rows: FileTreeRow[]
    onToggleExpand: (row: FileTreeRow) => void
    onOpenPreview: (row: FileTreeRow) => void
    onOpenPinned: (row: FileTreeRow) => void
    onOpenSearchMatch: (path: string) => void
}

export const ExplorerPanel: FC<ExplorerPanelProps> = ({ projectId, rows, onToggleExpand, onOpenPreview, onOpenPinned, onOpenSearchMatch }) => {
    const [view, setView] = useState<ExplorerView>('files')

    return (
        <div className='bg-explorer-background flex h-full min-h-0 w-full flex-col'>
            <div role='tablist' aria-label='탐색 사이드바 전환' className='border-app-border flex shrink-0 items-center gap-1 border-b px-2 py-1.5'>
                {EXPLORER_VIEWS.map(({ id, label, icon: Icon }) => (
                    <button
                        key={id}
                        type='button'
                        role='tab'
                        aria-selected={view === id}
                        aria-label={label}
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

            <div className='min-h-0 flex-1'>
                {view === 'files' && (
                    <FileTree rows={rows} onToggleExpand={onToggleExpand} onOpenPreview={onOpenPreview} onOpenPinned={onOpenPinned} />
                )}
                {view === 'search' && <SearchPanelContainer projectId={projectId} onOpenMatch={onOpenSearchMatch} />}
                {view === 'git' && <GitPanelContainer projectId={projectId} />}
            </div>
        </div>
    )
}
