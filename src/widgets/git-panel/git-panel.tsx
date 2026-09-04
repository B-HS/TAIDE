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
import { GitSectionHeader } from '@features/git/git-section-header'
import { StashList } from '@features/git/stash-list'
import { ScrollContainer } from '@shared/scroll/scroll-container'
import type { GitSectionId } from '@entities/git/git-section-collapse-memory'
import { readGitSectionCollapseState, writeGitSectionCollapsed } from '@entities/git/git-section-collapse-memory'
import { GIT_SECTION_ROVING_SELECTOR, resolveNextChangeRowIndex } from '@widgets/git-panel/change-row-navigation'
import { CommitDetailPanel } from '@widgets/git-panel/commit-detail-panel'
import { resolveCommitGate } from '@widgets/git-panel/commit-gate'
import { buildGitSections } from '@widgets/git-panel/git-sections'
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
    const sectionsRef = useRef<HTMLDivElement>(null)

    const [discardTargets, setDiscardTargets] = useState<string[] | null>(null)
    const [confirmStageAllOpen, setConfirmStageAllOpen] = useState(false)
    const [selectedCommitId, setSelectedCommitId] = useState<string | null>(null)
    const [collapsedSections, setCollapsedSections] = useState(readGitSectionCollapseState)

    const { mergeRows, stagedRows, unstagedRows, sections, showNoChanges } = buildGitSections({
        rows,
        stashCount: stashes.length,
        graphCount: graphCommits.length,
        collapsed: collapsedSections,
    })
    const commitGate = resolveCommitGate(rows)
    const selectedCommit = selectedCommitId ? (graphCommits.find((commit) => commit.id === selectedCommitId) ?? null) : null

    const toggleSection = (id: GitSectionId) => {
        const collapsed = !collapsedSections[id]
        writeGitSectionCollapsed(id, collapsed)
        setCollapsedSections({ ...collapsedSections, [id]: collapsed })
    }

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

    const handleSectionsKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
        const items = [...(sectionsRef.current?.querySelectorAll<HTMLElement>(GIT_SECTION_ROVING_SELECTOR) ?? [])]
        const activeItem = event.target instanceof HTMLElement ? event.target.closest<HTMLElement>(GIT_SECTION_ROVING_SELECTOR) : null
        const nextIndex = resolveNextChangeRowIndex(event.key, activeItem ? items.indexOf(activeItem) : -1, items.length)
        if (nextIndex < 0) return
        event.preventDefault()
        items[nextIndex].focus()
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
                <IconButton
                    label={t('git.stashPush')}
                    icon={<Archive className='size-3.5' />}
                    disabled={!canStash}
                    onClick={onStashPush}
                    side='bottom'
                    containerClassName='ml-auto'
                    className='hover:bg-explorer-item-hover flex size-5 shrink-0 items-center justify-center rounded-sm disabled:opacity-50'
                />
                {hasRemote && (
                    <IconButton
                        label={t('git.sync')}
                        icon={isSyncing ? <Loader2 className='size-3.5 animate-spin' /> : <RefreshCw className='size-3.5' />}
                        disabled={isSyncing}
                        onClick={onSync}
                        side='bottom'
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
                <div ref={sectionsRef} role='group' aria-label={t('git.title')} onKeyDown={handleSectionsKeyDown}>
                    {sections.merge.visible && (
                        <GitChangeGroup
                            variant='merge'
                            rows={mergeRows}
                            expanded={!sections.merge.collapsed}
                            onToggle={() => toggleSection('merge')}
                            onOpenFile={onOpenFile}
                            onOpenChanges={onOpenChanges}
                            onCopyPath={onCopyPath}
                            onRevealInExplorer={onRevealInExplorer}
                        />
                    )}

                    {sections.staged.visible && (
                        <GitChangeGroup
                            variant='staged'
                            rows={stagedRows}
                            expanded={!sections.staged.collapsed}
                            onToggle={() => toggleSection('staged')}
                            onUnstage={onUnstage}
                            onOpenFile={onOpenFile}
                            onOpenChanges={onOpenChanges}
                            onCopyPath={onCopyPath}
                            onRevealInExplorer={onRevealInExplorer}
                        />
                    )}

                    {sections.changes.visible && (
                        <GitChangeGroup
                            variant='unstaged'
                            rows={unstagedRows}
                            expanded={!sections.changes.collapsed}
                            onToggle={() => toggleSection('changes')}
                            onStage={onStage}
                            onDiscardRequest={setDiscardTargets}
                            onOpenFile={onOpenFile}
                            onOpenChanges={onOpenChanges}
                            onCopyPath={onCopyPath}
                            onRevealInExplorer={onRevealInExplorer}
                        />
                    )}

                    {showNoChanges && <div className='text-app-sidebar-icon-default px-2 py-1.5 text-xs'>{t('git.noChanges')}</div>}

                    {sections.stashes.visible && (
                        <div>
                            <GitSectionHeader
                                title={t('git.stash')}
                                count={sections.stashes.count}
                                expanded={!sections.stashes.collapsed}
                                onToggle={() => toggleSection('stashes')}
                            />
                            {!sections.stashes.collapsed && (
                                <StashList stashes={stashes} disabled={isStashing} onApply={onStashApply} onDrop={onStashDrop} />
                            )}
                        </div>
                    )}

                    {sections.graph.visible && (
                        <div>
                            <GitSectionHeader
                                title={t('git.graph')}
                                count={sections.graph.count}
                                expanded={!sections.graph.collapsed}
                                onToggle={() => toggleSection('graph')}
                            />
                            {!sections.graph.collapsed && (
                                <CommitGraph
                                    projectId={projectId}
                                    commits={graphCommits}
                                    selectedCommitId={selectedCommitId}
                                    onSelectCommit={(id) => setSelectedCommitId((current) => (current === id ? null : id))}
                                    onOpenFile={onOpenFile}
                                />
                            )}
                            {!sections.graph.collapsed && selectedCommit && (
                                <CommitDetailPanel
                                    key={selectedCommit.id}
                                    projectId={projectId}
                                    commit={selectedCommit}
                                    onClose={() => setSelectedCommitId(null)}
                                />
                            )}
                        </div>
                    )}
                </div>
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
