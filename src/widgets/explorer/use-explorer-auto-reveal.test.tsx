import { describe, expect, mock, test } from 'bun:test'
import type { ProjectLayout, Settings, Tab } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, renderHookWithProviders, waitFor } from '@shared/testing/render'

/**
 * The live half of `explorer.autoReveal`: `decideAutoReveal` already owns the rules (and has its own
 * tests), so what is left here is everything the hook alone can get wrong — reading the *main* pane
 * tree's active file, refusing a path outside the project root, claiming a path before the async
 * reveal so a re-render mid-flight cannot fire it twice, and never selecting when the reveal itself
 * failed.
 *
 * The hook reads layout and settings through `useQuery`, so both entity `.ipc` modules are stubbed
 * before it is pulled in through a *dynamic* `import()`; each fake covers its module's whole export
 * surface because `mock.module` is process-global and last-registration-wins
 * (`docs/memory/test-conventions.md` §3). `layout.ipc` additionally reaches monaco through
 * `layout.query.ts` → `tab-path-change.ts`, which is stubbed the same way.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

const rejectLikeUnavailableIpc = () => Promise.reject(new Error('ipc unavailable under bun:test'))

const layoutImpl = { current: null as ProjectLayout | null }
const settingsImpl = { current: {} as Settings }

mock.module('@entities/layout/layout.ipc', () => ({
    getLayout: () => Promise.resolve(layoutImpl.current),
    openTab: rejectLikeUnavailableIpc,
    closeTab: rejectLikeUnavailableIpc,
    activateTab: rejectLikeUnavailableIpc,
    moveTab: rejectLikeUnavailableIpc,
    splitPane: rejectLikeUnavailableIpc,
    openTabInSplit: rejectLikeUnavailableIpc,
    resizePane: rejectLikeUnavailableIpc,
    focusPane: rejectLikeUnavailableIpc,
    pinTab: rejectLikeUnavailableIpc,
    setTabPreview: rejectLikeUnavailableIpc,
    setTabDirty: rejectLikeUnavailableIpc,
    setTerminalSession: rejectLikeUnavailableIpc,
    reopenClosedTab: rejectLikeUnavailableIpc,
    openUntitledTab: rejectLikeUnavailableIpc,
    convertUntitledTab: rejectLikeUnavailableIpc,
    moveTabToWindow: rejectLikeUnavailableIpc,
    setShellView: rejectLikeUnavailableIpc,
    setTabViewState: rejectLikeUnavailableIpc,
    applyTabPathChange: rejectLikeUnavailableIpc,
}))

mock.module('@entities/settings/settings.ipc', () => ({
    emptySettingsPatch: () => ({}),
    getSettings: () => Promise.resolve(settingsImpl.current),
    updateSettings: rejectLikeUnavailableIpc,
    setThemeId: rejectLikeUnavailableIpc,
}))

const importUseExplorerAutoReveal = () => import('@widgets/explorer/use-explorer-auto-reveal')

const PROJECT_ID = 'project-1'
const PROJECT_ROOT = '/project'
const ACTIVE_PATH = '/project/src/app.tsx'

const buildFileTab = (path: string): Tab => ({ id: 'tab-1', kind: { kind: 'file', path }, title: path, dirty: false })

const buildLayout = (tabs: Tab[], shellView?: ProjectLayout['shellView']): ProjectLayout => ({
    version: 2,
    root: { node: 'leaf', id: 'leaf-1', tabs, active: tabs[0]?.id ?? null },
    focusedPane: 'leaf-1',
    revision: 1,
    shellView,
})

const buildRow = (path: string): FileTreeRow => ({
    id: path,
    path,
    name: path.slice(path.lastIndexOf('/') + 1),
    depth: 1,
    kind: 'file',
    expanded: false,
    gitStatus: null,
})

type AutoRevealOverrides = {
    layout?: ProjectLayout | null
    settings?: Partial<Settings>
    rows?: FileTreeRow[]
    explorerViewActive?: boolean
    projectRoot?: string | null
    revealFails?: boolean
}

const renderAutoReveal = async ({
    layout = buildLayout([buildFileTab(ACTIVE_PATH)]),
    settings = {},
    rows = [],
    explorerViewActive = true,
    projectRoot = PROJECT_ROOT,
    revealFails = false,
}: AutoRevealOverrides = {}) => {
    const { useExplorerAutoReveal } = await importUseExplorerAutoReveal()
    layoutImpl.current = layout
    settingsImpl.current = settings as Settings

    /**
     * Both queries are seeded *and* backed by the stubs above. The seed is what makes the test
     * independent of module load order: bun caches a module the first time it is imported, so a
     * `mock.module` registered here can be bypassed entirely when an earlier test file already
     * pulled `settings.query`/`layout.query` in against the real `.ipc` (the hazard
     * `docs/memory/test-conventions.md` §3 describes). With the cache seeded, that only means the
     * mount refetch rejects instead of resolving — and React Query keeps the cached data on a
     * failed refetch, so the hook still sees exactly these inputs either way.
     */
    const queryClient = createTestQueryClient()
    queryClient.setQueryData(QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID), layout)
    queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settings)

    const revealCalls: { projectId: string; path: string }[] = []
    const selectCalls: string[] = []
    const revealTreeNode = (input: { projectId: string; path: string }) => {
        revealCalls.push(input)
        if (revealFails) return Promise.reject(new Error('reveal failed'))
        return Promise.resolve({ rows: [], total: 0 })
    }

    const rendered = renderHookWithProviders(
        (props: { rows: FileTreeRow[] }) =>
            useExplorerAutoReveal({
                projectId: PROJECT_ID,
                projectRoot,
                rows: props.rows,
                explorerViewActive,
                setSelectPathRequest: (path: string) => selectCalls.push(path),
                revealTreeNode: revealTreeNode as Parameters<typeof useExplorerAutoReveal>[0]['revealTreeNode'],
            }),
        { queryClient, initialProps: { rows } },
    )

    return { ...rendered, revealCalls, selectCalls }
}

/**
 * Lets the layout/settings queries settle and the resulting effect run before asserting a
 * *negative*. Wrapped in `act` because the query resolutions it waits for are what re-render the
 * hook — outside `act`, React reports every one of those updates as an unwrapped state change.
 */
const settle = () =>
    act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0))
        await new Promise((resolve) => setTimeout(resolve, 0))
    })

describe('useExplorerAutoReveal', () => {
    test('보이지 않는 활성 파일은 tree_reveal 로 펼친 뒤 선택을 요청한다', async () => {
        const { revealCalls, selectCalls } = await renderAutoReveal()

        await waitFor(() => expect(selectCalls).toEqual([ACTIVE_PATH]))

        expect(revealCalls).toEqual([{ projectId: PROJECT_ID, path: ACTIVE_PATH }])
    })

    test('이미 화면에 있는 행이면 IPC 없이 선택만 요청한다', async () => {
        const { revealCalls, selectCalls } = await renderAutoReveal({ rows: [buildRow(ACTIVE_PATH)] })

        await waitFor(() => expect(selectCalls).toEqual([ACTIVE_PATH]))

        expect(revealCalls).toEqual([])
    })

    test('설정이 꺼져 있으면 아무 것도 하지 않는다', async () => {
        const { revealCalls, selectCalls } = await renderAutoReveal({ settings: { explorerAutoReveal: false } })

        await settle()

        expect(revealCalls).toEqual([])
        expect(selectCalls).toEqual([])
    })

    test('설정 값이 아직 없으면 기본 동작(켜짐)으로 본다', async () => {
        const { selectCalls } = await renderAutoReveal({ settings: {} })

        await waitFor(() => expect(selectCalls).toEqual([ACTIVE_PATH]))
    })

    test('사이드바가 접혀 있거나 Zen 이면 트리를 건드리지 않는다', async () => {
        const collapsed = await renderAutoReveal({ layout: buildLayout([buildFileTab(ACTIVE_PATH)], { sidebarCollapsed: true }) })
        const zen = await renderAutoReveal({ layout: buildLayout([buildFileTab(ACTIVE_PATH)], { zen: true }) })

        await settle()

        expect(collapsed.revealCalls).toEqual([])
        expect(collapsed.selectCalls).toEqual([])
        expect(zen.revealCalls).toEqual([])
        expect(zen.selectCalls).toEqual([])
    })

    test('사이드바가 검색·git 등 다른 뷰를 보여주는 중이면 아무 것도 하지 않는다', async () => {
        const { revealCalls, selectCalls } = await renderAutoReveal({ explorerViewActive: false })

        await settle()

        expect(revealCalls).toEqual([])
        expect(selectCalls).toEqual([])
    })

    test('프로젝트 루트 밖의 파일은 보낼 곳이 없으므로 요청하지 않는다', async () => {
        const { revealCalls, selectCalls } = await renderAutoReveal({ layout: buildLayout([buildFileTab('/elsewhere/other.ts')]) })

        await settle()

        expect(revealCalls).toEqual([])
        expect(selectCalls).toEqual([])
    })

    test('활성 탭이 파일이 아니면 아무 것도 하지 않는다', async () => {
        const terminalTab: Tab = { id: 'tab-1', kind: { kind: 'terminal', sessionId: '', cwd: null }, title: 'terminal', dirty: false }
        const { revealCalls, selectCalls } = await renderAutoReveal({ layout: buildLayout([terminalTab]) })

        await settle()

        expect(revealCalls).toEqual([])
        expect(selectCalls).toEqual([])
    })

    test('같은 파일이 계속 활성인 채 rows 만 갱신돼도 한 번만 드러낸다 (reveal 이 행 페이지를 갈아끼우는 되먹임 차단)', async () => {
        const { rerender, revealCalls, selectCalls } = await renderAutoReveal()

        await waitFor(() => expect(selectCalls.length).toBe(1))
        rerender({ rows: [buildRow(ACTIVE_PATH)] })
        rerender({ rows: [buildRow(ACTIVE_PATH), buildRow('/project/src/other.ts')] })
        await settle()

        expect(revealCalls.length).toBe(1)
        expect(selectCalls.length).toBe(1)
    })

    test('reveal 이 실패하면 선택 요청도 보내지 않는다 (사용자가 요청한 동작이 아니므로 조용히 멈춘다)', async () => {
        const { revealCalls, selectCalls } = await renderAutoReveal({ revealFails: true })

        await waitFor(() => expect(revealCalls.length).toBe(1))
        await settle()

        expect(selectCalls).toEqual([])
    })
})
