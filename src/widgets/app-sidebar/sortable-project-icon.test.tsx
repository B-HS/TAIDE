import { describe, expect, test } from 'bun:test'
import { DndContext } from '@dnd-kit/core'
import { SortableContext } from '@dnd-kit/sortable'
import type { ShellSlotEdge } from '@shared/api/bindings'
import { TooltipProvider } from '@shared/ui/tooltip'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'
import { SortableProjectIcon } from '@widgets/app-sidebar/sortable-project-icon'

/**
 * The menu route to a shell split (contract §0.1, 2a — the sidebar-to-slot drag arrives in 2b). Four
 * directions, targeting the focused slot, and disabled while the window has no slot to split at all.
 *
 * Nothing pre-checks whether the project is already in another slot: `project_open_in_slot` refuses
 * that server-side (§0.1 S-3) and the sidebar surfaces the error, so a menu that greyed the entries
 * out on a guess would only be able to disagree with the server.
 *
 * `useSortable` needs the dnd-kit context its real parent supplies, so the harness reproduces the
 * sidebar's `DndContext`/`SortableContext` pair rather than the icon carrying a fallback for tests.
 */
const PROJECT = { id: 'project-1', root: '/tmp/project-1', name: 'project-1' }

const OPEN_IN_SLOT_LABELS = ['shellSlot.openToTheRight', 'shellSlot.openBelow', 'shellSlot.openToTheLeft', 'shellSlot.openAbove']

const renderIcon = async (canOpenInShellSlot: boolean) => {
    const edges: ShellSlotEdge[] = []
    renderWithProviders(
        <TooltipProvider>
            <DndContext>
                <SortableContext items={[PROJECT.id]}>
                    <SortableProjectIcon
                        project={PROJECT}
                        active
                        dragging={false}
                        agents={[]}
                        badgeEnabled={false}
                        canOpenInShellSlot={canOpenInShellSlot}
                        onActivate={() => {}}
                        onOpenInShellSlot={(edge) => edges.push(edge)}
                    />
                </SortableContext>
            </DndContext>
        </TooltipProvider>,
    )

    fireEvent.contextMenu(screen.getByRole('button', { name: PROJECT.name }))
    await screen.findByRole('menuitem', { name: 'project.close' })

    return { edges }
}

describe('SortableProjectIcon 슬롯 분할 메뉴', () => {
    test('오른쪽·아래·왼쪽·위에 열기 4항목이 이 순서로 붙는다', async () => {
        await renderIcon(true)

        const labels = screen.getAllByRole('menuitem').map((item) => item.textContent)

        expect(labels.filter((label) => label && OPEN_IN_SLOT_LABELS.includes(label))).toEqual(OPEN_IN_SLOT_LABELS)
    })

    test('항목을 고르면 그 방향으로 슬롯 열기를 요청한다', async () => {
        const { edges } = await renderIcon(true)

        fireEvent.click(screen.getByRole('menuitem', { name: 'shellSlot.openBelow' }))

        expect(edges).toEqual(['bottom'])
    })

    test('분할할 슬롯이 없으면 4항목 전부 비활성이다', async () => {
        const { edges } = await renderIcon(false)

        for (const label of OPEN_IN_SLOT_LABELS) {
            const item = screen.getByRole('menuitem', { name: label })
            expect(item.getAttribute('data-disabled')).not.toBeNull()
            fireEvent.click(item)
        }

        expect(edges).toEqual([])
    })
})
