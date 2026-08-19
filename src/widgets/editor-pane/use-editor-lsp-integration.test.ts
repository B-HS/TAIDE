import { describe, expect, mock, test } from 'bun:test'

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
}))

mock.module('@entities/project/project.ipc', () => ({
    listProjects: () => Promise.resolve([]),
    getProject: () => Promise.resolve(null),
    getActiveProjectId: () => Promise.resolve(null),
    openProject: () => Promise.resolve(null),
    closeProject: () => Promise.resolve(undefined),
    activateProject: () => Promise.resolve(undefined),
    reorderProjects: () => Promise.resolve(undefined),
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
