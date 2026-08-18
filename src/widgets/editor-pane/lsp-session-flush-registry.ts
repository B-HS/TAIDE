type LspSessionAllFlushFn = () => void

let flushAll: LspSessionAllFlushFn | null = null

/**
 * `lsp-session-registry.ts` calls this once, at module load, to hand this registry its
 * `flushAllLspSessionDisposals`. Exists so app-level providers that need to reach it —
 * `hot-exit-flush-provider.tsx` — don't have to statically import `lsp-session-registry.ts`
 * directly: that module pulls in real monaco worker bundles that only Vite's dev/build pipeline
 * can resolve, which breaks `bun test`'s static import-graph resolution for anything that imports
 * it (the same constraint `lsp-session-registry.test.ts` documents on its own `mock.module` setup).
 * `lsp-session-registry.ts` is always part of the app's initial bundle (a static dependency of
 * `editor-area.tsx`, mounted unconditionally at boot), so by the time a hot-exit could ever fire
 * it has already registered — {@link flushAllLspSessions} calling this before registration would
 * only ever happen if no LSP session could possibly exist yet either.
 */
export const registerLspSessionAllFlush = (handler: LspSessionAllFlushFn) => {
    flushAll = handler
}

export const flushAllLspSessions = () => {
    flushAll?.()
}
