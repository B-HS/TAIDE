import type { FC } from 'react'
import { useState } from 'react'
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
    useCheckoutGitBranch,
    useCommitGit,
    useCreateGitBranch,
    useDiscardGitPaths,
    useDropGitStash,
    usePushGitStash,
    usePullGit,
    usePushGit,
    useStageGitPaths,
    useUnstageGitPaths,
} from '@entities/git/git.query'
import { useOpenTab } from '@entities/layout/layout.query'
import { systemRevealPath } from '@entities/system/system.ipc'
import { GitPanel } from '@widgets/git-panel/git-panel'

type GitPanelContainerProps = {
    projectId: ProjectId
}

const fileNameOf = (path: string) => path.slice(path.lastIndexOf('/') + 1)

export const GitPanelContainer: FC<GitPanelContainerProps> = ({ projectId }) => {
    const [commitMessage, setCommitMessage] = useState('')

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
    const { data: branches = [] } = useQuery(gitBranchesQueryOptions(projectId))
    const { mutate: checkoutBranch } = useCheckoutGitBranch(projectId)
    const { mutate: createBranch } = useCreateGitBranch(projectId)
    const { data: stashes = [] } = useQuery(gitStashesQueryOptions(projectId))
    const { mutate: pushStash, isPending: isStashPushing } = usePushGitStash(projectId)
    const { mutate: applyStash, isPending: isStashApplying } = useApplyGitStash(projectId)
    const { mutate: dropStash, isPending: isStashDropping } = useDropGitStash(projectId)

    const handleStashPush = () =>
        pushStash(
            { projectId, message: null },
            { onSuccess: () => toast.success(t('git.stashPushed')), onError: (error) => toast.error(error.message) },
        )

    const handleStashApply = (index: number) => applyStash({ projectId, index }, { onError: (error) => toast.error(error.message) })

    const handleStashDrop = (index: number) => dropStash({ projectId, index }, { onError: (error) => toast.error(error.message) })

    const handleCheckoutBranch = (name: string) =>
        checkoutBranch(
            { projectId, name },
            { onSuccess: () => toast.success(t('git.branchSwitched', { name })), onError: (error) => toast.error(error.message) },
        )

    const handleCreateBranch = (name: string) =>
        createBranch(
            { projectId, name, checkout: true },
            { onSuccess: () => toast.success(t('git.branchSwitched', { name })), onError: (error) => toast.error(error.message) },
        )

    const notifyError = (error: Error) => toast.error(error.message)

    const handleCommit = () => {
        const hasStaged = (status?.rows ?? []).some((row) => row.staged !== null)
        commit(
            { projectId, message: commitMessage, options: { amend: false, stageAll: !hasStaged } },
            { onSuccess: () => setCommitMessage(''), onError: notifyError },
        )
    }

    const handleSync = () => pull(projectId, { onSuccess: () => push(projectId, { onError: notifyError }), onError: notifyError })

    const openFileTab = (path: string) =>
        openTab({ projectId, kind: { kind: 'file', path }, title: fileNameOf(path), target: null, preview: true }, { onError: notifyError })

    const openDiffTab = (path: string, group: 'staged' | 'unstaged') =>
        openTab(
            { projectId, kind: { kind: 'diff', path, staged: group === 'staged' }, title: `${fileNameOf(path)} (diff)`, target: null, preview: true },
            { onError: notifyError },
        )

    if (isError) {
        return (
            <div className='bg-panel-background text-app-sidebar-icon-default flex h-full w-full items-center justify-center p-4 text-center text-sm'>
                {t('git.notARepository')}
            </div>
        )
    }

    return (
        <GitPanel
            branch={status?.branch ?? null}
            ahead={status?.ahead ?? 0}
            behind={status?.behind ?? 0}
            hasRemote={status?.hasRemote ?? false}
            remote={remotes[0] ?? null}
            rows={status?.rows ?? []}
            commitMessage={commitMessage}
            onCommitMessageChange={setCommitMessage}
            onCommit={handleCommit}
            isCommitting={isCommitting}
            onStage={(paths) => stagePaths({ projectId, paths }, { onError: notifyError })}
            onUnstage={(paths) => unstagePaths({ projectId, paths }, { onError: notifyError })}
            onDiscard={(paths) => discardPaths({ projectId, paths }, { onError: notifyError })}
            onOpenFile={openFileTab}
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
            onCreateBranch={handleCreateBranch}
            isSyncing={isPushing || isPulling}
            graphCommits={log}
        />
    )
}
