import type { FC, PropsWithChildren } from 'react'
import { useRef } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import type { FsChange, Project, ProjectId, ProjectLayout } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { PROJECT_SCOPED_KEYS, PROJECT_SCOPED_PATH_KEY_PREFIXES, QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { isStaleLayoutRevision } from '@shared/lib/layout-revision'
import { collectAllPaneTabs } from '@shared/lib/pane-tree'
import { refreshTreeDir } from '@entities/tree/tree.ipc'
import { pruneOpenWithOverrides } from '@entities/editor/open-with-registry'
import { flushLspSessionsForProject } from '@entities/lsp/lsp-session-flush-registry'
import { useLspSessionsQueryInvalidationSync } from '@entities/lsp/lsp.query'

const PATH_SEPARATOR = '/'

const parentDirOf = (path: string) => {
    const index = path.lastIndexOf(PATH_SEPARATOR)
    return index <= 0 ? PATH_SEPARATOR : path.slice(0, index)
}

/**
 * A changed file `path` is cached under two independent bare-path keys — `FILE.CONTENT`
 * (`file.query.ts`'s `fileQueryOptions`, open editor tabs) and `FILE.RAW` (`fileRawQueryOptions`,
 * `preview-pane.tsx`'s binary/image/PDF preview) — both `staleTime: Infinity` and so both silently
 * stale forever unless invalidated explicitly. `FILE.RAW` has no `onSuccess` invalidation anywhere
 * in `entities/file/file.query.ts` (contract §1-b), so this watcher echo is its only refresh path.
 */
export const filePathQueryKeysToInvalidate = (path: string) => [QUERY_KEY.FILE.CONTENT(path), QUERY_KEY.FILE.RAW(path)] as const

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
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.LAYOUT.DETAIL(payload.projectId) })
    })

    useTauriEvent(events.themeChanged, () => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })
    })

    useTauriEvent(events.gitStatusChanged, ({ payload }) => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(payload.projectId) })
    })

    useTauriEvent(events.gitRefsChanged, ({ payload }) => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(payload.projectId) })
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

    useTauriEvent(events.remoteStateChanged, ({ payload }) => queryClient.setQueryData(QUERY_KEY.REMOTE.STATUS, payload.status))

    useTauriEvent(events.syncStateChanged, ({ payload }) => {
        queryClient.setQueryData(QUERY_KEY.SYNC.STATUS, payload.status)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.SETTINGS.CURRENT })
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.THEME.ALL })
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.LOCALE.ALL })
    })

    useTauriEvent(events.fsChanged, ({ payload }) => {
        const { projectId, change } = payload

        for (const path of change.paths) {
            for (const queryKey of filePathQueryKeysToInvalidate(path)) void queryClient.invalidateQueries({ queryKey })
        }

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
