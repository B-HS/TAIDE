import { describe, expect, test } from 'bun:test'

/**
 * `ipc-sync-provider.tsx` must reach LSP session disposal through `lsp-session-flush-registry.ts`
 * rather than importing `lsp-session-registry.ts` directly — that module pulls in real monaco
 * worker bundles (`?worker` imports) that only Vite's dev/build pipeline can resolve, which makes
 * `bun test`'s static import-graph resolution fail for anything that imports it, this provider
 * included. A dynamic `import()` here is enough to catch a regression back to the direct import:
 * Bun resolves the whole static import graph up front, so a `Missing 'default' export ...
 * ts.worker.js?worker'` `SyntaxError` surfaces at import time, before any test body runs.
 */
describe('IpcSyncProvider 모듈 로드', () => {
    test('lsp-session-flush-registry 간접 참조라 monaco worker 를 정적 임포트 그래프에 끌어들이지 않는다', async () => {
        const imported = await import('@app/providers/ipc-sync-provider')
        expect(typeof imported.IpcSyncProvider).toBe('function')
    })
})
