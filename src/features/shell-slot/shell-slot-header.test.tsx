import { describe, expect, test } from 'bun:test'
import { TooltipProvider } from '@shared/ui/tooltip'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'
import { ShellSlotHeader } from '@features/shell-slot/shell-slot-header'

/**
 * The slot header only exists while the window holds more than one slot, so the case that needs
 * locking is the *last* slot: `shell_slot_close` refuses it server-side (a window with projects open
 * always shows one of them), and the button has to say so before the user clicks rather than
 * answering with an error toast.
 *
 * `TooltipProvider` is composed in because `renderWithProviders` carries only Query + i18n, and
 * `IconButton` is a radix tooltip.
 */
const SLOT_LABEL = 'taide'

const renderHeader = (canClose: boolean, rootMissing = false) => {
    const closes: number[] = []
    renderWithProviders(
        <TooltipProvider>
            <ShellSlotHeader label={SLOT_LABEL} rootMissing={rootMissing} canClose={canClose} onClose={() => closes.push(1)} />
        </TooltipProvider>,
    )
    return { closes, closeButton: screen.getByRole('button', { name: 'shellSlot.close' }) }
}

describe('ShellSlotHeader', () => {
    test('슬롯의 프로젝트 이름을 보여준다', () => {
        renderHeader(true)

        expect(screen.getByText(SLOT_LABEL)).toBeTruthy()
    })

    test('닫기 버튼이 슬롯 닫기를 요청한다', () => {
        const { closes, closeButton } = renderHeader(true)

        fireEvent.click(closeButton)

        expect(closes).toHaveLength(1)
    })

    test('루트가 사라진 프로젝트면 경고 배지를 단다 — 빈 트리와 구분할 유일한 표시다', () => {
        renderHeader(true, true)

        expect(screen.getByRole('img', { name: 'app.recentProjectRootMissing' })).toBeTruthy()
    })

    test('루트가 멀쩡하면 경고 배지가 없다', () => {
        renderHeader(true)

        expect(screen.queryByRole('img', { name: 'app.recentProjectRootMissing' })).toBeNull()
    })

    test('마지막 슬롯이면 닫기 버튼이 비활성이다 — 서버도 같은 이유로 거부한다', () => {
        const { closes, closeButton } = renderHeader(false)

        expect(closeButton.hasAttribute('disabled')).toBe(true)

        fireEvent.click(closeButton)

        expect(closes).toEqual([])
    })
})
