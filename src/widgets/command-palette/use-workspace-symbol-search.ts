import { useEffect } from 'react'
import type { NormalizedWorkspaceSymbol, WorkspaceSymbolSearch } from '@shared/lib/lsp/adapters/workspace-symbol'
import type { PaletteMode } from '@shared/lib/command-palette-query'
import { listSessionRecordsForProject } from '@entities/lsp/lsp-session-registry'

export type WorkspaceSymbolState = { query: string; results: NormalizedWorkspaceSymbol[] }

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
        let cancelled = false

        const load = async () => {
            if (!trimmedQuery) {
                onResult({ query: searchTerm, results: [] })
                return
            }
            const sessionRecords = listSessionRecordsForProject(activeProjectId)
            const readySessions = await Promise.all(sessionRecords.map((record) => record.ready.catch(() => null)))
            if (cancelled) return
            const clients = readySessions.filter((session) => session !== null).map((session) => session.client)
            const results = await workspaceSymbolSearch.search(clients, trimmedQuery)
            if (!cancelled) onResult({ query: searchTerm, results })
        }

        void load()

        return () => {
            cancelled = true
            workspaceSymbolSearch.cancel()
        }
    }, [mode, open, activeProjectId, searchTerm, workspaceSymbolSearch, onResult])
}
