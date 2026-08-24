import type { FC } from 'react'
import { useEffect, useState } from 'react'
import type { languages } from 'monaco-editor'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { monaco } from '@shared/lib/monaco/setup'
import { findActiveTab } from '@shared/lib/pane-tree'
import { loadDocumentSymbolsForPath } from '@shared/lib/lsp/document-symbol-session-waiters'
import { fileQueryOptions } from '@entities/file/file.query'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { resolveLspRoot } from '@entities/lsp/lsp.ipc'
import { filterAvailableLspServers } from '@entities/lsp/lsp.constant'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { requestReveal } from '@entities/editor/reveal-registry'
import { waitForLspSessionForRoot } from '@entities/lsp/lsp-session-registry'
import { OutlinePanel } from '@features/outline/outline-panel'

type OutlinePanelContainerProps = {
    projectId: ProjectId
}

type SymbolsForPath = { path: string; symbols: languages.DocumentSymbol[] }

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

        const availableServerIds = filterAvailableLspServers(servers, languageId).map((server) => server.id)
        if (availableServerIds.length === 0) return

        return loadDocumentSymbolsForPath({
            monaco,
            availableServerIds,
            path: activePath,
            projectId,
            fallbackRoot: project?.root,
            resolveRoot: resolveLspRoot,
            waitForSession: waitForLspSessionForRoot,
            onLoaded: (symbols) => setSymbolsForPath({ path: activePath, symbols }),
        })
    }, [activePath, languageId, servers, projectId, project?.root])

    const handleSelectSymbol = (symbol: languages.DocumentSymbol) => {
        if (!activePath) return
        requestReveal(activePath, symbol.selectionRange.startLineNumber, symbol.selectionRange.startColumn)
    }

    return <OutlinePanel hasActiveFile={!!activePath} symbols={symbols} onSelectSymbol={handleSelectSymbol} />
}
