import { useEffect } from 'react'
import type { languages } from 'monaco-editor'
import type { LspServerDetection, OpenedFile } from '@shared/api/bindings'
import type { PaletteMode } from '@shared/lib/command-palette-query'
import { loadDocumentSymbolsForPath } from '@shared/lib/lsp/document-symbol-session-waiters'
import { monaco } from '@shared/lib/monaco/setup'
import { resolveLspRoot } from '@entities/lsp/lsp.ipc'
import { filterAvailableLspServers } from '@entities/lsp/lsp.constant'
import { waitForLspSessionForRoot } from '@entities/lsp/lsp-session-registry'

export type DocumentSymbolState = { path: string; symbols: languages.DocumentSymbol[] }

type UseDocumentSymbolLoaderParams = {
    mode: PaletteMode
    open: boolean
    activeProjectId: string | null
    activePath: string | null
    activeFile: OpenedFile | undefined
    lspServers: LspServerDetection[] | undefined
    activeProjectRoot: string | undefined
    onLoaded: (state: DocumentSymbolState) => void
}

/**
 * `⌘O`/symbol-nav mode's document-symbol lookup — root-aware (`loadDocumentSymbolsForPath` +
 * `waitForLspSessionForRoot`, contract `docs/acknowledge/2026-08-19-editor-pane-batch-contract.md`
 * §1.2) because it has a concrete `activePath` to resolve a root from. Contrast the `⌘T`
 * Workspace Symbol effect in `use-workspace-symbol-search.ts`, which stays root-agnostic
 * (`listSessionRecordsForProject`) on purpose: it has no single document path to resolve a root
 * against (it searches every root the project has open at once), so there is no "wrong root" to
 * pick.
 */
export const useDocumentSymbolLoader = ({
    mode,
    open,
    activeProjectId,
    activePath,
    activeFile,
    lspServers,
    activeProjectRoot,
    onLoaded,
}: UseDocumentSymbolLoaderParams) => {
    useEffect(() => {
        if (mode !== 'symbol' || !open || !activeProjectId || !activePath || !activeFile || !lspServers) return

        const languageId = activeFile.languageId
        const availableServerIds = filterAvailableLspServers(lspServers, languageId).map((server) => server.id)

        return loadDocumentSymbolsForPath({
            monaco,
            availableServerIds,
            path: activePath,
            projectId: activeProjectId,
            fallbackRoot: activeProjectRoot,
            resolveRoot: resolveLspRoot,
            waitForSession: waitForLspSessionForRoot,
            onLoaded: (symbols) => onLoaded({ path: activePath, symbols }),
        })
    }, [mode, open, activeProjectId, activePath, activeFile, lspServers, activeProjectRoot, onLoaded])
}
