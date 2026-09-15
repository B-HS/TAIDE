import { describe, expect, mock, test } from 'bun:test'
import type { PaneNode, ProjectLayout } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { createTestQueryClient, renderHookWithProviders, waitFor } from '@shared/testing/render'

const PROJECT_ID = 'project-1'
const FOCUSED_PANE_ID = 'leaf-7'

const buildLayout = (focusedPane: string): ProjectLayout => {
    const leaf: PaneNode = { node: 'leaf', id: focusedPane, tabs: [], active: null }
    return { version: 2, root: leaf, focusedPane, revision: 1 }
}

/**
 * `useOpenTerminalTab` is the single entry point the command palette's `new-terminal` command and
 * the Welcome screen's terminal button share (d-58 contract §1.C). The branch worth locking is
 * `target`: it must resolve to the calling window's focused pane out of the layout cache rather
 * than being left `null`, because a `null` target makes Rust fall back to the *main* tree's
 * `focused_pane` and a Welcome tab living in an auxiliary window would then open its terminal in
 * the wrong window. With no layout cached yet it degrades to `null`, which is exactly that Rust
 * fallback.
 *
 * The stubbing follows `use-open-file-tab.test.tsx`: `layout.query.ts` reaches monaco (through
 * `tab-path-change.ts`) and its own `.ipc` module at import time, so both are replaced before it is
 * pulled in through a dynamic `import()`, and the `layout.ipc` fake covers that module's *entire*
 * export surface because `mock.module` is process-global and last-registration-wins
 * (`docs/memory/test-conventions.md` §3).
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

const rejectLikeUnavailableIpc = () => Promise.reject(new Error('ipc unavailable under bun:test'))

type OpenTabInput = { projectId: string; kind: { kind: string; sessionId?: string }; title: string; target: string | null; preview: boolean }

const capturedOpenTabCalls: OpenTabInput[] = []

mock.module('@entities/layout/layout.ipc', () => ({
    getLayout: rejectLikeUnavailableIpc,
    openTab: (input: OpenTabInput) => {
        capturedOpenTabCalls.push(input)
        return Promise.resolve(buildLayout(FOCUSED_PANE_ID))
    },
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

const importLayoutQuery = () => import('@entities/layout/layout.query')

const seedLayout = async (queryClient: ReturnType<typeof createTestQueryClient>) => {
    await queryClient.fetchQuery({
        queryKey: QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID),
        queryFn: () => Promise.resolve(buildLayout(FOCUSED_PANE_ID)),
        gcTime: Infinity,
    })
}

describe('useOpenTerminalTab', () => {
    test('빈 sessionId 의 터미널 탭을 이 창의 포커스 pane 에 연다', async () => {
        const { useOpenTerminalTab } = await importLayoutQuery()
        const queryClient = createTestQueryClient()
        await seedLayout(queryClient)

        const { result } = renderHookWithProviders(() => useOpenTerminalTab(PROJECT_ID), { queryClient })
        result.current()
        await waitFor(() => expect(capturedOpenTabCalls.length).toBeGreaterThan(0))

        expect(capturedOpenTabCalls.at(-1)).toEqual({
            projectId: PROJECT_ID,
            kind: { kind: 'terminal', sessionId: '' },
            title: 'terminal.title',
            target: FOCUSED_PANE_ID,
            preview: false,
        })
    })

    test('레이아웃 캐시가 아직 없으면 target 을 null 로 넘겨 Rust 폴백에 맡긴다', async () => {
        const { useOpenTerminalTab } = await importLayoutQuery()
        const queryClient = createTestQueryClient()
        const before = capturedOpenTabCalls.length

        const { result } = renderHookWithProviders(() => useOpenTerminalTab(PROJECT_ID), { queryClient })
        result.current()
        await waitFor(() => expect(capturedOpenTabCalls.length).toBeGreaterThan(before))

        expect(capturedOpenTabCalls.at(-1)?.target).toBeNull()
    })
})
