import { describe, expect, test } from 'bun:test'
import {
    getServerRequestHandler,
    registerServerRequestHandler,
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

    test('먼저 등록한 핸들러의 dispose 는 자신과 동일할 때만 지운다 — 이미 대체된 핸들러를 실수로 지우지 않는다 (F7#12)', async () => {
        const disposeFirst = registerServerRequestHandler('test/registry-identity', async () => 'first')
        registerServerRequestHandler('test/registry-identity', async () => 'second')

        disposeFirst()

        const resolved = getServerRequestHandler('test/registry-identity')
        await expect(resolved?.(undefined)).resolves.toBe('second')

        unregisterServerRequestHandler('test/registry-identity')
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
    /**
     * `workspace/codeLens/refresh` also has no *process-wide* handler here on purpose (F7#4) — a
     * global fallback fired every open session's listeners on a refresh push from any one server,
     * so two unrelated projects' sessions recomputed lenses whenever the other's server asked for
     * a refresh. `lsp-session-registry.ts`'s `createSession` now registers this method per LSP
     * client instead (`client.ts`'s instance-level handler, the `workspace/applyEdit`/
     * `workspace/semanticTokens/refresh` precedent), and `adapters/code-lens.test.ts` covers its
     * per-client fan-out (`triggerCodeLensRefresh`).
     *
     * `workspace/inlayHint/refresh` has no handler here on purpose — `buildInitializeParams`
     * (lsp-session-registry.ts) never declares `workspace.inlayHint.refreshSupport`, and no
     * adapter subscribes to a refresh signal for inlay hints, so a well-behaved server should
     * never send this request. Falling through to `client.ts`'s unregistered-method response
     * (`-32601 MethodNotFound`) is the honest answer.
     */
    test('workspace/codeLens/refresh 는 process-wide 기본 핸들러로 등록되어 있지 않다 (세션 스코프로 이관됨)', () => {
        expect(getServerRequestHandler('workspace/codeLens/refresh')).toBeUndefined()
    })

    test('workspace/inlayHint/refresh 는 등록되어 있지 않다 (refreshSupport 를 선언하지 않았으므로)', () => {
        expect(getServerRequestHandler('workspace/inlayHint/refresh')).toBeUndefined()
    })
})
