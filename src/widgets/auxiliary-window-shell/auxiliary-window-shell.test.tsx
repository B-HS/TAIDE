import { afterEach, beforeEach, describe, expect, mock, test } from 'bun:test'
import * as sonner from 'sonner'
import { clearKeymapChordState } from '@shared/lib/keymap/keymap-chord-store'
import { renderWithProviders } from '@shared/testing/render'

/**
 * `⌘P`/`⌘⇧P` in an auxiliary window used to be silent: `App` deliberately keeps `CommandPalette` out
 * of that branch, so the keys reached no handler at all. The shell now claims both keymap ids and
 * says why, which is what these cases lock.
 *
 * Three module fakes, all registered before the shell is pulled in through a *dynamic* `import()`
 * (`mock.module` is process-global and last-registration-wins — `docs/memory/test-conventions.md`
 * §3): the real `sonner` namespace is snapshotted *before* registration and spread into the
 * replacement so no export disappears for whatever file runs next, `EditorArea` is stubbed because
 * this window's editor body has nothing to do with the shortcut, and `@shared/lib/monaco/setup`
 * follows the existing precedent — `entities/layout/layout.query` reaches monaco at import time.
 *
 * `@tauri-apps/api/window` is deliberately *not* mocked. The shell closes its own window once the
 * tree empties (and `layout_get` fails here, since there is no IPC), which needs `getCurrentWindow()`
 * to resolve — so this file seeds the same `window.__TAURI_INTERNALS__` shape
 * (`docs/memory/test-conventions.md` §4) and removes it afterwards, instead of replacing the module
 * process-wide with a fake window that has no `label` and would make every later file's
 * `isRemoteMirrorRuntime` read the wrong runtime.
 */
const realSonner = { ...sonner }

const MAIN_WINDOW_LABEL = 'main'

const infoMessages: string[] = []
const ignoreToast = () => undefined
const toastFake = Object.assign(ignoreToast, {
    ...realSonner.toast,
    info: (message: string) => {
        infoMessages.push(String(message))
    },
})

mock.module('sonner', () => ({ ...realSonner, toast: toastFake }))
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))
mock.module('@widgets/editor-area/editor-area', () => ({ EditorArea: () => null }))

const importShell = () => import('@widgets/auxiliary-window-shell/auxiliary-window-shell')

const PROJECT_ID = 'project-1'
const WINDOW_SLOT = 1

const renderShell = async () => {
    const { AuxiliaryWindowShell } = await importShell()
    return renderWithProviders(<AuxiliaryWindowShell projectId={PROJECT_ID} windowSlot={WINDOW_SLOT} />)
}

const pressKey = (init: KeyboardEventInit) => window.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }))

describe('AuxiliaryWindowShell 팔레트 단축키 안내', () => {
    beforeEach(() => {
        infoMessages.length = 0
        clearKeymapChordState()
        window.__TAURI_INTERNALS__ = { metadata: { currentWindow: { label: MAIN_WINDOW_LABEL } } }
    })

    afterEach(() => {
        window.__TAURI_INTERNALS__ = undefined
    })

    test('⌘P 는 메인 창 전용이라는 안내 토스트를 띄운다', async () => {
        await renderShell()

        pressKey({ key: 'p', code: 'KeyP', metaKey: true })

        expect(infoMessages).toEqual(['palette.mainWindowOnly'])
    })

    test('⌘⇧P 도 같은 안내를 띄운다', async () => {
        await renderShell()

        pressKey({ key: 'p', code: 'KeyP', metaKey: true, shiftKey: true })

        expect(infoMessages).toEqual(['palette.mainWindowOnly'])
    })

    test('팔레트와 무관한 키는 안내를 띄우지 않는다', async () => {
        await renderShell()

        pressKey({ key: 'j', code: 'KeyJ', metaKey: true })

        expect(infoMessages).toEqual([])
    })
})
