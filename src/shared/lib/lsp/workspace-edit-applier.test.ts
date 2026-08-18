import { describe, expect, test } from 'bun:test'
import { IpcError } from '@shared/api/unwrap-result'
import { consumeExternallyDirtyModel } from '@shared/lib/lsp/model-dirty-tracker'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { preloadPeekModel } from '@shared/lib/lsp/peek-model-preload'
import type { TextEdit, WorkspaceEdit } from '@shared/lib/lsp/protocol'
import { applyTextEditsToContent, applyWorkspaceEdit, type WorkspaceEditApplierDeps } from '@shared/lib/lsp/workspace-edit-applier'

type FakeModelEditCall = { range: unknown; text: string }[]

/** `value` is a static stand-in for "the model's content after the edit" — this fake never actually applies `pushEditOperations`' ranges/text to it, so tests that care about the post-edit value assert against whatever `value` was passed in, not a real splice. */
const createFakeModel = (value = 'fake-model-value') => {
    const edits: FakeModelEditCall[] = []
    return {
        model: {
            pushEditOperations: (_before: unknown, operations: FakeModelEditCall, _compute: unknown) => {
                edits.push(operations)
                return null
            },
            getValue: () => value,
        },
        getEdits: () => edits,
    }
}

/**
 * `openModels` maps a `file://` uri to a fake model — mirrors `monaco.editor.getModel` being the
 * single source of truth the applier consults (no separate `@entities/editor/model-registry`
 * dependency, which would drag the real monaco worker setup into this unit test).
 */
const createFakeMonaco = (openModels: Record<string, ReturnType<typeof createFakeModel>['model']> = {}) =>
    ({
        Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
        editor: {
            getModel: (uri: { toString: () => string }) => openModels[uri.toString()] ?? null,
            getEditors: () => [] as { getModel: () => unknown }[],
        },
    }) as unknown as Monaco

type CallLog = { fn: string; args: unknown[] }[]

const FAKE_ACTIVE_PROJECT_ID = 'proj-1'

const createFakeDeps = (overrides: Partial<WorkspaceEditApplierDeps> = {}) => {
    const calls: CallLog = []
    /** Kept separate from `calls` — the 5 file-IPC deps above already have tests asserting an exact `calls` sequence, and the cross-file mirror write (§`mirrorBackgroundModelEdit`) fires alongside those on every "open model, no attached editor" fake (this file's `createFakeMonaco` reports no attached editors by default), which would otherwise break every one of those unrelated assertions. */
    const mirrorCalls: CallLog = []
    const files = new Map<string, string>()

    const deps: WorkspaceEditApplierDeps = {
        openFile: (async (path: string) => {
            calls.push({ fn: 'openFile', args: [path] })
            if (!files.has(path)) throw new IpcError({ code: 'NotFound', message: `not found: ${path}` })
            return { content: files.get(path) } as never
        }) as WorkspaceEditApplierDeps['openFile'],
        saveFile: (async (input: { path: string; content: string }) => {
            calls.push({ fn: 'saveFile', args: [input] })
            files.set(input.path, input.content)
            return null
        }) as WorkspaceEditApplierDeps['saveFile'],
        createEntry: (async (input: { path: string; isDir: boolean }) => {
            calls.push({ fn: 'createEntry', args: [input] })
            if (files.has(input.path)) throw new IpcError({ code: 'InvalidArgument', message: `already exists: ${input.path}` })
            files.set(input.path, '')
            return null
        }) as WorkspaceEditApplierDeps['createEntry'],
        renameEntry: (async (input: { from: string; to: string }) => {
            calls.push({ fn: 'renameEntry', args: [input] })
            const content = files.get(input.from) ?? ''
            files.delete(input.from)
            files.set(input.to, content)
            return null
        }) as WorkspaceEditApplierDeps['renameEntry'],
        deleteEntry: (async (path: string) => {
            calls.push({ fn: 'deleteEntry', args: [path] })
            files.delete(path)
            return null
        }) as WorkspaceEditApplierDeps['deleteEntry'],
        getActiveProjectId: (async () => {
            mirrorCalls.push({ fn: 'getActiveProjectId', args: [] })
            return FAKE_ACTIVE_PROJECT_ID
        }) as WorkspaceEditApplierDeps['getActiveProjectId'],
        mirrorDirtyExternally: (async (input: { projectId: string; path: string; content: string }) => {
            mirrorCalls.push({ fn: 'mirrorDirtyExternally', args: [input] })
            return null
        }) as WorkspaceEditApplierDeps['mirrorDirtyExternally'],
        ...overrides,
    }

    return { deps, calls, mirrorCalls, files }
}

describe('applyTextEditsToContent', () => {
    test('단일 edit 을 range 만큼 치환한다', () => {
        const edits: TextEdit[] = [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 5 } }, newText: 'howdy' }]
        expect(applyTextEditsToContent('hello world', edits)).toBe('howdy world')
    })

    test('여러 edit 을 배열 순서와 무관하게 오프셋 안전하게 적용한다 (역순 적용)', () => {
        const content = 'const a = 1\nconst b = 2\n'
        const edits: TextEdit[] = [
            { range: { start: { line: 0, character: 6 }, end: { line: 0, character: 7 } }, newText: 'x' },
            { range: { start: { line: 1, character: 6 }, end: { line: 1, character: 7 } }, newText: 'y' },
        ]
        expect(applyTextEditsToContent(content, edits)).toBe('const x = 1\nconst y = 2\n')
    })

    test('CRLF 줄바꿈에서도 character 오프셋을 정확히 계산한다', () => {
        const content = 'line0\r\nline1\r\nline2'
        const edits: TextEdit[] = [{ range: { start: { line: 2, character: 0 }, end: { line: 2, character: 5 } }, newText: 'LINE2' }]
        expect(applyTextEditsToContent(content, edits)).toBe('line0\r\nline1\r\nLINE2')
    })

    test('빈 edit 배열은 원본을 그대로 반환한다', () => {
        expect(applyTextEditsToContent('unchanged', [])).toBe('unchanged')
    })
})

describe('applyWorkspaceEdit — changes', () => {
    test('열린 모델이 있으면 pushEditOperations 로 적용하고 파일 IPC 를 호출하지 않는다', async () => {
        const { model, getEdits } = createFakeModel()
        const { deps, calls } = createFakeDeps()
        /**
         * `/wea-open.ts` (not the more generic `/a.ts`) — `isPeekPreloadedModel` is a module-level
         * singleton shared across every test *file* in a single `bun test` run (bun does not
         * isolate modules per file the way Jest does), and several `peek-model-preload.test.ts`
         * cases mark generic paths like `/a.ts`/`/b.ts` as peek-preloaded with the real (60s)
         * default TTL, which never expires within a test run. A path this test alone owns keeps
         * it independent of that cross-file state.
         */
        const monaco = createFakeMonaco({ 'file:///wea-open.ts': model })

        const edit: WorkspaceEdit = {
            changes: { 'file:///wea-open.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result).toEqual({ applied: true })
        expect(getEdits()).toHaveLength(1)
        expect(calls).toEqual([])
    })

    test('열린 모델이 없으면 읽기 → 치환 → 저장으로 적용한다', async () => {
        const { deps, calls, files } = createFakeDeps()
        files.set('/a.ts', 'const a = 1')
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            changes: { 'file:///a.ts': [{ range: { start: { line: 0, character: 6 }, end: { line: 0, character: 7 } }, newText: 'renamed' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result).toEqual({ applied: true })
        expect(files.get('/a.ts')).toBe('const renamed = 1')
        expect(calls.map((c) => c.fn)).toEqual(['openFile', 'saveFile'])
    })

    test('여러 uri 의 changes 를 전부 적용한다', async () => {
        const { deps, files } = createFakeDeps()
        files.set('/a.ts', 'a')
        files.set('/b.ts', 'b')
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            changes: {
                'file:///a.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'A' }],
                'file:///b.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'B' }],
            },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result).toEqual({ applied: true })
        expect(files.get('/a.ts')).toBe('A')
        expect(files.get('/b.ts')).toBe('B')
    })

    test('저장 실패는 applied:false 와 사유를 반환한다', async () => {
        const { deps } = createFakeDeps({
            openFile: (async () => {
                throw new Error('disk read failed')
            }) as WorkspaceEditApplierDeps['openFile'],
        })
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            changes: { 'file:///a.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, newText: 'x' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result).toEqual({ applied: false, failureReason: 'disk read failed' })
    })
})

describe('applyWorkspaceEdit — documentChanges (resource operations)', () => {
    test('배열 순서대로 create → edit → rename → delete 를 적용한다', async () => {
        const { deps, calls, files } = createFakeDeps()
        files.set('/old.ts', 'unused')
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            documentChanges: [
                { kind: 'create', uri: 'file:///new.ts' },
                {
                    textDocument: { uri: 'file:///new.ts', version: null },
                    edits: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, newText: 'content' }],
                },
                { kind: 'rename', oldUri: 'file:///old.ts', newUri: 'file:///renamed.ts' },
                { kind: 'delete', uri: 'file:///renamed.ts' },
            ],
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result).toEqual({ applied: true })
        expect(calls.map((c) => c.fn)).toEqual(['createEntry', 'openFile', 'saveFile', 'openFile', 'renameEntry', 'deleteEntry'])
        expect(files.has('/renamed.ts')).toBe(false)
        expect(files.get('/new.ts')).toBe('content')
    })

    test('CreateFile 은 기본적으로 이미 존재하면 실패한다', async () => {
        const { deps, files } = createFakeDeps()
        files.set('/a.ts', 'existing')
        const monaco = createFakeMonaco()

        const result = await applyWorkspaceEdit(monaco, { documentChanges: [{ kind: 'create', uri: 'file:///a.ts' }] }, deps)

        expect(result.applied).toBe(false)
    })

    test('CreateFile 의 ignoreIfExists 는 이미 존재해도 성공으로 취급한다', async () => {
        const { deps, files } = createFakeDeps()
        files.set('/a.ts', 'existing')
        const monaco = createFakeMonaco()

        const result = await applyWorkspaceEdit(
            monaco,
            { documentChanges: [{ kind: 'create', uri: 'file:///a.ts', options: { ignoreIfExists: true } }] },
            deps,
        )

        expect(result).toEqual({ applied: true })
        expect(files.get('/a.ts')).toBe('existing')
    })

    test('CreateFile 의 overwrite 는 존재 여부와 무관하게 빈 내용으로 덮어쓴다', async () => {
        const { deps, calls, files } = createFakeDeps()
        files.set('/a.ts', 'existing')
        const monaco = createFakeMonaco()

        const result = await applyWorkspaceEdit(
            monaco,
            { documentChanges: [{ kind: 'create', uri: 'file:///a.ts', options: { overwrite: true } }] },
            deps,
        )

        expect(result).toEqual({ applied: true })
        expect(files.get('/a.ts')).toBe('')
        expect(calls.map((c) => c.fn)).toEqual(['saveFile'])
    })

    test('RenameFile 은 기본적으로 대상이 존재하면 실패하고 renameEntry 를 호출하지 않는다', async () => {
        const { deps, calls, files } = createFakeDeps()
        files.set('/from.ts', 'a')
        files.set('/to.ts', 'b')
        const monaco = createFakeMonaco()

        const result = await applyWorkspaceEdit(
            monaco,
            { documentChanges: [{ kind: 'rename', oldUri: 'file:///from.ts', newUri: 'file:///to.ts' }] },
            deps,
        )

        expect(result.applied).toBe(false)
        expect(calls.some((c) => c.fn === 'renameEntry')).toBe(false)
    })

    test('RenameFile 의 ignoreIfExists 는 대상이 존재하면 아무 것도 하지 않고 성공한다', async () => {
        const { deps, calls, files } = createFakeDeps()
        files.set('/from.ts', 'a')
        files.set('/to.ts', 'b')
        const monaco = createFakeMonaco()

        const result = await applyWorkspaceEdit(
            monaco,
            { documentChanges: [{ kind: 'rename', oldUri: 'file:///from.ts', newUri: 'file:///to.ts', options: { ignoreIfExists: true } }] },
            deps,
        )

        expect(result).toEqual({ applied: true })
        expect(calls.some((c) => c.fn === 'renameEntry')).toBe(false)
        expect(files.get('/to.ts')).toBe('b')
    })

    test('DeleteFile 의 ignoreIfNotExists 는 없는 파일이면 deleteEntry 를 호출하지 않고 성공한다', async () => {
        const { deps, calls } = createFakeDeps()
        const monaco = createFakeMonaco()

        const result = await applyWorkspaceEdit(
            monaco,
            { documentChanges: [{ kind: 'delete', uri: 'file:///missing.ts', options: { ignoreIfNotExists: true } }] },
            deps,
        )

        expect(result).toEqual({ applied: true })
        expect(calls.some((c) => c.fn === 'deleteEntry')).toBe(false)
    })

    test('중간 operation 이 실패하면 이후 operation 은 실행하지 않는다', async () => {
        const { deps, calls, files } = createFakeDeps()
        files.set('/a.ts', 'existing')
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            documentChanges: [
                { kind: 'create', uri: 'file:///a.ts' },
                { kind: 'delete', uri: 'file:///should-not-run.ts' },
            ],
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result.applied).toBe(false)
        expect(calls.some((c) => c.fn === 'deleteEntry')).toBe(false)
    })
})

/** Fake monaco carrying real, mutable models (`getValue`/`setValue`/`isDisposed`) — `createFakeModel` above is push-only and can't stand in for a peek-preload orphan. */
const createFakeMonacoWithLiveModels = () => {
    const models = new Map<string, { value: string; getValue: () => string; setValue: (next: string) => void; isDisposed: () => boolean }>()
    const monaco = {
        Uri: {
            parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }),
            file: (path: string) => ({ fsPath: path, toString: () => `file://${path}` }),
        },
        editor: {
            getModel: (uri: { toString: () => string }) => models.get(uri.toString()) ?? null,
            createModel: (content: string, _languageId: string, uri: { toString: () => string }) => {
                const model = {
                    value: content,
                    getValue: () => model.value,
                    setValue: (next: string) => (model.value = next),
                    isDisposed: () => false,
                }
                models.set(uri.toString(), model)
                return model
            },
            getEditors: () => [] as { getModel: () => unknown }[],
        },
    }
    return monaco as unknown as Monaco
}

describe('applyWorkspaceEdit — 백그라운드 탭(에디터 미부착) 모델 편집은 외부 dirty 로 표시된다', () => {
    test('어떤 에디터에도 부착되지 않은 열린 모델을 편집하면 model-dirty-tracker 에 표시하고 hot-exit 미러도 즉시 기록한다', async () => {
        const { model, getEdits } = createFakeModel('background edit applied')
        const { deps, mirrorCalls } = createFakeDeps()
        const monaco = createFakeMonaco({ 'file:///bg.ts': model })

        const edit: WorkspaceEdit = {
            changes: { 'file:///bg.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result).toEqual({ applied: true })
        expect(getEdits()).toHaveLength(1)
        expect(consumeExternallyDirtyModel('/bg.ts')).toBe(true)
        expect(mirrorCalls).toEqual([
            { fn: 'getActiveProjectId', args: [] },
            { fn: 'mirrorDirtyExternally', args: [{ projectId: FAKE_ACTIVE_PROJECT_ID, path: '/bg.ts', content: 'background edit applied' }] },
        ])
    })

    test('활성 프로젝트가 없으면(모든 프로젝트 종료) hot-exit 미러 기록을 건너뛴다', async () => {
        const { model } = createFakeModel()
        const { deps, mirrorCalls } = createFakeDeps({
            getActiveProjectId: (async () => {
                mirrorCalls.push({ fn: 'getActiveProjectId', args: [] })
                return null
            }) as WorkspaceEditApplierDeps['getActiveProjectId'],
        })
        const monaco = createFakeMonaco({ 'file:///bg-no-project.ts': model })

        const edit: WorkspaceEdit = {
            changes: { 'file:///bg-no-project.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result).toEqual({ applied: true })
        expect(mirrorCalls).toEqual([{ fn: 'getActiveProjectId', args: [] }])
    })

    test('options.projectId 가 주어지면 활성 프로젝트 대신 그 프로젝트로 미러를 기록한다 (세션의 소유 프로젝트 ≠ 전역 활성 프로젝트)', async () => {
        const { model, getEdits } = createFakeModel('owning-project edit')
        const { deps, mirrorCalls } = createFakeDeps()
        const monaco = createFakeMonaco({ 'file:///owning.ts': model })
        const OWNING_PROJECT_ID = 'proj-owning'

        const edit: WorkspaceEdit = {
            changes: { 'file:///owning.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps, { projectId: OWNING_PROJECT_ID })

        expect(result).toEqual({ applied: true })
        expect(getEdits()).toHaveLength(1)
        expect(mirrorCalls).toEqual([
            { fn: 'mirrorDirtyExternally', args: [{ projectId: OWNING_PROJECT_ID, path: '/owning.ts', content: 'owning-project edit' }] },
        ])
    })

    test('현재 에디터에 부착된(포그라운드) 모델을 편집하면 표시하지 않는다 — 자체 onDidChangeModelContent 경로가 이미 추적한다', async () => {
        const { model, getEdits } = createFakeModel()
        const { deps } = createFakeDeps()
        const monaco = {
            Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
            editor: {
                getModel: (uri: { toString: () => string }) => (uri.toString() === 'file:///fg.ts' ? model : null),
                getEditors: () => [{ getModel: () => model }],
            },
        } as unknown as Monaco

        const edit: WorkspaceEdit = {
            changes: { 'file:///fg.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result).toEqual({ applied: true })
        expect(getEdits()).toHaveLength(1)
        expect(consumeExternallyDirtyModel('/fg.ts')).toBe(false)
    })
})

describe('applyWorkspaceEdit — peek 프리로드 orphan 모델은 열린 탭으로 오인하지 않는다', () => {
    test('peek 로 선생성된 orphan 모델은 pushEditOperations 대신 파일 IPC 로 적용되고, 디스크 저장 후 모델 내용도 동기화된다', async () => {
        const monaco = createFakeMonacoWithLiveModels()
        await preloadPeekModel(monaco, '/b.ts', { readFile: async () => ({ content: 'const a = 1', languageId: 'typescript', tier: 'normal' }) })

        const { deps, calls, files } = createFakeDeps()
        files.set('/b.ts', 'const a = 1')

        const edit: WorkspaceEdit = {
            changes: { 'file:///b.ts': [{ range: { start: { line: 0, character: 6 }, end: { line: 0, character: 7 } }, newText: 'renamed' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result).toEqual({ applied: true })
        expect(files.get('/b.ts')).toBe('const renamed = 1')
        expect(calls.map((c) => c.fn)).toEqual(['openFile', 'saveFile'])
        expect(monaco.editor.getModel(monaco.Uri.parse('file:///b.ts'))?.getValue()).toBe('const renamed = 1')
    })
})

describe('applyWorkspaceEdit — refused tier 파일', () => {
    test('refused tier(대용량/바이너리) 파일은 빈 content 위에 편집을 적용하지 않고 실패를 반환한다', async () => {
        const openFileCalls: string[] = []
        const saveFileCalls: { path: string; content: string }[] = []
        const deps: WorkspaceEditApplierDeps = {
            openFile: (async (path: string) => {
                openFileCalls.push(path)
                return { content: '', tier: 'refused' } as never
            }) as WorkspaceEditApplierDeps['openFile'],
            saveFile: (async (input: { path: string; content: string }) => {
                saveFileCalls.push(input)
                return null
            }) as WorkspaceEditApplierDeps['saveFile'],
            createEntry: (async () => null) as WorkspaceEditApplierDeps['createEntry'],
            renameEntry: (async () => null) as WorkspaceEditApplierDeps['renameEntry'],
            deleteEntry: (async () => null) as WorkspaceEditApplierDeps['deleteEntry'],
            getActiveProjectId: (async () => FAKE_ACTIVE_PROJECT_ID) as WorkspaceEditApplierDeps['getActiveProjectId'],
            mirrorDirtyExternally: (async () => null) as WorkspaceEditApplierDeps['mirrorDirtyExternally'],
        }
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            changes: { 'file:///huge.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, newText: 'x' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps)

        expect(result.applied).toBe(false)
        expect(openFileCalls).toEqual(['/huge.ts'])
        expect(saveFileCalls).toEqual([])
    })
})

describe('applyWorkspaceEdit — allowedRoot', () => {
    test('허용된 root 밖의 경로를 대상으로 하면 적용 없이 실패를 반환한다', async () => {
        const { deps, calls } = createFakeDeps()
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            changes: { 'file:///outside/secret.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, newText: 'x' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps, { allowedRoot: '/workspace' })

        expect(result).toEqual({ applied: false, failureReason: 'edit rejected: outside workspace root' })
        expect(calls).toEqual([])
    })

    test('허용된 root 하위 경로는 정상적으로 적용된다', async () => {
        const { deps, files } = createFakeDeps()
        files.set('/workspace/a.ts', 'a')
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            changes: { 'file:///workspace/a.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'A' }] },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps, { allowedRoot: '/workspace' })

        expect(result).toEqual({ applied: true })
        expect(files.get('/workspace/a.ts')).toBe('A')
    })

    test('rename 은 oldUri/newUri 둘 중 하나라도 root 밖이면 renameEntry 를 호출하지 않는다', async () => {
        const { deps, calls, files } = createFakeDeps()
        files.set('/workspace/old.ts', 'x')
        const monaco = createFakeMonaco()

        const result = await applyWorkspaceEdit(
            monaco,
            { documentChanges: [{ kind: 'rename', oldUri: 'file:///workspace/old.ts', newUri: 'file:///outside/new.ts' }] },
            deps,
            { allowedRoot: '/workspace' },
        )

        expect(result.applied).toBe(false)
        expect(calls.some((c) => c.fn === 'renameEntry')).toBe(false)
    })

    test('".." 세그먼트로 문자열 접두사만 흉내낸 경로는 정규화 후에도 root 밖으로 거부된다', async () => {
        const { deps, calls } = createFakeDeps()
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            changes: {
                'file:///workspace/../outside/secret.ts': [
                    { range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, newText: 'x' },
                ],
            },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps, { allowedRoot: '/workspace' })

        expect(result).toEqual({ applied: false, failureReason: 'edit rejected: outside workspace root' })
        expect(calls).toEqual([])
    })

    test('root 하위로 되돌아오는 ".." 는 정규화 후 root 내부로 정확히 인식되어 허용된다', async () => {
        const { deps, files } = createFakeDeps()
        files.set('/workspace/sub/../a.ts', 'a')
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            changes: {
                'file:///workspace/sub/../a.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'A' }],
            },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps, { allowedRoot: '/workspace' })

        expect(result).toEqual({ applied: true })
        expect(files.get('/workspace/sub/../a.ts')).toBe('A')
    })

    test('Windows 스타일 backslash fsPath 도 같은 backslash root 하위면 허용된다 (구분자 불일치로 전량 거부되던 버그)', async () => {
        const { deps, files } = createFakeDeps()
        files.set('C:\\workspace\\a.ts', 'a')
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            changes: {
                'file://C:\\workspace\\a.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'A' }],
            },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps, { allowedRoot: 'C:\\workspace' })

        expect(result).toEqual({ applied: true })
        expect(files.get('C:\\workspace\\a.ts')).toBe('A')
    })

    test('Windows 스타일 경로도 ".." 로 root 를 벗어나면 정규화 후 거부된다', async () => {
        const { deps, calls } = createFakeDeps()
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            changes: {
                'file://C:\\workspace\\..\\outside\\secret.ts': [
                    { range: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, newText: 'x' },
                ],
            },
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps, { allowedRoot: 'C:\\workspace' })

        expect(result).toEqual({ applied: false, failureReason: 'edit rejected: outside workspace root' })
        expect(calls).toEqual([])
    })
})

describe('applyWorkspaceEdit — getDocumentVersion (stale 편집 거절)', () => {
    test('TextDocumentEdit 의 version 이 클라이언트가 추적 중인 버전과 다르면 적용을 거절한다', async () => {
        const { deps, calls } = createFakeDeps()
        const monaco = createFakeMonaco()

        const edit: WorkspaceEdit = {
            documentChanges: [
                {
                    textDocument: { uri: 'file:///wea-version.ts', version: 3 },
                    edits: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' }],
                },
            ],
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps, { getDocumentVersion: () => 5 })

        expect(result.applied).toBe(false)
        expect(calls).toEqual([])
    })

    test('version 이 null 이면(버전 불문 편집) 검증 없이 적용한다', async () => {
        const { model, getEdits } = createFakeModel()
        const { deps } = createFakeDeps()
        const monaco = createFakeMonaco({ 'file:///wea-version.ts': model })

        const edit: WorkspaceEdit = {
            documentChanges: [
                {
                    textDocument: { uri: 'file:///wea-version.ts', version: null },
                    edits: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' }],
                },
            ],
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps, { getDocumentVersion: () => 5 })

        expect(result).toEqual({ applied: true })
        expect(getEdits()).toHaveLength(1)
    })

    test('버전이 일치하면 적용한다', async () => {
        const { model, getEdits } = createFakeModel()
        const { deps } = createFakeDeps()
        const monaco = createFakeMonaco({ 'file:///wea-version.ts': model })

        const edit: WorkspaceEdit = {
            documentChanges: [
                {
                    textDocument: { uri: 'file:///wea-version.ts', version: 5 },
                    edits: [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' }],
                },
            ],
        }
        const result = await applyWorkspaceEdit(monaco, edit, deps, { getDocumentVersion: () => 5 })

        expect(result).toEqual({ applied: true })
        expect(getEdits()).toHaveLength(1)
    })
})
