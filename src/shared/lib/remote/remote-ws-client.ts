import { isRecord, numberOf, parseJson, stringOf } from '@shared/lib/remote/remote-json'

const RESPONSE_TAG = 0x02
const CHANNEL_TAG = 0x01
const RECONNECT_DELAY_MS = 1_000
const CHANNEL_FRAME_HEADER_BYTES = 9
const RESPONSE_FRAME_HEADER_BYTES = 5

export type NonResponseFrame =
    | { kind: 'chan'; channelId: number; index: number; message: unknown }
    | { kind: 'chanEnd'; channelId: number; index: number }
    | { kind: 'chanBinary'; channelId: number; index: number; buffer: ArrayBuffer }
    | { kind: 'event'; event: string; payload: string }

export type RemoteWsClient = {
    invoke: (command: string, args: unknown) => Promise<unknown>
}

type PendingResolver = { resolve: (value: unknown) => void; reject: (reason: unknown) => void }

export const createRemoteWsClient = (onFrame: (frame: NonResponseFrame) => void): RemoteWsClient => {
    const pending = new Map<number, PendingResolver>()
    const outbox: string[] = []
    let socket: WebSocket | null = null
    let nextSeq = 1

    const rejectAll = () => {
        for (const resolver of pending.values()) {
            resolver.reject({ code: 'RemoteDisconnected', message: '원격 연결이 끊어졌습니다' })
        }
        pending.clear()
    }

    const handleTextFrame = (raw: string) => {
        const frame = parseJson(raw)
        if (!isRecord(frame)) return

        if (frame.t === 'resp') {
            const seq = numberOf(frame.seq)
            const resolver = pending.get(seq)
            if (!resolver) return
            pending.delete(seq)
            if (frame.ok === true) resolver.resolve(frame.payload)
            else resolver.reject(frame.payload)
            return
        }
        if (frame.t === 'chan') {
            onFrame({ kind: 'chan', channelId: numberOf(frame.channelId), index: numberOf(frame.index), message: frame.message })
            return
        }
        if (frame.t === 'chanEnd') {
            onFrame({ kind: 'chanEnd', channelId: numberOf(frame.channelId), index: numberOf(frame.index) })
            return
        }
        if (frame.t === 'event') {
            onFrame({ kind: 'event', event: stringOf(frame.event), payload: stringOf(frame.payload) })
        }
    }

    const handleBinaryFrame = (buffer: ArrayBuffer) => {
        if (buffer.byteLength < 1) return
        const view = new DataView(buffer)
        const tag = view.getUint8(0)

        if (tag === RESPONSE_TAG) {
            if (buffer.byteLength < RESPONSE_FRAME_HEADER_BYTES) return
            const seq = view.getUint32(1)
            const resolver = pending.get(seq)
            if (!resolver) return
            pending.delete(seq)
            resolver.resolve(buffer.slice(RESPONSE_FRAME_HEADER_BYTES))
            return
        }
        if (tag === CHANNEL_TAG) {
            if (buffer.byteLength < CHANNEL_FRAME_HEADER_BYTES) return
            const channelId = view.getUint32(1)
            const index = view.getUint32(5)
            onFrame({ kind: 'chanBinary', channelId, index, buffer: buffer.slice(CHANNEL_FRAME_HEADER_BYTES) })
        }
    }

    const connect = () => {
        const url = new URL('/__taide/ws', location.href)
        url.protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
        const next = new WebSocket(url)
        next.binaryType = 'arraybuffer'

        next.onopen = () => {
            socket = next
            while (outbox.length > 0) {
                const message = outbox.shift()
                if (message) next.send(message)
            }
        }
        next.onmessage = (event) => {
            if (typeof event.data === 'string') handleTextFrame(event.data)
            else if (event.data instanceof ArrayBuffer) handleBinaryFrame(event.data)
        }
        next.onclose = () => {
            socket = null
            rejectAll()
            setTimeout(connect, RECONNECT_DELAY_MS)
        }
        next.onerror = () => next.close()
    }

    connect()

    const invoke = (command: string, args: unknown) => {
        const seq = nextSeq
        nextSeq += 1
        const message = JSON.stringify({ seq, command, args: args ?? null })
        return new Promise<unknown>((resolve, reject) => {
            pending.set(seq, { resolve, reject })
            if (socket && socket.readyState === WebSocket.OPEN) socket.send(message)
            else outbox.push(message)
        })
    }

    return { invoke }
}
