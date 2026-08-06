export type TerminalLinkMatch = {
    path: string
    line: number | undefined
    column: number | undefined
    startIndex: number
    endIndex: number
    text: string
}

const TERMINAL_LINK_PATTERN = /(?:^|[\s'"`([<])((?:~|\.{1,2})?\/?[\w.@+-]+(?:\/[\w.@+-]+)*\.[A-Za-z0-9]{1,10})(?::(\d+))?(?::(\d+))?/g

export const findTerminalLinkMatches = (text: string): TerminalLinkMatch[] => {
    const pattern = new RegExp(TERMINAL_LINK_PATTERN)
    const matches: TerminalLinkMatch[] = []
    let execResult: RegExpExecArray | null

    while ((execResult = pattern.exec(text))) {
        const [full, path, lineToken, columnToken] = execResult
        const pathOffsetInFull = full.indexOf(path)
        const startIndex = execResult.index + pathOffsetInFull
        const matchedText = full.slice(pathOffsetInFull)
        const endIndex = startIndex + matchedText.length

        matches.push({
            path,
            line: lineToken !== undefined ? Number(lineToken) : undefined,
            column: columnToken !== undefined ? Number(columnToken) : undefined,
            startIndex,
            endIndex,
            text: matchedText,
        })

        if (pattern.lastIndex === execResult.index) pattern.lastIndex += 1
    }

    return matches
}
