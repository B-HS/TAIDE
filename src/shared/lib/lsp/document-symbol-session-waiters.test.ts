import { describe, expect, test } from 'bun:test'
import type { LspServerId, ProjectId } from '@shared/api/bindings'
import { createLspClient } from '@shared/lib/lsp/client'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import type { ServerCapabilities } from '@shared/lib/lsp/protocol'
import { buildDocumentSymbolWaiters, loadDocumentSymbolsForPath } from '@shared/lib/lsp/document-symbol-session-waiters'
import type { DocumentSymbolSessionWaiter } from '@shared/lib/lsp/document-symbol-session-waiters'

const PROJECT_ID = 'project-1' as ProjectId
const SERVER_A = 'server-a' as LspServerId
const SERVER_B = 'server-b' as LspServerId

const flushMicrotasks = () => new Promise<void>((resolve) => setTimeout(resolve, 0))

const fakeMonaco = {
    Uri: { file: (path: string) => ({ toString: () => `file://${path}` }) },
    languages: { SymbolKind: new Proxy({}, { get: (_target, prop) => prop }) },
} as unknown as Monaco

const createTestLspClient = async (capabilities: ServerCapabilities, handleRequest: (method: string, params: unknown) => unknown) => {
    const client = createLspClient({
        send: (message) => {
            if (!isJsonRpcRequest(message)) return
            if (message.method === 'initialize') {
                client.handleMessage({ jsonrpc: '2.0', id: message.id, result: { capabilities } })
                return
            }
            client.handleMessage({ jsonrpc: '2.0', id: message.id, result: handleRequest(message.method, message.params) })
        },
        onNotification: () => {},
    })
    await client.initialize({})
    return client
}

type FakeSession = { ready: Promise<{ client: LspClient }> }

const sessionWaiterFor = (client: LspClient): DocumentSymbolSessionWaiter<FakeSession> => ({
    promise: Promise.resolve({ ready: Promise.resolve({ client }) }),
    cancel: () => {},
})

describe('buildDocumentSymbolWaiters — root-aware 소비처 공통화 (contract §1.2, breadcrumbs/outline/palette 공용)', () => {
    test('서버별로 resolveLspRoot 가 반환한 루트를 그대로 waitForSession 에 넘긴다(다중 루트 정확한 선택)', async () => {
        const waitCalls: { projectId: ProjectId; serverId: LspServerId; root: string }[] = []

        const waiters = await buildDocumentSymbolWaiters<null>({
            availableServerIds: [SERVER_A, SERVER_B],
            path: '/repo/packages/a/index.ts',
            projectId: PROJECT_ID,
            fallbackRoot: '/repo',
            isCancelled: () => false,
            resolveRoot: ({ serverId }) => Promise.resolve(serverId === SERVER_A ? '/repo/packages/a' : null),
            waitForSession: (projectId, serverId, root) => {
                waitCalls.push({ projectId, serverId, root })
                return { promise: Promise.resolve(null), cancel: () => {} }
            },
        })

        expect(waitCalls).toEqual([
            { projectId: PROJECT_ID, serverId: SERVER_A, root: '/repo/packages/a' },
            { projectId: PROJECT_ID, serverId: SERVER_B, root: '/repo' },
        ])
        expect(waiters).toHaveLength(2)
    })

    test('resolveLspRoot 이 거부되거나 null 이고 fallbackRoot 도 없으면 그 서버는 건너뛴다(엉뚱한 세션 대기 방지)', async () => {
        const waitCalls: LspServerId[] = []

        const waiters = await buildDocumentSymbolWaiters<null>({
            availableServerIds: [SERVER_A, SERVER_B],
            path: '/repo/index.ts',
            projectId: PROJECT_ID,
            fallbackRoot: undefined,
            isCancelled: () => false,
            resolveRoot: ({ serverId }) => (serverId === SERVER_A ? Promise.reject(new Error('resolve failed')) : Promise.resolve(null)),
            waitForSession: (_projectId, serverId) => {
                waitCalls.push(serverId)
                return { promise: Promise.resolve(null), cancel: () => {} }
            },
        })

        expect(waitCalls).toEqual([])
        expect(waiters).toEqual([])
    })

    test('루트 해석 도중 취소되면 waitForSession 을 전혀 호출하지 않는다(대기자 누수 방지)', async () => {
        const waitCalls: LspServerId[] = []

        const waiters = await buildDocumentSymbolWaiters<null>({
            availableServerIds: [SERVER_A],
            path: '/repo/index.ts',
            projectId: PROJECT_ID,
            fallbackRoot: '/repo',
            isCancelled: () => true,
            resolveRoot: () => Promise.resolve('/repo'),
            waitForSession: (_projectId, serverId) => {
                waitCalls.push(serverId)
                return { promise: Promise.resolve(null), cancel: () => {} }
            },
        })

        expect(waitCalls).toEqual([])
        expect(waiters).toEqual([])
    })
})

describe('loadDocumentSymbolsForPath — waiter 순회+요청+cleanup 캡슐화(breadcrumbs/outline/palette 공용 로더)', () => {
    test('첫 서버가 documentSymbolProvider 를 미지원이면 다음 waiter 로 넘어간다', async () => {
        const unsupportedClient = await createTestLspClient({}, () => null)
        const supportedClient = await createTestLspClient({ documentSymbolProvider: true }, (method) =>
            method === 'textDocument/documentSymbol'
                ? [
                      {
                          name: 'foo',
                          kind: 12,
                          range: { start: { line: 0, character: 0 }, end: { line: 0, character: 3 } },
                          selectionRange: { start: { line: 0, character: 0 }, end: { line: 0, character: 3 } },
                      },
                  ]
                : null,
        )
        const loaded: unknown[] = []

        loadDocumentSymbolsForPath<FakeSession>({
            monaco: fakeMonaco,
            availableServerIds: [SERVER_A, SERVER_B],
            path: '/repo/index.ts',
            projectId: PROJECT_ID,
            fallbackRoot: '/repo',
            resolveRoot: () => Promise.resolve('/repo'),
            waitForSession: (_projectId, serverId) => sessionWaiterFor(serverId === SERVER_A ? unsupportedClient : supportedClient),
            onLoaded: (symbols) => loaded.push(symbols),
        })

        await flushMicrotasks()
        await flushMicrotasks()

        expect(loaded).toHaveLength(1)
        expect((loaded[0] as { name: string }[]).map((symbol) => symbol.name)).toEqual(['foo'])
    })

    test('모든 waiter 가 소진되면 onLoaded([]) 를 1회 호출한다', async () => {
        const loaded: unknown[] = []

        loadDocumentSymbolsForPath<FakeSession>({
            monaco: fakeMonaco,
            availableServerIds: [SERVER_A, SERVER_B],
            path: '/repo/index.ts',
            projectId: PROJECT_ID,
            fallbackRoot: '/repo',
            resolveRoot: () => Promise.resolve('/repo'),
            waitForSession: () => ({ promise: Promise.resolve(null), cancel: () => {} }),
            onLoaded: (symbols) => loaded.push(symbols),
        })

        await flushMicrotasks()
        await flushMicrotasks()

        expect(loaded).toEqual([[]])
    })

    test('반환된 cleanup 호출 후에는 onLoaded 가 호출되지 않고 각 waiter 의 cancel 이 전부 불린다', async () => {
        const cancelCalls: LspServerId[] = []
        const loaded: unknown[] = []
        const neverResolves = new Promise<FakeSession | null>(() => {})

        const cleanup = loadDocumentSymbolsForPath<FakeSession>({
            monaco: fakeMonaco,
            availableServerIds: [SERVER_A, SERVER_B],
            path: '/repo/index.ts',
            projectId: PROJECT_ID,
            fallbackRoot: '/repo',
            resolveRoot: () => Promise.resolve('/repo'),
            waitForSession: (_projectId, serverId) => ({
                promise: neverResolves,
                cancel: () => cancelCalls.push(serverId),
            }),
            onLoaded: (symbols) => loaded.push(symbols),
        })

        await flushMicrotasks()
        await flushMicrotasks()
        cleanup()

        expect(cancelCalls.sort()).toEqual([SERVER_A, SERVER_B])
        expect(loaded).toEqual([])
    })
})
