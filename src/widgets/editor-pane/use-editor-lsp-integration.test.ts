import { describe, expect, mock, test } from 'bun:test'
import type { ProjectLayout } from '@shared/api/bindings'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { WorkspaceEditApplierDeps } from '@shared/lib/lsp/workspace-edit-applier'

/**
 * `use-editor-lsp-integration.ts` transitively imports `@shared/lib/monaco/setup` (real monaco
 * worker bundles bun test cannot load — same constraint `lsp-session-registry.test.ts` documents)
 * and both `@entities/lsp/lsp.ipc`/`@entities/project/project.ipc` (Tauri command bindings this
 * environment cannot invoke). Stubbing all three, then reaching the module under test through a
 * *dynamic* `import()` (not a static one — Bun resolves the whole static import graph, including the
 * offending files, before a same-file `mock.module` call would ever run), is what lets this file load
 * it at all. None of these fakes' functions are ever actually invoked by this file's tests — only
 * `resolveLspSessionRootForSave` (a plain exported function, no React/monaco touched) is under test,
 * with its own `resolveRoot` passed in directly rather than through the mocked IPC.
 */
const FAKE_MONACO = { Uri: { file: (path: string) => ({ toString: () => `file://${path}` }) } }

mock.module('@shared/lib/monaco/setup', () => ({ monaco: FAKE_MONACO }))

mock.module('@entities/lsp/lsp.ipc', () => ({
    spawnLspSession: () => Promise.resolve('fake-session'),
    sendLspMessage: () => Promise.resolve(),
    stopLspSession: () => Promise.resolve(),
    restartLspSession: () => Promise.resolve(),
    confirmLspReinitialize: () => Promise.resolve(),
    listLspSessions: () => Promise.resolve([]),
    detectLspServers: () => Promise.resolve([]),
    resolveLspRoot: () => Promise.resolve(null),
    installLspServer: () => Promise.resolve(),
    cancelLspInstall: () => Promise.resolve(),
    reportLspReinitializeFailure: () => Promise.resolve(),
}))

mock.module('@entities/project/project.ipc', () => ({
    listProjects: () => Promise.resolve([]),
    getProject: () => Promise.resolve(null),
    getActiveProjectId: () => Promise.resolve(null),
    openProject: () => Promise.resolve(null),
    closeProject: () => Promise.resolve(undefined),
    activateProject: () => Promise.resolve(undefined),
    reorderProjects: () => Promise.resolve(undefined),
    listRecentProjects: () => Promise.resolve([]),
    /**
     * Completes the fake's coverage of `project.ipc.ts`'s export surface. `mock.module` is
     * process-global and last-registration-wins, so a partial fake here made
     * `project.query.ts`'s `import { setProjectDisplay }` fail with a `SyntaxError` for every file
     * sharing the run — this file's own 4 tests plus `use-lsp-session.test.ts` and
     * `pane-node-view-welcome.test.tsx`, none of which mock `project.ipc` themselves
     * (`docs/quality-assurance/2026-09-04-test-gap-map.md`, `widgets/**` 부분 실행 깨짐). Masked in a
     * full `bun test` run only because entities load first; a scoped `bun test src/widgets/...` hit
     * it every time.
     */
    setProjectDisplay: () => Promise.resolve(undefined),
    /** Same completeness requirement as `setProjectDisplay` above, for the export d-67 #9 added. */
    forgetRecentProjects: () => Promise.resolve({ removed: 0, skippedWithDrafts: 0, groupsChanged: false }),
}))

const importUseEditorLspIntegration = () => import('@widgets/editor-pane/use-editor-lsp-integration')

describe('resolveLspSessionRootForSave (root-aware 전환, contract 4)', () => {
    test('projectRoot 이 아직 없으면(project 쿼리 미로딩) root 를 결정할 수 없어 null 이다', async () => {
        const { resolveLspSessionRootForSave } = await importUseEditorLspIntegration()
        const result = await resolveLspSessionRootForSave({
            serverId: 'ts-server',
            path: '/proj/a.ts',
            projectRoot: null,
            resolveRoot: () => Promise.resolve('/proj/packages/a'),
        })
        expect(result).toBeNull()
    })

    test('resolveLspRoot 이 값을 돌려주면 projectRoot 대신 그 값을 쓴다 — 다중 루트에서 파일이 속한 워크스페이스 root', async () => {
        const { resolveLspSessionRootForSave } = await importUseEditorLspIntegration()
        const result = await resolveLspSessionRootForSave({
            serverId: 'ts-server',
            path: '/proj/packages/a/src/index.ts',
            projectRoot: '/proj',
            resolveRoot: () => Promise.resolve('/proj/packages/a'),
        })
        expect(result).toBe('/proj/packages/a')
    })

    test('resolveLspRoot 이 null 을 돌려주면 projectRoot 로 폴백한다 — use-lsp-session.ts 의 attachLspSession 과 동일 결정', async () => {
        const { resolveLspSessionRootForSave } = await importUseEditorLspIntegration()
        const result = await resolveLspSessionRootForSave({
            serverId: 'ts-server',
            path: '/proj/a.ts',
            projectRoot: '/proj',
            resolveRoot: () => Promise.resolve(null),
        })
        expect(result).toBe('/proj')
    })

    test('resolveLspRoot 이 실패해도(reject) projectRoot 로 폴백한다', async () => {
        const { resolveLspSessionRootForSave } = await importUseEditorLspIntegration()
        const result = await resolveLspSessionRootForSave({
            serverId: 'ts-server',
            path: '/proj/a.ts',
            projectRoot: '/proj',
            resolveRoot: () => Promise.reject(new Error('boom')),
        })
        expect(result).toBe('/proj')
    })
})

/**
 * Drives the real chain the fix installs — `applyWorkspaceEdit` → `markModelDirtyExternally` →
 * `onModelEditedExternally` → this module's handler — with fakes only at the two real boundaries
 * (monaco, the layout mutation). Everything between is production code, which is the point: the
 * defect (wave-2 #2) was that nothing connected those ends at all.
 */
const importWorkspaceEditApplier = () => import('@shared/lib/lsp/workspace-edit-applier')
const importModelDirtyTracker = () => import('@shared/lib/lsp/model-dirty-tracker')

const FILE_TAB = (id: string, path: string, dirty?: boolean) => ({ id, kind: { kind: 'file' as const, path }, title: id, dirty })

const createLayoutWithBackgroundTabs = () =>
    ({
        version: 1,
        focusedPane: 'pane-main',
        root: {
            node: 'leaf',
            id: 'pane-main',
            active: 'tab-front',
            tabs: [
                FILE_TAB('tab-bg', '/bg.ts'),
                FILE_TAB('tab-bg-already-dirty', '/bg.ts', true),
                FILE_TAB('tab-front', '/front.ts'),
                { id: 'tab-terminal', kind: { kind: 'terminal' as const, sessionId: 'session-1' }, title: 'zsh' },
            ],
        },
        auxiliaryWindows: [
            {
                slot: 1,
                focusedPane: 'pane-aux',
                root: { node: 'leaf', id: 'pane-aux', active: 'tab-bg-aux', tabs: [FILE_TAB('tab-bg-aux', '/bg.ts')] },
            },
        ],
    }) as unknown as ProjectLayout

/** The applier only reaches `getActiveProjectId`/`mirrorDirtyExternally` on the background-model path; the remaining file-IPC deps would be a test failure if they were ever called. */
const createApplierDeps = () => {
    const unreachable = () => Promise.reject(new Error('unexpected file IPC call'))
    return {
        openFile: unreachable,
        saveFile: unreachable,
        createEntry: unreachable,
        renameEntry: unreachable,
        deleteEntry: unreachable,
        getActiveProjectId: () => Promise.resolve('proj-1'),
        mirrorDirtyExternally: () => Promise.resolve(null),
    } as unknown as WorkspaceEditApplierDeps
}

const createApplierMonaco = (openModels: Record<string, { pushEditOperations: () => null; getValue: () => string }>) =>
    ({
        Uri: { parse: (uri: string) => ({ fsPath: uri.replace('file://', ''), toString: () => uri }) },
        editor: {
            getModel: (uri: { toString: () => string }) => openModels[uri.toString()] ?? null,
            getEditors: () => [],
        },
    }) as unknown as Monaco

describe('createExternalModelEditTabDirtyHandler — 백그라운드 탭에 착지한 WorkspaceEdit 은 탭 dirty 를 세운다 (wave-2 #2)', () => {
    test('WorkspaceEdit 이 미부착 모델에 적용되면 그 경로의 모든 file 탭(보조 창 포함)에 setTabDirty(true) 를 보낸다', async () => {
        const { createExternalModelEditTabDirtyHandler } = await importUseEditorLspIntegration()
        const { applyWorkspaceEdit } = await importWorkspaceEditApplier()
        const { onModelEditedExternally, consumeExternallyDirtyModel } = await importModelDirtyTracker()

        const calls: { tabId: string; dirty: boolean }[] = []
        const unsubscribe = onModelEditedExternally(
            createExternalModelEditTabDirtyHandler({
                getLayout: () => createLayoutWithBackgroundTabs(),
                setTabDirty: (input) => calls.push(input),
            }),
        )

        const monaco = createApplierMonaco({ 'file:///bg.ts': { pushEditOperations: () => null, getValue: () => 'edited' } })
        const edit = { changes: { 'file:///bg.ts': [{ range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } }, newText: 'x' }] } }
        const result = await applyWorkspaceEdit(monaco, edit, createApplierDeps())
        unsubscribe()
        consumeExternallyDirtyModel('/bg.ts')

        expect(result).toEqual({ applied: true })
        expect(calls).toEqual([
            { tabId: 'tab-bg', dirty: true },
            { tabId: 'tab-bg-aux', dirty: true },
        ])
    })

    test('이미 dirty 인 탭·다른 경로·터미널 탭은 건드리지 않고, 레이아웃 캐시가 비어 있으면 아무것도 보내지 않는다', async () => {
        const { createExternalModelEditTabDirtyHandler } = await importUseEditorLspIntegration()

        const otherPathCalls: { tabId: string; dirty: boolean }[] = []
        createExternalModelEditTabDirtyHandler({
            getLayout: () => createLayoutWithBackgroundTabs(),
            setTabDirty: (input) => otherPathCalls.push(input),
        })('/never-open.ts')
        expect(otherPathCalls).toEqual([])

        const emptyCacheCalls: { tabId: string; dirty: boolean }[] = []
        createExternalModelEditTabDirtyHandler({ getLayout: () => undefined, setTabDirty: (input) => emptyCacheCalls.push(input) })('/bg.ts')
        expect(emptyCacheCalls).toEqual([])
    })
})
