import type { FC } from 'react'
import { useEffect, useState } from 'react'
import type { languages } from 'monaco-editor'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { findActiveTab } from '@shared/lib/pane-tree'
import { requestDocumentSymbols } from '@shared/lib/lsp/adapters/document-symbol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import { fileQueryOptions } from '@entities/file/file.query'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { requestReveal } from '@entities/editor/reveal-registry'
import { waitForLspSession } from '@widgets/editor-pane/lsp-session-registry'
import { OutlinePanel } from '@widgets/outline-panel/outline-panel'

type OutlinePanelContainerProps = {
    projectId: ProjectId
}

type SymbolsForPath = { path: string; symbols: languages.DocumentSymbol[] }

export const OutlinePanelContainer: FC<OutlinePanelContainerProps> = ({ projectId }) => {
    const [symbolsForPath, setSymbolsForPath] = useState<SymbolsForPath | null>(null)

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
        const waiters = availableServerIds.map((serverId) => waitForLspSession(projectId, serverId))

        const load = async () => {
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
            waiters.forEach(({ cancel }) => cancel())
        }
    }, [activePath, languageId, servers, projectId])

    const handleSelectSymbol = (symbol: languages.DocumentSymbol) => {
        if (!activePath) return
        requestReveal(activePath, symbol.selectionRange.startLineNumber, symbol.selectionRange.startColumn)
    }

    return <OutlinePanel hasActiveFile={!!activePath} symbols={symbols} onSelectSymbol={handleSelectSymbol} />
}
