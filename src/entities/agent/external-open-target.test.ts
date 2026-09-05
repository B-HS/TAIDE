import { describe, expect, test } from 'bun:test'
import type { Project } from '@shared/api/bindings'
import { resolveExternalOpenTarget } from '@entities/agent/external-open-target'

const project = (id: string, root: string): Project => ({ id, root, name: id })

const TMP_PROMPT_PATH = '/var/folders/zz/T/claude-prompt-1234.md'

describe('resolveExternalOpenTarget', () => {
    test('루트에 속한 파일은 그 프로젝트로 열고 루트 밖 표시가 없다', () => {
        const projects = [project('a', '/Users/me/a'), project('b', '/Users/me/b')]
        expect(resolveExternalOpenTarget({ path: '/Users/me/b/src/main.rs', projects, activeProjectId: 'a' })).toEqual({
            projectId: 'b',
            isOutsideProjectRoot: false,
        })
    })

    test('중첩된 루트는 더 구체적인(긴) 루트의 프로젝트가 이긴다', () => {
        const projects = [project('outer', '/Users/me/ws'), project('inner', '/Users/me/ws/pkg')]
        expect(resolveExternalOpenTarget({ path: '/Users/me/ws/pkg/index.ts', projects, activeProjectId: 'outer' })?.projectId).toBe('inner')
    })

    test('tmpdir 의 Claude Code 임시파일은 활성 프로젝트의 루트 밖 탭으로 연다', () => {
        const projects = [project('a', '/Users/me/a'), project('b', '/Users/me/b')]
        expect(resolveExternalOpenTarget({ path: TMP_PROMPT_PATH, projects, activeProjectId: 'b' })).toEqual({
            projectId: 'b',
            isOutsideProjectRoot: true,
        })
    })

    test('활성 프로젝트가 목록에 없으면 첫 프로젝트로 폴백한다', () => {
        const projects = [project('a', '/Users/me/a')]
        expect(resolveExternalOpenTarget({ path: TMP_PROMPT_PATH, projects, activeProjectId: 'stale' })?.projectId).toBe('a')
        expect(resolveExternalOpenTarget({ path: TMP_PROMPT_PATH, projects, activeProjectId: null })?.projectId).toBe('a')
    })

    test('열린 프로젝트가 하나도 없을 때만 null 이다', () => {
        expect(resolveExternalOpenTarget({ path: TMP_PROMPT_PATH, projects: [], activeProjectId: null })).toBeNull()
    })

    test('루트 접두어만 같은 형제 경로는 루트 밖으로 본다', () => {
        const projects = [project('a', '/Users/me/a')]
        expect(resolveExternalOpenTarget({ path: '/Users/me/a-other/file.ts', projects, activeProjectId: 'a' })?.isOutsideProjectRoot).toBe(true)
    })
})
