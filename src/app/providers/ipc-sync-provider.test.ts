import { describe, expect, test } from 'bun:test'

/**
 * `ipc-sync-provider.tsx` must reach LSP session disposal through `lsp-session-flush-registry.ts`
 * rather than importing `lsp-session-registry.ts` directly — that module pulls in real monaco
 * worker bundles (`?worker` imports) that only Vite's dev/build pipeline can resolve, which makes
 * `bun test`'s static import-graph resolution fail for anything that imports it, this provider
 * included. A dynamic `import()` here is enough to catch a regression back to the direct import:
 * Bun resolves the whole static import graph up front, so a `Missing 'default' export ...
 * ts.worker.js?worker'` `SyntaxError` surfaces at import time, before any test body runs.
 */
describe('IpcSyncProvider 모듈 로드', () => {
    test('lsp-session-flush-registry 간접 참조라 monaco worker 를 정적 임포트 그래프에 끌어들이지 않는다', async () => {
        const imported = await import('@app/providers/ipc-sync-provider')
        expect(typeof imported.IpcSyncProvider).toBe('function')
    })
})

describe('syncTreeRowsForChangedDirs', () => {
    test('dirs 가 빈 배열이면 아무 것도 호출하지 않는다', async () => {
        const { syncTreeRowsForChangedDirs } = await import('@app/providers/ipc-sync-provider')
        const calls: string[] = []
        const setTreeRows = () => calls.push('setTreeRows')
        const invalidateTreeRows = () => calls.push('invalidateTreeRows')

        await syncTreeRowsForChangedDirs([], { refreshTreeDir: () => Promise.resolve({ rows: [], total: 0 }), setTreeRows, invalidateTreeRows })

        expect(calls).toEqual([])
    })

    test('디렉토리 1개가 성공하면 반환된 page 를 그대로 캐시에 쓴다 (N+1 회피)', async () => {
        const { syncTreeRowsForChangedDirs } = await import('@app/providers/ipc-sync-provider')
        const page = { rows: [], total: 0 }
        const refreshedDirs: string[] = []
        let written: unknown
        let invalidated = false

        await syncTreeRowsForChangedDirs(['/a'], {
            refreshTreeDir: (dir) => {
                refreshedDirs.push(dir)
                return Promise.resolve(page)
            },
            setTreeRows: (p) => {
                written = p
            },
            invalidateTreeRows: () => {
                invalidated = true
            },
        })

        expect(refreshedDirs).toEqual(['/a'])
        expect(written).toBe(page)
        expect(invalidated).toBe(false)
    })

    test('디렉토리 1개가 실패하면 캐시를 직접 쓰지 않고 무효화로 폴백한다', async () => {
        const { syncTreeRowsForChangedDirs } = await import('@app/providers/ipc-sync-provider')
        let written: unknown
        let invalidated = false

        await syncTreeRowsForChangedDirs(['/a'], {
            refreshTreeDir: () => Promise.reject(new Error('disk error')),
            setTreeRows: (p) => {
                written = p
            },
            invalidateTreeRows: () => {
                invalidated = true
            },
        })

        expect(written).toBeUndefined()
        expect(invalidated).toBe(true)
    })

    test('디렉토리 여러 개는 완료 순서를 임의로 채택하지 않고, 전건을 refresh 한 뒤 무효화로 정리한다', async () => {
        const { syncTreeRowsForChangedDirs } = await import('@app/providers/ipc-sync-provider')
        const refreshedDirs: string[] = []
        let written: unknown
        let invalidated = false

        await syncTreeRowsForChangedDirs(['/a', '/b', '/c'], {
            refreshTreeDir: (dir) => {
                refreshedDirs.push(dir)
                return Promise.resolve({ rows: [], total: 0 })
            },
            setTreeRows: (p) => {
                written = p
            },
            invalidateTreeRows: () => {
                invalidated = true
            },
        })

        expect(refreshedDirs.sort()).toEqual(['/a', '/b', '/c'])
        expect(written).toBeUndefined()
        expect(invalidated).toBe(true)
    })

    test('디렉토리 여러 개 중 일부가 실패해도 나머지 refresh 는 모두 시도된 뒤 무효화한다 (조기 reject 로 단축되지 않는다)', async () => {
        const { syncTreeRowsForChangedDirs } = await import('@app/providers/ipc-sync-provider')
        const refreshedDirs: string[] = []
        let invalidated = false

        await syncTreeRowsForChangedDirs(['/a', '/b'], {
            refreshTreeDir: (dir) => {
                refreshedDirs.push(dir)
                return dir === '/a' ? Promise.reject(new Error('disk error')) : Promise.resolve({ rows: [], total: 0 })
            },
            setTreeRows: () => undefined,
            invalidateTreeRows: () => {
                invalidated = true
            },
        })

        expect(refreshedDirs.sort()).toEqual(['/a', '/b'])
        expect(invalidated).toBe(true)
    })
})
