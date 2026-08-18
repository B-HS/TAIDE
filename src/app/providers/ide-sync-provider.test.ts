import { describe, expect, mock, test } from 'bun:test'

/**
 * `ide-sync-provider.tsx` reaches `@entities/editor/model-registry` (for `getModel`),
 * `@shared/hooks/use-monaco-markers`, and `@features/problems/problem-severity` — all three pull in
 * `@shared/lib/monaco/setup`, which imports real monaco-editor worker bundles (`?worker` imports)
 * that only Vite's dev/build pipeline can resolve. `bun test` cannot load them at all (same
 * constraint `lsp-session-registry.test.ts` documents on its own `mock.module` setup: importing a
 * module that reaches `monaco/setup` fails with "Missing 'default' export ... ts.worker.js?worker"
 * before any test code runs). Stubbing `@shared/lib/monaco/setup`, then reaching the module under
 * test through a *dynamic* `import()` (not a static import — Bun resolves the whole static import
 * graph, including the offending worker files, before a same-file `mock.module` call would ever
 * run) is what makes this file able to load `ide-sync-provider.tsx` at all.
 */
const FAKE_MONACO = {
    Uri: {
        file: (path: string) => ({ toString: () => `file://${path}` }),
        parse: (value: string) => ({ toString: () => value }),
    },
    editor: {
        getModelMarkers: () => [],
        onDidChangeMarkers: () => {},
    },
}

mock.module('@shared/lib/monaco/setup', () => ({ monaco: FAKE_MONACO }))

describe('IdeSyncProvider 모듈 로드', () => {
    test('monaco worker 를 정적 임포트 그래프에서 우회해 로드된다', async () => {
        const imported = await import('@app/providers/ide-sync-provider')
        expect(typeof imported.IdeSyncProvider).toBe('function')
    })
})
