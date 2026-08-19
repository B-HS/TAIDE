import type { FC, PropsWithChildren } from 'react'
import { useRef } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import type { Project, ProjectId } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { PROJECT_SCOPED_KEYS, PROJECT_SCOPED_PATH_KEY_PREFIXES, QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { refreshTreeDir } from '@entities/tree/tree.ipc'
import { flushLspSessionsForProject } from '@entities/lsp/lsp-session-flush-registry'
import { useLspSessionsQueryInvalidationSync } from '@entities/lsp/lsp.query'

const PATH_SEPARATOR = '/'

const parentDirOf = (path: string) => {
    const index = path.lastIndexOf(PATH_SEPARATOR)
    return index <= 0 ? PATH_SEPARATOR : path.slice(0, index)
}

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
 * `layout:changed`'s `revision` is a per-project monotonic counter (contract X1#11) — a window that
 * has already observed revision `N` must ignore any later delivery of a revision `<= N`. Without
 * this, two mutations completing close together each schedule their own `invalidateQueries` →
 * `getLayout` refetch, and nothing but delivery order guarantees the *newer* revision's refetch is
 * the one that lands last in the query cache; a duplicated or out-of-order event replays a fetch
 * for a revision this window has already moved past, and TanStack Query has no way to know that
 * fetch is for stale data. Gating on the event's own revision before ever starting that refetch
 * removes the redundant/out-of-order trigger at the source instead.
 */
export const isStaleLayoutRevision = (lastObservedRevision: number | undefined, incomingRevision: number) =>
    lastObservedRevision !== undefined && incomingRevision <= lastObservedRevision

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
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) })
        }

        /**
         * `change.fromApp` (contract X1#10) marks a watcher batch whose every path was just written
         * by this same app (`file_save`/`file_create`/`file_rename`/`file_delete`/`file_copy`/
         * `search_replace`'s echo, self-write TTL-marked in `infra::self_write`). The tree refresh
         * below is skipped entirely for such a batch — every from_app-marked operation either never
         * touches the tree (a content-only save) or already refreshes it directly at its own call
         * site (`explorer-container.tsx`'s `refreshTreeDir` after create/rename/delete/copy/paste),
         * so this handler's refresh would only repeat that same, more expensive directory-listing
         * round trip a second time (R4#13's "N+1", completed at its root now that the echo is
         * skippable instead of merely deduplicated).
         *
         * The `FILE.CONTENT` invalidation above is deliberately *not* gated the same way.
         * `useSaveFile`/`useRenameEntry`/`useCopyEntry`/`useDeleteEntry` do already invalidate it
         * directly `onSuccess` (so skipping it for those specific operations would be equally safe),
         * but `entities/search`'s `useReplaceSearch` (a project-wide find-and-replace) has no
         * `onSuccess` invalidation of its own — this watcher echo is the *only* path that refreshes
         * an already-open tab's content after a replace touches its file. Gating this on `fromApp`
         * too would leave that tab silently stale. `entities/search` is outside this contract's
         * ownership, so the fix belongs there (give `useReplaceSearch` its own `onSuccess`, matching
         * its sibling mutations) rather than here.
         */
        if (change.fromApp) return

        const dirs = [...new Set(change.paths.map(parentDirOf))]
        void syncTreeRowsForChangedDirs(dirs, {
            refreshTreeDir: (dir) => refreshTreeDir({ projectId, dir }),
            setTreeRows: (page) => queryClient.setQueryData(QUERY_KEY.TREE.ROWS(projectId), page),
            invalidateTreeRows: () => void queryClient.invalidateQueries({ queryKey: QUERY_KEY.TREE.ROWS(projectId) }),
        })
    })

    return children
}
