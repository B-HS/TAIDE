import { describe, expect, test } from 'bun:test'
import type { StatusRow } from '@shared/api/bindings'
import type { GitSectionId } from '@entities/git/git-section-collapse-memory'
import { GIT_SECTION_DEFAULT_COLLAPSED } from '@entities/git/git-section-collapse-memory'
import { buildGitSections } from '@widgets/git-panel/git-sections'

const rowOf = (path: string, overrides: Partial<StatusRow> = {}): StatusRow => ({
    path,
    absPath: `/repo/${path}`,
    origPath: null,
    origAbsPath: null,
    staged: null,
    unstaged: null,
    isConflicted: false,
    ...overrides,
})

const buildWithDefaults = (input: {
    rows?: StatusRow[]
    stashCount?: number
    graphCount?: number
    collapsed?: Partial<Record<GitSectionId, boolean>>
}) =>
    buildGitSections({
        rows: input.rows ?? [],
        stashCount: input.stashCount ?? 0,
        graphCount: input.graphCount ?? 0,
        collapsed: { ...GIT_SECTION_DEFAULT_COLLAPSED, ...input.collapsed },
    })

describe('buildGitSections', () => {
    test('행을 충돌·스테이지·미스테이지 그룹으로 나눈다', () => {
        const result = buildWithDefaults({
            rows: [
                rowOf('conflict.ts', { isConflicted: true }),
                rowOf('staged.ts', { staged: 'modified' }),
                rowOf('unstaged.ts', { unstaged: 'modified' }),
            ],
        })

        expect(result.mergeRows.map((row) => row.path)).toEqual(['conflict.ts'])
        expect(result.stagedRows.map((row) => row.path)).toEqual(['staged.ts'])
        expect(result.unstagedRows.map((row) => row.path)).toEqual(['unstaged.ts'])
    })

    test('한 파일이 스테이지·미스테이지를 동시에 가지면 두 그룹에 모두 들어간다', () => {
        const result = buildWithDefaults({ rows: [rowOf('both.ts', { staged: 'modified', unstaged: 'modified' })] })

        expect(result.sections.staged.count).toBe(1)
        expect(result.sections.changes.count).toBe(1)
    })

    test('충돌 행은 스테이지·미스테이지 그룹에 세지 않는다', () => {
        const result = buildWithDefaults({ rows: [rowOf('conflict.ts', { isConflicted: true, staged: 'modified', unstaged: 'modified' })] })

        expect(result.sections.merge.count).toBe(1)
        expect(result.sections.staged.visible).toBe(false)
        expect(result.sections.changes.visible).toBe(false)
    })

    test('비어 있는 섹션은 보이지 않는다', () => {
        const result = buildWithDefaults({ rows: [rowOf('unstaged.ts', { unstaged: 'modified' })] })

        expect(result.sections.merge.visible).toBe(false)
        expect(result.sections.staged.visible).toBe(false)
        expect(result.sections.changes.visible).toBe(true)
    })

    test('스태시가 0건이면 스태시 섹션을 그리지 않는다 — 변경이 있어도 마찬가지', () => {
        const result = buildWithDefaults({ rows: [rowOf('unstaged.ts', { unstaged: 'modified' })], stashCount: 0 })

        expect(result.sections.stashes.visible).toBe(false)
    })

    test('스태시가 있으면 변경이 없어도 스태시 섹션을 그린다', () => {
        const result = buildWithDefaults({ stashCount: 2 })

        expect(result.sections.stashes.visible).toBe(true)
        expect(result.sections.stashes.count).toBe(2)
    })

    test('커밋이 없으면 그래프 섹션을 그리지 않는다', () => {
        expect(buildWithDefaults({}).sections.graph.visible).toBe(false)
        expect(buildWithDefaults({ graphCount: 3 }).sections.graph.visible).toBe(true)
    })

    test('접힌 섹션도 개수를 계속 보고한다', () => {
        const result = buildWithDefaults({ rows: [rowOf('staged.ts', { staged: 'modified' })], collapsed: { staged: true } })

        expect(result.sections.staged.collapsed).toBe(true)
        expect(result.sections.staged.visible).toBe(true)
        expect(result.sections.staged.count).toBe(1)
    })

    test('기본 접힘 상태는 Stashes 에만 적용된다', () => {
        const result = buildWithDefaults({ rows: [rowOf('unstaged.ts', { unstaged: 'modified' })], stashCount: 1, graphCount: 1 })

        expect(result.sections.stashes.collapsed).toBe(true)
        expect(result.sections.changes.collapsed).toBe(false)
        expect(result.sections.graph.collapsed).toBe(false)
    })

    test('변경 그룹 3종이 모두 비면 변경사항 없음을 표시한다', () => {
        expect(buildWithDefaults({}).showNoChanges).toBe(true)
        expect(buildWithDefaults({ stashCount: 1, graphCount: 1 }).showNoChanges).toBe(true)
        expect(buildWithDefaults({ rows: [rowOf('unstaged.ts', { unstaged: 'modified' })] }).showNoChanges).toBe(false)
        expect(buildWithDefaults({ rows: [rowOf('conflict.ts', { isConflicted: true })] }).showNoChanges).toBe(false)
    })

    test('그룹에 속하지 않는 행이 있어도 변경사항 없음으로 보고한다', () => {
        expect(buildWithDefaults({ rows: [rowOf('ghost.ts')] }).showNoChanges).toBe(true)
    })

    test('변경·스태시·커밋이 전부 없으면 다섯 섹션 모두 보이지 않고 변경사항 없음만 남는다', () => {
        const result = buildWithDefaults({})

        expect(Object.values(result.sections).every((section) => !section.visible)).toBe(true)
        expect(Object.values(result.sections).every((section) => section.count === 0)).toBe(true)
        expect(result.showNoChanges).toBe(true)
    })

    test('섹션 키 순서는 렌더 순서(Merge → Staged → Changes → Stashes → Graph)다', () => {
        expect(Object.keys(buildWithDefaults({}).sections)).toEqual(['merge', 'staged', 'changes', 'stashes', 'graph'])
    })

    test('같은 그룹의 여러 행은 입력 순서를 유지하고 개수로 집계된다', () => {
        const result = buildWithDefaults({
            rows: [rowOf('b.ts', { unstaged: 'modified' }), rowOf('a.ts', { unstaged: 'added' }), rowOf('c.ts', { staged: 'modified' })],
        })

        expect(result.unstagedRows.map((row) => row.path)).toEqual(['b.ts', 'a.ts'])
        expect(result.sections.changes.count).toBe(2)
        expect(result.sections.staged.count).toBe(1)
    })

    test('접힘 상태는 다섯 섹션 각각 입력값을 그대로 반영한다', () => {
        const collapsed = { merge: true, staged: true, changes: true, stashes: false, graph: true }
        const result = buildWithDefaults({ rows: [rowOf('conflict.ts', { isConflicted: true })], stashCount: 1, graphCount: 1, collapsed })

        expect(Object.fromEntries(Object.entries(result.sections).map(([id, section]) => [id, section.collapsed]))).toEqual(collapsed)
    })

    test('접힘은 가시성에 영향을 주지 않는다 — 0건 섹션은 펼쳐 있어도 숨김', () => {
        const result = buildWithDefaults({ collapsed: { stashes: false, graph: false } })

        expect(result.sections.stashes.visible).toBe(false)
        expect(result.sections.graph.visible).toBe(false)
    })
})
