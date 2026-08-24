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
    const reportReinitializeFailureCalls: { sessionId: string; generation: number }[] = []
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
     * Lets a test simulate a respawned process that never answers `initialize` (R7#1's bounded-retry
     * timeout/backoff) — the next `count` outgoing `initialize` requests get logged to `sentMessages`
     * as usual but never receive the auto-response `sendLspMessage` below would otherwise queue,
     * leaving `client.initialize(...)` pending until `withTimeout` in `lsp-session-registry.ts` gives
     * up on it. Consumed one at a time as `initialize` requests actually go out, not per call.
     */
    let suppressedInitializeResponses = 0
    const suppressNextInitializeResponses = (count: number) => {
        suppressedInitializeResponses = count
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
            if (suppressedInitializeResponses > 0) {
                suppressedInitializeResponses -= 1
            } else {
                const spawn = spawns.findLast((entry) => entry.sessionId === sessionId)
                queueMicrotask(() => spawn?.onMessage(JSON.stringify({ jsonrpc: '2.0', id: parsed.id, result: { capabilities: {} } })))
            }
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

    const reportLspReinitializeFailure = (sessionId: string, generation: number) => {
        reportReinitializeFailureCalls.push({ sessionId, generation })
        return Promise.resolve()
    }

    return {
        spawnLspSession,
        sendLspMessage,
        stopLspSession,
        resolveLspRoot,
        confirmLspReinitialize,
        reportLspReinitializeFailure,
        setSharesSessions,
        suppressNextInitializeResponses,
        spawns,
        stopCalls,
        sentMessages,
        confirmReinitializeCalls,
        reportReinitializeFailureCalls,
        /**
         * Neither `lsp-session-registry.ts` nor this file's own tests use these four — they exist
         * purely so this `mock.module('@entities/lsp/lsp.ipc', ...)` call covers the real module's
         * entire export surface. `mock.module` is process-global and last-registration-wins, so
         * whichever of this fake and `entities/lsp/lsp.query.test.ts`'s own (differently-shaped) fake
         * happens to load last would otherwise silently replace the other for the whole test run —
         * making both fakes a superset of the real module keeps either winning a safe no-op for the
         * file that didn't.
         */
        restartLspSession: () => Promise.resolve(),
        listLspSessions: () => Promise.resolve([]),
        detectLspServers: () => Promise.resolve([]),
        installLspServer: () => Promise.resolve(),
        cancelLspInstall: () => Promise.resolve(),
    }
}

const fakeLspIpc = createFakeLspIpc()

mock.module('@shared/lib/monaco/setup', () => ({ monaco: FAKE_MONACO }))
mock.module('@entities/lsp/lsp.ipc', () => fakeLspIpc)

const importRegistry = () => import('@entities/lsp/lsp-session-registry')

const PROJECT_ID = 'project-1' as Parameters<Awaited<ReturnType<typeof importRegistry>>['acquireLspSession']>[0]
const SERVER_ID = 'server-1' as Parameters<Awaited<ReturnType<typeof importRegistry>>['acquireLspSession']>[1]
const TEST_GRACE_MS = 20

/**
 * Comfortably longer than the worst-case reinitialize retry sequence a test drives with
 * `{ timeoutMs: 20, maxAttempts: 3, retryDelayMs: 10 }`-shaped overrides (up to 3 * (20 + 10) = 90ms
 * of real elapsed time) — used to flush every attempt/backoff/microtask hop before asserting.
 */
const REINIT_TEST_SETTLE_MS = 300

describe('acquireLspSession / releaseLspSession — dispose 유예', () => {
    test('유예 기간 내 재획득하면 동일 record 를 반환하고 dispose 되지 않는다', async () => {
        const { acquireLspSession, releaseLspSession, peekLspSessionForRoot } = await importRegistry()
        const root = '/tmp/project-a'

        const first = acquireLspSession(PROJECT_ID, SERVER_ID, root)
        await first.record.ready

        releaseLspSession(first.key, first.record, TEST_GRACE_MS)
        expect(peekLspSessionForRoot(PROJECT_ID, SERVER_ID, root)).toBe(first.record)

        const second = acquireLspSession(PROJECT_ID, SERVER_ID, root)
        expect(second.record).toBe(first.record)
        expect(second.record.group.refCount).toBe(1)
        expect(second.record.group.disposeTimer).toBeNull()

        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))
        expect(peekLspSessionForRoot(PROJECT_ID, SERVER_ID, root)).toBe(first.record)
        expect(fakeLspIpc.stopCalls).toHaveLength(0)

        releaseLspSession(second.key, second.record, TEST_GRACE_MS)
        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))
    })

    test('유예 기간이 지나면 세션이 dispose 되고 레지스트리에서 제거된다', async () => {
        const { acquireLspSession, releaseLspSession, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-expiry` as typeof SERVER_ID
        const root = '/tmp/project-b'

        const handle = acquireLspSession(PROJECT_ID, serverId, root)
        await handle.record.ready

        releaseLspSession(handle.key, handle.record, TEST_GRACE_MS)
        expect(peekLspSessionForRoot(PROJECT_ID, serverId, root)).toBe(handle.record)

        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))

        expect(peekLspSessionForRoot(PROJECT_ID, serverId, root)).toBeNull()
        expect(fakeLspIpc.stopCalls.some((call) => call.root === root)).toBe(true)
    })

    test('강제 정리(flushLspSessionDisposal)는 유예 타이머를 기다리지 않고 즉시 dispose 한다', async () => {
        const { acquireLspSession, releaseLspSession, flushLspSessionDisposal, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-force` as typeof SERVER_ID
        const root = '/tmp/project-c'

        const handle = acquireLspSession(PROJECT_ID, serverId, root)
        await handle.record.ready

        releaseLspSession(handle.key, handle.record, TEST_GRACE_MS)
        expect(peekLspSessionForRoot(PROJECT_ID, serverId, root)).toBe(handle.record)

        flushLspSessionDisposal(handle.key, handle.record)

        expect(peekLspSessionForRoot(PROJECT_ID, serverId, root)).toBeNull()
        expect(handle.record.group.disposeTimer).toBeNull()

        await new Promise((resolve) => setTimeout(resolve, 0))
        expect(fakeLspIpc.stopCalls.some((call) => call.root === root)).toBe(true)
    })

    test('활성 refCount 상태(유예 진입 전)에서 flushLspSessionDisposal 을 호출해도 아무 일도 하지 않는다', async () => {
        const { acquireLspSession, flushLspSessionDisposal, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-noop` as typeof SERVER_ID
        const root = '/tmp/project-d'

        const handle = acquireLspSession(PROJECT_ID, serverId, root)
        await handle.record.ready

        flushLspSessionDisposal(handle.key, handle.record)

        expect(peekLspSessionForRoot(PROJECT_ID, serverId, root)).toBe(handle.record)
        expect(handle.record.group.refCount).toBe(1)
    })
})

describe('flushLspSessionsForProject / flushAllLspSessionDisposals — 프로젝트 닫기·앱 종료 확정 정리', () => {
    const OTHER_PROJECT_ID = 'project-2' as typeof PROJECT_ID

    test('같은 프로젝트의 유예 중인 세션만 즉시 dispose 하고, 다른 프로젝트의 유예 세션은 건드리지 않는다', async () => {
        const { acquireLspSession, releaseLspSession, flushLspSessionsForProject, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-project-scope` as typeof SERVER_ID
        const ownRoot = '/tmp/project-own'
        const otherRoot = '/tmp/project-other'

        const own = acquireLspSession(PROJECT_ID, serverId, ownRoot)
        await own.record.ready
        releaseLspSession(own.key, own.record, TEST_GRACE_MS)

        const other = acquireLspSession(OTHER_PROJECT_ID, serverId, otherRoot)
        await other.record.ready
        releaseLspSession(other.key, other.record, TEST_GRACE_MS)

        flushLspSessionsForProject(PROJECT_ID)

        expect(peekLspSessionForRoot(PROJECT_ID, serverId, ownRoot)).toBeNull()
        expect(peekLspSessionForRoot(OTHER_PROJECT_ID, serverId, otherRoot)).toBe(other.record)

        flushLspSessionsForProject(OTHER_PROJECT_ID)
        await new Promise((resolve) => setTimeout(resolve, 0))
    })

    test('활성 세션(refCount>0, 언마운트 전)도 강제로 즉시 dispose 된다 — projectClosed 는 팬이 언마운트되기 전에 동기 도착한다', async () => {
        const { acquireLspSession, flushLspSessionsForProject, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-project-scope-active` as typeof SERVER_ID
        const root = '/tmp/project-active'

        const handle = acquireLspSession(PROJECT_ID, serverId, root)
        await handle.record.ready

        flushLspSessionsForProject(PROJECT_ID)

        expect(peekLspSessionForRoot(PROJECT_ID, serverId, root)).toBeNull()

        await new Promise((resolve) => setTimeout(resolve, 0))
        expect(fakeLspIpc.stopCalls.some((call) => call.root === root)).toBe(true)
    })

    test('모든 프로젝트의 유예 중인 세션을 한 번에 정리한다', async () => {
        const { acquireLspSession, releaseLspSession, flushAllLspSessionDisposals, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-flush-all` as typeof SERVER_ID
        const ownRoot = '/tmp/project-flush-all-own'
        const otherRoot = '/tmp/project-flush-all-other'

        const own = acquireLspSession(PROJECT_ID, serverId, ownRoot)
        await own.record.ready
        releaseLspSession(own.key, own.record, TEST_GRACE_MS)

        const other = acquireLspSession(OTHER_PROJECT_ID, serverId, otherRoot)
        await other.record.ready
        releaseLspSession(other.key, other.record, TEST_GRACE_MS)

        flushAllLspSessionDisposals()

        expect(peekLspSessionForRoot(PROJECT_ID, serverId, ownRoot)).toBeNull()
        expect(peekLspSessionForRoot(OTHER_PROJECT_ID, serverId, otherRoot)).toBeNull()
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
        const { acquireLspSession, releaseLspSession, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-shared-b` as typeof SERVER_ID
        const rootA = '/tmp/shared-b-root-a'
        const rootB = '/tmp/shared-b-root-b'
        fakeLspIpc.setSharesSessions(serverId)

        const first = acquireLspSession(PROJECT_ID, serverId, rootA)
        const session = await first.record.ready
        const second = acquireLspSession(PROJECT_ID, serverId, rootB)
        await second.record.ready

        releaseLspSession(first.key, first.record, TEST_GRACE_MS)
        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))

        expect(peekLspSessionForRoot(PROJECT_ID, serverId, rootA)).not.toBeNull()
        expect(fakeLspIpc.stopCalls.some((call) => call.sessionId === session.sessionId)).toBe(false)

        releaseLspSession(second.key, second.record, TEST_GRACE_MS)
        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))

        expect(peekLspSessionForRoot(PROJECT_ID, serverId, rootA)).toBeNull()
        const stoppedRoots = fakeLspIpc.stopCalls.filter((call) => call.sessionId === session.sessionId).map((call) => call.root)
        expect(new Set(stoppedRoots)).toEqual(new Set([rootA, rootB]))
    })

    test('합류된 세션도 프로젝트 강제 정리(flushLspSessionsForProject) 시 한 번만 dispose 된다', async () => {
        const { acquireLspSession, flushLspSessionsForProject, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-shared-c` as typeof SERVER_ID
        const rootA = '/tmp/shared-c-root-a'
        const rootB = '/tmp/shared-c-root-b'
        fakeLspIpc.setSharesSessions(serverId)

        const first = acquireLspSession(PROJECT_ID, serverId, rootA)
        const session = await first.record.ready
        const second = acquireLspSession(PROJECT_ID, serverId, rootB)
        await second.record.ready

        flushLspSessionsForProject(PROJECT_ID)
        await new Promise((resolve) => setTimeout(resolve, 0))

        expect(peekLspSessionForRoot(PROJECT_ID, serverId, rootA)).toBeNull()
        const stoppedRoots = fakeLspIpc.stopCalls.filter((call) => call.sessionId === session.sessionId).map((call) => call.root)
        expect(new Set(stoppedRoots)).toEqual(new Set([rootA, rootB]))
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

    test('재핸드셰이크 첫 시도가 무응답으로 타임아웃되면 재시도해 결국 성공한다 (무한 limbo 방지)', async () => {
        const { acquireLspSession, handleLspSessionStatusChanged } = await importRegistry()
        const serverId = `${SERVER_ID}-reinit-timeout-retry` as typeof SERVER_ID

        const handle = acquireLspSession(PROJECT_ID, serverId, '/tmp/reinit-timeout-retry-root')
        const session = await handle.record.ready

        fakeLspIpc.suppressNextInitializeResponses(1)
        handleLspSessionStatusChanged(
            { sessionId: session.sessionId, status: 'crashed', lastError: 'boom', generation: 1 },
            { timeoutMs: 20, maxAttempts: 3, retryDelayMs: 10 },
        )

        await new Promise((resolve) => setTimeout(resolve, REINIT_TEST_SETTLE_MS))

        const initializeCalls = fakeLspIpc.sentMessages.filter(
            (message) => message.sessionId === session.sessionId && message.method === 'initialize',
        )
        expect(initializeCalls.length).toBeGreaterThanOrEqual(3)
        expect(fakeLspIpc.confirmReinitializeCalls).toContainEqual({ sessionId: session.sessionId, generation: 1 })
        expect(handle.record.group.isReinitializing).toBe(false)
    })

    test('모든 재시도가 실패하면 lsp_confirm_reinitialize 를 호출하지 않고 isReinitializing 을 해제한다 (무한 재시도 아님)', async () => {
        const { acquireLspSession, handleLspSessionStatusChanged, acquireDocument } = await importRegistry()
        const serverId = `${SERVER_ID}-reinit-exhausted` as typeof SERVER_ID

        const handle = acquireLspSession(PROJECT_ID, serverId, '/tmp/reinit-exhausted-root')
        const session = await handle.record.ready

        fakeLspIpc.suppressNextInitializeResponses(3)
        handleLspSessionStatusChanged(
            { sessionId: session.sessionId, status: 'crashed', lastError: 'boom', generation: 1 },
            { timeoutMs: 10, maxAttempts: 3, retryDelayMs: 5 },
        )

        await new Promise((resolve) => setTimeout(resolve, REINIT_TEST_SETTLE_MS))

        expect(fakeLspIpc.confirmReinitializeCalls.some((call) => call.sessionId === session.sessionId)).toBe(false)
        expect(fakeLspIpc.reportReinitializeFailureCalls).toContainEqual({ sessionId: session.sessionId, generation: 1 })
        expect(handle.record.group.isReinitializing).toBe(false)

        const uri = 'file:///tmp/reinit-exhausted-root/new-file.ts'
        acquireDocument(handle.record, session.client, uri, 'typescript', 'const a = 1')
        const didOpenAfterExhaustion = fakeLspIpc.sentMessages.find(
            (message) =>
                message.sessionId === session.sessionId &&
                message.method === 'textDocument/didOpen' &&
                (message.params as { textDocument: { uri: string } }).textDocument.uri === uri,
        )
        expect(didOpenAfterExhaustion).toBeDefined()
    })

    test('재핸드셰이크 진행 중(isReinitializing) 새로 열린 문서는 acquireDocument 가 직접 didOpen 을 보내지 않고, 재핸드셰이크의 replay 가 대신 보낸다', async () => {
        const { acquireLspSession, handleLspSessionStatusChanged, acquireDocument } = await importRegistry()
        const serverId = `${SERVER_ID}-reinit-mid-open` as typeof SERVER_ID
        const uri = 'file:///tmp/reinit-mid-open-root/mid.ts'
        FAKE_MODELS.set(uri, { getLanguageId: () => 'typescript', getValue: () => 'const mid = 1' })

        const handle = acquireLspSession(PROJECT_ID, serverId, '/tmp/reinit-mid-open-root')
        const session = await handle.record.ready

        fakeLspIpc.suppressNextInitializeResponses(1)
        handleLspSessionStatusChanged(
            { sessionId: session.sessionId, status: 'crashed', lastError: 'boom', generation: 1 },
            { timeoutMs: 20, maxAttempts: 3, retryDelayMs: 10 },
        )
        /** `reinitializeSession` starts with `await record.ready` (already-resolved, but still a real microtask hop) before setting `isReinitializing` — flush that hop before relying on the flag. */
        await new Promise((resolve) => setTimeout(resolve, 0))

        expect(handle.record.group.isReinitializing).toBe(true)
        acquireDocument(handle.record, session.client, uri, 'typescript', 'const mid = 1')

        const didOpenDuringGate = fakeLspIpc.sentMessages.find(
            (message) =>
                message.sessionId === session.sessionId &&
                message.method === 'textDocument/didOpen' &&
                (message.params as { textDocument: { uri: string } }).textDocument.uri === uri,
        )
        expect(didOpenDuringGate).toBeUndefined()

        await new Promise((resolve) => setTimeout(resolve, REINIT_TEST_SETTLE_MS))

        const didOpenCallsForUri = fakeLspIpc.sentMessages.filter(
            (message) =>
                message.sessionId === session.sessionId &&
                message.method === 'textDocument/didOpen' &&
                (message.params as { textDocument: { uri: string } }).textDocument.uri === uri,
        )
        expect(didOpenCallsForUri).toHaveLength(1)

        FAKE_MODELS.delete(uri)
    })
})

describe('finalizeSessionDisposal — spawn 진행 중 강제 정리 시 sessionsByKey 잔존 방지 (R7#7 회귀)', () => {
    test('spawn resolve 전에 flushLspSessionsForProject 가 호출되면 키가 즉시 정리되어, resolve 이후 같은 root 재acquire 시 새 spawn 이 생긴다', async () => {
        const { acquireLspSession, flushLspSessionsForProject, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-spawn-flush` as typeof SERVER_ID
        const root = '/tmp/spawn-flush-root'

        const first = acquireLspSession(PROJECT_ID, serverId, root)
        flushLspSessionsForProject(PROJECT_ID)

        expect(peekLspSessionForRoot(PROJECT_ID, serverId, root)).toBeNull()

        await first.record.ready.catch(() => undefined)
        await new Promise((resolve) => setTimeout(resolve, 0))

        const spawnCountBefore = fakeLspIpc.spawns.length
        const second = acquireLspSession(PROJECT_ID, serverId, root)
        await second.record.ready

        expect(fakeLspIpc.spawns.length).toBe(spawnCountBefore + 1)
        expect(second.record).not.toBe(first.record)
    })
})

describe('peekLspSessionForRoot / waitForLspSessionForRoot — 다중 root 정확한 선택 (R7#7 회귀)', () => {
    test('비공유 서버에서 root 별로 독립된 세션을 정확히 반환한다', async () => {
        const { acquireLspSession, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-multi-root` as typeof SERVER_ID

        const first = acquireLspSession(PROJECT_ID, serverId, '/tmp/multi-root-a')
        await first.record.ready
        const second = acquireLspSession(PROJECT_ID, serverId, '/tmp/multi-root-b')
        await second.record.ready

        expect(first.record).not.toBe(second.record)
        expect(peekLspSessionForRoot(PROJECT_ID, serverId, '/tmp/multi-root-a')).toBe(first.record)
        expect(peekLspSessionForRoot(PROJECT_ID, serverId, '/tmp/multi-root-b')).toBe(second.record)
        expect(peekLspSessionForRoot(PROJECT_ID, serverId, '/tmp/multi-root-c')).toBeNull()
    })

    test('유예 중(refCount 0, 아직 dispose 전) 세션은 대기자로 등록되지 않고 즉시 그 record 로 resolve 된다', async () => {
        const { acquireLspSession, releaseLspSession, waitForLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-wait-root-grace` as typeof SERVER_ID
        const root = '/tmp/wait-root-grace'

        const handle = acquireLspSession(PROJECT_ID, serverId, root)
        await handle.record.ready
        releaseLspSession(handle.key, handle.record, TEST_GRACE_MS)

        const waiter = waitForLspSessionForRoot(PROJECT_ID, serverId, root)
        expect(await waiter.promise).toBe(handle.record)

        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))
    })

    test('대기 중인 root 가 직접 획득되면 그 record 로 정확히 한 번만 resolve 된다', async () => {
        const { acquireLspSession, waitForLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-wait-root-direct` as typeof SERVER_ID
        const root = '/tmp/wait-root-direct'

        const waiter = waitForLspSessionForRoot(PROJECT_ID, serverId, root)
        let resolveCount = 0
        void waiter.promise.then(() => {
            resolveCount += 1
        })

        const handle = acquireLspSession(PROJECT_ID, serverId, root)
        await handle.record.ready
        await Promise.resolve()

        expect(await waiter.promise).toBe(handle.record)
        expect(resolveCount).toBe(1)
    })

    test('다른 root 가 먼저 획득되어도 대기자는 깨지 않고, 목표 root 가 나중에 획득되면 그 record 로 resolve 된다(다중 root 회귀)', async () => {
        const { acquireLspSession, waitForLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-wait-root-other` as typeof SERVER_ID

        const waiter = waitForLspSessionForRoot(PROJECT_ID, serverId, '/tmp/wait-root-other-b')
        let resolveCount = 0
        void waiter.promise.then(() => {
            resolveCount += 1
        })

        const handleA = acquireLspSession(PROJECT_ID, serverId, '/tmp/wait-root-other-a')
        await handleA.record.ready
        await Promise.resolve()
        expect(resolveCount).toBe(0)

        const handleB = acquireLspSession(PROJECT_ID, serverId, '/tmp/wait-root-other-b')
        await handleB.record.ready

        expect(await waiter.promise).toBe(handleB.record)
        expect(resolveCount).toBe(1)
    })

    test('cancel 은 목표 root 의 대기자 큐에서만 제거하고, 취소 후 그 root 가 획득돼도 다시 깨지 않는다', async () => {
        const { acquireLspSession, waitForLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-wait-root-cancel` as typeof SERVER_ID

        const waiter = waitForLspSessionForRoot(PROJECT_ID, serverId, '/tmp/wait-root-cancel-target')
        let resolveCount = 0
        void waiter.promise.then(() => {
            resolveCount += 1
        })

        waiter.cancel()

        const handle = acquireLspSession(PROJECT_ID, serverId, '/tmp/wait-root-cancel-target')
        await handle.record.ready
        await Promise.resolve()

        expect(resolveCount).toBe(0)
    })
})

describe('createSession — 합류 시 dispose 타이머 재무장 (correctness minor)', () => {
    test('두 root 모두 spawn 완료 전에 이미 release 되어 있으면(합류 후 refCount 0) 합류된 그룹에 dispose 타이머가 재무장된다', async () => {
        const { acquireLspSession, releaseLspSession, flushLspSessionDisposal, peekLspSessionForRoot } = await importRegistry()
        const serverId = `${SERVER_ID}-shared-rearm` as typeof SERVER_ID
        const rootA = '/tmp/shared-rearm-root-a'
        const rootB = '/tmp/shared-rearm-root-b'
        fakeLspIpc.setSharesSessions(serverId)

        const first = acquireLspSession(PROJECT_ID, serverId, rootA)
        await first.record.ready
        releaseLspSession(first.key, first.record, TEST_GRACE_MS)

        const second = acquireLspSession(PROJECT_ID, serverId, rootB)
        releaseLspSession(second.key, second.record, TEST_GRACE_MS)

        await second.record.ready

        expect(second.record.group).toBe(first.record.group)
        expect(second.record.group.refCount).toBe(0)
        expect(second.record.group.disposeTimer).not.toBeNull()

        flushLspSessionDisposal(second.key, second.record)
        expect(peekLspSessionForRoot(PROJECT_ID, serverId, rootA)).toBeNull()

        await new Promise((resolve) => setTimeout(resolve, TEST_GRACE_MS * 3))
    })
})

describe('createSession — sibling.ready 대기 타임아웃 (correctness minor)', () => {
    test('sibling 의 initialize 가 응답 없이 멈춰 있어도, sibling 대기 타임아웃 이후에는 자신의 spawn 을 진행한다', async () => {
        const { acquireLspSession } = await importRegistry()
        const serverId = `${SERVER_ID}-sibling-timeout` as typeof SERVER_ID

        fakeLspIpc.suppressNextInitializeResponses(1)
        const first = acquireLspSession(PROJECT_ID, serverId, '/tmp/sibling-timeout-root-a')

        const spawnCountBeforeSecond = fakeLspIpc.spawns.length
        const second = acquireLspSession(PROJECT_ID, serverId, '/tmp/sibling-timeout-root-b', undefined, TEST_GRACE_MS)

        await new Promise((resolve) => setTimeout(resolve, 0))
        expect(fakeLspIpc.spawns.length).toBe(spawnCountBeforeSecond)

        await second.record.ready
        expect(fakeLspIpc.spawns.length).toBe(spawnCountBeforeSecond + 1)

        void first.record.ready.catch(() => undefined)
    })
})
