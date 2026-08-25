import { describe, expect, mock, test } from 'bun:test'

/**
 * `keybindings-runtime-provider.tsx` reaches `@shared/lib/monaco/monaco-keybinding-runtime`, which imports
 * `@shared/lib/monaco/setup` — real monaco-editor worker bundles (`?worker` imports) that only
 * Vite's dev/build pipeline can resolve, and that `bun test` cannot load at all. Stubbing
 * `@shared/lib/monaco/setup`, then reaching the module under test through a *dynamic* `import()`
 * (not a static import), is the same workaround `ide-sync-provider.test.ts` documents.
 */
const FAKE_MONACO = { editor: { addKeybindingRules: () => ({ dispose: () => {} }) } }

mock.module('@shared/lib/monaco/setup', () => ({ monaco: FAKE_MONACO }))

describe('KeybindingsRuntimeProvider 모듈 로드', () => {
    test('컴포넌트 함수로 export 된다', async () => {
        const imported = await import('@app/providers/keybindings-runtime-provider')
        expect(typeof imported.KeybindingsRuntimeProvider).toBe('function')
    })
})
