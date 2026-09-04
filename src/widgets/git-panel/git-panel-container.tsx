import type { FC } from 'react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ProjectId } from '@shared/api/bindings'
import {
    gitBranchesQueryOptions,
    gitLogQueryOptions,
    gitStashesQueryOptions,
    gitRemotesQueryOptions,
    gitStatusQueryOptions,
    useApplyGitStash,
    useCancelCommitMessageGeneration,
    useCheckoutGitBranch,
    useCheckoutRemoteGitBranch,
    useCommitGit,
    useCreateGitBranch,
    useDiscardGitPaths,
    useDropGitStash,
    useGenerateAiCommitMessage,
    useInitGitRepository,
    usePushGitStash,
    usePullGit,
    usePushGit,
    useStageGitPaths,
    useUnstageGitPaths,
} from '@entities/git/git.query'
import { readCommitMessageDraft, writeCommitMessageDraft } from '@entities/git/commit-message-memory'
import { useOpenFileTab, useOpenTab } from '@entities/layout/layout.query'
import { systemRevealPath } from '@entities/system/system.ipc'
import type { GitDiffTarget } from '@features/git/git-change-group'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { fileNameOf } from '@shared/lib/relative-path'
import { Button } from '@shared/ui/button'
import { buildRecentCommitsSummaryForAi, sanitizeAiCommitMessageResponse } from '@widgets/git-panel/ai-commit-message'
import { resolveCommitGate } from '@widgets/git-panel/commit-gate'
import { GitPanel } from '@widgets/git-panel/git-panel'

type GitPanelContainerProps = {
    projectId: ProjectId
}

export const GitPanelContainer: FC<GitPanelContainerProps> = ({ projectId }) => {
    /**
     * Mirrors `commitMessageRequestId` state so `handleGenerateCommitMessage`'s async callbacks can
     * check "am I still the latest request?" without a stale closure — state read inside a
     * `.then`/`finally` body would still see the value from the render that started this call, not
     * whatever a later cancel-then-restart click has since set it to.
     */
    const latestCommitMessageRequestIdRef = useRef<string | null>(null)

    const [commitMessage, setCommitMessage] = useState(() => readCommitMessageDraft(projectId))
    const [commitMessageRequestId, setCommitMessageRequestId] = useState<string | null>(null)
    const [scopedProjectId, setScopedProjectId] = useState(projectId)

    /**
     * Nothing above the sidebar remounts when the active project changes, so the message state has
     * to be re-read for the newly active project during render — otherwise the previous project's
     * unsent message stays in the box and can be committed into the wrong repository (audit §4-B
     * C6). The outgoing project's message needs no saving here: `applyCommitMessage` writes it
     * through on every keystroke.
     */
    if (scopedProjectId !== projectId) {
        setScopedProjectId(projectId)
        setCommitMessage(readCommitMessageDraft(projectId))
    }

    const applyCommitMessage = (message: string) => {
        setCommitMessage(message)
        writeCommitMessageDraft(projectId, message)
    }

    const { t } = useTranslation()

    const { data: status, isError } = useQuery(gitStatusQueryOptions(projectId))
    const { data: log = [] } = useQuery(gitLogQueryOptions(projectId))
    const { data: remotes = [] } = useQuery(gitRemotesQueryOptions(projectId))

    const { mutate: stagePaths } = useStageGitPaths(projectId)
    const { mutate: unstagePaths } = useUnstageGitPaths(projectId)
    const { mutate: discardPaths } = useDiscardGitPaths(projectId)
    const { mutate: commit, isPending: isCommitting } = useCommitGit(projectId)
    const { mutate: push, isPending: isPushing } = usePushGit(projectId)
    const { mutate: pull, isPending: isPulling } = usePullGit(projectId)
    const { mutate: openTab } = useOpenTab(projectId)
    const openFileTab = useOpenFileTab()
    const { data: branches = [] } = useQuery(gitBranchesQueryOptions(projectId))
    const { mutate: checkoutBranch } = useCheckoutGitBranch(projectId)
    const { mutate: checkoutRemoteBranch } = useCheckoutRemoteGitBranch(projectId)
    const { mutate: createBranch } = useCreateGitBranch(projectId)
    const { data: stashes = [] } = useQuery(gitStashesQueryOptions(projectId))
    const { mutate: pushStash, isPending: isStashPushing } = usePushGitStash(projectId)
    const { mutate: applyStash, isPending: isStashApplying } = useApplyGitStash(projectId)
    const { mutate: dropStash, isPending: isStashDropping } = useDropGitStash(projectId)
    const { mutate: initRepository, isPending: isInitializing } = useInitGitRepository(projectId)
    const { mutateAsync: generateCommitMessage } = useGenerateAiCommitMessage(projectId)
    const { mutateAsync: cancelCommitMessageGeneration } = useCancelCommitMessageGeneration()

    const handleStashPush = () =>
        pushStash(
            { projectId, message: null },
            { onSuccess: () => toast.success(t('git.stashPushed')), onError: (error) => toast.error(describeIpcError(error)) },
        )

    const handleStashApply = (index: number) => applyStash({ projectId, index }, { onError: (error) => toast.error(describeIpcError(error)) })

    const handleStashDrop = (index: number) => dropStash({ projectId, index }, { onError: (error) => toast.error(describeIpcError(error)) })

    const handleCheckoutBranch = (name: string) =>
        checkoutBranch(
            { projectId, name },
            { onSuccess: () => toast.success(t('git.branchSwitched', { name })), onError: (error) => toast.error(describeIpcError(error)) },
        )

    const handleCheckoutRemoteBranch = (remoteRef: string) =>
        checkoutRemoteBranch(
            { projectId, remoteRef },
            {
                onSuccess: () => toast.success(t('git.branchSwitched', { name: remoteRef })),
                onError: (error) => toast.error(describeIpcError(error)),
            },
        )

    const handleCreateBranch = (name: string) =>
        createBranch(
            { projectId, name, checkout: true },
            { onSuccess: () => toast.success(t('git.branchSwitched', { name })), onError: (error) => toast.error(describeIpcError(error)) },
        )

    const notifyError = (error: Error) => toast.error(describeIpcError(error))

    const handleInitRepository = () =>
        initRepository(projectId, { onSuccess: () => toast.success(t('git.initSuccess')), onError: () => toast.error(t('git.initFailed')) })

    /**
     * `stageAll` is what makes the conflict gate matter here as well as in `GitPanel`: with an
     * unresolved merge and nothing else staged, committing would stage every conflicted file —
     * markers and all — as the merge resolution (audit §4-B A4).
     */
    const handleCommit = () => {
        const gate = resolveCommitGate(status?.rows ?? [])
        if (gate === 'blockedByConflicts') return
        commit(
            { projectId, message: commitMessage, options: { amend: false, stageAll: gate === 'confirmStageAll' } },
            { onSuccess: () => applyCommitMessage(''), onError: notifyError },
        )
    }

    /**
     * A cancel-then-restart click (or a plain cancel) may supersede this call's own request before
     * it resolves — every check against `latestCommitMessageRequestIdRef.current` below drops a
     * stale result instead of clobbering whatever a newer request (or the user, after cancelling)
     * has since put in the commit message input/state.
     */
    const handleGenerateCommitMessage = async () => {
        if (commitMessageRequestId) {
            latestCommitMessageRequestIdRef.current = null
            await cancelCommitMessageGeneration(commitMessageRequestId).catch(() => undefined)
            setCommitMessageRequestId(null)
            return
        }

        const requestId = crypto.randomUUID()
        latestCommitMessageRequestIdRef.current = requestId
        setCommitMessageRequestId(requestId)
        try {
            const { diff, response } = await generateCommitMessage({ requestId, recentCommitsSummary: buildRecentCommitsSummaryForAi(log) })
            if (latestCommitMessageRequestIdRef.current !== requestId) return
            if (!response.text) return

            const sanitized = sanitizeAiCommitMessageResponse(response.text)
            if (!sanitized) {
                toast.error(t('git.commitMessageEmptyResponse'))
                return
            }

            applyCommitMessage(sanitized)
            const notices = [
                diff.usedFallback ? t('git.commitMessageUsedUnstaged') : null,
                diff.truncated ? t('git.commitMessageDiffTruncated') : null,
                diff.skippedFiles.length > 0 ? t('git.commitMessageFilesSkipped', { count: diff.skippedFiles.length }) : null,
            ].filter((notice): notice is string => notice !== null)
            toast.success(t('git.commitMessageGenerated'), { description: notices.length > 0 ? notices.join(' · ') : undefined })
        } catch (error) {
            if (latestCommitMessageRequestIdRef.current === requestId) {
                toast.error(t('git.generateCommitMessageFailed'), { description: describeIpcError(error) })
            }
        } finally {
            if (latestCommitMessageRequestIdRef.current === requestId) setCommitMessageRequestId(null)
        }
    }

    const handleSync = () => pull(projectId, { onSuccess: () => push(projectId, { onError: notifyError }), onError: notifyError })

    const openDiffTab = (target: GitDiffTarget, group: 'staged' | 'unstaged') =>
        openTab(
            {
                projectId,
                kind: { kind: 'diff', path: target.path, staged: group === 'staged', beforePath: target.beforePath },
                title: `${fileNameOf(target.path)} (diff)`,
                target: null,
                preview: true,
            },
            { onError: notifyError },
        )

    if (isError) {
        return (
            <div className='bg-panel-background text-app-sidebar-icon-default flex h-full w-full flex-col items-center justify-center gap-3 p-4 text-center text-sm'>
                <span>{t('git.notARepository')}</span>
                <Button type='button' variant='outline' size='sm' disabled={isInitializing} onClick={handleInitRepository}>
                    {t('git.initRepository')}
                </Button>
            </div>
        )
    }

    return (
        <GitPanel
            projectId={projectId}
            branch={status?.branch ?? null}
            ahead={status?.ahead ?? 0}
            behind={status?.behind ?? 0}
            hasRemote={status?.hasRemote ?? false}
            remote={remotes[0] ?? null}
            rows={status?.rows ?? []}
            commitMessage={commitMessage}
            onCommitMessageChange={applyCommitMessage}
            onCommit={handleCommit}
            isCommitting={isCommitting}
            onGenerateCommitMessage={() => void handleGenerateCommitMessage()}
            isGeneratingCommitMessage={commitMessageRequestId !== null}
            onStage={(paths) => stagePaths({ projectId, paths }, { onError: notifyError })}
            onUnstage={(paths) => unstagePaths({ projectId, paths }, { onError: notifyError })}
            onDiscard={(paths) => discardPaths({ projectId, paths }, { onError: notifyError })}
            onOpenFile={(path) => openFileTab({ projectId, path, target: null, preview: true })}
            onOpenChanges={openDiffTab}
            onCopyPath={(path) => void navigator.clipboard.writeText(path)}
            onRevealInExplorer={(path) => void systemRevealPath(path).catch(notifyError)}
            onSync={handleSync}
            branches={branches}
            stashes={stashes}
            canStash={(status?.rows.length ?? 0) > 0}
            isStashing={isStashPushing || isStashApplying || isStashDropping}
            onStashPush={handleStashPush}
            onStashApply={handleStashApply}
            onStashDrop={handleStashDrop}
            onCheckoutBranch={handleCheckoutBranch}
            onCheckoutRemoteBranch={handleCheckoutRemoteBranch}
            onCreateBranch={handleCreateBranch}
            isSyncing={isPushing || isPulling}
            graphCommits={log}
        />
    )
}
