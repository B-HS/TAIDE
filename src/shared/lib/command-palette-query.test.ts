import { describe, expect, test } from 'bun:test'
import type { languages } from 'monaco-editor'
import { buildCommandModeQuery, flattenDocumentSymbols, parseLineModeTarget, parsePaletteQuery } from '@shared/lib/command-palette-query'

describe('parsePaletteQuery', () => {
    test('">" 로 시작하지 않으면 파일 모드로 판단하고 입력 전체를 검색어로 본다', () => {
        expect(parsePaletteQuery('foo.ts')).toEqual({ mode: 'files', searchTerm: 'foo.ts' })
    })

    test('빈 입력은 파일 모드다', () => {
        expect(parsePaletteQuery('')).toEqual({ mode: 'files', searchTerm: '' })
    })

    test('">" 로 시작하면 커맨드 모드로 판단하고 접두사를 제거한 나머지를 검색어로 본다', () => {
        expect(parsePaletteQuery('>reload')).toEqual({ mode: 'commands', searchTerm: 'reload' })
    })

    test('">" 단독 입력은 검색어가 빈 커맨드 모드다', () => {
        expect(parsePaletteQuery('>')).toEqual({ mode: 'commands', searchTerm: '' })
    })

    test('">" 뒤 공백은 검색어에서 제거된다', () => {
        expect(parsePaletteQuery('>   reload window')).toEqual({ mode: 'commands', searchTerm: 'reload window' })
    })

    test('"@" 로 시작하면 symbol 모드다(문서 심볼)', () => {
        expect(parsePaletteQuery('@handleSave')).toEqual({ mode: 'symbol', searchTerm: 'handleSave' })
    })

    test('"@" 단독 입력은 검색어가 빈 symbol 모드다', () => {
        expect(parsePaletteQuery('@')).toEqual({ mode: 'symbol', searchTerm: '' })
    })

    test('":" 로 시작하면 line 모드다(줄 이동)', () => {
        expect(parsePaletteQuery(':123')).toEqual({ mode: 'line', searchTerm: '123' })
    })

    test('":" 뒤 "줄:열" 표기도 검색어로 그대로 보존한다', () => {
        expect(parsePaletteQuery(':123:45')).toEqual({ mode: 'line', searchTerm: '123:45' })
    })

    test('"#" 로 시작하면 workspaceSymbol 모드다(워크스페이스 심볼)', () => {
        expect(parsePaletteQuery('#handleSave')).toEqual({ mode: 'workspaceSymbol', searchTerm: 'handleSave' })
    })
})

describe('buildCommandModeQuery', () => {
    test('검색어 없이 호출하면 ">" 만 반환한다', () => {
        expect(buildCommandModeQuery()).toBe('>')
    })

    test('검색어를 붙이면 ">검색어" 형태로 반환한다', () => {
        expect(buildCommandModeQuery('reload')).toBe('>reload')
    })
})

describe('parseLineModeTarget', () => {
    test('숫자만 있으면 1열로 취급한다', () => {
        expect(parseLineModeTarget('123')).toEqual({ line: 123, column: 1 })
    })

    test('"줄:열" 표기를 파싱한다', () => {
        expect(parseLineModeTarget('123:45')).toEqual({ line: 123, column: 45 })
    })

    test('앞뒤 공백은 무시한다', () => {
        expect(parseLineModeTarget('  42  ')).toEqual({ line: 42, column: 1 })
    })

    test('빈 문자열은 null 이다', () => {
        expect(parseLineModeTarget('')).toBeNull()
    })

    test('숫자가 아닌 입력은 null 이다', () => {
        expect(parseLineModeTarget('abc')).toBeNull()
    })

    test('0 이하의 줄/열은 null 이다', () => {
        expect(parseLineModeTarget('0')).toBeNull()
        expect(parseLineModeTarget('1:0')).toBeNull()
    })

    test('콜론이 2개 이상이면 형식이 맞지 않아 null 이다', () => {
        expect(parseLineModeTarget('1:2:3')).toBeNull()
    })
})

describe('flattenDocumentSymbols', () => {
    const buildSymbol = (overrides: Partial<languages.DocumentSymbol> = {}): languages.DocumentSymbol => ({
        name: 'symbol',
        detail: '',
        kind: 12,
        tags: [],
        range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 1 },
        selectionRange: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 1 },
        ...overrides,
    })

    test('빈 배열은 빈 배열을 반환한다', () => {
        expect(flattenDocumentSymbols([])).toEqual([])
    })

    test('최상위 심볼은 containerLabel 이 빈 문자열이다', () => {
        const result = flattenDocumentSymbols([buildSymbol({ name: 'handleSave' })])
        expect(result).toEqual([{ name: 'handleSave', detail: '', kind: 12, containerLabel: '', selectionRange: buildSymbol().selectionRange }])
    })

    test('children 을 재귀적으로 평탄화하며 조상 이름을 "부모 > 자식" 형태의 containerLabel 로 쌓는다', () => {
        const result = flattenDocumentSymbols([
            buildSymbol({
                name: 'MyClass',
                children: [buildSymbol({ name: 'method', children: [buildSymbol({ name: 'inner' })] })],
            }),
        ])

        expect(result.map((symbol) => [symbol.name, symbol.containerLabel])).toEqual([
            ['MyClass', ''],
            ['method', 'MyClass'],
            ['inner', 'MyClass > method'],
        ])
    })

    test('형제 심볼은 서로 다른 depth 로 섞이지 않고 순서대로 나열된다', () => {
        const result = flattenDocumentSymbols([buildSymbol({ name: 'a', children: [buildSymbol({ name: 'a1' })] }), buildSymbol({ name: 'b' })])

        expect(result.map((symbol) => symbol.name)).toEqual(['a', 'a1', 'b'])
    })
})
