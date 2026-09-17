import { describe, expect, mock, spyOn, test } from 'bun:test'
import type { SessionShellState } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { SHELL_SLOT_ID_ATTRIBUTE } from '@shared/lib/shell-slot'
import { useShellSlotFocus } from '@shared/lib/shell-slot-context'
import { TooltipProvider } from '@shared/ui/tooltip'
import { ShellSlotHeader } from '@features/shell-slot/shell-slot-header'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * Focus tracking is a *DOM* concern (contract §0.1 U-7): Rust persists which slot is focused, but
 * the signal is this window's own pointerdown/focusin, resolved to the slot that contains the event
 * target. These cases lock the three outcomes that matter — a click inside a slot takes focus, a
 * click outside every slot (a portal's menu, the sidebar, the status bar) deliberately changes
 * nothing, and keyboard focus moving into a slot counts the same as a click.
 *
 * No `mock.module` here. The two commands the provider calls reject on their own under `bun:test`
 * (no `__TAURI_INTERNALS__`), and React Query keeps seeded data through a failed refetch — so
 * seeding `SESSION.SHELL_STATE` is enough to give the provider a tree, and the focus write is
 * allowed to fail exactly as an offline round trip would. That is also what the local override
 * exists for: focus has to move within the frame, not after the IPC answers.
 *
 * `@shared/lib/monaco/setup` is stubbed because `entities/layout` — reached through
 * `entities/project/project.query` — touches monaco at import time
 * (`docs/memory/test-conventions.md` §3).
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

const importProvider = () => import('@app/providers/shell-slot-provider')

const LEFT_SLOT_ID = 'shellslot-left'
const RIGHT_SLOT_ID = 'shellslot-right'
const LEFT_PROJECT_ID = 'project-left'
const RIGHT_PROJECT_ID = 'project-right'

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

const FocusProbe = () => {
    const { focusedShellSlotId, focusedProjectId } = useShellSlotFocus()
    return <output>{`${focusedShellSlotId ?? 'none'}/${focusedProjectId ?? 'none'}`}</output>
}

const focusLabel = () => screen.getByRole('status').textContent

/** Drains the mutation's own promise chain (retryer hop, `onSettled`, the notify-manager microtask) so its React Query state lands *inside* the `act` scope the events were fired in, instead of after it. */
const settleMutation = () => new Promise((resolve) => setTimeout(resolve, 0))

/** Seeded with `gcTime: Infinity` because the test client collects observer-less queries immediately (`docs/memory/test-conventions.md` §3). */
const renderProvider = async () => {
    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.SESSION.SHELL_STATE, queryFn: () => SHELL_STATE, gcTime: Infinity })
    const { ShellSlotProvider } = await importProvider()

    return renderWithProviders(
        <ShellSlotProvider>
            <FocusProbe />
            <div {...{ [SHELL_SLOT_ID_ATTRIBUTE]: LEFT_SLOT_ID }}>
                <button type='button'>left editor</button>
            </div>
            <div {...{ [SHELL_SLOT_ID_ATTRIBUTE]: RIGHT_SLOT_ID }}>
                <TooltipProvider>
                    <ShellSlotHeader label='right project' rootMissing={false} canClose onClose={() => undefined} />
                </TooltipProvider>
                <button type='button'>right editor</button>
            </div>
            <button type='button'>status bar</button>
        </ShellSlotProvider>,
        { queryClient },
    )
}

describe('ShellSlotProvider 포커스 추적', () => {
    test('세션이 알려준 포커스 슬롯과 그 프로젝트로 시작한다', async () => {
        await renderProvider()

        expect(focusLabel()).toBe(`${LEFT_SLOT_ID}/${LEFT_PROJECT_ID}`)
    })

    test('다른 슬롯 안을 클릭하면 그 슬롯으로 포커스가 옮겨간다', async () => {
        await renderProvider()

        fireEvent.pointerDown(screen.getByRole('button', { name: 'right editor' }))

        expect(focusLabel()).toBe(`${RIGHT_SLOT_ID}/${RIGHT_PROJECT_ID}`)
    })

    test('키보드 포커스가 슬롯 안으로 들어가도 같다 — 클릭만이 신호가 아니다', async () => {
        await renderProvider()

        fireEvent.focusIn(screen.getByRole('button', { name: 'right editor' }))

        expect(focusLabel()).toBe(`${RIGHT_SLOT_ID}/${RIGHT_PROJECT_ID}`)
    })

    test('클릭 한 번이 내는 pointerdown+focusin 쌍은 포커스 IPC 를 한 번만 부른다', async () => {
        const focusShellSlot = spyOn(commands, 'sessionFocusShellSlot').mockResolvedValue({ status: 'ok', data: null })
        await renderProvider()

        const target = screen.getByRole('button', { name: 'right editor' })
        await act(async () => {
            fireEvent.pointerDown(target)
            fireEvent.focusIn(target)
            await settleMutation()
        })

        expect(focusShellSlot).toHaveBeenCalledTimes(1)
        expect(focusLabel()).toBe(`${RIGHT_SLOT_ID}/${RIGHT_PROJECT_ID}`)
    })

    test('다른 슬롯의 헤더 ✕ 를 눌러도 그 슬롯으로 포커스가 옮겨가지 않는다 — 닫기는 포커스 이동이 아니다', async () => {
        const focusShellSlot = spyOn(commands, 'sessionFocusShellSlot').mockResolvedValue({ status: 'ok', data: null })
        await renderProvider()

        const closeButton = screen.getByRole('button', { name: 'shellSlot.close' })
        await act(async () => {
            fireEvent.pointerDown(closeButton)
            fireEvent.focusIn(closeButton)
            await settleMutation()
        })

        expect(focusShellSlot).not.toHaveBeenCalled()
        expect(focusLabel()).toBe(`${LEFT_SLOT_ID}/${LEFT_PROJECT_ID}`)
    })

    test('슬롯 밖(상태바·사이드바·포털)을 눌러도 포커스는 그대로다', async () => {
        await renderProvider()

        fireEvent.pointerDown(screen.getByRole('button', { name: 'right editor' }))
        fireEvent.pointerDown(screen.getByRole('button', { name: 'status bar' }))

        expect(focusLabel()).toBe(`${RIGHT_SLOT_ID}/${RIGHT_PROJECT_ID}`)
    })
})
