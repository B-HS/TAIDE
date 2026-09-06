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

    test('path:line:col 뒤의 콜론은 링크 밖으로 남긴다', () => {
        const [match] = findTerminalLinkMatches('src/main.rs:12:3: error[E0433]')
        expect(match).toMatchObject({ path: 'src/main.rs', line: 12, column: 3 })
        expect(match?.text).toBe('src/main.rs:12:3')
    })

    test('tsc 의 path(line,col) 형태를 매칭한다', () => {
        const text = 'src/a.ts(12,3): error TS2304: Cannot find name'
        const [match] = findTerminalLinkMatches(text)
        expect(match).toBeDefined()
        if (!match) return
        expect(match).toMatchObject({ path: 'src/a.ts', line: 12, column: 3 })
        expect(text.slice(match.startIndex, match.endIndex)).toBe('src/a.ts(12,3)')
    })

    test('path(line) 형태는 column 없이 매칭한다', () => {
        const [match] = findTerminalLinkMatches('src/a.ts(12): warning')
        expect(match).toMatchObject({ path: 'src/a.ts', line: 12, column: undefined })
        expect(match?.text).toBe('src/a.ts(12)')
    })

    test('python 트레이스백의 큰따옴표 + line N 을 매칭한다', () => {
        const matches = findTerminalLinkMatches('  File "/abs/x.py", line 42, in <module>')
        expect(matches).toHaveLength(1)
        expect(matches[0]).toMatchObject({ path: '/abs/x.py', line: 42, column: undefined })
        expect(matches[0]?.text).toBe('/abs/x.py", line 42')
    })

    test('python 트레이스백의 작은따옴표 + line N 을 매칭한다', () => {
        const [match] = findTerminalLinkMatches("File '/abs/x.py', line 7")
        expect(match).toMatchObject({ path: '/abs/x.py', line: 7, column: undefined })
        expect(match?.text).toBe("/abs/x.py', line 7")
    })

    test('GitHub 식 #L 좌표를 매칭한다', () => {
        const [match] = findTerminalLinkMatches('src/a.ts#L42')
        expect(match).toMatchObject({ path: 'src/a.ts', line: 42, column: undefined })
        expect(match?.text).toBe('src/a.ts#L42')
    })

    test('GitHub 식 #L 범위는 시작 줄만 취한다', () => {
        const [withLPrefix] = findTerminalLinkMatches('src/a.ts#L10-L20')
        expect(withLPrefix).toMatchObject({ path: 'src/a.ts', line: 10, column: undefined })
        expect(withLPrefix?.text).toBe('src/a.ts#L10-L20')

        const [withoutLPrefix] = findTerminalLinkMatches('src/a.ts#L10-20')
        expect(withoutLPrefix).toMatchObject({ path: 'src/a.ts', line: 10, column: undefined })
        expect(withoutLPrefix?.text).toBe('src/a.ts#L10-20')
    })

    test('path:line-line 범위는 시작 줄만 취한다', () => {
        const [match] = findTerminalLinkMatches('see src/a.ts:10-20 for context')
        expect(match).toMatchObject({ path: 'src/a.ts', line: 10, column: undefined })
        expect(match?.text).toBe('src/a.ts:10-20')
    })

    test('여러 접미사가 겹치면 가장 긴 접미사를 채택한다', () => {
        const [colonPair] = findTerminalLinkMatches('at file.ts:12:3')
        expect(colonPair?.text).toBe('file.ts:12:3')

        const [range] = findTerminalLinkMatches('at file.ts:12-30')
        expect(range?.text).toBe('file.ts:12-30')
    })

    test('꺾쇠로 감싼 경로도 좌표까지 매칭한다', () => {
        const [match] = findTerminalLinkMatches('<file.ts:3>')
        expect(match).toMatchObject({ path: 'file.ts', line: 3, column: undefined })
        expect(match?.text).toBe('file.ts:3')
    })

    test('URL 내부의 경로는 파일 링크로 매칭하지 않는다', () => {
        expect(findTerminalLinkMatches('see http://x/y.ts for detail')).toHaveLength(0)
        expect(findTerminalLinkMatches('https://github.com/o/r/blob/main/src/a.ts#L42')).toHaveLength(0)
    })

    test('버전 문자열에는 좌표를 붙이지 않는다', () => {
        const [match] = findTerminalLinkMatches('taide v1.2.3 released')
        expect(match).toMatchObject({ path: 'v1.2.3', line: undefined, column: undefined })
        expect(match?.text).toBe('v1.2.3')
    })

    test('접미사 뒤에 이어지는 다음 링크도 각각 매칭한다', () => {
        const matches = findTerminalLinkMatches('src/a.ts(1,2) then src/b.ts#L3')
        expect(matches).toHaveLength(2)
        expect(matches[0]).toMatchObject({ path: 'src/a.ts', line: 1, column: 2 })
        expect(matches[1]).toMatchObject({ path: 'src/b.ts', line: 3, column: undefined })
    })
})
