import type { LogEntry } from '@shared/api/bindings'
import type { FC } from 'react'
import { useRef } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { computeGraphLanes } from '@shared/lib/graph-lanes'

export type GraphLogEntry = LogEntry

type CommitGraphProps = {
    commits: GraphLogEntry[]
}

const ROW_HEIGHT = 24
const LANE_WIDTH = 14
const NODE_RADIUS = 3
const STROKE_WIDTH = 1.5
const LANE_COLOR_COUNT = 12
const SECONDS_PER_MINUTE = 60
const MINUTES_PER_HOUR = 60
const HOURS_PER_DAY = 24
const VIEWPORT_HEIGHT_PX = 320
const OVERSCAN = 12

const laneX = (lane: number) => lane * LANE_WIDTH + LANE_WIDTH / 2
const laneColor = (lane: number) => `var(--taide-graph-lane${(lane % LANE_COLOR_COUNT) + 1})`

const formatRelativeTime = (timeUnix: number) => {
    const diffSeconds = Math.max(0, Math.floor(Date.now() / 1000) - timeUnix)
    const diffMinutes = Math.floor(diffSeconds / SECONDS_PER_MINUTE)
    const diffHours = Math.floor(diffMinutes / MINUTES_PER_HOUR)
    const diffDays = Math.floor(diffHours / HOURS_PER_DAY)
    if (diffDays > 0) return `${diffDays}일 전`
    if (diffHours > 0) return `${diffHours}시간 전`
    if (diffMinutes > 0) return `${diffMinutes}분 전`
    return '방금'
}

export const CommitGraph: FC<CommitGraphProps> = ({ commits }) => {
    const parentRef = useRef<HTMLDivElement>(null)
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

    return (
        <div ref={parentRef} className='min-w-0 overflow-y-auto' style={{ maxHeight: VIEWPORT_HEIGHT_PX }}>
            <div style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}>
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                    const node = nodes[virtualRow.index]
                    const commit = commits[virtualRow.index]
                    return (
                        <div
                            key={virtualRow.key}
                            className='hover:bg-explorer-item-hover flex items-center gap-2 px-2 text-xs'
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
                                        <span key={ref} className='text-graph-ref-branch border-graph-ref-branch rounded-sm border px-1 text-[10px]'>
                                            {ref}
                                        </span>
                                    ))}
                                </span>
                            )}
                            <span className='truncate'>{commit.summary}</span>
                            <span className='text-app-sidebar-icon-default shrink-0'>{commit.author}</span>
                            <span className='text-app-sidebar-icon-default shrink-0'>{formatRelativeTime(commit.timeUnix ?? 0)}</span>
                            <span className='text-app-sidebar-icon-default shrink-0 font-mono'>{commit.id.slice(0, 7)}</span>
                        </div>
                    )
                })}
            </div>
        </div>
    )
}
