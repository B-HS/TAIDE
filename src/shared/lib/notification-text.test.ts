import { describe, expect, test } from 'bun:test'
import type { ProjectLayout, ProjectRef, Tab } from '@shared/api/bindings'
import {
    findTerminalSessionOwner,
    findTerminalTabTitle,
    joinNotificationBody,
    notificationBodyFragment,
    resolveProjectNotificationTitle,
} from '@shared/lib/notification-text'

const buildTerminalTab = (id: string, sessionId: string, title: string): Tab => ({ id, kind: { kind: 'terminal', sessionId }, title })

const buildLayout = (tabs: Tab[], auxiliaryTabs: Tab[] = []): ProjectLayout => ({
    version: 1,
    root: { node: 'leaf', id: 'leaf', tabs, active: tabs[0]?.id ?? null },
    focusedPane: 'leaf',
    auxiliaryWindows:
        auxiliaryTabs.length > 0
            ? [
                  {
                      slot: 1,
                      root: { node: 'leaf', id: 'aux-leaf', tabs: auxiliaryTabs, active: auxiliaryTabs[0]?.id ?? null },
                      focusedPane: 'aux-leaf',
                  },
              ]
            : [],
})

const buildProject = (id: string, name: string, label?: string): ProjectRef => ({
    id,
    root: `/tmp/${id}`,
    name,
    display: label === undefined ? undefined : { icon: null, label, color: null },
})

describe('joinNotificationBody', () => {
    test('있는 조각만 가운뎃점으로 잇는다', () => {
        expect(joinNotificationBody(['Push complete', 'main'])).toBe('Push complete · main')
    })

    test('없는 조각은 구분자를 남기지 않고 빠진다', () => {
        expect(joinNotificationBody(['Push complete', undefined, null, '   '])).toBe('Push complete')
    })

    test('전부 비면 빈 문자열이다', () => {
        expect(joinNotificationBody([null, undefined, ''])).toBe('')
    })
})

describe('notificationBodyFragment', () => {
    test('값이 있으면 앞에 구분자를 붙인 꼬리를 만든다', () => {
        expect(notificationBodyFragment('bash')).toBe(' · bash')
    })

    test('값이 없으면 빈 문자열이라 템플릿에 꼬리가 남지 않는다', () => {
        expect(notificationBodyFragment(null)).toBe('')
        expect(notificationBodyFragment('  ')).toBe('')
    })
})

describe('resolveProjectNotificationTitle', () => {
    const projects = [buildProject('p1', 'taide'), buildProject('p2', 'oiia', 'OI')]

    test('표시 라벨이 있으면 라벨을 제목으로 쓴다', () => {
        expect(resolveProjectNotificationTitle({ projects, projectId: 'p2', fallbackTitle: 'Agent finished' })).toBe('OI')
    })

    test('표시 라벨이 없으면 프로젝트 이름을 쓴다', () => {
        expect(resolveProjectNotificationTitle({ projects, projectId: 'p1', fallbackTitle: 'Agent finished' })).toBe('taide')
    })

    test('캐시에 없는 프로젝트면 이벤트 제목으로 폴백한다', () => {
        expect(resolveProjectNotificationTitle({ projects, projectId: 'p3', fallbackTitle: 'Agent finished' })).toBe('Agent finished')
    })

    test('프로젝트를 특정할 수 없으면 이벤트 제목으로 폴백한다', () => {
        expect(resolveProjectNotificationTitle({ projects, projectId: null, fallbackTitle: 'Agent finished' })).toBe('Agent finished')
    })

    test('목록 자체가 아직 없으면 이벤트 제목으로 폴백한다', () => {
        expect(resolveProjectNotificationTitle({ projects: undefined, projectId: 'p1', fallbackTitle: 'Agent finished' })).toBe('Agent finished')
    })
})

describe('findTerminalTabTitle', () => {
    const layout = buildLayout([buildTerminalTab('t1', 's1', 'claude')], [buildTerminalTab('t2', 's2', 'build')])

    test('메인 트리의 터미널 탭 제목을 찾는다', () => {
        expect(findTerminalTabTitle(layout, 's1')).toBe('claude')
    })

    test('보조 창으로 옮긴 탭도 찾는다', () => {
        expect(findTerminalTabTitle(layout, 's2')).toBe('build')
    })

    test('해당 세션의 탭이 없으면 null 이다', () => {
        expect(findTerminalTabTitle(layout, 's3')).toBeNull()
    })

    test('레이아웃이 없거나 세션이 비면 null 이다', () => {
        expect(findTerminalTabTitle(undefined, 's1')).toBeNull()
        expect(findTerminalTabTitle(layout, '')).toBeNull()
    })
})

describe('findTerminalSessionOwner', () => {
    const entries = [
        { projectId: 'p1', layout: buildLayout([buildTerminalTab('t1', 's1', 'claude')]) },
        { projectId: 'p2', layout: buildLayout([buildTerminalTab('t2', 's2', 'build')]) },
    ]

    test('세션을 가진 프로젝트와 탭 제목을 함께 돌려준다', () => {
        expect(findTerminalSessionOwner(entries, 's2')).toEqual({ projectId: 'p2', tabTitle: 'build' })
    })

    test('어느 레이아웃도 갖고 있지 않으면 null 이다 (이미 닫힌 탭)', () => {
        expect(findTerminalSessionOwner(entries, 's9')).toBeNull()
    })

    test('레이아웃이 아직 캐시되지 않은 프로젝트는 건너뛴다', () => {
        expect(findTerminalSessionOwner([{ projectId: 'p1', layout: undefined }, ...entries], 's1')).toEqual({ projectId: 'p1', tabTitle: 'claude' })
    })
})
