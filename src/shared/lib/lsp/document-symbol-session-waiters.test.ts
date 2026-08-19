import { describe, expect, test } from 'bun:test'
import type { LspServerId, ProjectId } from '@shared/api/bindings'
import { buildDocumentSymbolWaiters } from '@shared/lib/lsp/document-symbol-session-waiters'

const PROJECT_ID = 'project-1' as ProjectId
const SERVER_A = 'server-a' as LspServerId
const SERVER_B = 'server-b' as LspServerId

describe('buildDocumentSymbolWaiters — root-aware 소비처 공통화 (contract §1.2, breadcrumbs/outline/palette 공용)', () => {
    test('서버별로 resolveLspRoot 가 반환한 루트를 그대로 waitForSession 에 넘긴다(다중 루트 정확한 선택)', async () => {
        const waitCalls: { projectId: ProjectId; serverId: LspServerId; root: string }[] = []

        const waiters = await buildDocumentSymbolWaiters<null>({
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
        const waitCalls: LspServerId[] = []

        const waiters = await buildDocumentSymbolWaiters<null>({
            availableServerIds: [SERVER_A, SERVER_B],
            path: '/repo/index.ts',
            projectId: PROJECT_ID,
            fallbackRoot: undefined,
            isCancelled: () => false,
            resolveRoot: ({ serverId }) => (serverId === SERVER_A ? Promise.reject(new Error('resolve failed')) : Promise.resolve(null)),
            waitForSession: (_projectId, serverId) => {
                waitCalls.push(serverId)
                return { promise: Promise.resolve(null), cancel: () => {} }
            },
        })

        expect(waitCalls).toEqual([])
        expect(waiters).toEqual([])
    })

    test('루트 해석 도중 취소되면 waitForSession 을 전혀 호출하지 않는다(대기자 누수 방지)', async () => {
        const waitCalls: LspServerId[] = []

        const waiters = await buildDocumentSymbolWaiters<null>({
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
