import { describe, expect, test } from 'bun:test'
import type { Project } from '@shared/api/bindings'
import { TooltipProvider } from '@shared/ui/tooltip'
import { SidebarAddProjectMenu } from '@features/project/sidebar-add-project-menu'
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The sidebar's `+` menu (d-58 contract §1.D). It is pure UI, so what these tests pin is the part
 * that is easy to get wrong without a renderer: that a `rootMissing` entry cannot be activated (the
 * folder is gone — opening it would only produce an error toast), and that the short display label
 * wins over the project name exactly as the sidebar icon and the native `File > Open Recent` menu
 * resolve it, so the three surfaces name the same project the same way.
 */
const buildProject = (overrides: Partial<Project> = {}): Project => ({ id: 'p-1', root: '/repo/alpha', name: 'alpha', ...overrides })

const renderMenu = async (recentProjects: Project[] = []) => {
    const calls: string[] = []
    const selected: Project[] = []
    const rendered = renderWithProviders(
        <TooltipProvider>
            <SidebarAddProjectMenu
                recentProjects={recentProjects}
                onOpenByPath={() => calls.push('openByPath')}
                onOpenViaFinder={() => calls.push('openViaFinder')}
                onSelectRecent={(project) => selected.push(project)}
            />
        </TooltipProvider>,
    )

    fireEvent.keyDown(screen.getByRole('button', { name: 'sidebar.addProjectMenu' }), { key: 'Enter' })
    await screen.findByRole('menuitem', { name: 'sidebar.openByPath' })

    return { ...rendered, calls, selected }
}

describe('SidebarAddProjectMenu 열기 항목', () => {
    test('경로로 열기 항목이 onOpenByPath 를 부른다', async () => {
        const { calls } = await renderMenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'sidebar.openByPath' }))

        expect(calls).toEqual(['openByPath'])
    })

    test('Finder 로 열기 항목이 onOpenViaFinder 를 부른다', async () => {
        const { calls } = await renderMenu()

        fireEvent.click(screen.getByRole('menuitem', { name: 'sidebar.openViaFinder' }))

        expect(calls).toEqual(['openViaFinder'])
    })

    test('최근 프로젝트가 없으면 최근 섹션 자체가 없다 — 빈 라벨만 남지 않는다', async () => {
        await renderMenu()

        expect(screen.queryByText('app.recentItems')).toBeNull()
        expect(screen.getAllByRole('menuitem')).toHaveLength(2)
    })
})

describe('SidebarAddProjectMenu 최근 프로젝트', () => {
    test('표시 라벨이 있으면 이름 대신 라벨을 그리고, 없으면 이름을 그린다', async () => {
        await renderMenu([
            buildProject({ id: 'p-1', name: 'alpha', root: '/repo/alpha', display: { label: 'AL' } }),
            buildProject({ id: 'p-2', name: 'beta', root: '/repo/beta' }),
        ])

        expect(screen.getByText('AL')).toBeTruthy()
        expect(screen.queryByText('alpha')).toBeNull()
        expect(screen.getByText('beta')).toBeTruthy()
    })

    test('항목을 고르면 그 프로젝트 전체가 onSelectRecent 로 전달된다 (root 로 열어야 하므로)', async () => {
        const project = buildProject({ id: 'p-2', name: 'beta', root: '/repo/beta' })
        const { selected } = await renderMenu([project])

        fireEvent.click(screen.getByRole('menuitem', { name: /\/repo\/beta/ }))

        expect(selected).toEqual([project])
    })

    test('rootMissing 항목은 비활성이라 눌러도 콜백이 가지 않는다', async () => {
        const { selected } = await renderMenu([buildProject({ id: 'p-3', name: 'gone', root: '/repo/gone', rootMissing: true })])
        const item = screen.getByRole('menuitem', { name: /\/repo\/gone/ })

        expect(item.getAttribute('data-disabled')).not.toBeNull()
        expect(screen.getByText('app.recentProjectRootMissing')).toBeTruthy()

        fireEvent.click(item)

        expect(selected).toEqual([])
    })
})
