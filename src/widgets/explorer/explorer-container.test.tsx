import { afterAll, beforeAll, describe, expect, mock, test } from 'bun:test'
import type { Project, TreeRowPage } from '@shared/api/bindings'
import type { SearchPanelRequest } from '@shared/lib/bridge/search-panel-bridge'
import { subscribeOpenSearchPanel } from '@shared/lib/bridge/search-panel-bridge'
import { TooltipProvider } from '@shared/ui/tooltip'
import { createTestQueryClient, fireEvent, renderWithProviders, screen, waitFor } from '@shared/testing/render'

/**
 * d-67 #14/#15 — a project whose folder is gone (an unplugged drive restored from session, or a
 * root that disappeared mid-run) rendered as an ordinary *empty* tree: `tree_rows` fails and the
 * container fell back to `page?.rows ?? []`, so nothing on screen distinguished "the folder is
 * missing" from "the folder is empty", and the one recovery — close and reopen, which is what
 * re-attaches the watcher and git capability — was never offered.
 *
 * `@shared/lib/monaco/setup` is faked because the explorer's outline view reaches monaco's
 * `SymbolKind` table at import time, and the two `.ipc` modules are the container's data sources.
 * `mock.module` is process-global and last-registration-wins (`docs/memory/test-conventions.md`
 * §3), so each fake covers its module's entire export surface.
 */
mock.module('@shared/lib/monaco/setup', () => ({
    monaco: {
        Uri: { file: () => ({ toString: () => '' }) },
        editor: {},
        languages: { SymbolKind: {} },
        MarkerSeverity: {},
    },
}))

const PROJECT_ID = 'project-1'
const PROJECT_ROOT = '/volumes/external/project-1'
const IGNORED_DIR_NAME = 'node_modules'

const rejectLikeUnavailableIpc = () => Promise.reject(new Error('ipc unavailable under bun:test'))

const treeRowsOutcome: { current: () => Promise<TreeRowPage> } = { current: () => Promise.resolve({ rows: [], total: 0 }) }
const project: { current: Project } = { current: { id: PROJECT_ID, root: PROJECT_ROOT, name: 'project-1' } }
const closedProjectIds: string[] = []
const openedRoots: string[] = []

mock.module('@entities/tree/tree.ipc', () => ({
    getTreeRows: () => treeRowsOutcome.current(),
    toggleTreeNode: rejectLikeUnavailableIpc,
    collapseAllTreeNodes: rejectLikeUnavailableIpc,
    revealTreeNode: rejectLikeUnavailableIpc,
    refreshTreeDir: rejectLikeUnavailableIpc,
}))

mock.module('@entities/project/project.ipc', () => ({
    listProjects: () => Promise.resolve([]),
    listRecentProjects: () => Promise.resolve([]),
    forgetRecentProjects: rejectLikeUnavailableIpc,
    getProject: () => Promise.resolve(project.current),
    getActiveProjectId: () => Promise.resolve(null),
    openProject: (root: string) => {
        openedRoots.push(root)
        return Promise.resolve(null)
    },
    closeProject: (projectId: string) => {
        closedProjectIds.push(projectId)
        return Promise.resolve(undefined)
    },
    activateProject: rejectLikeUnavailableIpc,
    reorderProjects: rejectLikeUnavailableIpc,
    setProjectDisplay: rejectLikeUnavailableIpc,
}))

const importExplorerContainer = () => import('@widgets/explorer/explorer-container')

const renderExplorer = async () => {
    const { ExplorerContainer } = await importExplorerContainer()
    const queryClient = createTestQueryClient()
    renderWithProviders(
        <TooltipProvider>
            <ExplorerContainer projectId={PROJECT_ID} zen={false} sidebarCollapsed={false} />
        </TooltipProvider>,
        { queryClient },
    )
}

describe('ExplorerContainer 프로젝트 폴더 부재', () => {
    test('복원된 프로젝트의 root_missing 이면 전용 빈 상태와 다시 열기를 그린다', async () => {
        project.current = { id: PROJECT_ID, root: PROJECT_ROOT, name: 'project-1', rootMissing: true }
        treeRowsOutcome.current = () => Promise.resolve({ rows: [], total: 0 })

        await renderExplorer()

        await waitFor(() => expect(screen.getByText('explorer.projectRootMissing')).toBeTruthy())
        expect(screen.getByText(PROJECT_ROOT)).toBeTruthy()
        expect(screen.getByRole('button', { name: 'explorer.reopenProject' })).toBeTruthy()
    })

    test('세션 도중 폴더가 사라져 tree_rows 가 실패해도 같은 빈 상태를 그린다 (복원 플래그는 갱신되지 않는다)', async () => {
        project.current = { id: PROJECT_ID, root: PROJECT_ROOT, name: 'project-1' }
        treeRowsOutcome.current = () => Promise.reject(new Error('read_dir failed'))

        await renderExplorer()

        await waitFor(() => expect(screen.getByText('explorer.projectRootMissing')).toBeTruthy())
    })

    test('다시 열기는 프로젝트를 닫고 같은 root 로 다시 연다 — 워처·git 능력이 그때 붙는다', async () => {
        project.current = { id: PROJECT_ID, root: PROJECT_ROOT, name: 'project-1', rootMissing: true }
        treeRowsOutcome.current = () => Promise.resolve({ rows: [], total: 0 })
        const closedBefore = closedProjectIds.length
        const openedBefore = openedRoots.length

        await renderExplorer()
        await waitFor(() => expect(screen.getByRole('button', { name: 'explorer.reopenProject' })).toBeTruthy())
        fireEvent.click(screen.getByRole('button', { name: 'explorer.reopenProject' }))

        await waitFor(() => expect(openedRoots.length).toBe(openedBefore + 1))
        expect(closedProjectIds.slice(closedBefore)).toEqual([PROJECT_ID])
        expect(openedRoots.at(-1)).toBe(PROJECT_ROOT)
    })

    test('폴더가 멀쩡하면 빈 상태를 그리지 않는다', async () => {
        project.current = { id: PROJECT_ID, root: PROJECT_ROOT, name: 'project-1' }
        treeRowsOutcome.current = () =>
            Promise.resolve({
                rows: [{ path: `${PROJECT_ROOT}/src`, name: 'src', kind: 'directory', depth: 0, expanded: false, hasChildren: true }],
                total: 1,
            })

        await renderExplorer()

        await waitFor(() => expect(screen.getByRole('button', { name: 'explorer.refresh' })).toBeTruthy())
        expect(screen.queryByText('explorer.projectRootMissing')).toBeNull()
        expect(screen.queryByRole('button', { name: 'explorer.reopenProject' })).toBeNull()
    })
})

const VIRTUAL_VIEWPORT_SIZE_PX = 600

/**
 * `@tanstack/virtual-core` measures its scroll window from `offsetWidth`/`offsetHeight`, which
 * happy-dom answers `0` for on every element because it runs no layout — so the file tree renders
 * its total height and not a single row, and nothing is there to right-click. Redefining the two
 * getters for the duration of this block is the same class of layout-free-DOM stand-in
 * `docs/memory/test-conventions.md` §5 prescribes for xterm's canvas measuring pass.
 */
const overrideOffsetSize = (value: number) => {
    Object.defineProperty(HTMLElement.prototype, 'offsetWidth', { configurable: true, get: () => value })
    Object.defineProperty(HTMLElement.prototype, 'offsetHeight', { configurable: true, get: () => value })
}

/**
 * d-67 #23 — "폴더에서 찾기" 가 include glob 을 보내면 walk 가 `IGNORED_DIR_NAMES` 를 먼저
 * 가지치기해 버려 `node_modules` 같은 폴더를 지정한 검색이 언제나 거짓 0건을 돌려줬다. 사용자가
 * 직접 가리킨 폴더는 glob 필터가 아니라 walk 루트를 옮기는 `scopeDir` 로 내려가야 한다.
 */
describe('ExplorerContainer 폴더에서 찾기', () => {
    const nativeOffsetWidth = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'offsetWidth')
    const nativeOffsetHeight = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'offsetHeight')

    beforeAll(() => overrideOffsetSize(VIRTUAL_VIEWPORT_SIZE_PX))

    afterAll(() => {
        if (nativeOffsetWidth) Object.defineProperty(HTMLElement.prototype, 'offsetWidth', nativeOffsetWidth)
        if (nativeOffsetHeight) Object.defineProperty(HTMLElement.prototype, 'offsetHeight', nativeOffsetHeight)
    })

    test('디렉토리 행은 includeGlob 이 아니라 프로젝트 상대 scopeDir 로 검색 패널을 연다', async () => {
        project.current = { id: PROJECT_ID, root: PROJECT_ROOT, name: 'project-1' }
        treeRowsOutcome.current = () =>
            Promise.resolve({
                rows: [
                    {
                        path: `${PROJECT_ROOT}/${IGNORED_DIR_NAME}`,
                        name: IGNORED_DIR_NAME,
                        kind: 'directory',
                        depth: 0,
                        expanded: false,
                        hasChildren: true,
                    },
                ],
                total: 1,
            })
        let received: SearchPanelRequest | null = null
        const unsubscribe = subscribeOpenSearchPanel((request) => {
            received = request
        })

        try {
            await renderExplorer()
            const row = await screen.findByRole('treeitem', { name: IGNORED_DIR_NAME })
            fireEvent.contextMenu(row)
            fireEvent.click(await screen.findByRole('menuitem', { name: 'explorer.findInFolder' }))

            await waitFor(() => expect(received).not.toBeNull())
            expect(received).toMatchObject({ scopeDir: IGNORED_DIR_NAME, includeGlob: null })
        } finally {
            unsubscribe()
        }
    })
})
