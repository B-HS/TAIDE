import { expect, test } from 'bun:test'
import { createRemoteWsClient } from '@shared/lib/remote/remote-ws-client'
import { isRecord, numberOf, parseJson, stringOf } from '@shared/lib/remote/remote-json'
import { getRemoteConnectionRevision, subscribeRemoteConnection } from '@shared/lib/remote/connection-revision'

const RECONNECT_TEST_TIMEOUT_MS = 5_000
const CLOSE_RESTART_CODE = 1_012

test(
    '연결 실패로 거절한 요청은 재전송하지 않고 복구 후 새 요청만 실행한다',
    async () => {
        let acceptConnections = false
        const commands: string[] = []
        const recovered = Promise.withResolvers<void>()
        const originalLocation = Object.getOwnPropertyDescriptor(globalThis, 'location')
        const server = Bun.serve({
            hostname: '127.0.0.1',
            port: 0,
            fetch: async (request, server) => {
                if (acceptConnections && server.upgrade(request)) return
                return Bun.fetch('data:text/plain,connection-unavailable')
            },
            websocket: {
                message: (socket, message) => {
                    const request = parseJson(String(message))
                    if (!isRecord(request)) return
                    commands.push(stringOf(request.command))
                    socket.send(JSON.stringify({ t: 'resp', seq: numberOf(request.seq), ok: true, payload: null }))
                },
            },
        })
        Object.defineProperty(globalThis, 'location', { configurable: true, value: new URL(`http://127.0.0.1:${server.port}`) })
        const client = createRemoteWsClient(
            () => {},
            () => recovered.resolve(),
        )
        try {
            await expect(client.invoke('file_rename', { from: '/old', to: '/new' })).rejects.toBeDefined()
            acceptConnections = true
            await recovered.promise
            expect(commands).toEqual([])
            await client.invoke('file_open', { path: '/old' })
            expect(commands).toEqual(['file_open'])
        } finally {
            client.dispose()
            server.stop(true)
            if (originalLocation) Object.defineProperty(globalThis, 'location', originalLocation)
        }
    },
    RECONNECT_TEST_TIMEOUT_MS,
)

test(
    '이미 연결했던 소켓의 복구 신호로 출력 채널을 다시 연결한다',
    async () => {
        const originalLocation = Object.getOwnPropertyDescriptor(globalThis, 'location')
        const initialRevision = getRemoteConnectionRevision()
        const reattached = Promise.withResolvers<void>()
        let attachCount = 0
        const server = Bun.serve({
            hostname: '127.0.0.1',
            port: 0,
            fetch: async (request, server) => {
                if (server.upgrade(request)) return
                return Bun.fetch('data:text/plain,upgrade-required')
            },
            websocket: {
                message: (socket, message) => {
                    const request = parseJson(String(message))
                    if (!isRecord(request)) return
                    if (request.command === 'pty_attach') attachCount += 1
                    socket.send(JSON.stringify({ t: 'resp', seq: numberOf(request.seq), ok: true, payload: null }))
                    if (request.command === 'disconnect') socket.close(CLOSE_RESTART_CODE)
                },
            },
        })
        Object.defineProperty(globalThis, 'location', { configurable: true, value: new URL(`http://127.0.0.1:${server.port}`) })
        const client = createRemoteWsClient(() => {})
        const unsubscribe = subscribeRemoteConnection(async () => {
            await client.invoke('pty_attach', { sessionId: 'existing' })
            reattached.resolve()
        })
        try {
            await client.invoke('pty_attach', { sessionId: 'existing' })
            expect(getRemoteConnectionRevision()).toBe(initialRevision)
            await client.invoke('disconnect', {})
            await reattached.promise
            expect(attachCount).toBe(1 + 1)
            expect(getRemoteConnectionRevision()).toBe(initialRevision + 1)
        } finally {
            unsubscribe()
            client.dispose()
            server.stop(true)
            if (originalLocation) Object.defineProperty(globalThis, 'location', originalLocation)
        }
    },
    RECONNECT_TEST_TIMEOUT_MS,
)
