import { describe, expect, test } from 'bun:test'
import { GIT_SECTION_DEFAULT_COLLAPSED, GIT_SECTION_IDS, toGitSectionCollapsedIds, toGitSectionCollapsedMap } from '@entities/git/git-section'

describe('toGitSectionCollapsedMap', () => {
    test('설정이 아직 없으면 기본값(Stashes 만 접힘)이다', () => {
        expect(toGitSectionCollapsedMap(undefined)).toEqual(GIT_SECTION_DEFAULT_COLLAPSED)
        expect(toGitSectionCollapsedMap(null)).toEqual(GIT_SECTION_DEFAULT_COLLAPSED)
    })

    test('빈 목록은 기본값이 아니라 "아무것도 접히지 않음"이다', () => {
        expect(toGitSectionCollapsedMap([])).toEqual({ merge: false, staged: false, changes: false, stashes: false, graph: false })
    })

    test('목록에 있는 섹션만 접힘으로 읽는다', () => {
        expect(toGitSectionCollapsedMap(['staged', 'graph'])).toEqual({
            merge: false,
            staged: true,
            changes: false,
            stashes: false,
            graph: true,
        })
    })

    test('이 빌드가 모르는 id 는 무시한다 — Rust 는 값 집합을 검증하지 않는다', () => {
        expect(toGitSectionCollapsedMap(['stashes', 'submodules'])).toEqual({
            merge: false,
            staged: false,
            changes: false,
            stashes: true,
            graph: false,
        })
    })

    test('다섯 섹션의 키를 항상 전부 채운다', () => {
        expect(Object.keys(toGitSectionCollapsedMap(['merge']))).toEqual([...GIT_SECTION_IDS])
    })
})

describe('toGitSectionCollapsedIds', () => {
    test('접힌 섹션만 렌더 순서대로 모은다', () => {
        expect(toGitSectionCollapsedIds({ merge: false, staged: true, changes: false, stashes: true, graph: false })).toEqual(['staged', 'stashes'])
    })

    test('입력 객체의 키 순서와 무관하게 GIT_SECTION_IDS 순서로 쓴다', () => {
        expect(toGitSectionCollapsedIds({ graph: true, stashes: true, changes: true, staged: true, merge: true })).toEqual([...GIT_SECTION_IDS])
    })

    test('접힌 섹션이 없으면 빈 목록이다', () => {
        expect(toGitSectionCollapsedIds({ merge: false, staged: false, changes: false, stashes: false, graph: false })).toEqual([])
    })

    test('맵으로 읽었다가 다시 목록으로 쓰면 같은 목록이 나온다', () => {
        expect(toGitSectionCollapsedIds(toGitSectionCollapsedMap(['changes', 'graph']))).toEqual(['changes', 'graph'])
    })
})
