import { describe, expect, test } from 'bun:test'
import { flattenSelectionRangeChain, toMonacoSelectionRangeChain } from '@shared/lib/lsp/adapters/selection-range'

describe('flattenSelectionRangeChain', () => {
    test('undefined 는 빈 배열을 반환한다', () => {
        expect(flattenSelectionRangeChain(undefined)).toEqual([])
    })

    test('parent 체인을 안쪽에서 바깥쪽 순서로 펼친다', () => {
        const innermost = {
            range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
            parent: {
                range: { start: { line: 0, character: 0 }, end: { line: 0, character: 5 } },
                parent: { range: { start: { line: 0, character: 0 }, end: { line: 1, character: 0 } } },
            },
        }
        const chain = flattenSelectionRangeChain(innermost)
        expect(chain).toHaveLength(3)
        expect(chain[0]).toBe(innermost)
        expect(chain[2]).toBe(innermost.parent.parent)
    })
})

describe('toMonacoSelectionRangeChain', () => {
    test('LSP range 체인을 1-based monaco range 체인으로 변환한다', () => {
        const chain = toMonacoSelectionRangeChain({
            range: { start: { line: 1, character: 2 }, end: { line: 1, character: 6 } },
            parent: { range: { start: { line: 0, character: 0 }, end: { line: 2, character: 0 } } },
        })
        expect(chain).toEqual([
            { range: { startLineNumber: 2, startColumn: 3, endLineNumber: 2, endColumn: 7 } },
            { range: { startLineNumber: 1, startColumn: 1, endLineNumber: 3, endColumn: 1 } },
        ])
    })

    test('null 입력은 빈 배열을 반환한다', () => {
        expect(toMonacoSelectionRangeChain(null)).toEqual([])
    })
})
