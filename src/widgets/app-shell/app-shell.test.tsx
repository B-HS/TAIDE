import { describe, expect, mock, spyOn, test } from 'bun:test'
import type { ProjectRef, SessionShellState } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { ShellSlotFocusProvider } from '@shared/lib/shell-slot-context'
import { TooltipProvider } from '@shared/ui/tooltip'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The window-level state `AppShell` owns on behalf of every slot below it. Only the Problems pair is
 * exercised here: it is the one flag that lives *above* the slots (the status bar's toggle has to
 * reach the focused slot), so it is also the one that can outlive the slot it belongs to.
 *
 * Five module fakes, all registered before the shell is pulled in through a dynamic `import()`
 * (`mock.module` is process-global and last-registration-wins — `docs/memory/test-conventions.md`
 * §3). `ProjectShell` carries the same stub shape `shell-slot-tree-view.test.tsx` registers, so
 * whichever of the two files runs last the other still reads the flag off the rendered text. The
 * status bar is reduced to its Problems pair, the title bar and sidebar to nothing — none of them
 * has anything to do with this file, and all three open project-scoped queries that cannot resolve
 * without IPC. `@tauri-apps/api/webview` is faked because the drag-and-drop subscription calls
 * `getCurrentWebview()` synchronously inside an effect, which throws with no `__TAURI_INTERNALS__`;
 * this is the only module in `src` that imports it.
 *
 * The real `ShellSlotTreeView` is kept — the slot header's ✕ is the only user-facing way to reach
 * `onCloseSlot`, and going through it is what makes this a wiring test rather than a restatement of
 * the reducer.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))
mock.module('@widgets/app-shell/project-shell', () => ({
    ProjectShell: ({ projectId, isProblemsOpen }: { projectId: string; isProblemsOpen: boolean }) => (
        <div>{`shell:${projectId}${isProblemsOpen ? ' problems' : ''}`}</div>
    ),
}))
mock.module('@widgets/window-chrome/title-bar-content', () => ({ TitleBarContent: () => null }))
mock.module('@widgets/app-sidebar/app-sidebar', () => ({ AppSidebar: () => null }))
mock.module('@widgets/window-chrome/status-bar-content', () => ({
    StatusBarContent: ({ onToggleProblems }: { onToggleProblems: () => void }) => (
        <button type='button' onClick={onToggleProblems}>
            toggle problems
        </button>
    ),
}))
mock.module('@tauri-apps/api/webview', () => ({ getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) }))

const importShell = () => import('@widgets/app-shell/app-shell')

const LEFT_SLOT_ID = 'shellslot-left'
const RIGHT_SLOT_ID = 'shellslot-right'
const LEFT_PROJECT_ID = 'project-left'
const RIGHT_PROJECT_ID = 'project-right'

const PROJECTS: ProjectRef[] = [
    { id: LEFT_PROJECT_ID, root: '/tmp/left', name: 'left' },
    { id: RIGHT_PROJECT_ID, root: '/tmp/right', name: 'right' },
]

const SHELL_STATE: SessionShellState = {
    tree: {
        node: 'split',
        dir: 'horizontal',
        sizes: [50, 50],
        children: [
            { node: 'leaf', slotId: LEFT_SLOT_ID, projectId: LEFT_PROJECT_ID },
            { node: 'leaf', slotId: RIGHT_SLOT_ID, projectId: RIGHT_PROJECT_ID },
        ],
    },
    focused: LEFT_SLOT_ID,
    windowChrome: { zen: false, sidebarRailCollapsed: false },
}

/** Drains the mutation's own promise chain so its React Query state lands *inside* the `act` scope the click was fired in. */
const settleMutation = () => new Promise((resolve) => setTimeout(resolve, 0))

/**
 * Both reads are seeded rather than mocked, with per-key `staleTime`/`gcTime` defaults so the shell
 * never refetches them: the test client collects observer-less queries immediately, and another
 * file's process-global `@entities/project/project.ipc` fake (`listProjects: () => []`) would
 * otherwise win the mount refetch and drop the window to the Welcome screen
 * (`docs/memory/test-conventions.md` §3).
 *
 * Focus is supplied directly instead of mounting `ShellSlotProvider`, which owns a DOM concern this
 * file does not test.
 */
const renderShell = async () => {
    const queryClient = createTestQueryClient()
    queryClient.setQueryDefaults(QUERY_KEY.PROJECT.LIST, { staleTime: Infinity, gcTime: Infinity })
    queryClient.setQueryDefaults(QUERY_KEY.SESSION.SHELL_STATE, { staleTime: Infinity, gcTime: Infinity })
    queryClient.setQueryData(QUERY_KEY.PROJECT.LIST, PROJECTS)
    queryClient.setQueryData(QUERY_KEY.SESSION.SHELL_STATE, SHELL_STATE)
    const { AppShell } = await importShell()

    return renderWithProviders(
        <TooltipProvider>
            <ShellSlotFocusProvider focusedShellSlotId={LEFT_SLOT_ID} focusedProjectId={LEFT_PROJECT_ID}>
                <AppShell />
            </ShellSlotFocusProvider>
        </TooltipProvider>,
        { queryClient },
    )
}

const shellTexts = () => screen.getAllByText(/^shell:/).map((element) => element.textContent)

const closeSlot = async (index: number) => {
    const closeButton = screen.getAllByRole('button', { name: 'shellSlot.close' })[index]
    await act(async () => {
        fireEvent.click(closeButton)
        await settleMutation()
    })
}

describe('AppShell 슬롯별 Problems 상태', () => {
    test('상태바 토글은 포커스 슬롯의 Problems 만 연다', async () => {
        await renderShell()

        fireEvent.click(screen.getByRole('button', { name: 'toggle problems' }))

        expect(shellTexts()).toEqual([`shell:${LEFT_PROJECT_ID} problems`, `shell:${RIGHT_PROJECT_ID}`])
    })

    test('슬롯을 닫으면 그 슬롯의 Problems 표시도 함께 잊는다', async () => {
        await renderShell()
        const closeShellSlot = spyOn(commands, 'shellSlotClose').mockResolvedValue({ status: 'ok', data: null })

        fireEvent.click(screen.getByRole('button', { name: 'toggle problems' }))
        await closeSlot(0)

        expect(closeShellSlot).toHaveBeenCalledWith(LEFT_SLOT_ID)
        expect(shellTexts()).toEqual([`shell:${LEFT_PROJECT_ID}`, `shell:${RIGHT_PROJECT_ID}`])
    })

    test('닫기가 거부되면 Problems 표시는 그대로 남는다 — 마지막 슬롯은 서버가 거절한다', async () => {
        await renderShell()
        spyOn(commands, 'shellSlotClose').mockResolvedValue({ status: 'error', error: { code: 'InvalidArgument', message: 'last slot' } })

        fireEvent.click(screen.getByRole('button', { name: 'toggle problems' }))
        await closeSlot(0)

        expect(shellTexts()).toEqual([`shell:${LEFT_PROJECT_ID} problems`, `shell:${RIGHT_PROJECT_ID}`])
    })
})
