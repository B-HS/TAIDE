import { afterEach, describe, expect, mock, test } from 'bun:test'
import type { PaneNode, ProjectLayout } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { createTestQueryClient, renderWithProviders, screen } from '@shared/testing/render'

const SYMBOL_KIND_NAMES = [
    'File',
    'Module',
    'Namespace',
    'Package',
    'Class',
    'Struct',
    'Interface',
    'Enum',
    'EnumMember',
    'Constant',
    'Constructor',
    'Method',
    'Function',
    'Property',
    'Field',
    'Variable',
] as const

/** `outline-symbol-row.tsx` builds its icon map from `SymbolKind` at module scope, so the stub has to carry every member that map indexes. */
const FAKE_SYMBOL_KIND = Object.fromEntries(SYMBOL_KIND_NAMES.map((name, index) => [name, index]))

/**
 * The outline follows *this* window's active file. It used to call `activeFilePathOf(layout)` on the
 * raw `ProjectLayout`, which is structurally a `WindowPaneTree` and therefore type-checks while
 * always answering with the main tree — so an auxiliary window listed the symbols of a file it was
 * not showing, revealed into a window the user was not looking at, and asked the LSP for document
 * symbols twice (audit #6). `ExplorerContainer`, and with it this panel, has been mounted in
 * auxiliary windows since d-62 §1.D.
 *
 * The container reaches `@shared/lib/monaco/setup` (real monaco worker bundles `bun test` cannot
 * load), so it is stubbed before the module is pulled in through a *dynamic* `import()` — the same
 * workaround `command-palette.test.tsx` and `ide-sync-provider.test.ts` document.
 */
mock.module('@shared/lib/monaco/setup', () => ({
    monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {}, languages: { SymbolKind: FAKE_SYMBOL_KIND } },
}))

const importContainer = () => import('@widgets/outline-panel/outline-panel-container')

const PROJECT_ID = 'project-1'
const MAIN_PATH = '/repo/main.ts'
const AUXILIARY_PATH = '/repo/aux.ts'

const MAIN_WINDOW_URL = '/'
const AUXILIARY_WINDOW_URL = `/?projectId=${PROJECT_ID}&windowSlot=1`

/** See `pane-tree.test.ts` — the harness pins `location.search` to empty, and this is the one same-document way to move it. */
const enterWindowUrl = (url: string) => window.history.replaceState({}, '', url)

afterEach(() => enterWindowUrl(MAIN_WINDOW_URL))

const buildFileLeaf = (id: string, path: string): PaneNode => ({
    node: 'leaf',
    id,
    tabs: [{ id: `${id}-tab`, kind: { kind: 'file', path }, title: path }],
    active: `${id}-tab`,
})

const buildLayout = (auxiliaryWindows: ProjectLayout['auxiliaryWindows']): ProjectLayout => ({
    version: 2,
    root: buildFileLeaf('main-leaf', MAIN_PATH),
    focusedPane: 'main-leaf',
    auxiliaryWindows,
})

/** `gcTime: Infinity` on the seed because the test client collects observer-less queries immediately (`docs/memory/test-conventions.md` §3). */
const renderOutline = async (layout: ProjectLayout) => {
    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID), queryFn: () => layout, gcTime: Infinity })
    const { OutlinePanelContainer } = await importContainer()
    return renderWithProviders(<OutlinePanelContainer projectId={PROJECT_ID} />, { queryClient })
}

const LAYOUT_WITH_AUXILIARY_WINDOW = () => buildLayout([{ slot: 1, root: buildFileLeaf('aux-leaf', AUXILIARY_PATH), focusedPane: 'aux-leaf' }])

describe('OutlinePanelContainer 창별 활성 파일', () => {
    test('보조 창에서는 그 창의 활성 파일만 조회한다 (main 창 파일은 건드리지 않는다)', async () => {
        enterWindowUrl(AUXILIARY_WINDOW_URL)

        const { queryClient } = await renderOutline(LAYOUT_WITH_AUXILIARY_WINDOW())

        expect(queryClient.getQueryState(QUERY_KEY.FILE.CONTENT(AUXILIARY_PATH))).toBeTruthy()
        expect(queryClient.getQueryState(QUERY_KEY.FILE.CONTENT(MAIN_PATH))).toBeUndefined()
    })

    test('main 창에서는 main 트리의 활성 파일을 조회한다', async () => {
        const { queryClient } = await renderOutline(LAYOUT_WITH_AUXILIARY_WINDOW())

        expect(queryClient.getQueryState(QUERY_KEY.FILE.CONTENT(MAIN_PATH))).toBeTruthy()
        expect(queryClient.getQueryState(QUERY_KEY.FILE.CONTENT(AUXILIARY_PATH))).toBeUndefined()
    })

    test('slot 이 사라진 보조 창은 main 창 파일로 폴백하지 않고 활성 파일 없음을 그린다', async () => {
        enterWindowUrl(AUXILIARY_WINDOW_URL)

        const { queryClient } = await renderOutline(buildLayout([]))

        expect(screen.getByText('outline.noActiveFile')).toBeDefined()
        expect(queryClient.getQueryState(QUERY_KEY.FILE.CONTENT(MAIN_PATH))).toBeUndefined()
    })
})
