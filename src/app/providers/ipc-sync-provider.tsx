import type { FC, PropsWithChildren } from 'react'
import { useRef } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import type { FsChange, Project, ProjectId, ProjectLayout, TerminalSession } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { GIT_SCOPE_DIFF, GIT_SCOPE_GUTTER, PROJECT_SCOPED_KEYS, PROJECT_SCOPED_PATH_KEY_PREFIXES, QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { isStaleLayoutRevision } from '@shared/lib/layout-revision'
import { collectAllPaneTabs } from '@shared/lib/pane-tree'
import { isGitQueryScopeMutable } from '@entities/git/git.query'
import { refreshTreeDir } from '@entities/tree/tree.ipc'
import { pruneOpenWithOverrides } from '@entities/editor/open-with-registry'
import { markTerminalSessionExited } from '@entities/terminal/terminal-session-cache'
import { flushLspSessionsForProject } from '@entities/lsp/lsp-session-flush-registry'
import { useLspSessionsQueryInvalidationSync } from '@entities/lsp/lsp.query'

const PATH_SEPARATOR = '/'

const parentDirOf = (path: string) => {
    const index = path.lastIndexOf(PATH_SEPARATOR)
    return index <= 0 ? PATH_SEPARATOR : path.slice(0, index)
}

const FILE_SCOPE_CONTENT = QUERY_KEY.FILE.CONTENT('')[1]
const FILE_SCOPE_RAW = QUERY_KEY.FILE.RAW('')[1]

/**
 * Whether a cached `FILE.*` query is one of the two bare-path-keyed leaves — `FILE.CONTENT`
 * (`file.query.ts`'s `fileQueryOptions`, open editor tabs) and `FILE.RAW` (`fileRawQueryOptions`,
 * `preview-pane.tsx`'s binary/image/PDF preview) — keyed by a path in `changedPaths`. Both are
 * `staleTime: Infinity` and so both stay silently stale forever unless invalidated explicitly;
 * `FILE.RAW` has no `onSuccess` invalidation anywhere in `entities/file/file.query.ts` (contract
 * §1-b), so this watcher echo is its only refresh path.
 *
 * This is the `predicate` half of one `invalidateQueries({ queryKey: QUERY_KEY.FILE.ALL, predicate
 * })` call, replacing the previous `paths.length × 2` separate `invalidateQueries` calls (audit
 * §1-13). Each of those walked the *entire* query cache on its own, so a large watcher batch — a
 * branch switch, an `npm install`, a project-wide replace — cost thousands of full cache scans plus
 * a notify pass each; the predicate form pays exactly one. The scopes are read off the key factories
 * rather than re-typed as string literals so a rename of either leaf is a type error here.
 *
 * `FILE.MIRRORS`/`UNTITLED_MIRRORS` sit under the same `FILE.ALL` prefix but are keyed by
 * `ProjectId`, not by a path, so the scope check excludes them — hot-exit mirrors must never be
 * dropped by a watcher echo.
 */
export const isFilePathQueryForChangedPaths = (queryKey: readonly unknown[], changedPaths: ReadonlySet<string>) => {
    const [, scope, path] = queryKey
    if (scope !== FILE_SCOPE_CONTENT && scope !== FILE_SCOPE_RAW) return false
    return typeof path === 'string' && changedPaths.has(path)
}

/**
 * Whether a cached `GIT.*` query is one of the two path-scoped, worktree-derived leaves
 * (`GUTTER`/`DIFF`) *and* keyed by a path in `changedPaths`. This is the `predicate` half of the
 * `fs:changed` handler's `invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId), predicate
 * })` call below — `queryKey` and `predicate` combine with AND (TanStack Query's `matchQuery`, same
 * combination `isGitQueryScopeMutable` in `git.query.ts` uses), so the `GIT.PROJECT(projectId)` half
 * already keeps a *different* project's identically-shaped `['git', otherProjectId, 'gutter', path]`
 * key out of the match. This helper itself checks scope and path only — it deliberately does not
 * re-check `projectId`, and returns `true` for a key under any project as long as scope/path match
 * (see the "타 프로젝트 키" case in `ipc-sync-provider.test.ts`).
 *
 * `DIFF` matches on scope alone, without checking `mode` (`workdirVsIndex`/`indexVsHead`) —
 * sweeping `indexVsHead` too is a deliberate, small over-invalidation: it never actually changes from
 * a worktree edit (it compares two `.git`-internal states), so this is intentionally broader than the
 * worktree axis it stands in for, accepted to avoid enumerating the `DiffMode` union here by hand.
 * The real cost is one extra `git_diff_file` refetch, and only when a staged-diff tab happens to be
 * open for that exact path.
 */
export const isGitWorktreeQueryForChangedPaths = (queryKey: readonly unknown[], changedPaths: ReadonlySet<string>) => {
    const [, , scope, path] = queryKey
    if (scope !== GIT_SCOPE_GUTTER && scope !== GIT_SCOPE_DIFF) return false
    return typeof path === 'string' && changedPaths.has(path)
}

/**
 * Whether a `layout:changed` event is this window's own echo of a layout mutation whose response has
 * already been written into the cache (`applyFreshLayout`), and so needs no refetch at all.
 *
 * Every layout-mutating command emits this event, and every one of them also returns the new
 * `ProjectLayout` to its caller — so in the window that issued the mutation the event is pure echo.
 * `lastLayoutRevisionByProjectRef` alone cannot see that: it only remembers revisions delivered *as
 * events*, so a mutation's own echo always looked new and cost a full `get_layout` round trip on top
 * of the response already in hand (audit §1-5). Pane resize is the visible case — a drag ends with
 * one `layout_resize_pane` and paid two IPC calls for it — but the same duplicate refetch fired on
 * every tab open/close/activate/move/pin and every dirty-flag toggle.
 *
 * Comparing against the *cached* revision keeps every genuinely external change refetching: another
 * window's mutation (or this window's, if its response has not landed yet) leaves the cache behind
 * the event's revision, which fails this check and invalidates as before. `undefined` — nothing
 * cached yet — also invalidates, since there is no evidence the event has been accounted for.
 */
export const isLayoutEchoAlreadyInCache = (cachedRevision: number | undefined, eventRevision: number) =>
    cachedRevision !== undefined && cachedRevision >= eventRevision

/**
 * Matches a cached query's key against `PROJECT_SCOPED_PATH_KEY_PREFIXES` — `[domain, scope,
 * path, ...]` where `path` falls under `projectRoot` (the path itself, or a real descendant, never
 * an unrelated sibling directory that merely shares the prefix as a string — `/root-other` must not
 * match `/root`). Used as a `removeQueries({ predicate })` filter for `QUERY_KEY.FILE.CONTENT`/
 * `FILE.RAW`, the two leaves keyed by a bare file path instead of a `ProjectId` and so unreachable
 * by `PROJECT_SCOPED_KEYS`'s `(projectId) => key[]` sweep (contract §1.3(8)).
 */
export const isQueryKeyUnderProjectRoot = (queryKey: readonly unknown[], projectRoot: string) => {
    const [domain, scope, path] = queryKey
    if (typeof path !== 'string') return false
    if (!PROJECT_SCOPED_PATH_KEY_PREFIXES.some(([d, s]) => d === domain && s === scope)) return false
    return path === projectRoot || path.startsWith(`${projectRoot}${PATH_SEPARATOR}`)
}

/**
 * `change.fromApp` (contract X1#10) marks a watcher batch this app's own write caused, but only
 * `modified` never changes what the tree looks like — a content-only save touches no entry's
 * existence, name, or position. `created`/`renamed`/`removed` do, and two from_app-marked call sites
 * mutate the tree without refreshing it themselves: `shared/lib/lsp/workspace-edit-applier.ts`'s
 * `CreateFile`/`RenameFile`/`DeleteFile` WorkspaceEdit operations, and the remote dispatch's
 * `file_create`/`file_rename`/`file_delete` arms (the desktop window has no local call site to
 * refresh from for either). Narrowing the skip to `modified` keeps R4#13's N+1 savings for
 * content-only self-writes while leaving every tree-shape-changing kind on the watcher-echo refresh
 * this app relied on before from_app existed.
 */
export const isSelfEchoWithoutTreeImpact = (change: FsChange) => change.fromApp && change.kind === 'modified'

/**
 * The "reopen with" registry (contract §1.3(2)) is keyed by bare path, not scoped to any one
 * project's query cache, so pruning it on `projectClosed` has to know which paths are still open in
 * *other* projects before dropping every override that belonged only to the one that just closed.
 * `layoutEntries` is `queryClient.getQueriesData<ProjectLayout>({ queryKey: QUERY_KEY.LAYOUT.ALL })`
 * — every project's cached layout, main window and auxiliary windows both (`collectAllPaneTabs`) —
 * filtered down to every other project's still-open file tabs. Taking the raw entries as a parameter
 * rather than reading the query client directly keeps this pure and unit-testable, matching
 * `isStaleLayoutRevision`/`isQueryKeyUnderProjectRoot` above.
 */
export const collectOpenFilePathsOutsideProject = (
    layoutEntries: ReadonlyArray<readonly [readonly unknown[], ProjectLayout | undefined]>,
    excludedProjectId: ProjectId,
) =>
    layoutEntries
        .filter(([queryKey]) => queryKey[2] !== excludedProjectId)
        .flatMap(([, layout]) => (layout ? collectAllPaneTabs(layout) : []))
        .flatMap((tab) => (tab.kind.kind === 'file' ? [tab.kind.path] : []))

type RefreshTreeDirFn = (dir: string) => ReturnType<typeof refreshTreeDir>

/**
 * Refreshes every changed directory's tree cache entry and reconciles the `TREE.ROWS` query cache
 * with the result. A single-directory batch (the common case — the debounced watcher groups paths
 * by change kind, so most fs events resolve to exactly one parent dir) writes `refreshTreeDir`'s
 * page straight into the cache: the same one round trip `refreshTreeDir` already made, instead of
 * discarding it and paying for a second `TREE.ROWS` fetch on top (contract R4#13's "N+1"). A batch
 * spanning multiple directories can't reuse that shortcut — `Promise.all`'s result array preserves
 * *input* order, not completion order, so "the last resolved page" isn't well-defined, and Rust's
 * `tree_refresh` snapshot from any one call only reflects invalidations that landed before that call
 * took the per-project mutation guard. Every directory is still refreshed via `Promise.allSettled`
 * (so each one's cache entry is invalidated and re-read from disk even if a sibling call fails), and
 * the cache write comes from a plain `invalidateQueries` once all of them have settled —
 * `tree_rows`/`getTreeRows` reads the tree store's *current* state without rescanning disk, so by
 * then it already reflects every directory's invalidation. `invalidateQueries` is also the fallback
 * whenever the lone `refreshTreeDir` call in the single-directory path itself fails, so a partial
 * failure still ends in a correct, non-stale cache rather than a silently stale one.
 */
export const syncTreeRowsForChangedDirs = async (
    dirs: string[],
    deps: { refreshTreeDir: RefreshTreeDirFn; setTreeRows: (page: Awaited<ReturnType<RefreshTreeDirFn>>) => void; invalidateTreeRows: () => void },
) => {
    if (dirs.length === 0) return

    if (dirs.length === 1) {
        try {
            deps.setTreeRows(await deps.refreshTreeDir(dirs[0]))
        } catch {
            deps.invalidateTreeRows()
        }
        return
    }

    await Promise.allSettled(dirs.map((dir) => deps.refreshTreeDir(dir)))
    deps.invalidateTreeRows()
}

export const IpcSyncProvider: FC<PropsWithChildren> = ({ children }) => {
    const queryClient = useQueryClient()
    const lastLayoutRevisionByProjectRef = useRef(new Map<ProjectId, number>())

    useLspSessionsQueryInvalidationSync()

    useTauriEvent(events.projectListChanged, () => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.LIST })
    })

    useTauriEvent(events.projectOpened, ({ payload }) => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.DETAIL(payload.project.id) })
    })

    useTauriEvent(events.projectClosed, ({ payload }) => {
        /**
         * Read before the `PROJECT_SCOPED_KEYS` sweep below, which removes `QUERY_KEY.PROJECT.DETAIL`
         * (this same closing project's own entry) as one of its targets — the project's `root` has to
         * be captured from the cache first, or there is nothing left to build the path-prefix
         * predicate from.
         */
        const project = queryClient.getQueryData<Project>(QUERY_KEY.PROJECT.DETAIL(payload.projectId))

        pruneOpenWithOverrides(
            collectOpenFilePathsOutsideProject(queryClient.getQueriesData<ProjectLayout>({ queryKey: QUERY_KEY.LAYOUT.ALL }), payload.projectId),
        )

        for (const scopedKey of PROJECT_SCOPED_KEYS) queryClient.removeQueries({ queryKey: scopedKey(payload.projectId) })
        if (project) queryClient.removeQueries({ predicate: (query) => isQueryKeyUnderProjectRoot(query.queryKey, project.root) })

        lastLayoutRevisionByProjectRef.current.delete(payload.projectId)
        flushLspSessionsForProject(payload.projectId)
    })

    useTauriEvent(events.projectActivated, () => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL })
    })

    useTauriEvent(events.layoutChanged, ({ payload }) => {
        const lastRevision = lastLayoutRevisionByProjectRef.current.get(payload.projectId)
        if (isStaleLayoutRevision(lastRevision, payload.revision)) return

        lastLayoutRevisionByProjectRef.current.set(payload.projectId, payload.revision)
        const cachedLayout = queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(payload.projectId))
        if (isLayoutEchoAlreadyInCache(cachedLayout?.revision, payload.revision)) return

        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.LAYOUT.DETAIL(payload.projectId) })
    })

    useTauriEvent(events.themeChanged, () => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })
    })

    /**
     * Both `.git`-watcher events sweep the project's git subtree, but never the rev-immutable scopes
     * (`QUERY_KEY.GIT.REV_IMMUTABLE_SCOPES` — commit file lists and commit blob diffs, keyed by an
     * immutable SHA). `entities/git/git.query.ts`'s mutations already scope their own coarse
     * `GIT.PROJECT` invalidation with `isGitQueryScopeMutable`, but the event echo of those same
     * mutations did not, so the guarantee collapsed the moment the watcher echo landed a beat later:
     * every open commit-detail / file-history panel refetched its `staleTime: Infinity` blob on every
     * stage, unstage, stash and commit (audit §1-8). Same `queryKey` + `predicate` AND combination
     * the mutations use, so the project scoping is unchanged.
     */
    useTauriEvent(events.gitStatusChanged, ({ payload }) => {
        void queryClient.invalidateQueries({
            queryKey: QUERY_KEY.GIT.PROJECT(payload.projectId),
            predicate: (query) => isGitQueryScopeMutable(query.queryKey),
        })
    })

    useTauriEvent(events.gitRefsChanged, ({ payload }) => {
        void queryClient.invalidateQueries({
            queryKey: QUERY_KEY.GIT.PROJECT(payload.projectId),
            predicate: (query) => isGitQueryScopeMutable(query.queryKey),
        })
    })

    useTauriEvent(events.settingsChanged, ({ payload }) => {
        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, payload.settings)
        // The settings.json `AppFile` tab (`app-file-pane.tsx`) reads this same content through its
        // own `staleTime: Infinity` query, so without this it silently keeps showing whatever it
        // last fetched even after settings changed elsewhere (the settings screen, sync_download,
        // a remote session) — saving from that stale tab would then overwrite the just-applied
        // change right back to its old value. `SETTINGS.CURRENT` above only refreshes the settings
        // screen's own cache entry, not this one.
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.APP_FILE.CONTENT({ kind: 'settings' }) })
    })

    /**
     * A pty that exits has to be marked dead in every cached terminal roster, not just in the pane
     * that happens to be showing it. `pane-node-view.tsx` renders only each pane's *active* tab, so
     * a terminal tab sitting in the background has no mounted component at all — an exit while it is
     * hidden used to be heard by nobody, `terminal_sessions` (`staleTime: Infinity`) kept reporting
     * `running: true`, and returning to that tab attached to a dead session: a terminal that took
     * input, echoed nothing, and never offered the "[process exited]" restart button (audit §4-B
     * B14). Handled here rather than in `terminal-session.tsx` because this provider is mounted in
     * every window regardless of which tabs are open.
     *
     * The roster is edited in place instead of invalidated: a refetch is asynchronous, and a tab
     * remounting inside that window would read the still-stale `running: true` and attach to the
     * dead session anyway — the very race being closed. `markTerminalSessionExited` returns
     * `undefined` (no write) for rosters that don't hold this session, so the other projects'
     * entries `SESSIONS_ALL` sweeps over are left untouched.
     */
    useTauriEvent(events.terminalExited, ({ payload }) => {
        queryClient.setQueriesData<TerminalSession[]>({ queryKey: QUERY_KEY.TERMINAL.SESSIONS_ALL }, (sessions) =>
            markTerminalSessionExited(sessions, payload.sessionId),
        )
    })

    useTauriEvent(events.remoteStateChanged, ({ payload }) => queryClient.setQueryData(QUERY_KEY.REMOTE.STATUS, payload.status))

    useTauriEvent(events.syncStateChanged, ({ payload }) => {
        queryClient.setQueryData(QUERY_KEY.SYNC.STATUS, payload.status)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.SETTINGS.CURRENT })
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.LOCALE.ALL })
    })

    useTauriEvent(events.fsChanged, ({ payload }) => {
        const { projectId, change } = payload
        const changedPaths = new Set(change.paths)

        void queryClient.invalidateQueries({
            queryKey: QUERY_KEY.FILE.ALL,
            predicate: (query) => isFilePathQueryForChangedPaths(query.queryKey, changedPaths),
        })

        /**
         * The `.git`-directory watcher (`src-tauri/src/domain/git/watch.rs`'s `classify_git_change`)
         * only ever emits `git:status-changed`/`git:refs-changed` for writes under `index`/`HEAD`/
         * `refs` — it has no visibility into the worktree itself, so a change made by an external
         * editor or CLI (never touching `.git`) reaches neither of those events and the git panel,
         * status summary, and open-file gutters/diffs go stale forever (contract
         * docs/acknowledge/2026-08-27-d44-git-worktree-staleness-contract.md §0). This `fs:changed`
         * watcher echo is the only signal that ever sees such a change, so it stands in for the
         * missing worktree-axis git event here.
         *
         * Placed above the `isSelfEchoWithoutTreeImpact` early-return below — the same layer as the
         * `FILE.CONTENT`/`FILE.RAW` invalidation just above — for the same reason those are:
         * `entities/search`'s `useReplaceSearch` (a project-wide find-and-replace) has no `onSuccess`
         * invalidation of its own, so this watcher echo is the *only* path that ever refreshes
         * `GIT.*` after a replace touches a tracked file, exactly as it is the only refresh path for
         * `FILE.CONTENT`/`FILE.RAW`. Gating this block on `isSelfEchoWithoutTreeImpact` would leave
         * the git panel/gutters/diffs silently stale after every in-app replace, and — since that
         * gate fires identically in every window a project is open in — after every in-app save from
         * a *different* window too. The one case this placement is genuinely redundant for is
         * `useSaveFile`'s own `onSuccess` (`entities/file/file.query.ts`), which already invalidates
         * `GIT.PROJECT` for the saving window's own `fromApp`+`modified` echo: that one window pays
         * one extra `WATCH_DEBOUNCE_MS` (300ms)-later refetch on top of its immediate one. That
         * duplicate refetch is accepted as the price of covering `useReplaceSearch` and every other
         * open window without giving `entities/search` a bespoke `onSuccess` of its own
         * (`entities/search` is outside this contract's ownership).
         *
         * `STATUS` is invalidated unconditionally — it has no path axis to narrow by. `GUTTER`/`DIFF`
         * are narrowed to exactly this batch's paths via `isGitWorktreeQueryForChangedPaths`, ANDed
         * with the coarse `GIT.PROJECT` prefix (TanStack Query's `matchQuery` — same combination
         * `isGitQueryScopeMutable` in `git.query.ts` uses): the prefix is never invalidated bare, so
         * `LOG`/`BRANCHES`/`REMOTES`/`STASHES`/`TAGS`/etc. never match and stay untouched — they're
         * already the `.git` watcher's own axis (contract §1, over-invalidation guard).
         */
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.STATUS(projectId) })
        void queryClient.invalidateQueries({
            queryKey: QUERY_KEY.GIT.PROJECT(projectId),
            predicate: (query) => isGitWorktreeQueryForChangedPaths(query.queryKey, changedPaths),
        })

        /**
         * The palette's `SEARCH.PROJECT_FILES` quick-open index (contract §3, item d) is a flat
         * project-wide walk, not a per-directory page — unlike `TREE.ROWS` below, there is no
         * cheaper "just this directory" refresh to reuse, so a full re-walk on every `modified`
         * (content-only, never changes which files exist) would be pure waste on a large project
         * being actively edited/watched. `created`/`renamed`/`removed` are the only kinds that can
         * change the file set quick-open searches, so only those invalidate it.
         *
         * Placed above the `isSelfEchoWithoutTreeImpact` early-return below on purpose: that gate
         * decides only whether the (separate) Explorer tree needs refreshing, and this index has its
         * own, independent staleness rule — `kind` alone, with no `fromApp` check at all. Sitting
         * below the return would make this invalidation look conditioned on the tree gate when it
         * structurally isn't, even though the two rules currently agree on every `modified` change.
         */
        if (change.kind !== 'modified') void queryClient.invalidateQueries({ queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(projectId) })

        /**
         * `change.fromApp` (contract X1#10) marks a watcher batch whose every path was just written
         * by this same app (`file_save`/`file_create`/`file_rename`/`file_delete`/`file_copy`/
         * `search_replace`'s echo, self-write TTL-marked in `infra::self_write`). The tree refresh
         * below is skipped only for the `modified` kind (see `isSelfEchoWithoutTreeImpact`) — a
         * content-only self-write never changes what the tree looks like, so refreshing it would
         * only repeat the same, more expensive directory-listing round trip a second time (R4#13's
         * "N+1"). `created`/`renamed`/`removed` batches always refresh regardless of `fromApp`:
         * some from_app-marked call sites for those kinds (`explorer-container.tsx`'s own
         * create/rename/delete/copy/paste flows) already refresh the tree directly and would simply
         * pay for a redundant refresh here, but `shared/lib/lsp/workspace-edit-applier.ts`'s
         * WorkspaceEdit resource operations and the remote dispatch's `file_create`/`file_rename`/
         * `file_delete` arms do not — they have no local call site to refresh from — so this handler
         * is their only path back to a non-stale explorer tree.
         *
         * The `FILE.CONTENT`/`FILE.RAW` invalidations above are deliberately *not* gated the same
         * way. `useSaveFile`/`useRenameEntry`/`useCopyEntry`/`useDeleteEntry` do already invalidate
         * `FILE.CONTENT` directly `onSuccess` (so skipping it here for those specific operations
         * would be equally safe), but `entities/search`'s `useReplaceSearch` (a project-wide
         * find-and-replace) has no `onSuccess` invalidation of its own — this watcher echo is the
         * *only* path that refreshes an already-open tab's content after a replace touches its file.
         * `FILE.RAW` (`preview-pane.tsx`'s binary/image/PDF preview cache) has no `onSuccess`
         * invalidation anywhere at all — this watcher echo is its *only* refresh path, full stop.
         * Gating either on `fromApp` would leave that tab/preview silently stale. `entities/search`
         * is outside this contract's ownership, so the `FILE.CONTENT` gap belongs there (give
         * `useReplaceSearch` its own `onSuccess`, matching its sibling mutations) rather than here.
         */
        if (isSelfEchoWithoutTreeImpact(change)) return

        const dirs = [...new Set(change.paths.map(parentDirOf))]
        void syncTreeRowsForChangedDirs(dirs, {
            refreshTreeDir: (dir) => refreshTreeDir({ projectId, dir }),
            setTreeRows: (page) => queryClient.setQueryData(QUERY_KEY.TREE.ROWS(projectId), page),
            invalidateTreeRows: () => void queryClient.invalidateQueries({ queryKey: QUERY_KEY.TREE.ROWS(projectId) }),
        })
    })

    return children
}
