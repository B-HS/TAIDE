import type { LspServerDetection } from '@shared/api/bindings'

/**
 * The attach-eligibility predicate every LSP server picker applies — language match plus
 * `available` — kept in one place so `use-lsp-session.ts`'s own attach gate and every "which
 * server(s) should I ask" call site (Code Actions on Save, breadcrumbs/outline/command-palette
 * document-symbol lookups) can't drift out of lockstep with each other.
 */
export const filterAvailableLspServers = <T extends Pick<LspServerDetection, 'languageIds' | 'available'>>(servers: T[], languageId: string) =>
    servers.filter((server) => server.languageIds.includes(languageId) && server.available)
