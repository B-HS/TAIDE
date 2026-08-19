import { describe, expect, test } from 'bun:test'
import { createLspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { createWorkspaceApplyEditHandler } from '@shared/lib/lsp/workspace-edit-apply-handler'

const createFakeMonaco = () =>
    ({
        Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
        editor: { getModel: () => null, getEditors: () => [] as { getModel: () => unknown }[] },
    }) as unknown as Monaco

const createFakeClient = () => createLspClient({ send: () => {}, onNotification: () => {} })

const FAKE_PROJECT_ID = 'proj-1'

describe('createWorkspaceApplyEditHandler — 세션별 root 스코프', () => {
    test('allowedRoots 밖의 경로를 대상으로 하면 적용 없이 일반화된 사유로 거절한다 (절대경로를 서버에 노출하지 않는다)', async () => {
        const handler = createWorkspaceApplyEditHandler(createFakeMonaco(), new Set(['/workspace']), createFakeClient(), FAKE_PROJECT_ID)

        const result = await handler({
            edit: {
                changes: {
                    'file:///outside/secret.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, newText: 'x' }],
                },
            },
        })

        expect(result).toEqual({ applied: false, failureReason: 'edit rejected' })
    })

    test('allowedRoots 하위 경로는 정상 적용된다', async () => {
        const handler = createWorkspaceApplyEditHandler(createFakeMonaco(), new Set(['/workspace']), createFakeClient(), FAKE_PROJECT_ID)

        const result = await handler({ edit: { changes: {} } })

        expect(result).toEqual({ applied: true })
    })

    test('잘못된 params 는 root 검사 전에 그 사유를 그대로 반환한다 (경로 정보가 없어 일반화가 불필요)', async () => {
        const handler = createWorkspaceApplyEditHandler(createFakeMonaco(), new Set(['/workspace']), createFakeClient(), FAKE_PROJECT_ID)

        const result = await handler({})

        expect(result).toEqual({ applied: false, failureReason: 'invalid ApplyWorkspaceEditParams' })
    })

    test('handler 등록 이후 allowedRoots 집합에 추가된 root(R7#7 세션 join)도 그대로 허용된다 (스냅샷이 아니라 참조를 읽는다)', async () => {
        const joinedUri = 'file:///workspace-b/joined.ts'
        /**
         * A fake *open* model for `joinedUri` — an edit with no open model falls through to the
         * real file-IPC path (`applyTextEditsToUri`'s `deps.openFile`/`saveFile`, defaulted to the
         * real, unmocked Tauri bridge here since `createWorkspaceApplyEditHandler` doesn't take a
         * `deps` override), which fails outside a real Tauri context regardless of the root check
         * this test means to isolate. An open model instead takes the `pushEditOperations` branch,
         * which never touches file IPC (its background-mirror write is itself best-effort and
         * swallowed on failure).
         */
        const fakeMonaco = {
            Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
            editor: {
                getModel: (uri: { toString: () => string }) =>
                    uri.toString() === joinedUri ? { pushEditOperations: () => null, getValue: () => '' } : null,
                getEditors: () => [] as { getModel: () => unknown }[],
            },
        } as unknown as Monaco

        const allowedRoots = new Set(['/workspace-a'])
        const handler = createWorkspaceApplyEditHandler(fakeMonaco, allowedRoots, createFakeClient(), FAKE_PROJECT_ID)

        allowedRoots.add('/workspace-b')

        const result = await handler({
            edit: {
                changes: {
                    [joinedUri]: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, newText: 'x' }],
                },
            },
        })

        expect(result).toEqual({ applied: true })
    })
})
