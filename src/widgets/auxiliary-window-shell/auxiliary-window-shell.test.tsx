import { afterEach, beforeEach, describe, expect, mock, test } from 'bun:test'
import type { Project } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { TooltipProvider } from '@shared/ui/tooltip'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * An auxiliary window (`editor-<n>`) used to be editor-only: no explorer, no search, no SCM, and a
 * `⌘P` that answered with "main window only". d-62 §1.D closed that gap — the shell now mounts
 * `ExplorerContainer`, whose own view switcher brings `SearchPanelContainer` and `GitPanelContainer`
 * with it, all three pinned to the window's fixed `projectId`. These cases lock that the three views
 * are reachable (by click and by shortcut) and that the panel reads *this* window's project.
 *
 * Two module fakes, registered before the shell is pulled in through a *dynamic* `import()`
 * (`mock.module` is process-global and last-registration-wins — `docs/memory/test-conventions.md`
 * §3): `EditorArea` is stubbed because the editor body has nothing to do with the sidebar, and
 * `@shared/lib/monaco/setup` follows the existing precedent — `entities/layout/layout.query` reaches
 * monaco at import time.
 *
 * `@tauri-apps/api/window` is deliberately *not* mocked. The shell closes its own window once the
 * tree empties (and `layout_get` fails here, since there is no IPC), which needs `getCurrentWindow()`
 * to resolve — so this file seeds the same `window.__TAURI_INTERNALS__` shape
 * (`docs/memory/test-conventions.md` §4) and removes it afterwards, instead of replacing the module
 * process-wide with a fake window that has no `label` and would make every later file's
 * `isRemoteMirrorRuntime` read the wrong runtime.
 *
 * Every project-scoped query below this shell fails (no IPC), which is why the SCM view is asserted
 * through its "not a repository" branch rather than through a commit list — that branch is what
 * `GitPanelContainer` renders when `git_status` errors, so it is the mount itself that is locked
 * here, not the panel's populated state.
 */
const MONACO_SYMBOL_KIND_NAMES = [
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
]

/** The outline row builds its icon table from `monaco.languages.SymbolKind` at import time, so the stub has to carry distinct values for every kind that table names. */
const symbolKindStub = Object.fromEntries(MONACO_SYMBOL_KIND_NAMES.map((name, index) => [name, index]))

mock.module('@shared/lib/monaco/setup', () => ({
    monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {}, languages: { SymbolKind: symbolKindStub } },
}))
mock.module('@widgets/editor-area/editor-area', () => ({ EditorArea: () => null }))

const importShell = () => import('@widgets/auxiliary-window-shell/auxiliary-window-shell')

const MAIN_WINDOW_LABEL = 'main'
const PROJECT_ID = 'project-1'
const WINDOW_SLOT = 1
const PROJECT: Project = { id: PROJECT_ID, root: '/tmp/aux-project', name: 'aux-project' }

/**
 * The project detail is seeded with `gcTime: Infinity` because the test client collects
 * observer-less queries immediately (`docs/memory/test-conventions.md` §3), and `TooltipProvider` is
 * composed in here because `renderWithProviders` deliberately carries only Query + i18n — the view
 * switcher's `IconButton`s are radix tooltips.
 */
const renderShell = async () => {
    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.PROJECT.DETAIL(PROJECT_ID), queryFn: () => PROJECT, gcTime: Infinity })
    const { AuxiliaryWindowShell } = await importShell()
    return renderWithProviders(
        <TooltipProvider>
            <AuxiliaryWindowShell projectId={PROJECT_ID} windowSlot={WINDOW_SLOT} />
        </TooltipProvider>,
        { queryClient },
    )
}

const pressKey = (init: KeyboardEventInit) => window.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }))

const viewTab = (name: string) => screen.getByRole('tab', { name })

describe('AuxiliaryWindowShell 사이드바', () => {
    beforeEach(() => {
        window.__TAURI_INTERNALS__ = { metadata: { currentWindow: { label: MAIN_WINDOW_LABEL } } }
    })

    afterEach(() => {
        window.__TAURI_INTERNALS__ = undefined
    })

    test('창의 프로젝트로 탐색기 패널을 마운트한다', async () => {
        await renderShell()

        expect(screen.getByRole('tablist', { name: 'explorer.sidebarSwitchLabel' })).toBeTruthy()
        expect(screen.getByRole('heading', { name: PROJECT.name })).toBeTruthy()
    })

    test('검색 뷰로 전환하면 검색 패널이 마운트된다', async () => {
        await renderShell()

        act(() => fireEvent.click(viewTab('search.title')))

        expect(screen.getByPlaceholderText('search.placeholder')).toBeTruthy()
    })

    test('SCM 뷰로 전환하면 git 패널이 마운트된다', async () => {
        await renderShell()

        act(() => fireEvent.click(viewTab('git.title')))

        expect(await screen.findByText('git.notARepository')).toBeTruthy()
    })

    test('⌃⇧G 는 창 안의 탐색기 브리지를 통해 SCM 뷰를 연다', async () => {
        await renderShell()

        act(() => {
            pressKey({ key: 'g', code: 'KeyG', ctrlKey: true, shiftKey: true })
        })

        expect(viewTab('git.title').getAttribute('aria-selected')).toBe('true')
    })

    test('⌘⇧E 는 다시 파일 뷰로 돌아온다', async () => {
        await renderShell()

        act(() => {
            pressKey({ key: 'g', code: 'KeyG', ctrlKey: true, shiftKey: true })
            pressKey({ key: 'e', code: 'KeyE', metaKey: true, shiftKey: true })
        })

        expect(viewTab('explorer.title').getAttribute('aria-selected')).toBe('true')
    })
})
