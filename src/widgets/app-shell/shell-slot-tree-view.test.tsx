import { describe, expect, mock, test } from 'bun:test'
import type { ProjectRef, ShellSlotTree } from '@shared/api/bindings'
import { SHELL_SLOT_ID_ATTRIBUTE } from '@shared/lib/shell-slot'
import { TooltipProvider } from '@shared/ui/tooltip'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * How the window paints the slot tree (contract §1.B). Four properties are locked, in the order
 * they matter:
 *
 * 1. A single-leaf tree renders exactly the pre-split screen — one project shell, no slot header and
 *    no separator. Every window that never splits has to stay indistinguishable from before.
 * 2. A split renders one shell per leaf, each headed and each marked with its own slot id, which is
 *    what the focus tracker resolves a pointerdown against (`shell-slot-provider.tsx`).
 * 3. Zen shows only the focused slot and leaves the *tree* untouched (§0.1 S-6), so leaving Zen
 *    restores the arrangement rather than having collapsed it.
 * 4. Zen hides the other slots instead of unmounting them. A remount would throw away the editors,
 *    scroll positions and terminal buffers inside every shell — including the focused one, back when
 *    Zen swapped the component at the root of the tree — so the same DOM node has to survive a
 *    toggle. This is asserted by node identity rather than a mount counter, which keeps the stub
 *    below interchangeable with the one `app-shell.test.tsx` registers for the same module.
 *
 * `ProjectShell` is stubbed — it mounts the whole explorer/editor stack, none of which this file is
 * about. Registered before the view is pulled in through a dynamic `import()` because `mock.module`
 * is process-global and last-registration-wins (`docs/memory/test-conventions.md` §3).
 */
mock.module('@widgets/app-shell/project-shell', () => ({
    ProjectShell: ({ projectId, isProblemsOpen }: { projectId: string; isProblemsOpen: boolean }) => (
        <div>{`shell:${projectId}${isProblemsOpen ? ' problems' : ''}`}</div>
    ),
}))

const importTreeView = () => import('@widgets/app-shell/shell-slot-tree-view')

const LEFT_SLOT_ID = 'shellslot-left'
const RIGHT_SLOT_ID = 'shellslot-right'

const PROJECTS: ProjectRef[] = [
    { id: 'project-left', root: '/tmp/left', name: 'left' },
    { id: 'project-right', root: '/tmp/right', name: 'right' },
]

const LEFT_LEAF: ShellSlotTree = { node: 'leaf', slotId: LEFT_SLOT_ID, projectId: 'project-left' }
const RIGHT_LEAF: ShellSlotTree = { node: 'leaf', slotId: RIGHT_SLOT_ID, projectId: 'project-right' }
const SPLIT: ShellSlotTree = { node: 'split', dir: 'horizontal', sizes: [50, 50], children: [LEFT_LEAF, RIGHT_LEAF] }

const RESIZER_THICKNESS_PX = 4

const renderTree = async (tree: ShellSlotTree, zen = false) => {
    const { ShellSlotTreeView } = await importTreeView()
    const closedSlots: string[] = []
    const view = (zenMode: boolean) => (
        <TooltipProvider>
            <ShellSlotTreeView
                tree={tree}
                projects={PROJECTS}
                focusedShellSlotId={LEFT_SLOT_ID}
                zen={zenMode}
                resizerThickness={RESIZER_THICKNESS_PX}
                problemsOpenSlotIds={[]}
                onCloseProblems={() => {}}
                onCloseSlot={(slotId) => closedSlots.push(slotId)}
                onCommitSizes={() => {}}
            />
        </TooltipProvider>
    )
    const rendered = renderWithProviders(view(zen))
    return { ...rendered, closedSlots, setZen: (zenMode: boolean) => rendered.rerender(view(zenMode)) }
}

const renderedShells = () => screen.getAllByText(/^shell:/).map((element) => element.textContent)

const shellNodeOf = (projectId: string) => screen.getByText(`shell:${projectId}`)

const slotIdAttributes = (container: HTMLElement) =>
    [...container.querySelectorAll('[data-shell-slot-id]')].map((element) => element.getAttribute('data-shell-slot-id'))

/** `hidden` sits on the enclosing `Panel` (the flex item), not on the slot element itself, so "is this slot on screen?" is answered by looking upwards from the slot marker. */
const isSlotHidden = (container: HTMLElement, slotId: string) =>
    !!container.querySelector(`[${SHELL_SLOT_ID_ATTRIBUTE}='${slotId}']`)?.closest('[hidden]')

describe('ShellSlotTreeView', () => {
    test('슬롯이 하나면 헤더 없이 프로젝트 셸만 그린다 — 분할 이전 화면 그대로', async () => {
        const { container } = await renderTree(LEFT_LEAF)

        expect(renderedShells()).toEqual(['shell:project-left'])
        expect(screen.queryByRole('button', { name: 'shellSlot.close' })).toBeNull()
        expect(slotIdAttributes(container)).toEqual([LEFT_SLOT_ID])
    })

    test('2분할이면 슬롯마다 셸·헤더·슬롯 id 표식이 하나씩 붙는다', async () => {
        const { container } = await renderTree(SPLIT)

        expect(renderedShells()).toEqual(['shell:project-left', 'shell:project-right'])
        expect(screen.getAllByRole('button', { name: 'shellSlot.close' })).toHaveLength(2)
        expect(slotIdAttributes(container)).toEqual([LEFT_SLOT_ID, RIGHT_SLOT_ID])
    })

    test('헤더의 닫기는 그 슬롯만 닫는다', async () => {
        const { closedSlots } = await renderTree(SPLIT)

        fireEvent.click(screen.getAllByRole('button', { name: 'shellSlot.close' })[1])

        expect(closedSlots).toEqual([RIGHT_SLOT_ID])
    })

    test('Zen 이면 포커스 슬롯만 보이고, Zen 을 나가면 배치가 그대로 돌아온다 — 트리를 건드리지 않는다', async () => {
        const { container, setZen } = await renderTree(SPLIT, true)

        expect(isSlotHidden(container, LEFT_SLOT_ID)).toBe(false)
        expect(isSlotHidden(container, RIGHT_SLOT_ID)).toBe(true)
        expect(screen.queryByRole('button', { name: 'shellSlot.close' })).toBeNull()
        expect(slotIdAttributes(container)).toEqual([LEFT_SLOT_ID, RIGHT_SLOT_ID])

        setZen(false)

        expect(isSlotHidden(container, LEFT_SLOT_ID)).toBe(false)
        expect(isSlotHidden(container, RIGHT_SLOT_ID)).toBe(false)
        expect(renderedShells()).toEqual(['shell:project-left', 'shell:project-right'])
    })

    test('Zen 을 켜고 꺼도 어느 슬롯의 프로젝트 셸도 재마운트되지 않는다', async () => {
        const { setZen } = await renderTree(SPLIT)
        const focusedShell = shellNodeOf('project-left')
        const hiddenShell = shellNodeOf('project-right')

        setZen(true)

        expect(shellNodeOf('project-left')).toBe(focusedShell)
        expect(shellNodeOf('project-right')).toBe(hiddenShell)

        setZen(false)

        expect(shellNodeOf('project-left')).toBe(focusedShell)
        expect(shellNodeOf('project-right')).toBe(hiddenShell)
    })

    test('슬롯이 하나뿐인 창의 Zen 토글도 마찬가지다 — 분할하지 않은 창이 회귀의 진원지였다', async () => {
        const { setZen } = await renderTree(LEFT_LEAF)
        const shell = shellNodeOf('project-left')

        setZen(true)

        expect(shellNodeOf('project-left')).toBe(shell)

        setZen(false)

        expect(shellNodeOf('project-left')).toBe(shell)
    })
})
