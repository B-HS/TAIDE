import { describe, expect, test } from 'bun:test'
import type { ProjectGroup, ProjectRef } from '@shared/api/bindings'
import {
    clampProjectGroupName,
    normalizeProjectGroupName,
    resolveProjectGroupColorVar,
    resolveProjectGroupSections,
    withProjectGroupMember,
    withoutProjectGroupMember,
} from '@shared/lib/project-group'

/**
 * How the rail turns one flat `session.projects` plus a membership set into sections — the pure half
 * of `app-sidebar.tsx`. Order is the whole point of these cases: the open list owns it, a group's
 * `members` array does not.
 */
const project = (id: string): ProjectRef => ({ id, root: `/tmp/${id}`, name: id })

const ALPHA = project('project-alpha')

const BETA = project('project-beta')

const GAMMA = project('project-gamma')

const PROJECTS = [ALPHA, BETA, GAMMA]

const WORK: ProjectGroup = { id: 'group-work', name: 'Work', members: [GAMMA.id, ALPHA.id] }

const SIDE: ProjectGroup = { id: 'group-side', name: 'Side', members: [] }

const NULL_CHARACTER = String.fromCharCode(0)

describe('resolveProjectGroupSections', () => {
    test('멤버는 그룹의 members 순서가 아니라 열린 프로젝트 순서로 배열된다', () => {
        const { sections } = resolveProjectGroupSections(PROJECTS, [WORK])

        expect(sections.map(({ group }) => group.id)).toEqual([WORK.id])
        expect(sections[0].members.map((member) => member.id)).toEqual([ALPHA.id, GAMMA.id])
    })

    test('그룹이 주장하지 않은 프로젝트는 미분류로 남는다', () => {
        const { ungrouped } = resolveProjectGroupSections(PROJECTS, [WORK])

        expect(ungrouped.map((item) => item.id)).toEqual([BETA.id])
    })

    test('members 가 비었거나 아예 없는 그룹은 빈 섹션이다', () => {
        const { sections, ungrouped } = resolveProjectGroupSections(PROJECTS, [SIDE, { id: 'group-empty', name: 'Empty' }])

        expect(sections.map(({ members }) => members.length)).toEqual([0, 0])
        expect(ungrouped.map((item) => item.id)).toEqual([ALPHA.id, BETA.id, GAMMA.id])
    })

    test('열려 있지 않은 멤버는 레일에 그려지지 않는다 — 멤버십은 "열 수 있는 것" 이다', () => {
        const { sections } = resolveProjectGroupSections([ALPHA], [{ ...WORK, members: [ALPHA.id, 'project-closed'] }])

        expect(sections[0].members.map((member) => member.id)).toEqual([ALPHA.id])
    })

    test('그룹이 하나도 없으면 전부 미분류다 — 그룹 도입 전 레일과 같은 배열', () => {
        const { sections, ungrouped } = resolveProjectGroupSections(PROJECTS, [])

        expect(sections).toEqual([])
        expect(ungrouped).toEqual(PROJECTS)
    })

    test('같은 프로젝트를 두 그룹이 주장하면 마지막 그룹에만 들어간다 — 섹션은 언제나 분할이다', () => {
        const { sections, groupIdByProjectId } = resolveProjectGroupSections(PROJECTS, [WORK, { ...SIDE, members: [ALPHA.id] }])

        expect(sections[0].members.map((member) => member.id)).toEqual([GAMMA.id])
        expect(sections[1].members.map((member) => member.id)).toEqual([ALPHA.id])
        expect(groupIdByProjectId.get(ALPHA.id)).toBe(SIDE.id)
    })
})

describe('그룹 멤버 편집', () => {
    test('추가는 뒤에 붙이고 이미 있으면 원본을 그대로 돌려준다', () => {
        expect(withProjectGroupMember([ALPHA.id], BETA.id)).toEqual([ALPHA.id, BETA.id])
        expect(withProjectGroupMember(undefined, BETA.id)).toEqual([BETA.id])
        expect(withProjectGroupMember([ALPHA.id], ALPHA.id)).toEqual([ALPHA.id])
    })

    test('제거는 그 id 만 빼고 원본 배열은 바꾸지 않는다', () => {
        const members = [ALPHA.id, BETA.id]

        expect(withoutProjectGroupMember(members, ALPHA.id)).toEqual([BETA.id])
        expect(withoutProjectGroupMember(undefined, ALPHA.id)).toEqual([])
        expect(members).toEqual([ALPHA.id, BETA.id])
    })
})

describe('그룹 이름·색', () => {
    test('입력 중에는 제어문자만 빼고 40 코드포인트로 자르되 공백은 남긴다', () => {
        expect(clampProjectGroupName(`a${NULL_CHARACTER} b`)).toBe('a b')
        expect([...clampProjectGroupName('가'.repeat(50))].length).toBe(40)
    })

    test('제출 직전에는 앞뒤 공백까지 지운다 — Rust sanitize_group_name 과 같은 순서', () => {
        expect(normalizeProjectGroupName('  Work  ')).toBe('Work')
        expect(normalizeProjectGroupName('   ')).toBe('')
    })

    test('색은 ProjectDisplay 팔레트 토큰일 때만 CSS 변수가 된다', () => {
        expect(resolveProjectGroupColorVar('lane3')).toBe('var(--taide-graph-lane3)')
        expect(resolveProjectGroupColorVar('lane99')).toBeNull()
        expect(resolveProjectGroupColorVar(null)).toBeNull()
    })
})
