import { describe, expect, mock, test } from 'bun:test'

/**
 * `claude-diff-pane.tsx` imports `@shared/lib/monaco/setup`, which pulls in real monaco-editor
 * worker bundles (`?worker` imports) that only Vite's dev/build pipeline can resolve — `bun test`
 * cannot load them at all. Stubbing `@shared/lib/monaco/setup`, then reaching the module under test
 * through a *dynamic* `import()` (not a static import) is the same workaround
 * `ide-sync-provider.test.ts` documents. This project has no component-rendering test harness
 * (no `@testing-library/react`/DOM environment configured for `bun:test`), so this is a load-only
 * smoke test — the tab-close-vs-tab-switch distinction in the unmount effect (see that effect's own
 * doc comment) is verified by the pure `takePendingClaudeDiffIfUnresolved` behavior covered in
 * `claude-diff-registry.test.ts`, plus manual/QA verification of the actual close flow.
 */
const FAKE_MONACO = {
    editor: {
        createDiffEditor: () => ({ setModel: () => {}, dispose: () => {} }),
        createModel: () => ({ getValue: () => '', setValue: () => {}, dispose: () => {} }),
        setModelLanguage: () => {},
    },
}

mock.module('@shared/lib/monaco/setup', () => ({ monaco: FAKE_MONACO }))

describe('ClaudeDiffPane 모듈 로드', () => {
    test('컴포넌트 함수로 export 된다', async () => {
        const imported = await import('@widgets/claude-diff-pane/claude-diff-pane')
        expect(typeof imported.ClaudeDiffPane).toBe('function')
    })
})
