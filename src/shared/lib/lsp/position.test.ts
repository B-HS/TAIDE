import { describe, expect, test } from 'bun:test'
import { lspPositionToMonaco, lspRangeToMonaco, monacoPositionToLsp, monacoRangeToLsp } from '@shared/lib/lsp/position'

describe('lspPositionToMonaco', () => {
    test('0-based line/character 를 1-based lineNumber/column 으로 변환한다', () => {
        expect(lspPositionToMonaco({ line: 0, character: 0 })).toEqual({ lineNumber: 1, column: 1 })
    })

    test('임의의 위치를 정확히 오프셋한다', () => {
        expect(lspPositionToMonaco({ line: 4, character: 10 })).toEqual({ lineNumber: 5, column: 11 })
    })
})

describe('monacoPositionToLsp', () => {
    test('1-based lineNumber/column 을 0-based line/character 로 변환한다', () => {
        expect(monacoPositionToLsp({ lineNumber: 1, column: 1 })).toEqual({ line: 0, character: 0 })
    })

    test('임의의 위치를 정확히 오프셋한다', () => {
        expect(monacoPositionToLsp({ lineNumber: 5, column: 11 })).toEqual({ line: 4, character: 10 })
    })
})

describe('lspPositionToMonaco ↔ monacoPositionToLsp', () => {
    test('왕복 변환이 원본 값을 보존한다', () => {
        const original = { line: 42, character: 7 }
        expect(monacoPositionToLsp(lspPositionToMonaco(original))).toEqual(original)
    })
})

describe('lspRangeToMonaco', () => {
    test('range 의 start/end 를 각각 1-based 로 변환한다', () => {
        expect(
            lspRangeToMonaco({
                start: { line: 0, character: 0 },
                end: { line: 2, character: 5 },
            }),
        ).toEqual({ startLineNumber: 1, startColumn: 1, endLineNumber: 3, endColumn: 6 })
    })
})

describe('monacoRangeToLsp', () => {
    test('range 의 start/end 를 각각 0-based 로 변환한다', () => {
        expect(monacoRangeToLsp({ startLineNumber: 1, startColumn: 1, endLineNumber: 3, endColumn: 6 })).toEqual({
            start: { line: 0, character: 0 },
            end: { line: 2, character: 5 },
        })
    })
})

describe('lspRangeToMonaco ↔ monacoRangeToLsp', () => {
    test('왕복 변환이 원본 값을 보존한다', () => {
        const original = { start: { line: 3, character: 2 }, end: { line: 3, character: 9 } }
        expect(monacoRangeToLsp(lspRangeToMonaco(original))).toEqual(original)
    })
})
