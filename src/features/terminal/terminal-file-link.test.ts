import { describe, expect, test } from 'bun:test'
import type { ILink, Terminal } from '@xterm/xterm'
import { createTerminalFileLinkProvider, readTerminalRowColumns } from '@features/terminal/terminal-file-link'

const WIDE_CHAR_MIN_CODE_POINT = 0x1100

type FakeCell = { chars: string; width: number }

/**
 * Mirrors how xterm lays a string out in a buffer row: a CJK/emoji glyph occupies one cell of width
 * 2 followed by a zero-width continuation cell, everything else one cell of width 1. Everything the
 * link regex can match is ASCII, so the interesting case is what sits *before* a match on the row.
 */
const cellsOf = (text: string) =>
    [...text].flatMap<FakeCell>((char) =>
        (char.codePointAt(0) ?? 0) >= WIDE_CHAR_MIN_CODE_POINT
            ? [
                  { chars: char, width: 2 },
                  { chars: '', width: 0 },
              ]
            : [{ chars: char, width: 1 }],
    )

const createFakeTerm = (linesByIndex: Record<number, string>) => {
    const buffer = {
        active: {
            getLine: (index: number) => {
                const text = linesByIndex[index]
                if (text === undefined) return undefined
                const cells = cellsOf(text)
                return {
                    length: cells.length,
                    getCell: (column: number) => {
                        const cell = cells[column]
                        return cell ? { getChars: () => cell.chars, getWidth: () => cell.width } : undefined
                    },
                }
            },
        },
    }
    return { buffer } as unknown as Terminal
}

const collectLinks = (term: Terminal, onActivate: Parameters<typeof createTerminalFileLinkProvider>[1], bufferLineNumber: number) =>
    new Promise<ILink[] | undefined>((resolve) => {
        createTerminalFileLinkProvider(term, onActivate).provideLinks(bufferLineNumber, resolve)
    })

describe('readTerminalRowColumns', () => {
    test('단일폭 행은 문자열 인덱스와 열이 1:1 이다', () => {
        const { text, columns } = readTerminalRowColumns({
            length: 3,
            getCell: (column) => ({ getChars: () => 'abc'[column] ?? '', getWidth: () => 1 }),
        })
        expect(text).toBe('abc')
        expect(columns).toEqual([0, 1, 2, 3])
    })

    test('와이드 문자는 열을 2 소비하고 continuation 셀은 문자열에 나타나지 않는다', () => {
        const cells = cellsOf('한a')
        const { text, columns } = readTerminalRowColumns({
            length: cells.length,
            getCell: (column) => {
                const cell = cells[column]
                return cell ? { getChars: () => cell.chars, getWidth: () => cell.width } : undefined
            },
        })
        expect(text).toBe('한a')
        expect(columns).toEqual([0, 2, 3])
    })

    test('빈 셀은 공백으로 읽는다 — xterm translateToString 과 같은 규약', () => {
        const { text } = readTerminalRowColumns({ length: 2, getCell: () => ({ getChars: () => '', getWidth: () => 1 }) })
        expect(text).toBe('  ')
    })
})

describe('createTerminalFileLinkProvider', () => {
    test('매칭되는 경로가 없으면 undefined 를 콜백한다', async () => {
        const term = createFakeTerm({ 4: 'hello world' })
        const links = await collectLinks(term, () => undefined, 5)
        expect(links).toBeUndefined()
    })

    test('버퍼에 해당 줄이 없으면 undefined 를 콜백한다', async () => {
        const term = createFakeTerm({})
        const links = await collectLinks(term, () => undefined, 1)
        expect(links).toBeUndefined()
    })

    test('경로 하나를 1-based 버퍼 좌표의 range 로 매핑한다', async () => {
        const term = createFakeTerm({ 0: 'open file.ts:12:3 now' })
        const links = await collectLinks(term, () => undefined, 1)
        expect(links).toHaveLength(1)
        expect(links?.[0]).toMatchObject({
            text: 'file.ts:12:3',
            range: { start: { x: 6, y: 1 }, end: { x: 17, y: 1 } },
        })
    })

    test('한 줄에 여러 경로가 있으면 각각 별도 링크로 매핑한다', async () => {
        const term = createFakeTerm({ 2: 'diff src/a.ts:10 vs src/b.ts:20:2' })
        const links = await collectLinks(term, () => undefined, 3)
        expect(links).toHaveLength(2)
        expect(links?.[0].text).toBe('src/a.ts:10')
        expect(links?.[1].text).toBe('src/b.ts:20:2')
    })

    test('앞에 와이드 문자가 있으면 문자열 인덱스가 아니라 셀 열로 range 를 잡는다', async () => {
        const term = createFakeTerm({ 0: '한글 src/a.ts:10 끝' })
        const links = await collectLinks(term, () => undefined, 1)
        expect(links?.[0]).toMatchObject({
            text: 'src/a.ts:10',
            range: { start: { x: 6, y: 1 }, end: { x: 16, y: 1 } },
        })
    })

    test('서로게이트 페어(이모지) 접두도 코드유닛 단위로 열을 맞춘다', async () => {
        const term = createFakeTerm({ 0: '🚀 a.ts' })
        const links = await collectLinks(term, () => undefined, 1)
        expect(links?.[0]).toMatchObject({ text: 'a.ts', range: { start: { x: 4, y: 1 }, end: { x: 7, y: 1 } } })
    })

    test('링크의 activate 는 modifier 검증 없이 그대로 match/event 를 onActivate 에 전달한다', async () => {
        const term = createFakeTerm({ 0: 'see ./rel/path.rs:5' })
        const calls: { path: string; line: number | undefined }[] = []
        const links = await collectLinks(term, (match) => calls.push({ path: match.path, line: match.line }), 1)
        const fakeEvent = {} as MouseEvent
        links?.[0].activate(fakeEvent, links[0].text)
        expect(calls).toEqual([{ path: './rel/path.rs', line: 5 }])
    })

    test('decorations 는 항상 pointerCursor/underline 를 켠다', async () => {
        const term = createFakeTerm({ 0: 'file.ts' })
        const links = await collectLinks(term, () => undefined, 1)
        expect(links?.[0].decorations).toEqual({ pointerCursor: true, underline: true })
    })
})
