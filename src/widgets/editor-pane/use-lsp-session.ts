import { useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import type { FileSizeTier, LspInitializationOptionsValue, LspServerId, ProjectId, Settings } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
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
}

const attachLspSession = ({
    projectId,
    serverId,
    projectRoot,
    path,
    languageId,
    initializationOptions,
    isCodeLensEnabled,
}: AttachLspSessionInput) => {
    let disposed = false
    let openedUri: string | null = null
    let contentChangeDisposable: { dispose: () => void } | null = null
    let sessionHandle: { key: string; record: SessionRecord } | null = null

    void resolveLspRoot({ serverId, filePath: path })
        .catch(() => null)
        .then((resolvedRoot) => {
            if (disposed) return

            const handle = acquireLspSession(projectId, serverId, resolvedRoot ?? projectRoot, initializationOptions)
            sessionHandle = handle

            return handle.record.ready.then((session) => {
                if (disposed) return
                ensureLanguageRegistered(handle.record, session.client, serverId, languageId, isCodeLensEnabled)

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
                }),
            )

        return () => {
            cleanups.forEach((cleanup) => cleanup())
        }
    }, [enabled, languageId, path, projectId, tier, project?.root, servers, queryClient])
}
