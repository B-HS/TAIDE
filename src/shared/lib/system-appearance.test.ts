import { afterEach, describe, expect, test } from 'bun:test'
import { readSystemTheme, subscribeSystemTheme } from '@shared/lib/system-appearance'

/**
 * `followSystemTheme` reads the OS appearance through `matchMedia` alone, so both halves of the
 * guard matter: a webview that exposes no `matchMedia` (the remote page on an old browser) must
 * fall back to dark and hand back a no-op unsubscribe rather than throw inside a
 * `useSyncExternalStore` subscribe — React would then have no way to unsubscribe on unmount.
 *
 * happy-dom does implement `matchMedia`, but it answers `prefers-color-scheme` from its own window
 * settings; the media query object is replaced here so each test states the OS answer it is about
 * rather than depending on the harness default.
 */
type MediaQueryListener = (event: unknown) => void

const originalMatchMedia = window.matchMedia

const installMatchMedia = (matches: boolean) => {
    const queries: string[] = []
    const listeners: { type: string; listener: MediaQueryListener }[] = []
    const removed: { type: string; listener: MediaQueryListener }[] = []

    const fakeMatchMedia = (query: string) => {
        queries.push(query)
        return {
            matches,
            media: query,
            addEventListener: (type: string, listener: MediaQueryListener) => listeners.push({ type, listener }),
            removeEventListener: (type: string, listener: MediaQueryListener) => removed.push({ type, listener }),
        }
    }

    Object.defineProperty(window, 'matchMedia', { value: fakeMatchMedia, configurable: true, writable: true })
    return { queries, listeners, removed }
}

const removeMatchMedia = () => {
    Object.defineProperty(window, 'matchMedia', { value: undefined, configurable: true, writable: true })
}

afterEach(() => {
    Object.defineProperty(window, 'matchMedia', { value: originalMatchMedia, configurable: true, writable: true })
})

describe('readSystemTheme', () => {
    test('prefers-color-scheme: dark 가 일치하면 dark 를 돌려준다', () => {
        const stub = installMatchMedia(true)

        expect(readSystemTheme()).toBe('dark')
        expect(stub.queries).toEqual(['(prefers-color-scheme: dark)'])
    })

    test('일치하지 않으면 light 를 돌려준다', () => {
        installMatchMedia(false)

        expect(readSystemTheme()).toBe('light')
    })

    test('matchMedia 가 없는 환경에서는 dark 로 폴백한다', () => {
        removeMatchMedia()

        expect(readSystemTheme()).toBe('dark')
    })
})

describe('subscribeSystemTheme', () => {
    test('change 이벤트로 구독하고 반환된 해제 함수가 같은 리스너를 제거한다', () => {
        const stub = installMatchMedia(true)
        const onChange = () => undefined

        const unsubscribe = subscribeSystemTheme(onChange)

        expect(stub.listeners).toEqual([{ type: 'change', listener: onChange }])
        expect(stub.removed).toEqual([])

        unsubscribe()

        expect(stub.removed).toEqual([{ type: 'change', listener: onChange }])
    })

    test('matchMedia 가 없으면 구독하지 않고 호출 가능한 해제 함수를 돌려준다', () => {
        removeMatchMedia()

        const unsubscribe = subscribeSystemTheme(() => undefined)

        expect(typeof unsubscribe).toBe('function')
        expect(unsubscribe()).toBeUndefined()
    })
})
