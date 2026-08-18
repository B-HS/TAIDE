import { afterEach, beforeEach, describe, expect, test } from 'bun:test'
import { clearStaleWaitMarkersOnStartup, registerWaitMarker, takeWaitMarkers } from '@entities/agent/agent-wait-marker-registry'

const WAIT_MARKER_STORAGE_KEY = 'taide.agent.wait-markers'
const STARTUP_CLEANUP_DONE_KEY = 'taide.agent.wait-markers.startup-cleanup-done'

describe('agent-wait-marker-registry (localStorage 없는 환경 — 메모리 폴백)', () => {
    test('등록한 마커를 take 로 회수하고, 이후 재조회는 빈 배열을 반환한다', () => {
        registerWaitMarker('/a.ts', 'marker-1')
        expect(takeWaitMarkers('/a.ts')).toEqual(['marker-1'])
        expect(takeWaitMarkers('/a.ts')).toEqual([])
    })

    test('같은 경로에 여러 마커가 누적되면 전부 함께 반환한다', () => {
        registerWaitMarker('/b.ts', 'marker-1')
        registerWaitMarker('/b.ts', 'marker-2')
        expect(takeWaitMarkers('/b.ts')).toEqual(['marker-1', 'marker-2'])
    })

    test('등록된 적 없는 경로는 빈 배열을 반환한다', () => {
        expect(takeWaitMarkers('/never-registered.ts')).toEqual([])
    })

    test('서로 다른 경로의 마커는 독립적으로 관리된다', () => {
        registerWaitMarker('/c.ts', 'marker-c')
        registerWaitMarker('/d.ts', 'marker-d')
        expect(takeWaitMarkers('/c.ts')).toEqual(['marker-c'])
        expect(takeWaitMarkers('/d.ts')).toEqual(['marker-d'])
    })
})

/**
 * All TAIDE windows are the same origin (only the URL query string differs — see the registry's own
 * doc comment), so `localStorage` is what actually carries a marker registered in one window's JS
 * realm to another window's realm at tab-close time (contract #11 — a plain in-memory `Map` can't,
 * since each window has its own module instance). This block stubs a real `Storage` on `globalThis`
 * to exercise that branch directly, standing in for "a different window's realm reading the same
 * store" without needing two actual webviews.
 */
describe('agent-wait-marker-registry (localStorage 가용 환경 — 창 간 공유 시뮬레이션)', () => {
    const backingStore = new Map<string, string>()
    /**
     * Only `getItem`/`setItem` — the two members `agent-wait-marker-registry.ts` actually calls —
     * not a full `Storage` implementation (a `length` accessor would need `get length()`, which
     * `no-restricted-syntax`'s arrow-function-only rule rejects for a non-`MethodDefinition`
     * function).
     */
    const fakeLocalStorage = {
        getItem: (key: string) => backingStore.get(key) ?? null,
        setItem: (key: string, value: string) => {
            backingStore.set(key, value)
        },
    }

    beforeEach(() => {
        backingStore.clear()
        Object.defineProperty(globalThis, 'localStorage', { value: fakeLocalStorage, configurable: true, writable: true })
    })

    afterEach(() => {
        Reflect.deleteProperty(globalThis, 'localStorage')
    })

    test('한 realm 에서 등록한 마커를 다른 realm(같은 origin 의 다른 창)에서 회수할 수 있다', () => {
        registerWaitMarker('/shared.ts', 'marker-from-main-window')

        expect(takeWaitMarkers('/shared.ts')).toEqual(['marker-from-main-window'])
    })

    test('localStorage 에 경로→마커목록 JSON 으로 영속된다', () => {
        registerWaitMarker('/persist.ts', 'marker-x')

        const raw = fakeLocalStorage.getItem(WAIT_MARKER_STORAGE_KEY)
        expect(raw).not.toBeNull()
        expect(JSON.parse(raw ?? '{}')).toEqual({ '/persist.ts': ['marker-x'] })
    })

    test('손상된 JSON 이 저장돼 있어도 던지지 않고 빈 목록으로 취급한다', () => {
        fakeLocalStorage.setItem(WAIT_MARKER_STORAGE_KEY, '{not-json')

        expect(takeWaitMarkers('/whatever.ts')).toEqual([])
    })
})

describe('clearStaleWaitMarkersOnStartup — 진짜 재시작에서만 정리, reload 에서는 보존', () => {
    const localBackingStore = new Map<string, string>()
    const sessionBackingStore = new Map<string, string>()

    const fakeStorage = (backingStore: Map<string, string>) => ({
        getItem: (key: string) => backingStore.get(key) ?? null,
        setItem: (key: string, value: string) => {
            backingStore.set(key, value)
        },
    })

    beforeEach(() => {
        localBackingStore.clear()
        sessionBackingStore.clear()
        Object.defineProperty(globalThis, 'localStorage', { value: fakeStorage(localBackingStore), configurable: true, writable: true })
        Object.defineProperty(globalThis, 'sessionStorage', { value: fakeStorage(sessionBackingStore), configurable: true, writable: true })
    })

    afterEach(() => {
        Reflect.deleteProperty(globalThis, 'localStorage')
        Reflect.deleteProperty(globalThis, 'sessionStorage')
    })

    test('이전 실행에서 남은 마커를 앱 시작 시 한 번 비운다', () => {
        localBackingStore.set(WAIT_MARKER_STORAGE_KEY, JSON.stringify({ '/leftover.ts': ['stale-marker'] }))

        clearStaleWaitMarkersOnStartup()

        expect(takeWaitMarkers('/leftover.ts')).toEqual([])
        expect(sessionBackingStore.get(STARTUP_CLEANUP_DONE_KEY)).toBe('1')
    })

    test('같은 세션(reload)에서 다시 호출해도 그 사이 등록된 마커를 지우지 않는다', () => {
        clearStaleWaitMarkersOnStartup()
        registerWaitMarker('/still-open.ts', 'marker-after-cleanup')

        clearStaleWaitMarkersOnStartup()

        expect(takeWaitMarkers('/still-open.ts')).toEqual(['marker-after-cleanup'])
    })
})
