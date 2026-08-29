import type { GitBranch as GitBranchInfo, GitRemote, GitStashEntry, ProjectId, StatusRow } from '@shared/api/bindings'
import type { FC, KeyboardEvent } from 'react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Archive, ArrowDown, ArrowUp, Loader2, RefreshCw } from 'lucide-react'
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
import { IconButton } from '@shared/ui/icon-button'
import { CommitBox } from '@features/git/commit-box'
import type { GitDiffTarget } from '@features/git/git-change-group'
import { GitChangeGroup } from '@features/git/git-change-group'
import { ResourceGroupHeader } from '@features/git/resource-group-header'
import { StashList } from '@features/git/stash-list'
import { ScrollContainer } from '@shared/scroll/scroll-container'
import { resolveNextChangeRowIndex } from '@widgets/git-panel/change-row-navigation'
import { CommitDetailPanel } from '@widgets/git-panel/commit-detail-panel'
import { isStagedRow, isUnstagedRow, resolveCommitGate } from '@widgets/git-panel/commit-gate'
import { CommitGraph, type GraphLogEntry } from '@widgets/git-panel/commit-graph'

export type { GitStatusChangeKind } from '@features/git/status-row-item'

export type GitStatusRow = StatusRow

export type GitRemoteInfo = GitRemote

export type GitPanelProps = {
    projectId: ProjectId
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
    onGenerateCommitMessage: () => void
    isGeneratingCommitMessage: boolean
    onStage: (paths: string[]) => void
    onUnstage: (paths: string[]) => void
    onDiscard: (paths: string[]) => void
    onOpenFile: (path: string) => void
    onOpenChanges: (target: GitDiffTarget, group: 'staged' | 'unstaged') => void
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
    onCheckoutRemoteBranch: (remoteRef: string) => void
    onCreateBranch: (name: string) => void
    graphCommits: GraphLogEntry[]
}

export const GitPanel: FC<GitPanelProps> = ({
    projectId,
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
    onGenerateCommitMessage,
    isGeneratingCommitMessage,
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
    onCheckoutRemoteBranch,
    onCreateBranch,
    graphCommits,
}) => {
    const changesListRef = useRef<HTMLDivElement>(null)

    const [discardTargets, setDiscardTargets] = useState<string[] | null>(null)
    const [confirmStageAllOpen, setConfirmStageAllOpen] = useState(false)
    const [selectedCommitId, setSelectedCommitId] = useState<string | null>(null)

    const mergeRows = rows.filter((row) => row.isConflicted)
    const stagedRows = rows.filter(isStagedRow)
    const unstagedRows = rows.filter(isUnstagedRow)
    const commitGate = resolveCommitGate(rows)
    const selectedCommit = selectedCommitId ? (graphCommits.find((commit) => commit.id === selectedCommitId) ?? null) : null

    const requestCommit = () => {
        if (commitGate === 'blockedByConflicts') return
        if (commitGate === 'confirmStageAll') {
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

    const handleChangesKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
        const rowElements = [...(changesListRef.current?.querySelectorAll<HTMLElement>('[data-git-change-row]') ?? [])]
        const activeRow = event.target instanceof HTMLElement ? event.target.closest<HTMLElement>('[data-git-change-row]') : null
        const nextIndex = resolveNextChangeRowIndex(event.key, activeRow ? rowElements.indexOf(activeRow) : -1, rowElements.length)
        if (nextIndex < 0) return
        event.preventDefault()
        rowElements[nextIndex].focus()
    }

    const { t } = useTranslation()

    return (
        <div className='flex h-full min-h-0 w-full flex-col'>
            <div className='border-app-border flex h-8 shrink-0 items-center gap-1.5 border-b px-2 text-xs'>
                <BranchSwitcher
                    branches={branches}
                    currentBranch={branch}
                    disabled={!branch}
                    onCheckout={onCheckoutBranch}
                    onCheckoutRemote={onCheckoutRemoteBranch}
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
                    <IconButton
                        label={t('git.sync')}
                        icon={isSyncing ? <Loader2 className='size-3.5 animate-spin' /> : <RefreshCw className='size-3.5' />}
                        disabled={isSyncing}
                        onClick={onSync}
                        side='bottom'
                        containerClassName='ml-auto'
                        className='hover:bg-explorer-item-hover flex size-5 shrink-0 items-center justify-center rounded-sm disabled:opacity-50'
                    />
                )}
                {remote && <span className='text-app-sidebar-icon-default ml-1 shrink-0 truncate text-[11px]'>{remote.name}</span>}
            </div>

            <CommitBox
                message={commitMessage}
                onMessageChange={onCommitMessageChange}
                onCommit={requestCommit}
                isCommitting={isCommitting}
                onGenerateCommitMessage={onGenerateCommitMessage}
                isGeneratingCommitMessage={isGeneratingCommitMessage}
                canGenerateCommitMessage={stagedRows.length > 0 || unstagedRows.length > 0 || mergeRows.length > 0}
                blockedReason={commitGate === 'blockedByConflicts' ? t('git.commitBlockedByConflicts') : null}
            />

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
                <div ref={changesListRef} role='group' aria-label={t('git.title')} onKeyDown={handleChangesKeyDown}>
                    {mergeRows.length > 0 && (
                        <GitChangeGroup
                            variant='merge'
                            rows={mergeRows}
                            onOpenFile={onOpenFile}
                            onOpenChanges={onOpenChanges}
                            onCopyPath={onCopyPath}
                            onRevealInExplorer={onRevealInExplorer}
                        />
                    )}

                    {stagedRows.length > 0 && (
                        <GitChangeGroup
                            variant='staged'
                            rows={stagedRows}
                            onUnstage={onUnstage}
                            onOpenFile={onOpenFile}
                            onOpenChanges={onOpenChanges}
                            onCopyPath={onCopyPath}
                            onRevealInExplorer={onRevealInExplorer}
                        />
                    )}

                    {unstagedRows.length > 0 && (
                        <GitChangeGroup
                            variant='unstaged'
                            rows={unstagedRows}
                            onStage={onStage}
                            onDiscardRequest={setDiscardTargets}
                            onOpenFile={onOpenFile}
                            onOpenChanges={onOpenChanges}
                            onCopyPath={onCopyPath}
                            onRevealInExplorer={onRevealInExplorer}
                        />
                    )}
                </div>

                {graphCommits.length > 0 && (
                    <div className='border-app-border mt-2 border-t pt-2'>
                        <div className='text-panel-section-header px-2 pb-1 text-[11px] font-semibold tracking-wide uppercase'>{t('git.graph')}</div>
                        <CommitGraph
                            projectId={projectId}
                            commits={graphCommits}
                            selectedCommitId={selectedCommitId}
                            onSelectCommit={(id) => setSelectedCommitId((current) => (current === id ? null : id))}
                            onOpenFile={onOpenFile}
                        />
                        {selectedCommit && (
                            <CommitDetailPanel
                                key={selectedCommit.id}
                                projectId={projectId}
                                commit={selectedCommit}
                                onClose={() => setSelectedCommitId(null)}
                            />
                        )}
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
