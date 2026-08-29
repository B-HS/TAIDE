import type { IBufferCell, ILink, ILinkProvider, Terminal } from '@xterm/xterm'
import type { TerminalLinkMatch } from '@shared/lib/terminal-link'
import { findTerminalLinkMatches } from '@shared/lib/terminal-link'

const WHITESPACE_CELL_CHAR = ' '

type TerminalRowCell = Pick<IBufferCell, 'getChars' | 'getWidth'>

type TerminalRow = {
    length: number
    getCell: (column: number) => TerminalRowCell | undefined
}

/**
 * Reads one buffer row as the string xterm's own `translateToString` would produce, *plus* the cell
 * column each UTF-16 code unit of that string came from (`columns[i]` for index `i`, with one extra
 * trailing entry for the column just past the row's last cell).
 *
 * The column map is what makes a match's string offset usable as a link range: a CJK glyph occupies
 * **one** string character but **two** buffer cells (its zero-width continuation cell contributes no
 * character at all), so any wide character earlier on the row shifts every later cell right of where
 * a plain `index + 1` would put it. The link then underlines — and reacts to clicks on — the wrong
 * cells, drifting further with each wide glyph. Everything {@link findTerminalLinkMatches} can match
 * is single-width ASCII, but what precedes a match on the row (a CJK log prefix, an emoji status
 * marker) is arbitrary, which is why this walks cells instead of assuming a 1:1 mapping.
 *
 * xterm's public `IBufferLine.translateToString` cannot supply this: the internal `BufferLine`
 * implementation does take an `outColumns` out-parameter, but `BufferLineApiView` — the object the
 * public `buffer.active.getLine()` hands back — forwards only `(trimRight, startColumn, endColumn)`
 * and drops the fourth argument, so passing one would silently return an empty map. Walking the row
 * reproduces that implementation exactly instead: characters come from `getChars()` with an empty
 * cell reading as a space, and the next cell is `getWidth() || 1` columns along.
 */
export const readTerminalRowColumns = (line: TerminalRow) => {
    const columns: number[] = []
    let text = ''
    let column = 0

    while (column < line.length) {
        const cell = line.getCell(column)
        const chars = cell?.getChars() || WHITESPACE_CELL_CHAR
        text += chars
        for (let unit = 0; unit < chars.length; unit += 1) columns.push(column)
        column += cell?.getWidth() || 1
    }
    columns.push(column)

    return { text, columns }
}

/**
 * Builds an xterm `ILinkProvider` that turns {@link findTerminalLinkMatches}' regex hits on a
 * single buffer row into clickable file-path links (FR-G2). Deliberately scoped to one physical
 * row per `provideLinks` call — unlike `@xterm/addon-web-links`'s own `LinkComputer`, it does not
 * stitch a match across a wrapped line boundary (a path that wraps mid-token is not linkified;
 * accepted as a minimal-wiring scope cut, matching X-A's OSC7 precedent of skipping chunk-boundary
 * reassembly). String offsets are mapped onto buffer columns through
 * {@link readTerminalRowColumns} rather than used directly, so a row whose prefix contains wide
 * characters still underlines the cells the path actually occupies (audit §4-B C13).
 *
 * `onActivate` fires for every click on the link's range, unfiltered — the caller (`terminal-view
 * .tsx`) applies the same modifier gate ({@link shouldActivateTerminalLink}) the URL link handler
 * uses, so this provider stays a pure buffer-to-link mapping with no UI policy of its own.
 */
export const createTerminalFileLinkProvider = (term: Terminal, onActivate: (match: TerminalLinkMatch, event: MouseEvent) => void): ILinkProvider => ({
    provideLinks: (bufferLineNumber, callback) => {
        const line = term.buffer.active.getLine(bufferLineNumber - 1)
        if (!line) {
            callback(undefined)
            return
        }

        const { text, columns } = readTerminalRowColumns(line)
        const matches = findTerminalLinkMatches(text.trimEnd())
        if (matches.length === 0) {
            callback(undefined)
            return
        }

        const links: ILink[] = matches.map((match) => ({
            text: match.text,
            range: {
                start: { x: columns[match.startIndex] + 1, y: bufferLineNumber },
                end: { x: columns[match.endIndex], y: bufferLineNumber },
            },
            decorations: { pointerCursor: true, underline: true },
            activate: (event) => onActivate(match, event),
        }))
        callback(links)
    },
})
