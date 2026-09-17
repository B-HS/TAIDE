import { afterEach, describe, expect, mock, spyOn, test } from 'bun:test'
import type { Settings } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, renderWithProviders } from '@shared/testing/render'

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

const SEEDED_EDITOR_FONT_SIZE = 20
const INCREASED_EDITOR_FONT_SIZE = 21
const DECREASED_EDITOR_FONT_SIZE = 19

const MAIN_WINDOW_URL = '/'
const AUXILIARY_WINDOW_URL = '/?projectId=project-1&windowSlot=1'

/** See `pane-tree.test.ts` — the harness pins `location.search` to empty, and this is the one same-document way to move it. */
const enterWindowUrl = (url: string) => window.history.replaceState({}, '', url)

afterEach(() => enterWindowUrl(MAIN_WINDOW_URL))

const pressKey = (init: KeyboardEventInit) => window.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }))

const FONT_SIZE_UP_KEY: KeyboardEventInit = { key: '=', code: 'Equal', metaKey: true }
const FONT_SIZE_DOWN_KEY: KeyboardEventInit = { key: '-', code: 'Minus', metaKey: true }

/**
 * The settings cache is seeded rather than fetched (no IPC in the harness), and the `Settings` shape
 * is narrowed to the one field this provider reads — the same partial-seed shape
 * `pane-node-view-welcome.test.tsx` uses.
 */
const renderProvider = async () => {
    const queryClient = createTestQueryClient()
    queryClient.setQueryData<Partial<Settings>>(QUERY_KEY.SETTINGS.CURRENT, { editorFontSize: SEEDED_EDITOR_FONT_SIZE })
    const { KeybindingsRuntimeProvider } = await import('@app/providers/keybindings-runtime-provider')
    const rendered = renderWithProviders(<KeybindingsRuntimeProvider />, { queryClient })
    await act(async () => undefined)
    return rendered
}

const patchedEditorFontSizes = async (key: KeyboardEventInit) => {
    const settingsUpdate = spyOn(commands, 'settingsUpdate')
    await renderProvider()

    await act(async () => {
        pressKey(key)
    })
    return settingsUpdate.mock.calls.map(([patch]) => patch.editorFontSize)
}

/**
 * ⌘=/⌘− had a single owner, `StatusBarContent`, which only the main window mounts — so an auxiliary
 * editor window could not change its font size by key or by button (audit #14/#15). The provider is
 * mounted in both of `app.tsx`'s window branches, which makes it the one place the keymap entry can
 * live without either window going without it or the main window registering it twice.
 */
describe('KeybindingsRuntimeProvider 폰트 크기 단축키', () => {
    test('main 창에서 ⌘= 가 에디터 폰트 크기를 한 단계 올린다 (정확히 한 번)', async () => {
        expect(await patchedEditorFontSizes(FONT_SIZE_UP_KEY)).toEqual([INCREASED_EDITOR_FONT_SIZE])
    })

    test('보조 창에서도 ⌘= 가 동작한다', async () => {
        enterWindowUrl(AUXILIARY_WINDOW_URL)

        expect(await patchedEditorFontSizes(FONT_SIZE_UP_KEY)).toEqual([INCREASED_EDITOR_FONT_SIZE])
    })

    test('보조 창에서도 ⌘− 가 동작한다', async () => {
        enterWindowUrl(AUXILIARY_WINDOW_URL)

        expect(await patchedEditorFontSizes(FONT_SIZE_DOWN_KEY)).toEqual([DECREASED_EDITOR_FONT_SIZE])
    })
})
