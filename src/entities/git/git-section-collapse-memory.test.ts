import { beforeEach, describe, expect, test } from 'bun:test'
import {
    GIT_SECTION_DEFAULT_COLLAPSED,
    readGitSectionCollapseState,
    resetGitSectionCollapseMemoryForTests,
    writeGitSectionCollapsed,
} from '@entities/git/git-section-collapse-memory'

describe('git-section-collapse-memory', () => {
    beforeEach(resetGitSectionCollapseMemoryForTests)

    test('기본값은 Stashes 만 접힘이다', () => {
        expect(readGitSectionCollapseState()).toEqual({ merge: false, staged: false, changes: false, stashes: true, graph: false })
    })

    test('기본값 상수와 초기 스냅샷이 일치한다', () => {
        expect(readGitSectionCollapseState()).toEqual(GIT_SECTION_DEFAULT_COLLAPSED)
    })

    test('접은 섹션만 바뀌고 나머지는 기본값을 유지한다', () => {
        writeGitSectionCollapsed('staged', true)

        expect(readGitSectionCollapseState().staged).toBe(true)
        expect(readGitSectionCollapseState().changes).toBe(false)
        expect(readGitSectionCollapseState().stashes).toBe(true)
    })

    test('기본이 접힘인 섹션도 펼침으로 덮어쓸 수 있다', () => {
        writeGitSectionCollapsed('stashes', false)

        expect(readGitSectionCollapseState().stashes).toBe(false)
    })

    test('패널이 언마운트됐다 다시 마운트돼도 접힘 상태가 남는다', () => {
        writeGitSectionCollapsed('graph', true)
        const remounted = readGitSectionCollapseState()

        expect(remounted.graph).toBe(true)
    })

    test('같은 섹션을 다시 쓰면 마지막 값이 이긴다', () => {
        writeGitSectionCollapsed('changes', true)
        writeGitSectionCollapsed('changes', false)

        expect(readGitSectionCollapseState().changes).toBe(false)
    })

    test('스냅샷은 복사본이라 반환 객체를 바꿔도 메모리에 반영되지 않는다', () => {
        const snapshot = readGitSectionCollapseState()
        snapshot.merge = true

        expect(readGitSectionCollapseState().merge).toBe(false)
    })

    test('스냅샷은 호출마다 새 객체다', () => {
        expect(readGitSectionCollapseState()).not.toBe(readGitSectionCollapseState())
    })

    test('초기화하면 기록한 값이 전부 사라지고 기본값으로 돌아간다', () => {
        writeGitSectionCollapsed('merge', true)
        writeGitSectionCollapsed('stashes', false)
        resetGitSectionCollapseMemoryForTests()

        expect(readGitSectionCollapseState()).toEqual(GIT_SECTION_DEFAULT_COLLAPSED)
    })
})
