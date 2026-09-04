import { afterEach, describe, expect, mock, test } from 'bun:test'
import * as tauriWindow from '@tauri-apps/api/window'
import type { ThemeType } from '@shared/api/bindings'

/**
 * `applyWindowAppearance` is a one-line forwarder, and the two things worth locking are exactly the
 * two the caller depends on (contract d-45 §1#1·F-02): the value reaches tao's `set_theme`
 * untouched, and the *same* promise comes back so a caller that optimistically recorded the type as
 * applied can roll that record back when the native call rejects.
 *
 * `getCurrentWindow` delegates to the real export unless a test swaps `getCurrentWindowImpl`.
 * `mock.module` is process-global and last-registration-wins (`docs/memory/test-conventions.md` §3),
 * and `entities/notification/notify.test.ts` reads a real `getCurrentWindow().label` out of the
 * `window.__TAURI_INTERNALS__` it plants — a fake that stayed installed for the rest of the run
 * would silently answer that file's window-kind gate instead.
 */
type FakeWindow = { setTheme: (type?: ThemeType | null) => Promise<void> }

/**
 * Captured *before* `mock.module` replaces the module, because the namespace binding is live: after
 * the mock is installed, `tauriWindow.getCurrentWindow` resolves to the fake below, and restoring
 * from it would make the fake call itself forever — a spin that hangs whichever later test file
 * next asks for the current window.
 */
const realGetCurrentWindow = tauriWindow.getCurrentWindow as unknown as () => FakeWindow

const getCurrentWindowImpl = { current: realGetCurrentWindow }

mock.module('@tauri-apps/api/window', () => ({ ...tauriWindow, getCurrentWindow: () => getCurrentWindowImpl.current() }))

const importWindowAppearance = () => import('@shared/lib/window-appearance')

const installFakeWindow = (setTheme: FakeWindow['setTheme']) => {
    const calls: (ThemeType | null | undefined)[] = []
    getCurrentWindowImpl.current = () => ({
        setTheme: (type) => {
            calls.push(type)
            return setTheme(type)
        },
    })
    return calls
}

afterEach(() => {
    getCurrentWindowImpl.current = realGetCurrentWindow
})

describe('applyWindowAppearance', () => {
    test('전달받은 ThemeType 을 그대로 이 창의 setTheme 에 넘긴다', async () => {
        const { applyWindowAppearance } = await importWindowAppearance()
        const calls = installFakeWindow(() => Promise.resolve())

        await applyWindowAppearance('dark')
        await applyWindowAppearance('light')

        expect(calls).toEqual(['dark', 'light'])
    })

    test('호출마다 현재 창을 새로 조회한다 (창 핸들을 모듈에 캐시하지 않는다)', async () => {
        const { applyWindowAppearance } = await importWindowAppearance()
        let lookups = 0
        getCurrentWindowImpl.current = () => {
            lookups += 1
            return { setTheme: () => Promise.resolve() }
        }

        await applyWindowAppearance('dark')
        await applyWindowAppearance('dark')

        expect(lookups).toBe(2)
    })

    test('setTheme 의 promise 를 그대로 돌려준다 (성공)', async () => {
        const { applyWindowAppearance } = await importWindowAppearance()
        const pending = Promise.resolve()
        installFakeWindow(() => pending)

        expect(applyWindowAppearance('dark')).toBe(pending)
        await pending
    })

    test('네이티브 호출이 거부하면 그 거부가 호출부까지 전파된다 (낙관적 기록 롤백 경로)', async () => {
        const { applyWindowAppearance } = await importWindowAppearance()
        const failure = new Error('window not ready')
        installFakeWindow(() => Promise.reject(failure))

        await expect(applyWindowAppearance('light')).rejects.toBe(failure)
    })
})
