import { useEffect } from 'react'
import type { NormalizedWorkspaceSymbol, WorkspaceSymbolSearch } from '@shared/lib/lsp/adapters/workspace-symbol'
import type { PaletteMode } from '@shared/lib/command-palette-query'
import { listSessionRecordsForProject } from '@entities/lsp/lsp-session-registry'

export type WorkspaceSymbolState = { query: string; results: NormalizedWorkspaceSymbol[] }

/**
 * A workspace-symbol query fans out to *every* ready LSP session in the project, and each server
 * answers by walking its whole index — the most expensive request the palette can issue. Undebounced,
 * typing `handleSave` fired ten of them, nine of which were superseded before their results could
 * ever be shown (audit §1-11). Trailing-only: no intermediate prefix is worth asking about, and the
 * cleanup below already cancels the in-flight request of whichever keystroke did get through.
 */
const WORKSPACE_SYMBOL_SEARCH_DEBOUNCE_MS = 200

type UseWorkspaceSymbolSearchParams = {
    mode: PaletteMode
    open: boolean
    activeProjectId: string | null
    searchTerm: string
    workspaceSymbolSearch: WorkspaceSymbolSearch
    onResult: (state: WorkspaceSymbolState) => void
}

export const useWorkspaceSymbolSearch = ({
    mode,
    open,
    activeProjectId,
    searchTerm,
    workspaceSymbolSearch,
    onResult,
}: UseWorkspaceSymbolSearchParams) => {
    useEffect(() => {
        if (mode !== 'workspaceSymbol' || !open || !activeProjectId) return

        const trimmedQuery = searchTerm.trim()
        if (!trimmedQuery) {
            onResult({ query: searchTerm, results: [] })
            return
        }

        let cancelled = false

        const load = async () => {
            const sessionRecords = listSessionRecordsForProject(activeProjectId)
            const readySessions = await Promise.all(sessionRecords.map((record) => record.ready.catch(() => null)))
            if (cancelled) return
            const clients = readySessions.filter((session) => session !== null).map((session) => session.client)
            const results = await workspaceSymbolSearch.search(clients, trimmedQuery)
            if (!cancelled) onResult({ query: searchTerm, results })
        }

        const timerId = setTimeout(() => void load(), WORKSPACE_SYMBOL_SEARCH_DEBOUNCE_MS)

        return () => {
            cancelled = true
            clearTimeout(timerId)
            workspaceSymbolSearch.cancel()
        }
    }, [mode, open, activeProjectId, searchTerm, workspaceSymbolSearch, onResult])
}
