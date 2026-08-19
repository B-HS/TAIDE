import { describe, expect, test } from 'bun:test'

/**
 * F3#18 (`docs/acknowledge/2026-08-19-editor-pane-batch-contract.md` §1.2) replaced this hook's
 * internal `useSyncExternalStore` + `queryClient.getQueryCache().subscribe` pair with a standard
 * `useQuery({ queryKey: QUERY_KEY.SETTINGS.CURRENT, queryFn: skipToken, select })` — but this
 * project has no component/hook-render test harness (no `@testing-library/react`/DOM environment
 * configured for `bun:test`, same constraint `keybindings-editor.test.ts` documents), so the
 * reactive `select`/de-duplication behavior `useQuery` provides cannot be exercised by rendering
 * `useGlobalKeymap` here — it is a library-internal guarantee, not code this file owns. This stays
 * a load-only smoke test; `use-lsp-session.test.ts`'s `observeSemanticHighlightingSetting` covers
 * the sibling F3#18 replacement with a real assertion because that one is a plain
 * `QueryClient`-driven function reachable without rendering anything.
 */
describe('useGlobalKeymap 모듈 로드', () => {
    test('훅 함수로 export 된다', async () => {
        const imported = await import('@shared/hooks/use-global-keymap')
        expect(typeof imported.useGlobalKeymap).toBe('function')
    })
})
