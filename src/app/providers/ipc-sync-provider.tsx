import type { FC, PropsWithChildren } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { events } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { refreshTreeDir } from '@entities/tree/tree.ipc'

const PATH_SEPARATOR = '/'

const parentDirOf = (path: string) => {
    const index = path.lastIndexOf(PATH_SEPARATOR)
    return index <= 0 ? PATH_SEPARATOR : path.slice(0, index)
}

export const IpcSyncProvider: FC<PropsWithChildren> = ({ children }) => {
    const queryClient = useQueryClient()

    useTauriEvent(events.projectListChanged, () => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.LIST })
    })

    useTauriEvent(events.projectOpened, ({ payload }) => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.DETAIL(payload.project.id) })
    })

    useTauriEvent(events.projectClosed, ({ payload }) => {
        queryClient.removeQueries({ queryKey: QUERY_KEY.PROJECT.DETAIL(payload.projectId) })
        queryClient.removeQueries({ queryKey: QUERY_KEY.LAYOUT.DETAIL(payload.projectId) })
        queryClient.removeQueries({ queryKey: QUERY_KEY.TREE.ROWS(payload.projectId) })
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

    useTauriEvent(events.fsChanged, ({ payload }) => {
        const { projectId, change } = payload

        for (const path of change.paths) {
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(path) })
        }

        const dirs = new Set(change.paths.map(parentDirOf))
        void Promise.all([...dirs].map((dir) => refreshTreeDir({ projectId, dir }).catch(() => undefined))).then(() =>
            queryClient.invalidateQueries({ queryKey: QUERY_KEY.TREE.ROWS(projectId) }),
        )
    })

    return children
}
