import type { ComponentProps, FC, ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { File, Minus, Plus, Undo2 } from 'lucide-react'
import type { StatusRow } from '@shared/api/bindings'
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuSeparator, ContextMenuTrigger } from '@shared/ui/context-menu'
import { ResourceGroupHeader } from '@features/git/resource-group-header'
import type { GitStatusChangeKind, StatusRowAction } from '@features/git/status-row-item'
import { StatusRowItem } from '@features/git/status-row-item'

/**
 * What a "Open Changes" click asks a diff tab to show. `path` is always the *absolute* path, the
 * one representation every `TabKind::Diff` producer now agrees on: a repo-relative one made the
 * layout treat the same file as a different tab than the editor tab bar's "Open Changes" did, kept
 * `fs:changed` from ever invalidating the tab's `GIT.DIFF` cache entry (the watcher reports
 * absolute paths), and hid the file's conflict state from `DiffPane` (audit §4-B B10).
 *
 * `beforePath` is the original (left-hand) side for a rename, so the diff reads the pre-rename blob
 * instead of showing the file as wholly added or deleted (audit §4-B B11).
 */
export type GitDiffTarget = { path: string; beforePath: string | null }

type GitChangeGroupBaseProps = {
    onOpenFile: (path: string) => void
    onOpenChanges: (target: GitDiffTarget, group: 'staged' | 'unstaged') => void
    onCopyPath: (path: string) => void
    onRevealInExplorer: (path: string) => void
}

export type GitChangeGroupProps =
    | ({ variant: 'merge'; rows: StatusRow[] } & GitChangeGroupBaseProps)
    | ({ variant: 'staged'; rows: (StatusRow & { staged: GitStatusChangeKind })[]; onUnstage: (paths: string[]) => void } & GitChangeGroupBaseProps)
    | ({
          variant: 'unstaged'
          rows: (StatusRow & { unstaged: GitStatusChangeKind })[]
          onStage: (paths: string[]) => void
          onDiscardRequest: (paths: string[]) => void
      } & GitChangeGroupBaseProps)

type NormalizedGitChangeRow = Pick<ComponentProps<typeof StatusRowItem>, 'path' | 'origPath' | 'kind'> &
    Pick<StatusRow, 'absPath'> & {
        origAbsPath: string | null
    }

const diffTargetOf = (row: NormalizedGitChangeRow): GitDiffTarget => ({ path: row.absPath, beforePath: row.origAbsPath })

type GitChangeGroupContextMenuEntry =
    { key: string; type: 'separator' } | { key: string; type: 'item'; label: string; destructive?: boolean; onSelect: () => void }

type GitChangeGroupConfig = {
    title: string
    actionLabel?: string
    actionIcon?: ReactNode
    onAction?: () => void
    rows: NormalizedGitChangeRow[]
    buildActions: (row: NormalizedGitChangeRow) => StatusRowAction[]
    buildContextMenuEntries: (row: NormalizedGitChangeRow) => GitChangeGroupContextMenuEntry[]
    onRowClick: (row: NormalizedGitChangeRow) => void
}

type GitChangeGroupTranslate = ReturnType<typeof useTranslation>['t']

const buildMergeGroupConfig = (props: Extract<GitChangeGroupProps, { variant: 'merge' }>, t: GitChangeGroupTranslate): GitChangeGroupConfig => {
    const { rows, onOpenFile, onOpenChanges, onCopyPath, onRevealInExplorer } = props
    return {
        title: t('git.mergeChanges'),
        rows: rows.map((row) => ({
            path: row.path,
            origPath: row.origPath ?? null,
            absPath: row.absPath,
            origAbsPath: row.origAbsPath ?? null,
            kind: 'conflicted',
        })),
        buildActions: (row) => [
            { id: 'open-file', label: t('git.openFile'), icon: <File className='size-3' />, onClick: () => onOpenFile(row.absPath) },
        ],
        buildContextMenuEntries: (row) => [
            { key: 'open-file', type: 'item', label: t('git.openFile'), onSelect: () => onOpenFile(row.absPath) },
            { key: 'open-changes', type: 'item', label: t('git.openChanges'), onSelect: () => onOpenChanges(diffTargetOf(row), 'unstaged') },
            { key: 'sep-1', type: 'separator' },
            { key: 'copy-path', type: 'item', label: t('explorer.copyPath'), onSelect: () => onCopyPath(row.absPath) },
            { key: 'reveal', type: 'item', label: t('explorer.reveal'), onSelect: () => onRevealInExplorer(row.absPath) },
        ],
        onRowClick: (row) => onOpenChanges(diffTargetOf(row), 'unstaged'),
    }
}

const buildStagedGroupConfig = (props: Extract<GitChangeGroupProps, { variant: 'staged' }>, t: GitChangeGroupTranslate): GitChangeGroupConfig => {
    const { rows, onUnstage, onOpenFile, onOpenChanges, onCopyPath, onRevealInExplorer } = props
    return {
        title: t('git.stagedChanges'),
        actionLabel: t('git.unstageAll'),
        actionIcon: <Minus className='size-3' />,
        onAction: () => onUnstage(rows.map((row) => row.path)),
        rows: rows.map((row) => ({
            path: row.path,
            origPath: row.origPath ?? null,
            absPath: row.absPath,
            origAbsPath: row.origAbsPath ?? null,
            kind: row.staged,
        })),
        buildActions: (row) => [
            { id: 'unstage', label: t('git.unstageChanges'), icon: <Minus className='size-3' />, onClick: () => onUnstage([row.path]) },
            { id: 'open-file', label: t('git.openFile'), icon: <File className='size-3' />, onClick: () => onOpenFile(row.absPath) },
        ],
        buildContextMenuEntries: (row) => [
            { key: 'open-file', type: 'item', label: t('git.openFile'), onSelect: () => onOpenFile(row.absPath) },
            { key: 'open-changes', type: 'item', label: t('git.openChanges'), onSelect: () => onOpenChanges(diffTargetOf(row), 'staged') },
            { key: 'unstage', type: 'item', label: t('git.unstageChanges'), onSelect: () => onUnstage([row.path]) },
            { key: 'sep-1', type: 'separator' },
            { key: 'copy-path', type: 'item', label: t('explorer.copyPath'), onSelect: () => onCopyPath(row.absPath) },
            { key: 'reveal', type: 'item', label: t('explorer.reveal'), onSelect: () => onRevealInExplorer(row.absPath) },
        ],
        onRowClick: (row) => onOpenChanges(diffTargetOf(row), 'staged'),
    }
}

const buildUnstagedGroupConfig = (props: Extract<GitChangeGroupProps, { variant: 'unstaged' }>, t: GitChangeGroupTranslate): GitChangeGroupConfig => {
    const { rows, onStage, onDiscardRequest, onOpenFile, onOpenChanges, onCopyPath, onRevealInExplorer } = props
    return {
        title: t('git.changes'),
        actionLabel: t('git.stageAll'),
        actionIcon: <Plus className='size-3' />,
        onAction: () => onStage(rows.map((row) => row.path)),
        rows: rows.map((row) => ({
            path: row.path,
            origPath: row.origPath ?? null,
            absPath: row.absPath,
            origAbsPath: row.origAbsPath ?? null,
            kind: row.unstaged,
        })),
        buildActions: (row) => [
            { id: 'stage', label: t('git.stageChanges'), icon: <Plus className='size-3' />, onClick: () => onStage([row.path]) },
            { id: 'discard', label: t('git.discard'), icon: <Undo2 className='size-3' />, onClick: () => onDiscardRequest([row.path]) },
            { id: 'open-file', label: t('git.openFile'), icon: <File className='size-3' />, onClick: () => onOpenFile(row.absPath) },
        ],
        buildContextMenuEntries: (row) => [
            { key: 'open-file', type: 'item', label: t('git.openFile'), onSelect: () => onOpenFile(row.absPath) },
            { key: 'open-changes', type: 'item', label: t('git.openChanges'), onSelect: () => onOpenChanges(diffTargetOf(row), 'unstaged') },
            { key: 'stage', type: 'item', label: t('git.stageChanges'), onSelect: () => onStage([row.path]) },
            { key: 'discard', type: 'item', label: t('git.discard'), destructive: true, onSelect: () => onDiscardRequest([row.path]) },
            { key: 'sep-1', type: 'separator' },
            { key: 'copy-path', type: 'item', label: t('explorer.copyPath'), onSelect: () => onCopyPath(row.absPath) },
            { key: 'reveal', type: 'item', label: t('explorer.reveal'), onSelect: () => onRevealInExplorer(row.absPath) },
        ],
        onRowClick: (row) => onOpenChanges(diffTargetOf(row), 'unstaged'),
    }
}

const buildGitChangeGroupConfig = (props: GitChangeGroupProps, t: GitChangeGroupTranslate): GitChangeGroupConfig => {
    if (props.variant === 'merge') return buildMergeGroupConfig(props, t)
    if (props.variant === 'staged') return buildStagedGroupConfig(props, t)
    return buildUnstagedGroupConfig(props, t)
}

export const GitChangeGroup: FC<GitChangeGroupProps> = (props) => {
    const { t } = useTranslation()
    const config = buildGitChangeGroupConfig(props, t)

    return (
        <div>
            <ResourceGroupHeader
                title={config.title}
                count={config.rows.length}
                actionLabel={config.actionLabel}
                actionIcon={config.actionIcon}
                onAction={config.onAction}
            />
            {config.rows.map((row) => (
                <ContextMenu key={row.path}>
                    <ContextMenuTrigger>
                        <StatusRowItem
                            path={row.path}
                            origPath={row.origPath}
                            kind={row.kind}
                            selected={false}
                            actions={config.buildActions(row)}
                            onClick={() => config.onRowClick(row)}
                        />
                    </ContextMenuTrigger>
                    <ContextMenuContent>
                        {config.buildContextMenuEntries(row).map((entry) =>
                            entry.type === 'separator' ? (
                                <ContextMenuSeparator key={entry.key} />
                            ) : (
                                <ContextMenuItem key={entry.key} variant={entry.destructive ? 'destructive' : undefined} onSelect={entry.onSelect}>
                                    {entry.label}
                                </ContextMenuItem>
                            ),
                        )}
                    </ContextMenuContent>
                </ContextMenu>
            ))}
        </div>
    )
}
