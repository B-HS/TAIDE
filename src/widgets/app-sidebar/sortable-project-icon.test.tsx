import { describe, expect, test } from 'bun:test'
import { DndContext } from '@dnd-kit/core'
import { SortableContext } from '@dnd-kit/sortable'
import type { ProjectGroup, ProjectGroupId, ProjectRef, ShellSlotEdge } from '@shared/api/bindings'
import { TooltipProvider } from '@shared/ui/tooltip'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'
import { SortableProjectIcon } from '@widgets/app-sidebar/sortable-project-icon'

/**
 * The menu route to a shell split (contract §0.1, 2a — the sidebar-to-slot drag arrives in 2b) and
 * the group membership items (§0.1 U-6, 2c). Four split directions, targeting the focused slot, and
 * disabled while the window has no slot to split at all.
 *
 * Nothing pre-checks whether the project is already in another slot: `project_open_in_slot` refuses
 * that server-side (§0.1 S-3) and the sidebar surfaces the error, so a menu that greyed the entries
 * out on a guess would only be able to disagree with the server.
 *
 * `useSortable` needs the dnd-kit context its real parent supplies, so the harness reproduces the
 * sidebar's `DndContext`/`SortableContext` pair rather than the icon carrying a fallback for tests.
 */
const PROJECT: ProjectRef = { id: 'project-1', root: '/tmp/project-1', name: 'project-1' }

const WORK_GROUP: ProjectGroup = { id: 'group-work', name: 'Work', members: [PROJECT.id] }

const SIDE_GROUP: ProjectGroup = { id: 'group-side', name: 'Side' }

const OPEN_IN_SLOT_LABELS = ['shellSlot.openToTheRight', 'shellSlot.openBelow', 'shellSlot.openToTheLeft', 'shellSlot.openAbove']

type RenderIconOptions = { canOpenInShellSlot?: boolean; groups?: ProjectGroup[]; groupId?: ProjectGroupId | null; project?: ProjectRef }

const mountIcon = ({ canOpenInShellSlot = true, groups = [], groupId = null, project = PROJECT }: RenderIconOptions = {}) => {
    const edges: ShellSlotEdge[] = []
    const addedGroupIds: ProjectGroupId[] = []
    const actions: string[] = []

    renderWithProviders(
        <TooltipProvider>
            <DndContext>
                <SortableContext items={[PROJECT.id]}>
                    <SortableProjectIcon
                        project={project}
                        active
                        dragging={false}
                        agents={[]}
                        badgeEnabled={false}
                        canOpenInShellSlot={canOpenInShellSlot}
                        groups={groups}
                        groupId={groupId}
                        onActivate={() => {}}
                        onOpenInShellSlot={(edge) => edges.push(edge)}
                        onAddToGroup={(id) => addedGroupIds.push(id)}
                        onCreateGroup={() => actions.push('create')}
                        onRemoveFromGroup={() => actions.push('remove')}
                    />
                </SortableContext>
            </DndContext>
        </TooltipProvider>,
    )

    return { edges, addedGroupIds, actions, project }
}

/**
 * Radix marks the rest of the tree `aria-hidden` while a context menu is open, so the rail icon
 * itself is only queryable before the menu opens — which is why mounting and opening the menu are
 * two steps rather than one.
 */
const renderIcon = async (options: RenderIconOptions = {}) => {
    const mounted = mountIcon(options)

    fireEvent.contextMenu(screen.getByRole('button', { name: mounted.project.name }))
    await screen.findByRole('menuitem', { name: 'project.close' })

    return mounted
}

/**
 * The rail icon sits inside the tooltip trigger's wrapper, which is the element the dim lands on —
 * the tooltip copy itself only exists while the tooltip is open, so the dim is what a closed rail
 * can be asserted on.
 */
const iconWrapperOf = (name: string) => screen.getByRole('button', { name }).parentElement

/** Radix opens a submenu on hover or on ArrowRight; the keyboard route is the deterministic one in a harness with no layout (`tab-bar-context-menu.test.tsx` precedent). */
const openAddToGroupSubmenu = async () => {
    fireEvent.keyDown(screen.getByRole('menuitem', { name: 'projectGroup.addTo' }), { key: 'ArrowRight' })
    await screen.findByRole('menuitem', { name: 'projectGroup.newGroup' })
}

describe('SortableProjectIcon 슬롯 분할 메뉴', () => {
    test('오른쪽·아래·왼쪽·위에 열기 4항목이 이 순서로 붙는다', async () => {
        await renderIcon()

        const labels = screen.getAllByRole('menuitem').map((item) => item.textContent)

        expect(labels.filter((label) => label && OPEN_IN_SLOT_LABELS.includes(label))).toEqual(OPEN_IN_SLOT_LABELS)
    })

    test('항목을 고르면 그 방향으로 슬롯 열기를 요청한다', async () => {
        const { edges } = await renderIcon()

        fireEvent.click(screen.getByRole('menuitem', { name: 'shellSlot.openBelow' }))

        expect(edges).toEqual(['bottom'])
    })

    test('분할할 슬롯이 없으면 4항목 전부 비활성이다', async () => {
        const { edges } = await renderIcon({ canOpenInShellSlot: false })

        for (const label of OPEN_IN_SLOT_LABELS) {
            const item = screen.getByRole('menuitem', { name: label })
            expect(item.getAttribute('data-disabled')).not.toBeNull()
            fireEvent.click(item)
        }

        expect(edges).toEqual([])
    })
})

describe('SortableProjectIcon 그룹 메뉴', () => {
    test('그룹이 없어도 서브메뉴에 새 그룹 항목이 있다', async () => {
        const { actions } = await renderIcon()

        await openAddToGroupSubmenu()
        fireEvent.click(screen.getByRole('menuitem', { name: 'projectGroup.newGroup' }))

        expect(actions).toEqual(['create'])
    })

    test('서브메뉴는 그룹 목록을 그리고 이미 속한 그룹은 비활성이다', async () => {
        const { addedGroupIds } = await renderIcon({ groups: [WORK_GROUP, SIDE_GROUP], groupId: WORK_GROUP.id })

        await openAddToGroupSubmenu()

        expect(screen.getByRole('menuitem', { name: WORK_GROUP.name }).getAttribute('data-disabled')).not.toBeNull()

        fireEvent.click(screen.getByRole('menuitem', { name: SIDE_GROUP.name }))

        expect(addedGroupIds).toEqual([SIDE_GROUP.id])
    })

    test('그룹에서 제거는 속한 그룹이 있을 때만 활성이다', async () => {
        const { actions } = await renderIcon({ groups: [WORK_GROUP] })

        const item = screen.getByRole('menuitem', { name: 'projectGroup.removeFrom' })
        expect(item.getAttribute('data-disabled')).not.toBeNull()
        fireEvent.click(item)

        expect(actions).toEqual([])
    })

    test('속한 그룹이 있으면 그룹에서 제거가 동작한다', async () => {
        const { actions } = await renderIcon({ groups: [WORK_GROUP], groupId: WORK_GROUP.id })

        fireEvent.click(screen.getByRole('menuitem', { name: 'projectGroup.removeFrom' }))

        expect(actions).toEqual(['remove'])
    })
})

/**
 * A project restored from a session whose root is gone from disk (`ProjectRef.root_missing`) used to
 * look exactly like a healthy one in the rail, while `Open Recent` and the Welcome list already
 * disabled the very same entry (d-67 #14). It stays clickable on purpose: its slot is real, and
 * focusing it is how the explorer's "reopen" recovery is reached (d-67 #15).
 */
describe('SortableProjectIcon 루트 부재 표시', () => {
    test('root_missing 이면 아이콘을 흐리게 그린다', () => {
        mountIcon({ project: { ...PROJECT, rootMissing: true } })

        expect(iconWrapperOf(PROJECT.name)?.className).toContain('opacity-50')
    })

    test('root_missing 이어도 클릭은 막지 않는다 — 슬롯을 열어 복구 액션에 닿아야 한다', () => {
        mountIcon({ project: { ...PROJECT, rootMissing: true } })

        expect(screen.getByRole('button', { name: PROJECT.name }).hasAttribute('disabled')).toBe(false)
    })

    test('루트가 멀쩡하면 흐려지지 않는다', () => {
        mountIcon()

        expect(iconWrapperOf(PROJECT.name)?.className).not.toContain('opacity-50')
    })
})
