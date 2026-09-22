import type { QueryClient } from '@tanstack/react-query'
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { AppFileTarget, PaneId, ProjectId, ProjectLayout } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { i18next } from '@shared/i18n/i18n'
import { describeIpcError, isNotFoundIpcError } from '@shared/lib/ipc-error-message'
import { isStaleLayoutRevision } from '@shared/lib/layout-revision'
import { collectAllPaneTabs, currentWindowFileTabPane, currentWindowFocusedPane, findActiveTab } from '@shared/lib/pane-tree'
import { fileNameOf } from '@shared/lib/relative-path'
import type { RevealTarget } from '@entities/editor/reveal-registry'
import { revealInTab } from '@entities/editor/reveal-registry'
import { removePendingClaudeDiff } from '@entities/ide/claude-diff-registry'
import { reconcileRenamedLayoutPaths, releaseClosedFileTabPath } from '@entities/layout/tab-path-change'
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
        queryFn: async ({ client }) => {
            const layout = await getLayout(projectId ?? '')
            if (projectId) await reconcileRenamedLayoutPaths({ queryClient: client, projectId }, layout)
            return layout
        },
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

type OpenTabRequest = Parameters<typeof openTab>[0]

/**
 * Resolves a tab-open request's `null` `target` to *this window's* focused pane before it reaches
 * IPC. `layout_open_tab`'s own fallback for a missing target is `ProjectLayout.focused_pane` — the
 * **main** tree's pane (`domain::layout::commands`) — so a call site that means "wherever the user
 * is looking" was silently main-window-only. Every widget keeps passing `null`; only an auxiliary
 * window sees a difference, since in the main window the two resolve to the same pane
 * (`resolveWindowPaneTree`) — the same equivalence {@link useOpenTerminalTab} already relies on.
 *
 * It started to matter with d-62 §1.D, which gave auxiliary windows their own explorer, search and
 * SCM panels: without this, a file clicked in an auxiliary window's tree (or a diff opened from its
 * SCM panel) landed over in the main window. Doing it here rather than at each call site is what
 * keeps the fix from having to be re-applied per widget.
 *
 * The layout comes from the query cache rather than a `useQuery` because the request names its own
 * project, which is not necessarily the one the calling widget was rendered for
 * ({@link useOpenTabInProject}). An unpopulated cache resolves to `null` and lands on the server
 * fallback exactly as before.
 */
const withCurrentWindowTarget = (queryClient: QueryClient, request: OpenTabRequest): OpenTabRequest => ({
    ...request,
    target: request.target ?? currentWindowFocusedPane(queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(request.projectId))),
})

export const useOpenTab = (projectId: ProjectId | null) => {
    const queryClient = useQueryClient()
    return useLayoutMutation(projectId, (request: OpenTabRequest) => openTab(withCurrentWindowTarget(queryClient, request)))
}

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
 * Shared by the command palette's `new-terminal` command and the Welcome screen's terminal button
 * (d-58 contract §1.C) so the three things opening a terminal needs — the "a terminal belongs to a
 * project" precondition, the tab payload, the error toast — live in one place instead of being
 * re-typed per call site.
 *
 * `target` resolves through `currentWindowFocusedPane` rather than being left `null`: a `null`
 * target makes Rust fall back to `ProjectLayout.focused_pane`, which is the *main* tree's pane, so
 * a Welcome tab living in an auxiliary window would open its terminal over in the main window. In
 * the main window the two resolve to the same pane, which is why the palette — main-window only —
 * adopts this without a behaviour change.
 *
 * `i18next.t` is read directly instead of `useTranslation` for the same reason `git.query.ts` does:
 * `entities` holds no components, and these strings are tab/toast text built at call time rather
 * than rendered markup.
 */
export const useOpenTerminalTab = (projectId: ProjectId | null) => {
    const { mutate: openTab } = useOpenTab(projectId)
    const { data: layout } = useQuery(layoutQueryOptions(projectId))

    return () => {
        if (!projectId) {
            toast.info(i18next.t('app.openProjectFirst'))
            return
        }
        openTab(
            {
                projectId,
                kind: { kind: 'terminal', sessionId: '' },
                title: i18next.t('terminal.title'),
                target: currentWindowFocusedPane(layout),
                preview: false,
            },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
    }
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
        mutationFn: (request: OpenTabRequest) => openTab(withCurrentWindowTarget(queryClient, request)),
        onSuccess: (layout, variables) => applyFreshLayout(queryClient, variables.projectId, layout),
    })
}

type OpenFileTabRequest = { projectId: ProjectId; path: string; preview: boolean; target: PaneId | null; title?: string; reveal?: RevealTarget }

type OpenFileTabCallbacks = { onSuccess?: (layout: ProjectLayout) => void; onError?: (error: unknown) => void }

/**
 * The tab a just-returned `layout_open_tab` actually landed on, or `null` when the fresh layout
 * cannot confirm one. `open_tab` makes the tab it opened — a brand new one, the preview slot it
 * replaced, or the existing tab its dedupe reused — the ACTIVE tab of the target pane in every
 * branch, so the target pane's active tab *is* the opened tab. The pane is looked up across every
 * tree the project owns because an auxiliary window's panes live under `auxiliaryWindows[].root`.
 *
 * Confirming the kind and path before answering is what keeps a reveal off the wrong tab, and the
 * pane it confirms against is the one the *request* named — {@link useOpenFileTab} resolves the
 * request's `null` target through {@link withCurrentWindowTarget} before the call goes out and
 * hands that settled id here. Re-deriving the pane from the fresh layout's `focusedPane` instead
 * would name a different pane whenever the response's focus moved (another window's mutation
 * landing in the same round trip, a pane focused between request and response), and with the same
 * file already open in that other pane the reveal would move a cursor the user never asked to
 * navigate. A `null` id — an unpopulated layout cache, so Rust picked the pane through its own
 * `resolve_default_open_pane` — costs a reveal that does not happen, which is the safe half.
 */
const openedFileTabIdOf = (layout: ProjectLayout, paneId: PaneId | null, path: string) => {
    if (!paneId) return null

    const roots = [layout.root, ...(layout.auxiliaryWindows ?? []).map((window) => window.root)]
    const activeTab = roots.map((root) => findActiveTab(root, paneId)).find((tab) => tab !== null)
    return activeTab?.kind.kind === 'file' && activeTab.kind.path === path ? activeTab.id : null
}

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
 *
 * `reveal` is the third behaviour, and it lives here for exactly that reason: every "go to this
 * line of this file" caller used to queue the jump by PATH before opening, which let whichever
 * other pane already had the file steal both the cursor and the focus (audit #9). Resolving the
 * opened tab ({@link openedFileTabIdOf}) from the very layout the open returned is the only moment
 * a caller can name the tab it just created, so the reveal is handed to `revealInTab` from here
 * rather than being re-derived at each call site. Ordered before `callbacks.onSuccess` so a caller
 * that splits/activates in its own handler still finds the reveal already queued for its tab.
 *
 * The request is settled through {@link withCurrentWindowTarget} *here*, before it is handed to the
 * mutation, so the pane the reveal confirms against is the one this open asked for rather than the
 * response layout's `focusedPane` (d-67 #25). `useOpenTabInProject` applies the same resolution to
 * whatever it receives, and it is idempotent on an already-explicit target, so the call going out
 * over IPC is unchanged.
 */
export const useOpenFileTab = () => {
    const queryClient = useQueryClient()
    const { mutate: openTabInProject } = useOpenTabInProject()

    return ({ projectId, path, preview, target, title, reveal }: OpenFileTabRequest, callbacks?: OpenFileTabCallbacks) => {
        const layout = queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(projectId))
        const request = withCurrentWindowTarget(queryClient, {
            projectId,
            kind: { kind: 'file', path },
            title: title ?? fileNameOf(path),
            target: target ?? currentWindowFileTabPane(layout, path),
            preview,
        })

        return openTabInProject(request, {
            onSuccess: (layout) => {
                if (reveal) {
                    const openedTabId = openedFileTabIdOf(layout, request.target, path)
                    if (openedTabId) revealInTab(openedTabId, reveal)
                }
                callbacks?.onSuccess?.(layout)
            },
            onError: (error) => {
                if (isNotFoundIpcError(error)) void queryClient.invalidateQueries({ queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(projectId) })
                toast.error(describeIpcError(error))
                callbacks?.onError?.(error)
            },
        })
    }
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
