import { describe, expect, test } from 'bun:test'
import type { ILink, Terminal } from '@xterm/xterm'
import { createTerminalFileLinkProvider } from '@features/terminal/terminal-file-link'

type FakeBufferLine = { translateToString: (trimRight: boolean) => string }

const createFakeTerm = (linesByIndex: Record<number, string>) => {
    const buffer = {
        active: {
            getLine: (index: number): FakeBufferLine | undefined => {
                const text = linesByIndex[index]
                return text === undefined ? undefined : { translateToString: () => text }
            },
        },
    }
    return { buffer } as unknown as Terminal
}

const collectLinks = (term: Terminal, onActivate: Parameters<typeof createTerminalFileLinkProvider>[1], bufferLineNumber: number) =>
    new Promise<ILink[] | undefined>((resolve) => {
        createTerminalFileLinkProvider(term, onActivate).provideLinks(bufferLineNumber, resolve)
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
