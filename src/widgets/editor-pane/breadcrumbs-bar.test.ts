import { describe, expect, mock, test } from 'bun:test'
import type { LspServerId, ProjectId } from '@shared/api/bindings'

/**
 * `@shared/lib/monaco/setup` pulls in real monaco-editor worker bundles (`?worker` imports) that
 * `bun test` cannot resolve at all — `lsp-session-registry.test.ts` documents and works around the
 * same issue. `@entities/lsp/lsp.ipc` re-exports Tauri command bindings this environment cannot load
 * either. `breadcrumbs-bar.tsx` transitively touches both (directly, and via
 * `lsp-session-registry.ts`/`@features/editor/breadcrumb-segment`'s own imports), so both are faked
 * here for the same reasons those other test files fake them, and the module under test is reached
 * through a *dynamic* `import()` so the fakes are registered before its static import graph resolves.
 */
mock.module('@shared/lib/monaco/setup', () => ({
    monaco: { Uri: { file: (path: string) => ({ toString: () => `file://${path}` }), parse: (uri: string) => ({ toString: () => uri }) } },
}))
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

const importBreadcrumbsBar = () => import('@widgets/editor-pane/breadcrumbs-bar')

const PROJECT_ID = 'project-1' as ProjectId
const SERVER_A = 'server-a' as LspServerId
const SERVER_B = 'server-b' as LspServerId

describe('buildDocumentSymbolWaiters (breadcrumbs-bar) — root-aware 소비처 전환 (contract §1.2)', () => {
    test('서버별로 resolveLspRoot 가 반환한 루트를 그대로 waitForSession 에 넘긴다(다중 루트 정확한 선택)', async () => {
        const { buildDocumentSymbolWaiters } = await importBreadcrumbsBar()
        const waitCalls: { projectId: ProjectId; serverId: LspServerId; root: string }[] = []

        const waiters = await buildDocumentSymbolWaiters({
            availableServerIds: [SERVER_A, SERVER_B],
            path: '/repo/packages/a/index.ts',
            projectId: PROJECT_ID,
            fallbackRoot: '/repo',
            isCancelled: () => false,
            resolveRoot: ({ serverId }) => Promise.resolve(serverId === SERVER_A ? '/repo/packages/a' : null),
            waitForSession: (projectId, serverId, root) => {
                waitCalls.push({ projectId, serverId, root })
                return { promise: Promise.resolve(null), cancel: () => {} }
            },
        })

        expect(waitCalls).toEqual([
            { projectId: PROJECT_ID, serverId: SERVER_A, root: '/repo/packages/a' },
            { projectId: PROJECT_ID, serverId: SERVER_B, root: '/repo' },
        ])
        expect(waiters).toHaveLength(2)
    })

    test('resolveLspRoot 이 거부되거나 null 이고 fallbackRoot 도 없으면 그 서버는 건너뛴다(엉뚱한 세션 대기 방지)', async () => {
        const { buildDocumentSymbolWaiters } = await importBreadcrumbsBar()
        const waitCalls: LspServerId[] = []

        const waiters = await buildDocumentSymbolWaiters({
            availableServerIds: [SERVER_A, SERVER_B],
            path: '/repo/index.ts',
            projectId: PROJECT_ID,
            fallbackRoot: undefined,
            isCancelled: () => false,
            resolveRoot: ({ serverId }) => (serverId === SERVER_A ? Promise.reject(new Error('resolve failed')) : Promise.resolve(null)),
            waitForSession: (_projectId, serverId, _root) => {
                waitCalls.push(serverId)
                return { promise: Promise.resolve(null), cancel: () => {} }
            },
        })

        expect(waitCalls).toEqual([])
        expect(waiters).toEqual([])
    })

    test('루트 해석 도중 취소되면 waitForSession 을 전혀 호출하지 않는다(대기자 누수 방지)', async () => {
        const { buildDocumentSymbolWaiters } = await importBreadcrumbsBar()
        const waitCalls: LspServerId[] = []

        const waiters = await buildDocumentSymbolWaiters({
            availableServerIds: [SERVER_A],
            path: '/repo/index.ts',
            projectId: PROJECT_ID,
            fallbackRoot: '/repo',
            isCancelled: () => true,
            resolveRoot: () => Promise.resolve('/repo'),
            waitForSession: (_projectId, serverId) => {
                waitCalls.push(serverId)
                return { promise: Promise.resolve(null), cancel: () => {} }
            },
        })

        expect(waitCalls).toEqual([])
        expect(waiters).toEqual([])
    })
})
