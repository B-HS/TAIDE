import type { FC } from 'react'
import { useEffect, useState } from 'react'
import type { languages } from 'monaco-editor'
import { useQuery } from '@tanstack/react-query'
import type { LspServerId, ProjectId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { findActiveTab } from '@shared/lib/pane-tree'
import { requestDocumentSymbols } from '@shared/lib/lsp/adapters/document-symbol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { fileQueryOptions } from '@entities/file/file.query'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { resolveLspRoot } from '@entities/lsp/lsp.ipc'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { requestReveal } from '@entities/editor/reveal-registry'
import type { SessionRecord } from '@widgets/editor-pane/lsp-session-registry'
import { waitForLspSessionForRoot } from '@widgets/editor-pane/lsp-session-registry'
import { OutlinePanel } from '@features/outline/outline-panel'

type OutlinePanelContainerProps = {
    projectId: ProjectId
}

type SymbolsForPath = { path: string; symbols: languages.DocumentSymbol[] }

export type DocumentSymbolSessionWaiter = { promise: Promise<SessionRecord | null>; cancel: () => void }

type BuildDocumentSymbolWaitersInput = {
    availableServerIds: LspServerId[]
    path: string
    projectId: ProjectId
    fallbackRoot: string | undefined
    isCancelled: () => boolean
    resolveRoot: (input: { serverId: LspServerId; filePath: string }) => Promise<string | null>
    waitForSession: (projectId: ProjectId, serverId: LspServerId, root: string) => DocumentSymbolSessionWaiter
}

/**
 * root-aware conversion (`docs/acknowledge/2026-08-19-editor-pane-batch-contract.md` §1.2): resolves
 * each candidate server's actual LSP root for `path` (mirroring `use-lsp-session.ts`'s own
 * `resolveLspRoot(...) ?? projectRoot` acquire-time fallback exactly — a consumer that used a
 * different fallback could ask `waitForLspSessionForRoot` for a root key nothing was ever acquired
 * under) before waiting on a session, replacing the root-agnostic `waitForLspSession` that could
 * resolve to *any* root's session in a multi-root project (R7#7) — including one that never had
 * `path` open. `resolveRoot`/`waitForSession`/`isCancelled` are injected rather than imported
 * directly so this decision (which roots to wait on, and in what order) is a plain, directly
 * testable function of its inputs — this component has no render-test harness to reach for
 * (no DOM/testing-library environment configured for `bun:test` in this project).
 */
export const buildDocumentSymbolWaiters = async ({
    availableServerIds,
    path,
    projectId,
    fallbackRoot,
    isCancelled,
    resolveRoot,
    waitForSession,
}: BuildDocumentSymbolWaitersInput): Promise<DocumentSymbolSessionWaiter[]> => {
    const resolvedRoots = await Promise.all(availableServerIds.map((serverId) => resolveRoot({ serverId, filePath: path }).catch(() => null)))
    if (isCancelled()) return []

    return availableServerIds.flatMap((serverId, index) => {
        const root = resolvedRoots[index] ?? fallbackRoot
        return root ? [waitForSession(projectId, serverId, root)] : []
    })
}

export const OutlinePanelContainer: FC<OutlinePanelContainerProps> = ({ projectId }) => {
    const [symbolsForPath, setSymbolsForPath] = useState<SymbolsForPath | null>(null)

    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const activeTab = layout ? findActiveTab(layout.root, layout.focusedPane) : null
    const activePath = activeTab?.kind.kind === 'file' ? activeTab.kind.path : null

    const { data: file } = useQuery(fileQueryOptions(activePath))
    const { data: servers } = useQuery(lspServersQueryOptions())
    const languageId = file?.languageId ?? null

    const symbols = symbolsForPath?.path === activePath ? symbolsForPath.symbols : []

    useEffect(() => {
        if (!activePath || !languageId || !servers) return

        const availableServerIds = servers.filter((server) => server.languageIds.includes(languageId) && server.available).map((server) => server.id)
        if (availableServerIds.length === 0) return

        let cancelled = false
        let pendingCancels: (() => void)[] = []

        const load = async () => {
            const waiters = await buildDocumentSymbolWaiters({
                availableServerIds,
                path: activePath,
                projectId,
                fallbackRoot: project?.root,
                isCancelled: () => cancelled,
                resolveRoot: resolveLspRoot,
                waitForSession: waitForLspSessionForRoot,
            })
            pendingCancels = waiters.map((waiter) => waiter.cancel)

            for (const { promise } of waiters) {
                const session = await promise
                if (!session || cancelled) continue

                const ready = await session.ready.catch(() => null)
                if (!ready || cancelled) continue
                if (!ready.client.supports((capabilities) => isCapabilityEnabled(capabilities.documentSymbolProvider))) continue

                const uri = monaco.Uri.file(activePath).toString()
                const result = await requestDocumentSymbols(monaco, ready.client, uri).catch(() => [])
                if (!cancelled) {
                    setSymbolsForPath({ path: activePath, symbols: result })
                    return
                }
            }
            if (!cancelled) setSymbolsForPath({ path: activePath, symbols: [] })
        }

        void load()

        return () => {
            cancelled = true
            pendingCancels.forEach((cancel) => cancel())
        }
    }, [activePath, languageId, servers, projectId, project?.root])

    const handleSelectSymbol = (symbol: languages.DocumentSymbol) => {
        if (!activePath) return
        requestReveal(activePath, symbol.selectionRange.startLineNumber, symbol.selectionRange.startColumn)
    }

    return <OutlinePanel hasActiveFile={!!activePath} symbols={symbols} onSelectSymbol={handleSelectSymbol} />
}
