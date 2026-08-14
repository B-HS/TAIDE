import { describe, expect, test } from 'bun:test'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import {
    MONACO_SHOW_REFERENCES_COMMAND_ID,
    RUST_ANALYZER_GOTO_LOCATION_COMMAND_ID,
    RUST_ANALYZER_SHOW_REFERENCES_COMMAND_ID,
    createGotoLocationHandler,
    createShowReferencesHandler,
    registerLspClientNavigationCommands,
    registerSessionExecuteCommands,
} from '@shared/lib/lsp/command-relay'

type RegisteredCommand = { id: string; handler: (...args: unknown[]) => unknown }

const createFakeMonaco = () => {
    const registered: RegisteredCommand[] = []
    const monaco = {
        Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
        editor: {
            registerCommand: (id: string, handler: (...args: unknown[]) => unknown) => {
                registered.push({ id, handler })
                return { dispose: () => (registered.length = registered.filter((entry) => entry.id !== id || entry.handler !== handler).length) }
            },
        },
    }
    return { monaco: monaco as unknown as Monaco, registered }
}

describe('createShowReferencesHandler', () => {
    test('LSP 인자(uri 문자열·position·Location[])를 monaco goToLocations 인자로 변환해 peek 로 연다', async () => {
        const { monaco } = createFakeMonaco()
        const calls: unknown[][] = []
        const execute = async (...args: unknown[]) => {
            calls.push(args)
            return null
        }
        const handler = createShowReferencesHandler(monaco, execute)

        await handler(undefined, 'file:///a.ts', { line: 2, character: 4 }, [
            { uri: 'file:///b.ts', range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } } },
        ])

        expect(calls).toHaveLength(1)
        const [commandId, uri, position, locations, multiple, noResultsMessage, openInPeek] = calls[0] ?? []
        expect(commandId).toBe('editor.action.goToLocations')
        expect(uri).toEqual({ fsPath: '/a.ts', toString: expect.any(Function) })
        expect(position).toEqual({ lineNumber: 3, column: 5 })
        expect(locations).toEqual([{ uri: expect.anything(), range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 2 } }])
        expect(multiple).toBe('peek')
        expect(noResultsMessage).toBeUndefined()
        expect(openInPeek).toBe(true)
    })

    test('LocationLink(targetUri/targetRange) 도 처리한다', async () => {
        const { monaco } = createFakeMonaco()
        const calls: unknown[][] = []
        const handler = createShowReferencesHandler(monaco, async (...args) => {
            calls.push(args)
            return null
        })

        await handler(undefined, 'file:///a.ts', { line: 0, character: 0 }, [
            {
                targetUri: 'file:///b.ts',
                targetRange: { start: { line: 5, character: 0 }, end: { line: 5, character: 1 } },
                targetSelectionRange: { start: { line: 5, character: 0 }, end: { line: 5, character: 1 } },
            },
        ])

        const locations = calls[0]?.[3] as { range: { startLineNumber: number } }[]
        expect(locations[0]?.range.startLineNumber).toBe(6)
    })

    test('인자 형태가 LSP 시그니처와 다르면 아무 것도 실행하지 않는다', async () => {
        const { monaco } = createFakeMonaco()
        let called = false
        const handler = createShowReferencesHandler(monaco, async () => {
            called = true
            return null
        })

        await handler(undefined, { notAString: true }, { line: 0, character: 0 }, [])
        expect(called).toBe(false)
    })
})

describe('createGotoLocationHandler', () => {
    test('단일 Location 을 goToLocations(multiple: goto) 로 직접 이동시킨다 (peek 아님)', async () => {
        const { monaco } = createFakeMonaco()
        const calls: unknown[][] = []
        const handler = createGotoLocationHandler(monaco, async (...args) => {
            calls.push(args)
            return null
        })

        await handler(undefined, { uri: 'file:///a.rs', range: { start: { line: 10, character: 2 }, end: { line: 10, character: 5 } } })

        const [commandId, , position, locations, multiple] = calls[0] ?? []
        expect(commandId).toBe('editor.action.goToLocations')
        expect(position).toEqual({ lineNumber: 11, column: 3 })
        expect(locations).toHaveLength(1)
        expect(multiple).toBe('goto')
    })

    test('LocationLink 도 처리한다', async () => {
        const { monaco } = createFakeMonaco()
        const calls: unknown[][] = []
        const handler = createGotoLocationHandler(monaco, async (...args) => {
            calls.push(args)
            return null
        })

        await handler(undefined, {
            targetUri: 'file:///a.rs',
            targetRange: { start: { line: 1, character: 0 }, end: { line: 1, character: 1 } },
            targetSelectionRange: { start: { line: 1, character: 0 }, end: { line: 1, character: 1 } },
        })

        expect(calls).toHaveLength(1)
    })

    test('LocationLink 은 targetRange 가 아니라 targetSelectionRange 로 커서를 위치시킨다 (targetRange 는 문서 주석을 포함해 더 넓을 수 있다)', async () => {
        const { monaco } = createFakeMonaco()
        const calls: unknown[][] = []
        const handler = createGotoLocationHandler(monaco, async (...args) => {
            calls.push(args)
            return null
        })

        await handler(undefined, {
            targetUri: 'file:///a.rs',
            targetRange: { start: { line: 0, character: 0 }, end: { line: 5, character: 1 } },
            targetSelectionRange: { start: { line: 3, character: 4 }, end: { line: 3, character: 10 } },
        })

        const [, , position] = calls[0] ?? []
        expect(position).toEqual({ lineNumber: 4, column: 5 })
    })

    test('유효하지 않은 인자는 아무 것도 실행하지 않는다', async () => {
        const { monaco } = createFakeMonaco()
        let called = false
        const handler = createGotoLocationHandler(monaco, async () => {
            called = true
            return null
        })

        await handler(undefined, null)
        expect(called).toBe(false)
    })
})

describe('registerLspClientNavigationCommands', () => {
    test('showReferences 2종(monaco/rust-analyzer)과 gotoLocation 을 등록한다', () => {
        const { monaco, registered } = createFakeMonaco()
        registerLspClientNavigationCommands(monaco)

        const ids = registered.map((entry) => entry.id)
        expect(ids).toEqual([MONACO_SHOW_REFERENCES_COMMAND_ID, RUST_ANALYZER_SHOW_REFERENCES_COMMAND_ID, RUST_ANALYZER_GOTO_LOCATION_COMMAND_ID])
    })

    test('dispose 는 등록한 3개 커맨드를 모두 해제한다', () => {
        const { monaco } = createFakeMonaco()
        const disposeCalls: string[] = []
        const trackedMonaco = {
            ...monaco,
            editor: {
                registerCommand: (id: string) => ({ dispose: () => disposeCalls.push(id) }),
            },
        } as unknown as Monaco

        registerLspClientNavigationCommands(trackedMonaco).dispose()
        expect(disposeCalls).toEqual([
            MONACO_SHOW_REFERENCES_COMMAND_ID,
            RUST_ANALYZER_SHOW_REFERENCES_COMMAND_ID,
            RUST_ANALYZER_GOTO_LOCATION_COMMAND_ID,
        ])
    })
})

const createFakeClient = () => createLspClient({ send: () => {}, onNotification: () => {} })

describe('registerSessionExecuteCommands', () => {
    test('commands 목록이 없으면 아무 것도 등록하지 않는다', () => {
        const { monaco, registered } = createFakeMonaco()
        const client = createFakeClient()
        registerSessionExecuteCommands(monaco, client, undefined)
        expect(registered).toEqual([])
    })

    test('서버 executeCommandProvider.commands 각각을 monaco 커맨드로 등록한다', () => {
        const { monaco, registered } = createFakeMonaco()
        const client = createFakeClient()
        registerSessionExecuteCommands(monaco, client, ['gopls.tidy', 'gopls.test'])
        expect(registered.map((entry) => entry.id)).toEqual(['gopls.tidy', 'gopls.test'])
    })

    test('등록된 커맨드 실행 시 workspace/executeCommand 로 중계한다', async () => {
        const sentRequests: { method: string; params: unknown }[] = []
        const client = createLspClient({
            send: (message) => {
                if (!isJsonRpcRequest(message)) return
                if (message.method === 'initialize') {
                    client.handleMessage({
                        jsonrpc: '2.0',
                        id: message.id,
                        result: { capabilities: { executeCommandProvider: { commands: ['gopls.tidy'] } } },
                    })
                    return
                }
                sentRequests.push({ method: message.method, params: message.params })
                client.handleMessage({ jsonrpc: '2.0', id: message.id, result: null })
            },
            onNotification: () => {},
        })
        await client.initialize({})

        const { monaco, registered } = createFakeMonaco()
        registerSessionExecuteCommands(monaco, client, ['gopls.tidy'])

        await registered[0]?.handler(undefined, 'file:///go.mod')
        await new Promise((resolve) => setTimeout(resolve, 0))

        expect(sentRequests).toEqual([{ method: 'workspace/executeCommand', params: { command: 'gopls.tidy', arguments: ['file:///go.mod'] } }])
    })

    test('dispose 는 등록한 모든 커맨드를 해제한다', () => {
        const client = createFakeClient()
        const disposeCalls: string[] = []
        const trackedMonaco = {
            editor: { registerCommand: (id: string) => ({ dispose: () => disposeCalls.push(id) }) },
        } as unknown as Monaco

        registerSessionExecuteCommands(trackedMonaco, client, ['a', 'b']).dispose()
        expect(disposeCalls).toEqual(['a', 'b'])
    })
})
