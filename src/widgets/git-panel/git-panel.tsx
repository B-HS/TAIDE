import type { GitBranch as GitBranchInfo, GitRemote, GitStashEntry, StatusRow } from '@shared/api/bindings'
import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Archive, ArrowDown, ArrowUp, File, Loader2, Minus, Plus, RefreshCw, Undo2 } from 'lucide-react'
import { BranchSwitcher } from '@features/git/branch-switcher'
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
import { StashList } from '@features/git/stash-list'
import type { GitStatusChangeKind, StatusRowAction } from '@features/git/status-row-item'
import { StatusRowItem } from '@features/git/status-row-item'
import { ScrollContainer } from '@shared/scroll/scroll-container'
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
    branches: GitBranchInfo[]
    stashes: GitStashEntry[]
    canStash: boolean
    isStashing: boolean
    onStashPush: () => void
    onStashApply: (index: number) => void
    onStashDrop: (index: number) => void
    onCheckoutBranch: (name: string) => void
    onCreateBranch: (name: string) => void
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
    branches,
    stashes,
    canStash,
    isStashing,
    onStashPush,
    onStashApply,
    onStashDrop,
    onCheckoutBranch,
    onCreateBranch,
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

    const { t } = useTranslation()

    return (
        <div className='flex h-full min-h-0 w-full flex-col'>
            <div className='border-app-border flex shrink-0 items-center gap-1.5 border-b px-2 py-1.5 text-xs'>
                <BranchSwitcher
                    branches={branches}
                    currentBranch={branch}
                    disabled={!branch}
                    onCheckout={onCheckoutBranch}
                    onCreate={onCreateBranch}
                />
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
                        aria-label={t('git.sync')}
                        disabled={isSyncing}
                        onClick={onSync}
                        className='hover:bg-explorer-item-hover ml-auto flex size-5 shrink-0 items-center justify-center rounded-sm disabled:opacity-50'>
                        {isSyncing ? <Loader2 className='size-3.5 animate-spin' /> : <RefreshCw className='size-3.5' />}
                    </button>
                )}
                {remote && <span className='text-app-sidebar-icon-default ml-1 shrink-0 truncate text-[11px]'>{remote.name}</span>}
            </div>

            <CommitBox message={commitMessage} onMessageChange={onCommitMessageChange} onCommit={requestCommit} isCommitting={isCommitting} />

            <ScrollContainer className='min-h-0 flex-1'>
                {(stashes.length > 0 || canStash) && (
                    <div>
                        <ResourceGroupHeader
                            title={t('git.stash')}
                            count={stashes.length}
                            actionLabel={canStash ? t('git.stashPush') : undefined}
                            actionIcon={<Archive className='size-3' />}
                            onAction={canStash ? onStashPush : undefined}
                        />
                        <StashList stashes={stashes} disabled={isStashing} onApply={onStashApply} onDrop={onStashDrop} />
                    </div>
                )}
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
                                        <ContextMenuItem onSelect={() => onCopyPath(row.path)}>{t('explorer.copyPath')}</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onRevealInExplorer(row.path)}>{t('explorer.reveal')}</ContextMenuItem>
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
                                        <ContextMenuItem onSelect={() => onCopyPath(row.path)}>{t('explorer.copyPath')}</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onRevealInExplorer(row.path)}>{t('explorer.reveal')}</ContextMenuItem>
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
                                    label: t('git.discard'),
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
                                            {t('git.discard')}
                                        </ContextMenuItem>
                                        <ContextMenuSeparator />
                                        <ContextMenuItem onSelect={() => onCopyPath(row.path)}>{t('explorer.copyPath')}</ContextMenuItem>
                                        <ContextMenuItem onSelect={() => onRevealInExplorer(row.path)}>{t('explorer.reveal')}</ContextMenuItem>
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
            </ScrollContainer>

            <AlertDialog open={discardTargets !== null} onOpenChange={(open) => !open && setDiscardTargets(null)}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('git.discardTitle')}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {t('git.discardDescription', {
                                target: discardTargets?.length === 1 ? discardTargets[0] : t('git.fileCount', { count: discardTargets?.length ?? 0 }),
                            })}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction variant='destructive' onClick={confirmDiscard}>
                            {t('git.discardConfirm')}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>

            <AlertDialog open={confirmStageAllOpen} onOpenChange={setConfirmStageAllOpen}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t('git.stageAllTitle')}</AlertDialogTitle>
                        <AlertDialogDescription>{t('git.stageAllDescription')}</AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t('common.cancel')}</AlertDialogCancel>
                        <AlertDialogAction onClick={confirmStageAllAndCommit}>{t('git.commit')}</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    )
}
