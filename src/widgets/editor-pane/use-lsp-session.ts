import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { FileSizeTier, LspServerId, ProjectId } from '@shared/api/bindings'
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

type AttachLspSessionInput = {
    projectId: ProjectId
    serverId: LspServerId
    projectRoot: string
    path: string
    languageId: string
}

const attachLspSession = ({ projectId, serverId, projectRoot, path, languageId }: AttachLspSessionInput) => {
    let disposed = false
    let openedUri: string | null = null
    let contentChangeDisposable: { dispose: () => void } | null = null
    let sessionHandle: { key: string; record: SessionRecord } | null = null

    void resolveLspRoot({ serverId, filePath: path })
        .catch(() => null)
        .then((resolvedRoot) => {
            if (disposed) return

            const handle = acquireLspSession(projectId, serverId, resolvedRoot ?? projectRoot)
            sessionHandle = handle

            return handle.record.ready.then((session) => {
                if (disposed) return
                ensureLanguageRegistered(handle.record, session.client, serverId, languageId)

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

    useEffect(() => {
        if (!enabled || !languageId || tier === 'large' || tier === 'readOnly') return
        if (!project?.root || !servers) return

        const projectRoot = project.root
        const cleanups = servers
            .filter((server) => server.languageIds.includes(languageId) && server.available)
            .map((server) => attachLspSession({ projectId, serverId: server.id, projectRoot, path, languageId }))

        return () => {
            cleanups.forEach((cleanup) => cleanup())
        }
    }, [enabled, languageId, path, projectId, tier, project?.root, servers])
}
