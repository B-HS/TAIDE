import type { IPosition, IRange } from 'monaco-editor'
import type { LspPosition, LspRange } from '@shared/lib/lsp/protocol'

export const lspPositionToMonaco = (position: LspPosition): IPosition => ({
    lineNumber: position.line + 1,
    column: position.character + 1,
})

export const monacoPositionToLsp = (position: IPosition): LspPosition => ({
    line: position.lineNumber - 1,
    character: position.column - 1,
})

export const lspRangeToMonaco = (range: LspRange): IRange => ({
    startLineNumber: range.start.line + 1,
    startColumn: range.start.character + 1,
    endLineNumber: range.end.line + 1,
    endColumn: range.end.character + 1,
})

export const monacoRangeToLsp = (range: IRange): LspRange => ({
    start: { line: range.startLineNumber - 1, character: range.startColumn - 1 },
    end: { line: range.endLineNumber - 1, character: range.endColumn - 1 },
})
