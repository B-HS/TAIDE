import { describe, expect, test } from 'bun:test'
import { createCallbackRegistry } from '@shared/lib/remote/callback-registry'

/**
 * The remote page's stand-in for Tauri's `transformCallback` bookkeeping: `tauri-internals-shim.ts`
 * hands ids to the backend, which sends them back on a websocket frame. Two properties carry the
 * whole contract — an id is never reused (a late frame for a released callback must find nothing,
 * not somebody else's handler) and a `once` callback is unregistered by its own first run.
 */
describe('createCallbackRegistry', () => {
    test('id 는 1 부터 시작해 등록마다 증가하고 해제해도 재사용되지 않는다', () => {
        const registry = createCallbackRegistry()
        const noop = () => undefined

        const first = registry.transformCallback(noop)
        const second = registry.transformCallback(noop)
        registry.unregisterCallback(first)
        const third = registry.transformCallback(noop)

        expect(first).toBe(1)
        expect(second).toBe(2)
        expect(third).toBe(3)
    })

    test('runCallback 이 등록된 id 의 콜백에만 payload 를 전달한다', () => {
        const registry = createCallbackRegistry()
        const received: unknown[] = []
        const otherReceived: unknown[] = []

        const id = registry.transformCallback((payload) => received.push(payload))
        registry.transformCallback((payload) => otherReceived.push(payload))
        registry.runCallback(id, { event: 'fs:changed' })

        expect(received).toEqual([{ event: 'fs:changed' }])
        expect(otherReceived).toEqual([])
    })

    test('등록된 적 없는 id 로 실행해도 예외 없이 무시한다', () => {
        const registry = createCallbackRegistry()

        expect(() => registry.runCallback(42, 'payload')).not.toThrow()
    })

    test('once 콜백은 첫 실행 뒤 스스로 해제되어 두 번째 프레임을 받지 않는다', () => {
        const registry = createCallbackRegistry()
        const received: unknown[] = []

        const id = registry.transformCallback((payload) => received.push(payload), true)
        registry.runCallback(id, 'first')
        registry.runCallback(id, 'second')

        expect(received).toEqual(['first'])
        expect(registry.callbacks[id]).toBeUndefined()
    })

    test('once 가 아닌 콜백은 여러 번 실행해도 등록 상태를 유지한다', () => {
        const registry = createCallbackRegistry()
        const received: unknown[] = []

        const id = registry.transformCallback((payload) => received.push(payload))
        registry.runCallback(id, 'first')
        registry.runCallback(id, 'second')

        expect(received).toEqual(['first', 'second'])
        expect(typeof registry.callbacks[id]).toBe('function')
    })

    test('unregisterCallback 이후에는 같은 id 로 실행해도 호출되지 않는다', () => {
        const registry = createCallbackRegistry()
        const received: unknown[] = []

        const id = registry.transformCallback((payload) => received.push(payload))
        registry.unregisterCallback(id)
        registry.runCallback(id, 'after-release')

        expect(received).toEqual([])
        expect(registry.callbacks[id]).toBeUndefined()
    })

    test('once 콜백을 실행 전에 해제하면 once 기록도 함께 사라진다 (해제 후 재실행 무해)', () => {
        const registry = createCallbackRegistry()
        const received: unknown[] = []

        const id = registry.transformCallback((payload) => received.push(payload), true)
        registry.unregisterCallback(id)

        expect(() => registry.runCallback(id, 'ignored')).not.toThrow()
        expect(received).toEqual([])
    })

    test('레지스트리 인스턴스끼리 id 공간과 콜백을 공유하지 않는다', () => {
        const first = createCallbackRegistry()
        const second = createCallbackRegistry()
        const firstReceived: unknown[] = []

        const id = first.transformCallback((payload) => firstReceived.push(payload))
        second.runCallback(id, 'wrong-registry')

        expect(firstReceived).toEqual([])
        expect(second.callbacks[id]).toBeUndefined()
    })

    test('callbacks 는 현재 등록된 항목을 그대로 노출한다 (shim 이 id 존재 여부를 읽는 통로)', () => {
        const registry = createCallbackRegistry()
        const noop = () => undefined

        const first = registry.transformCallback(noop)
        const second = registry.transformCallback(noop)

        expect(Object.keys(registry.callbacks)).toEqual([String(first), String(second)])
    })
})
