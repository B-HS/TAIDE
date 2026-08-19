import { useEffect, type FC, type PropsWithChildren } from 'react'
import { useQueryClient, type QueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ExternalOpenRequest } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { i18next } from '@shared/i18n/i18n'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { useOpenTabInProject } from '@entities/layout/layout.query'
import { activateProject, listProjects, openProject } from '@entities/project/project.ipc'
import { clearStaleWaitMarkersOnStartup, registerWaitMarker } from '@entities/agent/agent-wait-marker-registry'
import { pendingExternalOpens, releaseWaitMarker } from '@entities/agent/agent.ipc'

const PATH_SEPARATOR = '/'

type OpenTabInProject = ReturnType<typeof useOpenTabInProject>['mutateAsync']

const isPathWithinRoot = (path: string, root: string) => path === root || path.startsWith(`${root}${PATH_SEPARATOR}`)

const tryOpenAsProject = async (path: string) => {
    try {
        await openProject(path)
        return true
    } catch {
        return false
    }
}

const openExternalFile = (openTabInProject: OpenTabInProject, projectId: string, path: string) => {
    const name = path.slice(path.lastIndexOf(PATH_SEPARATOR) + 1)
    return openTabInProject({ projectId, kind: { kind: 'file', path }, title: name, target: null, preview: true })
}

const processExternalOpenRequest = async (queryClient: QueryClient, openTabInProject: OpenTabInProject, request: ExternalOpenRequest) => {
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
        await openExternalFile(openTabInProject, owningProject.id, request.path)
        if (request.waitMarker) registerWaitMarker(request.path, request.waitMarker)
    } catch (error) {
        toast.error(error instanceof Error ? error.message : String(error))
        if (request.waitMarker) void releaseWaitMarker(request.waitMarker)
    }
}

const drainPendingExternalOpens = (queryClient: QueryClient, openTabInProject: OpenTabInProject) =>
    pendingExternalOpens().then((requests) => {
        for (const request of requests) void processExternalOpenRequest(queryClient, openTabInProject, request)
    })

export const AgentExternalOpenProvider: FC<PropsWithChildren> = ({ children }) => {
    const queryClient = useQueryClient()
    const { mutateAsync: openTabInProject } = useOpenTabInProject()

    /**
     * The backend always queues the request in `AgentStore` before emitting this event (single
     * source of truth), so the handler drains the queue rather than acting on the event payload
     * directly — acting on the payload too would leave the queued entry behind for the next
     * `pendingExternalOpens` drain (e.g. on Reload Window) to reprocess as a duplicate.
     */
    useTauriEvent(events.agentExternalOpen, () => void drainPendingExternalOpens(queryClient, openTabInProject))

    useEffect(() => {
        clearStaleWaitMarkersOnStartup()
        void drainPendingExternalOpens(queryClient, openTabInProject)
    }, [queryClient, openTabInProject])

    return children
}
