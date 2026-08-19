import type { FC, PropsWithChildren } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { events } from '@shared/api/bindings'
import { PROJECT_SCOPED_KEYS, QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { refreshTreeDir } from '@entities/tree/tree.ipc'
import { flushLspSessionsForProject } from '@entities/lsp/lsp-session-flush-registry'
import { useLspSessionsQueryInvalidationSync } from '@entities/lsp/lsp.query'

const PATH_SEPARATOR = '/'

const parentDirOf = (path: string) => {
    const index = path.lastIndexOf(PATH_SEPARATOR)
    return index <= 0 ? PATH_SEPARATOR : path.slice(0, index)
}

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

    useLspSessionsQueryInvalidationSync()

    useTauriEvent(events.projectListChanged, () => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.LIST })
    })

    useTauriEvent(events.projectOpened, ({ payload }) => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.DETAIL(payload.project.id) })
    })

    useTauriEvent(events.projectClosed, ({ payload }) => {
        for (const scopedKey of PROJECT_SCOPED_KEYS) queryClient.removeQueries({ queryKey: scopedKey(payload.projectId) })
        flushLspSessionsForProject(payload.projectId)
    })

    useTauriEvent(events.projectActivated, () => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL })
    })

    useTauriEvent(events.layoutChanged, ({ payload }) => {
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

        const dirs = [...new Set(change.paths.map(parentDirOf))]
        void syncTreeRowsForChangedDirs(dirs, {
            refreshTreeDir: (dir) => refreshTreeDir({ projectId, dir }),
            setTreeRows: (page) => queryClient.setQueryData(QUERY_KEY.TREE.ROWS(projectId), page),
            invalidateTreeRows: () => void queryClient.invalidateQueries({ queryKey: QUERY_KEY.TREE.ROWS(projectId) }),
        })
    })

    return children
}
