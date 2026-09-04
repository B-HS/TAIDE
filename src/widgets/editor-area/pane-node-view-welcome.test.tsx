import { describe, expect, mock, test } from 'bun:test'
import type { PaneNode, Settings, Tab } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { createTestQueryClient, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The "tab 0 shows Welcome" branch (batch 4 contract §B.2-2). `!activeTab` is exactly "this
 * window's pane tree holds no tabs at all" — `normalize_owned` collapses empty leaves out of a
 * split — so this is the whole-window empty state, and the setting decides between Welcome and the
 * old `editor.noFileOpen` line.
 *
 * `PaneNodeView` statically imports every pane body, including the monaco editor, so
 * `@shared/lib/monaco/setup` is stubbed before it is pulled in through a *dynamic* `import()`; the
 * branches under test render neither monaco nor xterm. `zen` is passed so the tab bar (dnd-kit,
 * unrelated to this branch) stays out of the tree.
 *
 * The auxiliary-window half of the condition is not reachable here: the harness pins
 * `location.search` to empty, so `getWindowContext()` always resolves to the main window
 * (`docs/memory/test-conventions.md` §4) — an auxiliary window closes itself when emptied anyway,
 * and that is an e2e concern.
 */
mock.module('@shared/lib/monaco/setup', () => ({
    monaco: {
        Uri: { file: () => ({ toString: () => '' }) },
        editor: {},
        languages: { InlineCompletionTriggerKind: { Automatic: 0, Explicit: 1 } },
    },
}))

const importPaneNodeView = () => import('@widgets/editor-area/pane-node-view')

const PROJECT_ID = 'project-1'
const PANE_ID = 'leaf-1'

const buildLeaf = (tabs: Tab[]): PaneNode => ({ node: 'leaf', id: PANE_ID, tabs, active: tabs[0]?.id ?? null })

const renderPane = async (settings: Partial<Settings> | undefined, tabs: Tab[] = []) => {
    const { PaneNodeView } = await importPaneNodeView()
    const queryClient = createTestQueryClient()
    if (settings) queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settings)

    return renderWithProviders(
        <PaneNodeView node={buildLeaf(tabs)} projectId={PROJECT_ID} focusedPaneId={PANE_ID} isDragging={false} overTarget={null} zen={true} />,
        { queryClient },
    )
}

describe('PaneNodeView 빈 편집 영역', () => {
    test('설정이 켜져 있으면 탭 0 인 pane 에 Welcome 을 그린다', async () => {
        await renderPane({ welcomeOnEmptyEditor: true })

        expect(await screen.findByRole('button', { name: 'app.openFile' })).toBeDefined()
        expect(screen.queryByText('editor.noFileOpen')).toBeNull()
    })

    test('설정이 꺼져 있으면 기존 안내 문구를 유지한다', async () => {
        await renderPane({ welcomeOnEmptyEditor: false })

        expect(screen.getByText('editor.noFileOpen')).toBeDefined()
        expect(screen.queryByRole('button', { name: 'app.openFile' })).toBeNull()
    })

    test('설정을 아직 읽지 못했으면 켜진 것으로 본다 (부팅 직후 빈 창이 문구로 깜빡이지 않게)', async () => {
        await renderPane(undefined)

        expect(screen.queryByText('editor.noFileOpen')).toBeNull()
        expect(await screen.findByRole('button', { name: 'app.openFile' })).toBeDefined()
    })

    test('lazy 청크가 오는 동안에도 안내 문구를 먼저 보여주지 않는다 (Suspense 폴백은 빈 배경)', async () => {
        await renderPane({ welcomeOnEmptyEditor: true })

        expect(screen.queryByText('editor.noFileOpen')).toBeNull()
    })

    test('탭이 하나라도 있으면 설정이 꺼져 있어도 빈 상태 문구를 그리지 않는다', async () => {
        const welcomeTab: Tab = { id: 'tab-1', kind: { kind: 'welcome' }, title: 'Welcome', dirty: false }

        await renderPane({ welcomeOnEmptyEditor: false }, [welcomeTab])

        expect(screen.queryByText('editor.noFileOpen')).toBeNull()
        expect(await screen.findByRole('button', { name: 'app.openFile' })).toBeDefined()
    })
})
