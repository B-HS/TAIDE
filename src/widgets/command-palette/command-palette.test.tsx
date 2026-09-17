import { afterEach, describe, expect, mock, test } from 'bun:test'
import type { PaneNode, Project, ProjectLayout } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The palette used to read the *global* active-project session for its file index, symbols, tab
 * opens and terminal, which is the single reason it could only ever be mounted in the main window
 * (Wave I contract §3.1). d-62 §1.D replaced that read with a `projectId` prop so an auxiliary
 * window can mount its own copy pinned to its own project — these cases lock that the quick-open
 * index follows the prop and nothing else.
 *
 * monaco is stubbed before the palette is pulled in through a *dynamic* `import()` (the widget
 * imports `@shared/lib/monaco/setup` at module scope, and `entities/layout/layout.query` reaches it
 * too); `mock.module` is process-global and last-registration-wins
 * (`docs/memory/test-conventions.md` §3).
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

const importPalette = () => import('@widgets/command-palette/command-palette')

const PROJECT_ID = 'project-1'
const OTHER_PROJECT_ID = 'project-2'
const PROJECT: Project = { id: PROJECT_ID, root: '/tmp/aux-project', name: 'aux-project' }
const PROJECT_FILES = ['/tmp/aux-project/src/main.ts']
const FILE_NAME = 'main.ts'

const pressKey = (init: KeyboardEventInit) => window.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }))

/** Both seeds use `gcTime: Infinity` because the test client collects observer-less queries immediately (`docs/memory/test-conventions.md` §3). */
const renderPalette = async (projectId: string) => {
    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.PROJECT.DETAIL(PROJECT_ID), queryFn: () => PROJECT, gcTime: Infinity })
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(PROJECT_ID), queryFn: () => PROJECT_FILES, gcTime: Infinity })
    const { CommandPalette } = await importPalette()
    const rendered = renderWithProviders(<CommandPalette projectId={projectId} />, { queryClient })
    act(() => {
        pressKey({ key: 'p', code: 'KeyP', metaKey: true })
    })
    return rendered
}

describe('CommandPalette 프로젝트 스코프', () => {
    test('prop 으로 받은 프로젝트의 파일을 quick open 에 올린다', async () => {
        await renderPalette(PROJECT_ID)

        expect(screen.getByPlaceholderText('palette.filePlaceholder')).toBeTruthy()
        expect(await screen.findByText(FILE_NAME)).toBeTruthy()
    })

    test('다른 프로젝트를 받으면 그 프로젝트의 파일 목록만 조회한다', async () => {
        const { queryClient } = await renderPalette(OTHER_PROJECT_ID)

        expect(screen.queryByText(FILE_NAME)).toBeNull()
        expect(queryClient.getQueryState(QUERY_KEY.SEARCH.PROJECT_FILES(OTHER_PROJECT_ID))).toBeTruthy()
    })
})

const MAIN_PATH = '/tmp/aux-project/src/main.ts'
const AUXILIARY_PATH = '/tmp/aux-project/src/aux.ts'
const SYMBOL_MODE_QUERY = '@'

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

const LAYOUT: ProjectLayout = {
    version: 2,
    root: buildFileLeaf('main-leaf', MAIN_PATH),
    focusedPane: 'main-leaf',
    auxiliaryWindows: [{ slot: 1, root: buildFileLeaf('aux-leaf', AUXILIARY_PATH), focusedPane: 'aux-leaf' }],
}

const openSymbolMode = async () => {
    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID), queryFn: () => LAYOUT, gcTime: Infinity })
    const { CommandPalette } = await importPalette()
    const rendered = renderWithProviders(<CommandPalette projectId={PROJECT_ID} />, { queryClient })
    act(() => {
        pressKey({ key: 'p', code: 'KeyP', metaKey: true })
    })
    act(() => {
        fireEvent.change(screen.getByPlaceholderText('palette.filePlaceholder'), { target: { value: SYMBOL_MODE_QUERY } })
    })
    return rendered
}

/**
 * `@`(Go to Symbol) and `:`(line) act on "the active file", which an auxiliary window's palette read
 * off the main tree — it listed another window's symbols and revealed into another window (audit #5).
 * The observable is which file the palette asks for: `fileQueryOptions(activePath)` is the only query
 * it starts in symbol mode.
 */
describe('CommandPalette 창별 활성 파일', () => {
    test('보조 창의 심볼 모드는 그 창의 활성 파일을 대상으로 한다', async () => {
        enterWindowUrl(AUXILIARY_WINDOW_URL)

        const { queryClient } = await openSymbolMode()

        expect(queryClient.getQueryState(QUERY_KEY.FILE.CONTENT(AUXILIARY_PATH))).toBeTruthy()
        expect(queryClient.getQueryState(QUERY_KEY.FILE.CONTENT(MAIN_PATH))).toBeUndefined()
    })

    test('main 창의 심볼 모드는 main 트리의 활성 파일을 대상으로 한다', async () => {
        const { queryClient } = await openSymbolMode()

        expect(queryClient.getQueryState(QUERY_KEY.FILE.CONTENT(MAIN_PATH))).toBeTruthy()
        expect(queryClient.getQueryState(QUERY_KEY.FILE.CONTENT(AUXILIARY_PATH))).toBeUndefined()
    })
})
