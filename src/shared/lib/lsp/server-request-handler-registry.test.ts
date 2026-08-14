import { describe, expect, test } from 'bun:test'
import {
    getServerRequestHandler,
    registerServerRequestHandler,
    subscribeCodeLensRefresh,
    unregisterServerRequestHandler,
} from '@shared/lib/lsp/server-request-handler-registry'

describe('registerServerRequestHandler / unregisterServerRequestHandler', () => {
    test('등록한 핸들러를 메서드 이름으로 조회할 수 있다', async () => {
        const handler = async (params: unknown) => ({ received: params })
        registerServerRequestHandler('test/registry-lookup', handler)

        const resolved = getServerRequestHandler('test/registry-lookup')
        expect(resolved).toBe(handler)
        await expect(resolved?.({ ok: true })).resolves.toEqual({ received: { ok: true } })

        unregisterServerRequestHandler('test/registry-lookup')
        expect(getServerRequestHandler('test/registry-lookup')).toBeUndefined()
    })

    test('register 가 반환하는 dispose 함수로도 해제할 수 있다', () => {
        const dispose = registerServerRequestHandler('test/registry-dispose', async () => null)
        expect(getServerRequestHandler('test/registry-dispose')).toBeDefined()

        dispose()
        expect(getServerRequestHandler('test/registry-dispose')).toBeUndefined()
    })

    test('나중에 등록한 핸들러가 같은 메서드의 이전 핸들러를 대체한다', async () => {
        registerServerRequestHandler('test/registry-replace', async () => 'first')
        registerServerRequestHandler('test/registry-replace', async () => 'second')

        const resolved = getServerRequestHandler('test/registry-replace')
        await expect(resolved?.(undefined)).resolves.toBe('second')

        unregisterServerRequestHandler('test/registry-replace')
    })

    test('등록되지 않은 메서드는 조회 시 undefined 를 반환한다', () => {
        expect(getServerRequestHandler('test/never-registered')).toBeUndefined()
    })
})

describe('기본 내장 핸들러 — workspace/configuration', () => {
    test('items 개수만큼 null 배열을 반환한다', async () => {
        const handler = getServerRequestHandler('workspace/configuration')
        expect(handler).toBeDefined()

        const result = await handler?.({ items: [{ section: 'a' }, { section: 'b' }, { section: 'c' }] })
        expect(result).toEqual([null, null, null])
    })

    test('items 필드가 없는 params 는 빈 배열을 반환한다', async () => {
        const handler = getServerRequestHandler('workspace/configuration')
        const result = await handler?.(undefined)
        expect(result).toEqual([])
    })
})

describe('기본 내장 핸들러 — 빈 result 응답', () => {
    test('client/registerCapability 는 null 을 반환한다', async () => {
        const handler = getServerRequestHandler('client/registerCapability')
        await expect(handler?.({})).resolves.toBeNull()
    })

    test('window/workDoneProgress/create 는 null 을 반환한다', async () => {
        const handler = getServerRequestHandler('window/workDoneProgress/create')
        await expect(handler?.({})).resolves.toBeNull()
    })

    test('client/unregisterCapability 는 null 을 반환한다', async () => {
        const handler = getServerRequestHandler('client/unregisterCapability')
        await expect(handler?.({})).resolves.toBeNull()
    })
})

describe('기본 내장 핸들러 — 리프레시 발화', () => {
    test('workspace/codeLens/refresh 는 null 을 반환하고 구독자를 발화한다', async () => {
        const received: number[] = []
        const unsubscribe = subscribeCodeLensRefresh(() => received.push(1))

        const handler = getServerRequestHandler('workspace/codeLens/refresh')
        const result = await handler?.(undefined)

        expect(result).toBeNull()
        expect(received).toEqual([1])

        unsubscribe()
        await handler?.(undefined)
        expect(received).toEqual([1])
    })

    /**
     * `workspace/inlayHint/refresh` has no handler here on purpose — `buildInitializeParams`
     * (lsp-session-registry.ts) never declares `workspace.inlayHint.refreshSupport`, and no
     * adapter subscribes to a refresh signal for inlay hints, so a well-behaved server should
     * never send this request. Falling through to `client.ts`'s unregistered-method response
     * (`-32601 MethodNotFound`) is the honest answer.
     */
    test('workspace/inlayHint/refresh 는 등록되어 있지 않다 (refreshSupport 를 선언하지 않았으므로)', () => {
        expect(getServerRequestHandler('workspace/inlayHint/refresh')).toBeUndefined()
    })
})
