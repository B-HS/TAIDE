import type { GitRemote, StatusRow } from '@shared/api/bindings'
import type { FC } from 'react'
import { useState } from 'react'
import { ArrowDown, ArrowUp, File, GitBranch, Loader2, Minus, Plus, RefreshCw, Undo2 } from 'lucide-react'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@shared/ui/alert-dialog'
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuSeparator, ContextMenuTrigger } from '@shared/ui/context-menu'
import { CommitBox } from '@features/git/commit-box'
import { ResourceGroupHeader } from '@features/git/resource-group-header'
import type { GitStatusChangeKind, StatusRowAction } from '@features/git/status-row-item'
import { StatusRowItem } from '@features/git/status-row-item'
import { CommitGraph, type GraphLogEntry } from '@widgets/git-panel/commit-graph'

export type { GitStatusChangeKind } from '@features/git/status-row-item'

export type GitStatusRow = StatusRow

export type GitRemoteInfo = GitRemote

export type GitPanelProps = {
    branch: string | null
    ahead: number
    behind: number
    hasRemote: boolean
    remote: GitRemoteInfo | null
    rows: GitStatusRow[]
    commitMessage: string
    onCommitMessageChange: (value: string) => void
    onCommit: () => void
    isCommitting: boolean
    onStage: (paths: string[]) => void
    onUnstage: (paths: string[]) => void
    onDiscard: (paths: string[]) => void
    onOpenFile: (path: string) => void
    onOpenChanges: (path: string, group: 'staged' | 'unstaged') => void
    onCopyPath: (path: string) => void
    onRevealInExplorer: (path: string) => void
    onSync: () => void
    isSyncing: boolean
    graphCommits: GraphLogEntry[]
}

const isStagedRow = (row: GitStatusRow): row is GitStatusRow & { staged: GitStatusChangeKind } => !row.isConflicted && row.staged !== null

const isUnstagedRow = (row: GitStatusRow): row is GitStatusRow & { unstaged: GitStatusChangeKind } => !row.isConflicted && row.unstaged !== null

export const GitPanel: FC<GitPanelProps> = ({
    branch,
    ahead,
    behind,
    hasRemote,
    remote,
    rows,
    commitMessage,
    onCommitMessageChange,
    onCommit,
    isCommitting,
    onStage,
    onUnstage,
    onDiscard,
    onOpenFile,
    onOpenChanges,
    onCopyPath,
    onRevealInExplorer,
    onSync,
    isSyncing,
    graphCommits,
}) => {
    const [discardTargets, setDiscardTargets] = useState<string[] | null>(null)
    const [confirmStageAllOpen, setConfirmStageAllOpen] = useState(false)

    const mergeRows = rows.filter((row) => row.isConflicted)
    const stagedRows = rows.filter(isStagedRow)
    const unstagedRows = rows.filter(isUnstagedRow)

    const requestCommit = () => {
        if (stagedRows.length === 0 && unstagedRows.length > 0) {
            setConfirmStageAllOpen(true)
            return
        }
        onCommit()
    }

    const confirmStageAllAndCommit = () => {
        onStage(unstagedRows.map((row) => row.path))
        onCommit()
        setConfirmStageAllOpen(false)
    }

    const confirmDiscard = () => {
        if (!discardTargets) return
        onDiscard(discardTargets)
        setDiscardTargets(null)
    }

    return (
        <div className='flex h-full min-h-0 w-full flex-col'>
            <div className='border-app-border flex shrink-0 items-center gap-1.5 border-b px-2 py-1.5 text-xs'>
                <GitBranch className='size-3.5 shrink-0' />
                <span className='truncate font-medium'>{branch ?? '리포지토리 없음'}</span>
                {hasRemote && ahead > 0 && (
                    <span className='flex items-center gap-0.5'>
                        <ArrowUp className='size-3' />
                        {ahead}
                    </span>
                )}
                {hasRemote && behind > 0 && (
                    <span className='flex items-center gap-0.5'>
                        <ArrowDown className='size-3' />
                        {behind}
                    </span>
                )}
                {hasRemote && (
                    <button
                        type='button'
                        aria-label='동기화'
                        disabled={isSyncing}
                        onClick={onSync}
                        className='hover:bg-explorer-item-hover ml-auto flex size-5 shrink-0 items-center justify-center rounded-sm disabled:opacity-50'>
                        {isSyncing ? <Loader2 className='size-3.5 animate-spin' /> : <RefreshCw className='size-3.5' />}
                    </button>
                )}
                {remote && <span className='text-app-sidebar-icon-default ml-1 shrink-0 truncate text-[11px]'>{remote.name}</span>}
            </div>

            <CommitBox message={commitMessage} onMessageChange={onCommitMessageChange} onCommit={requestCommit} isCommitting={isCommitting} />

            <div className='min-h-0 flex-1 overflow-y-auto'>
                {mergeRows.length > 0 && (
                    <div>
                        <ResourceGroupHeader title='Merge Changes' count={mergeRows.length} />
                        {mergeRows.map((row) => {
                            const actions: StatusRowAction[] = [
                                { id: 'open-file', label: 'Open File', icon: <File className='size-3' />, onClick: () => onOpenFile(row.path) },
                            ]
                            return (
                                <ContextMenu key={row.path}>
                                    <ContextMenuTrigger>
                                        <StatusRowItem
                                            path={row.path}
                                            origPath={row.origPath ?? null}
                                            kind='conflicted'
                                            selected={false}
                                            actions={actions}
                                            onClick={() => onOpenChanges(row.path, 'unstaged')}
                                        />
                                    </ContextMenuTrigger>
                                    <ContextMenuContent>
                                        <ContextMenuItem onSelect={() => onOpenFile(row.path)}>Open File</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onOpenChanges(row.path, 'unstaged')}>Open Changes</ContextMenuItem>
                                        <ContextMenuSeparator />
                                        <ContextMenuItem onSelect={() => onCopyPath(row.path)}>Copy Path</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onRevealInExplorer(row.path)}>Reveal in Explorer</ContextMenuItem>
                                    </ContextMenuContent>
                                </ContextMenu>
                            )
                        })}
                    </div>
                )}

                {stagedRows.length > 0 && (
                    <div>
                        <ResourceGroupHeader
                            title='Staged Changes'
                            count={stagedRows.length}
                            actionLabel='Unstage All'
                            actionIcon={<Minus className='size-3' />}
                            onAction={() => onUnstage(stagedRows.map((row) => row.path))}
                        />
                        {stagedRows.map((row) => {
                            const actions: StatusRowAction[] = [
                                { id: 'unstage', label: 'Unstage Changes', icon: <Minus className='size-3' />, onClick: () => onUnstage([row.path]) },
                                { id: 'open-file', label: 'Open File', icon: <File className='size-3' />, onClick: () => onOpenFile(row.path) },
                            ]
                            return (
                                <ContextMenu key={row.path}>
                                    <ContextMenuTrigger>
                                        <StatusRowItem
                                            path={row.path}
                                            origPath={row.origPath ?? null}
                                            kind={row.staged}
                                            selected={false}
                                            actions={actions}
                                            onClick={() => onOpenChanges(row.path, 'staged')}
                                        />
                                    </ContextMenuTrigger>
                                    <ContextMenuContent>
                                        <ContextMenuItem onSelect={() => onOpenFile(row.path)}>Open File</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onOpenChanges(row.path, 'staged')}>Open Changes</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onUnstage([row.path])}>Unstage Changes</ContextMenuItem>
                                        <ContextMenuSeparator />
                                        <ContextMenuItem onSelect={() => onCopyPath(row.path)}>Copy Path</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onRevealInExplorer(row.path)}>Reveal in Explorer</ContextMenuItem>
                                    </ContextMenuContent>
                                </ContextMenu>
                            )
                        })}
                    </div>
                )}

                {unstagedRows.length > 0 && (
                    <div>
                        <ResourceGroupHeader
                            title='Changes'
                            count={unstagedRows.length}
                            actionLabel='Stage All'
                            actionIcon={<Plus className='size-3' />}
                            onAction={() => onStage(unstagedRows.map((row) => row.path))}
                        />
                        {unstagedRows.map((row) => {
                            const actions: StatusRowAction[] = [
                                { id: 'stage', label: 'Stage Changes', icon: <Plus className='size-3' />, onClick: () => onStage([row.path]) },
                                {
                                    id: 'discard',
                                    label: 'Discard Changes',
                                    icon: <Undo2 className='size-3' />,
                                    onClick: () => setDiscardTargets([row.path]),
                                },
                                { id: 'open-file', label: 'Open File', icon: <File className='size-3' />, onClick: () => onOpenFile(row.path) },
                            ]
                            return (
                                <ContextMenu key={row.path}>
                                    <ContextMenuTrigger>
                                        <StatusRowItem
                                            path={row.path}
                                            origPath={row.origPath ?? null}
                                            kind={row.unstaged}
                                            selected={false}
                                            actions={actions}
                                            onClick={() => onOpenChanges(row.path, 'unstaged')}
                                        />
                                    </ContextMenuTrigger>
                                    <ContextMenuContent>
                                        <ContextMenuItem onSelect={() => onOpenFile(row.path)}>Open File</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onOpenChanges(row.path, 'unstaged')}>Open Changes</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onStage([row.path])}>Stage Changes</ContextMenuItem>
                                        <ContextMenuItem variant='destructive' onSelect={() => setDiscardTargets([row.path])}>
                                            Discard Changes
                                        </ContextMenuItem>
                                        <ContextMenuSeparator />
                                        <ContextMenuItem onSelect={() => onCopyPath(row.path)}>Copy Path</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onRevealInExplorer(row.path)}>Reveal in Explorer</ContextMenuItem>
                                    </ContextMenuContent>
                                </ContextMenu>
                            )
                        })}
                    </div>
                )}

                {graphCommits.length > 0 && (
                    <div className='border-app-border mt-2 border-t pt-2'>
                        <div className='text-panel-section-header px-2 pb-1 text-[11px] font-semibold tracking-wide uppercase'>Graph</div>
                        <CommitGraph commits={graphCommits} />
                    </div>
                )}
            </div>

            <AlertDialog open={discardTargets !== null} onOpenChange={(open) => !open && setDiscardTargets(null)}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>변경사항을 취소할까요?</AlertDialogTitle>
                        <AlertDialogDescription>
                            {discardTargets?.length === 1 ? discardTargets[0] : `${discardTargets?.length ?? 0}개 파일`}의 변경사항을 되돌립니다. 이
                            작업은 취소할 수 없습니다.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>취소</AlertDialogCancel>
                        <AlertDialogAction variant='destructive' onClick={confirmDiscard}>
                            변경 취소
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>

            <AlertDialog open={confirmStageAllOpen} onOpenChange={setConfirmStageAllOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>변경 전체를 스테이지하고 커밋할까요?</AlertDialogTitle>
                        <AlertDialogDescription>스테이지된 변경사항이 없습니다. 모든 변경사항을 스테이지한 뒤 커밋합니다.</AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>취소</AlertDialogCancel>
                        <AlertDialogAction onClick={confirmStageAllAndCommit}>커밋</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    )
}
