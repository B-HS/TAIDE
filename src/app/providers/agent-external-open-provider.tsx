import { useEffect, type FC, type PropsWithChildren } from 'react'
import { useQueryClient, type QueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ExternalOpenRequest } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { i18next } from '@shared/i18n/i18n'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { openTab } from '@entities/layout/layout.ipc'
import { activateProject, listProjects, openProject } from '@entities/project/project.ipc'
import { clearStaleWaitMarkersOnStartup, registerWaitMarker } from '@entities/agent/agent-wait-marker-registry'
import { pendingExternalOpens, releaseWaitMarker } from '@entities/agent/agent.ipc'

const PATH_SEPARATOR = '/'

const isPathWithinRoot = (path: string, root: string) => path === root || path.startsWith(`${root}${PATH_SEPARATOR}`)

const tryOpenAsProject = async (path: string) => {
    try {
        await openProject(path)
        return true
    } catch {
        return false
    }
}

const openExternalFile = async (queryClient: QueryClient, projectId: string, path: string) => {
    const name = path.slice(path.lastIndexOf(PATH_SEPARATOR) + 1)
    const layout = await openTab({ projectId, kind: { kind: 'file', path }, title: name, target: null, preview: true })
    queryClient.setQueryData(QUERY_KEY.LAYOUT.DETAIL(projectId), layout)
}

const processExternalOpenRequest = async (queryClient: QueryClient, request: ExternalOpenRequest) => {
    if (await tryOpenAsProject(request.path)) {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL })
        if (request.waitMarker) void releaseWaitMarker(request.waitMarker)
        return
    }

    const projects = await listProjects()
    const owningProject = projects.find((project) => isPathWithinRoot(request.path, project.root))
    if (!owningProject) {
        toast.info(i18next.t('app.openProjectFirst'))
        if (request.waitMarker) void releaseWaitMarker(request.waitMarker)
        return
    }

    try {
        await activateProject(owningProject.id)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL })
        await openExternalFile(queryClient, owningProject.id, request.path)
        if (request.waitMarker) registerWaitMarker(request.path, request.waitMarker)
    } catch (error) {
        toast.error(error instanceof Error ? error.message : String(error))
        if (request.waitMarker) void releaseWaitMarker(request.waitMarker)
    }
}

const drainPendingExternalOpens = (queryClient: QueryClient) =>
    pendingExternalOpens().then((requests) => {
        for (const request of requests) void processExternalOpenRequest(queryClient, request)
    })

export const AgentExternalOpenProvider: FC<PropsWithChildren> = ({ children }) => {
    const queryClient = useQueryClient()

    /**
     * The backend always queues the request in `AgentStore` before emitting this event (single
     * source of truth), so the handler drains the queue rather than acting on the event payload
     * directly — acting on the payload too would leave the queued entry behind for the next
     * `pendingExternalOpens` drain (e.g. on Reload Window) to reprocess as a duplicate.
     */
    useTauriEvent(events.agentExternalOpen, () => void drainPendingExternalOpens(queryClient))

    useEffect(() => {
        clearStaleWaitMarkersOnStartup()
        void drainPendingExternalOpens(queryClient)
    }, [queryClient])

    return children
}
