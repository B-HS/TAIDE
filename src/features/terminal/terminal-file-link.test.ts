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

const TEST_CWD = '/repo'

type ProviderDeps = Parameters<typeof createTerminalFileLinkProvider>[1]

/** Answers every candidate with an absolute path, except the ones named in `missing` (a match the backend could not open). */
const createResolverStub = (missing: string[] = []) => {
    const calls: { cwd: string; candidates: string[] }[] = []
    const resolveCandidates: ProviderDeps['resolveCandidates'] = async (cwd, candidates) => {
        calls.push({ cwd, candidates })
        return candidates.map((candidate) => (missing.includes(candidate) ? null : `${cwd}/${candidate}`))
    }
    return { calls, resolveCandidates }
}

const createProvider = (term: Terminal, deps: Partial<ProviderDeps> = {}) =>
    createTerminalFileLinkProvider(term, {
        getCwd: () => TEST_CWD,
        resolveCandidates: createResolverStub().resolveCandidates,
        onActivate: () => undefined,
        ...deps,
    })

const provideLinks = (provider: ReturnType<typeof createProvider>, bufferLineNumber: number) =>
    new Promise<ILink[] | undefined>((resolve) => {
        provider.provideLinks(bufferLineNumber, resolve)
    })

const collectLinks = (term: Terminal, deps: Partial<ProviderDeps>, bufferLineNumber: number) =>
    provideLinks(createProvider(term, deps), bufferLineNumber)

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
    test('매칭되는 경로가 없으면 resolver 를 부르지 않고 undefined 를 콜백한다', async () => {
        const term = createFakeTerm({ 4: 'hello world' })
        const resolver = createResolverStub()
        const links = await collectLinks(term, { resolveCandidates: resolver.resolveCandidates }, 5)
        expect(links).toBeUndefined()
        expect(resolver.calls).toHaveLength(0)
    })

    test('버퍼에 해당 줄이 없으면 undefined 를 콜백한다', async () => {
        const term = createFakeTerm({})
        const links = await collectLinks(term, {}, 1)
        expect(links).toBeUndefined()
    })

    test('cwd 를 아직 모르면 resolver 를 부르지 않고 undefined 를 콜백한다', async () => {
        const term = createFakeTerm({ 0: 'open file.ts' })
        const resolver = createResolverStub()
        const links = await collectLinks(term, { getCwd: () => null, resolveCandidates: resolver.resolveCandidates }, 1)
        expect(links).toBeUndefined()
        expect(resolver.calls).toHaveLength(0)
    })

    test('한 행의 후보 전체를 cwd 와 함께 한 번에 resolver 로 넘긴다', async () => {
        const term = createFakeTerm({ 2: 'diff src/a.ts:10 vs src/b.ts:20:2' })
        const resolver = createResolverStub()
        await collectLinks(term, { resolveCandidates: resolver.resolveCandidates }, 3)
        expect(resolver.calls).toEqual([{ cwd: TEST_CWD, candidates: ['src/a.ts', 'src/b.ts'] }])
    })

    test('경로 하나를 1-based 버퍼 좌표의 range 로 매핑한다', async () => {
        const term = createFakeTerm({ 0: 'open file.ts:12:3 now' })
        const links = await collectLinks(term, {}, 1)
        expect(links).toHaveLength(1)
        expect(links?.[0]).toMatchObject({
            text: 'file.ts:12:3',
            range: { start: { x: 6, y: 1 }, end: { x: 17, y: 1 } },
        })
    })

    test('한 줄에 여러 경로가 있으면 각각 별도 링크로 매핑한다', async () => {
        const term = createFakeTerm({ 2: 'diff src/a.ts:10 vs src/b.ts:20:2' })
        const links = await collectLinks(term, {}, 3)
        expect(links).toHaveLength(2)
        expect(links?.[0].text).toBe('src/a.ts:10')
        expect(links?.[1].text).toBe('src/b.ts:20:2')
    })

    test('해석되지 않은 후보는 링크가 되지 않고 나머지만 남는다', async () => {
        const term = createFakeTerm({ 2: 'diff src/a.ts:10 vs src/b.ts:20:2' })
        const links = await collectLinks(term, { resolveCandidates: createResolverStub(['src/a.ts']).resolveCandidates }, 3)
        expect(links).toHaveLength(1)
        expect(links?.[0].text).toBe('src/b.ts:20:2')
    })

    test('행의 모든 후보가 해석되지 않으면 undefined 를 콜백한다 — 정규식만으로는 밑줄을 긋지 않는다', async () => {
        const term = createFakeTerm({ 0: 'node v18.20.4' })
        const links = await collectLinks(term, { resolveCandidates: createResolverStub(['v18.20.4']).resolveCandidates }, 1)
        expect(links).toBeUndefined()
    })

    test('앞에 와이드 문자가 있으면 문자열 인덱스가 아니라 셀 열로 range 를 잡는다', async () => {
        const term = createFakeTerm({ 0: '한글 src/a.ts:10 끝' })
        const links = await collectLinks(term, {}, 1)
        expect(links?.[0]).toMatchObject({
            text: 'src/a.ts:10',
            range: { start: { x: 6, y: 1 }, end: { x: 16, y: 1 } },
        })
    })

    test('서로게이트 페어(이모지) 접두도 코드유닛 단위로 열을 맞춘다', async () => {
        const term = createFakeTerm({ 0: '🚀 a.ts' })
        const links = await collectLinks(term, {}, 1)
        expect(links?.[0]).toMatchObject({ text: 'a.ts', range: { start: { x: 4, y: 1 }, end: { x: 7, y: 1 } } })
    })

    test('링크의 activate 는 modifier 검증 없이 해석된 절대 경로를 실은 match 를 그대로 전달한다', async () => {
        const term = createFakeTerm({ 0: 'see ./rel/path.rs:5' })
        const calls: { path: string; resolvedPath: string; line: number | undefined }[] = []
        const links = await collectLinks(
            term,
            { onActivate: (match) => calls.push({ path: match.path, resolvedPath: match.resolvedPath, line: match.line }) },
            1,
        )
        const fakeEvent = {} as MouseEvent
        links?.[0].activate(fakeEvent, links[0].text)
        expect(calls).toEqual([{ path: './rel/path.rs', resolvedPath: `${TEST_CWD}/./rel/path.rs`, line: 5 }])
    })

    test('decorations 는 항상 pointerCursor/underline 를 켠다', async () => {
        const term = createFakeTerm({ 0: 'file.ts' })
        const links = await collectLinks(term, {}, 1)
        expect(links?.[0].decorations).toEqual({ pointerCursor: true, underline: true })
    })

    test('같은 (cwd, 행 텍스트) 를 다시 조회하면 캐시로 답하고 resolver 를 다시 부르지 않는다', async () => {
        const term = createFakeTerm({ 0: 'open file.ts:12', 1: 'open file.ts:12' })
        const resolver = createResolverStub()
        const provider = createProvider(term, { resolveCandidates: resolver.resolveCandidates })

        const first = await provideLinks(provider, 1)
        const second = await provideLinks(provider, 2)

        expect(first).toHaveLength(1)
        expect(second).toHaveLength(1)
        expect(resolver.calls).toHaveLength(1)
    })

    test('cwd 가 바뀌면 같은 행이라도 다시 해석한다', async () => {
        const term = createFakeTerm({ 0: 'open file.ts:12' })
        const resolver = createResolverStub()
        let cwd = TEST_CWD
        const provider = createProvider(term, { getCwd: () => cwd, resolveCandidates: resolver.resolveCandidates })

        await provideLinks(provider, 1)
        cwd = '/repo/nested'
        await provideLinks(provider, 1)

        expect(resolver.calls.map(({ cwd: calledCwd }) => calledCwd)).toEqual([TEST_CWD, '/repo/nested'])
    })

    test('부재(null) 응답도 캐시한다 — 행이 찍힌 뒤 생긴 파일은 다시 검사하지 않는다(문서화된 트레이드오프)', async () => {
        const term = createFakeTerm({ 0: 'open file.ts:12' })
        const missing = createResolverStub(['file.ts'])
        const found = createResolverStub()
        let hasFile = false
        const provider = createProvider(term, {
            resolveCandidates: (cwd, candidates) => (hasFile ? found.resolveCandidates(cwd, candidates) : missing.resolveCandidates(cwd, candidates)),
        })

        expect(await provideLinks(provider, 1)).toBeUndefined()
        hasFile = true

        expect(await provideLinks(provider, 1)).toBeUndefined()
        expect(missing.calls).toHaveLength(1)
        expect(found.calls).toHaveLength(0)
    })

    test('resolver 가 실패하면 링크 없이 undefined 를 콜백하고, 실패는 캐시하지 않아 다음 조회에서 다시 시도한다', async () => {
        const term = createFakeTerm({ 0: 'open file.ts:12' })
        const resolver = createResolverStub()
        let shouldFail = true
        const provider = createProvider(term, {
            getCwd: () => TEST_CWD,
            resolveCandidates: (cwd, candidates) =>
                shouldFail ? Promise.reject(new Error('ipc failed')) : resolver.resolveCandidates(cwd, candidates),
        })

        expect(await provideLinks(provider, 1)).toBeUndefined()
        shouldFail = false
        expect(await provideLinks(provider, 1)).toHaveLength(1)
    })
})
