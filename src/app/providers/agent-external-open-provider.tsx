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
import { activateProject, getActiveProjectId, listProjects, openProject } from '@entities/project/project.ipc'
import { clearStaleWaitMarkersOnStartup, registerWaitMarker } from '@entities/agent/agent-wait-marker-registry'
import { resolveExternalOpenTarget } from '@entities/agent/external-open-target'
import { pendingExternalOpens, releaseWaitMarker } from '@entities/agent/agent.ipc'

type OpenFileTab = ReturnType<typeof useOpenFileTab>

const tryOpenAsProject = async (path: string) => {
    try {
        await openProject(path)
        return true
    } catch {
        return false
    }
}

/**
 * A file that is not itself a project opens as a tab of the project `resolveExternalOpenTarget`
 * picks — the one whose root contains it, else the active project, which is how Claude Code's
 * Ctrl+G temp file (under the OS tmpdir, inside no root) reaches an editor at all. A `--wait`
 * request opens a *pinned* tab, never a preview one: `layout::service::open_tab` replaces a pane's
 * existing preview tab in place when another preview opens, and a replacement is not a close, so
 * the marker `useCloseTab` releases on close would never be released and the waiting CLI (and the
 * Claude Code prompt behind it) would sit until its timeout.
 */
const processExternalOpenRequest = async (queryClient: QueryClient, openFileTab: OpenFileTab, request: ExternalOpenRequest) => {
    if (await tryOpenAsProject(request.path)) {
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL })
        if (request.waitMarker) void releaseWaitMarker(request.waitMarker).catch(() => undefined)
        return
    }

    /**
     * The project lookup sits inside the same `try` as the activate/open below rather than ahead of
     * it: `listProjects`/`getActiveProjectId` go through `unwrapResult`, so a rejection there used to
     * escape this function entirely — past the `catch` that exists to toast the failure and release
     * the `--wait` marker — and land as an unhandled rejection at the `void` call sites below, with
     * the waiting CLI left hanging until its own timeout.
     */
    try {
        const [projects, activeProjectId] = await Promise.all([listProjects(), getActiveProjectId()])
        const target = resolveExternalOpenTarget({ path: request.path, projects, activeProjectId })
        if (!target) {
            toast.info(i18next.t('app.openProjectFirst'))
            if (request.waitMarker) void releaseWaitMarker(request.waitMarker).catch(() => undefined)
            return
        }

        await activateProject(target.projectId)
        void queryClient.invalidateQueries({ queryKey: QUERY_KEY.PROJECT.ALL })
        openFileTab(
            { projectId: target.projectId, path: request.path, target: null, preview: !request.waitMarker },
            {
                onSuccess: () => {
                    if (!request.waitMarker) return
                    registerWaitMarker(request.path, request.waitMarker)
                    toast.info(i18next.t('app.externalEditorTabHint'))
                },
                onError: () => {
                    if (request.waitMarker) void releaseWaitMarker(request.waitMarker).catch(() => undefined)
                },
            },
        )
    } catch (error) {
        toast.error(describeIpcError(error))
        if (request.waitMarker) void releaseWaitMarker(request.waitMarker).catch(() => undefined)
    }
}

/**
 * The `.catch` closes the chain for both call sites below at once — each one only ever `void`s the
 * returned promise, so a rejected `pendingExternalOpens()` (or a `processExternalOpenRequest` that
 * somehow still throws) would otherwise surface as an unhandled rejection. Nothing here is worth
 * interrupting the user for: the queue stays in `AgentStore` and the next drain reprocesses it.
 */
const drainPendingExternalOpens = (queryClient: QueryClient, openFileTab: OpenFileTab) =>
    pendingExternalOpens()
        .then((requests) => {
            for (const request of requests) void processExternalOpenRequest(queryClient, openFileTab, request)
        })
        .catch(() => undefined)

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
