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

    /**
     * `refreshTreeDir` already returns the tree's up-to-date `TreeRowPage` after invalidating one
     * directory (Rust's `tree_refresh` rebuilds the full page under the same per-project mutation
     * guard every `tree_toggle`/`tree_reveal` call does) — writing that straight into the cache
     * with `setQueryData` covers the common single-directory case with the same one round trip
     * `refreshTreeDir` already made, instead of discarding it and paying for a second `TREE.ROWS`
     * fetch on top (contract R4#13's "N+1"). The batch's last resolved page reflects every prior
     * directory's invalidation too, since they all serialize through that same per-project guard.
     * `invalidateQueries` is kept as the fallback for the (rare) case a directory refresh itself
     * fails, so a partial failure still ends in a correct, non-stale cache rather than a silently
     * stale one.
     */
    useTauriEvent(events.fsChanged, ({ payload }) => {
        const { projectId, change } = payload

        for (const path of change.paths) {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) })
        }

        const dirs = [...new Set(change.paths.map(parentDirOf))]
        void Promise.all(dirs.map((dir) => refreshTreeDir({ projectId, dir }))).then(
            (pages) => queryClient.setQueryData(QUERY_KEY.TREE.ROWS(projectId), pages[pages.length - 1]),
            () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.TREE.ROWS(projectId) }),
        )
    })

    return children
}
