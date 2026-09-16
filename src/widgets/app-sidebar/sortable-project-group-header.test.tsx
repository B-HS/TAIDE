import { describe, expect, spyOn, test } from 'bun:test'
import { DndContext } from '@dnd-kit/core'
import { SortableContext } from '@dnd-kit/sortable'
import { toast } from 'sonner'
import type { ProjectGroup } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { act, fireEvent, renderWithProviders, screen } from '@shared/testing/render'
import { SortableProjectGroupHeader } from '@widgets/app-sidebar/sortable-project-group-header'

/**
 * The group's row in the rail (contract §1.C): the fold, and the menu that opens, edits, or deletes
 * the group.
 *
 * The fold is asserted as a *command*, not as local state — `ProjectGroup.collapsed` lives in
 * `session.json` so the fold survives a restart and reaches every window, and a header that only
 * flipped a `useState` would look identical in this harness while losing the fold on the next mount.
 *
 * `useSortable` needs the dnd-kit context its real parent supplies, so the harness reproduces the
 * sidebar's `DndContext`/`SortableContext` pair (`sortable-project-icon.test.tsx` does the same).
 */
const GROUP: ProjectGroup = { id: 'group-work', name: 'Work', members: ['project-1'], collapsed: false }

const MENU_LABELS = ['projectGroup.open', 'projectGroup.edit', 'projectGroup.delete']

const renderHeader = (group: ProjectGroup = GROUP) =>
    renderWithProviders(
        <DndContext>
            <SortableContext items={[group.id]}>
                <SortableProjectGroupHeader group={group} />
            </SortableContext>
        </DndContext>,
    )

/** Drains the mutation's own promise chain so the IPC call it makes lands inside an `act` scope (`use-project-drag.test.tsx` precedent). */
const settle = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))))

const header = (group: ProjectGroup = GROUP) => screen.getByRole('button', { name: group.name })

const openMenu = async (group: ProjectGroup = GROUP) => {
    fireEvent.contextMenu(header(group))
    await screen.findByRole('menuitem', { name: 'projectGroup.open' })
}

describe('SortableProjectGroupHeader 접기', () => {
    test('펼쳐진 그룹은 이름과 펼침 상태를 알린다', () => {
        renderHeader()

        expect(header().getAttribute('aria-expanded')).toBe('true')
    })

    test('접힌 그룹은 접힘 상태를 알린다 — collapsed 가 없으면 펼침으로 읽는다', () => {
        renderHeader({ ...GROUP, collapsed: true })

        expect(header().getAttribute('aria-expanded')).toBe('false')

        renderHeader({ id: 'group-bare', name: 'Bare' })

        expect(screen.getByRole('button', { name: 'Bare' }).getAttribute('aria-expanded')).toBe('true')
    })

    test('헤더를 누르면 반대 접힘 상태를 세션에 쓴다', async () => {
        const setCollapsed = spyOn(commands, 'projectGroupSetCollapsed').mockResolvedValue({ status: 'ok', data: null })
        renderHeader()

        fireEvent.click(header())
        await settle()

        expect(setCollapsed).toHaveBeenCalledWith(GROUP.id, true)
    })
})

describe('SortableProjectGroupHeader 메뉴', () => {
    test('그룹 열기·이름 색 변경·그룹 삭제 3항목이 이 순서로 붙는다', async () => {
        renderHeader()
        await openMenu()

        expect(screen.getAllByRole('menuitem').map((item) => item.textContent)).toEqual(MENU_LABELS)
    })

    test('그룹 열기는 열린 개수와 건너뛴 개수를 토스트한다', async () => {
        spyOn(commands, 'projectGroupOpen').mockResolvedValue({ status: 'ok', data: { opened: ['project-1', 'project-2'], skipped: ['project-3'] } })
        const toastSuccess = spyOn(toast, 'success')
        renderHeader()
        await openMenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'projectGroup.open' }))
        await settle()

        expect(toastSuccess).toHaveBeenCalledWith('projectGroup.openResult')
    })

    test('그룹 열기가 거부되면 그 메시지를 토스트한다', async () => {
        spyOn(commands, 'projectGroupOpen').mockResolvedValue({ status: 'error', error: { code: 'NotFound', message: 'no such group' } })
        const toastError = spyOn(toast, 'error')
        renderHeader()
        await openMenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'projectGroup.open' }))
        await settle()

        expect(toastError).toHaveBeenCalledWith('no such group')
    })

    test('삭제는 확인 다이얼로그를 거친 뒤에만 커맨드를 보낸다 — 멤버는 열린 채로 남는다', async () => {
        const deleteGroup = spyOn(commands, 'projectGroupDelete').mockResolvedValue({ status: 'ok', data: null })
        renderHeader()
        await openMenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'projectGroup.delete' }))
        await screen.findByText('projectGroup.deleteConfirmDescription')

        expect(deleteGroup).not.toHaveBeenCalled()

        fireEvent.click(screen.getByRole('button', { name: 'projectGroup.delete' }))
        await settle()

        expect(deleteGroup).toHaveBeenCalledWith(GROUP.id)
    })

    test('이름 색 변경은 바뀐 축만 쓴다', async () => {
        const rename = spyOn(commands, 'projectGroupRename').mockResolvedValue({ status: 'ok', data: null })
        const setColor = spyOn(commands, 'projectGroupSetColor').mockResolvedValue({ status: 'ok', data: null })
        renderHeader()
        await openMenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'projectGroup.edit' }))
        fireEvent.change(await screen.findByRole('textbox'), { target: { value: 'Renamed' } })
        fireEvent.click(screen.getByRole('button', { name: 'common.save' }))
        await settle()

        expect(rename).toHaveBeenCalledWith(GROUP.id, 'Renamed')
        expect(setColor).not.toHaveBeenCalled()
    })
})
