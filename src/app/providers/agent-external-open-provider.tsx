import { useEffect, useEffectEvent, type FC, type PropsWithChildren } from 'react'
import { useQueryClient, type QueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ExternalOpenRequest } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { i18next } from '@shared/i18n/i18n'
import { QUERY_KEY } from '@shared/constants/query-key'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { useOpenFileTab } from '@entities/layout/layout.query'
import { activateProject, listProjects, openProject } from '@entities/project/project.ipc'
import { clearStaleWaitMarkersOnStartup, registerWaitMarker } from '@entities/agent/agent-wait-marker-registry'
import { pendingExternalOpens, releaseWaitMarker } from '@entities/agent/agent.ipc'

const PATH_SEPARATOR = '/'

type OpenFileTab = ReturnType<typeof useOpenFileTab>

const isPathWithinRoot = (path: string, root: string) => path === root || path.startsWith(`${root}${PATH_SEPARATOR}`)

const tryOpenAsProject = async (path: string) => {
    try {
        await openProject(path)
        return true
    } catch {
        return false
    }
}

const processExternalOpenRequest = async (queryClient: QueryClient, openFileTab: OpenFileTab, request: ExternalOpenRequest) => {
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
        openFileTab(
            { projectId: owningProject.id, path: request.path, target: null, preview: true },
            {
                onSuccess: () => {
                    if (request.waitMarker) registerWaitMarker(request.path, request.waitMarker)
                },
                onError: () => {
                    if (request.waitMarker) void releaseWaitMarker(request.waitMarker)
                },
            },
        )
    } catch (error) {
        toast.error(describeIpcError(error))
        if (request.waitMarker) void releaseWaitMarker(request.waitMarker)
    }
}

const drainPendingExternalOpens = (queryClient: QueryClient, openFileTab: OpenFileTab) =>
    pendingExternalOpens().then((requests) => {
        for (const request of requests) void processExternalOpenRequest(queryClient, openFileTab, request)
    })

export const AgentExternalOpenProvider: FC<PropsWithChildren> = ({ children }) => {
    const queryClient = useQueryClient()
    const openFileTab = useOpenFileTab()

    /**
     * `useOpenFileTab` returns a fresh closure on every render (it is a plain function, not a
     * mutation object), so the startup drain below reads it through a `useEffectEvent` instead of
     * listing it as a dependency — a dependency would re-run the drain on every render and
     * reprocess the same queued requests.
     */
    const drainOnMount = useEffectEvent(() => void drainPendingExternalOpens(queryClient, openFileTab))

    /**
     * The backend always queues the request in `AgentStore` before emitting this event (single
     * source of truth), so the handler drains the queue rather than acting on the event payload
     * directly — acting on the payload too would leave the queued entry behind for the next
     * `pendingExternalOpens` drain (e.g. on Reload Window) to reprocess as a duplicate.
     */
    useTauriEvent(events.agentExternalOpen, () => void drainPendingExternalOpens(queryClient, openFileTab))

    useEffect(() => {
        clearStaleWaitMarkersOnStartup()
        drainOnMount()
    }, [])

    return children
}
