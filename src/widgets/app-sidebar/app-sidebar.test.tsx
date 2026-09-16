import { describe, expect, test } from 'bun:test'
import { DndContext } from '@dnd-kit/core'
import type { ProjectGroup, ProjectRef } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { TooltipProvider } from '@shared/ui/tooltip'
import { createTestQueryClient, renderWithProviders, screen, within } from '@shared/testing/render'
import { AppSidebar } from '@widgets/app-sidebar/app-sidebar'

/**
 * How the rail lays a grouped sidebar out (contract §1.C): a section per group, its members drawn in
 * `session.projects` order rather than in the group's own `members` order, and everything no group
 * claims kept together at the bottom.
 *
 * Both reads are seeded and frozen the way `use-project-drag.test.tsx` does it — another file's
 * process-global `@entities/project/project.ipc` fake would otherwise win the mount refetch and
 * empty the rail. The remaining queries (agents, settings, recents) are left to fail; they degrade
 * to their defaults, which is exactly the state this file is about.
 */
const ALPHA: ProjectRef = { id: 'project-alpha', root: '/tmp/alpha', name: 'alpha' }

const BETA: ProjectRef = { id: 'project-beta', root: '/tmp/beta', name: 'beta' }

const GAMMA: ProjectRef = { id: 'project-gamma', root: '/tmp/gamma', name: 'gamma' }

const PROJECTS = [ALPHA, BETA, GAMMA]

const WORK: ProjectGroup = { id: 'group-work', name: 'Work', members: [GAMMA.id, ALPHA.id] }

const UNGROUPED_LABEL = 'projectGroup.ungrouped'

const renderSidebar = (groups: ProjectGroup[]) => {
    const queryClient = createTestQueryClient()
    queryClient.setQueryDefaults(QUERY_KEY.PROJECT.LIST, { staleTime: Infinity, gcTime: Infinity })
    queryClient.setQueryDefaults(QUERY_KEY.PROJECT_GROUP.LIST, { staleTime: Infinity, gcTime: Infinity })
    queryClient.setQueryData(QUERY_KEY.PROJECT.LIST, PROJECTS)
    queryClient.setQueryData(QUERY_KEY.PROJECT_GROUP.LIST, groups)

    return renderWithProviders(
        <TooltipProvider>
            <DndContext>
                <AppSidebar activeProjectId={ALPHA.id} draggingProjectId={null} onOpenSettings={() => {}} />
            </DndContext>
        </TooltipProvider>,
        { queryClient },
    )
}

/**
 * The rail's buttons carry their name as an `aria-label` (project icons) or as text (group headers),
 * so one reader covers both and a header's presence is asserted by the same list as its members. The
 * nameless entries dropped at the end are dnd-kit's drag-handle wrappers, which every `useSortable`
 * spreads a bare `role="button"` onto — they have no name precisely because the real control is the
 * element inside them.
 */
const namesIn = (container: HTMLElement) =>
    within(container)
        .getAllByRole('button')
        .map((button) => button.getAttribute('aria-label') ?? button.textContent ?? '')
        .filter((name) => name !== '')

const sectionOf = (name: string) => screen.getByRole('group', { name })

describe('AppSidebar 그룹 배치', () => {
    test('그룹 섹션은 헤더 아래에 멤버를 세션 순서로 그린다 — group.members 순서가 아니다', () => {
        renderSidebar([WORK])

        expect(namesIn(sectionOf(WORK.name))).toEqual([WORK.name, ALPHA.name, GAMMA.name])
    })

    test('그룹이 주장하지 않은 프로젝트는 하단 미분류 영역에 남는다', () => {
        renderSidebar([WORK])

        expect(namesIn(sectionOf(UNGROUPED_LABEL))).toEqual([BETA.name])
    })

    test('접힌 그룹은 헤더만 남기고 멤버 아이콘을 그리지 않는다', () => {
        renderSidebar([{ ...WORK, collapsed: true }])

        expect(namesIn(sectionOf(WORK.name))).toEqual([WORK.name])
        expect(screen.queryByRole('button', { name: ALPHA.name })).toBeNull()
    })

    test('그룹이 없으면 레일 전체가 미분류다 — 그룹 도입 전과 같은 한 줄', () => {
        renderSidebar([])

        expect(screen.queryByRole('group', { name: WORK.name })).toBeNull()
        expect(namesIn(sectionOf(UNGROUPED_LABEL))).toEqual([ALPHA.name, BETA.name, GAMMA.name])
    })
})
