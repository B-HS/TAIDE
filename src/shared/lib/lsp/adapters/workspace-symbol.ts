import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { Location, SymbolInformation, WorkspaceSymbol } from '@shared/lib/lsp/protocol'
import { lspPositionToMonaco } from '@shared/lib/lsp/position'

export const WORKSPACE_SYMBOL_SEARCH_DEBOUNCE_MS = 200

export type NormalizedWorkspaceSymbol = {
    name: string
    kind: number
    containerName?: string
    path: string
    line: number
    column: number
}

const hasResolvedLocation = (location: WorkspaceSymbol['location']): location is Location => 'range' in location

const toNormalizedWorkspaceSymbol = (monaco: Monaco, symbol: SymbolInformation | WorkspaceSymbol): NormalizedWorkspaceSymbol | null => {
    if (!hasResolvedLocation(symbol.location)) return null
    const uri = monaco.Uri.parse(symbol.location.uri)
    if (uri.scheme !== 'file') return null
    const position = lspPositionToMonaco(symbol.location.range.start)
    return {
        name: symbol.name,
        kind: symbol.kind,
        containerName: 'containerName' in symbol ? symbol.containerName : undefined,
        path: uri.fsPath,
        line: position.lineNumber,
        column: position.column,
    }
}

export const mergeWorkspaceSymbolResults = (
    monaco: Monaco,
    settled: readonly PromiseSettledResult<(SymbolInformation | WorkspaceSymbol)[] | null>[],
): NormalizedWorkspaceSymbol[] =>
    settled
        .flatMap((result) => (result.status === 'fulfilled' ? (result.value ?? []) : []))
        .map((symbol) => toNormalizedWorkspaceSymbol(monaco, symbol))
        .filter((symbol): symbol is NormalizedWorkspaceSymbol => symbol !== null)

/**
 * Queries `workspace/symbol` on every given session in parallel and merges the results —
 * `Promise.allSettled` means a session whose server doesn't advertise `workspaceSymbolProvider`
 * (rejected by `client.ts`'s `FEATURE_CAPABILITY_CHECKS`) or that errors out simply contributes no
 * results instead of failing the whole search, since a multi-language project's other sessions may
 * still answer.
 */
export const requestWorkspaceSymbols = async (monaco: Monaco, clients: readonly LspClient[], query: string): Promise<NormalizedWorkspaceSymbol[]> => {
    const settled = await Promise.allSettled(
        clients.map((client) => client.request<(SymbolInformation | WorkspaceSymbol)[] | null>('workspace/symbol', { query })),
    )
    return mergeWorkspaceSymbolResults(monaco, settled)
}

export type WorkspaceSymbolSearch = {
    search: (clients: readonly LspClient[], query: string) => Promise<NormalizedWorkspaceSymbol[]>
    cancel: () => void
}

/**
 * Debounces `requestWorkspaceSymbols` behind a single trailing timer per instance — mirrors
 * `adapters/code-lens.ts`'s `scheduleRefresh` timer shape. `cancel` (called from the palette's
 * effect cleanup on every keystroke/unmount) clears a still-pending timer so a superseded keystroke
 * never fires its own round trip to every session.
 */
export const createWorkspaceSymbolSearch = (monaco: Monaco, delayMs: number = WORKSPACE_SYMBOL_SEARCH_DEBOUNCE_MS): WorkspaceSymbolSearch => {
    let timer: ReturnType<typeof setTimeout> | null = null

    const cancel = () => {
        if (timer !== null) clearTimeout(timer)
        timer = null
    }

    const search = (clients: readonly LspClient[], query: string) =>
        new Promise<NormalizedWorkspaceSymbol[]>((resolve, reject) => {
            cancel()
            timer = setTimeout(() => {
                timer = null
                requestWorkspaceSymbols(monaco, clients, query).then(resolve, reject)
            }, delayMs)
        })

    return { search, cancel }
}
