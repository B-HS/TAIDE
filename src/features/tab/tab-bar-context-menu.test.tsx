import { describe, expect, test } from 'bun:test'
import type { SplitEdge } from '@features/tab/tab-context-menu'
import { TabBarContextMenu } from '@features/tab/tab-bar-context-menu'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The tab bar's empty-space menu (batch 4 contract §G.2-2). Unlike the terminal menu, entries that
 * do not apply are **hidden**, matching the tab menu's own policy (`docs/features/tabs.md` §3.1) —
 * `split` above all, because `layout_split` moves a named tab and a pane with no active tab would
 * get `NotFound` from Rust rather than a no-op.
 *
 * `buildTabBarMenuItems` has its own tests for the visibility table; what is added here is that the
 * rendered menu actually honours it, that separators still land on group boundaries *after* items
 * were filtered out, and that each id reaches the handler the pane bar passed for it.
 */
type MenuCalls = { name: string; edge?: SplitEdge }[]

const renderMenu = async ({ hasTabs = true, hasActiveTab = true, hasClosedTabs = true } = {}) => {
    const calls: MenuCalls = []
    const record = (name: string) => () => calls.push({ name })
    const rendered = renderWithProviders(
        <TabBarContextMenu
            hasTabs={hasTabs}
            hasActiveTab={hasActiveTab}
            hasClosedTabs={hasClosedTabs}
            onNewFile={record('newFile')}
            onNewTerminal={record('newTerminal')}
            onReopenClosedTab={record('reopenClosed')}
            onCloseSaved={record('closeSaved')}
            onCloseAll={record('closeAll')}
            onOpenWelcome={record('openWelcome')}
            onSplit={(edge) => calls.push({ name: 'split', edge })}>
            <div data-tab-bar-filler>filler</div>
        </TabBarContextMenu>,
    )
    const filler = rendered.container.querySelector('[data-tab-bar-filler]')
    if (!filler) throw new Error('filler not rendered')
    fireEvent.contextMenu(filler)
    await screen.findByRole('menuitem', { name: 'tab.newUntitledFile' })

    return { ...rendered, calls }
}

const visibleLabels = () => screen.getAllByRole('menuitem').map((item) => item.textContent)

describe('TabBarContextMenu 표시 규칙', () => {
    test('탭이 하나도 없고 닫은 탭도 없으면 만들기·Welcome 만 남는다', async () => {
        await renderMenu({ hasTabs: false, hasActiveTab: false, hasClosedTabs: false })

        expect(visibleLabels()).toEqual(['tab.newUntitledFile', 'tab.newTerminal', 'tab.openWelcome'])
    })

    test('활성 탭이 있으면 분할 항목이 나타난다 (탭 0 에서는 Rust 가 NotFound 를 내므로 숨긴다)', async () => {
        await renderMenu({ hasTabs: true, hasActiveTab: true, hasClosedTabs: false })

        expect(visibleLabels()).toContain('tab.split')
    })

    test('탭이 있어도 활성 탭이 없으면 분할은 숨긴다', async () => {
        await renderMenu({ hasTabs: true, hasActiveTab: false, hasClosedTabs: false })

        expect(visibleLabels()).not.toContain('tab.split')
        expect(visibleLabels()).toContain('tab.closeAll')
    })

    test('닫은 탭이 있을 때만 다시 열기가 나타난다', async () => {
        await renderMenu({ hasClosedTabs: true })

        expect(visibleLabels()).toContain('keymap.reopenClosedTab')
    })

    test('모든 조건이 갖춰지면 7 항목이 정해진 순서로 나온다', async () => {
        await renderMenu()

        expect(visibleLabels()).toEqual([
            'tab.newUntitledFile',
            'tab.newTerminal',
            'keymap.reopenClosedTab',
            'tab.closeSaved',
            'tab.closeAll',
            'tab.split',
            'tab.openWelcome',
        ])
    })

    test('구분선은 그룹 경계에만 그려지고, 항목이 걸러져도 어긋나지 않는다', async () => {
        await renderMenu()

        expect(screen.getAllByRole('separator').length).toBe(4)
    })

    test('한 그룹이 통째로 사라지면 구분선도 함께 줄어든다', async () => {
        await renderMenu({ hasTabs: false, hasActiveTab: false, hasClosedTabs: false })

        expect(screen.getAllByRole('separator').length).toBe(1)
    })
})

describe('TabBarContextMenu 동작', () => {
    test('새 터미널 항목이 자기 핸들러만 부른다', async () => {
        const { calls } = await renderMenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'tab.newTerminal' }))

        expect(calls.map((call) => call.name)).toEqual(['newTerminal'])
    })

    test('닫은 탭 다시 열기 항목이 자기 핸들러만 부른다 (선택 즉시 메뉴가 닫히므로 렌더당 1회)', async () => {
        const { calls } = await renderMenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'keymap.reopenClosedTab' }))

        expect(calls.map((call) => call.name)).toEqual(['reopenClosed'])
    })

    test('저장된 탭 닫기·모든 탭 닫기가 서로 다른 핸들러로 간다', async () => {
        const { calls } = await renderMenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'tab.closeAll' }))

        expect(calls.map((call) => call.name)).toEqual(['closeAll'])
    })

    test('분할 서브메뉴에서 고른 방향을 그대로 넘긴다', async () => {
        const { calls } = await renderMenu()

        fireEvent.keyDown(screen.getByRole('menuitem', { name: 'tab.split' }), { key: 'ArrowRight' })
        fireEvent.click(await screen.findByRole('menuitem', { name: 'editorArea.splitBottom' }))

        expect(calls).toEqual([{ name: 'split', edge: 'bottom' }])
    })

    test('Welcome 열기는 탭이 없을 때도 동작한다', async () => {
        const { calls } = await renderMenu({ hasTabs: false, hasActiveTab: false, hasClosedTabs: false })

        fireEvent.click(screen.getByRole('menuitem', { name: 'tab.openWelcome' }))

        expect(calls.map((call) => call.name)).toEqual(['openWelcome'])
    })
})
