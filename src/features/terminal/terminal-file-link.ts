import type { ILink, ILinkProvider, Terminal } from '@xterm/xterm'
import type { TerminalLinkMatch } from '@shared/lib/terminal-link'
import { findTerminalLinkMatches } from '@shared/lib/terminal-link'

/**
 * Builds an xterm `ILinkProvider` that turns {@link findTerminalLinkMatches}' regex hits on a
 * single buffer row into clickable file-path links (FR-G2). Deliberately scoped to one physical
 * row per `provideLinks` call — unlike `@xterm/addon-web-links`'s own `LinkComputer`, it does not
 * stitch a match across a wrapped line boundary (a path that wraps mid-token is not linkified;
 * accepted as a minimal-wiring scope cut, matching X-A's OSC7 precedent of skipping chunk-boundary
 * reassembly). Every character {@link findTerminalLinkMatches} can match (`\w`, `.`, `@`, `+`, `-`,
 * `~`, `/`) is single-cell-width ASCII, so a match's string offset within the row maps 1:1 onto the
 * row's buffer column — no `LinkComputer`-style wide-char correction is needed the way arbitrary
 * (possibly CJK-containing) URL text requires.
 *
 * `onActivate` fires for every click on the link's range, unfiltered — the caller (`terminal-view
 * .tsx`) applies the same modifier gate ({@link shouldActivateTerminalLink}) the URL link handler
 * uses, so this provider stays a pure buffer-to-link mapping with no UI policy of its own.
 */
export const createTerminalFileLinkProvider = (term: Terminal, onActivate: (match: TerminalLinkMatch, event: MouseEvent) => void): ILinkProvider => ({
    provideLinks: (bufferLineNumber, callback) => {
        const line = term.buffer.active.getLine(bufferLineNumber - 1)
        const text = line?.translateToString(true)
        if (!text) {
            callback(undefined)
            return
        }

        const matches = findTerminalLinkMatches(text)
        if (matches.length === 0) {
            callback(undefined)
            return
        }

        const links: ILink[] = matches.map((match) => ({
            text: match.text,
            range: { start: { x: match.startIndex + 1, y: bufferLineNumber }, end: { x: match.endIndex, y: bufferLineNumber } },
            decorations: { pointerCursor: true, underline: true },
            activate: (event) => onActivate(match, event),
        }))
        callback(links)
    },
})
