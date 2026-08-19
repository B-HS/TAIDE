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
type FakeModel = { getLanguageId: () => string; getValue: () => string }

const FAKE_MODELS = new Map<string, FakeModel>()

const FAKE_MONACO = {
    Uri: {
        file: (path: string) => ({ toString: () => `file://${path}` }),
        parse: (uri: string) => ({ toString: () => uri }),
    },
    editor: {
        getModel: (uri: { toString: () => string }) => FAKE_MODELS.get(uri.toString()) ?? null,
    },
}

type CapturedSpawn = { sessionId: string; onMessage: (raw: string) => void }

type SentMessage = { sessionId: string; id?: number | string; method?: string; params?: unknown; result?: unknown; error?: unknown }

const createFakeLspIpc = () => {
    const spawns: CapturedSpawn[] = []
    const stopCalls: { sessionId: string; root: string | undefined }[] = []
    const sentMessages: SentMessage[] = []
    const confirmReinitializeCalls: { sessionId: string; generation: number }[] = []
    let nextSessionId = 0
    const sharedServerIds = new Set<string>()
    const sessionIdByProjectServer = new Map<string, string>()

    /**
     * Marks `serverId` as a `shares_sessions: true` server (`domain::lsp::service::should_reuse_
     * session`) — the fake `spawnLspSession` below then returns the *same* session id for a second
     * root under the same `(projectId, serverId)`, mirroring `find_reusable_entry`+
     * `should_reuse_session`'s real reuse decision closely enough to exercise
     * `lsp-session-registry.ts`'s join-detection path (R7#7) without a real backend.
     */
    const setSharesSessions = (serverId: string) => sharedServerIds.add(serverId)

    const spawnLspSession = (args: { projectId: string; serverId: string; root: string; onMessage: (raw: string) => void }) => {
        const shareKey = `${args.projectId}::${args.serverId}`
        const reusable = sharedServerIds.has(args.serverId) ? sessionIdByProjectServer.get(shareKey) : undefined
        const sessionId = reusable ?? `fake-session-${++nextSessionId}`
        sessionIdByProjectServer.set(shareKey, sessionId)
        spawns.push({ sessionId, onMessage: args.onMessage })
        return Promise.resolve(sessionId)
    }

    /**
     * Answers the outgoing `initialize` request synchronously (as a resolved LSP handshake) so
     * `createSession`'s `await client.initialize(...)` — and therefore `record.ready` — actually
     * fulfills, instead of every acquired record staying permanently pending/rejected the way a
     * real Tauri-less test environment would leave it. Mirrors the exact JSON-RPC response shape
     * `isJsonRpcResponse` (`protocol.ts`) requires. Every outgoing message is also logged to
     * `sentMessages` (not just `initialize`) so a test can assert on the client's response to an
     * inbound server→client request (e.g. `workspace/applyEdit`) without adding a second fake.
     * Routes the `initialize` echo to whichever captured spawn owns `sessionId` (not just the most
     * recent spawn) — needed once a joined root's `spawnLspSession` call can also be captured for a
     * `sessionId` that already has an earlier, unrelated spawn entry.
     */
    const sendLspMessage = ({ sessionId, message }: { sessionId: string; message: string }) => {
        const parsed = { ...(JSON.parse(message) as Omit<SentMessage, 'sessionId'>), sessionId }
        sentMessages.push(parsed)
        if (parsed.method === 'initialize' && parsed.id !== undefined) {
            const spawn = spawns.findLast((entry) => entry.sessionId === sessionId)
            queueMicrotask(() => spawn?.onMessage(JSON.stringify({ jsonrpc: '2.0', id: parsed.id, result: { capabilities: {} } })))
        }
        return Promise.resolve()
    }

    const stopLspSession = (sessionId: string, root?: string) => {
        stopCalls.push({ sessionId, root })
        return Promise.resolve()
    }

    const resolveLspRoot = () => Promise.resolve(null)

    const confirmLspReinitialize = (sessionId: string, generation: number) => {
        confirmReinitializeCalls.push({ sessionId, generation })
        return Promise.resolve()
    }

    return {
        spawnLspSession,
        sendLspMessage,
        stopLspSession,
        resolveLspRoot,
        confirmLspReinitialize,
        setSharesSessions,
        spawns,
        stopCalls,
        sentMessages,
        confirmReinitializeCalls,
    }
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
        expect(second.record.group.refCount).toBe(1)
        expect(second.record.group.disposeTimer).toBeNull()

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
        expect(handle.record.group.disposeTimer).toBeNull()

        await new Promise((resolve) => setTimeout(resolve, 0))
        expect(fakeLspIpc.stopCalls.some((call) => call.root === '/tmp/project-c')).toBe(true)
    })

    test('활성 refCount 상태(유예 진입 전)에서 flushLspSessionDisposal 을 호출해도 아무 일도 하지 않는다', async () => {
        const { acquireLspSession, flushLspSessionDisposal, peekLspSession } = await importRegistry()

        const handle = acquireLspSession(PROJECT_ID, `${SERVER_ID}-noop`, '/tmp/project-d')
        await handle.record.ready

        flushLspSessionDisposal(handle.key, handle.record)

        expect(peekLspSession(PROJECT_ID, `${SERVER_ID}-noop` as typeof SERVER_ID)).toBe(handle.record)
        expect(handle.record.group.refCount).toBe(1)
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

    test('활성 세션(refCount>0, 언마운트 전)도 강제로 즉시 dispose 된다 — projectClosed 는 팬이 언마운트되기 전에 동기 도착한다', async () => {
        const { acquireLspSession, flushLspSessionsForProject, peekLspSession } = await importRegistry()

        const handle = acquireLspSession(PROJECT_ID, `${SERVER_ID}-project-scope-active`, '/tmp/project-active')
        await handle.record.ready

        flushLspSessionsForProject(PROJECT_ID)

        expect(peekLspSession(PROJECT_ID, `${SERVER_ID}-project-scope-active` as typeof SERVER_ID)).toBeNull()

        await new Promise((resolve) => setTimeout(resolve, 0))
        expect(fakeLspIpc.stopCalls.some((call) => call.root === '/tmp/project-active')).toBe(true)
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

describe('createSession — workspace/applyEdit 세션 핸들러 등록 시점 (T0 계약 #6)', () => {
    test('initialize 왕복이 끝나기 전에 도착한 workspace/applyEdit 요청도 이 세션의 root 스코프 핸들러가 처리한다', async () => {
        const { acquireLspSession } = await importRegistry()

        const handle = acquireLspSession(PROJECT_ID, `${SERVER_ID}-early-apply-edit`, '/tmp/project-early')
        /**
         * `spawnLspSession` (the fake below) pushes onto `fakeLspIpc.spawns` synchronously, before
         * returning its already-resolved promise — so by the time `acquireLspSession` returns
         * control here (still before `createSession`'s `await spawnLspSession(...)` has actually
         * suspended the caller), the spawn for this session is already captured. Firing an inbound
         * `workspace/applyEdit` request through it *here* — before ever awaiting `handle.record.ready`
         * — simulates a server response that outraces the `initialize` round-trip. Only the fix for
         * contract #6 (registering the root-scoped handler on `client` before `spawnLspSession` is
         * even called) makes this resolve as a handled edit instead of `-32601 MethodNotFound` —
         * there is deliberately no rootless process-wide fallback left to catch it (see
         * `workspace-edit-apply-handler.ts`).
         */
        const latestSpawn = fakeLspIpc.spawns.at(-1)
        latestSpawn?.onMessage(
            JSON.stringify({ jsonrpc: '2.0', id: 'early-apply-edit', method: 'workspace/applyEdit', params: { edit: { changes: {} } } }),
        )

        await handle.record.ready
        /**
         * The injected `workspace/applyEdit` request is handled on a promise chain
         * (`client.ts`'s `void handleServerRequest(raw)`) independent of `record.ready`'s own
         * `initialize` chain — awaiting `record.ready` alone doesn't guarantee that detached chain
         * has reached `deps.send` yet. A macrotask flush drains every microtask queued by either
         * chain first, matching this file's existing pattern for asserting on similarly detached
         * fire-and-forget calls (e.g. `stopLspSession`'s `.catch()` above).
         */
        await new Promise((resolve) => setTimeout(resolve, 0))

        const response = fakeLspIpc.sentMessages.find((message) => message.id === 'early-apply-edit')
        expect(response?.error).toBeUndefined()
        expect(response?.result).toEqual({ applied: true })
    })
})

describe('acquireLspSession — 다중 root 세션 공유 (R7#7)', () => {
    test('shares_sessions=true 서버는 같은 프로젝트의 다른 root 를 같은 세션(client)으로 합류시키고, 두 번째 initialize 를 보내지 않는다', async () => {
        const { acquireLspSession } = await importRegistry()
        const serverId = `${SERVER_ID}-shared-a` as typeof SERVER_ID
        fakeLspIpc.setSharesSessions(serverId)

        const first = acquireLspSession(PROJECT_ID, serverId, '/tmp/shared-root-a')
        const session1 = await first.record.ready

        const second = acquireLspSession(PROJECT_ID, serverId, '/tmp/shared-root-b')
        const session2 = await second.record.ready

        expect(session2.sessionId).toBe(session1.sessionId)
        expect(session2.client).toBe(session1.client)
        expect(second.record.group).toBe(first.record.group)
        expect(first.record.group.roots).toEqual(new Set(['/tmp/shared-root-a', '/tmp/shared-root-b']))

        const initializeCalls = fakeLspIpc.sentMessages.filter(
            (message) => message.method === 'initialize' && message.sessionId === session1.sessionId,
        )
        expect(initializeCalls).toHaveLength(1)
    })

    test('shares_sessions=false(기본) 서버는 같은 프로젝트의 다른 root 를 독립된 세션으로 취급한다', async () => {
        const { acquireLspSession } = await importRegistry()
        const serverId = `${SERVER_ID}-not-shared` as typeof SERVER_ID

        const first = acquireLspSession(PROJECT_ID, serverId, '/tmp/solo-root-a')
        const session1 = await first.record.ready

        const second = acquireLspSession(PROJECT_ID, serverId, '/tmp/solo-root-b')
        const session2 = await second.record.ready

        expect(session2.sessionId).not.toBe(session1.sessionId)
        expect(second.record.group).not.toBe(first.record.group)
    })

    test('합류된 root 중 하나만 release 되어도(다른 root 가 아직 활성) 세션이 dispose 되지 않고, 마지막 root 까지 release 되면 참여한 모든 root 로 stopLspSession 을 호출한다', async () => {
        const { acquireLspSession, releaseLspSession, peekLspSession } = await importRegistry()
        const serverId = `${SERVER_ID}-shared-b` as typeof SERVER_ID
        fakeLspIpc.setSharesSessions(serverId)

        const first = acquireLspSession(PROJECT_ID, serverId, '/tmp/shared-b-root-a')
        const session = await first.record.ready
        const second = acquireLspSession(PROJECT_ID, serverId, '/tmp/shared-b-root-b')
        await second.record.ready

        releaseLspSession(first.key, first.record, TEST_GRACE_MS)
        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))

        expect(peekLspSession(PROJECT_ID, serverId)).not.toBeNull()
        expect(fakeLspIpc.stopCalls.some((call) => call.sessionId === session.sessionId)).toBe(false)

        releaseLspSession(second.key, second.record, TEST_GRACE_MS)
        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))

        expect(peekLspSession(PROJECT_ID, serverId)).toBeNull()
        const stoppedRoots = fakeLspIpc.stopCalls.filter((call) => call.sessionId === session.sessionId).map((call) => call.root)
        expect(new Set(stoppedRoots)).toEqual(new Set(['/tmp/shared-b-root-a', '/tmp/shared-b-root-b']))
    })

    test('합류된 세션도 프로젝트 강제 정리(flushLspSessionsForProject) 시 한 번만 dispose 된다', async () => {
        const { acquireLspSession, flushLspSessionsForProject, peekLspSession } = await importRegistry()
        const serverId = `${SERVER_ID}-shared-c` as typeof SERVER_ID
        fakeLspIpc.setSharesSessions(serverId)

        const first = acquireLspSession(PROJECT_ID, serverId, '/tmp/shared-c-root-a')
        const session = await first.record.ready
        const second = acquireLspSession(PROJECT_ID, serverId, '/tmp/shared-c-root-b')
        await second.record.ready

        flushLspSessionsForProject(PROJECT_ID)
        await new Promise((resolve) => setTimeout(resolve, 0))

        expect(peekLspSession(PROJECT_ID, serverId)).toBeNull()
        const stoppedRoots = fakeLspIpc.stopCalls.filter((call) => call.sessionId === session.sessionId).map((call) => call.root)
        expect(new Set(stoppedRoots)).toEqual(new Set(['/tmp/shared-c-root-a', '/tmp/shared-c-root-b']))
    })
})

describe('handleLspSessionStatusChanged — 자동 재시작 재핸드셰이크 (R7#1)', () => {
    test('generation 증가 + crashed 상태 수신 시 pending 요청을 reject 하고 initialize 를 재실행하며, 열린 문서를 다시 didOpen 하고 lsp_confirm_reinitialize 를 호출한다', async () => {
        const { acquireLspSession, acquireDocument, handleLspSessionStatusChanged } = await importRegistry()
        const serverId = `${SERVER_ID}-reinit` as typeof SERVER_ID
        const uri = 'file:///tmp/reinit-root/a.ts'
        FAKE_MODELS.set(uri, { getLanguageId: () => 'typescript', getValue: () => 'const a = 1' })

        const handle = acquireLspSession(PROJECT_ID, serverId, '/tmp/reinit-root')
        const session = await handle.record.ready
        acquireDocument(handle.record, session.client, uri, 'typescript', 'const a = 1')

        /** Not one of `FEATURE_CAPABILITY_CHECKS` (`client.ts`) — sent unconditionally regardless of this fake's empty `initialize` capabilities, so it actually reaches `pendingRequests` for `rejectPendingRequests` to reject. */
        const pendingRequest = session.client.request('rust-analyzer/testPendingRequest', {})
        /** Marks the promise handled immediately (a separate `.catch()`, not consuming `pendingRequest` itself) so bun's unhandled-rejection reporting doesn't fire during the `await` below, before the `expect(...).rejects` assertion further down attaches its own handler. */
        pendingRequest.catch(() => undefined)

        handleLspSessionStatusChanged({ sessionId: session.sessionId, status: 'crashed', lastError: 'boom', generation: 1 })
        await new Promise((resolve) => setTimeout(resolve, 0))

        await expect(pendingRequest).rejects.toThrow('lsp session reinitializing after crash')

        const sessionMessages = fakeLspIpc.sentMessages.filter((message) => message.sessionId === session.sessionId)
        expect(sessionMessages.filter((message) => message.method === 'initialize')).toHaveLength(2)

        const didOpenParams = sessionMessages.find(
            (message) => message.method === 'textDocument/didOpen' && (message.params as { textDocument: { uri: string } }).textDocument.uri === uri,
        )
        expect(didOpenParams).toBeDefined()

        expect(fakeLspIpc.confirmReinitializeCalls).toContainEqual({ sessionId: session.sessionId, generation: 1 })

        FAKE_MODELS.delete(uri)
    })

    test('이미 관측한 generation 이하이면 재핸드셰이크를 시작하지 않는다', async () => {
        const { acquireLspSession, handleLspSessionStatusChanged } = await importRegistry()
        const serverId = `${SERVER_ID}-reinit-stale` as typeof SERVER_ID

        const handle = acquireLspSession(PROJECT_ID, serverId, '/tmp/reinit-stale-root')
        const session = await handle.record.ready
        const countInitializeCalls = () =>
            fakeLspIpc.sentMessages.filter((message) => message.sessionId === session.sessionId && message.method === 'initialize').length

        handleLspSessionStatusChanged({ sessionId: session.sessionId, status: 'crashed', lastError: 'first crash', generation: 2 })
        await new Promise((resolve) => setTimeout(resolve, 0))
        const initializeCountAfterFirst = countInitializeCalls()

        handleLspSessionStatusChanged({ sessionId: session.sessionId, status: 'crashed', lastError: 'stale duplicate', generation: 2 })
        handleLspSessionStatusChanged({ sessionId: session.sessionId, status: 'crashed', lastError: 'older', generation: 1 })
        await new Promise((resolve) => setTimeout(resolve, 0))

        expect(countInitializeCalls()).toBe(initializeCountAfterFirst)
    })

    test('status 가 crashed 가 아니면 initialize 를 재실행하지 않는다', async () => {
        const { acquireLspSession, handleLspSessionStatusChanged } = await importRegistry()
        const serverId = `${SERVER_ID}-reinit-running` as typeof SERVER_ID

        const handle = acquireLspSession(PROJECT_ID, serverId, '/tmp/reinit-running-root')
        const session = await handle.record.ready
        const initializeCountBefore = fakeLspIpc.sentMessages.filter(
            (message) => message.sessionId === session.sessionId && message.method === 'initialize',
        ).length

        handleLspSessionStatusChanged({ sessionId: session.sessionId, status: 'running', lastError: null, generation: 1 })
        await new Promise((resolve) => setTimeout(resolve, 0))

        const initializeCountAfter = fakeLspIpc.sentMessages.filter(
            (message) => message.sessionId === session.sessionId && message.method === 'initialize',
        ).length
        expect(initializeCountAfter).toBe(initializeCountBefore)
        expect(fakeLspIpc.confirmReinitializeCalls.some((call) => call.sessionId === session.sessionId)).toBe(false)
    })

    test('알려지지 않은 sessionId 는 조용히 무시한다', async () => {
        const { handleLspSessionStatusChanged } = await importRegistry()
        expect(() => handleLspSessionStatusChanged({ sessionId: 'never-acquired', status: 'crashed', lastError: null, generation: 1 })).not.toThrow()
    })
})
