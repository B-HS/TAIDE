import { describe, expect, mock, test } from 'bun:test'

/**
 * `@shared/lib/monaco/setup` pulls in real monaco-editor worker bundles (`?worker` imports) that
 * only Vite's dev/build pipeline can resolve — `bun test` cannot load them at all (confirmed:
 * importing `lsp-session-registry.ts` directly fails with "Missing 'default' export ... ts.worker.
 * js?worker" before any test code runs). Stubbing both real-IPC boundaries the registry touches
 * (`@shared/lib/monaco/setup`, `@entities/lsp/lsp.ipc`) via `mock.module`, then reaching the module
 * under test through a *dynamic* `import()` (not a static import — Bun resolves the whole static
 * import graph, including the offending worker files, before a same-file `mock.module` call would
 * ever run) is what makes this file able to load `lsp-session-registry.ts` at all.
 */
const FAKE_MONACO = { Uri: { file: (path: string) => ({ toString: () => `file://${path}` }) } }

type CapturedSpawn = { onMessage: (raw: string) => void }

const createFakeLspIpc = () => {
    const spawns: CapturedSpawn[] = []
    const stopCalls: { sessionId: string; root: string | undefined }[] = []
    let nextSessionId = 0

    const spawnLspSession = (args: { onMessage: (raw: string) => void }) => {
        spawns.push({ onMessage: args.onMessage })
        nextSessionId += 1
        return Promise.resolve(`fake-session-${nextSessionId}`)
    }

    /**
     * Answers the outgoing `initialize` request synchronously (as a resolved LSP handshake) so
     * `createSession`'s `await client.initialize(...)` — and therefore `record.ready` — actually
     * fulfills, instead of every acquired record staying permanently pending/rejected the way a
     * real Tauri-less test environment would leave it. Mirrors the exact JSON-RPC response shape
     * `isJsonRpcResponse` (`protocol.ts`) requires.
     */
    const sendLspMessage = ({ sessionId: _sessionId, message }: { sessionId: string; message: string }) => {
        const parsed = JSON.parse(message) as { id?: number; method?: string }
        if (parsed.method === 'initialize' && parsed.id !== undefined) {
            const latestSpawn = spawns.at(-1)
            queueMicrotask(() => latestSpawn?.onMessage(JSON.stringify({ jsonrpc: '2.0', id: parsed.id, result: { capabilities: {} } })))
        }
        return Promise.resolve()
    }

    const stopLspSession = (sessionId: string, root?: string) => {
        stopCalls.push({ sessionId, root })
        return Promise.resolve()
    }

    const resolveLspRoot = () => Promise.resolve(null)

    return { spawnLspSession, sendLspMessage, stopLspSession, resolveLspRoot, stopCalls }
}

const fakeLspIpc = createFakeLspIpc()

mock.module('@shared/lib/monaco/setup', () => ({ monaco: FAKE_MONACO }))
mock.module('@entities/lsp/lsp.ipc', () => fakeLspIpc)

const importRegistry = () => import('@widgets/editor-pane/lsp-session-registry')

const PROJECT_ID = 'project-1' as Parameters<Awaited<ReturnType<typeof importRegistry>>['acquireLspSession']>[0]
const SERVER_ID = 'server-1' as Parameters<Awaited<ReturnType<typeof importRegistry>>['acquireLspSession']>[1]
const TEST_GRACE_MS = 20

describe('acquireLspSession / releaseLspSession — dispose 유예', () => {
    test('유예 기간 내 재획득하면 동일 record 를 반환하고 dispose 되지 않는다', async () => {
        const { acquireLspSession, releaseLspSession, peekLspSession } = await importRegistry()

        const first = acquireLspSession(PROJECT_ID, SERVER_ID, '/tmp/project-a')
        await first.record.ready

        releaseLspSession(first.key, first.record, TEST_GRACE_MS)
        expect(peekLspSession(PROJECT_ID, SERVER_ID)).toBe(first.record)

        const second = acquireLspSession(PROJECT_ID, SERVER_ID, '/tmp/project-a')
        expect(second.record).toBe(first.record)
        expect(second.record.refCount).toBe(1)
        expect(second.record.disposeTimer).toBeNull()

        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))
        expect(peekLspSession(PROJECT_ID, SERVER_ID)).toBe(first.record)
        expect(fakeLspIpc.stopCalls).toHaveLength(0)

        releaseLspSession(second.key, second.record, TEST_GRACE_MS)
        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))
    })

    test('유예 기간이 지나면 세션이 dispose 되고 레지스트리에서 제거된다', async () => {
        const { acquireLspSession, releaseLspSession, peekLspSession } = await importRegistry()

        const handle = acquireLspSession(PROJECT_ID, `${SERVER_ID}-expiry`, '/tmp/project-b')
        await handle.record.ready

        releaseLspSession(handle.key, handle.record, TEST_GRACE_MS)
        expect(peekLspSession(PROJECT_ID, `${SERVER_ID}-expiry` as typeof SERVER_ID)).toBe(handle.record)

        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))

        expect(peekLspSession(PROJECT_ID, `${SERVER_ID}-expiry` as typeof SERVER_ID)).toBeNull()
        expect(fakeLspIpc.stopCalls.some((call) => call.root === '/tmp/project-b')).toBe(true)
    })

    test('강제 정리(flushLspSessionDisposal)는 유예 타이머를 기다리지 않고 즉시 dispose 한다', async () => {
        const { acquireLspSession, releaseLspSession, flushLspSessionDisposal, peekLspSession } = await importRegistry()

        const handle = acquireLspSession(PROJECT_ID, `${SERVER_ID}-force`, '/tmp/project-c')
        await handle.record.ready

        releaseLspSession(handle.key, handle.record, TEST_GRACE_MS)
        expect(peekLspSession(PROJECT_ID, `${SERVER_ID}-force` as typeof SERVER_ID)).toBe(handle.record)

        flushLspSessionDisposal(handle.key, handle.record)

        expect(peekLspSession(PROJECT_ID, `${SERVER_ID}-force` as typeof SERVER_ID)).toBeNull()
        expect(handle.record.disposeTimer).toBeNull()

        await new Promise((resolve) => setTimeout(resolve, 0))
        expect(fakeLspIpc.stopCalls.some((call) => call.root === '/tmp/project-c')).toBe(true)
    })

    test('활성 refCount 상태(유예 진입 전)에서 flushLspSessionDisposal 을 호출해도 아무 일도 하지 않는다', async () => {
        const { acquireLspSession, flushLspSessionDisposal, peekLspSession } = await importRegistry()

        const handle = acquireLspSession(PROJECT_ID, `${SERVER_ID}-noop`, '/tmp/project-d')
        await handle.record.ready

        flushLspSessionDisposal(handle.key, handle.record)

        expect(peekLspSession(PROJECT_ID, `${SERVER_ID}-noop` as typeof SERVER_ID)).toBe(handle.record)
        expect(handle.record.refCount).toBe(1)
    })
})

describe('flushLspSessionsForProject / flushAllLspSessionDisposals — 프로젝트 닫기·앱 종료 확정 정리', () => {
    const OTHER_PROJECT_ID = 'project-2' as typeof PROJECT_ID

    test('같은 프로젝트의 유예 중인 세션만 즉시 dispose 하고, 다른 프로젝트의 유예 세션은 건드리지 않는다', async () => {
        const { acquireLspSession, releaseLspSession, flushLspSessionsForProject, peekLspSession } = await importRegistry()

        const own = acquireLspSession(PROJECT_ID, `${SERVER_ID}-project-scope`, '/tmp/project-own')
        await own.record.ready
        releaseLspSession(own.key, own.record, TEST_GRACE_MS)

        const other = acquireLspSession(OTHER_PROJECT_ID, `${SERVER_ID}-project-scope`, '/tmp/project-other')
        await other.record.ready
        releaseLspSession(other.key, other.record, TEST_GRACE_MS)

        flushLspSessionsForProject(PROJECT_ID)

        expect(peekLspSession(PROJECT_ID, `${SERVER_ID}-project-scope` as typeof SERVER_ID)).toBeNull()
        expect(peekLspSession(OTHER_PROJECT_ID, `${SERVER_ID}-project-scope` as typeof SERVER_ID)).toBe(other.record)

        flushLspSessionsForProject(OTHER_PROJECT_ID)
        await new Promise((resolve) => setTimeout(resolve, 0))
    })

    test('활성 세션(refCount>0)이 섞여 있어도 그대로 유지된다', async () => {
        const { acquireLspSession, flushLspSessionsForProject, peekLspSession } = await importRegistry()

        const handle = acquireLspSession(PROJECT_ID, `${SERVER_ID}-project-scope-active`, '/tmp/project-active')
        await handle.record.ready

        flushLspSessionsForProject(PROJECT_ID)

        expect(peekLspSession(PROJECT_ID, `${SERVER_ID}-project-scope-active` as typeof SERVER_ID)).toBe(handle.record)
        expect(handle.record.refCount).toBe(1)
    })

    test('모든 프로젝트의 유예 중인 세션을 한 번에 정리한다', async () => {
        const { acquireLspSession, releaseLspSession, flushAllLspSessionDisposals, peekLspSession } = await importRegistry()

        const own = acquireLspSession(PROJECT_ID, `${SERVER_ID}-flush-all`, '/tmp/project-flush-all-own')
        await own.record.ready
        releaseLspSession(own.key, own.record, TEST_GRACE_MS)

        const other = acquireLspSession(OTHER_PROJECT_ID, `${SERVER_ID}-flush-all`, '/tmp/project-flush-all-other')
        await other.record.ready
        releaseLspSession(other.key, other.record, TEST_GRACE_MS)

        flushAllLspSessionDisposals()

        expect(peekLspSession(PROJECT_ID, `${SERVER_ID}-flush-all` as typeof SERVER_ID)).toBeNull()
        expect(peekLspSession(OTHER_PROJECT_ID, `${SERVER_ID}-flush-all` as typeof SERVER_ID)).toBeNull()
    })
})

describe('waitForLspSession — 유예 중 세션 이중 수신 방지', () => {
    test('유예 중(refCount 0, 아직 dispose 전) 세션은 대기자로 등록되지 않고 즉시 그 record 로 resolve 된다', async () => {
        const { acquireLspSession, releaseLspSession, waitForLspSession } = await importRegistry()

        const handle = acquireLspSession(PROJECT_ID, `${SERVER_ID}-wait`, '/tmp/project-e')
        await handle.record.ready
        releaseLspSession(handle.key, handle.record, TEST_GRACE_MS)

        const waiter = waitForLspSession(PROJECT_ID, `${SERVER_ID}-wait` as typeof SERVER_ID)
        const resolved = await waiter.promise
        expect(resolved).toBe(handle.record)

        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))
    })

    test('세션이 아직 없을 때 등록된 대기자는 acquireLspSession 이 새로 만든 record 로 정확히 한 번만 resolve 된다', async () => {
        const { acquireLspSession, waitForLspSession } = await importRegistry()

        const waiter = waitForLspSession(PROJECT_ID, `${SERVER_ID}-fresh-wait` as typeof SERVER_ID)
        let resolveCount = 0
        void waiter.promise.then(() => {
            resolveCount += 1
        })

        const handle = acquireLspSession(PROJECT_ID, `${SERVER_ID}-fresh-wait`, '/tmp/project-f')
        await handle.record.ready
        await Promise.resolve()

        expect(await waiter.promise).toBe(handle.record)
        expect(resolveCount).toBe(1)
    })
})
