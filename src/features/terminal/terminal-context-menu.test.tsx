import { describe, expect, test } from 'bun:test'
import type { SplitEdge } from '@features/tab/tab-context-menu'
import { TerminalContextMenu } from '@features/terminal/terminal-context-menu'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The terminal's right-click menu (batch 4 contract §F.2-6). The behaviour that separates it from
 * every other `*-context-menu.tsx` is the split submenu: a direction that does not fit the pane is
 * shown **disabled, never hidden** — a vanished direction reads as "this app cannot split
 * downwards", a greyed one reads as "not at this size". So each split test asserts both halves:
 * the entry is present *and* it is disabled.
 *
 * i18n has no bundles loaded under the harness, so every label assertion is the translation *key*
 * (`docs/memory/test-conventions.md` §2).
 */
const ALL_EDGES_AVAILABLE: Record<SplitEdge, boolean> = { top: true, bottom: true, left: true, right: true }

type MenuCalls = { name: string; edge?: SplitEdge }[]

const renderMenu = ({
    canCopy = true,
    canPaste = true,
    splitAvailability = ALL_EDGES_AVAILABLE,
}: {
    canCopy?: boolean
    canPaste?: boolean
    splitAvailability?: Record<SplitEdge, boolean>
} = {}) => {
    const calls: MenuCalls = []
    const record = (name: string) => () => calls.push({ name })
    const rendered = renderWithProviders(
        <TerminalContextMenu
            canCopy={canCopy}
            canPaste={canPaste}
            splitAvailability={splitAvailability}
            onOpenChange={(open) => calls.push({ name: open ? 'open' : 'close' })}
            onRestoreFocus={record('restoreFocus')}
            onCopy={record('copy')}
            onPaste={record('paste')}
            onSelectAll={record('selectAll')}
            onClear={record('clear')}
            onSplit={(edge) => calls.push({ name: 'split', edge })}
            onNewTerminal={record('newTerminal')}
            onKill={record('kill')}>
            <div data-terminal-surface>terminal</div>
        </TerminalContextMenu>,
    )
    const surface = rendered.container.querySelector('[data-terminal-surface]')
    if (!surface) throw new Error('terminal surface not rendered')
    fireEvent.contextMenu(surface)

    return { ...rendered, calls }
}

const openSplitSubmenu = async () => {
    fireEvent.keyDown(await screen.findByRole('menuitem', { name: 'tab.split' }), { key: 'ArrowRight' })
    return screen.findByRole('menuitem', { name: 'editorArea.splitTop' })
}

const isDisabled = (name: string) => screen.getByRole('menuitem', { name }).getAttribute('aria-disabled') === 'true'

describe('TerminalContextMenu', () => {
    test('우클릭하면 메뉴가 열리고 onOpenChange 로 알린다', async () => {
        const { calls } = renderMenu()

        await screen.findByRole('menuitem', { name: 'terminal.copy' })

        expect(calls).toEqual([{ name: 'open' }])
    })

    test('선택 영역이 없으면 복사만 비활성이고 나머지는 그대로 쓸 수 있다', async () => {
        renderMenu({ canCopy: false })

        await screen.findByRole('menuitem', { name: 'terminal.copy' })

        expect(isDisabled('terminal.copy')).toBe(true)
        expect(isDisabled('terminal.paste')).toBe(false)
        expect(isDisabled('terminal.selectAll')).toBe(false)
        expect(isDisabled('terminal.clear')).toBe(false)
        expect(isDisabled('terminal.kill')).toBe(false)
    })

    test('클립보드를 읽을 수 없으면 붙여넣기만 비활성이다 (원격 미러의 비보안 컨텍스트)', async () => {
        renderMenu({ canPaste: false })

        await screen.findByRole('menuitem', { name: 'terminal.paste' })

        expect(isDisabled('terminal.paste')).toBe(true)
        expect(isDisabled('terminal.copy')).toBe(false)
    })

    test('비활성 항목을 눌러도 핸들러가 실행되지 않는다', async () => {
        const { calls } = renderMenu({ canCopy: false })

        fireEvent.click(await screen.findByRole('menuitem', { name: 'terminal.copy' }))

        expect(calls.filter((call) => call.name === 'copy')).toEqual([])
    })

    test('활성 항목을 고르면 그 핸들러를 부른다', async () => {
        const { calls } = renderMenu()

        fireEvent.click(await screen.findByRole('menuitem', { name: 'terminal.selectAll' }))

        expect(calls.some((call) => call.name === 'selectAll')).toBe(true)
    })

    test('분할 4방향은 언제나 모두 표시된다', async () => {
        renderMenu({ splitAvailability: { top: false, bottom: false, left: false, right: false } })
        await openSplitSubmenu()

        expect(screen.getByRole('menuitem', { name: 'editorArea.splitTop' })).toBeDefined()
        expect(screen.getByRole('menuitem', { name: 'editorArea.splitBottom' })).toBeDefined()
        expect(screen.getByRole('menuitem', { name: 'editorArea.splitLeft' })).toBeDefined()
        expect(screen.getByRole('menuitem', { name: 'editorArea.splitRight' })).toBeDefined()
    })

    test('공간이 없는 방향만 비활성으로 표시한다 (숨기지 않는다)', async () => {
        renderMenu({ splitAvailability: { top: true, bottom: false, left: false, right: true } })
        await openSplitSubmenu()

        expect(isDisabled('editorArea.splitTop')).toBe(false)
        expect(isDisabled('editorArea.splitRight')).toBe(false)
        expect(isDisabled('editorArea.splitBottom')).toBe(true)
        expect(isDisabled('editorArea.splitLeft')).toBe(true)
    })

    test('가능한 방향을 고르면 그 방향을 onSplit 에 넘긴다', async () => {
        const { calls } = renderMenu()
        await openSplitSubmenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'editorArea.splitRight' }))

        expect(calls.filter((call) => call.name === 'split')).toEqual([{ name: 'split', edge: 'right' }])
    })

    test('불가능한 방향을 눌러도 분할을 요청하지 않는다', async () => {
        const { calls } = renderMenu({ splitAvailability: { ...ALL_EDGES_AVAILABLE, bottom: false } })
        await openSplitSubmenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'editorArea.splitBottom' }))

        expect(calls.filter((call) => call.name === 'split')).toEqual([])
    })

    test('새 터미널·종료는 분할 가능 여부와 무관하게 동작한다', async () => {
        const { calls } = renderMenu({ splitAvailability: { top: false, bottom: false, left: false, right: false } })

        fireEvent.click(await screen.findByRole('menuitem', { name: 'tab.newTerminal' }))

        expect(calls.some((call) => call.name === 'newTerminal')).toBe(true)
    })
})
