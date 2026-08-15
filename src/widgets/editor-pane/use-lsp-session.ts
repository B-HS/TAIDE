import { useEffect } from 'react'
import { hashKey, useQuery, useQueryClient } from '@tanstack/react-query'
import type { FileSizeTier, LspInitializationOptionsValue, LspServerId, ProjectId, Settings } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { triggerSemanticTokensRefresh } from '@shared/lib/lsp/adapters/semantic-tokens'
import { monacoRangeToLsp } from '@shared/lib/lsp/position'
import { getModel } from '@entities/editor/model-registry'
import { projectQueryOptions } from '@entities/project/project.query'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { resolveLspRoot } from '@entities/lsp/lsp.ipc'
import type { SessionRecord } from '@widgets/editor-pane/lsp-session-registry'
import {
    acquireDocument,
    acquireLspSession,
    ensureLanguageRegistered,
    releaseDocument,
    releaseLspSession,
} from '@widgets/editor-pane/lsp-session-registry'

type UseLspSessionInput = {
    projectId: ProjectId
    path: string
    languageId: string | null
    tier: FileSizeTier | null
    enabled: boolean
}

/**
 * Whether a file of this size tier ever gets an LSP session attached. Exported so callers that
 * need to know *without* mounting `useLspSession` (e.g. `editor-pane.tsx` deciding which servers
 * to wait on for Code Actions on Save / save notifications) can't drift out of sync with the
 * actual attach gate below — a session is never acquired for `large`/`readOnly` tier files
 * (`acquireLspSession` below is the only call site), so waiting on one for such a file waits
 * forever.
 */
export const isLspAttachableTier = (tier: FileSizeTier | null) => tier !== 'large' && tier !== 'readOnly'

type AttachLspSessionInput = {
    projectId: ProjectId
    serverId: LspServerId
    projectRoot: string
    path: string
    languageId: string
    initializationOptions: LspInitializationOptionsValue | null | undefined
    isCodeLensEnabled: () => boolean
    isSemanticHighlightingEnabled: () => boolean
    queryClient: ReturnType<typeof useQueryClient>
}

const SETTINGS_CURRENT_QUERY_HASH = hashKey(QUERY_KEY.SETTINGS.CURRENT)

const attachLspSession = ({
    projectId,
    serverId,
    projectRoot,
    path,
    languageId,
    initializationOptions,
    isCodeLensEnabled,
    isSemanticHighlightingEnabled,
    queryClient,
}: AttachLspSessionInput) => {
    let disposed = false
    let openedUri: string | null = null
    let contentChangeDisposable: { dispose: () => void } | null = null
    let sessionHandle: { key: string; record: SessionRecord } | null = null
    let unsubscribeSemanticHighlightingSetting: (() => void) | null = null

    void resolveLspRoot({ serverId, filePath: path })
        .catch(() => null)
        .then((resolvedRoot) => {
            if (disposed) return

            const handle = acquireLspSession(projectId, serverId, resolvedRoot ?? projectRoot, initializationOptions)
            sessionHandle = handle

            return handle.record.ready.then((session) => {
                if (disposed) return
                ensureLanguageRegistered(handle.record, session.client, serverId, languageId, isCodeLensEnabled, isSemanticHighlightingEnabled)

                /**
                 * Contract §3.1: the `editorSemanticHighlighting` toggle is an adapter-getter gate
                 * (`isSemanticHighlightingEnabled`, read fresh per request, no provider
                 * re-registration) that must still fire the registered provider's `onDidChange` on
                 * toggle — monaco's `ModelSemanticColoring` only ever recomputes on model-content
                 * change, provider/theme change, or the provider's own `onDidChange`, none of which a
                 * settings-only flip triggers by itself. Reusing `triggerSemanticTokensRefresh`
                 * (the same emitter the session-scoped `workspace/semanticTokens/refresh` handler
                 * fires) keeps this a one-emitter design instead of adding a second refresh channel.
                 */
                let lastSemanticHighlightingEnabled = isSemanticHighlightingEnabled()
                unsubscribeSemanticHighlightingSetting = queryClient.getQueryCache().subscribe((event) => {
                    if (event.type !== 'updated' || event.query.queryHash !== SETTINGS_CURRENT_QUERY_HASH) return
                    const nextEnabled = isSemanticHighlightingEnabled()
                    if (nextEnabled === lastSemanticHighlightingEnabled) return
                    lastSemanticHighlightingEnabled = nextEnabled
                    triggerSemanticTokensRefresh(session.client)
                })

                const model = getModel(path)
                if (!model) return

                const uri = model.uri.toString()
                openedUri = uri
                acquireDocument(handle.record, session.client, uri, languageId, model.getValue())

                contentChangeDisposable = model.onDidChangeContent((event) => {
                    const changes = event.changes.map((change) => ({
                        range: monacoRangeToLsp(change.range),
                        rangeLength: change.rangeLength,
                        text: change.text,
                    }))
                    session.client.didChange(uri, changes)
                })
            })
        })
        .catch(() => undefined)

    return () => {
        disposed = true
        contentChangeDisposable?.dispose()
        unsubscribeSemanticHighlightingSetting?.()
        if (!sessionHandle) return

        const { key, record } = sessionHandle
        void record.ready
            .then((session) => {
                if (openedUri) releaseDocument(record, session.client, openedUri)
            })
            .catch(() => undefined)
        releaseLspSession(key, record)
    }
}

export const useLspSession = ({ projectId, path, languageId, tier, enabled }: UseLspSessionInput) => {
    const { data: servers } = useQuery(lspServersQueryOptions())
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const queryClient = useQueryClient()

    useEffect(() => {
        if (!enabled || !languageId || !isLspAttachableTier(tier)) return
        if (!project?.root || !servers) return

        const projectRoot = project.root
        const isCodeLensEnabled = () => queryClient.getQueryData<Settings>(QUERY_KEY.SETTINGS.CURRENT)?.editorCodeLensEnabled ?? true
        const isSemanticHighlightingEnabled = () => queryClient.getQueryData<Settings>(QUERY_KEY.SETTINGS.CURRENT)?.editorSemanticHighlighting ?? true
        const cleanups = servers
            .filter((server) => server.languageIds.includes(languageId) && server.available)
            .map((server) =>
                attachLspSession({
                    projectId,
                    serverId: server.id,
                    projectRoot,
                    path,
                    languageId,
                    initializationOptions: server.initializationOptions,
                    isCodeLensEnabled,
                    isSemanticHighlightingEnabled,
                    queryClient,
                }),
            )

        return () => {
            cleanups.forEach((cleanup) => cleanup())
        }
    }, [enabled, languageId, path, projectId, tier, project?.root, servers, queryClient])
}
