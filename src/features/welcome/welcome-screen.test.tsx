import { describe, expect, test } from 'bun:test'
import type { ComponentProps } from 'react'
import { WelcomeScreen } from '@features/welcome/welcome-screen'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The "open a terminal" action added in d-58 §1.C. A terminal belongs to a project (Rust
 * `ensure_project_open`), so the button carries the same gate as "open file" and the hint line has
 * to appear whenever *either* action is unavailable — a disabled button with no explanation is the
 * regression this locks. Labels are asserted as locale keys because the test i18n instance carries
 * no bundles (`docs/memory/test-conventions.md` §2).
 */
const renderScreen = (props: Partial<ComponentProps<typeof WelcomeScreen>> = {}) => {
    const openedTerminals: number[] = []
    const rendered = renderWithProviders(
        <WelcomeScreen
            recentProjects={[]}
            recentProjectsUnavailable={false}
            shortcuts={[]}
            onOpenFolder={() => {}}
            canOpenFile={true}
            onOpenFile={() => {}}
            canOpenTerminal={true}
            onOpenTerminal={() => openedTerminals.push(1)}
            onSelectRecent={() => {}}
            {...props}
        />,
    )

    return { ...rendered, openedTerminals, terminalButton: screen.getByRole('button', { name: 'keymap.newTerminal' }) }
}

describe('WelcomeScreen 터미널 열기', () => {
    test('파일 열기 옆에 터미널 열기 버튼을 그린다', () => {
        const { terminalButton } = renderScreen()

        expect(terminalButton.hasAttribute('disabled')).toBe(false)
        expect(screen.getByRole('button', { name: 'app.openFile' })).toBeDefined()
    })

    test('클릭하면 onOpenTerminal 을 부른다', () => {
        const { terminalButton, openedTerminals } = renderScreen()

        fireEvent.click(terminalButton)

        expect(openedTerminals.length).toBe(1)
    })

    test('열린 프로젝트가 없으면 비활성이고 클릭해도 콜백이 불리지 않는다', () => {
        const { terminalButton, openedTerminals } = renderScreen({ canOpenTerminal: false })

        fireEvent.click(terminalButton)

        expect(terminalButton.hasAttribute('disabled')).toBe(true)
        expect(openedTerminals.length).toBe(0)
    })

    test('파일·터미널 중 하나라도 열 수 없으면 폴더를 먼저 열라는 안내를 보여준다', () => {
        renderScreen({ canOpenFile: false, canOpenTerminal: false })

        expect(screen.getByText('app.openFileHint')).toBeDefined()
    })

    test('둘 다 열 수 있으면 안내 문구는 나오지 않는다', () => {
        renderScreen()

        expect(screen.queryByText('app.openFileHint')).toBeNull()
    })
})
