import type { QueryClient } from '@tanstack/react-query'
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { AppFileTarget, PaneId, ProjectId, ProjectLayout } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { describeIpcError, isNotFoundIpcError } from '@shared/lib/ipc-error-message'
import { isStaleLayoutRevision } from '@shared/lib/layout-revision'
import { collectAllPaneTabs, currentWindowFocusedPane } from '@shared/lib/pane-tree'
import { fileNameOf } from '@shared/lib/relative-path'
import { removePendingClaudeDiff } from '@entities/ide/claude-diff-registry'
import { releaseClosedFileTabPath } from '@entities/layout/tab-path-change'
import {
    activateTab,
    closeTab,
    convertUntitledTab,
    focusPane,
    getLayout,
    moveTab,
    moveTabToWindow,
    openTab,
    openTabInSplit,
    openUntitledTab,
    pinTab,
    reopenClosedTab,
    resizePane,
    setShellView,
    setTabDirty,
    setTabPreview,
    setTabViewState,
    setTerminalSession,
    splitPane,
} from '@entities/layout/layout.ipc'

export const layoutQueryOptions = (projectId: ProjectId | null) =>
    queryOptions({
        queryKey: QUERY_KEY.LAYOUT.DETAIL(projectId ?? ''),
        queryFn: () => getLayout(projectId ?? ''),
        enabled: !!projectId,
    })

/**
 * Writes `layout` into the `LAYOUT.DETAIL` cache unless a fresher revision is already sitting
 * there — every layout mutation's `onSuccess` funnels through this instead of a raw `setQueryData`
 * (contract `2026-08-25-d42-e2e-defects-contract.md` §3, item b). Two mutations fired close
 * together — e.g. a keystroke's `setTabDirty({dirty:true})` racing a subsequent `⌘S`'s
 * `setTabDirty({dirty:false})` — resolve as two independent IPC round trips with no guarantee the
 * later call's *response* also arrives later: `AppState::begin_mutation`'s single app-wide async
 * mutex serializes the Rust-side writes themselves, but which of two concurrently in-flight
 * `invoke()` calls a still-queued frontend task settles first is not bound to invocation order. A
 * raw `setQueryData(key, layout)` in that handler would then let the stale `dirty:true` response
 * land *after* the fresh `dirty:false` one and silently overwrite it — and since `layoutQueryOptions`
 * has no periodic refetch (`staleTime`/`refetchOnWindowFocus` both conservative) and no further
 * `layout:changed` event necessarily follows, that stale dot then sits in the tab bar indefinitely
 * (the pilot's real-keyboard repro against `index.ts`/`README.md`, 10s+ before the run was
 * abandoned). Comparing against `ProjectLayout.revision` — the same monotonic counter
 * `ipc-sync-provider.tsx`'s `layout:changed` handler already guards with `isStaleLayoutRevision` —
 * closes the gap at its source instead of masking it with a delay or a forced refetch.
 */
export const applyFreshLayout = (queryClient: QueryClient, projectId: ProjectId | null, layout: ProjectLayout) => {
    const key = QUERY_KEY.LAYOUT.DETAIL(projectId ?? '')
    const current = queryClient.getQueryData<ProjectLayout>(key)
    if (isStaleLayoutRevision(current?.revision, layout.revision ?? 0)) return
    queryClient.setQueryData(key, layout)
}

const useLayoutMutation = <TVariables>(projectId: ProjectId | null, mutationFn: (variables: TVariables) => Promise<ProjectLayout>) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn,
        onSuccess: (layout) => applyFreshLayout(queryClient, projectId, layout),
    })
}

export const useOpenTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, openTab)

/**
 * Shared by every "open settings.json / a prompt template as a tab" call site
 * (`settings-view.tsx`'s header button, `settings-ai-section.tsx`'s prompt rows) so the
 * open-in-the-focused-pane-of-this-window + error-toast wiring exists in one place instead of
 * being copy-pasted per call site.
 */
export const useOpenAppFileTab = (projectId: ProjectId) => {
    const { mutate: openTab } = useOpenTab(projectId)
    const { data: layout } = useQuery(layoutQueryOptions(projectId))

    return (appFileTarget: AppFileTarget, title: string) =>
        openTab(
            { projectId, kind: { kind: 'appFile', target: appFileTarget }, title, target: currentWindowFocusedPane(layout), preview: false },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
}

/**
 * `useOpenTab` binds `projectId` at hook-call time, so it can't correctly cache a tab opened for a
 * *different* project than the one the calling widget was rendered for — the drag-and-drop-a-file
 * flow needs exactly that (the drop target may resolve to a project other than the currently active
 * one). This variant instead reads the target project from the mutation's own variables, matching
 * what `openTab`'s IPC call already receives (contract F4#4 — no more hand-rolled raw
 * `openTab()` + `setQueryData` in the widget).
 */
export const useOpenTabInProject = () => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: openTab,
        onSuccess: (layout, variables) => applyFreshLayout(queryClient, variables.projectId, layout),
    })
}

type OpenFileTabRequest = { projectId: ProjectId; path: string; preview: boolean; target: PaneId | null; title?: string }

type OpenFileTabCallbacks = { onSuccess?: (layout: ProjectLayout) => void; onError?: (error: unknown) => void }

/**
 * The single entry point for opening a `file` tab — every explorer/palette/search/git/problems/
 * breadcrumb/welcome/agent call site funnels through here instead of hand-writing
 * `openTab({ kind: { kind: 'file', … } }, { onError: toast })`, so the two behaviours a file open
 * needs stay in one place (contract `2026-09-04-usability-batch3-contract.md` §A.2 item 4). The one
 * deliberate holdout is `app-shell.tsx`'s drag-and-drop loop, which needs `mutateAsync` to open a
 * multi-file drop one at a time — see `docs/features/command-palette.md` §3.1.
 *
 * The first is the error toast itself, which every call site used to copy verbatim. The second is
 * the quick-open index repair: `layout_open_tab` now rejects a `file` tab whose path is not a file
 * on disk, and the overwhelming source of such a path is the palette's `SEARCH.PROJECT_FILES`
 * listing, which is a plain snapshot of one `search_list_files` walk with no backend cache behind
 * it. A `NotFound` therefore means *this listing is stale*, not merely "this open failed", so the
 * key is invalidated on the spot — the next ⌘P re-walks instead of offering the same dead row
 * again. Any other failure (`Forbidden` outside a project root, an `Io` error) says nothing about
 * the index and leaves it alone.
 *
 * `onSuccess` receives the fresh `ProjectLayout` because a caller sometimes needs the pane the tab
 * landed in (the explorer's "Open to the Side" splits on it) — it is the same value
 * `useOpenTabInProject`'s own `onSuccess` already wrote into the cache. This is also the extension
 * point for later "opening a file" behaviour (MRU recording, explorer auto-reveal) rather than
 * another round of call-site copy-paste.
 */
export const useOpenFileTab = () => {
    const queryClient = useQueryClient()
    const { mutate: openTabInProject } = useOpenTabInProject()

    return ({ projectId, path, preview, target, title }: OpenFileTabRequest, callbacks?: OpenFileTabCallbacks) =>
        openTabInProject(
            { projectId, kind: { kind: 'file', path }, title: title ?? fileNameOf(path), target, preview },
            {
                onSuccess: (layout) => callbacks?.onSuccess?.(layout),
                onError: (error) => {
                    if (isNotFoundIpcError(error)) void queryClient.invalidateQueries({ queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(projectId) })
                    toast.error(describeIpcError(error))
                    callbacks?.onError?.(error)
                },
            },
        )
}

/**
 * The closed tab is looked up across *every* tree the project owns, not just the main one: a tab
 * moved into an auxiliary window lives under `auxiliaryWindows[].root`, and a main-tree-only lookup
 * would read its close as "not a file tab" and skip the whole per-path release below.
 */
export const useCloseTab = (projectId: ProjectId | null) => {
    const queryClient = useQueryClient()
    return useMutation({
        mutationFn: closeTab,
        onSuccess: (layout, tabId) => {
            const previous = queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(projectId ?? ''))
            const closedKind = previous ? (collectAllPaneTabs(previous).find((tab) => tab.id === tabId)?.kind ?? null) : null
            if (closedKind?.kind === 'claudeDiff') removePendingClaudeDiff(closedKind.requestId)
            if (closedKind?.kind === 'file') releaseClosedFileTabPath({ queryClient, projectId, path: closedKind.path, layout })
            applyFreshLayout(queryClient, projectId, layout)
        },
    })
}

export const useActivateTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, activateTab)

export const useMoveTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, moveTab)

export const useSplitPane = (projectId: ProjectId | null) => useLayoutMutation(projectId, splitPane)

export const useOpenTabInSplit = (projectId: ProjectId | null) => useLayoutMutation(projectId, openTabInSplit)

export const useResizePane = (projectId: ProjectId | null) => useLayoutMutation(projectId, resizePane)

export const useFocusPane = (projectId: ProjectId | null) => useLayoutMutation(projectId, focusPane)

export const usePinTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, pinTab)

export const useSetTabPreview = (projectId: ProjectId | null) => useLayoutMutation(projectId, setTabPreview)

export const useSetTabDirty = (projectId: ProjectId | null) => useLayoutMutation(projectId, setTabDirty)

export const useSetTerminalSession = (projectId: ProjectId | null) => useLayoutMutation(projectId, setTerminalSession)

export const useReopenClosedTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, reopenClosedTab)

export const useOpenUntitledTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, openUntitledTab)

export const useConvertUntitledTab = (projectId: ProjectId | null) => useLayoutMutation(projectId, convertUntitledTab)

export const useMoveTabToWindow = (projectId: ProjectId | null) => useLayoutMutation(projectId, moveTabToWindow)

export const useSetShellView = (projectId: ProjectId | null) => useLayoutMutation(projectId, setShellView)

export const useSetTabViewState = (projectId: ProjectId | null) => useLayoutMutation(projectId, setTabViewState)
