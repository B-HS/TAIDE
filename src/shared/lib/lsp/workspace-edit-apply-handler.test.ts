import { describe, expect, test } from 'bun:test'
import { createLspClient } from '@shared/lib/lsp/client'
import { getServerRequestHandler } from '@shared/lib/lsp/server-request-handler-registry'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { createWorkspaceApplyEditHandler, registerWorkspaceApplyEditHandler } from '@shared/lib/lsp/workspace-edit-apply-handler'

const createFakeMonaco = () =>
    ({
        Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
        editor: { getModel: () => null, getEditors: () => [] as { getModel: () => unknown }[] },
    }) as unknown as Monaco

const createFakeClient = () => createLspClient({ send: () => {}, onNotification: () => {} })

const FAKE_PROJECT_ID = 'proj-1'

describe('registerWorkspaceApplyEditHandler', () => {
    test('workspace/applyEdit 핸들러를 레지스트리에 등록한다', () => {
        const dispose = registerWorkspaceApplyEditHandler(createFakeMonaco())
        expect(getServerRequestHandler('workspace/applyEdit')).toBeDefined()
        dispose()
        expect(getServerRequestHandler('workspace/applyEdit')).toBeUndefined()
    })

    test('빈 changes 를 가진 유효한 ApplyWorkspaceEditParams 는 applied:true 를 반환한다', async () => {
        const dispose = registerWorkspaceApplyEditHandler(createFakeMonaco())
        const handler = getServerRequestHandler('workspace/applyEdit')

        const result = await handler?.({ edit: { changes: {} } })

        expect(result).toEqual({ applied: true })
        dispose()
    })

    test('edit 필드가 없는 잘못된 params 는 applied:false 를 반환한다', async () => {
        const dispose = registerWorkspaceApplyEditHandler(createFakeMonaco())
        const handler = getServerRequestHandler('workspace/applyEdit')

        const result = await handler?.({})

        expect(result).toEqual({ applied: false, failureReason: 'invalid ApplyWorkspaceEditParams' })
        dispose()
    })
})

describe('createWorkspaceApplyEditHandler — 세션별 root 스코프', () => {
    test('allowedRoot 밖의 경로를 대상으로 하면 적용 없이 일반화된 사유로 거절한다 (절대경로를 서버에 노출하지 않는다)', async () => {
        const handler = createWorkspaceApplyEditHandler(createFakeMonaco(), '/workspace', createFakeClient(), FAKE_PROJECT_ID)

        const result = await handler({
            edit: {
                changes: {
                    'file:///outside/secret.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, newText: 'x' }],
                },
            },
        })

        expect(result).toEqual({ applied: false, failureReason: 'edit rejected' })
    })

    test('allowedRoot 하위 경로는 정상 적용된다', async () => {
        const handler = createWorkspaceApplyEditHandler(createFakeMonaco(), '/workspace', createFakeClient(), FAKE_PROJECT_ID)

        const result = await handler({ edit: { changes: {} } })

        expect(result).toEqual({ applied: true })
    })

    test('잘못된 params 는 root 검사 전에 그 사유를 그대로 반환한다 (경로 정보가 없어 일반화가 불필요)', async () => {
        const handler = createWorkspaceApplyEditHandler(createFakeMonaco(), '/workspace', createFakeClient(), FAKE_PROJECT_ID)

        const result = await handler({})

        expect(result).toEqual({ applied: false, failureReason: 'invalid ApplyWorkspaceEditParams' })
    })
})
