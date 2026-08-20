import type { IPosition, IRange } from 'monaco-editor'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import type { Location, LocationLink, LspPosition, LspRange } from '@shared/lib/lsp/protocol'

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

/**
 * A `LocationLink`'s `targetRange` spans the whole declaration (doc comments, attributes and all);
 * `targetSelectionRange` is the precise identifier span monaco uses for the cursor position and
 * Peek highlight (LSP 3.17 `LocationLink`; monaco's own `isLocationLink`/`goToLocations` fall back
 * to `range` — i.e. `targetRange` — whenever `targetSelectionRange` is absent). Dropping it here
 * used to land F12/Peek on the declaration's doc comment instead of the symbol itself. Shared by
 * `adapters/definition.ts` (goto-definition family), `adapters/references.ts` (which only ever
 * passes a plain `Location`, never a `LocationLink` — the `'targetUri' in item` branch is unused
 * from that call site but still correct for it), and `command-relay.ts`
 * (`rust-analyzer.gotoLocation`/`showReferences` client commands) — every caller needs the same LSP
 * `Location | LocationLink` shape normalized to monaco's.
 */
export const lspLocationToMonaco = (monaco: Monaco, item: Location | LocationLink) =>
    'targetUri' in item
        ? {
              uri: monaco.Uri.parse(item.targetUri),
              range: lspRangeToMonaco(item.targetRange),
              targetSelectionRange: lspRangeToMonaco(item.targetSelectionRange),
              ...(item.originSelectionRange ? { originSelectionRange: lspRangeToMonaco(item.originSelectionRange) } : {}),
          }
        : { uri: monaco.Uri.parse(item.uri), range: lspRangeToMonaco(item.range) }

/** The local filesystem path a `Location`/`LocationLink` points at, or `null` for a non-`file` scheme URI. */
export const lspLocationTargetPath = (monaco: Monaco, item: Location | LocationLink) => {
    const uri = monaco.Uri.parse('targetUri' in item ? item.targetUri : item.uri)
    return uri.scheme === 'file' ? uri.fsPath : null
}
