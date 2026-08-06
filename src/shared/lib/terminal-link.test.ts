import { describe, expect, test } from 'bun:test'
import { findTerminalLinkMatches } from '@shared/lib/terminal-link'

describe('findTerminalLinkMatches', () => {
    test('경로만 있으면 line/column 없이 매칭한다', () => {
        const [match] = findTerminalLinkMatches('open file.ts now')
        expect(match?.path).toBe('file.ts')
        expect(match?.line).toBeUndefined()
        expect(match?.column).toBeUndefined()
    })

    test('path:line 형태를 1-based line 으로 매칭한다', () => {
        const [match] = findTerminalLinkMatches('at file.ts:12')
        expect(match).toMatchObject({ path: 'file.ts', line: 12, column: undefined })
    })

    test('path:line:col 형태를 1-based 좌표로 매칭한다', () => {
        const [match] = findTerminalLinkMatches('at file.ts:12:3')
        expect(match).toMatchObject({ path: 'file.ts', line: 12, column: 3 })
    })

    test('상대 경로(./)를 매칭한다', () => {
        const [match] = findTerminalLinkMatches('see ./rel/path.rs:5')
        expect(match).toMatchObject({ path: './rel/path.rs', line: 5, column: undefined })
    })

    test('상위 상대 경로(../)를 매칭한다', () => {
        const [match] = findTerminalLinkMatches('see ../rel/path.rs:5')
        expect(match).toMatchObject({ path: '../rel/path.rs', line: 5, column: undefined })
    })

    test('절대 경로를 매칭한다', () => {
        const [match] = findTerminalLinkMatches('see /abs/path.rs:5:1')
        expect(match).toMatchObject({ path: '/abs/path.rs', line: 5, column: 1 })
    })

    test('홈 디렉토리(~) 경로를 매칭한다', () => {
        const [match] = findTerminalLinkMatches('see ~/notes/todo.md:3')
        expect(match).toMatchObject({ path: '~/notes/todo.md', line: 3, column: undefined })
    })

    test('한 줄에서 여러 링크를 전부 매칭한다', () => {
        const matches = findTerminalLinkMatches('diff src/a.ts:10 vs src/b.ts:20:2')
        expect(matches).toHaveLength(2)
        expect(matches[0]).toMatchObject({ path: 'src/a.ts', line: 10 })
        expect(matches[1]).toMatchObject({ path: 'src/b.ts', line: 20, column: 2 })
    })

    test('확장자가 없는 토큰은 매칭하지 않는다', () => {
        expect(findTerminalLinkMatches('hello world 123')).toHaveLength(0)
    })

    test('빈 문자열은 매칭하지 않는다', () => {
        expect(findTerminalLinkMatches('')).toHaveLength(0)
    })

    test('startIndex/endIndex 가 실제 경로 텍스트 범위를 가리킨다', () => {
        const text = 'run file.ts:12:3 now'
        const [match] = findTerminalLinkMatches(text)
        expect(match).toBeDefined()
        if (!match) return
        expect(text.slice(match.startIndex, match.endIndex)).toBe('file.ts:12:3')
    })

    test('따옴표로 감싼 경로는 따옴표를 제외하고 매칭한다', () => {
        const [match] = findTerminalLinkMatches(`open "file.ts:12"`)
        expect(match?.path).toBe('file.ts')
        expect(match?.text).toBe('file.ts:12')
    })
})
