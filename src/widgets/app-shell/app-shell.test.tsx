import type { FC } from 'react'
import { describe, expect, mock, spyOn, test } from 'bun:test'
import { DndContext, useDndContext, useDraggable } from '@dnd-kit/core'
import type { ProjectRef, SessionShellState } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { shellSlotDropDataOf } from '@shared/lib/project-drag'
import { ShellSlotFocusProvider } from '@shared/lib/shell-slot-context'
import { TooltipProvider } from '@shared/ui/tooltip'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * Reads whichever `DndContext` sits above its own position: the slot drop zones that context has
 * registered, and what it currently holds as the active drag. One on each side of the stub's own
 * provider is what turns nesting and isolation into something a test can read.
 */
const SlotDndProbe: FC<{ scope: string }> = ({ scope }) => {
    const { active, droppableContainers } = useDndContext()
    const slotIds = [...droppableContainers].flatMap(([, container]) => {
        const data = shellSlotDropDataOf(container)
        return data ? [data.slotId] : []
    })
    return <div>{`${scope} slots=${[...new Set(slotIds)].sort().join(',') || 'none'} active=${active?.id ?? 'none'}`}</div>
}

/** Stands in for a tab: something the stub's own `DndContext` can pick up, named so a test can grab it by role. */
const SlotTabDragStub: FC<{ id: string }> = ({ id }) => {
    const { setNodeRef, listeners, attributes } = useDraggable({ id })
    return (
        <button type='button' ref={setNodeRef} {...listeners} {...attributes}>
            {id}
        </button>
    )
}

/**
 * The window-level state `AppShell` owns on behalf of every slot below it. Only the Problems pair is
 * exercised here: it is the one flag that lives *above* the slots (the status bar's toggle has to
 * reach the focused slot), so it is also the one that can outlive the slot it belongs to.
 *
 * The second subject is the nesting `EditorArea` relies on, pinned here in the real shell rather
 * than in the synthetic tree of `project-drag-nesting.test.tsx`: the `ProjectShell` stub carries a
 * `DndContext` of its own where the editor's tab context would be, with a probe on each side of it.
 *
 * Five module fakes, all registered before the shell is pulled in through a dynamic `import()`
 * (`mock.module` is process-global and last-registration-wins — `docs/memory/test-conventions.md`
 * §3). `ProjectShell` still renders the same `shell:<projectId>` line
 * `shell-slot-tree-view.test.tsx` registers, so whichever of the two files runs last the other still
 * reads the flag off the rendered text; the drag stub beside it registers nothing outward and is
 * inert in that file. The status bar is reduced to its Problems pair, the title bar and sidebar to
 * nothing — none of them has anything to do with this file, and all three open project-scoped
 * queries that cannot resolve without IPC. `@tauri-apps/api/webview` is faked because the
 * drag-and-drop subscription calls `getCurrentWebview()` synchronously inside an effect, which
 * throws with no `__TAURI_INTERNALS__`; this is the only module in `src` that imports it.
 *
 * The real `ShellSlotTreeView` is kept — the slot header's ✕ is the only user-facing way to reach
 * `onCloseSlot`, and going through it is what makes this a wiring test rather than a restatement of
 * the reducer.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))
mock.module('@widgets/app-shell/project-shell', () => ({
    ProjectShell: ({ projectId, isProblemsOpen }: { projectId: string; isProblemsOpen: boolean }) => (
        <>
            <div>{`shell:${projectId}${isProblemsOpen ? ' problems' : ''}`}</div>
            <SlotDndProbe scope={`outer:${projectId}`} />
            <DndContext>
                <SlotDndProbe scope={`inner:${projectId}`} />
                <SlotTabDragStub id={`tab:${projectId}`} />
            </DndContext>
        </>
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

/**
 * Down and straight back up, then a real wait: a started drag leaves a capture-phase `click`
 * swallower on the document that dnd-kit only removes 50ms after the drag ends
 * (`AbstractPointerSensor.detach`), and leaving it behind would neutralize the clicks of whatever
 * test runs next in this process (`docs/memory/test-conventions.md` §3).
 */
const CLICK_SUPPRESSION_TEARDOWN_MS = 60

const dndProbeText = (side: string) => screen.getByText(new RegExp(`^${side}:${LEFT_PROJECT_ID} `)).textContent

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

describe('AppShell 중첩 DndContext', () => {
    test('슬롯 안의 탭 컨텍스트는 창의 프로젝트 컨텍스트 안에 중첩되고, 그 안에서 시작한 드래그는 바깥을 깨우지 않는다', async () => {
        await renderShell()

        expect(dndProbeText('outer')).toBe(`outer:${LEFT_PROJECT_ID} slots=${LEFT_SLOT_ID},${RIGHT_SLOT_ID} active=none`)
        expect(dndProbeText('inner')).toBe(`inner:${LEFT_PROJECT_ID} slots=none active=none`)

        act(() => {
            fireEvent.pointerDown(screen.getByRole('button', { name: `tab:${LEFT_PROJECT_ID}` }), { isPrimary: true, button: 0 })
        })

        expect(dndProbeText('inner')).toBe(`inner:${LEFT_PROJECT_ID} slots=none active=tab:${LEFT_PROJECT_ID}`)
        expect(dndProbeText('outer')).toBe(`outer:${LEFT_PROJECT_ID} slots=${LEFT_SLOT_ID},${RIGHT_SLOT_ID} active=none`)

        await act(async () => {
            fireEvent.pointerUp(document)
            await new Promise((resolve) => setTimeout(resolve, CLICK_SUPPRESSION_TEARDOWN_MS))
        })
    })
})
