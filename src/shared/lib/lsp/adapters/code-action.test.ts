import { describe, expect, test } from 'bun:test'
import type { CancellationToken, languages } from 'monaco-editor'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { ServerCapabilities } from '@shared/lib/lsp/protocol'
import { isJsonRpcRequest } from '@shared/lib/lsp/protocol'
import {
    applyCodeActionOrCommand,
    isLspCommandOnly,
    registerCodeAction,
    requestCodeActionsForKind,
    resolveEffectiveCodeAction,
    resolveMonacoCodeAction,
    type LspCodeAction,
} from '@shared/lib/lsp/adapters/code-action'

const createFakeToken = (isCancellationRequested: boolean): CancellationToken => ({
    isCancellationRequested,
    onCancellationRequested: () => ({ dispose: () => {} }),
})

describe('isLspCommandOnly', () => {
    test('command 이 최상위 문자열이면 Command 로 판별한다', () => {
        expect(isLspCommandOnly({ title: 'Tidy', command: 'gopls.tidy' })).toBe(true)
    })

    test('command 이 중첩 객체(CodeAction.command)면 CodeAction 으로 판별한다', () => {
        expect(isLspCommandOnly({ title: 'Fix', command: { title: 'Apply', command: 'apply' } })).toBe(false)
    })

    test('command 필드가 아예 없으면 CodeAction 으로 판별한다', () => {
        expect(isLspCommandOnly({ title: 'Fix', edit: { changes: {} } })).toBe(false)
    })
})

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

describe('resolveEffectiveCodeAction', () => {
    test('resolveProvider 미지원이면 원본을 그대로 반환한다', async () => {
        const client = await createTestLspClient({}, () => null)
        const action: LspCodeAction = { title: 'Fix', data: { id: 1 } }
        expect(await resolveEffectiveCodeAction(client, false, action)).toBe(action)
    })

    test('이미 edit 이 있으면 resolve 요청을 보내지 않는다', async () => {
        let requestCount = 0
        const client = await createTestLspClient({}, () => {
            requestCount += 1
            return null
        })
        const action: LspCodeAction = { title: 'Fix', edit: { changes: {} } }
        await resolveEffectiveCodeAction(client, true, action)
        expect(requestCount).toBe(0)
    })

    test('edit 이 없고 data 가 있으면 codeAction/resolve 로 채운다', async () => {
        const client = await createTestLspClient({ codeActionProvider: { resolveProvider: true } }, (method) => {
            if (method === 'codeAction/resolve') return { title: 'Fix', edit: { changes: { 'file:///a.ts': [] } } }
            return null
        })
        const action: LspCodeAction = { title: 'Fix', data: { id: 1 } }
        const resolved = await resolveEffectiveCodeAction(client, true, action)
        expect(resolved.edit).toEqual({ changes: { 'file:///a.ts': [] } })
    })

    test('edit 도 data 도 없어도 resolveProvider 를 지원하면 resolve 요청을 보낸다 (LSP/monaco 는 data 유무를 전제하지 않는다)', async () => {
        const client = await createTestLspClient({ codeActionProvider: { resolveProvider: true } }, (method) => {
            if (method === 'codeAction/resolve') return { title: 'Fix', edit: { changes: { 'file:///a.ts': [] } } }
            return null
        })
        const action: LspCodeAction = { title: 'Fix' }
        const resolved = await resolveEffectiveCodeAction(client, true, action)
        expect(resolved.edit).toEqual({ changes: { 'file:///a.ts': [] } })
    })

    test('resolve 요청이 실패하면 원본으로 폴백한다', async () => {
        const client = createLspClient({
            send: (message) => {
                if (!isJsonRpcRequest(message)) return
                if (message.method === 'initialize') {
                    client.handleMessage({
                        jsonrpc: '2.0',
                        id: message.id,
                        result: { capabilities: { codeActionProvider: { resolveProvider: true } } },
                    })
                    return
                }
                client.handleMessage({ jsonrpc: '2.0', id: message.id, error: { code: -32603, message: 'boom' } })
            },
            onNotification: () => {},
        })
        await client.initialize({})
        const action: LspCodeAction = { title: 'Fix', data: { id: 1 } }
        expect(await resolveEffectiveCodeAction(client, true, action)).toBe(action)
    })
})

describe('applyCodeActionOrCommand', () => {
    const createFakeMonaco = () =>
        ({
            Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
            editor: { getModel: () => null },
        }) as unknown as Monaco

    test('Command 전용 항목은 executeCommand 로 실행하고 applied:true 를 반환한다', async () => {
        const monaco = createFakeMonaco()
        const client = await createTestLspClient({}, () => null)
        const executed: unknown[] = []

        const result = await applyCodeActionOrCommand(monaco, client, false, { title: 'Tidy', command: 'gopls.tidy' }, async (command) => {
            executed.push(command)
        })

        expect(result).toEqual({ applied: true })
        expect(executed).toEqual([{ title: 'Tidy', command: 'gopls.tidy' }])
    })

    test('Command 실행이 실패하면 applied:false 를 반환한다', async () => {
        const monaco = createFakeMonaco()
        const client = await createTestLspClient({}, () => null)

        const result = await applyCodeActionOrCommand(monaco, client, false, { title: 'Tidy', command: 'gopls.tidy' }, async () => {
            throw new Error('server rejected')
        })

        expect(result).toEqual({ applied: false, failureReason: 'server rejected' })
    })

    test('edit 만 있는 CodeAction 은 적용하고 커맨드를 실행하지 않는다', async () => {
        const monaco = createFakeMonaco()
        const client = await createTestLspClient({}, () => null)
        let executeCalled = false

        const result = await applyCodeActionOrCommand(monaco, client, false, { title: 'Fix', edit: { changes: {} } }, async () => {
            executeCalled = true
        })

        expect(result).toEqual({ applied: true })
        expect(executeCalled).toBe(false)
    })

    test('edit 적용이 실패하면 command 를 실행하지 않는다 (edit 성공 후에만 command)', async () => {
        const throwingModel = {
            pushEditOperations: () => {
                throw new Error('edit rejected')
            },
        }
        const monaco = {
            Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
            editor: { getModel: () => throwingModel },
        } as unknown as Monaco
        const client = await createTestLspClient({}, () => null)
        let executeCalled = false

        const result = await applyCodeActionOrCommand(
            monaco,
            client,
            false,
            {
                title: 'Fix',
                edit: {
                    changes: { 'file:///a.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, newText: 'x' }] },
                },
                command: { title: 'after', command: 'after.cmd' },
            },
            async () => {
                executeCalled = true
            },
        )

        expect(result.applied).toBe(false)
        expect(executeCalled).toBe(false)
    })
})

describe('requestCodeActionsForKind', () => {
    test('capability 가 없으면 요청 없이 빈 배열을 반환한다', async () => {
        const client = await createTestLspClient({}, () => {
            throw new Error('should not request')
        })
        const result = await requestCodeActionsForKind(
            client,
            'file:///a.ts',
            { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } },
            [],
            'source.fixAll',
        )
        expect(result).toEqual([])
    })

    test('only 로 kind 를 지정해 요청하고, 응답을 kind 계층으로 재필터한다', async () => {
        const receivedParams: unknown[] = []
        const client = await createTestLspClient({ codeActionProvider: true }, (_method, params) => {
            receivedParams.push(params)
            return [
                { title: 'Organize', kind: 'source.organizeImports.ts', edit: { changes: {} } },
                { title: 'Unrelated', kind: 'quickfix', edit: { changes: {} } },
                { title: 'Command only', command: 'noop' },
            ]
        })

        const result = await requestCodeActionsForKind(
            client,
            'file:///a.ts',
            { start: { line: 0, character: 0 }, end: { line: 1, character: 0 } },
            [],
            'source.organizeImports',
        )

        expect(result.map((item) => item.title)).toEqual(['Organize', 'Command only'])
        expect(receivedParams).toEqual([
            {
                textDocument: { uri: 'file:///a.ts' },
                range: { start: { line: 0, character: 0 }, end: { line: 1, character: 0 } },
                context: { diagnostics: [], only: ['source.organizeImports'], triggerKind: 2 },
            },
        ])
    })
})

describe('resolveMonacoCodeAction', () => {
    const createFakeMonaco = (model: unknown = null) =>
        ({
            Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
            editor: { getModel: () => model },
        }) as unknown as Monaco

    const buildProvidedAction = async (monaco: Monaco, client: Awaited<ReturnType<typeof createTestLspClient>>) => {
        const provider = registerCodeActionProviderCapture(monaco, client)
        const fakeModel = { uri: { toString: () => 'file:///a.ts' }, getVersionId: () => 1, onDidChangeContent: () => ({ dispose: () => {} }) }
        const list = await provider.provideCodeActions(
            fakeModel as never,
            { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 1 } as never,
            { markers: [], trigger: 1 } as never,
            createFakeToken(false),
        )
        if (!list) throw new Error('provideCodeActions returned no list')
        return { list, provider }
    }

    const registerCodeActionProviderCapture = (monaco: Monaco, client: Awaited<ReturnType<typeof createTestLspClient>>) => {
        let captured: languages.CodeActionProvider | undefined
        const capturingMonaco = {
            ...monaco,
            languages: {
                registerCodeActionProvider: (_lang: string, provider: languages.CodeActionProvider) => {
                    captured = provider
                    return { dispose: () => {} }
                },
            },
        } as unknown as Monaco
        registerCodeAction(capturingMonaco, client, 'ruff', 'python')
        if (!captured) throw new Error('provider not captured')
        return captured
    }

    test('command-only action 은 resolve 시 변경 없이 그대로 반환한다', async () => {
        const monaco = createFakeMonaco()
        const client = await createTestLspClient({ codeActionProvider: true }, () => [{ title: 'Tidy', command: 'gopls.tidy' }])
        const { list } = await buildProvidedAction(monaco, client)
        const action = list.actions[0] as languages.CodeAction

        const resolved = await resolveMonacoCodeAction(monaco, client, false, action, createFakeToken(false))
        expect(resolved.edit).toBeUndefined()
        expect(resolved.command).toEqual({ id: 'gopls.tidy', title: 'Tidy', arguments: undefined })
    })

    test('edit 적용에 성공하면 command 를 채워 반환한다', async () => {
        const monaco = createFakeMonaco()
        const client = await createTestLspClient({ codeActionProvider: true }, () => [
            { title: 'Fix', edit: { changes: {} }, command: { title: 'after', command: 'after.cmd' } },
        ])
        const { list } = await buildProvidedAction(monaco, client)
        const action = list.actions[0] as languages.CodeAction

        const resolved = await resolveMonacoCodeAction(monaco, client, false, action, createFakeToken(false))
        expect(resolved.command).toEqual({ id: 'after.cmd', title: 'after', arguments: undefined })
        expect(resolved.edit).toBeUndefined()
    })

    test('목록이 폐기(stale)되면 아무 것도 하지 않고 그대로 반환한다', async () => {
        const monaco = createFakeMonaco()
        const client = await createTestLspClient({ codeActionProvider: true }, () => [
            { title: 'Fix', edit: { changes: {} }, command: { title: 'after', command: 'after.cmd' } },
        ])
        const { list } = await buildProvidedAction(monaco, client)
        list.dispose()
        const action = list.actions[0] as languages.CodeAction

        const resolved = await resolveMonacoCodeAction(monaco, client, false, action, createFakeToken(false))
        expect(resolved.command).toBeUndefined()
    })

    test('취소 토큰이 설정되면 아무 것도 하지 않는다', async () => {
        const monaco = createFakeMonaco()
        const client = await createTestLspClient({ codeActionProvider: true }, () => [
            { title: 'Fix', edit: { changes: {} }, command: { title: 'after', command: 'after.cmd' } },
        ])
        const { list } = await buildProvidedAction(monaco, client)
        const action = list.actions[0] as languages.CodeAction

        const resolved = await resolveMonacoCodeAction(monaco, client, false, action, createFakeToken(true))
        expect(resolved.command).toBeUndefined()
    })

    test('edit 적용이 실패하면 command 를 채우지 않고 action 을 그대로 반환한다 (사용자에게는 토스트로 알린다)', async () => {
        const throwingModel = {
            pushEditOperations: () => {
                throw new Error('apply failed')
            },
        }
        const monaco = createFakeMonaco(throwingModel)
        const client = await createTestLspClient({ codeActionProvider: true }, () => [
            {
                title: 'Fix',
                edit: {
                    changes: { 'file:///a.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' }] },
                },
                command: { title: 'after', command: 'after.cmd' },
            },
        ])
        const { list } = await buildProvidedAction(monaco, client)
        const action = list.actions[0] as languages.CodeAction

        const resolved = await resolveMonacoCodeAction(monaco, client, false, action, createFakeToken(false))
        expect(resolved.command).toBeUndefined()
        expect(resolved.edit).toBeUndefined()
    })
})

describe('registerCodeAction', () => {
    test('서버가 codeActionProvider 를 광고하지 않으면 provider 를 등록하지 않는다', async () => {
        const client = await createTestLspClient({}, () => null)
        let registered = false
        const monaco = {
            languages: { registerCodeActionProvider: () => (registered = true) },
        } as unknown as Monaco

        const disposable = registerCodeAction(monaco, client, 'ruff', 'python')
        expect(registered).toBe(false)
        expect(() => disposable.dispose()).not.toThrow()
    })

    test('codeActionProvider 가 boolean:true 면 표준 kind 3종으로 폴백한다', async () => {
        const client = await createTestLspClient({ codeActionProvider: true }, () => [])
        let receivedMetadata: languages.CodeActionProviderMetadata | undefined
        const monaco = {
            languages: {
                registerCodeActionProvider: (_lang: string, _provider: unknown, metadata: languages.CodeActionProviderMetadata) => {
                    receivedMetadata = metadata
                    return { dispose: () => {} }
                },
            },
        } as unknown as Monaco

        registerCodeAction(monaco, client, 'ruff', 'python')
        expect(receivedMetadata?.providedCodeActionKinds).toEqual(['quickfix', 'refactor', 'source'])
    })

    test('서버가 codeActionKinds 를 광고하면 그 값을 그대로 사용한다', async () => {
        const client = await createTestLspClient({ codeActionProvider: { codeActionKinds: ['quickfix', 'source.fixAll'] } }, () => [])
        let receivedMetadata: languages.CodeActionProviderMetadata | undefined
        const monaco = {
            languages: {
                registerCodeActionProvider: (_lang: string, _provider: unknown, metadata: languages.CodeActionProviderMetadata) => {
                    receivedMetadata = metadata
                    return { dispose: () => {} }
                },
            },
        } as unknown as Monaco

        registerCodeAction(monaco, client, 'ruff', 'python')
        expect(receivedMetadata?.providedCodeActionKinds).toEqual(['quickfix', 'source.fixAll'])
    })
})
