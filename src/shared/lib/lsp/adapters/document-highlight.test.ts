import { describe, expect, test } from 'bun:test'
import { toMonacoDocumentHighlight, toMonacoHighlightKind } from '@shared/lib/lsp/adapters/document-highlight'
import { DOCUMENT_HIGHLIGHT_KIND } from '@shared/lib/lsp/protocol'

describe('toMonacoHighlightKind', () => {
    test('kind 가 없으면 Text(0) 로 기본 처리한다', () => {
        expect(toMonacoHighlightKind(undefined)).toBe(0)
    })

    test('LSP Read(2) 를 monaco Read(1) 로 변환한다', () => {
        expect(toMonacoHighlightKind(DOCUMENT_HIGHLIGHT_KIND.READ)).toBe(1)
    })

    test('LSP Write(3) 를 monaco Write(2) 로 변환한다', () => {
        expect(toMonacoHighlightKind(DOCUMENT_HIGHLIGHT_KIND.WRITE)).toBe(2)
    })
})

describe('toMonacoDocumentHighlight', () => {
    test('LSP range 를 1-based monaco range 로 변환한다', () => {
        const result = toMonacoDocumentHighlight({
            range: { start: { line: 2, character: 4 }, end: { line: 2, character: 8 } },
            kind: DOCUMENT_HIGHLIGHT_KIND.WRITE,
        })
        expect(result).toEqual({ range: { startLineNumber: 3, startColumn: 5, endLineNumber: 3, endColumn: 9 }, kind: 2 })
    })
})
