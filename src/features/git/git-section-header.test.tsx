import { describe, expect, test } from 'bun:test'
import { GitSectionHeader } from '@features/git/git-section-header'
import { TooltipProvider } from '@shared/ui/tooltip'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The one header every SCM section uses (batch 4 contract §E.2-2). What is verified here is the
 * keyboard/ARIA contract the panel's roving focus depends on — it is a real `role="button"` stop
 * that reports `aria-expanded`, ArrowRight/ArrowLeft only act in the direction that would change
 * something, and a group action inside the header does *not* also toggle the section. The count
 * badge is asserted while collapsed on purpose: a collapsed "Staged Changes" that hid its count
 * reads as "nothing is staged".
 *
 * Sticky positioning, the chevron rotation and the hover reveal are CSS — happy-dom has no layout
 * (`docs/memory/test-conventions.md` §5), so those stay with the e2e spec and the hand-QA sheet.
 */
const renderHeader = (props: Partial<Parameters<typeof GitSectionHeader>[0]> = {}) => {
    const toggles: number[] = []
    const rendered = renderWithProviders(
        <TooltipProvider>
            <GitSectionHeader title='Staged Changes' count={3} expanded={true} onToggle={() => toggles.push(1)} {...props} actions={props.actions} />
        </TooltipProvider>,
    )

    return { ...rendered, toggles, header: screen.getByRole('button', { name: /Staged Changes/ }) }
}

describe('GitSectionHeader', () => {
    test('제목·개수를 그리고 접힘 상태를 aria-expanded 로 보고한다', () => {
        const { header } = renderHeader({ expanded: false })

        expect(header.getAttribute('aria-expanded')).toBe('false')
        expect(header.textContent).toContain('Staged Changes')
        expect(header.textContent).toContain('3')
    })

    test('펼쳐진 상태도 aria-expanded 로 구분된다', () => {
        const { header } = renderHeader({ expanded: true })

        expect(header.getAttribute('aria-expanded')).toBe('true')
    })

    test('로빙 포커스 정지점이 되도록 tabIndex 와 e2e 마커를 갖는다', () => {
        const { header } = renderHeader()

        expect(header.getAttribute('tabindex')).toBe('0')
        expect(header.hasAttribute('data-git-section-header')).toBe(true)
    })

    test('클릭하면 토글한다', () => {
        const { header, toggles } = renderHeader()

        fireEvent.click(header)

        expect(toggles.length).toBe(1)
    })

    test('Enter 와 Space 로 토글한다 (role=button 접근성 패턴)', () => {
        const { header, toggles } = renderHeader()

        fireEvent.keyDown(header, { key: 'Enter' })
        fireEvent.keyDown(header, { key: ' ' })

        expect(toggles.length).toBe(2)
    })

    test('Space 는 기본 스크롤 동작을 막는다', () => {
        const { header } = renderHeader()

        const notPrevented = fireEvent.keyDown(header, { key: ' ' })

        expect(notPrevented).toBe(false)
    })

    test('접혀 있을 때 ArrowRight 는 펼치고, 이미 펼쳐져 있으면 아무 일도 하지 않는다', () => {
        const collapsed = renderHeader({ expanded: false })
        fireEvent.keyDown(collapsed.header, { key: 'ArrowRight' })
        expect(collapsed.toggles.length).toBe(1)

        collapsed.unmount()

        const expanded = renderHeader({ expanded: true })
        fireEvent.keyDown(expanded.header, { key: 'ArrowRight' })
        expect(expanded.toggles.length).toBe(0)
    })

    test('펼쳐져 있을 때 ArrowLeft 는 접고, 이미 접혀 있으면 아무 일도 하지 않는다', () => {
        const expanded = renderHeader({ expanded: true })
        fireEvent.keyDown(expanded.header, { key: 'ArrowLeft' })
        expect(expanded.toggles.length).toBe(1)

        expanded.unmount()

        const collapsed = renderHeader({ expanded: false })
        fireEvent.keyDown(collapsed.header, { key: 'ArrowLeft' })
        expect(collapsed.toggles.length).toBe(0)
    })

    test('그룹 액션 버튼은 자기 핸들러만 부르고 섹션을 토글하지 않는다', () => {
        const actionClicks: string[] = []
        const { toggles } = renderHeader({
            actions: [{ id: 'stage-all', label: 'git.stageAll', icon: null, onClick: () => actionClicks.push('stage-all') }],
        })

        fireEvent.click(screen.getByRole('button', { name: 'git.stageAll' }))

        expect(actionClicks).toEqual(['stage-all'])
        expect(toggles.length).toBe(0)
    })

    test('액션이 없으면 액션 영역 자체를 그리지 않는다', () => {
        renderHeader()

        expect(screen.getAllByRole('button').length).toBe(1)
    })
})
