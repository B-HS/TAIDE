export type TerminalLinkMatch = {
    path: string
    line: number | undefined
    column: number | undefined
    startIndex: number
    endIndex: number
    text: string
}

const TERMINAL_LINK_PATH_PATTERN = /(?:^|[\s'"`([<])((?:~|\.{1,2})?\/?[\w.@+-]+(?:\/[\w.@+-]+)*\.[A-Za-z0-9]{1,10})/g

const TERMINAL_LINK_SUFFIX_PATTERNS = [
    /^:(\d+):(\d+)/,
    /^:(\d+)(?:-\d+)?/,
    /^\((\d+),(\d+)\)/,
    /^\((\d+)\)/,
    /^#L(\d+)(?:-L?\d+)?/,
    /^["'], line (\d+)/,
] as const

const matchTerminalLinkSuffix = (textAfterPath: string) =>
    TERMINAL_LINK_SUFFIX_PATTERNS.reduce<RegExpExecArray | null>((longest, pattern) => {
        const result = pattern.exec(textAfterPath)
        if (!result) return longest
        return longest !== null && longest[0].length >= result[0].length ? longest : result
    }, null)

/**
 * Finds file paths printed in terminal output, each carrying the 1-based line/column of the longest
 * coordinate suffix that follows it. `path` excludes the suffix, while `text` and `endIndex` include
 * it so the underline covers the coordinates. Accepted suffix syntaxes: `docs/features/terminal.md`.
 */
export const findTerminalLinkMatches = (text: string): TerminalLinkMatch[] => {
    const pattern = new RegExp(TERMINAL_LINK_PATH_PATTERN)
    const matches: TerminalLinkMatch[] = []
    let execResult: RegExpExecArray | null

    while ((execResult = pattern.exec(text))) {
        const [full, path] = execResult
        const startIndex = execResult.index + full.length - path.length
        const pathEndIndex = startIndex + path.length
        const suffix = matchTerminalLinkSuffix(text.slice(pathEndIndex))
        const lineToken = suffix?.[1]
        const columnToken = suffix?.[2]
        const endIndex = pathEndIndex + (suffix?.[0].length ?? 0)

        matches.push({
            path,
            line: lineToken !== undefined ? Number(lineToken) : undefined,
            column: columnToken !== undefined ? Number(columnToken) : undefined,
            startIndex,
            endIndex,
            text: text.slice(startIndex, endIndex),
        })

        pattern.lastIndex = endIndex
    }

    return matches
}
