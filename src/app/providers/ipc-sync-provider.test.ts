import { describe, expect, test } from 'bun:test'
import type { FsChange, ProjectLayout, Tab } from '@shared/api/bindings'

const buildFileTab = (id: string, path: string): Tab => ({ id, kind: { kind: 'file', path }, title: id })

const buildLeafLayout = (tabs: Tab[]): ProjectLayout => ({
    version: 1,
    root: { node: 'leaf', id: 'leaf', tabs, active: tabs[0]?.id ?? null },
    focusedPane: 'leaf',
})

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

describe('filePathQueryKeysToInvalidate', () => {
    test('경로 하나에 대해 FILE.CONTENT 와 FILE.RAW 쿼리키를 모두 반환한다', async () => {
        const { filePathQueryKeysToInvalidate } = await import('@app/providers/ipc-sync-provider')
        expect(filePathQueryKeysToInvalidate('/repo/a.png')).toEqual([
            ['file', 'content', '/repo/a.png'],
            ['file', 'raw', '/repo/a.png'],
        ])
    })
})

describe('isGitWorktreeQueryForChangedPaths', () => {
    test('GUTTER 스코프이고 path 가 changedPaths 에 있으면 true 다', async () => {
        const { isGitWorktreeQueryForChangedPaths } = await import('@app/providers/ipc-sync-provider')
        expect(isGitWorktreeQueryForChangedPaths(['git', 'project-1', 'gutter', '/repo/a.ts'], new Set(['/repo/a.ts']))).toBe(true)
    })

    test('DIFF 스코프이고 path 가 changedPaths 에 있으면 mode 축과 무관하게 true 다', async () => {
        const { isGitWorktreeQueryForChangedPaths } = await import('@app/providers/ipc-sync-provider')
        expect(isGitWorktreeQueryForChangedPaths(['git', 'project-1', 'diff', '/repo/a.ts', 'workdirVsIndex'], new Set(['/repo/a.ts']))).toBe(true)
        expect(isGitWorktreeQueryForChangedPaths(['git', 'project-1', 'diff', '/repo/a.ts', 'indexVsHead'], new Set(['/repo/a.ts']))).toBe(true)
    })

    test('path 가 changedPaths 에 없으면 false 다', async () => {
        const { isGitWorktreeQueryForChangedPaths } = await import('@app/providers/ipc-sync-provider')
        expect(isGitWorktreeQueryForChangedPaths(['git', 'project-1', 'gutter', '/repo/other.ts'], new Set(['/repo/a.ts']))).toBe(false)
    })

    test('GUTTER/DIFF 가 아닌 스코프(status·log 등)는 path 매치 여부와 무관하게 false 다', async () => {
        const { isGitWorktreeQueryForChangedPaths } = await import('@app/providers/ipc-sync-provider')
        expect(isGitWorktreeQueryForChangedPaths(['git', 'project-1', 'status'], new Set(['/repo/a.ts']))).toBe(false)
        expect(isGitWorktreeQueryForChangedPaths(['git', 'project-1', 'log'], new Set())).toBe(false)
    })

    test('타 프로젝트 키도 스코프·path 만 맞으면 true 다 — projectId 축 배제는 GIT.PROJECT 접두사와의 predicate AND 결합이 담당하므로 이 헬퍼 자체는 검사하지 않는다', async () => {
        const { isGitWorktreeQueryForChangedPaths } = await import('@app/providers/ipc-sync-provider')
        expect(isGitWorktreeQueryForChangedPaths(['git', 'other-project', 'gutter', '/repo/a.ts'], new Set(['/repo/a.ts']))).toBe(true)
    })
})

describe('isQueryKeyUnderProjectRoot', () => {
    test('FILE.CONTENT/FILE.RAW 접두사이면서 프로젝트 root 이하 경로면 true 다', async () => {
        const { isQueryKeyUnderProjectRoot } = await import('@app/providers/ipc-sync-provider')
        expect(isQueryKeyUnderProjectRoot(['file', 'content', '/repo/src/a.ts'], '/repo')).toBe(true)
        expect(isQueryKeyUnderProjectRoot(['file', 'raw', '/repo/README.md'], '/repo')).toBe(true)
        expect(isQueryKeyUnderProjectRoot(['file', 'content', '/repo'], '/repo')).toBe(true)
    })

    test('접두사가 다르면(FILE.MIRRORS 등) false 다', async () => {
        const { isQueryKeyUnderProjectRoot } = await import('@app/providers/ipc-sync-provider')
        expect(isQueryKeyUnderProjectRoot(['file', 'mirrors', 'project-1'], '/repo')).toBe(false)
        expect(isQueryKeyUnderProjectRoot(['tree', 'rows', 'project-1'], '/repo')).toBe(false)
    })

    test('root 를 문자열 접두사로만 공유하는 형제 디렉토리는 false 다 (/repo-other 는 /repo 의 하위가 아니다)', async () => {
        const { isQueryKeyUnderProjectRoot } = await import('@app/providers/ipc-sync-provider')
        expect(isQueryKeyUnderProjectRoot(['file', 'content', '/repo-other/a.ts'], '/repo')).toBe(false)
    })

    test('path 위치가 문자열이 아니면 false 다', async () => {
        const { isQueryKeyUnderProjectRoot } = await import('@app/providers/ipc-sync-provider')
        expect(isQueryKeyUnderProjectRoot(['file', 'content', 42], '/repo')).toBe(false)
    })
})

describe('isSelfEchoWithoutTreeImpact', () => {
    const change = (overrides: Partial<FsChange>): FsChange => ({ kind: 'modified', paths: ['/repo/a.ts'], fromApp: true, ...overrides })

    test('fromApp 이면서 kind 가 modified 면 트리 영향 없는 자기 에코다 (스킵 대상)', async () => {
        const { isSelfEchoWithoutTreeImpact } = await import('@app/providers/ipc-sync-provider')
        expect(isSelfEchoWithoutTreeImpact(change({ kind: 'modified', fromApp: true }))).toBe(true)
    })

    test.each(['created', 'renamed', 'removed'] as const)('fromApp 이어도 kind 가 %s 면 트리 구조가 바뀌므로 스킵하지 않는다', async (kind) => {
        const { isSelfEchoWithoutTreeImpact } = await import('@app/providers/ipc-sync-provider')
        expect(isSelfEchoWithoutTreeImpact(change({ kind, fromApp: true }))).toBe(false)
    })

    test('fromApp 이 false 면 kind 와 무관하게 스킵하지 않는다', async () => {
        const { isSelfEchoWithoutTreeImpact } = await import('@app/providers/ipc-sync-provider')
        expect(isSelfEchoWithoutTreeImpact(change({ kind: 'modified', fromApp: false }))).toBe(false)
    })
})

describe('collectOpenFilePathsOutsideProject', () => {
    test('닫힌 프로젝트 자신의 레이아웃 엔트리는 제외하고 다른 프로젝트의 열린 파일 경로만 모은다', async () => {
        const { collectOpenFilePathsOutsideProject } = await import('@app/providers/ipc-sync-provider')
        const entries: ReadonlyArray<readonly [readonly unknown[], ProjectLayout | undefined]> = [
            [['layout', 'detail', 'closing-project'], buildLeafLayout([buildFileTab('a', '/closing/a.ts')])],
            [['layout', 'detail', 'other-project'], buildLeafLayout([buildFileTab('b', '/other/b.ts')])],
        ]

        expect(collectOpenFilePathsOutsideProject(entries, 'closing-project')).toEqual(['/other/b.ts'])
    })

    test('보조 창(auxiliaryWindows)에 열린 파일 경로도 함께 모은다', async () => {
        const { collectOpenFilePathsOutsideProject } = await import('@app/providers/ipc-sync-provider')
        const layout: ProjectLayout = {
            ...buildLeafLayout([buildFileTab('main', '/other/main.ts')]),
            auxiliaryWindows: [
                {
                    slot: 1,
                    focusedPane: 'aux-leaf',
                    root: { node: 'leaf', id: 'aux-leaf', tabs: [buildFileTab('aux', '/other/aux.ts')], active: 'aux' },
                },
            ],
        }
        const entries: ReadonlyArray<readonly [readonly unknown[], ProjectLayout | undefined]> = [[['layout', 'detail', 'other-project'], layout]]

        expect(collectOpenFilePathsOutsideProject(entries, 'closing-project')).toEqual(['/other/main.ts', '/other/aux.ts'])
    })

    test('파일이 아닌 탭 종류(터미널 등)는 제외한다', async () => {
        const { collectOpenFilePathsOutsideProject } = await import('@app/providers/ipc-sync-provider')
        const terminalTab: Tab = { id: 't', kind: { kind: 'terminal', sessionId: 's1' }, title: 'term' }
        const entries: ReadonlyArray<readonly [readonly unknown[], ProjectLayout | undefined]> = [
            [['layout', 'detail', 'other-project'], buildLeafLayout([terminalTab, buildFileTab('f', '/other/f.ts')])],
        ]

        expect(collectOpenFilePathsOutsideProject(entries, 'closing-project')).toEqual(['/other/f.ts'])
    })

    test('아직 데이터가 없는(undefined) 레이아웃 엔트리는 건너뛴다', async () => {
        const { collectOpenFilePathsOutsideProject } = await import('@app/providers/ipc-sync-provider')
        const entries: ReadonlyArray<readonly [readonly unknown[], ProjectLayout | undefined]> = [[['layout', 'detail', 'other-project'], undefined]]

        expect(collectOpenFilePathsOutsideProject(entries, 'closing-project')).toEqual([])
    })
})
