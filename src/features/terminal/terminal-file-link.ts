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

/** A regex match the resolver confirmed is a real file, carrying the absolute path its activation opens. */
export type ResolvedTerminalLinkMatch = TerminalLinkMatch & { resolvedPath: string }

/**
 * How many resolved rows one provider remembers, evicted first-in-first-out. Hovering along a row
 * re-queries it on every entry, and a scrollback screen holds far fewer distinct rows than this, so
 * the cap only bounds a terminal that keeps producing new linkable output.
 */
const LINK_RESOLVE_CACHE_MAX_ROWS = 256

const CACHE_KEY_SEPARATOR = '\n'

export type TerminalFileLinkProviderDeps = {
    /** The session's live cwd (OSC 7), read per call — a row's paths mean different files after a `cd`. */
    getCwd: () => string | null
    resolveCandidates: (cwd: string, candidates: string[]) => Promise<(string | null)[]>
    onActivate: (match: ResolvedTerminalLinkMatch, event: MouseEvent) => void
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
 * **Only matches that resolve to a real file become links.** The regex alone underlined `v18.20.4`,
 * `127.0.0.1:8080` and `0.123s`, and a click on any of them could do nothing but raise a failure
 * toast; `resolveCandidates` answers the whole row in one call before any link exists, and its
 * answer is cached per (cwd, row text) so re-hovering a row costs nothing. A `null` answer is
 * cached as well — rows whose candidates are not files are the ones a pointer sweeps across most
 * in noisy output, so re-asking for them on every hover would defeat the cache; the price is that a
 * file created *after* its row was printed stays unlinked in that row until the entry is evicted or
 * the cwd changes (a deliberate trade-off, `docs/features/terminal.md` §6). A rejected call is the
 * one thing left uncached, so a transient IPC failure retries. Rows are resolved asynchronously —
 * xterm allows `provideLinks` to call back later and re-queries per row, so a link simply appears
 * once the answer arrives. The resolver is injected rather than imported because this is a
 * `features` module: the IPC call belongs to the `widgets` owner (`terminal-session.tsx`).
 *
 * Because activation carries the already-resolved absolute path, opening the file needs no second
 * round trip. `onActivate` itself fires for every click on the link's range, unfiltered — the caller
 * (`terminal-view.tsx`) applies the same modifier gate ({@link shouldActivateTerminalLink}) the URL
 * link handler uses, so this provider keeps no UI policy of its own.
 */
export const createTerminalFileLinkProvider = (
    term: Terminal,
    { getCwd, resolveCandidates, onActivate }: TerminalFileLinkProviderDeps,
): ILinkProvider => {
    const resolvedRows = new Map<string, (string | null)[]>()

    const resolveRow = async (cacheKey: string, cwd: string, candidates: string[]) => {
        const cached = resolvedRows.get(cacheKey)
        if (cached) return cached

        const resolved = await resolveCandidates(cwd, candidates)
        resolvedRows.set(cacheKey, resolved)
        if (resolvedRows.size > LINK_RESOLVE_CACHE_MAX_ROWS) {
            const [oldestKey] = resolvedRows.keys()
            resolvedRows.delete(oldestKey)
        }
        return resolved
    }

    return {
        provideLinks: (bufferLineNumber, callback) => {
            const line = term.buffer.active.getLine(bufferLineNumber - 1)
            const cwd = getCwd()
            if (!line || !cwd) {
                callback(undefined)
                return
            }

            const { text, columns } = readTerminalRowColumns(line)
            const rowText = text.trimEnd()
            const matches = findTerminalLinkMatches(rowText)
            if (matches.length === 0) {
                callback(undefined)
                return
            }

            void resolveRow(
                `${cwd}${CACHE_KEY_SEPARATOR}${rowText}`,
                cwd,
                matches.map((match) => match.path),
            )
                .then((resolved) => {
                    const links = matches.flatMap<ILink>((match, index) => {
                        const resolvedPath = resolved[index]
                        if (!resolvedPath) return []
                        return {
                            text: match.text,
                            range: {
                                start: { x: columns[match.startIndex] + 1, y: bufferLineNumber },
                                end: { x: columns[match.endIndex], y: bufferLineNumber },
                            },
                            decorations: { pointerCursor: true, underline: true },
                            activate: (event) => onActivate({ ...match, resolvedPath }, event),
                        }
                    })
                    callback(links.length > 0 ? links : undefined)
                })
                .catch(() => callback(undefined))
        },
    }
}
