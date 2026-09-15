import { describe, expect, mock, test } from 'bun:test'
import type { Project } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, renderWithProviders, screen } from '@shared/testing/render'

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
