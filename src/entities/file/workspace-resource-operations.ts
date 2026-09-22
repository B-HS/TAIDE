import type { QueryClient } from '@tanstack/react-query'
import type { OpenedFile } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { collectAllPaneTabs } from '@shared/lib/pane-tree'
import { isWithinRoot } from '@shared/lib/path-root'
import { deleteEntry, listMirrors, renameEntry } from '@entities/file/file.ipc'
import { getLayout } from '@entities/layout/layout.ipc'
import { applyFreshLayout } from '@entities/layout/layout.query'
import { followDeletedPathInTabs, followRenamedPathInTabs } from '@entities/layout/tab-path-change'
import { listProjects } from '@entities/project/project.ipc'
import { getModel } from '@entities/editor/model-registry'

const defaultDeps = {
    listProjects,
    getLayout,
    listMirrors,
    renameEntry,
    deleteEntry,
    followRenamedPathInTabs,
    followDeletedPathInTabs,
    readModelContent: (path: string) => getModel(path)?.getValue() ?? null,
}

export const createWorkspaceResourceOperations = (queryClient: QueryClient, deps = defaultDeps) => {
    const prepare = async (path: string, destination?: string) => {
        const projects = (await deps.listProjects()).filter((project) => isWithinRoot(path, project.root))
        if (projects.length === 0) throw new Error('Workspace file operation is outside open projects')
        if (destination && projects.some((project) => !isWithinRoot(destination, project.root))) {
            throw new Error('Moving files between project roots is not supported')
        }
        return Promise.all(
            projects.map(async (project) => {
                const [layout, mirrors] = await Promise.all([deps.getLayout(project.id), deps.listMirrors(project.id)])
                const hasUnsavedTab = collectAllPaneTabs(layout).some((tab) => {
                    if (tab.kind.kind !== 'file' || !isWithinRoot(tab.kind.path, path)) return false
                    const content = deps.readModelContent(tab.kind.path)
                    const saved = queryClient.getQueryData<OpenedFile>(QUERY_KEY.FILE.CONTENT(tab.kind.path))?.content
                    return tab.dirty || (content !== null && content !== saved)
                })
                if (!destination && (mirrors.some((mirror) => isWithinRoot(mirror.path, path)) || hasUnsavedTab)) {
                    throw new Error('Save or close unsaved files before deleting them')
                }
                queryClient.setQueryData(QUERY_KEY.FILE.MIRRORS(project.id), mirrors)
                return project.id
            }),
        )
    }

    const refresh = (projectId: string) => {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.GIT.PROJECT(projectId) })
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.TREE.ROWS(projectId) })
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(projectId) })
    }

    return {
        renameEntry: async ({ from, to }: Parameters<typeof renameEntry>[0]) => {
            const projectIds = await prepare(from, to)
            await deps.renameEntry({ from, to })
            for (const projectId of projectIds) {
                const result = await deps.followRenamedPathInTabs({ queryClient, projectId, from, to })
                applyFreshLayout(queryClient, projectId, result.layout)
                refresh(projectId)
            }
            void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.RAW(to) })
            return null
        },
        deleteEntry: async (path: string) => {
            const projectIds = await prepare(path)
            await deps.deleteEntry(path)
            for (const projectId of projectIds) {
                const result = await deps.followDeletedPathInTabs({ queryClient, projectId, path })
                applyFreshLayout(queryClient, projectId, result.layout)
                refresh(projectId)
            }
            return null
        },
    }
}
