import type { GitBranch as GitBranchInfo, GitRemote, GitStashEntry, ProjectId, StatusRow } from '@shared/api/bindings'
import type { FC, KeyboardEvent, MouseEvent as ReactMouseEvent } from 'react'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { defaultRangeExtractor, useVirtualizer } from '@tanstack/react-virtual'
import { Group, Panel, usePanelRef } from 'react-resizable-panels'
import type { Layout, LayoutChangedMeta } from 'react-resizable-panels'
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
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuSeparator, ContextMenuTrigger } from '@shared/ui/context-menu'
import { IconButton } from '@shared/ui/icon-button'
import { CommitBox } from '@features/git/commit-box'
import type { GitChangeGroupConfig, GitDiffTarget } from '@features/git/git-change-group'
import { buildGitChangeGroupConfig } from '@features/git/git-change-group'
import { GIT_SECTION_HEADER_HEIGHT_PX, GIT_SECTION_ROW_INDENT_CLASS, GitSectionHeader } from '@features/git/git-section-header'
import { PaneSeparator } from '@features/split/pane-separator'
import { StatusRowItem } from '@features/git/status-row-item'
import { StashList } from '@features/git/stash-list'
import { DEFAULT_RESIZER_THICKNESS, MIN_PANEL_SIZE_PX, RESIZE_HIT_TARGET_SIZE } from '@shared/constants/layout'
import { ScrollContainer } from '@shared/scroll/scroll-container'
import { DEFAULT_GIT_GRAPH_PANEL_SIZE_PX } from '@entities/git/git.constant'
import type { GitSectionId } from '@entities/git/git-section'
import { toGitSectionCollapsedIds, toGitSectionCollapsedMap } from '@entities/git/git-section'
import { schedulePaneResizeCommit } from '@entities/layout/pane-resize-commit'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import type { GitChangeSectionId, GitPanelFocusTarget } from '@widgets/git-panel/change-row-navigation'
import {
    buildGitChangeListRows,
    GIT_ROVING_ITEM_SELECTOR,
    gitChangeListHeaderIndexes,
    gitRovingItemSelector,
    parseGitRovingIndex,
    resolveGitPanelFocusTarget,
    resolveStickyHeaderIndex,
} from '@widgets/git-panel/change-row-navigation'
import { CommitDetailPanel } from '@widgets/git-panel/commit-detail-panel'
import { resolveCommitGate } from '@widgets/git-panel/commit-gate'
import { buildGitSections } from '@widgets/git-panel/git-sections'
import { CommitGraph, type GraphLogEntry } from '@widgets/git-panel/commit-graph'

export type { GitStatusChangeKind } from '@features/git/status-row-item'

export type GitStatusRow = StatusRow

export type GitRemoteInfo = GitRemote

/** Both a section header and a change row are one `h-6` line, so the whole list measures uniformly. */
const GIT_CHANGE_ROW_HEIGHT_PX = 24

const GIT_CHANGE_LIST_OVERSCAN = 12

/** Debounce key for the graph pane's persisted height — one pane, so one entry in the shared timer map. */
const GIT_GRAPH_PANEL_RESIZE_KEY = 'git:graph-panel'

/**
 * The roving item that owns focus for a keyboard event, and where it sits in the panel's item
 * sequence. Rows and virtualized headers carry the index on their positioning wrapper, so the
 * focusable element itself is a descendant; static section wrappers carry it on the element that
 * also holds the stash or graph list.
 */
const resolveRovingFocusTarget = (element: HTMLElement | null) => {
    if (!element) return null
    if (element.hasAttribute('tabindex')) return element
    return element.querySelector<HTMLElement>('[tabindex]')
}

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

/**
 * The panel body is a vertical resizable group: the change list and the stash section share the
 * scrolled pane on top, the commit graph owns its own pane below the separator (d-58 §1.H). The
 * graph used to sit inside the one scroll area with a hard-coded 320px viewport of its own, which
 * swallowed a short panel, never grew in a tall one, and left the space a collapsed section gave
 * back unused.
 *
 * The three resource groups render as one virtualized list, and three invariants hold it together.
 *
 * - **The list is the first thing inside the scroll viewport.** The virtualizer measures against
 *   that viewport (the top pane keeps a single scrollbar over the changes and the stashes), so the
 *   list's own offset inside it has to stay zero — anything inserted above it would need
 *   `scrollMargin` instead.
 * - **The top-most section header renders in flow, everything else absolutely.** A virtualized row
 *   is absolutely positioned, and `position: sticky` inside a 24px absolute box cannot move, so the
 *   header that should be pinned is pulled into the rendered range by `rangeExtractor` and drawn
 *   without a translate (`resolveStickyHeaderIndex`). Sticky headers are load-bearing here: they
 *   are what tells the user which group the row under the cursor belongs to (`features/git.md` §2).
 * - **One context menu, not one per row.** The panel used to mount a Radix `ContextMenu` root plus
 *   seven pre-built items for every change row (research 3a M1); now the list has a single root and
 *   the right-clicked row is resolved from the event. Right-clicking anything that is not a row
 *   (a header, the empty area) calls `preventDefault`, which is what keeps Radix from opening a
 *   menu for the previous row — `composeEventHandlers` skips its own handler once the event is
 *   defaulted. The target is remembered by path, not by index, and the menu's open state follows
 *   whether that path is still listed: a status refresh while the menu is open must not leave the
 *   items pointing at whichever file moved into that slot.
 *
 * Keyboard roving moves over item indexes rather than the mounted DOM, since only the rows inside
 * the virtual window exist: the change list first, then the stash header. The graph header is the
 * stop after those but lives in the other pane, so it is addressed by ref rather than by index
 * (`resolveGitPanelFocusTarget`).
 */
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
    const viewportRef = useRef<HTMLDivElement>(null)
    const sectionsRef = useRef<HTMLDivElement>(null)
    const graphHeaderRef = useRef<HTMLDivElement>(null)
    const graphPanelRef = usePanelRef()
    const pendingFocusIndexRef = useRef<number | null>(null)

    const [discardTargets, setDiscardTargets] = useState<string[] | null>(null)
    const [confirmStageAllOpen, setConfirmStageAllOpen] = useState(false)
    const [selectedCommitId, setSelectedCommitId] = useState<string | null>(null)
    const [contextMenuTarget, setContextMenuTarget] = useState<{ section: GitChangeSectionId; path: string } | null>(null)

    const { t } = useTranslation()
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: updateSettings } = useUpdateSettings()

    /**
     * The collapse map is component state with `Settings.gitSectionsCollapsed` as its write-through
     * mirror, rather than being read straight back out of the settings cache (d-58 review G-2). That
     * cache only moves when `settings_update` returns, so a toggle fired before that round trip
     * landed still started from the map the previous one had read, and dropped it. The ref is what a
     * toggle computes from: the state variable is a per-render snapshot, so two toggles dispatched
     * inside one task would both start from whatever that render captured.
     */
    const [collapsedSections, setCollapsedSections] = useState(() => toGitSectionCollapsedMap(settings?.gitSectionsCollapsed))
    const collapsedSectionsRef = useRef(collapsedSections)
    const storedCollapseAdoptedRef = useRef(settings !== undefined)

    const graphPanelSizePx = settings?.gitGraphPanelSizePx ?? DEFAULT_GIT_GRAPH_PANEL_SIZE_PX

    const { mergeRows, stagedRows, unstagedRows, sections, showNoChanges } = buildGitSections({
        rows,
        stashCount: stashes.length,
        graphCount: graphCommits.length,
        collapsed: collapsedSections,
    })
    const commitGate = resolveCommitGate(rows)
    const selectedCommit = selectedCommitId ? (graphCommits.find((commit) => commit.id === selectedCommitId) ?? null) : null

    const groupConfigs: Record<GitChangeSectionId, GitChangeGroupConfig> = {
        merge: buildGitChangeGroupConfig({ variant: 'merge', rows: mergeRows, onOpenFile, onOpenChanges, onCopyPath, onRevealInExplorer }, t),
        staged: buildGitChangeGroupConfig(
            { variant: 'staged', rows: stagedRows, onUnstage, onOpenFile, onOpenChanges, onCopyPath, onRevealInExplorer },
            t,
        ),
        changes: buildGitChangeGroupConfig(
            {
                variant: 'unstaged',
                rows: unstagedRows,
                onStage,
                onDiscardRequest: setDiscardTargets,
                onOpenFile,
                onOpenChanges,
                onCopyPath,
                onRevealInExplorer,
            },
            t,
        ),
    }

    const listRows = buildGitChangeListRows(
        (['merge', 'staged', 'changes'] as const).map((id) => ({
            id,
            visible: sections[id].visible,
            collapsed: sections[id].collapsed,
            rowKeys: groupConfigs[id].rows.map((row) => row.path),
        })),
    )
    const headerIndexes = gitChangeListHeaderIndexes(listRows)

    /**
     * `scrollPaddingStart` is what keeps the sticky header from eating the row a keyboard move just
     * scrolled to. `scrollToIndex`'s `'auto'` alignment scrolls a row above the window to exactly
     * the viewport top, which is the one place the section header is pinned — the row would land
     * underneath an opaque `z-10` header and vanish, focus and all. One row's worth of padding puts
     * it immediately below instead, and the same value re-triggers the scroll for a row already
     * sitting half under the header.
     */
    const rowVirtualizer = useVirtualizer({
        count: listRows.length,
        getScrollElement: () => viewportRef.current,
        estimateSize: () => GIT_CHANGE_ROW_HEIGHT_PX,
        scrollPaddingStart: GIT_CHANGE_ROW_HEIGHT_PX,
        overscan: GIT_CHANGE_LIST_OVERSCAN,
        getItemKey: (index) => listRows[index].id,
        rangeExtractor: (range) => {
            const stickyIndex = resolveStickyHeaderIndex(headerIndexes, range.startIndex)
            if (stickyIndex < 0) return defaultRangeExtractor(range)
            return [...new Set([stickyIndex, ...defaultRangeExtractor(range)])].sort((left, right) => left - right)
        },
    })

    const virtualRows = rowVirtualizer.getVirtualItems()
    const stickyHeaderIndex = resolveStickyHeaderIndex(headerIndexes, rowVirtualizer.range?.startIndex ?? 0)

    const stashesRovingIndex = listRows.length
    const rovingItemCount = listRows.length + (sections.stashes.visible ? 1 : 0)

    const contextMenuRow = contextMenuTarget
        ? (groupConfigs[contextMenuTarget.section].rows.find((row) => row.path === contextMenuTarget.path) ?? null)
        : null
    const contextMenuEntries =
        contextMenuRow && contextMenuTarget ? groupConfigs[contextMenuTarget.section].buildContextMenuEntries(contextMenuRow) : []

    /**
     * Collapse state survives the panel because it is persisted in `Settings` (d-58 §1.H, decision
     * §2 #3): the panel only renders while the sidebar's git view is selected, so it was thrown away
     * on every view switch, and again on every restart. Each change applies to the local map first
     * and persists that whole map in the same step, so a burst of toggles keeps every one of them —
     * the last write carries them all. A write that fails leaves the panel where the user put it and
     * says so through `settings.saveFailed` (`settings.query.ts`).
     */
    const writeCollapsedSections = (collapsed: Record<GitSectionId, boolean>) => {
        storedCollapseAdoptedRef.current = true
        collapsedSectionsRef.current = collapsed
        setCollapsedSections(collapsed)
        updateSettings({ ...emptySettingsPatch(), gitSectionsCollapsed: toGitSectionCollapsedIds(collapsed) })
    }

    const toggleSection = (id: GitSectionId) => writeCollapsedSections({ ...collapsedSectionsRef.current, [id]: !collapsedSectionsRef.current[id] })

    /**
     * A drag that crosses `minSize` collapses the graph pane, which is the same state the header's
     * chevron writes — recording it as a collapse is what keeps the two in step and stops the
     * collapsed height from being stored as the user's preferred graph height. Anything else is a
     * resize, debounced like every other pane resize in the app (`pane-resize-commit.ts`).
     */
    const handleGraphLayoutChanged = (_layout: Layout, meta: LayoutChangedMeta) => {
        if (!meta.isUserInteraction) return
        const panel = graphPanelRef.current
        if (!panel) return
        const collapsed = panel.isCollapsed()
        const current = collapsedSectionsRef.current
        if (collapsed !== current.graph) {
            writeCollapsedSections({ ...current, graph: collapsed })
            return
        }
        if (collapsed) return
        const sizePx = Math.round(panel.getSize().inPixels)
        schedulePaneResizeCommit(GIT_GRAPH_PANEL_RESIZE_KEY, () => updateSettings({ ...emptySettingsPatch(), gitGraphPanelSizePx: sizePx }))
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

    const focusRovingItemNow = (index: number) => {
        const target = resolveRovingFocusTarget(sectionsRef.current?.querySelector<HTMLElement>(gitRovingItemSelector(index)) ?? null)
        target?.focus()
        return target !== null
    }

    const graphHeaderElement = () => resolveRovingFocusTarget(graphHeaderRef.current)

    const rovingItemOf = (target: EventTarget | null) =>
        target instanceof HTMLElement ? target.closest<HTMLElement>(GIT_ROVING_ITEM_SELECTOR) : null

    const moveRovingFocus = (target: GitPanelFocusTarget) => {
        if (target.kind === 'graph') {
            graphHeaderElement()?.focus()
            return
        }
        if (target.index < listRows.length) rowVirtualizer.scrollToIndex(target.index)
        pendingFocusIndexRef.current = focusRovingItemNow(target.index) ? null : target.index
    }

    /**
     * Arrow keys are left alone while focus sits on a control *inside* a roving item — a stash's
     * Apply button — since those belong to the section's own content, not to the panel's item
     * sequence. Focus outside every item (the scroll container itself) still enters the sequence at
     * the top or bottom.
     */
    const handleSectionsKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
        const item = rovingItemOf(event.target)
        if (item && resolveRovingFocusTarget(item) !== event.target) return
        const index = parseGitRovingIndex(item?.dataset.gitRovingIndex)
        const next = resolveGitPanelFocusTarget(event.key, index < 0 ? null : { kind: 'item', index }, rovingItemCount, sections.graph.visible)
        if (!next) return
        event.preventDefault()
        moveRovingFocus(next)
    }

    /**
     * The graph pane's half of the same sequence. Only its header is a stop, so a key pressed on a
     * commit row (or anywhere else in the pane's body) is left to that content — the rows used to be
     * covered by the "inside a roving item" rule above, which no longer reaches across the separator.
     */
    const handleGraphPaneKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
        if (event.target !== graphHeaderElement()) return
        const next = resolveGitPanelFocusTarget(event.key, { kind: 'graph' }, rovingItemCount, true)
        if (!next) return
        event.preventDefault()
        moveRovingFocus(next)
    }

    const handleChangeListContextMenu = (event: ReactMouseEvent<HTMLDivElement>) => {
        const listRow = listRows[parseGitRovingIndex(rovingItemOf(event.target)?.dataset.gitRovingIndex)]
        if (!listRow || listRow.kind !== 'row') {
            event.preventDefault()
            setContextMenuTarget(null)
            return
        }
        setContextMenuTarget({ section: listRow.section, path: groupConfigs[listRow.section].rows[listRow.rowIndex].path })
    }

    /**
     * A row that ArrowDown just scrolled to is only mounted once the virtualizer has seen the
     * scroll land, so the focus request is retried after every render until the element exists.
     * The request is a ref rather than state because it is a pending DOM effect, not something the
     * panel renders — and the render that satisfies it is the virtualizer's own.
     */
    useEffect(() => {
        const pendingIndex = pendingFocusIndexRef.current
        if (pendingIndex === null) return
        if (focusRovingItemNow(pendingIndex)) pendingFocusIndexRef.current = null
    })

    /**
     * Adopts the stored collapse map when settings land after this mount — a cold start renders the
     * panel before the settings query resolves, and its defaults would otherwise hold for the whole
     * session. It runs at most once: once the user has toggled anything the local map is the newer
     * answer, and the payload arriving here is normally the echo of the panel's own write.
     */
    useEffect(() => {
        if (storedCollapseAdoptedRef.current || !settings) return
        storedCollapseAdoptedRef.current = true
        const stored = toGitSectionCollapsedMap(settings.gitSectionsCollapsed)
        collapsedSectionsRef.current = stored
        setCollapsedSections(stored)
    }, [settings])

    /**
     * Pushes the stored collapse state into the graph pane, which has no controlled "collapsed"
     * prop — the same external-widget sync `app-shell.tsx` does for the sidebar. Expanding goes
     * through `resize` rather than `expand`: `expand` restores whatever size this session happens to
     * remember, which is `minSize` for a pane that mounted collapsed, while the stored height is the
     * answer the user actually gave. `sections.graph.visible` is a dependency because the pane is
     * unmounted while the repository has no commits, so the sync has to run again when it returns.
     */
    useEffect(() => {
        const panel = graphPanelRef.current
        if (!panel) return
        if (panel.isCollapsed() === collapsedSections.graph) return
        if (collapsedSections.graph) panel.collapse()
        else panel.resize(graphPanelSizePx)
    }, [graphPanelRef, collapsedSections.graph, graphPanelSizePx, sections.graph.visible])

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

            <Group
                orientation='vertical'
                onLayoutChanged={handleGraphLayoutChanged}
                resizeTargetMinimumSize={RESIZE_HIT_TARGET_SIZE}
                className='min-h-0 flex-1'>
                <Panel id='git-changes' className='min-h-0'>
                    <ScrollContainer className='h-full' viewportRef={viewportRef}>
                        <div ref={sectionsRef} role='group' aria-label={t('git.title')} onKeyDown={handleSectionsKeyDown}>
                            <ContextMenu open={contextMenuRow !== null} onOpenChange={(open) => !open && setContextMenuTarget(null)}>
                                <ContextMenuTrigger asChild>
                                    <div
                                        style={{ height: rowVirtualizer.getTotalSize(), position: 'relative' }}
                                        onContextMenu={handleChangeListContextMenu}>
                                        {virtualRows.map((virtualRow) => {
                                            const listRow = listRows[virtualRow.index]
                                            const isSticky = virtualRow.index === stickyHeaderIndex
                                            const style = isSticky
                                                ? { position: 'sticky' as const, top: 0, width: '100%', height: virtualRow.size }
                                                : {
                                                      position: 'absolute' as const,
                                                      top: 0,
                                                      left: 0,
                                                      width: '100%',
                                                      height: virtualRow.size,
                                                      transform: `translateY(${virtualRow.start}px)`,
                                                  }

                                            if (listRow.kind === 'header') {
                                                return (
                                                    <div key={virtualRow.key} data-git-roving-index={virtualRow.index} style={style}>
                                                        <GitSectionHeader
                                                            title={groupConfigs[listRow.section].title}
                                                            count={sections[listRow.section].count}
                                                            expanded={!sections[listRow.section].collapsed}
                                                            onToggle={() => toggleSection(listRow.section)}
                                                            actions={groupConfigs[listRow.section].headerActions}
                                                        />
                                                    </div>
                                                )
                                            }

                                            const config = groupConfigs[listRow.section]
                                            const row = config.rows[listRow.rowIndex]

                                            return (
                                                <div
                                                    key={virtualRow.key}
                                                    data-git-roving-index={virtualRow.index}
                                                    style={style}
                                                    className={GIT_SECTION_ROW_INDENT_CLASS}>
                                                    <StatusRowItem
                                                        path={row.path}
                                                        origPath={row.origPath}
                                                        kind={row.kind}
                                                        selected={false}
                                                        actions={config.buildActions(row)}
                                                        onClick={() => config.onRowClick(row)}
                                                    />
                                                </div>
                                            )
                                        })}
                                    </div>
                                </ContextMenuTrigger>
                                <ContextMenuContent>
                                    {contextMenuEntries.map((entry) =>
                                        entry.type === 'separator' ? (
                                            <ContextMenuSeparator key={entry.key} />
                                        ) : (
                                            <ContextMenuItem
                                                key={entry.key}
                                                variant={entry.destructive ? 'destructive' : undefined}
                                                onSelect={entry.onSelect}>
                                                {entry.label}
                                            </ContextMenuItem>
                                        ),
                                    )}
                                </ContextMenuContent>
                            </ContextMenu>

                            {showNoChanges && <div className='text-app-sidebar-icon-default px-2 py-1.5 text-xs'>{t('git.noChanges')}</div>}

                            {sections.stashes.visible && (
                                <div data-git-roving-index={stashesRovingIndex}>
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
                        </div>
                    </ScrollContainer>
                </Panel>
                {sections.graph.visible && (
                    <PaneSeparator orientation='vertical' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />
                )}
                {sections.graph.visible && (
                    <Panel
                        id='git-graph'
                        panelRef={graphPanelRef}
                        className='min-h-0'
                        defaultSize={graphPanelSizePx}
                        minSize={MIN_PANEL_SIZE_PX}
                        collapsible
                        collapsedSize={GIT_SECTION_HEADER_HEIGHT_PX}>
                        <div className='flex h-full min-h-0 flex-col' onKeyDown={handleGraphPaneKeyDown}>
                            <div ref={graphHeaderRef} className='shrink-0'>
                                <GitSectionHeader
                                    title={t('git.graph')}
                                    count={sections.graph.count}
                                    expanded={!sections.graph.collapsed}
                                    onToggle={() => toggleSection('graph')}
                                />
                            </div>
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
                                <div className='flex max-h-1/2 shrink-0 flex-col'>
                                    <ScrollContainer className='min-h-0 flex-1'>
                                        <CommitDetailPanel
                                            key={selectedCommit.id}
                                            projectId={projectId}
                                            commit={selectedCommit}
                                            onClose={() => setSelectedCommitId(null)}
                                        />
                                    </ScrollContainer>
                                </div>
                            )}
                        </div>
                    </Panel>
                )}
            </Group>

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
