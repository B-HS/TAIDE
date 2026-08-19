import type { LogEntry, ProjectId } from '@shared/api/bindings'
import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { useVirtualizer } from '@tanstack/react-virtual'
import { RotateCcw, Tag, Tags } from 'lucide-react'
import { toast } from 'sonner'
import { cn } from '@shared/lib/cn'
import { createActivationKeyDownHandler } from '@shared/lib/activation-key'
import { computeGraphLanes } from '@shared/lib/graph-lanes'
import { relativeTimeToken } from '@shared/lib/relative-time'
import { tagsTargetingCommit } from '@shared/lib/git-tags'
import { OverlayScrollbar } from '@shared/scroll/overlay-scrollbar'
import { subscribeOpenCreateTagDialog } from '@shared/lib/create-tag-dialog-bridge'
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuSub,
    ContextMenuSubContent,
    ContextMenuSubTrigger,
    ContextMenuTrigger,
} from '@shared/ui/context-menu'
import { COMMIT_SHORT_HASH_LENGTH } from '@entities/git/git.constant'
import { gitTagsQueryOptions, useCreateGitTag, useDeleteGitTag, useRevertGitCommit } from '@entities/git/git.query'
import { CreateTagDialog } from '@features/git/create-tag-dialog'

export type GraphLogEntry = LogEntry

type CommitGraphProps = {
    projectId: ProjectId | null
    commits: GraphLogEntry[]
    selectedCommitId?: string | null
    onSelectCommit?: (id: string) => void
    onOpenFile: (path: string) => void
}

const ROW_HEIGHT = 24
const LANE_WIDTH = 14
const NODE_RADIUS = 3
const STROKE_WIDTH = 1.5
const LANE_COLOR_COUNT = 12
const VIEWPORT_HEIGHT_PX = 320
const OVERSCAN = 12

const laneX = (lane: number) => lane * LANE_WIDTH + LANE_WIDTH / 2
const laneColor = (lane: number) => `var(--taide-graph-lane${(lane % LANE_COLOR_COUNT) + 1})`

export const CommitGraph: FC<CommitGraphProps> = ({ projectId, commits, selectedCommitId = null, onSelectCommit, onOpenFile }) => {
    const parentRef = useRef<HTMLDivElement>(null)

    const [tagDialogTarget, setTagDialogTarget] = useState<string | null>(null)

    const nodes = computeGraphLanes(commits)
    const maxLane = nodes.reduce((max, node) => Math.max(max, node.lane, ...node.edges.map((edge) => edge.toLane), ...node.passthroughLanes), 0)
    const graphWidth = (maxLane + 1) * LANE_WIDTH
    const center = ROW_HEIGHT / 2

    const rowVirtualizer = useVirtualizer({
        count: nodes.length,
        getScrollElement: () => parentRef.current,
        estimateSize: () => ROW_HEIGHT,
        overscan: OVERSCAN,
        getItemKey: (index) => nodes[index].id,
    })

    const { t } = useTranslation()
    const { data: tags = [] } = useQuery(gitTagsQueryOptions(projectId))
    const { mutate: revertCommit } = useRevertGitCommit(projectId)
    const { mutate: createTag, isPending: isCreatingTag } = useCreateGitTag(projectId)
    const { mutate: deleteTag } = useDeleteGitTag(projectId)

    /**
     * A conflicted outcome routes straight to the first conflicted file (opening it in the editor,
     * where the inline conflict decorator lives) instead of leaving the user to discover the new
     * "Merge Changes" group on their own — the toast alone only announces that something needs
     * attention, not where.
     */
    const handleRevert = (commitId: string) => {
        if (!projectId) return
        revertCommit(
            { projectId, rev: commitId },
            {
                onSuccess: (outcome) => {
                    if (!outcome.conflicted) {
                        toast.success(t('git.revertSuccess'))
                        return
                    }
                    toast.warning(t('git.revertConflict'))
                    if (outcome.conflictedAbsPaths[0]) onOpenFile(outcome.conflictedAbsPaths[0])
                },
                onError: (error) => toast.error(error.message),
            },
        )
    }

    const handleCreateTag = ({ name, message }: { name: string; message: string }) => {
        if (!projectId || !tagDialogTarget) return
        createTag(
            { projectId, name, target: tagDialogTarget, opts: { annotated: true, message: message || null } },
            {
                onSuccess: () => {
                    toast.success(t('git.tagCreated'))
                    setTagDialogTarget(null)
                },
                onError: (error) => toast.error(error.message),
            },
        )
    }

    const handleDeleteTag = (name: string) => {
        if (!projectId) return
        deleteTag({ projectId, name }, { onSuccess: () => toast.success(t('git.tagDeleted')), onError: (error) => toast.error(error.message) })
    }

    useEffect(() => subscribeOpenCreateTagDialog(({ target }) => setTagDialogTarget(target)), [])

    return (
        <div className='relative min-w-0'>
            <div ref={parentRef} className='scrollbar-hidden overflow-y-auto' style={{ maxHeight: VIEWPORT_HEIGHT_PX }}>
                <div style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
                    {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                        const node = nodes[virtualRow.index]
                        const commit = commits[virtualRow.index]
                        const relativeTime = relativeTimeToken(commit.timeUnix ?? 0)
                        const commitTags = tagsTargetingCommit(tags, commit.id)
                        return (
                            <ContextMenu key={virtualRow.key}>
                                <ContextMenuTrigger>
                                    <div
                                        role={onSelectCommit ? 'button' : undefined}
                                        tabIndex={onSelectCommit ? 0 : undefined}
                                        onClick={onSelectCommit ? () => onSelectCommit(commit.id) : undefined}
                                        onKeyDown={onSelectCommit ? createActivationKeyDownHandler(() => onSelectCommit(commit.id)) : undefined}
                                        className={cn(
                                            'hover:bg-explorer-item-hover flex items-center gap-2 px-2 text-xs',
                                            onSelectCommit && 'cursor-default select-none',
                                            commit.id === selectedCommitId && 'bg-explorer-item-selected',
                                        )}
                                        style={{
                                            position: 'absolute',
                                            top: 0,
                                            left: 0,
                                            width: '100%',
                                            height: virtualRow.size,
                                            transform: `translateY(${virtualRow.start}px)`,
                                        }}>
                                        <svg width={graphWidth} height={ROW_HEIGHT} className='shrink-0'>
                                            {node.passthroughLanes.map((lane) => (
                                                <line
                                                    key={`pass-${lane}`}
                                                    x1={laneX(lane)}
                                                    y1={0}
                                                    x2={laneX(lane)}
                                                    y2={ROW_HEIGHT}
                                                    stroke={laneColor(lane)}
                                                    strokeWidth={STROKE_WIDTH}
                                                />
                                            ))}
                                            {node.continuesFromAbove && (
                                                <line
                                                    x1={laneX(node.lane)}
                                                    y1={0}
                                                    x2={laneX(node.lane)}
                                                    y2={center}
                                                    stroke={laneColor(node.lane)}
                                                    strokeWidth={STROKE_WIDTH}
                                                />
                                            )}
                                            {node.edges.map((edge) => (
                                                <path
                                                    key={`edge-${edge.parentId}`}
                                                    d={
                                                        edge.toLane === node.lane
                                                            ? `M ${laneX(node.lane)} ${center} L ${laneX(node.lane)} ${ROW_HEIGHT}`
                                                            : `M ${laneX(node.lane)} ${center} C ${laneX(node.lane)} ${ROW_HEIGHT}, ${laneX(edge.toLane)} ${center}, ${laneX(edge.toLane)} ${ROW_HEIGHT}`
                                                    }
                                                    fill='none'
                                                    stroke={laneColor(node.lane)}
                                                    strokeWidth={STROKE_WIDTH}
                                                />
                                            ))}
                                            <circle cx={laneX(node.lane)} cy={center} r={NODE_RADIUS} fill={laneColor(node.lane)} />
                                        </svg>
                                        {commit.refs.length > 0 && (
                                            <span className='flex shrink-0 gap-1'>
                                                {commit.refs.map((ref) => (
                                                    <span
                                                        key={ref}
                                                        className='text-graph-ref-branch border-graph-ref-branch rounded-sm border px-1 text-[10px]'>
                                                        {ref}
                                                    </span>
                                                ))}
                                            </span>
                                        )}
                                        <span className='truncate'>{commit.summary}</span>
                                        <span className='text-app-sidebar-icon-default shrink-0'>{commit.author}</span>
                                        <span className='text-app-sidebar-icon-default shrink-0'>{t(relativeTime.key, relativeTime.params)}</span>
                                        <span className='text-app-sidebar-icon-default shrink-0 font-mono'>
                                            {commit.id.slice(0, COMMIT_SHORT_HASH_LENGTH)}
                                        </span>
                                    </div>
                                </ContextMenuTrigger>
                                <ContextMenuContent>
                                    <ContextMenuItem onSelect={() => handleRevert(commit.id)}>
                                        <RotateCcw className='size-4' />
                                        {t('git.revert')}
                                    </ContextMenuItem>
                                    <ContextMenuItem onSelect={() => setTagDialogTarget(commit.id)}>
                                        <Tag className='size-4' />
                                        {t('git.createTag')}
                                    </ContextMenuItem>
                                    {commitTags.length > 0 && (
                                        <ContextMenuSub>
                                            <ContextMenuSubTrigger>
                                                <Tags className='size-4' />
                                                {t('git.deleteTag')}
                                            </ContextMenuSubTrigger>
                                            <ContextMenuSubContent>
                                                {commitTags.map((tag) => (
                                                    <ContextMenuItem key={tag.name} variant='destructive' onSelect={() => handleDeleteTag(tag.name)}>
                                                        {tag.name}
                                                    </ContextMenuItem>
                                                ))}
                                            </ContextMenuSubContent>
                                        </ContextMenuSub>
                                    )}
                                </ContextMenuContent>
                            </ContextMenu>
                        )
                    })}
                </div>
            </div>
            <OverlayScrollbar viewportRef={parentRef} orientation='vertical' />
            <CreateTagDialog
                open={tagDialogTarget !== null}
                targetLabel={tagDialogTarget ?? ''}
                isPending={isCreatingTag}
                onOpenChange={(open) => !open && setTagDialogTarget(null)}
                onConfirm={handleCreateTag}
            />
        </div>
    )
}
