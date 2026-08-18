import { createCallbackRegistry } from '@shared/lib/remote/callback-registry'
import { isRecord, numberOf, parseJson, stringOf } from '@shared/lib/remote/remote-json'
import { createRemoteWsClient, type NonResponseFrame } from '@shared/lib/remote/remote-ws-client'

declare global {
    interface Window {
        __TAURI_INTERNALS__?: unknown
        isTauri?: boolean
    }
}

/**
 * Window label the remote-mirror shim reports through `getCurrentWindow().label`. Must stay
 * distinct from every real desktop window label (`main`, `editor-<n>` — see `tauri.conf.json`
 * and `domain::window::commands::open_auxiliary_window`), because backend LSP session ownership
 * (`domain::lsp::commands::SessionEntry.channels`, keyed by this label) treats two callers with
 * the same label as the *same* window and lets one silently steal the other's message channel.
 * Kept in sync with the label the Rust side already documents as fixed for remote clients
 * (`domain/lsp/commands.rs` — "the remote client's fixed `\"remote\"` label").
 */
export const REMOTE_WINDOW_LABEL = 'remote'
const FILE_ROUTE = '/__taide/file'

type EventListener = { event: string; callbackId: number }

export const installRemoteInternalsShim = () => {
    if (typeof window === 'undefined') return
    if (window.__TAURI_INTERNALS__) return

    const registry = createCallbackRegistry()
    const eventListeners = new Map<number, EventListener>()
    let nextEventId = 1

    const deliverEvent = (event: string, payloadJson: string) => {
        const payload = parseJson(payloadJson)
        for (const [eventId, listener] of eventListeners) {
            if (listener.event !== event) continue
            registry.runCallback(listener.callbackId, { event, id: eventId, payload })
        }
    }

    const handleFrame = (frame: NonResponseFrame) => {
        if (frame.kind === 'chan') {
            registry.runCallback(frame.channelId, { message: frame.message, index: frame.index })
            return
        }
        if (frame.kind === 'chanBinary') {
            registry.runCallback(frame.channelId, { message: frame.buffer, index: frame.index })
            return
        }
        if (frame.kind === 'chanEnd') {
            registry.runCallback(frame.channelId, { end: true, index: frame.index })
            return
        }
        deliverEvent(frame.event, frame.payload)
    }

    const client = createRemoteWsClient(handleFrame)

    const handleListen = (args: unknown) => {
        const event = isRecord(args) ? stringOf(args.event) : ''
        const callbackId = isRecord(args) ? numberOf(args.handler) : 0
        const eventId = nextEventId
        nextEventId += 1
        eventListeners.set(eventId, { event, callbackId })
        return Promise.resolve(eventId)
    }

    const handleUnlisten = (args: unknown) => {
        if (isRecord(args)) eventListeners.delete(numberOf(args.eventId))
        return Promise.resolve(null)
    }

    const invoke = (command: string, args: unknown): Promise<unknown> => {
        if (command === 'plugin:event|listen') return handleListen(args)
        if (command === 'plugin:event|unlisten') return handleUnlisten(args)
        if (command.startsWith('plugin:event|emit')) return Promise.resolve(null)
        if (command === 'plugin:dialog|open' || command === 'plugin:dialog|save') return Promise.resolve(null)
        if (command.startsWith('plugin:')) return Promise.resolve(null)
        return client.invoke(command, args)
    }

    window.__TAURI_INTERNALS__ = {
        isTauri: true,
        metadata: {
            currentWindow: { label: REMOTE_WINDOW_LABEL },
            currentWebview: { label: REMOTE_WINDOW_LABEL },
        },
        plugins: {},
        callbacks: registry.callbacks,
        transformCallback: registry.transformCallback,
        runCallback: registry.runCallback,
        unregisterCallback: registry.unregisterCallback,
        invoke,
        convertFileSrc: (path: string) => `${FILE_ROUTE}?path=${encodeURIComponent(path)}`,
    }

    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
        unregisterListener: (_event: string, eventId: number) => {
            eventListeners.delete(eventId)
        },
    }

    window.isTauri = true
}
